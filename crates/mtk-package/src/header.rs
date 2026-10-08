//! 64-byte fixed package header specification and encoding/decoding.

use crate::error::{PackageError, PackageResult};

/// Fixed size of MTK package header in bytes.
pub const HEADER_SIZE: usize = 64;

/// Magic bytes for MTK package files: `b"MTKP"`.
pub const MAGIC_MTKP: [u8; 4] = [0x4D, 0x54, 0x4B, 0x50];

/// Current major version of MTK package format.
pub const CURRENT_VERSION_MAJOR: u16 = 1;

/// Current minor version of MTK package format.
pub const CURRENT_VERSION_MINOR: u16 = 0;

/// Package profile representing the purpose and layout rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum PackageProfile {
    /// Full precompiled asset cache (Atlas, Standalone, Models, Biomes).
    AssetCache = 0,
    /// Lightweight scene interchange (sparse voxels, tree-shaken textures/models).
    SceneInterchange = 1,
}

impl PackageProfile {
    pub fn from_u16(val: u16) -> Option<Self> {
        match val {
            0 => Some(Self::AssetCache),
            1 => Some(Self::SceneInterchange),
            _ => None,
        }
    }
}

/// Default or per-chunk compression algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CompressionType {
    None = 0,
    Zstd = 1,
    Lz4 = 2,
}

impl CompressionType {
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            0 => Some(Self::None),
            1 => Some(Self::Zstd),
            2 => Some(Self::Lz4),
            _ => None,
        }
    }
}

/// Checksum algorithm for header and chunk data integrity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ChecksumType {
    Crc32 = 0,
    XxHash64 = 1,
}

impl ChecksumType {
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            0 => Some(Self::Crc32),
            1 => Some(Self::XxHash64),
            _ => None,
        }
    }
}

/// Flags bitfield in package header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeaderFlags(pub u16);

impl HeaderFlags {
    pub const FLAG_LITTLE_ENDIAN: u16 = 1 << 0;
    pub const FLAG_TOC_ZSTD: u16 = 1 << 1;
    pub const FLAG_SELF_CONTAINED: u16 = 1 << 2;
    pub const FLAG_EXTERNAL_REF: u16 = 1 << 3;

    pub fn default_asset_cache() -> Self {
        Self(Self::FLAG_LITTLE_ENDIAN | Self::FLAG_TOC_ZSTD | Self::FLAG_SELF_CONTAINED)
    }

    pub fn is_little_endian(&self) -> bool {
        (self.0 & Self::FLAG_LITTLE_ENDIAN) != 0
    }

    pub fn is_toc_zstd(&self) -> bool {
        (self.0 & Self::FLAG_TOC_ZSTD) != 0
    }

    pub fn is_self_contained(&self) -> bool {
        (self.0 & Self::FLAG_SELF_CONTAINED) != 0
    }
}

/// 64-byte fixed package header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageHeader {
    pub magic: [u8; 4],
    pub version_major: u16,
    pub version_minor: u16,
    pub profile: PackageProfile,
    pub flags: HeaderFlags,
    pub compression: CompressionType,
    pub checksum_type: ChecksumType,
    pub reserved: [u8; 2],
    pub created_at: u64,
    pub fingerprint: [u8; 16],
    pub toc_offset: u64,
    pub toc_length: u64,
    pub chunk_count: u32,
    pub header_crc32: u32,
}

impl PackageHeader {
    /// Constructs a new AssetCache header with standard defaults.
    pub fn new_asset_cache(fingerprint: [u8; 16], created_at: u64) -> Self {
        Self {
            magic: MAGIC_MTKP,
            version_major: CURRENT_VERSION_MAJOR,
            version_minor: CURRENT_VERSION_MINOR,
            profile: PackageProfile::AssetCache,
            flags: HeaderFlags::default_asset_cache(),
            compression: CompressionType::Zstd,
            checksum_type: ChecksumType::Crc32,
            reserved: [0, 0],
            created_at,
            fingerprint,
            toc_offset: 0,
            toc_length: 0,
            chunk_count: 0,
            header_crc32: 0,
        }
    }

    /// Serializes header into a 64-byte array and calculates CRC32 for the first 60 bytes.
    pub fn to_bytes(&self) -> [u8; HEADER_SIZE] {
        let mut buf = [0u8; HEADER_SIZE];
        buf[0..4].copy_from_slice(&self.magic);
        buf[4..6].copy_from_slice(&self.version_major.to_le_bytes());
        buf[6..8].copy_from_slice(&self.version_minor.to_le_bytes());
        buf[8..10].copy_from_slice(&(self.profile as u16).to_le_bytes());
        buf[10..12].copy_from_slice(&self.flags.0.to_le_bytes());
        buf[12] = self.compression as u8;
        buf[13] = self.checksum_type as u8;
        buf[14..16].copy_from_slice(&self.reserved);
        buf[16..24].copy_from_slice(&self.created_at.to_le_bytes());
        buf[24..40].copy_from_slice(&self.fingerprint);
        buf[40..48].copy_from_slice(&self.toc_offset.to_le_bytes());
        buf[48..56].copy_from_slice(&self.toc_length.to_le_bytes());
        buf[56..60].copy_from_slice(&self.chunk_count.to_le_bytes());

        // Calculate CRC32 of first 60 bytes
        let calculated_crc = crc32fast::hash(&buf[0..60]);
        buf[60..64].copy_from_slice(&calculated_crc.to_le_bytes());

        buf
    }

    /// Deserializes and validates a 64-byte slice.
    pub fn from_bytes(slice: &[u8]) -> PackageResult<Self> {
        if slice.len() < HEADER_SIZE {
            return Err(PackageError::OutOfBounds {
                offset: 0,
                length: HEADER_SIZE as u64,
                file_size: slice.len() as u64,
            });
        }

        let mut magic = [0u8; 4];
        magic.copy_from_slice(&slice[0..4]);
        if magic != MAGIC_MTKP {
            return Err(PackageError::InvalidMagic {
                expected: MAGIC_MTKP,
                found: magic,
            });
        }

        let expected_crc = u32::from_le_bytes([slice[60], slice[61], slice[62], slice[63]]);
        let calculated_crc = crc32fast::hash(&slice[0..60]);
        if expected_crc != calculated_crc {
            return Err(PackageError::HeaderCrcMismatch {
                expected: expected_crc,
                calculated: calculated_crc,
            });
        }

        let version_major = u16::from_le_bytes([slice[4], slice[5]]);
        let version_minor = u16::from_le_bytes([slice[6], slice[7]]);
        if version_major != CURRENT_VERSION_MAJOR {
            return Err(PackageError::UnsupportedVersion {
                major: version_major,
                minor: version_minor,
            });
        }

        let profile_raw = u16::from_le_bytes([slice[8], slice[9]]);
        let profile = PackageProfile::from_u16(profile_raw).ok_or_else(|| {
            PackageError::Serialization(format!("Unknown package profile: {}", profile_raw))
        })?;

        let flags = HeaderFlags(u16::from_le_bytes([slice[10], slice[11]]));
        let compression_raw = slice[12];
        let compression = CompressionType::from_u8(compression_raw).ok_or_else(|| {
            PackageError::Serialization(format!("Unknown compression type: {}", compression_raw))
        })?;

        let checksum_type_raw = slice[13];
        let checksum_type = ChecksumType::from_u8(checksum_type_raw).ok_or_else(|| {
            PackageError::Serialization(format!("Unknown checksum type: {}", checksum_type_raw))
        })?;

        let mut reserved = [0u8; 2];
        reserved.copy_from_slice(&slice[14..16]);

        let created_at = u64::from_le_bytes(slice[16..24].try_into().unwrap());

        let mut fingerprint = [0u8; 16];
        fingerprint.copy_from_slice(&slice[24..40]);

        let toc_offset = u64::from_le_bytes(slice[40..48].try_into().unwrap());
        let toc_length = u64::from_le_bytes(slice[48..56].try_into().unwrap());
        let chunk_count = u32::from_le_bytes(slice[56..60].try_into().unwrap());

        Ok(Self {
            magic,
            version_major,
            version_minor,
            profile,
            flags,
            compression,
            checksum_type,
            reserved,
            created_at,
            fingerprint,
            toc_offset,
            toc_length,
            chunk_count,
            header_crc32: expected_crc,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_roundtrip_and_crc() {
        let mut header = PackageHeader::new_asset_cache([42u8; 16], 1728345600);
        header.toc_offset = 1024;
        header.toc_length = 512;
        header.chunk_count = 8;

        let bytes = header.to_bytes();
        assert_eq!(bytes.len(), 64);

        let parsed = PackageHeader::from_bytes(&bytes).expect("Failed to parse valid header");
        assert_eq!(parsed.magic, MAGIC_MTKP);
        assert_eq!(parsed.version_major, 1);
        assert_eq!(parsed.version_minor, 0);
        assert_eq!(parsed.profile, PackageProfile::AssetCache);
        assert_eq!(parsed.fingerprint, [42u8; 16]);
        assert_eq!(parsed.toc_offset, 1024);
        assert_eq!(parsed.toc_length, 512);
        assert_eq!(parsed.chunk_count, 8);
    }

    #[test]
    fn test_corrupt_header_crc() {
        let header = PackageHeader::new_asset_cache([0u8; 16], 100);
        let mut bytes = header.to_bytes();
        bytes[10] ^= 0xFF; // Corrupt a flag byte

        let err = PackageHeader::from_bytes(&bytes).unwrap_err();
        match err {
            PackageError::HeaderCrcMismatch { .. } => {}
            other => panic!("Expected HeaderCrcMismatch, got {:?}", other),
        }
    }
}

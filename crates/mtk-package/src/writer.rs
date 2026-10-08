//! Streaming package builder and writer.

use std::fs::File;
use std::io::{Cursor, Seek, SeekFrom, Write};
use std::path::Path;

use crate::error::{PackageError, PackageResult};
use crate::header::{CompressionType, HeaderFlags, PackageHeader, PackageProfile, HEADER_SIZE};
use crate::toc::{ChunkEntry, TableOfContents};

/// Chunk compression and serialization options.
#[derive(Debug, Clone)]
pub struct ChunkWriteOptions {
    pub compression: CompressionType,
    pub zstd_level: i32,
}

impl Default for ChunkWriteOptions {
    fn default() -> Self {
        Self {
            compression: CompressionType::Zstd,
            zstd_level: 3,
        }
    }
}

impl ChunkWriteOptions {
    /// No compression (recommended for PNG/JPEG images).
    pub fn raw() -> Self {
        Self {
            compression: CompressionType::None,
            zstd_level: 0,
        }
    }

    /// Fast Zstd compression (level 3, default).
    pub fn zstd_fast() -> Self {
        Self {
            compression: CompressionType::Zstd,
            zstd_level: 3,
        }
    }

    /// Higher Zstd compression (level 7).
    pub fn zstd_high() -> Self {
        Self {
            compression: CompressionType::Zstd,
            zstd_level: 7,
        }
    }
}

/// Streaming writer for MTK binary package files.
pub struct MtkPackageWriter<W: Write + Seek> {
    writer: W,
    header: PackageHeader,
    toc: TableOfContents,
    current_offset: u64,
}

impl MtkPackageWriter<File> {
    /// Creates a new package writer creating or truncating a file at `path`.
    pub fn create(
        path: impl AsRef<Path>,
        profile: PackageProfile,
        fingerprint: [u8; 16],
    ) -> PackageResult<Self> {
        let file = File::create(path)?;
        Self::new(file, profile, fingerprint)
    }
}

impl<W: Write + Seek> MtkPackageWriter<W> {
    /// Creates a new package writer from a seekable stream.
    pub fn new(
        mut writer: W,
        profile: PackageProfile,
        fingerprint: [u8; 16],
    ) -> PackageResult<Self> {
        let epoch_now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let header = match profile {
            PackageProfile::AssetCache => PackageHeader::new_asset_cache(fingerprint, epoch_now),
            PackageProfile::SceneInterchange => {
                let mut h = PackageHeader::new_asset_cache(fingerprint, epoch_now);
                h.profile = PackageProfile::SceneInterchange;
                h.flags = HeaderFlags(
                    HeaderFlags::FLAG_LITTLE_ENDIAN
                        | HeaderFlags::FLAG_TOC_ZSTD
                        | HeaderFlags::FLAG_SELF_CONTAINED,
                );
                h
            }
        };

        // Write placeholder for 64-byte header
        let placeholder = [0u8; HEADER_SIZE];
        writer.write_all(&placeholder)?;

        Ok(Self {
            writer,
            header,
            toc: TableOfContents::new(),
            current_offset: HEADER_SIZE as u64,
        })
    }

    /// Adds a chunk to the package. All chunks are automatically aligned to 64-byte boundaries.
    pub fn add_chunk(
        &mut self,
        chunk_type: [u8; 4],
        identifier: impl Into<String>,
        decompressed_data: &[u8],
        options: &ChunkWriteOptions,
    ) -> PackageResult<()> {
        let id = identifier.into();

        // 1. Ensure 64-byte alignment
        let remainder = self.current_offset % 64;
        if remainder != 0 {
            let padding_needed = (64 - remainder) as usize;
            let padding = vec![0u8; padding_needed];
            self.writer.write_all(&padding)?;
            self.current_offset += padding_needed as u64;
        }

        let chunk_start_offset = self.current_offset;
        let decompressed_size = decompressed_data.len() as u64;

        // 2. Compute CRC32 checksum on decompressed payload
        let checksum = crc32fast::hash(decompressed_data) as u64;

        // 3. Compress data according to options
        let (compressed_bytes, actual_compression) = match options.compression {
            CompressionType::None => (decompressed_data.to_vec(), CompressionType::None),
            CompressionType::Zstd => {
                let compressed =
                    zstd::encode_all(Cursor::new(decompressed_data), options.zstd_level).map_err(
                        |e| PackageError::CompressionFailed {
                            identifier: id.clone(),
                            source: e,
                        },
                    )?;
                // If compressed size is larger than uncompressed, fallback to raw
                if compressed.len() >= decompressed_data.len() {
                    (decompressed_data.to_vec(), CompressionType::None)
                } else {
                    (compressed, CompressionType::Zstd)
                }
            }
            CompressionType::Lz4 => {
                // Future expansion: fallback to raw or error
                (decompressed_data.to_vec(), CompressionType::None)
            }
        };

        let compressed_size = compressed_bytes.len() as u64;

        // 4. Write payload
        self.writer.write_all(&compressed_bytes)?;
        self.current_offset += compressed_size;

        // 5. Record TOC entry
        self.toc.add_entry(ChunkEntry {
            chunk_type,
            offset: chunk_start_offset,
            compressed_size,
            decompressed_size,
            compression: actual_compression,
            reserved: [0; 7],
            checksum,
            identifier: id,
        });

        Ok(())
    }

    /// Appends string data as a chunk (encoded as UTF-8).
    pub fn add_str_chunk(
        &mut self,
        chunk_type: [u8; 4],
        identifier: impl Into<String>,
        str_data: &str,
        options: &ChunkWriteOptions,
    ) -> PackageResult<()> {
        self.add_chunk(chunk_type, identifier, str_data.as_bytes(), options)
    }

    /// Appends JSON-serializable data as a chunk.
    pub fn add_json_chunk<T: serde::Serialize>(
        &mut self,
        chunk_type: [u8; 4],
        identifier: impl Into<String>,
        value: &T,
        options: &ChunkWriteOptions,
    ) -> PackageResult<()> {
        let json_bytes =
            serde_json::to_vec(value).map_err(|e| PackageError::Serialization(e.to_string()))?;
        self.add_chunk(chunk_type, identifier, &json_bytes, options)
    }

    /// Finalizes the package: writes TOC, updates and writes final 64-byte header.
    pub fn finish(mut self) -> PackageResult<PackageHeader> {
        // 1. Align TOC to 64-byte boundary
        let remainder = self.current_offset % 64;
        if remainder != 0 {
            let padding_needed = (64 - remainder) as usize;
            let padding = vec![0u8; padding_needed];
            self.writer.write_all(&padding)?;
            self.current_offset += padding_needed as u64;
        }

        let toc_offset = self.current_offset;

        // 2. Encode TOC (Zstd compressed if flag is set)
        let toc_bytes = if self.header.flags.is_toc_zstd() {
            self.toc.encode_zstd(3)?
        } else {
            self.toc.encode_bytes()?
        };

        let toc_length = toc_bytes.len() as u64;
        self.writer.write_all(&toc_bytes)?;
        self.current_offset += toc_length;

        // 3. Update header fields
        self.header.toc_offset = toc_offset;
        self.header.toc_length = toc_length;
        self.header.chunk_count = self.toc.entries.len() as u32;

        // 4. Seek to 0 and write finalized 64-byte header
        self.writer.seek(SeekFrom::Start(0))?;
        let header_bytes = self.header.to_bytes();
        self.writer.write_all(&header_bytes)?;
        self.writer.flush()?;

        Ok(self.header)
    }
}

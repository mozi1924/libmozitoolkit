use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use flate2::read::{GzDecoder, ZlibDecoder};

use crate::error::SaveError;

/// Number of chunks along one axis of a 32x32 Anvil region file.
pub const REGION_CHUNKS_AXIS: usize = 32;
/// Total number of chunks in a single region file (32 * 32 = 1024).
pub const REGION_TOTAL_CHUNKS: usize = REGION_CHUNKS_AXIS * REGION_CHUNKS_AXIS;
/// Size of the Anvil location header in bytes (1024 entries * 4 bytes = 4096).
pub const REGION_HEADER_SIZE: usize = 4096;
/// Size of one Anvil sector in bytes (4 KiB).
pub const SECTOR_SIZE: usize = 4096;

/// Location entry inside an Anvil region file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkLocation {
    /// Offset in 4 KiB sectors from the start of the file.
    pub sector_offset: u32,
    /// Number of 4 KiB sectors allocated for this chunk.
    pub sector_count: u8,
}

impl ChunkLocation {
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.sector_offset == 0 || self.sector_count == 0
    }

    #[inline]
    pub fn file_offset(&self) -> u64 {
        (self.sector_offset as u64) * (SECTOR_SIZE as u64)
    }
}

/// Anvil MCA region file reader with direct sector seeking and zero unneeded decompression.
#[derive(Debug)]
pub struct RegionFile {
    path: PathBuf,
    file: File,
    locations: [ChunkLocation; REGION_TOTAL_CHUNKS],
}

impl RegionFile {
    /// Opens an Anvil `.mca` region file and loads its 4096-byte chunk location header.
    pub fn open(path: &Path) -> Result<Self, SaveError> {
        let mut file = File::open(path)?;
        let mut header = [0u8; REGION_HEADER_SIZE];
        file.read_exact(&mut header)
            .map_err(|e| SaveError::InvalidRegionHeader(format!("Header read failed for {}: {}", path.display(), e)))?;

        let mut locations = [ChunkLocation { sector_offset: 0, sector_count: 0 }; REGION_TOTAL_CHUNKS];
        for i in 0..REGION_TOTAL_CHUNKS {
            let offset_idx = i * 4;
            let sector_offset = ((header[offset_idx] as u32) << 16)
                | ((header[offset_idx + 1] as u32) << 8)
                | (header[offset_idx + 2] as u32);
            let sector_count = header[offset_idx + 3];
            locations[i] = ChunkLocation {
                sector_offset,
                sector_count,
            };
        }

        Ok(Self {
            path: path.to_path_buf(),
            file,
            locations,
        })
    }

    /// Computes the linear chunk index (0..1024) within a region from world chunk coordinates.
    #[inline]
    pub fn chunk_index(chunk_x: i32, chunk_z: i32) -> usize {
        let rel_x = chunk_x.rem_euclid(REGION_CHUNKS_AXIS as i32) as usize;
        let rel_z = chunk_z.rem_euclid(REGION_CHUNKS_AXIS as i32) as usize;
        rel_x + rel_z * REGION_CHUNKS_AXIS
    }

    /// Checks if a chunk at world chunk coordinates `(chunk_x, chunk_z)` exists in this region file.
    #[inline]
    pub fn has_chunk(&self, chunk_x: i32, chunk_z: i32) -> bool {
        let idx = Self::chunk_index(chunk_x, chunk_z);
        !self.locations[idx].is_empty()
    }

    /// Gets the chunk location entry for world chunk coordinates `(chunk_x, chunk_z)`.
    #[inline]
    pub fn chunk_location(&self, chunk_x: i32, chunk_z: i32) -> ChunkLocation {
        let idx = Self::chunk_index(chunk_x, chunk_z);
        self.locations[idx]
    }

    /// Reads and decompresses chunk payload bytes for world chunk `(chunk_x, chunk_z)`.
    ///
    /// Returns `Ok(None)` if the chunk is not generated or empty.
    pub fn read_chunk_decompressed(
        &mut self,
        chunk_x: i32,
        chunk_z: i32,
    ) -> Result<Option<Vec<u8>>, SaveError> {
        let loc = self.chunk_location(chunk_x, chunk_z);
        if loc.is_empty() {
            return Ok(None);
        }

        // Seek to chunk sector offset
        self.file.seek(SeekFrom::Start(loc.file_offset()))?;

        // 4 bytes: length of chunk payload (big endian)
        let mut len_buf = [0u8; 4];
        self.file.read_exact(&mut len_buf)?;
        let chunk_len = u32::from_be_bytes(len_buf);

        if chunk_len == 0 {
            return Ok(None);
        }

        // 1 byte: compression type
        let mut comp_type = [0u8; 1];
        self.file.read_exact(&mut comp_type)?;

        let compressed_size = (chunk_len - 1) as usize;
        let mut compressed_data = vec![0u8; compressed_size];
        self.file.read_exact(&mut compressed_data)?;

        let mut decompressed = Vec::new();
        match comp_type[0] {
            1 => {
                // GZip
                let mut gz = GzDecoder::new(&compressed_data[..]);
                gz.read_to_end(&mut decompressed)
                    .map_err(|e| SaveError::DecompressionFailed(format!("GZip decompression error: {}", e)))?;
            }
            2 => {
                // Zlib / Deflate (standard Java Edition)
                let mut zlib = ZlibDecoder::new(&compressed_data[..]);
                zlib.read_to_end(&mut decompressed)
                    .map_err(|e| SaveError::DecompressionFailed(format!("Zlib decompression error: {}", e)))?;
            }
            3 => {
                // Uncompressed
                decompressed = compressed_data;
            }
            other => return Err(SaveError::UnsupportedCompression(other)),
        }

        Ok(Some(decompressed))
    }

    /// Returns the file path of this region file.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

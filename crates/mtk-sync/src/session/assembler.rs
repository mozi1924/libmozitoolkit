//! # Baseline Stream Assembler (RFC v3.1 Section 4.3 - 4.6 & Section 5.1)
//!
//! Reconstructs a full VoxelStorage volume from progressive chunk streams,
//! verifies chunk xxHash64, decompresses Zstd payloads, and validates the canonical state_hash SSOT.

use std::collections::HashMap;
use std::io::Read;

use glam::IVec3;
use mtk_voxel::storage::{SectionBiomes, SectionStorage, VoxelStorage};

use crate::protocol::constants::{CancelReason, ResyncReason, MAX_SELECTION_VOLUME};
use crate::protocol::hasher::compute_canonical_state_hash;
use crate::protocol::packet::Packet;

/// Helper to unpack LSB-first bit-packed indices into u16 vector.
pub fn unpack_lsb_indices(bytes: &[u8], count: usize, bits_per_item: u8) -> Vec<u16> {
    if count == 0 {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(count);

    match bits_per_item {
        8 => {
            for &b in bytes.iter().take(count) {
                out.push(b as u16);
            }
        }
        16 => {
            let limit = count.min(bytes.len() / 2);
            for i in 0..limit {
                let val = u16::from_le_bytes([bytes[i * 2], bytes[i * 2 + 1]]);
                out.push(val);
            }
        }
        1 => {
            let mut extracted = 0;
            for &byte in bytes {
                for bit in 0..8 {
                    if extracted >= count {
                        break;
                    }
                    out.push(((byte >> bit) & 0x01) as u16);
                    extracted += 1;
                }
                if extracted >= count {
                    break;
                }
            }
        }
        2 => {
            let mut extracted = 0;
            for &byte in bytes {
                for slot in 0..4 {
                    if extracted >= count {
                        break;
                    }
                    out.push(((byte >> (slot * 2)) & 0x03) as u16);
                    extracted += 1;
                }
                if extracted >= count {
                    break;
                }
            }
        }
        4 => {
            let mut extracted = 0;
            for &byte in bytes {
                for slot in 0..2 {
                    if extracted >= count {
                        break;
                    }
                    out.push(((byte >> (slot * 4)) & 0x0F) as u16);
                    extracted += 1;
                }
                if extracted >= count {
                    break;
                }
            }
        }
        _ => {
            // Fallback: unpack as byte-aligned if unrecognized bit width
            for &b in bytes.iter().take(count) {
                out.push(b as u16);
            }
        }
    }

    out
}

/// Baseline assembly state machine.
pub struct BaselineAssembler {
    pub epoch: u64,
    pub snapshot_uuid: [u8; 16],
    pub snapshot_seq: u64,
    pub min_pos: IVec3,
    pub size: IVec3,
    pub total_voxels: usize,
    pub state_hash: u64,
    pub palette_revision: u32,
    pub total_chunks: u32,
    pub compression_mode: u8,
    pub palette_bits_per_block: u8,
    pub biome_bits_per_quart: u8,
    pub dimension: String,
    pub block_palette: Vec<String>,
    pub biome_palette: Vec<String>,

    pub chunks_received: u32,
    pub expected_next_voxel_index: u64,
    pub raw_voxel_indices: Vec<u16>,
    pub raw_quart_biome_indices: Vec<u16>,
}

impl BaselineAssembler {
    /// Creates a new assembler from BASELINE_BEGIN packet.
    pub fn new(begin_pkt: &Packet) -> Result<Self, CancelReason> {
        let (
            epoch,
            snapshot_uuid,
            snapshot_seq,
            min_pos,
            size,
            state_hash,
            palette_revision,
            total_chunks,
            _total_encoded_bytes,
            _total_decoded_bytes,
            compression_mode,
            palette_bits_per_block,
            biome_bits_per_quart,
            dimension,
            block_palette,
            biome_palette,
        ) = match begin_pkt {
            Packet::BaselineBegin {
                epoch,
                snapshot_uuid,
                snapshot_seq,
                min_pos,
                size,
                state_hash,
                palette_revision,
                total_chunks,
                total_encoded_bytes,
                total_decoded_bytes,
                compression_mode,
                palette_bits_per_block,
                biome_bits_per_quart,
                dimension,
                block_palette,
                biome_palette,
            } => (
                *epoch,
                *snapshot_uuid,
                *snapshot_seq,
                *min_pos,
                *size,
                *state_hash,
                *palette_revision,
                *total_chunks,
                *total_encoded_bytes,
                *total_decoded_bytes,
                *compression_mode,
                *palette_bits_per_block,
                *biome_bits_per_quart,
                dimension.clone(),
                block_palette.clone(),
                biome_palette.clone(),
            ),
            _ => return Err(CancelReason::ReasonBadChunk),
        };

        if size.x <= 0 || size.y <= 0 || size.z <= 0 {
            return Err(CancelReason::ReasonBadChunk);
        }

        let total_voxels = (size.x as u64)
            .checked_mul(size.y as u64)
            .and_then(|v| v.checked_mul(size.z as u64))
            .ok_or(CancelReason::ReasonBadChunk)?;

        if total_voxels > MAX_SELECTION_VOLUME {
            return Err(CancelReason::ReasonBadChunk);
        }

        Ok(Self {
            epoch,
            snapshot_uuid,
            snapshot_seq,
            min_pos,
            size,
            total_voxels: total_voxels as usize,
            state_hash,
            palette_revision,
            total_chunks,
            compression_mode,
            palette_bits_per_block,
            biome_bits_per_quart,
            dimension,
            block_palette,
            biome_palette,
            chunks_received: 0,
            expected_next_voxel_index: 0,
            raw_voxel_indices: Vec::with_capacity(total_voxels as usize),
            raw_quart_biome_indices: Vec::new(),
        })
    }

    /// Appends and processes a single BASELINE_CHUNK.
    pub fn process_chunk(&mut self, chunk: &Packet) -> Result<(), ResyncReason> {
        let (
            epoch,
            snapshot_uuid,
            chunk_index,
            total_chunks,
            first_voxel_index,
            voxel_count,
            _payload_len,
            decoded_target_bytes,
            _chunk_xxhash64,
            payload,
        ) = match chunk {
            Packet::BaselineChunk {
                epoch,
                snapshot_uuid,
                chunk_index,
                total_chunks,
                first_voxel_index,
                voxel_count,
                payload_len,
                decoded_target_bytes,
                chunk_xxhash64,
                payload,
            } => (
                *epoch,
                *snapshot_uuid,
                *chunk_index,
                *total_chunks,
                *first_voxel_index,
                *voxel_count,
                *payload_len,
                *decoded_target_bytes,
                *chunk_xxhash64,
                payload,
            ),
            _ => return Err(ResyncReason::ChunkCorrupted),
        };

        if epoch != self.epoch || snapshot_uuid != self.snapshot_uuid {
            return Err(ResyncReason::ChunkCorrupted);
        }

        if total_chunks != self.total_chunks {
            return Err(ResyncReason::ChunkCorrupted);
        }

        if chunk_index != self.chunks_received {
            return Err(ResyncReason::SequenceGap);
        }

        if first_voxel_index != self.expected_next_voxel_index {
            return Err(ResyncReason::SequenceGap);
        }

        // Decompress payload
        let decompressed: Vec<u8> = if self.compression_mode == 1 {
            let mut decoder = zstd::stream::Decoder::new(&payload[..])
                .map_err(|_| ResyncReason::ChunkCorrupted)?;
            let mut buf = Vec::with_capacity(decoded_target_bytes as usize);
            decoder
                .read_to_end(&mut buf)
                .map_err(|_| ResyncReason::ChunkCorrupted)?;
            buf
        } else {
            payload.clone()
        };

        // Unpack voxel indices
        let bits = if self.palette_bits_per_block > 0 {
            self.palette_bits_per_block
        } else if self.block_palette.len() > 256 {
            16
        } else {
            8
        };

        let unpacked = unpack_lsb_indices(&decompressed, voxel_count as usize, bits);
        self.raw_voxel_indices.extend(unpacked);

        // Check if there are remaining biome bytes at the end of the chunk
        let voxel_bytes_len = (voxel_count as usize * bits as usize).div_ceil(8);
        if decompressed.len() > voxel_bytes_len {
            let biome_slice = &decompressed[voxel_bytes_len..];
            let biome_bits = if self.biome_bits_per_quart > 0 {
                self.biome_bits_per_quart
            } else {
                8
            };
            let remaining_biomes = biome_slice.len() * 8 / biome_bits as usize;
            let unpacked_biomes = unpack_lsb_indices(biome_slice, remaining_biomes, biome_bits);
            self.raw_quart_biome_indices.extend(unpacked_biomes);
        }

        self.expected_next_voxel_index += voxel_count as u64;
        self.chunks_received += 1;

        Ok(())
    }

    /// Validates BASELINE_END, reconstructs VoxelStorage, and verifies state_hash SSOT.
    pub fn finalize(&self, end_pkt: &Packet) -> Result<VoxelStorage, ResyncReason> {
        let (epoch, snapshot_uuid, snapshot_seq, total_chunks_sent, final_state_hash, status) =
            match end_pkt {
                Packet::BaselineEnd {
                    epoch,
                    snapshot_uuid,
                    snapshot_seq,
                    total_chunks_sent,
                    final_state_hash,
                    status,
                } => (
                    *epoch,
                    *snapshot_uuid,
                    *snapshot_seq,
                    *total_chunks_sent,
                    *final_state_hash,
                    *status,
                ),
                _ => return Err(ResyncReason::ChunkCorrupted),
            };

        if status != 0 {
            return Err(ResyncReason::ChunkCorrupted);
        }

        if epoch != self.epoch
            || snapshot_uuid != self.snapshot_uuid
            || snapshot_seq != self.snapshot_seq
        {
            return Err(ResyncReason::ChunkCorrupted);
        }

        if total_chunks_sent != self.chunks_received {
            return Err(ResyncReason::SequenceGap);
        }

        if self.raw_voxel_indices.len() < self.total_voxels {
            return Err(ResyncReason::ChunkCorrupted);
        }

        // Build VoxelStorage
        let mut storage = VoxelStorage::new();
        storage.min_x = self.min_pos.x;
        storage.min_y = self.min_pos.y;
        storage.min_z = self.min_pos.z;
        storage.size_x = self.size.x;
        storage.size_y = self.size.y;
        storage.size_z = self.size.z;
        storage.has_explicit_bounds = true;

        if let Some(first_biome) = self.biome_palette.first() {
            storage.primary_biome = Some(first_biome.clone());
        }

        let p_len = self.block_palette.len();
        let sx = self.size.x as usize;
        let _sy = self.size.y as usize;
        let sz = self.size.z as usize;
        let slice_area = sx * sz; // X-fastest, Z-next, Y-slowest

        let qw = (self.size.x + 3) / 4;
        let _qh = (self.size.y + 3) / 4;
        let qd = (self.size.z + 3) / 4;
        let quart_slice_area = (qw * qd) as usize;

        let mut section_biomes_map: HashMap<IVec3, SectionBiomes> = HashMap::new();

        for (idx, &p_idx_raw) in self
            .raw_voxel_indices
            .iter()
            .enumerate()
            .take(self.total_voxels)
        {
            let p_idx = p_idx_raw as usize;
            if p_idx >= p_len {
                continue;
            }
            let state_str = &self.block_palette[p_idx];

            // Invert RFC canonical 3D formula:
            // index = (x - min_x) + W * ((z - min_z) + D * (y - min_y))
            let ly = idx / slice_area;
            let rem = idx % slice_area;
            let lz = rem / sx;
            let lx = rem % sx;

            let wx = self.min_pos.x + lx as i32;
            let wy = self.min_pos.y + ly as i32;
            let wz = self.min_pos.z + lz as i32;

            let sec_coord = IVec3::new(wx >> 4, wy >> 4, wz >> 4);
            let sec = storage
                .sections
                .entry(sec_coord)
                .or_insert_with(|| SectionStorage::new(sec_coord));

            let blx = (wx & 15) as usize;
            let bly = (wy & 15) as usize;
            let blz = (wz & 15) as usize;

            sec.set_local(blx, bly, blz, state_str);
        }

        // Apply Biomes if present
        let bp_len = self.biome_palette.len();
        if bp_len > 0 && !self.raw_quart_biome_indices.is_empty() {
            for (q_idx, &b_idx_raw) in self.raw_quart_biome_indices.iter().enumerate() {
                let b_idx = b_idx_raw as usize;
                if b_idx >= bp_len {
                    continue;
                }
                let biome_str = &self.biome_palette[b_idx];

                let qy = (q_idx / quart_slice_area) as i32;
                let q_rem = q_idx % quart_slice_area;
                let qz = (q_rem / qw as usize) as i32;
                let qx = (q_rem % qw as usize) as i32;

                let wx = self.min_pos.x + qx * 4;
                let wy = self.min_pos.y + qy * 4;
                let wz = self.min_pos.z + qz * 4;

                let sec_coord = IVec3::new(wx >> 4, wy >> 4, wz >> 4);
                let sb = section_biomes_map.entry(sec_coord).or_insert_with(|| {
                    if bp_len == 1 {
                        SectionBiomes::uniform(biome_str)
                    } else {
                        SectionBiomes::new(self.biome_palette.clone(), vec![0u8; 64])
                    }
                });

                let sec_qx = (qx & 3) as usize;
                let sec_qy = (qy & 3) as usize;
                let sec_qz = (qz & 3) as usize;
                sb.set_quart_biome(sec_qx, sec_qy, sec_qz, biome_str);
            }

            for (sec_coord, sb) in section_biomes_map {
                if let Some(sec) = storage.sections.get_mut(&sec_coord) {
                    sec.biomes = Some(sb);
                }
            }
        }

        // Validate state_hash SSOT
        let computed_hash = compute_canonical_state_hash(
            self.min_pos,
            self.size,
            &self.dimension,
            |x, y, z| storage.get_block(x, y, z).to_string(),
            |qx, qy, qz| {
                let wx = self.min_pos.x + qx * 4;
                let wy = self.min_pos.y + qy * 4;
                let wz = self.min_pos.z + qz * 4;
                storage.get_biome(wx, wy, wz).to_string()
            },
        );

        if computed_hash != final_state_hash {
            return Err(ResyncReason::StateHashMismatch);
        }

        Ok(storage)
    }
}

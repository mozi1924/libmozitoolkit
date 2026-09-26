//! # Binary Packet Codec for Live Sync
//!
//! High-performance Little-Endian binary packet decoder and encoder.

use glam::IVec3;

use super::constants::*;
use super::error::ProtocolError;
use super::packet::*;

/// Decodes a raw binary WebSocket frame into a strongly-typed `Packet`.
pub fn decode_packet(data: &[u8]) -> Result<Packet, ProtocolError> {
    if data.len() < HEADER_SIZE {
        return Err(ProtocolError::PacketTooShort {
            expected: HEADER_SIZE,
            actual: data.len(),
        });
    }

    let magic = [data[0], data[1]];
    if magic != PROTOCOL_MAGIC {
        return Err(ProtocolError::InvalidMagic(data[0], data[1]));
    }

    let version = data[2];
    if version != PROTOCOL_VERSION {
        return Err(ProtocolError::UnsupportedVersion(version));
    }

    let packet_type_raw = data[3];
    let packet_type = PacketType::from_u8(packet_type_raw)
        .ok_or(ProtocolError::UnknownPacketType(packet_type_raw))?;

    let mut offset = HEADER_SIZE;

    match packet_type {
        PacketType::SelectionInfo => {
            if data.len() < offset + SELECTION_INFO_SIZE {
                return Err(ProtocolError::PacketTooShort {
                    expected: offset + SELECTION_INFO_SIZE,
                    actual: data.len(),
                });
            }
            let min_x = i32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
            let min_y = i32::from_le_bytes(data[offset + 4..offset + 8].try_into().unwrap());
            let min_z = i32::from_le_bytes(data[offset + 8..offset + 12].try_into().unwrap());
            let size_x = i32::from_le_bytes(data[offset + 12..offset + 16].try_into().unwrap());
            let size_y = i32::from_le_bytes(data[offset + 16..offset + 20].try_into().unwrap());
            let size_z = i32::from_le_bytes(data[offset + 20..offset + 24].try_into().unwrap());

            Ok(Packet::SelectionInfo {
                min_pos: IVec3::new(min_x, min_y, min_z),
                size: IVec3::new(size_x, size_y, size_z),
            })
        }

        PacketType::FullSnapshot => {
            if data.len() < offset + SELECTION_INFO_SIZE + 2 {
                return Err(ProtocolError::PacketTooShort {
                    expected: offset + SELECTION_INFO_SIZE + 2,
                    actual: data.len(),
                });
            }
            let min_x = i32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
            let min_y = i32::from_le_bytes(data[offset + 4..offset + 8].try_into().unwrap());
            let min_z = i32::from_le_bytes(data[offset + 8..offset + 12].try_into().unwrap());
            let size_x = i32::from_le_bytes(data[offset + 12..offset + 16].try_into().unwrap());
            let size_y = i32::from_le_bytes(data[offset + 16..offset + 20].try_into().unwrap());
            let size_z = i32::from_le_bytes(data[offset + 20..offset + 24].try_into().unwrap());
            offset += SELECTION_INFO_SIZE;

            let palette_count = u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap()) as usize;
            offset += 2;

            let mut palette = Vec::with_capacity(palette_count);
            for _ in 0..palette_count {
                if data.len() < offset + 2 {
                    return Err(ProtocolError::TruncatedPayload { field: "block_palette" });
                }
                let str_len = u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap()) as usize;
                offset += 2;
                if data.len() < offset + str_len {
                    return Err(ProtocolError::TruncatedPayload { field: "block_palette_entry" });
                }
                let s = std::str::from_utf8(&data[offset..offset + str_len])
                    .map_err(|e| ProtocolError::Utf8Error {
                        field: "block_palette",
                        detail: e.to_string(),
                    })?
                    .to_string();
                offset += str_len;
                palette.push(s);
            }

            if data.len() < offset + 1 {
                return Err(ProtocolError::TruncatedPayload { field: "index_format" });
            }
            let index_bytes_per_block = data[offset];
            offset += 1;

            let total_blocks = (size_x * size_y * size_z).max(0) as usize;
            let mut grid_indices = Vec::with_capacity(total_blocks);

            if index_bytes_per_block == 1 {
                if data.len() < offset + total_blocks {
                    return Err(ProtocolError::TruncatedPayload { field: "grid_indices_u8" });
                }
                for &b in &data[offset..offset + total_blocks] {
                    grid_indices.push(b as u16);
                }
                offset += total_blocks;
            } else if index_bytes_per_block == 2 {
                if data.len() < offset + total_blocks * 2 {
                    return Err(ProtocolError::TruncatedPayload { field: "grid_indices_u16" });
                }
                for i in 0..total_blocks {
                    let val = u16::from_le_bytes(data[offset + i * 2..offset + i * 2 + 2].try_into().unwrap());
                    grid_indices.push(val);
                }
                offset += total_blocks * 2;
            } else {
                return Err(ProtocolError::InvalidIndexFormat(index_bytes_per_block));
            }

            let mut biome_palette = None;
            let mut biome_indices = None;

            if data.len() >= offset + 2 {
                let b_count = u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap()) as usize;
                offset += 2;
                let mut bp = Vec::with_capacity(b_count);
                for _ in 0..b_count {
                    if data.len() < offset + 2 {
                        break;
                    }
                    let b_len = u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap()) as usize;
                    offset += 2;
                    if data.len() < offset + b_len {
                        break;
                    }
                    if let Ok(s) = std::str::from_utf8(&data[offset..offset + b_len]) {
                        bp.push(s.to_string());
                    }
                    offset += b_len;
                }

                if b_count == 1 {
                    biome_indices = Some(vec![0u16; total_blocks]);
                    biome_palette = Some(bp);
                } else if b_count > 1 && data.len() >= offset + 1 {
                    let b_idx_bytes = data[offset];
                    offset += 1;
                    let mut bi = Vec::with_capacity(total_blocks);
                    if b_idx_bytes == 1 && data.len() >= offset + total_blocks {
                        for &b in &data[offset..offset + total_blocks] {
                            bi.push(b as u16);
                        }
                    } else if b_idx_bytes == 2 && data.len() >= offset + total_blocks * 2 {
                        for i in 0..total_blocks {
                            let val = u16::from_le_bytes(data[offset + i * 2..offset + i * 2 + 2].try_into().unwrap());
                            bi.push(val);
                        }
                    }
                    biome_indices = Some(bi);
                    biome_palette = Some(bp);
                } else if !bp.is_empty() {
                    biome_palette = Some(bp);
                }
            }

            Ok(Packet::FullSnapshot {
                min_pos: IVec3::new(min_x, min_y, min_z),
                size: IVec3::new(size_x, size_y, size_z),
                palette,
                grid_indices,
                biome_palette,
                biome_indices,
            })
        }

        PacketType::DeltaUpdate => {
            if data.len() < offset + DELTA_HEADER_SIZE {
                return Err(ProtocolError::PacketTooShort {
                    expected: offset + DELTA_HEADER_SIZE,
                    actual: data.len(),
                });
            }
            let seq_id = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
            let min_x = i32::from_le_bytes(data[offset + 4..offset + 8].try_into().unwrap());
            let min_y = i32::from_le_bytes(data[offset + 8..offset + 12].try_into().unwrap());
            let min_z = i32::from_le_bytes(data[offset + 12..offset + 16].try_into().unwrap());
            let change_count = u16::from_le_bytes(data[offset + 16..offset + 18].try_into().unwrap()) as usize;
            offset += DELTA_HEADER_SIZE;

            let mut changes = Vec::with_capacity(change_count);
            for _ in 0..change_count {
                if data.len() < offset + DELTA_CHANGE_PREFIX_SIZE {
                    return Err(ProtocolError::TruncatedPayload { field: "delta_change_prefix" });
                }
                let rel_x = u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap()) as i32;
                let rel_y = u16::from_le_bytes(data[offset + 2..offset + 4].try_into().unwrap()) as i32;
                let rel_z = u16::from_le_bytes(data[offset + 4..offset + 6].try_into().unwrap()) as i32;
                let str_len = u16::from_le_bytes(data[offset + 6..offset + 8].try_into().unwrap()) as usize;
                offset += DELTA_CHANGE_PREFIX_SIZE;

                if data.len() < offset + str_len {
                    return Err(ProtocolError::TruncatedPayload { field: "delta_state_str" });
                }
                let state_str = std::str::from_utf8(&data[offset..offset + str_len])
                    .map_err(|e| ProtocolError::Utf8Error {
                        field: "delta_state",
                        detail: e.to_string(),
                    })?
                    .to_string();
                offset += str_len;

                changes.push(DeltaChange {
                    rel_pos: IVec3::new(rel_x, rel_y, rel_z),
                    state: state_str,
                });
            }

            Ok(Packet::DeltaUpdate {
                seq_id,
                min_pos: IVec3::new(min_x, min_y, min_z),
                changes,
            })
        }

        PacketType::SectionManifest => {
            if data.len() < offset + MANIFEST_HEADER_SIZE {
                return Err(ProtocolError::PacketTooShort {
                    expected: offset + MANIFEST_HEADER_SIZE,
                    actual: data.len(),
                });
            }
            let seq_id = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
            let section_count = u32::from_le_bytes(data[offset + 4..offset + 8].try_into().unwrap()) as usize;
            offset += MANIFEST_HEADER_SIZE;

            if data.len() < offset + section_count * MANIFEST_ENTRY_SIZE {
                return Err(ProtocolError::TruncatedPayload { field: "section_manifest_entries" });
            }

            let mut sections = Vec::with_capacity(section_count);
            for i in 0..section_count {
                let entry_offset = offset + i * MANIFEST_ENTRY_SIZE;
                let sx = i32::from_le_bytes(data[entry_offset..entry_offset + 4].try_into().unwrap());
                let sy = i32::from_le_bytes(data[entry_offset + 4..entry_offset + 8].try_into().unwrap());
                let sz = i32::from_le_bytes(data[entry_offset + 8..entry_offset + 12].try_into().unwrap());
                let crc = u32::from_le_bytes(data[entry_offset + 12..entry_offset + 16].try_into().unwrap());
                sections.push(ManifestSectionEntry {
                    coord: IVec3::new(sx, sy, sz),
                    crc32: crc,
                });
            }

            Ok(Packet::SectionManifest { seq_id, sections })
        }

        PacketType::SectionSnapshot => {
            if data.len() < offset + SECTION_SNAPSHOT_HEADER_SIZE {
                return Err(ProtocolError::PacketTooShort {
                    expected: offset + SECTION_SNAPSHOT_HEADER_SIZE,
                    actual: data.len(),
                });
            }
            let sec_x = i32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
            let sec_y = i32::from_le_bytes(data[offset + 4..offset + 8].try_into().unwrap());
            let sec_z = i32::from_le_bytes(data[offset + 8..offset + 12].try_into().unwrap());

            let start_x = i32::from_le_bytes(data[offset + 12..offset + 16].try_into().unwrap());
            let start_y = i32::from_le_bytes(data[offset + 16..offset + 20].try_into().unwrap());
            let start_z = i32::from_le_bytes(data[offset + 20..offset + 24].try_into().unwrap());

            let size_x = i32::from_le_bytes(data[offset + 24..offset + 28].try_into().unwrap());
            let size_y = i32::from_le_bytes(data[offset + 28..offset + 32].try_into().unwrap());
            let size_z = i32::from_le_bytes(data[offset + 32..offset + 36].try_into().unwrap());

            let palette_count = u16::from_le_bytes(data[offset + 36..offset + 38].try_into().unwrap()) as usize;
            offset += SECTION_SNAPSHOT_HEADER_SIZE;

            let mut palette = Vec::with_capacity(palette_count);
            for _ in 0..palette_count {
                if data.len() < offset + 2 {
                    return Err(ProtocolError::TruncatedPayload { field: "section_palette" });
                }
                let str_len = u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap()) as usize;
                offset += 2;
                if data.len() < offset + str_len {
                    return Err(ProtocolError::TruncatedPayload { field: "section_palette_entry" });
                }
                let s = std::str::from_utf8(&data[offset..offset + str_len])
                    .map_err(|e| ProtocolError::Utf8Error {
                        field: "section_palette",
                        detail: e.to_string(),
                    })?
                    .to_string();
                offset += str_len;
                palette.push(s);
            }

            if data.len() < offset + 1 {
                return Err(ProtocolError::TruncatedPayload { field: "section_index_format" });
            }
            let index_bytes = data[offset];
            offset += 1;

            let total_blocks = (size_x * size_y * size_z).max(0) as usize;
            let mut grid_indices = Vec::with_capacity(total_blocks);

            if index_bytes == 1 {
                if data.len() < offset + total_blocks {
                    return Err(ProtocolError::TruncatedPayload { field: "section_indices_u8" });
                }
                for &b in &data[offset..offset + total_blocks] {
                    grid_indices.push(b as u16);
                }
                offset += total_blocks;
            } else if index_bytes == 2 {
                if data.len() < offset + total_blocks * 2 {
                    return Err(ProtocolError::TruncatedPayload { field: "section_indices_u16" });
                }
                for i in 0..total_blocks {
                    let val = u16::from_le_bytes(data[offset + i * 2..offset + i * 2 + 2].try_into().unwrap());
                    grid_indices.push(val);
                }
                offset += total_blocks * 2;
            } else {
                return Err(ProtocolError::InvalidIndexFormat(index_bytes));
            }

            let mut biome_palette = None;
            let mut biome_indices = None;

            if data.len() >= offset + 2 {
                let b_count = u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap()) as usize;
                offset += 2;
                let mut bp = Vec::with_capacity(b_count);
                for _ in 0..b_count {
                    if data.len() < offset + 2 {
                        break;
                    }
                    let b_len = u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap()) as usize;
                    offset += 2;
                    if data.len() < offset + b_len {
                        break;
                    }
                    if let Ok(s) = std::str::from_utf8(&data[offset..offset + b_len]) {
                        bp.push(s.to_string());
                    }
                    offset += b_len;
                }

                if b_count == 1 {
                    biome_indices = Some(vec![0u16; total_blocks]);
                    biome_palette = Some(bp);
                } else if b_count > 1 && data.len() >= offset + 1 {
                    let b_idx_bytes = data[offset];
                    offset += 1;
                    let mut bi = Vec::with_capacity(total_blocks);
                    if b_idx_bytes == 1 && data.len() >= offset + total_blocks {
                        for &b in &data[offset..offset + total_blocks] {
                            bi.push(b as u16);
                        }
                    } else if b_idx_bytes == 2 && data.len() >= offset + total_blocks * 2 {
                        for i in 0..total_blocks {
                            let val = u16::from_le_bytes(data[offset + i * 2..offset + i * 2 + 2].try_into().unwrap());
                            bi.push(val);
                        }
                    }
                    biome_indices = Some(bi);
                    biome_palette = Some(bp);
                } else if !bp.is_empty() {
                    biome_palette = Some(bp);
                }
            }

            Ok(Packet::SectionSnapshot {
                sec_coord: IVec3::new(sec_x, sec_y, sec_z),
                start_pos: IVec3::new(start_x, start_y, start_z),
                size: IVec3::new(size_x, size_y, size_z),
                palette,
                grid_indices,
                biome_palette,
                biome_indices,
            })
        }

        PacketType::HandshakeInfo => {
            if data.len() < offset + HANDSHAKE_INFO_HEADER_SIZE {
                return Err(ProtocolError::PacketTooShort {
                    expected: offset + HANDSHAKE_INFO_HEADER_SIZE,
                    actual: data.len(),
                });
            }
            let total_sections = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
            let non_empty_sections = u32::from_le_bytes(data[offset + 4..offset + 8].try_into().unwrap());
            let total_volume = u32::from_le_bytes(data[offset + 8..offset + 12].try_into().unwrap());
            let dim_len = u16::from_le_bytes(data[offset + 12..offset + 14].try_into().unwrap()) as usize;
            offset += HANDSHAKE_INFO_HEADER_SIZE;

            if data.len() < offset + dim_len + 2 {
                return Err(ProtocolError::TruncatedPayload { field: "handshake_dimension_flags" });
            }
            let dimension = std::str::from_utf8(&data[offset..offset + dim_len])
                .map_err(|e| ProtocolError::Utf8Error {
                    field: "handshake_dimension",
                    detail: e.to_string(),
                })?
                .to_string();
            offset += dim_len;

            let flags = u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap());

            Ok(Packet::HandshakeInfo {
                total_sections,
                non_empty_sections,
                total_volume,
                dimension,
                flags,
            })
        }

        PacketType::StreamBegin => {
            if data.len() < offset + STREAM_BEGIN_SIZE {
                return Err(ProtocolError::PacketTooShort {
                    expected: offset + STREAM_BEGIN_SIZE,
                    actual: data.len(),
                });
            }
            let stream_id = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
            let total_sections = u32::from_le_bytes(data[offset + 4..offset + 8].try_into().unwrap());
            let flags = u16::from_le_bytes(data[offset + 8..offset + 10].try_into().unwrap());

            Ok(Packet::StreamBegin {
                stream_id,
                total_sections,
                flags,
            })
        }

        PacketType::StreamEnd => {
            if data.len() < offset + STREAM_END_SIZE {
                return Err(ProtocolError::PacketTooShort {
                    expected: offset + STREAM_END_SIZE,
                    actual: data.len(),
                });
            }
            let stream_id = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
            let sent_sections = u32::from_le_bytes(data[offset + 4..offset + 8].try_into().unwrap());
            let status_code = u16::from_le_bytes(data[offset + 8..offset + 10].try_into().unwrap());

            Ok(Packet::StreamEnd {
                stream_id,
                sent_sections,
                status: StreamStatus::from_u16(status_code),
            })
        }

        PacketType::ReqFullSync => Ok(Packet::ReqFullSync),

        PacketType::ReqSectionSync => {
            if data.len() < offset + 2 {
                return Err(ProtocolError::PacketTooShort {
                    expected: offset + 2,
                    actual: data.len(),
                });
            }
            let count = u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap()) as usize;
            offset += 2;
            if data.len() < offset + count * 12 {
                return Err(ProtocolError::TruncatedPayload { field: "req_section_sync_entries" });
            }
            let mut sections = Vec::with_capacity(count);
            for i in 0..count {
                let ent = offset + i * 12;
                let sx = i32::from_le_bytes(data[ent..ent + 4].try_into().unwrap());
                let sy = i32::from_le_bytes(data[ent + 4..ent + 8].try_into().unwrap());
                let sz = i32::from_le_bytes(data[ent + 8..ent + 12].try_into().unwrap());
                sections.push(IVec3::new(sx, sy, sz));
            }
            Ok(Packet::ReqSectionSync { sections })
        }

        PacketType::SyncConfig => {
            if data.len() < offset + 3 {
                return Err(ProtocolError::PacketTooShort {
                    expected: offset + 3,
                    actual: data.len(),
                });
            }
            let throttle_mode = data[offset];
            let target_fps = data[offset + 1];
            let is_active = data[offset + 2] != 0;

            Ok(Packet::SyncConfig {
                throttle_mode,
                target_fps,
                is_active,
            })
        }
    }
}

/// Encodes a Client-to-Server Full Sync Request packet (0x80).
pub fn encode_full_sync_request() -> Vec<u8> {
    let mut buf = Vec::with_capacity(HEADER_SIZE);
    buf.extend_from_slice(&PROTOCOL_MAGIC);
    buf.push(PROTOCOL_VERSION);
    buf.push(PacketType::ReqFullSync as u8);
    buf
}

/// Encodes Section Repair Requests (0x81) split into chunks of at most `max_batch_size` (default 64).
pub fn encode_repair_requests(sections: &[IVec3], max_batch_size: usize) -> Vec<Vec<u8>> {
    if sections.is_empty() {
        return Vec::new();
    }
    let chunk_size = if max_batch_size == 0 { 64 } else { max_batch_size };
    let mut packets = Vec::new();

    for chunk in sections.chunks(chunk_size) {
        let count = chunk.len() as u16;
        let mut buf = Vec::with_capacity(HEADER_SIZE + 2 + chunk.len() * 12);
        buf.extend_from_slice(&PROTOCOL_MAGIC);
        buf.push(PROTOCOL_VERSION);
        buf.push(PacketType::ReqSectionSync as u8);
        buf.extend_from_slice(&count.to_le_bytes());
        for pos in chunk {
            buf.extend_from_slice(&pos.x.to_le_bytes());
            buf.extend_from_slice(&pos.y.to_le_bytes());
            buf.extend_from_slice(&pos.z.to_le_bytes());
        }
        packets.push(buf);
    }

    packets
}

/// Encodes a Sync Configuration update packet (0x82).
pub fn encode_sync_config(throttle_mode: u8, target_fps: u8, is_active: bool) -> Vec<u8> {
    let mut buf = Vec::with_capacity(HEADER_SIZE + 3);
    buf.extend_from_slice(&PROTOCOL_MAGIC);
    buf.push(PROTOCOL_VERSION);
    buf.push(PacketType::SyncConfig as u8);
    buf.push(throttle_mode);
    buf.push(target_fps);
    buf.push(if is_active { 1 } else { 0 });
    buf
}

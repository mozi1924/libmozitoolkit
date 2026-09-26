//! # `mtk-py` Protocol Codec Bindings
//!
//! Exposes binary packet decoding and request encoding directly to Python.

use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList};

use glam::IVec3;
use mtk_sync::protocol::*;

/// Decodes a raw binary packet into a Python dict.
#[pyfunction]
pub fn decode_packet<'py>(py: Python<'py>, data: &[u8]) -> PyResult<Bound<'py, PyDict>> {
    let packet = mtk_sync::protocol::decode_packet(data)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

    let dict = PyDict::new(py);

    match packet {
        Packet::SelectionInfo { min_pos, size } => {
            dict.set_item("type", "SELECTION_INFO")?;
            dict.set_item("min_x", min_pos.x)?;
            dict.set_item("min_y", min_pos.y)?;
            dict.set_item("min_z", min_pos.z)?;
            dict.set_item("size_x", size.x)?;
            dict.set_item("size_y", size.y)?;
            dict.set_item("size_z", size.z)?;
        }
        Packet::FullSnapshot {
            min_pos,
            size,
            palette,
            grid_indices,
            biome_palette,
            biome_indices,
        } => {
            dict.set_item("type", "FULL_SNAPSHOT")?;
            dict.set_item("min_x", min_pos.x)?;
            dict.set_item("min_y", min_pos.y)?;
            dict.set_item("min_z", min_pos.z)?;
            dict.set_item("size_x", size.x)?;
            dict.set_item("size_y", size.y)?;
            dict.set_item("size_z", size.z)?;
            dict.set_item("palette", palette)?;
            dict.set_item("grid_indices", grid_indices)?;
            dict.set_item("biome_palette", biome_palette)?;
            dict.set_item("biome_indices", biome_indices)?;
        }
        Packet::DeltaUpdate {
            seq_id,
            min_pos,
            changes,
        } => {
            dict.set_item("type", "DELTA_UPDATE")?;
            dict.set_item("seq_id", seq_id)?;
            dict.set_item("min_x", min_pos.x)?;
            dict.set_item("min_y", min_pos.y)?;
            dict.set_item("min_z", min_pos.z)?;

            let change_list = PyList::empty(py);
            for c in changes {
                let tup = (c.rel_pos.x, c.rel_pos.y, c.rel_pos.z, c.state);
                let _ = change_list.append(tup);
            }
            dict.set_item("changes", change_list)?;
        }
        Packet::SectionManifest { seq_id, sections } => {
            dict.set_item("type", "SECTION_MANIFEST")?;
            dict.set_item("seq_id", seq_id)?;
            let sec_list = PyList::empty(py);
            for s in sections {
                let tup = (s.coord.x, s.coord.y, s.coord.z, s.crc32);
                let _ = sec_list.append(tup);
            }
            dict.set_item("sections", sec_list)?;
        }
        Packet::SectionSnapshot {
            sec_coord,
            start_pos,
            size,
            palette,
            grid_indices,
            biome_palette,
            biome_indices,
        } => {
            dict.set_item("type", "SECTION_SNAPSHOT")?;
            dict.set_item("sec_x", sec_coord.x)?;
            dict.set_item("sec_y", sec_coord.y)?;
            dict.set_item("sec_z", sec_coord.z)?;
            dict.set_item("start_x", start_pos.x)?;
            dict.set_item("start_y", start_pos.y)?;
            dict.set_item("start_z", start_pos.z)?;
            dict.set_item("size_x", size.x)?;
            dict.set_item("size_y", size.y)?;
            dict.set_item("size_z", size.z)?;
            dict.set_item("palette", palette)?;
            dict.set_item("grid_indices", grid_indices)?;
            dict.set_item("biome_palette", biome_palette)?;
            dict.set_item("biome_indices", biome_indices)?;
        }
        Packet::HandshakeInfo {
            total_sections,
            non_empty_sections,
            total_volume,
            dimension,
            flags,
        } => {
            dict.set_item("type", "HANDSHAKE_INFO")?;
            dict.set_item("total_sections", total_sections)?;
            dict.set_item("non_empty_sections", non_empty_sections)?;
            dict.set_item("total_volume", total_volume)?;
            dict.set_item("dimension", dimension)?;
            dict.set_item("flags", flags)?;
        }
        Packet::StreamBegin {
            stream_id,
            total_sections,
            flags,
        } => {
            dict.set_item("type", "STREAM_BEGIN")?;
            dict.set_item("stream_id", stream_id)?;
            dict.set_item("total_sections", total_sections)?;
            dict.set_item("flags", flags)?;
        }
        Packet::StreamEnd {
            stream_id,
            sent_sections,
            status,
        } => {
            dict.set_item("type", "STREAM_END")?;
            dict.set_item("stream_id", stream_id)?;
            dict.set_item("sent_sections", sent_sections)?;
            dict.set_item("status", status as u16)?;
        }
        Packet::ReqFullSync => {
            dict.set_item("type", "REQ_FULL_SYNC")?;
        }
        Packet::ReqSectionSync { sections } => {
            dict.set_item("type", "REQ_SECTION_SYNC")?;
            let list = PyList::empty(py);
            for s in sections {
                let _ = list.append((s.x, s.y, s.z));
            }
            dict.set_item("sections", list)?;
        }
        Packet::SyncConfig {
            throttle_mode,
            target_fps,
            is_active,
        } => {
            dict.set_item("type", "SYNC_CONFIG")?;
            dict.set_item("throttle_mode", throttle_mode)?;
            dict.set_item("target_fps", target_fps)?;
            dict.set_item("is_active", is_active)?;
        }
    }

    Ok(dict)
}

/// Encodes a Full Sync Request (0x80) packet as bytes.
#[pyfunction]
pub fn encode_full_sync_request<'py>(py: Python<'py>) -> Bound<'py, PyBytes> {
    let bytes = mtk_sync::protocol::encode_full_sync_request();
    PyBytes::new(py, &bytes)
}

/// Encodes Section Repair Requests (0x81) into a list of byte packets.
#[pyfunction]
#[pyo3(signature = (sections, max_batch_size=64))]
pub fn encode_repair_requests<'py>(
    py: Python<'py>,
    sections: Vec<(i32, i32, i32)>,
    max_batch_size: usize,
) -> Bound<'py, PyList> {
    let vec: Vec<IVec3> = sections
        .into_iter()
        .map(|(x, y, z)| IVec3::new(x, y, z))
        .collect();
    let packets = mtk_sync::protocol::encode_repair_requests(&vec, max_batch_size);

    let list = PyList::empty(py);
    for pkt in &packets {
        let _ = list.append(PyBytes::new(py, pkt));
    }
    list
}

/// Encodes a Sync Configuration (0x82) packet as bytes.
#[pyfunction]
#[pyo3(signature = (throttle_mode=0, target_fps=60, is_active=true))]
pub fn encode_sync_config<'py>(
    py: Python<'py>,
    throttle_mode: u8,
    target_fps: u8,
    is_active: bool,
) -> Bound<'py, PyBytes> {
    let bytes = mtk_sync::protocol::encode_sync_config(throttle_mode, target_fps, is_active);
    PyBytes::new(py, &bytes)
}

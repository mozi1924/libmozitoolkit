//! # `mtk-py` Live Sync Session and Networking Binding
//!
//! Exposes `LiveSyncSession` native WebSocket sync engine and event dispatcher to Python.

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use glam::IVec3;
use mtk_sync::{LiveSyncSession, SyncEvent};

use crate::cull::PyFaceCuller;
use crate::mesh::PyMeshData;
use crate::voxel::{PyMesherConfig, PyVoxelStorage};

/// Python wrapper for `LiveSyncSession` real-time WebSocket synchronization engine.
#[pyclass(name = "LiveSyncSession")]
pub struct PyLiveSyncSession {
    inner: LiveSyncSession,
}

#[pymethods]
impl PyLiveSyncSession {
    #[new]
    #[pyo3(signature = (config=None, culler=None))]
    pub fn new(config: Option<&PyMesherConfig>, culler: Option<&PyFaceCuller>) -> Self {
        let cfg = config.map(|c| c.inner.clone());
        let cul = culler.map(|c| c.inner.clone());
        Self {
            inner: LiveSyncSession::new(cfg, cul),
        }
    }

    /// Starts the background sync client thread connecting to Minecraft at `url`.
    #[pyo3(signature = (url="ws://localhost:8765", auto_reconnect=true, max_reconnect_attempts=5))]
    pub fn start(
        &mut self,
        url: &str,
        auto_reconnect: bool,
        max_reconnect_attempts: usize,
    ) -> PyResult<()> {
        self.inner
            .start(url, auto_reconnect, max_reconnect_attempts)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e))
    }

    /// Stops the live sync session cleanly and disconnects from server.
    pub fn stop(&mut self) {
        self.inner.stop();
    }

    /// Polls all pending synchronization events from the background Rust engine (non-blocking).
    pub fn poll_events<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        let events = self.inner.poll_events();
        let list = PyList::empty(py);

        for evt in events {
            let dict = PyDict::new(py);
            match evt {
                SyncEvent::StatusChange(status) => {
                    dict.set_item("type", "STATUS_CHANGE")?;
                    dict.set_item("status", status)?;
                }
                SyncEvent::SelectionUpdated { min_pos, size } => {
                    dict.set_item("type", "SELECTION_UPDATED")?;
                    dict.set_item("min_x", min_pos.x)?;
                    dict.set_item("min_y", min_pos.y)?;
                    dict.set_item("min_z", min_pos.z)?;
                    dict.set_item("size_x", size.x)?;
                    dict.set_item("size_y", size.y)?;
                    dict.set_item("size_z", size.z)?;
                }
                SyncEvent::Handshake {
                    total_sections,
                    non_empty_sections,
                    total_volume,
                    dimension,
                    flags,
                } => {
                    dict.set_item("type", "HANDSHAKE")?;
                    dict.set_item("total_sections", total_sections)?;
                    dict.set_item("non_empty_sections", non_empty_sections)?;
                    dict.set_item("total_volume", total_volume)?;
                    dict.set_item("dimension", dimension)?;
                    dict.set_item("flags", flags)?;
                }
                SyncEvent::SectionMeshReady { coord, mesh } => {
                    dict.set_item("type", "SECTION_MESH_READY")?;
                    dict.set_item("sec_x", coord.x)?;
                    dict.set_item("sec_y", coord.y)?;
                    dict.set_item("sec_z", coord.z)?;
                    dict.set_item("coord", (coord.x, coord.y, coord.z))?;
                    dict.set_item("mesh", PyMeshData { inner: mesh })?;
                }
                SyncEvent::StreamProgress {
                    current,
                    total,
                    message,
                } => {
                    dict.set_item("type", "STREAM_PROGRESS")?;
                    dict.set_item("current", current)?;
                    dict.set_item("total", total)?;
                    dict.set_item("message", message)?;
                }
                SyncEvent::StreamFinished {
                    stream_id,
                    built_sections,
                } => {
                    dict.set_item("type", "STREAM_FINISHED")?;
                    dict.set_item("stream_id", stream_id)?;
                    dict.set_item("built_sections", built_sections)?;
                }
                SyncEvent::DeltaApplied {
                    change_count,
                    affected_sections,
                } => {
                    dict.set_item("type", "DELTA_APPLIED")?;
                    dict.set_item("change_count", change_count)?;
                    let aff_list = PyList::empty(py);
                    for c in affected_sections {
                        let _ = aff_list.append((c.x, c.y, c.z));
                    }
                    dict.set_item("affected_sections", aff_list)?;
                }
                SyncEvent::Verified {
                    is_verified,
                    message,
                } => {
                    dict.set_item("type", "VERIFIED")?;
                    dict.set_item("is_verified", is_verified)?;
                    dict.set_item("message", message)?;
                }
                SyncEvent::Warning(msg) => {
                    dict.set_item("type", "WARNING")?;
                    dict.set_item("message", msg)?;
                }
                SyncEvent::Error(msg) => {
                    dict.set_item("type", "ERROR")?;
                    dict.set_item("message", msg)?;
                }
            }
            list.append(dict)?;
        }

        Ok(list)
    }

    /// Sends a Full Sync Request (0x80) to server.
    pub fn send_full_sync_request(&self) -> PyResult<()> {
        self.inner
            .send_full_sync_request()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e))
    }

    /// Sends Section Repair Requests (0x81) to server.
    pub fn send_repair_request(&self, sections: Vec<(i32, i32, i32)>) -> PyResult<()> {
        let vec: Vec<IVec3> = sections
            .into_iter()
            .map(|(x, y, z)| IVec3::new(x, y, z))
            .collect();
        self.inner
            .send_repair_request(&vec)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e))
    }

    /// Sends Sync Configuration (0x82) to server.
    #[pyo3(signature = (throttle_mode=0, target_fps=60, is_active=true))]
    pub fn send_sync_config(
        &self,
        throttle_mode: u8,
        target_fps: u8,
        is_active: bool,
    ) -> PyResult<()> {
        self.inner
            .send_sync_config(throttle_mode, target_fps, is_active)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e))
    }

    /// Returns a copy of the underlying `VoxelStorage`.
    pub fn get_storage(&self) -> PyVoxelStorage {
        let st = self.inner.storage.read().unwrap();
        PyVoxelStorage { inner: st.clone() }
    }
}

//! # Live Sync Session Controller
//!
//! Orchestrates the background network client, VoxelStorage, SectionMesher, and event routing.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, RwLock};
use std::thread::{self, JoinHandle};

use crossbeam_channel::{Receiver, Sender};
use glam::IVec3;
use mtk_cull::FaceCuller;

use mtk_voxel::mesher::{DeltaMesher, SectionMesher};
use mtk_voxel::storage::VoxelStorage;
use mtk_voxel::types::MesherConfig;

use crate::client::{ClientMessage, SyncClient};
use crate::events::SyncEvent;
use crate::protocol::*;

/// High-level Live Sync Session managing connection, storage, multi-threaded meshing, and event queues.
pub struct LiveSyncSession {
    /// In-memory 3D Voxel storage shared with background mesher threads.
    pub storage: Arc<RwLock<VoxelStorage>>,
    /// Active Mesher configuration.
    pub config: MesherConfig,
    /// Face Culling rules.
    pub culler: FaceCuller,

    client: Option<SyncClient>,
    event_sender: Sender<SyncEvent>,
    event_receiver: Receiver<SyncEvent>,

    worker_running: Arc<AtomicBool>,
    worker_handle: Option<JoinHandle<()>>,

    current_stream_id: Arc<AtomicU32>,
}

impl LiveSyncSession {
    /// Creates a new `LiveSyncSession`.
    pub fn new(config: Option<MesherConfig>, culler: Option<FaceCuller>) -> Self {
        let (event_sender, event_receiver) = crossbeam_channel::unbounded::<SyncEvent>();
        Self {
            storage: Arc::new(RwLock::new(VoxelStorage::new())),
            config: config.unwrap_or_default(),
            culler: culler.unwrap_or_default(),
            client: None,
            event_sender,
            event_receiver,
            worker_running: Arc::new(AtomicBool::new(false)),
            worker_handle: None,
            current_stream_id: Arc::new(AtomicU32::new(0)),
        }
    }

    /// Starts the live sync session connecting to the given WebSocket `url`.
    pub fn start(&mut self, url: &str, auto_reconnect: bool, max_reconnect_attempts: usize) -> Result<(), String> {
        self.stop();

        let (msg_sender, msg_receiver) = crossbeam_channel::unbounded::<ClientMessage>();
        let client = SyncClient::connect(
            url.to_string(),
            msg_sender,
            auto_reconnect,
            max_reconnect_attempts,
        )?;

        let storage_clone = self.storage.clone();
        let event_sender_clone = self.event_sender.clone();
        let config_clone = self.config.clone();
        let culler_clone = self.culler.clone();
        let stream_id_clone = self.current_stream_id.clone();
        let worker_running = Arc::new(AtomicBool::new(true));
        self.worker_running = worker_running.clone();

        let handle = thread::spawn(move || {
            Self::event_worker_loop(
                msg_receiver,
                storage_clone,
                event_sender_clone,
                config_clone,
                culler_clone,
                stream_id_clone,
                worker_running,
            );
        });

        self.client = Some(client);
        self.worker_handle = Some(handle);

        // Send initial sync config to Minecraft
        let _ = self.send_sync_config(0, 60, true);
        Ok(())
    }

    /// Stops the live sync session cleanly.
    pub fn stop(&mut self) {
        self.worker_running.store(false, Ordering::SeqCst);
        if let Some(mut client) = self.client.take() {
            client.stop();
        }
        if let Some(handle) = self.worker_handle.take() {
            let _ = handle.join();
        }
    }

    /// Polls all ready sync events from the background queue (non-blocking).
    pub fn poll_events(&self) -> Vec<SyncEvent> {
        let mut events = Vec::new();
        while let Ok(evt) = self.event_receiver.try_recv() {
            events.push(evt);
        }
        events
    }

    /// Sends a Full Sync Request (0x80) to Minecraft server.
    pub fn send_full_sync_request(&self) -> Result<(), String> {
        if let Some(ref client) = self.client {
            client.send_packet(encode_full_sync_request())
        } else {
            Err("LiveSyncSession is not connected".to_string())
        }
    }

    /// Sends Section Repair Requests (0x81) to Minecraft server.
    pub fn send_repair_request(&self, sections: &[IVec3]) -> Result<(), String> {
        if let Some(ref client) = self.client {
            for packet in encode_repair_requests(sections, 64) {
                client.send_packet(packet)?;
            }
            Ok(())
        } else {
            Err("LiveSyncSession is not connected".to_string())
        }
    }

    /// Sends Sync Configuration (0x82) to Minecraft server.
    pub fn send_sync_config(&self, throttle_mode: u8, target_fps: u8, is_active: bool) -> Result<(), String> {
        if let Some(ref client) = self.client {
            client.send_packet(encode_sync_config(throttle_mode, target_fps, is_active))
        } else {
            Err("LiveSyncSession is not connected".to_string())
        }
    }

    fn event_worker_loop(
        msg_receiver: Receiver<ClientMessage>,
        storage: Arc<RwLock<VoxelStorage>>,
        event_sender: Sender<SyncEvent>,
        config: MesherConfig,
        culler: FaceCuller,
        stream_id_atomic: Arc<AtomicU32>,
        running: Arc<AtomicBool>,
    ) {
        while running.load(Ordering::Relaxed) {
            match msg_receiver.recv_timeout(std::time::Duration::from_millis(100)) {
                Ok(ClientMessage::Status(status)) => {
                    let _ = event_sender.send(SyncEvent::StatusChange(status));
                }
                Ok(ClientMessage::Disconnected) => {
                    let _ = event_sender.send(SyncEvent::StatusChange("DISCONNECTED".to_string()));
                    break;
                }
                Ok(ClientMessage::PacketReceived(packet)) => {
                    Self::handle_packet(
                        packet,
                        &storage,
                        &event_sender,
                        &config,
                        &culler,
                        &stream_id_atomic,
                    );
                }
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => continue,
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
            }
        }
    }

    fn handle_packet(
        packet: Packet,
        storage: &Arc<RwLock<VoxelStorage>>,
        event_sender: &Sender<SyncEvent>,
        config: &MesherConfig,
        culler: &FaceCuller,
        stream_id_atomic: &Arc<AtomicU32>,
    ) {
        match packet {
            Packet::SelectionInfo { min_pos, size } => {
                {
                    let mut st = storage.write().unwrap();
                    st.set_bounds(min_pos.x, min_pos.y, min_pos.z, size.x, size.y, size.z);
                }
                // Preemptive stream invalidation
                stream_id_atomic.fetch_add(1, Ordering::SeqCst);
                let _ = event_sender.send(SyncEvent::SelectionUpdated { min_pos, size });
            }

            Packet::HandshakeInfo {
                total_sections,
                non_empty_sections,
                total_volume,
                dimension,
                flags,
            } => {
                let _ = event_sender.send(SyncEvent::Handshake {
                    total_sections,
                    non_empty_sections,
                    total_volume,
                    dimension,
                    flags,
                });
            }

            Packet::FullSnapshot {
                min_pos,
                size,
                palette,
                grid_indices,
                biome_palette,
                biome_indices,
            } => {
                // Check if identical to skip heavy rebuild
                let identical = {
                    let st = storage.read().unwrap();
                    st.is_snapshot_identical(
                        min_pos.x,
                        min_pos.y,
                        min_pos.z,
                        size.x,
                        size.y,
                        size.z,
                        &palette,
                        &grid_indices,
                    )
                };

                if identical {
                    let _ = event_sender.send(SyncEvent::Verified {
                        is_verified: true,
                        message: "Data 100% identical, skipping rebuild".to_string(),
                    });
                    return;
                }

                // Ingest full snapshot
                let non_empty_sections = {
                    let mut st = storage.write().unwrap();
                    st.set_full_snapshot(
                        min_pos.x,
                        min_pos.y,
                        min_pos.z,
                        size.x,
                        size.y,
                        size.z,
                        &palette,
                        &grid_indices,
                        biome_palette.as_deref(),
                        biome_indices.as_deref(),
                    );
                    st.get_all_non_empty_sections()
                };

                let total = non_empty_sections.len();
                let _ = event_sender.send(SyncEvent::StreamProgress {
                    current: 0,
                    total,
                    message: format!("Meshing {} sections...", total),
                });

                // Build meshes in parallel using Rayon
                let padded_sections: Vec<_> = {
                    let st = storage.read().unwrap();
                    non_empty_sections
                        .iter()
                        .map(|&coord| st.get_section_padded_array(coord))
                        .collect()
                };

                if let Ok(results) = SectionMesher::mesh_sections_parallel(
                    &padded_sections,
                    culler,
                    |_| None,
                    config,
                    None,
                ) {
                    for (i, (coord, mesh)) in results.into_iter().enumerate() {
                        let _ = event_sender.send(SyncEvent::SectionMeshReady { coord, mesh });
                        let _ = event_sender.send(SyncEvent::StreamProgress {
                            current: i + 1,
                            total,
                            message: format!("Meshed chunk ({}/{})", i + 1, total),
                        });
                    }
                }

                let _ = event_sender.send(SyncEvent::StreamFinished {
                    stream_id: 0,
                    built_sections: total,
                });
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
                let padded = {
                    let mut st = storage.write().unwrap();
                    if st.size_x > 0 && !st.contains(start_pos.x, start_pos.y, start_pos.z) {
                        return;
                    }
                    st.set_section_snapshot(
                        sec_coord.x,
                        sec_coord.y,
                        sec_coord.z,
                        start_pos.x,
                        start_pos.y,
                        start_pos.z,
                        size.x,
                        size.y,
                        size.z,
                        &palette,
                        &grid_indices,
                        biome_palette.as_deref(),
                        biome_indices.as_deref(),
                    );
                    st.get_section_padded_array(sec_coord)
                };

                // Immediately mesh and dispatch this section
                let mesh = SectionMesher::mesh_section(&padded, culler, |_| None, config);
                let _ = event_sender.send(SyncEvent::SectionMeshReady {
                    coord: sec_coord,
                    mesh,
                });
            }

            Packet::DeltaUpdate {
                min_pos,
                changes,
                ..
            } => {
                let borrowed_changes: Vec<(i32, i32, i32, &str)> = changes
                    .iter()
                    .map(|c| (c.rel_pos.x + min_pos.x, c.rel_pos.y + min_pos.y, c.rel_pos.z + min_pos.z, c.state.as_str()))
                    .collect();

                let rebuilt_meshes = {
                    let mut st = storage.write().unwrap();
                    st.apply_delta_update(min_pos.x, min_pos.y, min_pos.z, &borrowed_changes);
                    DeltaMesher::rebuild_dirty_sections(&mut st, culler, |_| None, config)
                };

                let affected: Vec<IVec3> = rebuilt_meshes.iter().map(|(c, _)| *c).collect();
                for (coord, mesh) in rebuilt_meshes {
                    let _ = event_sender.send(SyncEvent::SectionMeshReady { coord, mesh });
                }

                let _ = event_sender.send(SyncEvent::DeltaApplied {
                    change_count: changes.len(),
                    affected_sections: affected,
                });
            }

            Packet::SectionManifest { seq_id: _, sections } => {
                let raw_entries: Vec<(i32, i32, i32, u32)> = sections
                    .iter()
                    .map(|s| (s.coord.x, s.coord.y, s.coord.z, s.crc32))
                    .collect();

                let mismatched = {
                    let mut st = storage.write().unwrap();
                    st.validate_manifest(&raw_entries, None)
                };

                if mismatched.is_empty() {
                    let _ = event_sender.send(SyncEvent::Verified {
                        is_verified: true,
                        message: "100% in sync with scene".to_string(),
                    });
                } else {
                    let _ = event_sender.send(SyncEvent::Verified {
                        is_verified: false,
                        message: format!("Detected {} out-of-sync sections", mismatched.len()),
                    });
                }
            }

            Packet::StreamBegin {
                stream_id,
                total_sections,
                flags: _,
            } => {
                stream_id_atomic.store(stream_id, Ordering::SeqCst);
                let _ = event_sender.send(SyncEvent::StreamProgress {
                    current: 0,
                    total: total_sections as usize,
                    message: format!("Receiving {} chunks...", total_sections),
                });
            }

            Packet::StreamEnd {
                stream_id,
                sent_sections,
                status: _,
            } => {
                let _ = event_sender.send(SyncEvent::StreamFinished {
                    stream_id,
                    built_sections: sent_sections as usize,
                });
            }

            _ => {}
        }
    }
}

impl Drop for LiveSyncSession {
    fn drop(&mut self) {
        self.stop();
    }
}

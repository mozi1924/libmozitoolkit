use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, RwLock};
use std::thread::{self, JoinHandle};

use crossbeam_channel::{Receiver, Sender};
use glam::IVec3;
use mtk_core::mesh::MeshData;
use mtk_cull::FaceCuller;
use mtk_model::baked::{BakedModel, BakedModelDatabase};

use mtk_voxel::mesher::{DeltaMesher, SectionMesher};
use mtk_voxel::storage::VoxelStorage;
use mtk_voxel::types::MesherConfig;

use crate::client::{ClientCommand, ClientMessage, SyncClient};
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
    /// Prebaked Minecraft blockstate model database for custom JSON models.
    pub model_db: Option<Arc<BakedModelDatabase>>,
    /// Whether to merge all chunk sections into a single, seamless world mesh.
    pub unified_mesh: bool,

    client: Option<SyncClient>,
    event_sender: Sender<SyncEvent>,
    event_receiver: Receiver<SyncEvent>,

    worker_running: Arc<AtomicBool>,
    worker_handle: Option<JoinHandle<()>>,

    current_stream_id: Arc<AtomicU32>,
    sync_requested: Arc<AtomicBool>,
}

impl LiveSyncSession {
    /// Creates a new `LiveSyncSession`.
    pub fn new(
        config: Option<MesherConfig>,
        culler: Option<FaceCuller>,
        model_db: Option<Arc<BakedModelDatabase>>,
        unified_mesh: bool,
    ) -> Self {
        let (event_sender, event_receiver) = crossbeam_channel::unbounded::<SyncEvent>();
        Self {
            storage: Arc::new(RwLock::new(VoxelStorage::new())),
            config: config.unwrap_or_default(),
            culler: culler.unwrap_or_default(),
            model_db,
            unified_mesh,
            client: None,
            event_sender,
            event_receiver,
            worker_running: Arc::new(AtomicBool::new(false)),
            worker_handle: None,
            current_stream_id: Arc::new(AtomicU32::new(0)),
            sync_requested: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Default for LiveSyncSession {
    fn default() -> Self {
        Self::new(None, None, None, true)
    }
}

impl LiveSyncSession {

    /// Sets or replaces the baked model database.
    pub fn set_model_db(&mut self, model_db: Option<Arc<BakedModelDatabase>>) {
        self.model_db = model_db;
    }

    /// Toggles single unified world mesh mode.
    pub fn set_unified_mesh(&mut self, unified_mesh: bool) {
        self.unified_mesh = unified_mesh;
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
        let model_db_clone = self.model_db.clone();
        let unified_mesh = self.unified_mesh;
        let stream_id_clone = self.current_stream_id.clone();
        let sync_requested_clone = self.sync_requested.clone();
        self.sync_requested.store(false, Ordering::SeqCst);
        let worker_running = Arc::new(AtomicBool::new(true));
        self.worker_running = worker_running.clone();

        let cmd_sender = client.get_cmd_sender();
        let handle = thread::spawn(move || {
            Self::event_worker_loop(
                msg_receiver,
                cmd_sender,
                storage_clone,
                event_sender_clone,
                config_clone,
                culler_clone,
                model_db_clone,
                unified_mesh,
                stream_id_clone,
                sync_requested_clone,
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

    /// Meshes the entire active VoxelStorage volume and returns a unified `MeshData`.
    pub fn get_world_mesh(&self) -> MeshData {
        let st = self.storage.read().unwrap();
        let non_empty = st.get_all_non_empty_sections();
        if non_empty.is_empty() {
            return MeshData::new();
        }

        let mut config = self.config.clone();
        if config.origin_centered {
            let (min_x, min_y, min_z, sz_x, sz_y, sz_z) = st.get_bounds();
            if sz_x > 0 && sz_y > 0 && sz_z > 0 {
                config.selection_bounds = Some(([min_x, min_y, min_z], [sz_x, sz_y, sz_z]));
            }
        }

        let padded: Vec<_> = non_empty.iter().map(|&c| st.get_section_padded_array(c)).collect();
        let db_opt = self.model_db.clone();
        let model_lookup = move |state: &str| -> Option<Arc<BakedModel>> {
            db_opt.as_ref().and_then(|db| db.get(state).cloned().map(Arc::new))
        };

        if let Ok(results) = SectionMesher::mesh_sections_parallel(
            &padded,
            &self.culler,
            model_lookup,
            &config,
            None,
        ) {
            let section_meshes: Vec<_> = results.into_iter().map(|(_, m)| m).collect();
            let mut merged = MeshData::merge_all(&section_meshes);
            if config.weld_vertices {
                merged.weld_spatial_vertices(1e-4);
            }
            merged
        } else {
            MeshData::new()
        }
    }

    /// Sends a Full Sync Request (0x80) to Minecraft server.
    pub fn send_full_sync_request(&self) -> Result<(), String> {
        self.sync_requested.store(true, Ordering::SeqCst);
        if let Some(ref client) = self.client {
            client.send_packet(encode_full_sync_request())
        } else {
            Err("LiveSyncSession is not connected".to_string())
        }
    }

    /// Sends Section Repair Requests (0x81) to Minecraft server.
    pub fn send_repair_request(&self, sections: &[IVec3]) -> Result<(), String> {
        self.sync_requested.store(true, Ordering::SeqCst);
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

    /// Dispatches a raw packet into the session pipeline synchronously (useful for testing or direct ingestion).
    pub fn process_packet_direct(
        &self,
        packet: Packet,
        section_mesh_cache: &mut HashMap<IVec3, MeshData>,
        stream_total_sections: &mut usize,
        stream_received_sections: &mut usize,
    ) {
        let mut config = self.config.clone();
        Self::handle_packet(
            packet,
            &self.storage,
            &self.event_sender,
            &mut config,
            &self.culler,
            &self.model_db,
            self.unified_mesh,
            section_mesh_cache,
            &self.current_stream_id,
            stream_total_sections,
            stream_received_sections,
            None,
            &self.sync_requested,
        );
    }

    fn event_worker_loop(
        msg_receiver: Receiver<ClientMessage>,
        cmd_sender: Sender<ClientCommand>,
        storage: Arc<RwLock<VoxelStorage>>,
        event_sender: Sender<SyncEvent>,
        mut config: MesherConfig,
        culler: FaceCuller,
        model_db: Option<Arc<BakedModelDatabase>>,
        unified_mesh: bool,
        stream_id_atomic: Arc<AtomicU32>,
        sync_requested: Arc<AtomicBool>,
        running: Arc<AtomicBool>,
    ) {
        let mut section_mesh_cache: HashMap<IVec3, MeshData> = HashMap::new();
        let mut stream_total_sections: usize = 0;
        let mut stream_received_sections: usize = 0;

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
                        &mut config,
                        &culler,
                        &model_db,
                        unified_mesh,
                        &mut section_mesh_cache,
                        &stream_id_atomic,
                        &mut stream_total_sections,
                        &mut stream_received_sections,
                        Some(&cmd_sender),
                        &sync_requested,
                    );
                }
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => continue,
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
            }
        }
    }

    pub fn handle_packet(
        packet: Packet,
        storage: &Arc<RwLock<VoxelStorage>>,
        event_sender: &Sender<SyncEvent>,
        config: &mut MesherConfig,
        culler: &FaceCuller,
        model_db: &Option<Arc<BakedModelDatabase>>,
        unified_mesh: bool,
        section_mesh_cache: &mut HashMap<IVec3, MeshData>,
        stream_id_atomic: &Arc<AtomicU32>,
        stream_total_sections: &mut usize,
        stream_received_sections: &mut usize,
        cmd_sender: Option<&Sender<ClientCommand>>,
        sync_requested: &Arc<AtomicBool>,
    ) {
        let model_lookup = |state: &str| -> Option<Arc<BakedModel>> {
            model_db.as_ref().and_then(|db| db.get(state).cloned().map(Arc::new))
        };

        match packet {
            Packet::SelectionInfo { min_pos, size } => {
                let bounds_changed = {
                    let mut st = storage.write().unwrap();
                    st.set_bounds(min_pos.x, min_pos.y, min_pos.z, size.x, size.y, size.z)
                };
                let new_bounds = Some(([min_pos.x, min_pos.y, min_pos.z], [size.x, size.y, size.z]));
                let bounds_differ = config.selection_bounds != new_bounds;
                if config.origin_centered {
                    config.selection_bounds = new_bounds;
                }

                if bounds_changed || bounds_differ {
                    sync_requested.store(false, Ordering::SeqCst);
                    section_mesh_cache.clear();
                    // If storage already has sections, rebuild all of them so world mesh reflects new origin/bounds
                    let non_empty = {
                        let st = storage.read().unwrap();
                        st.get_all_non_empty_sections()
                    };
                    if !non_empty.is_empty() {
                        let padded_sections: Vec<_> = {
                            let st = storage.read().unwrap();
                            non_empty
                                .iter()
                                .map(|&coord| st.get_section_padded_array(coord))
                                .collect()
                        };
                        if let Ok(results) = SectionMesher::mesh_sections_parallel(
                            &padded_sections,
                            culler,
                            &model_lookup,
                            config,
                            None,
                        ) {
                            for (coord, mesh) in results {
                                if !mesh.is_empty() {
                                    section_mesh_cache.insert(coord, mesh);
                                }
                            }
                            if unified_mesh {
                                let meshes: Vec<_> = section_mesh_cache.values().cloned().collect();
                                let mut world_mesh = MeshData::merge_all(&meshes);
                                if config.weld_vertices {
                                    world_mesh.weld_spatial_vertices(1e-4);
                                }
                                let _ = event_sender.send(SyncEvent::WorldMeshReady { mesh: world_mesh });
                            }
                        }
                    }
                }

                // Invalidate any previous stream state on selection change
                stream_id_atomic.store(0, Ordering::SeqCst);
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

                section_mesh_cache.clear();

                if config.origin_centered {
                    config.selection_bounds = Some(([min_pos.x, min_pos.y, min_pos.z], [size.x, size.y, size.z]));
                }

                if let Ok(results) = SectionMesher::mesh_sections_parallel(
                    &padded_sections,
                    culler,
                    &model_lookup,
                    config,
                    None,
                ) {
                    for (coord, mesh) in results {
                        if !mesh.is_empty() {
                            section_mesh_cache.insert(coord, mesh);
                        }
                    }

                    if unified_mesh {
                        let mut world_mesh = MeshData::new();
                        for mesh in section_mesh_cache.values() {
                            world_mesh.append_mesh(mesh);
                        }
                        if config.weld_vertices {
                            world_mesh.weld_spatial_vertices(1e-4);
                        }
                        let _ = event_sender.send(SyncEvent::WorldMeshReady { mesh: world_mesh });
                    } else {
                        for (i, (coord, mesh)) in section_mesh_cache.iter().enumerate() {
                            let _ = event_sender.send(SyncEvent::SectionMeshReady { coord: *coord, mesh: mesh.clone() });
                            let _ = event_sender.send(SyncEvent::StreamProgress {
                                current: i + 1,
                                total,
                                message: format!("Meshed chunk ({}/{})", i + 1, total),
                            });
                        }
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
                // Ingest into VoxelStorage
                {
                    let mut st = storage.write().unwrap();
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
                }

                let is_streaming = stream_id_atomic.load(Ordering::SeqCst) != 0;
                if is_streaming {
                    *stream_received_sections += 1;
                    let _ = event_sender.send(SyncEvent::StreamProgress {
                        current: *stream_received_sections,
                        total: *stream_total_sections,
                        message: format!("Receiving chunk ({}/{})", *stream_received_sections, *stream_total_sections),
                    });
                    // Two-phase streaming: wait for StreamEnd so all neighboring sections are present in memory.
                } else {
                    // Out-of-stream single-section update or repair
                    let padded = {
                        let st = storage.read().unwrap();
                        if config.origin_centered && config.selection_bounds.is_none() {
                            let (min_x, min_y, min_z, sz_x, sz_y, sz_z) = st.get_bounds();
                            if sz_x > 0 && sz_y > 0 && sz_z > 0 {
                                config.selection_bounds = Some(([min_x, min_y, min_z], [sz_x, sz_y, sz_z]));
                            }
                        }
                        st.get_section_padded_array(sec_coord)
                    };

                    let mesh = SectionMesher::mesh_section(&padded, culler, &model_lookup, config);
                    if mesh.is_empty() {
                        section_mesh_cache.remove(&sec_coord);
                    } else {
                        section_mesh_cache.insert(sec_coord, mesh.clone());
                    }

                    if unified_mesh {
                        let mut world_mesh = MeshData::new();
                        for m in section_mesh_cache.values() {
                            world_mesh.append_mesh(m);
                        }
                        if config.weld_vertices {
                            world_mesh.weld_spatial_vertices(1e-4);
                        }
                        let _ = event_sender.send(SyncEvent::WorldMeshReady { mesh: world_mesh });
                    } else {
                        let _ = event_sender.send(SyncEvent::SectionMeshReady {
                            coord: sec_coord,
                            mesh,
                        });
                    }
                }
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
                    if config.origin_centered && config.selection_bounds.is_none() {
                        let (min_x, min_y, min_z, sz_x, sz_y, sz_z) = st.get_bounds();
                        if sz_x > 0 && sz_y > 0 && sz_z > 0 {
                            config.selection_bounds = Some(([min_x, min_y, min_z], [sz_x, sz_y, sz_z]));
                        }
                    }
                    DeltaMesher::rebuild_dirty_sections(&mut st, culler, &model_lookup, config)
                };

                let affected: Vec<IVec3> = rebuilt_meshes.iter().map(|(c, _)| *c).collect();
                for (coord, mesh) in rebuilt_meshes {
                    if mesh.is_empty() {
                        section_mesh_cache.remove(&coord);
                    } else {
                        section_mesh_cache.insert(coord, mesh.clone());
                    }
                    if !unified_mesh {
                        let _ = event_sender.send(SyncEvent::SectionMeshReady { coord, mesh });
                    }
                }

                if unified_mesh {
                    let mut world_mesh = MeshData::new();
                    for m in section_mesh_cache.values() {
                        world_mesh.append_mesh(m);
                    }
                    if config.weld_vertices {
                        world_mesh.weld_spatial_vertices(1e-4);
                    }
                    let _ = event_sender.send(SyncEvent::WorldMeshReady { mesh: world_mesh });
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

                let (mismatched, is_storage_empty) = {
                    let mut st = storage.write().unwrap();
                    let mismatches = st.validate_manifest(&raw_entries, None);
                    let empty = st.get_all_non_empty_sections().is_empty();
                    (mismatches, empty)
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

                    // Automatically request synchronization according to live sync protocol contract
                    let is_streaming = stream_id_atomic.load(Ordering::SeqCst) != 0;
                    if let Some(sender) = cmd_sender {
                        if !is_streaming && !sync_requested.load(Ordering::SeqCst) {
                            if is_storage_empty || mismatched.len() > 64 || mismatched.len() == raw_entries.len() {
                                let _ = sender.send(ClientCommand::Send(encode_full_sync_request()));
                                sync_requested.store(true, Ordering::SeqCst);
                            } else {
                                for packet in encode_repair_requests(&mismatched, 64) {
                                    let _ = sender.send(ClientCommand::Send(packet));
                                }
                                sync_requested.store(true, Ordering::SeqCst);
                            }
                        }
                    }
                }
            }

            Packet::StreamBegin {
                stream_id,
                total_sections,
                flags: _,
            } => {
                stream_id_atomic.store(stream_id, Ordering::SeqCst);
                sync_requested.store(false, Ordering::SeqCst);
                *stream_total_sections = total_sections as usize;
                *stream_received_sections = 0;
                let _ = event_sender.send(SyncEvent::StreamProgress {
                    current: 0,
                    total: *stream_total_sections,
                    message: format!("Receiving {} chunks...", *stream_total_sections),
                });
            }

            Packet::StreamEnd {
                stream_id,
                sent_sections: _,
                status: _,
            } => {
                stream_id_atomic.store(0, Ordering::SeqCst);
                sync_requested.store(false, Ordering::SeqCst);

                // Phase 2 (BUILD): Build all sections with complete neighbor data in memory
                let (bounds, padded_sections) = {
                    let st = storage.read().unwrap();
                    let b = st.get_bounds();
                    let non_empty = st.get_all_non_empty_sections();
                    let padded: Vec<_> = non_empty
                        .iter()
                        .map(|&c| st.get_section_padded_array(c))
                        .collect();
                    (b, padded)
                };

                if config.origin_centered {
                    let (min_x, min_y, min_z, sz_x, sz_y, sz_z) = bounds;
                    if sz_x > 0 && sz_y > 0 && sz_z > 0 {
                        config.selection_bounds = Some(([min_x, min_y, min_z], [sz_x, sz_y, sz_z]));
                    }
                }

                section_mesh_cache.clear();
                let total = padded_sections.len();

                if total > 0 {
                    if let Ok(results) = SectionMesher::mesh_sections_parallel(
                        &padded_sections,
                        culler,
                        &model_lookup,
                        config,
                        None,
                    ) {
                        for (coord, mesh) in results {
                            if !mesh.is_empty() {
                                section_mesh_cache.insert(coord, mesh);
                            }
                        }
                    }
                }

                if unified_mesh {
                    let mut world_mesh = MeshData::new();
                    for m in section_mesh_cache.values() {
                        world_mesh.append_mesh(m);
                    }
                    if config.weld_vertices {
                        world_mesh.weld_spatial_vertices(1e-4);
                    }
                    let _ = event_sender.send(SyncEvent::WorldMeshReady { mesh: world_mesh });
                } else {
                    for (i, (coord, mesh)) in section_mesh_cache.iter().enumerate() {
                        let _ = event_sender.send(SyncEvent::SectionMeshReady { coord: *coord, mesh: mesh.clone() });
                        let _ = event_sender.send(SyncEvent::StreamProgress {
                            current: i + 1,
                            total,
                            message: format!("Meshed chunk ({}/{})", i + 1, total),
                        });
                    }
                }

                let _ = event_sender.send(SyncEvent::StreamFinished {
                    stream_id,
                    built_sections: total,
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

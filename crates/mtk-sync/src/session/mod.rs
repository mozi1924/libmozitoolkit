use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, RwLock};
use std::thread::{self, JoinHandle};

use crossbeam_channel::{Receiver, Sender};
use glam::IVec3;
use mtk_core::mesh::MeshData;
use mtk_cull::FaceCuller;
use mtk_model::baked::{BakedModel, BakedModelDatabase};
use mtk_voxel::mesher::SectionMesher;
use mtk_voxel::storage::VoxelStorage;
use mtk_voxel::types::MesherConfig;

use crate::client::{ClientMessage, SyncClient};
use crate::events::SyncEvent;
use crate::protocol::*;

pub mod dispatcher;

pub use dispatcher::*;

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
            event_worker_loop(
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
        dispatcher::handle_packet(
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

    /// Associated function for packet handling (compatibility).
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
        cmd_sender: Option<&Sender<crate::client::ClientCommand>>,
        sync_requested: &Arc<AtomicBool>,
    ) {
        dispatcher::handle_packet(
            packet,
            storage,
            event_sender,
            config,
            culler,
            model_db,
            unified_mesh,
            section_mesh_cache,
            stream_id_atomic,
            stream_total_sections,
            stream_received_sections,
            cmd_sender,
            sync_requested,
        );
    }
}

impl Default for LiveSyncSession {
    fn default() -> Self {
        Self::new(None, None, None, true)
    }
}

impl Drop for LiveSyncSession {
    fn drop(&mut self) {
        self.stop();
    }
}

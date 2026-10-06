use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, RwLock};
use std::thread::{self, JoinHandle};

use crossbeam_channel::{Receiver, Sender};
use glam::IVec3;
use mtk_core::mesh::MeshData;
use mtk_cull::FaceCuller;
use mtk_model::baked::BakedModelDatabase;
use mtk_voxel::types::MesherConfig;
use mtk_voxel::VoxelWorld;

use crate::client::{ClientMessage, SyncClient};
use crate::events::SyncEvent;
use crate::protocol::*;

pub mod dispatcher;

pub use dispatcher::*;

/// High-level Live Sync Session managing WebSocket streaming, VoxelWorld scene synchronization, and event queues.
pub struct LiveSyncSession {
    /// In-memory 3D Voxel World scene engine managing storage, meshing, and cache.
    pub world: Arc<RwLock<VoxelWorld>>,

    client: Option<SyncClient>,
    event_sender: Sender<SyncEvent>,
    event_receiver: Receiver<SyncEvent>,

    worker_running: Arc<AtomicBool>,
    is_connected: Arc<AtomicBool>,
    status: Arc<std::sync::RwLock<String>>,
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
        let world = VoxelWorld::new(config, culler, model_db, unified_mesh);

        Self {
            world: Arc::new(RwLock::new(world)),
            client: None,
            event_sender,
            event_receiver,
            worker_running: Arc::new(AtomicBool::new(false)),
            is_connected: Arc::new(AtomicBool::new(false)),
            status: Arc::new(std::sync::RwLock::new("DISCONNECTED".to_string())),
            worker_handle: None,
            current_stream_id: Arc::new(AtomicU32::new(0)),
            sync_requested: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Sets or replaces the baked model database.
    pub fn set_model_db(&mut self, model_db: Option<Arc<BakedModelDatabase>>) {
        self.world.write().unwrap().set_model_db(model_db);
    }

    /// Toggles single unified world mesh mode.
    pub fn set_unified_mesh(&mut self, unified_mesh: bool) {
        self.world.write().unwrap().set_unified_mesh(unified_mesh);
    }

    /// Clears the cached section meshes and unified world mesh on the underlying VoxelWorld,
    /// marking all sections dirty.
    pub fn clear_cache(&self) {
        self.world.write().unwrap().clear_cache();
    }

    /// Sets or updates the active mesher configuration on the underlying VoxelWorld.
    pub fn set_config(&self, config: mtk_voxel::MesherConfig) {
        self.world.write().unwrap().set_config(config);
    }

    /// Hot-reloads mesher configuration and model database on the underlying VoxelWorld,
    /// purging all cached section meshes.
    pub fn hot_reload(
        &self,
        config: Option<mtk_voxel::MesherConfig>,
        model_db: Option<Arc<BakedModelDatabase>>,
    ) {
        let mut w = self.world.write().unwrap();
        if let Some(cfg) = config {
            w.set_config(cfg);
        }
        if let Some(db) = model_db {
            w.set_model_db(Some(db));
        }
        w.clear_cache();
    }

    /// Returns whether the background sync session worker is actively running.
    pub fn is_active(&self) -> bool {
        self.worker_running.load(Ordering::SeqCst)
    }

    /// Returns whether the session is actively connected to the server.
    pub fn is_connected(&self) -> bool {
        self.is_connected.load(Ordering::SeqCst)
    }

    /// Returns the current connection status string ("CONNECTED", "CONNECTING...", "DISCONNECTED", etc.).
    pub fn status(&self) -> String {
        self.status.read().unwrap().clone()
    }

    /// Starts the live sync session connecting to the given WebSocket `url`.
    pub fn start(
        &mut self,
        url: &str,
        auto_reconnect: bool,
        max_reconnect_attempts: usize,
    ) -> Result<(), String> {
        self.stop();

        *self.status.write().unwrap() = "CONNECTING...".to_string();
        self.is_connected.store(false, Ordering::SeqCst);

        let (msg_sender, msg_receiver) = crossbeam_channel::unbounded::<ClientMessage>();
        let client = SyncClient::connect(
            url.to_string(),
            msg_sender,
            auto_reconnect,
            max_reconnect_attempts,
        )?;

        let world_clone = self.world.clone();
        let event_sender_clone = self.event_sender.clone();
        let stream_id_clone = self.current_stream_id.clone();
        let sync_requested_clone = self.sync_requested.clone();
        self.sync_requested.store(false, Ordering::SeqCst);
        let worker_running = Arc::new(AtomicBool::new(true));
        self.worker_running = worker_running.clone();
        let is_connected_clone = self.is_connected.clone();
        let status_clone = self.status.clone();

        let cmd_sender = client.get_cmd_sender();
        let handle = thread::spawn(move || {
            event_worker_loop(
                msg_receiver,
                cmd_sender,
                world_clone,
                event_sender_clone,
                stream_id_clone,
                sync_requested_clone,
                worker_running,
                is_connected_clone,
                status_clone,
            );
        });

        self.client = Some(client);
        self.worker_handle = Some(handle);

        // Send initial sync config to Minecraft
        let _ = self.send_sync_config(0, 60, true);
        Ok(())
    }

    /// Stops the live sync session cleanly and shuts down the underlying connection.
    pub fn stop(&mut self) {
        self.worker_running.store(false, Ordering::SeqCst);
        self.is_connected.store(false, Ordering::SeqCst);
        *self.status.write().unwrap() = "DISCONNECTED".to_string();
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

    /// Queries the current full merged world geometry as a `MeshData`.
    pub fn get_world_mesh(&self) -> MeshData {
        let mut w = self.world.write().unwrap();
        if let Some(m) = w.get_world_mesh() {
            m.clone()
        } else {
            w.rebuild_all().cloned().unwrap_or_default()
        }
    }

    /// Returns the active slice of Atlas Chunk IDs referenced by the current world mesh.
    pub fn used_chunk_ids(&self) -> Vec<u32> {
        self.world.read().unwrap().used_chunk_ids().to_vec()
    }

    /// Sends a Full Sync Request (0x80) to Minecraft server.
    pub fn send_full_sync_request(&self) -> Result<(), String> {
        self.sync_requested.store(true, Ordering::SeqCst);
        let _ = self.event_sender.send(SyncEvent::StreamProgress {
            stage: "sync_request".to_string(),
            current: 0,
            total: 1,
            message: "Requesting full snapshot from server...".to_string(),
        });
        if let Some(ref client) = self.client {
            client.send_packet(encode_full_sync_request())
        } else {
            Err("LiveSyncSession is not connected".to_string())
        }
    }

    /// Sends Section Repair Requests (0x81) to Minecraft server.
    pub fn send_repair_request(&self, sections: &[IVec3]) -> Result<(), String> {
        self.sync_requested.store(true, Ordering::SeqCst);
        let _ = self.event_sender.send(SyncEvent::StreamProgress {
            stage: "sync_request".to_string(),
            current: 0,
            total: sections.len().max(1),
            message: format!("Requesting repair for {} sections...", sections.len()),
        });
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
    pub fn send_sync_config(
        &self,
        throttle_mode: u8,
        target_fps: u8,
        is_active: bool,
    ) -> Result<(), String> {
        if let Some(ref client) = self.client {
            client.send_packet(encode_sync_config(throttle_mode, target_fps, is_active))
        } else {
            Err("LiveSyncSession is not connected".to_string())
        }
    }

    /// Dispatches a raw packet into the session pipeline synchronously.
    pub fn process_packet_direct(&self, packet: Packet) {
        dispatcher::handle_packet(
            packet,
            0,
            &self.world,
            &self.event_sender,
            &self.current_stream_id,
            &mut 0,
            &mut 0,
            &mut 0,
            None,
            &self.sync_requested,
            false,
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

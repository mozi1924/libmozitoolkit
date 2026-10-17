//! # Sync Session Dispatcher and State Machine (RFC v3.1 Section 5)
//!
//! Event-driven background worker thread managing handshake negotiation,
//! progressive baseline chunk assembly, snapshot barrier staging, gap detection,
//! and parallel Rayon world meshing.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, Sender};
use glam::IVec3;
use mtk_voxel::VoxelWorld;

use super::assembler::BaselineAssembler;
use super::staging::DeltaStagingBuffer;
use crate::client::{ClientCommand, ClientMessage};
use crate::events::SyncEvent;
use crate::protocol::*;

/// Sync session lifecycle states (RFC Section 5.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncLifecycle {
    Disconnected,
    Connecting,
    Negotiating,
    Verifying,
    ReceivingBaseline,
    ValidatingHash,
    ReplayingStaged,
    BuildingMesh,
    Live,
}

/// Internal session state data container.
pub struct SessionState {
    pub lifecycle: SyncLifecycle,
    pub epoch: u64,
    pub sequence: u64,
    pub server_uuid: [u8; 16],
    pub min_pos: IVec3,
    pub size: IVec3,
    pub dimension: String,
    pub palette_revision: u32,
    pub palette_map: HashMap<u16, String>,
    pub staging: DeltaStagingBuffer,
    pub assembler: Option<BaselineAssembler>,
    pub last_mesh_time: Option<Instant>,
    pub pending_dirty_rebuild: bool,
}

impl SessionState {
    pub fn new() -> Self {
        Self {
            lifecycle: SyncLifecycle::Connecting,
            epoch: 0,
            sequence: 0,
            server_uuid: [0u8; 16],
            min_pos: IVec3::ZERO,
            size: IVec3::ZERO,
            dimension: "minecraft:overworld".to_string(),
            palette_revision: 0,
            palette_map: HashMap::new(),
            staging: DeltaStagingBuffer::new(),
            assembler: None,
            last_mesh_time: None,
            pending_dirty_rebuild: false,
        }
    }
}

impl Default for SessionState {
    fn default() -> Self {
        Self::new()
    }
}

/// Background worker loop processing incoming network messages and driving the sync state machine.
pub fn event_worker_loop(
    msg_receiver: Receiver<ClientMessage>,
    cmd_sender: Sender<ClientCommand>,
    world: Arc<RwLock<VoxelWorld>>,
    event_sender: Sender<SyncEvent>,
    running: Arc<AtomicBool>,
    is_connected: Arc<AtomicBool>,
    status_lock: Arc<RwLock<String>>,
    generation_atomic: Arc<AtomicU64>,
    state_lock: Arc<Mutex<SessionState>>,
    initial_hello: Option<Packet>,
) {
    while running.load(Ordering::Relaxed) {
        match msg_receiver.recv_timeout(Duration::from_millis(20)) {
            Ok(ClientMessage::Status(status)) => {
                let conn = status == "CONNECTED";
                is_connected.store(conn, Ordering::SeqCst);
                if let Ok(mut guard) = status_lock.write() {
                    *guard = status.clone();
                }
                let _ = event_sender.send(SyncEvent::StatusChange(status));

                if conn {
                    if let Ok(mut state) = state_lock.lock() {
                        state.lifecycle = SyncLifecycle::Negotiating;
                    }
                    // Send initial HELLO packet
                    let hello_pkt = initial_hello.clone().unwrap_or(Packet::Hello {
                        min_version: PROTOCOL_VERSION as u16,
                        max_version: PROTOCOL_VERSION as u16,
                        capabilities: 0x03, // Zstd + DNA Direct Memory
                        flags: 0,
                        cached_world_uuid: [0u8; 16],
                        cached_epoch: 0,
                        cached_seq: 0,
                        cached_min: IVec3::ZERO,
                        cached_size: IVec3::ZERO,
                        cached_state_hash: 0,
                        auth_token: String::new(),
                        cached_dimension: "minecraft:overworld".to_string(),
                    });
                    let bytes = encode_packet(&hello_pkt);
                    let _ = cmd_sender.send(ClientCommand::Send(bytes));
                }
            }
            Ok(ClientMessage::Disconnected) => {
                is_connected.store(false, Ordering::SeqCst);
                if let Ok(mut guard) = status_lock.write() {
                    *guard = "DISCONNECTED".to_string();
                }
                running.store(false, Ordering::SeqCst);
                if let Ok(mut state) = state_lock.lock() {
                    state.lifecycle = SyncLifecycle::Disconnected;
                }
                let _ = event_sender.send(SyncEvent::StatusChange("DISCONNECTED".to_string()));
                break;
            }
            Ok(ClientMessage::PacketReceived { packet, bytes }) => {
                if let Ok(mut state) = state_lock.lock() {
                    handle_packet(
                        packet,
                        bytes,
                        &mut state,
                        &world,
                        &cmd_sender,
                        &event_sender,
                        &generation_atomic,
                    );
                }
            }
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                if let Ok(mut state) = state_lock.lock() {
                    check_debounced_remesh(&mut state, &world, &event_sender, &generation_atomic);
                }
                continue;
            }
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
        }

        if let Ok(mut state) = state_lock.lock() {
            check_debounced_remesh(&mut state, &world, &event_sender, &generation_atomic);
        }
    }
}

/// Checks and performs throttled incremental dirty remeshing.
pub fn check_debounced_remesh(
    state: &mut SessionState,
    world: &Arc<RwLock<VoxelWorld>>,
    event_sender: &Sender<SyncEvent>,
    generation_atomic: &Arc<AtomicU64>,
) {
    if !state.pending_dirty_rebuild || state.lifecycle != SyncLifecycle::Live {
        return;
    }

    let can_remesh = state
        .last_mesh_time
        .is_none_or(|t| t.elapsed() >= Duration::from_millis(60));

    if can_remesh {
        state.pending_dirty_rebuild = false;
        state.last_mesh_time = Some(Instant::now());

        let mut w = world.write().unwrap();
        let _ = w.rebuild_dirty();
        if let Some(wm) = w.get_world_mesh().cloned() {
            let gen = generation_atomic.fetch_add(1, Ordering::SeqCst) + 1;
            let _ = event_sender.send(SyncEvent::WorldMeshReady {
                mesh: wm,
                generation: gen,
            });
        }
    }
}

/// Dispatches an individual packet against the session state machine.
pub fn handle_packet(
    packet: Packet,
    bytes_len: usize,
    state: &mut SessionState,
    world: &Arc<RwLock<VoxelWorld>>,
    cmd_sender: &Sender<ClientCommand>,
    event_sender: &Sender<SyncEvent>,
    generation_atomic: &Arc<AtomicU64>,
) {
    match packet {
        Packet::Welcome {
            negotiated_version: _,
            server_uuid,
            epoch,
            server_seq,
            current_min,
            current_size,
            current_state_hash: _,
            decision,
            message_or_dimension,
        } => {
            state.server_uuid = server_uuid;
            state.epoch = epoch;
            state.min_pos = current_min;
            state.size = current_size;
            state.dimension = message_or_dimension.clone();

            let _ = event_sender.send(SyncEvent::SelectionUpdated {
                min_pos: current_min,
                size: current_size,
            });

            let decision_str = match decision {
                SyncDecision::VerifiedSkipRebuild => "VERIFIED_SKIP_REBUILD",
                SyncDecision::ReplayDeltas => "REPLAY_DELTAS",
                SyncDecision::FullBaselineRequired => "FULL_BASELINE_REQUIRED",
                SyncDecision::Rejected => "REJECTED",
            };

            let _ = event_sender.send(SyncEvent::Handshake {
                epoch,
                sequence: server_seq,
                decision: decision_str.to_string(),
                dimension: message_or_dimension.clone(),
            });

            match decision {
                SyncDecision::VerifiedSkipRebuild => {
                    // Update sequence and enter LIVE immediately without rebuilding geometry
                    state.sequence = server_seq;
                    state.lifecycle = SyncLifecycle::Live;
                    let _ = event_sender.send(SyncEvent::Verified {
                        is_verified: true,
                        message: "100% in sync with scene (Zero-Rebuild verified)".to_string(),
                    });
                }
                SyncDecision::FullBaselineRequired => {
                    state.staging.clear();
                    state.assembler = None;
                    state.lifecycle = SyncLifecycle::ReceivingBaseline;
                    let _ = event_sender.send(SyncEvent::StreamProgress {
                        stage: "sync_download".to_string(),
                        current: 0,
                        total: 100,
                        message: "Waiting for Baseline stream...".to_string(),
                    });
                }
                SyncDecision::ReplayDeltas => {
                    state.lifecycle = SyncLifecycle::ReplayingStaged;
                }
                SyncDecision::Rejected => {
                    state.lifecycle = SyncLifecycle::Disconnected;
                    let _ = event_sender.send(SyncEvent::Error(format!(
                        "Connection rejected by server: {}",
                        message_or_dimension
                    )));
                }
            }
        }

        Packet::BaselineBegin {
            min_pos,
            size,
            ref block_palette,
            total_chunks,
            ..
        } => {
            state.min_pos = min_pos;
            state.size = size;
            let _ = event_sender.send(SyncEvent::SelectionUpdated { min_pos, size });

            state.palette_map.clear();
            for (idx, p) in block_palette.iter().enumerate() {
                state.palette_map.insert(idx as u16, p.clone());
            }

            match BaselineAssembler::new(&packet) {
                Ok(assembler) => {
                    state.assembler = Some(assembler);
                    state.lifecycle = SyncLifecycle::ReceivingBaseline;
                    let _ = event_sender.send(SyncEvent::StreamProgress {
                        stage: "sync_download".to_string(),
                        current: 0,
                        total: total_chunks as usize,
                        message: format!("Receiving Baseline stream (0/{})", total_chunks),
                    });
                }
                Err(reason) => {
                    let cancel_pkt = Packet::Cancel {
                        epoch: state.epoch,
                        snapshot_uuid: [0u8; 16],
                        reason,
                    };
                    let _ = cmd_sender.send(ClientCommand::Send(encode_packet(&cancel_pkt)));
                    let _ = event_sender.send(SyncEvent::Error(
                        "Failed initializing baseline stream assembler".to_string(),
                    ));
                }
            }
        }

        Packet::BaselineChunk {
            chunk_index,
            total_chunks,
            ..
        } => {
            if let Some(ref mut assembler) = state.assembler {
                match assembler.process_chunk(&packet) {
                    Ok(()) => {
                        let _ = event_sender.send(SyncEvent::StreamProgress {
                            stage: "sync_download".to_string(),
                            current: (chunk_index + 1) as usize,
                            total: total_chunks as usize,
                            message: format!(
                                "Receiving chunk {}/{}",
                                chunk_index + 1,
                                total_chunks
                            ),
                        });
                    }
                    Err(reason) => {
                        let resync_pkt = Packet::ResyncRequest {
                            epoch: state.epoch,
                            missing_from_seq: state.sequence,
                            missing_to_seq: state.sequence + 1,
                            reason,
                        };
                        let _ = cmd_sender.send(ClientCommand::Send(encode_packet(&resync_pkt)));
                    }
                }
            }
        }

        Packet::BaselineEnd {
            snapshot_uuid,
            snapshot_seq,
            ..
        } => {
            state.lifecycle = SyncLifecycle::ValidatingHash;
            let _ = event_sender.send(SyncEvent::StreamProgress {
                stage: "sync_meshing".to_string(),
                current: 50,
                total: 100,
                message: "Validating state hash and assembling voxels...".to_string(),
            });

            if let Some(assembler) = state.assembler.take() {
                match assembler.finalize(&packet) {
                    Ok(assembled_storage) => {
                        // 1. Sync assembled VoxelStorage to VoxelWorld
                        {
                            let mut w = world.write().unwrap();
                            w.sync_from_storage(&assembled_storage);
                        }

                        // 2. Commit Baseline ACK
                        let ack_pkt = Packet::BaselineAck {
                            epoch: state.epoch,
                            snapshot_uuid,
                            committed_seq: snapshot_seq,
                            result_code: BaselineAckResult::SuccessCommitted,
                        };
                        let _ = cmd_sender.send(ClientCommand::Send(encode_packet(&ack_pkt)));

                        // 3. Replay in-flight staged deltas (S + 1 .. latest)
                        state.lifecycle = SyncLifecycle::ReplayingStaged;
                        match state.staging.drain_replaying_after_snapshot(snapshot_seq) {
                            Ok(replayed_packets) => {
                                let mut current_seq = snapshot_seq;
                                for pkt in replayed_packets {
                                    if let Packet::DeltaOp {
                                        seq_id, changes, ..
                                    } = pkt
                                    {
                                        current_seq = seq_id;
                                        apply_delta_entries_to_world(
                                            &changes,
                                            &state.palette_map,
                                            world,
                                            state.min_pos,
                                        );
                                    }
                                }
                                state.sequence = current_seq;
                            }
                            Err(resync_reason) => {
                                let resync_pkt = Packet::ResyncRequest {
                                    epoch: state.epoch,
                                    missing_from_seq: snapshot_seq + 1,
                                    missing_to_seq: snapshot_seq + 2,
                                    reason: resync_reason,
                                };
                                let _ = cmd_sender
                                    .send(ClientCommand::Send(encode_packet(&resync_pkt)));
                            }
                        }

                        // 4. Build unified MeshData using parallel Rayon mesher
                        state.lifecycle = SyncLifecycle::BuildingMesh;
                        let _ = event_sender.send(SyncEvent::StreamProgress {
                            stage: "sync_meshing".to_string(),
                            current: 80,
                            total: 100,
                            message: "Building unified world mesh...".to_string(),
                        });

                        let (world_mesh, built_sections) = {
                            let mut w = world.write().unwrap();
                            let _ = w.rebuild_all();
                            let mesh = w.get_world_mesh().cloned().unwrap_or_default();
                            let count = w.storage.sections.len();
                            (mesh, count)
                        };

                        let gen = generation_atomic.fetch_add(1, Ordering::SeqCst) + 1;
                        let _ = event_sender.send(SyncEvent::WorldMeshReady {
                            mesh: world_mesh,
                            generation: gen,
                        });
                        let _ = event_sender.send(SyncEvent::StreamFinished { built_sections });

                        state.lifecycle = SyncLifecycle::Live;
                    }
                    Err(resync_reason) => {
                        let ack_pkt = Packet::BaselineAck {
                            epoch: state.epoch,
                            snapshot_uuid,
                            committed_seq: snapshot_seq,
                            result_code: BaselineAckResult::ValidationFailed,
                        };
                        let _ = cmd_sender.send(ClientCommand::Send(encode_packet(&ack_pkt)));

                        let resync_pkt = Packet::ResyncRequest {
                            epoch: state.epoch,
                            missing_from_seq: snapshot_seq,
                            missing_to_seq: snapshot_seq + 1,
                            reason: resync_reason,
                        };
                        let _ = cmd_sender.send(ClientCommand::Send(encode_packet(&resync_pkt)));
                    }
                }
            }
        }

        Packet::DeltaOp {
            epoch,
            seq_id,
            count: _,
            ref changes,
            ..
        } => {
            if epoch != state.epoch {
                return;
            }

            if state.lifecycle == SyncLifecycle::ReceivingBaseline {
                let _ = state.staging.push(seq_id, packet, bytes_len);
                return;
            }

            if state.lifecycle == SyncLifecycle::Live {
                let expected = state.sequence + 1;
                if seq_id == expected || state.sequence == 0 {
                    state.sequence = seq_id;
                    let affected = apply_delta_entries_to_world(
                        changes,
                        &state.palette_map,
                        world,
                        state.min_pos,
                    );
                    state.pending_dirty_rebuild = true;
                    let _ = event_sender.send(SyncEvent::DeltaApplied {
                        change_count: changes.len(),
                        affected_sections: affected,
                    });
                } else if seq_id > expected {
                    // Sequence Gap detected
                    let resync_pkt = Packet::ResyncRequest {
                        epoch: state.epoch,
                        missing_from_seq: expected,
                        missing_to_seq: seq_id - 1,
                        reason: ResyncReason::SequenceGap,
                    };
                    let _ = cmd_sender.send(ClientCommand::Send(encode_packet(&resync_pkt)));
                }
            }
        }

        Packet::PaletteUpdate {
            epoch,
            new_revision,
            entries,
            ..
        } => {
            if epoch == state.epoch {
                state.palette_revision = new_revision;
                for (id, str_val) in entries {
                    state.palette_map.insert(id, str_val);
                }
            }
        }

        Packet::Ping { timestamp } => {
            let pong = Packet::Pong { timestamp };
            let _ = cmd_sender.send(ClientCommand::Send(encode_packet(&pong)));
        }

        Packet::Error { message, .. } => {
            let _ = event_sender.send(SyncEvent::Error(message));
        }

        _ => {}
    }
}

/// Applies a list of delta entries into VoxelWorld storage.
fn apply_delta_entries_to_world(
    changes: &[DeltaChangeEntry],
    palette_map: &HashMap<u16, String>,
    world: &Arc<RwLock<VoxelWorld>>,
    min_pos: IVec3,
) -> Vec<IVec3> {
    let mut string_changes: Vec<(i32, i32, i32, &str)> = Vec::with_capacity(changes.len());
    for entry in changes {
        let abs_x = min_pos.x + entry.rel_pos.x;
        let abs_y = min_pos.y + entry.rel_pos.y;
        let abs_z = min_pos.z + entry.rel_pos.z;
        let state_str = palette_map
            .get(&entry.state_id)
            .map(|s| s.as_str())
            .unwrap_or("minecraft:air");
        string_changes.push((abs_x, abs_y, abs_z, state_str));
    }

    let mut w = world.write().unwrap();
    w.apply_delta_update(min_pos.x, min_pos.y, min_pos.z, &string_changes);

    let mut affected = Vec::new();
    for &(x, y, z, _) in &string_changes {
        let sec = IVec3::new(x >> 4, y >> 4, z >> 4);
        if !affected.contains(&sec) {
            affected.push(sec);
        }
    }
    affected
}

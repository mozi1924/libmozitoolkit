use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, RwLock};

use crossbeam_channel::{Receiver, Sender};
use glam::IVec3;
use mtk_voxel::VoxelWorld;

use crate::client::{ClientCommand, ClientMessage};
use crate::events::SyncEvent;
use crate::protocol::*;

/// Runs the background event worker loop processing incoming network messages and packet dispatch.
pub fn event_worker_loop(
    msg_receiver: Receiver<ClientMessage>,
    cmd_sender: Sender<ClientCommand>,
    world: Arc<RwLock<VoxelWorld>>,
    event_sender: Sender<SyncEvent>,
    stream_id_atomic: Arc<AtomicU32>,
    sync_requested: Arc<AtomicBool>,
    running: Arc<AtomicBool>,
    is_connected: Arc<AtomicBool>,
    status_lock: Arc<std::sync::RwLock<String>>,
) {
    let mut stream_total_sections: usize = 0;
    let mut stream_received_sections: usize = 0;
    let mut stream_received_bytes: usize = 0;
    let mut pending_non_streaming_updates = false;
    let mut pending_stream_mesh_rebuild = false;
    let mut last_sync_request_time: Option<std::time::Instant> = None;

    while running.load(Ordering::Relaxed) {
        match msg_receiver.recv_timeout(std::time::Duration::from_millis(50)) {
            Ok(ClientMessage::Status(status)) => {
                let conn = status == "CONNECTED";
                is_connected.store(conn, Ordering::SeqCst);
                if let Ok(mut guard) = status_lock.write() {
                    *guard = status.clone();
                }
                let _ = event_sender.send(SyncEvent::StatusChange(status));
            }
            Ok(ClientMessage::Disconnected) => {
                is_connected.store(false, Ordering::SeqCst);
                if let Ok(mut guard) = status_lock.write() {
                    *guard = "DISCONNECTED".to_string();
                }
                running.store(false, Ordering::SeqCst);
                let _ = event_sender.send(SyncEvent::StatusChange("DISCONNECTED".to_string()));
                break;
            }
            Ok(ClientMessage::PacketReceived { packet, bytes }) => {
                let is_sec_snapshot = matches!(packet, Packet::SectionSnapshot { .. });
                let is_delta_update = matches!(packet, Packet::DeltaUpdate { .. });
                let is_stream_end = matches!(packet, Packet::StreamEnd { .. });
                let more_pending = !msg_receiver.is_empty();
                handle_packet(
                    packet,
                    bytes,
                    &world,
                    &event_sender,
                    &stream_id_atomic,
                    &mut stream_total_sections,
                    &mut stream_received_sections,
                    &mut stream_received_bytes,
                    Some(&cmd_sender),
                    &sync_requested,
                    more_pending,
                );
                if (is_sec_snapshot && stream_id_atomic.load(Ordering::SeqCst) == 0)
                    || (is_delta_update && more_pending)
                {
                    pending_non_streaming_updates = true;
                }
                if is_stream_end && more_pending {
                    pending_stream_mesh_rebuild = true;
                }
            }
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                if pending_non_streaming_updates {
                    pending_non_streaming_updates = false;
                    sync_requested.store(false, Ordering::SeqCst);
                    last_sync_request_time = None;
                    let (unified, world_mesh) = {
                        let mut w = world.write().unwrap();
                        let _ = w.rebuild_dirty();
                        let wm = if w.unified_mesh {
                            w.get_world_mesh().cloned()
                        } else {
                            None
                        };
                        (w.unified_mesh, wm)
                    };
                    if unified {
                        if let Some(mesh) = world_mesh {
                            let _ = event_sender.send(SyncEvent::WorldMeshReady { mesh });
                        }
                    }
                }
                if pending_stream_mesh_rebuild {
                    pending_stream_mesh_rebuild = false;
                    let (unified, world_mesh) = {
                        let mut w = world.write().unwrap();
                        let m = w.rebuild_all().cloned().unwrap_or_default();
                        (w.unified_mesh, m)
                    };
                    if unified && !world_mesh.is_empty() {
                        let _ = event_sender.send(SyncEvent::WorldMeshReady { mesh: world_mesh });
                    }
                }
                // Auto-expire sync_requested latch after silence to recover from dropped packets
                if sync_requested.load(Ordering::SeqCst)
                    && stream_id_atomic.load(Ordering::SeqCst) == 0
                {
                    if let Some(t) = last_sync_request_time {
                        if t.elapsed() > std::time::Duration::from_millis(1500) {
                            sync_requested.store(false, Ordering::SeqCst);
                            last_sync_request_time = None;
                        }
                    } else {
                        last_sync_request_time = Some(std::time::Instant::now());
                    }
                } else {
                    last_sync_request_time = None;
                }
                continue;
            }
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
        }

        // If a batch of non-streaming section snapshots finished and channel is now quiet, remesh once
        if pending_non_streaming_updates && msg_receiver.is_empty() {
            pending_non_streaming_updates = false;
            sync_requested.store(false, Ordering::SeqCst);
            last_sync_request_time = None;
            let (unified, world_mesh) = {
                let mut w = world.write().unwrap();
                let _ = w.rebuild_dirty();
                let wm = if w.unified_mesh {
                    w.get_world_mesh().cloned()
                } else {
                    None
                };
                (w.unified_mesh, wm)
            };
            if unified {
                if let Some(mesh) = world_mesh {
                    let _ = event_sender.send(SyncEvent::WorldMeshReady { mesh });
                }
            }
        }

        // If consecutive stream batches finished and channel is now quiet, remesh all once
        if pending_stream_mesh_rebuild && msg_receiver.is_empty() {
            pending_stream_mesh_rebuild = false;
            let (unified, world_mesh) = {
                let mut w = world.write().unwrap();
                let m = w.rebuild_all().cloned().unwrap_or_default();
                (w.unified_mesh, m)
            };
            if unified && !world_mesh.is_empty() {
                let _ = event_sender.send(SyncEvent::WorldMeshReady { mesh: world_mesh });
            }
        }
    }

    running.store(false, Ordering::SeqCst);
    is_connected.store(false, Ordering::SeqCst);
    if let Ok(mut guard) = status_lock.write() {
        *guard = "DISCONNECTED".to_string();
    }
}

/// Dispatches a single packet into the session pipeline and triggers corresponding meshing and events.
pub fn handle_packet(
    packet: Packet,
    packet_bytes: usize,
    world: &Arc<RwLock<VoxelWorld>>,
    event_sender: &Sender<SyncEvent>,
    stream_id_atomic: &Arc<AtomicU32>,
    stream_total_sections: &mut usize,
    stream_received_sections: &mut usize,
    stream_received_bytes: &mut usize,
    cmd_sender: Option<&Sender<ClientCommand>>,
    sync_requested: &Arc<AtomicBool>,
    more_pending: bool,
) {
    match packet {
        Packet::SelectionInfo { min_pos, size } => {
            {
                let mut w = world.write().unwrap();
                w.set_bounds(min_pos.x, min_pos.y, min_pos.z, size.x, size.y, size.z);
            }
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
            sync_requested.store(false, Ordering::SeqCst);
            let identical = {
                let w = world.read().unwrap();
                w.storage.is_snapshot_identical(
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

            let _ = event_sender.send(SyncEvent::StreamProgress {
                stage: "sync_download".to_string(),
                current: 1,
                total: 1,
                message: if packet_bytes > 0 {
                    format!(
                        "Received full snapshot [{:.1} MB]",
                        packet_bytes as f64 / (1024.0 * 1024.0)
                    )
                } else {
                    "Received full snapshot, rebuilding world mesh...".to_string()
                },
            });

            let (total_sections, unified, mesh) = {
                let mut w = world.write().unwrap();
                w.set_full_snapshot(
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
                let total = w.storage.get_all_non_empty_sections().len();
                let sender = event_sender.clone();
                let m = w
                    .rebuild_all_with_progress(Some(&|p| {
                        let _ = sender.send(SyncEvent::StreamProgress {
                            stage: "sync_meshing".to_string(),
                            current: p.current,
                            total: p.total,
                            message: p.message,
                        });
                    }))
                    .cloned()
                    .unwrap_or_default();
                (total, w.unified_mesh, m)
            };

            if unified {
                let _ = event_sender.send(SyncEvent::WorldMeshReady { mesh });
            } else {
                let w = world.read().unwrap();
                for (i, (&coord, s_mesh)) in w.get_section_cache().iter().enumerate() {
                    let _ = event_sender.send(SyncEvent::SectionMeshReady {
                        coord,
                        mesh: s_mesh.clone(),
                    });
                    let _ = event_sender.send(SyncEvent::StreamProgress {
                        stage: "sync_meshing".to_string(),
                        current: i + 1,
                        total: total_sections,
                        message: format!("Meshed chunk ({}/{})", i + 1, total_sections),
                    });
                }
            }

            let _ = event_sender.send(SyncEvent::StreamFinished {
                stream_id: 0,
                built_sections: total_sections,
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
            {
                let mut w = world.write().unwrap();
                w.set_section_snapshot(
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
                *stream_received_bytes += packet_bytes;
                let mb_str = if *stream_received_bytes > 0 {
                    format!(
                        " [{:.1} MB]",
                        *stream_received_bytes as f64 / (1024.0 * 1024.0)
                    )
                } else {
                    String::new()
                };
                let _ = event_sender.send(SyncEvent::StreamProgress {
                    stage: "sync_download".to_string(),
                    current: *stream_received_sections,
                    total: *stream_total_sections,
                    message: format!(
                        "Receiving chunk ({}/{}){}",
                        *stream_received_sections, *stream_total_sections, mb_str
                    ),
                });
            } else if !more_pending {
                let (unified, section_mesh, world_mesh) = {
                    let mut w = world.write().unwrap();
                    let _ = w.rebuild_dirty();
                    let s_mesh = w
                        .get_section_cache()
                        .get(&sec_coord)
                        .cloned()
                        .unwrap_or_default();
                    let w_mesh = if w.unified_mesh {
                        w.get_world_mesh().cloned()
                    } else {
                        None
                    };
                    (w.unified_mesh, s_mesh, w_mesh)
                };

                if unified {
                    if let Some(mesh) = world_mesh {
                        let _ = event_sender.send(SyncEvent::WorldMeshReady { mesh });
                    }
                } else {
                    let _ = event_sender.send(SyncEvent::SectionMeshReady {
                        coord: sec_coord,
                        mesh: section_mesh,
                    });
                }
            }
        }

        Packet::DeltaUpdate {
            min_pos, changes, ..
        } => {
            let borrowed_changes: Vec<(i32, i32, i32, &str)> = changes
                .iter()
                .map(|c| {
                    (
                        c.rel_pos.x + min_pos.x,
                        c.rel_pos.y + min_pos.y,
                        c.rel_pos.z + min_pos.z,
                        c.state.as_str(),
                    )
                })
                .collect();

            if more_pending {
                // Batching fast-path: Ingest delta into voxel memory immediately (<0.1ms),
                // defer remeshing until incoming queue clears to coalesce rapid redstone bursts.
                {
                    let mut w = world.write().unwrap();
                    w.apply_delta_update(min_pos.x, min_pos.y, min_pos.z, &borrowed_changes);
                }
                let _ = event_sender.send(SyncEvent::DeltaApplied {
                    change_count: changes.len(),
                    affected_sections: Vec::new(),
                });
                return;
            }

            let (rebuilt, unified, world_mesh) = {
                let mut w = world.write().unwrap();
                w.apply_delta_update(min_pos.x, min_pos.y, min_pos.z, &borrowed_changes);
                let r = w.rebuild_dirty().unwrap_or_default();
                let wm = if w.unified_mesh {
                    w.get_world_mesh().cloned()
                } else {
                    None
                };
                (r, w.unified_mesh, wm)
            };

            let affected: Vec<IVec3> = rebuilt.iter().map(|(c, _)| *c).collect();
            if unified {
                if let Some(mesh) = world_mesh {
                    let _ = event_sender.send(SyncEvent::WorldMeshReady { mesh });
                }
            } else {
                for (coord, mesh) in rebuilt {
                    let _ = event_sender.send(SyncEvent::SectionMeshReady { coord, mesh });
                }
            }

            let _ = event_sender.send(SyncEvent::DeltaApplied {
                change_count: changes.len(),
                affected_sections: affected,
            });
        }

        Packet::SectionManifest {
            seq_id: _,
            sections,
        } => {
            let raw_entries: Vec<(i32, i32, i32, u32)> = sections
                .iter()
                .map(|s| (s.coord.x, s.coord.y, s.coord.z, s.crc32))
                .collect();

            let (mismatched, is_storage_empty) = {
                let mut w = world.write().unwrap();
                let mismatches = w.storage.validate_manifest(&raw_entries, None);
                let empty = w.storage.get_all_non_empty_sections().is_empty();
                (mismatches, empty)
            };

            let is_streaming = stream_id_atomic.load(Ordering::SeqCst) != 0;

            if mismatched.is_empty() {
                let (rebuilt_mesh, is_unified) = {
                    let mut w = world.write().unwrap();
                    let needs_remesh =
                        w.get_world_mesh().is_none() || !w.storage.dirty_sections.is_empty();
                    if needs_remesh && !w.storage.get_all_non_empty_sections().is_empty() {
                        let m = w.rebuild_all().cloned().unwrap_or_default();
                        (Some(m), w.unified_mesh)
                    } else {
                        (None, w.unified_mesh)
                    }
                };
                if is_unified {
                    if let Some(m) = rebuilt_mesh {
                        if !m.is_empty() {
                            let _ = event_sender.send(SyncEvent::WorldMeshReady { mesh: m });
                        }
                    }
                }
                let _ = event_sender.send(SyncEvent::Verified {
                    is_verified: true,
                    message: "100% in sync with scene".to_string(),
                });
            } else if !is_streaming {
                let _ = event_sender.send(SyncEvent::Verified {
                    is_verified: false,
                    message: format!("Detected {} out-of-sync sections", mismatched.len()),
                });

                if let Some(sender) = cmd_sender {
                    if !sync_requested.load(Ordering::SeqCst) {
                        if is_storage_empty || mismatched.len() == raw_entries.len() {
                            let _ = sender.send(ClientCommand::Send(encode_full_sync_request()));
                            sync_requested.store(true, Ordering::SeqCst);
                        } else {
                            for packet in encode_repair_requests(&mismatched, 512) {
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
            *stream_received_bytes = 0;
            let _ = event_sender.send(SyncEvent::StreamProgress {
                stage: "sync_download".to_string(),
                current: 0,
                total: *stream_total_sections,
                message: format!("Receiving {} chunks from server...", *stream_total_sections),
            });
        }

        Packet::StreamEnd {
            stream_id,
            sent_sections: _,
            status: _,
        } => {
            stream_id_atomic.store(0, Ordering::SeqCst);
            sync_requested.store(false, Ordering::SeqCst);

            let built = *stream_received_sections;
            let _ = event_sender.send(SyncEvent::StreamFinished {
                stream_id,
                built_sections: built,
            });

            if more_pending {
                return;
            }

            let (total, unified, mesh) = {
                let mut w = world.write().unwrap();
                let sender = event_sender.clone();
                let m = w
                    .rebuild_all_with_progress(Some(&|p| {
                        let _ = sender.send(SyncEvent::StreamProgress {
                            stage: "sync_meshing".to_string(),
                            current: p.current,
                            total: p.total,
                            message: p.message,
                        });
                    }))
                    .cloned()
                    .unwrap_or_default();
                let total_sections = w.get_section_cache().len();
                (total_sections, w.unified_mesh, m)
            };

            if unified {
                let _ = event_sender.send(SyncEvent::WorldMeshReady { mesh });
            } else {
                let w = world.read().unwrap();
                for (i, (&coord, s_mesh)) in w.get_section_cache().iter().enumerate() {
                    let _ = event_sender.send(SyncEvent::SectionMeshReady {
                        coord,
                        mesh: s_mesh.clone(),
                    });
                    let _ = event_sender.send(SyncEvent::StreamProgress {
                        stage: "sync_meshing".to_string(),
                        current: i + 1,
                        total,
                        message: format!("Meshed chunk ({}/{})", i + 1, total),
                    });
                }
            }
        }

        _ => {}
    }
}

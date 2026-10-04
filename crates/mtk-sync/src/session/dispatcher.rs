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
) {
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
                handle_packet(
                    packet,
                    &world,
                    &event_sender,
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

/// Dispatches a single packet into the session pipeline and triggers corresponding meshing and events.
pub fn handle_packet(
    packet: Packet,
    world: &Arc<RwLock<VoxelWorld>>,
    event_sender: &Sender<SyncEvent>,
    stream_id_atomic: &Arc<AtomicU32>,
    stream_total_sections: &mut usize,
    stream_received_sections: &mut usize,
    cmd_sender: Option<&Sender<ClientCommand>>,
    sync_requested: &Arc<AtomicBool>,
) {
    match packet {
        Packet::SelectionInfo { min_pos, size } => {
            let (bounds_changed, has_sections, unified) = {
                let mut w = world.write().unwrap();
                let changed = w.set_bounds(min_pos.x, min_pos.y, min_pos.z, size.x, size.y, size.z);
                let has = !w.storage.get_all_non_empty_sections().is_empty();
                (changed, has, w.unified_mesh)
            };

            if bounds_changed && has_sections {
                sync_requested.store(false, Ordering::SeqCst);
                let mesh = {
                    let mut w = world.write().unwrap();
                    w.rebuild_all().cloned().unwrap_or_default()
                };
                if unified && !mesh.is_empty() {
                    let _ = event_sender.send(SyncEvent::WorldMeshReady { mesh });
                }
            }

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
                let m = w.rebuild_all().cloned().unwrap_or_default();
                (total, w.unified_mesh, m)
            };

            let _ = event_sender.send(SyncEvent::StreamProgress {
                current: total_sections,
                total: total_sections,
                message: format!("Meshing {} sections...", total_sections),
            });

            if unified {
                let _ = event_sender.send(SyncEvent::WorldMeshReady { mesh });
            } else {
                let w = world.read().unwrap();
                for (i, (&coord, s_mesh)) in w.get_section_cache().iter().enumerate() {
                    let _ = event_sender.send(SyncEvent::SectionMeshReady { coord, mesh: s_mesh.clone() });
                    let _ = event_sender.send(SyncEvent::StreamProgress {
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
                let _ = event_sender.send(SyncEvent::StreamProgress {
                    current: *stream_received_sections,
                    total: *stream_total_sections,
                    message: format!("Receiving chunk ({}/{})", *stream_received_sections, *stream_total_sections),
                });
            } else {
                let (unified, section_mesh, world_mesh) = {
                    let mut w = world.write().unwrap();
                    let s_mesh = w.rebuild_single_section(sec_coord);
                    let w_mesh = if w.unified_mesh { w.get_world_mesh().cloned() } else { None };
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
            min_pos,
            changes,
            ..
        } => {
            let borrowed_changes: Vec<(i32, i32, i32, &str)> = changes
                .iter()
                .map(|c| (c.rel_pos.x + min_pos.x, c.rel_pos.y + min_pos.y, c.rel_pos.z + min_pos.z, c.state.as_str()))
                .collect();

            let (rebuilt, unified, world_mesh) = {
                let mut w = world.write().unwrap();
                w.apply_delta_update(min_pos.x, min_pos.y, min_pos.z, &borrowed_changes);
                let r = w.rebuild_dirty().unwrap_or_default();
                let wm = if w.unified_mesh { w.get_world_mesh().cloned() } else { None };
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

        Packet::SectionManifest { seq_id: _, sections } => {
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

            let (total, unified, mesh) = {
                let mut w = world.write().unwrap();
                let m = w.rebuild_all().cloned().unwrap_or_default();
                let total_sections = w.get_section_cache().len();
                (total_sections, w.unified_mesh, m)
            };

            if unified {
                let _ = event_sender.send(SyncEvent::WorldMeshReady { mesh });
            } else {
                let w = world.read().unwrap();
                for (i, (&coord, s_mesh)) in w.get_section_cache().iter().enumerate() {
                    let _ = event_sender.send(SyncEvent::SectionMeshReady { coord, mesh: s_mesh.clone() });
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

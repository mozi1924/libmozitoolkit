use mtk_sync::LiveSyncSession;
use mtk_voxel::types::MesherConfig;

#[test]
fn test_sync_session_initialization() {
    let session = LiveSyncSession::new(Some(MesherConfig::default()), None, None, true);
    let events = session.poll_events();
    assert!(events.is_empty());
    assert!(session.world.read().unwrap().unified_mesh);
}

#[test]
fn test_sync_session_packet_flow() {
    let session = LiveSyncSession::new(Some(MesherConfig::default()), None, None, true);

    // 1. Manually test storage update and meshing on session world
    {
        let mut w = session.world.write().unwrap();
        w.set_bounds(0, 0, 0, 16, 16, 16);
        let palette = vec!["minecraft:air".to_string(), "minecraft:stone".to_string()];
        let mut grid = vec![0u16; 4096];
        grid[0] = 1;
        w.set_section_snapshot(0, 0, 0, 0, 0, 0, 16, 16, 16, &palette, &grid, None, None);
        assert_eq!(w.get_block(0, 0, 0), "minecraft:stone");
    }

    // 2. Test get_world_mesh on session (welded by default: 8 vertices, 36 indices, 24 UVs)
    let world_mesh = session.get_world_mesh();
    assert!(!world_mesh.is_empty());
    assert_eq!(world_mesh.positions.len(), 8); // 1 cube welded vertices
    assert_eq!(world_mesh.indices.len(), 36); // 6 faces * 2 tris * 3 indices
    assert_eq!(world_mesh.uvs.len(), 24); // 6 faces * 4 corner UVs
}

#[test]
fn test_two_phase_streaming_cross_chunk_culling_and_welding() {
    use glam::IVec3;
    use mtk_sync::protocol::packet::Packet;
    use mtk_sync::SyncEvent;

    let session = LiveSyncSession::new(Some(MesherConfig::default()), None, None, true);

    // Stream 2 sections:
    // Section (0, 0, 0) has stone at (15, 0, 0).
    // Section (1, 0, 0) has stone at (16, 0, 0).
    // These two stone blocks touch across chunk boundary x=15 and x=16!

    // Step 1: StreamBegin
    session.process_packet_direct(Packet::StreamBegin {
        stream_id: 42,
        total_sections: 2,
        flags: 0,
    });

    // Step 2: SectionSnapshot (0, 0, 0)
    let palette_a = vec!["minecraft:air".to_string(), "minecraft:stone".to_string()];
    let mut grid_a = vec![0u16; 4096];
    grid_a[15 * 256] = 1;
    session.process_packet_direct(Packet::SectionSnapshot {
        sec_coord: IVec3::new(0, 0, 0),
        start_pos: IVec3::new(0, 0, 0),
        size: IVec3::new(16, 16, 16),
        palette: palette_a,
        grid_indices: grid_a,
        biome_palette: None,
        biome_indices: None,
    });

    // Step 3: SectionSnapshot (1, 0, 0)
    let palette_b = vec!["minecraft:air".to_string(), "minecraft:stone".to_string()];
    let mut grid_b = vec![0u16; 4096];
    grid_b[0] = 1;
    session.process_packet_direct(Packet::SectionSnapshot {
        sec_coord: IVec3::new(1, 0, 0),
        start_pos: IVec3::new(16, 0, 0),
        size: IVec3::new(16, 16, 16),
        palette: palette_b,
        grid_indices: grid_b,
        biome_palette: None,
        biome_indices: None,
    });

    // Step 4: StreamEnd
    session.process_packet_direct(Packet::StreamEnd {
        stream_id: 42,
        sent_sections: 2,
        status: mtk_sync::protocol::constants::StreamStatus::Success,
    });

    // Meshing should now have completed!
    let events = session.poll_events();
    let world_mesh_event = events.iter().find_map(|e| match e {
        SyncEvent::WorldMeshReady { mesh } => Some(mesh),
        _ => None,
    });

    assert!(
        world_mesh_event.is_some(),
        "WorldMeshReady event must be emitted on StreamEnd"
    );
    let mesh = world_mesh_event.unwrap();

    // Two touching unit cubes:
    // Faces: 6 + 6 - 2 (culled shared boundary face) = 10 faces!
    // Each face has 4 corner UVs -> 10 * 4 = 40 UVs
    // Each face has 2 triangles -> 10 * 6 = 60 indices
    // Vertices: 8 + 8 - 4 (welded shared vertices on x=16 plane) = 12 vertices!
    assert_eq!(
        mesh.uvs.len(),
        40,
        "Expected 10 faces (40 loop UVs), found {}",
        mesh.uvs.len()
    );
    assert_eq!(
        mesh.indices.len(),
        60,
        "Expected 60 triangle indices, found {}",
        mesh.indices.len()
    );
    assert_eq!(
        mesh.positions.len(),
        12,
        "Expected 12 welded spatial vertices across chunk seam, found {}",
        mesh.positions.len()
    );
}

#[test]
fn test_auto_sync_request_on_manifest_mismatch() {
    use glam::IVec3;
    use mtk_sync::client::ClientCommand;
    use mtk_sync::protocol::constants::PacketType;
    use mtk_sync::protocol::packet::{ManifestSectionEntry, Packet};
    use mtk_sync::{session::dispatcher, LiveSyncSession, SyncEvent};
    use mtk_voxel::types::MesherConfig;
    use std::sync::atomic::{AtomicBool, AtomicU32};
    use std::sync::Arc;

    let session = LiveSyncSession::new(Some(MesherConfig::default()), None, None, true);
    let (cmd_sender, cmd_receiver) = crossbeam_channel::unbounded::<ClientCommand>();
    let (event_sender, _event_receiver) = crossbeam_channel::unbounded::<SyncEvent>();
    let sync_requested = Arc::new(AtomicBool::new(false));
    let stream_id = Arc::new(AtomicU32::new(0));
    let mut total_sec = 0;
    let mut rec_sec = 0;

    // 1. Cold start / empty storage: manifest with 1 non-empty section
    let manifest_packet = Packet::SectionManifest {
        seq_id: 1,
        sections: vec![ManifestSectionEntry {
            coord: IVec3::new(0, 0, 0),
            crc32: 0xDEADBEEF,
        }],
    };

    dispatcher::handle_packet(
        manifest_packet,
        0,
        &session.world,
        &event_sender,
        &stream_id,
        &mut total_sec,
        &mut rec_sec,
        &mut 0,
        Some(&cmd_sender),
        &sync_requested,
        false,
    );

    // Should have sent ReqFullSync (0x80)
    let cmd = cmd_receiver
        .try_recv()
        .expect("Should have sent a command to server");
    match cmd {
        ClientCommand::Send(bytes) => {
            assert_eq!(bytes[2], 0x02, "Version should be 2");
            assert_eq!(
                bytes[3],
                PacketType::ReqFullSync as u8,
                "Packet type should be ReqFullSync (0x80)"
            );
        }
        _ => panic!("Expected ClientCommand::Send"),
    }
    assert!(
        sync_requested.load(std::sync::atomic::Ordering::SeqCst),
        "sync_requested flag should be set"
    );
}

#[test]
fn test_sync_session_status_and_rapid_stop() {
    let mut session = LiveSyncSession::new(Some(MesherConfig::default()), None, None, true);
    assert!(!session.is_active());
    assert!(!session.is_connected());
    assert_eq!(session.status(), "DISCONNECTED");

    // Start session with non-existent port; should enter connecting state
    let start_res = session.start("ws://127.0.0.1:59999", true, 5);
    assert!(start_res.is_ok());
    assert!(session.is_active());
    assert!(!session.is_connected());

    // Give it a tiny moment to attempt connect and enter reconnecting sleep
    std::thread::sleep(std::time::Duration::from_millis(50));

    let start_time = std::time::Instant::now();
    session.stop();
    let elapsed = start_time.elapsed();

    // Verify rapid stop without waiting for full reconnect backoffs (1500ms)
    assert!(
        elapsed < std::time::Duration::from_millis(500),
        "session.stop() took too long: {:?}",
        elapsed
    );
    assert!(!session.is_active());
    assert!(!session.is_connected());
    assert_eq!(session.status(), "DISCONNECTED");
}

#[test]
fn test_post_stream_manifest_verification_and_streaming_suppression() {
    use glam::IVec3;
    use mtk_sync::protocol::packet::{ManifestSectionEntry, Packet};
    use mtk_sync::SyncEvent;

    let session = LiveSyncSession::new(Some(MesherConfig::default()), None, None, true);

    // 1. Begin stream
    session.process_packet_direct(Packet::StreamBegin {
        stream_id: 100,
        total_sections: 1,
        flags: 0,
    });

    // 2. While streaming, receiving a manifest (e.g. before chunks finish) must NOT emit mismatch error
    session.process_packet_direct(Packet::SectionManifest {
        seq_id: 100,
        sections: vec![ManifestSectionEntry {
            coord: IVec3::new(0, 0, 0),
            crc32: 0x12345678,
        }],
    });
    let evts = session.poll_events();
    let has_unverified = evts.iter().any(|e| {
        matches!(
            e,
            SyncEvent::Verified {
                is_verified: false,
                ..
            }
        )
    });
    assert!(
        !has_unverified,
        "Must suppress false mismatch verification while streaming"
    );

    // 3. Snapshot arrives
    let palette = vec!["minecraft:air".to_string(), "minecraft:stone".to_string()];
    let mut grid = vec![0u16; 4096];
    grid[0] = 1;
    session.process_packet_direct(Packet::SectionSnapshot {
        sec_coord: IVec3::new(0, 0, 0),
        start_pos: IVec3::new(0, 0, 0),
        size: IVec3::new(16, 16, 16),
        palette,
        grid_indices: grid,
        biome_palette: None,
        biome_indices: None,
    });

    // 4. Stream ends
    session.process_packet_direct(Packet::StreamEnd {
        stream_id: 100,
        sent_sections: 1,
        status: mtk_sync::protocol::constants::StreamStatus::Success,
    });

    // Calculate actual local CRC of the section we just ingested
    let expected_crc = {
        let mut w = session.world.write().unwrap();
        w.storage
            .calculate_and_store_section_crc(IVec3::new(0, 0, 0))
    };

    // 5. Post-stream Manifest arrives with matching CRC
    session.process_packet_direct(Packet::SectionManifest {
        seq_id: 100,
        sections: vec![ManifestSectionEntry {
            coord: IVec3::new(0, 0, 0),
            crc32: expected_crc,
        }],
    });

    let final_evts = session.poll_events();
    let verified_event = final_evts.iter().find_map(|e| match e {
        SyncEvent::Verified {
            is_verified,
            message,
        } => Some((*is_verified, message.clone())),
        _ => None,
    });

    assert_eq!(
        verified_event,
        Some((true, "100% in sync with scene".to_string())),
        "Must verify 100% in sync upon receiving post-stream manifest"
    );
}

#[test]
fn test_sync_session_clear_cache_and_hot_reload() {
    let session = LiveSyncSession::new(Some(MesherConfig::default()), None, None, true);

    {
        let mut w = session.world.write().unwrap();
        w.set_bounds(0, 0, 0, 16, 16, 16);
        let palette = vec!["minecraft:air".to_string(), "minecraft:stone".to_string()];
        let mut grid = vec![0u16; 4096];
        grid[0] = 1;
        w.set_section_snapshot(0, 0, 0, 0, 0, 0, 16, 16, 16, &palette, &grid, None, None);
    }

    let mesh = session.get_world_mesh();
    assert!(!mesh.is_empty());
    assert!(session.world.read().unwrap().get_world_mesh().is_some());

    // Clear cache
    session.clear_cache();
    assert!(session.world.read().unwrap().get_world_mesh().is_none());

    // Hot reload
    let new_cfg = MesherConfig {
        enable_ao: false,
        ..Default::default()
    };
    session.hot_reload(Some(new_cfg), None);
    assert!(!session.world.read().unwrap().config.enable_ao);
    assert!(session.world.read().unwrap().get_world_mesh().is_none());

    // Remesh
    let re_mesh = session.get_world_mesh();
    assert!(!re_mesh.is_empty());
}

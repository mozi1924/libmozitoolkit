//! Regression test for Live Sync incremental remeshing.
//!
//! The baseline path rebuilds the whole world (full spatial weld), but live deltas go
//! through `check_debounced_remesh` → `VoxelWorld::rebuild_dirty`. That incremental path
//! used to skip cross-Section welding entirely, silently splitting every previously
//! welded chunk seam after the first block edit.

use std::collections::HashSet;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, RwLock};

use crossbeam_channel::unbounded;
use mtk_sync::events::SyncEvent;
use mtk_sync::session::{check_debounced_remesh, SessionState, SyncLifecycle};
use mtk_voxel::types::MesherConfig;
use mtk_voxel::VoxelWorld;

fn assert_no_duplicate_positions(mesh: &mtk_core::mesh::MeshData) {
    let mut seen = HashSet::new();
    for p in &mesh.positions {
        assert!(
            seen.insert([p[0].to_bits(), p[1].to_bits(), p[2].to_bits()]),
            "duplicate vertex position survived live delta remeshing: {p:?}"
        );
    }
}

#[test]
fn test_live_delta_remesh_keeps_cross_section_seams_welded() {
    let config = MesherConfig {
        weld_vertices: true,
        origin_centered: true,
        ..Default::default()
    };

    let mut world = VoxelWorld::new(Some(config), None, None);
    world.set_bounds(0, 0, 0, 32, 16, 16);
    // Two cubes facing each other across the x = 16 Section seam.
    world.set_block(15, 0, 0, "minecraft:stone", None);
    world.set_block(16, 0, 0, "minecraft:stone", None);

    let baseline = world.rebuild_all().unwrap().clone();
    assert_eq!(baseline.vertex_count(), 12);
    assert_no_duplicate_positions(&baseline);

    // A live DELTA_OP touches one block in Section 0 and schedules a dirty remesh.
    world.apply_delta_update(0, 0, 0, &[(0, 5, 5, "minecraft:dirt")]);

    let world = Arc::new(RwLock::new(world));
    let mut state = SessionState::new();
    state.lifecycle = SyncLifecycle::Live;
    state.pending_dirty_rebuild = true;
    state.last_mesh_time = None;

    let (event_sender, event_receiver) = unbounded();
    let generation = Arc::new(AtomicU64::new(7));
    check_debounced_remesh(&mut state, &world, &event_sender, &generation);

    let mesh = {
        let guard = world.read().unwrap();
        guard.get_world_mesh().unwrap().clone()
    };
    // Two welded seam cubes (12) + one welded dirt cube (8).
    assert_eq!(mesh.vertex_count(), 20);
    assert_eq!(mesh.face_count(), 16);
    assert_no_duplicate_positions(&mesh);

    let mut emitted = None;
    while let Ok(event) = event_receiver.try_recv() {
        if let SyncEvent::WorldMeshReady {
            mesh, generation, ..
        } = event
        {
            emitted = Some((mesh, generation));
        }
    }
    let (emitted_mesh, emitted_generation) =
        emitted.expect("delta remesh must emit WorldMeshReady");
    assert_eq!(emitted_generation, 8);
    assert_eq!(emitted_mesh.vertex_count(), 20);
    assert_no_duplicate_positions(&emitted_mesh);
}

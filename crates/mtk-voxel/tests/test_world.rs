use mtk_voxel::types::MesherConfig;
use mtk_voxel::VoxelWorld;

#[test]
fn test_voxel_world_lifecycle_and_used_chunks() {
    let config = MesherConfig {
        weld_vertices: true,
        origin_centered: true,
        ..Default::default()
    };

    let mut world = VoxelWorld::new(Some(config), None, None, true);
    world.set_bounds(0, 0, 0, 16, 16, 16);
    world.set_block(0, 0, 0, "minecraft:stone", None);
    world.set_block(1, 0, 0, "minecraft:dirt", None);

    let mesh = world.rebuild_all().unwrap();
    assert!(!mesh.is_empty());
    assert!(mesh.vertex_count() > 0);

    // Initial used chunks (unit cubes default to chunk 0 without atlas)
    let used_chunks = world.used_chunk_ids();
    assert_eq!(used_chunks, &[0]);

    // Test incremental delta modification
    world.set_block(0, 0, 0, "minecraft:air", None);
    let rebuilt = world.rebuild_dirty().unwrap();
    assert_eq!(rebuilt.len(), 1);

    let updated_mesh = world.get_world_mesh().unwrap();
    assert!(!updated_mesh.is_empty());

    // Test compact materials
    let mapping = world.compact_world_mesh_materials();
    assert_eq!(mapping, vec![0]);
}

#[test]
fn test_voxel_world_rebuild_with_progress() {
    use mtk_core::progress::ProgressReport;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    let mut world = VoxelWorld::new(None, None, None, true);
    world.set_bounds(0, 0, 0, 32, 16, 16);
    world.set_block(0, 0, 0, "minecraft:stone", None);
    world.set_block(16, 0, 0, "minecraft:stone", None);

    let progress_events = Arc::new(AtomicUsize::new(0));
    let pe_clone = Arc::clone(&progress_events);

    let cb = move |r: ProgressReport| {
        pe_clone.fetch_add(1, Ordering::SeqCst);
        assert!(r.current <= r.total);
    };

    let mesh = world.rebuild_all_with_progress(Some(&cb)).unwrap();
    assert!(!mesh.is_empty());
    // Should have reported progress for meshing and assembly
    assert!(progress_events.load(Ordering::SeqCst) >= 2);
}

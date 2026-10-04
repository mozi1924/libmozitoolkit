use mtk_voxel::VoxelWorld;
use mtk_voxel::types::MesherConfig;

#[test]
fn test_voxel_world_lifecycle_and_used_chunks() {
    let mut config = MesherConfig::default();
    config.weld_vertices = true;
    config.origin_centered = true;

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

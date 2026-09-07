use glam::IVec3;
use mtk_cull::FaceCuller;
use mtk_voxel::mesher::SectionMesher;
use mtk_voxel::types::MesherConfig;
use mtk_voxel::world::VoxelStorage;

#[test]
fn test_solid_cube_culling_in_mesher() {
    let mut world = VoxelStorage::new();
    world.set_bounds(0, 0, 0, 16, 16, 16);

    // 2 adjacent solid cubes at (0,0,0) and (1,0,0)
    world.set_block(0, 0, 0, "minecraft:stone", None);
    world.set_block(1, 0, 0, "minecraft:stone", None);

    let padded = world.get_section_padded_array(IVec3::new(0, 0, 0));
    let culler = FaceCuller::default();
    let config = MesherConfig::default();

    let mesh = SectionMesher::mesh_section(&padded, &culler, |_| None, &config);

    // Two touching cubes have 2 * 6 - 2 = 10 visible faces = 20 triangles
    assert_eq!(mesh.face_count(), 10);
    assert_eq!(mesh.triangle_count(), 20);
    assert_eq!(mesh.vertex_count(), 40);
}

#[test]
fn test_fluid_and_solid_meshing() {
    let mut world = VoxelStorage::new();
    world.set_bounds(0, 0, 0, 16, 16, 16);

    world.set_block(0, 0, 0, "minecraft:stone", None);
    world.set_block(0, 1, 0, "minecraft:water", None);

    let padded = world.get_section_padded_array(IVec3::new(0, 0, 0));
    let culler = FaceCuller::default();
    let config = MesherConfig::default();

    let mesh = SectionMesher::mesh_section(&padded, &culler, |_| None, &config);

    // The water block on top of stone should generate visible fluid faces,
    // and the bottom of water against solid stone should be occluded!
    assert!(mesh.face_count() > 0);
}

#[cfg(feature = "parallel")]
#[test]
fn test_parallel_batch_mesher() {
    let mut world = VoxelStorage::new();
    world.set_bounds(0, 0, 0, 32, 16, 16);

    world.set_block(0, 0, 0, "minecraft:oak_planks", None);
    world.set_block(16, 0, 0, "minecraft:stone", None);

    let p0 = world.get_section_padded_array(IVec3::new(0, 0, 0));
    let p1 = world.get_section_padded_array(IVec3::new(1, 0, 0));

    let culler = FaceCuller::default();
    let config = MesherConfig::default();

    let results = SectionMesher::mesh_sections_parallel(
        &[p0, p1],
        &culler,
        |_| None,
        &config,
        Some(2),
    )
    .expect("Parallel meshing should succeed");

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].0, IVec3::new(0, 0, 0));
    assert_eq!(results[1].0, IVec3::new(1, 0, 0));
    assert!(!results[0].1.is_empty());
    assert!(!results[1].1.is_empty());
}

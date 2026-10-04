use glam::IVec3;
use mtk_cull::FaceCuller;
use mtk_voxel::mesher::SectionMesher;
use mtk_voxel::source::ingest_from_source;
use mtk_voxel::storage::{VoxelPointCloud, VoxelStorage};
use mtk_voxel::types::{CoordinateSystem, MesherConfig};

#[test]
fn test_point_cloud_extraction_no_culling_and_roundtrip() {
    let mut world = VoxelStorage::new();
    world.set_bounds(0, 0, 0, 16, 16, 16);

    // Build a solid 3x3x3 cube of stone blocks
    // Internal 1x1x1 center block (1, 1, 1) is completely occluded from all 6 sides
    for x in 0..3 {
        for y in 0..3 {
            for z in 0..3 {
                world.set_block(x, y, z, "minecraft:stone", Some("minecraft:desert"));
            }
        }
    }
    world.set_block(10, 10, 10, "minecraft:diamond_block", Some("minecraft:plains"));

    let config = MesherConfig::default();

    // 1. Verify point cloud extraction contains ALL 28 blocks without culling
    let cloud = world.to_point_cloud(&config);
    assert_eq!(cloud.len(), 28, "All 28 blocks including fully occluded ones must be present");

    // Check that occluded center block (1, 1, 1) is unconditionally preserved
    let mut found_center = false;
    for i in 0..cloud.len() {
        if cloud.block_x[i] == 1 && cloud.block_y[i] == 1 && cloud.block_z[i] == 1 {
            assert_eq!(cloud.block_states[i], "minecraft:stone");
            assert_eq!(cloud.biomes[i], "minecraft:desert");
            found_center = true;
            break;
        }
    }
    assert!(found_center, "Occluded center block must be in point cloud");

    // 2. Roundtrip: reconstruct VoxelStorage from point cloud
    let restored_world = VoxelStorage::from_point_cloud(&cloud);
    for x in 0..3 {
        for y in 0..3 {
            for z in 0..3 {
                assert_eq!(restored_world.get_block(x, y, z), "minecraft:stone");
                assert_eq!(restored_world.get_biome(x, y, z), "minecraft:desert");
            }
        }
    }
    assert_eq!(restored_world.get_block(10, 10, 10), "minecraft:diamond_block");
    assert_eq!(restored_world.get_biome(10, 10, 10), "minecraft:plains");
}

#[test]
fn test_point_cloud_mesh_data_conversion() {
    let mut cloud = VoxelPointCloud::new();
    cloud.push(
        [1.5, 2.5, 3.5],
        [1, 2, 3],
        "minecraft:oak_stairs[facing=east]".to_string(),
        "minecraft:forest".to_string(),
        15,
    );

    let mesh_data = cloud.to_mesh_data();
    assert_eq!(mesh_data.positions.len(), 1);
    assert_eq!(mesh_data.indices.len(), 0, "Point cloud mesh data must have no face indices");

    let restored_cloud = VoxelPointCloud::from_mesh_data(&mesh_data).expect("Must parse from MeshData");
    assert_eq!(restored_cloud.len(), 1);
    assert_eq!(restored_cloud.block_x[0], 1);
    assert_eq!(restored_cloud.block_y[0], 2);
    assert_eq!(restored_cloud.block_z[0], 3);
    assert_eq!(restored_cloud.block_states[0], "minecraft:oak_stairs[facing=east]");
    assert_eq!(restored_cloud.biomes[0], "minecraft:forest");
    assert_eq!(restored_cloud.light_levels[0], 15);
}

#[test]
fn test_user_carving_via_point_cloud_deletion() {
    // 1. Create a 4x4x4 solid stone cube (64 blocks total)
    let mut world = VoxelStorage::new();
    world.set_bounds(0, 0, 0, 16, 16, 16);

    for x in 0..4 {
        for y in 0..4 {
            for z in 0..4 {
                world.set_block(x, y, z, "minecraft:stone", Some("minecraft:plains"));
            }
        }
    }

    let culler = FaceCuller::default();
    let config = MesherConfig::default();

    // 2. Initial meshing of the solid cube:
    // Outer boundary has 4*4 = 16 faces per direction * 6 directions = 96 quads (192 triangles)
    let padded = world.get_section_padded_array(IVec3::new(0, 0, 0));
    let initial_mesh = SectionMesher::mesh_section(&padded, &culler, |_| None, &config);
    let initial_quad_count = initial_mesh.quad_count();
    assert_eq!(initial_quad_count, 96, "Initial 4x4x4 solid cube should only have 96 outer quads");

    // 3. Extract unculled point cloud -> must have all 64 points
    let original_cloud = world.to_point_cloud(&config);
    assert_eq!(original_cloud.len(), 64);

    // 4. Simulate user in Blender Edit Mode selecting and deleting internal blocks
    // User deletes a 2x2x2 cavity inside the cube: [1..=2, 1..=2, 1..=2] (8 blocks deleted)
    let mut modified_cloud = VoxelPointCloud::new();
    for i in 0..original_cloud.len() {
        let bx = original_cloud.block_x[i];
        let by = original_cloud.block_y[i];
        let bz = original_cloud.block_z[i];

        let in_cavity = (1..=2).contains(&bx) && (1..=2).contains(&by) && (1..=2).contains(&bz);
        if !in_cavity {
            modified_cloud.push(
                [
                    original_cloud.positions[i * 3],
                    original_cloud.positions[i * 3 + 1],
                    original_cloud.positions[i * 3 + 2],
                ],
                [bx, by, bz],
                original_cloud.block_states[i].clone(),
                original_cloud.biomes[i].clone(),
                original_cloud.light_levels[i],
            );
        }
    }
    assert_eq!(modified_cloud.len(), 56, "64 - 8 = 56 blocks remain");

    // 5. Reconstruct VoxelStorage from modified point cloud and remesh
    let carved_world = VoxelStorage::from_point_cloud(&modified_cloud);
    assert_eq!(carved_world.get_block(1, 1, 1), "minecraft:air", "Deleted block is now air");

    let carved_padded = carved_world.get_section_padded_array(IVec3::new(0, 0, 0));
    let carved_mesh = SectionMesher::mesh_section(&carved_padded, &culler, |_| None, &config);

    // 6. Verify that carving out the 2x2x2 cavity exposed new internal cavity faces!
    // Internal cavity has 2*2 = 4 faces per direction * 6 directions = 24 new inner quads.
    // Total quads = 96 outer quads + 24 inner quads = 120 quads.
    assert_eq!(carved_mesh.quad_count(), 120, "Mesh should now include both outer and carved inner cavity faces");
    assert!(carved_mesh.quad_count() > initial_quad_count);
}

#[test]
fn test_point_cloud_voxel_source_ingestion() {
    let mut cloud = VoxelPointCloud::new();
    cloud.push([0.5, 0.5, 0.5], [0, 0, 0], "minecraft:stone".to_string(), "minecraft:plains".to_string(), 0);
    cloud.push([16.5, 0.5, 0.5], [16, 0, 0], "minecraft:gold_block".to_string(), "minecraft:desert".to_string(), 0);

    let mut source = cloud.as_source();
    let mut target = VoxelStorage::new();

    let count = ingest_from_source(&mut source, &mut target, None).expect("Ingest must succeed");
    assert_eq!(count, 2, "Should have loaded 2 sections");
    assert_eq!(target.get_block(0, 0, 0), "minecraft:stone");
    assert_eq!(target.get_block(16, 0, 0), "minecraft:gold_block");
}

#[test]
fn test_point_cloud_origin_centered_alignment() {
    let mut storage = VoxelStorage::new();
    storage.set_bounds(0, 0, 0, 10, 10, 10);
    storage.set_block(0, 0, 0, "minecraft:stone", None);
    storage.set_block(9, 9, 9, "minecraft:stone", None);

    let config = MesherConfig {
        origin_centered: true,
        coordinate_system: CoordinateSystem::ZUpRightHanded,
        ..Default::default()
    };

    let cloud = storage.to_point_cloud(&config);
    assert_eq!(cloud.len(), 2);
    assert!(cloud.bounds.is_some());
    let bounds = cloud.bounds.unwrap();
    assert_eq!(bounds, [0, 0, 0, 10, 10, 10]);

    // For block (0, 0, 0): center in MC is (0.5, 0.5, 0.5)
    // Bounds center: cx = 5.0, by = 0.0, cz = 5.0
    // Centered MC: (0.5 - 5.0, 0.5 - 0.0, 0.5 - 5.0) = (-4.5, 0.5, -4.5)
    // Z-Up (x, -z, y): (-4.5, 4.5, 0.5)
    for i in 0..cloud.len() {
        if cloud.block_x[i] == 0 && cloud.block_y[i] == 0 && cloud.block_z[i] == 0 {
            let p_off = i * 3;
            assert!((cloud.positions[p_off] - (-4.5)).abs() < 1e-5);
            assert!((cloud.positions[p_off + 1] - 4.5).abs() < 1e-5);
            assert!((cloud.positions[p_off + 2] - 0.5).abs() < 1e-5);
        }
    }

    // Roundtrip back to storage should preserve exact bounds
    let restored = cloud.reconstruct_storage();
    assert_eq!(restored.get_bounds(), (0, 0, 0, 10, 10, 10));
}

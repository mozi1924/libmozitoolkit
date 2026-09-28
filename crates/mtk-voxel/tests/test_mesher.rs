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
    assert_eq!(mesh.vertex_count(), 12); // Welded: 12 vertices

    let unwelded_config = MesherConfig {
        weld_vertices: false,
        ..Default::default()
    };
    let unwelded_mesh = SectionMesher::mesh_section(&padded, &culler, |_| None, &unwelded_config);
    assert_eq!(unwelded_mesh.vertex_count(), 40);
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

#[test]
fn test_mesher_custom_attributes() {
    let mut world = VoxelStorage::new();
    world.set_bounds(0, 0, 0, 16, 16, 16);
    world.set_block(0, 0, 0, "minecraft:stone", None);

    let padded = world.get_section_padded_array(IVec3::new(0, 0, 0));
    let culler = FaceCuller::default();
    let config = MesherConfig::default();

    let mesh = SectionMesher::mesh_section(&padded, &culler, |_| None, &config);
    assert_eq!(mesh.face_count(), 6);

    let expected_attrs = [
        "mtk_source_texture_key",
        "mtk_material_slot",
        "mtk_atlas_chunk_id",
        "mtk_atlas_texture_id",
        "mtk_uv_tiling_transform",
        "mtk_uv_transform",
        "mtk_uv_rotation",
        "mtk_uv_mode",
        "mtk_biome_tint_data",
        "mtk_biome_tint_color",
        "mtk_colormap_uv",
        "mtk_block_x",
        "mtk_block_y",
        "mtk_block_z",
        "mtk_face_dir",
    ];

    for attr_name in &expected_attrs {
        assert!(
            mesh.has_custom_attribute(attr_name),
            "Mesh should contain custom attribute: {}",
            attr_name
        );
    }
}

#[test]
fn test_grass_block_meshing_no_duplicate_overlay_faces() {
    use std::collections::HashMap;
    use std::sync::Arc;
    use mtk_core::direction::Direction;
    use mtk_model::baker::{BakedElement, BakedFace, BakedModel};

    let mut world = VoxelStorage::new();
    world.set_bounds(0, 0, 0, 16, 16, 16);
    world.set_block(0, 0, 0, "minecraft:grass_block[snowy=false]", None);

    let padded = world.get_section_padded_array(IVec3::new(0, 0, 0));
    let culler = FaceCuller::default();

    // Construct a simulated grass_block BakedModel with 2 elements:
    // Element 0: 6 canonical faces (down=dirt, up=grass_block_top, 4x grass_block_side)
    // Element 1: 4 overlay decal faces (4x grass_block_side_overlay)
    let mut el0_faces = HashMap::new();
    for dir in Direction::ALL {
        let tex = match dir {
            Direction::Down => "minecraft:block/dirt",
            Direction::Up => "minecraft:block/grass_block_top",
            _ => "minecraft:block/grass_block_side",
        };
        el0_faces.insert(
            dir,
            BakedFace {
                direction: dir,
                texture: tex.to_string(),
                tint_index: if dir == Direction::Up { 0 } else { -1 },
                cullface: Some(dir),
                vertices: [glam::Vec3::ZERO; 4],
                uvs: [glam::Vec2::ZERO; 4],
                ..Default::default()
            },
        );
    }

    let mut el1_faces = HashMap::new();
    for dir in [Direction::North, Direction::South, Direction::West, Direction::East] {
        el1_faces.insert(
            dir,
            BakedFace {
                direction: dir,
                texture: "minecraft:block/grass_block_side_overlay".to_string(),
                tint_index: 0,
                cullface: Some(dir),
                vertices: [glam::Vec3::ZERO; 4],
                uvs: [glam::Vec2::ZERO; 4],
                ..Default::default()
            },
        );
    }

    let grass_model = Arc::new(BakedModel {
        block_state: "minecraft:grass_block[snowy=false]".to_string(),
        elements: vec![
            BakedElement {
                from_pos: [0.0, 0.0, 0.0],
                to_pos: [16.0, 16.0, 16.0],
                faces: el0_faces,
            },
            BakedElement {
                from_pos: [0.0, 0.0, 0.0],
                to_pos: [16.0, 16.0, 16.0],
                faces: el1_faces,
            },
        ],
        obj_faces: Vec::new(),
        faces: std::array::from_fn(|_| BakedFace::default()),
        is_cube: true,
        is_opaque: true,
        is_emissive: false,
        emissive_level: 0.0,
    });

    let config = MesherConfig {
        weld_vertices: false,
        ..Default::default()
    };

    let mesh = SectionMesher::mesh_section(
        &padded,
        &culler,
        |name| if name.contains("grass_block") { Some(grass_model.clone()) } else { None },
        &config,
    );

    // Exactly 6 cube faces should be emitted; the 4 overlay decal faces must be skipped!
    assert_eq!(mesh.face_count(), 6, "Must emit exactly 6 faces, not 10 faces!");
    assert_eq!(mesh.triangle_count(), 12);
    assert_eq!(mesh.vertex_count(), 24);
}


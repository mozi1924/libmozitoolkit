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

    let results =
        SectionMesher::mesh_sections_parallel(&[p0, p1], &culler, |_| None, &config, Some(2))
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
    use mtk_core::direction::Direction;
    use mtk_model::baker::{BakedElement, BakedFace, BakedModel};
    use std::collections::HashMap;
    use std::sync::Arc;

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
    for dir in [
        Direction::North,
        Direction::South,
        Direction::West,
        Direction::East,
    ] {
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
        cull_meta: None,
        culled_faces: Default::default(),
        unculled_faces: Default::default(),
    });

    let config = MesherConfig {
        weld_vertices: false,
        ..Default::default()
    };

    let mesh = SectionMesher::mesh_section(
        &padded,
        &culler,
        |name| {
            if name.contains("grass_block") {
                Some(grass_model.clone())
            } else {
                None
            }
        },
        &config,
    );

    // Exactly 6 cube faces should be emitted; the 4 overlay decal faces must be skipped!
    assert_eq!(
        mesh.face_count(),
        6,
        "Must emit exactly 6 faces, not 10 faces!"
    );
    assert_eq!(mesh.triangle_count(), 12);
    assert_eq!(mesh.vertex_count(), 24);

    // Verify tint data attributes
    let tint_data_attr = mesh
        .get_custom_attribute("mtk_biome_tint_data")
        .expect("Must have mtk_biome_tint_data");
    let tint_data = match &tint_data_attr.data {
        mtk_core::attributes::AttributeData::Float4(v) => v,
        _ => panic!("Expected Float4"),
    };
    assert_eq!(tint_data.len(), 6);

    let dir_attr = mesh
        .get_custom_attribute("mtk_face_dir")
        .expect("Must have mtk_face_dir");
    let dirs = match &dir_attr.data {
        mtk_core::attributes::AttributeData::UInt8(v) => v,
        _ => panic!("Expected UInt8"),
    };

    for i in 0..6 {
        let dir = dirs[i];
        let td = tint_data[i];
        if dir == Direction::Up.to_index() as u8 {
            // Up face: top grass, tinted
            assert_eq!(td[0], 1.0, "Top face base_weight must be 1.0");
            assert_eq!(td[1], 1.0, "Top face overlay_weight must be 1.0");
            assert_eq!(td[2], 1.0, "Top face tint_weight must be 1.0");
            assert_eq!(td[3], 1.0, "Top face tint_type must be 1 (GRASS)");
        } else if dir == Direction::Down.to_index() as u8 {
            // Down face: dirt, untinted
            assert_eq!(
                td[2], 0.0,
                "Dirt bottom face tint_weight must be 0.0 (untinted!)"
            );
        } else {
            // Side faces: grass_block_side, base untinted, overlay tinted
            assert_eq!(
                td[0], 0.0,
                "Side face base_weight must be 0.0 (dirt base untinted)"
            );
            assert_eq!(
                td[1], 1.0,
                "Side face overlay_weight must be 1.0 (grass overlay tinted)"
            );
            assert_eq!(
                td[2], 1.0,
                "Side face tint_weight must be 1.0 (tint enabled)"
            );
            assert_eq!(td[3], 1.0, "Side face tint_type must be 1 (GRASS)");
        }
    }
}

#[test]
fn test_biome_transition_smoothing() {
    let mut world = VoxelStorage::new();
    world.set_bounds(0, 0, 0, 16, 16, 16);
    // Left half (x in 0..8): plains, Right half (x in 8..16): desert
    for x in 0..16 {
        for z in 0..16 {
            let biome = if x < 8 {
                "minecraft:plains"
            } else {
                "minecraft:desert"
            };
            world.set_block(x, 0, z, "minecraft:grass_block", Some(biome));
        }
    }

    let padded = world.get_section_padded_array(IVec3::new(0, 0, 0));
    assert!(
        padded.biome_data.is_some(),
        "Padded array must contain smoothed biome data"
    );

    let biome_cols = padded.biome_data.as_ref().unwrap();
    assert_eq!(biome_cols.len(), 256);

    let plains_pal = mtk_material::get_biome_palette("plains");
    let desert_pal = mtk_material::get_biome_palette("desert");

    // Deep in plains (x = 0, z = 8)
    let col_plains = &biome_cols[8];
    // Deep in desert (x = 15, z = 8)
    let col_desert = &biome_cols[15 * 16 + 8];
    // On the transition boundary (x = 7, z = 8 and x = 8, z = 8)
    let col_trans_7 = &biome_cols[7 * 16 + 8];
    let col_trans_8 = &biome_cols[8 * 16 + 8];

    // Colormap UVs must smoothly transition
    let _plains_uv = plains_pal.colormap_uv();
    let _desert_uv = desert_pal.colormap_uv();

    assert!(col_plains.colormap_uv[0] > col_desert.colormap_uv[0]);
    assert!(col_trans_7.colormap_uv[0] <= col_plains.colormap_uv[0] + 1e-4);
    assert!(col_trans_7.colormap_uv[0] >= col_trans_8.colormap_uv[0] - 1e-4);
    assert!(col_trans_8.colormap_uv[0] >= col_desert.colormap_uv[0] - 1e-4);

    // Mesh the section and verify mtk_colormap_uv and mtk_biome_tint_color
    let culler = FaceCuller::default();
    let config = MesherConfig::default();
    let mesh = SectionMesher::mesh_section(&padded, &culler, |_| None, &config);

    let cm_attr = mesh
        .get_custom_attribute("mtk_colormap_uv")
        .expect("Must have mtk_colormap_uv");
    let cm_uvs = match &cm_attr.data {
        mtk_core::attributes::AttributeData::Float3(v) => v,
        _ => panic!("Expected Float3"),
    };
    assert!(!cm_uvs.is_empty());

    let color_attr = mesh
        .get_custom_attribute("mtk_biome_tint_color")
        .expect("Must have mtk_biome_tint_color");
    let colors = match &color_attr.data {
        mtk_core::attributes::AttributeData::Float4(v) => v,
        _ => panic!("Expected Float4"),
    };
    assert!(!colors.is_empty());

    // Verify there are multiple distinct intermediate values in mtk_colormap_uv across the transition!
    let mut distinct_u = std::collections::HashSet::new();
    for uv in cm_uvs {
        distinct_u.insert((uv[0] * 100.0).round() as i32);
    }
    assert!(
        distinct_u.len() > 2,
        "Transition zone must produce continuous blended gradient values, not a binary step!"
    );
}

#[test]
fn test_selection_boundary_no_color_bleeding() {
    let mut world = VoxelStorage::new();
    // Selection with non-zero world coordinates
    let min_x = 100;
    let min_y = 64;
    let min_z = 200;
    let size_x = 16;
    let size_y = 16;
    let size_z = 16;

    world.set_bounds(min_x, min_y, min_z, size_x, size_y, size_z);

    // Populate every block in the selection with desert biome
    for x in 0..16 {
        for z in 0..16 {
            world.set_block(
                min_x + x,
                min_y,
                min_z + z,
                "minecraft:grass_block",
                Some("minecraft:desert"),
            );
        }
    }

    let sec_coord = IVec3::new(min_x >> 4, min_y >> 4, min_z >> 4);
    let padded = world.get_section_padded_array(sec_coord);
    assert!(padded.biome_data.is_some());

    let biome_cols = padded.biome_data.as_ref().unwrap();
    let desert_pal = mtk_material::get_biome_palette("desert");
    let desert_uv = desert_pal.colormap_uv();
    let desert_grass = desert_pal.grass_linear();

    // Verify all 256 columns, especially boundary columns, have ZERO bleeding into default Plains
    for lx in 0..16 {
        for lz in 0..16 {
            let col = &biome_cols[lx * 16 + lz];
            assert!(
                (col.colormap_uv[0] - desert_uv[0]).abs() < 1e-4,
                "Col ({}, {}) colormap_uv[0] was {}, expected desert {}",
                lx,
                lz,
                col.colormap_uv[0],
                desert_uv[0]
            );
            assert!(
                (col.colormap_uv[1] - desert_uv[1]).abs() < 1e-4,
                "Col ({}, {}) colormap_uv[1] was {}, expected desert {}",
                lx,
                lz,
                col.colormap_uv[1],
                desert_uv[1]
            );
            assert!(
                (col.grass_color[0] - desert_grass[0]).abs() < 1e-4,
                "Col ({}, {}) grass_color was contaminated by bleeding",
                lx,
                lz
            );
        }
    }
}

#[test]
fn test_waterlogged_isolated_block_meshing() {
    let mut world = VoxelStorage::new();
    world.set_bounds(0, 0, 0, 16, 16, 16);

    // An isolated kelp block in air at (2, 2, 2) (canonical vanilla state without explicit waterlogged)
    world.set_block(2, 2, 2, "minecraft:kelp[age=0]", None);
    // An isolated waterlogged slab in air at (5, 5, 5)
    world.set_block(
        5,
        5,
        5,
        "minecraft:oak_slab[type=bottom,waterlogged=true]",
        None,
    );

    let padded = world.get_section_padded_array(IVec3::new(0, 0, 0));
    let culler = FaceCuller::default();
    let config = MesherConfig::default();

    let mesh = SectionMesher::mesh_section(&padded, &culler, |_| None, &config);

    // Both isolated kelp and isolated waterlogged slab MUST emit water geometry
    // into the mesh even without pre-baked element models!
    // Specifically, water top, bottom, and side faces should be emitted in open air.
    assert!(mesh.face_count() > 0);
    // 2 isolated waterlogged blocks in air with fallback unit cube:
    // each emits 6 fluid faces + 6 solid block faces = 12 faces per block = 24 total faces!
    assert_eq!(mesh.face_count(), 24);
}

#[test]
fn test_waterlogged_isolated_block_with_baked_model() {
    let mut world = VoxelStorage::new();
    world.set_bounds(0, 0, 0, 16, 16, 16);

    // An isolated waterlogged slab in air at (5, 5, 5)
    world.set_block(
        5,
        5,
        5,
        "minecraft:oak_slab[type=bottom,waterlogged=true]",
        None,
    );

    let padded = world.get_section_padded_array(IVec3::new(0, 0, 0));
    let culler = FaceCuller::default();
    let config = MesherConfig::default();

    // Create a mock baked model for the slab (without waterlogged in its block_state)
    let face = mtk_model::baker::BakedFace {
        direction: mtk_core::Direction::Up,
        vertices: [
            glam::Vec3::new(0.0, 0.5, 0.0),
            glam::Vec3::new(0.0, 0.5, 1.0),
            glam::Vec3::new(1.0, 0.5, 1.0),
            glam::Vec3::new(1.0, 0.5, 0.0),
        ],
        texture: "minecraft:block/oak_planks".to_string(),
        ..Default::default()
    };
    let mut faces_map = std::collections::HashMap::new();
    faces_map.insert(mtk_core::Direction::Up, face);
    let mut slab_model = mtk_model::baker::BakedModel {
        block_state: "minecraft:oak_slab[type=bottom]".to_string(),
        elements: vec![mtk_model::baker::BakedElement {
            from_pos: [0.0, 0.0, 0.0],
            to_pos: [16.0, 8.0, 16.0],
            faces: faces_map,
        }],
        obj_faces: Vec::new(),
        faces: std::array::from_fn(|_| mtk_model::baker::BakedFace::default()),
        is_cube: false,
        is_opaque: false,
        is_emissive: false,
        emissive_level: 0.0,
        cull_meta: None,
        culled_faces: Default::default(),
        unculled_faces: Vec::new(),
    };
    slab_model.rebuild_face_buckets();
    let slab_arc = std::sync::Arc::new(slab_model);

    let mesh = SectionMesher::mesh_section(
        &padded,
        &culler,
        |state| {
            if state.contains("oak_slab") {
                Some(slab_arc.clone())
            } else {
                None
            }
        },
        &config,
    );

    // 1 solid model face + 6 fluid faces (top, bottom, 4 sides) = 7 faces!
    assert_eq!(mesh.face_count(), 7);
}

#[test]
fn test_alternate_blocks_variant_sampling() {
    use mtk_model::baked::{BakedElement, BakedFace, BakedModel, BakedVariantGroup};
    use mtk_voxel::mesher::ModelSource;
    use std::sync::Arc;

    let mut world = VoxelStorage::new();
    world.set_bounds(0, 0, 0, 16, 16, 16);

    // Place 16 dirt blocks along a row
    for x in 0..16 {
        world.set_block(x, 0, 0, "minecraft:dirt", None);
    }

    // Create 4 distinct variants of a unit cube model, distinguished by texture
    let mut variants = Vec::new();
    for i in 0..4 {
        let mut faces_map = std::collections::HashMap::new();
        faces_map.insert(
            mtk_core::Direction::Up,
            BakedFace {
                direction: mtk_core::Direction::Up,
                cullface: Some(mtk_core::Direction::Up),
                texture: format!("minecraft:block/dirt_var_{}", i),
                vertices: [
                    glam::Vec3::new(0.0, 1.0, 0.0),
                    glam::Vec3::new(0.0, 1.0, 1.0),
                    glam::Vec3::new(1.0, 1.0, 1.0),
                    glam::Vec3::new(1.0, 1.0, 0.0),
                ],
                uvs: [
                    glam::Vec2::new(0.0, 0.0),
                    glam::Vec2::new(0.0, 1.0),
                    glam::Vec2::new(1.0, 1.0),
                    glam::Vec2::new(1.0, 0.0),
                ],
                ..Default::default()
            },
        );
        let mut m = BakedModel {
            block_state: "minecraft:dirt".to_string(),
            elements: vec![BakedElement {
                from_pos: [0.0, 0.0, 0.0],
                to_pos: [16.0, 16.0, 16.0],
                faces: faces_map,
            }],
            obj_faces: Vec::new(),
            faces: std::array::from_fn(|_| BakedFace::default()),
            is_cube: true,
            is_opaque: true,
            is_emissive: false,
            emissive_level: 0.0,
            cull_meta: None,
            culled_faces: Default::default(),
            unculled_faces: Vec::new(),
        };
        m.rebuild_face_buckets();
        variants.push(m);
    }

    let group = Arc::new(BakedVariantGroup::new(
        "minecraft:dirt".to_string(),
        variants,
        vec![1, 1, 1, 1],
    ));

    let padded = world.get_section_padded_array(IVec3::new(0, 0, 0));
    let culler = FaceCuller::default();
    let config = MesherConfig {
        origin_centered: false,
        weld_vertices: false,
        enable_alternate_blocks: true,
        ..Default::default()
    };

    let group_clone = group.clone();
    let mesh = SectionMesher::mesh_section_with_source(
        &padded,
        &culler,
        move |_| ModelSource::Variant(group_clone.clone()),
        &config,
    );

    let tex_attr = mesh
        .get_custom_attribute("mtk_source_texture_key")
        .expect("Must have mtk_source_texture_key");
    let sampled_textures: std::collections::HashSet<String> = match &tex_attr.data {
        mtk_core::attributes::AttributeData::String(v) => v.iter().cloned().collect(),
        _ => panic!("Expected String attribute"),
    };

    // Out of 16 blocks, multiple variants must have been sampled (not just 1 static variant!)
    assert!(
        sampled_textures.len() > 1,
        "Expected multiple variants to be sampled across 16 blocks, got {:?}",
        sampled_textures
    );

    // When enable_alternate_blocks is FALSE, only primary variant (dirt_var_0) must be sampled
    let disabled_config = MesherConfig {
        origin_centered: false,
        weld_vertices: false,
        enable_alternate_blocks: false,
        ..Default::default()
    };
    let group_clone2 = group.clone();
    let mesh_disabled = SectionMesher::mesh_section_with_source(
        &padded,
        &culler,
        move |_| ModelSource::Variant(group_clone2.clone()),
        &disabled_config,
    );
    let disabled_attr = mesh_disabled
        .get_custom_attribute("mtk_source_texture_key")
        .expect("Must have mtk_source_texture_key");
    let disabled_textures: std::collections::HashSet<String> = match &disabled_attr.data {
        mtk_core::attributes::AttributeData::String(v) => v.iter().cloned().collect(),
        _ => panic!("Expected String attribute"),
    };
    assert_eq!(disabled_textures.len(), 1);
    assert!(disabled_textures.contains("minecraft:block/dirt_var_0"));
}

#[test]
fn test_plant_position_offsets() {
    let mut world = VoxelStorage::new();
    world.set_bounds(0, 0, 0, 16, 16, 16);

    // Place a flower at (3, 5, 7) and a vertical duplicate at (3, 6, 7)
    world.set_block(3, 5, 7, "minecraft:poppy", None);
    world.set_block(3, 6, 7, "minecraft:poppy", None);
    world.set_block(8, 5, 2, "minecraft:poppy", None);

    let padded = world.get_section_padded_array(IVec3::new(0, 0, 0));
    let culler = FaceCuller::default();

    // 1. With plant offsets enabled
    let config_enabled = MesherConfig {
        origin_centered: false,
        weld_vertices: false,
        enable_random_offsets: true,
        ..Default::default()
    };
    let mesh_enabled = SectionMesher::mesh_section(&padded, &culler, |_| None, &config_enabled);

    // Poppy at (3, 5, 7) and (3, 6, 7):
    let offset_3_7 = mtk_core::random::get_block_offset(mtk_core::random::OffsetType::XZ, 3, 5, 7);
    assert!(offset_3_7.x.abs() > 0.001 || offset_3_7.z.abs() > 0.001);
    assert!(offset_3_7.x >= -0.25 && offset_3_7.x <= 0.25);
    assert!(offset_3_7.z >= -0.25 && offset_3_7.z <= 0.25);
    assert_eq!(offset_3_7.y, 0.0);

    // Check that vertices at (3, 5, 7) are shifted by offset_3_7
    let mut found_shifted = false;
    for pos in &mesh_enabled.positions {
        if (pos[1] - 5.0).abs() < 1.01 {
            let frac_x = pos[0] - 3.0;
            if (frac_x - (0.0 + offset_3_7.x)).abs() < 1e-4
                || (frac_x - (1.0 + offset_3_7.x)).abs() < 1e-4
            {
                found_shifted = true;
                break;
            }
        }
    }
    assert!(
        found_shifted,
        "Plant vertices should be shifted by offset_3_7"
    );

    // 2. With plant offsets disabled
    let config_disabled = MesherConfig {
        origin_centered: false,
        weld_vertices: false,
        enable_random_offsets: false,
        ..Default::default()
    };
    let mesh_disabled = SectionMesher::mesh_section(&padded, &culler, |_| None, &config_disabled);

    // All vertex fractional coordinates must be exact integers (0.0 or 1.0 relative to block pos)
    for pos in &mesh_disabled.positions {
        let fx = (pos[0] - pos[0].round()).abs();
        let fz = (pos[2] - pos[2].round()).abs();
        assert!(
            fx < 1e-4,
            "Expected integer X coordinates without offset, got {}",
            pos[0]
        );
        assert!(
            fz < 1e-4,
            "Expected integer Z coordinates without offset, got {}",
            pos[2]
        );
    }
}

#[test]
fn test_submerged_seagrass_and_kelp_seamless_meshing() {
    let mut world = VoxelStorage::new();
    world.set_bounds(0, 0, 0, 16, 16, 16);

    // Seabed: dirt at (1, 0, 1)
    world.set_block(1, 0, 1, "minecraft:dirt", None);
    // Seagrass at (1, 1, 1) (vanilla state string without waterlogged=true)
    world.set_block(1, 1, 1, "minecraft:seagrass", None);
    // Water above seagrass at (1, 2, 1)
    world.set_block(1, 2, 1, "minecraft:water[level=0]", None);
    // Water adjacent to seagrass at (1, 1, 2)
    world.set_block(1, 1, 2, "minecraft:water[level=0]", None);

    let padded = world.get_section_padded_array(IVec3::new(0, 0, 0));
    let culler = FaceCuller::default();
    let config = MesherConfig {
        origin_centered: false,
        weld_vertices: false,
        mesh_fluids: true,
        ..Default::default()
    };

    let mesh = SectionMesher::mesh_section(&padded, &culler, |_| None, &config);

    // Seagrass at (1, 1, 1) must emit water geometry merged with water above and adjacent.
    // The mutual face between seagrass (1, 1, 1) and water above (1, 2, 1) must be CULLED.
    // The mutual face between seagrass (1, 1, 1) and water adjacent (1, 1, 2) must be CULLED.
    assert!(mesh.face_count() > 0);

    // Verify there is no horizontal water face at y=2.0 between seagrass and water above
    for (i, norm) in mesh.normals.iter().enumerate().step_by(4) {
        let p0 = mesh.positions[mesh.indices[i / 4 * 6] as usize];
        // If normal is (0, 1, 0) or (0, -1, 0) and at x=1, z=1
        if (p0[0] - 1.0).abs() < 0.1 && (p0[2] - 1.0).abs() < 0.1 {
            // Internal horizontal boundary at y=2.0 between (1,1,1) and (1,2,1) MUST NOT exist
            assert!(
                !((p0[1] - 2.0).abs() < 1e-3 && norm[1].abs() > 0.9),
                "Internal dividing water face at y=2.0 between seagrass and water above must be culled!"
            );
        }
    }
}

use mtk_voxel::fluid::{
    calculate_corner_average, calculate_fluid_corner_heights, get_fluid_base_height, FluidType,
    MAX_FLUID_HEIGHT,
};

#[test]
fn test_fluid_base_heights() {
    assert_eq!(get_fluid_base_height("minecraft:water"), MAX_FLUID_HEIGHT);
    assert_eq!(
        get_fluid_base_height("minecraft:water[level=0]"),
        MAX_FLUID_HEIGHT
    );
    assert_eq!(
        get_fluid_base_height("minecraft:flowing_water[level=1]"),
        7.0 / 9.0
    );
    assert_eq!(
        get_fluid_base_height("minecraft:flowing_water[level=7]"),
        1.0 / 9.0
    );
    assert_eq!(
        get_fluid_base_height("minecraft:oak_stairs[waterlogged=true]"),
        MAX_FLUID_HEIGHT
    );
}

#[test]
fn test_still_source_surface_tension() {
    // A still source water surrounded by air should stay at MAX_FLUID_HEIGHT (8/9) without drooping
    let corner_h = calculate_corner_average(MAX_FLUID_HEIGHT, 0.0, 0.0, 0.0, true);
    assert_eq!(corner_h, MAX_FLUID_HEIGHT);
}

#[test]
fn test_fluid_corner_height_calculation() {
    let get_state = |x: i32, y: i32, z: i32| -> String {
        if x == 0 && y == 64 && z == 0 {
            "minecraft:water".to_string()
        } else {
            "minecraft:air".to_string()
        }
    };

    let (c_nw, c_ne, c_se, c_sw) =
        calculate_fluid_corner_heights(get_state, 0, 64, 0, FluidType::Water);

    assert_eq!(c_nw, MAX_FLUID_HEIGHT);
    assert_eq!(c_ne, MAX_FLUID_HEIGHT);
    assert_eq!(c_se, MAX_FLUID_HEIGHT);
    assert_eq!(c_sw, MAX_FLUID_HEIGHT);
}

#[test]
fn test_fluid_side_and_top_texture_keys_with_atlas() {
    use glam::IVec3;
    use mtk_core::Direction;
    use mtk_cull::FaceCuller;
    use mtk_resource::ResourceLocation;
    use mtk_texture::{AtlasAddressMap, AtlasSpriteLocation};
    use mtk_voxel::storage::VoxelStorage;
    use mtk_voxel::types::MesherConfig;
    use mtk_voxel::SectionMesher;

    let mut address_map = AtlasAddressMap::new();
    address_map.sprites.insert(
        ResourceLocation::new("minecraft", "block/water_still"),
        AtlasSpriteLocation {
            chunk_id: 1,
            category: "blocks".to_string(),
            texture_id: 201,
            uv_bounds: [0.0, 0.0, 0.5, 0.5],
            frame_0_uv_bounds: [0.0, 0.0, 0.5, 0.5],
            ..Default::default()
        },
    );
    address_map.sprites.insert(
        ResourceLocation::new("minecraft", "block/water_flow"),
        AtlasSpriteLocation {
            chunk_id: 1,
            category: "blocks".to_string(),
            texture_id: 202,
            uv_bounds: [0.5, 0.0, 1.0, 1.0],
            frame_0_uv_bounds: [0.5, 0.0, 1.0, 1.0],
            ..Default::default()
        },
    );

    let config = MesherConfig {
        atlas_address_map: Some(std::sync::Arc::new(address_map)),
        enable_ao: false,
        ..Default::default()
    };

    let culler = FaceCuller::default();

    // 1. Still water isolated block:
    // Top face should be water_still (id 201).
    // Bottom face should be water_still (id 201).
    // North, South, West, East side faces should ALL be water_flow (id 202)!
    let mut still_world = VoxelStorage::new();
    still_world.set_bounds(0, 0, 0, 16, 16, 16);
    still_world.set_block(4, 4, 4, "minecraft:water[level=0]", None);

    let padded_still = still_world.get_section_padded_array(IVec3::new(0, 0, 0));
    let still_mesh = SectionMesher::mesh_section(&padded_still, &culler, |_| None, &config);

    assert_eq!(still_mesh.face_count(), 6);

    let keys_attr = still_mesh
        .get_custom_attribute("mtk_source_texture_key")
        .expect("Must have texture keys");
    let keys = match &keys_attr.data {
        mtk_core::attributes::AttributeData::String(s) => s,
        _ => panic!("Expected String attribute"),
    };
    let dirs_attr = still_mesh
        .get_custom_attribute("mtk_face_dir")
        .expect("Must have face dirs");
    let dirs = match &dirs_attr.data {
        mtk_core::attributes::AttributeData::UInt8(d) => d,
        _ => panic!("Expected UInt8 attribute"),
    };

    for i in 0..6 {
        let dir = dirs[i];
        let key = &keys[i];
        if dir == Direction::Up.to_index() as u8 || dir == Direction::Down.to_index() as u8 {
            assert_eq!(
                key, "minecraft:block/water_still",
                "Up/Down face of still water must use water_still"
            );
        } else {
            assert_eq!(
                key, "minecraft:block/water_flow",
                "Side face of fluid must ALWAYS use water_flow"
            );
        }
    }

    // 2. Flowing water block:
    // Top face should also be water_flow!
    let mut flow_world = VoxelStorage::new();
    flow_world.set_bounds(0, 0, 0, 16, 16, 16);
    flow_world.set_block(4, 4, 4, "minecraft:water[level=1]", None);

    let padded_flow = flow_world.get_section_padded_array(IVec3::new(0, 0, 0));
    let flow_mesh = SectionMesher::mesh_section(&padded_flow, &culler, |_| None, &config);

    assert_eq!(flow_mesh.face_count(), 6);

    let flow_keys_attr = flow_mesh
        .get_custom_attribute("mtk_source_texture_key")
        .expect("Must have texture keys");
    let flow_keys = match &flow_keys_attr.data {
        mtk_core::attributes::AttributeData::String(s) => s,
        _ => panic!("Expected String attribute"),
    };
    let flow_dirs_attr = flow_mesh
        .get_custom_attribute("mtk_face_dir")
        .expect("Must have face dirs");
    let flow_dirs = match &flow_dirs_attr.data {
        mtk_core::attributes::AttributeData::UInt8(d) => d,
        _ => panic!("Expected UInt8 attribute"),
    };

    for i in 0..6 {
        let dir = flow_dirs[i];
        let key = &flow_keys[i];
        if dir == Direction::Down.to_index() as u8 {
            assert_eq!(
                key, "minecraft:block/water_still",
                "Down face of fluid uses water_still"
            );
        } else {
            assert_eq!(
                key, "minecraft:block/water_flow",
                "Top and side faces of flowing water must use water_flow"
            );
        }
    }
}

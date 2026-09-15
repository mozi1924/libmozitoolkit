use std::collections::HashMap;
use mtk_material::{
    clean_identifier, decode_grid_atlas_uv, remap_local_to_atlas,
    remap_mesh_multi_uvs_parallel, remap_mesh_uvs_parallel, GridAtlasSpec, MaterialResolver,
};
use mtk_resource::ResourceLocation;
use mtk_texture::{AtlasAddressMap, AtlasSpriteLocation, SpriteKind};

#[test]
fn test_grid_atlas_spec_detection() {
    let spec = GridAtlasSpec {
        atlas_name_patterns: vec!["TerrainRGB".to_string(), "terrain".to_string()],
        atlas_suffix_patterns: vec!["-rgba".to_string(), "_rgb".to_string()],
        ..Default::default()
    };

    assert!(spec.matches_atlas_name("TerrainRGB.png"));
    assert!(spec.matches_atlas_name("terrain-rgba.001.png"));
    assert!(spec.matches_atlas_name("world_rgb.png"));
    assert!(!spec.matches_atlas_name("stone.png"));
}

#[test]
fn test_grid_atlas_uv_decode() {
    let mut swatch_map = HashMap::new();
    swatch_map.insert(0, vec!["grass_block_top".to_string(), "grass_top".to_string()]);

    let spec = GridAtlasSpec {
        swatch_size: 18.0,
        tile_size: 16.0,
        border: 1.0,
        image_width: 1024,
        image_height: 1024,
        atlas_name_patterns: vec!["terrain".to_string()],
        atlas_suffix_patterns: vec![],
        swatch_to_candidates: swatch_map,
    };

    // Swatch 0 is at col 0, row 0 on 1024x1024 image
    // Center of swatch 0: x in [1..17], y in [1..17] -> pixel (9, 9)
    let u = 9.0 / 1024.0;
    let v = 1.0 - (9.0 / 1024.0);
    let (cands, local_uv) = decode_grid_atlas_uv(u, v, &spec);

    assert!(cands.is_some());
    assert_eq!(cands.unwrap()[0], "grass_block_top");
    // Local UV should be around center (0.5, 0.5)
    assert!((local_uv[0] - 0.5).abs() < 1e-2);
    assert!((local_uv[1] - 0.5).abs() < 1e-2);
}

#[test]
fn test_clean_identifier() {
    assert_eq!(clean_identifier("block/stone.png"), "stone");
    assert_eq!(clean_identifier("minecraft:stone.001"), "stone");
    assert_eq!(clean_identifier("textures/block/grass_block_top"), "grass_block_top");
}

#[test]
fn test_resolver_and_atlas_projection() {
    let mut address_map = AtlasAddressMap::new();
    let stone_loc = ResourceLocation::new("minecraft", "block/stone");

    address_map.sprites.insert(
        stone_loc.clone(),
        AtlasSpriteLocation {
            chunk_id: 0,
            category: "blocks".to_string(),
            is_animated: false,
            sprite_kind: SpriteKind::StaticAtlas,
            texture_id: 42,
            uv_bounds: [0.0, 0.0, 0.25, 0.25],
            frame_0_uv_bounds: [0.0, 0.0, 0.25, 0.25],
            local_uv_bounds: [0.0, 0.0, 1.0, 1.0],
            frame_uv_step: [0.0, 0.0],
            pixel_rect: [0, 0, 16, 16],
            strip_pixel_rect: [0, 0, 16, 16],
            frame_size: [16, 16],
            frame_count: 1,
            animation: None,
            has_normal: false,
            has_specular: false,
            has_overlay: false,
        },
    );

    let mut custom_aliases = HashMap::new();
    custom_aliases.insert("custom_rock_mat".to_string(), vec!["block/stone".to_string()]);

    let resolved = MaterialResolver::resolve(
        "custom_rock_mat",
        Some(&custom_aliases),
        &address_map,
    );
    assert!(resolved.is_some());
    let (res_id, sprite) = resolved.unwrap();
    assert_eq!(res_id.path, "block/stone");
    assert_eq!(sprite.chunk_id, 0);

    // Project local [0.5, 0.5] to atlas
    let projected = remap_local_to_atlas(0.5, 0.5, sprite);
    assert_eq!(projected, [0.125, 0.125]);
}

#[test]
fn test_parallel_batch_mesh_remap() {
    let mut address_map = AtlasAddressMap::new();
    let dirt_loc = ResourceLocation::new("minecraft", "block/dirt");

    address_map.sprites.insert(
        dirt_loc,
        AtlasSpriteLocation {
            chunk_id: 1,
            category: "blocks".to_string(),
            is_animated: false,
            sprite_kind: SpriteKind::StaticAtlas,
            texture_id: 10,
            uv_bounds: [0.5, 0.5, 1.0, 1.0],
            frame_0_uv_bounds: [0.5, 0.5, 1.0, 1.0],
            local_uv_bounds: [0.0, 0.0, 1.0, 1.0],
            frame_uv_step: [0.0, 0.0],
            pixel_rect: [32, 32, 16, 16],
            strip_pixel_rect: [32, 32, 16, 16],
            frame_size: [16, 16],
            frame_count: 1,
            animation: None,
            has_normal: false,
            has_specular: false,
            has_overlay: false,
        },
    );

    let mut custom_aliases = HashMap::new();
    custom_aliases.insert("muddy_ground".to_string(), vec!["block/dirt".to_string()]);

    // 2 quads (8 loops)
    let mut uvs = vec![
        [0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0],
        [0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0],
    ];
    let face_materials = vec!["muddy_ground".to_string(), "dirt".to_string()];
    let face_loop_ranges = vec![(0, 4), (4, 4)];

    let result = remap_mesh_uvs_parallel(
        &mut uvs,
        &face_materials,
        &face_loop_ranges,
        &address_map,
        Some(&custom_aliases),
        None,
    );

    assert_eq!(result.face_count, 2);
    assert_eq!(result.unmapped_faces, 0);
    assert_eq!(result.face_chunk_ids, vec![1, 1]);
    assert_eq!(result.face_texture_ids, vec![10, 10]);

    // First vertex [0, 0] should project to [0.5, 0.5]
    assert_eq!(uvs[0], [0.5, 0.5]);
    // Third vertex [1, 1] should project to [1.0, 1.0]
    assert_eq!(uvs[2], [1.0, 1.0]);

    // Test Multi-UV remapper
    let multi_res = remap_mesh_multi_uvs_parallel(
        &uvs,
        &face_materials,
        &face_loop_ranges,
        &address_map,
        Some(&custom_aliases),
        None,
    );
    assert_eq!(multi_res.face_count, 2);
    assert_eq!(multi_res.local_uvs.len(), 8);
    assert_eq!(multi_res.atlas_uvs.len(), 8);
    assert_eq!(multi_res.face_uv_modes, vec![0, 0]);
}

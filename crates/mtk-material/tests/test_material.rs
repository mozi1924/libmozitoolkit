use mtk_material::{
    clean_icecube_name, clean_jmc2obj_name, decode_mineways_uv, is_mineways_atlas_name,
    lookup_swatch, remap_local_to_atlas, remap_mesh_multi_uvs_parallel, remap_mesh_uvs_parallel,
    resolve_icecube_candidates, resolve_jmc2obj_candidates, ImporterOrigin, MaterialResolver,
};
use mtk_resource::ResourceLocation;
use mtk_texture::{AtlasAddressMap, AtlasSpriteLocation, SpriteKind};

#[test]
fn test_mineways_swatch_lookup() {
    assert_eq!(lookup_swatch(0), Some(("grass_block_top", "grass_top")));
    assert_eq!(lookup_swatch(1), Some(("stone", "")));
    assert_eq!(lookup_swatch(4), Some(("oak_planks", "planks_oak")));
    assert_eq!(lookup_swatch(25), Some(("chest_top", "chest_top")));
}

#[test]
fn test_mineways_atlas_detection() {
    assert!(is_mineways_atlas_name("TerrainRGB.png"));
    assert!(is_mineways_atlas_name("terrain-rgba.001.png"));
    assert!(is_mineways_atlas_name("world-RGBA.png"));
    assert!(!is_mineways_atlas_name("stone.png"));
}

#[test]
fn test_mineways_uv_decode() {
    // Swatch 0 is at col 0, row 0 on 1024x1024 image
    // Center of swatch 0: x in [1..17], y in [1..17] -> pixel (9, 9)
    // u = 9 / 1024, v = 1.0 - 9 / 1024
    let u = 9.0 / 1024.0;
    let v = 1.0 - (9.0 / 1024.0);
    let (pri, alt, local_uv) = decode_mineways_uv(u, v, 1024, 1024);

    assert_eq!(pri, Some("grass_block_top"));
    assert_eq!(alt, Some("grass_top"));
    // Local UV should be around center (0.5, 0.5)
    assert!((local_uv[0] - 0.5).abs() < 1e-2);
    assert!((local_uv[1] - 0.5).abs() < 1e-2);
}

#[test]
fn test_jmc2obj_name_cleaner() {
    assert_eq!(
        clean_jmc2obj_name("minecraft_block-stone"),
        "stone"
    );
    assert_eq!(
        clean_jmc2obj_name("minecraft_block-grass_block_top.001"),
        "grass_block_top"
    );
    assert_eq!(
        clean_jmc2obj_name("jmc2obj_block-stone-desert"),
        "stone"
    );
}

#[test]
fn test_icecube_name_cleaner() {
    assert_eq!(
        clean_icecube_name("ice_cube_block_stone"),
        "stone"
    );
    assert_eq!(
        clean_icecube_name("m_block_grass_block_side"),
        "grass_block_side"
    );
}

#[test]
fn test_candidate_generation() {
    let cands = resolve_jmc2obj_candidates("grass_block_top");
    assert!(cands.contains(&"block/grass_block_top".to_string()));

    let cands_ic = resolve_icecube_candidates("dirt");
    assert!(cands_ic.contains(&"block/dirt".to_string()));
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
        },
    );

    let resolved = MaterialResolver::resolve(
        "minecraft_block-stone",
        ImporterOrigin::Jmc2Obj,
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
        },
    );

    // 2 quads (8 loops)
    let mut uvs = vec![
        [0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0],
        [0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0],
    ];
    let face_materials = vec!["minecraft_block-dirt".to_string(), "dirt".to_string()];
    let face_loop_ranges = vec![(0, 4), (4, 4)];

    let result = remap_mesh_uvs_parallel(
        &mut uvs,
        &face_materials,
        &face_loop_ranges,
        &address_map,
        ImporterOrigin::Auto,
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
        ImporterOrigin::Auto,
        None,
    );
    assert_eq!(multi_res.face_count, 2);
    assert_eq!(multi_res.local_uvs.len(), 8);
    assert_eq!(multi_res.atlas_uvs.len(), 8);
    assert_eq!(multi_res.face_uv_modes, vec![0, 0]);
}

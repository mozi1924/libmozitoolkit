use mtk_material::biome::{
    classify_tint_category, BiomeResolver,
    TINT_TYPE_DRY_FOLIAGE, TINT_TYPE_FOLIAGE, TINT_TYPE_GRASS,
    TINT_TYPE_HARDCODED, TINT_TYPE_NONE,
};

#[test]
fn test_classify_tint_category_authoritative() {
    // 1. None (Untinted natural textures)
    assert_eq!(classify_tint_category("dead_bush", Some("dead_bush"), None), "none");
    assert_eq!(classify_tint_category("dead_bush", Some("dead_bush"), Some(0)), "none");
    assert_eq!(classify_tint_category("potted_dead_bush", Some("potted_dead_bush"), None), "none");
    assert_eq!(classify_tint_category("azalea_leaves", Some("azalea_leaves"), None), "none");
    assert_eq!(classify_tint_category("cherry_leaves", Some("cherry_leaves"), None), "none");
    assert_eq!(classify_tint_category("pale_oak_leaves", Some("pale_oak_leaves"), None), "none");

    // 2. Dry Foliage
    assert_eq!(classify_tint_category("leaf_litter", Some("leaf_litter"), Some(0)), "dry_foliage");
    assert_eq!(classify_tint_category("short_dry_grass", Some("short_dry_grass"), Some(0)), "dry_foliage");
    assert_eq!(classify_tint_category("tall_dry_grass", Some("tall_dry_grass"), Some(0)), "dry_foliage");

    // 3. Grass
    assert_eq!(classify_tint_category("short_grass", Some("short_grass"), Some(0)), "grass");
    assert_eq!(classify_tint_category("bush", Some("bush"), Some(0)), "grass");
    assert_eq!(classify_tint_category("grass_block_top", Some("grass_block"), Some(0)), "grass");
    assert_eq!(classify_tint_category("sugar_cane", Some("sugar_cane"), Some(0)), "grass");

    // 4. Foliage
    assert_eq!(classify_tint_category("oak_leaves", Some("oak_leaves"), Some(0)), "foliage");
    assert_eq!(classify_tint_category("jungle_leaves", Some("jungle_leaves"), Some(0)), "foliage");
    assert_eq!(classify_tint_category("vine", Some("vine"), Some(0)), "foliage");

    // 5. Hardcoded
    assert_eq!(classify_tint_category("spruce_leaves", Some("spruce_leaves"), Some(0)), "hardcoded");
    assert_eq!(classify_tint_category("birch_leaves", Some("birch_leaves"), Some(0)), "hardcoded");
    assert_eq!(classify_tint_category("lily_pad", Some("lily_pad"), Some(0)), "hardcoded");
}

#[test]
fn test_biome_resolver_seeded_and_tint_info() {
    let resolver = BiomeResolver::new();

    // dead_bush must be TINT_TYPE_NONE with tint_weight 0.0
    let db_info = resolver.get_tint_info("dead_bush", Some("dead_bush"), None);
    assert_eq!(db_info.tint_type, TINT_TYPE_NONE);
    assert_eq!(db_info.tint_weight, 0.0);
    assert_eq!(db_info.tint_category, "none");

    // leaf_litter must be TINT_TYPE_DRY_FOLIAGE
    let ll_info = resolver.get_tint_info("leaf_litter", Some("leaf_litter"), Some(0));
    assert_eq!(ll_info.tint_type, TINT_TYPE_DRY_FOLIAGE);
    assert_eq!(ll_info.tint_weight, 1.0);
    assert_eq!(ll_info.tint_category, "dry_foliage");

    // bush must be TINT_TYPE_GRASS
    let bush_info = resolver.get_tint_info("bush", Some("bush"), Some(0));
    assert_eq!(bush_info.tint_type, TINT_TYPE_GRASS);
    assert_eq!(bush_info.tint_weight, 1.0);
    assert_eq!(bush_info.tint_category, "grass");

    // oak_leaves must be TINT_TYPE_FOLIAGE
    let oak_info = resolver.get_tint_info("oak_leaves", Some("oak_leaves"), Some(0));
    assert_eq!(oak_info.tint_type, TINT_TYPE_FOLIAGE);
    assert_eq!(oak_info.tint_weight, 1.0);
    assert_eq!(oak_info.tint_category, "foliage");

    // spruce_leaves must be TINT_TYPE_HARDCODED
    let spruce_info = resolver.get_tint_info("spruce_leaves", Some("spruce_leaves"), Some(0));
    assert_eq!(spruce_info.tint_type, TINT_TYPE_HARDCODED);
    assert!(spruce_info.is_hardcoded);
    assert!(spruce_info.hardcoded_color.is_some());
    assert_eq!(spruce_info.hardcoded_hex.as_deref(), Some("#619961"));

    // grass_block_side has overlay
    assert_eq!(
        resolver.get_overlay_texture("grass_block_side"),
        Some("grass_block_side_overlay")
    );
}

#[test]
fn test_grass_block_dirt_untinted_and_negative_tint_index() {
    let resolver = BiomeResolver::new();

    // Bottom dirt face of grass block with tint_index = -1 must NOT be tinted
    let dirt_info = resolver.get_tint_info("dirt", Some("grass_block"), Some(-1));
    assert_eq!(dirt_info.tint_type, TINT_TYPE_NONE);
    assert_eq!(dirt_info.tint_weight, 0.0);
    assert_eq!(dirt_info.tint_category, "none");

    // Face with explicit negative tint_index is always untinted
    assert_eq!(classify_tint_category("dirt", Some("grass_block"), Some(-1)), "none");
    assert_eq!(classify_tint_category("grass_block_top", Some("grass_block"), Some(-1)), "none");
    assert_eq!(classify_tint_category("oak_leaves", Some("oak_leaves"), Some(-1)), "none");

    // Multi-layer flora: pink petals (layer 0 = petals untinted, layer 1 = stem grass)
    assert_eq!(classify_tint_category("pink_petals", Some("pink_petals"), Some(0)), "none");
    assert_eq!(classify_tint_category("pink_petals_stem", Some("pink_petals"), Some(1)), "grass");
    assert_eq!(classify_tint_category("pink_petals", Some("pink_petals"), Some(-1)), "none");
}

#[test]
fn test_unit_cube_tint_index() {
    use mtk_core::direction::Direction;

    // Grass block: Up is 0, Down (dirt) and sides are -1
    assert_eq!(mtk_material::get_unit_cube_tint_index("grass_block", Direction::Up), 0);
    assert_eq!(mtk_material::get_unit_cube_tint_index("grass_block", Direction::Down), -1);
    assert_eq!(mtk_material::get_unit_cube_tint_index("grass_block", Direction::North), -1);

    // Leaves: All directions are 0
    assert_eq!(mtk_material::get_unit_cube_tint_index("oak_leaves", Direction::Up), 0);
    assert_eq!(mtk_material::get_unit_cube_tint_index("oak_leaves", Direction::Down), 0);

    // Untinted blocks: -1
    assert_eq!(mtk_material::get_unit_cube_tint_index("dirt", Direction::Up), -1);
    assert_eq!(mtk_material::get_unit_cube_tint_index("stone", Direction::Up), -1);
    assert_eq!(mtk_material::get_unit_cube_tint_index("dead_bush", Direction::Up), -1);
}

#[test]
fn test_biome_resolver_model_inheritance() {
    use serde_json::json;
    use std::collections::HashMap;

    let mut resolver = BiomeResolver::new();
    let mut models = HashMap::new();

    // Parent model defining textures and elements
    models.insert(
        "block/parent_leaves".to_string(),
        json!({
            "textures": {
                "all": "custom_oak_leaves"
            },
            "elements": [
                {
                    "faces": {
                        "all": {
                            "texture": "#all",
                            "tintindex": 0
                        }
                    }
                }
            ]
        }),
    );

    // Child model inheriting from parent_leaves
    models.insert(
        "block/oak_leaves".to_string(),
        json!({
            "parent": "block/parent_leaves",
            "textures": {
                "all": "05m_oak_leaves_cube"
            }
        }),
    );

    resolver.set_models(models);

    // Child should inherit elements and resolve custom texture as foliage
    let info = resolver.get_tint_info("05m_oak_leaves_cube", Some("oak_leaves"), Some(0));
    assert_eq!(info.tint_type, TINT_TYPE_FOLIAGE);
    assert_eq!(info.tint_category, "foliage");
    assert_eq!(info.tint_weight, 1.0);
}

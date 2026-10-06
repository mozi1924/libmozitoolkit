use mtk_material::{
    compute_mesh_material_props, get_block_emission_strength, get_block_sticker_threshold,
    get_block_transmission_weight, is_thin_wall_block, is_transmissive_block,
    register_material_properties_json, reset_material_properties_to_default,
};

#[test]
fn test_default_properties_and_dynamic_json_override() {
    // 1. Initial state (vanilla defaults)
    reset_material_properties_to_default();

    assert_eq!(get_block_emission_strength("torch", None, None), 14.0);
    assert_eq!(get_block_emission_strength("beacon", None, None), 15.0);
    assert!(is_thin_wall_block("oak_leaves", None));
    assert!(is_transmissive_block("glass", None));
    assert_eq!(get_block_transmission_weight("glass", None), 1.0);
    assert_eq!(get_block_sticker_threshold("glass", None), 0.55);
    assert_eq!(get_block_sticker_threshold("water", None), 0.95);

    // Unknown mod block before override
    assert_eq!(
        get_block_emission_strength("my_mod:magic_crystal", None, None),
        0.0
    );
    assert!(!is_thin_wall_block("my_mod:magic_leaf", None));
    assert!(!is_transmissive_block("my_mod:magic_orb", None));

    // 2. Dynamic JSON Override / Merge
    let override_json = r#"{
        "emission": {
            "static_blocks": {
                "torch": 5.0,
                "my_mod:magic_crystal": 12.0
            }
        },
        "thin_wall": {
            "exact_blocks": ["my_mod:magic_leaf"]
        },
        "transmissive": {
            "exact_blocks": ["my_mod:magic_orb"],
            "sticker_thresholds": {
                "my_mod:magic_orb": 0.42
            }
        }
    }"#;

    register_material_properties_json(override_json).expect("valid json merge");

    // Verify overrides took effect immediately without recompilation
    assert_eq!(get_block_emission_strength("torch", None, None), 5.0);
    assert_eq!(
        get_block_emission_strength("my_mod:magic_crystal", None, None),
        12.0
    );
    assert!(is_thin_wall_block("my_mod:magic_leaf", None));
    assert!(is_transmissive_block("my_mod:magic_orb", None));
    assert_eq!(get_block_sticker_threshold("my_mod:magic_orb", None), 0.42);

    // Verify non-overridden blocks remain intact
    assert_eq!(get_block_emission_strength("beacon", None, None), 15.0);
    assert!(is_thin_wall_block("oak_leaves", None));
    assert!(is_transmissive_block("glass", None));

    // Batch evaluator verify
    let batch = compute_mesh_material_props(
        &[
            "minecraft:block/glass".to_string(),
            "my_mod:textures/magic_orb".to_string(),
        ],
        Some(&[
            "minecraft:glass".to_string(),
            "my_mod:magic_orb".to_string(),
        ]),
    );
    assert_eq!(batch.len(), 2);
    // glass: [0, 0, 1.0, 0.55]
    assert_eq!(batch[0], [0.0, 0.0, 1.0, 0.55]);
    // my_mod:magic_orb: [0, 0, 1.0, 0.42]
    assert_eq!(batch[1], [0.0, 0.0, 1.0, 0.42]);

    // 3. Reset to default
    reset_material_properties_to_default();
    assert_eq!(get_block_emission_strength("torch", None, None), 14.0);
    assert_eq!(
        get_block_emission_strength("my_mod:magic_crystal", None, None),
        0.0
    );
}

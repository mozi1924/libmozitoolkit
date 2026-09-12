use std::fs;
use libmtk::{precompile_all_assets, CacheManifest, PrecompileConfig, ResourcePackStack, MemoryPack};

#[test]
fn test_precompile_all_assets_end_to_end() {
    let mut stack = ResourcePackStack::new();
    let mut pack = MemoryPack::new("TestVanilla");

    // 1. Add sample texture (16x16 red PNG)
    let img = image::RgbaImage::from_pixel(16, 16, image::Rgba([255, 0, 0, 255]));
    let mut png_bytes = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut png_bytes);
    img.write_to(&mut cursor, image::ImageFormat::Png).unwrap();
    pack.insert("assets/minecraft/textures/block/stone.png", png_bytes);

    // 2. Add sample block model JSON
    let model_json = r##"{
        "textures": { "all": "minecraft:block/stone" },
        "elements": [
            {
                "from": [0, 0, 0],
                "to": [16, 16, 16],
                "faces": {
                    "up":    { "texture": "#all", "cullface": "up" },
                    "down":  { "texture": "#all", "cullface": "down" },
                    "north": { "texture": "#all", "cullface": "north" },
                    "south": { "texture": "#all", "cullface": "south" },
                    "west":  { "texture": "#all", "cullface": "west" },
                    "east":  { "texture": "#all", "cullface": "east" }
                }
            }
        ]
    }"##;
    pack.insert("assets/minecraft/models/block/stone.json", model_json.as_bytes().to_vec());

    // 3. Add sample blockstate JSON
    let blockstate_json = r##"{
        "variants": {
            "": { "model": "minecraft:block/stone" }
        }
    }"##;
    pack.insert("assets/minecraft/blockstates/stone.json", blockstate_json.as_bytes().to_vec());

    stack.push_pack(Box::new(pack));

    let temp_cache_dir = std::env::temp_dir().join(format!("mtk_test_cache_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_cache_dir);

    let config = PrecompileConfig {
        max_atlas_width: 1024,
        max_atlas_height: 1024,
        atlas_category: "blocks".to_string(),
        compile_atlas: true,
        compile_standalone: true,
        compile_models: true,
    };

    let result = precompile_all_assets(&stack, &temp_cache_dir, &config).unwrap();
    assert!(result.success);
    assert_eq!(result.pack_count, 1);
    assert!(result.atlas_chunks >= 1);
    assert_eq!(result.standalone_textures, 1);
    assert!(result.baked_models >= 1);

    // Verify cache files existence
    assert!(temp_cache_dir.join("atlas/atlas_mapping.json").exists());
    assert!(temp_cache_dir.join("standalone/standalone_mapping.json").exists());
    assert!(temp_cache_dir.join("models/models.bin").exists());
    assert!(temp_cache_dir.join("cache_manifest.json").exists());

    // Verify manifest
    let manifest = CacheManifest::read_from_dir(&temp_cache_dir).unwrap();
    assert_eq!(manifest.format_version, 1);
    assert_eq!(manifest.pack_count, 1);
    assert_eq!(manifest.fingerprint, stack.compute_stack_fingerprint());
    assert!(manifest.is_valid_for(&stack.compute_stack_fingerprint()));

    // Clean up
    let _ = fs::remove_dir_all(&temp_cache_dir);
}

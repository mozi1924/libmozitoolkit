use mtk_resource::{AnimationFrame, AnimationMetadata, MemoryPack, ResourceLocation, ResourcePackStack};
use mtk_texture::standalone::{
    align_standalone_channels, ChannelData, ChannelType, StandaloneBuilder, StandaloneConfig,
    STANDALONE_FORMAT_VERSION,
};
use mtk_texture::RgbaBuffer;

#[test]
fn test_standalone_multi_channel_pbr_alignment() {
    // 1. Albedo: 16x512 (32 frames of 16x16)
    let albedo_buf = RgbaBuffer::solid(16, 512, 0, 200, 200, 255);
    let albedo_meta = AnimationMetadata {
        frametime: 2,
        interpolate: true,
        width: Some(16),
        height: Some(16),
        frames: Some((0..32).map(AnimationFrame::Index).collect()),
    };

    // 2. Normal: 16x16 (1 frame static)
    let normal_buf = RgbaBuffer::solid(16, 16, 128, 128, 255, 255);

    // 3. Specular: 64x64 (1 frame static high-res)
    let spec_buf = RgbaBuffer::solid(64, 64, 255, 0, 100, 200);

    let channels = vec![
        ChannelData {
            channel_type: ChannelType::Albedo,
            buffer: albedo_buf,
            metadata: Some(albedo_meta),
        },
        ChannelData {
            channel_type: ChannelType::Normal,
            buffer: normal_buf,
            metadata: None,
        },
        ChannelData {
            channel_type: ChannelType::Specular,
            buffer: spec_buf,
            metadata: None,
        },
    ];

    let result = align_standalone_channels(channels);
    assert!(result.is_animated);
    let anim = result.animation.expect("Must have animation meta");

    assert_eq!(anim.frame_width, 16);
    assert_eq!(anim.frame_height, 16);
    assert_eq!(anim.image_width, 16);
    assert_eq!(anim.image_height, 512);
    assert_eq!(anim.total_frames, 32);
    assert_eq!(anim.frametime, 2);
    assert!(anim.interpolate);
    assert_eq!(anim.frames.len(), 32);
    assert_eq!(anim.v_scale, 16.0 / 512.0);
    assert_eq!(anim.v_offset, 1.0 - (16.0 / 512.0));

    // Channels check
    assert_eq!(result.channels.len(), 3);

    // Albedo (16x512)
    assert_eq!(result.channels[0].buffer.width, 16);
    assert_eq!(result.channels[0].buffer.height, 512);

    // Normal (aligned to 16x512)
    assert_eq!(result.channels[1].buffer.width, 16);
    assert_eq!(result.channels[1].buffer.height, 512);
    assert_eq!(
        result.channels[1].metadata.as_ref().unwrap().frametime,
        2
    );

    // Specular (aligned to 64x2048)
    assert_eq!(result.channels[2].buffer.width, 64);
    assert_eq!(result.channels[2].buffer.height, 2048);
    assert_eq!(
        result.channels[2].metadata.as_ref().unwrap().frametime,
        2
    );
}

#[test]
fn test_standalone_builder_end_to_end() {
    let mut mem_pack = MemoryPack::new("TestPack");

    // Add static texture: stone
    let stone_albedo = RgbaBuffer::solid(16, 16, 100, 100, 100, 255)
        .to_png_bytes()
        .unwrap();
    let stone_normal = RgbaBuffer::solid(16, 16, 128, 128, 255, 255)
        .to_png_bytes()
        .unwrap();
    mem_pack.insert("assets/minecraft/textures/block/stone.png", stone_albedo);
    mem_pack.insert("assets/minecraft/textures/block/stone_n.png", stone_normal);

    // Add animated texture: sea_lantern (16x64 = 4 frames of 16x16)
    let sea_albedo = RgbaBuffer::solid(16, 64, 0, 200, 200, 255)
        .to_png_bytes()
        .unwrap();
    let sea_spec = RgbaBuffer::solid(16, 16, 255, 255, 255, 255)
        .to_png_bytes()
        .unwrap();
    let sea_mcmeta = br#"{"animation": {"frametime": 3, "interpolate": true}}"#;

    mem_pack.insert("assets/minecraft/textures/block/sea_lantern.png", sea_albedo);
    mem_pack.insert("assets/minecraft/textures/block/sea_lantern_s.png", sea_spec);
    mem_pack.insert(
        "assets/minecraft/textures/block/sea_lantern.png.mcmeta",
        sea_mcmeta.to_vec(),
    );

    let mut stack = ResourcePackStack::new();
    stack.push_pack(Box::new(mem_pack));

    let temp_dir = std::env::temp_dir().join(format!("mtk_standalone_test_{:x}", std::time::SystemTime::now().elapsed().unwrap().as_nanos()));
    let builder = StandaloneBuilder::new(StandaloneConfig {
        stack_hash: Some("testhash123".to_string()),
        filter_prefix: None,
    });

    let res = builder.build_to_dir(&stack, &temp_dir).expect("Build standalone must succeed");

    assert_eq!(res.format_version, STANDALONE_FORMAT_VERSION);
    assert_eq!(res.texture_count, 2); // stone and sea_lantern
    assert!(res.mapping_path.exists());

    let mapping_str = std::fs::read_to_string(&res.mapping_path).unwrap();
    let mapping: serde_json::Value = serde_json::from_str(&mapping_str).unwrap();

    assert_eq!(mapping["format_version"], 2);
    assert_eq!(mapping["stack_hash"], "testhash123");

    // Verify fallback entry
    let fallback = &mapping["textures"]["mozi:fallback"];
    assert!(fallback.is_object());
    assert_eq!(fallback["files"]["albedo"], "textures/mtk_fallback.png");
    assert!(temp_dir.join("textures/mtk_fallback.png").exists());

    // Verify stone
    let stone = &mapping["textures"]["minecraft:block/stone"];
    assert!(stone.is_object());
    assert_eq!(stone["is_animated"], false);
    let stone_albedo_file = stone["files"]["albedo"].as_str().unwrap();
    let stone_normal_file = stone["files"]["normal"].as_str().unwrap();
    assert!(temp_dir.join(stone_albedo_file).exists());
    assert!(temp_dir.join(stone_normal_file).exists());

    // Verify sea_lantern
    let sea = &mapping["textures"]["minecraft:block/sea_lantern"];
    assert!(sea.is_object());
    assert_eq!(sea["is_animated"], true);
    assert_eq!(sea["animation"]["total_frames"], 4);
    assert_eq!(sea["animation"]["frametime"], 3);
    assert_eq!(sea["animation"]["interpolate"], true);

    let sea_albedo_file = sea["files"]["albedo"].as_str().unwrap();
    let sea_spec_file = sea["files"]["specular"].as_str().unwrap();
    assert!(temp_dir.join(sea_albedo_file).exists());
    assert!(temp_dir.join(sea_spec_file).exists());

    // Clean up
    let _ = std::fs::remove_dir_all(&temp_dir);
}

use mtk_resource::{AnimationMetadata, ResourceLocation};
use mtk_texture::{
    bake_paletted_permutation, AtlasBuilder, AtlasBuilderConfig, DecodedSprite, RgbaBuffer,
};

#[test]
fn test_paletted_permutation_baking() {
    // 1x2 palette key: Red (255, 0, 0) and Green (0, 255, 0)
    let mut key_pal = RgbaBuffer::new(2, 1);
    key_pal.set_pixel(0, 0, [255, 0, 0, 255]);
    key_pal.set_pixel(1, 0, [0, 255, 0, 255]);

    // 1x2 target permutation: Blue (0, 0, 255) and Yellow (255, 255, 0)
    let mut perm_pal = RgbaBuffer::new(2, 1);
    perm_pal.set_pixel(0, 0, [0, 0, 255, 255]);
    perm_pal.set_pixel(1, 0, [255, 255, 0, 255]);

    // Base 2x2 image with Red, Green, and White pixels
    let mut base = RgbaBuffer::new(2, 2);
    base.set_pixel(0, 0, [255, 0, 0, 255]);   // Will become Blue
    base.set_pixel(1, 0, [0, 255, 0, 255]);   // Will become Yellow
    base.set_pixel(0, 1, [255, 255, 255, 255]); // Unchanged White
    base.set_pixel(1, 1, [0, 0, 0, 0]);       // Transparent

    let baked = bake_paletted_permutation(&base, &key_pal, &perm_pal).unwrap();
    assert_eq!(baked.get_pixel(0, 0), [0, 0, 255, 255]);
    assert_eq!(baked.get_pixel(1, 0), [255, 255, 0, 255]);
    assert_eq!(baked.get_pixel(0, 1), [255, 255, 255, 255]);
    assert_eq!(baked.get_pixel(1, 1), [0, 0, 0, 0]);
}

#[test]
fn test_atlas_builder_with_pbr_and_animation() {
    let builder = AtlasBuilder::new(AtlasBuilderConfig {
        max_width: 1024,
        max_height: 1024,
        mip_level: 0,
        padding: 1,
    });

    let stone_id = ResourceLocation::parse("minecraft:block/stone").unwrap();
    let diamond_id = ResourceLocation::parse("minecraft:block/diamond_ore").unwrap();

    let stone_albedo = RgbaBuffer::solid(16, 16, 120, 120, 120, 255);
    let diamond_albedo = RgbaBuffer::solid(16, 32, 0, 200, 255, 255); // 2 frames of 16x16
    let diamond_normal = RgbaBuffer::solid(16, 16, 128, 128, 255, 255); // 1 frame static normal
    let diamond_spec = RgbaBuffer::solid(16, 16, 255, 255, 255, 255);   // 1 frame static specular

    let sprites = vec![
        DecodedSprite {
            sprite_id: stone_id.clone(),
            albedo: stone_albedo,
            normal: None,
            specular: None,
            frame_width: 16,
            frame_height: 16,
            frame_count: 1,
            metadata: None,
        },
        DecodedSprite {
            sprite_id: diamond_id.clone(),
            albedo: diamond_albedo,
            normal: Some(diamond_normal),
            specular: Some(diamond_spec),
            frame_width: 16,
            frame_height: 16,
            frame_count: 2,
            metadata: Some(AnimationMetadata {
                frametime: 2,
                interpolate: true,
                ..Default::default()
            }),
        },
    ];

    let baked = builder.build_from_sprites(sprites).unwrap();
    // Dual chunks: 1 static chunk + 1 animated chunk
    assert_eq!(baked.chunks.len(), 2);

    let static_chunk = &baked.chunks[0];
    assert!(!static_chunk.is_animated);
    assert_eq!(static_chunk.file_stem(), "blocks_chunk_001");

    let anim_chunk = &baked.chunks[1];
    assert!(anim_chunk.is_animated);
    assert_eq!(anim_chunk.file_stem(), "blocks_anim_chunk_001");
    assert!(anim_chunk.normal.is_some());
    assert!(anim_chunk.specular.is_some());

    // Check address map
    let stone_loc = baked.address_map.lookup_static(&stone_id).unwrap();
    assert!(!stone_loc.is_animated);
    assert_eq!(stone_loc.chunk_id, 0);
    assert!(!stone_loc.has_normal);
    assert!(!stone_loc.has_specular);
    assert_eq!(stone_loc.frame_size, [16, 16]);

    // Static Frame 0 lookup for diamond_ore (chunk 0)
    let diamond_static = baked.address_map.lookup_static(&diamond_id).unwrap();
    assert!(!diamond_static.is_animated);
    assert_eq!(diamond_static.chunk_id, 0);

    // Dedicated animated strip lookup for diamond_ore (chunk 1)
    let diamond_anim = baked.address_map.lookup_animated(&diamond_id).unwrap();
    assert!(diamond_anim.is_animated);
    assert_eq!(diamond_anim.chunk_id, 1);
    assert!(diamond_anim.has_normal);
    assert!(diamond_anim.has_specular);
    assert_eq!(diamond_anim.frame_count, 2);
    assert!(diamond_anim.animation.is_some());
    assert_eq!(diamond_anim.frame_size, [16, 16]);
    assert!(diamond_anim.frame_uv_step[1] > 0.0);
}

#[test]
fn test_pbr_auto_tiling_for_animated_sprite() {
    let builder = AtlasBuilder::new(AtlasBuilderConfig::default());
    let water_id = ResourceLocation::parse("minecraft:block/water_still").unwrap();

    // 4 frames of 16x16 = 16x64 albedo
    let mut water_albedo = RgbaBuffer::new(16, 64);
    for frame in 0..4 {
        for y in 0..16 {
            for x in 0..16 {
                water_albedo.set_pixel(x, frame * 16 + y, [0, 100, 200 + frame as u8 * 10, 255]);
            }
        }
    }

    // 1 frame 16x16 static normal
    let water_normal = RgbaBuffer::solid(16, 16, 128, 128, 255, 255);

    let sprites = vec![DecodedSprite {
        sprite_id: water_id.clone(),
        albedo: water_albedo,
        normal: Some(water_normal),
        specular: None,
        frame_width: 16,
        frame_height: 16,
        frame_count: 4,
        metadata: Some(AnimationMetadata {
            frametime: 1,
            ..Default::default()
        }),
    }];

    let baked = builder.build_from_sprites(sprites).unwrap();
    // 2 chunks: 1 static chunk (Frame 0) + 1 animated chunk (4-frame strip)
    assert_eq!(baked.chunks.len(), 2);
    let anim_chunk = &baked.chunks[1];
    assert!(anim_chunk.is_animated);
    assert_eq!(anim_chunk.file_stem(), "blocks_anim_chunk_001");

    let norm_buf = anim_chunk.normal.as_ref().unwrap();
    let loc = baked.address_map.lookup_animated(&water_id).unwrap();
    // Normal buffer should have 4 tiled frames at the sprite's strip location
    for frame in 0..4 {
        let sample_y = loc.pixel_rect[1] + frame * 16 + 8;
        let px = norm_buf.get_pixel(loc.pixel_rect[0] + 8, sample_y);
        assert_eq!(px, [128, 128, 255, 255], "Frame {} normal pixel mismatch", frame);
    }
}

#[test]
fn test_companion_dimension_scaling_and_alignment() {
    let builder = AtlasBuilder::new(AtlasBuilderConfig {
        max_width: 1024,
        max_height: 1024,
        mip_level: 0,
        padding: 0,
    });

    // Simulate lava_flow: 32x32 single-frame, 2 frames = 32x64 albedo
    // But companion specular is 16x16 solid emission!
    let lava_id = ResourceLocation::parse("minecraft:block/lava_flow").unwrap();
    let lava_albedo = RgbaBuffer::solid(32, 64, 255, 100, 0, 255);
    let lava_spec_16 = RgbaBuffer::solid(16, 16, 10, 20, 250, 255); // Blue channel = 250 (Emission)

    // DecodedSprite with companion auto-aligned
    let aligned_spec = lava_spec_16.align_companion_to_albedo(32, 32, 2);
    assert_eq!(aligned_spec.width, 32);
    assert_eq!(aligned_spec.height, 64);

    let sprites = vec![DecodedSprite {
        sprite_id: lava_id.clone(),
        albedo: lava_albedo,
        normal: None,
        specular: Some(aligned_spec),
        frame_width: 32,
        frame_height: 32,
        frame_count: 2,
        metadata: Some(AnimationMetadata {
            frametime: 1,
            ..Default::default()
        }),
    }];

    let baked = builder.build_from_sprites(sprites).unwrap();
    assert_eq!(baked.chunks.len(), 2);

    // 1. Static chunk (Frame 0: 32x32)
    let static_chunk = &baked.chunks[0];
    let static_spec = static_chunk.specular.as_ref().unwrap();
    let static_loc = baked.address_map.lookup_static(&lava_id).unwrap();

    // Verify all four quadrants of the 32x32 static sprite have full emission (250)
    for sample_x in [static_loc.pixel_rect[0] + 4, static_loc.pixel_rect[0] + 28] {
        for sample_y in [static_loc.pixel_rect[1] + 4, static_loc.pixel_rect[1] + 28] {
            let px = static_spec.get_pixel(sample_x, sample_y);
            assert_eq!(px, [10, 20, 250, 255], "Static frame 0 pixel at ({}, {}) was not fully covered", sample_x, sample_y);
        }
    }

    // 2. Animated chunk (Frame 0 and Frame 1: 32x64)
    let anim_chunk = &baked.chunks[1];
    let anim_spec = anim_chunk.specular.as_ref().unwrap();
    let anim_loc = baked.address_map.lookup_animated(&lava_id).unwrap();
    for frame in 0..2 {
        let sample_y = anim_loc.pixel_rect[1] + frame * 32 + 16;
        let px = anim_spec.get_pixel(anim_loc.pixel_rect[0] + 16, sample_y);
        assert_eq!(px, [10, 20, 250, 255], "Animated frame {} specular emission mismatch", frame);
    }
}

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
    let diamond_albedo = RgbaBuffer::solid(16, 16, 0, 200, 255, 255);
    let diamond_normal = RgbaBuffer::solid(16, 16, 128, 128, 255, 255);
    let diamond_spec = RgbaBuffer::solid(16, 16, 255, 255, 255, 255);

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
    assert_eq!(baked.chunks.len(), 1);

    let chunk = &baked.chunks[0];
    assert!(chunk.normal.is_some());
    assert!(chunk.specular.is_some());

    // Check address map
    let stone_loc = baked.address_map.lookup(&stone_id).unwrap();
    assert!(!stone_loc.has_normal);
    assert!(!stone_loc.has_specular);
    assert_eq!(stone_loc.frame_size, [16, 16]);

    let diamond_loc = baked.address_map.lookup(&diamond_id).unwrap();
    assert!(diamond_loc.has_normal);
    assert!(diamond_loc.has_specular);
    assert_eq!(diamond_loc.frame_count, 2);
    assert!(diamond_loc.animation.is_some());
}

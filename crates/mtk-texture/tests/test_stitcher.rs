use mtk_texture::{smallest_encompassing_power_of_two, Stitcher};

#[test]
fn test_power_of_two_helper() {
    assert_eq!(smallest_encompassing_power_of_two(0), 1);
    assert_eq!(smallest_encompassing_power_of_two(1), 1);
    assert_eq!(smallest_encompassing_power_of_two(15), 16);
    assert_eq!(smallest_encompassing_power_of_two(16), 16);
    assert_eq!(smallest_encompassing_power_of_two(17), 32);
    assert_eq!(smallest_encompassing_power_of_two(1000), 1024);
}

#[test]
fn test_mixed_resolution_stitching() {
    let mut stitcher = Stitcher::new(4096, 4096, 0);

    // Register a mix of HD and vanilla resolution sprites
    stitcher.register_sprite("stone", 16, 16, "minecraft:block/stone");
    stitcher.register_sprite("dirt", 16, 16, "minecraft:block/dirt");
    stitcher.register_sprite("hd_sword", 128, 128, "minecraft:item/diamond_sword");
    stitcher.register_sprite("hd_water", 256, 256, "minecraft:block/water_still");
    stitcher.register_sprite("huge_block", 512, 512, "minecraft:block/huge");
    stitcher.register_sprite("tall_texture", 64, 128, "minecraft:block/door");

    let result = stitcher.stitch().unwrap();
    assert_eq!(result.chunks.len(), 1);

    let chunk = &result.chunks[0];
    assert!(chunk.width.is_power_of_two());
    assert!(chunk.height.is_power_of_two());
    assert_eq!(chunk.slots.len(), 6);

    // Verify non-overlapping layout
    for i in 0..chunk.slots.len() {
        let a = &chunk.slots[i];
        assert!(a.x + a.width <= chunk.width);
        assert!(a.y + a.height <= chunk.height);

        for j in (i + 1)..chunk.slots.len() {
            let b = &chunk.slots[j];
            let overlap_x = a.x < b.x + b.width && a.x + a.width > b.x;
            let overlap_y = a.y < b.y + b.height && a.y + a.height > b.y;
            assert!(
                !(overlap_x && overlap_y),
                "Slots '{}' and '{}' overlap!",
                a.entry,
                b.entry
            );
        }
    }
}

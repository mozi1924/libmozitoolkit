use glam::IVec3;
use mtk_voxel::storage::SectionStorage;
use mtk_voxel::world::VoxelStorage;
use mtk_voxel::{crc32, get_empty_section_crc, EMPTY_SECTION_CRC};

#[test]
fn test_crc_consistency() {
    let empty_crc = get_empty_section_crc(4096);
    assert_eq!(empty_crc, EMPTY_SECTION_CRC);

    let crc_stone = crc32(b"minecraft:stone");
    assert_ne!(crc_stone, 0);
    assert_ne!(crc_stone, EMPTY_SECTION_CRC);
}

#[test]
fn test_section_palette_mutation() {
    let mut sec = SectionStorage::new(IVec3::new(0, 0, 0));
    assert!(sec.is_empty());
    assert_eq!(sec.palette.len(), 1);
    assert_eq!(sec.palette[0], "minecraft:air");

    sec.set_local(0, 0, 0, "minecraft:stone");
    sec.set_local(1, 0, 0, "minecraft:oak_planks");
    sec.set_local(2, 0, 0, "minecraft:stone"); // Reuse palette index

    assert_eq!(sec.palette.len(), 3);
    assert_eq!(sec.non_air_count, 3);
    assert_eq!(sec.get_local_state(0, 0, 0), "minecraft:stone");
    assert_eq!(sec.get_local_state(1, 0, 0), "minecraft:oak_planks");
    assert_eq!(sec.get_local_state(2, 0, 0), "minecraft:stone");

    // Clearing back to air
    sec.set_local(0, 0, 0, "minecraft:air");
    sec.set_local(1, 0, 0, "minecraft:air");
    sec.set_local(2, 0, 0, "minecraft:air");
    assert!(sec.is_empty());
    assert_eq!(sec.compute_crc(), EMPTY_SECTION_CRC);
}

#[test]
fn test_world_bounds_and_pruning() {
    let mut world = VoxelStorage::new();
    world.set_bounds(0, 0, 0, 32, 32, 32);

    world.set_block(5, 5, 5, "minecraft:diamond_block", None);
    world.set_block(25, 25, 25, "minecraft:gold_block", None);

    assert_eq!(world.get_block(5, 5, 5), "minecraft:diamond_block");
    assert_eq!(world.get_block(25, 25, 25), "minecraft:gold_block");

    // Shrink bounds to [0..16, 0..16, 0..16]
    world.set_bounds(0, 0, 0, 16, 16, 16);

    assert_eq!(world.get_block(5, 5, 5), "minecraft:diamond_block");
    // Out-of-bounds block should be pruned
    assert_eq!(world.get_block(25, 25, 25), "minecraft:air");
}

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

#[test]
fn test_section_snapshot_and_manifest_validation() {
    let mut world = VoxelStorage::new();
    world.set_bounds(0, 0, 0, 16, 16, 16);

    let palette = vec!["minecraft:air".to_string(), "minecraft:stone".to_string()];
    let mut grid = vec![0u16; 4096];
    grid[0] = 1; // (0,0,0) is stone

    world.set_section_snapshot(0, 0, 0, 0, 0, 0, 16, 16, 16, &palette, &grid, None, None);
    assert_eq!(world.get_block(0, 0, 0), "minecraft:stone");

    let crc = world.calculate_and_store_section_crc(IVec3::new(0, 0, 0));
    assert_ne!(crc, EMPTY_SECTION_CRC);

    // Matching manifest: returns empty mismatch
    let manifest = vec![(0, 0, 0, crc)];
    let mismatches = world.validate_manifest(&manifest, None);
    assert!(mismatches.is_empty());

    // Mismatched CRC: returns coord
    let bad_manifest = vec![(0, 0, 0, 0x12345678)];
    let bad_mismatches = world.validate_manifest(&bad_manifest, None);
    assert_eq!(bad_mismatches, vec![IVec3::new(0, 0, 0)]);
}

#[test]
fn test_manifest_metadata_export_import() {
    let mut world = VoxelStorage::new();
    world.set_bounds(0, 0, 0, 32, 16, 16);
    world.set_block(1, 2, 3, "minecraft:iron_block", None);
    world.calculate_and_store_section_crc(IVec3::new(0, 0, 0));

    let json_meta = world.export_manifest_metadata();
    assert_eq!(json_meta["size_x"], 32);

    let mut restored = VoxelStorage::new();
    assert!(restored.import_manifest_metadata(&json_meta));
    assert_eq!(restored.min_x, 0);
    assert_eq!(restored.size_x, 32);
    assert_eq!(restored.section_crc_map.get(&IVec3::new(0, 0, 0)), world.section_crc_map.get(&IVec3::new(0, 0, 0)));
}


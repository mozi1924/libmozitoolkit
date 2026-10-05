use std::path::Path;
use glam::IVec3;
use mtk_save::region::RegionFile;
use mtk_save::SaveLoader;
use mtk_voxel::storage::VoxelStorage;

#[test]
fn test_region_chunk_indexing() {
    assert_eq!(RegionFile::chunk_index(0, 0), 0);
    assert_eq!(RegionFile::chunk_index(1, 0), 1);
    assert_eq!(RegionFile::chunk_index(0, 1), 32);
    assert_eq!(RegionFile::chunk_index(31, 31), 1023);

    // Negative coordinates wrap using Euclidean remainder
    assert_eq!(RegionFile::chunk_index(-1, -1), 31 + 31 * 32);
    assert_eq!(RegionFile::chunk_index(-32, 0), 0);
}

#[test]
fn test_real_world_level_dat_if_present() {
    let test_save_path = Path::new("/home/mozi/.minecraft/versions/26.2-Fabric/saves/New World");
    if !test_save_path.exists() {
        return;
    }

    let level_data = SaveLoader::read_level_data(test_save_path).expect("Failed reading level.dat");
    assert_eq!(level_data.level_name, "New World");
    assert!(level_data.data_version >= 2844);
    println!("Loaded real level.dat: {:?}", level_data);
}

#[test]
fn test_real_world_bounded_box_loading() {
    let test_save_path = Path::new("/home/mozi/.minecraft/versions/26.2-Fabric/saves/New World");
    if !test_save_path.exists() {
        return;
    }

    let mut storage = VoxelStorage::new();
    let min_coord = IVec3::new(0, -64, 0);
    let max_coord = IVec3::new(15, 64, 15);

    let loaded = SaveLoader::load_box_into_storage(
        test_save_path,
        "overworld",
        min_coord,
        max_coord,
        &mut storage,
    )
    .expect("Failed loading bounded world box");

    assert!(loaded > 0, "Should have loaded at least one non-empty chunk section");
    let (bx, by, bz, sx, sy, sz) = storage.get_bounds();
    assert_eq!(bx, 0);
    assert_eq!(by, -64);
    assert_eq!(bz, 0);
    assert_eq!(sx, 16);
    assert_eq!(sy, 129);
    assert_eq!(sz, 16);
    println!("Loaded {} sections. Active bounds: ({}, {}, {}, size: {}x{}x{})", loaded, bx, by, bz, sx, sy, sz);
}

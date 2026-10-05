# `mtk-save`

High-performance, host-agnostic Minecraft save (Anvil MCA) loader with zero-copy SIMD NBT decoding (`simdnbt`) and on-demand spatial indexing for MoziToolKit.

---

## 1. Overview & Architecture

`mtk-save` provides high-throughput ingestion of modern Minecraft Java Edition saves (1.18+, 1.20+, 1.21+ / `DataVersion` >= 2844).

Key responsibilities:
- **`level.dat` Inspection**: Zero-copy parsing of world metadata, version info, spawn points, and dimension discovery.
- **Anvil `.mca` Stream Seeker**: Parses 4096-byte chunk location tables and selectively seeks and decompresses only requested chunk sectors.
- **Modern Chunk Decoding**: Bit-unpacks palette-indexed `block_states` (with canonical blockstate string serialization) and 3D biomes (4x4x4 cells) into `SectionStorage`.
- **On-Demand Spatial Slicing**: Given a 3D bounding box selection `[min_block, max_block]`, resolves and loads only the intersecting regions, chunks, and sections, preventing unnecessary memory allocation for multi-gigabyte saves.
- **`VoxelSource` Integration**: Implements `mtk_voxel::source::VoxelSource` for streaming into `VoxelWorld` meshing pipelines.

---

## 2. Core Types & Public API

- **`LevelData`**: Deserialized world metadata from `level.dat`.
- **`RegionFile`**: Direct-seek reader for 1024-chunk Anvil `.mca` files.
- **`ChunkSection`**: Parsed 16x16x16 chunk section containing canonical blockstate strings and biomes.
- **`AnvilWorldSource`**: `VoxelSource` implementation supporting bounded coordinate queries.
- **`SaveLoader`**: High-level loader providing automatic directory discovery and spatial bounding box extraction into `VoxelStorage`.

---

## 3. Minimal Usage Example

```rust
use std::path::Path;
use glam::IVec3;
use mtk_save::{SaveLoader, LevelData};
use mtk_voxel::storage::VoxelStorage;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let save_dir = Path::new("/path/to/saves/MyWorld");

    // 1. Inspect metadata
    let level_data = SaveLoader::read_level_data(save_dir)?;
    println!("World: {}, Version: {}", level_data.level_name, level_data.version_name);

    // 2. Load bounded 3D selection into VoxelStorage
    let min_coord = IVec3::new(-32, -64, -32);
    let max_coord = IVec3::new(32, 128, 32);
    let mut storage = VoxelStorage::new();

    let loaded_sections = SaveLoader::load_box_into_storage(
        save_dir,
        "overworld",
        min_coord,
        max_coord,
        &mut storage,
    )?;

    println!("Loaded {} sections within bounds", loaded_sections);
    Ok(())
}
```

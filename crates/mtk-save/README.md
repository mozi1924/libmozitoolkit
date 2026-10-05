# mtk-save

[![Crate](https://img.shields.io/badge/crate-mtk--save-blue.svg)](Cargo.toml)
[![Rust](https://img.shields.io/badge/Rust-1.80%2B-orange.svg)](https://www.rust-lang.org)
[![NBT](https://img.shields.io/badge/NBT-Zero--Copy%20Builtin-brightgreen.svg)](src/nbt/mod.rs)
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](../../LICENSE)

**`mtk-save`** 是 `libmozitoolkit` (`libmtk`) 套件中的现代 Minecraft Java 版存档加载与空间切片引擎。纯 Safe Rust 实现，严格遵循 **Host-Agnostic（宿主无关）** 原则，基于内置零拷贝 NBT 解码器与 Anvil `.mca` 区域文件直接寻址，为上层体素网格化 (`mtk-voxel`)、实时协同与 Blender 插件提供极速的按需 3D 空间裁剪与流式数据源 (`VoxelSource`) 注入能力。

---

## 目录 (Table of Contents)

- [1. 架构定位与核心职责](#1-架构定位与核心职责)
- [2. 核心模块与系统拓扑](#2-核心模块与系统拓扑)
- [3. 核心数据结构与契约](#3-核心数据结构与契约)
  - [3.1 世界元数据与环境 (`level.rs`)](#31-世界元数据与环境-levelrs)
  - [3.2 Anvil 区域文件与扇区寻址 (`region.rs`)](#32-anvil-区域文件与扇区寻址-regionrs)
  - [3.3 现代 1.18+ 区块切片解包 (`chunk.rs`)](#33-现代-118-区块切片解包-chunkrs)
  - [3.4 按需空间切片加载器 (`loader.rs`)](#34-按需空间切片加载器-loaderrs)
  - [3.5 统一体素源桥接 (`source.rs`)](#35-统一体素源桥接-sourcers)
- [4. 核心公共 API 清单](#4-核心公共-api-清单)
- [5. 快速上手示例 (Quick Start)](#5-快速上手示例-quick-start)
  - [示例 1：读取并检查世界元数据 (`level.dat`)](#示例-1读取并检查世界元数据-leveldat)
  - [示例 2：按需 3D 边界盒切片加载至体素存储](#示例-2按需-3d-边界盒切片加载至体素存储)
  - [示例 3：通过 `AnvilWorldSource` 流式驱动体素世界](#示例-3通过-anvilworldsource-流式驱动体素世界)
- [6. Feature 开关与依赖](#6-feature-开关与依赖)
- [7. 开源协议 (License)](#7-开源协议-license)

---

## 1. 架构定位与核心职责

`mtk-save` 专为大规模 Minecraft Java 版世界存档（1.18+、1.20+、1.21+ / `DataVersion` >= 2844）的高吞吐解析与局部按需提取而设计：

- **`level.dat` 极速审阅**：零拷贝反序列化世界版本号、生成时间戳、出生点坐标与多维度（主世界 Overworld、下界 The Nether、末地 The End）寻址。
- **Anvil `.mca` 零开销寻址**：直接解析 4096 字节 Location Table 与时间戳表，按 4KiB 扇区直接定位目标 Chunk，仅解压所需区块的压缩数据流（Zlib / Gzip）。
- **现代 Chunk Section 纯位解包**：高效解密 384 高度范围（$Y \in [-64, 320)$）的世界切片，解包紧凑位紧缩长整型数组（Bit-Unpacking），提取调色板方块状态与 3D 4×4×4 生物群系单元。
- **按需 3D 空间切片 (On-Demand Spatial Slicing)**：依据用户指定的 3D 轴对齐包围盒 $[P_{\min}, P_{\max}]$，精确推算相交的 Region / Chunk / Section 坐标范围，仅加载落入选区内的区块，杜绝数十 GB 全图扫描导致的内存崩溃。
- **统一体素源 (`VoxelSource`) 桥接**：实现 `mtk_voxel::source::VoxelSource`，使外部 Anvil 存档能够直接作为数据源流式驱动 `SectionMesher` 与 `VoxelWorld`。

---

## 2. 核心模块与系统拓扑

```
┌────────────────────────────────────────────────────────┐
│               Minecraft Java Save Directory            │
│  level.dat  │  region/r.*.*.mca  │  DIM-1  │  DIM1     │
└───────────────────────────┬────────────────────────────┘
                            │ 寻址与流式读取
                            ▼
┌────────────────────────────────────────────────────────┐
│                        mtk-save                        │
│  ┌───────────────────────┐  ┌───────────────────────┐  │
│  │       LevelData       │  │      RegionFile       │  │
│  │ (元数据/维度/出生点)   │  │ (4KiB 扇区直接 Seek)  │  │
│  └───────────────────────┘  └───────────┬───────────┘  │
│                                         │ 压缩扇区数据  │
│                                         ▼              │
│                             ┌───────────────────────┐  │
│                             │      ChunkParser      │  │
│                             │ (内置零拷贝 / 位解包)  │  │
│                             └───────────┬───────────┘  │
│                                         │ Section 字典 │
│                             ┌───────────▼───────────┐  │
│                             │      SaveLoader       │  │
│                             │ (3D 边界盒空间切片)   │  │
│                             └───────────┬───────────┘  │
└─────────────────────────────────────────┼──────────────┘
                                          │ 填充 / 驱动
                                          ▼
                ┌───────────────────────────────────┐
                │   mtk-voxel (VoxelStorage / AO)   │
                └───────────────────────────────────┘
```

---

## 3. 核心数据结构与契约

### 3.1 世界元数据与环境 (`level.rs`)

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LevelData {
    pub level_name: String,
    pub version_name: String,
    pub data_version: i32,
    pub spawn_x: i32,
    pub spawn_y: i32,
    pub spawn_z: i32,
    pub time: i64,
    pub day_time: i64,
    pub hardcore: bool,
    pub clear_weather_time: i32,
    pub rain_time: i32,
    pub raining: bool,
    pub thunder_time: i32,
    pub thundering: bool,
}
```
- `LevelData::from_slice(bytes: &[u8]) -> Result<Self, SaveError>`: 基于内置零拷贝 NBT 从 `level.dat` 字节流解包。

---

### 3.2 Anvil 区域文件与扇区寻址 (`region.rs`)

```rust
pub const REGION_CHUNKS_AXIS: usize = 32;
pub const REGION_TOTAL_CHUNKS: usize = 1024;
pub const SECTOR_SIZE: usize = 4096;
pub const REGION_HEADER_SIZE: usize = 8192; // 4096 (Locations) + 4096 (Timestamps)

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkLocation {
    pub offset_sectors: u32,
    pub sector_count: u8,
}

pub struct RegionFile {
    pub region_x: i32,
    pub region_z: i32,
    locations: [ChunkLocation; REGION_TOTAL_CHUNKS],
    timestamps: [u32; REGION_TOTAL_CHUNKS],
}
```
- `RegionFile::open(path: impl AsRef<Path>) -> Result<Self, SaveError>`: 打开并解析 `.mca` 头部 8192 字节寻址表。
- `region.has_chunk(chunk_x, chunk_z) -> bool`: O(1) 判定区域内是否存在目标区块。
- `region.read_chunk_decompressed(chunk_x, chunk_z) -> Result<Option<Vec<u8>>, SaveError>`: 仅定位、读取并解压目标区块的 NBT 字节流。

---

### 3.3 现代 1.18+ 区块切片解包 (`chunk.rs`)

```rust
pub struct ChunkParser;

pub fn format_canonical_blockstate(block_comp: &NbtCompound) -> String;
```
- `ChunkParser::parse_chunk_sections(bytes: &[u8]) -> Result<HashMap<i8, SectionData>, SaveError>`: 解密 Chunk NBT，逐 Section 解包 BlockStates 与 Biomes。
- `format_canonical_blockstate`: 保证方块状态属性键值升序排列，生成全局一致的权威规范状态字符串。

---

### 3.4 按需空间切片加载器 (`loader.rs`)

```rust
pub struct SaveLoader;
```
- `SaveLoader::read_level_data(save_dir: impl AsRef<Path>) -> Result<LevelData, SaveError>`: 快速读取并审阅世界配置。
- `SaveLoader::load_box_into_storage(...) -> Result<usize, SaveError>`: 给定世界目录、维度名称与 3D 轴对齐空间范围 $[P_{\min}, P_{\max}]$，精确裁剪提取相交区块并灌入 `VoxelStorage`。
- `SaveLoader::load_box_into_storage_with_progress(..., progress: Option<ProgressCallback>) -> Result<usize, SaveError>`: 带实时物理进度节流汇报的空间切片加载器。

---

### 3.5 统一体素源桥接 (`source.rs`)

```rust
pub struct AnvilWorldSource {
    save_dir: PathBuf,
    dimension: String,
    min_bound: IVec3,
    max_bound: IVec3,
    cached_regions: HashMap<(i32, i32), RegionFile>,
}
```
- 实现了 `mtk_voxel::source::VoxelSource` 特征，提供 `get_voxel(pos)` 坐标直接探查与按需缓存。

---

## 4. 核心公共 API 清单

| 函数 / 方法 | 签名 / 描述 |
| :--- | :--- |
| `SaveLoader::read_level_data` | `(save_dir: impl AsRef<Path>) -> Result<LevelData, SaveError>`：读取世界元数据。 |
| `SaveLoader::load_box_into_storage` | `(save_dir, dimension, min_coord, max_coord, &mut storage) -> Result<usize, SaveError>`：按 3D 边界范围切片加载体素。 |
| `SaveLoader::load_box_into_storage_with_progress` | `(..., &mut storage, progress: Option<ProgressCallback>) -> Result<usize, SaveError>`：带进度汇报的切片加载。 |
| `RegionFile::open` | `(path: impl AsRef<Path>) -> Result<Self, SaveError>`：解析 Anvil 区域文件寻址头。 |
| `RegionFile::read_chunk_decompressed` | `(&mut self, cx, cz) -> Result<Option<Vec<u8>>, SaveError>`：直接解压单区块 NBT。 |
| `ChunkParser::parse_chunk_sections` | `(bytes: &[u8]) -> Result<HashMap<i8, SectionData>, SaveError>`：解密区块内所有 Section。 |
| `AnvilWorldSource::new` | `(save_dir, dimension, min_bound, max_bound) -> Self`：创建流式体素源。 |

---

## 5. 快速上手示例 (Quick Start)

在 `Cargo.toml` 中添加依赖：

```toml
[dependencies]
mtk-save = { version = "0.1.0" }
mtk-voxel = { version = "0.1.0" }
glam = "0.29"
```

### 示例 1：读取并检查世界元数据 (`level.dat`)

```rust
use std::path::Path;
use mtk_save::SaveLoader;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let save_dir = Path::new("/path/to/saves/MyWorld");
    let level_data = SaveLoader::read_level_data(save_dir)?;

    println!("World Name: {}", level_data.level_name);
    println!("Version: {} (DataVersion: {})", level_data.version_name, level_data.data_version);
    println!("Spawn Pos: ({}, {}, {})", level_data.spawn_x, level_data.spawn_y, level_data.spawn_z);
    println!("Hardcore: {}, Thundering: {}", level_data.hardcore, level_data.thundering);

    Ok(())
}
```

### 示例 2：按需 3D 边界盒切片加载至体素存储

```rust
use std::path::Path;
use glam::IVec3;
use mtk_save::SaveLoader;
use mtk_voxel::storage::VoxelStorage;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let save_dir = Path::new("/path/to/saves/MyWorld");
    let mut storage = VoxelStorage::new();

    // 定义需要提取的世界 3D 范围（例如以出生点为中心的 64x192x64 选区）
    let min_coord = IVec3::new(-32, -64, -32);
    let max_coord = IVec3::new(32, 128, 32);

    let loaded_sections = SaveLoader::load_box_into_storage(
        save_dir,
        "overworld", // 主世界
        min_coord,
        max_coord,
        &mut storage,
    )?;

    println!("Successfully loaded {} sections within 3D bounds", loaded_sections);
    Ok(())
}
```

### 示例 3：通过 `AnvilWorldSource` 流式驱动体素世界

```rust
use std::path::Path;
use glam::IVec3;
use mtk_save::AnvilWorldSource;
use mtk_voxel::source::VoxelSource;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let save_dir = Path::new("/path/to/saves/MyWorld");
    let min_pos = IVec3::new(0, -64, 0);
    let max_pos = IVec3::new(128, 64, 128);

    let mut source = AnvilWorldSource::new(save_dir, "overworld", min_pos, max_pos);

    // 随机探查方块状态
    let query_pos = IVec3::new(10, 64, 10);
    if let Some(state_str) = source.get_block_state(query_pos) {
        println!("Block at {:?}: {}", query_pos, state_str);
    }

    Ok(())
}
```

---

## 6. Feature 开关与依赖

`mtk-save` 在 `Cargo.toml` 中提供特性开关：

| Feature | 默认启用 | 描述 |
| :--- | :---: | :--- |
| `std` | 是 | 启用标准库与文件系统 I/O。 |
| `serde` | 否 | 为 `LevelData` 等元数据结构体实现 `Serialize` 与 `Deserialize`。 |

---

## 7. 开源协议 (License)

本项目遵循 [GNU General Public License v3.0 or later (GPL-3.0-or-later)](../../LICENSE) 开源协议。

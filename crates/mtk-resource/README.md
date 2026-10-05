# mtk-resource

[![Crate](https://img.shields.io/badge/crate-mtk--resource-blue.svg)](Cargo.toml)
[![Rust](https://img.shields.io/badge/Rust-1.80%2B-orange.svg)](https://www.rust-lang.org)
[![Rayon](https://img.shields.io/badge/Rayon-Parallel-red.svg)](https://github.com/rayon-rs/rayon)
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](../../LICENSE)

**`mtk-resource`** 是 `libmozitoolkit` (`libmtk`) 体系中的无头资源包虚拟文件系统 (VFS)、原版资产标识符规范、`.png.mcmeta` 动图元数据解析、原版 `atlases/*.json` 数据模型与 CTM 连接纹理规则引擎。遵循 **Host-Agnostic（宿主无关）** 原则，为上层图集装箱 (`mtk-texture`)、模型烘焙 (`mtk-model`) 与预编译管线提供统一的多源资产读取与覆盖层叠抽象。

---

## 目录 (Table of Contents)

- [1. 架构定位与职责边界 (Architecture & Responsibilities)](#1-架构定位与职责边界-architecture--responsibilities)
- [2. 核心数据结构与枚举 (Core Structures & Enums)](#2-核心数据结构与枚举-core-structures--enums)
  - [2.1 标识符系统 (`identifier.rs`)](#21-标识符系统-identifierrs)
  - [2.2 虚拟文件系统与多层资源栈 (`pack/`)](#22-虚拟文件系统与多层资源栈-pack)
  - [2.3 纹理动图元数据 (`meta.rs`)](#23-纹理动图元数据-metars)
  - [2.4 原版图集配置规范 (`atlas.rs`)](#24-原版图集配置规范-atlasrs)
  - [2.5 CTM 规则与解算器 (`ctm/`)](#25-ctm-规则与解算器-ctm)
- [3. 核心公共 API 清单 (Public APIs)](#3-核心公共-api-清单-public-apis)
  - [3.1 `ResourcePackStack` 常用方法](#31-resourcepackstack-常用方法)
  - [3.2 CTM 解算方法](#32-ctm-解算方法)
- [4. 快速上手示例 (Quick Start)](#4-快速上手示例-quick-start)
  - [示例 1：构建多层资源栈并解算 PBR 伴随贴图](#示例-1构建多层资源栈并解算-pbr-伴随贴图)
  - [示例 2：加载图集定义与收集待烘焙精灵](#示例-2加载图集定义与收集待烘焙精灵)
  - [示例 3：解析 CTM 规则并执行邻域连接解算](#示例-3解析-ctm-规则并执行邻域连接解算)
- [5. Feature 开关与依赖](#5-feature-开关与依赖)
- [6. 开源协议 (License)](#6-开源协议-license)

---

## 1. 架构定位与职责边界 (Architecture & Responsibilities)

`mtk-resource` 是 `libmozitoolkit` 体系中的资产发现与虚拟文件系统（VFS）底座，具备以下核心职责与边界：

- **多源 VFS 抽象**：统一抽象物理目录 (`DirectoryPack`)、内存文件表 (`MemoryPack`) 以及 ZIP/JAR 压缩包 (`ZipPack`)，支持多层资源包自顶向下按优先级覆盖叠加 (`ResourcePackStack`)。
- **细粒度 PBR 伴随贴图解算**：支持按通道独立探测并合并法线贴图 (`_n.png`, `_N.png`)、高光粗糙度贴图 (`_s.png`, `_S.png`)、叠加层 (`_overlay.png`, `grass_block_side_overlay.png`) 与动画元数据 (`.png.mcmeta`)。
- **原版图集配置解析**：完整支持 Minecraft 1.20+ 数据驱动的 `atlases/*.json` 规范（`directory`, `single`, `filter`, `unstitch`, `paletted_permutations`）以及内置默认图集模板。
- **CTM 连接纹理规则引擎**：解析 OptiFine 与 Continuity 规范的 `.properties` 文件，提供 CTM 47、CTM 17、水平/垂直连接、重复模式与加权伪随机变体解算。
- **纯数据无头设计**：零宿主依赖，纯数据输入输出，全面支持 Rayon 多线程与 `serde` 序列化。

---

## 2. 核心数据结构与枚举 (Core Structures & Enums)

### 2.1 标识符系统 (`identifier.rs`)
```rust
pub const DEFAULT_NAMESPACE: &str = "minecraft";

#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ResourceLocation {
    pub namespace: String,
    pub path: String,
}
```
- `ResourceLocation::new(namespace, path) -> Self`: 构造命名空间与路径。
- `ResourceLocation::vanilla(path) -> Self`: 构造 `minecraft` 默认命名空间资源。
- `ResourceLocation::parse(s: &str) -> Result<Self, ResourceError>`: 解析 `"minecraft:block/stone"` 或 `"block/stone"`。
- `ResourceLocation::parse_texture_path(input: &str) -> Result<Self, ResourceError>`: 容错解析任意物理/相对贴图路径。
- `loc.to_asset_path(category_dir, extension) -> String`: 转换为物理资产相对路径（如 `"assets/minecraft/textures/block/stone.png"`）。
- `ResourceLocation::from_asset_path(path, category_dir, extension) -> Option<Self>`: 从物理资产路径逆向提取标识符。
- `ResourceLocation::model_candidate_asset_paths(input: &str) -> Vec<String>`: 生成模型寻址候选路径列表。

---

### 2.2 虚拟文件系统与多层资源栈 (`pack/`)
```rust
pub trait ResourcePack: Send + Sync {
    fn name(&self) -> &str;
    fn open(&self, relative_path: &str) -> Option<Vec<u8>>;
    fn list_files(&self, prefix: &str) -> Vec<String>;
    fn has_file(&self, relative_path: &str) -> bool;
}

pub struct DirectoryPack { /* ... */ }
pub struct MemoryPack { /* ... */ }
#[cfg(feature = "zip")]
pub struct ZipPack { /* ... */ }

pub struct ResourcePackStack {
    packs: Vec<Box<dyn ResourcePack>>,
}
```
- `is_companion_asset_path(file: &str) -> bool`: 权威判断贴图是否为伴随贴图（`_n.png`, `_s.png`, `_overlay.png` 等），避免误入图集主精灵表。
- `PbrCompanions`: 单一纹理对应的复合伴随数据（Albedo, Normal, Specular, Overlay, MCMETA）。
- `DiscoveredSprite`: 图集构建前收集的完整待解码精灵图描述。

---

### 2.3 纹理动图元数据 (`meta.rs`)
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AnimationFrame {
    Index(u32),
    Detailed { index: u32, time: u32 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnimationMetadata {
    pub frametime: u32,
    pub interpolate: bool,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub frames: Option<Vec<AnimationFrame>>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TextureMetadata {
    pub animation: Option<AnimationMetadata>,
}
```

---

### 2.4 原版图集配置规范 (`atlas.rs`)
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AtlasSource {
    Directory { source: String, prefix: String },
    Single { resource: ResourceLocation, sprite: Option<ResourceLocation> },
    Filter { pattern: AtlasFilterPattern },
    Unstitch { resource: ResourceLocation, divisor_x: f64, divisor_y: f64, regions: Vec<UnstitchRegion> },
    PalettedPermutations {
        palette_key: ResourceLocation,
        permutations: HashMap<String, ResourceLocation>,
        textures: Vec<ResourceLocation>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AtlasCategory {
    Blocks, Items, Particles, Chests, ShulkerBoxes,
    BannerPatterns, ShieldPatterns, ArmorTrims, DecoratedPot,
    Paintings, Celestials, Gui, MapDecorations, Entities, Misc,
    Custom(ResourceLocation),
}
```

---

### 2.5 CTM 规则与解算器 (`ctm/`)
```rust
pub enum CtmMethod {
    Full { inner_seams: bool },
    Compact { inner_seams: bool },
    Horizontal,
    Vertical,
    HorizontalVertical,
    VerticalHorizontal,
    Top,
    Repeat { width: u32, height: u32 },
    Random { weights: Vec<f32>, total_weight: f32, symmetry: CtmSymmetry, linked: bool },
    Fixed,
    Overlay,
}

pub struct CtmRule {
    pub name: String,
    pub priority: i32,
    pub method: CtmMethod,
    pub match_blocks: Vec<BlockMatch>,
    pub match_tiles: Vec<ResourceLocation>,
    pub tiles: Vec<Option<ResourceLocation>>,
    pub connect_logic: ConnectLogic,
    /* ... */
}

pub struct CtmSolver {
    pub rules: Vec<CtmRule>,
    /* ... */
}
```

---

## 3. 核心公共 API 清单 (Public APIs)

### 3.1 `ResourcePackStack` 常用方法
| 方法签名 | 描述 |
| :--- | :--- |
| `push_pack(pack: Box<dyn ResourcePack>)` | 推入高优先级资源包（栈顶）。 |
| `append_pack(pack: Box<dyn ResourcePack>)` | 追加低优先级兜底资源包（栈底）。 |
| `open_asset_raw(asset_path: &str) -> Option<Vec<u8>>` | 按优先级自顶向下读取原始资产字节。 |
| `open_texture_raw(loc: &ResourceLocation) -> Option<Vec<u8>>` | 读取指定纹理的原始 PNG 字节。 |
| `list_all_texture_locations() -> Vec<ResourceLocation>` | 扫描所有活跃包中的非伴随主纹理标识符。 |
| `load_atlas_category(cat: &AtlasCategory) -> AtlasDefinition` | 加载或自动回退至默认类别图集定义。 |
| `resolve_pbr_companions(loc: &ResourceLocation) -> PbrCompanions` | 跨包细粒度聚合 Albedo/Normal/Specular/Overlay/MCMETA。 |
| `collect_sprites_for_atlas(def: &AtlasDefinition) -> Result<Vec<DiscoveredSprite>, ResourceError>` | 展开图集定义并提取所有待缝合精灵数据。 |
| `load_ctm_rules() -> Vec<CtmRule>` | 扫描并解析活跃包中的所有 OptiFine/Continuity CTM 规则。 |
| `compute_stack_fingerprint() -> String` | 计算当前资源包栈拓扑与顺序的唯一哈希指纹。 |

### 3.2 CTM 解算方法
| 方法签名 | 描述 |
| :--- | :--- |
| `CtmRule::parse_properties(path, ns, content) -> Option<CtmRule>` | 解析 `.properties` 配置文件。 |
| `CtmSolver::new(rules: Vec<CtmRule>) -> Self` | 构建带优先级倒序索引的高速 CTM 解算器。 |
| `solver.resolve_face(state, face, pos, base_tile, biome, get_block) -> Option<ResourceLocation>` | 根据邻域方块状态解算目标面替换贴图。 |

---

## 4. 快速上手示例 (Quick Start)

### 示例 1：构建多层资源栈并解算 PBR 伴随贴图
```rust
use mtk_resource::{
    DirectoryPack, MemoryPack, ResourceLocation, ResourcePackStack,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut stack = ResourcePackStack::new();

    // 1. 底部载入基础材质包
    let base_pack = DirectoryPack::new("vanilla", "/path/to/minecraft/assets");
    stack.append_pack(Box::new(base_pack));

    // 2. 顶部覆盖高清 LabPBR 扩展包（仅包含 _n.png 与 _s.png）
    let mut pbr_pack = MemoryPack::new("lab_pbr_patch");
    pbr_pack.insert(
        "assets/minecraft/textures/block/diamond_ore_n.png",
        vec![/* normal map bytes */],
    );
    pbr_pack.insert(
        "assets/minecraft/textures/block/diamond_ore_s.png",
        vec![/* specular map bytes */],
    );
    stack.push_pack(Box::new(pbr_pack));

    // 3. 跨层解析 diamond_ore 的复合伴随数据
    let loc = ResourceLocation::parse("minecraft:block/diamond_ore")?;
    let companions = stack.resolve_pbr_companions(&loc);

    assert!(companions.albedo.is_some());   // 来自 base_pack
    assert!(companions.normal.is_some());   // 来自 pbr_pack
    assert!(companions.specular.is_some()); // 来自 pbr_pack

    Ok(())
}
```

### 示例 2：加载图集定义与收集待烘焙精灵
```rust
use mtk_resource::{AtlasCategory, ResourcePackStack};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stack = ResourcePackStack::new();
    // 载入标准 blocks 类别定义（若无 atlases/blocks.json 则回退内置模板）
    let atlas_def = stack.load_atlas_category(&AtlasCategory::Blocks);

    // 展开并提取所有独立精灵及其伴随层
    let sprites = stack.collect_sprites_for_atlas(&atlas_def)?;
    println!("Discovered {} sprites for blocks atlas", sprites.len());

    Ok(())
}
```

### 示例 3：解析 CTM 规则并执行邻域连接解算
```rust
use glam::IVec3;
use mtk_core::direction::Direction;
use mtk_resource::{CtmRule, CtmSolver};

fn main() {
    let properties = r#"
matchBlocks=minecraft:bookshelf
method=horizontal
tiles=0 1 2 3
"#;

    let rule = CtmRule::parse_properties(
        "optifine/ctm/bookshelf.properties",
        "minecraft",
        properties,
    ).expect("Failed to parse CTM rule");

    let solver = CtmSolver::new(vec![rule]);

    // 模拟邻域方块查询闭包
    let get_block = |pos: IVec3| -> Option<&'static str> {
        if pos == IVec3::new(1, 0, 0) {
            Some("minecraft:bookshelf")
        } else {
            None
        }
    };

    let target_tile = solver.resolve_face(
        "minecraft:bookshelf",
        Direction::North,
        IVec3::ZERO,
        None,
        None,
        get_block,
    );

    if let Some(tile) = target_tile {
        println!("Connected CTM sub-tile: {}", tile);
    }
}
```

---

## 5. Feature 开关与依赖

`mtk-resource` 在 `Cargo.toml` 中提供特性开关：

| Feature | 默认启用 | 描述 |
| :--- | :---: | :--- |
| `std` | 是 | 启用标准库与 `mtk-core/std`、`serde/std`、`zip`。 |
| `zip` | 是 | 启用 `zip` crate 支持直接读取 `.zip` / `.jar` 资源包文件。 |
| `parallel` | 是 | 启用 `rayon` 多线程并发加速（如大批量文件扫描与 CTM 规则预加载）。 |

---

## 6. 开源协议 (License)

本项目遵循 [GNU General Public License v3.0 or later (GPL-3.0-or-later)](../../LICENSE) 开源协议。

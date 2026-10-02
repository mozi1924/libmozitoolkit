# libmtk

[![Crates.io](https://img.shields.io/badge/crates.io-libmtk-orange.svg)](https://crates.io/crates/libmtk)
[![Documentation](https://docs.rs/libmtk/badge.svg)](https://docs.rs/libmtk)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)

**`libmtk`** 是 MoziToolKit 2.0 的统一顶层门面（Unified Facade）与端到端资产编译引擎。纯 Rust 实现，严格遵循 **Host-Agnostic（宿主无关）** 架构设计，汇聚并编排所有底层领域子 Crate，对外提供一致的资产预编译（Asset Prebake）、高级几何处理管线（Mesh Pipeline）与统一错误处理系统。

---

## 架构拓扑 (Sub-Crates Topology)

`libmtk` 作为顶层入口，完整汇聚并按模块重导出以下领域 Crate：

| 领域模块 | 来源 Crate | 核心职责 |
| :--- | :--- | :--- |
| **`libmtk::core`** | [`mtk-core`](../mtk-core/) | 基础几何基元、连续网格缓冲 (`MeshData`)、6 向拓扑与自适应像素细分/智能挤出算子。 |
| **`libmtk::cull`** | [`mtk-cull`](../mtk-cull/) | 原版 1.21+ 邻域面遮挡判定器 (`FaceCuller`)、2D 矩形差集切分与 `MeshSanitizer` 叠面净化。 |
| **`libmtk::model`** | `mtk-model` | BlockState 解析、1.21+ Block Model JSON 递归继承展开、模型烘焙数据库 (`BakedModelDatabase`)。 |
| **`libmtk::voxel`** | `mtk-voxel` | 16x16x16 Section 体素存储、平滑 AO 计算、5x5 生物群系平滑核与无头区块网格化器 (`SectionMesher`)。 |
| **`libmtk::resource`**| `mtk-resource` | 虚拟文件系统 (VFS, `ResourcePackStack`)、.mcmeta 动图解析与 CTM 17/47 连接纹理求解器。 |
| **`libmtk::texture`** | `mtk-texture` | 2D 矩形装箱 (`Stitcher`)、多类别 PBR 图集烘焙、Overlay 贴图合成与 Standalone 资产对齐。 |
| **`libmtk::material`**| `mtk-material` | 66 种原版生物群系调色板引擎 (SSOT)、`BiomeResolver` 映射提取与 Rayon 并行 UV 重映射。 |
| **`libmtk::sync`** | `mtk-sync` | 原生 WebSocket 实时协同客户端 (`LiveSyncSession`) 与 Yefira 二进制小端序协议编解码器。 |

---

## 核心特性与顶层功能 (Top-Level Features)

### 1. 端到端无头资产预编译 (`libmtk::prebake`)

一键将离线资源包栈（Vanilla + 材质包）全量烘焙为运行时持久化高速缓存（受 `ASSET_CACHE_FORMAT_VERSION` 版本约束）：
- **多类别图集烘焙**：自动装箱 Blocks、Items、ArmorTrims、Beds 等图集大图，并行编码 PNG 并落盘；
- **伴随贴图与 Overlay 提取**：自动合成 `_overlay.png` 并在图集地址映射表 (`AtlasAddressMap`) 中建立别名对齐；
- **模型图集 UV 预解算**：多线程全量烘焙模型，将图集 UV 坐标与 Chunk/Texture ID 预先注入 `BakedModelDatabase` 并序列化为二进制快照 (`models.bin`)；
- **调色板与生物群系映射**：提取 `biome_mapping.json` 供运行时极速着色。

### 2. 高级网格处理管线 (`libmtk::pipeline`)

提供单次调用、数据输入输出（Data-In Data-Out）的外部网格处理入口 `process_mesh`：
- 材质名称清洗与别名规范化解析；
- 多线程 Rayon 并行 Atlas 与 Standalone 归一化 [0, 1] 双 UV 重映射；
- 生成结构化的网格统计与材质分配元数据。

### 3. 统一错误系统 (`MtkError`)

使用 `thiserror` 统一封装模型、体素、贴图、资源、材质、I/O 与并发线程池错误。

---

## 核心 API 清单 (Core API Reference)

```rust
// 1. 统一错误类型
pub enum MtkError {
    Model(mtk_model::ModelError),
    Voxel(mtk_voxel::VoxelError),
    Texture(mtk_texture::error::TextureError),
    Resource(mtk_resource::error::ResourceError),
    Material(mtk_material::MaterialError),
    Io(std::io::Error),
    Json(serde_json::Error),
    Bincode(bincode::Error),
    ThreadPool(String),
}

// 2. 端到端资产预编译
pub fn precompile_all_assets(
    stack: &ResourcePackStack,
    cache_dir: impl AsRef<Path>,
    config: &PrecompileConfig,
) -> Result<PrecompileResult, MtkError>;

pub fn prebake_all_models(
    stack: &ResourcePackStack,
    atlas_map: Option<&AtlasAddressMap>,
) -> Result<BakedModelDatabase, MtkError>;

// 3. 高级网格处理管线
pub fn process_mesh(
    mesh: &MeshData,
    material_names: &[String],
    address_map: Option<&AtlasAddressMap>,
    config: &MeshPipelineConfig,
) -> ProcessMeshOutput;
```

---

## 快速上手 (Quick Start)

在 `Cargo.toml` 中添加依赖：

```toml
[dependencies]
libmtk = { version = "0.1.0" }
```

### 1. 执行全量资源包预编译

```rust
use std::path::Path;
use libmtk::{
    precompile_all_assets, CacheManifest, DirectoryPack, PrecompileConfig,
    ResourcePackStack,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut stack = ResourcePackStack::new();

    // 载入原版资源目录或材质包 ZIP
    stack.push_pack(Box::new(DirectoryPack::new(
        "Vanilla",
        Path::new("/path/to/minecraft/resourcepack"),
    )));

    let output_cache_dir = Path::new("/path/to/cache_dir");

    let config = PrecompileConfig {
        max_atlas_width: 4096,
        max_atlas_height: 4096,
        atlas_category: "all".to_string(),
        compile_atlas: true,
        compile_standalone: true,
        compile_models: true,
        num_threads: None, // 使用全部可用 CPU 核心
    };

    // 执行预编译流水线
    let result = precompile_all_assets(&stack, output_cache_dir, &config)?;

    println!(
        "Precompilation complete: {} baked models, {} atlas chunks in {:?}",
        result.baked_models,
        result.atlas_chunks,
        output_cache_dir
    );

    // 读取并校验元数据清单
    if let Some(manifest) = CacheManifest::read_from_dir(output_cache_dir) {
        assert!(manifest.is_valid_for(&result.fingerprint));
    }

    Ok(())
}
```

### 2. 高级网格处理流水线

```rust
use libmtk::{process_mesh, MeshData, MeshPipelineConfig};

fn main() {
    let input_mesh = MeshData::new();
    let raw_materials = vec!["stone".to_string(), "grass_block_top".to_string()];

    let config = MeshPipelineConfig {
        custom_aliases: None,
        generate_secondary_uv: true, // 为 PBR 生成第二套 [0, 1] UV
        grid_atlas_spec: None,
    };

    let output = process_mesh(&input_mesh, &raw_materials, None, &config);

    println!(
        "Processed mesh: {} vertices, {} slots resolved",
        output.mesh.vertex_count(),
        output.stats.resolved_slots
    );
}
```

---

## Feature Flags

| Feature | 默认启用 | 描述 |
| :--- | :---: | :--- |
| `std` | 是 | 启用标准库与各子模块的标准实现支持。 |
| `parallel` | 是 | 启用 `rayon` 多线程并发加速（图集编码、模型烘焙、UV 重映射等）。 |
| `sync` | 是 | 启用原生 WebSocket 实时同步客户端与网络协议编解码 (`mtk-sync`)。 |

---

## 许可证 (License)

本项目采用 [MIT OR Apache-2.0](LICENSE) 双重开源许可证。

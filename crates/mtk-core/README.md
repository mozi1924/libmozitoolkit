# mtk-core

[![Crates.io](https://img.shields.io/badge/crates.io-mtk--core-orange.svg)](https://crates.io/crates/mtk-core)
[![Documentation](https://docs.rs/mtk-core/badge.svg)](https://docs.rs/mtk-core)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)

**`mtk-core`** 是 MoziToolKit (`libmtk`) 体系的底层几何计算核心与标准数据契约底座。该 Crate 纯 Rust 实现，严格遵循 **Host-Agnostic（宿主无关）** 与 **Data-In / Data-Out（纯数据输入输出）** 原则，为上层体素化、模型烘焙、材质映射与 DCC 宿主（如 Blender 扩展 `MoziToolKit`）提供高性能网格缓冲、6 向拓扑、自适应像素网格切分以及智能挤出侧面 UV 修复算子。

---

## 核心特性 (Key Features)

- **扁平连续几何缓冲 (`MeshData`)**：零堆碎片化的连续顶点、法线、UV、四边形索引 (`quad_indices`) 与通用层级自定义属性 (`MeshAttribute`) 容器，天然适配 GPU 缓冲区与 NumPy / DCC 零拷贝映射。
- **高性能多边形拓扑容器 (`FlatPolygonMesh`)**：针对多边形网格（N-gon / Quad / Tri）提供展平的 1D 循环缓冲区（Loop Buffers），消除成千上万个嵌套 `Vec<Vec<T>>` 堆分配开销。
- **Minecraft 6 向拓扑与坐标系转换**：标准 6 方向枚举 (`Direction`)、位掩码 (`DirMask`)，以及 Minecraft 局部坐标到标准化右手 Z-Up 坐标（`mc_local_to_centered_z_up`, `mc_world_to_z_up`）的精准转换。
- **自适应像素网格细分 (`subdivide`)**：基于纹理分辨率与 UV 物理边长智能计算细分网格，自动适应非正方形图集与长条动图安全截断，支持顶点属性的双线性插值 (`interpolate_bilinear_*`) 与微距流形焊接。
- **智能挤出与侧面 UV 重构 (`extrude`)**：智能判断挤出法线朝向，自动修复因几何挤出产生的拉伸 UV，支持图集安全边距钳位（Atlas Safe Padding Clamping）以及 3D Perlin / Cellular 噪声随机挤出。

---

## 模块结构与核心 API (Architecture & Core Types)

### 1. 几何缓冲与基元 (`mesh`, `geometry`, `attributes`)

| 类型 / 结构体 | 说明 |
| :--- | :--- |
| [`MeshData`](src/mesh.rs) | 核心连续网格缓冲，包含 `positions`, `normals`, `indices`, `uvs`, `secondary_uvs`, `colors`, `face_materials`, `face_tint_indices`, `quad_indices`, `custom_attributes`。 |
| [`Quad`](src/geometry.rs) | 4 顶点平面四边形基元。支持 `Quad::unit_cube_face(Direction)` 生成标准单位立方体面（对齐 Minecraft 1.21+ FaceInfo 逆时针环绕规范）。 |
| [`FlatPolygonMesh`](src/polygon.rs) | 连续 1D 循环拓扑网格表示，提供 `face_vertices(i)` 与 `face_uvs(i)` 零拷贝切片访问器。 |
| [`Aabb2d`](src/geometry.rs) / [`Aabb3d`](src/geometry.rs) | 2D / 3D 轴对齐包围盒，提供相交、包含判定与矩形布尔运算。 |
| [`FaceAttributes`](src/attributes.rs) | 面属性包，包含 `material_slot: MaterialSlotId` 与 `tint_index: TintIndex`。 |
| [`MeshAttribute`](src/attributes.rs) | 通用动态属性层，支持 `Point`, `Corner`, `Face`, `Mesh` 作用域及多种数值类型（`AttributeData`）。 |

### 2. 6 向拓扑与坐标系 (`direction`, `geometry`)

| 函数 / 枚举 | 签名 / 描述 |
| :--- | :--- |
| [`Direction`](src/direction.rs) | `Down` (0), `Up` (1), `North` (2), `South` (3), `West` (4), `East` (5)。支持 `opposite()`, `to_vector()`, `normal()` 等。 |
| [`DirMask`](src/direction.rs) | 6 方向位掩码（基于 `bitflags`），支持高效的相邻面快速位运算。 |
| `mc_local_to_centered_z_up` | `(lx: f32, ly: f32, lz: f32) -> Vec3`：将 Minecraft 局部体素坐标转换为以方块中心为原点的右手 Z-Up 坐标。 |
| `mc_world_to_z_up` | `(wx: f32, wy: f32, wz: f32) -> Vec3`：将 Minecraft 世界坐标 (+X East, +Y Up, +Z South) 转换为标准右手 Z-Up 坐标 (+X East, +Y North, +Z Up)。 |

### 3. 自适应像素切分 (`subdivide`)

| 函数 | 描述 |
| :--- | :--- |
| `calculate_face_target_grid` | `(&uvs, tex_w, tex_h, pixels_per_face, max_subdivisions) -> (u32, u32)`：根据 UV 坐标与贴图尺寸推算最优细分列数和行数。 |
| `calculate_pixel_grid_cut_factors` | `(&uvs, tex_w, tex_h, pixels_per_face, max_subdivisions) -> (Vec<f32>, Vec<f32>)`：计算四边形在纹理整数像素边界上的归一化切分参数因子。 |
| `adaptive_pixel_split_mesh` | `(mesh, face_resolutions, default_res, pixels_per_face, max_subdiv, weld_dist) -> MeshData`：全网格批量自适应像素细分，支持多边形属性双线性插值与顶点焊接。 |
| `weld_mesh_vertices` | `(mesh, threshold) -> MeshData`：基于空间哈希的高性能顶点焊接去重。 |

### 4. 智能挤出与 UV 修复 (`extrude`, `extrude_mesh`)

| 函数 / 枚举 | 描述 |
| :--- | :--- |
| [`ExtrudeUvMode`](src/extrude.rs) | `Smart`（智能法线判定）、`Inward`（边缘内缩采样）、`Outward`（外推连续条带采样）。 |
| [`ExtrudeNoiseType`](src/extrude.rs) | `UniformRandom`（伪随机）、`Perlin`（连续梯度噪声）、`Cellular`（Voronoi 细胞阶梯噪声）。 |
| `repair_extruded_side_uv` | `(...) -> [[f32; 2]; 4]`：计算侧面重构的 4 角点 UV，并执行 Safe Padding Clamping 避免图集相邻纹理渗色。 |
| `process_mesh_extrude_repair` | `(input: &ExtrudeMeshInput) -> ExtrudeMeshOutput`：全网格批量 Data-In Data-Out 挤出修复门面算子。 |
| `process_flat_mesh_extrude_repair` | `(mesh: &FlatPolygonMesh, ...) -> ExtrudeMeshOutput`：基于扁平连续内存网格的高性能挤出修复。 |
| `process_random_extrude_mesh` | `(input: &RandomExtrudeMeshInput) -> RandomExtrudeMeshOutput`：单批次完成离散面随机挤出、3D 噪声位移与侧面拓扑 UV 缝合。 |

### 5. 确定性随机与植被偏移 (`random`)

| 函数 / 结构体 | 描述 |
| :--- | :--- |
| `mc_coordinate_seed` | `(x: i32, y: i32, z: i32) -> i64`：1:1 对标原版 `Mth.getSeed`，基于世界绝对整数坐标计算确定性 64 位哈希种子。 |
| [`JavaRandom`](src/random.rs) | 1:1 对标 Java 标准 48 位线性同余随机数发生器（LCG），提供 `next_int(bound)`、`next_float()` 等。 |
| [`OffsetType`](src/random.rs) | `None`, `XZ`（花草水平偏移）, `XYZ`（草丛/蕨类/树根 3D 偏移与微下沉）。 |
| `get_block_offset` | `(offset_type, x, y, z) -> Vec3`：1:1 计算植被在方块内的连续位置抖动偏移量。 |
| `determine_block_offset_type` | `(block_id_or_state: &str) -> OffsetType`：依据原版规则判定方块标识符的植被偏移类型。 |

---

## 快速上手 (Quick Start)

在 `Cargo.toml` 中添加依赖：

```toml
[dependencies]
mtk-core = { version = "0.1.0" }
glam = "0.29"
```

### 1. 构建基础单位立方体网格

```rust
use glam::{Vec2, Vec3};
use mtk_core::attributes::FaceAttributes;
use mtk_core::direction::Direction;
use mtk_core::geometry::Quad;
use mtk_core::mesh::MeshData;

fn main() {
    let mut mesh = MeshData::new();

    // 为 6 个主方向构建标准四边形面
    for dir in Direction::ALL {
        let quad = Quad::unit_cube_face(dir);
        let attrs = FaceAttributes {
            material_slot: 0,
            tint_index: -1,
        };
        mesh.append_quad(&quad, &attrs);
    }

    assert_eq!(mesh.vertex_count(), 24);
    assert_eq!(mesh.triangle_count(), 12);
    assert_eq!(mesh.quad_count(), 6);
}
```

### 2. 执行自适应像素网格细分

```rust
use mtk_core::mesh::MeshData;
use mtk_core::subdivide::adaptive_pixel_split_mesh;
use mtk_core::geometry::Quad;
use mtk_core::direction::Direction;
use mtk_core::attributes::FaceAttributes;

fn main() {
    let mut mesh = MeshData::new();
    let quad = Quad::unit_cube_face(Direction::Up);
    mesh.append_quad(&quad, &FaceAttributes::default());

    // 对面执行 16x16 纹理自适应像素切分（每个像素 1 个四边形）
    let subdivided = adaptive_pixel_split_mesh(
        &mesh,
        None,             // 使用默认分辨率
        [16, 16],         // default_resolution: 16x16
        1.0,              // pixels_per_face
        64,               // max_subdivisions
        1e-4,             // weld_dist
    );

    println!(
        "Subdivided mesh: {} vertices, {} triangles",
        subdivided.vertex_count(),
        subdivided.triangle_count()
    );
}
```

### 3. 空间坐标系变换

```rust
use mtk_core::geometry::{mc_local_to_centered_z_up, mc_world_to_z_up};
use glam::Vec3;

fn main() {
    // 局部体素原点 (0.5, 0.5, 0.5) 变换到中心对齐 Z-Up 坐标系
    let local_center = mc_local_to_centered_z_up(0.5, 0.5, 0.5);
    assert_eq!(local_center, Vec3::ZERO);

    // Minecraft 世界坐标 (+X East, +Y Up, +Z South) -> 右手 Z-Up (+X East, +Y North, +Z Up)
    let world_pos = mc_world_to_z_up(10.0, 64.0, -20.0);
    assert_eq!(world_pos, Vec3::new(10.0, 20.0, 64.0));
}
```

---

## Feature Flags

| Feature | 默认启用 | 描述 |
| :--- | :---: | :--- |
| `std` | 是 | 启用标准库与 `glam/std`。禁用时支持 `no_std`（依赖 `alloc`）。 |
| `serde` | 是 | 为核心数据结构实现 `Serialize` 与 `Deserialize`。 |
| `parallel` | 否 | 启用 `rayon` 多线程并行加速（如空间顶点焊接与大批量细分）。 |

---

## 许可证 (License)

本项目采用 [MIT OR Apache-2.0](LICENSE) 双重开源许可证。

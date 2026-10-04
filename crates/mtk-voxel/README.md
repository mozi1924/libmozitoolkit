# mtk-voxel

[![Crate](https://img.shields.io/badge/crate-mtk--voxel-blue.svg)](Cargo.toml)
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](../../LICENSE)

**`mtk-voxel`** 是 `libmozitoolkit` (`libmtk`) 套件中的核心体素（Voxel）几何引擎与空间存储 Crate。遵循 **无宿主依赖 (Host-Agnostic)**、**纯数据输入输出 (Data-In, Data-Out)** 与 **多源解耦** 架构原则，提供高吞吐 16×16×16 Chunk Section 紧凑体素存储、3D 稀疏世界容器、物理流体网格重建、平滑环境光遮蔽 (Smooth AO)、多源体素抽象以及多线程高性能网格化器 (Mesher)。

---

## 目录 (Table of Contents)

- [1. 架构定位与核心职责](#1-架构定位与核心职责)
- [2. 核心模块与系统拓扑](#2-核心模块与系统拓扑)
- [3. 核心数据结构与抽象 Trait](#3-核心数据结构与抽象-trait)
  - [3.1 紧凑体素存储 (`storage`)](#31-紧凑体素存储-storage)
  - [3.2 统一体素源抽象 (`source`)](#32-统一体素源抽象-source)
  - [3.3 网格化配置与统计 (`types`)](#33-网格化配置与统计-types)
- [4. 核心算法与功能实现](#4-核心算法与功能实现)
  - [4.1 18×18×18 邻域采样与零分支遮挡剔除](#41-181818-邻域采样与零分支遮挡剔除)
  - [4.2 4 顶点平滑 AO 与各向异性对角线折痕消除](#42-4-顶点平滑-ao-与各向异性对角线折痕消除)
  - [4.3 物理流体表面与流向 UV 重建](#43-物理流体表面与流向-uv-重建)
  - [4.4 5×5 (R=2) 生物群系平滑过渡与边界防渗色](#44-55-r2-生物群系平滑过渡与边界防渗色)
  - [4.5 增量网格重构 (`DeltaMesher`)](#45-增量网格重构-deltamesher)
- [5. 快速上手示例 (Quick Start)](#5-快速上手示例-quick-start)
  - [示例 1：构建体素世界并生成标准化网格](#示例-1构建体素世界并生成标准化网格)
  - [示例 2：增量修改与局部网格更新 (`DeltaMesher`)](#示例-2增量修改与局部网格更新-deltamesher)
  - [示例 3：实现自定义 `VoxelSource` 外部数据源](#示例-3实现自定义-voxelsource-外部数据源)
- [6. Feature 开关与依赖](#6-feature-开关与依赖)

---

## 1. 架构定位与核心职责

在 `libmozitoolkit` 体系中，`mtk-voxel` 承担**体素几何与空间组织权威核心**的职责：

```
                ┌─────────────────────────────────────────┐
                │        上层应用 / 绑定 / 宿主前端        │
                │ (mtk-sync / libmtk / mtk-py / Blender)  │
                └───────────────────┬─────────────────────┘
                                    │ 消费 / 驱动
                                    ▼
┌────────────────────────────────────────────────────────────────────────┐
│                              mtk-voxel                                 │
│  ┌────────────────────────┐  ┌──────────────────────────────────────┐  │
│  │   VoxelSource (抽象)   │  │        VoxelStorage (3D 稀疏世界)     │  │
│  │ (Anvil / Stream / DB)  │  │ (Section 字典 / 选区裁切 / CRC 清单)  │  │
│  └───────────┬────────────┘  └──────────────────┬───────────────────┘  │
│              │ Ingestion                        │ PaddedVoxelArray     │
│              └─────────────────► ┌──────────────▼─────────────┐        │
│                                  │   SectionMesher / AO       │        │
│                                  │ (流体重建 / Biome 渐变)    │        │
│                                  └──────────────┬─────────────┘        │
└─────────────────────────────────────────────────┼──────────────────────┘
                                                  ▼
                                ┌───────────────────────────────────┐
                                │   MeshData (mtk-core POD 网格)    │
                                └───────────────────────────────────┘
```

- **纯数据流规范**：以紧凑的 1D/3D 缓冲、BlockState 字符串或 Palette 索引为输入，直接产出包含顶点位置、法线、四边形/三角形索引、UV 及 16 项标准面属性（材料槽、图集 Chunk ID、Biome Tint、发光等级 `mtk_emission` 等）的 `MeshData`。
- **宿主无关 (Host-Agnostic)**：核心内部采用标准 3D 几何坐标系运算，支持运行时输出 `Minecraft` 原生、`ZUpRightHanded` (Unreal/Blender) 或 `YUpRightHanded` (Unity/WebGPU) 坐标。
- **极致吞吐设计**：采用 4,096 紧凑定长数组、18×18×18 邻域垫片 (Apron)、Palette 级着色预解析（消除内部热循环哈希查找）以及 Rayon 多区块并行。

---

## 2. 核心模块与系统拓扑

| 模块路径 | 职责与功能 |
| :--- | :--- |
| **`storage::section`** | 16×16×16 单区块切片存储 (`SectionStorage`)，密集调色板映射与 non-air 数量追踪。 |
| **`storage::world`** | 3D 稀疏区块世界容器 (`VoxelStorage`)、选区包围盒裁剪、CRC32 清单比对与原子版本控制。 |
| **`storage::crc`** | 快速 CRC32 计算器、规范 BlockState 字符串提取器与空区块 CRC 表。 |
| **`source`** | 统一体素源抽象 Trait：`VoxelReader`、`VoxelWriter`、`VoxelSource` 及批量灌流管线 `ingest_from_source`。 |
| **`mesher::section_mesher`**| 高性能区块网格化器 (`SectionMesher`)，集成遮挡判定、AO、着色元数据绑定与顶点空间焊接。 |
| **`mesher::delta_mesher`**  | 增量网格化器 (`DeltaMesher`)，脏区块识别与 3×3×3 邻域涟漪扩散重构。 |
| **`mesher::ao`**            | 原版 4 顶点平滑环境光遮蔽算法与各向异性对角线翻转决策 (`should_flip_quad_diagonal`)。 |
| **`mesher::shading`**       | 材质着色元数据解析、CTM 调度与 Palette 级别预解算缓存 (`PaletteMeshingData`)。 |
| **`mesher::collector`**     | 逐面材质槽、图集 ID、Biome 染色与纹理键属性收集器 (`FaceAttributesCollector`)。 |
| **`fluid`**                 | 物理流体表面高度解算、流向矢量 (`vx, vz`)、流体 Top/Side UV 映射与网格发射。 |
| **`biome`**                 | 5×5 (R=2) 反距离权重核平滑生物群系柱计算 (`SmoothedBiomeColumn`) 与边界钳位防护。 |
| **`types`**                 | 配置参数 `MesherConfig`、坐标系枚举 `CoordinateSystem`、统计结果 `WorldMeshBuildResult` 与错误类型 `VoxelError`。 |

---

## 3. 核心数据结构与抽象 Trait

### 3.1 紧凑体素存储 (`storage`)

#### `SectionStorage`
16×16×16 单区块切片，采用定长数组 `Box<[u16; 4096]>` 配合局部 `palette: Vec<String>` 存储（0 号槽位固定为 `"minecraft:air"`）：
```rust
pub struct SectionStorage {
    pub coord: IVec3,
    pub palette: Vec<String>,
    pub voxels: Box<[u16; 4096]>,
    pub non_air_count: u32,
    pub cached_crc: Option<u32>,
}
```
- `is_empty() -> bool`：若 `non_air_count == 0` 则为极速纯空判定。
- `get_local_state(x, y, z) -> &str`：根据局部坐标 `0..16` 获取方块状态。
- `set_local(x, y, z, state) -> bool`：更新局部方块，自动维护 Palette、`non_air_count` 并使 CRC 缓存失效。
- `build_padded_array(get_neighbor_fn) -> PaddedVoxelArray`：向外扩展 1 格构建 18×18×18 采样窗口。

#### `PaddedVoxelArray`
网格化器的零分支采样垫片（包含 `-1..=16` 范围，空间尺寸 18×18×18 = 5,832 体素）：
```rust
pub struct PaddedVoxelArray {
    pub coord: IVec3,
    pub palette: Vec<String>,
    pub padded_voxels: Vec<u16>,
    pub is_empty: bool,
    pub biome_data: Option<Vec<SmoothedBiomeColumn>>, // 256 列平滑 Biome 数据
}
```

#### `VoxelStorage`
3D 稀疏世界容器，管理全局选区包围盒与稀疏 `SectionStorage` 哈希表：
```rust
pub struct VoxelStorage {
    pub min_x: i32, pub min_y: i32, pub min_z: i32,
    pub size_x: i32, pub size_y: i32, pub size_z: i32,
    pub sections: HashMap<IVec3, SectionStorage>,
    pub biome_map: HashMap<IVec3, String>,
    pub biome_column_map: HashMap<[i32; 2], String>,
    pub primary_biome: Option<String>,
    pub dirty_sections: HashSet<IVec3>,
    pub section_crc_map: HashMap<IVec3, u32>,
    pub generation: AtomicU64,
}
```
- **选区裁剪与接缝脏标记**：调用 `set_bounds(...)` 自动修剪越界区块，并将边界接缝处的区块自动加入 `dirty_sections` 以便更新交界面遮挡。
- **快照与增量更新**：`set_full_snapshot`、`set_section_snapshot`、`apply_delta_update`。
- **清单比对**：`validate_manifest` 用于对比服务器端的区块 CRC32 列表，精准找出不同步的区块坐标。
- **内嵌调试世界**：`create_debug_world() -> Result<Self, VoxelError>` 内置包含 529 区块切片与 32,539 方块状态的原版 Minecraft 调试世界快照（`debug_world_snapshot.json.gz`），内部采用 `OnceLock` 线程安全懒加载缓存，后续调用克隆仅需 <1ms。

---

### 3.2 统一体素源抽象 (`source`)

为了将体素存储与输入上游（如 Anvil 存档、LevelDB、实时网络流、点云逆向推算）彻底解耦，`mtk-voxel` 定义了统一抽象接口：

```rust
/// 只读空间访问
pub trait VoxelReader {
    fn get_block(&self, x: i32, y: i32, z: i32) -> &str;
    fn get_section(&self, sx: i32, sy: i32, sz: i32) -> Option<&SectionStorage>;
    fn contains_section(&self, sx: i32, sy: i32, sz: i32) -> bool;
    fn block_bounds(&self) -> Option<(IVec3, IVec3)>;
    fn get_biome(&self, x: i32, y: i32, z: i32) -> &str;
}

/// 写入与标记
pub trait VoxelWriter {
    fn set_block(&mut self, x: i32, y: i32, z: i32, state: &str, biome: Option<&str>);
    fn set_section(&mut self, sx: i32, sy: i32, sz: i32, section: SectionStorage);
    fn mark_section_dirty(&mut self, sx: i32, sy: i32, sz: i32);
    fn set_bounds(&mut self, min_x: i32, min_y: i32, min_z: i32, size_x: i32, size_y: i32, size_z: i32);
    fn clear(&mut self);
}

/// 跨数据源统一生产者
pub trait VoxelSource {
    fn source_name(&self) -> &str;
    fn estimated_section_count(&self) -> Option<usize>;
    fn has_section(&self, sx: i32, sy: i32, sz: i32) -> bool;
    fn load_section(&mut self, sx: i32, sy: i32, sz: i32) -> Result<Option<SectionStorage>, VoxelError>;
    fn section_bounds(&self) -> Option<(IVec3, IVec3)>;
    fn sample_biome(&self, x: i32, y: i32, z: i32) -> Option<&str>;
}
```

使用 `ingest_from_source(source, target, sections)` 可将数据无缝搬运至任意 `VoxelWriter`。

---

### 3.3 网格化配置与统计 (`types`)

```rust
pub struct MesherConfig {
    pub coordinate_system: CoordinateSystem, // Minecraft / ZUpRightHanded / YUpRightHanded
    pub origin_centered: bool,               // 是否将原点居中到底部中心
    pub weld_vertices: bool,                 // 是否合并共面/重合顶点形成流形拓扑
    pub selection_bounds: Option<([i32; 3], [i32; 3])>,
    pub enable_ao: bool,                     // 是否计算 4 顶点 AO 遮蔽
    pub exclude_hidden_volume: bool,         // 是否剔除复杂模型内部遮挡面
    pub mesh_fluids: bool,                   // 是否生成水/岩浆物理曲面
    pub ctm_solver: Option<Arc<CtmSolver>>,  // CTM 47/17 连接纹理求解器
    pub atlas_address_map: Option<Arc<AtlasAddressMap>>, // 图集 UV 映射表
    pub biome_resolver: Option<Arc<BiomeResolver>>,      // 生物群系解析器
    pub custom_aliases: Option<Arc<HashMap<String, Vec<String>>>>,
}
```

---

## 4. 核心算法与功能实现

### 4.1 18×18×18 邻域采样与零分支遮挡剔除

在遍历单区块的 4,096 个内部体素时，网格化器直接索引 `PaddedVoxelArray`（尺寸 18×18×18）：
- 内部坐标 `(lx, ly, lz)` 对应垫片坐标 `(lx+1, ly+1, lz+1)`；
- 6 个相邻方向的邻域方块直接通过固定偏移 `(px+dx, py+dy, pz+dz)` 获取，**无需任何边界条件分支或越界检查**；
- **两阶段预解析加速**：在进入循环前构建 `PaletteMeshingData`，将各 BlockState 的模型面、材质槽、图集 UV 及染色规则预先解析至 Palette 级（单区块仅执行数十次），热循环内部为纯连续内存读取。
- **6 向分桶与跳过**：模型面划分为 6 个主方向分桶与免剔除分桶。若邻居方块不透明，1 次判定即可直接跳过该方向的所有面。

### 4.2 4 顶点平滑 AO 与各向异性对角线折痕消除

对指定朝向的面，通过采样邻近 3 个方块（2 个正交邻居与 1 个对角邻居）计算 4 个顶点的离散遮挡级别 $0..=3$：

$$\text{AO} = \begin{cases} 0 & \text{若 side1 与 side2 均不透明（角被遮挡）} \\ 3 - (\text{side1} + \text{side2} + \text{corner}) & \text{否则} \end{cases}$$

遮挡等级线性映射为亮度乘子 $[0.2, 0.467, 0.733, 1.0]$。

#### 各向异性折痕消除 (`should_flip_quad_diagonal`)
四边形剖分为 2 个三角形时，若对角线剖分方向与光照梯度不匹配，会产生明显的斜向三角面光照撕裂。`mtk-voxel` 通过比较对角顶点 AO 之和进行动态拓扑翻转：
```rust
#[inline]
pub fn should_flip_quad_diagonal(ao: [u8; 4]) -> bool {
    (ao[0] as i32 + ao[2] as i32) > (ao[1] as i32 + ao[3] as i32)
}
```

---

### 4.3 物理流体表面与流向 UV 重建

- **流体高度插值**：`calculate_fluid_corner_heights` 采样中心与 8 邻域流体高度，结合 Minecraft 原版物理规则（静止水源表面张力 $8/9$ 高度、下落水流高度为 $1.0$）计算 4 角高度 $(c_{\text{NW}}, c_{\text{NE}}, c_{\text{SE}}, c_{\text{SW}})$。
- **流向矢量与 UV 旋转**：`calculate_fluid_flow_vector` 依据周围 4 邻域及下层方块高度差计算流动矢量 $(v_x, v_z)$ 与旋转角 $\theta = \text{atan2}(v_z, v_x) - \frac{\pi}{2}$，并在生成顶面网格时对 UV 应用旋转变换矩阵。

---

### 4.4 5×5 (R=2) 生物群系平滑过渡与边界防渗色

内置 Minecraft 原版 5×5 ($R=2$) 反距离权重卷积核 (`BIOME_KERNEL_R2`)：

```rust
pub const BIOME_KERNEL_R2: &[(i32, i32, f32)] = &[
    (-2, -2, 0.2612), (-2, -1, 0.3090), ... (0, 0, 1.0000), ... (2, 2, 0.2612),
];
```

- 在构建 `PaddedVoxelArray` 时，为切片的 256 列 ($16\times 16$) 预计算 `SmoothedBiomeColumn`（包含草方块、树叶、干枯树叶、水体的线性 RGBA 颜色及 Colormap 采样 UV）。
- **选区边缘钳位防护 (Edge Clamping)**：针对选区外部未加载区域，采样坐标被严格钳位至选区有效边界，彻底杜绝边界处因外推默认值产生 Plains 平原默认色彩渗色。

---

### 4.5 增量网格重构 (`DeltaMesher`)

在运行时发生方块放置或破坏时，调用 `DeltaMesher::rebuild_dirty_sections`：
- 仅提取 `world.dirty_sections` 标记的区块进行邻域重建；
- 支持 `find_affected_sections`：对于普通方块变动，自动扩散并脏化相邻的 6 向邻域区块；对于流体变动，自动将脏标记扩展至 $3\times 3\times 3$ 邻域。

---

## 5. 快速上手示例 (Quick Start)

### 示例 1：构建体素世界并生成标准化网格

```rust
use glam::IVec3;
use mtk_cull::FaceCuller;
use mtk_voxel::mesher::SectionMesher;
use mtk_voxel::storage::VoxelStorage;
use mtk_voxel::types::{CoordinateSystem, MesherConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 初始化 3D 稀疏世界容器
    let mut world = VoxelStorage::new();
    world.set_bounds(0, 0, 0, 16, 16, 16);

    // 2. 放置方块
    world.set_block(0, 0, 0, "minecraft:stone", Some("minecraft:plains"));
    world.set_block(1, 0, 0, "minecraft:dirt", Some("minecraft:plains"));
    world.set_block(0, 1, 0, "minecraft:water[level=0]", Some("minecraft:plains"));

    // 3. 配置网格化器
    let config = MesherConfig {
        coordinate_system: CoordinateSystem::ZUpRightHanded, // 转换为 Blender / Unreal Z-Up
        origin_centered: true,                               // 底部中心对齐
        weld_vertices: true,                                 // 空间顶点焊接
        enable_ao: true,                                     // 开启平滑 AO
        mesh_fluids: true,                                   // 生成流体曲面
        ..Default::default()
    };

    let culler = FaceCuller::default();

    // 4. 提取垫片数据并执行网格化
    let padded = world.get_section_padded_array(IVec3::new(0, 0, 0));
    let mesh = SectionMesher::mesh_section(&padded, &culler, |_| None, &config);

    println!("生成的顶点数: {}", mesh.positions.len());
    println!("生成的三角形索引数: {}", mesh.indices.len());
    assert!(!mesh.is_empty());

    Ok(())
}
```

---

### 示例 2：增量修改与局部网格更新 (`DeltaMesher`)

```rust
use mtk_cull::FaceCuller;
use mtk_voxel::mesher::DeltaMesher;
use mtk_voxel::storage::VoxelStorage;
use mtk_voxel::types::MesherConfig;

fn main() {
    let mut world = VoxelStorage::new();
    world.set_bounds(0, 0, 0, 32, 16, 32);
    let culler = FaceCuller::default();
    let config = MesherConfig::default();

    // 初始设置
    world.set_block(5, 5, 5, "minecraft:diamond_block", None);
    let _initial_meshes = DeltaMesher::rebuild_dirty_sections(&mut world, &culler, |_| None, &config);

    // 模拟运行时修改方块
    let changes = vec![
        (5, 5, 5, "minecraft:air"),
        (6, 5, 5, "minecraft:gold_block"),
    ];
    world.apply_delta_update(0, 0, 0, &changes);

    // 仅重新构建受影响的区块
    let updated_meshes = DeltaMesher::rebuild_dirty_sections(&mut world, &culler, |_| None, &config);
    println!("受影响并重构的区块数: {}", updated_meshes.len());
}
```

---

### 示例 3：实现自定义 `VoxelSource` 外部数据源

```rust
use glam::IVec3;
use mtk_voxel::source::{ingest_from_source, VoxelSource};
use mtk_voxel::storage::{SectionStorage, VoxelStorage};
use mtk_voxel::types::VoxelError;

struct ProceduralSphereSource {
    radius: f32,
}

impl VoxelSource for ProceduralSphereSource {
    fn source_name(&self) -> &str {
        "ProceduralSphere"
    }

    fn has_section(&self, sx: i32, sy: i32, sz: i32) -> bool {
        sx.abs() <= 1 && sy.abs() <= 1 && sz.abs() <= 1
    }

    fn load_section(&mut self, sx: i32, sy: i32, sz: i32) -> Result<Option<SectionStorage>, VoxelError> {
        let coord = IVec3::new(sx, sy, sz);
        let mut section = SectionStorage::new(coord);
        let base_x = sx * 16;
        let base_y = sy * 16;
        let base_z = sz * 16;

        for x in 0..16 {
            for y in 0..16 {
                for z in 0..16 {
                    let wx = (base_x + x) as f32;
                    let wy = (base_y + y) as f32;
                    let wz = (base_z + z) as f32;
                    if (wx * wx + wy * wy + wz * wz).sqrt() <= self.radius {
                        section.set_local(x as usize, y as usize, z as usize, "minecraft:stone");
                    }
                }
            }
        }

        Ok(if section.is_empty() { None } else { Some(section) })
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut source = ProceduralSphereSource { radius: 12.0 };
    let mut storage = VoxelStorage::new();

    let loaded = ingest_from_source(&mut source, &mut storage, Some(&[IVec3::new(0, 0, 0)]))?;
    println!("成功灌入 {} 个区块", loaded);
    assert_eq!(storage.get_block(0, 0, 0), "minecraft:stone");

    Ok(())
}
```

---

## 6. Feature 开关与依赖

`mtk-voxel` 在 `Cargo.toml` 中提供精细化的编译特性开关：

```toml
[features]
default = ["std", "parallel", "serde"]
std = ["mtk-core/std", "mtk-cull/std", "mtk-model/std", "mtk-resource/std", "mtk-texture/std"]
parallel = ["dep:rayon", "mtk-model/parallel", "mtk-resource/parallel", "mtk-texture/parallel"]
serde = ["dep:serde", "dep:serde_json", "mtk-core/serde", "mtk-cull/serde"]
```

- **`parallel`** (默认开启)：引入 `rayon` 启用多区块切片并行网格化 (`SectionMesher::mesh_sections`)。
- **`serde`** (默认开启)：为 `SectionStorage`、`VoxelStorage`、`MesherConfig`、`WorldMeshBuildResult` 等结构体提供序列化与反序列化支持。

# libmozitoolkit 核心抽象与 API 导航手册 (API Reference)

`libmozitoolkit` (`libmtk`) 采用**原子化、高内聚、分层治理**的模块架构。各子模块的详细 API 签名、内部数据模型与专属代码示例已由各 Crate 专属 `README.md` 作为第一事实源维护。

本文档定义 **全局跨模块通用核心契约 (System-Wide Contracts)**，并提供全工作区模块的快速 API 导航矩阵。

---

## 1. 模块 API 导航矩阵 (Crates & Bindings Map)

| 层次 (Layer) | Crate / Binding | 核心定位与职责 | API 与技术手册 |
| :--- | :--- | :--- | :--- |
| **Layer 0: 基础几何** | **[`mtk-core`](../crates/mtk-core/README.md)** | 标准连续几何缓冲 (`MeshData`)、6向拓扑与标准化右手坐标转换、自适应像素网格切分 (`subdivide`)、智能挤出与 UV 修复 (`extrude`) | [查看 API 手册 ↗](../crates/mtk-core/README.md) |
| **Layer 1: 遮挡与清洗** | **[`mtk-cull`](../crates/mtk-cull/README.md)** | 1.21+ 邻域遮挡状态机 (`FaceCuller`)、2D 矩形差集切分 (Subtract Rect)、外部网格叠面消重 (`MeshSanitizer`) | [查看 API 手册 ↗](../crates/mtk-cull/README.md) |
| **Layer 1: 模型与状态** | **[`mtk-model`](../crates/mtk-model/README.md)** | BlockState 状态机解析 (multipart/variants)、1.21+ Block Model JSON 递归继承展开与几何烘焙、`BakedModelDatabase` 多级回退、OBJ 加载 | [查看 API 手册 ↗](../crates/mtk-model/README.md) |
| **Layer 1: 资源包 VFS** | **[`mtk-resource`](../crates/mtk-resource/README.md)** | 虚拟文件系统 (Zip/Jar/Dir)、PBR 多层资源栈合并、`.png.mcmeta` 动图元数据、`atlases/*.json` 规范与 CTM 47/17 连接纹理求解器 | [查看 API 手册 ↗](../crates/mtk-resource/README.md) |
| **Layer 1: 贴图与图集** | **[`mtk-texture`](../crates/mtk-texture/README.md)** | 2D 矩形二叉装箱 Stitcher、多通道 PBR 图集烘焙、盔甲调色板置换、抗渗色填充与 Standalone PBR 资产库生成器 | [查看 API 手册 ↗](../crates/mtk-texture/README.md) |
| **Layer 1: 材质与生物群系** | **[`mtk-material`](../crates/mtk-material/README.md)** | 66 种原版生物群系调色板与色彩数学权威唯一事实源 (SSOT)、`BiomeResolver` 染色模型扫描器、Rayon 并行多 UV 重映射与别名解析 | [查看 API 手册 ↗](../crates/mtk-material/README.md) |
| **Layer 1: 体素世界与网格化** | **[`mtk-voxel`](../crates/mtk-voxel/README.md)** | 16×16×16 区块体素存储 (`SectionStorage` / `VoxelStorage`)、全量无剔除体素点云 (`VoxelPointCloud`)、平滑 AO 算法、物理流体曲面、贪婪网格化、增量局部重构 (`DeltaMesher`)、统一 3D 场景引擎 (`VoxelWorld`) 与统一体素源抽象 (`VoxelSource`) | [查看 API 手册 ↗](../crates/mtk-voxel/README.md) |
| **Layer 1: 实时网络协同** | **[`mtk-sync`](../crates/mtk-sync/README.md)** | 原生多线程 WebSocket 协同客户端 (`SyncClient`)、小端序二进制协议编解码、两阶段流式增量更新与单一世界大网格 (`WorldMeshReady`) 会话管理 | [查看 API 手册 ↗](../crates/mtk-sync/README.md) |
| **Layer 1: 存档与空间切片** | **[`mtk-save`](../crates/mtk-save/README.md)** | 现代 Minecraft 存档加载核心：基于内置零拷贝 NBT、Anvil `.mca` 区域寻址、384 高度解包与按需空间切片流式数据源 (`VoxelSource`) | [查看 API 手册 ↗](../crates/mtk-save/README.md) |
| **Layer 2: 领域聚合门面** | **[`libmtk`](../crates/libmtk/README.md)** | 统一顶层门面、端到端全量资产预编译管线 (`precompile_all_assets`)、统一错误处理系统 (`MtkError`) | [查看 API 手册 ↗](../crates/libmtk/README.md) |
| **Layer 3: Python 绑定** | **[`mtk-py`](../bindings/mtk-py/README.md)** | **P0 核心绑定**：PyO3 驱动的 `libmtk_py` 模块，为 MoziToolKit (Blender 4.2+ 插件) 提供零拷贝 Buffer Protocol / NumPy 视图与极速批处理算子 | [查看 API 手册 ↗](../bindings/mtk-py/README.md) |
| **Layer 3: 独立命令行** | **[`mtk-cli`](../crates/mtk-cli/README.md)** | **P1 核心工具**：独立无头命令行终端工具 (`mtk`)，支持图集烘焙、模型导出、CTM 求解与遮挡校验 | [查看 API 手册 ↗](../crates/mtk-cli/README.md) |
| **Layer 3: 性能基准测试** | **[`mtk-bench`](../crates/mtk-bench/README.md)** | **P1 压测套件**：4000+ 区块段多核网格化与数百万次遮挡剔除压力基准套件 | [查看 API 手册 ↗](../crates/mtk-bench/README.md) |
| **Layer 3: 通用 C-ABI** | **[`mtk-ffi`](../bindings/mtk-ffi/README.md)** | **P2 远期预备**：纯 C-ABI 兼容动态库/静态库与 C 语言头文件 (`mtk.h`)，面向 C/C++、C# (Unity)、Godot、Maya 插件 | [查看 API 手册 ↗](../bindings/mtk-ffi/README.md) |
| **Layer 3: WebAssembly** | **[`mtk-wasm`](../bindings/mtk-wasm/README.md)** | **P2 远期预备**：wasm-bindgen WebAssembly 绑定，暴露线性内存 TypedArray 视图，面向 WebGPU / Three.js 网页预览 | [查看 API 手册 ↗](../bindings/mtk-wasm/README.md) |

---

## 2. 全局通用核心契约 (System-Wide Contracts)

### 2.1 标准化 3D 几何坐标系规范 (Coordinate Systems)
为了杜绝宿主耦合与坐标系混淆，底层内核统一遵循以下坐标变换规范：
- **Minecraft 原始坐标系**：
  - 局部方块坐标：$[0.0, 16.0]$（+X East, +Y Up, +Z South）。
  - 世界体素坐标：整数三元组 $(X, Y, Z)$。
- **标准化右手 Z-Up 坐标系 (Standard Right-Handed Z-Up)**：
  - $+X$ 轴向东 (East), $+Y$ 轴向北 (North), $+Z$ 轴向上 (Up)。
  - `mc_local_to_centered_z_up(lx, ly, lz) -> Vec3`：将局部方块坐标转换为以方块几何中心为原点 $[-0.5, 0.5]$ 的右手 Z-Up 浮点坐标。
  - `mc_world_to_z_up(wx, wy, wz) -> Vec3`：将 Minecraft 世界坐标转换为标准右手 Z-Up 坐标。
- **标准化右手 Y-Up 坐标系 (Standard Right-Handed Y-Up)**：
  - 适用于 WebGPU / Three.js / OpenGL 默认管线，通过 `MesherCoordinateSystem::YUpRightHanded` 选项目标输出。

---

### 2.2 扁平连续内存几何契约 (Flat Buffer Protocol)

为了确保跨 Crate 调用与跨语言（Python / WASM / C-ABI）极速数据传输，所有几何数据均标准化为扁平连续 1D 缓冲区：

#### ① `MeshData` (基础三角网格与 Quad 顶角容器)
```rust
pub struct MeshData {
    pub positions: Vec<[f32; 3]>,           // 连续 3D 顶点位置流 [N, 3]
    pub normals: Vec<[f32; 3]>,             // 连续 3D 法线流 [N, 3]
    pub indices: Vec<u32>,                  // 三角形顶角索引 [M] (M = 3 * Triangles)
    pub quad_indices: Option<Vec<u32>>,     // 四边形顶角索引 [Q * 4] (保留 Quad 拓扑)
    pub uvs: Vec<[f32; 2]>,                 // 主 UV 坐标流 [N, 2]
    pub secondary_uvs: Option<Vec<[f32; 2]>>, // 备用/局部 UV 坐标流 [N, 2]
    pub colors: Option<Vec<[f32; 4]>>,      // 顶点颜色 / Linear RGBA [N, 4]
    pub face_materials: Vec<MaterialSlotId>,// 逐面材质插槽映射 [F]
    pub face_tint_indices: Vec<TintIndex>,  // 逐面染色索引 [F]
    pub custom_attributes: HashMap<String, MeshAttribute>, // 按名称索引的泛型自定义属性 (Point/Corner/Face/Mesh 域)
}
```

#### ② `FlatPolygonMesh` (多边形循环拓扑连续缓冲，对接 Blender Polygon Loops)
```rust
pub struct FlatPolygonMesh {
    pub positions: Vec<[f32; 3]>,           // [N, 3] 顶点坐标
    pub loop_vertices: Vec<u32>,            // [L] Blender Mesh Loop 顶点索引
    pub loop_uvs: Vec<[f32; 2]>,            // [L, 2] 逐角隅 Loop 域 UV 坐标
    pub face_loop_starts: Vec<u32>,         // [F] 逐面 Loop 起始偏移
    pub face_loop_totals: Vec<u32>,         // [F] 逐面顶点总数 (Quad 为 4)
    pub face_materials: Vec<u32>,           // [F] 逐面材质插槽 ID
}
```

---

### 2.3 生物群系调色板与色彩数学单一事实源 (Biome SSOT)
- **唯一权威来源**：`mtk-material::biome`。
- **66 种原版官方生物群系规范表**：包含所有 Overworld, Nether, The End 群系的规范温度、湿度、草方块颜色、树叶颜色与水体颜色。
- **硬编码方块染色与红石信号强度 (Hardcoded Tints & Redstone Wire)**：
  - 云杉树叶、白桦树叶、睡莲与红石引线均为硬编码染色类别 (`TINT_TYPE_HARDCODED = 4`)；
  - 红石引线 (`redstone_wire`) 统一维护 0~15 级标准 sRGB Hex 表与 Linear RGBA 线性渐变色彩，发光强度按 `power / 15.0` 计算；
  - 烘焙网格注入 `mtk_biome_tint_data` (`[1.0, 1.0, 1.0, 4.0]`) 与 `mtk_emission` 属性，着色器阶段无缝自适应渲染。
- **色彩空间规范**：
  - 宿主与渲染器内部色彩混合**必须在标准线性空间（Linear RGBA, $[0.0, 1.0]$）中进行**；
  - 导出/展示贴图时使用权威转换公式双向映射：
    $$\text{Linear} = \begin{cases} \frac{\text{sRGB}}{12.92} & \text{sRGB} \le 0.04045 \\ \left(\frac{\text{sRGB} + 0.055}{1.055}\right)^{2.4} & \text{sRGB} > 0.04045 \end{cases}$$

---

### 2.4 材质物理属性与网格属性驱动单一事实源 (Material Properties SSOT)
- **唯一权威来源**：`mtk-material::properties`。
- **四元组物理属性 (`MaterialProps = [f32; 4]`)**：
  - `emission`: 原版标准 0..15 级发光强度（支持营火、熔炉、红石火把、铜灯等状态机）；
  - `thin_wall`: 树叶、作物、花草薄壁植被半透散射标志（0.0 或 1.0）；
  - `transmission`: 玻璃、水体、冰块等介质物理透射权重（0.0 或 1.0）；
  - `sticker_threshold`: 双层贴纸与折射分离阈值（玻璃 0.55，水体/流体 0.95）。
- **Rayon 并行批处理**：提供 `compute_mesh_material_props` 与扁平数组 `compute_flat_material_props`，直通 Blender 网格 `mtk_material_props` 面域属性（Float4 / FloatColor）。
- **数据驱动配置与免编译动态注入 (Data-Driven Configuration)**：
  - 核心物理属性表从编译期硬编码解耦为外部可配置的 JSON 规范文件（`assets/material_properties.json`）；
  - 提供 `register_material_properties_json(json_str)` 与 `load_material_properties_json_replace(json_str)`，支持上层在运行时免重新编译增量覆盖或替换注册表；
  - 提供 `reset_material_properties_to_default()` 随时复位为内置原版权威配置。

---

### 2.5 统一错误处理系统 (`MtkError`)
`libmtk` 顶层门面将所有子领域错误聚合为单一 `MtkError`，并实现 `From<...>` 自动向上传播，杜绝 panic 崩溃：
```rust
pub enum MtkError {
    Model(mtk_model::ModelError),
    Voxel(mtk_voxel::VoxelError),
    Texture(mtk_texture::error::TextureError),
    Resource(mtk_resource::error::ResourceError),
    Material(mtk_material::MaterialError),
    Save(mtk_save::SaveError),          // feature = "save"
    Io(std::io::Error),
    Json(serde_json::Error),
    Bincode(bincode::Error),
    ThreadPool(String),
}
```

---

### 2.5 资产预编译缓存格式规范 (`ASSET_CACHE_FORMAT_VERSION`)
由 `libmtk::prebake` 产出的资产预编译缓存目录结构受全局版本号严格约束（当前版本：`ASSET_CACHE_FORMAT_VERSION = 1`）：
```
cache_output_dir/
├── cache_manifest.json     # 全局清单、时间戳、CRC 校验与版本号
├── atlas_mapping.json      # 全量材质名 -> 图集坐标/UV 映射表
├── biome_mapping.json      # 全量方块模型染色规则与 Tint 元数据
├── atlases/                # 烘焙完成的 PNG 图集文件
│   ├── blocks_0.png        # 基础 Albedo 图集
│   ├── blocks_0_n.png      # Normal 法线图集
│   └── blocks_0_s.png      # Specular 镜面高光图集
└── models/                 # 烘焙完成的预解析二进制方块几何库
```

---

### 2.6 1:1 Minecraft 原版视觉对齐契约 (Alternate Blocks & Plant Offsets)
为了在无多余网络开销的前提下完美还原原版世界视觉效果，系统采用客户端纯算术空间坐标哈希管线：
- **核心空间哈希与随机数生成器** (`mtk_core::random`):
  - `mc_coordinate_seed(x, y, z) -> i64`: 1:1 对齐 Java 原版 `Mth.getSeed(x, y, z)`。
  - `JavaRandom`: 48 位线性同余伪随机数生成器 (`java.util.Random` / `SingleThreadedRandomSource`)。
- **预烘焙交替模型组** (`mtk_model::baked::BakedVariantGroup`):
  - 针对自然方块（泥土、石头、沙子、草方块、睡莲等）的多旋转分支预烘焙为离散模型，支持按空间坐标种子快速抽样 (`select_by_pos`)。
- **植物空间确定性抖动** (`mtk_core::random::OffsetType`):
  - 对花草、蕨类 (`OffsetType::XZ`) 及竹子 (`OffsetType::XYZ`) 依 `Mth.getSeed(x, 0, z)` 沿水平轴进行 $\pm 0.25$ 偏移，两格高植物上下层茎秆水平位移严格保持一致。
- **网格化源抽象与配置** (`mtk_voxel`):
  - `ModelSource`: 统一表征单模型 (`Single`)、预烘焙变体组 (`Variant`) 或纯方块回退 (`None`)。
  - `MesherConfig.enable_alternate_blocks` (默认 `true`): 控制是否启用交替模型采样。
  - `MesherConfig.enable_random_offsets` (默认 `true`): 控制是否启用植物坐标抖动。

---

### 2.7 现代 Minecraft 存档加载契约 (`mtk-save` & Spatial Slicing)
为支持现代 Minecraft 1.18+ / 1.20+ / 1.21+ (`DataVersion` >= 2844) 大规模存档按需读取与极速渲染，系统建立了流式空间切片与零拷贝解包契约：
- **`LevelData`**: 从 `level.dat` 零拷贝读取世界元数据（`level_name`、`version_name`、`data_version`、`spawn` 三维坐标、`time`、`day_time`、`hardcore` 等）。
- **`RegionFile`**: 针对 32x32 区块 Anvil `.mca` 文件直接寻址 4096 字节 Location Table，按扇区偏移直接 Seek 并仅解压目标区块。
- **`ChunkParser`**: 解密现代 Section 结构，对紧凑长整型数组（`bits_per_block >= 4`）进行纯位解包，并将 Palette 状态规范化为唯一排序属性字符串（如 `minecraft:oak_stairs[facing=north,half=bottom]`）。
- **按需切片加载器 (`SaveLoader`)**:
  - `load_box_into_storage(save_dir, dimension, min_block, max_block, &mut storage)`: 依据用户 3D 边界盒直接定位所需相交的 Region / Chunk / Section，仅加载并填充所选区域进入 `VoxelStorage`，彻底杜绝数十 GB 全图扫描。
  - `AnvilWorldSource`: 遵循统一 `VoxelSource` 接口，支持向 `VoxelWorld` 持续提供体素切片流。

---

### 2.8 真实物理进度上报与节流契约 (Physical Progress Reporting & Throttling)
为彻底杜绝长耗时计算中的黑盒假死与估算假进度，底层内核在 `mtk-core` 建立了通用的真实物理进度上报契约：
- **`ProgressReport`**: 包含阶段标识符 `stage`（如 `"meshing_sections"`、`"assembling_world_mesh"`）、真实物理已完成计数 `current`、总任务数 `total`、描述文本 `message` 与百分比计算 `percent()`。
- **`ProgressCallback`**: 跨线程回调契约 `&dyn Fn(ProgressReport) + Send + Sync`。
- **`ProgressThrottler`**: 面向多线程（Rayon）高频任务的原子无锁节流器，通过 `inc()` / `inc_by()` 配合步进控制（默认 1% 或每完成 $N$ 项触发一次），杜绝跨语言锁争抢并 100% 确保终态触发。
- **全链路模块接入 (`mtk-voxel`, `mtk-sync`, `mtk-save`, `libmtk::prebake`)**:
  - `VoxelWorld::rebuild_all_with_progress(callback)`: 在 Rayon 并行重构体素网格时，实时将各 Chunk Section 的真实烘焙进度向外派发。
  - `mtk-sync`: 实时将网格烘焙进度转化为 `SyncEvent::StreamProgress` 并向前端推送，配合 Blender 宿主状态栏 API（`wm.progress_*` 与 `workspace.status_text_set`）实现全链路底栏真实进度条。
  - `SaveLoader::load_box_into_storage_with_progress(...)`: 在遍历 Anvil MCA 区域与解压 Chunk NBT 期间实时汇报 `load_chunks` 物理进度，使世界存档导入在 MCA 解压与网格构建全流程透明可见。
  - `libmtk::precompile_all_assets_with_progress(...)` / `prebake_all_models_with_progress(...)`: 在端到端材质与资产预编译流程中，按阶段分发真实物理计数（`prebake_biome`、`prebake_atlas`、`prebake_standalone`、`prebake_models`、`prebake_manifest`），使图集拼接、独立材质对齐与模型变体烘焙全流程实时反馈在 DCC 宿主底栏。

---

### 2.9 通用二进制包与场景交换契约 (`MTKP` / Package Specification)
为了根除散文件磁盘 I/O 碎片，并在 Minecraft 场景交换中彻底避免传统 3D 网格的“几何爆炸”，系统定义了统一的通用二进制中间包契约：
- **`Header & TOC` 架构**：固定 64 字节文件头、64 字节对齐分块、Zstd 块级压缩索引表，支持 `mmap` 零内存开销快速寻址。
- **两大业务 Profile**：
  1. **`AssetCache Profile` (`.mtkcache`)**：存储全量预烘焙图集、Standalone PBR 贴图库与全局模型数据库。
  2. **`SceneInterchange Profile` (`.mtkscene`)**：轻量级自包含场景交换包。仅存储稀疏体素网格（`VOXL`）、场景实际引用的独立贴图（`TXTR`，非固化图集）与剪枝方块模型（`MODL`），由接收方现场根据宿主配置实时重构网格。
- 完整规范定义参见：[**`docs/PACKAGE_SPEC.md`**](./PACKAGE_SPEC.md)。




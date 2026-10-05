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

### 2.4 统一错误处理系统 (`MtkError`)
所有对外公开的高阶 API 均返回 `Result<T, MtkError>`，杜绝 panic 崩溃：
```rust
pub enum MtkError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Image(image::ImageError),
    ResourceNotFound(String),
    ModelNotFound(String),
    InvalidTextureReference(String),
    AtlasStitchFailed(String),
    CacheVersionMismatch { expected: u32, found: u32 },
    SyncError(String),
    Custom(String),
}
```

---

### 2.5 资产预编译缓存格式规范 (`ASSET_CACHE_FORMAT_VERSION`)
由 `libmtk::prebake` 产出的资产预编译缓存目录结构受全局版本号严格约束（当前版本：`ASSET_CACHE_FORMAT_VERSION = 3`）：
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


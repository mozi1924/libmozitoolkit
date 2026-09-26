# GEMINI.md - AI Agent 协作与开发规范

本文档是供所有 AI Coding Agent（如 Antigravity / Gemini / Claude）以及核心开发者阅读的统一协作指南与项目宪章。

---

## 1. 工作区概览 (Workspaces Map)

在当前开发环境中，存在 3 个协同工作区，各司其职且职责边界严格清晰：

| 工作区路径 | 项目名称 / 角色 | 核心职责与定位 |
| :--- | :--- | :--- |
| **`../libmozitoolkit`** | **`libmtk` (Rust Core)** | **通用 3D / Voxel 处理引擎核心**。纯 Rust 实现，严格遵循 Host-Agnostic（宿主无关）、纯数据输入输出（Data-in, Data-out）、高性能多端兼容（WASM / Python / FFI / C-ABI）。 |
| **`../MoziToolKit`** | **`MoziToolKit` (Blender Addon)** | **DCC 宿主插件前端**。负责 Blender 4.2+ UI 交互、操作符注册、材质节点构建以及通过 `bridge/` 层调用 `libmtk` 导出的 Python 扩展极速灌入网格与贴图数据。 |
| **`../MiEx`** | **`MiEx` (Reference & Ecosystem)** | **Minecraft / Hytale 世界导出工具**（Java / USD）。用于参考生态资产导出标准、验证世界数据流结构，并作为大规模场景和模型的测试资产源。 |

---

## 2. 核心规约与必须遵守的铁律 (Golden Rules)

### 规则 1：文档同步维护 (Documentation Integrity)
- 当发生任何**架构设计调整**、**Crate 依赖变更**、**公共 API (Public API) 增删或签名变更**、**内存数据布局 (MeshData / Buffer Layout) 调整**时，**必须同步更新** `docs/` 下的内部约定文档（如 `docs/ARCHITECTURE.md`、`docs/API_REFERENCE.md`、`docs/BINDINGS_DESIGN.md`）。

### 规则 2：Crate 变更同步 README (Workspace Sync)
- 当在 `crates/` 或 `bindings/` 下**创建新的 Crate**、**删除 Crate** 或**调整 Crate 命名与定位**时，**必须立即更新**根目录 `README.md` 中的“模块结构 (Workspace Crates)”表格及 `Cargo.toml` 的 workspace 配置。

### 规则 3：严格的宿主无关与无头设计 (Host-Agnostic & Headless)
- **Rust 核心层（`crates/mtk-*` 与 `crates/libmtk`）绝不允许引入任何特定 DCC / 宿主软件的专有库或 API**（例如 Blender C API、Maya API、Unity API 等）。
- 核心层只接受标准 POD 数据、通用几何流（Flat Buffers / Indices / UVs / Vertices）与无头图像/贴图内存，产出标准网格与材质描述。
- **坐标系命名规范**：全面采用标准化 3D 几何坐标系命名（如 `z_up_coordinates`、`mc_local_to_centered_z_up`、`mc_world_to_z_up`），严禁在底层出现 `blender_coordinates` 等 DCC 专属标识符。
- 所有的宿主专有操作（如 Blender 的 `bpy.data.meshes.new`、材质节点树链接）必须且只能在外部宿主胶水层（Python / C# / JS）完成。

### 规则 4：胶水层轻量化与 Data-In Data-Out 契约 (Bindings & Zero-Copy Policy)
- 胶水层（`bindings/mtk-py`、`bindings/mtk-ffi`、`bindings/mtk-wasm`）只负责**跨语言数据类型转换与内存生命周期桥接**，严禁在胶水层编写复杂计算逻辑。
- 针对大块几何（顶点/法线/UV/索引）与贴图像素数据，优先采用连续内存缓冲（Buffer Protocol / memoryview / TypedArray）实现零拷贝或极低拷贝传输。
- 几何与 UV 算子（如挤出修复、随机挤出、自适应像素网格切分）必须遵循统一的 **Data In, Data Out** 批处理契约，支持保留 Quad 四边形拓扑结构与顶点/面属性（顶点色、蒙皮权重、自定义图层）的双线性插值。

### 规则 5：生物群系与调色板单一事实源 (Biome Palette SSOT)
- 所有生物群系（Biome）、方块 Tint 与颜色映射逻辑必须且只能以 `mtk-material::biome` 为唯一权威事实源（Single Source of Truth, SSOT），内置 66 种原版规范调色板与线性色彩数学（sRGB 与 Linear RGBA 双向转换）。
- 严禁在其他子模块（如 `mtk-voxel`）或宿主前端中分散、硬编码重复的生物群系调色板。

### 规则 6：工作区虚拟环境与轮子编译规范 (Workspace Venv & Wheel Build Policy)
- 当需要使用 `maturin` 编译 Python 绑定轮子（Wheel, `.whl`）或进行 Python 绑定调试时，**必须严格使用工作区内的虚拟环境（如 `/home/mozi/libmozitoolkit/.venv`）**。
- 若当前工作区内不存在虚拟环境，**必须首先在工作区根目录下创建专属虚拟环境**（如 `python3 -m venv .venv`），并在该虚拟环境中安装 `maturin`，严禁污染或依赖宿主系统全局环境。
- 编译完成的 Release 轮子包（`target/wheels/*.whl`）需及时同步拷贝至 Blender 插件前端目录（`/home/mozi/MoziToolKit/wheels/`）供插件端使用，并进行端到端测试。

### 规则 7：资产缓存与预编译规范 (Precompile Cache Contract)
- 资产预烘焙（Atlas 拼接、Standalone PBR 对齐、Model Baking、BiomeResolver 映射预提取）由 `libmtk::prebake` 统一无头处理，产出格式受 `ASSET_CACHE_FORMAT_VERSION` 版本约束。
- 宿主端优先从编译缓存加载（`atlas_mapping.json`、`biome_mapping.json`、`cache_manifest.json`），避免运行时对大体积 ZIP/JAR 资产包进行重复解析。

---

## 3. 核心领域架构与模块划分

```
libmozitoolkit/
├── crates/
│   ├── mtk-core/       -> 基础几何基元、Quad、MeshData、自适应像素切分与挤出算子
│   ├── mtk-cull/       -> 6 向邻域遮挡状态机、面剔除、2D 矩形差集切分
│   ├── mtk-model/      -> BlockState 状态解析、1.21+ Block Model JSON 烘焙、OBJ 解析
│   ├── mtk-voxel/      -> 16x16x16 Chunk Section 体素存储、流体曲面、平滑 AO 与网络协同
│   ├── mtk-resource/   -> 虚拟文件系统 (VFS)、.mcmeta 动图元数据、原版 atlases/*.json 解析
│   ├── mtk-texture/    -> 矩形装箱 Stitcher、多类别 PBR 图集与 Overlay 烘焙、Standalone 转换
│   ├── mtk-material/   -> 66 生物群系调色板引擎、BiomeResolver、并行 UV 重映射与别名解析
│   ├── libmtk/         -> 统一顶层门面、端到端预编译管线 (Prebake) 与统一错误处理
│   ├── mtk-bench/      -> 性能压测与基准测试套件
│   └── mtk-cli/        -> 独立命令行工具 (mtk precompile / bench / inspect)
└── bindings/
    ├── mtk-py/         -> PyO3 + maturin Python 绑定 (libmtk_py)
    ├── mtk-ffi/        -> C-ABI 动态/静态库与 C 头文件 (cbindgen)
    └── mtk-wasm/       -> wasm-bindgen WebAssembly 绑定
```

---

## 4. 目录与内部文档索引

- **系统架构设计**：[`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md)
- **公共 API 与核心抽象参考**：[`docs/API_REFERENCE.md`](docs/API_REFERENCE.md)
- **跨语言绑定与胶水层规范**：[`docs/BINDINGS_DESIGN.md`](docs/BINDINGS_DESIGN.md)

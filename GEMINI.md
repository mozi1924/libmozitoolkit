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

### 规则 6：工作区虚拟环境、双态联动与轮子编译规范 (Workspace Venv & Dual-Mode Build Policy)
- 当需要使用 `maturin` 编译 Python 绑定轮子（Wheel, `.whl`）或进行 Python 绑定调试时，**必须严格使用工作区内的虚拟环境（如 `/home/mozi/libmozitoolkit/.venv`）**。
- 若当前工作区内不存在虚拟环境，**必须首先在工作区根目录下创建专属虚拟环境**（如 `python3 -m venv .venv`），并在该虚拟环境中安装 `maturin`，严禁污染或依赖宿主系统全局环境。
- **开发态直通与发布态隔离 (Dev Direct-Link vs Release Wheel Isolation)**：
  - **日常开发态 (Dev Direct Link)**：`cargo build --release -p mtk-py --features extension-module` 产出 `target/release/liblibmtk_py.so`。通过软链接（`MoziToolKit/dev/lib/libmtk_py.so`）直接打通 Blender 宿主，代码改动后 Blender 重载插件即可即时生效，严禁在开发期频繁构建或安装轮子。
  - **发布态 (Release Packaging)**：对外正式打包时通过 `maturin build --release` 产出符合 CPython 3.11+ 标准 ABI (`abi3`) 的规范 Wheel 包，由 `MoziToolKit/build.py` 在发布阶段统一装配并隔离。
  - **严禁将编译产物直接安装（`pip install`）到宿主系统的全局 Python 或 Blender 全局 `bpy` 环境中**，杜绝破坏环境隔离与沙箱完整性。

### 规则 7：资产缓存与预编译规范 (Precompile Cache Contract)
- 资产预烘焙（Atlas 拼接、Standalone PBR 对齐、Model Baking、BiomeResolver 映射预提取）由 `libmtk::prebake` 统一无头处理，产出格式受 `ASSET_CACHE_FORMAT_VERSION` 版本约束。
- 宿主端优先从编译缓存加载（`atlas_mapping.json`、`biome_mapping.json`、`cache_manifest.json`），避免运行时对大体积 ZIP/JAR 资产包进行重复解析。

### 规则 8：单文件规模红线与模块化防膨胀 (File Length Redline & Submodule Decomposition)
- **源码文件行数上限原则**：任何 Rust 源码文件（除集中测试用例 `tests/` 外）常规行数**建议控制在 500 行以内，硬性红线严禁突破 700~800 行**。
- **严禁无界追加巨石代码**：当向现有模块新增功能时，若预估或实际导致文件接近 600 行，**必须主动创建子模块**（如 `mod.rs` + 子领域文件，参考 `mtk-voxel::storage::world`、`mtk-sync::session`、`mtk-cull::rules` 的模块化实践）进行职责解耦，严禁在一个文件中无休止地堆砌长篇代码。
- **严格禁止领域越界污染**：
  - 数据模型层（如 `mtk-model`）严禁内联编写复杂的 2D/3D 几何投影、多边形裁剪或叠面碰撞检测算法；
  - 所有通用几何清理、共面去重与微观遮挡算法**必须且只能归属 `mtk-core` 或 `mtk-cull`（如 `mtk-cull::FaceSanitizer`）**。

### 规则 9：Crate 级 README 与文档单一事实源 (Crate README & Doc Integrity)
- **Crate 专属说明不可缺失**：`crates/` 与 `bindings/` 下的所有子 Crate **必须包含专属 `README.md`**，包含其架构定位、输入输出契约、核心公共结构体与方法、以及一个极简使用示例。
- **公共 API 变动强制双向维护**：当子 Crate 的 Public API 发生变更或新增导出时，除更新根目录 `docs/API_REFERENCE.md` 外，**必须同步维护该 Crate 自身的 `README.md`**。

### 规则 10：内存安全、可见性与 Panic 防御规约 (Safety, Visibility & Panic Defense)
- **禁止在生产代码中滥用 Panic**：胶水层（`bindings/mtk-py`, `bindings/mtk-ffi`, `bindings/mtk-wasm`）严禁在非测试代码中随意调用 `.unwrap()` / `.expect()`；必须通过 `Result` / `PyResult` 进行优雅的错误向上传播，杜绝因 Rust panic 导致宿主进程（如 Blender 桌面端）无预警崩溃。
- **最小可见性原则**：严格控制结构体字段与内部辅助函数的可见性，严禁为了跨模块调用方便而无脑使用 `pub`，优先使用 `pub(crate)`、私有字段或显式 getter/setter 维持类型的不变性保证。

### 规则 11：架构设计与 CLI 一致性原则 (Feature & CLI Parity)
- 根目录文档与架构手册中声明的 CLI 命令行工具（`mtk-cli`）命令（如 `precompile`、`inspect` 等），必须严格对齐底层真实实现或明确注明开发阶段，严禁在文档中宣称未实现的命令，杜绝设计脱节。

---

## 3. 核心领域架构与模块划分

```
libmozitoolkit/
├── crates/
│   ├── mtk-core/       -> 基础几何基元、Quad、MeshData、自适应像素切分与挤出算子
│   ├── mtk-cull/       -> 6 向邻域遮挡状态机、FaceSanitizer 叠面消重清洗器、2D 矩形差集切分
│   ├── mtk-model/      -> BlockState 状态解析、1.21+ Block Model JSON 烘焙、OBJ 解析
│   ├── mtk-voxel/      -> 16x16x16 Chunk Section 体素存储、流体曲面、平滑 AO 与体素源抽象
│   ├── mtk-sync/       -> 原生 WebSocket 实时同步客户端、小端序二进制协议与会话管理
│   ├── mtk-resource/   -> 虚拟文件系统 (VFS)、.mcmeta 动图元数据、原版 atlases/*.json 解析
│   ├── mtk-texture/    -> 矩形装箱 Stitcher、多类别 PBR 图集与 Overlay 烘焙、Standalone 转换
│   ├── mtk-material/   -> 66 生物群系调色板引擎、BiomeResolver、并行 UV 重映射与别名解析
│   ├── libmtk/         -> 统一顶层门面、端到端预编译管线 (Prebake) 与统一错误处理
│   ├── mtk-bench/      -> 性能压测与基准测试套件
│   └── mtk-cli/        -> 独立命令行工具 (mtk)
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


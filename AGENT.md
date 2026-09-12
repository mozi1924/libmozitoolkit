# AGENT.md - AI Agent 协作与开发规范

本文档是供所有 AI Coding Agent（如 Antigravity / Gemini）以及核心开发者阅读的统一协作指南与项目宪章。

---

## 1. 工作区概览 (Workspaces Map)

在当前开发环境中，存在 4 个核心工作区，各司其职且职责边界严格清晰：

| 工作区路径 | 项目名称 / 角色 | 核心职责与定位 |
| :--- | :--- | :--- |
| **`/home/mozi/libmozitoolkit`** | **`libmtk` (Rust Core)** | **通用 3D / Voxel 处理引擎核心**。纯 Rust 实现，严格遵循 Host-Agnostic（宿主无关）、纯数据输入输出（Data-in, Data-out）、高性能多端兼容（WASM / Python / FFI / C-ABI）。 |
| **`/home/mozi/MoziToolKit`** | **`MoziToolKit` (Blender Addon)** | **DCC 宿主插件前端**。负责 Blender UI 交互、操作符注册、材质节点构建以及通过 `libmtk` 导出的 Python 绑定/动态库高效灌入网格与贴图数据。 |
| **`/home/mozi/MiEx`** | **`MiEx` (Data Extractor)** | **Minecraft 资产与数据提取器**。负责从 Minecraft Jar / 存档中提取 BlockState、Model JSON、纹理图集、生物群系等原始资产数据。 |
| **`/home/mozi/mc`** | **`mc` (Assets & Testing)** | **测试资产与环境**。包含用于集成测试的 Minecraft 原版/Mod 资源包、测试世界存档、OBJ/GLTF 样本等。 |

---

## 2. 核心规约与必须遵守的铁律 (Golden Rules)

### 规则 1：文档同步维护 (Documentation Integrity)
- 当发生任何**架构设计调整**、**Crate 依赖变更**、**公共 API (Public API) 增删或签名变更**、**内存数据布局 (MeshData / Buffer Layout) 调整**时，**必须同步更新** `docs/` 下的内部约定文档（如 `docs/ARCHITECTURE.md`、`docs/API_REFERENCE.md`、`docs/BINDINGS_DESIGN.md`）。

### 规则 2：Crate 变更同步 README (Workspace Sync)
- 当在 `crates/` 下**创建新的 Crate**、**删除 Crate** 或**调整 Crate 命名与定位**时，**必须立即更新**根目录的 `README.md` 中的“模块结构 (Workspace Crates)”表格及 `Cargo.toml` workspace 配置。

### 规则 3：严格的宿主无关与无头设计 (Host-Agnostic & Headless)
- **Rust 核心层（`crates/mtk-*` 与 `crates/libmtk`）绝不允许引入任何特定 DCC / 宿主软件的专有库或 API**（例如 Blender C API、Maya API、Unity API 等）。
- 核心层只接受标准 POD 数据、通用几何流（Flat Buffers / Indices / UVs / Vertices）与无头图像/贴图内存，产出标准网格与材质描述。
- 所有的宿主专有操作（如 Blender 的 `bpy.data.meshes.new`、材质节点树链接）必须且只能在外部宿主胶水层（Python / C# / JS）完成。

### 规则 4：胶水层轻量化与零拷贝原则 (Bindings Policy)
- 胶水层（`crates/mtk-py`、`crates/mtk-ffi`、`crates/mtk-wasm`）只负责**跨语言数据类型转换与内存生命周期桥接**，禁止在胶水层编写复杂计算逻辑。
- 针对大块几何（顶点/法线/UV/索引）与贴图像素数据，优先采用连续内存缓冲（Buffer Protocol / Raw Pointers / TypedArray）实现零拷贝或单次拷贝传输。

### 规则 5：工作区虚拟环境与轮子编译规范 (Workspace Venv & Wheel Build Policy)
- 当需要使用 `maturin` 编译 Python 绑定轮子（Wheel, `.whl`）或进行 Python 绑定调试时，**必须严格使用工作区内的虚拟环境（如 `/home/mozi/libmozitoolkit/.venv`）**。
- 若当前工作区内不存在虚拟环境，**必须首先在工作区根目录下创建专属虚拟环境**（如 `python3 -m venv .venv`），并在该虚拟环境中安装 `maturin`，严禁污染或依赖宿主系统全局环境。
- 编译完成的 Release 轮子包（`target/wheels/*.whl`）需及时同步拷贝至 Blender 插件前端目录（`/home/mozi/MoziToolKit/wheels/`）供插件端使用。

---

## 3. 目录与内部文档索引

- **系统架构设计**：[`docs/ARCHITECTURE.md`](file:///home/mozi/libmozitoolkit/docs/ARCHITECTURE.md)
- **公共 API 与核心抽象参考**：[`docs/API_REFERENCE.md`](file:///home/mozi/libmozitoolkit/docs/API_REFERENCE.md)
- **跨语言绑定与胶水层规范**：[`docs/BINDINGS_DESIGN.md`](file:///home/mozi/libmozitoolkit/docs/BINDINGS_DESIGN.md)


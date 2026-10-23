# GEMINI.md - AI Agent 协作与开发规范

本文档是供所有 AI Coding Agent（如 Antigravity / Gemini / Claude）以及核心开发者阅读的统一协作指南与项目宪章。

---

## 1. 工作区概览 (Workspaces Map)

在当前开发环境中，存在 3 个协同工作区，各司其职且职责边界严格清晰：

| 工作区路径 | 项目名称 / 角色 | 核心职责与定位 |
| :--- | :--- | :--- |
| **`../libmozitoolkit`** | **`libmtk` (Rust Core - 当前工作区)** | **通用 3D / Voxel 处理引擎核心**。纯 Rust 实现，严格遵循 Host-Agnostic（宿主无关）、纯数据输入输出（Data-in, Data-out）、高性能跨平台。产出通用 Python 绑定 (`libmtk_py`) 与独立 Blender 宿主内存适配器 (`libmtk_blender_dna`)。 |
| **`../MoziToolKit`** | **`MoziToolKit` (Blender Addon)** | **DCC 宿主插件前端**。负责 Blender 4.2+ UI 交互、操作符、标准 PBR 节点构建、场景装配，通过纯数据 `bridge/` 层消费底层 Rust 核心算子与 DNA 内存直灌。 |
| **`../yefira`** | **`yefira` (Live Sync Server Mod)** | **Minecraft 实时协同服务端模组**。基于 Fabric / Forge / NeoForge 的跨版本伴生模组，提供服务端权威体素扫描 (CanonicalVoxelScanner)、XXHash64 校验自愈、增量批分片与 RFC v3.1 WebSocket 协同流。 |

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

### 规则 4：胶水层轻量化、最小暴露面与 Data-In Data-Out 契约 (Bindings & Zero-Copy Policy)
- **胶水层职责单一**：`bindings/mtk-py` 只负责跨语言数据类型转换与内存生命周期桥接，严禁在胶水层编写复杂计算逻辑。
- **零调用方绝对清除 (No Zero-Consumer APIs)**：Python 绑定与公共 API 严格保持极简主义，严禁暴露前端未消费的冗余方法或保留废弃 shim。
- **连续内存直传优先**：针对几何（顶点/法线/UV/索引）与贴图像素数据，优先采用连续内存缓冲（Buffer Protocol / memoryview）或通过独立适配器（`mtk-blender-dna`）实现零拷贝传输。
- **统一数据契约**：几何与 UV 算子（挤出修复、随机挤出、自适应像素网格切分）必须遵循统一的 **Data In, Data Out** 批处理契约，保留 Quad 四边形拓扑结构与属性双线性插值。

### 规则 5：生物群系与调色板单一事实源 (Biome Palette SSOT)
- 所有生物群系（Biome）、方块 Tint 与颜色映射逻辑必须且只能以 `mtk-material::biome` 为唯一权威事实源（Single Source of Truth, SSOT），内置 66 种原版规范调色板与线性色彩数学（sRGB 与 Linear RGBA 双向转换）。
- 严禁在其他子模块（如 `mtk-voxel`）或宿主前端中分散、硬编码重复的生物群系调色板。

### 规则 6：双轮子体系与双态开发构建规范 (Dual-Wheel & Dual-Mode Build Policy)
- **双轮子划分**：
  - `bindings/mtk-py`：编译产出通用的 `libmtk_py` 扩展；
  - `adapters/blender/mtk-blender-dna-py`：编译产出专为 Blender C++ 内存直写打造的独立 `libmtk_blender_dna` 适配器扩展。
- **工作区虚拟环境约束**：编译与调试 Python 绑定轮子时，必须严格使用工作区内的虚拟环境（`./.venv`），严禁污染全局环境。
- **开发态直通与发布态隔离 (Dev Direct-Link vs Release Wheel Isolation)**：
  - **日常开发态 (Dev Direct Link)**：使用 Cargo 标准别名直通构建（推荐 `cargo build-all-py` 一键构建双动态库，或分别使用 `cargo build-py` 与 `cargo build-dna`；Debug 版对应 `-dev` 后缀）。别名已默认且必须附带 `--features extension-module`，以杜绝 Blender 动态载入时的双 Python 运行时符号冲突与段错误崩溃。
  - **严禁在 `Cargo.toml` 的 `default` 中开启 `extension-module`**：由于 Rust `cargo test` 会编译独立可执行测试二进制，默认开启会导致测试二进制因缺失 `libpython` 符号而无法链接，破坏 `cargo test --workspace` 门禁。因此 `extension-module` 只能通过 Cargo 别名（开发态）或 `maturin` / `pyproject.toml`（发布态）按需激活。
  - **软链接直通**：构建产物通过软链接（`MoziToolKit/dev/lib/`）直通 Blender 插件前端，修改编译后即时生效，开发期严禁频繁构建安装轮子。
  - **发布态 (Release Packaging)**：通过 `maturin build --release` 产出符合 CPython 3.11+ 标准 ABI (`abi3`) 的跨平台 Wheels（支持 6 平台矩阵），由发布流水线统一装配。
  - **严禁全局安装**：严禁通过 `pip install` 将轮子安装到宿主系统的全局 Python 或 Blender 全局 `bpy` 环境中。

### 规则 7：资产缓存与预编译标准 PBR 规范 (Precompile Cache & Canonical PBR)
- **PBR 预解码权威通道**：资产预烘焙（Atlas 拼接、Standalone PBR 对齐、Model Baking、BiomeResolver）由 `libmtk::prebake` 统一无头处理，版本受 `ASSET_CACHE_FORMAT_VERSION = 5` 约束。
- **彻底告别运行时 LabPBR**：预编译期直接将 LabPBR 解码为规范 PBR 图集通道（Albedo, Normal_GL, Roughness, Metallic, Aux/Emission），丢弃原始 `_n` / `_s`。彻底废弃 `mtk_material_props` 物理属性表。

### 规则 8：单文件规模红线与模块化防膨胀 (File Length Redline & Submodule Decomposition)
- **源码文件行数上限原则**：任何 Rust 源码文件（除集中测试用例 `tests/` 外）常规行数建议控制在 500 行以内，硬性红线严禁突破 700~800 行。
- **严禁无界追加巨石代码**：当向现有模块新增功能时，若预估或实际导致文件接近 600 行，必须主动创建子模块进行职责解耦。
- **严格禁止领域越界污染**：
  - 数据模型层（如 `mtk-model`）严禁内联编写复杂的 2D/3D 几何投影或裁剪算法；
  - 所有通用几何清理、共面去重与微观遮挡算法必须且只能归属 `mtk-core` 或 `mtk-cull`。

### 规则 9：Crate 级 README 与文档单一事实源 (Crate README & Doc Integrity)
- `crates/` 与 `bindings/` 下的所有子 Crate 必须包含专属 `README.md`，包含架构定位、输入输出契约、核心公共结构体与方法及极简示例。
- 公共 API 变更时必须同步维护该 Crate 自身的 `README.md` 与根目录 `docs/API_REFERENCE.md`。

### 规则 10：内存安全、可见性与 Panic 防御规约 (Safety, Visibility & Panic Defense)
- **禁止在生产代码中滥用 Panic**：胶水层与适配层（`bindings/mtk-py`, `adapters/blender/mtk-blender-dna` 等）严禁在非测试代码中随意调用 `.unwrap()` / `.expect()`；必须通过 `Result` / `PyResult` 进行优雅的错误向上传播。
- **最小可见性原则**：严格控制结构体字段与内部辅助函数的可见性，严禁为了跨模块调用方便而滥用 `pub`。

### 规则 11：全链路性能探针规范 (mtk-profile Probes)
- 所有计算密集型核心算子（如 `subdivide`、`extrude`、`VoxelWorld` mesher、`mtk-sync` 事件派发）必须接入 `mtk-profile` 探针体系，支持微秒级零损耗耗时统计，为前端提供权威的性能指标。

### 规则 12：代码检索与构建目录过滤规范 (Code Search & Target Exclusion)
- **优先使用 ripgrep (`rg`)**：首选且尽量使用 `rg` 命令检索代码。
- **使用 `grep` 时必须忽略 `target` 目录**：必须显式添加 `--exclude-dir=target`（以及 `--exclude-dir=.git`、`--exclude-dir=.venv` 等），严禁扫描底层 Rust 构建产物。

### 规则 13：测试门禁与测试资产规范 (Test Gate & Asset Policy)
- **提交前必须通过的三道门禁（必须与 CI 完全一致，缺一不可）**：
  ```bash
  cargo fmt --all -- --check
  cargo clippy --workspace --all-targets -- -D warnings
  cargo test --workspace
  ```
  禁止以“本地能跑”为由提交带 fmt 差异、Clippy 警告或失败测试的代码；任何 warning 一律在源头修复。
- **严禁硬编码个人绝对路径**：源码、测试、文档中一律禁止出现个人绝对路径。
- **统一使用 `mtk-testkit` 解析测试资产**：所有测试依赖资产必须通过 `mtk-testkit` 提供的工具函数解析，资产缺失时必须优雅跳过（打印跳过原因并 `return`），严禁 panic。
- **Windows 与 6 平台兼容性**：核心代码与测试必须保证在 Windows、macOS (arm64/x64) 及 Linux 平台无警告编译且测试通过。

---

## 3. 核心领域架构与模块划分

```
libmozitoolkit/
├── crates/
│   ├── mtk-core/       -> 基础几何基元、Quad、MeshData、自适应像素切分与挤出算子
│   ├── mtk-cull/       -> 6 向邻域遮挡状态机、2D 矩形差集切分
│   ├── mtk-model/      -> BlockState 状态解析、1.21+ Block Model JSON 烘焙、OBJ 解析
│   ├── mtk-voxel/      -> 16x16x16 Chunk Section 体素存储、流体曲面、平滑 AO 与体素源抽象
│   ├── mtk-sync/       -> RFC v3.1 原生 WebSocket 实时同步客户端、状态机与会话管理
│   ├── mtk-resource/   -> 虚拟文件系统 (VFS)、.mcmeta 动图元数据、原版 atlases/*.json 解析与 CTM
│   ├── mtk-texture/    -> 矩形装箱 Stitcher、多类别 PBR 图集与 Overlay 烘焙、Standalone 转换
│   ├── mtk-material/   -> 66 生物群系调色板引擎、规范 PBR 通道预解码、并行 UV 重映射
│   ├── mtk-package/    -> 单文件预编译资产缓存 (.mtkcache) 与自包含场景交换包 (.mtkscene) 编解码
│   ├── mtk-profile/    -> 全链路轻量级执行性能诊断与探针体系
│   ├── libmtk/         -> 极简应用编排层：端到端预编译管线 (Prebake) 与网格管线 (Pipeline)
│   ├── mtk-bench/      -> 性能压测与基准测试套件
│   ├── mtk-cli/        -> 独立命令行工具 (mtk)
│   └── mtk-testkit/    -> 仅测试用资产解析器与内置 JSON fixtures (publish = false)
├── bindings/
│   ├── mtk-py/          -> 宿主无关 PyO3 + maturin Python 绑定 (libmtk_py)
│   └── mtk-wasm/        -> wasm-bindgen WebAssembly 绑定
└── adapters/
    ├── mtk-host-memory/ -> 宿主硬件内存指针检查与原始切片复制基础底座
    └── blender/         -> 专为 Blender 打造的 C++ 连续内存直接写入适配器 (mtk-blender-dna / libmtk_blender_dna)
```

---

## 4. 目录与内部文档索引

- **系统架构设计**：[`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md)
- **公共 API 与核心抽象参考**：[`docs/API_REFERENCE.md`](docs/API_REFERENCE.md)
- **跨语言绑定与胶水层规范**：[`docs/BINDINGS_DESIGN.md`](docs/BINDINGS_DESIGN.md)

---

## 5. 版本路线与开发计划

- [TODO.md](./TODO.md)

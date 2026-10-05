# libmozitoolkit (`libmtk`)

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)

**`libmozitoolkit`** 是专为高性能 Minecraft 几何数据解析、微观遮挡剔除、体素流网格化与高保真资产烘焙设计的核心跨平台通用 Rust 库套件。

作为 **MoziToolKit 2.0** 的高性能无头核心引擎，本项目遵循 **零宿主依赖（Host-Agnostic）**、**纯数据输入输出（Data-in, Data-out）** 与 **WASM 原生兼容** 原则，既可直接嵌入 Blender 插件，也能无缝运行于浏览器前端（WebGPU / Three.js）、独立 GUI 软件或现代游戏引擎（Bevy / Godot / Unreal）。

---

## 模块结构与技术文档 (Workspace Crates)

本项目采用分层内聚治理架构，每个 Crate/Binding 均配有独立的 `README.md` 作为该模块的权威 API 与技术规范事实源：

| Crate / Binding | 优先级 | 职责与功能 | 专属技术文档与 API |
| :--- | :---: | :--- | :--- |
| **[`mtk-core`](crates/mtk-core)** | 核心 | 基础几何基元、紧凑网格缓冲容器 (`MeshData`)、Minecraft 标准 6 向拓扑、自适应像素网格切分 (`subdivide`) 与智能挤出/UV 修复 (`extrude`) | [📖 `crates/mtk-core/README.md`](crates/mtk-core/README.md) |
| **[`mtk-cull`](crates/mtk-cull)** | 核心 | 6 向邻域遮挡状态机、原版剔除规则、2D 矩形差集切分与外部网格叠面消重 (`MeshSanitizer`) | [📖 `crates/mtk-cull/README.md`](crates/mtk-cull/README.md) |
| **[`mtk-model`](crates/mtk-model)** | 核心 | 无头 BlockState 状态机解析与对齐 1.21+ 规范的 Model JSON 模型烘焙、OBJ 导出/加载 | [📖 `crates/mtk-model/README.md`](crates/mtk-model/README.md) |
| **[`mtk-voxel`](crates/mtk-voxel)** | 核心 | 纯体素核心：16x16x16 Chunk Section 体素存储、流体曲面计算、平滑环境光遮蔽 (AO)、网格化器 (Mesher)、差量网格构建与统一体素源抽象接口 (`VoxelSource`) | [📖 `crates/mtk-voxel/README.md`](crates/mtk-voxel/README.md) |
| **[`mtk-sync`](crates/mtk-sync)** | 核心 | 实时网络协同：原生 WebSocket 客户端、小端序二进制协议编解码、增量同步与 Live Sync 会话生命周期管理 | [📖 `crates/mtk-sync/README.md`](crates/mtk-sync/README.md) |
| **[`mtk-resource`](crates/mtk-resource)** | 核心 | 无头资源包虚拟文件系统 (VFS)、.mcmeta 动图元数据、原版 atlases/*.json 规范解析与 CTM 47/17 连接纹理求解 | [📖 `crates/mtk-resource/README.md`](crates/mtk-resource/README.md) |
| **[`mtk-texture`](crates/mtk-texture)** | 核心 | 矩形空间装箱 Stitcher、多类别 PBR 图集与 Companion Overlay 贴图烘焙、Standalone 资源层级规范对齐 | [📖 `crates/mtk-texture/README.md`](crates/mtk-texture/README.md) |
| **[`mtk-material`](crates/mtk-material)** | 核心 | 66 种原版生物群系调色板与线性色彩数学引擎 (SSOT)、`BiomeResolver` 模型扫描与预编译映射、多线程并行 UV 重映射与外部别名解算 | [📖 `crates/mtk-material/README.md`](crates/mtk-material/README.md) |
| **[`mtk-save`](crates/mtk-save)** | 核心 | 现代 Minecraft 存档加载核心：基于 `simdnbt` 的零拷贝 NBT 解码、Anvil `.mca` 区域文件直接寻址、384 高度世界 3D 生物群系与方块状态解包、按需空间切片流式数据源 (`VoxelSource`) | [📖 `crates/mtk-save/README.md`](crates/mtk-save/README.md) |
| **[`libmtk`](crates/libmtk)** | 核心 | 统一顶层门面 Crate，聚合各子模块并提供端到端高阶资产预编译管线 (`precompile`) 与统一错误处理 (`MtkError`) | [📖 `crates/libmtk/README.md`](crates/libmtk/README.md) |
| **[`mtk-py`](bindings/mtk-py)** | **P0 (当前重点)** | 基于 PyO3 的 Python 扩展模块 (`libmtk_py`)，为 MoziToolKit Blender 插件提供零拷贝内存视图与极速批处理算子 | [📖 `bindings/mtk-py/README.md`](bindings/mtk-py/README.md) |
| **[`mtk-cli`](crates/mtk-cli)** | **P1 (工具链)** | 独立命令行工具，支持资源包预编译 (`precompile`)、性能基准 (`bench`) 与资产缓存校验 (`inspect`) | [📖 `crates/mtk-cli/README.md`](crates/mtk-cli/README.md) |
| **[`mtk-bench`](crates/mtk-bench)** | **P1 (基准压测)** | 4000 区块与网格面剔除基准性能压测套件 | [📖 `crates/mtk-bench/README.md`](crates/mtk-bench/README.md) |
| **[`mtk-ffi`](bindings/mtk-ffi)** | **P2 (远期预备)** | 纯 C-ABI 动态与静态链接库及 C 头文件 (`mtk.h`)，供 C/C++、C# (Unity)、Go、Godot 跨语言调用 | [📖 `bindings/mtk-ffi/README.md`](bindings/mtk-ffi/README.md) |
| **[`mtk-wasm`](bindings/mtk-wasm)** | **P2 (远期预备)** | 基于 wasm-bindgen 的 WebAssembly 绑定，支持浏览器/Node.js/WebGPU 零拷贝 TypedArray 视图 | [📖 `bindings/mtk-wasm/README.md`](bindings/mtk-wasm/README.md) |

---

## 项目文档与设计规范

- 🤖 **[Agent 协作与开发规范 (`GEMINI.md`)](GEMINI.md)**：工作区协同、开发铁律、多工作区交互、Crate 新增/变更规约。
- 📋 **[开发路线与 TODO 清单 (`TODO.md`)](TODO.md)**：阶段演进规划、核心算法与数据管道里程碑。
- 🏛️ **[系统架构设计 (`docs/ARCHITECTURE.md`)](docs/ARCHITECTURE.md)**：分层拓扑、模块职责、数据流向。
- 📖 **[API 抽象与全局契约手册 (`docs/API_REFERENCE.md`)](docs/API_REFERENCE.md)**：全局通用契约、坐标系规范、连续内存协议、错误体系。
- 🔌 **[跨语言绑定与胶水层设计 (`docs/BINDINGS_DESIGN.md`)](docs/BINDINGS_DESIGN.md)**：Python Wheel、C-ABI FFI、WASM 规范与 Blender 对接范式。

---

## 构建与测试

### 1. 本地原生编译与测试
```bash
cargo test --workspace
```

### 2. 独立 CLI 工具构建
```bash
cargo build --release -p mtk-cli
```

### 3. WebAssembly 兼容性校验
```bash
cargo check --workspace --target wasm32-unknown-unknown
```

### 4. Python 扩展构建 (需工作区专属虚拟环境)
```bash
source .venv/bin/activate
maturin build --release -m bindings/mtk-py/Cargo.toml
```

---

## 开源协议

本项目基于 **GNU General Public License v3.0 or later (GPL-3.0-or-later)** 开源。

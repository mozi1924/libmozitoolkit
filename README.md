# libmozitoolkit (`libmtk`)

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)

**`libmozitoolkit`** 是专为高性能 Minecraft 几何数据解析、微观遮挡剔除、体素流网格化与高保真资产烘焙设计的核心跨平台通用 Rust 库套件。

作为 **MoziToolKit 2.0** 的高性能无头核心引擎，本项目遵循 **零宿主依赖（Host-Agnostic）**、**纯数据输入输出（Data-in, Data-out）** 与 **WASM 原生兼容** 原则，既可直接嵌入 Blender 插件，也能无缝运行于浏览器前端（WebGPU / Three.js）、独立 GUI 软件或现代游戏引擎（Bevy / Godot / Unreal）。

---

## 模块结构 (Workspace Crates)

| Crate | 职责与功能 |
| :--- | :--- |
| **[`mtk-core`](crates/mtk-core)** | 基础几何基元、紧凑网格缓冲容器 (`MeshData`)、Minecraft 标准 6 向拓扑、自适应像素网格切分 (`subdivide`) 与智能挤出/UV 修复 (`extrude`) |
| **[`mtk-cull`](crates/mtk-cull)** | 6 向邻域遮挡状态机、原版剔除规则与 2D 矩形差集切分（消除内部重叠面） |
| **[`mtk-model`](crates/mtk-model)** | 无头 BlockState 状态机解析与对齐 1.21+ 规范的 Model JSON 模型烘焙、OBJ 导出/加载 |
| **[`mtk-voxel`](crates/mtk-voxel)** | 纯体素核心：16x16x16 Chunk Section 体素存储、流体曲面计算、平滑环境光遮蔽 (AO)、网格化器 (Mesher)、差量网格构建与统一体素源抽象接口 (`VoxelSource` / `VoxelReader` / `VoxelWriter`) |
| **[`mtk-sync`](crates/mtk-sync)** | 实时网络协同：原生 WebSocket 客户端、小端序二进制协议编解码、增量同步与 Live Sync 会话生命周期管理 |
| **[`mtk-resource`](crates/mtk-resource)** | 无头资源包虚拟文件系统 (VFS)、.mcmeta 动图元数据、原版 atlases/*.json 规范解析与 CTM 47/17 连接纹理求解 |
| **[`mtk-texture`](crates/mtk-texture)** | 矩形空间装箱 Stitcher、多类别 PBR 图集与 Companion Overlay 贴图烘焙、Standalone 资源层级规范对齐 |
| **[`mtk-material`](crates/mtk-material)** | 66 种原版生物群系调色板与线性色彩数学引擎 (SSOT)、`BiomeResolver` 模型扫描与预编译映射、多线程并行 UV 重映射与外部别名解算 |
| **[`libmtk`](crates/libmtk)** | 统一顶层门面 Crate，聚合各子模块并提供端到端高阶资产预编译管线 (`precompile`) 与统一错误处理 (`MtkError`) |
| **[`mtk-bench`](crates/mtk-bench)** | 4000 区块与网格面剔除基准性能压测套件 |
| **[`mtk-cli`](crates/mtk-cli)** | 独立命令行工具，支持资源包预编译 (`precompile`)、性能基准 (`bench`) 与资产缓存校验 (`inspect`) |
| **[`mtk-py`](bindings/mtk-py)** | 基于 PyO3 的 Python 扩展模块 (`libmtk_py`)，为 MoziToolKit Blender 插件提供零拷贝内存视图与极速批处理算子 |
| **[`mtk-ffi`](bindings/mtk-ffi)** | 纯 C-ABI 动态与静态链接库及 C 头文件 (`mtk.h`)，供 C/C++、C# (Unity)、Go、Godot 跨语言调用 |
| **[`mtk-wasm`](bindings/mtk-wasm)** | 基于 wasm-bindgen 的 WebAssembly 绑定，支持浏览器/Node.js/WebGPU 零拷贝 TypedArray 视图 |

---

## 项目文档与设计规范

- 🤖 **[Agent 协作与开发规范 (`GEMINI.md`)](GEMINI.md)**：工作区协同、开发铁律、多工作区交互、Crate 新增/变更规约。
- 🏛️ **[系统架构设计 (`docs/ARCHITECTURE.md`)](docs/ARCHITECTURE.md)**：分层拓扑、模块职责、数据流向。
- 📖 **[API 与核心抽象手册 (`docs/API_REFERENCE.md`)](docs/API_REFERENCE.md)**：Public API 清单、`mtk-core` 数据契约、核心算子。
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

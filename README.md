# libmozitoolkit (`libmtk`)

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)

**`libmozitoolkit`** 是专为高性能 Minecraft 几何数据解析、微观遮挡剔除、体素流网格化与高保真资产烘焙设计的核心跨平台通用 Rust 库套件。

作为 **MoziToolKit 2.0** 的高性能无头核心引擎，本项目遵循 **零宿主依赖（Host-Agnostic）**、**纯数据输入输出（Data-in, Data-out）** 与 **WASM 原生兼容** 原则，既可直接嵌入 Blender 插件，也能无缝运行于浏览器前端（WebGPU / Three.js）、独立 GUI 软件或现代游戏引擎（Bevy / Godot / Unreal）。

---

## 模块结构 (Workspace Crates)

| Crate | 职责与功能 |
| :--- | :--- |
| **[`mtk-core`](crates/mtk-core)** | 基础几何基元、紧凑网格缓冲容器 (`MeshData`)、Minecraft 标准 6 向拓扑与跨平台并发探测 |
| **[`mtk-cull`](crates/mtk-cull)** | 6 向邻域遮挡状态机、原版剔除规则与 2D 矩形差集切分（消除内部重叠面） |
| **[`mtk-model`](crates/mtk-model)** | 无头 BlockState 状态机解析与对齐 1.21+ 规范的 Model JSON 模型烘焙 |
| **[`mtk-voxel`](crates/mtk-voxel)** | 16x16x16 Chunk Section 体素存储、流体曲面计算、平滑环境光遮蔽 (AO) 与世界网格组装 |
| **[`mtk-resource`](crates/mtk-resource)** | 无头资源包虚拟文件系统 (VFS)、.mcmeta 动图元数据与原版 atlases/*.json 解析 |
| **[`mtk-texture`](crates/mtk-texture)** | 矩形空间分割 Stitcher、调色板排列 Permutation 烘焙与 PBR 材质图集生成 |
| **[`libmtk`](crates/libmtk)** | 统一顶层门面 Crate，聚合各子模块并提供一站式便利接口与统一错误处理 (`MtkError`) |

---

## 项目文档与设计规范

- 🤖 **[Agent 协作与开发规范 (`AGENT.md`)](AGENT.md)**：开发铁律、多工作区交互、Crate 新增/变更规约。
- 🏛️ **[系统架构设计 (`docs/ARCHITECTURE.md`)](docs/ARCHITECTURE.md)**：分层拓扑、模块职责、数据流向。
- 📖 **[API 与核心抽象手册 (`docs/API_REFERENCE.md`)](docs/API_REFERENCE.md)**：Public API 清单、`mtk-core` 数据契约。
- 🔌 **[跨语言绑定与胶水层设计 (`docs/BINDINGS_DESIGN.md`)](docs/BINDINGS_DESIGN.md)**：Python Wheel、C-ABI FFI、WASM 规范。


---

## 构建与测试

### 1. 本地原生编译与测试
```bash
cargo test --workspace
```

### 2. WebAssembly 兼容性校验
```bash
cargo check --workspace --target wasm32-unknown-unknown
```

---

## 开源协议

本项目基于 **GNU General Public License v3.0 or later (GPL-3.0-or-later)** 开源。

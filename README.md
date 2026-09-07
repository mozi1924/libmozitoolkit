# libmozitoolkit (`libmtk`)

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)

**`libmozitoolkit`** 是专为高性能 Minecraft 几何数据解析、微观遮挡剔除、体素流网格化与高保真资产烘焙设计的核心跨平台通用 Rust 库套件。

作为 **MoziToolKit 2.0** 的高性能无头核心引擎，本项目遵循 **零宿主依赖（Host-Agnostic）**、**纯数据输入输出（Data-in, Data-out）** 与 **WASM 原生兼容** 原则，既可直接嵌入 Blender 插件，也能无缝运行于浏览器前端（WebGPU / Three.js）、独立 GUI 软件或现代游戏引擎（Bevy / Godot / Unreal）。

---

## 模块结构 (Workspace Crates)

| Crate | 职责与功能 |
| :--- | :--- |
| **[`mtk-core`](crates/mtk-core)** | 基础几何基元、紧凑网格缓冲容器 (`MeshData`)、Minecraft 标准 6 向拓扑与面属性 |
| **[`mtk-cull`](crates/mtk-cull)** | 6 向邻域遮挡状态机、原版剔除规则与 2D 矩形差集切分（消除内部重叠面） |
| **[`mtk-model`](crates/mtk-model)** | 无头 BlockState 状态机解析与对齐 1.21+ 规范的 Model JSON 抽象 |
| **[`libmtk`](crates/libmtk)** | 统一顶层门面 Crate，聚合各子模块并提供一站式便利接口 |

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

# mtk-wasm

[![Crate](https://img.shields.io/badge/crate-mtk--wasm-blue.svg)](Cargo.toml)
[![Rust](https://img.shields.io/badge/Rust-1.80%2B-orange.svg)](https://www.rust-lang.org)
[![WebAssembly](https://img.shields.io/badge/WASM-wasm--bindgen-blue.svg)](https://rustwasm.github.io/docs/wasm-bindgen/)
[![WebGPU](https://img.shields.io/badge/WebGPU-Ready-green.svg)](https://www.w3.org/TR/webgpu/)
[![Status](https://img.shields.io/badge/Roadmap%20Priority-P2%20Future%20Prep-blueviolet.svg)]()
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](../../LICENSE)

**`mtk-wasm`** 是基于 [wasm-bindgen](https://github.com/rustwasm/wasm-bindgen) 构建的高性能 WebAssembly 跨平台绑定库，为现代 Web 浏览器、Node.js 环境以及 WebGPU / Three.js 渲染管线提供体素网格化与遮挡剔除算力。

---

## 目录 (Table of Contents)

- [1. 架构定位与演进优先级](#1-架构定位与演进优先级)
- [2. 核心导出类型与 API 清单](#2-核心导出类型与-api-清单)
  - [2.1 全局控制与诊断](#21-全局控制与诊断)
  - [2.2 遮挡状态机 (`WasmFaceCuller`)](#22-遮挡状态机-wasmfaceculler)
  - [2.3 体素世界与紧凑存储 (`WasmVoxelStorage`)](#23-体素世界与紧凑存储-wasmvoxelstorage)
  - [2.4 高性能网格化器 (`WasmSectionMesher`)](#24-高性能网格化器-wasmsectionmesher)
  - [2.5 几何网格结果与零拷贝内存视图 (`WasmMeshData`)](#25-几何网格结果与零拷贝内存视图-wasmmeshdata)
- [3. 构建、编译与测试](#3-构建编译与测试)
- [4. 真实调用示例](#4-真实调用示例)
- [5. Feature 开关与依赖](#5-feature-开关与依赖)
- [6. 开源协议 (License)](#6-开源协议-license)

---

## 1. 架构定位与演进优先级

- **演进优先级**：`P2`（**远期预备与 Web 端 / WebGPU 交互展示扩展**）。
- **核心定位**：将 `libmtk` 的体素存储、6 向遮挡状态机与网格化器编译为无头 WASM 字节码，无缝嵌入 Web 前端（如基于 Three.js / Babylon.js 的网页版 3D 预览器、WebGPU 实时体素世界编辑器）。
- **性能与零拷贝特性**：
  1. **零拷贝指针直传**：直接暴露 WASM 线性内存指针（`positions_ptr`、`indices_ptr` 等），JavaScript 可直接通过 `new Float32Array(wasm.memory.buffer, ptr, len)` 构造 TypedArray 视图，免除数据序列化与深拷贝开销。
  2. **WebGPU 极速缓冲写入**：支持将 WASM 内存视图直接传递给 `GPUQueue.writeBuffer()`。
  3. **多线程并行能力**：通过 `parallel` 特性集成 `wasm-bindgen-rayon`，支持在 Web Workers 池中并发执行网格切分与体素网格化。

---

## 2. 核心导出类型与 API 清单

### 2.1 全局控制与诊断
- **`init_panic_hook()`**：初始化控制台 Panic 堆栈跟踪，在浏览器 DevTools 中打印可读的 Rust 崩溃信息。
- **`version() -> string`**：返回底层 `libmtk` 版本号。
- **`set_hardware_concurrency(count: number)`** / **`get_hardware_concurrency() -> number`**：设置/获取并发计算线程数（通常传入 `navigator.hardwareConcurrency`）。

---

### 2.2 `WasmMeshData` (网格缓冲容器)
- **几何状态**：
  - `vertex_count` (getter): 顶点数量
  - `triangle_count` (getter): 三角形数量
  - `face_count` (getter): 四边形面数量
  - `is_empty()`: 是否为空
  - `clear()`: 清空网格数据
  - `append_mesh(other: WasmMeshData)`: 合并另一个网格
  - `append_unit_cube_face(direction_id: number, material_slot: number)`: 追加单位立方体面（0:Down, 1:Up, 2:North, 3:South, 4:West, 5:East）
- **零拷贝底层内存指针（推荐用于 WebGPU）**：
  - `positions_ptr() -> number`, `positions_len() -> number`
  - `normals_ptr() -> number`, `normals_len() -> number`
  - `uvs_ptr() -> number`, `uvs_len() -> number`
  - `indices_ptr() -> number`, `indices_len() -> number`
  - `face_materials_ptr() -> number`, `face_materials_len() -> number`
- **复制型 TypedArray（便捷集成 Three.js）**：
  - `get_flat_positions() -> Float32Array`
  - `get_flat_normals() -> Float32Array`
  - `get_flat_uvs() -> Float32Array`
  - `get_indices() -> Uint32Array`
  - `get_face_materials() -> Uint16Array`

---

### 2.3 `WasmFaceCuller` (遮挡剔除器)
- **构造函数**：`new(leaves_cull_mode: number, glass_cull_mode: number)`
  - `leaves_cull_mode`: `0` (SingleFace), `1` (Fancy), `2` (Fast), `3` (None)
  - `glass_cull_mode`: `0` (Group), `1` (SameBlock), `2` (None)
- **缓存管理**：`clear_cache()`, `cache_len() -> number`

---

### 2.4 `WasmVoxelStorage` & `WasmMesherConfig` (体素存储与配置)
- **`WasmVoxelStorage`**：
  - `new()`: 创建稀疏 16x16x16 体素容器
  - `set_bounds(min_x, min_y, min_z, size_x, size_y, size_z) -> boolean`: 设置活动世界包围盒
  - `set_block(x, y, z, block_state: string) -> boolean`: 设置指定方块
  - `get_block(x, y, z) -> string`: 获取指定方块
  - `dirty_section_count() -> number`: 脏区块段数量
  - `clear_dirty_sections()`: 清空脏标记
- **`WasmMesherConfig`**：
  - `new(enable_ao: boolean, mesh_fluids: boolean, z_up_coordinates: boolean, num_threads?: number)`

---

### 2.5 `WasmSectionMesher` (网格化生成器)
- **`mesh_world(storage, config?, culler?) -> WasmMeshData`**：执行全场景体素网格化并返回合并的紧凑几何网格。

---

## 3. 构建、编译与测试

### 3.1 使用 `wasm-pack` 构建 npm 包
```bash
# 安装 wasm-pack
cargo install wasm-pack

# 构建 Web 目标 (产物位于 bindings/mtk-wasm/pkg/)
wasm-pack build bindings/mtk-wasm --target web --release

# 构建 Node.js 目标
wasm-pack build bindings/mtk-wasm --target nodejs --release
```

### 3.2 运行原生与 WebAssembly 兼容性校验
```bash
# 验证原生测试
cargo test -p mtk-wasm

# 验证 wasm32-unknown-unknown 编译兼容性
cargo check -p mtk-wasm --target wasm32-unknown-unknown
```

---

## 4. 真实调用示例

### 4.1 Web 浏览器与 Three.js / WebGPU 示例 (JavaScript / TypeScript)
```javascript
import init, {
    init_panic_hook,
    version,
    WasmVoxelStorage,
    WasmMesherConfig,
    WasmFaceCuller,
    WasmSectionMesher
} from './pkg/mtk_wasm.js';

async function run() {
    // 1. 初始化 WASM 模块与 Panic Hook
    const wasm = await init();
    init_panic_hook();
    console.log(`libmtk WASM initialized, version: ${version()}`);

    // 2. 构造体素世界
    const storage = new WasmVoxelStorage();
    storage.set_bounds(0, 0, 0, 16, 16, 16);
    storage.set_block(0, 0, 0, "minecraft:stone");
    storage.set_block(0, 1, 0, "minecraft:oak_stairs[facing=north,half=bottom,shape=straight]");

    // 3. 配置网格化器与遮挡剔除
    const config = new WasmMesherConfig(true, true, true, 4);
    const culler = new WasmFaceCuller(0, 0);

    // 4. 生成几何网格
    const mesh = WasmSectionMesher.mesh_world(storage, config, culler);
    console.log(`Vertices: ${mesh.vertex_count}, Triangles: ${mesh.triangle_count}`);

    // 5. 零拷贝直传 WebGPU 缓冲区 (Zero-Copy Buffer Upload)
    const positionsView = new Float32Array(
        wasm.memory.buffer,
        mesh.positions_ptr(),
        mesh.positions_len()
    );
    const indicesView = new Uint32Array(
        wasm.memory.buffer,
        mesh.indices_ptr(),
        mesh.indices_len()
    );

    // 直接写入 WebGPU 顶点/索引缓冲，无需任何内存深拷贝
    // device.queue.writeBuffer(vertexBuffer, 0, positionsView);
    // device.queue.writeBuffer(indexBuffer, 0, indicesView);
}

run();
```

---

## 5. Feature 开关与依赖

`mtk-wasm` 在 `Cargo.toml` 中提供特性开关：

| Feature | 默认启用 | 描述 |
| :--- | :---: | :--- |
| `std` | 是 | 启用标准库与各子模块的 WebAssembly 兼容子集。 |
| `parallel` | 否 | 启用 `wasm-bindgen-rayon` 支持 Web Workers 线程池多核并行网格化。 |

---

## 6. 开源协议 (License)

本项目遵循 [GNU General Public License v3.0 or later (GPL-3.0-or-later)](../../LICENSE) 开源协议。

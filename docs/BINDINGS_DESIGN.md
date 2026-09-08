# 跨语言绑定与胶水层架构设计 (Bindings Design)

本文档定义 `libmozitoolkit` 跨平台跨语言绑定（Bindings / FFI）的工程结构、导出协议与内存管理规范。

---

## 1. 胶水层定位与核心原则

1. **计算与胶水严格分离**：所有的核心算法、网格生成、图集拼接均在底层 Rust Crates 完成。Bindings 层仅作为数据封包、格式转换和跨语言内存映射的桥梁。
2. **零拷贝优先 (Zero-Copy First)**：对于海量几何数据（`positions`、`normals`、`uvs`、`indices`）与大尺寸贴图像素（`RgbaBuffer`），优先通过共享内存切片、Buffer Protocol 或 TypedArray 传递，严禁低效的跨语言逐元素复制。
3. **严格的内存生命周期管理**：FFI 与宿主语言交互时，必须提供配对的释放函数（如 `mtk_mesh_free`、`mtk_buffer_free`），杜绝跨语言内存泄漏或野指针。

---

## 2. 目标平台与实现方案

```
crates/
  ├── mtk-py/    -> Python CPython 扩展模块 (PyO3 + maturin)
  ├── mtk-ffi/   -> C-ABI 动态/静态库 (extern "C" + cbindgen)
  └── mtk-wasm/  -> 浏览器与 Node.js 模块 (wasm-bindgen + wasm-pack)
```

### 2.1 Python 绑定 (`crates/mtk-py`)
- **技术栈**：`PyO3` + `maturin` (构建发布为标准 `.whl` 轮子)。
- **核心数据对接**：
  - 将 `MeshData` 的各个字段封装为支持 Python Buffer Protocol 的对象，或者直接转换为 `numpy.ndarray` 视图。
  - Blender Python 侧可直接通过 `bmesh` 或 `mesh.vertices.foreach_set("co", np_positions.ravel())` 纳秒级完成几何灌入。
- **目标场景**：作为 `MoziToolKit` Blender 插件的极速后端，彻底替换原有纯 Python 瓶颈逻辑。

### 2.2 通用 C-ABI 动态链接库 (`crates/mtk-ffi`)
- **技术栈**：`crate-type = ["cdylib", "staticlib"]` + `cbindgen` (自动生成 `include/mtk.h`)。
- **C 接口结构示例**：
  ```c
  typedef struct {
      const float* positions;
      size_t vertex_count;
      const float* normals;
      const float* uvs;
      const uint32_t* indices;
      size_t index_count;
      const uint16_t* face_materials;
      size_t face_count;
  } MtkMeshView;

  MtkMeshHandle* mtk_mesher_build_section(...);
  MtkMeshView mtk_mesh_get_view(MtkMeshHandle* handle);
  void mtk_mesh_free(MtkMeshHandle* handle);
  ```
- **目标场景**：C/C++ 原生引擎、Golang 服务端后端、Godot (GDExtension)、Unity (C# P/Invoke)、Unreal Engine 插件。

### 2.3 WebAssembly 绑定 (`crates/mtk-wasm`)
- **技术栈**：`wasm-bindgen` + `wasm-pack` (发布为 npm package / ES Module)。
- **数据对接**：
  - 利用 `js_sys::Float32Array::view` / `js_sys::Uint32Array::view` 直接将 WASM 线性内存暴露给 JavaScript 前端。
  - Three.js / Babylon.js / WebGPU 渲染管线可直接通过 `BufferAttribute` 引用 WASM 内存并上传 GPU。
- **目标场景**：Web 网页端 3D 预览器、WebGPU 在线地图渲染、轻量前端数据转换工具。

---

## 3. 开发与测试流程规范

1. 任何新添加的公共能力，应先在底层 crate 实现单元测试，并在 `libmtk` 聚合。
2. 随后在各 bindings crate（`mtk-py` / `mtk-ffi` / `mtk-wasm`）中导出对应胶水接口，并编写各语言对应的集成测试（pytest / c_test / wasm-pack test）。

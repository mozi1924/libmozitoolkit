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
bindings/
  ├── mtk-py/    -> Python CPython 扩展模块 (PyO3 + maturin)
  ├── mtk-ffi/   -> C-ABI 动态/静态库 (extern "C" + cbindgen)
  └── mtk-wasm/  -> 浏览器与 Node.js 模块 (wasm-bindgen + wasm-pack)
```

### 2.1 Python 绑定 (`bindings/mtk-py`)
- **技术栈**：`PyO3` + `maturin` (构建发布为标准 `abi3` 轮子)。
- **核心数据对接**：
  - `MeshData`：提供 `positions_memoryview(py)`、`indices_memoryview(py)`、`uvs_memoryview(py)`、`normals_memoryview(py)`、`face_materials_memoryview(py)` 等只读内存视图，直接返回 `memoryview`（或与 `numpy.frombuffer` / Blender `foreach_set` 零拷贝对接）。
  - `VoxelStorage`：3D 稀疏体素容器，支持 `set_full_snapshot` 纳秒级写入大片选区数据与 Palette 映射。
  - `SectionMesher`：多线程并行网格化引擎（`mesh_world`、`mesh_sections_split`、`rebuild_dirty_sections`）。
  - `ResourcePackStack` 与 `AtlasBuilder`：资源包层叠查找与全套 PBR 图集烘焙管线。
- **Blender Python 高效灌入范式示例**：
  ```python
  import numpy as np
  import bpy
  import libmtk_py as mtk

  # 1. 建立体素容器并灌入快照
  storage = mtk.VoxelStorage()
  storage.set_bounds(0, 0, 0, 16, 16, 16)
  storage.set_block(0, 0, 0, "minecraft:stone")

  # 2. 生成 Blender 坐标系网格
  config = mtk.MesherConfig(enable_ao=True, mesh_fluids=True, blender_coordinates=True)
  mesh_data = mtk.SectionMesher.mesh_world(storage, config)

  # 3. 极速灌入 Blender Mesh (零拷贝 Buffer Protocol)
  v_count = mesh_data.vertex_count
  t_count = mesh_data.triangle_count
  if v_count > 0:
      b_mesh = bpy.data.meshes.new(name="MtkWorld")
      b_mesh.vertices.add(v_count)
      b_mesh.loops.add(t_count * 3)
      b_mesh.polygons.add(t_count)

      # 纳秒级 foreach_set 批量灌入
      b_mesh.vertices.foreach_set("co", np.frombuffer(mesh_data.positions_memoryview(), dtype=np.float32))
      b_mesh.loops.foreach_set("vertex_index", np.frombuffer(mesh_data.indices_memoryview(), dtype=np.uint32))
      b_mesh.polygons.foreach_set("loop_start", np.arange(0, t_count * 3, 3, dtype=np.int32))
      b_mesh.polygons.foreach_set("loop_total", np.full(t_count, 3, dtype=np.int32))
      b_mesh.update()
  ```
- **目标场景**：作为 `MoziToolKit` Blender 插件的极速后端，彻底替换原有纯 Python 瓶颈逻辑。

### 2.2 通用 C-ABI 动态链接库 (`bindings/mtk-ffi`)
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

### 2.3 WebAssembly 绑定 (`bindings/mtk-wasm`)
- **技术栈**：`wasm-bindgen` + `wasm-pack` (发布为 npm package / ES Module)。
- **数据对接**：
  - 利用 `js_sys::Float32Array::view` / `js_sys::Uint32Array::view` 直接将 WASM 线性内存暴露给 JavaScript 前端。
  - Three.js / Babylon.js / WebGPU 渲染管线可直接通过 `BufferAttribute` 引用 WASM 内存并上传 GPU。
- **目标场景**：Web 网页端 3D 预览器、WebGPU 在线地图渲染、轻量前端数据转换工具。

---

## 3. 开发与测试流程规范

1. 任何新添加的公共能力，应先在底层 crate 实现单元测试，并在 `libmtk` 聚合。
2. 随后在各 bindings crate（`mtk-py` / `mtk-ffi` / `mtk-wasm`）中导出对应胶水接口，并编写各语言对应的集成测试（pytest / c_test / wasm-pack test）。

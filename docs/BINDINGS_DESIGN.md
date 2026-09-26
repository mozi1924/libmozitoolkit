# 跨语言绑定与胶水层架构设计 (Bindings Design)

本文档定义 `libmozitoolkit` 跨平台跨语言绑定（Bindings / FFI）的工程结构、导出协议、内存管理以及与 DCC 宿主（如 Blender）的高效对接规范。

---

## 1. 胶水层定位与核心原则

1. **计算与胶水严格分离**：所有的核心几何重构、网格细分、挤出 UV 修复、图集装箱、生物群系属性计算均在底层 Rust Crates 完成。Bindings 层仅作为数据封包、格式转换和跨语言内存映射的桥梁。
2. **零拷贝与扁平视图优先 (Zero-Copy First)**：对于海量几何数据（`positions`、`normals`、`uvs`、`indices`）与大尺寸贴图像素（`RgbaBuffer`），优先通过共享内存切片、Python Buffer Protocol (`memoryview`) 或 WebAssembly `TypedArray` 传递，严禁低效的跨语言逐元素循环复制。
3. **Data In, Data Out 批处理范式**：对于涉及复杂场景网格变换的操作（如挤出修复、随机挤出、像素细分），前端将网格数据以原生数组形式传入，Rust 核心完成全量拓扑推算与修复，仅返回修改增量或新拓扑，大幅降低跨语言调用频次。
4. **严格的内存生命周期管理**：FFI 与宿主语言交互时，必须提供配对的释放函数（如 `mtk_mesh_free`、`mtk_buffer_free`），杜绝跨语言内存泄漏或野指针。

---

## 2. 目标平台与实现方案

```
bindings/
  ├── mtk-py/    -> Python CPython 扩展模块 (PyO3 + maturin) -> libmtk_py
  ├── mtk-ffi/   -> C-ABI 动态/静态库 (extern "C" + cbindgen) -> libmtk_ffi
  └── mtk-wasm/  -> 浏览器与 Node.js 模块 (wasm-bindgen + wasm-pack) -> mtk_wasm
```

---

### 2.1 Python 绑定 (`bindings/mtk-py`)

- **构建与分发规范**：
  - 必须在工作区专属虚拟环境（`.venv`）中调用 `maturin build --release` 编译标准 Wheel。
  - 严格遵循 Blender 4.2+ 扩展轮子规范，编译产物仅同步拷贝至 `MoziToolKit/wheels/` 并由 `blender_manifest.toml` 统一声明。
  - **严禁将编译产物安装（`pip install`）到系统全局 Python 或宿主 Blender 的全局 `bpy` 环境中**，杜绝破坏沙箱与环境隔离。

#### 核心导出模块与 API 概览

##### 1. 几何与网格 (`mtk.mesh`)
- `PyMeshData`:
  - `positions_memoryview(py)`: 零拷贝返回 `[N, 3]` f32 顶点坐标缓冲。
  - `normals_memoryview(py)`: 零拷贝返回 `[N, 3]` f32 法线缓冲。
  - `uvs_memoryview(py)`: 零拷贝返回 `[N, 2]` f32 主 UV 缓冲。
  - `indices_memoryview(py)`: 零拷贝返回 `[M]` u32 三角形/多边形索引。
  - `face_materials_memoryview(py)`: 逐面材质插槽 ID。
- **高阶 Data-In Data-Out 算子**：
  - `process_flat_mesh_extrude_repair(...)`:
    使用扁平连续 1D 缓冲区（`positions`, `loop_vertices`, `loop_uvs`, `face_loop_starts`, `face_loop_totals`, `face_materials`）进行极致性能的批量挤出修复，彻底消除 Python 列表嵌套开销，完美适配 Blender `foreach_get` 与 NumPy 数组。
  - `process_mesh_extrude_repair(...)`:
    输入全网格位置、面顶点索引、循环 UV 与像素步长，由 Rust 核心并行分析面面积与法线朝向，输出修改后的稀疏 UV、同步材质与 Crease 锐边。
  - `process_random_extrude_mesh(...)`:
    单批次完成离散选区面随机挤出、3D 噪声位移（Uniform / Perlin / Cellular）、侧面拓扑缝合与自动 UV 修复。
  - `adaptive_pixel_split_mesh(...)`:
    自适应像素网格细分，基于贴图分辨率与 UV 物理跨度切分，保留 Quad 拓扑并双线性插值所有顶点属性。
  - `slice_face_by_pixel_grid(...)` / `calculate_face_target_grid(...)` / `calculate_pixel_grid_cut_factors(...)`。

##### 2. 材质与生物群系 (`mtk.material`)
- `PyBiomeResolver`:
  - `from_file(path)`: 从预编译的 `biome_mapping.json` 极速载入映射。
  - `to_json()` / `from_json(json_str)`: 序列化支持。
- `compute_biome_tint_attributes(...)`: Rayon 多线程批量计算网格面的 Tint 颜色与 Colormap UV 属性。
- `get_biome_meta(name)` / `get_all_biomes()`: 查询 66 原版生物群系规范参数（温度、湿度、草方块/树叶/水体颜色）。
- `srgb_to_linear(color)` / `linear_to_srgb(color)` / `get_colormap_uv(temp, humidity)`: 线性色彩空间数学。
- `MaterialResolver`: 材质别名解算与多通道 UV 重映射 (`remap_mesh_multi_uvs_parallel`)。

##### 3. 体素与实时同步 (`mtk.voxel` & `mtk.sync`)
- `VoxelStorage`: 稀疏体素世界存储，纳秒级快照更新与选区包围盒裁剪。
- `SectionMesher`:
  - `mesh_world(storage, config)`: 多线程并行世界网格化。
  - `MesherConfig(enable_ao=True, mesh_fluids=True, z_up_coordinates=True)`: 标准化 3D 几何坐标系配置。
- `LiveSyncSession`: 原生 WebSocket 后台协同管道。

#### Blender Python 极速灌入范式示例

```python
import numpy as np
import bpy
import libmtk_py as mtk

# 1. 创建体素容器并灌入区块数据
storage = mtk.VoxelStorage()
storage.set_bounds(0, 0, 0, 16, 16, 16)
storage.set_block(0, 0, 0, "minecraft:stone")

# 2. 生成标准右手 Z-Up 网格
config = mtk.MesherConfig(enable_ao=True, mesh_fluids=True, z_up_coordinates=True)
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

---

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
      const uint32_t* face_materials;
      size_t face_count;
  } MtkMeshView;

  MtkMeshHandle* mtk_mesher_build_section(...);
  MtkMeshView mtk_mesh_get_view(MtkMeshHandle* handle);
  void mtk_mesh_free(MtkMeshHandle* handle);
  ```
- **目标场景**：C/C++ 原生引擎、Golang 服务端后端、Godot (GDExtension)、Unity (C# P/Invoke)、Unreal Engine 插件。

---

### 2.3 WebAssembly 绑定 (`bindings/mtk-wasm`)

- **技术栈**：`wasm-bindgen` + `wasm-pack` (发布为 npm package / ES Module)。
- **数据对接**：
  - 利用 `js_sys::Float32Array::view` / `js_sys::Uint32Array::view` 直接将 WASM 线性内存暴露给 JavaScript 前端。
  - Three.js / Babylon.js / WebGPU 渲染管线可直接通过 `BufferAttribute` 引用 WASM 内存并上传 GPU。
- **目标场景**：Web 网页端 3D 预览器、WebGPU 在线地图渲染、轻量前端数据转换工具。

---

## 3. 前后端协作契约与最佳实践

1. **Bridge 层作为唯一门面**：
   在 Blender 插件端 (`MoziToolKit`)，必须通过 `bridge/` 模块（如 `bridge/mesh.py`, `bridge/extrude.py`, `bridge/subdivide.py`, `bridge/assets.py`）统一封装对 `libmtk_py` 的调用，禁止在 UI 或 Operator 中直接调用底层的 CPython C 扩展。
2. **优雅降级与错误反馈**：
   当用户环境未正确加载 `libmtk_py` 动态库时，`bridge` 层必须给出清晰的引导提示（如指向偏好设置中的 Wheel 安装面板），而非直接抛出未捕获的 ImportError 导致崩溃。
3. **测试覆盖铁律**：
   任何新添加的核心能力，必须先在底层 Rust Crate 实现单元测试并保证 `cargo test` 绿灯，随后在 `mtk-py` 导出并编写对应的 Python 集成测试。

# mtk-py (`libmtk_py`)

[![Crate](https://img.shields.io/badge/crate-mtk--py-blue.svg)](Cargo.toml)
[![Rust](https://img.shields.io/badge/Rust-1.80%2B-orange.svg)](https://www.rust-lang.org)
[![Python](https://img.shields.io/badge/Python-3.11%2B%20%28ABI3%29-blue.svg)](https://www.python.org)
[![PyO3](https://img.shields.io/badge/PyO3-0.24%2B-green.svg)](https://pyo3.rs)
[![Status](https://img.shields.io/badge/Roadmap%20Priority-P0%20Core-brightgreen.svg)]()
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](../../LICENSE)

`mtk-py` 是基于 [PyO3](https://pyo3.rs) 与 CPython Stable ABI (`abi3-py311`) 构建的高性能 Python 扩展二进制模块（编译产物为 `libmtk_py`）。

---

## 目录 (Table of Contents)

- [1. 架构定位与演进优先级](#1-架构定位与演进优先级)
- [2. 核心导出模块与 API 清单](#2-核心导出模块与-api-清单)
- [3. 构建、编译与测试指南](#3-构建编译与测试指南)
- [4. 真实代码调用示例](#4-真实代码调用示例)
  - [示例 1：零拷贝网格创建与 Blender 数据灌入](#示例-1零拷贝网格创建与-blender-数据灌入)
  - [示例 2：使用 `process_mesh` 执行多通道 UV 重映射](#示例-2使用-process_mesh-执行多通道-uv-重映射)
  - [示例 3：体素世界存储与多核并行网格化 (`SectionMesher`)](#示例-3体素世界存储与多核并行网格化-sectionmesher)
- [5. Feature 开关与依赖](#5-feature-开关与依赖)
- [6. 开源协议 (License)](#6-开源协议-license)

---

## 1. 架构定位与演进优先级

- **演进优先级**：`P0`（**核心绑定与当前生产事实标准**）。
- **核心定位**：作为 **MoziToolKit 2.0 (Blender 4.2+ 扩展插件)** 的唯一核心胶水层（Bridge Backend），全面取代前端低效的纯 Python 网格切分、UV 修复、遮挡剔除、体素网格化与图集资产预编译算法。
- **设计哲学**：
  1. **Data-In, Data-Out 纯数据批处理契约**：仅负责跨语言数据传输与内存生命周期管理，不内嵌宿主专有状态。
  2. **零拷贝内存视图 (Zero-Copy Buffer Protocol)**：底层几何缓冲（`positions`、`normals`、`uvs`、`indices`、自定义属性）直接通过 Python `memoryview` 与 Buffer 协议输出，支持 Blender `b_mesh.vertices.foreach_set` 与 NumPy 数组毫秒级批量灌入。
  3. **Blender 4.2+ 扩展轮子隔离规范**：独立打包为规范 Wheel 文件并存放在 `MoziToolKit/wheels/`，由 Blender 扩展系统本地隔离加载，严禁全局 `bpy` 污染。

---

## 2. 核心导出模块与 API 清单

Python 模块名：`libmtk_py`

### 2.1 几何网格与自定义属性 (`mesh` & `attributes`)
- **`MeshData` (`PyMeshData`)**：连续内存紧凑几何网格缓冲容器。
  - 属性：`vertex_count`, `triangle_count`, `face_count`, `is_empty`
  - 构造与修改：`MeshData()`, `MeshData.with_capacity(verts, indices, faces)`, `append_mesh(other)`, `append_unit_cube_face(dir, slot, tint)`, `clear()`
  - 内存视图（零拷贝）：`positions_memoryview()`, `normals_memoryview()`, `uvs_memoryview()`, `secondary_uvs_memoryview()`, `indices_memoryview()`, `material_slots_memoryview()`, `tint_indices_memoryview()`, `flags_memoryview()`
  - 扁平列表提取：`get_flat_positions()`, `get_flat_normals()`, `get_flat_uvs()`, `get_flat_secondary_uvs()`, `get_indices()`, `get_face_materials()`, `get_face_tints()`
  - 顶点与面自定义属性：`add_string_attribute(name, domain, values)`, `add_attribute_from_buffer(name, domain, data_type, buffer)`, `get_attribute_data(name)`, `has_attribute(name)`, `list_attributes()`
- **`AttributeDomain`**：属性域枚举，支持 `"point"` (顶点) 与 `"face"` (面)。
- **`process_mesh(mesh, material_names, atlas=None, aliases=None, generate_secondary_uv=True, grid_atlas_spec=None)`**：统一高阶网格处理流水线，多线程并行执行材质状态求解、图集 UV 重映射、次级 [0, 1] PBR UV 生成与自定义属性标记。

### 2.2 遮挡剔除与叠面消重 (`cull`)
- **`FaceCuller` (`PyFaceCuller`)**：原版方块与复杂连接体遮挡状态机。
  - `FaceCuller(leaves_cull_mode=0, glass_cull_mode=0)`
  - `is_face_occluded(block, neighbor, direction, pos_a=None, pos_b=None)`
  - `clear_cache()`, `cache_len`
- **`cull_mesh_faces(mesh, culler, ...)`**：对输入网格执行 2D 矩形差集切分与微观几何遮挡剔除。

### 2.3 自适应像素网格切分 (`subdivide`)
- **`calculate_face_target_grid(uvs, tex_w, tex_h, max_grid)`**：根据 UV 跨越的纹理像素计算切分密度。
- **`calculate_pixel_grid_cut_factors(uvs, tex_w, tex_h, margin, max_subdiv)`**：计算 U/V 方向的归一化网格切分线。
- **`slice_face_by_pixel_grid(positions, uvs, tex_w, tex_h, margin, max_subdiv)`**：对单多边形面进行自适应多边形裁剪与像素对齐切分。
- **`adaptive_pixel_split_mesh(mesh, ...)`**：整网格批量自适应像素切分。

### 2.4 智能挤出与 UV 修复 (`extrude` & `uv`)
- **挤出算子**：
  - `process_mesh_extrude_repair(mesh_data, config_dict)`：智能侧面 UV 修复与四边形拓扑保持挤出。
  - `process_flat_mesh_extrude_repair(positions, uvs, indices, face_mat, ...)`：扁平数组极速挤出修复。
  - `process_random_extrude_mesh(mesh_data, noise_config)`：基于 3D 噪声的随机高度起伏挤出。
- **UV 几何数学算法**：
  - `calculate_uv_area(quad_uvs)`、`get_uv_bounds(quad_uvs)`、`get_uv_center(quad_uvs)`
  - `is_uv_collapsed(quad_uvs)`、`detect_uv_rotation(quad_uvs)`、`straighten_uv(quad_uvs)`
  - `scale_uv(quad_uvs, factor)`、`normalize_uv_for_atlas_tiling(quad_uvs)`、`restore_atlas_tiling_uv(...)`
  - `repair_quad_fluid_uv(...)`、`batch_repair_fluid_uv(...)`、`get_fluid_top_uvs()`、`get_fluid_side_uvs()`

### 2.5 体素存储与网格化 (`voxel` & `mesher`)
- **`VoxelPointCloud` (`PyVoxelPointCloud`)**：全量无剔除 3D 体素点云容器，支持 Blender 点网格 attributes 往返和零拷贝内存视图。
  - `to_mesh_data() -> MeshData`, `from_mesh_data(mesh) -> VoxelPointCloud`
  - `to_storage() -> VoxelStorage`, `from_arrays(...)`
  - `positions_memoryview()`, `block_x_memoryview()`, `block_y_memoryview()`, `block_z_memoryview()`, `light_levels_memoryview()`
  - `get_block_states()`, `get_biomes()`, `get_block_positions()`
- **`VoxelStorage` (`PyVoxelStorage`)**：16x16x16 稀疏 Chunk Section 体素容器，带脏标记追踪。
  - `to_point_cloud(config=None) -> VoxelPointCloud`, `from_point_cloud(cloud) -> VoxelStorage`, `load_from_point_cloud(cloud)`
  - `create_debug_world()` / `create_debug_world_storage()`：一键加载内置嵌入的原版调试世界快照（529 sections, 32,539 blocks）。
  - `set_bounds(min_x, min_y, min_z, size_x, size_y, size_z)`, `set_block(x, y, z, state, tint)`, `get_block(x, y, z)`
  - `dirty_section_count()`, `clear_dirty_sections()`
- **`VoxelWorld` (`PyVoxelWorld`)**：统一 3D 场景引擎，支持增量区块缓存与全量世界网格管理。
  - `to_point_cloud() -> VoxelPointCloud`, `load_from_point_cloud(cloud)`
  - `from_storage(storage, config=None, culler=None, model_db=None, unified_mesh=True, num_threads=None)`
  - `create_debug_world(config=None, culler=None, model_db=None, unified_mesh=True, num_threads=None)`
  - `rebuild_all() -> MeshData`, `rebuild_dirty() -> Dict`, `num_threads` (getter/setter)
- **`MesherConfig` (`PyMesherConfig`)**：网格化配置（AO 开关、流体曲面、坐标系转换、线程数、顶点焊接）。
- **`SectionMesher` (`PySectionMesher`)**：
  - `mesh_world(py, storage, config=None, culler=None, resolver=None) -> MeshData`
  - `mesh_sections_split(py, storage, config=None, ...) -> Dict[Tuple[int,int,int], MeshData]`
  - `rebuild_dirty_sections(py, storage, config=None, ...) -> Dict[Tuple[int,int,int], MeshData]`

### 2.6 实时网络协同 (`sync` & `protocol`)
- **`LiveSyncSession` (`PyLiveSyncSession`)**：原生 WebSocket 实时协同客户端。
  - `connect()`, `disconnect()`, `is_connected()`, `poll_events()`
  - `send_block_update(x, y, z, block_state)`, `request_full_sync(min_pos, max_pos)`
- **二进制协议编解码**：`decode_packet(bytes)`, `encode_full_sync_request(...)`, `encode_repair_requests(...)`, `encode_sync_config(...)`

### 2.7 资源包预烘焙与图集 (`resource` & `texture`)
- **`ResourcePackStack` (`PyResourcePackStack`)**：虚拟文件系统 (VFS) 资源包层叠加载器。
- **`AtlasBuilder` / `BakedAtlas`**：多类别 PBR 空间装箱与烘焙图集容器。
- **`StandaloneBuilder` / `StandaloneResult`**：独立 PBR 材质资产生成器。
- **`precompile_all_assets(jar_path, output_dir, ...)`**：端到端一键无头预烘焙管线。
- **像素分析**：`sample_uv_alpha_f32`, `batch_analyze_transparent_faces_f32`, `batch_analyze_transparent_faces_u8`

### 2.8 生物群系与材质解析 (`material`)
- **`BiomeResolver` (`PyBiomeResolver`)**：原版 66 种生物群系调色板映射与混合器。
- **`MaterialResolver` (`PyMaterialResolver`)**：着色器材质槽位与外部别名解算器。
- **`GridAtlasSpec` (`PyGridAtlasSpec`)**：网格图集元数据规范。
- **色彩数学**：`compute_biome_tint_attributes`, `blend_biome_colors`, `sample_colormap_pixel`, `srgb_to_linear`, `linear_to_srgb`

### 2.9 模型烘焙 (`model`)
- **`ModelBaker` (`PyModelBaker`)**：无头 1.21+ BlockState 模型烘焙引擎。
  - `bake_all(stack, atlas=None)`：并行烘焙所有 BlockState 模型并返回 `BakedModelDatabase`。
  - `bake_blockstate(stack, state_str, clip_hidden=true)`：烘焙单个 BlockState 几何网格与贴图列表。
- **`BakedModelDatabase` (`PyBakedModelDatabase`)**：预编译方块几何模型库。
  - `variant_group_count()`, `has_variant_group(state_str)`, `variant_count(state_str)`：变体分支群组查询。
  - `select_variant_index(state_str, x, y, z)`：按 3D 坐标种子确定性确定变体索引（对齐原版 Java LCG）。
  - `bake_and_register_variant_group(stack, state_str, atlas=None)`：按需解析 BlockState JSON 烘焙全部旋转分支并自动重映射图集。
  - `get_mesh(state_str, clip_hidden=true)`, `to_bincode_bytes()`, `from_bincode_bytes(bytes)`, `remap_to_atlas(atlas)`

---

## 3. 构建、编译与测试指南

> [!IMPORTANT]
> 必须严格在当前工作区的虚拟环境 (`./.venv`) 中执行构建，严禁使用全局 Python 或 `pip install` 污染全局 `bpy`！

### 3.1 激活虚拟环境并编译 Python 模块
```bash
cd <libmozitoolkit 工作区根目录>
source .venv/bin/activate

# 本地开发模式构建 (生成动态库供测试)
maturin develop -m bindings/mtk-py/Cargo.toml

# 生产 Release 轮子构建 (产出至 target/wheels/)
maturin build --release -m bindings/mtk-py/Cargo.toml
```

### 3.2 同步至 Blender 插件前端 (`MoziToolKit`)
```bash
# 将编译生成的 .whl 拷贝到 MoziToolKit 的 wheels 目录
cp target/wheels/libmtk_py-*.whl ../MoziToolKit/wheels/
```

### 3.3 运行 Python 绑定测试
```bash
# 运行 mtk-py 专属单元测试
pytest bindings/mtk-py/tests/

# 运行 MoziToolKit 的 Bridge 测试套件
pytest ../MoziToolKit/tests/
```

---

## 4. 真实代码调用示例

### 示例 1：零拷贝网格创建与 Blender 数据灌入
```python
import libmtk_py
import numpy as np

# 1. 构造连续几何缓冲
positions = [
    0.0, 0.0, 0.0,
    1.0, 0.0, 0.0,
    1.0, 1.0, 0.0,
    0.0, 1.0, 0.0,
]
uvs = [0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0]
indices = [0, 1, 2, 0, 2, 3]

mesh = libmtk_py.MeshData.from_raw_buffers(positions, uvs, indices)
print(f"Vertices: {mesh.vertex_count}, Faces: {mesh.face_count}")

# 2. 零拷贝提取 memoryview 并在 NumPy / Blender foreach_set 中使用
pos_mv = mesh.positions_memoryview()
pos_np = np.frombuffer(pos_mv, dtype=np.float32).reshape(-1, 3)

# 3. 添加自定义属性
mesh.add_string_attribute("mtk_source_texture", "face", ["minecraft:block/stone", "minecraft:block/stone"])
print("Has attribute:", mesh.has_attribute("mtk_source_texture"))
```

### 示例 2：端到端网格图集重映射流水线 (`process_mesh`)
```python
import libmtk_py

# 加载图集地址映射 JSON
atlas_json = '{"chunks":[], "sprites":{"minecraft:block/stone":{"chunk_id":0,"texture_id":1,"uv_bounds":[0.0,0.0,0.25,0.25],"frame_0_uv_bounds":[0.0,0.0,0.25,0.25],"local_uv_bounds":[0.0,0.0,1.0,1.0],"pixel_rect":[0,0,16,16],"frame_size":[16,16],"frame_count":1,"is_animated":false,"has_normal":false,"has_specular":false}}}'
atlas = libmtk_py.BakedAtlas.from_mapping_json(atlas_json)

# 创建输入网格
mesh = libmtk_py.MeshData.from_raw_buffers(
    [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0],
    [0, 1, 2, 0, 2, 3]
)

# 执行并行 UV 重映射与次级 PBR UV 计算
out_mesh, materials, stats = libmtk_py.process_mesh(
    mesh,
    material_names=["Tile_Stone"],
    atlas=atlas,
    aliases={"Tile_Stone": ["minecraft:block/stone"]},
    generate_secondary_uv=True,
)

print("Resolved slots:", stats["resolved_slots"])
print("Mapped canonical material:", materials[0]["canonical_name"])
print("New Atlas UVs:", out_mesh.get_flat_uvs())
print("Secondary PBR UVs:", out_mesh.get_flat_secondary_uvs())
```

### 示例 3：体素世界存储与多核并行网格化 (`SectionMesher`)
```python
import libmtk_py

# 1. 初始化稀疏体素存储
storage = libmtk_py.VoxelStorage()
storage.set_bounds(0, 0, 0, 16, 16, 16)
storage.set_block(0, 0, 0, "minecraft:stone")
storage.set_block(1, 0, 0, "minecraft:oak_stairs[facing=north,half=bottom,shape=straight]")

# 2. 网格化配置与遮挡剔除器
config = libmtk_py.MesherConfig()
culler = libmtk_py.FaceCuller(leaves_cull_mode=0, glass_cull_mode=0)

# 3. 生成紧凑几何网格
mesh = libmtk_py.SectionMesher.mesh_world(storage, config=config, culler=culler)
print(f"Generated Voxel Mesh: {mesh.vertex_count} vertices, {mesh.triangle_count} triangles")
```

---

## 5. Feature 开关与依赖

`mtk-py` 在 `Cargo.toml` 中提供特性开关：

| Feature | 默认启用 | 描述 |
| :--- | :---: | :--- |
| `extension-module` | 否 | 由 `maturin` 在构建 Python C 扩展动态链接库时启用。 |
| `zip` | 是 | 传递给 `mtk-resource`，支持直接读取 `.zip` / `.jar` 资源包。 |
| `parallel` | 是 | 启用 `rayon` 多线程并发加速（模型展开、体素网格化、图像处理）。 |

---

## 6. 开源协议 (License)

本项目遵循 [GNU General Public License v3.0 or later (GPL-3.0-or-later)](../../LICENSE) 开源协议。

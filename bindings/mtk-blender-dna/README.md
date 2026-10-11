# mtk-blender-dna

Blender DNA direct raw memory pointer zero-copy adapter for `libmtk`.

## 架构定位

`mtk-blender-dna` 位于 `bindings/` 目录下，作为特化的宿主互操作（Host Interoperability / FFI）适配器层存在。它负责将 `mtk-core`（`MeshData`）与 `mtk-voxel`（`VoxelPointCloud`）中的几何与体素流直接高效、无拷贝地灌入 Blender 底层连续内存缓冲区（如 DNA `CustomData` 属性图层）。

### 核心隔离原则
1. **Unsafe 物理隔离**：本 Crate 包含裸指针与 `std::ptr::copy_nonoverlapping` 操作；所有 `crates/` 核心算子绝对保持 `#![forbid(unsafe_code)]`，彻底杜绝宿主依赖与未定义行为。
2. **纯宿主边界**：核心层（`mtk-core` 等）纯数据计算，不感知 Blender；本 Crate 专注处理内存首地址与指针边界防护。

## 输入输出契约

- **输入**：`&MeshData` / `&VoxelPointCloud`，目标裸内存地址 `dst_ptr: usize`，可选容量上限 `max_bytes: Option<usize>`。
- **输出**：`DnaResult<usize>`（成功写入的字节数）或 `DnaResult<(Vec<String>, usize)>`（带调色板）。
- **防护守卫**：
  - 非空校验（`dst_ptr != 0`）。
  - 缓冲区溢出保护（`required_bytes <= max_bytes`）。

## 核心方法

- `copy_positions_to_ptr(mesh, dst_ptr, max_bytes)`: 顶点坐标直写 (`float32 * 3`)
- `copy_normals_to_ptr(mesh, dst_ptr, max_bytes)`: 顶点法线直写 (`float32 * 3`)
- `copy_indices_to_ptr(mesh, dst_ptr, max_bytes)`: 三角面索引直写 (`uint32`)
- `copy_quad_indices_to_ptr(mesh, dst_ptr, max_bytes)`: 四边面索引直写 (`uint32`)
- `copy_loop_uvs_to_ptr(mesh, dst_ptr, max_bytes)`: 循环 UV 直写 (`float32 * 2`)
- `copy_face_materials_to_ptr(mesh, dst_ptr, max_bytes)`: 材质槽位直写 (`int32`)
- `copy_colors_to_ptr(mesh, dst_ptr, max_bytes)`: 顶点 RGBA 色直写 (`float32 * 4`)
- `copy_point_cloud_positions_to_ptr(cloud, dst_ptr, max_bytes)`: 体素点云坐标直写
- `copy_point_cloud_to_ptrs(cloud, ...)`: 体素点云全属性批量直写

## 使用示例

```rust
use mtk_blender_dna::copy_positions_to_ptr;
use mtk_core::mesh::MeshData;

let mut mesh = MeshData::default();
mesh.positions = vec![[0.0, 0.0, 0.0], [1.0, 1.0, 1.0]];

let mut buffer = [0.0f32; 6];
let dst_ptr = buffer.as_mut_ptr() as usize;

let bytes = copy_positions_to_ptr(&mesh, dst_ptr, Some(24))?;
assert_eq!(bytes, 24);
# Ok::<(), mtk_blender_dna::DnaError>(())
```

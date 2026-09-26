# libmozitoolkit API 与核心抽象参考手册 (API Reference)

本文档系统性梳理 `libmtk` 及各子 Crate 的 Public API、数据抽象与核心接口规范。

---

## 1. 核心底座：`mtk-core`

`mtk-core` 包含无宿主依赖的底层几何、网格缓冲、方向拓扑、像素网格切分与智能挤出修复算子。

### 1.1 核心数据结构

#### `MeshData` (网格数据缓冲容器)
标准的扁平连续内存几何缓冲：
```rust
pub struct MeshData {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
    pub uvs: Vec<[f32; 2]>,
    pub secondary_uvs: Option<Vec<[f32; 2]>>,
    pub colors: Option<Vec<[f32; 4]>>,
    pub face_materials: Vec<MaterialSlotId>,
    pub face_tint_indices: Vec<TintIndex>,
}
```
**主要方法**：
- `MeshData::new()` / `MeshData::with_capacity(...)`
- `mesh.append_quad(&Quad, &FaceAttributes)`: 追加一个四边形（自动剖分为 2 个 CCW 三角形）
- `mesh.append_mesh(&MeshData)`: 合并另一个网格数据
- `mesh.vertex_count()`, `mesh.triangle_count()`, `mesh.face_count()`, `mesh.is_empty()`, `mesh.clear()`

#### `Quad` (通用四边形基元)
```rust
pub struct Quad {
    pub vertices: [Vec3; 4],
    pub uvs: [Vec2; 4],
    pub normal: Vec3,
}
```
- `Quad::unit_cube_face(direction: Direction) -> Self`: 构建标准单位立方体的某个面。
- `Quad::sub_quad(direction: Direction, min: Vec2, max: Vec2) -> Self`: 构建部分矩形面（用于剔除裁剪后）。

#### `Direction` & `DirMask` (6 向拓扑与位掩码)
- `Direction`: `Down` (0), `Up` (1), `North` (2), `South` (3), `West` (4), `East` (5)。
  - `dir.opposite()`: 取相反方向。
  - `dir.to_vector() -> IVec3`: 转换为整型偏移向量。
  - `dir.to_normal() -> Vec3`: 转换为浮点单位法线向量。
- `DirMask`: `bitflags` 封装的 6 方向位掩码，支持快速位运算检查相邻面。

#### `Aabb2d` & `Aabb3d` (轴对齐包围盒)
- 2D/3D 包围盒相交检测、包含判断、并集与相交裁剪计算。

#### 标准 3D 几何坐标系转换工具
- `mc_local_to_centered_z_up(lx, ly, lz) -> Vec3`: 将 Minecraft 局部方块坐标转换为以方块中心为原点的标准右手 Z-Up 坐标。
- `mc_world_to_z_up(wx, wy, wz) -> Vec3`: 将 Minecraft 世界坐标 (+X East, +Y Up, +Z South) 转换为标准右手 Z-Up 坐标 (+X East, +Y North, +Z Up)。

---

### 1.2 像素网格自适应细分算子 (`subdivide.rs`)

- `calculate_face_target_grid(uvs, tex_w, tex_h, pixels_per_face, max_subdivisions) -> (u32, u32)`:
  基于纹理分辨率和 UV 物理边长向量精准推算目标 (cols, rows) 细分分辨率，自动适应旋转 UV、非正方形图集与长条动图安全截断（防面数爆炸）。
- `calculate_pixel_grid_cut_factors(uvs, tex_w, tex_h, pixels_per_face, max_subdivisions) -> (Vec<f32>, Vec<f32>)`:
  计算四边形在 U/V 轴内非均匀吸附在纹理整数像素边界上的切分参数因子（[0.0, ..., 1.0]）。
- `slice_face_by_pixel_grid(positions, uvs, tex_w, tex_h, pixels_per_face, max_subdivisions)`:
  单面多边形像素网格切分，输出细分后的顶点与 UV 列表。
- `adaptive_pixel_split_mesh(mesh, face_resolutions, default_resolution, pixels_per_face, max_subdivisions, weld_dist) -> MeshData`:
  对全网格执行自适应四边形像素网格细分，支持双线性插值所有顶点属性（顶点色、蒙皮权重、法线）并自动执行微距流形焊接。
- `weld_mesh_vertices(mesh, threshold) -> MeshData`: 空间距离拓扑焊接。

---

### 1.3 智能挤出与 UV 修复算子 (`extrude.rs` & `extrude_mesh.rs`)

- `ExtrudeUvMode`:
  - `Smart`: 智能判定挤出方向与法线点积，向外挤出自动采用内缩采样，内凹挤出自动采用相邻外推采样。
  - `Inward`: 边缘内缩采样（默认 0.1 像素安全距离）。
  - `Outward`: 外推连续条带采样。
- `ExtrudeNoiseType`: `UniformRandom` (均匀伪随机), `Perlin` (3D 连续梯度噪声), `Cellular` (Voronoi 细胞阶梯噪声)。
- `repair_extruded_side_uv(uv_base_a, uv_base_b, top_normal, extrude_vec, mode, step_u, step_v, top_uv_bounds, adjacent_uv_strip) -> [[f32; 2]; 4]`:
  计算侧面重构的 4 个角点 UV，并执行 Safe Padding Clamping 防止采样溢出到图集邻近精灵图。
- `FlatPolygonMesh`:
  扁平连续内存网格拓扑表示结构体（消除数千个嵌套堆分配 `Vec<Vec<T>>`），直接适配 GPU/NumPy 连续 1D 缓冲区：
  ```rust
  pub struct FlatPolygonMesh {
      pub positions: Vec<[f32; 3]>,
      pub loop_vertices: Vec<u32>,
      pub loop_uvs: Vec<[f32; 2]>,
      pub face_loop_starts: Vec<u32>,
      pub face_loop_totals: Vec<u32>,
      pub face_materials: Vec<u32>,
  }
  ```
  提供 `from_nested`, `from_flat_buffers`, `to_nested`, `face_vertices(i) -> &[u32]`, `face_uvs(i) -> &[[f32; 2]]` 等零拷贝切片访问器。
- `process_flat_mesh_extrude_repair(mesh: &FlatPolygonMesh, selected_faces: &[u32], pixel_steps: &[[f32; 2]], config: &MeshExtrudeRepairConfig, target_side_faces: Option<&[u32]>) -> ExtrudeMeshOutput`:
  基于扁平连续内存网格的全网格批量挤出修复算子，输出 `ExtrudeMeshOutput`（包含 `modified_face_uvs`, `modified_face_materials`, `modified_edges`, `modified_edge_creases`, `repaired_count`）。
- `process_mesh_extrude_repair(input: &ExtrudeMeshInput) -> ExtrudeMeshOutput`:
  全网格批量 Data In Data Out 挤出修复兼容门面，内部委托至 `process_flat_mesh_extrude_repair`。
- `process_random_extrude_mesh(input: &RandomExtrudeMeshInput) -> RandomExtrudeMeshOutput`:
  单批次完成离散选区面随机挤出、3D 噪声几何位移、侧面拓扑缝合与自动 UV 修复。

---

## 2. 剔除引擎：`mtk-cull`

负责体素邻域遮挡判断、微观 2D 矩形差集切分算法以及外部网格内部遮挡剔除。

### 2.1 核心类型与函数
- `BlockCullMeta`: 方块剔除元数据（包含各方向的不透明度、剔除分类、遮挡矩形等）。
- `CullCategory`: `Opaque`, `Transparent`, `Leaves`, `Glass`, `Fluid`, `Custom`。
- `FaceCuller`: 邻域面剔除判定器。
- `subtract_rect(subject: Aabb2d, clip: Aabb2d) -> SmallVec<[Aabb2d; 4]>`: 2D 矩形差集切分。
- `subtract_rect_multi(subject: Aabb2d, clips: &[Aabb2d]) -> Vec<Aabb2d>`: 多矩形连续差集切分。
- `should_skip_rendering(curr_cat: CullCategory, neighbor_cat: CullCategory, ...) -> bool`: 原版渲染跳过规则。
- `cull_mesh_faces(mesh: &MeshData, config: &MeshCullConfig) -> MeshCullResult`: 对外部导入的静态网格执行 6 向空间微观遮挡剔除。

---

## 3. 模型与状态机：`mtk-model`

负责 BlockState 字符串解析、1.21+ Block Model JSON 树展开与烘焙。

### 3.1 核心类型与函数
- `BlockState`: 解析 `minecraft:oak_stairs[facing=east,half=bottom,shape=straight]` 为状态名与键值对 Map。
- `BlockModelJson`: 反序列化 Minecraft 原版 Model JSON 结构（`parent`, `textures`, `elements`, `display`）。
- `ModelBaker`: 模型烘焙器，负责递归解析父模型引用、继承纹理变量、根据 UV 旋转和 Element 构建 `BakedModel`。
- `BakedModel`: 包含预计算好的 6 向四边形列表 (`Quad` + `FaceAttributes`) 与未指定 cullface 的自由面。
- `WavefrontObjParser` / `ModObjLoader`: 解析通用 Wavefront OBJ 模型并转化为 `MeshData`。

---

## 4. 纹理与图集：`mtk-texture`

负责 PBR 贴图合并、多类别空间装箱图集拼接 (Atlas Stitching)、Overlay 贴图合成与 Standalone 资产对齐。

### 4.1 核心类型与函数
- `RgbaBuffer`: 扁平 `Vec<u8>` RGBA 图像内存容器（支持裁剪、拷贝、混合、尺寸调整）。
- `Stitcher`: 2D 矩形空间装箱算法（MaxRects / Guillotine）。
- `AtlasBuilder`:
  - `builder.add_sprite(location, rgba_buffer)`
  - `builder.build() -> BakedAtlas`
  - `builder.build_categories(pack_stack: &ResourcePackStack) -> HashMap<AtlasCategory, BakedAtlas>`:
    多类别（Blocks, Items, ArmorTrims, Beds, Chests, ShulkerBoxes 等）批量烘焙，自动合成伴随的 `_overlay.png` 贴图。
- `BakedAtlas`: 包含烘焙后的合成贴图大图（Albedo / Normal / Specular / Roughness / Overlay）与 `AtlasAddressMap`。
- `StandaloneBuilder`: 将资源包贴图对齐导出为 Minecraft 标准资源目录结构的独立 PBR 材质库。

---

## 5. 资源包与 VFS：`mtk-resource`

负责 Minecraft 资源包的虚拟文件系统解压、加载与 CTM 连接纹理计算。

### 5.1 核心类型与函数
- `ResourcePackStack`: 多层资源包叠加栈（按优先级自顶向下查询材质与模型，支持 Companion PBR 贴图探测）。
- `DirectoryPack`, `MemoryPack`, `ZipPack`: VFS 数据源适配器。
- `CtmSolver`: CTM 47 / 17 规则求解器。
- `AnimationMetadata`: 解析 `.mcmeta` 动画帧时长与插值配置。
- `AtlasDefinition`: 解析 Minecraft 1.20+ 原版 `atlases/*.json` 配置文件。

---

## 6. 体素与实时同步引擎：`mtk-voxel`

负责 16x16x16 Section 体素存储、平滑环境光遮蔽 (AO) 计算、流体曲面、网格生成、二进制网络协议编解码与原生 WebSocket 实时协同。

### 6.1 核心类型与函数
- `VoxelStorage`: 3D 稀疏世界体素容器，支持包围盒动态裁剪、局部区块快照 `set_section_snapshot`、CRC32 清单比对 `validate_manifest` 与快照一致性比对。
- `SectionStorage`: 紧凑的高性能 16x16x16 方块状态 ID 存储。
- `PaddedVoxelArray`: 带有 1 格外边框 (18x18x18) 的体素采样窗口。
- `SectionMesher`:
  - 输入：`PaddedVoxelArray`, `ModelBaker`, `MesherConfig`（配置 `z_up_coordinates: bool` 标准化输出）。
  - 输出：`MeshData`。
- `DeltaMesher`: 增量网格化器，针对单点方块破坏/放置与脏区块，快速并行重构局部几何面。
- `calculate_face_ao(neighbors: &[bool; 8]) -> [f32; 4]`: 原版 4 顶点平滑 AO 遮蔽因子计算。
- `FluidType`, `calculate_fluid_corner_heights`: 水/岩浆流体网格与流向计算。

### 6.2 二进制协议编解码 (`mtk_voxel::protocol`)
- `decode_packet(data: &[u8]) -> Result<Packet, ProtocolError>`: 极速解析二进制小端序数据包。
- `encode_full_sync_request()`, `encode_repair_requests(...)`, `encode_sync_config(...)`。
- `Packet`: 强类型数据包枚举（`SelectionInfo`, `FullSnapshot`, `DeltaUpdate`, `SectionManifest` 等）。

### 6.3 原生实时同步会话 (`mtk_voxel::sync`)
- `LiveSyncSession`: 启动原生后台 WebSocket 传输线程与网格构建管道，对外提供 `poll_events() -> Vec<SyncEvent>` 非阻塞事件队列。

---

## 7. 材质与生物群系调色板：`mtk-material`

负责 66 种原版生物群系调色板引擎 (SSOT)、模型扫描与预编译映射、多线程 Rayon 并行 UV 重映射与外部别名解算。

### 7.1 核心类型与函数
- `CANONICAL_BIOMES`: 内置包含 66 种 Minecraft 26.2 官方生物群系调色板常数（`PLAINS`, `DESERT`, `SWAMP`, `CHERRY_GROVE` 等）。
- `BiomePalette`:
  - 属性：`id`, `name`, `temperature`, `humidity`, `grass_hex`, `foliage_hex`, `dry_foliage_hex`, `water_hex` 等。
  - 方法：`grass_linear()`, `foliage_linear()`, `water_linear()`, `colormap_uv()`。
- `BiomeResolver`:
  - 扫描资源包模型 JSON 发现 `tintindex` 与伴随贴图图层。
  - 支持 `to_json()`, `from_json()`, `from_file(path)` 极速加载预烘焙的 `biome_mapping.json`。
- `compute_mesh_biome_attributes` / `compute_mesh_biome_attributes_custom`:
  利用 Rayon 多线程对网格面纹理键进行并行分析，生成打包的 Tint 颜色与 Colormap UV 属性。
- 色彩空间转换函数：
  - `srgb_to_linear(c) -> c` / `linear_to_srgb(c) -> c`
  - `get_colormap_uv(temperature, humidity) -> [f32; 2]`
  - `hex_to_linear_rgba(hex) -> [f32; 4]`
- `MaterialResolver`: 数据驱动的通用材质与贴图匹配器。
- `remap_mesh_multi_uvs_parallel`: 并行输出 Atlas UV 与 Local/Standalone [0, 1] UV 及逐面路由通道。

---

## 8. 端到端预编译引擎：`libmtk::prebake`

负责一键将资源包无头预编译为运行时持久化高速缓存。

- `precompile_all_assets(pack_stack, output_dir, config) -> Result<PrecompileResult, MtkError>`:
  执行全量资源包预烘焙，包含多类别图集烘焙、Companion Overlay 输出、Standalone PBR 结构整理、多线程模型烘焙以及 `biome_mapping.json` 导出。
- `CacheManifest`: 记录缓存版本号 (`ASSET_CACHE_FORMAT_VERSION`)、哈希指纹、时间戳与清单。

---

## 9. 独立命令行工具：`mtk-cli`

`mtk` 命令行工具提供无头环境与 CI 脚本支持：
- `mtk precompile <pack_paths...> -o <cache_dir>`: 预编译指定资源包栈。
- `mtk bench`: 运行区块网格化与面剔除基准测试。
- `mtk inspect <cache_dir>`: 检查预编译缓存的完整性与元数据。

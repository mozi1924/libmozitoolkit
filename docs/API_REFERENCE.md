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
    pub quad_indices: Option<Vec<u32>>,
    pub uvs: Vec<[f32; 2]>,
    pub secondary_uvs: Option<Vec<[f32; 2]>>,
    pub colors: Option<Vec<[f32; 4]>>,
    pub face_materials: Vec<MaterialSlotId>,
    pub face_tint_indices: Vec<TintIndex>,
}
```
**主要方法**：
- `MeshData::new()` / `MeshData::with_capacity(...)`
- `MeshData::merge_all(meshes: &[MeshData]) -> Self`: 批量合并网格集合，单次预计算所有顶点、索引、Quad 与自定义属性容量，彻底消除渐进式内存重分配。
- `mesh.append_quad(&Quad, &FaceAttributes)`: 追加一个四边形（自动剖分为 2 个 CCW 三角形并维护四边形循环顶角）
- `mesh.append_mesh(&MeshData)`: 合并另一个网格数据
- `mesh.weld_spatial_vertices(tolerance: f32)`: 空间距离顶点去重焊接（Rayon 并行重映射三角与 Quad 索引，Per-Corner UV/AO 保留）
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
- `CullCategory`: `SolidOpaque`, `GlassTranslucent`, `WaterFluid`, `LavaFluid`, `Leaves`, `NoCulling`。精准区分实心不透明体（如基岩、石块）与半透明/非立方体。
- `FaceCuller`: 邻域面剔除判定器。
- `subtract_rect(subject: Aabb2d, clip: Aabb2d) -> SmallVec<[Aabb2d; 4]>`: 2D 矩形差集切分。
- `subtract_rect_multi(subject: Aabb2d, clips: &[Aabb2d]) -> Vec<Aabb2d>`: 多矩形连续差集切分。
- `should_skip_rendering(curr_cat: CullCategory, neighbor_cat: CullCategory, ...) -> bool`: 原版渲染跳过规则。
- `check_coplanar_overlap(verts_a: &[Vec3], norm_a: Vec3, verts_b: &[Vec3], norm_b: Vec3, tol: f32) -> Option<CoplanarRelation>`: 统一的共面正交切线投影与 2D 包围盒相交检测算子，返回面法线朝向 (`SameDirection`, `OppositeDirection`) 与覆盖类型 (`Exact`, `ContainedInA`, `ContainedInB`, `Partial`)。
- `MeshSanitizer`: 统一网格清理与几何消重工具门面：
  - `deduplicate_quads<T>(quads: &[(T, [Vec3; 4], Vec3)], tolerance: f32) -> HashSet<T>`: 批量检测并消除同向重复面、背靠背反向贴合接触面及完全包含面。
  - `clip_quad_excluding_hidden_volume(...)`: 轴对齐四边形隐藏体包围盒差集裁剪。
  - `sanitize_mesh(mesh: &MeshData, config: &MeshCullConfig) -> MeshCullResult`: 对任意 `MeshData` 连续缓冲执行去重与贴合剔除。
- `MeshCullConfig`: 网格剔除与叠面消除配置项：
  - `tolerance: f32`: 空间几何重合与共面判定容差（默认 1e-4）。
  - `cull_duplicates: bool`: 是否消除几何位置完全重叠且法线同向的重复面（Duplicate Faces，默认 true）。
  - `cull_coplanar_opposite: bool`: 是否消除背靠背共面贴合的内部接触面（Contact Faces，默认 false）。
  - `preserve_quads: bool`: 是否在剔除后重构并保留 Quad 拓扑与 `quad_indices`（默认 true）。
- `cull_mesh_faces(mesh: &MeshData, config: &MeshCullConfig) -> MeshCullResult`: 对网格执行精确微观遮挡与叠面剔除，无缝保留所有 `Face` / `Point` / `Corner` / `Mesh` 自定义属性与 Quad 拓扑。

---

## 3. 模型与状态机：`mtk-model`

负责 BlockState 字符串解析、1.21+ Block Model JSON 树展开与烘焙。几何面的差集裁剪与共面消重统一委托至 `mtk-cull::MeshSanitizer`。

### 3.1 核心类型与函数
- `BlockState`: 解析 `minecraft:oak_stairs[facing=east,half=bottom,shape=straight]` 为状态名与键值对 Map。
- `BlockModelJson`: 反序列化 Minecraft 原版 Model JSON 结构（`parent`, `textures`, `elements`, `display`）。
- `ModelBaker`: 模型烘焙器，负责递归解析父模型引用、继承纹理变量、根据 UV 旋转和 Element 构建 `BakedModel`。精准维护非立方体及不透明方块词缀白名单与精确守卫（如对 `bed` 进行精确/后缀匹配，避免误伤 `bedrock`）。
- `BakedFace`: 模型四边形面描述，包含局部坐标顶点 (`vertices`)、原版 UV (`uvs`)、法线 (`normal`)、纹理标识符 (`texture`)、tint 索引 (`tint_index`) 与剔除方向 (`cullface`)。Phase 2 新增预计算图集字段 `atlas_uvs: Option<[Vec2; 4]>`、`atlas_chunk_id: Option<u16>`、`atlas_texture_id: Option<u32>` 及 `remap_to_atlas_bounds(...)` 离线坐标烘焙能力。
- `ModelMeshOptions`: 网格化提取配置结构体：
  - `clip_hidden_volume: bool`: 裁剪内部隐藏体素包围盒。
  - `cull_duplicates: bool`: 自动消除几何同向重合面（杜绝 DCC 视口/渲染器 Z-fighting 闪烁）。
  - `cull_coplanar_opposite: bool`: 剔除模型内部背靠背贴合的无用接触面。
  - `tolerance: f32`: 几何判定容差。
- `BakedModel`: 包含预计算好的 6 向四边形列表 (`Quad` + `FaceAttributes`) 与未指定 cullface 的自由面、原始方块要素元素 (`BakedElement`) 以及预烘焙的面遮挡剔除元数据 (`cull_meta: Option<BlockCullMeta>`)。
  - `deduplicate_faces(&mut self) -> usize`: 委托 `mtk_cull::MeshSanitizer` 在 Element 面粒度直接消除同向重合面与反向贴合面，并自动重构 6 向分桶。
  - `to_mesh_with_options(&self, options: &ModelMeshOptions) -> MeshData`: 带叠面消除与隐藏体裁剪的网格导出。
  - `to_mesh_with_textures(...)`: 批量导出带完整面属性与叠面消除的网格。
  - 提供 6 向分桶字段 `culled_faces: [Vec<BakedFace>; 6]` 与 `unculled_faces: Vec<BakedFace>`，支持 `rebuild_face_buckets()`、`get_face_buckets()` 以及 `remap_to_atlas_with(...)` 离线批量预映射。
- `BakedModelDatabase`: 烘焙模型数据库容器，支持快速序列化与反序列化（bincode 二进制高速流，内置 V0/V1/V2/V3 多版本向后兼容回退与自动面分桶重建）。
  - `db.deduplicate_all() -> usize`: 批量对数据库中所有模型执行面去重与内部接触面剔除。
  - `db.remap_to_atlas_with(lookup_fn)`: 离线对数据库中所有模型面（含分桶面）批量注入图集 UV 与 Chunk/Texture ID。
  - `db.get(state: &str) -> Option<&BakedModel>`:
    多级智能 BlockState 寻址匹配引擎：
    - **Tier 1 (Exact Match)**: 极速精确哈希查询（针对纯净模型定义键）。
    - **Tier 2 (Canonical Filter)**: 剥离世界运行时非几何状态属性（如 `waterlogged`, `occupied`, `distance`, `persistent`, `stage`, `unstable`, `conditional`, `disarmed`）进行规范键查询。
    - **Tier 3 (Compatibility Subset Match)**: 提取方块标识与关键几何变体属性（如 `facing`, `half`, `part`, `type`, `shape`, `open`, `lit`, `axis`）进行属性子集模糊降级匹配。
    - **Tier 4 (Base ID Fallback)**: 回退到基础 Block ID 默认形态。
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
    多类别（Blocks, Items, ArmorTrims, Beds, Chests, ShulkerBoxes 等）批量烘焙，自动合成伴随的 `_overlay.png` 贴图。对伴随的 Overlay 纹理自动在 `AtlasAddressMap` 中注册同位置别名（如 `minecraft:block/grass_block_side_overlay`），避免寻址模糊并保证 UV 与基底方块 1:1 对齐。
- `BakedAtlas`: 包含烘焙后的合成贴图大图（Albedo / Normal / Specular / Roughness / Overlay）与 `AtlasAddressMap`。
- `StandaloneBuilder`: 将资源包贴图对齐导出为 Minecraft 标准资源目录结构的独立 PBR 材质库。

---

## 5. 资源包与 VFS：`mtk-resource`

负责 Minecraft 资源包的虚拟文件系统解压、加载与 CTM 连接纹理计算。

### 5.1 核心类型与函数
- `is_companion_asset_path(file: &str) -> bool`: 检查贴图路径是否为伴随贴图（`_n.png`, `_s.png`, `_overlay.png`, `grass_block_side_overlay.png` 等），避免其被误收录为图集独立 Albedo 图块。
- `ResourcePackStack`: 多层资源包叠加栈（按优先级自顶向下查询材质与模型，支持 Companion PBR 贴图探测）。
- `DirectoryPack`, `MemoryPack`, `ZipPack`: VFS 数据源适配器。
- `CtmSolver`: CTM 47 / 17 规则求解器。
- `AnimationMetadata`: 解析 `.mcmeta` 动画帧时长与插值配置。
- `AtlasDefinition`: 解析 Minecraft 1.20+ 原版 `atlases/*.json` 配置文件。

---

## 6. 体素核心与抽象引擎：`mtk-voxel`

负责 16x16x16 Section 体素存储、平滑环境光遮蔽 (AO) 计算、流体曲面、网格生成，以及支持多输入源适配的统一抽象 Trait。

### 6.1 体素源与访问抽象 (`mtk_voxel::source`)
- `VoxelReader`: 统一只读体素访问接口。
  - `get_block(x, y, z) -> &str`: 获取指定方块坐标处的完整 BlockState 字符串。
  - `get_section(sx, sy, sz) -> Option<&SectionStorage>`: 获取切片引用。
  - `contains_section(sx, sy, sz) -> bool`: 检查切片是否存在。
  - `block_bounds() -> Option<(IVec3, IVec3)>`: 获取世界包围盒。
  - `get_biome(x, y, z) -> &str`: 获取方块坐标处的生物群系。
- `VoxelWriter`: 统一体素修改与区块写入接口。
  - `set_block(x, y, z, state, biome)`: 单点写入方块与生物群系。
  - `set_section(sx, sy, sz, section)`: 整段写入/替换 16x16x16 切片。
  - `mark_section_dirty(sx, sy, sz)`: 标记切片为脏状态（触发重构网格）。
  - `set_bounds(min_x, min_y, min_z, size_x, size_y, size_z)`: 设定活动包围盒。
  - `clear()`: 清空所有切片与状态。
- `VoxelSource`: 跨数据源统一生产者 Trait（供离线存档 MCA/LevelDB、实时网络流、点云/网格逆向推算、程序化生成统一实现）。
  - `source_name() -> &str`: 数据源标识。
  - `has_section(sx, sy, sz) -> bool`: 检查切片是否就绪。
  - `load_section(sx, sy, sz) -> Result<Option<SectionStorage>, VoxelError>`: 按需加载切片。
  - `section_bounds() -> Option<(IVec3, IVec3)>`: 提供切片坐标包围盒。
- `ingest_from_source(source, target, sections)`: 从任意 `VoxelSource` 批量流式灌入 `VoxelWriter`。

### 6.2 存储与网格化器
- `VoxelStorage`: 3D 稀疏世界体素容器（实现 `VoxelReader` 与 `VoxelWriter`），支持包围盒动态裁剪、局部区块快照 `set_section_snapshot`、CRC32 清单比对 `validate_manifest` 与 `ingest_source` 快速灌流。内置 `biome_column_map` 二维列群系高速缓存与选区边界边缘约束（Edge Clamping），彻底杜绝选区边缘外推采样产生平原默认色渗色。
- `SectionStorage`: 紧凑的高性能 16x16x16 方块状态 ID 存储。
- `PaddedVoxelArray`: 带有 1 格外边框 (18x18x18) 的体素采样窗口。新增 `biome_data: Option<Vec<SmoothedBiomeColumn>>` 字段携带 256 列 (16x16) 平滑生物群系数据。
- `SmoothedBiomeColumn` / `get_smoothed_column_biome`:
  基于原版 5x5 (R=2) 反距离权重核的平滑生物群系柱数据结构与解算函数，平滑计算草方块色彩 (`grass_color`)、树叶色彩 (`foliage_color`)、干枯树叶色彩 (`dry_foliage_color`)、水体色彩 (`water_color`) 与 Colormap 三角形采样 UV (`colormap_uv`)。针对选区边界执行边缘向内钳位与中心回退保护，消除边界渗色。
- `SectionMesher`:
  - 核心定位：专注于 18x18x18 体素邻域遍历、邻域遮挡拓扑与 AO 计算。与材质着色元数据解析解耦：
    - `mtk_voxel::mesher::shading`: 专职负责 CTM 解算器调度 (`resolve_model_face_shading`, `resolve_unit_cube_face_shading`)、Atlas 寻址与 Biome 采样着色 (`sample_biome_tint`, `build_palette_meshing_data`)。
    - `mtk_voxel::mesher::emitter`: 专职负责顶点空间坐标变换、AO 亮度插值、各向异性对角线翻转与面属性发射 (`emit_baked_face`, `emit_unit_cube_face`)。
  - 输入：`PaddedVoxelArray`, `ModelBaker`, `MesherConfig`（配置 `z_up_coordinates: bool` 标准化输出、`origin_centered: bool` 底部中心原点对齐、`weld_vertices: bool` 空间顶点焊接、`selection_bounds: Option<([i32; 3], [i32; 3])>` 包围盒对齐、`atlas: Option<Arc<AtlasAddressMap>>` 图集寻址、`biome_resolver: Option<Arc<BiomeResolver>>` 生物群系调色板着色与 `custom_aliases` 别名映射）。
  - 输出：`MeshData`（包含 `mtk_source_texture_key`、`mtk_material_slot`、`mtk_atlas_chunk_id`、`mtk_uv_tiling_transform`、`mtk_biome_tint_data` 等 15 项标准面属性）。自动剔除多 Element 模型中的冗余 Overlay Decal 面（如草方块侧面叠加层），由前端着色器单面多重采样无缝合成，杜绝共面发黑与 Z-fighting。
  - **生物群系平滑过渡 (Biome Transition Smoothing)**：在发射网格面时，针对染色面动态索引柱级 `SmoothedBiomeColumn`，将平滑后的调色板颜色与 Colormap UV 注入面属性，使视口中生物群系交界处呈现原版无缝柔和渐变。
  - **Palette 级预解析加速 (Phase 2)**：在遍历 4,096 体素前构建 `palette_meshing_data`，将模型面/单面方块的材质槽、纹理别名、图集 UV 映射及生物群系着色预先解算至 Palette 级别（单区块仅执行 10~50 次），消除了内部热循环中重复的字符串分配、哈希查找与 UV 矩形变换。
  - **6 向分桶与快速通道 (Phase 3)**：将模型面划分为 6 个主方向分桶 (`culled_faces: [Vec; 6]`) 与免剔除分桶 (`unculled_faces`)。热循环中若邻居方块不透明，1 次判定即可直接跳过该方向的所有面；对于免剔除的植被/交叉模型（草、花、火把等），完全跳过邻居采样与遮挡判定直接发射几何。
- `DeltaMesher`: 增量网格化器，针对单点方块破坏/放置与脏区块，快速并行重构局部几何面。
- `calculate_face_ao(neighbors: &[bool; 8]) -> [f32; 4]`: 原版 4 顶点平滑 AO 遮蔽因子计算。
- `FluidType`, `calculate_fluid_corner_heights`: 水/岩浆流体网格与流向计算（流体面同步注入平滑水体色彩与 Colormap UV）。

---

## 7. 实时网络协同引擎：`mtk-sync`

负责与 Minecraft 伴随插件/模组的原生 WebSocket 双向流式通信、二进制协议编解码与 Live Sync 会话生命周期管理。

### 7.1 二进制协议编解码 (`mtk_sync::protocol`)
- `PROTOCOL_MAGIC`: 固定魔数头部 `[0x4D, 0x43]` (`"MC"`)。
- `PROTOCOL_VERSION`: 规范协议主版本号 `0x02`（对齐 Yefira 2.0 规范）。
- `MIN_SUPPORTED_PROTOCOL_VERSION` (`0x01`) / `MAX_SUPPORTED_PROTOCOL_VERSION` (`0x02`): 支持的双向协议版本边界。
- `is_supported_protocol_version(version: u8) -> bool`: 校验入站数据包版本兼容性。
- `decode_packet(data: &[u8]) -> Result<Packet, ProtocolError>`: 极速解析二进制小端序数据包。
- `encode_full_sync_request()`, `encode_repair_requests(...)`, `encode_sync_config(...)`。
- `Packet`: 强类型数据包枚举（`SelectionInfo`, `FullSnapshot`, `DeltaUpdate`, `SectionManifest`, `StreamBegin`, `StreamEnd` 等）。

### 7.2 原生实时同步会话 (`mtk_sync`)
- `LiveSyncSession`: 启动原生后台多线程 WebSocket 传输线程 (`SyncClient`) 与 `VoxelStorage` 驱动通道，对外提供 `poll_events() -> Vec<SyncEvent>` 非阻塞事件队列。
  - 支持 `model_db: Option<Arc<BakedModelDatabase>>` 注入，全面支持纯 JSON 模型与复杂模组几何剖分。
  - 支持 `unified_mesh: bool` 单一世界无缝网格模式，派发 `WorldMeshReady` 事件，彻底根除相邻小区块拼接发黑与次表面散射 (SSS) 撕裂缺陷。
  - 支持 `get_world_mesh() -> MeshData` 主动提取当前世界全量合并网格。
- `SyncClient`: 基于 `tungstenite` 与 `crossbeam-channel` 的低延迟网络套接字守护线程，支持心跳检测与断线自动重连。

---

## 7. 材质与生物群系调色板：`mtk-material`

负责 66 种原版生物群系调色板引擎 (SSOT)、模型扫描与预编译映射、多线程 Rayon 并行 UV 重映射与外部别名解算。

### 7.1 核心类型与函数
- `CANONICAL_BIOMES`: 内置包含 66 种 Minecraft 26.2 官方生物群系调色板常数（`PLAINS`, `DESERT`, `SWAMP`, `CHERRY_GROVE` 等）。
- `BLOCK_TINT_REGISTRY`: 原版权威方块染色分层注册表（覆盖 35+ 种方块，精确支持多层植被如粉红花簇/野花的花瓣与茎叶分层染色）。
- `HARDCODED_BLOCK_TINTS`: 独立于生物群系的固定色方块表（云杉树叶、白桦树叶、睡莲、西瓜茎、南瓜茎、红石线等）。
- `classify_tint_category(clean_stem, block_name, tint_index) -> &'static str`:
  权威染色语义分类器（返回 `"grass"`, `"foliage"`, `"dry_foliage"`, `"water"`, `"hardcoded"`, `"none"`）。严格遵循原版规范：`tint_index < 0` 永不染色；优先依据 `BLOCK_TINT_REGISTRY` 与 `tint_index` 进行精确图层语义解析；未知模型 `ti >= 0` 回退至 `"none"`。
- `get_unit_cube_tint_index(clean_block, dir) -> i16`: 单元立方体网格化面染色索引确定（草方块顶面为 0，底面泥土与侧面为 -1；树叶/草本全向为 0；非染色方块全向为 -1）。
- `BiomePalette`:
  - 属性：`id`, `name`, `temperature`, `humidity`, `grass_hex`, `foliage_hex`, `dry_foliage_hex`, `water_hex` 等。
  - 方法：`grass_linear()`, `foliage_linear()`, `water_linear()`, `colormap_uv()`。
- `BiomeResolver`:
  - 扫描资源包模型 JSON 发现 `tintindex` 与伴随贴图图层。
  - 支持 `set_models(models)` 动态注入方块模型并递归解析 `parent` 继承树合并材质贴图与 elements。
  - 支持 `to_json()`, `from_json()`, `from_file(path)` 极速加载预烘焙的 `biome_mapping.json`。
- `compute_mesh_biome_attributes` / `compute_mesh_biome_attributes_custom`:
  利用 Rayon 多线程对网格面纹理键进行并行分析，生成打包的 Tint 颜色与 Colormap UV 属性。
- `sample_colormap_pixel(pixels, width, height, temp, hum, channels) -> [f32; 3]`:
  标准原版 256x256 三角形 Colormap 像素采样算法。
- 色彩空间转换函数：
  - `srgb_to_linear(c) -> c` / `linear_to_srgb(c) -> c`
  - `get_colormap_uv(temperature, humidity) -> [f32; 2]`
  - `hex_to_linear_rgba(hex) -> [f32; 4]`
- `MaterialResolver`: 数据驱动的通用材质与贴图匹配器。
- `remap_mesh_multi_uvs_parallel`: 并行输出 Atlas UV 与 Local/Standalone [0, 1] UV 及逐面路由通道。


---

## 8. 端到端预编译引擎：`libmtk::prebake`

负责一键将资源包无头预编译为运行时持久化高速缓存。

- `PrecompileConfig`:
  - `export_standalone: bool`: 是否导出单体 PBR 贴图。
  - `num_threads: Option<usize>`: Rayon 线程池大小配置（`None` 为系统全部可用核心）。
- `precompile_all_assets(pack_stack, output_dir, config) -> Result<PrecompileResult, MtkError>`:
  执行全量资源包预烘焙，包含多类别图集烘焙、Rayon 并行 Atlas Chunk PNG 编码与落盘、Companion Overlay 输出、Standalone PBR 结构整理、多线程模型烘焙（自动串联图集寻址表注入 Atlas UV 预解算）以及 `biome_mapping.json` 导出。
- `prebake_all_models(stack, atlas_map: Option<&AtlasAddressMap>) -> Result<BakedModelDatabase, ...>`:
  全量烘焙资源包中的模型，若传入图集映射表，则在烘焙期自动完成图集 UV 与 Chunk/Texture ID 预解算并注入 `BakedModel`。
- `CacheManifest`: 记录缓存版本号 (`ASSET_CACHE_FORMAT_VERSION`)、哈希指纹、时间戳与清单。

---

## 9. 独立命令行工具：`mtk-cli`

`mtk` 命令行工具提供无头环境与 CI 脚本支持：
- `mtk precompile <pack_paths...> -o <cache_dir>`: 预编译指定资源包栈。
- `mtk bench`: 运行区块网格化与面剔除基准测试。
- `mtk inspect <cache_dir>`: 检查预编译缓存的完整性与元数据。

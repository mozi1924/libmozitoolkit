# libmozitoolkit API 与核心抽象参考手册 (API Reference)

本文档系统性梳理 `libmtk` 及各子 Crate 的 Public API、数据抽象与核心接口规范。

---

## 1. 核心底座：`mtk-core`

`mtk-core` 包含无宿主依赖的底层几何、网格缓冲和方向拓扑抽象。

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

#### 坐标系转换工具
- `mc_local_to_blender(v: Vec3) -> Vec3`
- `mc_world_to_blender(v: Vec3) -> Vec3`

---

## 2. 剔除引擎：`mtk-cull`

负责体素邻域遮挡判断与微观 2D 矩形差集切分算法（消除共面重叠）。

### 2.1 核心类型与函数
- `BlockCullMeta`: 方块剔除元数据（包含各方向的不透明度、剔除分类、遮挡矩形等）。
- `CullCategory`: `Opaque`, `Transparent`, `Leaves`, `Glass`, `Fluid`, `Custom`。
- `FaceCuller`: 邻域面剔除判定器。
- `subtract_rect(subject: Aabb2d, clip: Aabb2d) -> SmallVec<[Aabb2d; 4]>`: 2D 矩形差集切分（从 subject 中减去 clip 区域，返回剩余的 0~4 个不相交小矩形）。
- `subtract_rect_multi(subject: Aabb2d, clips: &[Aabb2d]) -> Vec<Aabb2d>`: 多矩形连续差集切分。
- `should_skip_rendering(curr_cat: CullCategory, neighbor_cat: CullCategory, ...) -> bool`: 原版渲染跳过规则。

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

负责 PBR 贴图合并、空间分割图集拼接 (Atlas Stitching) 与 UV 映射。

### 4.1 核心类型与函数
- `RgbaBuffer`: 扁平 `Vec<u8>` RGBA 图像内存容器（支持裁剪、拷贝、混合、尺寸调整）。
- `Stitcher`: 2D 矩形空间装箱算法（MaxRects / Guillotine），用于高效拼接图集。
- `AtlasBuilder`:
  - `builder.add_sprite(location, rgba_buffer)`
  - `builder.build() -> BakedAtlas`
- `BakedAtlas`: 包含烘焙后的合成贴图大图（Albedo / Normal / Specular / Roughness）与 `AtlasAddressMap`（Sprite 原始 UV 到图集 UV 的变换矩阵/映射矩形）。

---

## 5. 资源包与 VFS：`mtk-resource`

负责 Minecraft 资源包的虚拟文件系统解压、加载与 CTM 连接纹理计算。

### 5.1 核心类型与函数
- `ResourcePackStack`: 多层资源包叠加栈（按优先级自顶向下查询材质与模型）。
- `DirectoryPack`, `MemoryPack`, `ZipPack`: VFS 数据源适配器。
- `CtmSolver`: CTM (Connected Textures Mod) 47 / 17 规则求解器，根据相邻方块状态计算当前面应该使用的子贴图索引。
- `AnimationMetadata`: 解析 `.mcmeta` 动画帧时长与插值配置。

---

## 6. 体素与实时同步引擎：`mtk-voxel`

负责 16x16x16 Section 体素存储、平滑环境光遮蔽 (AO) 计算、生物群系过渡、网格生成、二进制网络协议编解码与原生 WebSocket 实时协同。

### 6.1 核心类型与函数
- `VoxelStorage`: 3D 稀疏世界体素容器，支持包围盒动态裁剪、局部区块快照 `set_section_snapshot`、CRC32 清单比对 `validate_manifest`、快照一致性比对 `is_snapshot_identical` 与场景元数据导入/导出 `export_manifest_metadata` / `import_manifest_metadata`。
- `SectionStorage`: 紧凑的高性能 16x16x16 方块状态 ID 存储（支持调色板与位压缩）。
- `PaddedVoxelArray`: 带有 1 格外边框 (18x18x18) 的体素采样窗口，供网格化时无锁读取邻域。
- `SectionMesher`:
  - 输入：`PaddedVoxelArray`, `ModelBaker`, `MesherConfig`。
  - 输出：`MeshData`（包含顶点、法线、UV、面材质索引、面属性与包围盒）。
- `DeltaMesher`: 增量网格化器，针对单点方块破坏/放置与脏区块，快速并行重构受影响的局部几何面。
- `calculate_face_ao(neighbors: &[bool; 8]) -> [f32; 4]`: 原版 4 顶点平滑 AO 遮蔽因子计算。
- `get_smoothed_biome_data(...)`: 生物群系颜色平滑混合采样。
- `FluidType`, `calculate_fluid_corner_heights`: 水/岩浆流体网格与流向计算。

### 6.2 二进制协议编解码 (`mtk_voxel::protocol`)
- `decode_packet(data: &[u8]) -> Result<Packet, ProtocolError>`: 极速解析 Minecraft Yefira 二进制小端序数据包。
- `encode_full_sync_request() -> Vec<u8>`: 编码客户端全量快照请求包 (0x80)。
- `encode_repair_requests(sections: &[IVec3], max_batch_size: usize) -> Vec<Vec<u8>>`: 编码局部区块修复请求分片包 (0x81)。
- `encode_sync_config(throttle_mode: u8, target_fps: u8, is_active: bool) -> Vec<u8>`: 编码同步限流与帧率配置包 (0x82)。
- `Packet`: 强类型数据包枚举（`SelectionInfo`, `FullSnapshot`, `DeltaUpdate`, `SectionManifest`, `SectionSnapshot`, `HandshakeInfo`, `StreamBegin`, `StreamEnd` 等）。

### 6.3 原生实时同步会话 (`mtk_voxel::sync`)
- `LiveSyncSession`:
  - `session.start(url, auto_reconnect, max_reconnect_attempts)`: 启动原生 WebSocket 后台网络与网格构建管道。
  - `session.stop()`: 优雅断开连接。
  - `session.poll_events() -> Vec<SyncEvent>`: 非阻塞轮询已生成的 `SectionMeshReady`、`StatusChange`、`SelectionUpdated`、`Verified` 等事件。
  - `session.send_full_sync_request()`, `session.send_repair_request(...)`, `session.send_sync_config(...)`。
  - `session.get_storage()`: 访问内部共享的 `VoxelStorage`。
- `SyncEvent`: 供宿主/胶水层消费的高级同步事件。

---

## 7. 材质与映射：`mtk-material`
- `MaterialResolver`: 依据 Block ID / OBJ Material Name / 贴图哈希匹配现代 PBR 材质规范。
- `clean_icecube_name`, `clean_jmc2obj_name`, `decode_mineways_uv`, `remap_mesh_uvs_parallel`。


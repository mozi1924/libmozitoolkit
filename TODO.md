# 📋 libmozitoolkit (`libmtk`) 开发计划与版本路线图 (Roadmap)

本文档为 `libmtk` 底层核心库的功能演进规划，严格遵循**宿主无关（Host-Agnostic）**与**纯数据输入输出（Data-in, Data-out）**设计原则。前端 Blender 插件规划见 [`MoziToolKit/TODO.md`](../MoziToolKit/TODO.md)。

---

## 📌 版本演进总览 (Milestones Overview)

| 版本 | 阶段定位 | 核心主题与边界 | 状态 |
| :--- | :--- | :--- | :--- |
| **v2.0 正式版** | **核心计算内核** | **通用 3D/体素世界几何、双图集烘焙与材质渲染基石 (Production Core)**<br>完成 Anvil 存档解析、BlockState 烘焙、真实 3D 生物群系平滑采样、CTM 求解、**静态+动态双图集 (Dual-Atlas)** 以及直接指针零拷贝直写。 | ✅ **正式发版 (Released / Code Frozen)** |
| **v2.1** | **次级大版本** | **高级实体、特殊渲染器、动作重定向与场景交换 (Advanced Entities & Interchange)**<br>支持 1.21.4+ 铜像 Special Renderers 模型、骨骼动画坐标系重定向数学库、实体 MCA 文件解析、填充地图解码、方块实体 NBT 与 `.mtkscene` 自包含场景包。 | 📋 **规划排期中** |
| **v2.2** | **通用面部与格式** | **面部捕捉网络流、平滑滤波与蓝图全覆盖 (Facial Stream & Blueprint Expansion)**<br>Rust 原生高性能 LiveLinkFace UDP (11111) 与 WebSocket 异步接收、OneEuroFilter 平滑滤波、ARKit 52 向量化解算；`.litematic` 蓝图解析；2D 俯视小地图切片渲染。 | 📋 **规划排期中** |
| **v2.3+** | **万能生态扩展** | **全格式与全场景覆盖 (Multi-format & Ecosystem Expansion)**<br>基岩版 LevelDB 存储与模组世界扩展、体素反向逆向推算器 (Voxel Guesser)。 | 🔮 **远景规划** |

---

## 🎯 v2.0 正式版：核心世界计算与材质渲染基石 (✅ 已完成)

### 1. 双图集 (Dual-Atlas: Static & Animated) 烘焙与寻址映射 (✅ 已完成)
- [x] **全覆盖静态图集 (Static Chunk)**
  - [x] 全量材质保底机制：将所有方块贴图（包括动态序列帧贴图的 Frame 0 静态表示）烘焙入静态图集；
  - [x] 为未开启动画材质的用户提供零额外 Shader 开销的极致性能基础；
- [x] **动态图集与长条切片 (Animated Chunk)**
  - [x] 将 `.png.mcmeta` 动画贴图按垂直长条打包至专用 `anim_chunk`；
  - [x] `AtlasAddressMap` 提供 `lookup_static(loc)` 与 `lookup_animated(loc)` 双通道寻址。

### 2. 核心几何与 UV 算子库 (`mtk-core` - ✅ 已完成)
- [x] **自适应像素网格切分系统 (`mtk-core::subdivide`)**
  - [x] 四边形自适应细分推算与非均匀像素吸附；
  - [x] 属性双线性插值（UV、法线、顶点色、Deform 蒙皮权重、自定义属性）；
  - [x] 动画长条贴图单帧安全截断机制；
  - [x] 空间哈希拓扑微距焊接 (`weld_spatial_vertices`)。
- [x] **智能挤出与 UV 修复系统 (`mtk-core::extrude`)**
  - [x] 侧面 UV 塌陷判定与 SMART / INWARD / OUTWARD 模式几何重构；
  - [x] 3D Perlin 连续梯度噪声、Voronoi 细胞阶梯噪声与随机高度生成；
  - [x] 扁平连续网格批处理与 Crease 锐边标记。
- [x] **空间邻域遮挡剔除系统 (`mtk-cull`)**
  - [x] 6 向邻域状态机与 2D 矩形投影差集微观剔除；
  - [x] `FaceSanitizer` 叠面消重与共面微距几何清洗。

### 3. Minecraft 存档与体素源抽象 (`mtk-save`, `mtk-voxel` - ✅ 已完成)
- [x] **高性能 Java 原版存档格式解析**
  - [x] 4KiB 扇区直接寻址 Anvil / MCA 文件读取；
  - [x] 零拷贝 NBT 二进制反序列化（Chunk、BlockStates、Biomes、`LevelData`）；
  - [x] 线性压缩与非压缩调色板解包，3D 空间切片按需加载器 (`SaveLoader`)。
- [x] **统一体素源抽象层 (`mtk-voxel::source`)**
  - [x] 抽象 `VoxelSource`、`VoxelReader` 与 `VoxelWriter` 特征；
  - [x] 跨 Java 原版存档 (`AnvilWorldSource`)、全量体素点云 (`PointCloudVoxelSource`) 统一切片抽象。

### 4. BlockState 烘焙与世界重构 (`mtk-model`, `mtk-voxel` - ✅ 已完成)
- [x] **无头 Block Model JSON 烘焙引擎**
  - [x] 对齐 1.21+ 规范的 BlockState 条件规则（multipart、variants）求值；
  - [x] 元素旋转、UV Lock、TintIndex、透明度与自发光烘焙；
  - [x] 混合式实体模型架构（BER 参数化回退箱子/床/头颅/钟/陶罐/传送门，外部材质包优先）；
  - [x] 空间确定性交替模型随机旋转 (`BakedVariantGroup`) 与植物空间抖动偏移。
- [x] **真实 3D Quart Biome 染色采样与 5×5 平滑过渡核**
  - [x] 从 4×4×4 Quart 调色板查询真实 3D Biome（沼泽双色草、热带雨林、黑森林、恶地等）；
  - [x] 5×5 (R=2) 原版均匀盒式滤波（Uniform Box Filter）平滑混合，消除硬切接缝。
- [x] **CTM 连接纹理规则求解器 (`mtk-resource::ctm`)**
  - [x] OptiFine / Continuity CTM 规则解析与 47-tile 拓扑求解；
  - [x] `SectionMesher` 跨区块连通性求解与端到端回归验证。

### 5. 现代 Blender 5.0+ 零拷贝内存直写管道 (`bindings/mtk-py` - ✅ 已完成)
- [x] **C++ 连续内存指针直接写入 (`mesh/direct_ptr.rs`)**
  - [x] `direct_copy_positions_to_ptr`、`direct_copy_normals_to_ptr`、`direct_copy_indices_to_ptr`、`direct_copy_loop_uvs_to_ptr`、`direct_copy_face_materials_to_ptr` 等直接指针拷贝；
  - [x] 硬件带宽级内存直接写入，彻底消除 Python 对象封装开销。

### 6. 单文件预编译资产缓存 (`crates/mtk-package` - ✅ 已完成)
- [x] **通用二进制容器编解码器**
  - [x] 64 字节 Header + 块级 Zstd 压缩 TOC + 64 字节内存对齐；
  - [x] `precompile_all_assets` 输出单一 `<fingerprint>.mtkcache`，包含图集、Standalone PBR、模型库与 Biome 映射表。

### 7. 实时网络同步协议与连接健壮性 (`mtk-sync` - ✅ 已完成)
- [x] 小端序增量同步协议、原生 WebSocket 客户端、CRC32 清单比对自愈；
- [x] `SyncClient::stop()` 针对底层 TCP Stream 注入 `shutdown(Shutdown::Both)`，确保连接瞬间切断与端口释放。

### 8. v2.0 正式发版质量门禁与接口冻结 (P0 - ✅ 已完成)
- [x] `cargo test --workspace` 全绿（全工作区数十个 Crate 测试 100% 通过）；
- [x] `cargo clippy --workspace --all-targets -- -D warnings` 零告警通过；
- [x] `cargo fmt --all -- --check` 代码格式化一致性校验；
- [x] 导出稳定的 CPython 3.11+ 标准 ABI (`abi3`) 动态链接扩展（`libmtk_py` 通过 `tools/build_wheels.py` 构建验证通过）。

---

## 🧭 Host-Agnostic 胶水下沉 (2026-10-18)

> 背景：Blender 前端近期在胶水层积累了材质路由、Live Sync 状态与进度语义。
> 这些逻辑对 C4D / 独立查看器同样适用，应留在 Rust 核心，而不是复制到每个宿主。
> 详细清单见 [`MoziToolKit/docs/planning/2026-10-18_material_animation_and_rust_downshift.md`](../MoziToolKit/docs/planning/2026-10-18_material_animation_and_rust_downshift.md)。

### ✅ 已完成
- [x] `mtk-material` 统一 static/anim 图集路由：
  - `resolve_animated_sprite` / `resolve_routed_sprite` / `extract_animation_metrics`；
  - `remap_mesh_multi_uvs_parallel_with_options`（旧函数保持静态 Frame 0 兼容）；
  - `resolve_mesh_material_routes` / `reroute_mesh_animation`（已有 Atlas UV 的静态↔动画重投影）。
- [x] `mtk-voxel` mesher 的动画策略改为委托 `mtk-material`，消除双实现。
- [x] `mtk-sync`：
  - `PersistedSessionState`（JSON roundtrip、UUID hex、HELLO、user_modified）；
  - `normalize_stream_progress`（两阶段权重只在 Rust 计算）；
  - `SyncEvent::Handshake.decision_code`（避免宿主拿字符串和数字比较）。
- [x] `bindings/mtk-py` 暴露上述 API。
- [x] **P1 Biome 即时更新下沉**：
  - `mtk-material::biome::reapply_uniform_biome_tint_flat`（Rayon 扁平输出）；
  - PyO3 零拷贝 memoryview；Blender 侧 `foreach_set` 写入；
  - 498k faces 实测 6~8 ms → ~2.1 ms。
- [x] DNA 适用范围实测：通用 face 属性 `foreach_set` 与 DNA memcpy 等价，
      且 DNA 需要手动 `mesh.update()`；DNA 继续用于拓扑/位置/法线/UV/顶点色/点云。

### 📋 后续
- [x] **P1 外部网格材质提取**：Blender 侧 `foreach_get` + NumPy 索引；
  Rust `remap_mesh_multi_uvs_paletted_with_options`（palette + u32 indices）与零拷贝 id/metric buffer；
  500k faces replace 2.65 s → ~0.95 s。
- [ ] **P1 1M faces 进一步优化**：palette-native biome / source-key 展开优化，目标 < 1.2 s。
- [x] **P2 `mtk-sync` 事件合并**：`poll_events_coalesced` + 最新 world mesh generation。
- [x] **P2 资产缓存解析**：`libmtk::resolve_active_cache_package` + PyO3。
- [x] **P2 `bindings/mtk-ffi` 首批**：material route / persisted-state / biome recolor C-ABI；
- [x] **代码/文档清理**：修正失效 API 文档链接；保留稳定的 C-ABI，不删除已发布 Rust API。
- [ ] **P2 `bindings/mtk-ffi` 后续**：按 C4D 实际约定补 opaque session/cache handle 与头文件示例。

---

## 🚀 v2.1 版本：高级实体、特殊渲染器、动作重定向与场景交换 (规划排期中 📋)

> **2.1 版本主题**：拓展特殊渲染模型、实体数据流、动作重定向数学库与自包含场景交换包。

### 1. 1.21.4+ 客户端特殊模型攻坚 (Special Renderers & Copper Golem Statue) (P1)
- [ ] **解析客户端模型新语法 (`mtk-model::special`)**
  - [ ] 扩展模型反序列化器，识别 `"type": "minecraft:special"` 与 `"type": "minecraft:select"`；
- [ ] **铜像 / 铜傀儡像 (Copper Golem Statue) 几何模型生成**
  - [ ] 当缺少传统 `"elements"` 字段时，路由至特殊渲染器模型构建器；
  - [ ] 内置铜傀儡像 4 种氧化态（未氧化、斑驳、锈蚀、完全氧化）与各姿态（standing, sitting, running, star）的盒模型拓扑与贴图解算。

### 2. 动画曲线与骨骼重定向数学库 (`mtk-anim::retarget` - P1)
- [ ] **Mine-imator 关键帧与曲线解算**
  - [ ] 实现 Linear、Instant 与 Bézier 贝塞尔缓动曲线的离散数值解算器；
- [ ] **骨骼变换与坐标轴重定向矩阵流 (Bone Retargeting Math)**
  - [ ] 实现针对 Y/Z 轴向翻转、四元数局部与世界旋转重定向的高性能批量批处理计算；
  - [ ] 为前端提供从 MI 临时参考骨骼快速重定向至 Rig2 / Ice Cube / BPS / Thomas 的向量化通道。

### 3. 原版静态与悬挂实体解析 (`mtk-save::entity`) (P1)
- [ ] **`entities/r.X.Z.mca` 独立实体区域文件解析**
  - [ ] 扩展 MCA 读取器，支持 Minecraft 1.17+ 独立实体区域文件；
  - [ ] 基于选区 AABB 过滤空间相交实体。
- [ ] **艺术画 (`PaintingEntity`) 解析**
  - [ ] 提取 `variant`, `facing`, `pos`，内置原版 40+ 种画作尺寸表，生成带木质背板的三维画框网格。
- [ ] **物品展示框 (`ItemFrameEntity`) 解析**
  - [ ] 提取展示框旋转角度、发光属性与内容物信息。
- [ ] **原版填充地图数据解码器 (`mtk-save::map`)**
  - [ ] 解析 `data/map_<id>.dat` Gzip NBT 文件；
  - [ ] 内置 60+ 种 `MapColor` 基础色彩表与 4 级明暗倍率，快速解码 128×128 RGBA8 连续像素流。

### 4. 高级方块实体 NBT 图元生成器 (`mtk-model::ber_nbt`) (P2)
- [ ] **告示牌文字生成器 (Sign Text)**
  - [ ] 解析 `front_text` / `back_text`，按原版点阵排版生成面文字图元与发光材质标记。
- [ ] **旗帜多层花纹合成 (Banner Patterns)**
  - [ ] 解析 `Patterns` 列表，依照原版公式按顺序进行 16 种颜色的多层 Alpha 混合合成。
- [ ] **双箱 (Double Chest) 几何缝合与铰链预留**
  - [ ] 依据 BlockState 的 `type=left / type=right` 自动缝合为宽箱子网格，并为箱盖预留铰链轴心。

### 5. 自包含轻量场景交换包 (`mtk-package::scene`) (P2)
- [ ] **导出端按需剪枝 (`export_scene_package`)**
  - [ ] 扫描选区体素世界，提取最小依赖模型与独立贴图闭包（Tree-shaking）；
  - [ ] 稀疏 16×16×16 Section 游程编码写入 `VOXL` Chunk，贴图写入 `TXTR` Chunk。
- [ ] **导入端现场重构 (`import_scene_package`)**
  - [ ] 读取 `VOXL` 与 `TXTR`，现场构建微型图集或材质槽，重构世界几何。

---

## 🛠️ v2.2 版本：通用面部网络流、平滑滤波与蓝图全覆盖 (规划排期中 📋)

### 1. 原生高性能面部捕捉网络流与平滑滤波 (`mtk-face` / `mtk-net` - P1 核心 🌟)
- [ ] **高并发无阻塞网络监听器**
  - [ ] 原生 Rust 实现 LiveLinkFace (UDP 11111) 与 WebSocket 异步数据包监听；
  - [ ] 零拷贝快速解析 ARKit 52 种 Blendshape 浮点数组与头部姿态四元数；
- [ ] **自适应抗抖动平滑滤波器 (One Euro Filter & Kalman)**
  - [ ] 实现针对高频面捕噪声的 One Euro Filter 自适应低通滤波，消除面部微颤；
  - [ ] 处理网络丢包抖动与插值补偿；
- [ ] **通用骨骼偏移向量计算**
  - [ ] 接收前端传入的通用映射规则表，Rust 端多线程执行矩阵乘加，直接将 Blendshape 流折算为骨骼位移/旋转流，零延迟注入 Blender。

### 2. 建筑蓝图文件直接解析器 (`mtk-schematic` - P1)
- [ ] **`.litematic` / `.schematic` 纯 Rust 解析**
  - [ ] 无需世界存档，高性能解包 Litematica 区域、BlockState 调色板与方块实体；
  - [ ] 直接对接 `VoxelSource`，一键生成完整建筑几何。

### 3. 2D 俯视微型小地图切片渲染器 (`mtk-voxel::minimap` - P1)
- [ ] **快速光线表面高度与群系色彩投影**
  - [ ] 基于体素流快速推算地表高度、水深与方块群系颜色；
  - [ ] 极速输出连续 RGBA8 贴图内存，供前端 2D 小地图选区器实时缩放平移预览。

---

## 🔮 v2.3+ 远景：万能格式与生态扩展 (Future Vision)

- [ ] **基岩版（Bedrock Edition）存储读取 (`mtk-voxel::storage::leveldb`)**
  - 嵌入式纯 Rust LevelDB 存储格式解析与 SubChunk 运行时 ID 映射。
- [ ] **体素反向逆向推算器 (Voxel Guesser / Mesh Inferrer)**
  - 从第三方网格（Mineways / MiEx 导出的 OBJ/USD）中反向提取空间占位、材质与 UV，推导方块 ID 与 BlockState。

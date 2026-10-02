# 📋 libmozitoolkit (`libmtk`) 开发路线与 TODO 清单

本文档为 `libmtk` 底层核心库的功能演进规划，严格遵循**宿主无关（Host-Agnostic）**与**纯数据输入输出（Data-in, Data-out）**设计原则。

---

## 阶段一：核心几何与 UV 算子库（✅ 已完成）
- [x] **自适应像素网格切分系统 (`mtk-core::subdivide`)**
  - [x] 四边形自适应细分推算（基于纹理分辨率与 UV 物理边长向量）。
  - [x] 非均匀吸附在纹理整数像素边界上的切分参数计算。
  - [x] 属性双线性插值（UV、法线、顶点色、蒙皮权重、自定义属性图层）。
  - [x] 动画长条贴图单帧正方形安全截断机制（防面数爆炸）。
  - [x] 拓扑微距焊接与流形缝合清理 (`weld_spatial_vertices`)。
- [x] **智能挤出与 UV 修复系统 (`mtk-core::extrude`)**
  - [x] 侧面 UV 塌陷判定与 SMART / INWARD / OUTWARD 模式几何重构。
  - [x] Atlas 图集安全边界裁剪（Safe Padding Clamp）。
  - [x] 3D Perlin 连续梯度噪声、Voronoi 细胞阶梯噪声与 Uniform Random 随机高度生成器。
  - [x] 扁平连续内存网格 (`FlatPolygonMesh`) 批量批处理与 Crease 锐边标记。
- [x] **空间邻域遮挡剔除系统 (`mtk-cull`)**
  - [x] 基于 6 向邻域状态机与 2D 矩形投影差集的微观遮挡剔除。
  - [x] `FaceSanitizer` 叠面消重与共面微距几何清洗。
- [x] **Python 极速胶水绑定 (`bindings/mtk-py`)**
  - [x] `libmtk_py` 导出切分、挤出与剔除批量接口，NumPy/Buffer Protocol 零拷贝对接。

---

## 阶段二：Minecraft 存档与体素源抽象（🚧 规划中）
- [ ] **Java 原版存档格式解析 (`mtk-voxel::storage::anvil`)**
  - [ ] 高性能 Anvil / MCA 文件格式读取器。
  - [ ] 快速无内存分配 NBT 二进制反序列化器（Chunk、BlockStates、Heightmaps、Biomes）。
  - [ ] 线性压缩与非压缩调色板（Palette）数据解包。
- [ ] **Java 模组（Modded）世界支持**
  - [ ] 动态扩展方块命名空间与自定义 BlockState 属性解析（兼容 Forge/Fabric/NeoForge）。
- [ ] **基岩版（Bedrock Edition）存档读取 (`mtk-voxel::storage::leveldb`)**
  - [ ] 嵌入式纯 Rust LevelDB 存储格式解析。
  - [ ] 基岩版独特的 SubChunk 格式、方块状态定义与运行时 ID 映射。
- [ ] **统一体素源抽象层 (`mtk-voxel::source`)**
  - [ ] 统一抽象 `VoxelSource`、`VoxelReader` 与 `VoxelWriter` 特征（Trait）。
  - [ ] 实现跨 Java / Bedrock / 内存动态体素的无缝切换与流式切片迭代。

---

## 阶段三：BlockState 烘焙、逆向推算与世界重构（🚧 规划中）
- [ ] **无头 Block Model JSON 烘焙引擎 (`mtk-model`)**
  - [ ] 对齐 1.21+ 规范的 BlockState 条件规则（multipart、variants）求值。
  - [ ] Block Model 元素旋转、UV Lock、TintIndex 与透明度烘焙。
- [ ] **体素反向逆向推算器 (Voxel Guesser / Mesh Inferrer)**
  - [ ] 从已有导入来源的网格（如 Mineways/MiEx 导出的 OBJ/USD）中反向提取空间占位、材质与 UV。
  - [ ] 结合启发式几何特征与生物群系色彩逆向推导方块 ID 与 BlockState。
- [ ] **基于体素的世界重构网格化器 (`mtk-voxel::mesher`)**
  - [ ] 贪婪网格化（Greedy Meshing）与多材质四边形合并。
  - [ ] 平滑环境光遮蔽 (Smooth AO) 与流体（水/岩浆）曲面重构。
  - [ ] 直接套用外部高精度材质包（Resourcepack）模型进行几何世界重构。

---

## 阶段四：实时网络同步与体素元数据流（🚧 规划中）
- [ ] **小端序二进制增量同步协议 (`mtk-sync::protocol`)**
  - [ ] 完善 Block Update、Chunk Section Delta、Entity/Player Transform 事件编解码。
  - [ ] 高并发 WebSocket 客户端与无锁环形事件队列。
- [ ] **体素元数据与属性映射**
  - [ ] 设计紧凑的高保真体素元数据表示（方块ID、状态、方向、附加 NBT 属性）。
  - [ ] 构建体素数据 $\leftrightarrow$ 点云（Point Cloud）/ 网格属性（Mesh Attributes）的双向元数据映射机制。
- [ ] **增量式实时世界状态持久化**
  - [ ] 内存世界差量更新与快速快照导出。

---

## 阶段五：跨平台绑定与生态扩展（📅 远期规划）
- [ ] **Python 扩展完善（P0 核心）**
  - [ ] 为阶段二、三、四的新能力导出 Pythonic 零拷贝 API。
  - [ ] 与 Blender 前端 `MoziToolKit` 保持 100% 同步演进与 ABI 隔离。
- [ ] **无头独立 CLI 工具链 (`mtk-cli` - P1)**
  - [ ] `mtk precompile`：端到端材质图集拼接与资源包预烘焙。
  - [ ] `mtk inspect`：资产缓存与 MCA 存档诊断。
  - [ ] `mtk bench`：网格化与遮挡剔除基准性能压测。
- [ ] **跨语言 C-ABI FFI 与 WebAssembly 绑定 (`mtk-ffi` / `mtk-wasm` - P2)**
  - [ ] 导出标准 C 头文件 (`mtk.h`)，为 Maya、Houdini、C# (Unity)、Godot 扩展预备。
  - [ ] 导出 WebGPU / WebAssembly 零拷贝 TypedArray 视图，为 Web 端预览工具预备。

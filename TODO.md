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

## 阶段二：Minecraft 存档与体素源抽象（核心已完成 ✅ / 模组与基岩版待扩展 🚧）
- [x] **Java 原版存档格式解析 (`mtk-save`)**
  - [x] 高性能 Anvil / MCA 文件格式读取器（4KiB 扇区直接寻址）。
  - [x] 快速内置零拷贝 NBT 二进制反序列化器（Chunk、BlockStates、Biomes、`LevelData`）。
  - [x] 线性压缩与非压缩调色板（Palette）数据解包。
  - [x] 3D 轴对齐空间切片按需加载器（`SaveLoader`，支持多维度与物理进度汇报）。
- [ ] **Java 模组（Modded）世界支持**
  - [ ] 动态扩展方块命名空间与自定义 BlockState 属性解析（兼容 Forge/Fabric/NeoForge）。
- [ ] **基岩版（Bedrock Edition）存档读取 (`mtk-voxel::storage::leveldb`)**
  - [ ] 嵌入式纯 Rust LevelDB 存储格式解析。
  - [ ] 基岩版独特的 SubChunk 格式、方块状态定义与运行时 ID 映射。
- [x] **统一体素源抽象层 (`mtk-voxel::source`)**
  - [x] 统一抽象 `VoxelSource`、`VoxelReader` 与 `VoxelWriter` 特征（Trait）。
  - [x] 实现跨 Java 原版存档 (`AnvilWorldSource`)、全量体素点云 (`PointCloudVoxelSource`)、纯代码调试世界的无缝切换与流式切片迭代。

---

## 阶段三：BlockState 烘焙、逆向推算与世界重构（核心已完成 ✅ / 逆向推算待开发 🚧）
- [x] **无头 Block Model JSON 烘焙引擎 (`mtk-model`)**
  - [x] 对齐 1.21+ 规范的 BlockState 条件规则（multipart、variants）求值。
  - [x] Block Model 元素旋转、UV Lock、TintIndex、透明度与自发光烘焙。
  - [x] 混合式实体模型架构（BER 参数化回退箱子/床/头颅/钟/陶罐/传送门，外部材质包优先）。
  - [x] 预烘焙交替方块模型随机旋转（`BakedVariantGroup`，1:1 对齐 Java 版 `Mth.getSeed` + `JavaRandom` 空间确定性采样）。
  - [x] 植物三维空间确定性抖动（Plant Offsets，双层植物 Y=0 对齐位移）。
  - [x] 紧凑模型数据库序列化与透明 zstd 压缩缓存。
- [ ] **体素反向逆向推算器 (Voxel Guesser / Mesh Inferrer)**
  - [ ] 从已有导入来源的网格（如 Mineways/MiEx 导出的 OBJ/USD）中反向提取空间占位、材质与 UV。
  - [ ] 结合启发式几何特征与生物群系色彩逆向推导方块 ID 与 BlockState。
- [ ] **基于体素的世界重构网格化器 (`mtk-voxel::mesher`)**
  - [ ] 贪婪网格化（Greedy Meshing）与多材质四边形合并。
  - [x] 平滑环境光遮蔽 (Smooth AO) 与流体（水/岩浆）曲面重构。
  - [x] 直接套用外部高精度材质包（Resourcepack）模型进行几何世界重构。
  - [x] 5×5 (R=2) 生物群系平滑过渡与边界防渗色。
- [ ] **动态资产热重载与区块网格缓存清空 (`mtk-voxel::world`)**
  - [ ] 完善 `VoxelWorld::clear_cache()` 与动态热重载支持，避免材质包重新预编译后因脏区块网格缓存导致 UV/模型错位。

---

## 阶段四：实时网络同步与体素元数据流（核心已完成 ✅ / 连接健壮性待深化 🚧）
- [x] **小端序二进制增量同步协议 (`mtk-sync::protocol`)**
  - [x] 完善 Block Update、Chunk Section Delta、选区变动、握手与两阶段流传输事件编解码。
  - [x] 原生 WebSocket 客户端与非阻塞事件队列（`LiveSyncSession`）。
  - [x] CRC32 清单比对与分批差量自愈修复机制。
- [x] **跨平台连接健壮性与强制断开 (`mtk-sync::client`)**
  - [x] `SyncClient::stop()` 针对底层 TCP Stream 注入 `shutdown(Shutdown::Both)`，确保宿主工程切换或重连时套接字瞬间切断，消除服务端连接占位与挂起。
- [x] **体素元数据与属性映射**
  - [x] 设计紧凑的高保真体素元数据表示（方块ID、状态、绝对世界坐标、光照等级、生物群系）。
  - [x] 构建体素数据 $\leftrightarrow$ 点云（`VoxelPointCloud`）/ 网格属性（Mesh Attributes）的双向元数据映射机制。
  - [x] 支持用户交互删点雕刻并无损重构网格（`PointCloudVoxelSource`）。
- [x] **增量式实时世界状态持久化**
  - [x] 内存世界差量更新与快速快照导出。

---

## 阶段五：跨平台绑定与生态扩展（基础已落地 ✅ / 持续演进 🚧）
- [x] **Python 扩展完善（P0 核心）**
  - [x] 为阶段二、三、四的新能力导出 Pythonic 零拷贝 API（`mtk-save` 存档检测与切片网格化、`VoxelPointCloud`、`VoxelWorld`、`LiveSyncSession`、Buffer Protocol 流体修复）。
  - [x] 与 Blender 前端 `MoziToolKit` 保持 100% 同步演进与 ABI 隔离。
- [x] **无头独立 CLI 工具链 (`mtk-cli` - P1)**
  - [x] `mtk atlas`（端到端图集烘焙与预编译）。
  - [x] `mtk model` / `mtk export` / `mtk verify`（模型几何排查、OBJ 导出与规则校验）。
  - [x] `mtk bench`（网格化、模型烘焙与性能压测）。
- [x] **跨语言 C-ABI FFI 与 WebAssembly 绑定 (`mtk-ffi` / `mtk-wasm` - P2)**
  - [x] 导出标准 C 头文件 (`mtk.h`)，为 Maya、Houdini、C# (Unity)、Godot 扩展预备。
  - [x] 导出 WebGPU / WebAssembly 零拷贝 TypedArray 视图，为 Web 端预览工具预备。

---

## 阶段六：通用二进制中间包与场景交换容器 (`mtk-package` / 规范已就绪 📋 / 待实现 🚧)
- [x] **规范与容器架构设计（✅ 已落地）**
  - [x] 确立统一 Header (64B) + 块级 Zstd 压缩 TOC + 64 字节内存对齐规范（详见 [`docs/PACKAGE_SPEC.md`](docs/PACKAGE_SPEC.md)）。
  - [x] 确立两大业务 Profile：`AssetCache` (`.mtkcache`) 与 `SceneInterchange` (`.mtkscene`)。
- [ ] **通用二进制容器编解码器 (`crates/mtk-resource::package` 或独立 crate)**
  - [ ] 实现 `MtkPackageWriter`：流式 Chunk 写入、自动 64 字节边界 Padding、Zstd 块压缩与 xxHash64/CRC32 校验码计算。
  - [ ] 实现 `MtkPackageReader`：基于 `memmap2` 零内存拷贝打开、Header 校验、TOC 解析与单 Chunk 懒加载。
- [ ] **全量资产预编译缓存打包迁移 (`libmtk::prebake`)**
  - [ ] `precompile_all_assets_to_package`：将图集切片、Standalone PBR 贴图、`models.bin`、Biome 映射表打包为单一 `.mtkcache` 文件。
  - [ ] 提供解包/就地挂载能力，完全替代文件系统散文件展开，降低磁盘 I/O 碎片与复制开销。
- [ ] **自包含轻量场景交换包管线 (`mtk-voxel::package`)**
  - [ ] **导出端按需剪枝 (`export_scene_package`)**：
    - [ ] 扫描所选体素世界的 BlockState，提取最小依赖模型与独立贴图闭包（Tree-shaking）。
    - [ ] 稀疏 16×16×16 Section 游程编码 (RLE) 与位打包写入 `VOXL` Chunk。
    - [ ] 独立贴图去重写入 `TXTR` Chunk（严禁存固化图集，保留原始高画质/PBR伴生图）。
  - [ ] **导入端现场重构 (`import_scene_package`)**：
    - [ ] 读取 `VOXL` 填充 `VoxelStorage`。
    - [ ] 内存现场构建场景专属微型图集（On-the-fly Atlas）或独立材质槽。
    - [ ] 执行 `SectionMesher` 动态重构世界几何并零拷贝灌入宿主网格。


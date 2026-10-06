# MTK 二进制中间存储包与场景交换容器规范 (Package Specification)

本文档是 `libmozitoolkit` (`libmtk`) 与 `MoziToolKit` 生态的通用二进制存储规范，定义了统一的 **MTK 二进制容器结构 (`.mtk` / `.mtkpack` / `.mtkscene`)**，涵盖**全量资产预编译缓存**与**轻量级场景中间交换**两大业务场景。

---

## 1. 设计动机与核心哲学

### 1.1 现实痛点
1. **散文件膨胀与 I/O 碎片 (Loose Files Overhead)**：
   目前全量资产预编译缓存直接展开在文件系统目录中（包含数千至数万个小尺寸 PNG 贴图、模型 JSON、映射表等）。这导致了磁盘 inode 激增、文件系统元数据查找延迟、跨磁盘/跨机复制极慢、以及缓存完整性校验成本极高。
2. **传统 3D 交换格式的“几何爆炸” (Geometry Explosion in 3D Formats)**：
   若将 Minecraft 建筑/世界选区直接导出为传统网格格式（OBJ、USD、FBX、glTF），数十万个方块会膨胀为数百万个多边形面、海量法线与 UV 缓冲，导致文件体积迅速膨胀至数百 MB 甚至数 GB。而且一旦烘焙为网格，面剔除规则、环境光遮蔽 (AO) 样式、图集分辨率即被永久固化，接收方无法再根据 DCC 宿主或材质管线灵活调整。

### 1.2 核心哲学
- **统一底层容器 (Unified Container Architecture)**：
  缓存包与交换包共享完全一致的二进制底层物理结构（统一文件头、分块目录表 TOC、Zstd 块压缩、64 字节对齐）。
- **Profile 业务分化 (Specialized Profiles)**：
  - **`AssetCache Profile`（全量预编译缓存包）**：打包全量预烘焙图集、Standalone PBR 贴图库、全局模型数据库与生物群系映射表，用于本地毫秒级即时加载。
  - **`SceneInterchange Profile`（轻量场景交换包）**：**不存全量网格**（仅存稀疏体素网格），**不存预烘焙图集**（仅按需打包场景用到的独立材质贴图），**按需剪枝模型**。在接收方端现场根据宿主配置实时重构网格并按需构建材质节点或现场图集。

---

## 2. 二进制物理布局 (Physical Layout)

文件整体采用 **Header + Chunk Payloads + Table of Contents (TOC)** 的分块组织形式，支持内存映射 (`mmap`) 零内存开销快速寻址。

```
┌────────────────────────────────────────────────────────┐
│ Fixed 64-Byte Header (固定文件头，含 Magic、版本、TOC 偏移)  │
├────────────────────────────────────────────────────────┤
│ Chunk 0: META  (元数据描述，对齐至 64 字节边界)           │
├────────────────────────────────────────────────────────┤
│ Chunk 1: VOXL  (体素 Section 数据块)                     │
├────────────────────────────────────────────────────────┤
│ Chunk 2: MODL  (方块模型定义 / 烘焙模型缓存)             │
├────────────────────────────────────────────────────────┤
│ Chunk 3: TXTR  (独立贴图 / 纹理数据块流)                 │
├────────────────────────────────────────────────────────┤
│ ... (其余数据分块)                                      │
├────────────────────────────────────────────────────────┤
│ Table of Contents (TOC，分块索引表，可独立 Zstd 压缩)    │
└────────────────────────────────────────────────────────┘
```

---

## 3. 文件头规范 (Header Specification)

固定占用 **64 字节**，所有数值字段严格遵循**小端序 (Little-Endian)**。

| 偏移 (Offset) | 类型 (Type) | 字段名称 (Name) | 详细描述 (Description) |
| :--- | :--- | :--- | :--- |
| `0x00` | `[u8; 4]` | `magic` | 固定魔数，ASCII `b"MTKP"` (`0x4D, 0x54, 0x4B, 0x50`) |
| `0x04` | `u16` | `version_major` | 主版本号（格式发生非兼容性结构调整时递增，当前为 `1`） |
| `0x06` | `u16` | `version_minor` | 次版本号（新增可选 Chunk 或向前兼容字段时递增，当前为 `0`） |
| `0x08` | `u16` | `profile` | 包类型：`0 = AssetCache` (全量缓存), `1 = SceneInterchange` (场景交换) |
| `0x0A` | `u16` | `flags` | 特性位标志：<br>• Bit 0: LittleEndian (固定为 1)<br>• Bit 1: TOC 经 Zstd 压缩<br>• Bit 2: 完全自包含 (Self-Contained)<br>• Bit 3: 外部资产引用模式 (Referenced) |
| `0x0C` | `u8` | `compression` | 默认分块压缩算法：`0 = None`, `1 = Zstd`, `2 = LZ4` |
| `0x0D` | `u8` | `checksum_type` | 校验和算法：`0 = CRC32`, `1 = xxHash64` |
| `0x0E` | `[u8; 2]` | `reserved` | 预留对齐填充，填 `0x00` |
| `0x10` | `u64` | `created_at` | 创建时间 Unix Epoch 时间戳（秒） |
| `0x18` | `[u8; 16]` | `fingerprint` | 内容唯一指纹 / 资源栈哈希 / UUID |
| `0x28` | `u64` | `toc_offset` | 分块索引目录表 (TOC) 在文件中的绝对字节偏移 |
| `0x30` | `u64` | `toc_length` | 分块索引目录表 (TOC) 占用的字节长度 |
| `0x38` | `u32` | `chunk_count` | 包含的数据分块条目数量 |
| `0x3C` | `u32` | `header_crc32` | 前 60 字节 (`0x00..0x3C`) 的 CRC32 校验值，防止文件头损坏 |

---

## 4. 分块目录表 (Table of Contents - TOC)

TOC 位于文件末尾（或由 `toc_offset` 明确寻址）。当 `flags` 中的 Bit 1 置位时，整个 TOC 缓冲区通过 Zstd 解压后解析。

TOC 由 `chunk_count` 个连续的条目组成：

```rust
pub struct ChunkEntry {
    /// 分块类型标识 (如 b"META", b"VOXL", b"MODL", b"TXTR", b"BIOM", b"ATLS")
    pub chunk_type: [u8; 4],
    /// 分块在文件中的绝对字节起始偏移 (必须 64 字节对齐)
    pub offset: u64,
    /// 压缩后占用字节长度 (若未压缩则等于 decompressed_size)
    pub compressed_size: u64,
    /// 原始解压后字节长度
    pub decompressed_size: u64,
    /// 专用分块压缩算法 (0 = 无压缩, 1 = Zstd, 2 = LZ4)
    pub compression: u8,
    /// 预留字段对齐
    pub reserved: [u8; 7],
    /// 分块原始数据的 64 位校验码 (xxHash64 或 CRC32)
    pub checksum: u64,
    /// 资源唯一路径或逻辑键名 (UTF-8 字符串，前缀 u16 长度)
    pub identifier: String,
}
```

---

## 5. 核心分块类型 (Chunk Protocols)

### 5.1 `META` - 全局元数据块
* **格式**：JSON 或 Bincode 序列化。
* **内容**：
  * 生成工具标识与版本（如 `libmozitoolkit v0.2.0`, `Blender 4.2`）。
  * 场景坐标 AABB 选区 `(min_x, min_y, min_z) -> (max_x, max_y, max_z)`。
  * 依赖资源包列表与哈希指纹。
  * 场景统计信息（方块总数、不重复方块种类、依赖纹理数）。

### 5.2 `VOXL` - 稀疏体素网格块 (仅 SceneInterchange)
* **存储拓扑**：
  1. **全局 BlockState 调色板**：`Vec<String>`（例如 `["minecraft:air", "minecraft:stone", "minecraft:oak_log[axis=y]", ...]`），赋予每个状态唯一的 `u16` 状态 ID。
  2. **稀疏 Section 列表**：仅记录非空 16×16×16 Section：
     - Section 三维坐标 `(sec_x: i32, sec_y: i32, sec_z: i32)`。
     - Section 方块数据：局部轻量调色板 + 4/8/16 位紧凑打包数组 + RLE 游程编码。
     - 生物群系数据：4×4×4 Quart 局部生物群系调色板与索引流。
     - 流体液位：可选的高紧凑水深/岩浆层掩码。

### 5.3 `MODL` - 模型定义与几何块
* **存储内容**：
  * **按需剪枝 (Tree-shaking)**：仅序列化场景体素实际引用到的方块模型。
  * 支持两种内嵌格式：
    1. **`BlockModelJson` 树**：轻量级 JSON 结构（体积极小，由接收方现场根据材质栈烘焙）。
    2. **`BakedModelDatabase`**：已预烘焙的旋转变换与多边形基元（体积略大，但导入时免去烘焙计算，达到极致重构速度）。

### 5.4 `TXTR` - 独立贴图与纹理块
* **存储原则**：
  * 场景交换包中**严禁存储固化的大尺寸图集**，统一存储独立的原始纹理条目（Sprite）。
  * 相同纹理全场景**严格去重**（例如场景中 50,000 个石头仅包含一个 `textures/block/stone.png` 条目）。
  * 支持原始 PNG 字节流（压缩率高、通用性好）或 Raw RGBA + Zstd（极速流式灌入 GPU）。
  * 可选携带伴生 PBR 贴图（`_n.png` 法线、`_s.png` 高光光泽度）。

### 5.5 `ATLS` - 图集与映射块 (仅 AssetCache)
* 存储全量烘焙出的图集切片（`atlas_0.png`, `atlas_0_n.png`, `atlas_0_s.png`）。
* 存储 `AtlasAddressMap` 映射表，使宿主无需重新执行装箱计算，直接根据 UV 坐标寻址。

### 5.6 `BIOM` - 生物群系调色板映射块
* 存储原版及自定义生物群系色阶映射表与 Colormap 贴图（`grass.png`, `foliage.png`）。
* 用于重构网格时在 Python / DCC 端即时计算双线性插值顶点色与 Shader 调色。

---

## 6. 两大业务 Profile 规范对照

| 特性维度 | Profile A: `AssetCache` (`.mtkcache`) | Profile B: `SceneInterchange` (`.mtkscene`) |
| :--- | :--- | :--- |
| **应用场景** | 全量材质包预编译缓存、本地资产库 | 用户工程分享、跨机器/跨 DCC 场景交换 |
| **体素数据 (`VOXL`)** | ❌ 无 | ✅ 仅包含选区内的稀疏体素网格 |
| **贴图策略 (`TXTR` / `ATLS`)** | ✅ 包含完整多通道 Atlas 与 Standalone PBR | ✅ **仅包含场景引用的独立贴图 (按需)**，不存固化图集 |
| **模型策略 (`MODL`)** | ✅ 包含全局所有方块的完整模型数据库 | ✅ **仅包含场景中实际使用的模型闭包 (剪枝)** |
| **网格模型数据** | ❌ 不存烘焙的世界大网格 | ❌ **不存全量网格**，现场根据体素+模型实时重构 |
| **典型文件体积** | 50 MB ~ 500 MB (取决于材质包分辨率) | **500 KB ~ 15 MB** (体积小 1~2 个数量级) |
| **重建灵活性** | 高速直通读取 | 极高（导入端可自选 Greedy Meshing、AO、图集/独立材质） |

---

## 7. 现场重构管线 (Import & Reconstruct Pipeline)

当接收方导入 `.mtkscene` 文件时，`libmtk` 与 `MoziToolKit` 执行以下无损重构流程：

```
                    ┌─────────────────────────┐
                    │  Read .mtkscene (mmap)  │
                    └────────────┬────────────┘
                                 │
           ┌─────────────────────┴─────────────────────┐
           ▼                                           ▼
┌───────────────────────┐                   ┌─────────────────────────┐
│ Extract Voxel Storage │                   │ Extract Needed Textures │
│  (Sparse Sections)    │                   │   & Model Definitions   │
└──────────┬────────────┘                   └────────────┬────────────┘
           │                                             │
           │                                             ▼
           │                                ┌─────────────────────────┐
           │                                │ Build On-The-Fly Atlas  │
           │                                │ or Standalone Shaders   │
           │                                └────────────┬────────────┘
           │                                             │
           └─────────────────────┬───────────────────────┘
                                 ▼
                    ┌─────────────────────────┐
                    │  Run SectionMesher      │
                    │  (Greedy / AO / Tint)   │
                    └────────────┬────────────┘
                                 ▼
                    ┌─────────────────────────┐
                    │   Emit MeshData into    │
                    │  Blender Scene (Zero-C) │
                    └─────────────────────────┘
```

1. **零拷贝秒级解析**：通过 `mmap` 读取文件头与 TOC，解析元数据与依赖清单。
2. **材质管线决策**：
   - 若用户选择“图集模式”：在内存中现场调用 `AtlasBuilder` 将提取出的独立贴图快速拼成一张紧凑场景专属小图集，并重新计算 UV 偏移；
   - 若用户选择“独立材质模式”：直接为各贴图构建独立的 Principled BSDF 节点树。
3. **体素世界网格化**：
   - 将 `VOXL` 数据喂入 `VoxelStorage`；
   - 根据用户导入面板配置（如是否启用平滑 AO、是否开启面剔除、是否合并共面），调用 `SectionMesher` 动态生成几何网格流；
4. **批量灌入场景**：通过 Buffer Protocol 将顶点与索引极速写入 DCC 宿主物体。

---

## 8. 实施规范与约束

1. **宿主无关原则**：容器编解码器（Reader/Writer）必须是纯 Rust 实现，严禁依赖任何 DCC API。
2. **对齐与零拷贝**：所有 Chunk 数据起始偏移必须 64 字节对齐，以支持无损 Direct I/O 与 `mmap`。
3. **容错与前瞻兼容性**：读写器遇到未知非关键 Chunk 类型时，必须平滑跳过而不是 Panic 崩溃。

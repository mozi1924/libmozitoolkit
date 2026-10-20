# MTK 二进制资产缓存包规范 (Package Specification)

本文档是 `libmozitoolkit` (`libmtk`) 与 `MoziToolKit` 生态的通用二进制存储规范，定义了统一的 **MTK 资产缓存容器结构 (`.mtkcache`)**。

---

## 1. 设计动机与核心哲学

### 1.1 现实痛点
1. **散文件膨胀与 I/O 碎片 (Loose Files Overhead)**：
   目前全量资产预编译缓存直接展开在文件系统目录中（包含数千至数万个小尺寸 PNG 贴图、模型 JSON、映射表等）。这导致了磁盘 inode 激增、文件系统元数据查找延迟、跨磁盘/跨机复制极慢、以及缓存完整性校验成本极高。

### 1.2 核心哲学
- **统一底层容器 (Unified Container Architecture)**：
  固定文件头、分块目录表 TOC、Zstd 块压缩与 64 字节对齐。
- **唯一业务形态**：`AssetCache` 包打包全量预烘焙图集、Standalone PBR 贴图库、全局模型数据库与生物群系映射表，用于本地毫秒级即时加载。2.0 不再提供场景交换 Profile。

---

## 2. 二进制物理布局 (Physical Layout)

文件整体采用 **Header + Chunk Payloads + Table of Contents (TOC)** 的分块组织形式，支持内存映射 (`mmap`) 零内存开销快速寻址。

```
┌────────────────────────────────────────────────────────┐
│ Fixed 64-Byte Header (固定文件头，含 Magic、版本、TOC 偏移)  │
├────────────────────────────────────────────────────────┤
│ Chunk 0: META  (元数据描述，对齐至 64 字节边界)           │
├────────────────────────────────────────────────────────┤
│ Chunk 1: MODL  (方块模型定义 / 烘焙模型缓存)             │
├────────────────────────────────────────────────────────┤
│ Chunk 2: TXTR  (独立贴图 / 纹理数据块流)                 │
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
| `0x08` | `u16` | `profile` | 包类型：仅 `0 = AssetCache` |
| `0x0A` | `u16` | `flags` | 特性位标志：<br>• Bit 0: LittleEndian (固定为 1)<br>• Bit 1: TOC 经 Zstd 压缩<br>• Bit 2: 完全自包含 (Self-Contained)<br>• Bit 3: 保留 |
| `0x0C` | `u8` | `compression` | 默认分块压缩算法：`0 = None`, `1 = Zstd` |
| `0x0D` | `u8` | `checksum_type` | 校验和算法：仅 `0 = CRC32` |
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
    /// 分块类型标识 (如 b"META", b"MODL", b"TXTR", b"BIOM", b"ATLS")
    pub chunk_type: [u8; 4],
    /// 分块在文件中的绝对字节起始偏移 (必须 64 字节对齐)
    pub offset: u64,
    /// 压缩后占用字节长度 (若未压缩则等于 decompressed_size)
    pub compressed_size: u64,
    /// 原始解压后字节长度
    pub decompressed_size: u64,
    /// 专用分块压缩算法 (0 = 无压缩, 1 = Zstd)
    pub compression: u8,
    /// 预留字段对齐
    pub reserved: [u8; 7],
    /// 分块原始数据的 CRC32 校验码
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

### 5.2 `MODL` - 模型定义与几何块
* **存储内容**：
  * 使用 **`BakedModelDatabase`**：已预烘焙的旋转变换与多边形基元，加载时免去烘焙计算。

### 5.3 `TXTR` - 独立贴图与纹理块
* **存储原则**：
  * 存储 Standalone PBR 独立纹理条目与原始 PNG 字节流。
  * 相同纹理严格去重。
  * 可选携带伴生 PBR 贴图（`_n.png` 法线、`_s.png` 高光光泽度）。

### 5.4 `ATLS` - 图集与映射块
* 存储全量烘焙出的图集切片（`atlas_0.png`, `atlas_0_n.png`, `atlas_0_s.png`）。
* 存储 `AtlasAddressMap` 映射表，使宿主无需重新执行装箱计算，直接根据 UV 坐标寻址。

### 5.5 `BIOM` - 生物群系调色板映射块
* 存储原版及自定义生物群系色阶映射表与 Colormap 贴图（`grass.png`, `foliage.png`）。
* 用于重构网格时在 Python / DCC 端即时计算双线性插值顶点色与 Shader 调色。

---

## 6. 实施规范与约束

1. **宿主无关原则**：容器编解码器（Reader/Writer）必须是纯 Rust 实现，严禁依赖任何 DCC API。
2. **对齐与零拷贝**：所有 Chunk 数据起始偏移必须 64 字节对齐，以支持无损 Direct I/O 与 `mmap`。
3. **容错与前瞻兼容性**：读写器遇到未知非关键 Chunk 类型时，必须平滑跳过而不是 Panic 崩溃。

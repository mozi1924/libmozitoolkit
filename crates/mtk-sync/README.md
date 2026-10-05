# mtk-sync

[![Crate](https://img.shields.io/badge/crate-mtk--sync-blue.svg)](Cargo.toml)
[![Rust](https://img.shields.io/badge/Rust-1.80%2B-orange.svg)](https://www.rust-lang.org)
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](../../LICENSE)

**`mtk-sync`** 是 `libmozitoolkit` (`libmtk`) 套件中的实时网络协同与二进制增量流客户端 Crate。遵循 **无宿主依赖 (Host-Agnostic)** 原则，专为建立 Minecraft（伴随模组/插件如 Yefira）与 3D DCC 宿主（如 Blender MoziToolKit 插件、WebGPU 视口或独立渲染器）之间的高性能双向二进制实时通信通道而设计。

---

## 目录 (Table of Contents)

- [1. 架构定位与核心职责](#1-架构定位与核心职责)
- [2. 核心模块与系统拓扑](#2-核心模块与系统拓扑)
- [3. 二进制网络协议规范 (`protocol`)](#3-二进制网络协议规范-protocol)
  - [3.1 数据包头部与版本协商](#31-数据包头部与版本协商)
  - [3.2 强类型数据包定义 (`Packet`)](#32-强类型数据包定义-packet)
  - [3.3 编解码器 (`codec`)](#33-编解码器-codec)
- [4. 会话与生命周期管理 (`session` & `client`)](#4-会话与生命周期管理-session--client)
  - [4.1 实时会话控制器 (`LiveSyncSession`)](#41-实时会话控制器-livesyncsession)
  - [4.2 原生 WebSocket 传输客户端 (`SyncClient`)](#42-原生-websocket-传输客户端-syncclient)
  - [4.3 强类型宿主事件流 (`SyncEvent`)](#43-强类型宿主事件流-syncevent)
- [5. 核心协同机制与数据流](#5-核心协同机制与数据流)
  - [5.1 两阶段流式传输 (Two-Phase Progressive Streaming)](#51-两阶段流式传输-two-phase-progressive-streaming)
  - [5.2 单一世界大网格模式 (`unified_mesh`)](#52-单一世界大网格模式-unified_mesh)
  - [5.3 增量差量更新与局部重构 (`DeltaUpdate`)](#53-增量差量更新与局部重构-deltaupdate)
  - [5.4 CRC32 清单比对与自愈修复机制](#54-crc32-清单比对与自愈修复机制)
- [6. 快速上手示例 (Quick Start)](#6-快速上手示例-quick-start)
  - [示例 1：启动 Live Sync 会话并轮询事件流](#示例-1启动-live-sync-会话并轮询事件流)
  - [示例 2：手动解析二进制数据包与单包测试](#示例-2手动解析二进制数据包与单包测试)
  - [示例 3：编码并发送客户端同步控制指令](#示例-3编码并发送客户端同步控制指令)
- [7. Feature 开关与依赖](#7-feature-开关与依赖)
- [8. 开源协议 (License)](#8-开源协议-license)

---

## 1. 架构定位与核心职责

`mtk-sync` 将实时网络 I/O、二进制协议反序列化、体素内存镜像维护与多线程后台网格化完整封装为自闭环的异步事件流：

```
┌─────────────────────────────────┐
│     Minecraft (Yefira Mod)      │
│  Selection / Chunks / Deltas    │
└────────────────┬────────────────┘
                 │ WebSocket (Little-Endian Binary)
                 ▼
┌────────────────────────────────────────────────────────────────────────┐
│                               mtk-sync                                 │
│  ┌────────────────────────┐  ┌──────────────────────────────────────┐  │
│  │   SyncClient (守护线程)  │  │        decode_packet (协议编解码)    │  │
│  │ (tungstenite / 重连机制)│  │ (Selection/Snapshot/Delta/Manifest)  │  │
│  └───────────┬────────────┘  └──────────────────┬───────────────────┘  │
│              │ ClientMessage                    │ Packet               │
│              └─────────────────► ┌──────────────▼─────────────┐        │
│                                  │   LiveSyncSession / 调度器 │        │
│                                  │ (VoxelStorage / Rayon Mesher)       │
│                                  └──────────────┬─────────────┘        │
└─────────────────────────────────────────────────┼──────────────────────┘
                                                  │ SyncEvent (通道分发)
                                                  ▼
                                ┌───────────────────────────────────┐
                                │     DCC 宿主前端 (如 Blender)      │
                                │   poll_events() / 网格极速灌入     │
                                └───────────────────────────────────┘
```

- **二进制协议极速解析**：全量采用 Little-Endian（小端序）POD 紧凑内存布局，避免低效的 JSON/文本协议开销。
- **两阶段无缝缝合**：支持两阶段渐进式传输协议（`StreamBegin` $\to$ `SectionSnapshot` 内存入库 $\to$ `StreamEnd` 全局协同并行网格化），彻底消除跨区块面剔除错误与共面裂隙。
- **非阻塞事件轮询**：宿主仅需在每帧调用 `session.poll_events()` 提取构建完毕的 `MeshData` 或状态变动，不阻塞 UI 渲染主循环。

---

## 2. 核心模块与系统拓扑

| 模块路径 | 职责与功能 |
| :--- | :--- |
| **`protocol::constants`** | 协议固定常数（魔数 `[0x4D, 0x43]`、版本号 `0x02`）、数据包类型枚举 `PacketType` 与包头定长定义。 |
| **`protocol::packet`** | 强类型数据包模型 `Packet`、`DeltaChange`、`ManifestSectionEntry` 与流状态 `StreamStatus`。 |
| **`protocol::codec`** | 高性能小端序二进制编解码器：`decode_packet`、`encode_full_sync_request`、`encode_repair_requests`、`encode_sync_config`。 |
| **`protocol::error`** | 协议反序列化错误类型 `ProtocolError`（魔数错误、版本不兼容、负载截断等）。 |
| **`client`** | 基于 `tungstenite` 的原生 WebSocket 客户端后台守护线程 (`SyncClient`)，支持指数退避重连与心跳保活。 |
| **`events`** | 供宿主前端订阅消费的高层事件枚举 `SyncEvent`（`WorldMeshReady`、`SectionMeshReady`、`SelectionUpdated` 等）。 |
| **`session`** | 高阶实时会话控制器 (`LiveSyncSession`)，统一协调网络客户端、`VoxelStorage` 镜像、`model_db` 与多线程网格化。 |
| **`session::dispatcher`**| 后台数据包调度引擎 (`event_worker_loop` / `handle_packet`)，执行两阶段流控、增量差分应用与 CRC 自愈。 |

---

## 3. 二进制网络协议规范 (`protocol`)

### 3.1 数据包头部与版本协商

所有通信数据包均以 4 字节固定头部起始：

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|       Magic: 'M' (0x4D)       |       Magic: 'C' (0x43)       |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|     Version: 0x02 (Yefira)    |     PacketType (0x01..0x82)   |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

- **`PROTOCOL_MAGIC`**: `[0x4D, 0x43]` (ASCII `"MC"`)。
- **`PROTOCOL_VERSION`**: `0x02`（对齐 Yefira 2.0 规范）。
- **兼容性检查**: `is_supported_protocol_version(v)` 保证在 `0x01..=0x02` 范围内双向兼容。

---

### 3.2 强类型数据包定义 (`Packet`)

```rust
pub enum Packet {
    // === 服务端 -> 客户端 (S -> C) ===
    /// 0x01: 选区包围盒变动 (min_pos, size)
    SelectionInfo { min_pos: IVec3, size: IVec3 },

    /// 0x02: 全量选区快照 (Palette + 密集体素索引 + Biome 数据)
    FullSnapshot {
        min_pos: IVec3, size: IVec3,
        palette: Vec<String>, grid_indices: Vec<u16>,
        biome_palette: Option<Vec<String>>, biome_indices: Option<Vec<u16>>,
    },

    /// 0x03: 方块增量修改列表
    DeltaUpdate { seq_id: u32, min_pos: IVec3, changes: Vec<DeltaChange> },

    /// 0x05: 区块 CRC32 清单
    SectionManifest { seq_id: u32, sections: Vec<ManifestSectionEntry> },

    /// 0x06: 单个 16x16x16 区块快照
    SectionSnapshot {
        sec_coord: IVec3, start_pos: IVec3, size: IVec3,
        palette: Vec<String>, grid_indices: Vec<u16>,
        biome_palette: Option<Vec<String>>, biome_indices: Option<Vec<u16>>,
    },

    /// 0x07: 握手与场景元数据 (区块总量、体素总量、维度名等)
    HandshakeInfo { total_sections: u32, non_empty_sections: u32, total_volume: u32, dimension: String, flags: u16 },

    /// 0x08: 渐进流式传输开始
    StreamBegin { stream_id: u32, total_sections: u32, flags: u16 },

    /// 0x09: 渐进流式传输结束
    StreamEnd { stream_id: u32, sent_sections: u32, status: StreamStatus },

    // === 客户端 -> 服务端 (C -> S) ===
    /// 0x80: 客户端主动请求全量同步
    ReqFullSync,

    /// 0x81: 客户端请求指定区块修复列表
    ReqSectionSync { sections: Vec<IVec3> },

    /// 0x82: 客户端同步参数配置 (节流模式、目标 FPS、激活状态)
    SyncConfig { throttle_mode: u8, target_fps: u8, is_active: bool },
}
```

---

### 3.3 编解码器 (`codec`)

```rust
// 解码入站二进制字节帧
pub fn decode_packet(data: &[u8]) -> Result<Packet, ProtocolError>;

// 编码全量同步请求包 (0x80)
pub fn encode_full_sync_request() -> Vec<u8>;

// 编码分批区块修复请求 (0x81, 默认按 64 个区块自动切包)
pub fn encode_repair_requests(sections: &[IVec3], max_batch_size: usize) -> Vec<Vec<u8>>;

// 编码同步配置包 (0x82)
pub fn encode_sync_config(throttle_mode: u8, target_fps: u8, is_active: bool) -> Vec<u8>;
```

---

## 4. 会话与生命周期管理 (`session` & `client`)

### 4.1 实时会话控制器 (`LiveSyncSession`)

`LiveSyncSession` 是面向宿主的高层 API 门面：

```rust
pub struct LiveSyncSession {
    pub storage: Arc<RwLock<VoxelStorage>>,
    pub config: MesherConfig,
    pub culler: FaceCuller,
    pub model_db: Option<Arc<BakedModelDatabase>>, // 注入的 BlockState JSON 烘焙模型数据库
    pub unified_mesh: bool,                        // 是否合并为单一无缝大网格
    // ... 内部客户端与线程句柄
}
```

#### 关键方法
- `new(config, culler, model_db, unified_mesh) -> Self`：创建会话实例。
- `start(url, auto_reconnect, max_reconnect_attempts) -> Result<(), String>`：连接 Minecraft 服务端并启动后台 I/O 与网格化守护线程。
- `stop()`：优雅终止网络连接与后台 Worker 线程。
- `poll_events() -> Vec<SyncEvent>`：非阻塞提取当前已就绪的所有高阶事件。
- `get_world_mesh() -> MeshData`：主动并行网格化当前 `VoxelStorage` 内的所有区块并合并为焊接后的全局网格。
- `send_full_sync_request()` / `send_repair_request(sections)` / `send_sync_config(...)`：向服务端下发控制指令。

---

### 4.2 原生 WebSocket 传输客户端 (`SyncClient`)

基于 `tungstenite` 与 `crossbeam-channel` 实现，运行在独立 OS 线程中：
- 支持指定重连次数上限与指数退避重连；
- 设置底层 TCP Read Timeout（100ms），在接收网络数据的同时兼顾处理出站指令队列，杜绝线程死锁；
- 自动响应 WebSocket Ping/Pong 保活心跳帧。

---

### 4.3 强类型宿主事件流 (`SyncEvent`)

```rust
pub enum SyncEvent {
    StatusChange(String),                      // 连接状态变更 ("CONNECTED", "DISCONNECTED" 等)
    SelectionUpdated { min_pos: IVec3, size: IVec3 }, // 选区尺寸或位置发生改变
    Handshake { total_sections: u32, ... },    // 场景元数据就绪
    SectionMeshReady { coord: IVec3, mesh: MeshData }, // 单个切片网格就绪 (独立切片模式)
    WorldMeshReady { mesh: MeshData },         // 全局合并大网格就绪 (unified_mesh 模式)
    StreamProgress { stage: String, current: usize, total: usize, message: String }, // 批处理传输与网格构建多阶段物理进度
    StreamFinished { stream_id: u32, built_sections: usize },         // 流传输结束
    DeltaApplied { change_count: usize, affected_sections: Vec<IVec3> }, // 增量修改已应用
    Verified { is_verified: bool, message: String }, // 校验结果
    Warning(String),                           // 非致命告警
    Error(String),                             // 异常错误
}
```

---

## 5. 核心协同机制与数据流

### 5.1 两阶段流式传输 (Two-Phase Progressive Streaming)

当场景规模较大（如上千个区块切片）时，一次性发送巨石封包会导致严重网络停顿。`mtk-sync` 采用**两阶段流水线**：

1. **阶段 1：数据摄取 (Ingestion Phase)**
   - 服务端下发 `StreamBegin { total_sections, stream_id }`，标记进入流模式；
   - 连续下发若干 `SectionSnapshot` 数据包，调度器仅将其解析写入 `VoxelStorage`，**不触发单区块网格化**（避免由于相邻区块尚未到达而产生错误的跨区块边缘单面面剔除）；
   - 同时向前端分发 `StreamProgress` 汇报接收进度。
2. **阶段 2：协同构建 (Build Phase)**
   - 服务端下发 `StreamEnd`；
   - 此时所有相关体素切片已完整就绪在内存中，调度器调动 Rayon 多线程线程池对所有非空区块并行调用 `SectionMesher::mesh_sections_parallel` 进行完整 18×18×18 垫片邻域遮挡判定与空间顶点焊接；
   - 构建完成后一次性发射 `WorldMeshReady` 或批量 `SectionMeshReady`。

---

### 5.2 单一世界大网格模式 (`unified_mesh`)

当 `unified_mesh = true`（默认启用）时：
- 调度器维护一个全局区块网格缓存 `section_mesh_cache: HashMap<IVec3, MeshData>`；
- 当局部切片发生变更时，自动与缓存内其它切片网格进行极速内存拼接 (`MeshData::merge_all`) 并执行全局空间顶点焊接 (`weld_spatial_vertices`)；
- 派发 `WorldMeshReady` 事件，宿主端仅需更新单一 Object，**彻底消除相邻小 Chunk 拼接处次表面散射 (SSS) 撕裂与法线黑缝**。

---

### 5.3 增量差量更新与局部重构 (`DeltaUpdate`)

当玩家在 Minecraft 中破坏或放置方块时：
1. 服务端下发 `DeltaUpdate { min_pos, changes }`；
2. 调度器调用 `VoxelStorage::apply_delta_update` 精准修改内存中的对应体素；
3. 驱动 `mtk-voxel::DeltaMesher` 仅重构被修改方块所在的脏区块切片；
4. 更新本地 `section_mesh_cache` 并向宿主派发 `DeltaApplied` 与更新后的网格事件。

---

### 5.4 CRC32 清单比对与自愈修复机制

服务端周期性广播 `SectionManifest` 数据包携带各区块的权威 CRC32 校验码：
1. 调度器在本地调用 `VoxelStorage::validate_manifest` 进行哈希比对；
2. 若本地存储为空或差异区块数量过多（$> 64$ 个），客户端自动向服务端发送 `ReqFullSync` (0x80) 请求全量快照；
3. 若仅有零星几个区块数据不一致，客户端将损坏的区块坐标分批编码为 `ReqSectionSync` (0x81) 发起针对性差量修复。

---

## 6. 快速上手示例 (Quick Start)

### 示例 1：启动 Live Sync 会话并轮询事件流

```rust
use std::thread;
use std::time::Duration;
use mtk_sync::{LiveSyncSession, SyncEvent};
use mtk_voxel::types::{CoordinateSystem, MesherConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 配置网格化参数
    let config = MesherConfig {
        coordinate_system: CoordinateSystem::ZUpRightHanded, // 转换为 Blender / Unreal 坐标
        origin_centered: true,
        weld_vertices: true,
        ..Default::default()
    };

    // 2. 初始化 LiveSyncSession (启用单一世界大网格)
    let mut session = LiveSyncSession::new(Some(config), None, None, true);

    // 3. 连接 Minecraft Yefira 模组 (本地默认端口 25566)
    println!("正在连接 Live Sync 协同服务器...");
    session.start("ws://127.0.0.1:25566", true, 5)?;

    // 4. 事件轮询主循环 (通常置于宿主帧循环中)
    for _ in 0..100 {
        thread::sleep(Duration::from_millis(50));

        let events = session.poll_events();
        for evt in events {
            match evt {
                SyncEvent::StatusChange(status) => {
                    println!("[状态变更]: {}", status);
                }
                SyncEvent::SelectionUpdated { min_pos, size } => {
                    println!("[选区更新]: 最小坐标 {:?}, 尺寸 {:?}", min_pos, size);
                }
                SyncEvent::WorldMeshReady { mesh } => {
                    println!("[全量网格就绪]: 顶点数 = {}, 三角面数 = {}", mesh.positions.len(), mesh.indices.len() / 3);
                }
                SyncEvent::DeltaApplied { change_count, affected_sections } => {
                    println!("[增量变动]: {} 个方块修改, 影响 {} 个区块", change_count, affected_sections.len());
                }
                SyncEvent::StreamProgress { stage, current, total, message } => {
                    println!("[进度 - {} ({}/{})]: {}", stage, current, total, message);
                }
                SyncEvent::Verified { is_verified, message } => {
                    println!("[CRC 校验]: 验证结果 = {}, 信息 = {}", is_verified, message);
                }
                _ => {}
            }
        }
    }

    // 5. 退出会话
    session.stop();
    Ok(())
}
```

---

### 示例 2：手动解析二进制数据包与单包测试

```rust
use glam::IVec3;
use mtk_sync::protocol::{decode_packet, Packet};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 构造一个模拟 SelectionInfo 二进制小端序数据包
    // Header: Magic [0x4D, 0x43] + Version 0x02 + PacketType 0x01
    // Payload: 6 个 i32: min_x=0, min_y=64, min_z=0, size_x=16, size_y=16, size_z=16
    let mut raw_packet = vec![0x4D, 0x43, 0x02, 0x01];
    raw_packet.extend_from_slice(&0i32.to_le_bytes());  // min_x
    raw_packet.extend_from_slice(&64i32.to_le_bytes()); // min_y
    raw_packet.extend_from_slice(&0i32.to_le_bytes());  // min_z
    raw_packet.extend_from_slice(&16i32.to_le_bytes()); // size_x
    raw_packet.extend_from_slice(&16i32.to_le_bytes()); // size_y
    raw_packet.extend_from_slice(&16i32.to_le_bytes()); // size_z

    // 解码数据包
    match decode_packet(&raw_packet)? {
        Packet::SelectionInfo { min_pos, size } => {
            println!("解析成功: 选区位置 = {:?}, 尺寸 = {:?}", min_pos, size);
            assert_eq!(min_pos, IVec3::new(0, 64, 0));
            assert_eq!(size, IVec3::new(16, 16, 16));
        }
        _ => panic!("非预期的数据包类型"),
    }

    Ok(())
}
```

---

### 示例 3：编码并发送客户端同步控制指令

```rust
use glam::IVec3;
use mtk_sync::protocol::{encode_full_sync_request, encode_repair_requests, encode_sync_config};

fn main() {
    // 1. 编码全量同步请求包 (0x80)
    let full_sync_bin = encode_full_sync_request();
    assert_eq!(full_sync_bin, vec![0x4D, 0x43, 0x02, 0x80]);

    // 2. 编码损坏区块修复请求 (0x81, 自动分包)
    let broken_sections = vec![
        IVec3::new(0, 4, 0),
        IVec3::new(1, 4, 0),
    ];
    let repair_packets = encode_repair_requests(&broken_sections, 64);
    println!("生成了 {} 个修复请求分包", repair_packets.len());

    // 3. 编码同步配置指令 (0x82, 开启 60 FPS 节流)
    let config_bin = encode_sync_config(0, 60, true);
    assert_eq!(config_bin.len(), 7); // Header(4) + throttle(1) + fps(1) + active(1)
}
```

---

## 7. Feature 开关与依赖

`mtk-sync` 在 `Cargo.toml` 中提供特性开关：

```toml
[features]
default = ["serde"]
serde = ["dep:serde", "dep:serde_json"]
```

- **`serde`** (默认开启)：为 `Packet`、`SyncEvent`、`DeltaChange`、`ManifestSectionEntry` 等结构体提供序列化与反序列化支持。

---

## 8. 开源协议 (License)

本项目遵循 [GNU General Public License v3.0 or later (GPL-3.0-or-later)](../../LICENSE) 开源协议。


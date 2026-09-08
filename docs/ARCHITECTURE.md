# libmozitoolkit 系统架构设计 (Architecture)

## 1. 架构定位与愿景

`libmozitoolkit` (`libmtk`) 是一个**高性能、无宿主依赖 (Host-Agnostic)、纯数据驱动的通用 3D 与体素 (Voxel) 几何处理内核套件**。

### 设计原则
1. **纯数据流输入输出 (Data-in, Data-out)**：输入原始体素/网格/UV/流数据/PBR贴图，输出扁平化的连续内存网格流（Positions, Normals, UVs, Indices, Material Slots）与图集内存。
2. **原子化与高内聚 (Atomic & Modular)**：每个子 crate 职责单一、边界清晰，可独立引用、独立编译。
3. **多端目标与原生跨语言支持**：原生兼容 WebAssembly (WASM)、Python CPython 扩展模块 (Wheel)、以及通用 C-ABI 动态/静态库 (`.so` / `.dll` / `.dylib` / `.h`)。
4. **零宿主耦合**：核心层不依赖任何 DCC 软件（如 Blender、Maya）专有 API，只在宿主端做数据灌入。

---

## 2. 4 层分层架构拓扑

```mermaid
graph TD
    subgraph Layer3_Bindings["Layer 3: 胶水与跨语言绑定 (Bindings & FFI)"]
        WASM["mtk-wasm (wasm-bindgen / npm)"]
        PY["mtk-py (PyO3 / maturin / Wheel)"]
        FFI["mtk-ffi (C-ABI / cdylib / staticlib / .h)"]
    end

    subgraph Layer2_Facade["Layer 2: 领域聚合门面 (Unified Engine)"]
        LIBMTK["libmtk (Pipeline / High-level Sessions)"]
    end

    subgraph Layer1_Domain["Layer 1: 领域子系统 (Domain Engines)"]
        VOXEL["mtk-voxel (Mesher / AO / Biome)"]
        MODEL["mtk-model (BlockState / Baker / OBJ)"]
        TEXTURE["mtk-texture (Atlas / PBR Packing / UV)"]
        RESOURCE["mtk-resource (VFS / Pack / CTM)"]
        CULL["mtk-cull (Occlusion / Rect Difference)"]
        MAT["mtk-material (材质映射/替换 [规划中])"]
        NET["mtk-net (实时数据流/网络协议 [规划中])"]
        MESH["mtk-meshopt (网格重构/减面/优化 [规划中])"]
    end

    subgraph Layer0_Core["Layer 0: 核心数据与几何基础设施 (Foundation)"]
        CORE["mtk-core (POD 网格/顶点布局/通用几何/数学抽象)"]
    end

    WASM --> LIBMTK
    PY --> LIBMTK
    FFI --> LIBMTK

    LIBMTK --> VOXEL
    LIBMTK --> MODEL
    LIBMTK --> TEXTURE
    LIBMTK --> RESOURCE
    LIBMTK --> CULL
    LIBMTK --> MAT
    LIBMTK --> NET
    LIBMTK --> MESH

    VOXEL --> CORE
    VOXEL --> CULL
    MODEL --> CORE
    TEXTURE --> CORE
    RESOURCE --> CORE
    CULL --> CORE
    MAT --> CORE
    NET --> CORE
    MESH --> CORE
```

---

## 3. 各 Crate 职责矩阵

| Crate 路径 | 核心定位 | 依赖上游 | 输出与产物 |
| :--- | :--- | :--- | :--- |
| **`crates/mtk-core`** | 纯净数据底座、POD 顶点/面数据结构、通用数学与几何基元 | 仅依赖 `glam`, `serde` (可选) | `MeshData`, `Quad`, `Aabb2d/3d`, `Direction`, `DirMask` |
| **`crates/mtk-cull`** | 面剔除状态机、2D/3D 矩形差集切分 (Subtract Rect) | `mtk-core` | `BlockCullMeta`, `FaceCuller`, 剔除裁剪后微矩形列表 |
| **`crates/mtk-model`** | BlockState 状态解析、1.21+ Block Model JSON 烘焙、Wavefront OBJ 解析 | `mtk-core` | `BlockState`, `BlockModelJson`, `BakedModel`, `MeshData` |
| **`crates/mtk-texture`**| 空间分割图集拼接器 (Stitcher)、PBR 通道合并、UV 空间重映射 | `mtk-core` | `AtlasBuilder`, `BakedAtlas`, `RgbaBuffer`, UV 坐标映射表 |
| **`crates/mtk-resource`**| 虚拟文件系统 (VFS / Zip / Memory / Dir)、CTM 47/17 连接纹理求解、动画元数据 | `mtk-core` | `ResourcePackStack`, `CtmSolver`, `AnimationMetadata` |
| **`crates/mtk-voxel`**  | 16x16x16 Chunk Section 体素存储、平滑 AO 计算、生物群系过渡、网格化器 (Mesher) | `mtk-core`, `mtk-cull` | `SectionStorage`, `SectionMesher`, `DeltaMesher`, `WorldMeshBuildResult` |
| **`crates/mtk-material`** *(规划中)* | 材质替换匹配引擎 (jmc2obj / Mineways / Block ID / Texture Name 到 PBR 映射) | `mtk-core` | `MaterialRuleMatcher`, `PbrMaterialSpec` |
| **`crates/mtk-net`** *(规划中)* | 实时流式网络协议解析 (WebSocket/TCP/二进制帧) 与世界增量同步 | `mtk-core` | `NetworkPacketDecoder`, `WorldDeltaEvent` |
| **`crates/mtk-meshopt`** *(规划中)* | 网格减面、LOD 生成、共面合并、拓扑整理与 MikkTSpace 法线重算 | `mtk-core` | 优化后的 `MeshData`, 多级 LOD 网格 |
| **`crates/libmtk`** | 顶层统一 Facade 库，提供开箱即用的高阶 Pipeline 与一站式统一错误处理 `MtkError` | 全部 Layer 1 Crates | 高阶 API、统一 Error 与 Pipeline |
| **`bindings/mtk-py`** | Python 动态扩展模块 (PyO3 + maturin)，提供 `PyMeshData` 与扁平数组传输 | `libmtk`, `mtk-core` | `libmtk_py` CPython 轮子 (.whl) |
| **`bindings/mtk-ffi`** | 纯 C-ABI 动态/静态库与 C 头文件 (cbindgen)，跨语言无缝调用 | `libmtk`, `mtk-core` | `libmtk_ffi.so` / `.dll` / `.dylib`, `mtk.h` |
| **`bindings/mtk-wasm`** | WebAssembly 绑定 (wasm-bindgen)，暴露 TypedArray 视图 | `libmtk`, `mtk-core` | `mtk_wasm.wasm` + `mtk_wasm.js` npm 包 |

---

## 4. 跨端数据交换标准 (Host-Agnostic Protocol)

为了保证高性能与零拷贝，Rust 导出层与宿主层（Python/JS/C#）约定以下**扁平连续内存结构 (Flat Contiguous Buffers)**：

```
Rust 处理内核 (libmtk)
  │
  ├─ 顶点位置流:  [x0, y0, z0, x1, y1, z1, ...]           -> f32 连续数组
  ├─ 顶点法线流:  [nx0, ny0, nz0, nx1, ny1, nz1, ...]     -> f32 连续数组
  ├─ 纹理 UV 流:  [u0, v0, u1, v1, ...]                   -> f32 连续数组
  ├─ 顶点颜色流:  [r0, g0, b0, a0, ...] (可选)            -> f32 连续数组
  ├─ 三角形索引:  [i0, i1, i2, i3, i4, i5, ...]           -> u32 连续数组
  ├─ 材质插槽映射: [mat_id_0, mat_id_1, ...]               -> u16 / u32 数组
  └─ 贴图数据流:  [r, g, b, a, r, g, b, a, ...]           -> u8 连续像素内存
```

宿主端通过指针与长度（或 Buffer Protocol / TypedArray）直接读取，并在宿主内部调用专有场景 API 批量构建，杜绝逐元素低效循环。

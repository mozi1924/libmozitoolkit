# mtk-ffi

[![Crate](https://img.shields.io/badge/crate-mtk--ffi-blue.svg)](Cargo.toml)
[![Rust](https://img.shields.io/badge/Rust-1.80%2B-orange.svg)](https://www.rust-lang.org)
[![C-ABI](https://img.shields.io/badge/C--ABI-C99%20Compatible-blue.svg)](https://en.wikipedia.org/wiki/C99)
[![Status](https://img.shields.io/badge/Roadmap%20Priority-P2%20Future%20Prep-blueviolet.svg)]()
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](../../LICENSE)

**`mtk-ffi`** 提供纯 C 兼容二进制应用编程接口（Pure C-ABI），产出标准动态链接库（`.so` / `.dll` / `.dylib`）、静态库（`.a` / `.lib`）与 C 语言头文件（`mtk.h`）。

---

## 目录 (Table of Contents)

- [1. 架构定位与演进优先级](#1-架构定位与演进优先级)
- [2. 核心数据类型与 C-ABI 函数](#2-核心数据类型与-c-abi-函数)
  - [2.1 数据结构](#21-数据结构)
  - [2.2 C-ABI 函数契约](#22-c-abi-函数契约)
- [3. 构建、编译与头文件生成](#3-构建编译与头文件生成)
- [4. 真实调用示例](#4-真实调用示例)
  - [4.1 C/C++ 宿主调用示例](#41-cc-宿主调用示例)
  - [4.2 C# (Unity / .NET P/Invoke) 调用示例](#42-c-unity--net-pinvoke-调用示例)
- [5. 开源协议 (License)](#5-开源协议-license)

---

## 1. 架构定位与演进优先级

- **演进优先级**：`P2`（**远期预备与多引擎/DCC 跨语言生态扩展**）。
- **核心定位**：为非 Python 环境（如 C/C++ 原生宿主、C# / Unity P/Invoke、Godot GDExtension、Go cgo 以及 Maya / Houdini 原生插件）提供零开销、无垃圾回收干扰的底层 C-ABI 接口。
- **内存模型**：
  - 不透明堆句柄 (`Opaque Pointer`) 隔离 Rust 内部数据结构；
  - 导出只读连续内存视图结构体 (`MtkMeshView`)，直接提供顶点、法线、UV、索引与材质槽位的原生指针，支持极速内存复制或 GPU 缓冲直传。

---

## 2. 核心数据类型与 C-ABI 函数

### 2.1 数据结构

#### `MtkMeshView`
只读连续网格缓冲指针视图（POD 结构）：
```c
typedef struct MtkMeshView {
    const float *positions;       /* 扁平顶点坐标 [x0, y0, z0, x1, y1, z1, ...] */
    size_t vertex_count;          /* 顶点数量 */
    const float *normals;         /* 扁平顶点法线 [nx0, ny0, nz0, ...] */
    const float *uvs;             /* 扁平顶点 UV [u0, v0, u1, v1, ...] */
    const uint32_t *indices;      /* 三角形索引数组 */
    size_t index_count;           /* 索引数量 (每 3 个构成一个三角形) */
    const uint16_t *face_materials;/* 面材质槽位 ID 数组 */
    size_t face_count;            /* 面数量 */
} MtkMeshView;
```

#### `MtkMeshBuffer`
不透明堆容器（Opaque Pointer），持有 Rust 原生 `MeshData`。

---

### 2.2 核心 C 函数清单

| 函数签名 | 说明 |
| :--- | :--- |
| `MtkMeshBuffer* mtk_mesh_buffer_new(void)` | 创建并返回一个新的网格缓冲区堆句柄 |
| `void mtk_mesh_buffer_free(MtkMeshBuffer *buffer)` | 释放网格缓冲区堆内存 |
| `void mtk_mesh_buffer_clear(MtkMeshBuffer *buffer)` | 清空网格缓冲区中的几何数据 |
| `int32_t mtk_mesh_buffer_append_unit_cube_face(MtkMeshBuffer *buffer, uint8_t direction_id, uint16_t material_slot)` | 追加指定朝向的原版单位立方体面（0:Down, 1:Up, 2:North, 3:South, 4:West, 5:East），成功返回 0，失败返回 -1 |
| `MtkMeshView mtk_mesh_buffer_get_view(MtkMeshBuffer *buffer)` | 获取当前网格缓冲区的只读指针视图 |
| `const char* mtk_version(void)` | 返回以 null 结尾的静态版本号字符串 |

---

## 3. 构建、编译与头文件生成

### 3.1 编译动态与静态链接库
```bash
# 编译 Debug 目标
cargo build -p mtk-ffi

# 编译 Release 目标 (产物位于 target/release/libmtk_ffi.so / .a)
cargo build --release -p mtk-ffi
```

### 3.2 生成 C 头文件 (`mtk.h`)
使用 `cbindgen` 根据 `cbindgen.toml` 自动生成 C-ABI 头文件：
```bash
cargo install cbindgen
cbindgen --config bindings/mtk-ffi/cbindgen.toml --crate mtk-ffi --output bindings/mtk-ffi/include/mtk.h
```

### 3.3 运行单元测试
```bash
cargo test -p mtk-ffi
```

---

## 4. 真实调用示例

### 4.1 C / C++ 调用示例 (`main.c`)
```c
#include <stdio.h>
#include <stdint.h>
#include "mtk.h"

int main() {
    printf("libmtk FFI Version: %s\n", mtk_version());

    // 1. 创建网格缓冲区
    MtkMeshBuffer *buffer = mtk_mesh_buffer_new();
    if (!buffer) {
        fprintf(stderr, "Failed to allocate MtkMeshBuffer\n");
        return 1;
    }

    // 2. 追加方块表面 (Direction 1: Up, Material Slot: 42)
    mtk_mesh_buffer_append_unit_cube_face(buffer, 1, 42);

    // 3. 获取底层连续内存指针视图
    MtkMeshView view = mtk_mesh_buffer_get_view(buffer);
    printf("Vertex Count: %zu, Index Count: %zu, Face Count: %zu\n",
           view.vertex_count, view.index_count, view.face_count);

    if (view.vertex_count > 0 && view.positions) {
        printf("First Vertex: (%f, %f, %f)\n",
               view.positions[0], view.positions[1], view.positions[2]);
    }

    // 4. 清理并释放堆内存
    mtk_mesh_buffer_free(buffer);
    return 0;
}
```

### 4.2 C# (Unity / .NET P/Invoke) 调用示例
```csharp
using System;
using System.Runtime.InteropServices;

public static class MtkNative
{
    private const string LibName = "mtk_ffi";

    [StructLayout(LayoutKind.Sequential)]
    public struct MtkMeshView
    {
        public IntPtr Positions;
        public UIntPtr VertexCount;
        public IntPtr Normals;
        public IntPtr Uvs;
        public IntPtr Indices;
        public UIntPtr IndexCount;
        public IntPtr FaceMaterials;
        public UIntPtr FaceCount;
    }

    [DllImport(LibName, EntryPoint = "mtk_version")]
    public static extern IntPtr GetVersion();

    [DllImport(LibName, EntryPoint = "mtk_mesh_buffer_new")]
    public static extern IntPtr MeshBufferNew();

    [DllImport(LibName, EntryPoint = "mtk_mesh_buffer_free")]
    public static extern void MeshBufferFree(IntPtr buffer);

    [DllImport(LibName, EntryPoint = "mtk_mesh_buffer_append_unit_cube_face")]
    public static extern int AppendUnitCubeFace(IntPtr buffer, byte directionId, ushort materialSlot);

    [DllImport(LibName, EntryPoint = "mtk_mesh_buffer_get_view")]
    public static extern MtkMeshView GetView(IntPtr buffer);
}
```

---

## 5. 开源协议 (License)

本项目遵循 [GNU General Public License v3.0 or later (GPL-3.0-or-later)](../../LICENSE) 开源协议。

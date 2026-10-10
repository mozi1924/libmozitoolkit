# `mtk-blender` (Blender Hardware-Accelerated Direct Memory Bindings)

专为 Blender 宿主量身定制的高性能硬件级直接内存直写与 DNA 属性灌入扩展库 (`libmtk_blender`)。

---

## 1. 架构定位与隔离职责

- **宿主专属性**：专供 MoziToolKit Blender 插件内部调用，不对通用 Python 平台 (PyPI) 强制分发；
- **非安全代码物理隔离**：收敛所有高危裸指针 (`usize` / `*mut u8`) 内存拷贝，彻底与通用纯净安全库 `mtk-py` 物理隔离；
- **边界防御守卫 (Bounds Guard)**：在执行直接内存拷贝前严格验证目标指针非空与最大允许写入字节数 (`max_bytes`)，杜绝因 Blender 内部结构体变动或指针计算错误引发的 `SIGSEGV` 崩溃；
- **优雅降级伙伴**：作为性能最高的第一阶通道 (Tier 1)，所有接口均与 `mtk-py` 的通用 Python Buffer Protocol (Tier 2: `memoryview`) 严格对齐。若宿主环境不可用，无缝降级至通用通道。

---

## 2. 导出 API

### 网格拓扑与属性直写 (`libmtk_blender`)

所有直写函数均遵循安全边界校验：
```python
import libmtk_blender

# 1. 顶点位置直接灌入 Blender CustomData position 缓冲
copied_bytes = libmtk_blender.copy_positions_to_ptr(pos_memoryview, dst_ptr, max_bytes=v_count * 12)

# 2. 顶点法线直接灌入
copied_bytes = libmtk_blender.copy_normals_to_ptr(norm_memoryview, dst_ptr, max_bytes=v_count * 12)

# 3. 循环 UV (Loop UV) 直接灌入
copied_bytes = libmtk_blender.copy_loop_uvs_to_ptr(uv_memoryview, dst_ptr, max_bytes=num_loops * 8)

# 4. 面材质索引直接灌入
copied_bytes = libmtk_blender.copy_face_materials_to_ptr(mat_memoryview, dst_ptr, max_bytes=num_polys * 4)

# 5. 顶点/角点颜色直接灌入
copied_bytes = libmtk_blender.copy_colors_to_ptr(color_memoryview, dst_ptr, max_bytes=num_colors * 16)

# 6. 通用连续缓冲区安全直写
copied_bytes = libmtk_blender.copy_buffer_to_ptr(src_buffer, dst_ptr, max_bytes=buffer_size)
```

---

## 3. 编译构建

作为独立 Python C-Extension 编译：
```bash
cargo build --release -p mtk-blender --features extension-module
```
产出 `target/release/liblibmtk_blender.so` (`.dylib` / `.pyd`)。

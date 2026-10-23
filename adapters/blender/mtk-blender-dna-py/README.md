# libmtk_blender_dna

Standalone ABI3 wheel exposing generic raw-memory copies for Blender DNA buffers.

```python
import libmtk_blender_dna as dna

dna.api_version()  # 1
dna.copy_f32(src_view, dst_ptr, count, max_bytes=...)
dna.copy_i32(...)
dna.copy_u32(...)
dna.copy_u8(...)
```

It has no dependency on `libmtk_py`, `mtk-core`, or `mtk-voxel`.

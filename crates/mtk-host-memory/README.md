# mtk-host-memory

Host-agnostic raw slice → destination pointer copy primitives.

- No `mtk-core` / `mtk-voxel` / host SDK dependency.
- Typed `f32` / `i32` / `u32` / `u8` copies with null/alignment/capacity guards.
- Host adapters obtain source slices via Python's Buffer Protocol and pass a host-owned destination address.

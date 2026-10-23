# mtk-blender-dna

Blender-oriented semantic wrappers over `mtk-host-memory`.

The adapter knows only raw slices and destination pointers. It has no
`mtk-core` / `mtk-voxel` dependency; the Python bridge extracts typed
`memoryview`s from the core wheel and resolves Blender `CustomData` pointers.

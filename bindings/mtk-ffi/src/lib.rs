//! # `mtk-ffi` (C-ABI Dynamic and Static Library Interface)
//!
//! Provides a pure C-compatible ABI and struct definitions for calling `libmtk`
//! from C, C++, C#, Go, Godot (GDExtension), and Unity (P/Invoke).

use std::ffi::c_char;

use mtk_core::mesh::MeshData;
use mtk_core::direction::Direction;
use mtk_core::geometry::Quad;
use mtk_core::attributes::FaceAttributes;

/// Direct read-only pointer view of contiguous mesh buffer data.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct MtkMeshView {
    /// Pointer to flat vertex positions `[x0, y0, z0, x1, y1, z1, ...]`.
    pub positions: *const f32,
    /// Number of vertices (each vertex has 3 floats).
    pub vertex_count: usize,

    /// Pointer to flat vertex normals `[nx0, ny0, nz0, nx1, ny1, nz1, ...]`.
    pub normals: *const f32,

    /// Pointer to flat vertex UVs `[u0, v0, u1, v1, ...]`.
    pub uvs: *const f32,

    /// Pointer to triangle indices.
    pub indices: *const u32,
    /// Number of indices (each triangle has 3 indices).
    pub index_count: usize,

    /// Pointer to face material IDs.
    pub face_materials: *const u16,
    /// Number of face material records.
    pub face_count: usize,
}

/// Opaque heap container holding `MeshData`.
pub struct MtkMeshBuffer {
    inner: MeshData,
}

/// Creates a new empty `MtkMeshBuffer`.
#[no_mangle]
pub extern "C" fn mtk_mesh_buffer_new() -> *mut MtkMeshBuffer {
    Box::into_raw(Box::new(MtkMeshBuffer {
        inner: MeshData::new(),
    }))
}

/// Destroys and frees an `MtkMeshBuffer`.
#[no_mangle]
pub unsafe extern "C" fn mtk_mesh_buffer_free(buffer: *mut MtkMeshBuffer) {
    if !buffer.is_null() {
        drop(Box::from_raw(buffer));
    }
}

/// Clears geometry from an existing mesh buffer.
#[no_mangle]
pub unsafe extern "C" fn mtk_mesh_buffer_clear(buffer: *mut MtkMeshBuffer) {
    if let Some(buf) = buffer.as_mut() {
        buf.inner.clear();
    }
}

/// Appends a unit cube face to the buffer (0: Down, 1: Up, 2: North, 3: South, 4: West, 5: East).
#[no_mangle]
pub unsafe extern "C" fn mtk_mesh_buffer_append_unit_cube_face(
    buffer: *mut MtkMeshBuffer,
    direction_id: u8,
    material_slot: u16,
) -> i32 {
    if let Some(buf) = buffer.as_mut() {
        let dir = match direction_id {
            0 => Direction::Down,
            1 => Direction::Up,
            2 => Direction::North,
            3 => Direction::South,
            4 => Direction::West,
            5 => Direction::East,
            _ => return -1,
        };
        let quad = Quad::unit_cube_face(dir);
        let attr = FaceAttributes {
            material_slot,
            ..Default::default()
        };
        buf.inner.append_quad(&quad, &attr);
        0
    } else {
        -1
    }
}

/// Obtains a read-only C view into the continuous buffers.
///
/// # Safety
/// The returned pointers are valid only as long as `buffer` is not modified or freed.
#[no_mangle]
pub unsafe extern "C" fn mtk_mesh_buffer_get_view(buffer: *mut MtkMeshBuffer) -> MtkMeshView {
    if let Some(buf) = buffer.as_mut() {
        let v_count = buf.inner.vertex_count();

        MtkMeshView {
            positions: buf.inner.positions_flat().as_ptr(),
            vertex_count: v_count,
            normals: buf.inner.normals_flat().as_ptr(),
            uvs: buf.inner.uvs_flat().as_ptr(),
            indices: buf.inner.indices.as_ptr(),
            index_count: buf.inner.indices.len(),
            face_materials: buf.inner.face_materials.as_ptr(),
            face_count: buf.inner.face_materials.len(),
        }
    } else {
        MtkMeshView {
            positions: std::ptr::null(),
            vertex_count: 0,
            normals: std::ptr::null(),
            uvs: std::ptr::null(),
            indices: std::ptr::null(),
            index_count: 0,
            face_materials: std::ptr::null(),
            face_count: 0,
        }
    }
}

/// Returns the library version as a static null-terminated C string.
#[no_mangle]
pub extern "C" fn mtk_version() -> *const c_char {
    static VERSION: &[u8] = concat!(env!("CARGO_PKG_VERSION"), "\0").as_bytes();
    VERSION.as_ptr() as *const c_char
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CStr;

    #[test]
    fn test_ffi_version() {
        let ptr = mtk_version();
        assert!(!ptr.is_null());
        let c_str = unsafe { CStr::from_ptr(ptr) };
        assert_eq!(c_str.to_str().unwrap(), env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn test_ffi_mesh_buffer_lifecycle() {
        unsafe {
            let buf = mtk_mesh_buffer_new();
            assert!(!buf.is_null());

            let res = mtk_mesh_buffer_append_unit_cube_face(buf, 1, 42);
            assert_eq!(res, 0);

            let view = mtk_mesh_buffer_get_view(buf);
            assert_eq!(view.vertex_count, 4);
            assert_eq!(view.index_count, 6);
            assert_eq!(view.face_count, 1);
            assert!(!view.positions.is_null());
            assert!(!view.indices.is_null());
            assert_eq!(*view.face_materials, 42);

            mtk_mesh_buffer_clear(buf);
            let empty_view = mtk_mesh_buffer_get_view(buf);
            assert_eq!(empty_view.vertex_count, 0);
            assert_eq!(empty_view.index_count, 0);

            mtk_mesh_buffer_free(buf);
        }
    }
}


//! # `mtk-ffi` (C-ABI Dynamic and Static Library Interface)
//!
//! Provides a pure C-compatible ABI and struct definitions for calling `libmtk`
//! from C, C++, C#, Go, Godot (GDExtension), and Unity (P/Invoke).

use std::ffi::c_char;

use mtk_core::attributes::FaceAttributes;
use mtk_core::direction::Direction;
use mtk_core::geometry::Quad;
use mtk_core::mesh::MeshData;

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
///
/// # Safety
/// `buffer` must be null or a pointer previously returned by [`mtk_mesh_buffer_new`]
/// that has not already been freed.
#[no_mangle]
pub unsafe extern "C" fn mtk_mesh_buffer_free(buffer: *mut MtkMeshBuffer) {
    if !buffer.is_null() {
        drop(Box::from_raw(buffer));
    }
}

/// Clears geometry from an existing mesh buffer.
///
/// # Safety
/// `buffer` must be null or a valid pointer returned by [`mtk_mesh_buffer_new`].
#[no_mangle]
pub unsafe extern "C" fn mtk_mesh_buffer_clear(buffer: *mut MtkMeshBuffer) {
    if let Some(buf) = buffer.as_mut() {
        buf.inner.clear();
    }
}

/// Appends a unit cube face to the buffer (0: Down, 1: Up, 2: North, 3: South, 4: West, 5: East).
///
/// # Safety
/// `buffer` must be null or a valid pointer returned by [`mtk_mesh_buffer_new`].
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
    fn test_ffi_material_routes_and_sync_state() {
        use std::ffi::CString;

        let atlas = CString::new(
            r#"{"chunks":[],"sprites":{"minecraft:block/stone":{"chunk_id":0,"texture_id":7,"uv_bounds":[0,0,1,1],"frame_0_uv_bounds":[0,0,1,1],"frame_count":1,"has_normal":false,"has_specular":false}}}"#,
        )
        .unwrap();
        let keys = CString::new(r#"["minecraft:block/stone"]"#).unwrap();
        let mut route = MtkMaterialRoute::default();
        let count =
            unsafe { mtk_resolve_material_routes(atlas.as_ptr(), keys.as_ptr(), 0, &mut route, 1) };
        assert_eq!(count, 1);
        assert_eq!(route.chunk_id, 0);
        assert_eq!(route.texture_id, 7);
        assert_eq!(route.mapped, 1);
        assert_eq!(route.is_animated, 0);

        let state = CString::new(
            r#"{"server_uuid":"00112233445566778899aabbccddeeff","epoch":3,"sequence":9,"min":[0,0,0],"size":[16,16,16],"dimension":"minecraft:overworld","state_hash":123,"user_modified":false}"#,
        )
        .unwrap();
        assert_eq!(
            unsafe { mtk_sync_state_is_reconnect_candidate(state.as_ptr()) },
            1
        );

        let base = [1u8];
        let packed = [0.0f32, 0.0, 1.0, 1.0];
        let grass = [0.2f32, 0.8, 0.2, 1.0];
        let foliage = [0.1f32, 0.6, 0.1, 1.0];
        let dry = [0.5f32, 0.4, 0.1, 1.0];
        let water = [0.1f32, 0.2, 0.9, 1.0];
        let uv = [0.2f32, 0.32];
        let mut out_packed = [0.0f32; 4];
        let mut out_colors = [0.0f32; 4];
        let mut out_uvs = [0.0f32; 3];
        let rc = unsafe {
            mtk_biome_reapply_uniform_tint(
                base.as_ptr(),
                packed.as_ptr(),
                std::ptr::null(),
                1,
                grass.as_ptr(),
                foliage.as_ptr(),
                dry.as_ptr(),
                water.as_ptr(),
                uv.as_ptr(),
                0,
                0,
                0,
                out_packed.as_mut_ptr(),
                out_colors.as_mut_ptr(),
                out_uvs.as_mut_ptr(),
            )
        };
        assert_eq!(rc, 0);
        assert_eq!(out_colors, grass);
        assert_eq!(out_packed[3], 1.0);
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

// ---------------------------------------------------------------------------
// Material routing / biome / persisted sync state (host-agnostic C ABI)
// ---------------------------------------------------------------------------

/// Resolved atlas routing for one face.
#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct MtkMaterialRoute {
    pub chunk_id: u16,
    pub texture_id: u32,
    /// 1 when the face targets a dedicated animated atlas chunk.
    pub is_animated: u8,
    /// `[total_frames, frametime, interpolate]`.
    pub anim_timing: [f32; 3],
    /// `[frame_width, frame_height, 0]`.
    pub anim_frame_size: [f32; 3],
    /// 1 when the material name was resolved to an atlas sprite.
    pub mapped: u8,
}

unsafe fn cstr<'a>(ptr: *const c_char) -> Option<&'a str> {
    if ptr.is_null() {
        return None;
    }
    std::ffi::CStr::from_ptr(ptr).to_str().ok()
}

/// Resolve per-face atlas routes from an atlas-mapping JSON and a JSON array of texture keys.
///
/// Returns the number of routes written, or a negative value on parse/argument error.
/// The caller supplies `out_routes` with capacity `capacity`.
///
/// # Safety
/// `atlas_json` and `keys_json` must be valid null-terminated UTF-8 strings.
/// `out_routes` must be writable for at least `capacity` elements.
#[no_mangle]
pub unsafe extern "C" fn mtk_resolve_material_routes(
    atlas_json: *const c_char,
    keys_json: *const c_char,
    enable_animated_textures: u8,
    out_routes: *mut MtkMaterialRoute,
    capacity: usize,
) -> i32 {
    let Some(atlas_json) = cstr(atlas_json) else {
        return -1;
    };
    let Some(keys_json) = cstr(keys_json) else {
        return -2;
    };
    let Ok(address_map) = mtk_texture::AtlasAddressMap::from_json(atlas_json) else {
        return -3;
    };
    let Ok(keys) = serde_json::from_str::<Vec<String>>(keys_json) else {
        return -4;
    };
    let routes = mtk_material::resolve_mesh_material_routes(
        &keys,
        None,
        &address_map,
        enable_animated_textures != 0,
    );
    if out_routes.is_null() || capacity < routes.len() {
        return -5;
    }
    for (idx, route) in routes.iter().enumerate() {
        *out_routes.add(idx) = MtkMaterialRoute {
            chunk_id: route.chunk_id,
            texture_id: route.texture_id,
            is_animated: route.is_animated as u8,
            anim_timing: route.anim_timing,
            anim_frame_size: route.anim_frame_size,
            mapped: route.mapped as u8,
        };
    }
    routes.len() as i32
}

/// Parse persisted Live Sync state JSON and return 1 if it is a valid reconnect candidate.
///
/// Returns 0 for valid-but-non-reconnectable state and a negative value for invalid JSON.
///
/// # Safety
/// `state_json` must be a valid null-terminated UTF-8 string.
#[no_mangle]
pub unsafe extern "C" fn mtk_sync_state_is_reconnect_candidate(state_json: *const c_char) -> i32 {
    let Some(json) = cstr(state_json) else {
        return -1;
    };
    match mtk_sync::PersistedSessionState::from_json(json) {
        Ok(state) => state.is_reconnect_candidate() as i32,
        Err(_) => -2,
    }
}

/// Re-apply a uniform biome palette to flat face buffers.
///
/// `base_tint_types` has `face_count` u8 entries; `packed_tint_data` and
/// `existing_tint_colors` have `face_count * 4` f32 entries. `existing_tint_colors`
/// may be null. Output pointers must have the same sizes as the corresponding inputs.
/// Returns 0 on success, negative on invalid arguments.
///
/// # Safety
/// All non-null pointers must be valid for the sizes described above and must not alias.
#[no_mangle]
pub unsafe extern "C" fn mtk_biome_reapply_uniform_tint(
    base_tint_types: *const u8,
    packed_tint_data: *const f32,
    existing_tint_colors: *const f32,
    face_count: usize,
    grass_color: *const f32,
    foliage_color: *const f32,
    dry_foliage_color: *const f32,
    water_color: *const f32,
    colormap_uv: *const f32,
    has_custom_grass: u8,
    has_custom_foliage: u8,
    has_custom_dry_foliage: u8,
    out_packed_tint_data: *mut f32,
    out_tint_colors: *mut f32,
    out_colormap_uvs: *mut f32,
) -> i32 {
    if base_tint_types.is_null()
        || packed_tint_data.is_null()
        || grass_color.is_null()
        || foliage_color.is_null()
        || dry_foliage_color.is_null()
        || water_color.is_null()
        || colormap_uv.is_null()
        || out_packed_tint_data.is_null()
        || out_tint_colors.is_null()
        || out_colormap_uvs.is_null()
    {
        return -1;
    }

    let base = std::slice::from_raw_parts(base_tint_types, face_count);
    let packed = std::slice::from_raw_parts(packed_tint_data as *const [f32; 4], face_count);
    let existing: &[[f32; 4]] = if existing_tint_colors.is_null() {
        &[]
    } else {
        std::slice::from_raw_parts(existing_tint_colors as *const [f32; 4], face_count)
    };
    let color4 = |ptr: *const f32| [*ptr, *ptr.add(1), *ptr.add(2), *ptr.add(3)];
    let uv = [*colormap_uv, *colormap_uv.add(1)];

    let result = mtk_material::reapply_uniform_biome_tint_flat(
        base,
        packed,
        existing,
        mtk_material::BiomeColorSet {
            grass: color4(grass_color),
            foliage: color4(foliage_color),
            dry_foliage: color4(dry_foliage_color),
            water: color4(water_color),
            colormap_uv: uv,
        },
        has_custom_grass != 0,
        has_custom_foliage != 0,
        has_custom_dry_foliage != 0,
    );

    std::ptr::copy_nonoverlapping(
        result.packed_tint_data.as_ptr(),
        out_packed_tint_data,
        result.packed_tint_data.len(),
    );
    std::ptr::copy_nonoverlapping(
        result.tint_colors.as_ptr(),
        out_tint_colors,
        result.tint_colors.len(),
    );
    std::ptr::copy_nonoverlapping(
        result.colormap_uvs.as_ptr(),
        out_colormap_uvs,
        result.colormap_uvs.len(),
    );
    0
}

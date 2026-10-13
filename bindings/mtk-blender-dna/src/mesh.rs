//! Direct memory copy adapters for `mtk_core::mesh::MeshData`.

use mtk_core::mesh::MeshData;

use crate::error::DnaResult;
use crate::guard::validate_aligned_pointer_and_bounds;

/// Directly copies vertex positions into a contiguous destination pointer as raw float32 values (`len * 3`).
pub fn copy_positions_to_ptr(
    mesh: &MeshData,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> DnaResult<usize> {
    let count = mesh.positions.len();
    if count == 0 {
        return Ok(0);
    }
    let byte_size = count * std::mem::size_of::<[f32; 3]>();
    validate_aligned_pointer_and_bounds(
        dst_ptr,
        byte_size,
        std::mem::align_of::<f32>(),
        max_bytes,
    )?;
    unsafe {
        std::ptr::copy_nonoverlapping(
            mesh.positions.as_ptr() as *const u8,
            dst_ptr as *mut u8,
            byte_size,
        );
    }
    Ok(byte_size)
}

/// Directly copies vertex normals into a contiguous destination pointer as raw float32 values (`len * 3`).
pub fn copy_normals_to_ptr(
    mesh: &MeshData,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> DnaResult<usize> {
    let count = mesh.normals.len();
    if count == 0 {
        return Ok(0);
    }
    let byte_size = count * std::mem::size_of::<[f32; 3]>();
    validate_aligned_pointer_and_bounds(
        dst_ptr,
        byte_size,
        std::mem::align_of::<f32>(),
        max_bytes,
    )?;
    unsafe {
        std::ptr::copy_nonoverlapping(
            mesh.normals.as_ptr() as *const u8,
            dst_ptr as *mut u8,
            byte_size,
        );
    }
    Ok(byte_size)
}

/// Directly copies triangle indices into a contiguous destination pointer as raw uint32 values.
pub fn copy_indices_to_ptr(
    mesh: &MeshData,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> DnaResult<usize> {
    let count = mesh.indices.len();
    if count == 0 {
        return Ok(0);
    }
    let byte_size = count * std::mem::size_of::<u32>();
    validate_aligned_pointer_and_bounds(
        dst_ptr,
        byte_size,
        std::mem::align_of::<u32>(),
        max_bytes,
    )?;
    unsafe {
        std::ptr::copy_nonoverlapping(
            mesh.indices.as_ptr() as *const u8,
            dst_ptr as *mut u8,
            byte_size,
        );
    }
    Ok(byte_size)
}

/// Directly copies reconstructed quad indices into a contiguous destination pointer as raw uint32 values.
pub fn copy_quad_indices_to_ptr(
    mesh: &MeshData,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> DnaResult<usize> {
    let quads_owned;
    let quads: &[u32] = if let Some(ref quads) = mesh.quad_indices {
        quads.as_slice()
    } else {
        quads_owned = mesh.reconstruct_quad_indices();
        quads_owned.as_slice()
    };

    if quads.is_empty() {
        return Ok(0);
    }
    let byte_size = std::mem::size_of_val(quads);
    validate_aligned_pointer_and_bounds(
        dst_ptr,
        byte_size,
        std::mem::align_of::<u32>(),
        max_bytes,
    )?;
    unsafe {
        std::ptr::copy_nonoverlapping(quads.as_ptr() as *const u8, dst_ptr as *mut u8, byte_size);
    }
    Ok(byte_size)
}

/// Directly copies expanded per-loop UVs into a contiguous destination pointer as raw float32 pairs.
pub fn copy_loop_uvs_to_ptr(
    mesh: &MeshData,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> DnaResult<usize> {
    let is_quad = mesh.is_quad_topology();
    let total_loops = mesh.loop_count();

    if total_loops == 0 {
        return Ok(0);
    }

    let byte_size = total_loops * 2 * std::mem::size_of::<f32>();
    validate_aligned_pointer_and_bounds(
        dst_ptr,
        byte_size,
        std::mem::align_of::<f32>(),
        max_bytes,
    )?;

    if mesh.uvs.len() == total_loops {
        let uvs_flat = mesh.uvs_flat();
        unsafe {
            std::ptr::copy_nonoverlapping(
                uvs_flat.as_ptr() as *const u8,
                dst_ptr as *mut u8,
                byte_size,
            );
        }
        return Ok(byte_size);
    }

    let dst = dst_ptr as *mut f32;
    let mut offset = 0usize;
    if is_quad {
        let quads_owned;
        let quads: &[u32] = if let Some(ref quads) = mesh.quad_indices {
            quads.as_slice()
        } else {
            quads_owned = mesh.reconstruct_quad_indices();
            quads_owned.as_slice()
        };
        for &v_idx in quads {
            let uv = mesh.uvs.get(v_idx as usize).copied().unwrap_or([0.0, 0.0]);
            unsafe {
                *dst.add(offset) = uv[0];
                *dst.add(offset + 1) = uv[1];
            }
            offset += 2;
        }
    } else {
        for &v_idx in &mesh.indices {
            let uv = mesh.uvs.get(v_idx as usize).copied().unwrap_or([0.0, 0.0]);
            unsafe {
                *dst.add(offset) = uv[0];
                *dst.add(offset + 1) = uv[1];
            }
            offset += 2;
        }
    }
    Ok(offset * std::mem::size_of::<f32>())
}

/// Directly copies face material slots converted to 32-bit integers into the destination pointer.
pub fn copy_face_materials_to_ptr(
    mesh: &MeshData,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> DnaResult<usize> {
    let count = mesh.face_materials.len();
    if count == 0 {
        return Ok(0);
    }
    let byte_size = count * std::mem::size_of::<i32>();
    validate_aligned_pointer_and_bounds(
        dst_ptr,
        byte_size,
        std::mem::align_of::<i32>(),
        max_bytes,
    )?;

    let dst = dst_ptr as *mut i32;
    for (i, &mat) in mesh.face_materials.iter().enumerate() {
        unsafe {
            *dst.add(i) = mat as i32;
        }
    }
    Ok(byte_size)
}

/// Directly copies polygon loop start offsets into the destination pointer as 32-bit integers.
pub fn copy_loop_starts_to_ptr(
    mesh: &MeshData,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> DnaResult<usize> {
    let is_quad = mesh.is_quad_topology();
    let poly_count = mesh.poly_count();
    if poly_count == 0 {
        return Ok(0);
    }
    let byte_size = poly_count * std::mem::size_of::<i32>();
    validate_aligned_pointer_and_bounds(
        dst_ptr,
        byte_size,
        std::mem::align_of::<i32>(),
        max_bytes,
    )?;

    let stride = if is_quad { 4i32 } else { 3i32 };
    let dst = dst_ptr as *mut i32;
    for i in 0..poly_count {
        unsafe {
            *dst.add(i) = (i as i32) * stride;
        }
    }
    Ok(byte_size)
}

/// Directly copies polygon loop totals into the destination pointer as 32-bit integers.
pub fn copy_loop_totals_to_ptr(
    mesh: &MeshData,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> DnaResult<usize> {
    let is_quad = mesh.is_quad_topology();
    let poly_count = mesh.poly_count();
    if poly_count == 0 {
        return Ok(0);
    }
    let byte_size = poly_count * std::mem::size_of::<i32>();
    validate_aligned_pointer_and_bounds(
        dst_ptr,
        byte_size,
        std::mem::align_of::<i32>(),
        max_bytes,
    )?;

    let stride = if is_quad { 4i32 } else { 3i32 };
    let dst = dst_ptr as *mut i32;
    for i in 0..poly_count {
        unsafe {
            *dst.add(i) = stride;
        }
    }
    Ok(byte_size)
}

/// Directly copies vertex RGBA colors into the destination pointer as raw float32 values (`len * 4`).
pub fn copy_colors_to_ptr(
    mesh: &MeshData,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> DnaResult<usize> {
    if let Some(ref colors) = mesh.colors {
        if colors.is_empty() {
            return Ok(0);
        }
        let byte_size = colors.len() * std::mem::size_of::<[f32; 4]>();
        validate_aligned_pointer_and_bounds(
            dst_ptr,
            byte_size,
            std::mem::align_of::<f32>(),
            max_bytes,
        )?;
        unsafe {
            std::ptr::copy_nonoverlapping(
                colors.as_ptr() as *const u8,
                dst_ptr as *mut u8,
                byte_size,
            );
        }
        Ok(byte_size)
    } else {
        Ok(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mesh_direct_copy() {
        let mesh = MeshData {
            positions: vec![[0.0, 0.0, 0.0], [1.0, 1.0, 1.0]],
            indices: vec![0, 1, 0],
            ..Default::default()
        };

        let mut pos_buf = [0.0f32; 6];
        let bytes = copy_positions_to_ptr(&mesh, pos_buf.as_mut_ptr() as usize, None).unwrap();
        assert_eq!(bytes, 24);
        assert_eq!(pos_buf, [0.0, 0.0, 0.0, 1.0, 1.0, 1.0]);

        let mut idx_buf = [0u32; 3];
        let bytes = copy_indices_to_ptr(&mesh, idx_buf.as_mut_ptr() as usize, None).unwrap();
        assert_eq!(bytes, 12);
        assert_eq!(idx_buf, [0, 1, 0]);
    }

    #[test]
    fn test_quad_indices_and_loop_uv_copy() {
        // Two triangles forming a canonical quad: [0, 1, 2] and [0, 2, 3] with 1 face material
        let mesh = MeshData {
            indices: vec![0, 1, 2, 0, 2, 3],
            face_materials: vec![0],
            uvs: vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            ..Default::default()
        };

        let mut quad_buf = [0u32; 4];
        let bytes = copy_quad_indices_to_ptr(&mesh, quad_buf.as_mut_ptr() as usize, None).unwrap();
        assert_eq!(bytes, 16);
        assert_eq!(quad_buf.len(), 4);

        let mut uv_buf = [0.0f32; 8];
        let uv_bytes = copy_loop_uvs_to_ptr(&mesh, uv_buf.as_mut_ptr() as usize, None).unwrap();
        assert_eq!(uv_bytes, 32);
    }
}

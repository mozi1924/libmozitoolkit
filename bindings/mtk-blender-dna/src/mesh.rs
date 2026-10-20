//! Direct memory copy adapters for `mtk_core::mesh::MeshData`.

use mtk_core::mesh::{MeshData, UvLayout};

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
    if mesh.uv_layout == UvLayout::PerLoop {
        if mesh.uvs.is_empty() {
            return Ok(0);
        }
        let byte_size = mesh.uvs.len() * 2 * std::mem::size_of::<f32>();
        validate_aligned_pointer_and_bounds(
            dst_ptr,
            byte_size,
            std::mem::align_of::<f32>(),
            max_bytes,
        )?;
        unsafe {
            std::ptr::copy_nonoverlapping(
                mesh.uvs.as_ptr() as *const u8,
                dst_ptr as *mut u8,
                byte_size,
            );
        }
        return Ok(byte_size);
    }

    let loops = mesh.loop_uvs_expanded();
    if loops.is_empty() {
        return Ok(0);
    }
    let byte_size = loops.len() * 2 * std::mem::size_of::<f32>();
    validate_aligned_pointer_and_bounds(
        dst_ptr,
        byte_size,
        std::mem::align_of::<f32>(),
        max_bytes,
    )?;
    let dst = dst_ptr as *mut f32;
    for (i, uv) in loops.iter().enumerate() {
        unsafe {
            *dst.add(i * 2) = uv[0];
            *dst.add(i * 2 + 1) = uv[1];
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

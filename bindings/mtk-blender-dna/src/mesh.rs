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
    if let Some(ref quads) = mesh.quad_indices {
        if quads.is_empty() {
            return Ok(0);
        }
        let byte_size = quads.len() * std::mem::size_of::<u32>();
        validate_aligned_pointer_and_bounds(
            dst_ptr,
            byte_size,
            std::mem::align_of::<u32>(),
            max_bytes,
        )?;
        unsafe {
            std::ptr::copy_nonoverlapping(
                quads.as_ptr() as *const u8,
                dst_ptr as *mut u8,
                byte_size,
            );
        }
        return Ok(byte_size);
    }

    let quad_count = mesh.indices.len() / 6;
    if quad_count == 0 {
        return Ok(0);
    }
    let byte_size = quad_count * 4 * std::mem::size_of::<u32>();
    validate_aligned_pointer_and_bounds(
        dst_ptr,
        byte_size,
        std::mem::align_of::<u32>(),
        max_bytes,
    )?;

    let dst = dst_ptr as *mut u32;
    for q in 0..quad_count {
        let base = q * 6;
        let t1 = [
            mesh.indices[base],
            mesh.indices[base + 1],
            mesh.indices[base + 2],
        ];
        let t2 = [
            mesh.indices[base + 3],
            mesh.indices[base + 4],
            mesh.indices[base + 5],
        ];

        let mut o1_idx = 0;
        for i in 0..3 {
            if !t2.contains(&t1[i]) {
                o1_idx = i;
                break;
            }
        }
        let o1 = t1[o1_idx];
        let d1 = t1[(o1_idx + 2) % 3];
        let d2 = t1[(o1_idx + 1) % 3];
        let o2 = t2.iter().copied().find(|v| !t1.contains(v)).unwrap_or(d2);

        let out_base = q * 4;
        unsafe {
            *dst.add(out_base) = d1;
            *dst.add(out_base + 1) = o1;
            *dst.add(out_base + 2) = d2;
            *dst.add(out_base + 3) = o2;
        }
    }
    Ok(byte_size)
}

/// Directly copies expanded per-loop UVs into a contiguous destination pointer as raw float32 pairs.
pub fn copy_loop_uvs_to_ptr(
    mesh: &MeshData,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> DnaResult<usize> {
    let is_quad = mesh.quad_indices.is_some()
        || (!mesh.indices.is_empty()
            && mesh.indices.len() % 6 == 0
            && mesh.face_materials.len() == mesh.indices.len() / 6);

    let total_loops = if is_quad {
        if let Some(ref quads) = mesh.quad_indices {
            quads.len()
        } else {
            (mesh.indices.len() / 6) * 4
        }
    } else {
        mesh.indices.len()
    };

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
        if let Some(ref quads) = mesh.quad_indices {
            for &v_idx in quads {
                let uv = mesh.uvs.get(v_idx as usize).copied().unwrap_or([0.0, 0.0]);
                unsafe {
                    *dst.add(offset) = uv[0];
                    *dst.add(offset + 1) = uv[1];
                }
                offset += 2;
            }
        } else {
            let quad_count = mesh.indices.len() / 6;
            for q in 0..quad_count {
                let base = q * 6;
                let t1 = [
                    mesh.indices[base],
                    mesh.indices[base + 1],
                    mesh.indices[base + 2],
                ];
                let t2 = [
                    mesh.indices[base + 3],
                    mesh.indices[base + 4],
                    mesh.indices[base + 5],
                ];
                let mut o1_idx = 0;
                for i in 0..3 {
                    if !t2.contains(&t1[i]) {
                        o1_idx = i;
                        break;
                    }
                }
                let o1 = t1[o1_idx];
                let d1 = t1[(o1_idx + 2) % 3];
                let d2 = t1[(o1_idx + 1) % 3];
                let o2 = t2.iter().copied().find(|v| !t1.contains(v)).unwrap_or(d2);

                for v_idx in [d1, o1, d2, o2] {
                    let uv = mesh.uvs.get(v_idx as usize).copied().unwrap_or([0.0, 0.0]);
                    unsafe {
                        *dst.add(offset) = uv[0];
                        *dst.add(offset + 1) = uv[1];
                    }
                    offset += 2;
                }
            }
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
    let is_quad = mesh.quad_indices.is_some()
        || (!mesh.indices.is_empty()
            && mesh.indices.len() % 6 == 0
            && mesh.face_materials.len() == mesh.indices.len() / 6);
    let poly_count = if is_quad {
        mesh.face_materials.len().max(mesh.indices.len() / 6)
    } else {
        mesh.indices.len() / 3
    };
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
    let is_quad = mesh.quad_indices.is_some()
        || (!mesh.indices.is_empty()
            && mesh.indices.len() % 6 == 0
            && mesh.face_materials.len() == mesh.indices.len() / 6);
    let poly_count = if is_quad {
        mesh.face_materials.len().max(mesh.indices.len() / 6)
    } else {
        mesh.indices.len() / 3
    };
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
}

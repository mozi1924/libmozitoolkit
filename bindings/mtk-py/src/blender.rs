//! Blender-specific hardware-accelerated direct memory and DNA CustomData pointer bindings.
//! Compiled only when the `blender` cargo feature is enabled (enabled by default).

use pyo3::buffer::PyBuffer;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use crate::mesh::PyMeshData;
use crate::point_cloud::PyVoxelPointCloud;

#[inline]
pub fn validate_pointer_and_bounds(
    dst_ptr: usize,
    required_bytes: usize,
    max_bytes: Option<usize>,
) -> PyResult<()> {
    if dst_ptr == 0 {
        return Err(PyValueError::new_err(
            "Destination pointer cannot be null (0)",
        ));
    }
    if let Some(limit) = max_bytes {
        if required_bytes > limit {
            return Err(PyValueError::new_err(format!(
                "Direct copy buffer overflow protection: required {} bytes, but destination max_bytes is {}",
                required_bytes, limit
            )));
        }
    }
    Ok(())
}

/// Copies raw contiguous buffer bytes directly into a destination memory pointer with bounds checking.
#[pyfunction]
#[pyo3(signature = (src_buffer, dst_ptr, max_bytes=None))]
pub fn copy_buffer_to_ptr(
    src_buffer: &Bound<'_, PyAny>,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<usize> {
    let py_buf: PyBuffer<u8> = PyBuffer::get(src_buffer)?;
    let len = py_buf.len_bytes();
    if len == 0 {
        return Ok(0);
    }
    validate_pointer_and_bounds(dst_ptr, len, max_bytes)?;
    unsafe {
        std::ptr::copy_nonoverlapping(py_buf.buf_ptr() as *const u8, dst_ptr as *mut u8, len);
    }
    Ok(len)
}

/// Directly copies vertex positions into a contiguous destination pointer as raw float32 values (`len * 3`).
pub fn direct_copy_positions_to_ptr(
    mesh: &PyMeshData,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<usize> {
    let count = mesh.inner.positions.len();
    if count == 0 {
        return Ok(0);
    }
    let byte_size = count * std::mem::size_of::<[f32; 3]>();
    validate_pointer_and_bounds(dst_ptr, byte_size, max_bytes)?;
    unsafe {
        std::ptr::copy_nonoverlapping(
            mesh.inner.positions.as_ptr() as *const u8,
            dst_ptr as *mut u8,
            byte_size,
        );
    }
    Ok(byte_size)
}

/// Directly copies vertex normals into a contiguous destination pointer as raw float32 values (`len * 3`).
pub fn direct_copy_normals_to_ptr(
    mesh: &PyMeshData,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<usize> {
    let count = mesh.inner.normals.len();
    if count == 0 {
        return Ok(0);
    }
    let byte_size = count * std::mem::size_of::<[f32; 3]>();
    validate_pointer_and_bounds(dst_ptr, byte_size, max_bytes)?;
    unsafe {
        std::ptr::copy_nonoverlapping(
            mesh.inner.normals.as_ptr() as *const u8,
            dst_ptr as *mut u8,
            byte_size,
        );
    }
    Ok(byte_size)
}

/// Directly copies triangle indices into a contiguous destination pointer as raw uint32 values.
pub fn direct_copy_indices_to_ptr(
    mesh: &PyMeshData,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<usize> {
    let count = mesh.inner.indices.len();
    if count == 0 {
        return Ok(0);
    }
    let byte_size = count * std::mem::size_of::<u32>();
    validate_pointer_and_bounds(dst_ptr, byte_size, max_bytes)?;
    unsafe {
        std::ptr::copy_nonoverlapping(
            mesh.inner.indices.as_ptr() as *const u8,
            dst_ptr as *mut u8,
            byte_size,
        );
    }
    Ok(byte_size)
}

/// Directly copies reconstructed quad indices into a contiguous destination pointer as raw uint32 values.
pub fn direct_copy_quad_indices_to_ptr(
    mesh: &PyMeshData,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<usize> {
    let quads = mesh.inner.reconstruct_quad_indices();
    if quads.is_empty() {
        return Ok(0);
    }
    let byte_size = quads.len() * std::mem::size_of::<u32>();
    validate_pointer_and_bounds(dst_ptr, byte_size, max_bytes)?;
    unsafe {
        std::ptr::copy_nonoverlapping(quads.as_ptr() as *const u8, dst_ptr as *mut u8, byte_size);
    }
    Ok(byte_size)
}

/// Directly copies expanded per-loop UVs into a contiguous destination pointer as raw float32 pairs.
pub fn direct_copy_loop_uvs_to_ptr(
    mesh: &PyMeshData,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<usize> {
    let is_quad = mesh.inner.quad_indices.is_some()
        || (!mesh.inner.indices.is_empty()
            && mesh.inner.indices.len() % 6 == 0
            && mesh.inner.face_materials.len() == mesh.inner.indices.len() / 6);

    let total_loops = if is_quad {
        if let Some(ref quads) = mesh.inner.quad_indices {
            quads.len()
        } else {
            (mesh.inner.indices.len() / 6) * 4
        }
    } else {
        mesh.inner.indices.len()
    };

    if total_loops == 0 {
        return Ok(0);
    }

    let byte_size = total_loops * 2 * std::mem::size_of::<f32>();
    validate_pointer_and_bounds(dst_ptr, byte_size, max_bytes)?;

    if mesh.inner.uvs.len() == total_loops {
        let uvs_flat = mesh.inner.uvs_flat();
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
        let quads = mesh.inner.reconstruct_quad_indices();
        for &v_idx in &quads {
            let uv = mesh
                .inner
                .uvs
                .get(v_idx as usize)
                .copied()
                .unwrap_or([0.0, 0.0]);
            unsafe {
                *dst.add(offset) = uv[0];
                *dst.add(offset + 1) = uv[1];
            }
            offset += 2;
        }
    } else {
        for &v_idx in &mesh.inner.indices {
            let uv = mesh
                .inner
                .uvs
                .get(v_idx as usize)
                .copied()
                .unwrap_or([0.0, 0.0]);
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
pub fn direct_copy_face_materials_to_ptr(
    mesh: &PyMeshData,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<usize> {
    let count = mesh.inner.face_materials.len();
    if count == 0 {
        return Ok(0);
    }
    let byte_size = count * std::mem::size_of::<i32>();
    validate_pointer_and_bounds(dst_ptr, byte_size, max_bytes)?;

    let dst = dst_ptr as *mut i32;
    for (i, &mat) in mesh.inner.face_materials.iter().enumerate() {
        unsafe {
            *dst.add(i) = mat as i32;
        }
    }
    Ok(byte_size)
}

/// Directly copies polygon loop start offsets into the destination pointer as 32-bit integers.
pub fn direct_copy_loop_starts_to_ptr(
    mesh: &PyMeshData,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<usize> {
    let is_quad = mesh.inner.quad_indices.is_some()
        || (!mesh.inner.indices.is_empty()
            && mesh.inner.indices.len() % 6 == 0
            && mesh.inner.face_materials.len() == mesh.inner.indices.len() / 6);
    let poly_count = if is_quad {
        mesh.inner
            .face_materials
            .len()
            .max(mesh.inner.indices.len() / 6)
    } else {
        mesh.inner.indices.len() / 3
    };
    if poly_count == 0 {
        return Ok(0);
    }
    let byte_size = poly_count * std::mem::size_of::<i32>();
    validate_pointer_and_bounds(dst_ptr, byte_size, max_bytes)?;

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
pub fn direct_copy_loop_totals_to_ptr(
    mesh: &PyMeshData,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<usize> {
    let is_quad = mesh.inner.quad_indices.is_some()
        || (!mesh.inner.indices.is_empty()
            && mesh.inner.indices.len() % 6 == 0
            && mesh.inner.face_materials.len() == mesh.inner.indices.len() / 6);
    let poly_count = if is_quad {
        mesh.inner
            .face_materials
            .len()
            .max(mesh.inner.indices.len() / 6)
    } else {
        mesh.inner.indices.len() / 3
    };
    if poly_count == 0 {
        return Ok(0);
    }
    let byte_size = poly_count * std::mem::size_of::<i32>();
    validate_pointer_and_bounds(dst_ptr, byte_size, max_bytes)?;

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
pub fn direct_copy_colors_to_ptr(
    mesh: &PyMeshData,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<usize> {
    if let Some(ref colors) = mesh.inner.colors {
        if colors.is_empty() {
            return Ok(0);
        }
        let byte_size = colors.len() * std::mem::size_of::<[f32; 4]>();
        validate_pointer_and_bounds(dst_ptr, byte_size, max_bytes)?;
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

/// Directly copies voxel point cloud 3D positions into a destination memory pointer as raw float32 triples (`len * 3`).
pub fn direct_copy_point_cloud_positions_to_ptr(
    cloud: &PyVoxelPointCloud,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<usize> {
    let count = cloud.len();
    if count == 0 {
        return Ok(0);
    }
    let byte_size = cloud.inner.positions.len() * std::mem::size_of::<f32>();
    validate_pointer_and_bounds(dst_ptr, byte_size, max_bytes)?;
    unsafe {
        std::ptr::copy_nonoverlapping(
            cloud.inner.positions.as_ptr() as *const u8,
            dst_ptr as *mut u8,
            byte_size,
        );
    }
    Ok(byte_size)
}

/// Directly copies discrete integer block X coordinates into a destination pointer as raw int32 values (`len`).
pub fn direct_copy_point_cloud_block_x_to_ptr(
    cloud: &PyVoxelPointCloud,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<usize> {
    let count = cloud.inner.block_x.len();
    if count == 0 {
        return Ok(0);
    }
    let byte_size = count * std::mem::size_of::<i32>();
    validate_pointer_and_bounds(dst_ptr, byte_size, max_bytes)?;
    unsafe {
        std::ptr::copy_nonoverlapping(
            cloud.inner.block_x.as_ptr() as *const u8,
            dst_ptr as *mut u8,
            byte_size,
        );
    }
    Ok(byte_size)
}

/// Directly copies discrete integer block Y coordinates into a destination pointer as raw int32 values (`len`).
pub fn direct_copy_point_cloud_block_y_to_ptr(
    cloud: &PyVoxelPointCloud,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<usize> {
    let count = cloud.inner.block_y.len();
    if count == 0 {
        return Ok(0);
    }
    let byte_size = count * std::mem::size_of::<i32>();
    validate_pointer_and_bounds(dst_ptr, byte_size, max_bytes)?;
    unsafe {
        std::ptr::copy_nonoverlapping(
            cloud.inner.block_y.as_ptr() as *const u8,
            dst_ptr as *mut u8,
            byte_size,
        );
    }
    Ok(byte_size)
}

/// Directly copies discrete integer block Z coordinates into a destination pointer as raw int32 values (`len`).
pub fn direct_copy_point_cloud_block_z_to_ptr(
    cloud: &PyVoxelPointCloud,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<usize> {
    let count = cloud.inner.block_z.len();
    if count == 0 {
        return Ok(0);
    }
    let byte_size = count * std::mem::size_of::<i32>();
    validate_pointer_and_bounds(dst_ptr, byte_size, max_bytes)?;
    unsafe {
        std::ptr::copy_nonoverlapping(
            cloud.inner.block_z.as_ptr() as *const u8,
            dst_ptr as *mut u8,
            byte_size,
        );
    }
    Ok(byte_size)
}

/// Directly converts block states into a palette and copies int32 palette indices into a destination pointer.
/// Returns `(palette, bytes_written)`.
pub fn direct_copy_point_cloud_state_indices_to_ptr(
    cloud: &PyVoxelPointCloud,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<(Vec<String>, usize)> {
    let count = cloud.inner.block_states.len();
    if count == 0 {
        return Ok((Vec::new(), 0));
    }
    let (palette, indices) = cloud.inner.build_state_palette_indices();
    let byte_size = count * std::mem::size_of::<i32>();
    validate_pointer_and_bounds(dst_ptr, byte_size, max_bytes)?;
    unsafe {
        std::ptr::copy_nonoverlapping(indices.as_ptr() as *const u8, dst_ptr as *mut u8, byte_size);
    }
    Ok((palette, byte_size))
}

/// Directly converts biomes into a palette and copies int32 palette indices into a destination pointer.
/// Returns `(biome_palette, bytes_written)`.
pub fn direct_copy_point_cloud_biome_indices_to_ptr(
    cloud: &PyVoxelPointCloud,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<(Vec<String>, usize)> {
    let count = cloud.inner.biomes.len();
    if count == 0 {
        return Ok((Vec::new(), 0));
    }
    let (palette, indices) = cloud.inner.build_biome_palette_indices();
    let byte_size = count * std::mem::size_of::<i32>();
    validate_pointer_and_bounds(dst_ptr, byte_size, max_bytes)?;
    unsafe {
        std::ptr::copy_nonoverlapping(indices.as_ptr() as *const u8, dst_ptr as *mut u8, byte_size);
    }
    Ok((palette, byte_size))
}

/// Directly copies light levels into a destination pointer as raw uint8 values (`len`).
pub fn direct_copy_point_cloud_light_levels_to_ptr(
    cloud: &PyVoxelPointCloud,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<usize> {
    let count = cloud.inner.light_levels.len();
    if count == 0 {
        return Ok(0);
    }
    let byte_size = count * std::mem::size_of::<u8>();
    validate_pointer_and_bounds(dst_ptr, byte_size, max_bytes)?;
    unsafe {
        std::ptr::copy_nonoverlapping(
            cloud.inner.light_levels.as_ptr(),
            dst_ptr as *mut u8,
            byte_size,
        );
    }
    Ok(byte_size)
}

/// Batch direct copy of all available voxel point cloud attributes in a single call.
/// Returns `(Option<state_palette>, Option<biome_palette>)`.
pub fn direct_copy_point_cloud_to_ptrs(
    cloud: &PyVoxelPointCloud,
    pos_ptr: Option<usize>,
    block_x_ptr: Option<usize>,
    block_y_ptr: Option<usize>,
    block_z_ptr: Option<usize>,
    state_idx_ptr: Option<usize>,
    biome_idx_ptr: Option<usize>,
    light_ptr: Option<usize>,
) -> PyResult<(Option<Vec<String>>, Option<Vec<String>>)> {
    if let Some(p) = pos_ptr {
        if p != 0 {
            direct_copy_point_cloud_positions_to_ptr(cloud, p, None)?;
        }
    }
    if let Some(p) = block_x_ptr {
        if p != 0 {
            direct_copy_point_cloud_block_x_to_ptr(cloud, p, None)?;
        }
    }
    if let Some(p) = block_y_ptr {
        if p != 0 {
            direct_copy_point_cloud_block_y_to_ptr(cloud, p, None)?;
        }
    }
    if let Some(p) = block_z_ptr {
        if p != 0 {
            direct_copy_point_cloud_block_z_to_ptr(cloud, p, None)?;
        }
    }
    let state_palette = if let Some(p) = state_idx_ptr {
        if p != 0 {
            let (pal, _) = direct_copy_point_cloud_state_indices_to_ptr(cloud, p, None)?;
            Some(pal)
        } else {
            None
        }
    } else {
        None
    };
    let biome_palette = if let Some(p) = biome_idx_ptr {
        if p != 0 {
            let (pal, _) = direct_copy_point_cloud_biome_indices_to_ptr(cloud, p, None)?;
            Some(pal)
        } else {
            None
        }
    } else {
        None
    };
    if let Some(p) = light_ptr {
        if p != 0 {
            direct_copy_point_cloud_light_levels_to_ptr(cloud, p, None)?;
        }
    }
    Ok((state_palette, biome_palette))
}

/// Registers the `blender` submodule on `libmtk_py` with standalone bounds-checked memory utilities.
pub fn register_submodule(py: Python<'_>, parent_module: &Bound<'_, PyModule>) -> PyResult<()> {
    let sub = PyModule::new(py, "blender")?;
    sub.add_function(wrap_pyfunction!(copy_buffer_to_ptr, &sub)?)?;
    parent_module.add_submodule(&sub)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pointer_bounds_guard() {
        assert!(validate_pointer_and_bounds(0, 100, None).is_err());
        assert!(validate_pointer_and_bounds(0x1000, 100, Some(50)).is_err());
        assert!(validate_pointer_and_bounds(0x1000, 100, Some(100)).is_ok());
        assert!(validate_pointer_and_bounds(0x1000, 100, Some(200)).is_ok());
    }

    #[test]
    fn test_point_cloud_direct_copy_to_ptrs() {
        let mut cloud = mtk_voxel::storage::VoxelPointCloud::new();
        cloud.push(
            [1.0, 2.0, 3.0],
            [10, 20, 30],
            "minecraft:stone".into(),
            "minecraft:plains".into(),
            15,
        );
        let py_cloud = PyVoxelPointCloud { inner: cloud };

        let mut pos_buf = [0.0f32; 3];
        let mut bx_buf = [0i32; 1];
        let mut by_buf = [0i32; 1];
        let mut bz_buf = [0i32; 1];
        let mut st_buf = [0i32; 1];
        let mut bm_buf = [0i32; 1];
        let mut lt_buf = [0u8; 1];

        let res = direct_copy_point_cloud_to_ptrs(
            &py_cloud,
            Some(pos_buf.as_mut_ptr() as usize),
            Some(bx_buf.as_mut_ptr() as usize),
            Some(by_buf.as_mut_ptr() as usize),
            Some(bz_buf.as_mut_ptr() as usize),
            Some(st_buf.as_mut_ptr() as usize),
            Some(bm_buf.as_mut_ptr() as usize),
            Some(lt_buf.as_mut_ptr() as usize),
        )
        .expect("Must succeed");

        assert_eq!(pos_buf, [1.0, 2.0, 3.0]);
        assert_eq!(bx_buf, [10]);
        assert_eq!(by_buf, [20]);
        assert_eq!(bz_buf, [30]);
        assert_eq!(st_buf, [0]);
        assert_eq!(bm_buf, [0]);
        assert_eq!(lt_buf, [15]);
        assert_eq!(res.0, Some(vec!["minecraft:stone".to_string()]));
        assert_eq!(res.1, Some(vec!["minecraft:plains".to_string()]));
    }
}

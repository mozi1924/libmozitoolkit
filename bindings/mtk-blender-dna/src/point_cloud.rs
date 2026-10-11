//! Direct memory copy adapters for `mtk_voxel::storage::point_cloud::VoxelPointCloud`.

use mtk_voxel::storage::point_cloud::VoxelPointCloud;

use crate::error::DnaResult;
use crate::guard::validate_pointer_and_bounds;

/// Directly copies voxel point cloud 3D positions into a destination memory pointer as raw float32 triples (`len * 3`).
pub fn copy_point_cloud_positions_to_ptr(
    cloud: &VoxelPointCloud,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> DnaResult<usize> {
    let count = cloud.len();
    if count == 0 {
        return Ok(0);
    }
    let byte_size = cloud.positions.len() * std::mem::size_of::<f32>();
    validate_pointer_and_bounds(dst_ptr, byte_size, max_bytes)?;
    unsafe {
        std::ptr::copy_nonoverlapping(
            cloud.positions.as_ptr() as *const u8,
            dst_ptr as *mut u8,
            byte_size,
        );
    }
    Ok(byte_size)
}

/// Directly copies discrete integer block X coordinates into a destination pointer as raw int32 values (`len`).
pub fn copy_point_cloud_block_x_to_ptr(
    cloud: &VoxelPointCloud,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> DnaResult<usize> {
    let count = cloud.block_x.len();
    if count == 0 {
        return Ok(0);
    }
    let byte_size = count * std::mem::size_of::<i32>();
    validate_pointer_and_bounds(dst_ptr, byte_size, max_bytes)?;
    unsafe {
        std::ptr::copy_nonoverlapping(
            cloud.block_x.as_ptr() as *const u8,
            dst_ptr as *mut u8,
            byte_size,
        );
    }
    Ok(byte_size)
}

/// Directly copies discrete integer block Y coordinates into a destination pointer as raw int32 values (`len`).
pub fn copy_point_cloud_block_y_to_ptr(
    cloud: &VoxelPointCloud,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> DnaResult<usize> {
    let count = cloud.block_y.len();
    if count == 0 {
        return Ok(0);
    }
    let byte_size = count * std::mem::size_of::<i32>();
    validate_pointer_and_bounds(dst_ptr, byte_size, max_bytes)?;
    unsafe {
        std::ptr::copy_nonoverlapping(
            cloud.block_y.as_ptr() as *const u8,
            dst_ptr as *mut u8,
            byte_size,
        );
    }
    Ok(byte_size)
}

/// Directly copies discrete integer block Z coordinates into a destination pointer as raw int32 values (`len`).
pub fn copy_point_cloud_block_z_to_ptr(
    cloud: &VoxelPointCloud,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> DnaResult<usize> {
    let count = cloud.block_z.len();
    if count == 0 {
        return Ok(0);
    }
    let byte_size = count * std::mem::size_of::<i32>();
    validate_pointer_and_bounds(dst_ptr, byte_size, max_bytes)?;
    unsafe {
        std::ptr::copy_nonoverlapping(
            cloud.block_z.as_ptr() as *const u8,
            dst_ptr as *mut u8,
            byte_size,
        );
    }
    Ok(byte_size)
}

/// Directly converts block states into a palette and copies int32 palette indices into a destination pointer.
/// Returns `(palette, bytes_written)`.
pub fn copy_point_cloud_state_indices_to_ptr(
    cloud: &VoxelPointCloud,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> DnaResult<(Vec<String>, usize)> {
    let count = cloud.block_states.len();
    if count == 0 {
        return Ok((Vec::new(), 0));
    }
    let (palette, indices) = cloud.build_state_palette_indices();
    let byte_size = count * std::mem::size_of::<i32>();
    validate_pointer_and_bounds(dst_ptr, byte_size, max_bytes)?;
    unsafe {
        std::ptr::copy_nonoverlapping(indices.as_ptr() as *const u8, dst_ptr as *mut u8, byte_size);
    }
    Ok((palette, byte_size))
}

/// Directly converts biomes into a palette and copies int32 palette indices into a destination pointer.
/// Returns `(biome_palette, bytes_written)`.
pub fn copy_point_cloud_biome_indices_to_ptr(
    cloud: &VoxelPointCloud,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> DnaResult<(Vec<String>, usize)> {
    let count = cloud.biomes.len();
    if count == 0 {
        return Ok((Vec::new(), 0));
    }
    let (palette, indices) = cloud.build_biome_palette_indices();
    let byte_size = count * std::mem::size_of::<i32>();
    validate_pointer_and_bounds(dst_ptr, byte_size, max_bytes)?;
    unsafe {
        std::ptr::copy_nonoverlapping(indices.as_ptr() as *const u8, dst_ptr as *mut u8, byte_size);
    }
    Ok((palette, byte_size))
}

/// Directly copies light levels into a destination pointer as raw uint8 values (`len`).
pub fn copy_point_cloud_light_levels_to_ptr(
    cloud: &VoxelPointCloud,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> DnaResult<usize> {
    let count = cloud.light_levels.len();
    if count == 0 {
        return Ok(0);
    }
    let byte_size = count * std::mem::size_of::<u8>();
    validate_pointer_and_bounds(dst_ptr, byte_size, max_bytes)?;
    unsafe {
        std::ptr::copy_nonoverlapping(cloud.light_levels.as_ptr(), dst_ptr as *mut u8, byte_size);
    }
    Ok(byte_size)
}

/// Batch direct copy of all available voxel point cloud attributes in a single call.
/// Returns `(Option<state_palette>, Option<biome_palette>)`.
pub fn copy_point_cloud_to_ptrs(
    cloud: &VoxelPointCloud,
    pos_ptr: Option<usize>,
    block_x_ptr: Option<usize>,
    block_y_ptr: Option<usize>,
    block_z_ptr: Option<usize>,
    state_idx_ptr: Option<usize>,
    biome_idx_ptr: Option<usize>,
    light_ptr: Option<usize>,
) -> DnaResult<(Option<Vec<String>>, Option<Vec<String>>)> {
    if let Some(p) = pos_ptr {
        if p != 0 {
            copy_point_cloud_positions_to_ptr(cloud, p, None)?;
        }
    }
    if let Some(p) = block_x_ptr {
        if p != 0 {
            copy_point_cloud_block_x_to_ptr(cloud, p, None)?;
        }
    }
    if let Some(p) = block_y_ptr {
        if p != 0 {
            copy_point_cloud_block_y_to_ptr(cloud, p, None)?;
        }
    }
    if let Some(p) = block_z_ptr {
        if p != 0 {
            copy_point_cloud_block_z_to_ptr(cloud, p, None)?;
        }
    }
    let state_palette = if let Some(p) = state_idx_ptr {
        if p != 0 {
            let (pal, _) = copy_point_cloud_state_indices_to_ptr(cloud, p, None)?;
            Some(pal)
        } else {
            None
        }
    } else {
        None
    };
    let biome_palette = if let Some(p) = biome_idx_ptr {
        if p != 0 {
            let (pal, _) = copy_point_cloud_biome_indices_to_ptr(cloud, p, None)?;
            Some(pal)
        } else {
            None
        }
    } else {
        None
    };
    if let Some(p) = light_ptr {
        if p != 0 {
            copy_point_cloud_light_levels_to_ptr(cloud, p, None)?;
        }
    }
    Ok((state_palette, biome_palette))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_point_cloud_direct_copy() {
        let cloud = VoxelPointCloud {
            positions: vec![0.5, 0.5, 0.5],
            block_x: vec![0],
            block_y: vec![1],
            block_z: vec![2],
            block_states: vec!["minecraft:stone".to_string()],
            biomes: vec!["minecraft:plains".to_string()],
            light_levels: vec![15],
            ..Default::default()
        };

        let mut pos_buf = [0.0f32; 3];
        let bytes =
            copy_point_cloud_positions_to_ptr(&cloud, pos_buf.as_mut_ptr() as usize, None).unwrap();
        assert_eq!(bytes, 12);
        assert_eq!(pos_buf, [0.5, 0.5, 0.5]);

        let mut bx_buf = [0i32; 1];
        copy_point_cloud_block_x_to_ptr(&cloud, bx_buf.as_mut_ptr() as usize, None).unwrap();
        assert_eq!(bx_buf[0], 0);

        let mut state_buf = [0i32; 1];
        let (pal, _) =
            copy_point_cloud_state_indices_to_ptr(&cloud, state_buf.as_mut_ptr() as usize, None)
                .unwrap();
        assert_eq!(pal, vec!["minecraft:stone"]);
        assert_eq!(state_buf[0], 0);
    }
}

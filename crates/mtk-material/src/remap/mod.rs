pub mod batch;

use mtk_texture::AtlasSpriteLocation;

/// Project a local [0.0..1.0] (or continuous tiled) UV coordinate into a target AtlasSprite's Frame 0 bounds.
#[inline]
pub fn remap_local_to_atlas(u: f32, v: f32, sprite: &AtlasSpriteLocation) -> [f32; 2] {
    let [u_min, v_min, u_max, v_max] = sprite.frame_0_uv_bounds;
    let width = u_max - u_min;
    let height = v_max - v_min;

    let target_u = u_min + u * width;
    let target_v = v_min + v * height;

    [target_u, target_v]
}

/// Invert an incoming Atlas UV coordinate back to local [0.0..1.0] space.
#[inline]
pub fn remap_atlas_to_local(u: f32, v: f32, sprite: &AtlasSpriteLocation) -> [f32; 2] {
    let [u_min, v_min, u_max, v_max] = sprite.frame_0_uv_bounds;
    let width = (u_max - u_min).max(1e-7);
    let height = (v_max - v_min).max(1e-7);

    let local_u = (u - u_min) / width;
    let local_v = (v - v_min) / height;

    [local_u, local_v]
}

/// Invert a UV coordinate from a source sprite and project it into a target sprite.
#[inline]
pub fn remap_sprite_to_sprite(
    u: f32,
    v: f32,
    source_sprite: &AtlasSpriteLocation,
    target_sprite: &AtlasSpriteLocation,
) -> [f32; 2] {
    let local = remap_atlas_to_local(u, v, source_sprite);
    remap_local_to_atlas(local[0], local[1], target_sprite)
}

/// Check if a 4-vertex quad UV layout is rotated by non-orthogonal angles (e.g. jmc2obj 45-degree flowing liquid diamond).
pub fn is_quad_uv_diamond(uvs: &[[f32; 2]; 4]) -> bool {
    let du0 = uvs[1][0] - uvs[0][0];
    let dv0 = uvs[1][1] - uvs[0][1];
    let du1 = uvs[2][0] - uvs[1][0];
    let dv1 = uvs[2][1] - uvs[1][1];

    // Check for ~45 degree diagonal vectors (du ≈ ±dv)
    let is_diag0 = (du0.abs() - dv0.abs()).abs() < 1e-3 && du0.abs() > 1e-4;
    let is_diag1 = (du1.abs() - dv1.abs()).abs() < 1e-3 && du1.abs() > 1e-4;

    is_diag0 && is_diag1
}

/// Straighten a 45-degree diamond quad UV back to orthogonal [0..1] aligned coordinates.
pub fn straighten_diamond_quad_uv(uvs: &mut [[f32; 2]; 4]) -> bool {
    if !is_quad_uv_diamond(uvs) {
        return false;
    }

    let u_min = uvs.iter().map(|p| p[0]).fold(f32::INFINITY, f32::min);
    let u_max = uvs.iter().map(|p| p[0]).fold(f32::NEG_INFINITY, f32::max);
    let v_min = uvs.iter().map(|p| p[1]).fold(f32::INFINITY, f32::min);
    let v_max = uvs.iter().map(|p| p[1]).fold(f32::NEG_INFINITY, f32::max);

    // Standard orthogonal corners based on vertex order
    uvs[0] = [u_min, v_min];
    uvs[1] = [u_max, v_min];
    uvs[2] = [u_max, v_max];
    uvs[3] = [u_min, v_max];

    true
}

//! Python bindings for UV geometric algorithms.

use glam::Vec2;
use pyo3::prelude::*;
use libmtk::core::uv;

#[inline]
fn tuples_to_vec2(uvs: &[(f32, f32)]) -> Vec<Vec2> {
    uvs.iter().map(|(u, v)| Vec2::new(*u, *v)).collect()
}

#[inline]
fn vec2_to_tuples(vecs: &[Vec2]) -> Vec<(f32, f32)> {
    vecs.iter().map(|v| (v.x, v.y)).collect()
}

#[pyfunction]
pub fn calculate_uv_area(uvs: Vec<(f32, f32)>) -> f32 {
    let vecs = tuples_to_vec2(&uvs);
    uv::calculate_uv_area(&vecs)
}

#[pyfunction]
pub fn get_uv_bounds(uvs: Vec<(f32, f32)>) -> (f32, f32, f32, f32, f32, f32) {
    let vecs = tuples_to_vec2(&uvs);
    let b = uv::get_uv_bounds(&vecs);
    (b.min.x, b.min.y, b.max.x, b.max.y, b.width(), b.height())
}

#[pyfunction]
pub fn get_uv_center(uvs: Vec<(f32, f32)>) -> (f32, f32) {
    let vecs = tuples_to_vec2(&uvs);
    let c = uv::get_uv_center(&vecs);
    (c.x, c.y)
}

#[pyfunction]
#[pyo3(signature = (uvs, area_threshold=None, dist_threshold=None, pixel_step=None))]
pub fn is_uv_collapsed(
    uvs: Vec<(f32, f32)>,
    area_threshold: Option<f32>,
    dist_threshold: Option<f32>,
    pixel_step: Option<(f32, f32)>,
) -> bool {
    let vecs = tuples_to_vec2(&uvs);
    uv::is_uv_collapsed(&vecs, area_threshold, dist_threshold, pixel_step)
}

#[pyfunction]
#[pyo3(signature = (angle_rad, tolerance=1e-3))]
pub fn is_orthogonal_angle(angle_rad: f32, tolerance: f32) -> bool {
    uv::is_orthogonal_angle(angle_rad, tolerance)
}

#[pyfunction]
#[pyo3(signature = (uvs, tolerance=1e-3))]
pub fn detect_uv_rotation(uvs: Vec<(f32, f32)>, tolerance: f32) -> f32 {
    let vecs = tuples_to_vec2(&uvs);
    uv::detect_uv_rotation(&vecs, tolerance)
}

#[pyfunction]
#[pyo3(signature = (uvs, angle=None))]
pub fn straighten_uv(uvs: Vec<(f32, f32)>, angle: Option<f32>) -> (f32, bool, Vec<(f32, f32)>) {
    let vecs = tuples_to_vec2(&uvs);
    let (ang, was_straightened, new_vecs) = uv::straighten_uv(&vecs, angle);
    (ang, was_straightened, vec2_to_tuples(&new_vecs))
}

#[pyfunction]
pub fn scale_uv(uvs: Vec<(f32, f32)>, scale_factor: f32) -> Vec<(f32, f32)> {
    let vecs = tuples_to_vec2(&uvs);
    let scaled = uv::scale_uv(&vecs, scale_factor);
    vec2_to_tuples(&scaled)
}

#[pyfunction]
#[pyo3(signature = (uvs, epsilon=1e-6))]
pub fn normalize_uv_for_atlas_tiling(
    uvs: Vec<(f32, f32)>,
    epsilon: f32,
) -> (Vec<(f32, f32)>, (f32, f32, f32), (f32, f32, f32)) {
    let vecs = tuples_to_vec2(&uvs);
    let (norm_vecs, scale, loc) = uv::normalize_uv_for_atlas_tiling(&vecs, epsilon);
    (
        vec2_to_tuples(&norm_vecs),
        (scale[0], scale[1], scale[2]),
        (loc[0], loc[1], loc[2]),
    )
}

#[pyfunction]
#[pyo3(signature = (uvs, epsilon=1e-4))]
pub fn uv_requires_atlas_tiling(uvs: Vec<(f32, f32)>, epsilon: f32) -> bool {
    let vecs = tuples_to_vec2(&uvs);
    uv::uv_requires_atlas_tiling(&vecs, epsilon)
}

#[pyfunction]
#[pyo3(signature = (u, v, scale=(1.0, 1.0, 1.0), location=(0.0, 0.0, 0.0), rotation=0.0))]
pub fn restore_atlas_tiling_uv(
    u: f32,
    v: f32,
    scale: (f32, f32, f32),
    location: (f32, f32, f32),
    rotation: f32,
) -> (f32, f32) {
    uv::restore_atlas_tiling_uv(
        u,
        v,
        [scale.0, scale.1, scale.2],
        [location.0, location.1, location.2],
        rotation,
    )
}

#[pyfunction]
#[pyo3(signature = (verts, uvs, normal=None, force=false, min_slope_threshold=0.005))]
pub fn repair_quad_fluid_uv(
    verts: Vec<(f32, f32, f32)>,
    uvs: Vec<(f32, f32)>,
    normal: Option<(f32, f32, f32)>,
    force: bool,
    min_slope_threshold: f32,
) -> PyResult<(bool, Vec<(f32, f32)>)> {
    if verts.len() != 4 || uvs.len() != 4 {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "repair_quad_fluid_uv requires exactly 4 vertices and 4 UV coordinates",
        ));
    }

    let v_arr = [
        glam::Vec3::new(verts[0].0, verts[0].1, verts[0].2),
        glam::Vec3::new(verts[1].0, verts[1].1, verts[1].2),
        glam::Vec3::new(verts[2].0, verts[2].1, verts[2].2),
        glam::Vec3::new(verts[3].0, verts[3].1, verts[3].2),
    ];
    let mut uv_arr = [
        Vec2::new(uvs[0].0, uvs[0].1),
        Vec2::new(uvs[1].0, uvs[1].1),
        Vec2::new(uvs[2].0, uvs[2].1),
        Vec2::new(uvs[3].0, uvs[3].1),
    ];
    let norm = normal.map(|(x, y, z)| glam::Vec3::new(x, y, z));

    let repaired = uv::repair_quad_fluid_uv(&v_arr, &mut uv_arr, norm, force, min_slope_threshold);
    let out_uvs = vec2_to_tuples(&uv_arr);
    Ok((repaired, out_uvs))
}

#[pyfunction]
#[pyo3(signature = (verts_flat, uvs_flat, normals_flat=None, force=false, min_slope_threshold=0.005))]
pub fn batch_repair_fluid_uv(
    verts_flat: Vec<f32>,
    mut uvs_flat: Vec<f32>,
    normals_flat: Option<Vec<f32>>,
    force: bool,
    min_slope_threshold: f32,
) -> (usize, Vec<f32>) {
    let n_slice = normals_flat.as_deref();
    let count = uv::batch_repair_fluid_uv(&verts_flat, &mut uvs_flat, n_slice, force, min_slope_threshold);
    (count, uvs_flat)
}

#[pyfunction]
#[pyo3(signature = (is_flowing=true, rotation=0.0))]
pub fn get_fluid_top_uvs(is_flowing: bool, rotation: f32) -> Vec<(f32, f32)> {
    let arr = uv::get_fluid_top_uvs(is_flowing, rotation);
    vec2_to_tuples(&arr)
}

#[pyfunction]
pub fn get_fluid_side_uvs(h_left_top: f32, h_right_top: f32) -> Vec<(f32, f32)> {
    let arr = uv::get_fluid_side_uvs(h_left_top, h_right_top);
    vec2_to_tuples(&arr)
}


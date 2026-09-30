//! # `mtk-py` Subdivision Bindings
//!
//! Exposes adaptive pixel grid subdivision and polygon slicing algorithms to Python.

use pyo3::prelude::*;

use crate::mesh::PyMeshData;

/// Calculates adaptive (cols, rows) subdivisions based on UV bounds and texture size.
#[pyfunction]
#[pyo3(signature = (uvs, tex_w, tex_h, pixels_per_face=1.0, max_subdivisions=64))]
pub fn calculate_face_target_grid(
    uvs: Vec<[f32; 2]>,
    tex_w: u32,
    tex_h: u32,
    pixels_per_face: f32,
    max_subdivisions: u32,
) -> (u32, u32) {
    mtk_core::subdivide::calculate_face_target_grid(
        &uvs,
        tex_w,
        tex_h,
        pixels_per_face,
        max_subdivisions,
    )
}

/// Calculates non-uniform [0, 1] parameter cut factors along U and V axes of a quad face,
/// directly snapping interior cuts to the integer pixel grid lines of the texture image.
#[pyfunction]
#[pyo3(signature = (uvs, tex_w, tex_h, pixels_per_face=1.0, max_subdivisions=64))]
pub fn calculate_pixel_grid_cut_factors(
    uvs: Vec<[f32; 2]>,
    tex_w: u32,
    tex_h: u32,
    pixels_per_face: f32,
    max_subdivisions: u32,
) -> (Vec<f32>, Vec<f32>) {
    mtk_core::subdivide::calculate_pixel_grid_cut_factors(
        &uvs,
        tex_w,
        tex_h,
        pixels_per_face,
        max_subdivisions,
    )
}

/// Slices a 2D/3D polygon strictly along the 2D texture pixel grid lines (X = 1, 2... and Y = 1, 2...).
///
/// Returns `(positions, uvs, faces, param_coords)` where faces contains vertex index lists for sub-faces.
#[pyfunction]
#[pyo3(signature = (positions, uvs, tex_w, tex_h, pixels_per_face=1.0, max_subdivisions=64))]
pub fn slice_face_by_pixel_grid(
    py: Python<'_>,
    positions: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    tex_w: u32,
    tex_h: u32,
    pixels_per_face: f32,
    max_subdivisions: u32,
) -> (
    Vec<[f32; 3]>,
    Vec<[f32; 2]>,
    Vec<Vec<u32>>,
    Vec<[f32; 2]>,
) {
    let res = py.allow_threads(|| {
        mtk_core::subdivide::slice_face_by_pixel_grid(
            &positions,
            &uvs,
            tex_w,
            tex_h,
            pixels_per_face,
            max_subdivisions,
        )
    });
    (res.positions, res.uvs, res.faces, res.param_coords)
}

/// Performs adaptive pixel grid subdivision on a `PyMeshData` buffer.
#[pyfunction]
#[pyo3(signature = (mesh, face_resolutions=None, default_resolution=(16, 16), pixels_per_face=1.0, max_subdivisions=64, weld_dist=1e-4))]
pub fn adaptive_pixel_split_mesh(
    py: Python<'_>,
    mesh: &PyMeshData,
    face_resolutions: Option<Vec<Option<(u32, u32)>>>,
    default_resolution: (u32, u32),
    pixels_per_face: f32,
    max_subdivisions: u32,
    weld_dist: f32,
) -> PyMeshData {
    let empty_vec = Vec::new();
    let res_slice = face_resolutions.as_deref().unwrap_or(&empty_vec);
    let output_mesh = py.allow_threads(|| {
        mtk_core::subdivide::adaptive_pixel_split_mesh(
            &mesh.inner,
            res_slice,
            default_resolution,
            pixels_per_face,
            max_subdivisions,
            weld_dist,
        )
    });
    PyMeshData { inner: output_mesh }
}

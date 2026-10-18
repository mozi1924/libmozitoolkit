//! Texture alpha sampling and transparency analysis Python bindings.

use pyo3::prelude::*;

#[pyfunction]
#[pyo3(signature = (u, v, width, height, pixels, invert_y=false))]
pub fn sample_uv_alpha_f32(
    u: f32,
    v: f32,
    width: u32,
    height: u32,
    pixels: Vec<f32>,
    invert_y: bool,
) -> f32 {
    mtk_texture::sample_alpha_f32(width, height, &pixels, u, v, invert_y)
}

#[pyfunction]
#[pyo3(signature = (face_uvs, width, height, pixels, mode="CENTER", threshold=0.01, invert_y=false))]
pub fn is_face_transparent_f32(
    face_uvs: Vec<(f32, f32)>,
    width: u32,
    height: u32,
    pixels: Vec<f32>,
    mode: &str,
    threshold: f32,
    invert_y: bool,
) -> bool {
    let vecs: Vec<glam::Vec2> = face_uvs
        .iter()
        .map(|(u, v)| glam::Vec2::new(*u, *v))
        .collect();
    let sample_mode = mtk_texture::SampleMode::from_str(mode);
    mtk_texture::is_face_transparent_f32(
        &vecs,
        width,
        height,
        &pixels,
        sample_mode,
        threshold,
        invert_y,
    )
}

#[pyfunction]
#[pyo3(signature = (faces_uvs, width, height, pixels, mode="CENTER", threshold=0.01, invert_y=false))]
pub fn batch_analyze_transparent_faces_f32(
    py: Python<'_>,
    faces_uvs: Vec<Vec<(f32, f32)>>,
    width: u32,
    height: u32,
    pixels: Vec<f32>,
    mode: &str,
    threshold: f32,
    invert_y: bool,
) -> Vec<bool> {
    let faces_vecs: Vec<Vec<glam::Vec2>> = faces_uvs
        .into_iter()
        .map(|uvs| uvs.iter().map(|(u, v)| glam::Vec2::new(*u, *v)).collect())
        .collect();
    let sample_mode = mtk_texture::SampleMode::from_str(mode);
    py.allow_threads(|| {
        mtk_texture::batch_analyze_transparent_faces_f32(
            &faces_vecs,
            width,
            height,
            &pixels,
            sample_mode,
            threshold,
            invert_y,
        )
    })
}

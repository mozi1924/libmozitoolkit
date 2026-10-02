//! # Mesh Face Emitter
//!
//! Handles emitting baked faces and unit cube faces into `MeshData` and
//! `FaceAttributesCollector`, calculating vertex positions, coordinate
//! transforms, AO brightness interpolation, and anisotropy diagonal flips.

use glam::{IVec3, Vec3};
use mtk_core::direction::Direction;
use mtk_core::mesh::MeshData;
use mtk_model::baked::BakedFace;

use crate::ao::{ao_level_to_brightness, should_flip_quad_diagonal};
use crate::mesher::collector::FaceAttributesCollector;
use crate::mesher::shading::ResolvedFaceShading;
use crate::types::MesherConfig;

/// Emits a baked model face into `MeshData` with AO shading and anisotropy diagonal flip.
#[inline]
pub fn emit_baked_face(
    mesh: &mut MeshData,
    face: &BakedFace,
    wx: f32,
    wy: f32,
    wz: f32,
    ao_levels: [u8; 4],
    config: &MesherConfig,
    collector: &mut FaceAttributesCollector,
    shading: &ResolvedFaceShading,
    final_tint_color: [f32; 4],
    final_colormap_uv: [f32; 3],
    block_pos: IVec3,
    dir: Direction,
) {
    let base_idx = mesh.positions.len() as u32;
    let norm = config.transform_direction(face.normal);
    let n = [norm.x, norm.y, norm.z];

    let colors = mesh.colors.get_or_insert_with(Vec::new);

    for i in 0..4 {
        let v = face.vertices[i];
        let p = config.transform_position(Vec3::new(wx + v.x, wy + v.y, wz + v.z));
        mesh.positions.push([p.x, p.y, p.z]);
        mesh.normals.push(n);
        if let Some(ref uvs) = shading.override_uvs {
            mesh.uvs.push([uvs[i].x, uvs[i].y]);
        } else {
            mesh.uvs.push([face.uvs[i].x, 1.0 - face.uvs[i].y]);
        }

        let ao_b = ao_level_to_brightness(ao_levels[i]);
        colors.push([ao_b, ao_b, ao_b, 1.0]);
    }

    mesh.quad_indices
        .get_or_insert_with(Vec::new)
        .extend_from_slice(&[base_idx, base_idx + 1, base_idx + 2, base_idx + 3]);

    // Triangulate with anisotropy diagonal flip
    if should_flip_quad_diagonal(ao_levels) {
        mesh.indices.push(base_idx);
        mesh.indices.push(base_idx + 1);
        mesh.indices.push(base_idx + 2);

        mesh.indices.push(base_idx);
        mesh.indices.push(base_idx + 2);
        mesh.indices.push(base_idx + 3);
    } else {
        mesh.indices.push(base_idx + 1);
        mesh.indices.push(base_idx + 2);
        mesh.indices.push(base_idx + 3);

        mesh.indices.push(base_idx);
        mesh.indices.push(base_idx + 1);
        mesh.indices.push(base_idx + 3);
    }

    mesh.face_materials.push(shading.mat_slot);
    mesh.face_tint_indices.push(face.tint_index);

    collector.push_face(
        shading.texture_key.clone(),
        shading.mat_slot as i32,
        shading.chunk_id,
        shading.tex_id,
        [1.0, 1.0, 0.0, 0.0],
        face.uv_rot,
        0,
        shading.tint_data,
        final_tint_color,
        final_colormap_uv,
        shading.emission,
        block_pos,
        dir.to_index() as u8,
    );
}

/// Emits a unit cube face into `MeshData` with AO shading and anisotropy diagonal flip.
#[inline]
pub fn emit_unit_cube_face(
    mesh: &mut MeshData,
    dir: Direction,
    wx: f32,
    wy: f32,
    wz: f32,
    ao_levels: [u8; 4],
    config: &MesherConfig,
    collector: &mut FaceAttributesCollector,
    shading: &ResolvedFaceShading,
    final_tint_color: [f32; 4],
    final_colormap_uv: [f32; 3],
    block_pos: IVec3,
) {
    let base_idx = mesh.positions.len() as u32;
    let norm = config.transform_direction(dir.normal());
    let n = [norm.x, norm.y, norm.z];

    let (v0, v1, v2, v3) = match dir {
        Direction::East => (
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(1.0, 0.0, 1.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
        ),
        Direction::West => (
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, 1.0, 1.0),
        ),
        Direction::Up => (
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 1.0),
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(1.0, 1.0, 0.0),
        ),
        Direction::Down => (
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 1.0),
        ),
        Direction::South => (
            Vec3::new(0.0, 1.0, 1.0),
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(1.0, 0.0, 1.0),
            Vec3::new(1.0, 1.0, 1.0),
        ),
        Direction::North => (
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ),
    };

    let colors = mesh.colors.get_or_insert_with(Vec::new);

    for (i, v) in [v0, v1, v2, v3].into_iter().enumerate() {
        let p = config.transform_position(Vec3::new(wx + v.x, wy + v.y, wz + v.z));
        mesh.positions.push([p.x, p.y, p.z]);
        mesh.normals.push(n);

        let ao_b = ao_level_to_brightness(ao_levels[i]);
        colors.push([ao_b, ao_b, ao_b, 1.0]);
    }

    mesh.quad_indices
        .get_or_insert_with(Vec::new)
        .extend_from_slice(&[base_idx, base_idx + 1, base_idx + 2, base_idx + 3]);

    if let Some(ref uvs) = shading.override_uvs {
        for uv in uvs {
            mesh.uvs.push([uv.x, uv.y]);
        }
    } else {
        mesh.uvs.push([0.0, 1.0]);
        mesh.uvs.push([0.0, 0.0]);
        mesh.uvs.push([1.0, 0.0]);
        mesh.uvs.push([1.0, 1.0]);
    }

    if should_flip_quad_diagonal(ao_levels) {
        mesh.indices.push(base_idx);
        mesh.indices.push(base_idx + 1);
        mesh.indices.push(base_idx + 2);

        mesh.indices.push(base_idx);
        mesh.indices.push(base_idx + 2);
        mesh.indices.push(base_idx + 3);
    } else {
        mesh.indices.push(base_idx + 1);
        mesh.indices.push(base_idx + 2);
        mesh.indices.push(base_idx + 3);

        mesh.indices.push(base_idx);
        mesh.indices.push(base_idx + 1);
        mesh.indices.push(base_idx + 3);
    }

    mesh.face_materials.push(shading.mat_slot);
    mesh.face_tint_indices.push(-1);

    collector.push_face(
        shading.texture_key.clone(),
        shading.mat_slot as i32,
        shading.chunk_id,
        shading.tex_id,
        [1.0, 1.0, 0.0, 0.0],
        0.0,
        0,
        shading.tint_data,
        final_tint_color,
        final_colormap_uv,
        shading.emission,
        block_pos,
        dir.to_index() as u8,
    );
}

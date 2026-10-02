//! # Discrete Noise Face Extrusion & Mesh Rebuilding
//!
//! Generates pseudo-random or continuous 3D noise vertex displacement,
//! creates side perimeter quads, and triggers automatic side UV repair.

use alloc::vec;
use alloc::vec::Vec;

use crate::extrude::generate_extrude_heights;
use super::repair::process_flat_mesh_extrude_repair;
use super::types::{
    compute_face_normal, FlatPolygonMesh, MeshExtrudeRepairConfig,
    RandomExtrudeMeshInput, RandomExtrudeMeshOutput,
};

/// Performs complete discrete face extrusion, noise vertex displacement, topology rebuilding,
/// and automatic side UV repair in a single batch pass.
pub fn process_random_extrude_mesh(input: &RandomExtrudeMeshInput) -> RandomExtrudeMeshOutput {
    let mut output = RandomExtrudeMeshOutput::default();
    if input.selected_faces.is_empty() {
        output.new_positions = input.positions.clone();
        output.new_face_vertices = input.face_vertices.clone();
        output.new_face_uvs = input.face_uvs.clone();
        output.new_face_materials = input.face_materials.clone();
        return output;
    }

    let mut centers = Vec::with_capacity(input.selected_faces.len());
    for &f_idx in &input.selected_faces {
        let f_verts = &input.face_vertices[f_idx as usize];
        let mut cx = 0.0f32;
        let mut cy = 0.0f32;
        let mut cz = 0.0f32;
        for &v in f_verts {
            let p = input.positions[v as usize];
            cx += p[0];
            cy += p[1];
            cz += p[2];
        }
        let cnt = f_verts.len() as f32;
        centers.push([cx / cnt, cy / cnt, cz / cnt]);
    }

    let heights = generate_extrude_heights(
        &centers,
        input.noise_type,
        input.min_height,
        input.max_height,
        input.noise_scale,
        input.seed,
        None,
    );

    let mut new_positions = input.positions.clone();
    let mut new_face_vertices = input.face_vertices.clone();
    let mut new_face_uvs = input.face_uvs.clone();
    let mut new_face_materials = input.face_materials.clone();
    let mut new_pixel_steps = input.pixel_steps.clone();

    let mut newly_extruded_faces = Vec::new();

    for (sel_i, &f_idx) in input.selected_faces.iter().enumerate() {
        let h = heights[sel_i];
        if h.abs() <= 1e-6 {
            continue;
        }

        let f_idx_u = f_idx as usize;
        let base_verts = input.face_vertices[f_idx_u].clone();
        let base_uvs = input.face_uvs[f_idx_u].clone();
        let mat_idx = input.face_materials.get(f_idx_u).copied().unwrap_or(0);
        let p_step = input.pixel_steps.get(f_idx_u).copied().unwrap_or([1.0 / 64.0, 1.0 / 64.0]);

        let norm = compute_face_normal(&base_verts, &input.positions);
        let n = base_verts.len();

        // Duplicate vertices for top face
        let mut top_vert_indices = Vec::with_capacity(n);
        for &bv in &base_verts {
            let old_p = input.positions[bv as usize];
            let new_p = [
                old_p[0] + norm[0] * h,
                old_p[1] + norm[1] * h,
                old_p[2] + norm[2] * h,
            ];
            let new_v_idx = new_positions.len() as u32;
            new_positions.push(new_p);
            top_vert_indices.push(new_v_idx);
        }

        // Replace original face with newly elevated top face
        new_face_vertices[f_idx_u] = top_vert_indices.clone();
        newly_extruded_faces.push(f_idx);

        // Generate side perimeter quads
        for i in 0..n {
            let next_i = (i + 1) % n;
            let v_base_a = base_verts[i];
            let v_base_b = base_verts[next_i];
            let v_top_a = top_vert_indices[i];
            let v_top_b = top_vert_indices[next_i];

            let uv_a = base_uvs[i];
            let uv_b = base_uvs[next_i];

            // CCW quad: v_top_a, v_top_b, v_base_b, v_base_a
            let side_face = vec![v_top_a, v_top_b, v_base_b, v_base_a];
            let side_uvs = vec![uv_a, uv_b, uv_b, uv_a];

            new_face_vertices.push(side_face);
            new_face_uvs.push(side_uvs);
            new_face_materials.push(mat_idx);
            new_pixel_steps.push(p_step);
        }
    }

    output.extruded_face_indices = newly_extruded_faces.clone();

    // Trigger topological side UV repair if enabled
    if input.repair_uv && !newly_extruded_faces.is_empty() {
        let flat_mesh = FlatPolygonMesh::from_nested(
            new_positions.clone(),
            &new_face_vertices,
            &new_face_uvs,
            &new_face_materials,
        );

        let repair_cfg = MeshExtrudeRepairConfig {
            uv_mode: input.uv_mode,
            repair_uv: true,
            add_crease: input.add_crease,
            crease_val: input.crease_val,
            only_collapsed: false,
        };

        let repair_out = process_flat_mesh_extrude_repair(
            &flat_mesh,
            &newly_extruded_faces,
            &new_pixel_steps,
            &repair_cfg,
            None,
        );

        for (mod_f, mod_uv) in repair_out.modified_face_uvs {
            if (mod_f as usize) < new_face_uvs.len() {
                new_face_uvs[mod_f as usize] = mod_uv;
            }
        }
        for (mod_f, mod_mat) in repair_out.modified_face_materials {
            if (mod_f as usize) < new_face_materials.len() {
                new_face_materials[mod_f as usize] = mod_mat;
            }
        }
        output.repaired_count = repair_out.repaired_count;
    }

    output.new_positions = new_positions;
    output.new_face_vertices = new_face_vertices;
    output.new_face_uvs = new_face_uvs;
    output.new_face_materials = new_face_materials;

    output
}

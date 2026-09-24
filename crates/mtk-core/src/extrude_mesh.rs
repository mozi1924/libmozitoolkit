//! # Batch Data-In Data-Out Mesh Extrusion & UV Repair Engine
//!
//! Provides high-performance full-mesh topological analysis, smart inward/outward UV reconstruction,
//! anisotropic pixel grid snapping, atlas clamping, crease marking, and random discrete noise extrusion.

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec;
use alloc::vec::Vec;
use glam::Vec2;

use crate::geometry::Aabb2d;
use crate::extrude::{
    generate_extrude_heights, ExtrudeNoiseType, ExtrudeUvMode,
};

/// Configuration options for batch mesh extrusion and side UV repair.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeshExtrudeRepairConfig {
    pub uv_mode: ExtrudeUvMode,
    pub repair_uv: bool,
    pub add_crease: bool,
    pub crease_val: f32,
    pub only_collapsed: bool,
}

impl Default for MeshExtrudeRepairConfig {
    fn default() -> Self {
        Self {
            uv_mode: ExtrudeUvMode::Smart,
            repair_uv: true,
            add_crease: false,
            crease_val: 1.0,
            only_collapsed: false,
        }
    }
}

/// Input mesh buffer payload for extrude UV repair.
#[derive(Debug, Clone)]
pub struct ExtrudeMeshInput {
    pub positions: Vec<[f32; 3]>,
    pub face_vertices: Vec<Vec<u32>>,
    pub face_uvs: Vec<Vec<[f32; 2]>>,
    pub face_materials: Vec<u32>,
    pub selected_faces: Vec<u32>,
    pub pixel_steps: Vec<[f32; 2]>,
    pub smart_side_faces: Option<Vec<u32>>,
    pub config: MeshExtrudeRepairConfig,
}

/// Output mesh updates from extrude UV repair.
#[derive(Debug, Clone, Default)]
pub struct ExtrudeMeshOutput {
    /// Sparse map of face index -> updated loop UVs
    pub modified_face_uvs: Vec<(u32, Vec<[f32; 2]>)>,
    /// Sparse map of side face index -> synchronized material index
    pub modified_face_materials: Vec<(u32, u32)>,
    /// Sparse list of ((v1, v2), crease_value)
    pub modified_edge_creases: Vec<((u32, u32), f32)>,
    /// Total number of side faces repaired
    pub repaired_count: usize,
}

/// Calculates 2D signed Shoelace area of a polygon loop in UV space.
#[inline]
pub fn calculate_uv_area(uvs: &[[f32; 2]]) -> f32 {
    let n = uvs.len();
    if n < 3 {
        return 0.0;
    }
    let mut area = 0.0f32;
    for i in 0..n {
        let p1 = uvs[i];
        let p2 = uvs[(i + 1) % n];
        area += p1[0] * p2[1] - p2[0] * p1[1];
    }
    0.5 * area.abs()
}

/// Determines if a UV polygon is collapsed to a line/point or degenerate area.
pub fn is_uv_collapsed(uvs: &[[f32; 2]], pixel_step: Option<[f32; 2]>) -> bool {
    if uvs.len() < 3 {
        return true;
    }
    let (area_thresh, dist_thresh) = if let Some([su, sv]) = pixel_step {
        (su * sv * 0.01, su.min(sv) * 0.02)
    } else {
        (1e-6, 1e-4)
    };

    if calculate_uv_area(uvs) < area_thresh {
        return true;
    }

    let mut max_dist_sq = 0.0f32;
    for i in 0..uvs.len() {
        for j in (i + 1)..uvs.len() {
            let du = uvs[i][0] - uvs[j][0];
            let dv = uvs[i][1] - uvs[j][1];
            let d2 = du * du + dv * dv;
            if d2 > max_dist_sq {
                max_dist_sq = d2;
            }
        }
    }

    max_dist_sq < (dist_thresh * dist_thresh)
}

/// Computes normal of a 3D polygon.
#[inline]
fn compute_face_normal(face_verts: &[u32], positions: &[[f32; 3]]) -> [f32; 3] {
    if face_verts.len() < 3 {
        return [0.0, 0.0, 1.0];
    }
    let mut nx = 0.0f32;
    let mut ny = 0.0f32;
    let mut nz = 0.0f32;
    let n = face_verts.len();
    for i in 0..n {
        let v1 = positions[face_verts[i] as usize];
        let v2 = positions[face_verts[(i + 1) % n] as usize];
        nx += (v1[1] - v2[1]) * (v1[2] + v2[2]);
        ny += (v1[2] - v2[2]) * (v1[0] + v2[0]);
        nz += (v1[0] - v2[0]) * (v1[1] + v2[1]);
    }
    let len = (nx * nx + ny * ny + nz * nz).sqrt();
    if len > 1e-6 {
        [nx / len, ny / len, nz / len]
    } else {
        [0.0, 0.0, 1.0]
    }
}

/// Performs complete batch UV repair and crease assignment across a mesh.
pub fn process_mesh_extrude_repair(input: &ExtrudeMeshInput) -> ExtrudeMeshOutput {
    let mut output = ExtrudeMeshOutput::default();
    if !input.config.repair_uv && !input.config.add_crease {
        return output;
    }

    let selected_faces_set: BTreeSet<u32> = input.selected_faces.iter().copied().collect();
    if selected_faces_set.is_empty() {
        return output;
    }

    // Build edge -> list of (face_index, edge_vert_idx_in_face)
    let mut edge_to_faces: BTreeMap<(u32, u32), Vec<(u32, usize)>> = BTreeMap::new();
    for (f_idx, f_verts) in input.face_vertices.iter().enumerate() {
        let n = f_verts.len();
        for i in 0..n {
            let v1 = f_verts[i];
            let v2 = f_verts[(i + 1) % n];
            let edge_key = if v1 < v2 { (v1, v2) } else { (v2, v1) };
            edge_to_faces.entry(edge_key).or_default().push((f_idx as u32, i));
        }
    }

    // Edge crease tracking
    let mut modified_creases: BTreeMap<(u32, u32), f32> = BTreeMap::new();
    let mut modified_uvs: BTreeMap<u32, Vec<[f32; 2]>> = BTreeMap::new();
    let mut modified_mats: BTreeMap<u32, u32> = BTreeMap::new();

    for &top_face_idx in &input.selected_faces {
        let top_face_idx_usize = top_face_idx as usize;
        if top_face_idx_usize >= input.face_vertices.len() {
            continue;
        }

        let top_verts = &input.face_vertices[top_face_idx_usize];
        let top_uvs = if top_face_idx_usize < input.face_uvs.len() {
            &input.face_uvs[top_face_idx_usize]
        } else {
            continue;
        };

        if top_verts.len() < 3 || top_uvs.len() != top_verts.len() {
            continue;
        }

        let [step_u, step_v] = if top_face_idx_usize < input.pixel_steps.len() {
            input.pixel_steps[top_face_idx_usize]
        } else {
            [1.0 / 64.0, 1.0 / 64.0]
        };

        // Calculate top face UV bounds
        let mut min_u = f32::MAX;
        let mut max_u = f32::MIN;
        let mut min_v = f32::MAX;
        let mut max_v = f32::MIN;
        let mut uv_sum_u = 0.0f32;
        let mut uv_sum_v = 0.0f32;
        let mut vert_to_uv: BTreeMap<u32, [f32; 2]> = BTreeMap::new();

        for (i, &v) in top_verts.iter().enumerate() {
            let uv = top_uvs[i];
            vert_to_uv.insert(v, uv);
            min_u = min_u.min(uv[0]);
            max_u = max_u.max(uv[0]);
            min_v = min_v.min(uv[1]);
            max_v = max_v.max(uv[1]);
            uv_sum_u += uv[0];
            uv_sum_v += uv[1];
        }

        let top_face_bounds = Aabb2d::new(
            Vec2::new(min_u, min_v),
            Vec2::new(max_u, max_v),
        );
        let top_face_uv_center = [
            uv_sum_u / top_verts.len() as f32,
            uv_sum_v / top_verts.len() as f32,
        ];
        let top_normal = compute_face_normal(top_verts, &input.positions);
        let top_material = input.face_materials.get(top_face_idx_usize).copied().unwrap_or(0);

        let n = top_verts.len();
        for i in 0..n {
            let v_top_a = top_verts[i];
            let v_top_b = top_verts[(i + 1) % n];
            let edge_key = if v_top_a < v_top_b { (v_top_a, v_top_b) } else { (v_top_b, v_top_a) };

            let linked_faces = match edge_to_faces.get(&edge_key) {
                Some(lf) => lf,
                None => continue,
            };

            for &(side_face_idx, _) in linked_faces {
                if side_face_idx == top_face_idx || selected_faces_set.contains(&side_face_idx) {
                    continue;
                }

                let side_face_idx_usize = side_face_idx as usize;
                if side_face_idx_usize >= input.face_vertices.len() {
                    continue;
                }
                let side_verts = &input.face_vertices[side_face_idx_usize];
                if side_verts.len() != 4 {
                    continue;
                }

                // Material sync
                let side_mat = input.face_materials.get(side_face_idx_usize).copied().unwrap_or(0);
                if side_mat != top_material {
                    modified_mats.insert(side_face_idx, top_material);
                }

                // Identify base verts
                let base_verts: Vec<u32> = side_verts
                    .iter()
                    .copied()
                    .filter(|&v| v != v_top_a && v != v_top_b)
                    .collect();

                if base_verts.len() != 2 {
                    continue;
                }

                // Find side edges to link v_top_a -> v_base_a and v_top_b -> v_base_b
                let mut v_base_a = None;
                let mut v_base_b = None;
                for j in 0..4 {
                    let sv1 = side_verts[j];
                    let sv2 = side_verts[(j + 1) % 4];
                    if sv1 == v_top_a && base_verts.contains(&sv2) {
                        v_base_a = Some(sv2);
                    } else if sv2 == v_top_a && base_verts.contains(&sv1) {
                        v_base_a = Some(sv1);
                    }
                    if sv1 == v_top_b && base_verts.contains(&sv2) {
                        v_base_b = Some(sv2);
                    } else if sv2 == v_top_b && base_verts.contains(&sv1) {
                        v_base_b = Some(sv1);
                    }
                }

                let (v_base_a, v_base_b) = match (v_base_a, v_base_b) {
                    (Some(ba), Some(bb)) if ba != bb => (ba, bb),
                    _ => continue,
                };

                let uv_a = match vert_to_uv.get(&v_top_a) {
                    Some(&uv) => uv,
                    None => continue,
                };
                let uv_b = match vert_to_uv.get(&v_top_b) {
                    Some(&uv) => uv,
                    None => continue,
                };

                let cur_side_uvs = modified_uvs
                    .get(&side_face_idx)
                    .or_else(|| input.face_uvs.get(side_face_idx_usize));

                if input.config.only_collapsed {
                    let is_tracked = input
                        .smart_side_faces
                        .as_ref()
                        .map(|s| s.contains(&side_face_idx))
                        .unwrap_or(false);
                    if !is_tracked {
                        if let Some(s_uvs) = cur_side_uvs {
                            if !is_uv_collapsed(s_uvs, Some([step_u, step_v])) {
                                continue;
                            }
                        }
                    }
                }

                let u_edge = [uv_b[0] - uv_a[0], uv_b[1] - uv_a[1]];
                let edge_uv_mid = [(uv_a[0] + uv_b[0]) * 0.5, (uv_a[1] + uv_b[1]) * 0.5];
                let v_out = [
                    edge_uv_mid[0] - top_face_uv_center[0],
                    edge_uv_mid[1] - top_face_uv_center[1],
                ];

                let u_edge_len = (u_edge[0] * u_edge[0] + u_edge[1] * u_edge[1]).sqrt();
                let uv_outward_dir = if u_edge_len > 1e-6 {
                    let mut perp = [-u_edge[1] / u_edge_len, u_edge[0] / u_edge_len];
                    if perp[0] * v_out[0] + perp[1] * v_out[1] < 0.0 {
                        perp = [-perp[0], -perp[1]];
                    }
                    perp
                } else {
                    [1.0, 0.0]
                };

                // Resolve UV mode
                let resolved_mode = match input.config.uv_mode {
                    ExtrudeUvMode::Smart => {
                        let pos_ta = input.positions[v_top_a as usize];
                        let pos_tb = input.positions[v_top_b as usize];
                        let pos_ba = input.positions[v_base_a as usize];
                        let pos_bb = input.positions[v_base_b as usize];
                        let ext_vec = [
                            ((pos_ta[0] - pos_ba[0]) + (pos_tb[0] - pos_bb[0])) * 0.5,
                            ((pos_ta[1] - pos_ba[1]) + (pos_tb[1] - pos_bb[1])) * 0.5,
                            ((pos_ta[2] - pos_ba[2]) + (pos_tb[2] - pos_bb[2])) * 0.5,
                        ];
                        let dot = ext_vec[0] * top_normal[0]
                            + ext_vec[1] * top_normal[1]
                            + ext_vec[2] * top_normal[2];
                        if dot < -1e-6 {
                            ExtrudeUvMode::Outward
                        } else {
                            ExtrudeUvMode::Inward
                        }
                    }
                    other => other,
                };

                // Check outward adjacent face strip
                let mut adjacent_strip = None;
                if resolved_mode == ExtrudeUvMode::Outward {
                    let base_edge_key = if v_base_a < v_base_b {
                        (v_base_a, v_base_b)
                    } else {
                        (v_base_b, v_base_a)
                    };
                    if let Some(base_linked) = edge_to_faces.get(&base_edge_key) {
                        for &(adj_f_idx, _) in base_linked {
                            if adj_f_idx != side_face_idx
                                && !selected_faces_set.contains(&adj_f_idx)
                            {
                                let adj_f_idx_u = adj_f_idx as usize;
                                let adj_mat = input.face_materials.get(adj_f_idx_u).copied().unwrap_or(0);
                                if adj_mat == top_material && adj_f_idx_u < input.face_uvs.len() {
                                    let adj_verts = &input.face_vertices[adj_f_idx_u];
                                    let adj_uvs = &input.face_uvs[adj_f_idx_u];
                                    let mut adj_map = BTreeMap::new();
                                    let mut adj_sum_u = 0.0f32;
                                    let mut adj_sum_v = 0.0f32;
                                    let mut adj_min_u = f32::MAX;
                                    let mut adj_max_u = f32::MIN;
                                    let mut adj_min_v = f32::MAX;
                                    let mut adj_max_v = f32::MIN;
                                    for (ai, &av) in adj_verts.iter().enumerate() {
                                        let auv = adj_uvs[ai];
                                        adj_map.insert(av, auv);
                                        adj_sum_u += auv[0];
                                        adj_sum_v += auv[1];
                                        adj_min_u = adj_min_u.min(auv[0]);
                                        adj_max_u = adj_max_u.max(auv[0]);
                                        adj_min_v = adj_min_v.min(auv[1]);
                                        adj_max_v = adj_max_v.max(auv[1]);
                                    }
                                    if let (Some(&adj_uva), Some(&adj_uvb)) = (adj_map.get(&v_base_a), adj_map.get(&v_base_b)) {
                                        let edge_uv = [adj_uvb[0] - adj_uva[0], adj_uvb[1] - adj_uva[1]];
                                        let edge_len = (edge_uv[0] * edge_uv[0] + edge_uv[1] * edge_uv[1]).sqrt();
                                        if edge_len > 1e-6 {
                                            let adj_cnt = adj_verts.len() as f32;
                                            let adj_mid = [(adj_uva[0] + adj_uvb[0]) * 0.5, (adj_uva[1] + adj_uvb[1]) * 0.5];
                                            let mut in_dir = [-edge_uv[1] / edge_len, edge_uv[0] / edge_len];
                                            if in_dir[0] * (adj_sum_u / adj_cnt - adj_mid[0]) + in_dir[1] * (adj_sum_v / adj_cnt - adj_mid[1]) < 0.0 {
                                                in_dir = [-in_dir[0], -in_dir[1]];
                                            }
                                            let offset_u = in_dir[0] * (step_u * 0.1);
                                            let offset_v = in_dir[1] * (step_v * 0.1);
                                            let pad_u = (step_u * 0.05).min((adj_max_u - adj_min_u).abs() * 0.1);
                                            let pad_v = (step_v * 0.05).min((adj_max_v - adj_min_v).abs() * 0.1);
                                            let min_su = adj_min_u + pad_u;
                                            let max_su = adj_max_u - pad_u;
                                            let min_sv = adj_min_v + pad_v;
                                            let max_sv = adj_max_v - pad_v;

                                            let mut base_a = adj_uva;
                                            let mut base_b = adj_uvb;
                                            let mut top_a = [base_a[0] + offset_u, base_a[1] + offset_v];
                                            let mut top_b = [base_b[0] + offset_u, base_b[1] + offset_v];

                                            if max_su >= min_su {
                                                base_a[0] = base_a[0].clamp(min_su, max_su);
                                                base_b[0] = base_b[0].clamp(min_su, max_su);
                                                top_a[0] = top_a[0].clamp(min_su, max_su);
                                                top_b[0] = top_b[0].clamp(min_su, max_su);
                                            }
                                            if max_sv >= min_sv {
                                                base_a[1] = base_a[1].clamp(min_sv, max_sv);
                                                base_b[1] = base_b[1].clamp(min_sv, max_sv);
                                                top_a[1] = top_a[1].clamp(min_sv, max_sv);
                                                top_b[1] = top_b[1].clamp(min_sv, max_sv);
                                            }

                                            adjacent_strip = Some((base_a, base_b, top_a, top_b));
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                let (uv_base_a_val, uv_base_b_val, uv_top_a_val, uv_top_b_val) =
                    if let Some((ba, bb, ta, tb)) = adjacent_strip {
                        (ba, bb, ta, tb)
                    } else {
                        let mut base_a = uv_a;
                        let mut base_b = uv_b;

                        // Anisotropic pixel grid boundary alignment on the base edge
                        if uv_outward_dir[0].abs() > 0.5 {
                            if uv_outward_dir[0] > 0.0 {
                                base_a[0] = (uv_a[0] / step_u - 1e-5).ceil() * step_u;
                                base_b[0] = (uv_b[0] / step_u - 1e-5).ceil() * step_u;
                            } else {
                                base_a[0] = (uv_a[0] / step_u + 1e-5).floor() * step_u;
                                base_b[0] = (uv_b[0] / step_u + 1e-5).floor() * step_u;
                            }
                        } else if uv_outward_dir[1].abs() > 0.5 {
                            if uv_outward_dir[1] > 0.0 {
                                base_a[1] = (uv_a[1] / step_v - 1e-5).ceil() * step_v;
                                base_b[1] = (uv_b[1] / step_v - 1e-5).ceil() * step_v;
                            } else {
                                base_a[1] = (uv_a[1] / step_v + 1e-5).floor() * step_v;
                                base_b[1] = (uv_b[1] / step_v + 1e-5).floor() * step_v;
                            }
                        }

                        let dir_mult = if resolved_mode == ExtrudeUvMode::Inward {
                            -1.0
                        } else {
                            1.0
                        };
                        let offset_u = uv_outward_dir[0] * dir_mult * (step_u * 0.1);
                        let offset_v = uv_outward_dir[1] * dir_mult * (step_v * 0.1);

                        let mut top_a = [base_a[0] + offset_u, base_a[1] + offset_v];
                        let mut top_b = [base_b[0] + offset_u, base_b[1] + offset_v];

                        let pad_u = (step_u * 0.05).min((top_face_bounds.max.x - top_face_bounds.min.x).abs() * 0.1);
                        let pad_v = (step_v * 0.05).min((top_face_bounds.max.y - top_face_bounds.min.y).abs() * 0.1);
                        let min_su = top_face_bounds.min.x + pad_u;
                        let max_su = top_face_bounds.max.x - pad_u;
                        let min_sv = top_face_bounds.min.y + pad_v;
                        let max_sv = top_face_bounds.max.y - pad_v;

                        if max_su >= min_su {
                            base_a[0] = base_a[0].clamp(min_su, max_su);
                            base_b[0] = base_b[0].clamp(min_su, max_su);
                            top_a[0] = top_a[0].clamp(min_su, max_su);
                            top_b[0] = top_b[0].clamp(min_su, max_su);
                        }
                        if max_sv >= min_sv {
                            base_a[1] = base_a[1].clamp(min_sv, max_sv);
                            base_b[1] = base_b[1].clamp(min_sv, max_sv);
                            top_a[1] = top_a[1].clamp(min_sv, max_sv);
                            top_b[1] = top_b[1].clamp(min_sv, max_sv);
                        }

                        (base_a, base_b, top_a, top_b)
                    };

                let mut expected_uvs = BTreeMap::new();
                expected_uvs.insert(v_top_a, uv_top_a_val);
                expected_uvs.insert(v_top_b, uv_top_b_val);
                expected_uvs.insert(v_base_a, uv_base_a_val);
                expected_uvs.insert(v_base_b, uv_base_b_val);

                let mut new_side_uvs = Vec::with_capacity(4);
                let mut uv_changed = false;
                for (k, &sv) in side_verts.iter().enumerate() {
                    let exp = match expected_uvs.get(&sv) {
                        Some(&u) => u,
                        None => cur_side_uvs.map(|u| u[k]).unwrap_or([0.0, 0.0]),
                    };
                    if let Some(s_uvs) = cur_side_uvs {
                        let du = exp[0] - s_uvs[k][0];
                        let dv = exp[1] - s_uvs[k][1];
                        if (du * du + dv * dv) > 1e-12 {
                            uv_changed = true;
                        }
                    } else {
                        uv_changed = true;
                    }
                    new_side_uvs.push(exp);
                }

                if uv_changed {
                    modified_uvs.insert(side_face_idx, new_side_uvs);
                    output.repaired_count += 1;
                }

                if input.config.add_crease {
                    let c_val = input.config.crease_val;
                    let top_edge_key = if v_top_a < v_top_b { (v_top_a, v_top_b) } else { (v_top_b, v_top_a) };
                    let base_edge_key = if v_base_a < v_base_b { (v_base_a, v_base_b) } else { (v_base_b, v_base_a) };
                    let side_edge_a = if v_top_a < v_base_a { (v_top_a, v_base_a) } else { (v_base_a, v_top_a) };
                    let side_edge_b = if v_top_b < v_base_b { (v_top_b, v_base_b) } else { (v_base_b, v_top_b) };

                    modified_creases.insert(top_edge_key, c_val);
                    modified_creases.insert(base_edge_key, c_val);
                    modified_creases.insert(side_edge_a, c_val);
                    modified_creases.insert(side_edge_b, c_val);
                }
            }
        }

        if input.config.add_crease {
            let n = top_verts.len();
            for i in 0..n {
                let v1 = top_verts[i];
                let v2 = top_verts[(i + 1) % n];
                let e_key = if v1 < v2 { (v1, v2) } else { (v2, v1) };
                modified_creases.insert(e_key, input.config.crease_val);
            }
        }
    }

    output.modified_face_uvs = modified_uvs.into_iter().collect();
    output.modified_face_materials = modified_mats.into_iter().collect();
    output.modified_edge_creases = modified_creases.into_iter().collect();

    output
}

/// Input parameters for batch discrete random noise extrusion.
#[derive(Debug, Clone)]
pub struct RandomExtrudeMeshInput {
    pub positions: Vec<[f32; 3]>,
    pub face_vertices: Vec<Vec<u32>>,
    pub face_uvs: Vec<Vec<[f32; 2]>>,
    pub face_materials: Vec<u32>,
    pub selected_faces: Vec<u32>,
    pub pixel_steps: Vec<[f32; 2]>,
    pub min_height: f32,
    pub max_height: f32,
    pub seed: u32,
    pub noise_type: ExtrudeNoiseType,
    pub noise_scale: f32,
    pub repair_uv: bool,
    pub uv_mode: ExtrudeUvMode,
    pub add_crease: bool,
    pub crease_val: f32,
}

/// Output data containing full reconstructed mesh after random discrete extrusion.
#[derive(Debug, Clone, Default)]
pub struct RandomExtrudeMeshOutput {
    pub new_positions: Vec<[f32; 3]>,
    pub new_face_vertices: Vec<Vec<u32>>,
    pub new_face_uvs: Vec<Vec<[f32; 2]>>,
    pub new_face_materials: Vec<u32>,
    pub extruded_face_indices: Vec<u32>,
    pub repaired_count: usize,
}

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

        // Add 4 side quads
        for i in 0..n {
            let b_v1 = base_verts[i];
            let b_v2 = base_verts[(i + 1) % n];
            let t_v1 = top_vert_indices[i];
            let t_v2 = top_vert_indices[(i + 1) % n];

            let side_face = vec![b_v1, b_v2, t_v2, t_v1];
            // Initial collapsed UV from base edge
            let u1 = base_uvs[i];
            let u2 = base_uvs[(i + 1) % n];
            let side_uv = vec![u1, u2, u2, u1];

            new_face_vertices.push(side_face);
            new_face_uvs.push(side_uv);
            new_face_materials.push(mat_idx);
            new_pixel_steps.push(p_step);
        }
    }

    if input.repair_uv || input.add_crease {
        let repair_input = ExtrudeMeshInput {
            positions: new_positions.clone(),
            face_vertices: new_face_vertices.clone(),
            face_uvs: new_face_uvs.clone(),
            face_materials: new_face_materials.clone(),
            selected_faces: newly_extruded_faces.clone(),
            pixel_steps: new_pixel_steps,
            smart_side_faces: None,
            config: MeshExtrudeRepairConfig {
                uv_mode: input.uv_mode,
                repair_uv: input.repair_uv,
                add_crease: input.add_crease,
                crease_val: input.crease_val,
                only_collapsed: true,
            },
        };

        let repair_out = process_mesh_extrude_repair(&repair_input);
        for (f_idx, mod_uv) in repair_out.modified_face_uvs {
            new_face_uvs[f_idx as usize] = mod_uv;
        }
        for (f_idx, mod_mat) in repair_out.modified_face_materials {
            new_face_materials[f_idx as usize] = mod_mat;
        }
        output.repaired_count = repair_out.repaired_count;
    }

    output.new_positions = new_positions;
    output.new_face_vertices = new_face_vertices;
    output.new_face_uvs = new_face_uvs;
    output.new_face_materials = new_face_materials;
    output.extruded_face_indices = newly_extruded_faces;

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_extrude_repair_cube_top_protrusion() {
        // Simple unit cube with top face extruded
        let positions = vec![
            // Base vertices 0..4
            [-0.5, -0.5, 0.0],
            [0.5, -0.5, 0.0],
            [0.5, 0.5, 0.0],
            [-0.5, 0.5, 0.0],
            // Top vertices 4..8
            [-0.5, -0.5, 1.0],
            [0.5, -0.5, 1.0],
            [0.5, 0.5, 1.0],
            [-0.5, 0.5, 1.0],
        ];

        let face_vertices = vec![
            // Face 0: Top face (selected)
            vec![4, 5, 6, 7],
            // Face 1: Side south
            vec![0, 1, 5, 4],
            // Face 2: Side east
            vec![1, 2, 6, 5],
            // Face 3: Side north
            vec![2, 3, 7, 6],
            // Face 4: Side west
            vec![3, 0, 4, 7],
        ];

        let face_uvs = vec![
            vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            // Collapsed UVs on side faces
            vec![[0.0, 0.0], [1.0, 0.0], [1.0, 0.0], [0.0, 0.0]],
            vec![[1.0, 0.0], [1.0, 1.0], [1.0, 1.0], [1.0, 0.0]],
            vec![[1.0, 1.0], [0.0, 1.0], [0.0, 1.0], [1.0, 1.0]],
            vec![[0.0, 1.0], [0.0, 0.0], [0.0, 0.0], [0.0, 1.0]],
        ];

        let input = ExtrudeMeshInput {
            positions,
            face_vertices,
            face_uvs,
            face_materials: vec![0; 5],
            selected_faces: vec![0],
            pixel_steps: vec![[1.0 / 16.0, 1.0 / 16.0]; 5],
            smart_side_faces: None,
            config: MeshExtrudeRepairConfig {
                uv_mode: ExtrudeUvMode::Smart,
                repair_uv: true,
                add_crease: true,
                crease_val: 1.0,
                only_collapsed: true,
            },
        };

        let output = process_mesh_extrude_repair(&input);
        assert_eq!(output.repaired_count, 4);
        assert_eq!(output.modified_face_uvs.len(), 4);

        for (_f_idx, uvs) in output.modified_face_uvs {
            assert_eq!(uvs.len(), 4);
            assert!(!is_uv_collapsed(&uvs, Some([1.0 / 16.0, 1.0 / 16.0])));
        }
    }
}

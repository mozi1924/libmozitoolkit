//! # Mesh Topology Analysis
//!
//! High-performance geometry and topology query functions, including hard/sharp edge
//! classification, dihedral angle computation, and boundary detection across flat buffers.

use glam::Vec3;
use rustc_hash::FxHashMap;

/// Records adjacent faces for a mesh edge.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct EdgeAdjacency {
    face_0: u32,
    face_1: u32,
    count: u8,
}

/// Analyzes mesh edges and identifies hard/sharp edges based on dihedral angle,
/// boundary detection, and explicit sharp flags.
///
/// # Arguments
/// - `positions`: Flat 3D vertex positions `[x0, y0, z0, x1, y1, z1, ...]`.
/// - `edge_verts`: Flat pairs of vertex indices for each edge `[v0_0, v1_0, v0_1, v1_1, ...]`. (Length = 2 * num_edges)
/// - `face_loop_starts`: Starting index in `loop_verts` for each face. (Length = num_faces)
/// - `face_loop_totals`: Number of vertices/loops in each face. (Length = num_faces)
/// - `loop_verts`: Vertex index for each polygon loop. (Length = num_loops)
/// - `sharp_angle_rad`: Angle in radians above which an edge is considered sharp.
/// - `explicit_sharp_edges`: Optional slice of length `num_edges` (1 for explicit sharp, 0 otherwise).
///
/// # Returns
/// A `Vec<u8>` of length `num_edges`, where `1` indicates a hard/sharp edge and `0` indicates a smooth edge.
pub fn analyze_hard_edges(
    positions: &[f32],
    edge_verts: &[u32],
    face_loop_starts: &[u32],
    face_loop_totals: &[u32],
    loop_verts: &[u32],
    sharp_angle_rad: f32,
    explicit_sharp_edges: Option<&[u8]>,
) -> Vec<u8> {
    let num_edges = edge_verts.len() / 2;
    if num_edges == 0 {
        return Vec::new();
    }

    let num_faces = face_loop_starts.len().min(face_loop_totals.len());

    // 1. Calculate face normals using Newell's polygon normal method
    let mut face_normals = Vec::with_capacity(num_faces);
    for f in 0..num_faces {
        let start = face_loop_starts[f] as usize;
        let tot = face_loop_totals[f] as usize;

        if tot < 3 || start + tot > loop_verts.len() {
            face_normals.push(Vec3::ZERO);
            continue;
        }

        let mut normal = Vec3::ZERO;
        for i in 0..tot {
            let curr_v = loop_verts[start + i] as usize;
            let next_v = loop_verts[start + (i + 1) % tot] as usize;

            if curr_v * 3 + 2 < positions.len() && next_v * 3 + 2 < positions.len() {
                let p_curr = Vec3::new(
                    positions[curr_v * 3],
                    positions[curr_v * 3 + 1],
                    positions[curr_v * 3 + 2],
                );
                let p_next = Vec3::new(
                    positions[next_v * 3],
                    positions[next_v * 3 + 1],
                    positions[next_v * 3 + 2],
                );

                normal.x += (p_curr.y - p_next.y) * (p_curr.z + p_next.z);
                normal.y += (p_curr.z - p_next.z) * (p_curr.x + p_next.x);
                normal.z += (p_curr.x - p_next.x) * (p_curr.y + p_next.y);
            }
        }

        let len_sq = normal.length_squared();
        if len_sq > 1e-10 {
            face_normals.push(normal / len_sq.sqrt());
        } else {
            face_normals.push(Vec3::ZERO);
        }
    }

    // 2. Build edge lookup map (v_min, v_max) -> edge_index
    let mut edge_map = FxHashMap::with_capacity_and_hasher(num_edges, Default::default());
    for e in 0..num_edges {
        let v0 = edge_verts[e * 2];
        let v1 = edge_verts[e * 2 + 1];
        let key = if v0 < v1 { (v0, v1) } else { (v1, v0) };
        edge_map.insert(key, e as u32);
    }

    // 3. Connect faces to edges
    let mut adjacencies = vec![EdgeAdjacency::default(); num_edges];
    for f in 0..num_faces {
        let start = face_loop_starts[f] as usize;
        let tot = face_loop_totals[f] as usize;
        if tot < 2 || start + tot > loop_verts.len() {
            continue;
        }

        for i in 0..tot {
            let v0 = loop_verts[start + i];
            let v1 = loop_verts[start + (i + 1) % tot];
            let key = if v0 < v1 { (v0, v1) } else { (v1, v0) };

            if let Some(&edge_idx) = edge_map.get(&key) {
                let adj = &mut adjacencies[edge_idx as usize];
                if adj.count == 0 {
                    adj.face_0 = f as u32;
                } else if adj.count == 1 {
                    adj.face_1 = f as u32;
                }
                adj.count = adj.count.saturating_add(1);
            }
        }
    }

    // 4. Classify each edge as hard (1) or smooth (0)
    let mut result = vec![0u8; num_edges];
    for e in 0..num_edges {
        if let Some(explicit) = explicit_sharp_edges {
            if e < explicit.len() && explicit[e] != 0 {
                result[e] = 1;
                continue;
            }
        }

        let adj = adjacencies[e];
        match adj.count {
            0 | 1 => {
                // Isolated or open boundary edge -> hard
                result[e] = 1;
            }
            2 => {
                let n0 = face_normals[adj.face_0 as usize];
                let n1 = face_normals[adj.face_1 as usize];

                if n0 == Vec3::ZERO || n1 == Vec3::ZERO {
                    result[e] = 1;
                } else {
                    let dot = n0.dot(n1).clamp(-1.0, 1.0);
                    let angle = dot.acos();
                    if angle > sharp_angle_rad {
                        result[e] = 1;
                    }
                }
            }
            _ => {
                // Non-manifold edge (> 2 incident faces) -> hard
                result[e] = 1;
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    #[test]
    fn test_analyze_hard_edges_cube() {
        // Simple unit cube: 8 vertices, 6 faces (quads), 12 edges
        // All face dihedral angles are 90 degrees (~1.57 rad).
        // With threshold 30 degrees (~0.52 rad), all 12 edges must be hard.
        let positions = vec![
            0.0, 0.0, 0.0, // 0
            1.0, 0.0, 0.0, // 1
            1.0, 1.0, 0.0, // 2
            0.0, 1.0, 0.0, // 3
            0.0, 0.0, 1.0, // 4
            1.0, 0.0, 1.0, // 5
            1.0, 1.0, 1.0, // 6
            0.0, 1.0, 1.0, // 7
        ];

        let edge_verts = vec![
            0, 1, 1, 2, 2, 3, 3, 0, // bottom 0..3
            4, 5, 5, 6, 6, 7, 7, 4, // top 4..7
            0, 4, 1, 5, 2, 6, 3, 7, // pillars 8..11
        ];

        let face_loop_starts = vec![0, 4, 8, 12, 16, 20];
        let face_loop_totals = vec![4, 4, 4, 4, 4, 4];
        let loop_verts = vec![
            0, 3, 2, 1, // bottom (-Z)
            4, 5, 6, 7, // top (+Z)
            0, 1, 5, 4, // front (-Y)
            2, 3, 7, 6, // back (+Y)
            0, 4, 7, 3, // left (-X)
            1, 2, 6, 5, // right (+X)
        ];

        let sharp_30_deg = 30.0 * PI / 180.0;
        let hard_mask = analyze_hard_edges(
            &positions,
            &edge_verts,
            &face_loop_starts,
            &face_loop_totals,
            &loop_verts,
            sharp_30_deg,
            None,
        );

        assert_eq!(hard_mask.len(), 12);
        assert!(hard_mask.iter().all(|&v| v == 1));

        // If threshold is 100 degrees (~1.74 rad), all 90 degree edges are smooth
        let sharp_100_deg = 100.0 * PI / 180.0;
        let smooth_mask = analyze_hard_edges(
            &positions,
            &edge_verts,
            &face_loop_starts,
            &face_loop_totals,
            &loop_verts,
            sharp_100_deg,
            None,
        );
        assert!(smooth_mask.iter().all(|&v| v == 0));
    }

    #[test]
    fn test_coplanar_quads_shared_edge_is_smooth() {
        // Two coplanar quads side by side on XY plane:
        // Quad 0: (0,0)-(1,0)-(1,1)-(0,1)
        // Quad 1: (1,0)-(2,0)-(2,1)-(1,1)
        // Shared edge between vertex 1 and 2 has dihedral angle 0.0 -> smooth (0).
        // Boundary edges have count 1 -> hard (1).
        let positions = vec![
            0.0, 0.0, 0.0, // 0
            1.0, 0.0, 0.0, // 1
            1.0, 1.0, 0.0, // 2
            0.0, 1.0, 0.0, // 3
            2.0, 0.0, 0.0, // 4
            2.0, 1.0, 0.0, // 5
        ];

        let edge_verts = vec![
            1, 2, // 0: Shared internal edge
            0, 1, // 1: Boundary
            2, 3, // 2: Boundary
            3, 0, // 3: Boundary
            1, 4, // 4: Boundary
            4, 5, // 5: Boundary
            5, 2, // 6: Boundary
        ];

        let face_loop_starts = vec![0, 4];
        let face_loop_totals = vec![4, 4];
        let loop_verts = vec![
            0, 1, 2, 3, // Quad 0
            1, 4, 5, 2, // Quad 1
        ];

        let mask = analyze_hard_edges(
            &positions,
            &edge_verts,
            &face_loop_starts,
            &face_loop_totals,
            &loop_verts,
            0.5,
            None,
        );

        assert_eq!(mask[0], 0, "Internal coplanar edge must be smooth");
        for i in 1..7 {
            assert_eq!(mask[i], 1, "Boundary edge {} must be hard", i);
        }
    }
}

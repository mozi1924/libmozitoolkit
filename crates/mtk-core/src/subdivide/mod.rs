//! # Adaptive Pixel Split & Quad Subdivider
//!
//! Provides pixel-density aware quad face subdivision with bilinear interpolation
//! of all vertex and corner attributes (positions, normals, UVs, colors, custom layers, bone deform weights).

pub mod clip;
pub mod grid;
pub mod mesh_split;

pub use clip::{slice_face_by_pixel_grid, SlicedFaceResult};
pub use grid::{
    calculate_face_target_grid, calculate_pixel_grid_cut_factors,
    interpolate_bilinear_2d, interpolate_bilinear_3d, interpolate_bilinear_4d,
    invert_quad_bilinear,
};
pub use mesh_split::{adaptive_pixel_split_mesh, weld_mesh_vertices};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::MeshData;

    #[test]
    fn test_calculate_face_target_grid_animated_strip() {
        let uvs = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        // 16x512 animated water strip
        let (cols, rows) = calculate_face_target_grid(&uvs, 16, 512, 1.0, 64);
        assert_eq!(cols, 16);
        assert_eq!(rows, 16, "Must clamp to single square frame 16x16 instead of 512!");
    }

    #[test]
    fn test_calculate_face_target_grid_atlas_subregion() {
        // 16x16 tile inside a 512x512 atlas
        let u0 = 32.0 / 512.0;
        let u1 = 48.0 / 512.0;
        let v0 = 64.0 / 512.0;
        let v1 = 80.0 / 512.0;
        let uvs = [[u0, v0], [u1, v0], [u1, v1], [u0, v1]];
        let (cols, rows) = calculate_face_target_grid(&uvs, 512, 512, 1.0, 64);
        assert_eq!(cols, 16);
        assert_eq!(rows, 16);
    }

    #[test]
    fn test_calculate_face_target_grid_rotated_uv() {
        // 16x16 tile rotated 90 degrees in a 512x512 atlas
        let u0 = 32.0 / 512.0;
        let u1 = 48.0 / 512.0;
        let v0 = 64.0 / 512.0;
        let v1 = 80.0 / 512.0;
        let uvs = [[u1, v0], [u1, v1], [u0, v1], [u0, v0]];
        let (cols, rows) = calculate_face_target_grid(&uvs, 512, 512, 1.0, 64);
        assert_eq!(cols, 16);
        assert_eq!(rows, 16);
    }

    #[test]
    fn test_calculate_pixel_grid_cut_factors_offset_uv() {
        // Quad from (1.3, 2.4) to (4.7, 5.8) in 16x16 pixel space
        let u0 = 1.3 / 16.0;
        let u1 = 4.7 / 16.0;
        let v0 = 2.4 / 16.0;
        let v1 = 5.8 / 16.0;
        let uvs = [[u0, v0], [u1, v0], [u1, v1], [u0, v1]];

        let (u_cuts, v_cuts) = calculate_pixel_grid_cut_factors(&uvs, 16, 16, 1.0, 64);
        assert_eq!(u_cuts.len(), 5, "Must have [0.0, cut(2.0), cut(3.0), cut(4.0), 1.0]");
        assert_eq!(v_cuts.len(), 5, "Must have [0.0, cut(3.0), cut(4.0), cut(5.0), 1.0]");

        // Verify that u_cuts[1] corresponds exactly to pixel 2.0
        let px_u1 = (u0 + u_cuts[1] * (u1 - u0)) * 16.0;
        assert!((px_u1 - 2.0).abs() < 1e-4, "Cut 1 must be at integer pixel 2.0, got {}", px_u1);
        let px_u2 = (u0 + u_cuts[2] * (u1 - u0)) * 16.0;
        assert!((px_u2 - 3.0).abs() < 1e-4, "Cut 2 must be at integer pixel 3.0, got {}", px_u2);
        let px_u3 = (u0 + u_cuts[3] * (u1 - u0)) * 16.0;
        assert!((px_u3 - 4.0).abs() < 1e-4, "Cut 3 must be at integer pixel 4.0, got {}", px_u3);
    }

    #[test]
    fn test_calculate_pixel_grid_cut_factors_rotated_90() {
        // 90-degree rotated quad from (2.4, 4.7) to (2.4, 1.3)
        let u0 = 2.4 / 16.0;
        let u1 = 5.8 / 16.0;
        let v0 = 4.7 / 16.0;
        let v1 = 1.3 / 16.0;
        let uvs = [[u0, v0], [u0, v1], [u1, v1], [u1, v0]];

        let (u_cuts, v_cuts) = calculate_pixel_grid_cut_factors(&uvs, 16, 16, 1.0, 64);
        assert_eq!(u_cuts.len(), 5);
        assert_eq!(v_cuts.len(), 5);
    }

    #[test]
    fn test_adaptive_pixel_split_quad() {
        let mut mesh = MeshData::new();
        mesh.positions = vec![
            [-0.5, -0.5, 0.0],
            [0.5, -0.5, 0.0],
            [0.5, 0.5, 0.0],
            [-0.5, 0.5, 0.0],
        ];
        mesh.normals = vec![[0.0, 0.0, 1.0]; 4];
        mesh.uvs = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        mesh.indices = vec![0, 1, 2, 0, 2, 3];
        mesh.face_materials = vec![0];
        mesh.face_tint_indices = vec![-1];

        let res = adaptive_pixel_split_mesh(&mesh, &[Some((2, 2))], (16, 16), 1.0, 64, 0.0);
        assert_eq!(res.indices.len(), 4 * 6, "2x2 sub-quads must result in 4 quads = 24 indices");
        assert_eq!(res.face_materials.len(), 4);
    }

    #[test]
    fn test_slice_face_by_pixel_grid_rotated_45() {
        // Quad on a 3D unit cube face (Z = 0)
        let positions = [
            [-1.0, -1.0, 0.0],
            [1.0, -1.0, 0.0],
            [1.0, 1.0, 0.0],
            [-1.0, 1.0, 0.0],
        ];
        // 45-degree rotated diamond UV in 16x16 space
        let uvs = [
            [0.5, 0.0],
            [1.0, 0.5],
            [0.5, 1.0],
            [0.0, 0.5],
        ];

        let result = slice_face_by_pixel_grid(&positions, &uvs, 16, 16, 1.0, 64);
        assert!(!result.faces.is_empty(), "Must produce sliced faces");
        assert!(result.faces.len() > 10, "16x16 pixel diamond must produce multiple faces");

        // Verify that in UV space, edges of interior quads are axis-aligned (dx=0 or dy=0)
        let mut interior_quad_count = 0;
        for face in &result.faces {
            if face.len() == 4 {
                let u0 = result.uvs[face[0] as usize];
                let u1 = result.uvs[face[1] as usize];
                let du_01 = (u1[0] - u0[0]) * 16.0;
                let dv_01 = (u1[1] - u0[1]) * 16.0;

                // An interior quad has exact 1.0 px width and height along X or Y
                let is_1x1 = (du_01.abs() - 1.0).abs() < 1e-3 || (dv_01.abs() - 1.0).abs() < 1e-3;
                if is_1x1 {
                    interior_quad_count += 1;
                }
            }
        }
        assert!(interior_quad_count > 0, "Must have 1x1 pixel interior quads");
    }
}

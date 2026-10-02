//! # Batch Data-In Data-Out Mesh Extrusion & UV Repair Engine
//!
//! Provides high-performance full-mesh topological analysis, smart inward/outward UV reconstruction,
//! anisotropic pixel grid snapping, atlas clamping, crease marking, and random discrete noise extrusion.

pub mod random;
pub mod repair;
pub mod types;

pub use random::process_random_extrude_mesh;
pub use repair::{process_flat_mesh_extrude_repair, process_mesh_extrude_repair};
pub use types::{
    ExtrudeMeshInput, ExtrudeMeshOutput, FlatPolygonMesh, MeshExtrudeRepairConfig,
    RandomExtrudeMeshInput, RandomExtrudeMeshOutput,
};

pub use crate::uv::{
    calculate_uv_area_2d as calculate_uv_area, is_uv_collapsed_2d as is_uv_collapsed,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extrude::ExtrudeUvMode;

    #[test]
    fn test_batch_extrude_repair_cube_top_protrusion() {
        let positions = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 1.0],
            [1.0, 1.0, 1.0],
            [0.0, 1.0, 1.0],
            [0.0, 0.0, 2.0],
            [1.0, 0.0, 2.0],
            [1.0, 1.0, 2.0],
            [0.0, 1.0, 2.0],
        ];

        let face_vertices = vec![
            vec![8, 9, 10, 11],
            vec![4, 5, 9, 8],
            vec![5, 6, 10, 9],
            vec![6, 7, 11, 10],
            vec![7, 4, 8, 11],
        ];

        let top_uv = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        let collapsed_uv = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 0.0], [0.0, 0.0]];

        let face_uvs = vec![
            top_uv,
            collapsed_uv.clone(),
            collapsed_uv.clone(),
            collapsed_uv.clone(),
            collapsed_uv.clone(),
        ];

        let input = ExtrudeMeshInput {
            positions,
            face_vertices,
            face_uvs,
            face_materials: vec![0, 0, 0, 0, 0],
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
        assert!(!output.modified_edge_creases.is_empty());
    }

    #[test]
    fn test_flat_polygon_mesh_and_flat_repair() {
        let positions_flat: Vec<f32> = vec![
            0.0, 0.0, 0.0,
            1.0, 0.0, 0.0,
            1.0, 1.0, 0.0,
            0.0, 1.0, 0.0,
            0.0, 0.0, 1.0,
            1.0, 0.0, 1.0,
            1.0, 1.0, 1.0,
            0.0, 1.0, 1.0,
        ];

        let loop_vertices = vec![
            4, 5, 6, 7, // Top face 0
            0, 1, 5, 4, // Side face 1
            1, 2, 6, 5, // Side face 2
            2, 3, 7, 6, // Side face 3
            3, 0, 4, 7, // Side face 4
        ];

        let top_uvs = vec![0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0];
        let col_uvs = vec![0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 0.0];
        let mut loop_uvs = Vec::new();
        loop_uvs.extend_from_slice(&top_uvs);
        for _ in 0..4 {
            loop_uvs.extend_from_slice(&col_uvs);
        }

        let face_loop_starts = vec![0, 4, 8, 12, 16];
        let face_loop_totals = vec![4, 4, 4, 4, 4];
        let face_materials = vec![0, 0, 0, 0, 0];

        let flat_mesh = FlatPolygonMesh::from_flat_buffers(
            &positions_flat,
            loop_vertices,
            &loop_uvs,
            face_loop_starts,
            face_loop_totals,
            face_materials,
        );

        assert_eq!(flat_mesh.face_count(), 5);

        let pixel_steps = vec![[1.0 / 16.0, 1.0 / 16.0]; 5];
        let cfg = MeshExtrudeRepairConfig {
            uv_mode: ExtrudeUvMode::Smart,
            repair_uv: true,
            add_crease: false,
            crease_val: 0.0,
            only_collapsed: true,
        };

        let output = process_flat_mesh_extrude_repair(&flat_mesh, &[0], &pixel_steps, &cfg, None);
        assert_eq!(output.repaired_count, 4);
    }
}

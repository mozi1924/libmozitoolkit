//! # Mesh Extrusion Types & Geometrical Helpers

use alloc::vec::Vec;

pub use crate::polygon::FlatPolygonMesh;

/// UV repair mode for extruded side polygons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ExtrudeUvMode {
    /// Smart projection mode: evaluates dot product of extrude vector and top face normal.
    /// Extrusion outward (>= -1e-6) uses INWARD sampling; indentation (< -1e-6) uses OUTWARD sampling.
    #[default]
    Smart,
    /// Inward sampling: samples from top face perimeter into the interior (by 0.1 pixel step).
    Inward,
    /// Outward sampling: extends UV coordinates from the adjacent base polygon.
    Outward,
}

/// Noise algorithm for random extrusion height generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ExtrudeNoiseType {
    /// Uniform pseudo-random distribution.
    #[default]
    UniformRandom,
    /// Continuous 3D Perlin gradient noise for smooth undulating terrain.
    Perlin,
    /// Voronoi / Cellular grid noise for flat faceted stone / brick steps.
    Cellular,
}

/// Configuration options for batch mesh extrusion and side UV repair.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MeshExtrudeRepairConfig {
    pub uv_mode: ExtrudeUvMode,
    pub repair_uv: bool,
    /// Whether to track modified feature edges (creases / contours)
    pub add_crease: bool,
    /// Crease sharpness value associated with feature edges (defaults to 1.0)
    pub crease_val: f32,
    pub only_collapsed: bool,
}

impl MeshExtrudeRepairConfig {
    /// Pure geometric alias for add_crease.
    #[inline]
    pub fn track_edges(&self) -> bool {
        self.add_crease
    }
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
    /// Explicit target side faces to repair (host-agnostic alias for smart_side_faces)
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
    /// Sparse list of ((v1, v2), crease_value) for DCC crease assignment
    pub modified_edge_creases: Vec<((u32, u32), f32)>,
    /// Pure geometric list of affected feature edges (v1, v2) where v1 < v2
    pub modified_edges: Vec<(u32, u32)>,
    /// Total number of side faces repaired
    pub repaired_count: usize,
}

/// Input data for random discrete face extrusion.
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

/// Computes normal of a 3D polygon.
#[inline]
pub(crate) fn compute_face_normal(face_verts: &[u32], positions: &[[f32; 3]]) -> [f32; 3] {
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

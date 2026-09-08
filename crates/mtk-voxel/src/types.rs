use std::sync::Arc;
use glam::Vec3;
use mtk_resource::CtmSolver;
use mtk_texture::atlas::AtlasAddressMap;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Coordinate space transformation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum CoordinateSystem {
    /// Canonical Minecraft coordinate system: +X East, +Y Up, +Z South.
    #[default]
    Minecraft,
    /// Canonical Blender coordinate system: +X East, +Y North (-Z_mc), +Z Up (+Y_mc).
    Blender,
}

/// Configuration options for the voxel section and world mesher.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MesherConfig {
    /// Target coordinate system transformation.
    pub coordinate_system: CoordinateSystem,
    /// Whether to center local geometry relative to active selection origin.
    pub origin_centered: bool,
    /// Whether to calculate 4-corner ambient occlusion on faces.
    pub enable_ao: bool,
    /// Whether internal overlapping faces of complex non-cube models are clipped.
    pub exclude_hidden_volume: bool,
    /// Whether to calculate fluid top and side surfaces.
    pub mesh_fluids: bool,
    /// Optional CTM rule solver for connected textures.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub ctm_solver: Option<Arc<CtmSolver>>,
    /// Optional Atlas address map for UV remapping and material slotting.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub atlas_address_map: Option<Arc<AtlasAddressMap>>,
}

impl Default for MesherConfig {
    fn default() -> Self {
        Self {
            coordinate_system: CoordinateSystem::Minecraft,
            origin_centered: false,
            enable_ao: true,
            exclude_hidden_volume: true,
            mesh_fluids: true,
            ctm_solver: None,
            atlas_address_map: None,
        }
    }
}

impl MesherConfig {
    /// Converts a local coordinate (x, y, z in block space [0..1] or world space)
    /// to the configured target coordinate system.
    #[inline]
    pub fn transform_coord(&self, pos: Vec3) -> Vec3 {
        match self.coordinate_system {
            CoordinateSystem::Minecraft => pos,
            CoordinateSystem::Blender => Vec3::new(pos.x, -pos.z, pos.y),
        }
    }
}

/// Statistics and metrics returned after a world mesh assembly pass.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct WorldMeshBuildResult {
    /// Total number of vertices generated.
    pub vertex_count: usize,
    /// Total number of polygon faces (quads/triangles) generated.
    pub face_count: usize,
    /// Number of full solid cube blocks meshed.
    pub cubes_count: usize,
    /// Number of partial/non-cube models meshed.
    pub props_count: usize,
    /// Number of fluid block faces meshed.
    pub fluids_count: usize,
}

/// Errors that can occur during voxel storage or meshing operations.
#[derive(Debug, Error)]
pub enum VoxelError {
    #[error("Thread pool error: {0}")]
    ThreadPoolError(String),
    #[error("Coordinate out of bounds: ({0}, {1}, {2})")]
    OutOfBounds(i32, i32, i32),
    #[error("Malformed snapshot data: {0}")]
    MalformedSnapshot(String),
}

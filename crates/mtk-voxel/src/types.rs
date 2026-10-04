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
    /// Standard Z-up right-handed coordinate system: +X East, +Y North (-Z_mc), +Z Up (+Y_mc).
    /// Used by Unreal Engine, Blender, 3ds Max, etc.
    ZUpRightHanded,
    /// Standard Y-up right-handed coordinate system: +X East, +Y Up (+Y_mc), +Z South (+Z_mc).
    /// Used by Unity, Godot, Three.js, WebGPU, etc.
    YUpRightHanded,
}

/// Configuration options for the voxel section and world mesher.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MesherConfig {
    /// Target coordinate system transformation.
    pub coordinate_system: CoordinateSystem,
    /// Whether to center local geometry relative to active selection origin (bottom center).
    pub origin_centered: bool,
    /// Whether to weld adjacent coplanar/shared vertices into manifold topology.
    pub weld_vertices: bool,
    /// Optional selection bounds `(min_pos, size)`: `([min_x, min_y, min_z], [size_x, size_y, size_z])`.
    pub selection_bounds: Option<([i32; 3], [i32; 3])>,
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
    /// Optional BiomeResolver for foliage, grass, and water tinting.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub biome_resolver: Option<Arc<mtk_material::BiomeResolver>>,
    /// Optional custom material/texture alias map.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub custom_aliases: Option<Arc<std::collections::HashMap<String, Vec<String>>>>,
    /// Whether to enable vanilla alternate block models (deterministic random rotation for dirt, stone, sand, etc.).
    pub enable_alternate_blocks: bool,
    /// Whether to enable deterministic position offsets for plants (flowers, tall grass, roots, etc.).
    pub enable_random_offsets: bool,
}

impl Default for MesherConfig {
    fn default() -> Self {
        Self {
            coordinate_system: CoordinateSystem::Minecraft,
            origin_centered: true,
            weld_vertices: true,
            selection_bounds: None,
            enable_ao: true,
            exclude_hidden_volume: true,
            mesh_fluids: true,
            ctm_solver: None,
            atlas_address_map: None,
            biome_resolver: None,
            custom_aliases: None,
            enable_alternate_blocks: true,
            enable_random_offsets: true,
        }
    }
}

impl MesherConfig {
    /// Converts a 3D vertex position in Minecraft coordinates `(x, y, z)`
    /// to the configured target coordinate system, taking into account origin centering.
    #[inline]
    pub fn transform_position(&self, pos: Vec3) -> Vec3 {
        let centered = if self.origin_centered {
            if let Some((min, size)) = self.selection_bounds {
                let cx = min[0] as f32 + size[0] as f32 / 2.0;
                let by = min[1] as f32;
                let cz = min[2] as f32 + size[2] as f32 / 2.0;
                Vec3::new(pos.x - cx, pos.y - by, pos.z - cz)
            } else {
                pos
            }
        } else {
            pos
        };

        match self.coordinate_system {
            CoordinateSystem::Minecraft => centered,
            CoordinateSystem::ZUpRightHanded => Vec3::new(centered.x, -centered.z, centered.y),
            CoordinateSystem::YUpRightHanded => centered,
        }
    }

    /// Converts a direction vector (normal) to the configured target coordinate system.
    /// Direction vectors are rotated/axis-swapped, but never translated by origin.
    #[inline]
    pub fn transform_direction(&self, dir: Vec3) -> Vec3 {
        match self.coordinate_system {
            CoordinateSystem::Minecraft => dir,
            CoordinateSystem::ZUpRightHanded => Vec3::new(dir.x, -dir.z, dir.y),
            CoordinateSystem::YUpRightHanded => dir,
        }
    }

    /// Converts a local coordinate (x, y, z in block space [0..1] or world space)
    /// to the configured target coordinate system.
    #[inline]
    pub fn transform_coord(&self, pos: Vec3) -> Vec3 {
        self.transform_position(pos)
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

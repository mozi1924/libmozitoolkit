//! # Voxel World Scene and Pipeline
//!
//! Provides the authoritative high-level `VoxelWorld` container integrating
//! sparse voxel storage, ambient occlusion, face culling, prebaked models,
//! multi-threaded Section meshing, incremental dirty remeshing, Section mesh
//! caching, world mesh assembly with vertex welding, and used material chunk
//! tracking.
//!
//! Fully host-agnostic and decoupled from WebSocket network synchronization.

use std::collections::HashMap;
use std::sync::Arc;

use glam::IVec3;
use mtk_core::mesh::MeshData;
use mtk_cull::FaceCuller;
use mtk_model::baker::baked_model::BakedModelDatabase;

use crate::source::VoxelSource;
use crate::storage::VoxelStorage;
use crate::types::{MesherConfig, VoxelError};

pub mod assembly;
pub mod debug;
pub mod rebuild;
pub mod sync;

/// Unified 3D Voxel World scene managing storage, multi-source ingestion,
/// Section mesh caching, incremental remeshing, and unified geometry assembly.
#[derive(Debug, Clone)]
pub struct VoxelWorld {
    /// In-memory 3D Voxel storage (sections, palettes, biomes, bounds).
    pub storage: VoxelStorage,
    /// Active mesher configuration (AO, fluids, Z-up, origin centering, etc.).
    pub config: MesherConfig,
    /// Face Culling rules and state machine.
    pub culler: FaceCuller,
    /// Prebaked Minecraft blockstate model database for custom models.
    pub model_db: Option<Arc<BakedModelDatabase>>,
    /// Custom thread count for Rayon parallel meshing (None for global pool).
    pub num_threads: Option<usize>,
    /// Cache of section meshes: `(sx, sy, sz) -> MeshData`.
    pub(crate) section_mesh_cache: HashMap<IVec3, MeshData>,
    /// Assembled unified world mesh buffer.
    pub(crate) world_mesh: Option<MeshData>,
    /// Execution profile of the most recent rebuild operation.
    pub last_profile: Option<mtk_core::ExecutionProfile>,
    /// Unique Atlas Chunk IDs actually present on the active world mesh geometry.
    pub(crate) used_chunk_ids: Vec<u32>,
}

impl VoxelWorld {
    /// Creates a new `VoxelWorld` with empty storage.
    pub fn new(
        config: Option<MesherConfig>,
        culler: Option<FaceCuller>,
        model_db: Option<Arc<BakedModelDatabase>>,
    ) -> Self {
        Self {
            storage: VoxelStorage::new(),
            config: config.unwrap_or_default(),
            culler: culler.unwrap_or_default(),
            model_db,
            num_threads: None,
            section_mesh_cache: HashMap::new(),
            world_mesh: None,
            used_chunk_ids: Vec::new(),
            last_profile: None,
        }
    }

    /// Creates a `VoxelWorld` taking ownership of an existing `VoxelStorage`.
    pub fn from_storage(
        storage: VoxelStorage,
        config: Option<MesherConfig>,
        culler: Option<FaceCuller>,
        model_db: Option<Arc<BakedModelDatabase>>,
    ) -> Self {
        Self::from_storage_with_threads(storage, config, culler, model_db, None)
    }

    /// Creates a `VoxelWorld` taking ownership of an existing `VoxelStorage` with explicit thread configuration.
    pub fn from_storage_with_threads(
        storage: VoxelStorage,
        config: Option<MesherConfig>,
        culler: Option<FaceCuller>,
        model_db: Option<Arc<BakedModelDatabase>>,
        num_threads: Option<usize>,
    ) -> Self {
        Self {
            storage,
            config: config.unwrap_or_default(),
            culler: culler.unwrap_or_default(),
            model_db,
            num_threads,
            section_mesh_cache: HashMap::new(),
            world_mesh: None,
            used_chunk_ids: Vec::new(),
            last_profile: None,
        }
    }

    /// Sets the worker thread count for parallel meshing.
    pub fn set_num_threads(&mut self, num_threads: Option<usize>) {
        self.num_threads = num_threads;
    }

    /// Returns the configured worker thread count.
    pub fn num_threads(&self) -> Option<usize> {
        self.num_threads
    }

    /// Updates or replaces the prebaked model database.
    pub fn set_model_db(&mut self, model_db: Option<Arc<BakedModelDatabase>>) {
        self.model_db = model_db;
    }

    /// Updates mesher configuration.
    pub fn set_config(&mut self, config: MesherConfig) {
        self.config = config;
    }

    /// Sets the active 3D selection bounding box.
    /// If bounds changed, clears mesh cache to force coordinate re-centering.
    pub fn set_bounds(
        &mut self,
        min_x: i32,
        min_y: i32,
        min_z: i32,
        size_x: i32,
        size_y: i32,
        size_z: i32,
    ) -> bool {
        let bounds_changed = self
            .storage
            .set_bounds(min_x, min_y, min_z, size_x, size_y, size_z);
        let new_bounds = Some(([min_x, min_y, min_z], [size_x, size_y, size_z]));
        let bounds_differ = self.config.selection_bounds != new_bounds;
        if self.config.origin_centered {
            self.config.selection_bounds = new_bounds;
        }
        if bounds_changed || bounds_differ {
            self.section_mesh_cache.clear();
            self.world_mesh = None;
            self.used_chunk_ids.clear();
            self.storage.mark_all_sections_dirty();
        }
        bounds_changed
    }

    /// Returns the active or calculated bounding box: `(min_x, min_y, min_z, size_x, size_y, size_z)`.
    pub fn get_bounds(&self) -> (i32, i32, i32, i32, i32, i32) {
        self.storage.get_bounds()
    }

    /// Gets the blockstate identifier string at world coordinate `(x, y, z)`.
    pub fn get_block(&self, x: i32, y: i32, z: i32) -> &str {
        self.storage.get_block(x, y, z)
    }

    /// Sets the blockstate at world coordinate `(x, y, z)` and marks affected sections dirty.
    pub fn set_block(&mut self, x: i32, y: i32, z: i32, state: &str, biome: Option<&str>) -> bool {
        self.storage.set_block(x, y, z, state, biome)
    }

    /// Ingests a full snapshot packet and invalidates existing section meshes.
    pub fn set_full_snapshot(
        &mut self,
        min_x: i32,
        min_y: i32,
        min_z: i32,
        size_x: i32,
        size_y: i32,
        size_z: i32,
        palette: &[String],
        grid_indices: &[u16],
        biome_palette: Option<&[String]>,
        biome_indices: Option<&[u16]>,
    ) {
        self.storage.set_full_snapshot(
            min_x,
            min_y,
            min_z,
            size_x,
            size_y,
            size_z,
            palette,
            grid_indices,
            biome_palette,
            biome_indices,
        );
        if self.config.origin_centered {
            self.config.selection_bounds = Some(([min_x, min_y, min_z], [size_x, size_y, size_z]));
        }
        self.section_mesh_cache.clear();
        self.world_mesh = None;
        self.used_chunk_ids.clear();
    }

    /// Ingests a single 16x16x16 chunk section snapshot.
    pub fn set_section_snapshot(
        &mut self,
        sec_x: i32,
        sec_y: i32,
        sec_z: i32,
        start_x: i32,
        start_y: i32,
        start_z: i32,
        size_x: i32,
        size_y: i32,
        size_z: i32,
        palette: &[String],
        grid_indices: &[u16],
        biome_palette: Option<&[String]>,
        biome_indices: Option<&[u16]>,
    ) {
        self.storage.set_section_snapshot(
            sec_x,
            sec_y,
            sec_z,
            start_x,
            start_y,
            start_z,
            size_x,
            size_y,
            size_z,
            palette,
            grid_indices,
            biome_palette,
            biome_indices,
        );
    }

    /// Applies a batch of block delta modifications.
    pub fn apply_delta_update(
        &mut self,
        min_x: i32,
        min_y: i32,
        min_z: i32,
        changes: &[(i32, i32, i32, &str)],
    ) -> Vec<(i32, i32, i32, String, String)> {
        self.storage
            .apply_delta_update(min_x, min_y, min_z, changes)
    }

    /// Ingests chunk sections from an abstract `VoxelSource` provider (Anvil save,
    /// Litematica schematic, structure NBT, or procedural generator).
    pub fn ingest_from_source<S: VoxelSource + ?Sized>(
        &mut self,
        source: &mut S,
        sections: Option<&[IVec3]>,
    ) -> Result<usize, VoxelError> {
        crate::source::ingest_from_source(source, &mut self.storage, sections)
    }

    /// Exports manifest metadata (bounds, generation, section CRCs) as JSON.
    #[cfg(feature = "serde")]
    pub fn export_manifest_metadata(&self) -> serde_json::Value {
        self.storage.export_manifest_metadata()
    }

    /// Imports manifest metadata from JSON to restore persistent scene state.
    #[cfg(feature = "serde")]
    pub fn import_manifest_metadata(&mut self, data: &serde_json::Value) -> bool {
        let ok = self.storage.import_manifest_metadata(data);
        if ok {
            self.sync_selection_bounds();
        }
        ok
    }

    /// Synchronizes selection bounds if origin centering is enabled.
    fn sync_selection_bounds(&mut self) {
        if self.config.origin_centered {
            let (min_x, min_y, min_z, sz_x, sz_y, sz_z) = self.storage.get_bounds();
            if sz_x > 0 && sz_y > 0 && sz_z > 0 {
                let new_bounds = Some(([min_x, min_y, min_z], [sz_x, sz_y, sz_z]));
                if self.config.selection_bounds != new_bounds {
                    self.config.selection_bounds = new_bounds;
                    self.section_mesh_cache.clear();
                    self.world_mesh = None;
                    self.used_chunk_ids.clear();
                    self.storage.mark_all_sections_dirty();
                }
            }
        }
    }

    /// Returns a reference to the assembled unified world mesh, if available.
    pub fn get_world_mesh(&self) -> Option<&MeshData> {
        self.world_mesh.as_ref()
    }

    /// Returns a reference to the cached section meshes.
    pub fn get_section_cache(&self) -> &HashMap<IVec3, MeshData> {
        &self.section_mesh_cache
    }

    /// Returns the exact slice of Atlas Chunk IDs actually referenced by the current world mesh.
    pub fn used_chunk_ids(&self) -> &[u32] {
        &self.used_chunk_ids
    }

    /// Compacts `world_mesh` material indices into contiguous `0..N-1` and returns the
    /// mapping of original Atlas Chunk IDs.
    pub fn compact_world_mesh_materials(&mut self) -> Vec<u16> {
        if let Some(ref mut m) = self.world_mesh {
            let mapping = m.compact_materials();
            self.used_chunk_ids = mapping.iter().map(|&id| id as u32).collect();
            mapping
        } else {
            Vec::new()
        }
    }

    /// Extracts an unculled point cloud representation of the current world geometry.
    pub fn to_point_cloud(&self) -> crate::storage::VoxelPointCloud {
        self.storage.to_point_cloud(&self.config)
    }

    /// Reconstructs world storage from a point cloud and clears cached meshes.
    pub fn load_from_point_cloud(&mut self, point_cloud: &crate::storage::VoxelPointCloud) {
        self.storage.load_from_point_cloud(point_cloud);
        self.section_mesh_cache.clear();
        self.world_mesh = None;
        self.used_chunk_ids.clear();
        self.sync_selection_bounds();
    }

    /// Clears the cached section meshes and unified world mesh, and marks all storage sections dirty.
    ///
    /// Used when assets (Atlas, Models, Biomes) or mesher configurations change dynamically,
    /// ensuring subsequent remeshing rebuilds all chunk sections with the updated definitions.
    pub fn clear_cache(&mut self) {
        self.section_mesh_cache.clear();
        self.world_mesh = None;
        self.used_chunk_ids.clear();
        self.storage.mark_all_sections_dirty();
    }

    /// Hot-reloads asset databases (model DB, atlas address map, biome resolver)
    /// and purges all cached section meshes so that future meshing immediately reflects new assets.
    pub fn hot_reload_assets(
        &mut self,
        model_db: Option<Arc<BakedModelDatabase>>,
        atlas_map: Option<Arc<mtk_texture::atlas::AtlasAddressMap>>,
        biome_resolver: Option<Arc<mtk_material::BiomeResolver>>,
    ) {
        self.model_db = model_db;
        self.config.atlas_address_map = atlas_map;
        self.config.biome_resolver = biome_resolver;
        self.clear_cache();
    }

    /// Clears all sections, cache, and world mesh.
    pub fn clear(&mut self) {
        self.storage.clear();
        self.section_mesh_cache.clear();
        self.world_mesh = None;
        self.used_chunk_ids.clear();
    }
}

impl Default for VoxelWorld {
    fn default() -> Self {
        Self::new(None, None, None)
    }
}

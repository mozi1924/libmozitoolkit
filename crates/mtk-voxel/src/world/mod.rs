//! # Voxel World Scene and Pipeline
//!
//! Provides the authoritative high-level `VoxelWorld` container integrating
//! sparse voxel storage, ambient occlusion, face culling, prebaked models,
//! multi-threaded Section meshing, incremental dirty remeshing, Section mesh
//! caching, unified world mesh assembly with vertex welding, and used material
//! chunk tracking.
//!
//! Fully host-agnostic and decoupled from WebSocket network synchronization.

use std::collections::HashMap;
use std::sync::Arc;

use glam::IVec3;
use mtk_core::mesh::MeshData;
use mtk_core::progress::{ProgressCallback, ProgressReport};
use mtk_cull::FaceCuller;
use mtk_model::baked::BakedModelDatabase;

use crate::mesher::{DeltaMesher, SectionMesher};
use crate::source::VoxelSource;
use crate::types::{MesherConfig, VoxelError};

// Re-export storage world items for backward compatibility
pub use crate::storage::world::*;

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
    /// Whether to merge all chunk sections into a single, seamless world mesh.
    pub unified_mesh: bool,
    /// Cache of section meshes: `(sx, sy, sz) -> MeshData`.
    section_mesh_cache: HashMap<IVec3, MeshData>,
    /// Assembled unified world mesh buffer.
    world_mesh: Option<MeshData>,
    /// Unique Atlas Chunk IDs actually present on the active world mesh geometry.
    used_chunk_ids: Vec<u32>,
}

impl VoxelWorld {
    /// Creates a new `VoxelWorld` with empty storage.
    pub fn new(
        config: Option<MesherConfig>,
        culler: Option<FaceCuller>,
        model_db: Option<Arc<BakedModelDatabase>>,
        unified_mesh: bool,
    ) -> Self {
        Self {
            storage: VoxelStorage::new(),
            config: config.unwrap_or_default(),
            culler: culler.unwrap_or_default(),
            model_db,
            num_threads: None,
            unified_mesh,
            section_mesh_cache: HashMap::new(),
            world_mesh: None,
            used_chunk_ids: Vec::new(),
        }
    }

    /// Creates a `VoxelWorld` taking ownership of an existing `VoxelStorage`.
    pub fn from_storage(
        storage: VoxelStorage,
        config: Option<MesherConfig>,
        culler: Option<FaceCuller>,
        model_db: Option<Arc<BakedModelDatabase>>,
        unified_mesh: bool,
    ) -> Self {
        Self::from_storage_with_threads(storage, config, culler, model_db, unified_mesh, None)
    }

    /// Creates a `VoxelWorld` taking ownership of an existing `VoxelStorage` with explicit thread configuration.
    pub fn from_storage_with_threads(
        storage: VoxelStorage,
        config: Option<MesherConfig>,
        culler: Option<FaceCuller>,
        model_db: Option<Arc<BakedModelDatabase>>,
        unified_mesh: bool,
        num_threads: Option<usize>,
    ) -> Self {
        Self {
            storage,
            config: config.unwrap_or_default(),
            culler: culler.unwrap_or_default(),
            model_db,
            num_threads,
            unified_mesh,
            section_mesh_cache: HashMap::new(),
            world_mesh: None,
            used_chunk_ids: Vec::new(),
        }
    }

    /// Loads the canonical embedded Minecraft debug world into a new `VoxelWorld`.
    pub fn create_debug_world(
        config: Option<MesherConfig>,
        culler: Option<FaceCuller>,
        model_db: Option<Arc<BakedModelDatabase>>,
        unified_mesh: bool,
    ) -> Result<Self, VoxelError> {
        Self::create_debug_world_with_threads(config, culler, model_db, unified_mesh, None)
    }

    /// Loads the canonical embedded Minecraft debug world into a new `VoxelWorld` with explicit thread configuration.
    pub fn create_debug_world_with_threads(
        config: Option<MesherConfig>,
        culler: Option<FaceCuller>,
        model_db: Option<Arc<BakedModelDatabase>>,
        unified_mesh: bool,
        num_threads: Option<usize>,
    ) -> Result<Self, VoxelError> {
        let storage = if let Some(ref db) = model_db {
            if !db.is_empty() {
                VoxelStorage::create_debug_world_from_model_db(db)?
            } else {
                VoxelStorage::create_debug_world()?
            }
        } else {
            VoxelStorage::create_debug_world()?
        };
        Ok(Self::from_storage_with_threads(
            storage,
            config,
            culler,
            model_db,
            unified_mesh,
            num_threads,
        ))
    }

    /// Creates a debug `VoxelWorld` from an explicit list of blockstate strings.
    pub fn create_debug_world_from_states<S: AsRef<str>>(
        states: &[S],
        config: Option<MesherConfig>,
        culler: Option<FaceCuller>,
        model_db: Option<Arc<BakedModelDatabase>>,
        unified_mesh: bool,
        num_threads: Option<usize>,
    ) -> Result<Self, VoxelError> {
        let storage = VoxelStorage::create_debug_world_from_states(states)?;
        Ok(Self::from_storage_with_threads(
            storage,
            config,
            culler,
            model_db,
            unified_mesh,
            num_threads,
        ))
    }

    /// Creates a debug `VoxelWorld` by enumerating blockstates across a `ResourcePackStack`.
    pub fn create_debug_world_from_pack_stack(
        stack: &mtk_resource::ResourcePackStack,
        config: Option<MesherConfig>,
        culler: Option<FaceCuller>,
        model_db: Option<Arc<BakedModelDatabase>>,
        unified_mesh: bool,
        num_threads: Option<usize>,
    ) -> Result<Self, VoxelError> {
        let storage = VoxelStorage::create_debug_world_from_pack_stack(stack)?;
        Ok(Self::from_storage_with_threads(
            storage,
            config,
            culler,
            model_db,
            unified_mesh,
            num_threads,
        ))
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

    /// Toggles single unified world mesh mode vs individual section mode.
    pub fn set_unified_mesh(&mut self, unified_mesh: bool) {
        self.unified_mesh = unified_mesh;
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

    /// Re-assembles and welds the unified world mesh from the current section cache without cloning.
    /// In incremental mode (`full_weld = false`), uses fast zero-copy buffer merge since individual
    /// sections are already internally welded by SectionMesher. Full spatial welding is applied on
    /// global rebuilds (`rebuild_all`).
    fn assemble_world_mesh(&mut self, full_weld: bool) {
        if self.unified_mesh {
            let meshes: Vec<&MeshData> = self.section_mesh_cache.values().collect();
            let merged = if self.config.weld_vertices && full_weld {
                MeshData::merge_welded_sections(&meshes, 1e-4)
            } else {
                MeshData::merge_all_refs(&meshes)
            };
            self.used_chunk_ids = merged
                .used_materials()
                .into_iter()
                .map(|id| id as u32)
                .collect();
            self.world_mesh = Some(merged);
        }
    }

    /// Re-meshes all non-empty sections in parallel and updates the section cache.
    pub fn rebuild_all(&mut self) -> Result<&MeshData, VoxelError> {
        self.rebuild_all_with_progress(None)
    }

    /// Re-meshes all non-empty sections in parallel with progress reporting and updates the section cache.
    pub fn rebuild_all_with_progress(
        &mut self,
        progress: Option<ProgressCallback>,
    ) -> Result<&MeshData, VoxelError> {
        self.sync_selection_bounds();

        let non_empty = self.storage.get_all_non_empty_sections();
        if non_empty.is_empty() {
            self.section_mesh_cache.clear();
            self.world_mesh = Some(MeshData::new());
            self.used_chunk_ids.clear();
            if let Some(cb) = progress {
                cb(ProgressReport::new(
                    "meshing_sections",
                    0,
                    0,
                    "No sections to mesh",
                ));
            }
            return Ok(self.world_mesh.as_ref().unwrap());
        }

        #[cfg(feature = "parallel")]
        let padded_sections: Vec<_> = {
            use rayon::prelude::*;
            mtk_core::constants::concurrency::execute_parallel(self.num_threads, || {
                non_empty
                    .par_iter()
                    .map(|&coord| self.storage.get_section_padded_array(coord))
                    .collect()
            })
            .map_err(VoxelError::ThreadPoolError)?
        };

        #[cfg(not(feature = "parallel"))]
        let padded_sections: Vec<_> = non_empty
            .iter()
            .map(|&coord| self.storage.get_section_padded_array(coord))
            .collect();

        let db_opt = self.model_db.as_deref();
        let model_lookup = |state: &str| -> crate::mesher::ModelSource {
            if let Some(db) = db_opt {
                if let Some(group) = db.get_variant_group(state) {
                    return crate::mesher::ModelSource::Variant(group);
                }
                if let Some(model) = db.get(state) {
                    return crate::mesher::ModelSource::Single(model);
                }
            }
            crate::mesher::ModelSource::None
        };

        let results = SectionMesher::mesh_sections_parallel_with_source_and_progress(
            &padded_sections,
            &self.culler,
            model_lookup,
            &self.config,
            self.num_threads,
            progress,
        )?;

        self.section_mesh_cache.clear();
        for (coord, mesh) in results {
            if !mesh.is_empty() {
                self.section_mesh_cache.insert(coord, mesh);
            }
        }

        if let Some(cb) = progress {
            let total = non_empty.len();
            cb(ProgressReport::new(
                "assembling_world_mesh",
                total,
                total,
                format!("Assembled world mesh from {} chunks", total),
            ));
        }

        self.assemble_world_mesh(true);
        self.storage.clear_dirty_sections();

        Ok(self.world_mesh.as_ref().unwrap())
    }

    /// Incrementally rebuilds only the modified/dirty sections and updates the section cache.
    /// Returns the list of updated `(section_coord, MeshData)`.
    pub fn rebuild_dirty(&mut self) -> Result<Vec<(IVec3, MeshData)>, VoxelError> {
        if self.storage.dirty_sections.is_empty() {
            return Ok(Vec::new());
        }

        self.sync_selection_bounds();

        let db_opt = self.model_db.as_deref();
        let model_lookup = |state: &str| -> crate::mesher::ModelSource {
            if let Some(db) = db_opt {
                if let Some(group) = db.get_variant_group(state) {
                    return crate::mesher::ModelSource::Variant(group);
                }
                if let Some(model) = db.get(state) {
                    return crate::mesher::ModelSource::Single(model);
                }
            }
            crate::mesher::ModelSource::None
        };

        let rebuilt = DeltaMesher::rebuild_dirty_sections_with_source(
            &mut self.storage,
            &self.culler,
            model_lookup,
            &self.config,
        );

        for (coord, mesh) in &rebuilt {
            if mesh.is_empty() {
                self.section_mesh_cache.remove(coord);
            } else {
                self.section_mesh_cache.insert(*coord, mesh.clone());
            }
        }

        self.assemble_world_mesh(false);

        Ok(rebuilt)
    }

    /// Rebuilds a single section (e.g. on repair request or targeted update).
    pub fn rebuild_single_section(&mut self, sec_coord: IVec3) -> MeshData {
        self.sync_selection_bounds();

        let padded = self.storage.get_section_padded_array(sec_coord);
        let db_opt = self.model_db.as_deref();
        let model_lookup = |state: &str| -> crate::mesher::ModelSource {
            if let Some(db) = db_opt {
                if let Some(group) = db.get_variant_group(state) {
                    return crate::mesher::ModelSource::Variant(group);
                }
                if let Some(model) = db.get(state) {
                    return crate::mesher::ModelSource::Single(model);
                }
            }
            crate::mesher::ModelSource::None
        };

        let mesh = SectionMesher::mesh_section_with_source(
            &padded,
            &self.culler,
            model_lookup,
            &self.config,
        );
        if mesh.is_empty() {
            self.section_mesh_cache.remove(&sec_coord);
        } else {
            self.section_mesh_cache.insert(sec_coord, mesh.clone());
        }

        self.assemble_world_mesh(false);
        mesh
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
        Self::new(None, None, None, true)
    }
}

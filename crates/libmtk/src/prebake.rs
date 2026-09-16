//! # Headless Asset Prebaking and Cache Engine
//!
//! Performs unified end-to-end asset precompilation (Atlas stitching, Standalone PBR alignment,
//! and multi-threaded Model baking) across a ResourcePackStack directly to persistent cache.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[cfg(feature = "parallel")]
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use mtk_material::BiomeResolver;
use mtk_model::{BakedModelDatabase, BlockModelJson, BlockStateDefinition, ModelBaker};
use mtk_resource::{AtlasCategory, ResourceLocation, ResourcePackStack};
use mtk_texture::{AtlasBuilder, AtlasBuilderConfig, StandaloneBuilder, StandaloneConfig};

use crate::MtkError;

/// Format version for compiled cache compatibility checking.
pub const ASSET_CACHE_FORMAT_VERSION: u32 = 1;

/// Configuration for unified asset precompilation.
#[derive(Debug, Clone)]
pub struct PrecompileConfig {
    pub max_atlas_width: u32,
    pub max_atlas_height: u32,
    pub atlas_category: String,
    pub compile_atlas: bool,
    pub compile_standalone: bool,
    pub compile_models: bool,
}

impl Default for PrecompileConfig {
    fn default() -> Self {
        Self {
            max_atlas_width: 4096,
            max_atlas_height: 4096,
            atlas_category: "all".to_string(),
            compile_atlas: true,
            compile_standalone: true,
            compile_models: true,
        }
    }
}

/// Statistics and summary of a precompilation run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrecompileResult {
    pub success: bool,
    pub pack_count: usize,
    pub atlas_chunks: usize,
    pub standalone_textures: usize,
    pub baked_models: usize,
    pub fingerprint: String,
    pub cache_dir: String,
}

/// Metadata manifest stored in `cache_manifest.json` for integrity and invalidation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheManifest {
    pub format_version: u32,
    pub fingerprint: String,
    pub pack_count: usize,
    pub atlas_chunks: usize,
    pub standalone_textures: usize,
    pub baked_models: usize,
    pub created_at_epoch_secs: u64,
}

impl CacheManifest {
    /// Read manifest from cache directory.
    pub fn read_from_dir(cache_dir: &Path) -> Option<Self> {
        let path = cache_dir.join("cache_manifest.json");
        let bytes = fs::read(path).ok()?;
        serde_json::from_slice(&bytes).ok()
    }

    /// Check if manifest matches a given stack fingerprint and format version.
    pub fn is_valid_for(&self, fingerprint: &str) -> bool {
        self.format_version == ASSET_CACHE_FORMAT_VERSION && self.fingerprint == fingerprint
    }
}

/// Executes unified end-to-end asset precompilation directly to disk.
pub fn precompile_all_assets(
    stack: &ResourcePackStack,
    cache_dir: impl AsRef<Path>,
    config: &PrecompileConfig,
) -> Result<PrecompileResult, MtkError> {
    let base_path = cache_dir.as_ref();
    let atlas_dir = base_path.join("atlas");
    let standalone_dir = base_path.join("standalone");
    let models_dir = base_path.join("models");

    let colormaps_dir = base_path.join("colormaps");

    fs::create_dir_all(&atlas_dir)?;
    fs::create_dir_all(&standalone_dir)?;
    fs::create_dir_all(&models_dir)?;
    fs::create_dir_all(&colormaps_dir)?;

    // 0. Discover Biome Tinting, Model tintindex, and Overlay Pairs (Save biome_mapping.json)
    let mut biome_resolver = BiomeResolver::new();
    biome_resolver.load_from_pack_stack(stack);
    if let Ok(biome_json) = biome_resolver.to_json() {
        let _ = fs::write(base_path.join("biome_mapping.json"), biome_json.as_bytes());
        let _ = fs::write(atlas_dir.join("biome_mapping.json"), biome_json.as_bytes());
    }

    // Extract standard vanilla & custom colormaps
    for cm_name in &["grass", "foliage", "dry_foliage"] {
        let asset_path = format!("assets/minecraft/textures/colormap/{}.png", cm_name);
        if let Some(bytes) = stack.open_asset_raw(&asset_path) {
            let _ = fs::write(colormaps_dir.join(format!("{}.png", cm_name)), bytes);
        }
    }

    let mut chunk_count = 0;
    let mut sa_count = 0;
    let mut baked_count = 0;

    // 1. Atlas Baking & File Persistence (Multi-Category)
    if config.compile_atlas {
        let mut def_list: Vec<(String, mtk_resource::AtlasDefinition)> = Vec::new();
        let mut seen_cats = std::collections::HashSet::new();

        if config.atlas_category != "all" && config.atlas_category != "blocks" && !config.atlas_category.is_empty() {
            let cat = AtlasCategory::parse(&config.atlas_category);
            let def = stack.load_atlas_category(&cat);
            def_list.push((cat.as_str().to_string(), def));
        } else {
            // 1. Discovered custom atlases from active packs
            for loc in stack.list_all_atlas_locations() {
                if let Ok(def) = stack.load_atlas_definition(&loc) {
                    let cat_name = loc.path.clone();
                    if seen_cats.insert(cat_name.clone()) {
                        def_list.push((cat_name, def));
                    }
                }
            }

            // 2. Standard vanilla Atlas categories
            for standard_cat in &AtlasCategory::ALL_STANDARD {
                let cat_name = standard_cat.as_str().to_string();
                if seen_cats.insert(cat_name.clone()) {
                    let def = stack.load_atlas_category(standard_cat);
                    def_list.push((cat_name, def));
                }
            }
        }

        let atlas_cfg = AtlasBuilderConfig {
            max_width: config.max_atlas_width,
            max_height: config.max_atlas_height,
            mip_level: 0,
            padding: 0,
        };
        let atlas_builder = AtlasBuilder::new(atlas_cfg);
        let baked_atlas = atlas_builder.build_categories(stack, &def_list)?;
        chunk_count = baked_atlas.chunks.len();

        let mapping_json = baked_atlas.address_map.to_json()?;
        fs::write(atlas_dir.join("atlas_mapping.json"), mapping_json)?;

        for chunk in &baked_atlas.chunks {
            let stem = chunk.file_stem();
            let albedo_bytes = chunk.albedo.to_png_bytes()?;
            fs::write(atlas_dir.join(format!("{}.png", stem)), albedo_bytes)?;

            if let Some(ref normal) = chunk.normal {
                let normal_bytes = normal.to_png_bytes()?;
                fs::write(atlas_dir.join(format!("{}_n.png", stem)), normal_bytes)?;
            }

            if let Some(ref specular) = chunk.specular {
                let spec_bytes = specular.to_png_bytes()?;
                fs::write(atlas_dir.join(format!("{}_s.png", stem)), spec_bytes)?;
            }

            if let Some(ref overlay) = chunk.overlay {
                let overlay_bytes = overlay.to_png_bytes()?;
                fs::write(atlas_dir.join(format!("{}_overlay.png", stem)), overlay_bytes)?;
            }
        }
    }

    // 2. Standalone Baking
    if config.compile_standalone {
        let sa_cfg = StandaloneConfig::default();
        let sa_builder = StandaloneBuilder::new(sa_cfg);
        let sa_res = sa_builder.build_to_dir(stack, &standalone_dir)?;
        sa_count = sa_res.texture_count;
    }

    // 3. Full-Scale Model Baking & Bincode Persistence
    if config.compile_models {
        let model_db = prebake_all_models(stack)?;
        baked_count = model_db.len();
        let bin_bytes = model_db.to_bincode()?;
        fs::write(models_dir.join("models.bin"), bin_bytes)?;
    }

    // 4. Save Cache Manifest
    let fingerprint = stack.compute_stack_fingerprint();
    let epoch_now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let manifest = CacheManifest {
        format_version: ASSET_CACHE_FORMAT_VERSION,
        fingerprint: fingerprint.clone(),
        pack_count: stack.len(),
        atlas_chunks: chunk_count,
        standalone_textures: sa_count,
        baked_models: baked_count,
        created_at_epoch_secs: epoch_now,
    };
    let manifest_json = serde_json::to_string_pretty(&manifest)?;
    fs::write(base_path.join("cache_manifest.json"), manifest_json)?;

    Ok(PrecompileResult {
        success: true,
        pack_count: stack.len(),
        atlas_chunks: chunk_count,
        standalone_textures: sa_count,
        baked_models: baked_count,
        fingerprint,
        cache_dir: base_path.to_string_lossy().to_string(),
    })
}

/// Prebakes all blockstates discovered across all active resource packs in the stack.
pub fn prebake_all_models(
    stack: &ResourcePackStack,
) -> Result<BakedModelDatabase, MtkError> {
    let blockstate_locs = stack.list_all_blockstate_locations();

    // 1. Preload all model JSONs across packs into a fast lookup map
    let mut model_cache: HashMap<String, BlockModelJson> = HashMap::new();
    for pack in stack.packs().iter().rev() {
        for file in pack.list_files("assets/") {
            if file.ends_with(".json") && file.contains("/models/") {
                if let Some(bytes) = pack.open(&file) {
                    if let Ok(model) = serde_json::from_slice::<BlockModelJson>(&bytes) {
                        if let Some(loc) = ResourceLocation::from_asset_path(&file, "models", "json") {
                            let canon = loc.as_string();
                            model_cache.insert(canon.clone(), model.clone());
                            if loc.namespace == "minecraft" {
                                model_cache.insert(loc.path.clone(), model.clone());
                            }
                        }
                    }
                }
            }
        }
    }

    // Helper closure to look up models with unified candidate paths
    let get_model = |model_id: &str| -> Option<BlockModelJson> {
        if let Some(m) = model_cache.get(model_id) {
            return Some(m.clone());
        }
        for path in ResourceLocation::model_candidate_asset_paths(model_id) {
            if let Some(loc) = ResourceLocation::from_asset_path(&path, "models", "json") {
                if let Some(m) = model_cache.get(&loc.as_string()).or_else(|| model_cache.get(&loc.path)) {
                    return Some(m.clone());
                }
            }
            if let Some(bytes) = stack.open_asset_raw(&path) {
                if let Ok(m) = serde_json::from_slice::<BlockModelJson>(&bytes) {
                    return Some(m);
                }
            }
        }
        None
    };

    // 2. Pre-load BlockStateDefinitions for all discovered locations
    let mut blockstate_defs: Vec<(ResourceLocation, BlockStateDefinition)> = Vec::new();
    for loc in &blockstate_locs {
        let path = loc.to_asset_path("blockstates", "json");
        if let Some(bytes) = stack.open_asset_raw(&path) {
            if let Ok(def) = serde_json::from_slice::<BlockStateDefinition>(&bytes) {
                blockstate_defs.push((loc.clone(), def));
            }
        }
    }

    // 3. Bake all states concurrently
    #[cfg(feature = "parallel")]
    let baked_pairs: Vec<(String, mtk_model::BakedModel)> = blockstate_defs
        .par_iter()
        .flat_map(|(loc, def)| {
            let states = def.enumerate_all_states(&loc.as_string());
            let mut local_baker = ModelBaker::new();
            let mut pairs = Vec::new();
            for state_str in states {
                if let Ok(baked) = local_baker.bake_blockstate(&state_str, Some(def), |id| get_model(id)) {
                    pairs.push((state_str, baked));
                }
            }
            pairs
        })
        .collect();

    #[cfg(not(feature = "parallel"))]
    let baked_pairs: Vec<(String, mtk_model::BakedModel)> = blockstate_defs
        .iter()
        .flat_map(|(loc, def)| {
            let states = def.enumerate_all_states(&loc.as_string());
            let mut local_baker = ModelBaker::new();
            let mut pairs = Vec::new();
            for state_str in states {
                if let Ok(baked) = local_baker.bake_blockstate(&state_str, Some(def), |id| get_model(id)) {
                    pairs.push((state_str, baked));
                }
            }
            pairs
        })
        .collect();

    let mut db = BakedModelDatabase::new();
    for (state_str, baked) in baked_pairs {
        db.insert(state_str, baked);
    }

    Ok(db)
}


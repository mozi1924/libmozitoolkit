//! End-to-end unified precompilation engine producing `.mtkcache` binary packages.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use mtk_core::progress::{ProgressCallback, ProgressReport, ProgressThrottler};
use mtk_material::BiomeResolver;
use mtk_package::{ChunkWriteOptions, MtkPackageWriter, PackageProfile};
use mtk_resource::{AtlasCategory, ResourcePackStack};
use mtk_texture::{AtlasBuilder, AtlasBuilderConfig, StandaloneBuilder, StandaloneConfig};

use crate::prebake::models::prebake_all_models_with_progress;
use crate::prebake::reader::{
    fingerprint_str_to_bytes16, CacheManifest, ASSET_CACHE_FORMAT_VERSION,
};
use crate::MtkError;

/// Configuration for unified asset precompilation.
#[derive(Debug, Clone)]
pub struct PrecompileConfig {
    pub max_atlas_width: u32,
    pub max_atlas_height: u32,
    pub atlas_category: String,
    pub compile_atlas: bool,
    pub compile_standalone: bool,
    pub compile_models: bool,
    pub num_threads: Option<usize>,
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
            num_threads: None,
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
    pub package_path: String,
    pub cache_dir: String,
}

/// Executes unified end-to-end asset precompilation directly to `.mtkcache` with progress reporting.
pub fn precompile_all_assets_with_progress(
    stack: &ResourcePackStack,
    cache_dir_or_file: impl AsRef<Path>,
    config: &PrecompileConfig,
    progress_callback: Option<ProgressCallback>,
) -> Result<PrecompileResult, MtkError> {
    let base_path = cache_dir_or_file.as_ref();
    mtk_core::constants::concurrency::execute_parallel(config.num_threads, || {
        precompile_all_assets_inner(stack, base_path, config, progress_callback)
    })
    .map_err(MtkError::ThreadPool)?
}

/// Executes unified end-to-end asset precompilation directly to `.mtkcache`.
pub fn precompile_all_assets(
    stack: &ResourcePackStack,
    cache_dir_or_file: impl AsRef<Path>,
    config: &PrecompileConfig,
) -> Result<PrecompileResult, MtkError> {
    precompile_all_assets_with_progress(stack, cache_dir_or_file, config, None)
}

fn precompile_all_assets_inner(
    stack: &ResourcePackStack,
    base_path: &Path,
    config: &PrecompileConfig,
    progress_callback: Option<ProgressCallback>,
) -> Result<PrecompileResult, MtkError> {
    let fingerprint = stack.compute_stack_fingerprint();

    // Determine target .mtkcache file path
    let package_path = if base_path.extension().is_some_and(|ext| ext == "mtkcache") {
        if let Some(parent) = base_path.parent() {
            fs::create_dir_all(parent)?;
        }
        base_path.to_path_buf()
    } else {
        fs::create_dir_all(base_path)?;
        base_path.join(format!("{}.mtkcache", fingerprint))
    };

    let fp_bytes = fingerprint_str_to_bytes16(&fingerprint);
    let mut writer = MtkPackageWriter::create(&package_path, PackageProfile::AssetCache, fp_bytes)?;

    // 0. Biome Tinting & Colormaps
    if let Some(cb) = progress_callback {
        cb(ProgressReport::new(
            "prebake_biome",
            0,
            1,
            "Extracting biome mappings and colormaps...",
        ));
    }

    let mut biome_resolver = BiomeResolver::new();
    biome_resolver.load_from_pack_stack(stack);
    let biome_json = biome_resolver.to_json()?;
    writer.add_str_chunk(
        *b"BIOM",
        "biome/mapping",
        &biome_json,
        &ChunkWriteOptions::zstd_fast(),
    )?;

    for cm_name in &["grass", "foliage", "dry_foliage"] {
        let asset_path = format!("assets/minecraft/textures/colormap/{}.png", cm_name);
        if let Some(bytes) = stack.open_asset_raw(&asset_path) {
            writer.add_chunk(
                *b"BIOM",
                format!("biome/colormap/{}", cm_name),
                &bytes,
                &ChunkWriteOptions::raw(),
            )?;
        }
    }

    if let Some(cb) = progress_callback {
        cb(ProgressReport::new(
            "prebake_biome",
            1,
            1,
            "Extracted biome mappings and colormaps",
        ));
    }

    let mut chunk_count = 0;
    let mut sa_count = 0;
    let mut baked_count = 0;
    let mut baked_atlas_opt = None;

    // 1. Atlas Baking & Packaging (Raw PNG streams, Zstd for address map)
    if config.compile_atlas {
        if let Some(cb) = progress_callback {
            cb(ProgressReport::new(
                "prebake_atlas",
                0,
                1,
                "Packing sprite textures into atlases...",
            ));
        }

        let mut def_list: Vec<(String, mtk_resource::AtlasDefinition)> = Vec::new();
        let mut seen_cats = std::collections::HashSet::new();

        if config.atlas_category != "all"
            && config.atlas_category != "blocks"
            && !config.atlas_category.is_empty()
        {
            let cat = AtlasCategory::parse(&config.atlas_category);
            let def = stack.load_atlas_category(&cat);
            def_list.push((cat.as_str().to_string(), def));
        } else {
            for loc in stack.list_all_atlas_locations() {
                if let Ok(def) = stack.load_atlas_definition(&loc) {
                    let cat_name = loc.path.clone();
                    if seen_cats.insert(cat_name.clone()) {
                        def_list.push((cat_name, def));
                    }
                }
            }
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
        writer.add_str_chunk(
            *b"ATLS",
            "atlas/mapping",
            &mapping_json,
            &ChunkWriteOptions::zstd_fast(),
        )?;

        let atlas_throttler =
            ProgressThrottler::new("prebake_atlas", chunk_count.max(1), progress_callback)
                .with_step(1)
                .with_prefix("Packaging atlas chunk textures");

        for chunk in &baked_atlas.chunks {
            let stem = chunk.file_stem();
            let albedo_bytes = chunk.albedo.to_png_bytes()?;
            writer.add_chunk(
                *b"ATLS",
                format!("atlas/textures/{}.png", stem),
                &albedo_bytes,
                &ChunkWriteOptions::raw(),
            )?;

            if let Some(ref normal) = chunk.normal {
                let normal_bytes = normal.to_png_bytes()?;
                writer.add_chunk(
                    *b"ATLS",
                    format!("atlas/textures/{}_n.png", stem),
                    &normal_bytes,
                    &ChunkWriteOptions::raw(),
                )?;
            }

            if let Some(ref specular) = chunk.specular {
                let spec_bytes = specular.to_png_bytes()?;
                writer.add_chunk(
                    *b"ATLS",
                    format!("atlas/textures/{}_s.png", stem),
                    &spec_bytes,
                    &ChunkWriteOptions::raw(),
                )?;
            }

            if let Some(ref overlay) = chunk.overlay {
                let overlay_bytes = overlay.to_png_bytes()?;
                writer.add_chunk(
                    *b"ATLS",
                    format!("atlas/textures/{}_overlay.png", stem),
                    &overlay_bytes,
                    &ChunkWriteOptions::raw(),
                )?;
            }
            atlas_throttler.inc();
        }

        baked_atlas_opt = Some(baked_atlas);
    }

    // 2. Standalone Baking (Raw PNG streams, Zstd for mapping table)
    if config.compile_standalone {
        if let Some(cb) = progress_callback {
            cb(ProgressReport::new(
                "prebake_standalone",
                0,
                1,
                "Aligning standalone PBR textures...",
            ));
        }

        let sa_cfg = StandaloneConfig::default();
        let sa_builder = StandaloneBuilder::new(sa_cfg);
        let (sa_mapping, sa_files) = sa_builder.build_records(stack)?;
        sa_count = sa_mapping.texture_count;

        let sa_mapping_json = serde_json::to_string(&sa_mapping)?;
        writer.add_str_chunk(
            *b"TXTR",
            "standalone/mapping",
            &sa_mapping_json,
            &ChunkWriteOptions::zstd_fast(),
        )?;

        for (rel_path, bytes) in &sa_files {
            writer.add_chunk(
                *b"TXTR",
                format!("standalone/{}", rel_path),
                bytes,
                &ChunkWriteOptions::raw(),
            )?;
        }

        if let Some(cb) = progress_callback {
            cb(ProgressReport::new(
                "prebake_standalone",
                sa_count,
                sa_count.max(1),
                format!("Packaged {} standalone textures into container", sa_count),
            ));
        }
    }

    // 3. Full-Scale Model Baking & Bincode Packaging (Zstd compressed)
    if config.compile_models {
        let atlas_map = baked_atlas_opt.as_ref().map(|a| &a.address_map);
        let model_db = prebake_all_models_with_progress(stack, atlas_map, progress_callback)?;
        baked_count = model_db.len();
        let bin_bytes = model_db.to_bincode()?;
        writer.add_chunk(
            *b"MODL",
            "models/database",
            &bin_bytes,
            &ChunkWriteOptions::zstd_fast(),
        )?;
    }

    // 4. Save Cache Manifest inside package
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
    writer.add_json_chunk(
        *b"META",
        "manifest",
        &manifest,
        &ChunkWriteOptions::zstd_fast(),
    )?;

    // 5. Finalize container
    writer.finish()?;

    if let Some(cb) = progress_callback {
        cb(ProgressReport::new(
            "prebake_manifest",
            1,
            1,
            "Saved .mtkcache binary package and completed precompilation",
        ));
    }

    let cache_dir_str = if base_path.is_file() {
        base_path
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| ".".to_string())
    } else {
        base_path.to_string_lossy().to_string()
    };

    Ok(PrecompileResult {
        success: true,
        pack_count: stack.len(),
        atlas_chunks: chunk_count,
        standalone_textures: sa_count,
        baked_models: baked_count,
        fingerprint,
        package_path: package_path.to_string_lossy().to_string(),
        cache_dir: cache_dir_str,
    })
}

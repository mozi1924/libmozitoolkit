//! # Standalone Material Asset Library Builder
//!
//! Precompiles standalone single-block textures with multi-channel PBR frame alignment,
//! UV scaling metadata, and deterministic fallback material into an atomic cache directory.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(feature = "parallel")]
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;

use mtk_resource::{ResourceLocation, ResourcePackStack};
use crate::error::TextureError;
use crate::image::RgbaBuffer;
use super::aligner::{
    align_standalone_channels, ChannelData, ChannelType, StandaloneAnimationMeta,
};

pub const STANDALONE_FORMAT_VERSION: u32 = 2;

/// Configuration options for building the standalone asset library.
#[derive(Debug, Clone, Default)]
pub struct StandaloneConfig {
    /// Optional hash identifier representing the pack stack state.
    pub stack_hash: Option<String>,
    /// Optional category or path prefix filter (e.g. `"block"`).
    pub filter_prefix: Option<String>,
}

/// Relative file paths for a single standalone texture entry's channels.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StandaloneFilePaths {
    pub albedo: Option<String>,
    pub normal: Option<String>,
    pub specular: Option<String>,
    pub overlay: Option<String>,
}

/// A single texture record inside `standalone_mapping.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StandaloneTextureRecord {
    pub namespace: String,
    pub texture_name: String,
    pub texture_key: String,
    pub canonical_key: String,
    pub files: StandaloneFilePaths,
    pub is_animated: bool,
    pub animation: Option<StandaloneAnimationMeta>,
    pub tint_info: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_fallback: Option<bool>,
}

/// The root data structure of `standalone_mapping.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StandaloneMapping {
    pub format_version: u32,
    pub stack_hash: String,
    pub texture_count: usize,
    pub textures: HashMap<String, StandaloneTextureRecord>,
    pub aliases: HashMap<String, String>,
}

/// Output summary of a completed standalone precompilation run.
#[derive(Debug, Clone)]
pub struct StandaloneResult {
    pub mapping_path: PathBuf,
    pub output_dir: PathBuf,
    pub texture_count: usize,
    pub format_version: u32,
}

/// High-performance Standalone Material Asset Library Generator.
#[derive(Debug, Clone, Default)]
pub struct StandaloneBuilder {
    pub config: StandaloneConfig,
}

impl StandaloneBuilder {
    pub fn new(config: StandaloneConfig) -> Self {
        Self { config }
    }

    /// Precompiles the entire resource pack stack into the target `output_dir`.
    pub fn build_to_dir(
        &self,
        stack: &ResourcePackStack,
        output_dir: impl AsRef<Path>,
    ) -> Result<StandaloneResult, TextureError> {
        let output_path = output_dir.as_ref().to_path_buf();
        let parent_dir = output_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        fs::create_dir_all(&parent_dir)?;

        let pid = std::process::id();
        let rand_suffix: u32 = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| (d.as_nanos() & 0xFFFF_FFFF) as u32)
            .unwrap_or(12345);

        let staging_name = format!(
            "{}_staging_{}_{:08x}",
            output_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("standalone"),
            pid,
            rand_suffix
        );
        let staging_dir = parent_dir.join(staging_name);
        let textures_dir = staging_dir.join("textures");
        fs::create_dir_all(&textures_dir)?;

        let stack_hash = self
            .config
            .stack_hash
            .clone()
            .unwrap_or_else(|| format!("{:08x}", rand_suffix));
        let short_hash = if stack_hash.len() >= 8 {
            &stack_hash[..8]
        } else {
            &stack_hash
        };

        // 1. Generate procedural 16x16 magenta/dark fallback checkerboard
        let mut fallback_buf = RgbaBuffer::solid(16, 16, 24, 24, 24, 255);
        for y in 0..16u32 {
            for x in 0..16u32 {
                if ((x / 4) + (y / 4)) % 2 == 0 {
                    fallback_buf.set_pixel(x, y, [255, 0, 255, 255]);
                }
            }
        }
        let fallback_bytes = fallback_buf.to_png_bytes()?;
        fs::write(textures_dir.join("mtk_fallback.png"), fallback_bytes)?;

        let fallback_record = StandaloneTextureRecord {
            namespace: "mozi".to_string(),
            texture_name: "fallback".to_string(),
            texture_key: "fallback".to_string(),
            canonical_key: "mozi:fallback".to_string(),
            files: StandaloneFilePaths {
                albedo: Some("textures/mtk_fallback.png".to_string()),
                normal: None,
                specular: None,
                overlay: None,
            },
            is_animated: false,
            animation: None,
            tint_info: json!({}),
            is_fallback: Some(true),
        };

        // 2. Discover all unique texture locations across the stack
        let mut loc_vec = stack.list_all_texture_locations();
        if let Some(prefix) = self.config.filter_prefix.as_deref() {
            loc_vec.retain(|loc| loc.path.starts_with(prefix));
        }

        // 3. Process textures (multi-threaded via Rayon when enabled)

        #[cfg(feature = "parallel")]
        let process_iter = loc_vec.par_iter();
        #[cfg(not(feature = "parallel"))]
        let process_iter = loc_vec.iter();

        let records: Vec<(ResourceLocation, StandaloneTextureRecord, Vec<(String, Vec<u8>)>)> =
            process_iter
                .filter_map(|loc| {
                    let companions = stack.resolve_pbr_companions(loc);
                    let raw_albedo = companions.albedo.as_ref()?;
                    let albedo_buf = RgbaBuffer::from_png_bytes(raw_albedo).ok()?;

                    let mut channels = vec![ChannelData {
                        channel_type: ChannelType::Albedo,
                        buffer: albedo_buf,
                        metadata: companions.mcmeta.clone(),
                    }];

                    if let Some(ref n_bytes) = companions.normal {
                        if let Ok(n_buf) = RgbaBuffer::from_png_bytes(n_bytes) {
                            channels.push(ChannelData {
                                channel_type: ChannelType::Normal,
                                buffer: n_buf,
                                metadata: None,
                            });
                        }
                    }

                    if let Some(ref s_bytes) = companions.specular {
                        if let Ok(s_buf) = RgbaBuffer::from_png_bytes(s_bytes) {
                            channels.push(ChannelData {
                                channel_type: ChannelType::Specular,
                                buffer: s_buf,
                                metadata: None,
                            });
                        }
                    }

                    // Multi-channel frame alignment
                    let align_res = align_standalone_channels(channels);

                    let clean_name = loc.path.replace('/', "-").replace(':', "-");
                    let mut file_paths = StandaloneFilePaths::default();
                    let mut out_files = Vec::new();

                    for ch in align_res.channels {
                        let ch_name = ch.channel_type.as_str();
                        let file_name = format!(
                            "{}_{}_{}_{}.png",
                            short_hash, loc.namespace, clean_name, ch_name
                        );
                        let rel_path = format!("textures/{}", file_name);

                        if let Ok(png_bytes) = ch.buffer.to_png_bytes() {
                            out_files.push((file_name, png_bytes));
                            match ch.channel_type {
                                ChannelType::Albedo => file_paths.albedo = Some(rel_path),
                                ChannelType::Normal => file_paths.normal = Some(rel_path),
                                ChannelType::Specular => file_paths.specular = Some(rel_path),
                                ChannelType::Overlay => file_paths.overlay = Some(rel_path),
                            }
                        }
                    }

                    let tex_name = loc.short_name().to_string();
                    let canonical_key = loc.as_string();

                    let record = StandaloneTextureRecord {
                        namespace: loc.namespace.clone(),
                        texture_name: tex_name,
                        texture_key: loc.path.clone(),
                        canonical_key,
                        files: file_paths,
                        is_animated: align_res.is_animated,
                        animation: align_res.animation,
                        tint_info: json!({}),
                        is_fallback: None,
                    };

                    Some((loc.clone(), record, out_files))
                })
                .collect();

        // 4. Write all encoded image files to staging directory
        #[cfg(feature = "parallel")]
        records.par_iter().for_each(|(_, _, files)| {
            for (fname, bytes) in files {
                let p = textures_dir.join(fname);
                let _ = fs::write(p, bytes);
            }
        });

        #[cfg(not(feature = "parallel"))]
        for (_, _, files) in &records {
            for (fname, bytes) in files {
                let p = textures_dir.join(fname);
                let _ = fs::write(p, bytes);
            }
        }

        // 5. Construct metadata mapping and alias table
        let mut textures_map = HashMap::new();
        let mut aliases_map = HashMap::new();

        // Insert fallback
        textures_map.insert("mozi:fallback".to_string(), fallback_record.clone());

        for (loc, rec, _) in &records {
            let full_key = loc.as_string();
            let canonical_key = rec.canonical_key.clone();

            textures_map.insert(full_key.clone(), rec.clone());
            textures_map.insert(canonical_key.clone(), rec.clone());

            aliases_map.entry(rec.texture_name.clone()).or_insert_with(|| canonical_key.clone());
            aliases_map.insert(rec.texture_key.clone(), canonical_key);
        }

        let mapping = StandaloneMapping {
            format_version: STANDALONE_FORMAT_VERSION,
            stack_hash: stack_hash.clone(),
            texture_count: records.len(),
            textures: textures_map,
            aliases: aliases_map,
        };

        let mapping_path = staging_dir.join("standalone_mapping.json");
        let mapping_json = serde_json::to_string_pretty(&mapping)
            .map_err(|e| TextureError::Baking(e.to_string()))?;
        fs::write(&mapping_path, mapping_json)?;

        // 6. Integrity check
        if !mapping_path.exists() || fs::metadata(&mapping_path)?.len() == 0 {
            let _ = fs::remove_dir_all(&staging_dir);
            return Err(TextureError::Baking(
                "Failed to generate valid standalone_mapping.json".to_string(),
            ));
        }

        // 7. Atomic Publication
        if output_path.exists() {
            let _ = fs::remove_dir_all(&output_path);
        }

        fs::rename(&staging_dir, &output_path)?;

        let final_mapping_path = output_path.join("standalone_mapping.json");
        Ok(StandaloneResult {
            mapping_path: final_mapping_path,
            output_dir: output_path,
            texture_count: records.len(),
            format_version: STANDALONE_FORMAT_VERSION,
        })
    }
}

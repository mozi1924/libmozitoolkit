//! High-Performance Block Model JSON & Biome Tinting Resolver in Rust.

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use serde_json::Value;

use super::hardcoded::{
    classify_tint_category, get_hardcoded_tint,
    TINT_TYPE_DRY_FOLIAGE, TINT_TYPE_FOLIAGE, TINT_TYPE_GRASS, TINT_TYPE_HARDCODED,
    TINT_TYPE_NONE, TINT_TYPE_WATER,
};

/// Complete resolved tint metadata for a single texture or block face.
#[derive(Debug, Clone, PartialEq)]
pub struct TintInfo {
    pub tint_type: u8,
    pub tint_category: String,
    pub tint_weight: f32,
    pub base_tint_weight: f32,
    pub overlay_tint_weight: f32,
    pub has_overlay: bool,
    pub overlay_texture: Option<String>,
    pub is_hardcoded: bool,
    pub hardcoded_color: Option<[f32; 4]>,
}

impl Default for TintInfo {
    fn default() -> Self {
        Self {
            tint_type: TINT_TYPE_NONE,
            tint_category: "none".to_string(),
            tint_weight: 0.0,
            base_tint_weight: 0.0,
            overlay_tint_weight: 0.0,
            has_overlay: false,
            overlay_texture: None,
            is_hardcoded: false,
            hardcoded_color: None,
        }
    }
}

/// Resource-pack aware Biome Resolver.
/// Discovers model JSON `tintindex` metadata and `side` / `overlay` texture pairings.
#[derive(Debug, Clone)]
pub struct BiomeResolver {
    pub overlay_pairs: HashMap<String, String>,
    pub texture_tint_categories: HashMap<String, String>,
    pub texture_hardcoded_colors: HashMap<String, [f32; 4]>,
}

impl Default for BiomeResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl BiomeResolver {
    /// Create a new BiomeResolver initialized with standard vanilla overlay pairs.
    pub fn new() -> Self {
        let mut overlay_pairs = HashMap::new();
        overlay_pairs.insert("grass_block_side".to_string(), "grass_block_side_overlay".to_string());
        overlay_pairs.insert("grass_block_snow".to_string(), "grass_block_side_overlay".to_string());
        overlay_pairs.insert("grass_side".to_string(), "grass_side_overlay".to_string());
        overlay_pairs.insert("grass_side_snowed".to_string(), "grass_side_overlay".to_string());

        Self {
            overlay_pairs,
            texture_tint_categories: HashMap::new(),
            texture_hardcoded_colors: HashMap::new(),
        }
    }

    /// Retrieve the paired overlay texture stem for a given base texture stem, if any.
    pub fn get_overlay_texture(&self, texture_stem: &str) -> Option<&str> {
        let clean = texture_stem.trim().to_ascii_lowercase();
        let unname = clean.strip_prefix("minecraft:").unwrap_or(&clean);
        let stem = unname.strip_prefix("block/").unwrap_or(unname);
        self.overlay_pairs.get(stem).map(|s| s.as_str())
    }

    /// Resolve full tint metadata for a given texture name, block name, and optional tint index.
    pub fn get_tint_info(
        &self,
        texture_name: &str,
        block_name: Option<&str>,
        tint_index: Option<i32>,
    ) -> TintInfo {
        let clean = texture_name.trim().to_ascii_lowercase();
        let unname = clean.strip_prefix("minecraft:").unwrap_or(&clean);
        let stem = unname.strip_prefix("block/").unwrap_or(unname);

        // 1. Check Hardcoded block tints
        if let Some(hc_col) = self.texture_hardcoded_colors.get(stem).copied().or_else(|| get_hardcoded_tint(stem)).or_else(|| block_name.and_then(get_hardcoded_tint)) {
            return TintInfo {
                tint_type: TINT_TYPE_HARDCODED,
                tint_category: "hardcoded".to_string(),
                tint_weight: 1.0,
                base_tint_weight: 1.0,
                overlay_tint_weight: 1.0,
                has_overlay: false,
                overlay_texture: None,
                is_hardcoded: true,
                hardcoded_color: Some(hc_col),
            };
        }

        // 2. Check Overlays (e.g. grass_block_side has grass_block_side_overlay)
        let overlay_stem = self.get_overlay_texture(stem).map(|s| s.to_string());
        let has_overlay = overlay_stem.is_some();

        // 3. Category classification
        let category = if let Some(cat) = self.texture_tint_categories.get(stem) {
            cat.as_str()
        } else {
            let cat = classify_tint_category(stem, block_name, tint_index);
            if cat == "none" && has_overlay {
                if let Some(ref o_stem) = overlay_stem {
                    classify_tint_category(o_stem, block_name, tint_index)
                } else {
                    cat
                }
            } else {
                cat
            }
        };

        match category {
            "grass" => {
                let base_weight = if has_overlay { 0.0 } else { 1.0 };
                TintInfo {
                    tint_type: TINT_TYPE_GRASS,
                    tint_category: "grass".to_string(),
                    tint_weight: 1.0,
                    base_tint_weight: base_weight,
                    overlay_tint_weight: 1.0,
                    has_overlay,
                    overlay_texture: overlay_stem,
                    is_hardcoded: false,
                    hardcoded_color: None,
                }
            }
            "foliage" => TintInfo {
                tint_type: TINT_TYPE_FOLIAGE,
                tint_category: "foliage".to_string(),
                tint_weight: 1.0,
                base_tint_weight: 1.0,
                overlay_tint_weight: 1.0,
                has_overlay,
                overlay_texture: overlay_stem,
                is_hardcoded: false,
                hardcoded_color: None,
            },
            "dry_foliage" => TintInfo {
                tint_type: TINT_TYPE_DRY_FOLIAGE,
                tint_category: "dry_foliage".to_string(),
                tint_weight: 1.0,
                base_tint_weight: 1.0,
                overlay_tint_weight: 1.0,
                has_overlay,
                overlay_texture: overlay_stem,
                is_hardcoded: false,
                hardcoded_color: None,
            },
            "water" => TintInfo {
                tint_type: TINT_TYPE_WATER,
                tint_category: "water".to_string(),
                tint_weight: 1.0,
                base_tint_weight: 1.0,
                overlay_tint_weight: 1.0,
                has_overlay,
                overlay_texture: overlay_stem,
                is_hardcoded: false,
                hardcoded_color: None,
            },
            "hardcoded" => {
                let hc = self.texture_hardcoded_colors.get(stem).copied()
                    .or_else(|| get_hardcoded_tint(stem))
                    .unwrap_or([0.38, 0.60, 0.38, 1.0]);
                TintInfo {
                    tint_type: TINT_TYPE_HARDCODED,
                    tint_category: "hardcoded".to_string(),
                    tint_weight: 1.0,
                    base_tint_weight: 1.0,
                    overlay_tint_weight: 1.0,
                    has_overlay: false,
                    overlay_texture: None,
                    is_hardcoded: true,
                    hardcoded_color: Some(hc),
                }
            }
            _ => TintInfo {
                tint_type: TINT_TYPE_NONE,
                tint_category: "none".to_string(),
                tint_weight: 0.0,
                base_tint_weight: 0.0,
                overlay_tint_weight: 0.0,
                has_overlay: false,
                overlay_texture: None,
                is_hardcoded: false,
                hardcoded_color: None,
            },
        }
    }

    /// Parse block models from a directory containing `assets/minecraft/models/block/*.json`.
    pub fn load_from_directory<P: AsRef<Path>>(&mut self, root: P) -> std::io::Result<()> {
        let models_dir = root.as_ref().join("assets/minecraft/models/block");
        if !models_dir.exists() {
            return Ok(());
        }
        for entry in std::fs::read_dir(models_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
                if let Ok(mut f) = File::open(&path) {
                    let mut s = String::new();
                    if f.read_to_string(&mut s).is_ok() {
                        if let Ok(val) = serde_json::from_str::<Value>(&s) {
                            let stem = path.file_stem().and_then(|st| st.to_str()).unwrap_or("");
                            self.parse_model_json(stem, &val);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Parse block models from any `ResourcePack` implementation.
    pub fn load_from_pack(&mut self, pack: &dyn mtk_resource::ResourcePack) {
        let files = pack.list_files("assets/minecraft/models/block/");
        for file in files {
            if file.ends_with(".json") {
                if let Some(bytes) = pack.open(&file) {
                    if let Ok(val) = serde_json::from_slice::<Value>(&bytes) {
                        let stem = Path::new(&file).file_stem().and_then(|s| s.to_str()).unwrap_or("");
                        self.parse_model_json(stem, &val);
                    }
                }
            }
        }
    }

    /// Parse block models across all layers of a `ResourcePackStack`.
    pub fn load_from_pack_stack(&mut self, stack: &mtk_resource::ResourcePackStack) {
        for pack in stack.packs() {
            self.load_from_pack(pack.as_ref());
        }
    }

    /// Parse block models from a .jar or .zip resource pack.
    pub fn load_from_zip<P: AsRef<Path>>(&mut self, zip_path: P) -> Result<(), Box<dyn std::error::Error>> {
        let file = File::open(zip_path)?;
        let mut archive = zip::ZipArchive::new(file)?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let name = file.name().to_string();
            if name.starts_with("assets/minecraft/models/block/") && name.ends_with(".json") {
                let mut s = String::new();
                if file.read_to_string(&mut s).is_ok() {
                    if let Ok(val) = serde_json::from_str::<Value>(&s) {
                        let stem = Path::new(&name).file_stem().and_then(|st| st.to_str()).unwrap_or("");
                        self.parse_model_json(stem, &val);
                    }
                }
            }
        }
        Ok(())
    }

    fn resolve_texture_var<'a>(&self, raw: &'a str, textures: &'a HashMap<String, String>) -> &'a str {
        let mut curr = raw;
        for _ in 0..8 {
            if let Some(var_name) = curr.strip_prefix('#') {
                if let Some(target) = textures.get(var_name) {
                    curr = target.as_str();
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        let clean = curr.strip_prefix("minecraft:").unwrap_or(curr);
        clean.strip_prefix("block/").unwrap_or(clean)
    }

    fn parse_model_json(&mut self, model_stem: &str, val: &Value) {
        let textures_obj = val.get("textures").and_then(|t| t.as_object());
        let mut textures_map = HashMap::new();
        if let Some(obj) = textures_obj {
            for (k, v) in obj {
                if let Some(s) = v.as_str() {
                    textures_map.insert(k.clone(), s.to_string());
                }
            }
        }

        // Overlay pairs discovery
        if let (Some(side), Some(overlay)) = (textures_map.get("side"), textures_map.get("overlay")) {
            let clean_side = self.resolve_texture_var(side, &textures_map);
            let clean_overlay = self.resolve_texture_var(overlay, &textures_map);
            if !clean_side.is_empty() && !clean_overlay.is_empty() && clean_side != clean_overlay {
                self.overlay_pairs.insert(clean_side.to_string(), clean_overlay.to_string());
            }
        }

        // Elements tintindex discovery
        if let Some(elements) = val.get("elements").and_then(|e| e.as_array()) {
            for elem in elements {
                if let Some(faces) = elem.get("faces").and_then(|f| f.as_object()) {
                    for (_f_name, f_data) in faces {
                        let tint_idx = f_data.get("tintindex").and_then(|ti| ti.as_i64()).map(|ti| ti as i32);
                        if let Some(ti) = tint_idx {
                            if ti >= 0 {
                                if let Some(raw_tex) = f_data.get("texture").and_then(|t| t.as_str()) {
                                    let clean_tex = self.resolve_texture_var(raw_tex, &textures_map);
                                    if !clean_tex.is_empty() {
                                        let cat = classify_tint_category(clean_tex, Some(model_stem), Some(ti));
                                        if cat != "none" {
                                            self.texture_tint_categories.insert(clean_tex.to_string(), cat.to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

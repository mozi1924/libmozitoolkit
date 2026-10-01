//! High-Performance Block Model JSON & Biome Tinting Resolver in Rust.

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::hardcoded::{
    classify_tint_category, get_hardcoded_tint_hex,
    TINT_TYPE_DRY_FOLIAGE, TINT_TYPE_FOLIAGE, TINT_TYPE_GRASS, TINT_TYPE_HARDCODED,
    TINT_TYPE_NONE, TINT_TYPE_WATER,
};
use super::palettes::hex_to_linear_rgba;

/// Complete resolved tint metadata for a single texture or block face.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    pub hardcoded_hex: Option<String>,
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
            hardcoded_hex: None,
        }
    }
}

/// Resource-pack aware Biome Resolver.
/// Discovers model JSON `tintindex` metadata and `side` / `overlay` texture pairings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BiomeResolver {
    pub overlay_pairs: HashMap<String, String>,
    pub texture_tint_categories: HashMap<String, String>,
    pub texture_hardcoded_colors: HashMap<String, [f32; 4]>,
    #[serde(default)]
    pub models: HashMap<String, Value>,
    #[serde(default)]
    pub models_by_stem: HashMap<String, Value>,
}

impl Default for BiomeResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl BiomeResolver {
    pub fn new() -> Self {
        let mut resolver = Self {
            overlay_pairs: HashMap::new(),
            texture_tint_categories: HashMap::new(),
            texture_hardcoded_colors: HashMap::new(),
            models: HashMap::new(),
            models_by_stem: HashMap::new(),
        };
        resolver.seed_defaults();
        resolver
    }

    /// Seed authoritative vanilla texture tint categories and overlay pairings (SSOT).
    pub fn seed_defaults(&mut self) {
        self.overlay_pairs.insert("grass_block_side".to_string(), "grass_block_side_overlay".to_string());
        self.overlay_pairs.insert("grass_block_snow".to_string(), "grass_block_side_overlay".to_string());
        self.overlay_pairs.insert("grass_side".to_string(), "grass_side_overlay".to_string());
        self.overlay_pairs.insert("grass_side_snowed".to_string(), "grass_side_overlay".to_string());

        // 1. Grass colormap (grass.png)
        for t in &[
            "grass_block_top",
            "grass_block_side_overlay",
            "grass_side_overlay",
            "short_grass",
            "grass",
            "tall_grass_top",
            "tall_grass_bottom",
            "fern",
            "potted_fern",
            "large_fern_top",
            "large_fern_bottom",
            "bush", // Vanilla 1.21.4 bush uses grass colormap
            "pink_petals_stem",
            "wildflowers_stem",
            "sugar_cane",
        ] {
            self.texture_tint_categories.insert(t.to_string(), "grass".to_string());
        }

        // 2. Foliage colormap (foliage.png)
        for t in &[
            "oak_leaves",
            "jungle_leaves",
            "acacia_leaves",
            "dark_oak_leaves",
            "mangrove_leaves",
            "vine",
            "bamboo_large_leaves",
            "bamboo_small_leaves",
        ] {
            self.texture_tint_categories.insert(t.to_string(), "foliage".to_string());
        }

        // 3. Dry foliage colormap (dry_foliage.png)
        for t in &[
            "leaf_litter",
            "pale_hanging_moss",
            "pale_hanging_moss_tip",
            "short_dry_grass",
            "tall_dry_grass",
        ] {
            self.texture_tint_categories.insert(t.to_string(), "dry_foliage".to_string());
        }

        // 4. Explicit non-tinted blocks (retain their natural textures)
        for t in &[
            "dead_bush",
            "potted_dead_bush",
            "firefly_bush",
            "azalea_leaves",
            "flowering_azalea_leaves",
            "potted_azalea_bush_plant",
            "potted_flowering_azalea_bush_plant",
            "cherry_leaves",
            "pale_oak_leaves",
            "dirt",
            "coarse_dirt",
            "rooted_dirt",
            "seagrass",
            "tall_seagrass",
            "kelp",
            "kelp_plant",
        ] {
            self.texture_tint_categories.insert(t.to_string(), "none".to_string());
        }

        // 5. Hardcoded non-colormap colors
        for t in &[
            "spruce_leaves",
            "birch_leaves",
            "lily_pad",
            "attached_melon_stem",
            "attached_pumpkin_stem",
            "melon_stem",
            "pumpkin_stem",
            "redstone_wire",
        ] {
            self.texture_tint_categories.insert(t.to_string(), "hardcoded".to_string());
        }
    }

    /// Set or update loaded block models and re-analyze.
    pub fn set_models(&mut self, models: HashMap<String, Value>) {
        self.models = models;
        self.update_models_index();
        self.overlay_pairs.clear();
        self.texture_tint_categories.clear();
        self.texture_hardcoded_colors.clear();
        self.seed_defaults();
        self.analyze_models();
    }

    /// Rebuild stem index lookup for loaded models.
    pub fn update_models_index(&mut self) {
        self.models_by_stem.clear();
        for (k, v) in &self.models {
            let stem = k.split('/').last().unwrap_or(k).trim_end_matches(".json");
            self.models_by_stem.insert(stem.to_string(), v.clone());
        }
    }

    /// Recursively resolve parent model inheritance and merge textures/elements.
    pub fn resolve_model(&self, model_key: &str, depth: usize) -> Option<Value> {
        if depth > 10 {
            return None;
        }
        let clean = model_key.trim().to_ascii_lowercase();
        let unname = clean.strip_prefix("minecraft:").unwrap_or(&clean);
        let stem = unname.split('/').last().unwrap_or(unname).trim_end_matches(".json");

        let model_val = self.models.get(unname)
            .or_else(|| self.models.get(stem))
            .or_else(|| self.models_by_stem.get(stem))?;

        let mut resolved = model_val.clone();
        if let Some(parent_str) = resolved.get("parent").and_then(|p| p.as_str()) {
            let p_clean = parent_str.trim().to_ascii_lowercase();
            let p_unname = p_clean.strip_prefix("minecraft:").unwrap_or(&p_clean);
            if let Some(parent_resolved) = self.resolve_model(p_unname, depth + 1) {
                // Merge textures: child textures override parent
                let mut merged_textures = serde_json::Map::new();
                if let Some(p_tex) = parent_resolved.get("textures").and_then(|t| t.as_object()) {
                    for (k, v) in p_tex {
                        merged_textures.insert(k.clone(), v.clone());
                    }
                }
                if let Some(c_tex) = resolved.get("textures").and_then(|t| t.as_object()) {
                    for (k, v) in c_tex {
                        merged_textures.insert(k.clone(), v.clone());
                    }
                }
                if let Some(obj) = resolved.as_object_mut() {
                    obj.insert("textures".to_string(), Value::Object(merged_textures));
                    // Inherit elements if child does not explicitly define its own
                    if !obj.contains_key("elements") {
                        if let Some(p_elem) = parent_resolved.get("elements") {
                            obj.insert("elements".to_string(), p_elem.clone());
                        }
                    }
                }
            }
        }
        Some(resolved)
    }

    /// Follow #variable reference chains in a model's textures dictionary.
    pub fn resolve_texture_var<'a>(&self, raw: &'a str, textures: &'a HashMap<String, String>) -> String {
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
        clean.strip_prefix("block/").unwrap_or(clean).to_string()
    }

    /// Analyze loaded models to discover overlay pairs, tint indexes, and custom foliage/grass textures.
    pub fn analyze_models(&mut self) {
        let model_keys: Vec<String> = self.models.keys().cloned().collect();
        for model_name in model_keys {
            let resolved = match self.resolve_model(&model_name, 0) {
                Some(r) => r,
                None => continue,
            };

            let mut textures_map = HashMap::new();
            if let Some(textures_obj) = resolved.get("textures").and_then(|t| t.as_object()) {
                for (k, v) in textures_obj {
                    if let Some(s) = v.as_str() {
                        textures_map.insert(k.clone(), s.to_string());
                    }
                }
            }

            // Discover overlay pairs
            if let (Some(side), Some(overlay)) = (textures_map.get("side"), textures_map.get("overlay")) {
                let clean_side = self.resolve_texture_var(side, &textures_map);
                let clean_overlay = self.resolve_texture_var(overlay, &textures_map);
                if !clean_side.is_empty()
                    && !clean_overlay.is_empty()
                    && clean_side != clean_overlay
                    && !clean_side.starts_with('#')
                    && !clean_overlay.starts_with('#')
                {
                    self.overlay_pairs.insert(clean_side, clean_overlay);
                }
            }
            if let (Some(cross), Some(cross_overlay)) = (textures_map.get("cross"), textures_map.get("cross_overlay")) {
                let clean_cross = self.resolve_texture_var(cross, &textures_map);
                let clean_overlay = self.resolve_texture_var(cross_overlay, &textures_map);
                if !clean_cross.is_empty()
                    && !clean_overlay.is_empty()
                    && clean_cross != clean_overlay
                    && !clean_cross.starts_with('#')
                    && !clean_overlay.starts_with('#')
                {
                    self.overlay_pairs.insert(clean_cross, clean_overlay);
                }
            }

            let model_stem = model_name
                .strip_prefix("block/").unwrap_or(&model_name)
                .split('/').last().unwrap_or(&model_name)
                .trim_end_matches(".json");

            if let Some(elements) = resolved.get("elements").and_then(|e| e.as_array()) {
                for elem in elements {
                    if let Some(faces) = elem.get("faces").and_then(|f| f.as_object()) {
                        for (_f_name, f_data) in faces {
                            let tint_idx = f_data.get("tintindex").and_then(|ti| ti.as_i64()).map(|ti| ti as i32);
                            if let Some(ti) = tint_idx {
                                if ti >= 0 {
                                    if let Some(raw_tex) = f_data.get("texture").and_then(|t| t.as_str()) {
                                        let clean_tex = self.resolve_texture_var(raw_tex, &textures_map);
                                        if !clean_tex.is_empty() && !clean_tex.starts_with('#') {
                                            let cat = classify_tint_category(&clean_tex, Some(model_stem), Some(ti));
                                            if cat != "none" {
                                                self.texture_tint_categories.insert(clean_tex, cat.to_string());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                let cat = classify_tint_category("", Some(model_stem), Some(0));
                if cat == "foliage" {
                    for (_k, tex) in &textures_map {
                        let clean_tex = self.resolve_texture_var(tex, &textures_map);
                        if !clean_tex.is_empty() && !clean_tex.starts_with('#') {
                            self.texture_tint_categories.insert(clean_tex, cat.to_string());
                        }
                    }
                }
            }
        }
    }

    /// Serialize BiomeResolver mapping table to a JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize BiomeResolver mapping table from a JSON string.
    pub fn from_json(json_str: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json_str)
    }

    /// Load BiomeResolver from a JSON file on disk.
    pub fn from_file<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let bytes = std::fs::read(path)?;
        serde_json::from_slice(&bytes)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
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

        // 1. Check Overlays (e.g. grass_block_side has grass_block_side_overlay)
        let overlay_stem = self.get_overlay_texture(stem).map(|s| s.to_string());
        let has_overlay = overlay_stem.is_some();

        // 2. In Minecraft rendering, a negative tint index indicates an untinted face,
        // UNLESS the face has an overlay companion (such as grass_block_side) where
        // the base quad has tint_index -1 but the companion overlay quad has tint_index 0.
        if let Some(ti) = tint_index {
            if ti < 0 && !has_overlay {
                return TintInfo::default();
            }
        }

        // 3. Check Hardcoded block tints
        if let Some(hex) = get_hardcoded_tint_hex(stem).or_else(|| block_name.and_then(get_hardcoded_tint_hex)) {
            let col = hex_to_linear_rgba(hex);
            return TintInfo {
                tint_type: TINT_TYPE_HARDCODED,
                tint_category: "hardcoded".to_string(),
                tint_weight: 1.0,
                base_tint_weight: 1.0,
                overlay_tint_weight: 1.0,
                has_overlay: false,
                overlay_texture: None,
                is_hardcoded: true,
                hardcoded_color: Some(col),
                hardcoded_hex: Some(hex.to_string()),
            };
        }
        if let Some(hc_col) = self.texture_hardcoded_colors.get(stem).copied() {
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
                hardcoded_hex: None,
            };
        }

        // 4. Category classification
        let category = if let Some(cat) = self.texture_tint_categories.get(stem) {
            cat.as_str()
        } else {
            let cat = classify_tint_category(stem, block_name, tint_index);
            if cat == "none" && has_overlay {
                if let Some(ref o_stem) = overlay_stem {
                    classify_tint_category(o_stem, block_name, None)
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
                    hardcoded_hex: None,
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
                hardcoded_hex: None,
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
                hardcoded_hex: None,
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
                hardcoded_hex: None,
            },
            "hardcoded" => {
                let hex_opt = get_hardcoded_tint_hex(stem).or_else(|| block_name.and_then(get_hardcoded_tint_hex));
                let hc = self.texture_hardcoded_colors.get(stem).copied()
                    .or_else(|| hex_opt.map(hex_to_linear_rgba))
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
                    hardcoded_hex: hex_opt.map(|s| s.to_string()),
                }
            }
            _ => TintInfo::default(),
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
                            self.models.insert(format!("block/{}", stem), val.clone());
                            self.models.insert(stem.to_string(), val);
                        }
                    }
                }
            }
        }
        self.update_models_index();
        self.analyze_models();
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
                        self.models.insert(format!("block/{}", stem), val.clone());
                        self.models.insert(stem.to_string(), val);
                    }
                }
            }
        }
        self.update_models_index();
        self.analyze_models();
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
                        self.models.insert(format!("block/{}", stem), val.clone());
                        self.models.insert(stem.to_string(), val);
                    }
                }
            }
        }
        self.update_models_index();
        self.analyze_models();
        Ok(())
    }
}

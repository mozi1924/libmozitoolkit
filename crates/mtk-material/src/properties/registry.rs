//! # Material Properties Registry & Configuration
//!
//! Provides data-driven physical material properties evaluation driven by external JSON configurations,
//! runtime Python API injection, and built-in vanilla fallbacks.

use std::collections::{HashMap, HashSet};
use std::sync::RwLock;

use serde::{Deserialize, Serialize};

/// Clean identifier by removing `minecraft:` prefix, `block/` path, and `.png` extension.
#[inline]
pub fn clean_name(s: &str) -> &str {
    let mut cur = s.trim();
    if let Some(rest) = cur.strip_prefix("minecraft:") {
        cur = rest;
    }
    if let Some(rest) = cur.strip_prefix("block/") {
        cur = rest;
    }
    if let Some(rest) = cur.strip_suffix(".png") {
        cur = rest;
    }
    cur
}

fn default_glass_sticker() -> f32 {
    0.55
}

fn default_fluid_sticker() -> f32 {
    0.95
}

/// Emission configuration subsection in JSON.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EmissionConfig {
    #[serde(default)]
    pub non_emissive_keywords: Vec<String>,
    #[serde(default)]
    pub static_blocks: HashMap<String, f32>,
    #[serde(default)]
    pub textures: HashMap<String, f32>,
}

/// Thin wall foliage configuration subsection in JSON.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ThinWallConfig {
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub exact_blocks: Vec<String>,
}

/// Transmissive dielectric configuration subsection in JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransmissiveConfig {
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub exact_blocks: Vec<String>,
    #[serde(default)]
    pub sticker_thresholds: HashMap<String, f32>,
    #[serde(default = "default_glass_sticker")]
    pub default_glass_sticker_threshold: f32,
    #[serde(default = "default_fluid_sticker")]
    pub default_fluid_sticker_threshold: f32,
}

impl Default for TransmissiveConfig {
    fn default() -> Self {
        Self {
            keywords: Vec::new(),
            exact_blocks: Vec::new(),
            sticker_thresholds: HashMap::new(),
            default_glass_sticker_threshold: default_glass_sticker(),
            default_fluid_sticker_threshold: default_fluid_sticker(),
        }
    }
}

/// Top-level material properties configuration schema.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MaterialPropertyConfig {
    #[serde(default)]
    pub version: Option<u32>,
    #[serde(default)]
    pub emission: EmissionConfig,
    #[serde(default)]
    pub thin_wall: ThinWallConfig,
    #[serde(default)]
    pub transmissive: TransmissiveConfig,
}

/// High-performance physical material properties catalog.
#[derive(Debug, Clone)]
pub struct MaterialPropertyRegistry {
    pub non_emissive_keywords: Vec<String>,
    pub static_emissions: HashMap<String, f32>,
    pub texture_emissions: HashMap<String, f32>,
    pub thin_wall_keywords: Vec<String>,
    pub thin_wall_exact: HashSet<String>,
    pub transmissive_keywords: Vec<String>,
    pub transmissive_exact: HashSet<String>,
    pub sticker_thresholds: HashMap<String, f32>,
    pub default_glass_sticker_threshold: f32,
    pub default_fluid_sticker_threshold: f32,
}

impl Default for MaterialPropertyRegistry {
    fn default() -> Self {
        Self::default_built_in()
    }
}

impl MaterialPropertyRegistry {
    /// Creates a registry from a deserialized configuration.
    pub fn from_config(config: MaterialPropertyConfig) -> Self {
        Self {
            non_emissive_keywords: config.emission.non_emissive_keywords,
            static_emissions: config.emission.static_blocks,
            texture_emissions: config.emission.textures,
            thin_wall_keywords: config.thin_wall.keywords,
            thin_wall_exact: config.thin_wall.exact_blocks.into_iter().collect(),
            transmissive_keywords: config.transmissive.keywords,
            transmissive_exact: config.transmissive.exact_blocks.into_iter().collect(),
            sticker_thresholds: config.transmissive.sticker_thresholds,
            default_glass_sticker_threshold: config.transmissive.default_glass_sticker_threshold,
            default_fluid_sticker_threshold: config.transmissive.default_fluid_sticker_threshold,
        }
    }

    /// Parses configuration from a JSON string.
    pub fn from_json(json_str: &str) -> Result<Self, serde_json::Error> {
        let config: MaterialPropertyConfig = serde_json::from_str(json_str)?;
        Ok(Self::from_config(config))
    }

    /// Loads the canonical built-in vanilla fallback embedded at compile time.
    pub fn default_built_in() -> Self {
        const DEFAULT_JSON: &str = include_str!("../../assets/material_properties.json");
        Self::from_json(DEFAULT_JSON).expect("valid embedded material properties json")
    }

    /// Merges an external configuration into this registry (overwriting collisions).
    pub fn merge_config(&mut self, config: MaterialPropertyConfig) {
        for kw in config.emission.non_emissive_keywords {
            if !self.non_emissive_keywords.contains(&kw) {
                self.non_emissive_keywords.push(kw);
            }
        }
        for (k, v) in config.emission.static_blocks {
            self.static_emissions.insert(k, v);
        }
        for (k, v) in config.emission.textures {
            self.texture_emissions.insert(k, v);
        }

        for kw in config.thin_wall.keywords {
            if !self.thin_wall_keywords.contains(&kw) {
                self.thin_wall_keywords.push(kw);
            }
        }
        for exact in config.thin_wall.exact_blocks {
            self.thin_wall_exact.insert(exact);
        }

        for kw in config.transmissive.keywords {
            if !self.transmissive_keywords.contains(&kw) {
                self.transmissive_keywords.push(kw);
            }
        }
        for exact in config.transmissive.exact_blocks {
            self.transmissive_exact.insert(exact);
        }
        for (k, v) in config.transmissive.sticker_thresholds {
            self.sticker_thresholds.insert(k, v);
        }
        if (config.transmissive.default_glass_sticker_threshold - default_glass_sticker()).abs()
            > f32::EPSILON
        {
            self.default_glass_sticker_threshold =
                config.transmissive.default_glass_sticker_threshold;
        }
        if (config.transmissive.default_fluid_sticker_threshold - default_fluid_sticker()).abs()
            > f32::EPSILON
        {
            self.default_fluid_sticker_threshold =
                config.transmissive.default_fluid_sticker_threshold;
        }
    }

    /// Merges an external JSON string into this registry.
    pub fn merge_json(&mut self, json_str: &str) -> Result<(), serde_json::Error> {
        let config: MaterialPropertyConfig = serde_json::from_str(json_str)?;
        self.merge_config(config);
        Ok(())
    }

    /// Evaluates emission strength for a block name, optional state properties, and optional texture name.
    pub fn get_block_emission_strength(
        &self,
        block_name: &str,
        properties: Option<&HashMap<String, String>>,
        texture_name: Option<&str>,
    ) -> f32 {
        let clean_b = clean_name(block_name);
        let clean_t = texture_name.map(clean_name).unwrap_or("");

        // 0. Exclude non-emissive keywords
        for kw in &self.non_emissive_keywords {
            if clean_b.contains(kw) || clean_t.contains(kw) {
                return 0.0;
            }
        }

        // 1. Dynamic state resolvers
        if let Some(props) = properties {
            let is_lit = props
                .get("lit")
                .map(|s| s.eq_ignore_ascii_case("true"))
                .unwrap_or(false);
            match clean_b {
                "campfire" => return if is_lit { 15.0 } else { 0.0 },
                "soul_campfire" => return if is_lit { 10.0 } else { 0.0 },
                "furnace" | "blast_furnace" | "smoker" => return if is_lit { 13.0 } else { 0.0 },
                "redstone_lamp" => return if is_lit { 15.0 } else { 0.0 },
                "redstone_torch" | "redstone_wall_torch" => return if is_lit { 7.0 } else { 0.0 },
                "redstone_ore" | "deepslate_redstone_ore" => return if is_lit { 9.0 } else { 0.0 },
                "copper_bulb" | "waxed_copper_bulb" => return if is_lit { 15.0 } else { 0.0 },
                "exposed_copper_bulb" | "waxed_exposed_copper_bulb" => {
                    return if is_lit { 12.0 } else { 0.0 }
                }
                "weathered_copper_bulb" | "waxed_weathered_copper_bulb" => {
                    return if is_lit { 8.0 } else { 0.0 }
                }
                "oxidized_copper_bulb" | "waxed_oxidized_copper_bulb" => {
                    return if is_lit { 4.0 } else { 0.0 }
                }
                "candle_cake" => return if is_lit { 3.0 } else { 0.0 },
                s if s == "candle" || s.ends_with("_candle") => {
                    if !is_lit {
                        return 0.0;
                    }
                    let candles = props
                        .get("candles")
                        .and_then(|v| v.parse::<f32>().ok())
                        .unwrap_or(1.0);
                    return candles * 3.0;
                }
                "sea_pickle" => {
                    let waterlogged = props
                        .get("waterlogged")
                        .map(|v| v.eq_ignore_ascii_case("true"))
                        .unwrap_or(true);
                    if !waterlogged {
                        return 0.0;
                    }
                    let pickles = props
                        .get("pickles")
                        .and_then(|v| v.parse::<f32>().ok())
                        .unwrap_or(1.0);
                    return pickles * 3.0 + 3.0;
                }
                "respawn_anchor" => {
                    let charges = props
                        .get("charges")
                        .and_then(|v| v.parse::<i32>().ok())
                        .unwrap_or(0);
                    return match charges {
                        1 => 3.0,
                        2 => 7.0,
                        3 => 11.0,
                        4 => 15.0,
                        _ => 0.0,
                    };
                }
                "cave_vines" | "cave_vines_plant" => {
                    let berries = props
                        .get("berries")
                        .map(|v| v.eq_ignore_ascii_case("true"))
                        .unwrap_or(false);
                    return if berries { 14.0 } else { 0.0 };
                }
                "light" => {
                    return props
                        .get("level")
                        .and_then(|v| v.parse::<f32>().ok())
                        .unwrap_or(15.0);
                }
                "redstone_wire" => {
                    return props
                        .get("power")
                        .and_then(|v| v.parse::<f32>().ok())
                        .unwrap_or(0.0);
                }
                _ => {}
            }
        }

        // 2. Eyeblossom special handling
        if clean_b.contains("eyeblossom") || clean_t.contains("eyeblossom") {
            if clean_b.contains("open") || clean_t.contains("open") {
                return 11.0;
            }
            return 0.0;
        }

        // 3. Static block registry lookup
        if let Some(&level) = self.static_emissions.get(clean_b) {
            return level;
        }

        // 4. Texture map registry lookup
        let t_base = clean_t.rsplit('/').next().unwrap_or(clean_t);
        if let Some(&level) = self.texture_emissions.get(clean_t) {
            return level;
        }
        if let Some(&level) = self.texture_emissions.get(t_base) {
            return level;
        }

        0.0
    }

    /// Checks if a block or texture represents thin-wall foliage or vegetation.
    pub fn is_thin_wall_block(&self, block_name: &str, texture_name: Option<&str>) -> bool {
        let clean_b = clean_name(block_name);
        let clean_t = texture_name.map(clean_name).unwrap_or("");

        // Exact match in set
        if self.thin_wall_exact.contains(clean_b)
            || (!clean_t.is_empty() && self.thin_wall_exact.contains(clean_t))
        {
            return true;
        }

        // Keyword fuzzy match
        for kw in &self.thin_wall_keywords {
            if clean_b.contains(kw) || (!clean_t.is_empty() && clean_t.contains(kw)) {
                return true;
            }
        }

        false
    }

    /// Checks if a block or texture is a transmissive / refractive dielectric (glass, water, ice, etc.).
    pub fn is_transmissive_block(&self, block_name: &str, texture_name: Option<&str>) -> bool {
        let clean_b = clean_name(block_name);
        let clean_t = texture_name.map(clean_name).unwrap_or("");

        // Exact match
        if self.transmissive_exact.contains(clean_b)
            || (!clean_t.is_empty() && self.transmissive_exact.contains(clean_t))
        {
            return true;
        }

        // Keyword match
        for kw in &self.transmissive_keywords {
            if clean_b.contains(kw) || (!clean_t.is_empty() && clean_t.contains(kw)) {
                if clean_b.contains("spyglass") || clean_t.contains("spyglass") {
                    return false;
                }
                return true;
            }
        }

        false
    }

    /// Evaluates transmission weight (1.0 for transmissive, 0.0 otherwise).
    #[inline]
    pub fn get_block_transmission_weight(
        &self,
        block_name: &str,
        texture_name: Option<&str>,
    ) -> f32 {
        if self.is_transmissive_block(block_name, texture_name) {
            1.0
        } else {
            0.0
        }
    }

    /// Evaluates alpha sticker threshold (e.g. 0.55 for glass decals, 0.95 for fluids/ice).
    pub fn get_block_sticker_threshold(&self, block_name: &str, texture_name: Option<&str>) -> f32 {
        let clean_b = clean_name(block_name);
        let clean_t = texture_name.map(clean_name).unwrap_or("");

        if let Some(&th) = self.sticker_thresholds.get(clean_b) {
            return th;
        }
        if let Some(&th) = self.sticker_thresholds.get(clean_t) {
            return th;
        }

        if clean_b.contains("water")
            || clean_t.contains("water")
            || clean_b.contains("ice")
            || clean_t.contains("ice")
            || clean_b.contains("slime")
            || clean_t.contains("slime")
            || clean_b.contains("honey")
            || clean_t.contains("honey")
        {
            return self.default_fluid_sticker_threshold;
        }

        self.default_glass_sticker_threshold
    }

    /// Evaluates packed material properties `[emission, thin_wall, transmission, sticker_threshold]`.
    #[inline]
    pub fn get_material_props(&self, block_name: &str, texture_name: Option<&str>) -> [f32; 4] {
        let emission = self.get_block_emission_strength(block_name, None, texture_name);
        let thin_wall = if self.is_thin_wall_block(block_name, texture_name) {
            1.0
        } else {
            0.0
        };
        let transmission = self.get_block_transmission_weight(block_name, texture_name);
        let sticker = self.get_block_sticker_threshold(block_name, texture_name);
        [emission, thin_wall, transmission, sticker]
    }
}

// ---------------------------------------------------------------------------
// Thread-Safe Global Registry Access
// ---------------------------------------------------------------------------

static GLOBAL_REGISTRY: RwLock<Option<MaterialPropertyRegistry>> = RwLock::new(None);

/// Executes a closure with a read lock on the global properties registry,
/// initializing with built-in defaults if uninitialized.
pub fn with_global_registry<R, F: FnOnce(&MaterialPropertyRegistry) -> R>(f: F) -> R {
    // 1. Fast read path
    {
        let guard = GLOBAL_REGISTRY.read().expect("read lock global registry");
        if let Some(ref reg) = *guard {
            return f(reg);
        }
    }

    // 2. Slow init write path
    {
        let mut guard = GLOBAL_REGISTRY.write().expect("write lock global registry");
        if guard.is_none() {
            *guard = Some(MaterialPropertyRegistry::default_built_in());
        }
    }

    // 3. Read again
    let guard = GLOBAL_REGISTRY
        .read()
        .expect("read lock global registry after init");
    f(guard.as_ref().unwrap())
}

/// Registers / merges external material properties JSON into the global registry.
pub fn register_material_properties_json(json_str: &str) -> Result<(), serde_json::Error> {
    let mut guard = GLOBAL_REGISTRY.write().expect("write lock global registry");
    if guard.is_none() {
        *guard = Some(MaterialPropertyRegistry::default_built_in());
    }
    if let Some(ref mut reg) = *guard {
        reg.merge_json(json_str)?;
    }
    Ok(())
}

/// Fully replaces the global registry with configuration from a JSON string.
pub fn load_material_properties_json_replace(json_str: &str) -> Result<(), serde_json::Error> {
    let reg = MaterialPropertyRegistry::from_json(json_str)?;
    let mut guard = GLOBAL_REGISTRY.write().expect("write lock global registry");
    *guard = Some(reg);
    Ok(())
}

/// Resets the global registry back to canonical built-in defaults.
pub fn reset_material_properties_to_default() {
    let mut guard = GLOBAL_REGISTRY.write().expect("write lock global registry");
    *guard = Some(MaterialPropertyRegistry::default_built_in());
}

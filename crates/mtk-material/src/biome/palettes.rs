//! Canonical Minecraft 26.2 Biome Palettes, Colormap UV Mapping, and Color Conversions.

/// Convert standard sRGB color component (0.0 .. 1.0) to Linear RGB.
#[inline]
pub fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Convert Linear RGB component (0.0 .. 1.0) to sRGB.
#[inline]
pub fn linear_to_srgb(c: f32) -> f32 {
    if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

/// Parse a hex color string (e.g. "#91BD59" or "91BD59") into sRGB float components `[r, g, b, a]`.
pub fn hex_to_srgb(hex: &str) -> [f32; 4] {
    let clean = hex.trim().trim_start_matches('#');
    let bytes = clean.as_bytes();
    if bytes.len() == 6 {
        let r = u8::from_str_radix(&clean[0..2], 16).unwrap_or(255) as f32 / 255.0;
        let g = u8::from_str_radix(&clean[2..4], 16).unwrap_or(255) as f32 / 255.0;
        let b = u8::from_str_radix(&clean[4..6], 16).unwrap_or(255) as f32 / 255.0;
        [r, g, b, 1.0]
    } else if bytes.len() == 8 {
        let r = u8::from_str_radix(&clean[0..2], 16).unwrap_or(255) as f32 / 255.0;
        let g = u8::from_str_radix(&clean[2..4], 16).unwrap_or(255) as f32 / 255.0;
        let b = u8::from_str_radix(&clean[4..6], 16).unwrap_or(255) as f32 / 255.0;
        let a = u8::from_str_radix(&clean[6..8], 16).unwrap_or(255) as f32 / 255.0;
        [r, g, b, a]
    } else {
        [1.0, 1.0, 1.0, 1.0]
    }
}

/// Parse a hex color string into Linear RGBA float components for Blender shaders.
pub fn hex_to_linear_rgba(hex: &str) -> [f32; 4] {
    let srgb = hex_to_srgb(hex);
    [
        srgb_to_linear(srgb[0]),
        srgb_to_linear(srgb[1]),
        srgb_to_linear(srgb[2]),
        srgb[3],
    ]
}

/// Compute standard Minecraft triangular Colormap UV coordinates from temperature and humidity.
#[inline]
pub fn get_colormap_uv(temperature: f32, humidity: f32) -> [f32; 2] {
    let t = temperature.clamp(0.0, 1.0);
    let h = humidity.clamp(0.0, 1.0) * t;
    [1.0 - t, h]
}

/// Canonical metadata and color definition for a single Minecraft biome.
#[derive(Debug, Clone, PartialEq)]
pub struct BiomePalette {
    pub id: &'static str,
    pub name: &'static str,
    pub grass_hex: &'static str,
    pub foliage_hex: &'static str,
    pub dry_foliage_hex: &'static str,
    pub water_hex: &'static str,
    pub temperature: f32,
    pub humidity: f32,
    pub has_custom_grass: bool,
    pub has_custom_foliage: bool,
    pub has_custom_dry_foliage: bool,
}

impl BiomePalette {
    #[inline]
    pub fn grass_linear(&self) -> [f32; 4] {
        hex_to_linear_rgba(self.grass_hex)
    }

    #[inline]
    pub fn foliage_linear(&self) -> [f32; 4] {
        hex_to_linear_rgba(self.foliage_hex)
    }

    #[inline]
    pub fn dry_foliage_linear(&self) -> [f32; 4] {
        hex_to_linear_rgba(self.dry_foliage_hex)
    }

    #[inline]
    pub fn water_linear(&self) -> [f32; 4] {
        hex_to_linear_rgba(self.water_hex)
    }

    #[inline]
    pub fn colormap_uv(&self) -> [f32; 2] {
        get_colormap_uv(self.temperature, self.humidity)
    }
}

pub static CANONICAL_BIOMES: &[BiomePalette] = &[
    BiomePalette { id: "badlands", name: "Badlands", grass_hex: "#90814D", foliage_hex: "#9E814D", dry_foliage_hex: "#A38046", water_hex: "#3F76E4", temperature: 2.0, humidity: 0.0, has_custom_grass: true, has_custom_foliage: true, has_custom_dry_foliage: false },
    BiomePalette { id: "bamboo_jungle", name: "Bamboo Jungle", grass_hex: "#59C93C", foliage_hex: "#30BB0B", dry_foliage_hex: "#A36346", water_hex: "#3F76E4", temperature: 0.95, humidity: 0.9, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "basalt_deltas", name: "Basalt Deltas", grass_hex: "#BFB755", foliage_hex: "#AEA42A", dry_foliage_hex: "#A38046", water_hex: "#3F76E4", temperature: 2.0, humidity: 0.0, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "beach", name: "Beach", grass_hex: "#91BD59", foliage_hex: "#77AB2F", dry_foliage_hex: "#A37546", water_hex: "#3F76E4", temperature: 0.8, humidity: 0.4, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "birch_forest", name: "Birch Forest", grass_hex: "#88BB67", foliage_hex: "#6BA941", dry_foliage_hex: "#A37246", water_hex: "#3F76E4", temperature: 0.6, humidity: 0.6, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "cherry_grove", name: "Cherry Grove", grass_hex: "#B6DB61", foliage_hex: "#B6DB61", dry_foliage_hex: "#A17148", water_hex: "#5DB7EF", temperature: 0.5, humidity: 0.8, has_custom_grass: true, has_custom_foliage: true, has_custom_dry_foliage: false },
    BiomePalette { id: "cold_ocean", name: "Cold Ocean", grass_hex: "#8EB971", foliage_hex: "#71A74D", dry_foliage_hex: "#A17448", water_hex: "#3D57D6", temperature: 0.5, humidity: 0.5, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "crimson_forest", name: "Crimson Forest", grass_hex: "#BFB755", foliage_hex: "#AEA42A", dry_foliage_hex: "#A38046", water_hex: "#3F76E4", temperature: 2.0, humidity: 0.0, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "dark_forest", name: "Dark Forest", grass_hex: "#507A32", foliage_hex: "#59AE30", dry_foliage_hex: "#7B5334", water_hex: "#3F76E4", temperature: 0.7, humidity: 0.8, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: true },
    BiomePalette { id: "deep_cold_ocean", name: "Deep Cold Ocean", grass_hex: "#8EB971", foliage_hex: "#71A74D", dry_foliage_hex: "#A17448", water_hex: "#3D57D6", temperature: 0.5, humidity: 0.5, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "deep_dark", name: "Deep Dark", grass_hex: "#91BD59", foliage_hex: "#77AB2F", dry_foliage_hex: "#A37546", water_hex: "#3F76E4", temperature: 0.8, humidity: 0.4, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "deep_frozen_ocean", name: "Deep Frozen Ocean", grass_hex: "#8EB971", foliage_hex: "#71A74D", dry_foliage_hex: "#A17448", water_hex: "#3938C9", temperature: 0.5, humidity: 0.5, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "deep_lukewarm_ocean", name: "Deep Lukewarm Ocean", grass_hex: "#8EB971", foliage_hex: "#71A74D", dry_foliage_hex: "#A17448", water_hex: "#45ADF2", temperature: 0.5, humidity: 0.5, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "deep_ocean", name: "Deep Ocean", grass_hex: "#8EB971", foliage_hex: "#71A74D", dry_foliage_hex: "#A17448", water_hex: "#3F76E4", temperature: 0.5, humidity: 0.5, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "desert", name: "Desert", grass_hex: "#BFB755", foliage_hex: "#AEA42A", dry_foliage_hex: "#A38046", water_hex: "#3F76E4", temperature: 2.0, humidity: 0.0, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "dripstone_caves", name: "Dripstone Caves", grass_hex: "#91BD59", foliage_hex: "#77AB2F", dry_foliage_hex: "#A37546", water_hex: "#3F76E4", temperature: 0.8, humidity: 0.4, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "end_barrens", name: "End Barrens", grass_hex: "#8EB971", foliage_hex: "#71A74D", dry_foliage_hex: "#A17448", water_hex: "#3F76E4", temperature: 0.5, humidity: 0.5, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "end_highlands", name: "End Highlands", grass_hex: "#8EB971", foliage_hex: "#71A74D", dry_foliage_hex: "#A17448", water_hex: "#3F76E4", temperature: 0.5, humidity: 0.5, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "end_midlands", name: "End Midlands", grass_hex: "#8EB971", foliage_hex: "#71A74D", dry_foliage_hex: "#A17448", water_hex: "#3F76E4", temperature: 0.5, humidity: 0.5, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "eroded_badlands", name: "Eroded Badlands", grass_hex: "#90814D", foliage_hex: "#9E814D", dry_foliage_hex: "#A38046", water_hex: "#3F76E4", temperature: 2.0, humidity: 0.0, has_custom_grass: true, has_custom_foliage: true, has_custom_dry_foliage: false },
    BiomePalette { id: "flower_forest", name: "Flower Forest", grass_hex: "#79C05A", foliage_hex: "#59AE30", dry_foliage_hex: "#A36D46", water_hex: "#3F76E4", temperature: 0.7, humidity: 0.8, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "forest", name: "Forest", grass_hex: "#79C05A", foliage_hex: "#59AE30", dry_foliage_hex: "#A36D46", water_hex: "#3F76E4", temperature: 0.7, humidity: 0.8, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "frozen_ocean", name: "Frozen Ocean", grass_hex: "#80B497", foliage_hex: "#60A17B", dry_foliage_hex: "#8F7A5A", water_hex: "#3938C9", temperature: 0.0, humidity: 0.5, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "frozen_peaks", name: "Frozen Peaks", grass_hex: "#80B497", foliage_hex: "#60A17B", dry_foliage_hex: "#8F7A5A", water_hex: "#3F76E4", temperature: -0.7, humidity: 0.9, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "frozen_river", name: "Frozen River", grass_hex: "#80B497", foliage_hex: "#60A17B", dry_foliage_hex: "#8F7A5A", water_hex: "#3938C9", temperature: 0.0, humidity: 0.5, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "grove", name: "Grove", grass_hex: "#80B497", foliage_hex: "#60A17B", dry_foliage_hex: "#8F7A5A", water_hex: "#3F76E4", temperature: -0.2, humidity: 0.8, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "ice_spikes", name: "Ice Spikes", grass_hex: "#80B497", foliage_hex: "#60A17B", dry_foliage_hex: "#8F7A5A", water_hex: "#3F76E4", temperature: 0.0, humidity: 0.5, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "jagged_peaks", name: "Jagged Peaks", grass_hex: "#80B497", foliage_hex: "#60A17B", dry_foliage_hex: "#8F7A5A", water_hex: "#3F76E4", temperature: -0.7, humidity: 0.9, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "jungle", name: "Jungle", grass_hex: "#59C93C", foliage_hex: "#30BB0B", dry_foliage_hex: "#A36346", water_hex: "#3F76E4", temperature: 0.95, humidity: 0.9, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "lukewarm_ocean", name: "Lukewarm Ocean", grass_hex: "#8EB971", foliage_hex: "#71A74D", dry_foliage_hex: "#A17448", water_hex: "#45ADF2", temperature: 0.5, humidity: 0.5, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "lush_caves", name: "Lush Caves", grass_hex: "#8EB971", foliage_hex: "#71A74D", dry_foliage_hex: "#A17448", water_hex: "#3F76E4", temperature: 0.5, humidity: 0.5, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "mangrove_swamp", name: "Mangrove Swamp", grass_hex: "#6A7039", foliage_hex: "#8DB127", dry_foliage_hex: "#7B5334", water_hex: "#3A7A6A", temperature: 0.8, humidity: 0.9, has_custom_grass: true, has_custom_foliage: true, has_custom_dry_foliage: true },
    BiomePalette { id: "meadow", name: "Meadow", grass_hex: "#83BB6D", foliage_hex: "#64A948", dry_foliage_hex: "#A17148", water_hex: "#0E4ECF", temperature: 0.5, humidity: 0.8, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "mushroom_fields", name: "Mushroom Fields", grass_hex: "#55C93F", foliage_hex: "#2BBB0F", dry_foliage_hex: "#A36246", water_hex: "#3F76E4", temperature: 0.9, humidity: 1.0, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "nether_wastes", name: "Nether Wastes", grass_hex: "#BFB755", foliage_hex: "#AEA42A", dry_foliage_hex: "#A38046", water_hex: "#3F76E4", temperature: 2.0, humidity: 0.0, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "ocean", name: "Ocean", grass_hex: "#8EB971", foliage_hex: "#71A74D", dry_foliage_hex: "#A17448", water_hex: "#3F76E4", temperature: 0.5, humidity: 0.5, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "old_growth_birch_forest", name: "Old Growth Birch Forest", grass_hex: "#88BB67", foliage_hex: "#6BA941", dry_foliage_hex: "#A37246", water_hex: "#3F76E4", temperature: 0.6, humidity: 0.6, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "old_growth_pine_taiga", name: "Old Growth Pine Taiga", grass_hex: "#86B87F", foliage_hex: "#68A55F", dry_foliage_hex: "#9C754D", water_hex: "#3F76E4", temperature: 0.3, humidity: 0.8, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "old_growth_spruce_taiga", name: "Old Growth Spruce Taiga", grass_hex: "#86B783", foliage_hex: "#68A464", dry_foliage_hex: "#9A764F", water_hex: "#3F76E4", temperature: 0.25, humidity: 0.8, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "pale_garden", name: "Pale Garden", grass_hex: "#778272", foliage_hex: "#878D76", dry_foliage_hex: "#A0A69C", water_hex: "#76889D", temperature: 0.7, humidity: 0.8, has_custom_grass: true, has_custom_foliage: true, has_custom_dry_foliage: true },
    BiomePalette { id: "plains", name: "Plains", grass_hex: "#91BD59", foliage_hex: "#77AB2F", dry_foliage_hex: "#A37546", water_hex: "#3F76E4", temperature: 0.8, humidity: 0.4, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "river", name: "River", grass_hex: "#8EB971", foliage_hex: "#71A74D", dry_foliage_hex: "#A17448", water_hex: "#3F76E4", temperature: 0.5, humidity: 0.5, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "savanna", name: "Savanna", grass_hex: "#BFB755", foliage_hex: "#AEA42A", dry_foliage_hex: "#A38046", water_hex: "#3F76E4", temperature: 2.0, humidity: 0.0, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "savanna_plateau", name: "Savanna Plateau", grass_hex: "#BFB755", foliage_hex: "#AEA42A", dry_foliage_hex: "#A38046", water_hex: "#3F76E4", temperature: 2.0, humidity: 0.0, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "small_end_islands", name: "Small End Islands", grass_hex: "#8EB971", foliage_hex: "#71A74D", dry_foliage_hex: "#A17448", water_hex: "#3F76E4", temperature: 0.5, humidity: 0.5, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "snowy_beach", name: "Snowy Beach", grass_hex: "#83B593", foliage_hex: "#64A278", dry_foliage_hex: "#917958", water_hex: "#3D57D6", temperature: 0.05, humidity: 0.3, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "snowy_plains", name: "Snowy Plains", grass_hex: "#80B497", foliage_hex: "#60A17B", dry_foliage_hex: "#8F7A5A", water_hex: "#3F76E4", temperature: 0.0, humidity: 0.5, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "snowy_slopes", name: "Snowy Slopes", grass_hex: "#80B497", foliage_hex: "#60A17B", dry_foliage_hex: "#8F7A5A", water_hex: "#3F76E4", temperature: -0.3, humidity: 0.9, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "snowy_taiga", name: "Snowy Taiga", grass_hex: "#80B497", foliage_hex: "#60A17B", dry_foliage_hex: "#8F7A5A", water_hex: "#3D57D6", temperature: -0.5, humidity: 0.4, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "soul_sand_valley", name: "Soul Sand Valley", grass_hex: "#BFB755", foliage_hex: "#AEA42A", dry_foliage_hex: "#A38046", water_hex: "#3F76E4", temperature: 2.0, humidity: 0.0, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "sparce_jungle", name: "Sparse Jungle", grass_hex: "#64C73F", foliage_hex: "#3EB80F", dry_foliage_hex: "#A36846", water_hex: "#3F76E4", temperature: 0.95, humidity: 0.8, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "sparse_jungle", name: "Sparse Jungle", grass_hex: "#64C73F", foliage_hex: "#3EB80F", dry_foliage_hex: "#A36846", water_hex: "#3F76E4", temperature: 0.95, humidity: 0.8, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "stony_peaks", name: "Stony Peaks", grass_hex: "#91BD59", foliage_hex: "#77AB2F", dry_foliage_hex: "#A37546", water_hex: "#3F76E4", temperature: 1.0, humidity: 0.3, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "stony_shore", name: "Stony Shore", grass_hex: "#8AB689", foliage_hex: "#6DA36E", dry_foliage_hex: "#957853", water_hex: "#3F76E4", temperature: 0.2, humidity: 0.3, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "sunflower_plains", name: "Sunflower Plains", grass_hex: "#91BD59", foliage_hex: "#77AB2F", dry_foliage_hex: "#A37546", water_hex: "#3F76E4", temperature: 0.8, humidity: 0.4, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "swamp", name: "Swamp", grass_hex: "#6A7039", foliage_hex: "#6A7039", dry_foliage_hex: "#7B5334", water_hex: "#617B64", temperature: 0.8, humidity: 0.9, has_custom_grass: true, has_custom_foliage: true, has_custom_dry_foliage: true },
    BiomePalette { id: "taiga", name: "Taiga", grass_hex: "#86B783", foliage_hex: "#68A464", dry_foliage_hex: "#9A764F", water_hex: "#3F76E4", temperature: 0.25, humidity: 0.8, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "the_end", name: "The End", grass_hex: "#8EB971", foliage_hex: "#71A74D", dry_foliage_hex: "#A17448", water_hex: "#3F76E4", temperature: 0.5, humidity: 0.5, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "the_void", name: "The Void", grass_hex: "#8EB971", foliage_hex: "#71A74D", dry_foliage_hex: "#A17448", water_hex: "#3F76E4", temperature: 0.5, humidity: 0.5, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "warm_ocean", name: "Warm Ocean", grass_hex: "#8EB971", foliage_hex: "#71A74D", dry_foliage_hex: "#A17448", water_hex: "#02B0E5", temperature: 0.5, humidity: 0.5, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "warped_forest", name: "Warped Forest", grass_hex: "#BFB755", foliage_hex: "#AEA42A", dry_foliage_hex: "#A38046", water_hex: "#3F76E4", temperature: 2.0, humidity: 0.0, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "windswept_forest", name: "Windswept Forest", grass_hex: "#86B783", foliage_hex: "#68A464", dry_foliage_hex: "#9A764F", water_hex: "#3F76E4", temperature: 0.2, humidity: 0.3, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "windswept_gravelly_hills", name: "Windswept Gravelly Hills", grass_hex: "#8AB689", foliage_hex: "#6DA36E", dry_foliage_hex: "#957853", water_hex: "#3F76E4", temperature: 0.2, humidity: 0.3, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "windswept_hills", name: "Windswept Hills", grass_hex: "#8AB689", foliage_hex: "#6DA36E", dry_foliage_hex: "#957853", water_hex: "#3F76E4", temperature: 0.2, humidity: 0.3, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "windswept_savanna", name: "Windswept Savanna", grass_hex: "#BFB755", foliage_hex: "#AEA42A", dry_foliage_hex: "#A38046", water_hex: "#3F76E4", temperature: 2.0, humidity: 0.0, has_custom_grass: false, has_custom_foliage: false, has_custom_dry_foliage: false },
    BiomePalette { id: "wooded_badlands", name: "Wooded Badlands", grass_hex: "#90814D", foliage_hex: "#9E814D", dry_foliage_hex: "#A38046", water_hex: "#3F76E4", temperature: 2.0, humidity: 0.0, has_custom_grass: true, has_custom_foliage: true, has_custom_dry_foliage: false },
];

/// Look up a canonical BiomePalette by name or ID (case-insensitive).
pub fn get_biome_palette(name_or_id: &str) -> &'static BiomePalette {
    let clean = name_or_id
        .trim()
        .to_ascii_lowercase();
    let unnamespaced = clean.strip_prefix("minecraft:").unwrap_or(&clean);

    for b in CANONICAL_BIOMES {
        if b.id == unnamespaced || b.name.to_ascii_lowercase() == unnamespaced {
            return b;
        }
    }
    // Default PLAINS
    &CANONICAL_BIOMES[40]
}

/// Blend multiple biome colors weighted together.
pub fn blend_biome_colors(biome_weights: &[(&str, f32)], channel: &str) -> [f32; 4] {
    let mut total_w = 0.0f32;
    let mut sum_r = 0.0f32;
    let mut sum_g = 0.0f32;
    let mut sum_b = 0.0f32;
    let mut sum_a = 0.0f32;

    for &(b_name, w) in biome_weights {
        if w <= 0.0 {
            continue;
        }
        let pal = get_biome_palette(b_name);
        let col = match channel {
            "grass" => pal.grass_linear(),
            "foliage" => pal.foliage_linear(),
            "dry_foliage" => pal.dry_foliage_linear(),
            "water" => pal.water_linear(),
            _ => [1.0, 1.0, 1.0, 1.0],
        };
        total_w += w;
        sum_r += col[0] * w;
        sum_g += col[1] * w;
        sum_b += col[2] * w;
        sum_a += col[3] * w;
    }

    if total_w > 0.0 {
        let inv = 1.0 / total_w;
        [sum_r * inv, sum_g * inv, sum_b * inv, sum_a * inv]
    } else {
        [1.0, 1.0, 1.0, 1.0]
    }
}

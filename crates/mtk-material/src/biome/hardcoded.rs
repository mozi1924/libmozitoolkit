//! Hardcoded Minecraft Block Tints, Canonical Block Tint Registry, and Classifier.

use mtk_core::direction::Direction;
use super::palettes::hex_to_linear_rgba;

pub const TINT_TYPE_NONE: u8 = 0;
pub const TINT_TYPE_GRASS: u8 = 1;
pub const TINT_TYPE_FOLIAGE: u8 = 2;
pub const TINT_TYPE_WATER: u8 = 3;
pub const TINT_TYPE_HARDCODED: u8 = 4;
pub const TINT_TYPE_DRY_FOLIAGE: u8 = 5;

/// Static table of vanilla blocks with fixed, non-colormap biome colors.
pub static HARDCODED_BLOCK_TINTS: &[(&str, &str)] = &[
    ("birch_leaves", "#80A755"),
    ("spruce_leaves", "#619961"),
    ("lily_pad", "#208030"),
    ("attached_melon_stem", "#E0C71C"),
    ("attached_pumpkin_stem", "#E0C71C"),
    ("melon_stem", "#E0C71C"),
    ("pumpkin_stem", "#E0C71C"),
    ("redstone_wire", "#4B0000"),
    ("redstone_wire_line0", "#4B0000"),
    ("redstone_wire_line1", "#4B0000"),
    ("redstone_wire_dot", "#4B0000"),
];

/// Canonical Minecraft Block Colors Registry.
/// Maps block stem to layer definitions: (category, default_weight).
pub static BLOCK_TINT_REGISTRY: &[(&str, &[(&str, f32)])] = &[
    // Grass & Flora
    ("grass_block", &[("grass", 1.0)]),
    ("short_grass", &[("grass", 1.0)]),
    ("grass", &[("grass", 1.0)]),
    ("tall_grass", &[("grass", 1.0)]),
    ("fern", &[("grass", 1.0)]),
    ("large_fern", &[("grass", 1.0)]),
    ("potted_fern", &[("grass", 1.0)]),
    ("bush", &[("grass", 1.0)]),
    ("sugar_cane", &[("grass", 1.0)]),
    // Multi-layer Flora (Layer 0 = Petals/Blank, Layer 1 = Stem/Grass)
    ("pink_petals", &[("none", 0.0), ("grass", 1.0)]),
    ("wildflowers", &[("none", 0.0), ("grass", 1.0)]),
    ("bamboo", &[("grass", 1.0)]),
    // Foliage
    ("oak_leaves", &[("foliage", 1.0)]),
    ("jungle_leaves", &[("foliage", 1.0)]),
    ("acacia_leaves", &[("foliage", 1.0)]),
    ("dark_oak_leaves", &[("foliage", 1.0)]),
    ("vine", &[("foliage", 1.0)]),
    ("mangrove_leaves", &[("foliage", 1.0)]),
    // Dry Foliage
    ("leaf_litter", &[("dry_foliage", 1.0)]),
    ("pale_hanging_moss", &[("dry_foliage", 1.0)]),
    ("pale_hanging_moss_tip", &[("dry_foliage", 1.0)]),
    ("pale_oak_leaves", &[("none", 1.0)]),
    // Water & Fluid
    ("water", &[("water", 1.0)]),
    ("flowing_water", &[("water", 1.0)]),
    ("water_still", &[("water", 1.0)]),
    ("water_flow", &[("water", 1.0)]),
    ("water_cauldron", &[("water", 1.0)]),
    ("bubble_column", &[("water", 1.0)]),
    // Hardcoded tints
    ("spruce_leaves", &[("hardcoded", 1.0)]),
    ("birch_leaves", &[("hardcoded", 1.0)]),
    ("lily_pad", &[("hardcoded", 1.0)]),
    ("attached_melon_stem", &[("hardcoded", 1.0)]),
    ("attached_pumpkin_stem", &[("hardcoded", 1.0)]),
    ("melon_stem", &[("hardcoded", 1.0)]),
    ("pumpkin_stem", &[("hardcoded", 1.0)]),
];

/// Explicit list of vanilla blocks and textures that must NEVER receive biome tint.
/// Dead bush is dry dead wood and must retain its natural brown texture.
/// Azalea, cherry, and pale oak leaves have custom pre-colored textures in vanilla.
/// Dirt, seagrass, and kelp are also never tinted.
pub static EXPLICIT_NONE_BLOCKS: &[&str] = &[
    "dead_bush",
    "potted_dead_bush",
    "firefly_bush",
    "azalea_leaves",
    "flowering_azalea_leaves",
    "potted_azalea_bush_plant",
    "potted_flowering_azalea_bush_plant",
    "potted_azalea_bush_side",
    "potted_azalea_bush_top",
    "cherry_leaves",
    "pale_oak_leaves",
    "seagrass",
    "tall_seagrass",
    "kelp",
    "kelp_plant",
    "dirt",
    "coarse_dirt",
    "rooted_dirt",
];

/// Known vanilla texture stems with a grass biome colour provider.
pub static KNOWN_GRASS_STEMS: &[&str] = &[
    "grass_block_top",
    "grass_block_side_overlay",
    "grass_side_overlay",
    "short_grass",
    "grass",
    "tall_grass_top",
    "tall_grass_bottom",
    "tall_grass",
    "fern",
    "large_fern_top",
    "large_fern_bottom",
    "large_fern",
    "sugar_cane",
    "potted_fern",
    "bush",
    "pink_petals_stem",
    "wildflowers_stem",
    "bamboo_large_leaves",
    "bamboo_small_leaves",
    "bamboo_stage0",
];

/// Known vanilla texture stems with a foliage biome colour provider.
pub static KNOWN_FOLIAGE_STEMS: &[&str] = &[
    "oak_leaves",
    "jungle_leaves",
    "acacia_leaves",
    "dark_oak_leaves",
    "vine",
    "mangrove_leaves",
];

/// Known vanilla texture stems with a dry foliage biome colour provider.
pub static KNOWN_DRY_FOLIAGE_STEMS: &[&str] = &[
    "leaf_litter",
    "pale_hanging_moss",
    "pale_hanging_moss_tip",
    "short_dry_grass",
    "tall_dry_grass",
    "dry_grass",
];

/// Known vanilla texture stems with a water biome colour provider.
pub static KNOWN_WATER_STEMS: &[&str] = &[
    "water",
    "water_still",
    "water_flow",
    "water_overlay",
    "water_cauldron",
    "bubble_column",
];

/// Returns true if the stem belongs to an explicit non-tinted block or texture.
pub fn is_explicit_none_tint(stem: &str) -> bool {
    let clean = stem.trim().to_ascii_lowercase();
    let unname = clean.strip_prefix("minecraft:").unwrap_or(&clean);
    let s = unname.strip_prefix("block/").unwrap_or(unname);

    if EXPLICIT_NONE_BLOCKS.iter().any(|&b| s == b || s.starts_with(b)) {
        return true;
    }
    s.contains("dead_bush") || s.contains("firefly_bush")
}

/// Returns true if the stem represents dry foliage (leaf litter, dry grass, etc.).
pub fn is_dry_foliage(stem: &str) -> bool {
    let clean = stem.trim().to_ascii_lowercase();
    let unname = clean.strip_prefix("minecraft:").unwrap_or(&clean);
    let s = unname.strip_prefix("block/").unwrap_or(unname);

    s.contains("leaf_litter")
        || s.contains("dry_foliage")
        || s.contains("dry_grass")
        || s.contains("short_dry_grass")
        || s.contains("tall_dry_grass")
        || s.contains("pale_hanging_moss")
        || s.starts_with("dry_")
}

/// Returns true if the stem represents grass-tinted vegetation.
pub fn is_grass(stem: &str) -> bool {
    let clean = stem.trim().to_ascii_lowercase();
    let unname = clean.strip_prefix("minecraft:").unwrap_or(&clean);
    let s = unname.strip_prefix("block/").unwrap_or(unname);

    if is_explicit_none_tint(s) || is_dry_foliage(s) {
        return false;
    }

    s == "grass"
        || s == "bush"
        || s == "short_grass"
        || s == "fern"
        || s == "sugar_cane"
        || s == "grass_block_top"
        || s == "grass_block_side_overlay"
        || s == "grass_side_overlay"
        || s.contains("short_grass")
        || s.contains("tall_grass")
        || s.contains("fern")
        || s.contains("sugar_cane")
        || s.contains("pink_petals_stem")
        || s.contains("wildflowers_stem")
        || s.contains("potted_fern")
}

/// Returns true if the stem represents foliage-tinted leaves or vines.
pub fn is_foliage(stem: &str) -> bool {
    let clean = stem.trim().to_ascii_lowercase();
    let unname = clean.strip_prefix("minecraft:").unwrap_or(&clean);
    let s = unname.strip_prefix("block/").unwrap_or(unname);

    if is_explicit_none_tint(s) || is_dry_foliage(s) || is_grass(s) {
        return false;
    }

    s.contains("oak_leaves")
        || s.contains("jungle_leaves")
        || s.contains("acacia_leaves")
        || s.contains("dark_oak_leaves")
        || s.contains("mangrove_leaves")
        || s.contains("vine")
        || s.contains("leaves")
        || s.contains("leaf")
}

/// Returns true if the stem represents water.
pub fn is_water(stem: &str) -> bool {
    let clean = stem.trim().to_ascii_lowercase();
    let unname = clean.strip_prefix("minecraft:").unwrap_or(&clean);
    let s = unname.strip_prefix("block/").unwrap_or(unname);

    s == "water" || s.contains("water_still") || s.contains("water_flow") || s.contains("water_cauldron") || s.contains("bubble_column")
}

/// Retrieve the hardcoded hex string for a block or texture stem, if any.
pub fn get_hardcoded_tint_hex(name: &str) -> Option<&'static str> {
    let clean = name.trim().to_ascii_lowercase();
    let unnamespaced = clean.strip_prefix("minecraft:").unwrap_or(&clean);
    let stem = unnamespaced.strip_prefix("block/").unwrap_or(unnamespaced);

    for &(k, hex) in HARDCODED_BLOCK_TINTS {
        if k == stem {
            return Some(hex);
        }
    }
    None
}

/// Retrieve the hardcoded linear color for a block or texture stem, if any.
pub fn get_hardcoded_tint(name: &str) -> Option<[f32; 4]> {
    get_hardcoded_tint_hex(name).map(hex_to_linear_rgba)
}

/// Classify a texture stem and/or block name into an authoritative tint category:
/// "grass", "foliage", "dry_foliage", "water", "hardcoded", or "none".
pub fn classify_tint_category(
    clean_stem: &str,
    block_name: Option<&str>,
    tint_index: Option<i32>,
) -> &'static str {
    // 0. Negative tint_index is strictly untinted in Minecraft specification
    if let Some(ti) = tint_index {
        if ti < 0 {
            return "none";
        }
    }

    let stem = clean_stem.trim().to_ascii_lowercase();
    let unname_stem = stem.strip_prefix("minecraft:").unwrap_or(&stem);
    let s_stem = unname_stem.strip_prefix("block/").unwrap_or(unname_stem);

    let b_norm = block_name.map(|b| {
        let clean = b.trim().to_ascii_lowercase();
        let unname = clean.strip_prefix("minecraft:").unwrap_or(&clean);
        let unblock = unname.strip_prefix("block/").unwrap_or(unname);
        let unmodels = unblock.strip_prefix("models/block/").unwrap_or(unblock);
        let s = unmodels.strip_prefix("models/").unwrap_or(unmodels);
        if let Some((base, _)) = s.split_once('[') {
            base.to_string()
        } else {
            s.to_string()
        }
    });

    if s_stem.is_empty() && b_norm.is_none() {
        return "none";
    }

    // 1. Explicitly untinted check
    if is_explicit_none_tint(s_stem) || b_norm.as_deref().map_or(false, is_explicit_none_tint) {
        return "none";
    }

    // 2. Block-level registry lookup
    if let Some(ref b) = b_norm {
        for &(reg_block, layers) in BLOCK_TINT_REGISTRY {
            if b == reg_block {
                if let Some(ti) = tint_index {
                    let idx = ti as usize;
                    if idx < layers.len() {
                        return layers[idx].0;
                    } else if !layers.is_empty() {
                        return layers.last().unwrap().0;
                    }
                } else {
                    for &(cat, _) in layers {
                        if cat != "none" {
                            return cat;
                        }
                    }
                }
            }
        }
    }

    // 3. Hardcoded block/texture tints
    if get_hardcoded_tint_hex(s_stem).is_some() || b_norm.as_deref().and_then(get_hardcoded_tint_hex).is_some() {
        return "hardcoded";
    }

    // 4. Canonical known stems
    if KNOWN_DRY_FOLIAGE_STEMS.iter().any(|&k| s_stem == k) {
        return "dry_foliage";
    }
    if KNOWN_GRASS_STEMS.iter().any(|&k| s_stem == k) {
        return "grass";
    }
    if KNOWN_FOLIAGE_STEMS.iter().any(|&k| s_stem == k) {
        return "foliage";
    }
    if KNOWN_WATER_STEMS.iter().any(|&k| s_stem == k) {
        return "water";
    }

    // 5. Heuristic fallback for leaves
    if s_stem.contains("leaves") || b_norm.as_deref().map_or(false, |b| b.contains("leaves")) {
        if s_stem.contains("spruce") || b_norm.as_deref().map_or(false, |b| b.contains("spruce")) {
            return "hardcoded";
        }
        if s_stem.contains("birch") || b_norm.as_deref().map_or(false, |b| b.contains("birch")) {
            return "hardcoded";
        }
        if s_stem.contains("cherry") || s_stem.contains("azalea") || s_stem.contains("pale_oak")
            || b_norm.as_deref().map_or(false, |b| b.contains("cherry") || b.contains("azalea") || b.contains("pale_oak"))
        {
            return "none";
        }
        return "foliage";
    }

    // 6. Suffix and keyword heuristics
    if s_stem.ends_with("_grass") || s_stem.ends_with("_fern") || s_stem.ends_with("_vine")
        || s_stem == "bush" || s_stem == "potted_bush"
    {
        return "grass";
    }
    if s_stem.contains("vine") {
        return "foliage";
    }
    if is_dry_foliage(s_stem) || b_norm.as_deref().map_or(false, |b| is_dry_foliage(b)) {
        return "dry_foliage";
    }
    if is_water(s_stem) || b_norm.as_deref().map_or(false, |b| is_water(b)) {
        return "water";
    }

    "none"
}

/// Determine the unit-cube face tint index for a given block and direction.
/// Returns 0 for tinted faces, -1 for untinted faces.
pub fn get_unit_cube_tint_index(clean_block: &str, dir: Direction) -> i16 {
    let clean = clean_block.trim().to_ascii_lowercase();
    let unname = clean.strip_prefix("minecraft:").unwrap_or(&clean);
    let s = unname.strip_prefix("block/").unwrap_or(unname);
    let base = s.split_once('[').map(|(b, _)| b).unwrap_or(s);

    if base == "grass_block" || base == "grass" {
        if dir == Direction::Up {
            0
        } else {
            -1
        }
    } else {
        let cat = classify_tint_category("", Some(base), Some(0));
        if cat != "none" {
            0
        } else {
            -1
        }
    }
}

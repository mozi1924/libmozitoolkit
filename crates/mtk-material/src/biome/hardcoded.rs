//! Hardcoded Minecraft Block Tints and Semantic Tint Category Classifier.

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
    ("redstone_wire", "#4B0000"),
    ("redstone_wire_line0", "#4B0000"),
    ("redstone_wire_line1", "#4B0000"),
    ("redstone_wire_dot", "#4B0000"),
];

/// Explicit list of vanilla blocks and textures that must NEVER receive biome tint.
/// Dead bush is dry dead wood and must retain its natural brown texture.
/// Azalea, cherry, and pale oak leaves have custom pre-colored textures in vanilla.
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
        || s.starts_with("dry_")
}

/// Returns true if the stem represents grass-tinted vegetation.
/// Note: Vanilla 1.21.4 'bush' uses the grass colormap (net.minecraft.client.color.block.BlockColors).
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
        || s.contains("grass_block")
        || s.contains("short_grass")
        || s.contains("tall_grass")
        || s.contains("fern")
        || s.contains("sugar_cane")
        || s.contains("pink_petals")
        || s.contains("wildflowers")
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
        || s.contains("bamboo_large_leaves")
        || s.contains("bamboo_small_leaves")
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

/// Retrieve the hardcoded linear color for a block or texture stem, if any.
pub fn get_hardcoded_tint(name: &str) -> Option<[f32; 4]> {
    let clean = name.trim().to_ascii_lowercase();
    let unnamespaced = clean.strip_prefix("minecraft:").unwrap_or(&clean);
    let stem = unnamespaced.strip_prefix("block/").unwrap_or(unnamespaced);

    for &(k, hex) in HARDCODED_BLOCK_TINTS {
        if k == stem {
            return Some(hex_to_linear_rgba(hex));
        }
    }
    None
}

/// Classify a texture stem and/or block name into an authoritative tint category:
/// "grass", "foliage", "dry_foliage", "water", "hardcoded", or "none".
pub fn classify_tint_category(
    clean_stem: &str,
    block_name: Option<&str>,
    tint_index: Option<i32>,
) -> &'static str {
    let stem = clean_stem.trim().to_ascii_lowercase();
    let b_stem = block_name.map(|b| {
        let b_clean = b.trim().to_ascii_lowercase();
        let b_unname = b_clean.strip_prefix("minecraft:").unwrap_or(&b_clean);
        b_unname.strip_prefix("block/").unwrap_or(b_unname).to_string()
    });

    if stem.is_empty() && b_stem.is_none() {
        return "none";
    }

    // 1. Explicitly check blocks/textures that must NEVER receive tint (e.g. dead bush, untinted leaves)
    if is_explicit_none_tint(&stem) || b_stem.as_deref().map_or(false, is_explicit_none_tint) {
        return "none";
    }

    // 2. Check Hardcoded block tints (e.g. spruce leaves, birch leaves, lily pad)
    if get_hardcoded_tint(&stem).is_some() || b_stem.as_deref().and_then(get_hardcoded_tint).is_some() {
        return "hardcoded";
    }

    // 3. Dry foliage matching (HIGHEST PRIORITY over generic 'leaf' and 'grass')
    // Fixes leaf_litter and dry grass falsely classified as generic leaves/foliage or grass!
    if is_dry_foliage(&stem) || b_stem.as_deref().map_or(false, is_dry_foliage) {
        return "dry_foliage";
    }

    // 4. Grass matching (vanilla green bush, short grass, tall grass, fern, sugar cane)
    if is_grass(&stem) || b_stem.as_deref().map_or(false, is_grass) {
        return "grass";
    }

    // 5. Foliage matching (oak, jungle, acacia, dark oak, mangrove leaves, vine)
    if is_foliage(&stem) || b_stem.as_deref().map_or(false, is_foliage) {
        return "foliage";
    }

    // 6. Water matching
    if is_water(&stem) || b_stem.as_deref().map_or(false, is_water) {
        return "water";
    }

    // 7. Tintindex >= 0 rules fallback
    if let Some(ti) = tint_index {
        if ti >= 0 {
            if stem.contains("dry") || b_stem.as_deref().map_or(false, |b| b.contains("dry")) {
                return "dry_foliage";
            }
            if stem.contains("grass") || stem.contains("fern") || stem.contains("sugar_cane") || stem.contains("bush") {
                return "grass";
            }
            if stem.contains("leaf") || stem.contains("leaves") || stem.contains("vine") {
                return "foliage";
            }
            if stem.contains("water") {
                return "water";
            }
            return "grass";
        }
    }

    "none"
}

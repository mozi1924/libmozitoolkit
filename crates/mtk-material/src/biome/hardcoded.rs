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
    ("sugar_cane", "#91BD59"),
];

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

/// Classify a texture stem and/or block name into a tint category:
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

    if stem.contains("firefly_bush") || b_stem.as_deref().map_or(false, |b| b.contains("firefly_bush")) {
        return "none";
    }

    // 1. Check Hardcoded block tints
    if get_hardcoded_tint(&stem).is_some() || b_stem.as_deref().and_then(get_hardcoded_tint).is_some() {
        return "hardcoded";
    }

    // 2. Tintindex >= 0 rules
    if let Some(ti) = tint_index {
        if ti >= 0 {
            if stem.contains("grass") || stem.contains("fern") || stem.contains("sugar_cane") {
                return "grass";
            }
            if stem.contains("leaf") || stem.contains("leaves") || stem.contains("vine") || stem.contains("bush") {
                return "foliage";
            }
            if stem.contains("water") {
                return "water";
            }
            if stem.contains("dry_foliage") {
                return "dry_foliage";
            }
            return "foliage";
        }
    }

    // 3. Texture stem semantic matching
    if stem.contains("grass_block_top")
        || stem.contains("grass_block_side_overlay")
        || stem.contains("short_grass")
        || stem.contains("tall_grass_top")
        || stem.contains("tall_grass_bottom")
        || stem.contains("fern")
        || stem.contains("sugar_cane")
        || stem == "grass"
    {
        return "grass";
    }

    if stem.contains("oak_leaves")
        || stem.contains("jungle_leaves")
        || stem.contains("acacia_leaves")
        || stem.contains("dark_oak_leaves")
        || stem.contains("mangrove_leaves")
        || stem.contains("vine")
    {
        return "foliage";
    }

    if stem.contains("water_still") || stem.contains("water_flow") || stem == "water" {
        return "water";
    }

    if stem.contains("dry_foliage") || stem.contains("leaf_litter") {
        return "dry_foliage";
    }

    // 4. Block name semantic matching
    if let Some(ref b) = b_stem {
        if b.contains("grass_block") || b.contains("short_grass") || b.contains("tall_grass") || b.contains("fern") {
            return "grass";
        }
        if b.contains("leaves") || b.contains("vine") {
            return "foliage";
        }
        if b.contains("water") {
            return "water";
        }
    }

    "none"
}

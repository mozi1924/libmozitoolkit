//! # Dielectric Transmission and Refraction Evaluation

pub const TRANSMISSION_EXACT: &[&str] = &[
    "glass", "glass_pane", "tinted_glass",
    "white_stained_glass", "orange_stained_glass", "magenta_stained_glass",
    "light_blue_stained_glass", "yellow_stained_glass", "lime_stained_glass",
    "pink_stained_glass", "gray_stained_glass", "light_gray_stained_glass",
    "cyan_stained_glass", "purple_stained_glass", "blue_stained_glass",
    "brown_stained_glass", "green_stained_glass", "red_stained_glass",
    "black_stained_glass",
    "white_stained_glass_pane", "orange_stained_glass_pane", "magenta_stained_glass_pane",
    "light_blue_stained_glass_pane", "yellow_stained_glass_pane", "lime_stained_glass_pane",
    "pink_stained_glass_pane", "gray_stained_glass_pane", "light_gray_stained_glass_pane",
    "cyan_stained_glass_pane", "purple_stained_glass_pane", "blue_stained_glass_pane",
    "brown_stained_glass_pane", "green_stained_glass_pane", "red_stained_glass_pane",
    "black_stained_glass_pane",
    "water", "flowing_water", "water_still", "water_flow",
    "ice", "packed_ice", "blue_ice", "frosted_ice",
    "slime_block", "slime", "honey_block", "honey",
    "beacon",
];

pub const TRANSMISSION_KEYWORDS: &[&str] = &[
    "glass", "glass_pane", "stained_glass", "tinted_glass",
    "water", "ice", "frosted_ice", "slime", "honey", "beacon",
];

fn clean_name(s: &str) -> &str {
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

/// Checks if a block or texture is a transmissive / refractive dielectric (glass, water, ice, etc.).
pub fn is_transmissive_block(block_name: &str, texture_name: Option<&str>) -> bool {
    let clean_b = clean_name(block_name);
    let clean_t = texture_name.map(clean_name).unwrap_or("");

    for &ex in TRANSMISSION_EXACT {
        if clean_b == ex || clean_t == ex {
            return true;
        }
    }

    for &kw in TRANSMISSION_KEYWORDS {
        if clean_b.contains(kw) || (!clean_t.is_empty() && clean_t.contains(kw)) {
            if clean_b.contains("spyglass") || clean_t.contains("spyglass") {
                return false;
            }
            return true;
        }
    }

    false
}

/// Returns transmission weight: 1.0 for glass/water/ice, 0.0 otherwise.
pub fn get_block_transmission_weight(block_name: &str, texture_name: Option<&str>) -> f32 {
    if is_transmissive_block(block_name, texture_name) {
        1.0
    } else {
        0.0
    }
}

/// Returns sticker threshold above which pixels are treated as surface stickers / decals.
/// Water/ice/slime/honey body alpha is ~0.70-0.75, so threshold is 0.95.
/// Glass and stained glass body alpha is 0.0-0.45, so threshold is 0.55.
pub fn get_block_sticker_threshold(block_name: &str, texture_name: Option<&str>) -> f32 {
    let clean_b = clean_name(block_name);
    let clean_t = texture_name.map(clean_name).unwrap_or("");

    for &kw in &["water", "ice", "slime", "honey"] {
        if clean_b.contains(kw) || clean_t.contains(kw) {
            return 0.95;
        }
    }

    0.55
}

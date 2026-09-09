pub mod table;

use table::{
    MINEWAYS_ATLAS_NAME_PATTERNS, MINEWAYS_ATLAS_SUFFIX_PATTERNS, MINEWAYS_TILES,
    MINEWAYS_TILES_TABLE_MAX_ID,
};

/// Check if a texture image or material name corresponds to a Mineways exported terrain atlas.
pub fn is_mineways_atlas_name(name: &str) -> bool {
    let clean = name.trim().to_lowercase();
    let stem = clean
        .strip_suffix(".png")
        .or_else(|| clean.strip_suffix(".jpg"))
        .unwrap_or(&clean);

    // Strip blender duplicate suffixes (e.g. ".001", "_001")
    let stem = if let Some(idx) = stem.rfind('.') {
        if stem[idx + 1..].chars().all(|c| c.is_ascii_digit()) {
            &stem[..idx]
        } else {
            stem
        }
    } else {
        stem
    };

    for &pat in MINEWAYS_ATLAS_NAME_PATTERNS {
        if stem == pat || stem.starts_with(pat) {
            return true;
        }
    }

    for &suf in MINEWAYS_ATLAS_SUFFIX_PATTERNS {
        if stem.ends_with(suf) {
            return true;
        }
    }

    false
}

/// Lookup canonical primary and alternative texture names for a given Mineways swatch ID.
pub fn lookup_swatch(swatch_id: usize) -> Option<(&'static str, &'static str)> {
    if swatch_id <= MINEWAYS_TILES_TABLE_MAX_ID {
        if let Some((_, _, pri, alt)) = MINEWAYS_TILES[swatch_id] {
            let pri_clean = pri
                .strip_prefix("MWO_")
                .or_else(|| pri.strip_prefix("MW_"))
                .unwrap_or(pri);
            let alt_clean = alt
                .strip_prefix("MWO_")
                .or_else(|| alt.strip_prefix("MW_"))
                .unwrap_or(alt);
            return Some((pri_clean, alt_clean));
        }
    }
    None
}

/// Convert a single Mineways Atlas UV coordinate to its local [0, 1] swatch coordinate.
#[inline]
pub fn remap_mineways_atlas_uv_to_local(
    u: f32,
    v: f32,
    image_width: u32,
    image_height: u32,
) -> [f32; 2] {
    let swatch_size = 18.0f32;
    let tile_size = 16.0f32;
    let border = 1.0f32;

    let tex_w = image_width.max(1) as f32;
    let tex_h = image_height.max(1) as f32;

    let px = u * tex_w;
    let py = (1.0 - v) * tex_h;

    let atlas_col = (px / swatch_size).floor();
    let atlas_row = (py / swatch_size).floor();

    let lu = (px - (atlas_col * swatch_size + border)) / tile_size;
    let lv = 1.0 - ((py - (atlas_row * swatch_size + border)) / tile_size);

    [lu, lv]
}

/// Decode a polygon UV coordinate on a Mineways atlas to canonical texture names and local UVs.
pub fn decode_mineways_uv(
    u: f32,
    v: f32,
    image_width: u32,
    image_height: u32,
) -> (Option<&'static str>, Option<&'static str>, [f32; 2]) {
    let swatch_size = 18.0f32;
    let tex_w = image_width.max(1) as f32;
    let tex_h = image_height.max(1) as f32;
    let swatches_per_row = (image_width / 18).max(1);

    let px = u * tex_w;
    let py = (1.0 - v) * tex_h;

    let atlas_col = (px / swatch_size).floor() as u32;
    let atlas_row = (py / swatch_size).floor() as u32;
    let swatch_id = (atlas_col + atlas_row * swatches_per_row) as usize;

    let names = lookup_swatch(swatch_id);
    let (pri, alt) = match names {
        Some((p, a)) => (Some(p), if a.is_empty() { None } else { Some(a) }),
        None => (None, None),
    };

    let local_uv = remap_mineways_atlas_uv_to_local(u, v, image_width, image_height);
    (pri, alt, local_uv)
}

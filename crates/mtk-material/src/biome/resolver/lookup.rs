use super::super::hardcoded::{
    classify_tint_category, get_hardcoded_tint_hex, TINT_TYPE_DRY_FOLIAGE, TINT_TYPE_FOLIAGE,
    TINT_TYPE_GRASS, TINT_TYPE_HARDCODED, TINT_TYPE_WATER,
};
use super::super::palettes::hex_to_linear_rgba;
use super::types::{BiomeResolver, TintInfo};

impl BiomeResolver {
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

        // Special handling for dynamic Redstone Wire signal strength (power 0..15)
        let is_redstone = stem.starts_with("redstone_dust")
            || stem.starts_with("redstone_wire")
            || block_name
                .is_some_and(|b| b.contains("redstone_wire") || b.contains("redstone_dust"));

        if is_redstone && stem != "redstone_dust_overlay" {
            let power = block_name
                .and_then(|b| {
                    if let Some(pos) = b.find("power=") {
                        let sub = &b[pos + 6..];
                        let end = sub.find(&[',', ']', ' '][..]).unwrap_or(sub.len());
                        sub[..end].parse::<u8>().ok()
                    } else if b.ends_with("_on") || b.contains("lit=true") {
                        Some(15)
                    } else if b.ends_with("_off") || b.contains("lit=false") {
                        Some(0)
                    } else {
                        None
                    }
                })
                .or_else(|| {
                    if stem.ends_with("_on") {
                        Some(15)
                    } else if stem.ends_with("_off") {
                        Some(0)
                    } else {
                        None
                    }
                })
                .unwrap_or(0);

            let col = super::super::hardcoded::get_redstone_wire_color(power);
            let hex = super::super::hardcoded::get_redstone_wire_hex(power);
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

        // 3. Check Hardcoded block tints
        if let Some(hex) =
            get_hardcoded_tint_hex(stem).or_else(|| block_name.and_then(get_hardcoded_tint_hex))
        {
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
                let hex_opt = get_hardcoded_tint_hex(stem)
                    .or_else(|| block_name.and_then(get_hardcoded_tint_hex));
                let hc = self
                    .texture_hardcoded_colors
                    .get(stem)
                    .copied()
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
}

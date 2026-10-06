//! # Biome Tint Resolver for Meshing
//!
//! Computes per-face Biome Tint data, linear colors, and colormap UVs.

/// Computes biome tint data and linear color for a face.
pub fn compute_face_tint(
    texture_key: &str,
    block_name: &str,
    tint_index: i16,
    biome_resolver: Option<&mtk_material::BiomeResolver>,
) -> ([f32; 4], [f32; 4], [f32; 3]) {
    let default_pal = mtk_material::biome::get_biome_palette("plains");
    let default_uv = default_pal.colormap_uv();
    let colormap_uv_3 = [default_uv[0], default_uv[1], 0.0];

    // Check if texture has an overlay companion (e.g. grass_block_side has grass_block_side_overlay)
    let has_overlay = if let Some(resolver) = biome_resolver {
        resolver.get_overlay_texture(texture_key).is_some()
    } else {
        texture_key.ends_with("grass_block_side") || texture_key.ends_with("grass_side")
    };

    // In Minecraft rendering, any face with negative tint index is strictly untinted,
    // UNLESS it has an overlay companion whose overlay part must be tinted.
    if tint_index < 0 && !has_overlay {
        let packed_data = [1.0, 1.0, 0.0, 0.0];
        return (packed_data, [1.0, 1.0, 1.0, 1.0], colormap_uv_3);
    }

    let t_idx = Some(tint_index as i32);

    if let Some(resolver) = biome_resolver {
        let info = resolver.get_tint_info(texture_key, Some(block_name), t_idx);
        let tw = info.tint_weight;
        let bw = info.base_tint_weight;
        let ow = info.overlay_tint_weight;
        let tt = info.tint_type;
        let is_hc = info.is_hardcoded;

        let packed_data = [bw, ow, tw, tt as f32];
        let final_col = match tt {
            mtk_material::TINT_TYPE_GRASS => default_pal.grass_linear(),
            mtk_material::TINT_TYPE_FOLIAGE => default_pal.foliage_linear(),
            mtk_material::TINT_TYPE_DRY_FOLIAGE => default_pal.dry_foliage_linear(),
            mtk_material::TINT_TYPE_WATER => default_pal.water_linear(),
            mtk_material::TINT_TYPE_HARDCODED => {
                info.hardcoded_color.unwrap_or([1.0, 1.0, 1.0, 1.0])
            }
            _ => {
                if is_hc {
                    info.hardcoded_color.unwrap_or([1.0, 1.0, 1.0, 1.0])
                } else {
                    [1.0, 1.0, 1.0, 1.0]
                }
            }
        };
        (packed_data, final_col, colormap_uv_3)
    } else {
        // Fallback when no biome_resolver is supplied:
        let cat = if has_overlay {
            "grass"
        } else {
            mtk_material::classify_tint_category(texture_key, Some(block_name), t_idx)
        };
        if cat != "none" {
            let bw = if has_overlay { 0.0 } else { 1.0 };
            let (tt, tw, col) = match cat {
                "grass" => (
                    mtk_material::TINT_TYPE_GRASS,
                    1.0,
                    default_pal.grass_linear(),
                ),
                "foliage" => (
                    mtk_material::TINT_TYPE_FOLIAGE,
                    1.0,
                    default_pal.foliage_linear(),
                ),
                "dry_foliage" => (
                    mtk_material::TINT_TYPE_DRY_FOLIAGE,
                    1.0,
                    default_pal.dry_foliage_linear(),
                ),
                "water" => (
                    mtk_material::TINT_TYPE_WATER,
                    1.0,
                    default_pal.water_linear(),
                ),
                "hardcoded" => {
                    let hc = mtk_material::get_hardcoded_tint(texture_key)
                        .or_else(|| mtk_material::get_hardcoded_tint(block_name))
                        .unwrap_or([1.0, 1.0, 1.0, 1.0]);
                    (mtk_material::TINT_TYPE_HARDCODED, 1.0, hc)
                }
                _ => (mtk_material::TINT_TYPE_NONE, 0.0, [1.0, 1.0, 1.0, 1.0]),
            };
            let packed_data = [bw, 1.0, tw, tt as f32];
            (packed_data, col, colormap_uv_3)
        } else {
            let packed_data = [1.0, 1.0, 0.0, 0.0]; // no tint
            (packed_data, [1.0, 1.0, 1.0, 1.0], colormap_uv_3)
        }
    }
}

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

    if let Some(resolver) = biome_resolver {
        let t_idx = if tint_index >= 0 { Some(tint_index as i32) } else { None };
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
    } else if tint_index >= 0 {
        let packed_data = [1.0, 1.0, 1.0, 1.0]; // default grass tint
        (packed_data, default_pal.grass_linear(), colormap_uv_3)
    } else {
        let packed_data = [1.0, 1.0, 0.0, 0.0]; // no tint
        (packed_data, [1.0, 1.0, 1.0, 1.0], colormap_uv_3)
    }
}

//! High-Speed Batch Mesh Attribute Generation for Biome Tinting and Colormaps.

#[cfg(feature = "parallel")]
use rayon::prelude::*;

use super::palettes::{blend_biome_colors, get_biome_palette};
use super::hardcoded::{TINT_TYPE_DRY_FOLIAGE, TINT_TYPE_FOLIAGE, TINT_TYPE_GRASS, TINT_TYPE_HARDCODED, TINT_TYPE_WATER};
use super::resolver::BiomeResolver;

/// Result structure containing flat aligned arrays for direct Blender mesh attribute injection.
#[derive(Debug, Clone)]
pub struct MeshBiomeAttributesResult {
    /// `mtk_biome_tint_data`: `(base_tint_weight, overlay_tint_weight, tint_weight, tint_type)`
    pub packed_tint_data: Vec<[f32; 4]>,
    /// `mtk_biome_tint_color`: `(r, g, b, a)` Linear RGBA
    pub tint_colors: Vec<[f32; 4]>,
    /// `mtk_colormap_uv`: `(u, v, 0.0)`
    pub colormap_uvs: Vec<[f32; 3]>,
}

/// Compute biome tinting attributes in parallel across all mesh faces.
pub fn compute_mesh_biome_attributes(
    face_texture_keys: &[String],
    biome_name: &str,
    multi_biomes: Option<&[(String, f32)]>,
    resolver: &BiomeResolver,
) -> MeshBiomeAttributesResult {
    let face_count = face_texture_keys.len();
    if face_count == 0 {
        return MeshBiomeAttributesResult {
            packed_tint_data: Vec::new(),
            tint_colors: Vec::new(),
            colormap_uvs: Vec::new(),
        };
    }

    let is_multi_biome = multi_biomes.map_or(false, |mb| !mb.is_empty());

    // Compute standard global biome colors and colormap UVs
    let (grass_col, foliage_col, dry_foliage_col, water_col, base_uv, has_cg, has_cf, has_cdf) = if !is_multi_biome {
        let pal = get_biome_palette(biome_name);
        (
            pal.grass_linear(),
            pal.foliage_linear(),
            pal.dry_foliage_linear(),
            pal.water_linear(),
            pal.colormap_uv(),
            pal.has_custom_grass,
            pal.has_custom_foliage,
            pal.has_custom_dry_foliage,
        )
    } else {
        let mb = multi_biomes.unwrap();
        let weights: Vec<(&str, f32)> = mb.iter().map(|(n, w)| (n.as_str(), *w)).collect();
        let g = blend_biome_colors(&weights, "grass");
        let f = blend_biome_colors(&weights, "foliage");
        let df = blend_biome_colors(&weights, "dry_foliage");
        let w = blend_biome_colors(&weights, "water");

        let mut total_w = 0.0f32;
        let mut sum_u = 0.0f32;
        let mut sum_v = 0.0f32;
        for &(b_name, wt) in &weights {
            if wt <= 0.0 {
                continue;
            }
            let pal = get_biome_palette(b_name);
            let uv = pal.colormap_uv();
            total_w += wt;
            sum_u += uv[0] * wt;
            sum_v += uv[1] * wt;
        }
        let uv = if total_w > 0.0 {
            [sum_u / total_w, sum_v / total_w]
        } else {
            [0.2, 0.32]
        };
        (g, f, df, w, uv, false, false, false)
    };

    let colormap_uv_3 = [base_uv[0], base_uv[1], 0.0f32];

    let compute_face = |key: &String| -> ([f32; 4], [f32; 4], [f32; 3]) {
        let tint_info = resolver.get_tint_info(key, None, None);
        let tw = tint_info.tint_weight;
        let base_w = tint_info.base_tint_weight;
        let overlay_w = tint_info.overlay_tint_weight;
        let tt = tint_info.tint_type;
        let is_hc = tint_info.is_hardcoded;

        let has_custom = match tt {
            TINT_TYPE_GRASS => has_cg,
            TINT_TYPE_FOLIAGE => has_cf,
            TINT_TYPE_DRY_FOLIAGE => has_cdf,
            _ => false,
        };

        let tint_type_val = if is_hc || has_custom {
            TINT_TYPE_HARDCODED as f32
        } else {
            tt as f32
        };

        let packed_data = [base_w, overlay_w, tw, tint_type_val];

        let final_col = match tt {
            TINT_TYPE_GRASS => grass_col,
            TINT_TYPE_FOLIAGE => foliage_col,
            TINT_TYPE_DRY_FOLIAGE => dry_foliage_col,
            TINT_TYPE_WATER => water_col,
            TINT_TYPE_HARDCODED => tint_info.hardcoded_color.unwrap_or([1.0, 1.0, 1.0, 1.0]),
            _ => {
                if is_hc {
                    tint_info.hardcoded_color.unwrap_or([1.0, 1.0, 1.0, 1.0])
                } else {
                    [1.0, 1.0, 1.0, 1.0]
                }
            }
        };

        (packed_data, final_col, colormap_uv_3)
    };

    #[cfg(feature = "parallel")]
    let results: Vec<([f32; 4], [f32; 4], [f32; 3])> = face_texture_keys.par_iter().map(compute_face).collect();

    #[cfg(not(feature = "parallel"))]
    let results: Vec<([f32; 4], [f32; 4], [f32; 3])> = face_texture_keys.iter().map(compute_face).collect();

    let mut packed_tint_data = Vec::with_capacity(face_count);
    let mut tint_colors = Vec::with_capacity(face_count);
    let mut colormap_uvs = Vec::with_capacity(face_count);

    for (p, c, u) in results {
        packed_tint_data.push(p);
        tint_colors.push(c);
        colormap_uvs.push(u);
    }

    MeshBiomeAttributesResult {
        packed_tint_data,
        tint_colors,
        colormap_uvs,
    }
}

pub use mtk_material::get_colormap_uv;
use std::collections::HashMap;

/// Precomputed inverse distance weights for 2D horizontal kernel radius `R=2`.
pub const BIOME_KERNEL_R2: &[(i32, i32, f32)] = &[
    (-2, -2, 0.2612),
    (-2, -1, 0.3090),
    (-2, 0, 0.3333),
    (-2, 1, 0.3090),
    (-2, 2, 0.2612),
    (-1, -2, 0.3090),
    (-1, -1, 0.4142),
    (-1, 0, 0.5000),
    (-1, 1, 0.4142),
    (-1, 2, 0.3090),
    (0, -2, 0.3333),
    (0, -1, 0.5000),
    (0, 0, 1.0000),
    (0, 1, 0.5000),
    (0, 2, 0.3333),
    (1, -2, 0.3090),
    (1, -1, 0.4142),
    (1, 0, 0.5000),
    (1, 1, 0.4142),
    (1, 2, 0.3090),
    (2, -2, 0.2612),
    (2, -1, 0.3090),
    (2, 0, 0.3333),
    (2, 1, 0.3090),
    (2, 2, 0.2612),
];

/// Metadata and color properties for a single Minecraft biome (delegated to authoritative mtk-material).
#[derive(Debug, Clone, PartialEq)]
pub struct BiomeMeta {
    pub temperature: f32,
    pub humidity: f32,
    pub water_color_linear: [f32; 4],
}

impl Default for BiomeMeta {
    fn default() -> Self {
        let pal = mtk_material::get_biome_palette("plains");
        Self {
            temperature: pal.temperature,
            humidity: pal.humidity,
            water_color_linear: pal.water_linear(),
        }
    }
}

/// Retrieves the canonical temperature, humidity, and water color from authoritative mtk-material biome palettes.
pub fn get_biome_meta(biome_id: &str) -> BiomeMeta {
    let pal = mtk_material::get_biome_palette(biome_id);
    BiomeMeta {
        temperature: pal.temperature,
        humidity: pal.humidity,
        water_color_linear: pal.water_linear(),
    }
}

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Precomputed smoothed biome colormap UV and linear colors for a single 1D column (x, z).
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SmoothedBiomeColumn {
    pub colormap_uv: [f32; 2],
    pub grass_color: [f32; 4],
    pub foliage_color: [f32; 4],
    pub dry_foliage_color: [f32; 4],
    pub water_color: [f32; 4],
}

impl Default for SmoothedBiomeColumn {
    fn default() -> Self {
        let pal = mtk_material::get_biome_palette("plains");
        Self {
            colormap_uv: pal.colormap_uv(),
            grass_color: pal.grass_linear(),
            foliage_color: pal.foliage_linear(),
            dry_foliage_color: pal.dry_foliage_linear(),
            water_color: pal.water_linear(),
        }
    }
}

/// Computes smooth biome blending over `(x ± 2, z ± 2)` horizontal neighborhood for all color channels.
pub fn get_smoothed_column_biome<F>(mut get_biome: F, x: i32, y: i32, z: i32) -> SmoothedBiomeColumn
where
    F: FnMut(i32, i32, i32) -> String,
{
    let mut total_w = 0.0f32;
    let mut sum_u = 0.0f32;
    let mut sum_v = 0.0f32;
    let mut sum_grass = [0.0f32; 4];
    let mut sum_foliage = [0.0f32; 4];
    let mut sum_dry = [0.0f32; 4];
    let mut sum_water = [0.0f32; 4];

    let mut cache = HashMap::<String, (&'static mtk_material::BiomePalette, [f32; 2])>::new();

    for &(dx, dz, weight) in BIOME_KERNEL_R2 {
        let bx = x + dx;
        let bz = z + dz;
        let b_name = get_biome(bx, y, bz);

        let (pal, uv) = if let Some(&(pal, uv)) = cache.get(&b_name) {
            (pal, uv)
        } else {
            let pal = mtk_material::get_biome_palette(&b_name);
            let uv = mtk_material::get_colormap_uv(pal.temperature, pal.humidity);
            cache.insert(b_name, (pal, uv));
            (pal, uv)
        };

        total_w += weight;
        sum_u += uv[0] * weight;
        sum_v += uv[1] * weight;

        let gc = pal.grass_linear();
        sum_grass[0] += gc[0] * weight;
        sum_grass[1] += gc[1] * weight;
        sum_grass[2] += gc[2] * weight;
        sum_grass[3] += gc[3] * weight;

        let fc = pal.foliage_linear();
        sum_foliage[0] += fc[0] * weight;
        sum_foliage[1] += fc[1] * weight;
        sum_foliage[2] += fc[2] * weight;
        sum_foliage[3] += fc[3] * weight;

        let dc = pal.dry_foliage_linear();
        sum_dry[0] += dc[0] * weight;
        sum_dry[1] += dc[1] * weight;
        sum_dry[2] += dc[2] * weight;
        sum_dry[3] += dc[3] * weight;

        let wc = pal.water_linear();
        sum_water[0] += wc[0] * weight;
        sum_water[1] += wc[1] * weight;
        sum_water[2] += wc[2] * weight;
        sum_water[3] += wc[3] * weight;
    }

    let inv_w = if total_w > 0.0 { 1.0 / total_w } else { 1.0 };
    SmoothedBiomeColumn {
        colormap_uv: [sum_u * inv_w, sum_v * inv_w],
        grass_color: [
            sum_grass[0] * inv_w,
            sum_grass[1] * inv_w,
            sum_grass[2] * inv_w,
            sum_grass[3] * inv_w,
        ],
        foliage_color: [
            sum_foliage[0] * inv_w,
            sum_foliage[1] * inv_w,
            sum_foliage[2] * inv_w,
            sum_foliage[3] * inv_w,
        ],
        dry_foliage_color: [
            sum_dry[0] * inv_w,
            sum_dry[1] * inv_w,
            sum_dry[2] * inv_w,
            sum_dry[3] * inv_w,
        ],
        water_color: [
            sum_water[0] * inv_w,
            sum_water[1] * inv_w,
            sum_water[2] * inv_w,
            sum_water[3] * inv_w,
        ],
    }
}

/// Computes smooth biome blending over `(x ± 2, z ± 2)` horizontal neighborhood.
/// Returns `(smoothed_colormap_uv, smoothed_water_linear_rgba)`.
pub fn get_smoothed_biome_data<F>(get_biome: F, x: i32, y: i32, z: i32) -> ([f32; 2], [f32; 4])
where
    F: FnMut(i32, i32, i32) -> String,
{
    let col = get_smoothed_column_biome(get_biome, x, y, z);
    (col.colormap_uv, col.water_color)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_colormap_uv() {
        let uv_plains = get_colormap_uv(0.8, 0.4);
        assert!(uv_plains[0] >= 0.0 && uv_plains[0] <= 1.0);
        assert!(uv_plains[1] >= 0.0 && uv_plains[1] <= 1.0);
    }

    #[test]
    fn test_smoothed_biome() {
        let (uv, water_col) =
            get_smoothed_biome_data(|_x, _y, _z| "minecraft:plains".to_string(), 0, 64, 0);
        assert!(uv[0] > 0.0);
        assert!(water_col[3] > 0.0);
    }
}

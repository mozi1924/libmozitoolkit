use std::collections::HashMap;

/// Precomputed inverse distance weights for 2D horizontal kernel radius `R=2`.
pub const BIOME_KERNEL_R2: &[(i32, i32, f32)] = &[
    (-2, -2, 0.2612), (-2, -1, 0.3090), (-2, 0, 0.3333), (-2, 1, 0.3090), (-2, 2, 0.2612),
    (-1, -2, 0.3090), (-1, -1, 0.4142), (-1, 0, 0.5000), (-1, 1, 0.4142), (-1, 2, 0.3090),
    (0, -2, 0.3333),  (0, -1, 0.5000),  (0, 0, 1.0000),  (0, 1, 0.5000),  (0, 2, 0.3333),
    (1, -2, 0.3090),  (1, -1, 0.4142),  (1, 0, 0.5000),  (1, 1, 0.4142),  (1, 2, 0.3090),
    (2, -2, 0.2612),  (2, -1, 0.3090),  (2, 0, 0.3333),  (2, 1, 0.3090),  (2, 2, 0.2612),
];

/// Metadata and color properties for a single Minecraft biome.
#[derive(Debug, Clone, PartialEq)]
pub struct BiomeMeta {
    pub temperature: f32,
    pub humidity: f32,
    pub water_color_linear: [f32; 4],
}

impl Default for BiomeMeta {
    fn default() -> Self {
        Self {
            temperature: 0.8,
            humidity: 0.4,
            water_color_linear: [0.05, 0.17, 0.77, 0.8],
        }
    }
}

/// Retrieves the canonical temperature, humidity, and water color for a biome ID.
pub fn get_biome_meta(biome_id: &str) -> BiomeMeta {
    let clean = biome_id.strip_prefix("minecraft:").unwrap_or(biome_id);
    match clean {
        "desert" => BiomeMeta {
            temperature: 2.0,
            humidity: 0.0,
            water_color_linear: [0.04, 0.23, 0.88, 0.8],
        },
        "forest" => BiomeMeta {
            temperature: 0.7,
            humidity: 0.8,
            water_color_linear: [0.05, 0.17, 0.77, 0.8],
        },
        "taiga" | "snowy_plains" => BiomeMeta {
            temperature: 0.25,
            humidity: 0.8,
            water_color_linear: [0.05, 0.22, 0.80, 0.8],
        },
        "swamp" => BiomeMeta {
            temperature: 0.8,
            humidity: 0.9,
            water_color_linear: [0.15, 0.20, 0.12, 0.8],
        },
        "jungle" => BiomeMeta {
            temperature: 0.95,
            humidity: 0.9,
            water_color_linear: [0.04, 0.28, 0.85, 0.8],
        },
        "ocean" | "deep_ocean" => BiomeMeta {
            temperature: 0.5,
            humidity: 0.5,
            water_color_linear: [0.04, 0.12, 0.75, 0.8],
        },
        "warm_ocean" => BiomeMeta {
            temperature: 0.8,
            humidity: 0.5,
            water_color_linear: [0.01, 0.45, 0.70, 0.8],
        },
        _ => BiomeMeta::default(),
    }
}

/// Computes the standard Minecraft 2D colormap UV coordinate `[u, v]` from temperature and humidity.
///
/// Uses the canonical triangular clamping formula: `t = clamp(t, 0, 1)`, `h = clamp(h, 0, 1) * t`.
#[inline]
pub fn get_colormap_uv(temperature: f32, humidity: f32) -> [f32; 2] {
    let t = temperature.clamp(0.0, 1.0);
    let h = (humidity.clamp(0.0, 1.0)) * t;
    [1.0 - t, h]
}

/// Computes smooth biome blending over `(x ± 2, z ± 2)` horizontal neighborhood.
/// Returns `(smoothed_colormap_uv, smoothed_water_linear_rgba)`.
pub fn get_smoothed_biome_data<F>(
    mut get_biome: F,
    x: i32,
    y: i32,
    z: i32,
) -> ([f32; 2], [f32; 4])
where
    F: FnMut(i32, i32, i32) -> String,
{
    let mut total_w = 0.0f32;
    let mut sum_u = 0.0f32;
    let mut sum_v = 0.0f32;
    let mut sum_wr = 0.0f32;
    let mut sum_wg = 0.0f32;
    let mut sum_wb = 0.0f32;
    let mut sum_wa = 0.0f32;

    let mut cache = HashMap::<String, ([f32; 2], [f32; 4])>::new();

    for &(dx, dz, weight) in BIOME_KERNEL_R2 {
        let bx = x + dx;
        let bz = z + dz;
        let b_name = get_biome(bx, y, bz);

        let (uv, w_col) = if let Some(&cached) = cache.get(&b_name) {
            cached
        } else {
            let meta = get_biome_meta(&b_name);
            let uv = get_colormap_uv(meta.temperature, meta.humidity);
            let item = (uv, meta.water_color_linear);
            cache.insert(b_name, item);
            item
        };

        total_w += weight;
        sum_u += uv[0] * weight;
        sum_v += uv[1] * weight;
        sum_wr += w_col[0] * weight;
        sum_wg += w_col[1] * weight;
        sum_wb += w_col[2] * weight;
        sum_wa += w_col[3] * weight;
    }

    let inv_w = if total_w > 0.0 { 1.0 / total_w } else { 1.0 };
    (
        [sum_u * inv_w, sum_v * inv_w],
        [sum_wr * inv_w, sum_wg * inv_w, sum_wb * inv_w, sum_wa * inv_w],
    )
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
        let (uv, water_col) = get_smoothed_biome_data(|_x, _y, _z| "minecraft:plains".to_string(), 0, 64, 0);
        assert!(uv[0] > 0.0);
        assert!(water_col[3] > 0.0);
    }
}

//! Fluid height sampling, corner averaging, and flow vector physics.

use core::f32::consts::PI;
use mtk_cull::engine::parse_block_name_and_props;

pub use mtk_core::constants::fluid::MAX_FLUID_HEIGHT;

/// Identifies supported Minecraft fluid types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FluidType {
    Water,
    Lava,
}

impl FluidType {
    pub fn from_name(name: &str) -> Option<Self> {
        let (raw_name, _) = parse_block_name_and_props(name);
        let clean = raw_name.strip_prefix("minecraft:").unwrap_or(&raw_name);
        let clean = clean.strip_prefix("flowing_").unwrap_or(clean);
        if clean == "water" {
            Some(Self::Water)
        } else if clean == "lava" {
            Some(Self::Lava)
        } else {
            None
        }
    }

    pub fn name_str(&self) -> &'static str {
        match self {
            Self::Water => "water",
            Self::Lava => "lava",
        }
    }
}

/// Computes fluid height in `[0..1]` for a single fluid or waterlogged blockstate string.
pub fn get_fluid_base_height(state_str: &str) -> f32 {
    if state_str.is_empty() {
        return 0.0;
    }

    let (name, props) = parse_block_name_and_props(state_str);
    let clean = name.strip_prefix("minecraft:").unwrap_or(&name);
    let is_waterlogged = props.get("waterlogged").map(|v| v.as_str()) == Some("true");
    if is_waterlogged {
        return MAX_FLUID_HEIGHT;
    }

    let clean = clean.strip_prefix("flowing_").unwrap_or(clean);

    if clean != "water" && clean != "lava" {
        return 0.0;
    }

    let level_int: u32 = props.get("level").and_then(|l| l.parse().ok()).unwrap_or(0);

    if level_int >= 8 {
        MAX_FLUID_HEIGHT
    } else if level_int > 0 {
        (8.0 - level_int as f32) / 9.0
    } else {
        MAX_FLUID_HEIGHT
    }
}

/// Samples fluid height at `(x, y, z)` relative to neighborhood:
/// - Returns `1.0` if same fluid and block directly above is also fluid/submerged.
/// - Returns own height in `(0..1)` if same fluid or waterlogged.
/// - Returns `-1.0` if solid opaque block (JMC2OBJ boundary preservation, excluded from averaging).
/// - Returns `0.0` if air or non-solid / non-fluid block.
pub fn sample_fluid_height<F>(
    mut get_state: F,
    x: i32,
    y: i32,
    z: i32,
    fluid_type: FluidType,
) -> f32
where
    F: FnMut(i32, i32, i32) -> String,
{
    let state_str = get_state(x, y, z);
    if state_str.is_empty() || state_str == "minecraft:air" || state_str == "air" {
        return 0.0;
    }

    let (name, props) = parse_block_name_and_props(&state_str);
    let clean = name.strip_prefix("minecraft:").unwrap_or(&name);
    let is_waterlogged = props.get("waterlogged").map(|v| v.as_str()) == Some("true");
    let clean = clean.strip_prefix("flowing_").unwrap_or(clean);

    let is_fluid_match =
        (clean == fluid_type.name_str()) || (fluid_type == FluidType::Water && is_waterlogged);

    if is_fluid_match {
        // Check if block directly above is also the same fluid / waterlogged
        let above_state = get_state(x, y + 1, z);
        if !above_state.is_empty() && above_state != "minecraft:air" {
            let (a_name, a_props) = parse_block_name_and_props(&above_state);
            let a_clean = a_name.strip_prefix("minecraft:").unwrap_or(&a_name);
            let a_waterlogged = a_props.get("waterlogged").map(|v| v.as_str()) == Some("true");
            let a_clean = a_clean.strip_prefix("flowing_").unwrap_or(a_clean);

            if (a_clean == fluid_type.name_str())
                || (fluid_type == FluidType::Water && a_waterlogged)
            {
                return 1.0;
            }
        }

        if is_waterlogged && fluid_type == FluidType::Water {
            return MAX_FLUID_HEIGHT;
        }

        return get_fluid_base_height(&state_str);
    }

    // Solid opaque block detection: never classify waterlogged blocks as solid cube boundaries
    if is_waterlogged {
        return 0.0;
    }

    let is_solid_cube = mtk_model::baker::rules::is_opaque_block(clean)
        && !mtk_model::baker::rules::is_known_non_cube(clean, false);

    if is_solid_cube {
        return -1.0;
    }

    0.0
}

/// Calculates the averaged fluid height for one corner between center, two adjacent neighbors,
/// and the diagonal neighbor using canonical Minecraft fluid mechanics.
pub fn calculate_corner_average(
    h_center: f32,
    h_adj1: f32,
    h_adj2: f32,
    h_diag: f32,
    is_source: bool,
) -> f32 {
    if h_center >= 1.0 || h_adj1 >= 1.0 || h_adj2 >= 1.0 {
        return 1.0;
    }

    // Still source block surface tension: maintain 8/9 unless bordering lower fluid
    if is_source {
        let fluid_samples: Vec<f32> = [h_adj1, h_adj2, h_diag]
            .iter()
            .copied()
            .filter(|&h| h > 0.0)
            .collect();
        if fluid_samples.is_empty() || fluid_samples.iter().all(|&h| h >= 0.8) {
            return MAX_FLUID_HEIGHT;
        }
    }

    let mut weighted_sum = 0.0f32;
    let mut total_weight = 0.0f32;

    // 1. Center fluid
    if h_center >= 0.8 {
        weighted_sum += h_center * 10.0;
        total_weight += 10.0;
    } else if h_center >= 0.0 {
        weighted_sum += h_center * 1.0;
        total_weight += 1.0;
    }

    // 2. Adjacent 1
    if h_adj1 >= 0.8 {
        weighted_sum += h_adj1 * 10.0;
        total_weight += 10.0;
    } else if h_adj1 >= 0.0 {
        weighted_sum += h_adj1 * 1.0;
        total_weight += 1.0;
    }

    // 3. Adjacent 2
    if h_adj2 >= 0.8 {
        weighted_sum += h_adj2 * 10.0;
        total_weight += 10.0;
    } else if h_adj2 >= 0.0 {
        weighted_sum += h_adj2 * 1.0;
        total_weight += 1.0;
    }

    // 4. Diagonal (only if at least one adjacent neighbor is fluid)
    if (h_adj1 > 0.0 || h_adj2 > 0.0) && h_diag >= 0.0 {
        if h_diag >= 1.0 {
            return 1.0;
        }
        if h_diag >= 0.8 {
            weighted_sum += h_diag * 10.0;
            total_weight += 10.0;
        } else {
            weighted_sum += h_diag * 1.0;
            total_weight += 1.0;
        }
    }

    if total_weight > 0.0 {
        weighted_sum / total_weight
    } else {
        h_center.max(0.0)
    }
}

/// Computes 4 corner heights `(c_NW, c_NE, c_SE, c_SW)` in `[0..1]` for the fluid block at `(x, y, z)`.
/// North is -Z, South is +Z, West is -X, East is +X.
pub fn calculate_fluid_corner_heights<F>(
    mut get_state: F,
    x: i32,
    y: i32,
    z: i32,
    fluid_type: FluidType,
) -> (f32, f32, f32, f32)
where
    F: FnMut(i32, i32, i32) -> String,
{
    // Submerged check
    let above_state = get_state(x, y + 1, z);
    if !above_state.is_empty() && above_state != "minecraft:air" {
        let (a_name, a_props) = parse_block_name_and_props(&above_state);
        let a_clean = a_name.strip_prefix("minecraft:").unwrap_or(&a_name);
        let a_waterlogged = a_props.get("waterlogged").map(|v| v.as_str()) == Some("true");
        let a_clean = a_clean.strip_prefix("flowing_").unwrap_or(a_clean);

        if (a_clean == fluid_type.name_str()) || (fluid_type == FluidType::Water && a_waterlogged) {
            return (1.0, 1.0, 1.0, 1.0);
        }
    }

    let state_str = get_state(x, y, z);
    let (_, props) = parse_block_name_and_props(&state_str);
    let is_source = props.get("waterlogged").map(|v| v.as_str()) == Some("true")
        || (!state_str.contains("flowing_")
            && props.get("level").map(|l| l.as_str()).unwrap_or("0") == "0");

    let h_center = sample_fluid_height(&mut get_state, x, y, z, fluid_type);
    let h_n = sample_fluid_height(&mut get_state, x, y, z - 1, fluid_type);
    let h_s = sample_fluid_height(&mut get_state, x, y, z + 1, fluid_type);
    let h_e = sample_fluid_height(&mut get_state, x + 1, y, z, fluid_type);
    let h_w = sample_fluid_height(&mut get_state, x - 1, y, z, fluid_type);

    let h_ne = sample_fluid_height(&mut get_state, x + 1, y, z - 1, fluid_type);
    let h_nw = sample_fluid_height(&mut get_state, x - 1, y, z - 1, fluid_type);
    let h_se = sample_fluid_height(&mut get_state, x + 1, y, z + 1, fluid_type);
    let h_sw = sample_fluid_height(&mut get_state, x - 1, y, z + 1, fluid_type);

    let c_nw = calculate_corner_average(h_center, h_n, h_w, h_nw, is_source);
    let c_ne = calculate_corner_average(h_center, h_n, h_e, h_ne, is_source);
    let c_se = calculate_corner_average(h_center, h_s, h_e, h_se, is_source);
    let c_sw = calculate_corner_average(h_center, h_s, h_w, h_sw, is_source);

    (c_nw, c_ne, c_se, c_sw)
}

/// Calculates horizontal flow direction vector `(vx, vz)` and UV rotation angle in radians.
pub fn calculate_fluid_flow_vector<F>(
    mut get_state: F,
    x: i32,
    y: i32,
    z: i32,
    fluid_type: FluidType,
    own_height: f32,
) -> (f32, f32, f32)
where
    F: FnMut(i32, i32, i32) -> String,
{
    let mut vx = 0.0f32;
    let mut vz = 0.0f32;

    for (dx, dz) in [(0, -1), (0, 1), (-1, 0), (1, 0)] {
        let nx = x + dx;
        let nz = z + dz;
        let n_state = get_state(nx, y, nz);

        if n_state.is_empty() || n_state == "minecraft:air" {
            // Check block below
            let below_state = get_state(nx, y - 1, nz);
            if !below_state.is_empty() && below_state != "minecraft:air" {
                let (b_name, b_props) = parse_block_name_and_props(&below_state);
                let b_clean = b_name.strip_prefix("minecraft:").unwrap_or(&b_name);
                let b_waterlogged = b_props.get("waterlogged").map(|v| v.as_str()) == Some("true");
                let b_clean = b_clean.strip_prefix("flowing_").unwrap_or(b_clean);

                if (b_clean == fluid_type.name_str())
                    || (fluid_type == FluidType::Water && b_waterlogged)
                {
                    let b_h = get_fluid_base_height(&below_state);
                    let diff = own_height - (b_h - MAX_FLUID_HEIGHT);
                    vx += dx as f32 * diff;
                    vz += dz as f32 * diff;
                }
            }
        } else {
            let (n_name, n_props) = parse_block_name_and_props(&n_state);
            let n_clean = n_name.strip_prefix("minecraft:").unwrap_or(&n_name);
            let n_waterlogged = n_props.get("waterlogged").map(|v| v.as_str()) == Some("true");
            let n_clean = n_clean.strip_prefix("flowing_").unwrap_or(n_clean);

            if (n_clean == fluid_type.name_str())
                || (fluid_type == FluidType::Water && n_waterlogged)
            {
                let n_h = get_fluid_base_height(&n_state);
                let diff = own_height - n_h;
                if diff.abs() > 1e-4 {
                    vx += dx as f32 * diff;
                    vz += dz as f32 * diff;
                }
            }
        }
    }

    let flow_len = (vx * vx + vz * vz).sqrt();
    if flow_len < 1e-4 {
        return (0.0, 0.0, 0.0);
    }

    let flow_angle = vz.atan2(vx) - (PI / 2.0);
    (vx, vz, flow_angle)
}

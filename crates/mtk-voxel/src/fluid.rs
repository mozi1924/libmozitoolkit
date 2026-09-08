use core::f32::consts::PI;
use glam::Vec3;
use mtk_core::direction::Direction;
use mtk_core::mesh::MeshData;
use mtk_cull::engine::parse_block_name_and_props;
use mtk_cull::FaceCuller;

use crate::fluid_uv::{get_fluid_side_uvs, get_fluid_top_uvs};
use crate::types::MesherConfig;

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
    if props.get("waterlogged").map(|v| v.as_str()) == Some("true") {
        return MAX_FLUID_HEIGHT;
    }

    let clean = name.strip_prefix("minecraft:").unwrap_or(&name);
    let clean = clean.strip_prefix("flowing_").unwrap_or(clean);

    if clean != "water" && clean != "lava" {
        return 0.0;
    }

    let level_int: u32 = props
        .get("level")
        .and_then(|l| l.parse().ok())
        .unwrap_or(0);

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
pub fn sample_fluid_height<F>(mut get_state: F, x: i32, y: i32, z: i32, fluid_type: FluidType) -> f32
where
    F: FnMut(i32, i32, i32) -> String,
{
    let state_str = get_state(x, y, z);
    if state_str.is_empty() || state_str == "minecraft:air" || state_str == "air" {
        return 0.0;
    }

    let (name, props) = parse_block_name_and_props(&state_str);
    let is_waterlogged = props.get("waterlogged").map(|v| v.as_str()) == Some("true");
    let clean = name.strip_prefix("minecraft:").unwrap_or(&name);
    let clean = clean.strip_prefix("flowing_").unwrap_or(clean);

    let is_fluid_match =
        (clean == fluid_type.name_str()) || (fluid_type == FluidType::Water && is_waterlogged);

    if is_fluid_match {
        // Check if block directly above is also the same fluid / waterlogged
        let above_state = get_state(x, y + 1, z);
        if !above_state.is_empty() && above_state != "minecraft:air" {
            let (a_name, a_props) = parse_block_name_and_props(&above_state);
            let a_waterlogged = a_props.get("waterlogged").map(|v| v.as_str()) == Some("true");
            let a_clean = a_name.strip_prefix("minecraft:").unwrap_or(&a_name);
            let a_clean = a_clean.strip_prefix("flowing_").unwrap_or(a_clean);

            if (a_clean == fluid_type.name_str()) || (fluid_type == FluidType::Water && a_waterlogged) {
                return 1.0;
            }
        }

        if is_waterlogged && fluid_type == FluidType::Water {
            return MAX_FLUID_HEIGHT;
        }

        return get_fluid_base_height(&state_str);
    }

    // Solid opaque block detection
    let is_solid_cube = match clean {
        "stone" | "dirt" | "grass_block" | "cobblestone" | "sand" | "gravel" | "oak_planks"
        | "spruce_planks" | "birch_planks" | "deepslate" | "bedrock" | "obsidian" | "netherrack"
        | "end_stone" => true,
        _ => !clean.contains("air") && !clean.contains("sapling") && !clean.contains("flower"),
    };

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
        let a_waterlogged = a_props.get("waterlogged").map(|v| v.as_str()) == Some("true");
        let a_clean = a_name.strip_prefix("minecraft:").unwrap_or(&a_name);
        let a_clean = a_clean.strip_prefix("flowing_").unwrap_or(a_clean);

        if (a_clean == fluid_type.name_str()) || (fluid_type == FluidType::Water && a_waterlogged) {
            return (1.0, 1.0, 1.0, 1.0);
        }
    }

    let state_str = get_state(x, y, z);
    let (_, props) = parse_block_name_and_props(&state_str);
    let is_source = props.get("waterlogged").map(|v| v.as_str()) == Some("true")
        || !state_str.contains("flowing_")
            && props.get("level").map(|l| l.as_str()).unwrap_or("0") == "0";

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
                let b_waterlogged = b_props.get("waterlogged").map(|v| v.as_str()) == Some("true");
                let b_clean = b_name.strip_prefix("minecraft:").unwrap_or(&b_name);
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
            let n_waterlogged = n_props.get("waterlogged").map(|v| v.as_str()) == Some("true");
            let n_clean = n_name.strip_prefix("minecraft:").unwrap_or(&n_name);
            let n_clean = n_clean.strip_prefix("flowing_").unwrap_or(n_clean);

            if (n_clean == fluid_type.name_str()) || (fluid_type == FluidType::Water && n_waterlogged) {
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

/// Emits complete fluid geometry (top, bottom, and 4 sides) for a block into `MeshData`.
pub fn emit_fluid_geometry<F>(
    mesh: &mut MeshData,
    x: i32,
    y: i32,
    z: i32,
    wx: f32,
    wy: f32,
    wz: f32,
    state_str: &str,
    mut get_state: F,
    culler: &FaceCuller,
    config: &MesherConfig,
    material_slot: u16,
) -> usize
where
    F: FnMut(i32, i32, i32) -> String,
{
    let fluid_type = match FluidType::from_name(state_str) {
        Some(ft) => ft,
        None => return 0,
    };

    let (c_nw, c_ne, c_se, c_sw) =
        calculate_fluid_corner_heights(&mut get_state, x, y, z, fluid_type);
    let own_height = get_fluid_base_height(state_str);
    let (flow_vx, flow_vz, flow_angle) =
        calculate_fluid_flow_vector(&mut get_state, x, y, z, fluid_type, own_height);
    let is_flowing = flow_vx.abs() > 1e-4 || flow_vz.abs() > 1e-4 || state_str.contains("flowing_");

    let own_meta = culler.get_meta(state_str, None, None);

    let mut faces_emitted = 0;

    // Helper to emit quad face
    let mut emit_quad = |verts: [Vec3; 4], uvs: [[f32; 2]; 4], norm: Vec3, _dir: Direction| {
        let base_idx = mesh.positions.len() as u32;
        let n = [norm.x, norm.y, norm.z];

        for i in 0..4 {
            let p = config.transform_coord(verts[i]);
            mesh.positions.push([p.x, p.y, p.z]);
            mesh.normals.push(n);
            mesh.uvs.push(uvs[i]);
        }

        mesh.indices.push(base_idx);
        mesh.indices.push(base_idx + 1);
        mesh.indices.push(base_idx + 2);

        mesh.indices.push(base_idx);
        mesh.indices.push(base_idx + 2);
        mesh.indices.push(base_idx + 3);

        mesh.face_materials.push(material_slot);
        mesh.face_tint_indices
            .push(if fluid_type == FluidType::Water { 0 } else { -1 });
        faces_emitted += 1;
    };

    // 1. TOP FACE (UP)
    let up_state = get_state(x, y + 1, z);
    let up_meta = if !up_state.is_empty() && up_state != "minecraft:air" {
        Some(culler.get_meta(&up_state, None, None))
    } else {
        None
    };

    if culler.should_render_face(
        &own_meta,
        up_meta.as_deref(),
        Direction::Up,
        None,
        None,
        None,
    ) {
        let v_nw = Vec3::new(wx, wy + c_nw, wz);
        let v_sw = Vec3::new(wx, wy + c_sw, wz + 1.0);
        let v_se = Vec3::new(wx + 1.0, wy + c_se, wz + 1.0);
        let v_ne = Vec3::new(wx + 1.0, wy + c_ne, wz);

        let top_uvs = get_fluid_top_uvs(is_flowing, if is_flowing { flow_angle } else { 0.0 });
        emit_quad([v_nw, v_sw, v_se, v_ne], top_uvs, Vec3::Y, Direction::Up);
    }

    // 2. BOTTOM FACE (DOWN)
    let down_state = get_state(x, y - 1, z);
    let down_meta = if !down_state.is_empty() && down_state != "minecraft:air" {
        Some(culler.get_meta(&down_state, None, None))
    } else {
        None
    };

    if culler.should_render_face(
        &own_meta,
        down_meta.as_deref(),
        Direction::Down,
        None,
        None,
        None,
    ) {
        let v_sw = Vec3::new(wx, wy, wz + 1.0);
        let v_nw = Vec3::new(wx, wy, wz);
        let v_ne = Vec3::new(wx + 1.0, wy, wz);
        let v_se = Vec3::new(wx + 1.0, wy, wz + 1.0);

        let bot_uvs = get_fluid_top_uvs(false, 0.0);
        emit_quad([v_sw, v_nw, v_ne, v_se], bot_uvs, -Vec3::Y, Direction::Down);
    }

    // 3. SIDE FACES (North, South, West, East)
    // North (-Z)
    let n_state = get_state(x, y, z - 1);
    let n_meta = if !n_state.is_empty() && n_state != "minecraft:air" {
        Some(culler.get_meta(&n_state, None, None))
    } else {
        None
    };
    if culler.should_render_face(
        &own_meta,
        n_meta.as_deref(),
        Direction::North,
        None,
        None,
        None,
    ) {
        let v_ne_top = Vec3::new(wx + 1.0, wy + c_ne, wz);
        let v_ne_bot = Vec3::new(wx + 1.0, wy, wz);
        let v_nw_bot = Vec3::new(wx, wy, wz);
        let v_nw_top = Vec3::new(wx, wy + c_nw, wz);
        let uvs = get_fluid_side_uvs(c_ne, c_nw);
        emit_quad(
            [v_ne_top, v_ne_bot, v_nw_bot, v_nw_top],
            uvs,
            -Vec3::Z,
            Direction::North,
        );
    }

    // South (+Z)
    let s_state = get_state(x, y, z + 1);
    let s_meta = if !s_state.is_empty() && s_state != "minecraft:air" {
        Some(culler.get_meta(&s_state, None, None))
    } else {
        None
    };
    if culler.should_render_face(
        &own_meta,
        s_meta.as_deref(),
        Direction::South,
        None,
        None,
        None,
    ) {
        let v_sw_top = Vec3::new(wx, wy + c_sw, wz + 1.0);
        let v_sw_bot = Vec3::new(wx, wy, wz + 1.0);
        let v_se_bot = Vec3::new(wx + 1.0, wy, wz + 1.0);
        let v_se_top = Vec3::new(wx + 1.0, wy + c_se, wz + 1.0);
        let uvs = get_fluid_side_uvs(c_sw, c_se);
        emit_quad(
            [v_sw_top, v_sw_bot, v_se_bot, v_se_top],
            uvs,
            Vec3::Z,
            Direction::South,
        );
    }

    // West (-X)
    let w_state = get_state(x - 1, y, z);
    let w_meta = if !w_state.is_empty() && w_state != "minecraft:air" {
        Some(culler.get_meta(&w_state, None, None))
    } else {
        None
    };
    if culler.should_render_face(
        &own_meta,
        w_meta.as_deref(),
        Direction::West,
        None,
        None,
        None,
    ) {
        let v_nw_top = Vec3::new(wx, wy + c_nw, wz);
        let v_nw_bot = Vec3::new(wx, wy, wz);
        let v_sw_bot = Vec3::new(wx, wy, wz + 1.0);
        let v_sw_top = Vec3::new(wx, wy + c_sw, wz + 1.0);
        let uvs = get_fluid_side_uvs(c_nw, c_sw);
        emit_quad(
            [v_nw_top, v_nw_bot, v_sw_bot, v_sw_top],
            uvs,
            -Vec3::X,
            Direction::West,
        );
    }

    // East (+X)
    let e_state = get_state(x + 1, y, z);
    let e_meta = if !e_state.is_empty() && e_state != "minecraft:air" {
        Some(culler.get_meta(&e_state, None, None))
    } else {
        None
    };
    if culler.should_render_face(
        &own_meta,
        e_meta.as_deref(),
        Direction::East,
        None,
        None,
        None,
    ) {
        let v_se_top = Vec3::new(wx + 1.0, wy + c_se, wz + 1.0);
        let v_se_bot = Vec3::new(wx + 1.0, wy, wz + 1.0);
        let v_ne_bot = Vec3::new(wx + 1.0, wy, wz);
        let v_ne_top = Vec3::new(wx + 1.0, wy + c_ne, wz);
        let uvs = get_fluid_side_uvs(c_se, c_ne);
        emit_quad(
            [v_se_top, v_se_bot, v_ne_bot, v_ne_top],
            uvs,
            Vec3::X,
            Direction::East,
        );
    }

    faces_emitted
}

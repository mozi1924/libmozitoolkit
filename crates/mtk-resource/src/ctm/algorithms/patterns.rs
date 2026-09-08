use glam::IVec3;
use mtk_core::direction::Direction;
use crate::ctm::tables::OVERLAY_17_LOOKUP;
use crate::ctm::types::{coordinate_random, extract_block_name, get_face_tangents, CtmSymmetry};

pub fn solve_repeat(
    face: Direction,
    world_pos: IVec3,
    width: u32,
    height: u32,
) -> Option<usize> {
    if width == 0 || height == 0 {
        return None;
    }
    let (up, left) = get_face_tangents(face);
    let u = match left {
        Direction::East | Direction::West => world_pos.x,
        Direction::Up | Direction::Down => world_pos.y,
        Direction::North | Direction::South => world_pos.z,
    }
    .rem_euclid(width as i32) as u32;

    let v = match up {
        Direction::East | Direction::West => world_pos.x,
        Direction::Up | Direction::Down => world_pos.y,
        Direction::North | Direction::South => world_pos.z,
    }
    .rem_euclid(height as i32) as u32;

    Some((v * width + u) as usize)
}

pub fn solve_random<F, S>(
    state: &str,
    face: Direction,
    world_pos: IVec3,
    weights: &[f32],
    total_weight: f32,
    symmetry: CtmSymmetry,
    linked: bool,
    num_tiles: usize,
    get_block: &F,
) -> usize
where
    F: Fn(IVec3) -> Option<S>,
    S: AsRef<str>,
{
    if num_tiles == 0 {
        return 0;
    }
    let (rx, ry, rz) = match symmetry {
        CtmSymmetry::None => (
            face.offset().x * 26,
            face.offset().y * 26,
            face.offset().z * 26,
        ),
        CtmSymmetry::Opposite => {
            let rand_dir = match face {
                Direction::South => Direction::North,
                Direction::West => Direction::East,
                Direction::Down => Direction::Up,
                other => other,
            };
            (
                rand_dir.offset().x * 26,
                rand_dir.offset().y * 26,
                rand_dir.offset().z * 26,
            )
        }
        CtmSymmetry::All => (0, 0, 0),
    };
    let mut qy = world_pos.y + ry;
    if linked {
        if let Some(below_state) = get_block(world_pos + IVec3::new(0, -1, 0)) {
            if extract_block_name(state) == extract_block_name(below_state.as_ref()) {
                qy -= 1;
            }
        }
    }
    let rand_val = coordinate_random(world_pos.x + rx, qy, world_pos.z + rz);
    if weights.is_empty() || total_weight <= 0.0 {
        return ((rand_val * (num_tiles as f32)) as usize).min(num_tiles - 1);
    }
    let target = rand_val * total_weight;
    let mut accum = 0.0f32;
    let mut chosen_idx = 0;
    for (i, &w) in weights.iter().enumerate().take(num_tiles) {
        accum += w;
        if target < accum {
            chosen_idx = i;
            break;
        }
    }
    chosen_idx
}

pub fn solve_overlay<F>(
    face: Direction,
    check_connect: &F,
) -> Option<usize>
where
    F: Fn(IVec3) -> bool,
{
    let (up, left) = get_face_tangents(face);
    let down = up.opposite();
    let right = left.opposite();

    let mut bits = 0u8;
    if check_connect(up.offset() + left.offset()) {
        bits |= 1 << 7;
    }
    if check_connect(up.offset()) {
        bits |= 1 << 6;
    }
    if check_connect(up.offset() + right.offset()) {
        bits |= 1 << 5;
    }
    if check_connect(right.offset()) {
        bits |= 1 << 4;
    }
    if check_connect(down.offset() + right.offset()) {
        bits |= 1 << 3;
    }
    if check_connect(down.offset()) {
        bits |= 1 << 2;
    }
    if check_connect(down.offset() + left.offset()) {
        bits |= 1 << 1;
    }
    if check_connect(left.offset()) {
        bits |= 1;
    }

    let tile_idx = OVERLAY_17_LOOKUP[bits as usize];
    if tile_idx >= 0 {
        Some(tile_idx as usize)
    } else {
        None
    }
}

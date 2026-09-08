use glam::IVec3;
use mtk_core::direction::Direction;
use crate::ctm::types::get_face_tangents;

pub fn solve_horizontal<F>(face: Direction, check_connect: &F) -> usize
where
    F: Fn(IVec3) -> bool,
{
    let (_up, left) = get_face_tangents(face);
    let right = left.opposite();
    let left_c = check_connect(left.offset());
    let right_c = check_connect(right.offset());

    match (left_c, right_c) {
        (true, true) => 1,
        (true, false) => 2,
        (false, true) => 0,
        (false, false) => 3,
    }
}

pub fn solve_vertical<F>(face: Direction, check_connect: &F) -> usize
where
    F: Fn(IVec3) -> bool,
{
    let (up, _left) = get_face_tangents(face);
    let down = up.opposite();
    let up_c = check_connect(up.offset());
    let down_c = check_connect(down.offset());

    match (up_c, down_c) {
        (true, true) => 1,
        (true, false) => 0,
        (false, true) => 2,
        (false, false) => 3,
    }
}

pub fn solve_horizontal_vertical<F>(face: Direction, check_connect: &F) -> usize
where
    F: Fn(IVec3) -> bool,
{
    let (up, left) = get_face_tangents(face);
    let down = up.opposite();
    let right = left.opposite();

    let left_c = check_connect(left.offset());
    let right_c = check_connect(right.offset());
    let mut tile_idx = match (left_c, right_c) {
        (true, true) => 1,
        (true, false) => 2,
        (false, true) => 0,
        (false, false) => 3,
    };
    if tile_idx == 3 {
        let up_c = check_connect(up.offset());
        let down_c = check_connect(down.offset());
        tile_idx = match (up_c, down_c) {
            (true, true) => 5,
            (true, false) => 4,
            (false, true) => 6,
            (false, false) => 3,
        };
    }
    tile_idx
}

pub fn solve_vertical_horizontal<F>(face: Direction, check_connect: &F) -> usize
where
    F: Fn(IVec3) -> bool,
{
    let (up, left) = get_face_tangents(face);
    let down = up.opposite();
    let right = left.opposite();

    let up_c = check_connect(up.offset());
    let down_c = check_connect(down.offset());
    let mut tile_idx = match (up_c, down_c) {
        (true, true) => 1,
        (true, false) => 0,
        (false, true) => 2,
        (false, false) => 3,
    };
    if tile_idx == 3 {
        let left_c = check_connect(left.offset());
        let right_c = check_connect(right.offset());
        tile_idx = match (left_c, right_c) {
            (true, true) => 5,
            (true, false) => 6,
            (false, true) => 4,
            (false, false) => 3,
        };
    }
    tile_idx
}

pub fn solve_top<F>(face: Direction, check_connect: &F) -> bool
where
    F: Fn(IVec3) -> bool,
{
    let (up, _left) = get_face_tangents(face);
    check_connect(up.offset())
}

use glam::IVec3;
use mtk_core::direction::Direction;
use crate::ctm::types::get_face_tangents;

/// Solves 5-tile compact CTM pattern.
pub fn solve_compact_ctm<F>(
    face: Direction,
    check_connect: &F,
) -> usize
where
    F: Fn(IVec3) -> bool,
{
    let (up, left) = get_face_tangents(face);
    let down = up.opposite();
    let right = left.opposite();

    let up_c = check_connect(up.offset());
    let down_c = check_connect(down.offset());
    let left_c = check_connect(left.offset());
    let right_c = check_connect(right.offset());

    match (up_c || down_c, left_c || right_c) {
        (true, true) => 4,
        (true, false) => 2,
        (false, true) => 3,
        (false, false) => 0,
    }
}

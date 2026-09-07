use alloc::vec::Vec;
use mtk_core::geometry::Aabb2d;

const EPS: f32 = 1e-5;

/// Subtracts `occluder` rectangle from `source` rectangle.
/// Returns a list of disjoint rectangles representing `source \ occluder`.
///
/// If `occluder` does not intersect `source`, returns `vec![source]`.
/// If `occluder` completely covers `source`, returns `vec![]`.
pub fn subtract_rect(source: &Aabb2d, occluder: &Aabb2d) -> Vec<Aabb2d> {
    if !source.intersects(occluder) {
        return alloc::vec![*source];
    }

    // If occluder completely covers source, nothing remains
    if occluder.contains_rect(source, EPS) {
        return Vec::new();
    }

    let mut result = Vec::with_capacity(4);

    // Bounding bounds of the intersection
    let inter_min_x = source.min.x.max(occluder.min.x);
    let inter_max_x = source.max.x.min(occluder.max.x);
    let inter_min_y = source.min.y.max(occluder.min.y);
    let inter_max_y = source.max.y.min(occluder.max.y);

    // 1. Bottom slice: [source.min.x .. source.max.x] x [source.min.y .. inter_min_y]
    if inter_min_y - source.min.y > EPS {
        result.push(Aabb2d::from_min_max(
            source.min.x,
            source.min.y,
            source.max.x,
            inter_min_y,
        ));
    }

    // 2. Top slice: [source.min.x .. source.max.x] x [inter_max_y .. source.max.y]
    if source.max.y - inter_max_y > EPS {
        result.push(Aabb2d::from_min_max(
            source.min.x,
            inter_max_y,
            source.max.x,
            source.max.y,
        ));
    }

    // 3. Left slice: [source.min.x .. inter_min_x] x [inter_min_y .. inter_max_y]
    if inter_min_x - source.min.x > EPS {
        result.push(Aabb2d::from_min_max(
            source.min.x,
            inter_min_y,
            inter_min_x,
            inter_max_y,
        ));
    }

    // 4. Right slice: [inter_max_x .. source.max.x] x [inter_min_y .. inter_max_y]
    if source.max.x - inter_max_x > EPS {
        result.push(Aabb2d::from_min_max(
            inter_max_x,
            inter_min_y,
            source.max.x,
            inter_max_y,
        ));
    }

    result
}

/// Iteratively subtracts multiple occluder rectangles from a source rectangle.
pub fn subtract_rect_multi(source: &Aabb2d, occluders: &[Aabb2d]) -> Vec<Aabb2d> {
    let mut current_pieces = alloc::vec![*source];

    for occ in occluders {
        if current_pieces.is_empty() {
            break;
        }
        let mut next_pieces = Vec::new();
        for piece in &current_pieces {
            let sub = subtract_rect(piece, occ);
            next_pieces.extend(sub);
        }
        current_pieces = next_pieces;
    }

    current_pieces
}

/// Checks if a source rectangle is completely covered by a sequence of occluders.
pub fn is_fully_occluded(source: &Aabb2d, occluders: &[Aabb2d]) -> bool {
    let remaining = subtract_rect_multi(source, occluders);
    remaining.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subtract_no_intersection() {
        let s = Aabb2d::from_min_max(0.0, 0.0, 0.5, 0.5);
        let o = Aabb2d::from_min_max(0.6, 0.6, 1.0, 1.0);
        let res = subtract_rect(&s, &o);
        assert_eq!(res.len(), 1);
        assert_eq!(res[0], s);
    }

    #[test]
    fn test_subtract_full_cover() {
        let s = Aabb2d::from_min_max(0.2, 0.2, 0.8, 0.8);
        let o = Aabb2d::from_min_max(0.0, 0.0, 1.0, 1.0);
        let res = subtract_rect(&s, &o);
        assert!(res.is_empty());
        assert!(is_fully_occluded(&s, &[o]));
    }

    #[test]
    fn test_subtract_half_slab() {
        // Bottom slab: [0..1] x [0..0.5] occluded by a bottom slab
        let s = Aabb2d::from_min_max(0.0, 0.0, 1.0, 1.0);
        let o = Aabb2d::from_min_max(0.0, 0.0, 1.0, 0.5);
        let res = subtract_rect(&s, &o);
        assert_eq!(res.len(), 1);
        // Remaining should be top half [0..1] x [0.5..1.0]
        assert!((res[0].min.y - 0.5).abs() < 1e-5);
        assert!((res[0].max.y - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_subtract_two_halves_fully_occluded() {
        let s = Aabb2d::from_min_max(0.0, 0.0, 1.0, 1.0);
        let o1 = Aabb2d::from_min_max(0.0, 0.0, 1.0, 0.5);
        let o2 = Aabb2d::from_min_max(0.0, 0.5, 1.0, 1.0);
        assert!(is_fully_occluded(&s, &[o1, o2]));
    }
}

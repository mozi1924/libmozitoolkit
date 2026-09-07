use alloc::vec::Vec;
use glam::Vec3;
use mtk_core::direction::Direction;
use mtk_core::geometry::Aabb2d;

use crate::types::{is_empty_rect, is_full_rect};

const EPS: f32 = 1e-4;

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

/// Checks if a single source rectangle is completely covered by a sequence of occluders.
#[inline]
pub fn is_fully_occluded(source: &Aabb2d, occluders: &[Aabb2d]) -> bool {
    let remaining = subtract_rect_multi(source, occluders);
    remaining.is_empty()
}

/// Checks if all rectangles in `target_rects` are completely covered by `neighbor_occluders`.
///
/// Returns `true` if target is 100% occluded (should be culled), `false` if any part remains visible.
/// Equivalent to Minecraft `Shapes.joinIsNotEmpty(targetShape, occluderShape, BooleanOp.ONLY_FIRST) == false`.
pub fn is_face_completely_occluded(
    target_rects: &[Aabb2d],
    neighbor_occluders: &[Aabb2d],
) -> bool {
    if target_rects.is_empty() {
        return true;
    }
    if neighbor_occluders.is_empty() {
        return false;
    }

    // Fast path: check if any neighbor occluder is full
    for occ in neighbor_occluders {
        if is_full_rect(occ, EPS) {
            return true;
        }
    }

    // Detailed 2D polygon subtraction for each target piece
    for target in target_rects {
        if is_empty_rect(target, EPS) {
            continue;
        }
        let mut pieces = alloc::vec![*target];
        for occ in neighbor_occluders {
            if is_empty_rect(occ, EPS) {
                continue;
            }
            let mut next_pieces = Vec::new();
            for p in &pieces {
                next_pieces.extend(subtract_rect(p, occ));
            }
            pieces = next_pieces;
            if pieces.is_empty() {
                break;
            }
        }
        // If any fragment of this target rect remains unoccluded, the face is visible!
        if !pieces.is_empty() {
            return false;
        }
    }

    true
}

/// Extract 2D `Aabb2d` on the outer plane for a single quad face's vertices in [0..1] space.
/// If the quad lies on the boundary plane corresponding to `direction`, returns `Some(Aabb2d)`;
/// otherwise returns `None`.
pub fn extract_quad_face_occlusion_rect(
    vertices: &[Vec3; 4],
    direction: Direction,
) -> Option<Aabb2d> {
    let mut min_x = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    let mut min_z = f32::INFINITY;
    let mut max_z = f32::NEG_INFINITY;

    for v in vertices {
        min_x = min_x.min(v.x);
        max_x = max_x.max(v.x);
        min_y = min_y.min(v.y);
        max_y = max_y.max(v.y);
        min_z = min_z.min(v.z);
        max_z = max_z.max(v.z);
    }

    match direction {
        Direction::East => {
            // Outer plane at X=1
            if (max_x - 1.0).abs() <= EPS && (min_x - 1.0).abs() <= EPS {
                let rect = Aabb2d::from_min_max(min_z, min_y, max_z, max_y);
                if !is_empty_rect(&rect, EPS) {
                    return Some(rect);
                }
            }
        }
        Direction::West => {
            // Outer plane at X=0
            if min_x.abs() <= EPS && max_x.abs() <= EPS {
                let rect = Aabb2d::from_min_max(min_z, min_y, max_z, max_y);
                if !is_empty_rect(&rect, EPS) {
                    return Some(rect);
                }
            }
        }
        Direction::Up => {
            // Outer plane at Y=1
            if (max_y - 1.0).abs() <= EPS && (min_y - 1.0).abs() <= EPS {
                let rect = Aabb2d::from_min_max(min_x, min_z, max_x, max_z);
                if !is_empty_rect(&rect, EPS) {
                    return Some(rect);
                }
            }
        }
        Direction::Down => {
            // Outer plane at Y=0
            if min_y.abs() <= EPS && max_y.abs() <= EPS {
                let rect = Aabb2d::from_min_max(min_x, min_z, max_x, max_z);
                if !is_empty_rect(&rect, EPS) {
                    return Some(rect);
                }
            }
        }
        Direction::South => {
            // Outer plane at Z=1
            if (max_z - 1.0).abs() <= EPS && (min_z - 1.0).abs() <= EPS {
                let rect = Aabb2d::from_min_max(min_x, min_y, max_x, max_y);
                if !is_empty_rect(&rect, EPS) {
                    return Some(rect);
                }
            }
        }
        Direction::North => {
            // Outer plane at Z=0
            if min_z.abs() <= EPS && max_z.abs() <= EPS {
                let rect = Aabb2d::from_min_max(min_x, min_y, max_x, max_y);
                if !is_empty_rect(&rect, EPS) {
                    return Some(rect);
                }
            }
        }
    }

    None
}

/// Derive 2D face occlusion rectangles on an outer boundary plane from 3D axis-aligned cuboids `[min_pos, max_pos]`.
pub fn extract_face_occlusion_from_boxes(
    elements_boxes: &[([f32; 3], [f32; 3])],
    direction: Direction,
) -> Vec<Aabb2d> {
    let mut rects = Vec::new();

    for (mins, maxs) in elements_boxes {
        let x0 = mins[0];
        let y0 = mins[1];
        let z0 = mins[2];
        let x1 = maxs[0];
        let y1 = maxs[1];
        let z1 = maxs[2];

        match direction {
            Direction::East => {
                if (x1 - 1.0).abs() <= EPS {
                    rects.push(Aabb2d::from_min_max(z0, y0, z1, y1));
                }
            }
            Direction::West => {
                if x0.abs() <= EPS {
                    rects.push(Aabb2d::from_min_max(z0, y0, z1, y1));
                }
            }
            Direction::Up => {
                if (y1 - 1.0).abs() <= EPS {
                    rects.push(Aabb2d::from_min_max(x0, z0, x1, z1));
                }
            }
            Direction::Down => {
                if y0.abs() <= EPS {
                    rects.push(Aabb2d::from_min_max(x0, z0, x1, z1));
                }
            }
            Direction::South => {
                if (z1 - 1.0).abs() <= EPS {
                    rects.push(Aabb2d::from_min_max(x0, y0, x1, y1));
                }
            }
            Direction::North => {
                if z0.abs() <= EPS {
                    rects.push(Aabb2d::from_min_max(x0, y0, x1, y1));
                }
            }
        }
    }

    rects.retain(|r| !is_empty_rect(r, EPS));
    rects
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
        assert!((res[0].min.y - 0.5).abs() < 1e-4);
        assert!((res[0].max.y - 1.0).abs() < 1e-4);
    }

    #[test]
    fn test_subtract_two_halves_fully_occluded() {
        let s = Aabb2d::from_min_max(0.0, 0.0, 1.0, 1.0);
        let o1 = Aabb2d::from_min_max(0.0, 0.0, 1.0, 0.5);
        let o2 = Aabb2d::from_min_max(0.0, 0.5, 1.0, 1.0);
        assert!(is_face_completely_occluded(&[s], &[o1, o2]));
    }

    #[test]
    fn test_extract_quad_face_occlusion_rect() {
        // Top face of a bottom half slab (Y=0.5 plane) -> Not on Y=1 outer boundary -> None
        let slab_top_inner = [
            Vec3::new(0.0, 0.5, 0.0),
            Vec3::new(0.0, 0.5, 1.0),
            Vec3::new(1.0, 0.5, 1.0),
            Vec3::new(1.0, 0.5, 0.0),
        ];
        assert!(extract_quad_face_occlusion_rect(&slab_top_inner, Direction::Up).is_none());

        // Bottom face of a bottom half slab (Y=0.0 plane) -> Full 2D face on boundary
        let slab_bottom_outer = [
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 1.0),
        ];
        let rect = extract_quad_face_occlusion_rect(&slab_bottom_outer, Direction::Down);
        assert!(rect.is_some());
        assert!(is_full_rect(&rect.unwrap(), 1e-4));
    }
}


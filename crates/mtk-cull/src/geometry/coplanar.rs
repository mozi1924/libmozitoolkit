//! Geometric coplanar relationship and face overlap deduction.
//!
//! Provides unified algorithms for coplanar face detection, 2D tangent plane
//! projection, and bounding box overlap classification (Exact, Contained, Partial).

use glam::Vec3;

/// Face normal alignment relationship between two coplanar faces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaceAlignment {
    /// Faces point in approximately the same direction (dot product > 0.99).
    SameDirection,
    /// Faces point in approximately opposite directions (dot product < -0.99).
    OppositeDirection,
}

/// 2D Projected overlap relationship between two coplanar faces.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CoplanarOverlap {
    /// Faces have identical projected 2D bounds within tolerance (exact duplicate).
    Exact,
    /// Face B is completely covered by / contained within Face A.
    ContainedInA,
    /// Face A is completely covered by / contained within Face B.
    ContainedInB,
    /// Partial overlap with intersection area.
    Partial { inter_area: f32 },
}

/// Geometric coplanar relationship between two 3D faces.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoplanarRelation {
    pub alignment: FaceAlignment,
    pub overlap: CoplanarOverlap,
}

/// Checks the geometric coplanar relationship and 2D projected overlap
/// between two 3D planar polygons (e.g. Quads or Triangles).
///
/// Returns `None` if the faces are not coplanar, not parallel, or have negligible area.
pub fn check_coplanar_overlap(
    verts_a: &[Vec3],
    norm_a: Vec3,
    verts_b: &[Vec3],
    norm_b: Vec3,
    tol: f32,
) -> Option<CoplanarRelation> {
    if verts_a.is_empty() || verts_b.is_empty() {
        return None;
    }

    let dot = norm_a.dot(norm_b);
    let is_same_dir = dot > 0.99;
    let is_opp_dir = dot < -0.99;
    if !is_same_dir && !is_opp_dir {
        return None;
    }

    let alignment = if is_same_dir {
        FaceAlignment::SameDirection
    } else {
        FaceAlignment::OppositeDirection
    };

    // Calculate center points
    let mut center_a = Vec3::ZERO;
    for v in verts_a {
        center_a += *v;
    }
    center_a /= verts_a.len() as f32;

    let mut center_b = Vec3::ZERO;
    for v in verts_b {
        center_b += *v;
    }
    center_b /= verts_b.len() as f32;

    // Check distance between planes along normal
    let plane_dist = ((center_b - center_a).dot(norm_a)).abs();
    if plane_dist > tol {
        return None;
    }

    // Compute 2D tangent basis (u_axis, v_axis)
    let up = if norm_a.y.abs() > 0.9 {
        Vec3::Z
    } else {
        Vec3::Y
    };
    let mut u_axis = up.cross(norm_a);
    let u_len = u_axis.length();
    if u_len < 1e-5 {
        u_axis = Vec3::X;
    } else {
        u_axis /= u_len;
    }
    let v_axis = norm_a.cross(u_axis).normalize();

    // Project verts_a to 2D
    let (mut u_min_a, mut u_max_a) = (f32::INFINITY, f32::NEG_INFINITY);
    let (mut v_min_a, mut v_max_a) = (f32::INFINITY, f32::NEG_INFINITY);
    for v in verts_a {
        let u = v.dot(u_axis);
        let vc = v.dot(v_axis);
        u_min_a = u_min_a.min(u);
        u_max_a = u_max_a.max(u);
        v_min_a = v_min_a.min(vc);
        v_max_a = v_max_a.max(vc);
    }
    let area_a = (u_max_a - u_min_a) * (v_max_a - v_min_a);
    if area_a <= 1e-6 {
        return None;
    }

    // Project verts_b to 2D
    let (mut u_min_b, mut u_max_b) = (f32::INFINITY, f32::NEG_INFINITY);
    let (mut v_min_b, mut v_max_b) = (f32::INFINITY, f32::NEG_INFINITY);
    for v in verts_b {
        let u = v.dot(u_axis);
        let vc = v.dot(v_axis);
        u_min_b = u_min_b.min(u);
        u_max_b = u_max_b.max(u);
        v_min_b = v_min_b.min(vc);
        v_max_b = v_max_b.max(vc);
    }
    let area_b = (u_max_b - u_min_b) * (v_max_b - v_min_b);
    if area_b <= 1e-6 {
        return None;
    }

    let inter_u_min = u_min_a.max(u_min_b);
    let inter_u_max = u_max_a.min(u_max_b);
    let inter_v_min = v_min_a.max(v_min_b);
    let inter_v_max = v_max_a.min(v_max_b);

    if inter_u_max > inter_u_min + tol && inter_v_max > inter_v_min + tol {
        let inter_area = (inter_u_max - inter_u_min) * (inter_v_max - inter_v_min);
        let is_exact = (u_min_a - u_min_b).abs() <= tol
            && (u_max_a - u_max_b).abs() <= tol
            && (v_min_a - v_min_b).abs() <= tol
            && (v_max_a - v_max_b).abs() <= tol;

        let overlap = if is_exact {
            CoplanarOverlap::Exact
        } else if inter_area >= area_b * 0.99 - tol {
            CoplanarOverlap::ContainedInA
        } else if inter_area >= area_a * 0.99 - tol {
            CoplanarOverlap::ContainedInB
        } else {
            CoplanarOverlap::Partial { inter_area }
        };

        Some(CoplanarRelation { alignment, overlap })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_same_direction() {
        let verts_a = [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let verts_b = verts_a;
        let norm = Vec3::Z;

        let rel = check_coplanar_overlap(&verts_a, norm, &verts_b, norm, 1e-3).unwrap();
        assert_eq!(rel.alignment, FaceAlignment::SameDirection);
        assert_eq!(rel.overlap, CoplanarOverlap::Exact);
    }

    #[test]
    fn test_exact_opposite_direction() {
        let verts_a = [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let verts_b = verts_a;

        let rel = check_coplanar_overlap(&verts_a, Vec3::Z, &verts_b, -Vec3::Z, 1e-3).unwrap();
        assert_eq!(rel.alignment, FaceAlignment::OppositeDirection);
        assert_eq!(rel.overlap, CoplanarOverlap::Exact);
    }

    #[test]
    fn test_contained_in_a() {
        let verts_a = [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(2.0, 2.0, 0.0),
            Vec3::new(0.0, 2.0, 0.0),
        ];
        let verts_b = [
            Vec3::new(0.5, 0.5, 0.0),
            Vec3::new(1.5, 0.5, 0.0),
            Vec3::new(1.5, 1.5, 0.0),
            Vec3::new(0.5, 1.5, 0.0),
        ];

        let rel = check_coplanar_overlap(&verts_a, Vec3::Z, &verts_b, Vec3::Z, 1e-3).unwrap();
        assert_eq!(rel.overlap, CoplanarOverlap::ContainedInA);
    }
}

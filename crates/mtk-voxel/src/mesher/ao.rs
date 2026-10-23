use glam::Vec3;
use mtk_core::constants::lighting::AO_LEVEL_MULTIPLIERS;
use mtk_core::direction::Direction;

/// Maps ambient occlusion discrete level `0..=3` to a linear brightness multiplier `[0.2..1.0]`.
#[inline]
pub fn ao_level_to_brightness(level: u8) -> f32 {
    let idx = (level as usize).min(AO_LEVEL_MULTIPLIERS.len() - 1);
    AO_LEVEL_MULTIPLIERS[idx]
}

/// Computes the ambient occlusion level `0..=3` for a single vertex given its 2 adjacent side blocks and diagonal corner block.
///
/// If both adjacent side blocks are solid opaque, light is completely blocked (corner is occluded, AO = 0).
#[inline]
pub fn vertex_ao(side1_opaque: bool, side2_opaque: bool, corner_opaque: bool) -> u8 {
    if side1_opaque && side2_opaque {
        0
    } else {
        3 - (side1_opaque as u8 + side2_opaque as u8 + corner_opaque as u8)
    }
}

/// Determines if a baked quad face is inset from the outer boundary of its direction.
///
/// For example, dirt path top face has Y = 15/16 < 1.0, slab top face has Y = 0.5 < 1.0.
#[inline]
pub fn is_face_inset(dir: Direction, vertices: &[Vec3; 4]) -> bool {
    const EPSILON: f32 = 1e-4;
    match dir {
        Direction::Up => vertices.iter().all(|v| v.y < 1.0 - EPSILON),
        Direction::Down => vertices.iter().all(|v| v.y > EPSILON),
        Direction::North => vertices.iter().all(|v| v.z > EPSILON),
        Direction::South => vertices.iter().all(|v| v.z < 1.0 - EPSILON),
        Direction::West => vertices.iter().all(|v| v.x > EPSILON),
        Direction::East => vertices.iter().all(|v| v.x < 1.0 - EPSILON),
    }
}

/// Computes 4-corner AO levels `[ao0, ao1, ao2, ao3]` for a face facing `dir`.
///
/// When `is_inset` is true (e.g. dirt path, slab, stairs whose face is inside the 1x1x1 unit cube),
/// light occlusion is checked against both the in-plane neighbors (offset = 0) and exterior neighbors.
///
/// Vertex ordering strictly matches `UNIT_CUBE_CORNERS` and `BakedFace.vertices` [v0, v1, v2, v3].
pub fn calculate_face_ao_with_inset<F>(dir: Direction, is_inset: bool, mut is_opaque: F) -> [u8; 4]
where
    F: FnMut(i32, i32, i32) -> bool,
{
    match dir {
        Direction::Up => {
            // v0: (-X, -Z)
            // v1: (-X, +Z)
            // v2: (+X, +Z)
            // v3: (+X, -Z)
            let (side_w, side_e, side_n, side_s, c_nw, c_sw, c_ne, c_se) = if is_inset {
                (
                    is_opaque(-1, 0, 0) || is_opaque(-1, 1, 0),
                    is_opaque(1, 0, 0) || is_opaque(1, 1, 0),
                    is_opaque(0, 0, -1) || is_opaque(0, 1, -1),
                    is_opaque(0, 0, 1) || is_opaque(0, 1, 1),
                    is_opaque(-1, 0, -1) || is_opaque(-1, 1, -1),
                    is_opaque(-1, 0, 1) || is_opaque(-1, 1, 1),
                    is_opaque(1, 0, -1) || is_opaque(1, 1, -1),
                    is_opaque(1, 0, 1) || is_opaque(1, 1, 1),
                )
            } else {
                (
                    is_opaque(-1, 1, 0),
                    is_opaque(1, 1, 0),
                    is_opaque(0, 1, -1),
                    is_opaque(0, 1, 1),
                    is_opaque(-1, 1, -1),
                    is_opaque(-1, 1, 1),
                    is_opaque(1, 1, -1),
                    is_opaque(1, 1, 1),
                )
            };

            let ao0 = vertex_ao(side_w, side_n, c_nw);
            let ao1 = vertex_ao(side_w, side_s, c_sw);
            let ao2 = vertex_ao(side_e, side_s, c_se);
            let ao3 = vertex_ao(side_e, side_n, c_ne);
            [ao0, ao1, ao2, ao3]
        }
        Direction::Down => {
            // v0: (-X, +Z)
            // v1: (-X, -Z)
            // v2: (+X, -Z)
            // v3: (+X, +Z)
            let (side_w, side_e, side_n, side_s, c_nw, c_sw, c_ne, c_se) = if is_inset {
                (
                    is_opaque(-1, 0, 0) || is_opaque(-1, -1, 0),
                    is_opaque(1, 0, 0) || is_opaque(1, -1, 0),
                    is_opaque(0, 0, -1) || is_opaque(0, -1, -1),
                    is_opaque(0, 0, 1) || is_opaque(0, -1, 1),
                    is_opaque(-1, 0, -1) || is_opaque(-1, -1, -1),
                    is_opaque(-1, 0, 1) || is_opaque(-1, -1, 1),
                    is_opaque(1, 0, -1) || is_opaque(1, -1, -1),
                    is_opaque(1, 0, 1) || is_opaque(1, -1, 1),
                )
            } else {
                (
                    is_opaque(-1, -1, 0),
                    is_opaque(1, -1, 0),
                    is_opaque(0, -1, -1),
                    is_opaque(0, -1, 1),
                    is_opaque(-1, -1, -1),
                    is_opaque(-1, -1, 1),
                    is_opaque(1, -1, -1),
                    is_opaque(1, -1, 1),
                )
            };

            let ao0 = vertex_ao(side_w, side_s, c_sw);
            let ao1 = vertex_ao(side_w, side_n, c_nw);
            let ao2 = vertex_ao(side_e, side_n, c_ne);
            let ao3 = vertex_ao(side_e, side_s, c_se);
            [ao0, ao1, ao2, ao3]
        }
        Direction::North => {
            // Face at Z = 0 (looking towards -Z)
            // v0: (+X, +Y, -Z)
            // v1: (+X, -Y, -Z)
            // v2: (-X, -Y, -Z)
            // v3: (-X, +Y, -Z)
            let (side_e, side_w, side_up, side_dn, c_eu, c_ed, c_wd, c_wu) = if is_inset {
                (
                    is_opaque(1, 0, 0) || is_opaque(1, 0, -1),
                    is_opaque(-1, 0, 0) || is_opaque(-1, 0, -1),
                    is_opaque(0, 1, 0) || is_opaque(0, 1, -1),
                    is_opaque(0, -1, 0) || is_opaque(0, -1, -1),
                    is_opaque(1, 1, 0) || is_opaque(1, 1, -1),
                    is_opaque(1, -1, 0) || is_opaque(1, -1, -1),
                    is_opaque(-1, -1, 0) || is_opaque(-1, -1, -1),
                    is_opaque(-1, 1, 0) || is_opaque(-1, 1, -1),
                )
            } else {
                (
                    is_opaque(1, 0, -1),
                    is_opaque(-1, 0, -1),
                    is_opaque(0, 1, -1),
                    is_opaque(0, -1, -1),
                    is_opaque(1, 1, -1),
                    is_opaque(1, -1, -1),
                    is_opaque(-1, -1, -1),
                    is_opaque(-1, 1, -1),
                )
            };

            let ao0 = vertex_ao(side_e, side_up, c_eu);
            let ao1 = vertex_ao(side_e, side_dn, c_ed);
            let ao2 = vertex_ao(side_w, side_dn, c_wd);
            let ao3 = vertex_ao(side_w, side_up, c_wu);
            [ao0, ao1, ao2, ao3]
        }
        Direction::South => {
            // Face at Z = +1 (looking towards +Z)
            // v0: (-X, +Y, +Z)
            // v1: (-X, -Y, +Z)
            // v2: (+X, -Y, +Z)
            // v3: (+X, +Y, +Z)
            let (side_w, side_e, side_up, side_dn, c_wu, c_wd, c_ed, c_eu) = if is_inset {
                (
                    is_opaque(-1, 0, 0) || is_opaque(-1, 0, 1),
                    is_opaque(1, 0, 0) || is_opaque(1, 0, 1),
                    is_opaque(0, 1, 0) || is_opaque(0, 1, 1),
                    is_opaque(0, -1, 0) || is_opaque(0, -1, 1),
                    is_opaque(-1, 1, 0) || is_opaque(-1, 1, 1),
                    is_opaque(-1, -1, 0) || is_opaque(-1, -1, 1),
                    is_opaque(1, -1, 0) || is_opaque(1, -1, 1),
                    is_opaque(1, 1, 0) || is_opaque(1, 1, 1),
                )
            } else {
                (
                    is_opaque(-1, 0, 1),
                    is_opaque(1, 0, 1),
                    is_opaque(0, 1, 1),
                    is_opaque(0, -1, 1),
                    is_opaque(-1, 1, 1),
                    is_opaque(-1, -1, 1),
                    is_opaque(1, -1, 1),
                    is_opaque(1, 1, 1),
                )
            };

            let ao0 = vertex_ao(side_w, side_up, c_wu);
            let ao1 = vertex_ao(side_w, side_dn, c_wd);
            let ao2 = vertex_ao(side_e, side_dn, c_ed);
            let ao3 = vertex_ao(side_e, side_up, c_eu);
            [ao0, ao1, ao2, ao3]
        }
        Direction::West => {
            // Face at X = 0 (looking towards -X)
            // v0: (-X, +Y, -Z)
            // v1: (-X, -Y, -Z)
            // v2: (-X, -Y, +Z)
            // v3: (-X, +Y, +Z)
            let (side_n, side_s, side_up, side_dn, c_nu, c_nd, c_sd, c_su) = if is_inset {
                (
                    is_opaque(0, 0, -1) || is_opaque(-1, 0, -1),
                    is_opaque(0, 0, 1) || is_opaque(-1, 0, 1),
                    is_opaque(0, 1, 0) || is_opaque(-1, 1, 0),
                    is_opaque(0, -1, 0) || is_opaque(-1, -1, 0),
                    is_opaque(0, 1, -1) || is_opaque(-1, 1, -1),
                    is_opaque(0, -1, -1) || is_opaque(-1, -1, -1),
                    is_opaque(0, -1, 1) || is_opaque(-1, -1, 1),
                    is_opaque(0, 1, 1) || is_opaque(-1, 1, 1),
                )
            } else {
                (
                    is_opaque(-1, 0, -1),
                    is_opaque(-1, 0, 1),
                    is_opaque(-1, 1, 0),
                    is_opaque(-1, -1, 0),
                    is_opaque(-1, 1, -1),
                    is_opaque(-1, -1, -1),
                    is_opaque(-1, -1, 1),
                    is_opaque(-1, 1, 1),
                )
            };

            let ao0 = vertex_ao(side_n, side_up, c_nu);
            let ao1 = vertex_ao(side_n, side_dn, c_nd);
            let ao2 = vertex_ao(side_s, side_dn, c_sd);
            let ao3 = vertex_ao(side_s, side_up, c_su);
            [ao0, ao1, ao2, ao3]
        }
        Direction::East => {
            // Face at X = +1 (looking towards +X)
            // v0: (+X, +Y, +Z)
            // v1: (+X, -Y, +Z)
            // v2: (+X, -Y, -Z)
            // v3: (+X, +Y, -Z)
            let (side_s, side_n, side_up, side_dn, c_su, c_sd, c_nd, c_nu) = if is_inset {
                (
                    is_opaque(0, 0, 1) || is_opaque(1, 0, 1),
                    is_opaque(0, 0, -1) || is_opaque(1, 0, -1),
                    is_opaque(0, 1, 0) || is_opaque(1, 1, 0),
                    is_opaque(0, -1, 0) || is_opaque(1, -1, 0),
                    is_opaque(0, 1, 1) || is_opaque(1, 1, 1),
                    is_opaque(0, -1, 1) || is_opaque(1, -1, 1),
                    is_opaque(0, -1, -1) || is_opaque(1, -1, -1),
                    is_opaque(0, 1, -1) || is_opaque(1, 1, -1),
                )
            } else {
                (
                    is_opaque(1, 0, 1),
                    is_opaque(1, 0, -1),
                    is_opaque(1, 1, 0),
                    is_opaque(1, -1, 0),
                    is_opaque(1, 1, 1),
                    is_opaque(1, -1, 1),
                    is_opaque(1, -1, -1),
                    is_opaque(1, 1, -1),
                )
            };

            let ao0 = vertex_ao(side_s, side_up, c_su);
            let ao1 = vertex_ao(side_s, side_dn, c_sd);
            let ao2 = vertex_ao(side_n, side_dn, c_nd);
            let ao3 = vertex_ao(side_n, side_up, c_nu);
            [ao0, ao1, ao2, ao3]
        }
    }
}

/// Computes 4-corner AO levels `[ao0, ao1, ao2, ao3]` for a face facing `dir`.
///
/// Convenience wrapper around `calculate_face_ao_with_inset` assuming a standard flush face.
#[inline]
pub fn calculate_face_ao<F>(dir: Direction, is_opaque: F) -> [u8; 4]
where
    F: FnMut(i32, i32, i32) -> bool,
{
    calculate_face_ao_with_inset(dir, false, is_opaque)
}

/// Determines whether a quad should flip its diagonal triangulation split (anisotropy flip)
/// to eliminate diagonal shading crease artifacts.
///
/// Returns `true` if triangulating as `(0-1-2, 0-2-3)` is optimal, or `false` if `(1-2-3, 0-1-3)` is optimal.
#[inline]
pub fn should_flip_quad_diagonal(ao: [u8; 4]) -> bool {
    (ao[0] as i32 + ao[2] as i32) > (ao[1] as i32 + ao[3] as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vertex_ao_logic() {
        assert_eq!(vertex_ao(false, false, false), 3);
        assert_eq!(vertex_ao(true, false, false), 2);
        assert_eq!(vertex_ao(true, true, false), 0); // Corner blocked
        assert_eq!(vertex_ao(true, true, true), 0);
    }

    #[test]
    fn test_anisotropy_flip() {
        // ao0=3, ao1=0, ao2=3, ao3=0 => sum02=6, sum13=0 => true
        assert!(should_flip_quad_diagonal([3, 0, 3, 0]));
        // ao0=0, ao1=3, ao2=0, ao3=3 => sum02=0, sum13=6 => false
        assert!(!should_flip_quad_diagonal([0, 3, 0, 3]));
    }

    #[test]
    fn test_is_face_inset() {
        let full_cube_up = [
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 1.0),
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(1.0, 1.0, 0.0),
        ];
        assert!(!is_face_inset(Direction::Up, &full_cube_up));

        let dirt_path_up = [
            Vec3::new(0.0, 15.0 / 16.0, 0.0),
            Vec3::new(0.0, 15.0 / 16.0, 1.0),
            Vec3::new(1.0, 15.0 / 16.0, 1.0),
            Vec3::new(1.0, 15.0 / 16.0, 0.0),
        ];
        assert!(is_face_inset(Direction::Up, &dirt_path_up));

        let slab_up = [
            Vec3::new(0.0, 0.5, 0.0),
            Vec3::new(0.0, 0.5, 1.0),
            Vec3::new(1.0, 0.5, 1.0),
            Vec3::new(1.0, 0.5, 0.0),
        ];
        assert!(is_face_inset(Direction::Up, &slab_up));
    }

    #[test]
    fn test_dirt_path_inset_ambient_occlusion() {
        // Dirt path top face: when isolated, all vertices have AO = 3
        let ao_isolated = calculate_face_ao_with_inset(Direction::Up, true, |_dx, _dy, _dz| false);
        assert_eq!(ao_isolated, [3, 3, 3, 3]);

        // When a solid block is placed on the East side (dx=1, dy=0):
        // v0 (-X, -Z) -> 3
        // v1 (-X, +Z) -> 3
        // v2 (+X, +Z) -> 2 (East neighbor occluded)
        // v3 (+X, -Z) -> 2 (East neighbor occluded)
        let ao_east_neighbor = calculate_face_ao_with_inset(Direction::Up, true, |dx, dy, dz| {
            dx == 1 && dy == 0 && dz == 0
        });
        assert_eq!(ao_east_neighbor, [3, 3, 2, 2]);

        // When solid blocks are placed on both East (1, 0, 0) and South (0, 0, 1):
        // v0 (-X, -Z) -> 3
        // v1 (-X, +Z) -> 2 (South neighbor)
        // v2 (+X, +Z) -> 0 (both East & South neighbors occluded -> corner fully blocked!)
        // v3 (+X, -Z) -> 2 (East neighbor)
        let ao_east_south = calculate_face_ao_with_inset(Direction::Up, true, |dx, dy, dz| {
            (dx == 1 && dy == 0 && dz == 0) || (dx == 0 && dy == 0 && dz == 1)
        });
        assert_eq!(ao_east_south, [3, 2, 0, 2]);
    }
}

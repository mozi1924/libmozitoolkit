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

/// Computes 4-corner AO levels `[ao0, ao1, ao2, ao3]` for a face facing `dir`.
///
/// `is_opaque` is queried with relative block offsets `(dx, dy, dz)` around the voxel.
pub fn calculate_face_ao<F>(dir: Direction, mut is_opaque: F) -> [u8; 4]
where
    F: FnMut(i32, i32, i32) -> bool,
{
    match dir {
        Direction::Up => {
            // Face at Y = +1
            // v0: (-X, +Z) => (-1, 1, 0), (0, 1, 1), (-1, 1, 1)
            let ao0 = vertex_ao(is_opaque(-1, 1, 0), is_opaque(0, 1, 1), is_opaque(-1, 1, 1));
            // v1: (+X, +Z) => (1, 1, 0), (0, 1, 1), (1, 1, 1)
            let ao1 = vertex_ao(is_opaque(1, 1, 0), is_opaque(0, 1, 1), is_opaque(1, 1, 1));
            // v2: (+X, -Z) => (1, 1, 0), (0, 1, -1), (1, 1, -1)
            let ao2 = vertex_ao(is_opaque(1, 1, 0), is_opaque(0, 1, -1), is_opaque(1, 1, -1));
            // v3: (-X, -Z) => (-1, 1, 0), (0, 1, -1), (-1, 1, -1)
            let ao3 = vertex_ao(is_opaque(-1, 1, 0), is_opaque(0, 1, -1), is_opaque(-1, 1, -1));
            [ao0, ao1, ao2, ao3]
        }
        Direction::Down => {
            // Face at Y = 0 (bottom)
            let ao0 = vertex_ao(is_opaque(-1, -1, 0), is_opaque(0, -1, -1), is_opaque(-1, -1, -1));
            let ao1 = vertex_ao(is_opaque(1, -1, 0), is_opaque(0, -1, -1), is_opaque(1, -1, -1));
            let ao2 = vertex_ao(is_opaque(1, -1, 0), is_opaque(0, -1, 1), is_opaque(1, -1, 1));
            let ao3 = vertex_ao(is_opaque(-1, -1, 0), is_opaque(0, -1, 1), is_opaque(-1, -1, 1));
            [ao0, ao1, ao2, ao3]
        }
        Direction::North => {
            // Face at Z = 0 (looking towards -Z)
            let ao0 = vertex_ao(is_opaque(1, 0, -1), is_opaque(0, 1, -1), is_opaque(1, 1, -1));
            let ao1 = vertex_ao(is_opaque(1, 0, -1), is_opaque(0, -1, -1), is_opaque(1, -1, -1));
            let ao2 = vertex_ao(is_opaque(-1, 0, -1), is_opaque(0, -1, -1), is_opaque(-1, -1, -1));
            let ao3 = vertex_ao(is_opaque(-1, 0, -1), is_opaque(0, 1, -1), is_opaque(-1, 1, -1));
            [ao0, ao1, ao2, ao3]
        }
        Direction::South => {
            // Face at Z = +1 (looking towards +Z)
            let ao0 = vertex_ao(is_opaque(-1, 0, 1), is_opaque(0, 1, 1), is_opaque(-1, 1, 1));
            let ao1 = vertex_ao(is_opaque(-1, 0, 1), is_opaque(0, -1, 1), is_opaque(-1, -1, 1));
            let ao2 = vertex_ao(is_opaque(1, 0, 1), is_opaque(0, -1, 1), is_opaque(1, -1, 1));
            let ao3 = vertex_ao(is_opaque(1, 0, 1), is_opaque(0, 1, 1), is_opaque(1, 1, 1));
            [ao0, ao1, ao2, ao3]
        }
        Direction::West => {
            // Face at X = 0 (looking towards -X)
            let ao0 = vertex_ao(is_opaque(-1, 0, -1), is_opaque(-1, 1, 0), is_opaque(-1, 1, -1));
            let ao1 = vertex_ao(is_opaque(-1, 0, -1), is_opaque(-1, -1, 0), is_opaque(-1, -1, -1));
            let ao2 = vertex_ao(is_opaque(-1, 0, 1), is_opaque(-1, -1, 0), is_opaque(-1, -1, 1));
            let ao3 = vertex_ao(is_opaque(-1, 0, 1), is_opaque(-1, 1, 0), is_opaque(-1, 1, 1));
            [ao0, ao1, ao2, ao3]
        }
        Direction::East => {
            // Face at X = +1 (looking towards +X)
            let ao0 = vertex_ao(is_opaque(1, 0, 1), is_opaque(1, 1, 0), is_opaque(1, 1, 1));
            let ao1 = vertex_ao(is_opaque(1, 0, 1), is_opaque(1, -1, 0), is_opaque(1, -1, 1));
            let ao2 = vertex_ao(is_opaque(1, 0, -1), is_opaque(1, -1, 0), is_opaque(1, -1, -1));
            let ao3 = vertex_ao(is_opaque(1, 0, -1), is_opaque(1, 1, 0), is_opaque(1, 1, -1));
            [ao0, ao1, ao2, ao3]
        }
    }
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
}

/// Computes canonical Minecraft UV coordinates for top and bottom fluid quad faces.
///
/// Flowing fluids sample a 16x16 window (`[0.25, 0.75]`) inside the 32x32 sprite,
/// centered at `(0.5, 0.5)`, with rotation baked directly into the coordinates.
/// Stationary source pools sample the standard full `[0, 1]` sprite.
pub fn get_fluid_top_uvs(is_flowing: bool, rotation: f32) -> [[f32; 2]; 4] {
    if !is_flowing {
        return [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]];
    }

    let base_uvs = [[0.25f32, 0.25f32], [0.25, 0.75], [0.75, 0.75], [0.75, 0.25]];
    if rotation.abs() < 1e-4 {
        return base_uvs;
    }

    // In Minecraft coordinate space where Y is inverted relative to standard UV space,
    // rotating UV by -rotation aligns with the fluid stream flow direction.
    let cos_t = (-rotation).cos();
    let sin_t = (-rotation).sin();

    let mut rotated = [[0.0f32; 2]; 4];
    for i in 0..4 {
        let du = base_uvs[i][0] - 0.5;
        let dv = base_uvs[i][1] - 0.5;
        rotated[i][0] = 0.5 + (du * cos_t - dv * sin_t);
        rotated[i][1] = 0.5 + (du * sin_t + dv * cos_t);
    }
    rotated
}

/// Computes Minecraft / Mineways-standard non-collapsed UV coordinates for vertical/slanted fluid side faces.
///
/// Side faces sample the `[0.0, 0.5]` quadrant of the 32x32 sprite, mapping
/// proportional 1-block height to 16 pixels.
#[inline]
pub fn get_fluid_side_uvs(h_left_top: f32, h_right_top: f32) -> [[f32; 2]; 4] {
    [
        [0.0, (1.0 - h_left_top) * 0.5],
        [0.0, 0.5],
        [0.5, 0.5],
        [0.5, (1.0 - h_right_top) * 0.5],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_still_top_uvs() {
        let uvs = get_fluid_top_uvs(false, 0.0);
        assert_eq!(uvs[0], [0.0, 0.0]);
        assert_eq!(uvs[1], [0.0, 1.0]);
        assert_eq!(uvs[2], [1.0, 1.0]);
        assert_eq!(uvs[3], [1.0, 0.0]);
    }

    #[test]
    fn test_flowing_top_uvs() {
        let uvs = get_fluid_top_uvs(true, 0.0);
        assert_eq!(uvs[0], [0.25, 0.25]);
        assert_eq!(uvs[2], [0.75, 0.75]);
    }

    #[test]
    fn test_side_uvs() {
        let uvs = get_fluid_side_uvs(1.0, 0.5);
        assert_eq!(uvs[0], [0.0, 0.0]);
        assert_eq!(uvs[1], [0.0, 0.5]);
        assert_eq!(uvs[2], [0.5, 0.5]);
        assert_eq!(uvs[3], [0.5, 0.25]);
    }
}

use mtk_voxel::fluid::{
    calculate_corner_average, calculate_fluid_corner_heights, get_fluid_base_height, FluidType,
    MAX_FLUID_HEIGHT,
};

#[test]
fn test_fluid_base_heights() {
    assert_eq!(get_fluid_base_height("minecraft:water"), MAX_FLUID_HEIGHT);
    assert_eq!(
        get_fluid_base_height("minecraft:water[level=0]"),
        MAX_FLUID_HEIGHT
    );
    assert_eq!(
        get_fluid_base_height("minecraft:flowing_water[level=1]"),
        7.0 / 9.0
    );
    assert_eq!(
        get_fluid_base_height("minecraft:flowing_water[level=7]"),
        1.0 / 9.0
    );
    assert_eq!(
        get_fluid_base_height("minecraft:oak_stairs[waterlogged=true]"),
        MAX_FLUID_HEIGHT
    );
}

#[test]
fn test_still_source_surface_tension() {
    // A still source water surrounded by air should stay at MAX_FLUID_HEIGHT (8/9) without drooping
    let corner_h = calculate_corner_average(MAX_FLUID_HEIGHT, 0.0, 0.0, 0.0, true);
    assert_eq!(corner_h, MAX_FLUID_HEIGHT);
}

#[test]
fn test_fluid_corner_height_calculation() {
    let get_state = |x: i32, y: i32, z: i32| -> String {
        if x == 0 && y == 64 && z == 0 {
            "minecraft:water".to_string()
        } else {
            "minecraft:air".to_string()
        }
    };

    let (c_nw, c_ne, c_se, c_sw) =
        calculate_fluid_corner_heights(get_state, 0, 64, 0, FluidType::Water);

    assert_eq!(c_nw, MAX_FLUID_HEIGHT);
    assert_eq!(c_ne, MAX_FLUID_HEIGHT);
    assert_eq!(c_se, MAX_FLUID_HEIGHT);
    assert_eq!(c_sw, MAX_FLUID_HEIGHT);
}

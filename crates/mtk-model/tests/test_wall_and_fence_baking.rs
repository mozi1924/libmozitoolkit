use std::collections::HashMap;

use mtk_model::{
    BakedModelDatabase, BlockModelJson, BlockStateDefinition, ModelBaker,
};

fn make_wall_post_model() -> BlockModelJson {
    serde_json::from_str(r##"{
        "textures": { "wall": "minecraft:block/cobblestone" },
        "elements": [
            {
                "from": [4, 0, 4],
                "to": [12, 16, 12],
                "faces": {
                    "down":  { "texture": "#wall", "cullface": "down" },
                    "up":    { "texture": "#wall", "cullface": "up" },
                    "north": { "texture": "#wall" },
                    "south": { "texture": "#wall" },
                    "west":  { "texture": "#wall" },
                    "east":  { "texture": "#wall" }
                }
            }
        ]
    }"##).unwrap()
}

fn make_wall_side_model() -> BlockModelJson {
    serde_json::from_str(r##"{
        "textures": { "wall": "minecraft:block/cobblestone" },
        "elements": [
            {
                "from": [5, 0, 0],
                "to": [11, 14, 8],
                "faces": {
                    "down":  { "texture": "#wall", "cullface": "down" },
                    "up":    { "texture": "#wall" },
                    "north": { "texture": "#wall", "cullface": "north" },
                    "south": { "texture": "#wall" },
                    "west":  { "texture": "#wall" },
                    "east":  { "texture": "#wall" }
                }
            }
        ]
    }"##).unwrap()
}

fn make_wall_def() -> BlockStateDefinition {
    let bs_json = r##"{
        "multipart": [
            {
                "apply": { "model": "minecraft:block/cobblestone_wall_post" },
                "when": { "up": "true" }
            },
            {
                "apply": { "model": "minecraft:block/cobblestone_wall_side", "uvlock": true },
                "when": { "north": "low" }
            },
            {
                "apply": { "model": "minecraft:block/cobblestone_wall_side", "uvlock": true, "y": 90 },
                "when": { "east": "low" }
            },
            {
                "apply": { "model": "minecraft:block/cobblestone_wall_side", "uvlock": true, "y": 180 },
                "when": { "south": "low" }
            },
            {
                "apply": { "model": "minecraft:block/cobblestone_wall_side", "uvlock": true, "y": 270 },
                "when": { "west": "low" }
            }
        ]
    }"##;
    serde_json::from_str(bs_json).unwrap()
}

#[test]
fn test_wall_multipart_enumeration_contains_none_and_base_id() {
    let def = make_wall_def();
    let states = def.enumerate_all_states("minecraft:cobblestone_wall");

    // Must contain base unparameterized block ID
    assert!(
        states.contains(&"minecraft:cobblestone_wall".to_string()),
        "Enumerated states must include base ID"
    );

    // Must contain isolated wall state where all 4 directions are none
    let isolated = "minecraft:cobblestone_wall[east=none,north=none,south=none,up=true,west=none]";
    assert!(
        states.contains(&isolated.to_string()),
        "Enumerated states must include isolated wall state with none directions: found {:?}",
        states.iter().filter(|s| s.contains("none")).take(5).collect::<Vec<_>>()
    );

    // Must contain straight wall state
    let straight = "minecraft:cobblestone_wall[east=none,north=low,south=low,up=false,west=none]";
    assert!(
        states.contains(&straight.to_string()),
        "Enumerated states must include straight wall state"
    );
}

#[test]
fn test_wall_isolated_and_connected_baking() {
    let def = make_wall_def();
    let mut baker = ModelBaker::new();

    let mut models = HashMap::new();
    models.insert("minecraft:block/cobblestone_wall_post".to_string(), make_wall_post_model());
    models.insert("minecraft:block/cobblestone_wall_side".to_string(), make_wall_side_model());

    // 1. Isolated wall (up=true, all 4 directions=none)
    let isolated_state = "minecraft:cobblestone_wall[east=none,north=none,south=none,up=true,west=none]";
    let baked_isolated = baker
        .bake_blockstate(isolated_state, Some(&def), |id| models.get(id).cloned())
        .expect("Isolated wall bake should succeed");

    let (mesh_iso, _) = baked_isolated.to_mesh_with_textures(false);
    assert_eq!(
        mesh_iso.face_count(),
        6,
        "Isolated wall must have only 6 faces (single post cuboid)"
    );
    let min_x = mesh_iso.positions.iter().map(|p| p[0]).fold(f32::INFINITY, f32::min);
    let max_x = mesh_iso.positions.iter().map(|p| p[0]).fold(f32::NEG_INFINITY, f32::max);
    let min_z = mesh_iso.positions.iter().map(|p| p[2]).fold(f32::INFINITY, f32::min);
    let max_z = mesh_iso.positions.iter().map(|p| p[2]).fold(f32::NEG_INFINITY, f32::max);

    // Post bounds in Blender space: 4/16 = 0.25, 12/16 = 0.75
    assert!((min_x - 0.25).abs() < 1e-4, "Post min_x should be 0.25, got {}", min_x);
    assert!((max_x - 0.75).abs() < 1e-4, "Post max_x should be 0.75, got {}", max_x);
    assert!((min_z - 0.25).abs() < 1e-4, "Post min_z should be 0.25, got {}", min_z);
    assert!((max_z - 0.75).abs() < 1e-4, "Post max_z should be 0.75, got {}", max_z);

    // 2. Straight wall (north=low, south=low, up=false, east=none, west=none)
    let straight_state = "minecraft:cobblestone_wall[east=none,north=low,south=low,up=false,west=none]";
    let baked_straight = baker
        .bake_blockstate(straight_state, Some(&def), |id| models.get(id).cloned())
        .expect("Straight wall bake should succeed");

    let (mesh_straight, _) = baked_straight.to_mesh_with_textures(false);
    let min_z_str = mesh_straight.positions.iter().map(|p| p[2]).fold(f32::INFINITY, f32::min);
    let max_z_str = mesh_straight.positions.iter().map(|p| p[2]).fold(f32::NEG_INFINITY, f32::max);
    let min_x_str = mesh_straight.positions.iter().map(|p| p[0]).fold(f32::INFINITY, f32::min);
    let max_x_str = mesh_straight.positions.iter().map(|p| p[0]).fold(f32::NEG_INFINITY, f32::max);

    // Z extends across the full block (0.0 to 1.0) because North & South are connected
    assert!((min_z_str - 0.0).abs() < 1e-4, "Straight wall min_z should be 0.0, got {}", min_z_str);
    assert!((max_z_str - 1.0).abs() < 1e-4, "Straight wall max_z should be 1.0, got {}", max_z_str);
    // X does NOT extend to boundaries (stays in 0.25..0.75 range)
    assert!(min_x_str >= 0.25 - 1e-4, "Straight wall min_x must not reach 0.0, got {}", min_x_str);
    assert!(max_x_str <= 0.75 + 1e-4, "Straight wall max_x must not reach 1.0, got {}", max_x_str);
}

#[test]
fn test_wall_database_lookup_with_waterlogged_and_unparameterized() {
    let def = make_wall_def();
    let mut baker = ModelBaker::new();

    let mut models = HashMap::new();
    models.insert("minecraft:block/cobblestone_wall_post".to_string(), make_wall_post_model());
    models.insert("minecraft:block/cobblestone_wall_side".to_string(), make_wall_side_model());

    let mut db = BakedModelDatabase::new();
    let all_states = def.enumerate_all_states("minecraft:cobblestone_wall");
    for st in all_states {
        if let Ok(baked) = baker.bake_blockstate(&st, Some(&def), |id| models.get(id).cloned()) {
            db.insert(st, baked);
        }
    }

    // 1. Query with waterlogged=false (should strip and find isolated post)
    let query_waterlogged = "minecraft:cobblestone_wall[east=none,north=none,south=none,west=none,up=true,waterlogged=false]";
    let resolved_waterlogged = db.get(query_waterlogged).expect("Must resolve waterlogged isolated wall");
    assert_eq!(
        resolved_waterlogged.to_mesh_with_textures(false).0.face_count(),
        6,
        "Waterlogged isolated wall must resolve to post only (6 faces)"
    );

    // 2. Query bare block ID (should resolve to post only)
    let resolved_bare = db.get("minecraft:cobblestone_wall").expect("Must resolve bare cobblestone_wall");
    assert_eq!(
        resolved_bare.to_mesh_with_textures(false).0.face_count(),
        6,
        "Bare cobblestone_wall must resolve to post only (6 faces)"
    );

    // 3. Query straight wall with waterlogged=false
    let query_straight = "minecraft:cobblestone_wall[east=none,north=low,south=low,west=none,up=false,waterlogged=false]";
    let resolved_straight = db.get(query_straight).expect("Must resolve straight wall");
    let (mesh_str, _) = resolved_straight.to_mesh_with_textures(false);
    let min_x = mesh_str.positions.iter().map(|p| p[0]).fold(f32::INFINITY, f32::min);
    let max_x = mesh_str.positions.iter().map(|p| p[0]).fold(f32::NEG_INFINITY, f32::max);
    assert!(min_x > 0.1, "Straight wall must not connect on X axis (min_x > 0.1), got {}", min_x);
    assert!(max_x < 0.9, "Straight wall must not connect on X axis (max_x < 0.9), got {}", max_x);
}

use std::collections::HashMap;

use glam::Vec3;
use mtk_model::{
    BlockModelJson, BlockStateDefinition, ModObjLoader, ModelBaker, WavefrontObjParser,
};

#[test]
fn test_stairs_blockstate_baking_and_culling() {
    let mut baker = ModelBaker::new();

    let bs_json = r##"{
        "variants": {
            "facing=east,half=bottom,shape=straight": {
                "model": "minecraft:block/oak_stairs",
                "y": 90,
                "uvlock": true
            }
        }
    }"##;

    let model_json = r##"{
        "textures": {
            "bottom": "minecraft:block/oak_planks",
            "top": "minecraft:block/oak_planks",
            "side": "minecraft:block/oak_planks"
        },
        "elements": [
            {
                "from": [0, 0, 0],
                "to": [16, 8, 16],
                "faces": {
                    "down":  { "texture": "#bottom", "cullface": "down" },
                    "up":    { "texture": "#top" },
                    "north": { "texture": "#side", "cullface": "north" },
                    "south": { "texture": "#side", "cullface": "south" },
                    "west":  { "texture": "#side", "cullface": "west" },
                    "east":  { "texture": "#side", "cullface": "east" }
                }
            },
            {
                "from": [8, 8, 0],
                "to": [16, 16, 16],
                "faces": {
                    "down":  { "texture": "#bottom" },
                    "up":    { "texture": "#top", "cullface": "up" },
                    "north": { "texture": "#side", "cullface": "north" },
                    "south": { "texture": "#side", "cullface": "south" },
                    "west":  { "texture": "#side" },
                    "east":  { "texture": "#side", "cullface": "east" }
                }
            }
        ]
    }"##;

    let def: BlockStateDefinition = serde_json::from_str(bs_json).unwrap();
    let model: BlockModelJson = serde_json::from_str(model_json).unwrap();

    let baked = baker
        .bake_blockstate(
            "minecraft:oak_stairs[facing=east,half=bottom,shape=straight]",
            Some(&def),
            |_id| Some(model.clone()),
        )
        .unwrap();

    assert!(!baked.is_cube);
    assert_eq!(baked.elements.len(), 2);

    // Unclipped mesh includes internal contact faces
    let unclipped_mesh = baked.to_mesh(false);
    assert_eq!(unclipped_mesh.face_count(), 12);

    // Clipped mesh: the contact faces (upper bottom and lower top overlap) are eliminated
    let clipped_mesh = baked.to_mesh(true);
    // Lower element top face [8..16, 8, 0..16] is fully covered by upper element,
    // and upper element bottom face [8..16, 8, 0..16] is fully covered by lower element.
    // Both contact regions should be clipped away!
    assert!(
        clipped_mesh.face_count() < unclipped_mesh.face_count(),
        "Clipped mesh faces ({}) should be fewer than unclipped ({})",
        clipped_mesh.face_count(),
        unclipped_mesh.face_count()
    );
}

#[test]
fn test_obj_polygon_fan_and_negative_indices() {
    let obj_text = r#"
# 5-vertex polygon and negative indices
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 1.0 1.0 0.0
v 0.5 1.5 0.0
v 0.0 1.0 0.0
vt 0.0 0.0
vt 1.0 0.0
vt 1.0 1.0
vt 0.5 1.0
vt 0.0 1.0
usemtl [1]minecraft:block/gold_block
f -5/-5 -4/-4 -3/-3 -2/-2 -1/-1
"#;

    let faces = WavefrontObjParser::parse_str(obj_text, None);
    // 5-vertex polygon fanned into 3 triangles: (0,1,2), (0,2,3), (0,3,4)
    assert_eq!(faces.len(), 3);
    for f in &faces {
        assert_eq!(f.verts.len(), 3);
        assert_eq!(f.material, "minecraft:block/gold_block");
        assert_eq!(f.tint_index, 1);
    }

    let baked = ModObjLoader::process_raw_faces(
        &faces,
        &HashMap::new(),
        "minecraft:block/dirt",
        0.0,
        0.0,
        0.0,
        Vec3::ZERO,
    );
    assert_eq!(baked.len(), 3);
}

#[test]
fn test_builtin_chest_and_bell_fallback() {
    let mut baker = mtk_model::ModelBaker::new();

    // Chest with no model elements in loader -> should fallback to builtin chest model
    let empty_loader = |_: &str| None;
    let chest_model = baker
        .bake_blockstate("minecraft:chest[facing=north,type=single]", None, empty_loader)
        .expect("Should bake chest using builtin model");

    assert!(!chest_model.elements.is_empty(), "Chest should have builtin elements");
    assert!(chest_model.elements.len() >= 3, "Single chest should have base, lid, and latch");

    // Bell with empty elements -> should have bell patches applied
    let bell_loader = |id: &str| {
        if id.contains("bell") {
            Some(mtk_model::BlockModelJson {
                parent: None,
                ambientocclusion: Some(true),
                textures: None,
                elements: Some(vec![]),
            })
        } else {
            None
        }
    };
    let bell_model = baker
        .bake_blockstate("minecraft:bell[attachment=floor,facing=north]", None, bell_loader)
        .expect("Should bake bell with patches");

    assert!(!bell_model.elements.is_empty(), "Bell should have patched elements");
}

#[test]
fn test_builtin_models_coverage() {
    let mut baker = mtk_model::ModelBaker::new();
    let empty_loader = |_: &str| None;

    // Bed
    let bed = baker
        .bake_blockstate("minecraft:red_bed[facing=north,part=foot]", None, empty_loader)
        .expect("Should bake red bed");
    assert!(!bed.elements.is_empty(), "Bed should have elements");

    // Shulker box
    let shulker = baker
        .bake_blockstate("minecraft:shulker_box[facing=up]", None, empty_loader)
        .expect("Should bake shulker box");
    assert!(!shulker.elements.is_empty(), "Shulker box should have elements");

    // Standing sign
    let sign = baker
        .bake_blockstate("minecraft:oak_sign[rotation=4]", None, empty_loader)
        .expect("Should bake sign");
    assert!(!sign.elements.is_empty(), "Sign should have elements");

    // Wall sign
    let wall_sign = baker
        .bake_blockstate("minecraft:oak_wall_sign[facing=north]", None, empty_loader)
        .expect("Should bake wall sign");
    assert!(!wall_sign.elements.is_empty(), "Wall sign should have elements");

    // Skull
    let skull = baker
        .bake_blockstate("minecraft:skeleton_skull[rotation=0]", None, empty_loader)
        .expect("Should bake skull");
    assert!(!skull.elements.is_empty(), "Skull should have elements");

    // End portal
    let end_portal = baker
        .bake_blockstate("minecraft:end_portal", None, empty_loader)
        .expect("Should bake end portal");
    assert!(!end_portal.elements.is_empty(), "End portal should have elements");
}


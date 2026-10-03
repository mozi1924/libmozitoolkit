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

#[test]
fn test_baked_model_cull_meta_serialization() {
    let mut baker = ModelBaker::new();
    let cube_json = r##"{
        "textures": { "all": "minecraft:block/stone" },
        "elements": [{
            "from": [0, 0, 0],
            "to": [16, 16, 16],
            "faces": {
                "down":  { "texture": "#all", "cullface": "down" },
                "up":    { "texture": "#all", "cullface": "up" },
                "north": { "texture": "#all", "cullface": "north" },
                "south": { "texture": "#all", "cullface": "south" },
                "west":  { "texture": "#all", "cullface": "west" },
                "east":  { "texture": "#all", "cullface": "east" }
            }
        }]
    }"##;
    let model: BlockModelJson = serde_json::from_str(cube_json).unwrap();
    let baked = baker.bake_blockstate("minecraft:stone", None, |_| Some(model.clone())).unwrap();
    assert!(baked.cull_meta.is_some());
    let meta = baked.cull_meta.as_ref().unwrap();
    assert_eq!(meta.category, mtk_cull::CullCategory::SolidOpaque);
    assert_eq!(meta.full_face_mask, mtk_core::direction::DirMask::ALL);

    // Test bincode roundtrip
    let bin = bincode::serialize(&baked).unwrap();
    let deserialized: mtk_model::baked::BakedModel = bincode::deserialize(&bin).unwrap();
    assert!(deserialized.cull_meta.is_some());
    let des_meta = deserialized.cull_meta.unwrap();
    assert_eq!(des_meta.category, mtk_cull::CullCategory::SolidOpaque);
    assert_eq!(des_meta.full_face_mask, mtk_core::direction::DirMask::ALL);
}

#[test]
fn test_baked_model_atlas_uv_remapping() {
    let mut baker = ModelBaker::new();
    let cube_json = r##"{
        "textures": { "all": "minecraft:block/stone" },
        "elements": [{
            "from": [0, 0, 0],
            "to": [16, 16, 16],
            "faces": {
                "up": { "texture": "#all", "cullface": "up" }
            }
        }]
    }"##;
    let model: BlockModelJson = serde_json::from_str(cube_json).unwrap();
    let mut baked = baker.bake_blockstate("minecraft:stone", None, |_| Some(model.clone())).unwrap();

    // Before remapping: atlas_uvs is None
    let up_face = baked.elements[0].faces.get(&mtk_core::direction::Direction::Up).unwrap();
    assert!(up_face.atlas_uvs.is_none());

    // Remap to atlas: bounds [0.25, 0.5, 0.5, 0.75], chunk_id 1, texture_id 42
    baked.remap_to_atlas_with(|tex| {
        if tex == "minecraft:block/stone" {
            Some(([0.25, 0.5, 0.5, 0.75], 1, 42))
        } else {
            None
        }
    });

    let remapped_face = baked.elements[0].faces.get(&mtk_core::direction::Direction::Up).unwrap();
    assert!(remapped_face.atlas_uvs.is_some());
    assert_eq!(remapped_face.atlas_chunk_id, Some(1));
    assert_eq!(remapped_face.atlas_texture_id, Some(42));

    let uvs = remapped_face.atlas_uvs.unwrap();
    // Verify UV values are strictly in [0.25..0.5] and [0.5..0.75]
    for uv in &uvs {
        assert!(uv.x >= 0.25 && uv.x <= 0.5);
        assert!(uv.y >= 0.5 && uv.y <= 0.75);
    }

    // Test bincode roundtrip serialization
    let bin = bincode::serialize(&baked).unwrap();
    let deserialized: mtk_model::baked::BakedModel = bincode::deserialize(&bin).unwrap();
    let des_face = deserialized.elements[0].faces.get(&mtk_core::direction::Direction::Up).unwrap();
    assert_eq!(des_face.atlas_uvs, Some(uvs));
    assert_eq!(des_face.atlas_chunk_id, Some(1));
    assert_eq!(des_face.atlas_texture_id, Some(42));
}

#[test]
fn test_baked_model_six_direction_bucketing() {
    use mtk_core::direction::Direction;
    use mtk_model::baker::ModelBaker;
    use mtk_model::parser::BlockModelJson;

    let mut baker = ModelBaker::new();

    let complex_json = r##"{
        "textures": {
            "top": "minecraft:block/stone",
            "bottom": "minecraft:block/dirt",
            "rod": "minecraft:block/iron_block"
        },
        "elements": [
            {
                "from": [0, 0, 0],
                "to": [16, 8, 16],
                "faces": {
                    "down": { "texture": "#bottom", "cullface": "down" },
                    "up": { "texture": "#top", "cullface": "up" },
                    "north": { "texture": "#rod" }
                }
            },
            {
                "from": [0, 8, 0],
                "to": [8, 16, 16],
                "faces": {
                    "up": { "texture": "#top", "cullface": "up" }
                }
            }
        ]
    }"##;
    let model: BlockModelJson = serde_json::from_str(complex_json).unwrap();
    let baked = baker.bake_blockstate("minecraft:custom_stairs", None, |_| Some(model.clone())).unwrap();

    // Verify bucketing
    // Up has 2 faces (one from elem 0, one from elem 1)
    let up_bucket = &baked.culled_faces[Direction::Up.to_index()];
    assert_eq!(up_bucket.len(), 2, "Up bucket must contain 2 culled faces");

    // Down has 1 face
    let down_bucket = &baked.culled_faces[Direction::Down.to_index()];
    assert_eq!(down_bucket.len(), 1, "Down bucket must contain 1 culled face");

    // South, West, East have 0 faces
    assert_eq!(baked.culled_faces[Direction::South.to_index()].len(), 0);
    assert_eq!(baked.culled_faces[Direction::West.to_index()].len(), 0);
    assert_eq!(baked.culled_faces[Direction::East.to_index()].len(), 0);

    // North has cullface == None, so it must be in unculled_faces
    assert_eq!(baked.unculled_faces.len(), 1, "North face without cullface must be in unculled_faces");
    assert_eq!(baked.unculled_faces[0].direction, Direction::North);

    // Test bincode roundtrip serialization preserving face buckets
    let bin = bincode::serialize(&baked).unwrap();
    let deserialized: mtk_model::baked::BakedModel = bincode::deserialize(&bin).unwrap();
    assert_eq!(deserialized.culled_faces[Direction::Up.to_index()].len(), 2);
    assert_eq!(deserialized.culled_faces[Direction::Down.to_index()].len(), 1);
    assert_eq!(deserialized.unculled_faces.len(), 1);
}

#[test]
fn test_bedrock_culling_metadata_distinct_from_bed() {
    let mut baker = mtk_model::ModelBaker::new();

    // Bedrock should bake as a standard 16x16x16 cube
    let cube_loader = |_: &str| {
        let json = r##"{
            "textures": { "all": "minecraft:block/bedrock" },
            "elements": [
                {
                    "from": [0, 0, 0],
                    "to": [16, 16, 16],
                    "faces": {
                        "down":  { "texture": "#all", "cullface": "down" },
                        "up":    { "texture": "#all", "cullface": "up" },
                        "north": { "texture": "#all", "cullface": "north" },
                        "south": { "texture": "#all", "cullface": "south" },
                        "west":  { "texture": "#all", "cullface": "west" },
                        "east":  { "texture": "#all", "cullface": "east" }
                    }
                }
            ]
        }"##;
        serde_json::from_str::<BlockModelJson>(json).ok()
    };

    let bedrock = baker
        .bake_blockstate("minecraft:bedrock", None, cube_loader)
        .expect("Should bake bedrock");

    assert!(bedrock.is_cube, "Bedrock must be recognized as a cube");
    assert!(bedrock.is_opaque, "Bedrock must be recognized as opaque");

    let cull_meta = bedrock.cull_meta.as_ref().expect("Bedrock must have cull_meta");
    assert_eq!(
        cull_meta.category,
        mtk_cull::CullCategory::SolidOpaque,
        "Bedrock category must be SolidOpaque, NOT GlassTranslucent or PartialShape"
    );
    assert!(cull_meta.is_full_cube, "Bedrock must be full cube in cull meta");
    assert!(cull_meta.is_opaque, "Bedrock must be opaque in cull meta");
    assert_eq!(
        cull_meta.full_face_mask,
        mtk_core::direction::DirMask::ALL,
        "Bedrock full_face_mask must cover all 6 faces"
    );

    // Conversely, verify red bed is NOT SolidOpaque
    let empty_loader = |_: &str| None;
    let bed = baker
        .bake_blockstate("minecraft:red_bed[facing=north,part=foot]", None, empty_loader)
        .expect("Should bake red bed");

    assert!(!bed.is_cube, "Bed must NOT be a full cube");
    assert!(!bed.is_opaque, "Bed must NOT be opaque");
    let bed_cull_meta = bed.cull_meta.as_ref().expect("Bed must have cull_meta");
    assert_ne!(
        bed_cull_meta.category,
        mtk_cull::CullCategory::SolidOpaque,
        "Bed category must NOT be SolidOpaque"
    );
}

#[test]
fn test_baker_deoverlapping_duplicate_faces() {
    let mut baker = mtk_model::ModelBaker::new();

    // Model with 2 identical overlapping north faces (causes Z-fighting in DCC if unhandled)
    let loader = |_: &str| {
        let json = r##"{
            "textures": { "tex": "minecraft:block/stone" },
            "elements": [
                {
                    "from": [0, 0, 0],
                    "to": [16, 16, 16],
                    "faces": {
                        "north": { "texture": "#tex", "cullface": "north" }
                    }
                },
                {
                    "from": [0, 0, 0],
                    "to": [16, 16, 16],
                    "faces": {
                        "north": { "texture": "#tex", "cullface": "north" }
                    }
                }
            ]
        }"##;
        serde_json::from_str::<mtk_model::BlockModelJson>(json).ok()
    };

    let mut baked = baker
        .bake_blockstate("minecraft:stone", None, loader)
        .expect("Should bake test model");

    // When exclude_hidden_volume (de-overlapping) is false, both identical faces exist
    let raw_mesh = baked.to_mesh(false);
    assert_eq!(raw_mesh.triangle_count(), 4, "Raw unclipped mesh should have 2 quads (4 triangles)");

    // When exclude_hidden_volume is true, duplicate face must be eliminated to prevent DCC Z-fighting
    let clean_mesh = baked.to_mesh(true);
    assert_eq!(clean_mesh.triangle_count(), 2, "Clean mesh must have exactly 1 quad (2 triangles) with duplicate eliminated");

    // Also verify deduplicate_faces on BakedModel structure directly
    let removed = baked.deduplicate_faces();
    assert_eq!(removed, 1, "deduplicate_faces must report 1 removed duplicate face");
    let after_mesh = baked.to_mesh(false);
    assert_eq!(after_mesh.triangle_count(), 2, "After deduplicate_faces, raw mesh must also have 1 quad");
}

#[test]
fn test_baker_coplanar_opposite_contact_faces() {
    let mut baker = mtk_model::ModelBaker::new();

    // Two stacked elements touching at y=8
    let loader = |_: &str| {
        let json = r##"{
            "textures": { "tex": "minecraft:block/oak_planks" },
            "elements": [
                {
                    "from": [0, 0, 0],
                    "to": [16, 8, 16],
                    "faces": {
                        "down": { "texture": "#tex", "cullface": "down" },
                        "up":   { "texture": "#tex" }
                    }
                },
                {
                    "from": [0, 8, 0],
                    "to": [16, 16, 16],
                    "faces": {
                        "down": { "texture": "#tex" },
                        "up":   { "texture": "#tex", "cullface": "up" }
                    }
                }
            ]
        }"##;
        serde_json::from_str::<mtk_model::BlockModelJson>(json).ok()
    };

    let mut baked = baker
        .bake_blockstate("minecraft:oak_slab", None, loader)
        .expect("Should bake slab");

    // Raw mesh has 4 faces (8 triangles)
    let raw_mesh = baked.to_mesh(false);
    assert_eq!(raw_mesh.triangle_count(), 8);

    // Clean mesh should eliminate the interior touching faces at y=8, leaving only external down and up
    let clean_mesh = baked.to_mesh(true);
    assert_eq!(clean_mesh.triangle_count(), 4, "Interior contacting faces at y=8 must be culled (leaving 2 outer quads)");

    // Also verify BakedModel deduplicate_faces
    let removed = baked.deduplicate_faces();
    assert_eq!(removed, 2, "Both contacting interior faces must be removed");
    let after_mesh = baked.to_mesh(false);
    assert_eq!(after_mesh.triangle_count(), 4);
}

#[test]
fn test_builtin_blockbench_json_models() {
    use mtk_model::builtin::BuiltinModelRegistry;
    use mtk_model::baker::ModelBaker;

    let test_model_ids = [
        "minecraft:block/chest",
        "minecraft:block/chest_left",
        "minecraft:block/chest_right",
        "minecraft:block/trapped_chest",
        "minecraft:block/ender_chest",
        "minecraft:block/bed_foot",
        "minecraft:block/bed_head",
        "minecraft:block/shulker_box",
        "minecraft:block/sign_standing",
        "minecraft:block/sign_wall",
        "minecraft:block/hanging_sign",
        "minecraft:block/wall_hanging_sign",
        "minecraft:block/skull",
        "minecraft:block/skull_wall",
        "minecraft:block/dragon_head",
        "minecraft:block/piglin_head",
        "minecraft:block/bell_floor",
        "minecraft:block/bell_ceiling",
        "minecraft:block/bell_wall",
        "minecraft:block/bell_between_walls",
        "minecraft:block/end_portal",
        "minecraft:block/decorated_pot",
    ];

    for model_id in test_model_ids {
        let model = BuiltinModelRegistry::get_builtin_model_by_id(model_id)
            .unwrap_or_else(|| panic!("Builtin model {} must exist", model_id));
        assert!(
            model.elements.is_some() && !model.elements.as_ref().unwrap().is_empty(),
            "Model {} must have elements",
            model_id
        );
    }

    let mut baker = ModelBaker::new();
    let empty_loader = |_: &str| None;

    // Test baking double chests (left and right)
    let chest_left = baker
        .bake_blockstate("minecraft:chest[facing=south,type=left]", None, empty_loader)
        .expect("Left chest should bake");
    assert_eq!(chest_left.elements.len(), 3);

    let chest_right = baker
        .bake_blockstate("minecraft:chest[facing=south,type=right]", None, empty_loader)
        .expect("Right chest should bake");
    assert_eq!(chest_right.elements.len(), 3);

    // Test baking bell in all 4 attachments
    for att in ["floor", "ceiling", "single_wall", "double_wall"] {
        let state = format!("minecraft:bell[attachment={},facing=north]", att);
        let bell = baker
            .bake_blockstate(&state, None, empty_loader)
            .unwrap_or_else(|_| panic!("Bell {} must bake", att));
        assert!(!bell.elements.is_empty(), "Bell {} must have elements", att);
        // Has both support frame elements and 2 bell body elements
        assert!(bell.elements.len() >= 3, "Bell {} must contain frame and body", att);
    }
}





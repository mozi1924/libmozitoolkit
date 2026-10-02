use mtk_core::direction::Direction;
use mtk_model::baker::baked_model::BakedModelDatabase;
use mtk_model::baker::ModelBaker;
use mtk_model::parser::blockstate::resolve_redstone_wire_connections;

#[test]
fn test_bake_redstone_wire_builtin_fallback() {
    let mut baker = ModelBaker::new();

    // Bare redstone wire with no external blockstate definition or model loader
    // should seamlessly use builtin fallback models.
    let baked = baker
        .bake_blockstate("minecraft:redstone_wire[power=15]", None, |_| None)
        .expect("Builtin fallback baking must succeed for redstone_wire");

    assert!(!baked.is_cube);
    assert!(baked.is_emissive);
    assert!((baked.emissive_level - 1.0).abs() < 1e-4);

    let (mesh, textures) = baked.to_mesh_with_textures(false);
    assert!(!mesh.positions.is_empty());
    assert!(textures.iter().any(|t| t.contains("redstone_dust_dot")));

    // Check MeshData custom attributes for tint_color and emission level
    use mtk_core::attributes::constants::{ATTR_BIOME_TINT_COLOR, ATTR_BIOME_TINT_DATA, ATTR_EMISSION};
    use mtk_core::attributes::AttributeData;

    let emission_attr = mesh.get_custom_attribute(ATTR_EMISSION).unwrap();
    if let AttributeData::Float(ref vals) = emission_attr.data {
        assert!(!vals.is_empty());
        assert!((vals[0] - 1.0).abs() < 1e-4);
    } else {
        panic!("Emission attribute must be Float");
    }

    let tint_col_attr = mesh.get_custom_attribute(ATTR_BIOME_TINT_COLOR).unwrap();
    if let AttributeData::Float4(ref cols) = tint_col_attr.data {
        assert!(!cols.is_empty());
        assert!((cols[0][0] - 1.0).abs() < 1e-4); // Max power red
    } else {
        panic!("Tint color attribute must be Float4");
    }

    let tint_data_attr = mesh.get_custom_attribute(ATTR_BIOME_TINT_DATA).unwrap();
    if let AttributeData::Float4(ref datas) = tint_data_attr.data {
        assert!(!datas.is_empty());
        assert_eq!(datas[0][3], 4.0); // TINT_TYPE_HARDCODED
    } else {
        panic!("Tint data attribute must be Float4");
    }
}

#[test]
fn test_bake_redstone_wire_straight_line_z() {
    let mut baker = ModelBaker::new();

    // Test axis=z alias
    let baked_axis = baker
        .bake_blockstate("minecraft:redstone_wire[axis=z,power=0]", None, |_| None)
        .unwrap();

    assert!(!baked_axis.is_emissive);
    assert_eq!(baked_axis.emissive_level, 0.0);

    let (_mesh, textures) = baked_axis.to_mesh_with_textures(false);
    assert!(textures.iter().any(|t| t.contains("redstone_dust_line0")));
    assert!(baked_axis.elements.iter().any(|el| el.faces.values().any(|f| f.texture.contains("redstone_dust_overlay"))));
    // Straight line in vanilla does not include dot
    assert!(!textures.iter().any(|t| t.contains("redstone_dust_dot")));
    assert_eq!(baked_axis.elements.len(), 4); // 2 elements (line0 + overlay) * 2 parts
}

#[test]
fn test_bake_redstone_wire_corner_and_cross() {
    let mut baker = ModelBaker::new();

    // 1. Corner (angled) includes dot + side elements
    let baked_corner = baker
        .bake_blockstate("minecraft:redstone_wire[east=side,north=side]", None, |_| None)
        .unwrap();
    let (_, textures_corner) = baked_corner.to_mesh_with_textures(false);
    assert!(textures_corner.iter().any(|t| t.contains("redstone_dust_dot")));
    assert!(textures_corner.iter().any(|t| t.contains("redstone_dust_line0")));
    assert!(textures_corner.iter().any(|t| t.contains("redstone_dust_line1")));

    // 2. Four-way cross includes dot + all 4 sides
    let baked_cross = baker
        .bake_blockstate(
            "minecraft:redstone_wire[east=side,north=side,south=side,west=side]",
            None,
            |_| None,
        )
        .unwrap();
    let (_, textures_cross) = baked_cross.to_mesh_with_textures(false);
    assert!(textures_cross.iter().any(|t| t.contains("redstone_dust_dot")));
    assert!(textures_cross.iter().any(|t| t.contains("redstone_dust_line0")));
    assert!(textures_cross.iter().any(|t| t.contains("redstone_dust_line1")));
    assert_eq!(baked_cross.elements.len(), 10); // 2 elements * 5 multipart matches
}

#[test]
fn test_bake_redstone_wire_vertical_ascending_wall() {
    let mut baker = ModelBaker::new();

    // Vertical ascending wire on north wall: north=up, south=side (auto-straightened)
    let baked = baker
        .bake_blockstate("minecraft:redstone_wire[north=up]", None, |_| None)
        .unwrap();

    let (mesh, textures) = baked.to_mesh_with_textures(false);
    assert!(textures.iter().any(|t| t.contains("redstone_dust_line1")));

    // Check that vertical ascending element on North wall exists (from_pos[2] == 0.25, to_pos[1] == 16.0)
    let has_vertical_element = baked.elements.iter().any(|el| {
        (el.from_pos[2] - 0.25).abs() < 1e-4 && (el.to_pos[1] - 16.0).abs() < 1e-4
    });
    assert!(has_vertical_element, "Baked model must contain the vertical wall element extending to y=16.0");

    // CRITICAL: Ensure the vertical quad faces SOUTH (into the room) and NOT NORTH (into the wall)
    let south_faces: Vec<_> = baked
        .elements
        .iter()
        .flat_map(|el| el.faces.iter())
        .filter(|(&dir, _)| dir == Direction::South)
        .collect();

    assert!(
        !south_faces.is_empty(),
        "Vertical wall wire MUST keep the South face pointing into the room!"
    );

    // Verify face normals on mesh
    let has_south_normal = mesh.normals.iter().any(|n| (n[2] - 1.0).abs() < 1e-4);
    assert!(
        has_south_normal,
        "Mesh must contain South-facing normals for the vertical wire on North wall"
    );
}

#[test]
fn test_redstone_connection_resolution_and_auto_straighten() {
    // Isolated wire -> all none
    let isolated = resolve_redstone_wire_connections(|_, _, _| None);
    assert_eq!(isolated.get("east").unwrap(), "none");
    assert_eq!(isolated.get("west").unwrap(), "none");
    assert_eq!(isolated.get("north").unwrap(), "none");
    assert_eq!(isolated.get("south").unwrap(), "none");

    // Single neighbor to the East -> auto-straightens East and West to "side"
    let single_east = resolve_redstone_wire_connections(|dx, dy, dz| {
        if dx == 1 && dy == 0 && dz == 0 {
            Some("minecraft:repeater")
        } else {
            None
        }
    });
    assert_eq!(single_east.get("east").unwrap(), "side");
    assert_eq!(single_east.get("west").unwrap(), "side");
    assert_eq!(single_east.get("north").unwrap(), "none");
    assert_eq!(single_east.get("south").unwrap(), "none");

    // Neighbor on solid block above at (0, 1, -1) -> North is "up"
    let north_up = resolve_redstone_wire_connections(|dx, dy, dz| {
        if dx == 0 && dy == 0 && dz == -1 {
            Some("minecraft:stone") // solid block in front
        } else if dx == 0 && dy == 1 && dz == -1 {
            Some("minecraft:redstone_wire") // wire on top of solid block
        } else {
            None
        }
    });
    assert_eq!(north_up.get("north").unwrap(), "up");
    // Single arm auto-straightening makes south "side"
    assert_eq!(north_up.get("south").unwrap(), "side");
}

#[test]
fn test_baked_model_database_redstone_power_stripping() {
    let mut db = BakedModelDatabase::new();
    let mut baker = ModelBaker::new();

    // Bake pure geometric model without power: redstone_wire[east=side,west=side]
    let baked = baker
        .bake_blockstate("minecraft:redstone_wire[east=side,west=side]", None, |_| None)
        .unwrap();

    db.insert("minecraft:redstone_wire[east=side,west=side]".to_string(), baked);

    // Querying with power=15 should match the prebaked model via Tier 2 non-geometric property stripping!
    let found = db.get("minecraft:redstone_wire[east=side,power=15,west=side]");
    assert!(
        found.is_some(),
        "BakedModelDatabase must match redstone_wire[east=side,west=side] when querying with power=15"
    );
}

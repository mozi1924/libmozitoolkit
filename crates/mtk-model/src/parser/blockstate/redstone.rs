//! Redstone Wire BlockState Normalization, Directional Aliasing, and Connection Resolution.
//!
//! Implements canonical Minecraft redstone wire connection rules, auto-straightening,
//! directional alias mapping (e.g. Mineways/Jmc2Obj `line0`, `dot`, `angled`, `three_way`),
//! and neighbor voxel connectivity resolution.

use std::collections::BTreeMap;

/// Known vanilla blocks that redstone wire can natively connect to.
pub static REDSTONE_CONNECTABLE_BLOCKS: &[&str] = &[
    "redstone_wire",
    "redstone_block",
    "redstone_torch",
    "redstone_wall_torch",
    "repeater",
    "comparator",
    "target",
    "lever",
    "observer",
    "lightning_rod",
    "daylight_detector",
    "calibrated_sculk_sensor",
    "detector_rail",
    "trapped_chest",
    "tripwire_hook",
    "lectern",
];

/// Checks if a block identifier can connect to a redstone wire.
pub fn is_redstone_connectable(name: &str) -> bool {
    let clean = name.strip_prefix("minecraft:").unwrap_or(name);
    let stem = clean.split_once('[').map(|(b, _)| b).unwrap_or(clean);

    if REDSTONE_CONNECTABLE_BLOCKS.contains(&stem) {
        return true;
    }
    if stem.ends_with("_button") || stem.ends_with("_pressure_plate") {
        return true;
    }
    false
}

/// Normalizes redstone wire BlockState properties by resolving aliases (facing, axis, shape),
/// applying vanilla single-arm auto-straightening, and providing default values for missing directions.
pub fn normalize_redstone_wire_properties(
    props: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let mut out = props.clone();

    // 1. Directional Aliases (facing, axis, shape, wire, variant)
    if let Some(axis) = out.get("axis").cloned() {
        if axis == "z" {
            out.insert("north".to_string(), "side".to_string());
            out.insert("south".to_string(), "side".to_string());
        } else if axis == "x" {
            out.insert("east".to_string(), "side".to_string());
            out.insert("west".to_string(), "side".to_string());
        }
    }

    if let Some(facing) = out.get("facing").cloned() {
        match facing.as_str() {
            "north" | "south" => {
                out.insert("north".to_string(), "side".to_string());
                out.insert("south".to_string(), "side".to_string());
            }
            "east" | "west" => {
                out.insert("east".to_string(), "side".to_string());
                out.insert("west".to_string(), "side".to_string());
            }
            _ => {}
        }
    }

    let shape_key = out
        .get("shape")
        .or_else(|| out.get("wire"))
        .or_else(|| out.get("variant"))
        .cloned();

    if let Some(shape) = shape_key {
        match shape.as_str() {
            "line0" | "line_z" | "straight_z" => {
                out.insert("north".to_string(), "side".to_string());
                out.insert("south".to_string(), "side".to_string());
                out.entry("east".to_string()).or_insert_with(|| "none".to_string());
                out.entry("west".to_string()).or_insert_with(|| "none".to_string());
            }
            "line1" | "line_x" | "straight_x" => {
                out.insert("east".to_string(), "side".to_string());
                out.insert("west".to_string(), "side".to_string());
                out.entry("north".to_string()).or_insert_with(|| "none".to_string());
                out.entry("south".to_string()).or_insert_with(|| "none".to_string());
            }
            "dot" => {
                out.insert("north".to_string(), "none".to_string());
                out.insert("south".to_string(), "none".to_string());
                out.insert("east".to_string(), "none".to_string());
                out.insert("west".to_string(), "none".to_string());
            }
            "angled" | "corner" => {
                out.entry("north".to_string()).or_insert_with(|| "side".to_string());
                out.entry("east".to_string()).or_insert_with(|| "side".to_string());
                out.entry("south".to_string()).or_insert_with(|| "none".to_string());
                out.entry("west".to_string()).or_insert_with(|| "none".to_string());
            }
            "three_way" | "t" | "tee" => {
                out.entry("north".to_string()).or_insert_with(|| "side".to_string());
                out.entry("south".to_string()).or_insert_with(|| "side".to_string());
                out.entry("east".to_string()).or_insert_with(|| "side".to_string());
                out.entry("west".to_string()).or_insert_with(|| "none".to_string());
            }
            "four_way" | "cross" => {
                out.insert("north".to_string(), "side".to_string());
                out.insert("south".to_string(), "side".to_string());
                out.insert("east".to_string(), "side".to_string());
                out.insert("west".to_string(), "side".to_string());
            }
            _ => {}
        }
    }

    // 2. Vanilla Auto-straightening (Single Arm Rule):
    // In Minecraft, if redstone wire only connects on one side, it automatically
    // extends through to the opposite side to form a 2-way straight line.
    let is_connected = |dir: &str| -> bool {
        out.get(dir).map(|s| s == "side" || s == "up").unwrap_or(false)
    };

    let north_conn = is_connected("north");
    let south_conn = is_connected("south");
    let east_conn = is_connected("east");
    let west_conn = is_connected("west");

    let count = north_conn as u8 + south_conn as u8 + east_conn as u8 + west_conn as u8;
    if count == 1 {
        if north_conn {
            out.insert("south".to_string(), "side".to_string());
        } else if south_conn {
            out.insert("north".to_string(), "side".to_string());
        } else if east_conn {
            out.insert("west".to_string(), "side".to_string());
        } else if west_conn {
            out.insert("east".to_string(), "side".to_string());
        }
    }

    // 3. Defaults: east, north, south, west default to "none", power defaults to "0"
    out.entry("east".to_string()).or_insert_with(|| "none".to_string());
    out.entry("north".to_string()).or_insert_with(|| "none".to_string());
    out.entry("south".to_string()).or_insert_with(|| "none".to_string());
    out.entry("west".to_string()).or_insert_with(|| "none".to_string());
    out.entry("power".to_string()).or_insert_with(|| "0".to_string());

    out
}

/// Resolves redstone wire connection directions (east, north, south, west) given a neighbor query callback.
/// `query(dx, dy, dz)` returns `Some(block_id)` if a block is present, or `None` if air / empty.
pub fn resolve_redstone_wire_connections<F>(mut query: F) -> BTreeMap<String, String>
where
    F: FnMut(i32, i32, i32) -> Option<&'static str>,
{
    let mut props = BTreeMap::new();

    // Check directions: (dx, dz, prop_name)
    const DIRS: [(i32, i32, &str); 4] = [
        (1, 0, "east"),
        (-1, 0, "west"),
        (0, 1, "south"),
        (0, -1, "north"),
    ];

    let block_above = query(0, 1, 0);
    let top_open = block_above.is_none();

    for &(dx, dz, name) in &DIRS {
        let same_level = query(dx, 0, dz);
        let mut state = "none";

        if let Some(b) = same_level {
            if is_redstone_connectable(b) {
                state = "side";
            }
        } else {
            // Check downward slope at (dx, -1, dz)
            if let Some(below) = query(dx, -1, dz) {
                if is_redstone_connectable(below) {
                    state = "side";
                }
            }
        }

        // Check upward vertical ascending wire at (dx, 1, dz)
        if top_open && same_level.is_some() {
            if let Some(above) = query(dx, 1, dz) {
                if above == "redstone_wire" || above == "minecraft:redstone_wire" {
                    state = "up";
                }
            }
        }

        props.insert(name.to_string(), state.to_string());
    }

    normalize_redstone_wire_properties(&props)
}

/// Maps DCC / Mineways / Jmc2Obj legacy material and mesh names to standard redstone wire BlockState properties.
/// Returns `Some(("redstone_wire", properties))` if recognized.
pub fn map_legacy_redstone_name(raw_name: &str) -> Option<(&'static str, BTreeMap<String, String>)> {
    let clean = raw_name.strip_prefix("minecraft:").unwrap_or(raw_name).to_ascii_lowercase();
    let stem = clean.split_once('[').map(|(b, _)| b).unwrap_or(&clean);

    let is_redstone = stem.starts_with("redstone_dust") || stem.starts_with("redstone_wire");
    if !is_redstone || stem == "redstone_wire" {
        return None;
    }

    let is_on = stem.ends_with("_on") || clean.contains("lit=true") || clean.contains("power=15");
    let is_off = stem.ends_with("_off") || clean.contains("lit=false") || clean.contains("power=0");
    let power_val = if is_on {
        "15"
    } else if is_off {
        "0"
    } else {
        "0"
    };

    let mut props = BTreeMap::new();
    props.insert("power".to_string(), power_val.to_string());

    if stem.contains("line0") {
        props.insert("shape".to_string(), "line0".to_string());
    } else if stem.contains("line1") {
        props.insert("shape".to_string(), "line1".to_string());
    } else if stem.contains("dot") {
        props.insert("shape".to_string(), "dot".to_string());
    } else if stem.contains("angled") || stem.contains("corner") {
        props.insert("shape".to_string(), "angled".to_string());
    } else if stem.contains("three_way") {
        props.insert("shape".to_string(), "three_way".to_string());
    } else if stem.contains("four_way") || stem.contains("cross") {
        props.insert("shape".to_string(), "four_way".to_string());
    }

    Some(("redstone_wire", normalize_redstone_wire_properties(&props)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_empty_wire() {
        let p = BTreeMap::new();
        let normalized = normalize_redstone_wire_properties(&p);
        assert_eq!(normalized.get("east").unwrap(), "none");
        assert_eq!(normalized.get("north").unwrap(), "none");
        assert_eq!(normalized.get("south").unwrap(), "none");
        assert_eq!(normalized.get("west").unwrap(), "none");
        assert_eq!(normalized.get("power").unwrap(), "0");
    }

    #[test]
    fn test_normalize_single_arm_auto_straightening() {
        let mut p = BTreeMap::new();
        p.insert("north".to_string(), "side".to_string());
        let normalized = normalize_redstone_wire_properties(&p);
        assert_eq!(normalized.get("north").unwrap(), "side");
        assert_eq!(normalized.get("south").unwrap(), "side");
        assert_eq!(normalized.get("east").unwrap(), "none");
        assert_eq!(normalized.get("west").unwrap(), "none");
    }

    #[test]
    fn test_directional_aliases() {
        let mut p = BTreeMap::new();
        p.insert("axis".to_string(), "x".to_string());
        let normalized = normalize_redstone_wire_properties(&p);
        assert_eq!(normalized.get("east").unwrap(), "side");
        assert_eq!(normalized.get("west").unwrap(), "side");
        assert_eq!(normalized.get("north").unwrap(), "none");
        assert_eq!(normalized.get("south").unwrap(), "none");

        let mut p2 = BTreeMap::new();
        p2.insert("shape".to_string(), "angled".to_string());
        let norm2 = normalize_redstone_wire_properties(&p2);
        assert_eq!(norm2.get("north").unwrap(), "side");
        assert_eq!(norm2.get("east").unwrap(), "side");
        assert_eq!(norm2.get("south").unwrap(), "none");
        assert_eq!(norm2.get("west").unwrap(), "none");
    }

    #[test]
    fn test_map_legacy_name() {
        let (name, props) = map_legacy_redstone_name("redstone_dust_line0_on").unwrap();
        assert_eq!(name, "redstone_wire");
        assert_eq!(props.get("power").unwrap(), "15");
        assert_eq!(props.get("north").unwrap(), "side");
        assert_eq!(props.get("south").unwrap(), "side");
        assert_eq!(props.get("east").unwrap(), "none");
        assert_eq!(props.get("west").unwrap(), "none");
    }
}

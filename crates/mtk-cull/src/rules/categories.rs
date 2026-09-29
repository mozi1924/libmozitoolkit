//! # Minecraft Block Categories & Parametric Shape Heuristics
//!
//! Provides catalogued name registries and parametric face occlusion resolvers
//! for standard vanilla Minecraft blocks, slabs, stairs, fluids, leaves, and partial blocks.

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use mtk_core::constants::geometry::EPS;
use mtk_core::direction::{DirMask, Direction};
use mtk_core::geometry::Aabb2d;
use mtk_core::Vec3;

use crate::rect_ops::extract_quad_face_occlusion_rect;
use crate::types::{
    is_full_rect, BlockCullMeta, CullCategory, FULL_FACE_RECT,
};

/// Known block names for air categories.
pub const AIR_NAMES: &[&str] = &[
    "air",
    "minecraft:air",
    "cave_air",
    "minecraft:cave_air",
    "void_air",
    "minecraft:void_air",
    "structure_void",
    "minecraft:structure_void",
    "bubble_column",
    "minecraft:bubble_column",
];

/// Known block names for fluid categories.
pub const FLUID_NAMES: &[&str] = &[
    "water",
    "minecraft:water",
    "flowing_water",
    "minecraft:flowing_water",
    "lava",
    "minecraft:lava",
    "flowing_lava",
    "minecraft:flowing_lava",
];

/// Known block names for leaves categories.
pub const LEAVES_NAMES: &[&str] = &[
    "oak_leaves",
    "minecraft:oak_leaves",
    "spruce_leaves",
    "minecraft:spruce_leaves",
    "birch_leaves",
    "minecraft:birch_leaves",
    "jungle_leaves",
    "minecraft:jungle_leaves",
    "acacia_leaves",
    "minecraft:acacia_leaves",
    "dark_oak_leaves",
    "minecraft:dark_oak_leaves",
    "mangrove_leaves",
    "minecraft:mangrove_leaves",
    "cherry_leaves",
    "minecraft:cherry_leaves",
    "azalea_leaves",
    "minecraft:azalea_leaves",
    "flowering_azalea_leaves",
    "minecraft:flowering_azalea_leaves",
    "pale_oak_leaves",
    "minecraft:pale_oak_leaves",
];

/// Known block names for glass categories.
pub const GLASS_NAMES: &[&str] = &[
    "glass",
    "minecraft:glass",
    "tinted_glass",
    "minecraft:tinted_glass",
    "white_stained_glass",
    "minecraft:white_stained_glass",
    "orange_stained_glass",
    "minecraft:orange_stained_glass",
    "magenta_stained_glass",
    "minecraft:magenta_stained_glass",
    "light_blue_stained_glass",
    "minecraft:light_blue_stained_glass",
    "yellow_stained_glass",
    "minecraft:yellow_stained_glass",
    "lime_stained_glass",
    "minecraft:lime_stained_glass",
    "pink_stained_glass",
    "minecraft:pink_stained_glass",
    "gray_stained_glass",
    "minecraft:gray_stained_glass",
    "light_gray_stained_glass",
    "minecraft:light_gray_stained_glass",
    "cyan_stained_glass",
    "minecraft:cyan_stained_glass",
    "purple_stained_glass",
    "minecraft:purple_stained_glass",
    "blue_stained_glass",
    "minecraft:blue_stained_glass",
    "brown_stained_glass",
    "minecraft:brown_stained_glass",
    "green_stained_glass",
    "minecraft:green_stained_glass",
    "red_stained_glass",
    "minecraft:red_stained_glass",
    "black_stained_glass",
    "minecraft:black_stained_glass",
    "ice",
    "minecraft:ice",
    "packed_ice",
    "minecraft:packed_ice",
    "blue_ice",
    "minecraft:blue_ice",
    "frosted_ice",
    "minecraft:frosted_ice",
    "slime_block",
    "minecraft:slime_block",
    "honey_block",
    "minecraft:honey_block",
    "powder_snow",
    "minecraft:powder_snow",
];

/// Known block names for non-occluding categories.
pub const NON_OCCLUDING_NAMES: &[&str] = &[
    "torch",
    "wall_torch",
    "soul_torch",
    "soul_wall_torch",
    "redstone_torch",
    "redstone_wall_torch",
    "lantern",
    "soul_lantern",
    "short_grass",
    "tall_grass",
    "fern",
    "large_fern",
    "dandelion",
    "poppy",
    "blue_orchid",
    "allium",
    "azure_bluet",
    "red_tulip",
    "orange_tulip",
    "white_tulip",
    "pink_tulip",
    "oxeye_daisy",
    "cornflower",
    "lily_of_the_valley",
    "wither_rose",
    "sunflower",
    "lilac",
    "rose_bush",
    "peony",
    "dead_bush",
    "sapling",
    "wheat",
    "carrots",
    "potatoes",
    "beetroots",
    "sweet_berry_bush",
    "ladder",
    "lever",
    "tripwire_hook",
    "tripwire",
    "vine",
    "scaffolding",
    "barrier",
    "light",
    "structure_void",
    "bubble_column",
    "kelp",
    "kelp_plant",
    "seagrass",
    "tall_seagrass",
    "sea_pickle",
    "sugar_cane",
    "bamboo",
    "bamboo_sapling",
    "cactus",
    "nether_wart",
    "crimson_roots",
    "warped_roots",
    "hanging_roots",
    "nether_sprouts",
    "spore_blossom",
    "small_dripleaf",
    "big_dripleaf",
    "big_dripleaf_stem",
    "lily_pad",
    "pink_petals",
    "wildflowers",
    "leaf_litter",
    "torchflower",
    "torchflower_crop",
    "pitcher_plant",
    "pitcher_crop",
    "cave_vines",
    "cave_vines_plant",
    "twisting_vines",
    "twisting_vines_plant",
    "weeping_vines",
    "weeping_vines_plant",
    "glow_lichen",
    "sculk_vein",
    "frogspawn",
    "turtle_egg",
    "sniffer_egg",
    "cobweb",
];

/// Check if block identifier is non-occluding vegetation, decoration, or plant.
pub fn is_non_occluding_block(name_low: &str) -> bool {
    if NON_OCCLUDING_NAMES.contains(&name_low) {
        return true;
    }
    if name_low.ends_with("_flower")
        || name_low.ends_with("_sapling")
        || name_low.ends_with("_torch")
        || name_low.ends_with("_lantern")
        || name_low.ends_with("_plant")
        || name_low.ends_with("_bush")
        || (name_low.ends_with("_roots") && name_low != "mangrove_roots")
        || name_low.ends_with("_vines")
        || name_low.ends_with("_sprouts")
        || name_low.ends_with("_petals")
        || name_low.ends_with("_lichen")
        || name_low.ends_with("_crop")
        || name_low.ends_with("_egg")
    {
        return true;
    }
    if name_low.contains("grass") && !name_low.contains("grass_block") {
        return true;
    }
    if name_low.contains("fern")
        || name_low.contains("dripleaf")
        || name_low.contains("kelp")
        || name_low.contains("seagrass")
        || name_low.contains("litter")
        || name_low == "sugar_cane"
        || (name_low.starts_with("bamboo") && !name_low.contains("block") && !name_low.contains("planks"))
        || name_low == "lily_pad"
        || name_low == "spore_blossom"
        || name_low == "sea_pickle"
        || name_low == "cobweb"
        || (name_low.contains("coral") && !name_low.contains("block"))
    {
        return true;
    }
    false
}

/// Suffixes identifying partial non-full blocks.
pub const PARTIAL_SHAPE_SUFFIXES: &[&str] = &[
    "_fence",
    "_fence_gate",
    "_wall",
    "_pane",
    "_bars",
    "_trapdoor",
    "_door",
    "_carpet",
    "_bed",
    "_sign",
    "_hanging_sign",
    "_head",
    "_skull",
    "_banner",
    "_candle",
    "_pot",
    "_rod",
    "_coral",
    "_fan",
    "_chain",
];

/// Exact names identifying partial non-full blocks.
pub const PARTIAL_SHAPE_EXACT_NAMES: &[&str] = &[
    "piston_head",
    "moving_piston",
    "iron_bars",
    "glass_pane",
    "chest",
    "trapped_chest",
    "ender_chest",
    "bell",
    "anvil",
    "chipped_anvil",
    "damaged_anvil",
    "cauldron",
    "water_cauldron",
    "lava_cauldron",
    "powder_snow_cauldron",
    "hopper",
    "brewing_stand",
    "flower_pot",
    "conduit",
    "beacon",
    "decorated_pot",
    "end_portal_frame",
    "end_portal",
    "end_gateway",
    "chain",
    "iron_chain",
    "copper_chain",
    "exposed_copper_chain",
    "weathered_copper_chain",
    "oxidized_copper_chain",
    "lever",
    "tripwire_hook",
    "tripwire",
    "repeater",
    "comparator",
    "daylight_detector",
    "lightning_rod",
    "end_rod",
    "dragon_egg",
    "scaffolding",
    "pointed_dripstone",
    "amethyst_cluster",
    "small_amethyst_bud",
    "medium_amethyst_bud",
    "large_amethyst_bud",
    "calibrated_sculk_sensor",
    "sculk_sensor",
    "sculk_shrieker",
    "sculk_vein",
    "snow",
    "ladder",
    "grindstone",
    "stonecutter",
    "lectern",
    "sniffer_egg",
];

/// Check if block identifier is a non-full or partial block.
pub fn is_non_full_or_partial_block(name_low: &str) -> bool {
    if PARTIAL_SHAPE_EXACT_NAMES.contains(&name_low)
        || is_non_occluding_block(name_low)
    {
        return true;
    }
    if PARTIAL_SHAPE_SUFFIXES.iter().any(|&s| name_low.ends_with(s)) {
        return true;
    }
    const KEYWORDS: &[&str] = &[
        "flower", "sapling", "torch", "lantern", "pane", "fence", "wall", "carpet", "trapdoor",
        "door",
    ];
    if KEYWORDS.iter().any(|&kw| name_low.contains(kw)) {
        return true;
    }
    false
}

/// Fast extraction of raw block name and properties from state string.
pub fn parse_block_name_and_props(state_str: &str) -> (String, BTreeMap<String, String>) {
    let trimmed = state_str.trim();
    if trimmed.is_empty() {
        return ("air".to_string(), BTreeMap::new());
    }

    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        #[cfg(feature = "serde")]
        {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
                let raw_state = val
                    .get("state")
                    .or_else(|| val.get("name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("air");

                let mut props = BTreeMap::new();
                if let Some(opaque_val) = val.get("opaque").and_then(|v| v.as_i64()) {
                    props.insert("__opaque".to_string(), opaque_val.to_string());
                }
                if let Some(type_val) = val.get("type").and_then(|v| v.as_i64()) {
                    props.insert("__type".to_string(), type_val.to_string());
                }

                if let Some(open) = raw_state.find('[') {
                    if let Some(close) = raw_state.rfind(']') {
                        let base = &raw_state[..open];
                        let name = base.split(':').next_back().unwrap_or(base).to_string();
                        for item in raw_state[open + 1..close].split(',') {
                            if let Some((k, v)) = item.split_once('=') {
                                props.insert(k.trim().to_string(), v.trim().to_string());
                            }
                        }
                        return (name, props);
                    }
                }
                let name = raw_state.split(':').next_back().unwrap_or(raw_state).to_string();
                return (name, props);
            }
        }
    }

    if let Some(open) = trimmed.find('[') {
        if let Some(close) = trimmed.rfind(']') {
            let base = &trimmed[..open];
            let name = base.split(':').next_back().unwrap_or(base).to_string();
            let mut props = BTreeMap::new();
            for item in trimmed[open + 1..close].split(',') {
                if let Some((k, v)) = item.split_once('=') {
                    props.insert(k.trim().to_string(), v.trim().to_string());
                }
            }
            return (name, props);
        }
    }

    let name = trimmed.split(':').next_back().unwrap_or(trimmed).to_string();
    (name, BTreeMap::new())
}

/// Derive canonical 2D face occlusion shapes for known partial / non-full blocks
/// based on block state properties when explicit detailed model elements are absent.
pub fn derive_parametric_face_shapes(
    name_low: &str,
    props: &BTreeMap<String, String>,
) -> [Vec<Aabb2d>; 6] {
    let mut shapes: [Vec<Aabb2d>; 6] = Default::default();

    // 1. Slabs
    if name_low.ends_with("_slab") {
        let slab_type = props.get("type").map(|s| s.as_str()).unwrap_or("bottom");
        if slab_type == "double" {
            for dir in Direction::ALL {
                shapes[dir.to_index()] = alloc::vec![FULL_FACE_RECT];
            }
            return shapes;
        } else if slab_type == "top" {
            shapes[Direction::Up.to_index()] = alloc::vec![FULL_FACE_RECT];
            let side_rect = Aabb2d::from_min_max(0.0, 0.5, 1.0, 1.0);
            shapes[Direction::North.to_index()] = alloc::vec![side_rect];
            shapes[Direction::South.to_index()] = alloc::vec![side_rect];
            shapes[Direction::East.to_index()] = alloc::vec![side_rect];
            shapes[Direction::West.to_index()] = alloc::vec![side_rect];
            return shapes;
        } else {
            // bottom
            shapes[Direction::Down.to_index()] = alloc::vec![FULL_FACE_RECT];
            let side_rect = Aabb2d::from_min_max(0.0, 0.0, 1.0, 0.5);
            shapes[Direction::North.to_index()] = alloc::vec![side_rect];
            shapes[Direction::South.to_index()] = alloc::vec![side_rect];
            shapes[Direction::East.to_index()] = alloc::vec![side_rect];
            shapes[Direction::West.to_index()] = alloc::vec![side_rect];
            return shapes;
        }
    }

    // 2. Snow layers (minecraft:snow)
    if name_low == "snow" || name_low.ends_with(":snow") {
        let layers: u32 = props
            .get("layers")
            .and_then(|l| l.parse().ok())
            .unwrap_or(1)
            .clamp(1, 8);
        let h = layers as f32 / 8.0;
        let side_rect = Aabb2d::from_min_max(0.0, 0.0, 1.0, h);
        shapes[Direction::Down.to_index()] = alloc::vec![FULL_FACE_RECT];
        if layers == 8 {
            shapes[Direction::Up.to_index()] = alloc::vec![FULL_FACE_RECT];
        }
        shapes[Direction::North.to_index()] = alloc::vec![side_rect];
        shapes[Direction::South.to_index()] = alloc::vec![side_rect];
        shapes[Direction::East.to_index()] = alloc::vec![side_rect];
        shapes[Direction::West.to_index()] = alloc::vec![side_rect];
        return shapes;
    }

    // 3. Carpets
    if name_low.ends_with("_carpet") || name_low == "carpet" {
        let h = 1.0 / 16.0;
        let side_rect = Aabb2d::from_min_max(0.0, 0.0, 1.0, h);
        shapes[Direction::Down.to_index()] = alloc::vec![FULL_FACE_RECT];
        shapes[Direction::North.to_index()] = alloc::vec![side_rect];
        shapes[Direction::South.to_index()] = alloc::vec![side_rect];
        shapes[Direction::East.to_index()] = alloc::vec![side_rect];
        shapes[Direction::West.to_index()] = alloc::vec![side_rect];
        return shapes;
    }

    // 4. Iron bars & Glass panes
    if name_low.ends_with("_bars")
        || name_low.ends_with("_pane")
        || name_low == "iron_bars"
        || name_low == "glass_pane"
    {
        let w0 = 7.0 / 16.0;
        let w1 = 9.0 / 16.0;
        let post_cap = Aabb2d::from_min_max(w0, w0, w1, w1);
        let side_cap = Aabb2d::from_min_max(w0, 0.0, w1, 1.0);
        shapes[Direction::Down.to_index()] = alloc::vec![post_cap];
        shapes[Direction::Up.to_index()] = alloc::vec![post_cap];
        if props.get("east").map(|s| s.as_str()) == Some("true") {
            shapes[Direction::East.to_index()] = alloc::vec![side_cap];
        }
        if props.get("west").map(|s| s.as_str()) == Some("true") {
            shapes[Direction::West.to_index()] = alloc::vec![side_cap];
        }
        if props.get("north").map(|s| s.as_str()) == Some("true") {
            shapes[Direction::North.to_index()] = alloc::vec![side_cap];
        }
        if props.get("south").map(|s| s.as_str()) == Some("true") {
            shapes[Direction::South.to_index()] = alloc::vec![side_cap];
        }
        return shapes;
    }

    // 5. Wooden & Nether Brick Fences
    if name_low.ends_with("_fence") || name_low == "fence" {
        let w0 = 7.0 / 16.0;
        let w1 = 9.0 / 16.0;
        let post_cap =
            Aabb2d::from_min_max(6.0 / 16.0, 6.0 / 16.0, 10.0 / 16.0, 10.0 / 16.0);
        let top_bar = Aabb2d::from_min_max(w0, 12.0 / 16.0, w1, 15.0 / 16.0);
        let bot_bar = Aabb2d::from_min_max(w0, 6.0 / 16.0, w1, 9.0 / 16.0);
        shapes[Direction::Down.to_index()] = alloc::vec![post_cap];
        shapes[Direction::Up.to_index()] = alloc::vec![post_cap];
        if props.get("east").map(|s| s.as_str()) == Some("true") {
            shapes[Direction::East.to_index()] = alloc::vec![bot_bar, top_bar];
        }
        if props.get("west").map(|s| s.as_str()) == Some("true") {
            shapes[Direction::West.to_index()] = alloc::vec![bot_bar, top_bar];
        }
        if props.get("north").map(|s| s.as_str()) == Some("true") {
            shapes[Direction::North.to_index()] = alloc::vec![bot_bar, top_bar];
        }
        if props.get("south").map(|s| s.as_str()) == Some("true") {
            shapes[Direction::South.to_index()] = alloc::vec![bot_bar, top_bar];
        }
        return shapes;
    }

    // 6. Stairs
    if name_low.ends_with("_stairs") {
        let half = props.get("half").map(|s| s.as_str()).unwrap_or("bottom");
        if half == "top" {
            shapes[Direction::Up.to_index()] = alloc::vec![FULL_FACE_RECT];
        } else {
            shapes[Direction::Down.to_index()] = alloc::vec![FULL_FACE_RECT];
        }
        return shapes;
    }

    shapes
}

/// Computes a BlockCullMeta directly for a state string without checking cache.
pub fn compute_block_cull_meta(
    state_str: &str,
    element_quads: Option<&[([Vec3; 4], Direction)]>,
    is_opaque_hint: Option<bool>,
) -> BlockCullMeta {
    let (name, props) = parse_block_name_and_props(state_str);
    let name_low = name.to_ascii_lowercase();

    let json_opaque = props.get("__opaque").and_then(|v| v.parse::<i64>().ok()).map(|v| v != 0);
    let effective_opaque_hint = is_opaque_hint.or(json_opaque);
    let json_type = props.get("__type").and_then(|v| v.parse::<i64>().ok());

    let is_waterlogged = props.get("waterlogged").map(|s| s.as_str()) == Some("true")
        || matches!(
            name_low.as_str(),
            "seagrass" | "tall_seagrass" | "kelp" | "kelp_plant" | "sea_pickle"
        )
        || name_low.contains("seagrass")
        || name_low.contains("kelp");
    let is_air = state_str.is_empty()
        || AIR_NAMES.iter().any(|&n| name_low == n)
        || name_low.ends_with("air");
    let is_fluid = !is_air
        && (FLUID_NAMES.iter().any(|&n| name_low == n)
            || name_low.contains("water")
            || name_low.contains("lava"));
    let is_leaves = !is_air
        && (LEAVES_NAMES.iter().any(|&n| name_low == n)
            || name_low.ends_with("_leaves")
            || name_low.ends_with("leaves")
            || name_low == "mangrove_roots");

    let is_pane = name_low.ends_with("_pane")
        || name_low.ends_with("_bars")
        || name_low == "glass_pane"
        || name_low == "iron_bars";
    let is_glass = !is_air
        && !is_pane
        && (GLASS_NAMES.iter().any(|&n| name_low == n)
            || (name_low.contains("stained_glass") && !is_pane)
            || (name_low.ends_with("glass") && !is_pane)
            || name_low.ends_with("ice"));
    let is_non_occluding = !is_air
        && (json_type == Some(1) || json_type == Some(4) || is_non_occluding_block(&name_low));
    let is_double_slab =
        name_low.ends_with("_slab") && props.get("type").map(|s| s.as_str()) == Some("double");
    let is_non_full = !is_air
        && !is_double_slab
        && (is_pane
            || name_low.ends_with("_slab")
            || name_low.ends_with("_stairs")
            || (name_low.contains("piston") && (props.get("extended").map(|s| s.as_str()) == Some("true") || name_low.contains("head")))
            || is_non_full_or_partial_block(&name_low));

    let (category, is_full_cube, is_opaque, cull_group, face_shapes, full_face_mask, empty_face_mask) = if is_air {
        (
            CullCategory::Air,
            false,
            false,
            "air".to_string(),
            <[Vec<Aabb2d>; 6]>::default(),
            DirMask::empty(),
            DirMask::ALL,
        )
    } else if is_fluid {
        (
            CullCategory::Fluid,
            false,
            false,
            if name_low.contains("water") {
                "water".to_string()
            } else {
                "lava".to_string()
            },
            <[Vec<Aabb2d>; 6]>::default(),
            DirMask::empty(),
            DirMask::ALL,
        )
    } else if is_glass {
        (
            CullCategory::GlassTranslucent,
            true,
            false,
            if name_low.contains("glass") {
                "glass".to_string()
            } else {
                name_low.clone()
            },
            <[Vec<Aabb2d>; 6]>::default(),
            DirMask::empty(),
            DirMask::ALL,
        )
    } else if is_leaves {
        (
            CullCategory::CutoutLeaves,
            true,
            false,
            "leaves".to_string(),
            <[Vec<Aabb2d>; 6]>::default(),
            DirMask::empty(),
            DirMask::ALL,
        )
    } else if is_non_occluding {
        (
            CullCategory::NonOccluding,
            false,
            false,
            "non_occluding".to_string(),
            <[Vec<Aabb2d>; 6]>::default(),
            DirMask::empty(),
            DirMask::ALL,
        )
    } else if let Some(quads) = element_quads {
        // Detailed shape analysis from model baked quads
        let is_full_cube = Direction::ALL.iter().all(|dir| {
            quads.iter().any(|(verts, d)| {
                if d == dir {
                    if let Some(rect) = extract_quad_face_occlusion_rect(verts, *dir) {
                        return is_full_rect(&rect, EPS);
                    }
                }
                false
            })
        });

        let is_opaque = effective_opaque_hint.unwrap_or(is_full_cube);

        if is_full_cube {
            if is_opaque {
                let mut face_shapes: [Vec<Aabb2d>; 6] = Default::default();
                for dir in Direction::ALL {
                    face_shapes[dir.to_index()] = alloc::vec![FULL_FACE_RECT];
                }
                (
                    CullCategory::SolidOpaque,
                    true,
                    true,
                    "solid".to_string(),
                    face_shapes,
                    DirMask::ALL,
                    DirMask::empty(),
                )
            } else {
                (
                    CullCategory::GlassTranslucent,
                    true,
                    false,
                    name_low.clone(),
                    <[Vec<Aabb2d>; 6]>::default(),
                    DirMask::empty(),
                    DirMask::ALL,
                )
            }
        } else {
            let mut face_shapes: [Vec<Aabb2d>; 6] = Default::default();
            for (verts, dir) in quads {
                if let Some(rect) = extract_quad_face_occlusion_rect(verts, *dir) {
                    face_shapes[dir.to_index()].push(rect);
                }
            }

            let mut full_face_mask = DirMask::empty();
            let mut empty_face_mask = DirMask::empty();
            for dir in Direction::ALL {
                let dir_shapes = &face_shapes[dir.to_index()];
                if dir_shapes.iter().any(|s| is_full_rect(s, EPS)) {
                    if is_opaque {
                        full_face_mask |= dir.mask();
                    } else {
                        empty_face_mask |= dir.mask();
                    }
                } else if dir_shapes.is_empty() {
                    empty_face_mask |= dir.mask();
                }
            }
            (
                CullCategory::PartialShape,
                false,
                is_opaque,
                "partial".to_string(),
                face_shapes,
                full_face_mask,
                empty_face_mask,
            )
        }
    } else if is_non_full {
        let face_shapes = derive_parametric_face_shapes(&name_low, &props);
        let mut full_face_mask = DirMask::empty();
        let mut empty_face_mask = DirMask::empty();
        for dir in Direction::ALL {
            let dir_shapes = &face_shapes[dir.to_index()];
            if dir_shapes.iter().any(|s| is_full_rect(s, EPS)) {
                full_face_mask |= dir.mask();
            } else if dir_shapes.is_empty() {
                empty_face_mask |= dir.mask();
            }
        }
        (
            CullCategory::PartialShape,
            false,
            false,
            "partial".to_string(),
            face_shapes,
            full_face_mask,
            empty_face_mask,
        )
    } else {
        let is_opaque = effective_opaque_hint.unwrap_or(true);
        if is_opaque {
            let mut face_shapes: [Vec<Aabb2d>; 6] = Default::default();
            for dir in Direction::ALL {
                face_shapes[dir.to_index()] = alloc::vec![FULL_FACE_RECT];
            }
            (
                CullCategory::SolidOpaque,
                true,
                true,
                "solid".to_string(),
                face_shapes,
                DirMask::ALL,
                DirMask::empty(),
            )
        } else {
            (
                CullCategory::GlassTranslucent,
                true,
                false,
                name_low.clone(),
                <[Vec<Aabb2d>; 6]>::default(),
                DirMask::empty(),
                DirMask::ALL,
            )
        }
    };

    BlockCullMeta {
        state_str: state_str.to_string(),
        block_name: name,
        category,
        is_full_cube,
        is_opaque,
        is_air,
        is_fluid,
        cull_group,
        face_shapes,
        full_face_mask,
        empty_face_mask,
        props,
        is_waterlogged,
        has_baked_model: element_quads.is_some(),
    }
}

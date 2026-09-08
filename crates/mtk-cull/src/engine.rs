use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use std::collections::HashMap;


use mtk_core::constants::geometry::EPS;
use mtk_core::direction::{DirMask, Direction};
use mtk_core::geometry::Aabb2d;
use mtk_core::{IVec3, Vec3};


use crate::rect_ops::{extract_quad_face_occlusion_rect, is_face_completely_occluded};
use crate::rules::should_skip_rendering;
use crate::types::{
    is_full_rect, BlockCullMeta, CullCategory, GlassCullMode, LeavesCullMode, FULL_FACE_RECT,
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
];

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
    if PARTIAL_SHAPE_EXACT_NAMES.iter().any(|&n| name_low == n)
        || NON_OCCLUDING_NAMES.iter().any(|&n| name_low == n)
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

                if let Some(open) = raw_state.find('[') {
                    if let Some(close) = raw_state.rfind(']') {
                        let base = &raw_state[..open];
                        let name = base.split(':').last().unwrap_or(base).to_string();
                        let mut props = BTreeMap::new();
                        for item in raw_state[open + 1..close].split(',') {
                            if let Some((k, v)) = item.split_once('=') {
                                props.insert(k.trim().to_string(), v.trim().to_string());
                            }
                        }
                        return (name, props);
                    }
                }
                let name = raw_state
                    .split(':')
                    .last()
                    .unwrap_or(raw_state)
                    .to_string();
                let mut props = BTreeMap::new();
                if let Some(p_obj) = val
                    .get("properties")
                    .or_else(|| val.get("props"))
                    .and_then(|v| v.as_object())
                {
                    for (k, v) in p_obj {
                        if let Some(v_str) = v.as_str() {
                            props.insert(k.clone(), v_str.to_string());
                        }
                    }
                }
                return (name, props);
            }
        }
    }

    if let Some(open) = trimmed.find('[') {
        if let Some(close) = trimmed.rfind(']') {
            let base = &trimmed[..open];
            let name = base.split(':').last().unwrap_or(base).to_string();
            let mut props = BTreeMap::new();
            for item in trimmed[open + 1..close].split(',') {
                if let Some((k, v)) = item.split_once('=') {
                    props.insert(k.trim().to_string(), v.trim().to_string());
                }
            }
            return (name, props);
        }
    }

    let name = trimmed.split(':').last().unwrap_or(trimmed).to_string();
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

    let is_waterlogged = props.get("waterlogged").map(|s| s.as_str()) == Some("true")
        || matches!(
            name_low.as_str(),
            "seagrass" | "tall_seagrass" | "kelp" | "kelp_plant"
        );
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
        && (NON_OCCLUDING_NAMES.iter().any(|&n| name_low == n)
            || name_low.ends_with("_flower")
            || name_low.ends_with("_sapling")
            || name_low.ends_with("_torch")
            || name_low.ends_with("_lantern")
            || name_low.ends_with("_plant")
            || name_low.ends_with("_bush"));
    let is_double_slab =
        name_low.ends_with("_slab") && props.get("type").map(|s| s.as_str()) == Some("double");
    let is_non_full = !is_air
        && !is_double_slab
        && (is_pane
            || name_low.ends_with("_slab")
            || name_low.ends_with("_stairs")
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
        let is_cube = !is_non_full || is_double_slab;
        let is_full_cube = is_cube;
        let is_opaque = is_opaque_hint.unwrap_or(is_full_cube) && !is_non_full;

        if is_full_cube && is_opaque {
            let mut face_shapes: [Vec<Aabb2d>; 6] = Default::default();
            for dir in Direction::ALL {
                face_shapes[dir.to_index()] = alloc::vec![FULL_FACE_RECT];
            }
            (
                CullCategory::SolidOpaque,
                is_full_cube,
                is_opaque,
                "solid".to_string(),
                face_shapes,
                DirMask::ALL,
                DirMask::empty(),
            )
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
                    full_face_mask |= dir.mask();
                } else if dir_shapes.is_empty() {
                    empty_face_mask |= dir.mask();
                }
            }
            (
                CullCategory::PartialShape,
                is_full_cube,
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
        let mut face_shapes: [Vec<Aabb2d>; 6] = Default::default();
        for dir in Direction::ALL {
            face_shapes[dir.to_index()] = alloc::vec![FULL_FACE_RECT];
        }
        (
            CullCategory::SolidOpaque,
            true,
            is_opaque_hint.unwrap_or(true),
            "solid".to_string(),
            face_shapes,
            DirMask::ALL,
            DirMask::empty(),
        )
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

/// High-Performance Unified Face Culling Engine.
#[derive(Debug)]
pub struct FaceCuller {
    pub leaves_cull_mode: LeavesCullMode,
    pub glass_cull_mode: GlassCullMode,
    #[cfg(feature = "std")]
    meta_cache: std::sync::RwLock<HashMap<String, Arc<BlockCullMeta>>>,
    #[cfg(not(feature = "std"))]
    meta_cache: RefCell<HashMap<String, Arc<BlockCullMeta>>>,
}

impl Clone for FaceCuller {
    fn clone(&self) -> Self {
        #[cfg(feature = "std")]
        {
            let cache_clone = self.meta_cache.read().map(|g| g.clone()).unwrap_or_default();
            Self {
                leaves_cull_mode: self.leaves_cull_mode,
                glass_cull_mode: self.glass_cull_mode,
                meta_cache: std::sync::RwLock::new(cache_clone),
            }
        }
        #[cfg(not(feature = "std"))]
        {
            Self {
                leaves_cull_mode: self.leaves_cull_mode,
                glass_cull_mode: self.glass_cull_mode,
                meta_cache: RefCell::new(self.meta_cache.borrow().clone()),
            }
        }
    }
}

impl Default for FaceCuller {
    fn default() -> Self {
        Self::new(LeavesCullMode::SingleFace, GlassCullMode::Group)
    }
}

impl FaceCuller {
    /// Creates a new `FaceCuller` engine with custom leaves and glass modes.
    pub fn new(leaves_cull_mode: LeavesCullMode, glass_cull_mode: GlassCullMode) -> Self {
        Self {
            leaves_cull_mode,
            glass_cull_mode,
            #[cfg(feature = "std")]
            meta_cache: std::sync::RwLock::new(HashMap::new()),
            #[cfg(not(feature = "std"))]
            meta_cache: RefCell::new(HashMap::new()),
        }
    }

    /// Clears the cached block culling metadata.
    pub fn clear_cache(&self) {
        #[cfg(feature = "std")]
        {
            if let Ok(mut cache) = self.meta_cache.write() {
                cache.clear();
            }
        }
        #[cfg(not(feature = "std"))]
        {
            self.meta_cache.borrow_mut().clear();
        }
    }

    /// Number of cached metadata entries.
    pub fn cache_len(&self) -> usize {
        #[cfg(feature = "std")]
        {
            self.meta_cache.read().map(|g| g.len()).unwrap_or(0)
        }
        #[cfg(not(feature = "std"))]
        {
            self.meta_cache.borrow().len()
        }
    }

    /// Retrieves or computes `BlockCullMeta` for a given blockstate string.
    pub fn get_meta(
        &self,
        state_str: &str,
        element_quads: Option<&[([Vec3; 4], Direction)]>,
        is_opaque_hint: Option<bool>,
    ) -> Arc<BlockCullMeta> {
        #[cfg(feature = "std")]
        {
            if let Ok(cache) = self.meta_cache.read() {
                if let Some(meta) = cache.get(state_str) {
                    if element_quads.is_some() && !meta.has_baked_model {
                        // Needs re-bake with detailed quads
                    } else {
                        return Arc::clone(meta);
                    }
                }
            }

            let meta = Arc::new(compute_block_cull_meta(
                state_str,
                element_quads,
                is_opaque_hint,
            ));

            if let Ok(mut cache) = self.meta_cache.write() {
                if cache.len() >= 8192 && !cache.contains_key(state_str) {
                    if let Some(first_key) = cache.keys().next().cloned() {
                        cache.remove(&first_key);
                    }
                }
                cache.insert(state_str.to_string(), Arc::clone(&meta));
            }
            meta
        }
        #[cfg(not(feature = "std"))]
        {
            let mut cache = self.meta_cache.borrow_mut();
            if let Some(meta) = cache.get(state_str) {
                if element_quads.is_some() && !meta.has_baked_model {
                    // Refresh
                } else {
                    return Arc::clone(meta);
                }
            }

            let meta = Arc::new(compute_block_cull_meta(
                state_str,
                element_quads,
                is_opaque_hint,
            ));

            if cache.len() >= 8192 && !cache.contains_key(state_str) {
                if let Some(first_key) = cache.keys().next().cloned() {
                    cache.remove(&first_key);
                }
            }

            cache.insert(state_str.to_string(), Arc::clone(&meta));
            meta
        }
    }


    /// Evaluates Minecraft 1.21+ canonical face visibility test.
    ///
    /// Returns `true` if this face SHOULD be rendered, `false` if culled.
    pub fn should_render_face(
        &self,
        state_meta: &BlockCullMeta,
        neighbor_meta: Option<&BlockCullMeta>,
        direction: Direction,
        quad_face_shape: Option<&[Aabb2d]>,
        block_pos: Option<IVec3>,
        neighbor_pos: Option<IVec3>,
    ) -> bool {
        if state_meta.is_air {
            return false;
        }

        let neighbor_meta = match neighbor_meta {
            Some(n) if !n.is_air => n,
            _ => return true,
        };

        let opp_dir = direction.opposite();

        // 1. Solid / glass blocks must never have external faces culled by adjacent non-full / partial blocks
        let is_snow_cover = direction == Direction::Up
            && (neighbor_meta.block_name == "snow"
                || neighbor_meta.block_name.ends_with(":snow"))
            && neighbor_meta.has_full_face(Direction::Down);

        if (state_meta.category == CullCategory::SolidOpaque
            || state_meta.category == CullCategory::GlassTranslucent)
            && !is_snow_cover
            && (neighbor_meta.category == CullCategory::NonOccluding
                || (neighbor_meta.category == CullCategory::PartialShape
                    && !(neighbor_meta.block_name.ends_with("_slab")
                        || neighbor_meta.block_name.ends_with("_stairs"))))
        {
            return true;
        }

        // 2. Neighbor full solid face check (neighborFaceShape == Shapes.block())
        if neighbor_meta.has_full_face(opp_dir) {
            if state_meta.category == CullCategory::Fluid && direction == Direction::Up {
                // Fluid top face (direction == Up) is physically below upper boundary (< 1.0 height).
                // Solid ceiling above does not occlude fluid top surface.
            } else {
                return false;
            }
        }

        // 3. Custom skipRendering check (glass, leaves, fluid, snow, roots)
        if should_skip_rendering(
            state_meta,
            neighbor_meta,
            direction,
            self.leaves_cull_mode,
            self.glass_cull_mode,
            block_pos,
            neighbor_pos,
        ) {
            return false;
        }

        // 4. Neighbor empty face check (neighborFaceShape == Shapes.empty())
        if neighbor_meta.has_empty_face(opp_dir) {
            return true;
        }

        // 5. State empty face check (stateFaceShape == Shapes.empty())
        if quad_face_shape.is_none() && state_meta.has_empty_face(direction) {
            return true;
        }

        // 6. 2D Boolean shape occlusion check
        let default_quad_shape = [FULL_FACE_RECT];
        let target_shapes: &[Aabb2d] = if let Some(shapes) = quad_face_shape {
            shapes
        } else {
            let s = state_meta.get_face_shapes(direction);
            if s.is_empty() {
                &default_quad_shape
            } else {
                s
            }
        };
        let neighbor_shapes = neighbor_meta.get_face_shapes(opp_dir);

        if is_face_completely_occluded(target_shapes, neighbor_shapes) {
            return false;
        }

        true
    }
}

/// Compute all visible face directions for a single voxel block against surrounding neighbors.
pub fn get_visible_face_directions<F>(
    x: i32,
    y: i32,
    z: i32,
    state_str: &str,
    culler: &FaceCuller,
    get_neighbor_state: F,
) -> Vec<Direction>
where
    F: Fn(i32, i32, i32) -> Option<String>,
{
    if state_str.is_empty() {
        return Vec::new();
    }
    let meta = culler.get_meta(state_str, None, None);
    if meta.is_air {
        return Vec::new();
    }

    let mut visible = Vec::with_capacity(6);
    let block_pos = IVec3::new(x, y, z);

    for dir in Direction::ALL {
        let offset = dir.offset();
        let n_pos = block_pos + offset;
        let n_state_opt = get_neighbor_state(n_pos.x, n_pos.y, n_pos.z);
        let n_meta_opt = n_state_opt.as_deref().map(|s| culler.get_meta(s, None, None));

        if culler.should_render_face(
            &meta,
            n_meta_opt.as_deref(),
            dir,
            None,
            Some(block_pos),
            Some(n_pos),
        ) {
            visible.push(dir);
        }
    }

    visible
}

//! # Block and Texture Emission Evaluation
//!
//! Canonical Minecraft Java Edition light emission levels (0.0 .. 15.0).

use std::collections::HashMap;

/// Non-emissive keywords to prevent false-positive matches (e.g. torchflower).
pub const NON_EMISSIVE_KEYWORDS: &[&str] = &[
    "torchflower",
    "fire_coral",
    "coral",
    "banner",
    "bed",
    "wool",
    "carpet",
    "concrete",
    "terracotta",
    "stained_glass",
    "glass_pane",
    "shulker",
    "pressure_plate",
    "lightning_rod",
    "redstone_torch_off",
    "unlit",
];

/// Exact static block emission levels.
pub const VANILLA_STATIC_EMISSIONS: &[(&str, f32)] = &[
    ("beacon", 15.0),
    ("conduit", 15.0),
    ("end_gateway", 15.0),
    ("end_portal", 15.0),
    ("froglight", 15.0),
    ("ochre_froglight", 15.0),
    ("verdant_froglight", 15.0),
    ("pearlescent_froglight", 15.0),
    ("glowstone", 15.0),
    ("jack_o_lantern", 15.0),
    ("lava", 15.0),
    ("flowing_lava", 15.0),
    ("sea_lantern", 15.0),
    ("shroomlight", 15.0),
    ("lantern", 15.0),
    ("fire", 15.0),
    ("torch", 14.0),
    ("wall_torch", 14.0),
    ("end_rod", 14.0),
    ("soul_torch", 10.0),
    ("soul_wall_torch", 10.0),
    ("soul_lantern", 10.0),
    ("soul_fire", 10.0),
    ("crying_obsidian", 10.0),
    ("nether_portal", 11.0),
    ("firefly_bush", 10.0),
    ("glow_lichen", 7.0),
    ("sculk_catalyst", 6.0),
    ("amethyst_cluster", 5.0),
    ("large_amethyst_bud", 4.0),
    ("magma_block", 3.0),
    ("magma", 3.0),
    ("medium_amethyst_bud", 2.0),
    ("brewing_stand", 1.0),
    ("dragon_egg", 1.0),
    ("sculk_sensor", 1.0),
    ("small_amethyst_bud", 1.0),
];

/// Exact emissive texture names.
pub const EMISSIVE_TEXTURES: &[(&str, f32)] = &[
    ("torch", 14.0),
    ("wall_torch", 14.0),
    ("soul_torch", 10.0),
    ("soul_wall_torch", 10.0),
    ("lantern", 15.0),
    ("copper_lantern", 15.0),
    ("exposed_copper_lantern", 15.0),
    ("weathered_copper_lantern", 15.0),
    ("oxidized_copper_lantern", 15.0),
    ("waxed_copper_lantern", 15.0),
    ("waxed_exposed_copper_lantern", 15.0),
    ("waxed_weathered_copper_lantern", 15.0),
    ("waxed_oxidized_copper_lantern", 15.0),
    ("soul_lantern", 10.0),
    ("glowstone", 15.0),
    ("sea_lantern", 15.0),
    ("shroomlight", 15.0),
    ("froglight", 15.0),
    ("ochre_froglight_side", 15.0),
    ("ochre_froglight_top", 15.0),
    ("verdant_froglight_side", 15.0),
    ("verdant_froglight_top", 15.0),
    ("pearlescent_froglight_side", 15.0),
    ("pearlescent_froglight_top", 15.0),
    ("lava", 15.0),
    ("lava_still", 15.0),
    ("lava_flow", 15.0),
    ("fire", 15.0),
    ("fire_0", 15.0),
    ("fire_1", 15.0),
    ("soul_fire", 10.0),
    ("soul_fire_0", 10.0),
    ("soul_fire_1", 10.0),
    ("campfire_fire", 15.0),
    ("campfire_log_lit", 15.0),
    ("soul_campfire_fire", 10.0),
    ("soul_campfire_log_lit", 10.0),
    ("redstone_lamp_on", 15.0),
    ("furnace_front_on", 13.0),
    ("blast_furnace_front_on", 13.0),
    ("smoker_front_on", 13.0),
    ("redstone_torch", 7.0),
    ("crying_obsidian", 10.0),
    ("magma", 3.0),
    ("cave_vines_lit", 14.0),
    ("cave_vines_plant_lit", 14.0),
    ("open_eyeblossom", 11.0),
    ("open_eyeblossom_emissive", 11.0),
    ("amethyst_cluster", 5.0),
    ("beacon", 15.0),
    ("conduit", 15.0),
    ("copper_bulb_lit", 15.0),
    ("copper_bulb_lit_powered", 15.0),
    ("exposed_copper_bulb_lit", 12.0),
    ("exposed_copper_bulb_lit_powered", 12.0),
    ("weathered_copper_bulb_lit", 8.0),
    ("weathered_copper_bulb_lit_powered", 8.0),
    ("oxidized_copper_bulb_lit", 4.0),
    ("oxidized_copper_bulb_lit_powered", 4.0),
    ("end_rod", 14.0),
    ("jack_o_lantern", 15.0),
    ("nether_portal", 11.0),
];

fn clean_name(s: &str) -> &str {
    let mut cur = s.trim();
    if let Some(rest) = cur.strip_prefix("minecraft:") {
        cur = rest;
    }
    if let Some(rest) = cur.strip_prefix("block/") {
        cur = rest;
    }
    if let Some(rest) = cur.strip_suffix(".png") {
        cur = rest;
    }
    cur
}

/// Evaluates emission strength for a block name, optional state properties, and optional texture name.
pub fn get_block_emission_strength(
    block_name: &str,
    properties: Option<&HashMap<String, String>>,
    texture_name: Option<&str>,
) -> f32 {
    let clean_b = clean_name(block_name);
    let clean_t = texture_name.map(clean_name).unwrap_or("");

    // 0. Exclude non-emissive keywords
    for &kw in NON_EMISSIVE_KEYWORDS {
        if clean_b.contains(kw) || clean_t.contains(kw) {
            return 0.0;
        }
    }

    // 1. State resolvers
    if let Some(props) = properties {
        let is_lit = props.get("lit").map(|s| s.eq_ignore_ascii_case("true")).unwrap_or(false);
        match clean_b {
            "campfire" => return if is_lit { 15.0 } else { 0.0 },
            "soul_campfire" => return if is_lit { 10.0 } else { 0.0 },
            "furnace" | "blast_furnace" | "smoker" => return if is_lit { 13.0 } else { 0.0 },
            "redstone_lamp" => return if is_lit { 15.0 } else { 0.0 },
            "redstone_torch" | "redstone_wall_torch" => return if is_lit { 7.0 } else { 0.0 },
            "redstone_ore" | "deepslate_redstone_ore" => return if is_lit { 9.0 } else { 0.0 },
            "copper_bulb" | "waxed_copper_bulb" => return if is_lit { 15.0 } else { 0.0 },
            "exposed_copper_bulb" | "waxed_exposed_copper_bulb" => return if is_lit { 12.0 } else { 0.0 },
            "weathered_copper_bulb" | "waxed_weathered_copper_bulb" => return if is_lit { 8.0 } else { 0.0 },
            "oxidized_copper_bulb" | "waxed_oxidized_copper_bulb" => return if is_lit { 4.0 } else { 0.0 },
            "candle_cake" => return if is_lit { 3.0 } else { 0.0 },
            s if s == "candle" || s.ends_with("_candle") => {
                if !is_lit {
                    return 0.0;
                }
                let candles = props.get("candles").and_then(|v| v.parse::<f32>().ok()).unwrap_or(1.0);
                return candles * 3.0;
            }
            "sea_pickle" => {
                let waterlogged = props.get("waterlogged").map(|v| v.eq_ignore_ascii_case("true")).unwrap_or(true);
                if !waterlogged {
                    return 0.0;
                }
                let pickles = props.get("pickles").and_then(|v| v.parse::<f32>().ok()).unwrap_or(1.0);
                return pickles * 3.0 + 3.0;
            }
            "respawn_anchor" => {
                let charges = props.get("charges").and_then(|v| v.parse::<i32>().ok()).unwrap_or(0);
                return match charges {
                    1 => 3.0,
                    2 => 7.0,
                    3 => 11.0,
                    4 => 15.0,
                    _ => 0.0,
                };
            }
            "cave_vines" | "cave_vines_plant" => {
                let berries = props.get("berries").map(|v| v.eq_ignore_ascii_case("true")).unwrap_or(false);
                return if berries { 14.0 } else { 0.0 };
            }
            "light" => {
                return props.get("level").and_then(|v| v.parse::<f32>().ok()).unwrap_or(15.0);
            }
            "redstone_wire" => {
                return props.get("power").and_then(|v| v.parse::<f32>().ok()).unwrap_or(0.0);
            }
            _ => {}
        }
    }

    // 2. Eyeblossom
    if clean_b.contains("eyeblossom") || clean_t.contains("eyeblossom") {
        if clean_b.contains("open") || clean_t.contains("open") {
            return 11.0;
        }
        return 0.0;
    }

    // 3. Static block table lookup
    for &(name, level) in VANILLA_STATIC_EMISSIONS {
        if clean_b == name {
            return level;
        }
    }

    // 4. Texture map lookup
    let t_base = clean_t.rsplit('/').next().unwrap_or(clean_t);
    for &(name, level) in EMISSIVE_TEXTURES {
        if clean_t == name || t_base == name {
            return level;
        }
    }

    0.0
}

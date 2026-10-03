//! Embedded vanilla fallback blockstates and JSON models strictly for Java BER entity blocks
//! (chests, bells, shulker boxes, skulls, end portal, decorated pot) lacking static JSON models in vanilla.

use std::collections::HashMap;
use crate::parser::blockstate::{BlockState, BlockStateDefinition, BlockStateResolver};
use crate::parser::model_json::BlockModelJson;

// BlockStates (Only true Java BlockEntityRenderer blocks)
pub const DEF_CHEST: &str = include_str!("../../assets/builtins/blockstates/chest.json");
pub const DEF_TRAPPED_CHEST: &str = include_str!("../../assets/builtins/blockstates/trapped_chest.json");
pub const DEF_ENDER_CHEST: &str = include_str!("../../assets/builtins/blockstates/ender_chest.json");
pub const DEF_SHULKER_BOX: &str = include_str!("../../assets/builtins/blockstates/shulker_box.json");
pub const DEF_SKULL: &str = include_str!("../../assets/builtins/blockstates/skull.json");
pub const DEF_WALL_SKULL: &str = include_str!("../../assets/builtins/blockstates/wall_skull.json");
pub const DEF_BELL: &str = include_str!("../../assets/builtins/blockstates/bell.json");
pub const DEF_END_PORTAL: &str = include_str!("../../assets/builtins/blockstates/end_portal.json");
pub const DEF_DECORATED_POT: &str = include_str!("../../assets/builtins/blockstates/decorated_pot.json");

// Models (Only true Java BlockEntityRenderer blocks)
pub const MODEL_CHEST: &str = include_str!("../../assets/builtins/models/chest.json");
pub const MODEL_CHEST_LEFT: &str = include_str!("../../assets/builtins/models/chest_left.json");
pub const MODEL_CHEST_RIGHT: &str = include_str!("../../assets/builtins/models/chest_right.json");
pub const MODEL_TRAPPED_CHEST: &str = include_str!("../../assets/builtins/models/trapped_chest.json");
pub const MODEL_TRAPPED_CHEST_LEFT: &str = include_str!("../../assets/builtins/models/trapped_chest_left.json");
pub const MODEL_TRAPPED_CHEST_RIGHT: &str = include_str!("../../assets/builtins/models/trapped_chest_right.json");
pub const MODEL_ENDER_CHEST: &str = include_str!("../../assets/builtins/models/ender_chest.json");
pub const MODEL_SHULKER_BOX: &str = include_str!("../../assets/builtins/models/shulker_box.json");
pub const MODEL_SKULL: &str = include_str!("../../assets/builtins/models/skull.json");
pub const MODEL_SKULL_WALL: &str = include_str!("../../assets/builtins/models/skull_wall.json");
pub const MODEL_DRAGON_HEAD: &str = include_str!("../../assets/builtins/models/dragon_head.json");
pub const MODEL_PIGLIN_HEAD: &str = include_str!("../../assets/builtins/models/piglin_head.json");
pub const MODEL_BELL_FLOOR: &str = include_str!("../../assets/builtins/models/bell_floor.json");
pub const MODEL_BELL_CEILING: &str = include_str!("../../assets/builtins/models/bell_ceiling.json");
pub const MODEL_BELL_WALL: &str = include_str!("../../assets/builtins/models/bell_wall.json");
pub const MODEL_BELL_BETWEEN_WALLS: &str = include_str!("../../assets/builtins/models/bell_between_walls.json");
pub const MODEL_END_PORTAL: &str = include_str!("../../assets/builtins/models/end_portal.json");
pub const MODEL_DECORATED_POT: &str = include_str!("../../assets/builtins/models/decorated_pot.json");

/// Retrieves an embedded builtin `BlockStateDefinition` by block name.
pub fn get_builtin_blockstate_def(name: &str) -> Option<BlockStateDefinition> {
    let clean = name.strip_prefix("minecraft:").unwrap_or(name);

    let raw_json = if clean == "chest" || clean.ends_with("_chest") {
        if clean == "trapped_chest" {
            DEF_TRAPPED_CHEST
        } else if clean == "ender_chest" {
            DEF_ENDER_CHEST
        } else {
            DEF_CHEST
        }
    } else if clean == "shulker_box" || clean.ends_with("_shulker_box") {
        DEF_SHULKER_BOX
    } else if clean == "bell" {
        DEF_BELL
    } else if clean == "end_portal" {
        DEF_END_PORTAL
    } else if clean == "decorated_pot" {
        DEF_DECORATED_POT
    } else if clean.contains("wall_skull") || clean.contains("wall_head") {
        DEF_WALL_SKULL
    } else if clean.contains("skull") || clean.contains("head") {
        DEF_SKULL
    } else {
        return None;
    };

    serde_json::from_str(raw_json).ok()
}

/// Retrieves an embedded builtin `BlockModelJson` by model resource path / ID.
pub fn get_builtin_model_by_id(model_id: &str) -> Option<BlockModelJson> {
    let clean = model_id.strip_prefix("minecraft:").unwrap_or(model_id);
    let stem = clean.strip_prefix("block/").unwrap_or(clean);

    let raw_json = match stem {
        "chest" => MODEL_CHEST,
        "chest_left" => MODEL_CHEST_LEFT,
        "chest_right" => MODEL_CHEST_RIGHT,
        "trapped_chest" => MODEL_TRAPPED_CHEST,
        "trapped_chest_left" => MODEL_TRAPPED_CHEST_LEFT,
        "trapped_chest_right" => MODEL_TRAPPED_CHEST_RIGHT,
        "ender_chest" => MODEL_ENDER_CHEST,
        "shulker_box" => MODEL_SHULKER_BOX,
        "skull" => MODEL_SKULL,
        "skull_wall" => MODEL_SKULL_WALL,
        "dragon_head" => MODEL_DRAGON_HEAD,
        "piglin_head" => MODEL_PIGLIN_HEAD,
        "bell_floor" => MODEL_BELL_FLOOR,
        "bell_ceiling" => MODEL_BELL_CEILING,
        "bell_wall" => MODEL_BELL_WALL,
        "bell_between_walls" => MODEL_BELL_BETWEEN_WALLS,
        "end_portal" => MODEL_END_PORTAL,
        "decorated_pot" => MODEL_DECORATED_POT,
        _ => return None,
    };

    serde_json::from_str(raw_json).ok()
}

/// Fallback model for a blockstate when no external model was discovered.
pub fn get_builtin_model_for_state(blockstate: &BlockState) -> Option<BlockModelJson> {
    if let Some(def) = get_builtin_blockstate_def(&blockstate.name) {
        let matches = BlockStateResolver::resolve(&def, blockstate);
        if let Some(first_match) = matches.first() {
            let clean = blockstate.name.strip_prefix("minecraft:").unwrap_or(&blockstate.name);
            let model_key = if clean.contains("dragon") {
                "dragon_head"
            } else if clean.contains("piglin") {
                "piglin_head"
            } else {
                &first_match.model_id
            };
            if let Some(mut model) = get_builtin_model_by_id(model_key) {
                // Adjust textures for variant colors and head types if applicable
                if clean.ends_with("_shulker_box") && clean != "shulker_box" {
                    let color = clean.strip_suffix("_shulker_box").unwrap_or("");
                    let tex = format!("minecraft:entity/shulker/shulker_{}", color);
                    let textures = model.textures.get_or_insert_with(HashMap::new);
                    textures.insert("particle".to_string(), crate::parser::model_json::TextureValue::Path(tex.clone()));
                    textures.insert("texture".to_string(), crate::parser::model_json::TextureValue::Path(tex));
                } else if clean.contains("player_head") || clean.contains("player_wall_head") {
                    let tex = "minecraft:entity/player/wide/steve".to_string();
                    let textures = model.textures.get_or_insert_with(HashMap::new);
                    textures.insert("particle".to_string(), crate::parser::model_json::TextureValue::Path(tex.clone()));
                    textures.insert("texture".to_string(), crate::parser::model_json::TextureValue::Path(tex));
                } else if clean.contains("zombie_head") || clean.contains("zombie_wall_head") {
                    let tex = "minecraft:entity/zombie/zombie".to_string();
                    let textures = model.textures.get_or_insert_with(HashMap::new);
                    textures.insert("particle".to_string(), crate::parser::model_json::TextureValue::Path(tex.clone()));
                    textures.insert("texture".to_string(), crate::parser::model_json::TextureValue::Path(tex));
                } else if clean.contains("creeper_head") || clean.contains("creeper_wall_head") {
                    let tex = "minecraft:entity/creeper/creeper".to_string();
                    let textures = model.textures.get_or_insert_with(HashMap::new);
                    textures.insert("particle".to_string(), crate::parser::model_json::TextureValue::Path(tex.clone()));
                    textures.insert("texture".to_string(), crate::parser::model_json::TextureValue::Path(tex));
                } else if clean.contains("wither_skeleton") {
                    let tex = "minecraft:entity/skeleton/wither_skeleton".to_string();
                    let textures = model.textures.get_or_insert_with(HashMap::new);
                    textures.insert("particle".to_string(), crate::parser::model_json::TextureValue::Path(tex.clone()));
                    textures.insert("texture".to_string(), crate::parser::model_json::TextureValue::Path(tex));
                }
                return Some(model);
            }
        }
    }
    None
}


//! Embedded vanilla fallback blockstates and JSON models for complex non-full blocks
//! like redstone wire, ensuring reliable baking even without external resource packs.

use std::sync::OnceLock;
use crate::parser::blockstate::BlockStateDefinition;
use crate::parser::model_json::BlockModelJson;

pub const BUILTIN_REDSTONE_WIRE_BLOCKSTATE: &str =
    include_str!("../../assets/builtins/blockstates/redstone_wire.json");

pub const MODEL_REDSTONE_DUST_DOT: &str =
    include_str!("../../assets/builtins/models/redstone_dust_dot.json");
pub const MODEL_REDSTONE_DUST_SIDE0: &str =
    include_str!("../../assets/builtins/models/redstone_dust_side0.json");
pub const MODEL_REDSTONE_DUST_SIDE1: &str =
    include_str!("../../assets/builtins/models/redstone_dust_side1.json");
pub const MODEL_REDSTONE_DUST_SIDE_ALT0: &str =
    include_str!("../../assets/builtins/models/redstone_dust_side_alt0.json");
pub const MODEL_REDSTONE_DUST_SIDE_ALT1: &str =
    include_str!("../../assets/builtins/models/redstone_dust_side_alt1.json");
pub const MODEL_REDSTONE_DUST_SIDE: &str =
    include_str!("../../assets/builtins/models/redstone_dust_side.json");
pub const MODEL_REDSTONE_DUST_SIDE_ALT: &str =
    include_str!("../../assets/builtins/models/redstone_dust_side_alt.json");
pub const MODEL_REDSTONE_DUST_UP: &str =
    include_str!("../../assets/builtins/models/redstone_dust_up.json");

/// Retrieves an embedded builtin `BlockStateDefinition` by block name.
pub fn get_builtin_blockstate_def(name: &str) -> Option<BlockStateDefinition> {
    let clean = name.strip_prefix("minecraft:").unwrap_or(name);
    if clean == "redstone_wire" {
        static DEF: OnceLock<BlockStateDefinition> = OnceLock::new();
        let def = DEF.get_or_init(|| {
            serde_json::from_str(BUILTIN_REDSTONE_WIRE_BLOCKSTATE)
                .expect("Failed to parse builtin redstone_wire.json blockstate")
        });
        return Some(def.clone());
    }
    None
}

/// Retrieves an embedded builtin `BlockModelJson` by model resource path / ID.
pub fn get_builtin_model_by_id(model_id: &str) -> Option<BlockModelJson> {
    let clean = model_id.strip_prefix("minecraft:").unwrap_or(model_id);
    let stem = clean.strip_prefix("block/").unwrap_or(clean);

    let raw_json = match stem {
        "redstone_dust_dot" => MODEL_REDSTONE_DUST_DOT,
        "redstone_dust_side0" => MODEL_REDSTONE_DUST_SIDE0,
        "redstone_dust_side1" => MODEL_REDSTONE_DUST_SIDE1,
        "redstone_dust_side_alt0" => MODEL_REDSTONE_DUST_SIDE_ALT0,
        "redstone_dust_side_alt1" => MODEL_REDSTONE_DUST_SIDE_ALT1,
        "redstone_dust_side" => MODEL_REDSTONE_DUST_SIDE,
        "redstone_dust_side_alt" => MODEL_REDSTONE_DUST_SIDE_ALT,
        "redstone_dust_up" => MODEL_REDSTONE_DUST_UP,
        _ => return None,
    };

    serde_json::from_str(raw_json).ok()
}

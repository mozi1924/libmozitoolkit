//! Built-in fallback BlockStates and Models for standard vanilla entity blocks.
//!
//! Provides clean, Blockbench-compatible standard Minecraft BlockModel JSONs and
//! BlockState definitions for blocks lacking static models in vanilla Java JARs
//! (Chests, Beds, Shulker Boxes, Signs, Bells, Skulls, etc.).

pub mod evaluator;
pub mod loader;
pub mod patch;
pub mod vanilla;

pub use evaluator::entity_uv_to_faces;
pub use loader::MiExModelLoader;
pub use patch::apply_bell_patches;
pub use vanilla::{get_builtin_blockstate_def, get_builtin_model_by_id};

use crate::parser::blockstate::{BlockState, BlockStateDefinition};
use crate::parser::model_json::BlockModelJson;

/// Universal Builtin Model Registry driving dynamic models from raw upstream MiEx JSON definitions
/// and embedded vanilla fallback definitions.
pub struct BuiltinModelRegistry;

impl BuiltinModelRegistry {
    /// Attempts to provide a builtin fallback `BlockModelJson` using upstream MiEx JSON templates.
    pub fn get_builtin_model(blockstate: &BlockState) -> Option<BlockModelJson> {
        if let Some(model) = MiExModelLoader::load_for_blockstate(blockstate) {
            return Some(model);
        }
        vanilla::get_builtin_model_for_state(blockstate)
    }

    /// Attempts to provide a builtin fallback `BlockStateDefinition` by block name.
    pub fn get_builtin_blockstate_def(name: &str) -> Option<BlockStateDefinition> {
        vanilla::get_builtin_blockstate_def(name)
    }

    /// Attempts to provide a builtin fallback `BlockModelJson` by model resource path / ID.
    pub fn get_builtin_model_by_id(model_id: &str) -> Option<BlockModelJson> {
        vanilla::get_builtin_model_by_id(model_id)
    }
}

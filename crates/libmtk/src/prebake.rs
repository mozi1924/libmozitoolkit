//! # Headless Asset Prebaking Engine
//!
//! Performs full-scale, multi-threaded prebaking of all blockstate models discovered in a ResourcePackStack.

use std::collections::HashMap;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

use mtk_model::{BlockModelJson, BlockStateDefinition, BakedModelDatabase, ModelBaker};
use mtk_resource::{ResourceLocation, ResourcePackStack};
use crate::MtkError;

/// Prebakes all blockstates discovered across all active resource packs in the stack.
pub fn prebake_all_models(
    stack: &ResourcePackStack,
) -> Result<BakedModelDatabase, MtkError> {
    let blockstate_locs = stack.list_all_blockstate_locations();

    // 1. Preload all model JSONs across packs into a fast lookup map
    let mut model_cache: HashMap<String, BlockModelJson> = HashMap::new();
    for pack in stack.packs().iter().rev() {
        for file in pack.list_files("assets/") {
            if file.ends_with(".json") && file.contains("/models/") {
                if let Some(bytes) = pack.open(&file) {
                    if let Ok(model) = serde_json::from_slice::<BlockModelJson>(&bytes) {
                        if let Some(loc) = ResourceLocation::from_asset_path(&file, "models", "json") {
                            let canon = loc.as_string();
                            model_cache.insert(canon.clone(), model.clone());
                            if loc.namespace == "minecraft" {
                                model_cache.insert(loc.path.clone(), model.clone());
                            }
                        }
                    }
                }
            }
        }
    }

    // Helper closure to look up models with fallbacks
    let get_model = |model_id: &str| -> Option<BlockModelJson> {
        if let Some(m) = model_cache.get(model_id) {
            return Some(m.clone());
        }
        let (ns, raw_path) = if let Some((ns, n)) = model_id.split_once(':') {
            (ns, n)
        } else {
            ("minecraft", model_id)
        };
        let candidates = [
            format!("{}:{}", ns, raw_path),
            format!("{}:block/{}", ns, raw_path),
            format!("{}:item/{}", ns, raw_path),
            format!("block/{}", raw_path),
            format!("item/{}", raw_path),
            raw_path.to_string(),
        ];
        for c in &candidates {
            if let Some(m) = model_cache.get(c) {
                return Some(m.clone());
            }
        }
        let raw_candidates = [
            format!("assets/{}/models/{}.json", ns, raw_path),
            format!("assets/{}/models/block/{}.json", ns, raw_path),
            format!("assets/{}/models/item/{}.json", ns, raw_path),
        ];
        for path in &raw_candidates {
            if let Some(bytes) = stack.open_asset_raw(path) {
                if let Ok(m) = serde_json::from_slice::<BlockModelJson>(&bytes) {
                    return Some(m);
                }
            }
        }
        None
    };

    // 2. Pre-load BlockStateDefinitions for all discovered locations
    let mut blockstate_defs: Vec<(ResourceLocation, BlockStateDefinition)> = Vec::new();
    for loc in &blockstate_locs {
        let path = loc.to_asset_path("blockstates", "json");
        if let Some(bytes) = stack.open_asset_raw(&path) {
            if let Ok(def) = serde_json::from_slice::<BlockStateDefinition>(&bytes) {
                blockstate_defs.push((loc.clone(), def));
            }
        }
    }

    // 3. Bake all states concurrently
    #[cfg(feature = "parallel")]
    let baked_pairs: Vec<(String, mtk_model::BakedModel)> = blockstate_defs
        .par_iter()
        .flat_map(|(loc, def)| {
            let states = def.enumerate_all_states(&loc.as_string());
            let mut local_baker = ModelBaker::new();
            let mut pairs = Vec::new();
            for state_str in states {
                if let Ok(baked) = local_baker.bake_blockstate(&state_str, Some(def), |id| get_model(id)) {
                    pairs.push((state_str, baked));
                }
            }
            pairs
        })
        .collect();

    #[cfg(not(feature = "parallel"))]
    let baked_pairs: Vec<(String, mtk_model::BakedModel)> = blockstate_defs
        .iter()
        .flat_map(|(loc, def)| {
            let states = def.enumerate_all_states(&loc.as_string());
            let mut local_baker = ModelBaker::new();
            let mut pairs = Vec::new();
            for state_str in states {
                if let Ok(baked) = local_baker.bake_blockstate(&state_str, Some(def), |id| get_model(id)) {
                    pairs.push((state_str, baked));
                }
            }
            pairs
        })
        .collect();

    let mut db = BakedModelDatabase::new();
    for (state_str, baked) in baked_pairs {
        db.insert(state_str, baked);
    }

    Ok(db)
}

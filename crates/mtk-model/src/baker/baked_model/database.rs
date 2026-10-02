use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};

use super::model::BakedModel;

/// In-memory database of baked models keyed by canonical BlockState strings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BakedModelDatabase {
    pub models: HashMap<String, BakedModel>,
}

impl BakedModelDatabase {
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
        }
    }

    pub fn insert(&mut self, state: String, model: BakedModel) {
        self.models.insert(state, model);
    }

    /// Eliminates overlapping, duplicate, and interior coplanar contacting faces
    /// across all baked models in the database, returning total faces removed.
    pub fn deduplicate_all(&mut self) -> usize {
        let mut total = 0;
        for model in self.models.values_mut() {
            total += model.deduplicate_faces();
        }
        total
    }

    /// Remaps all baked models in the database to atlas coordinates using a lookup closure.
    pub fn remap_to_atlas_with<F>(&mut self, mut lookup_fn: F)
    where
        F: FnMut(&str) -> Option<([f32; 4], u16, u32)>,
    {
        for model in self.models.values_mut() {
            model.remap_to_atlas_with(&mut lookup_fn);
        }
    }

    /// Resolves a baked model using multi-tiered smart BlockState resolution:
    /// 1. Exact string match in `self.models`.
    /// 2. Normalized match with non-geometric properties stripped (e.g. `waterlogged`, `occupied`).
    /// 3. Best compatibility subset match against variant keys for this block ID.
    /// 4. Base unparameterized block ID fallback (e.g. `minecraft:chest`).
    pub fn get(&self, state: &str) -> Option<&BakedModel> {
        // Tier 1: Fast exact match
        if let Some(model) = self.models.get(state) {
            return Some(model);
        }

        let parsed = match crate::parser::blockstate::BlockState::parse(state) {
            Ok(p) => p,
            Err(_) => return None,
        };

        let base_id = parsed.block_id();
        let canon_str = parsed.to_canonical_string();

        // Tier 1.5: Canonical match (handles whitespace and property ordering)
        if let Some(model) = self.models.get(&canon_str) {
            return Some(model);
        }

        // Tier 2: Strip known non-geometric properties that never affect block model geometry in vanilla
        const NON_GEOMETRIC_PROPS: &[&str] = &[
            "waterlogged",
            "occupied",
            "distance",
            "persistent",
            "stage",
            "unstable",
            "conditional",
            "disarmed",
        ];

        let mut has_non_geom = false;
        let mut filtered_props = parsed.properties.clone();
        for &prop in NON_GEOMETRIC_PROPS {
            if filtered_props.remove(prop).is_some() {
                has_non_geom = true;
            }
        }

        if has_non_geom {
            let canon_filtered = if filtered_props.is_empty() {
                base_id.clone()
            } else {
                let props_str: Vec<String> = filtered_props
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, v))
                    .collect();
                format!("{}[{}]", base_id, props_str.join(","))
            };

            if let Some(model) = self.models.get(&canon_filtered) {
                return Some(model);
            }
        }

        // Helper closure to find the best compatible variant for a given base_id and property map
        let find_best_variant = |target_base_id: &str, target_props: &BTreeMap<String, String>| -> Option<&BakedModel> {
            let prefix = format!("{}[", target_base_id);
            let mut best_model: Option<&BakedModel> = None;
            let mut best_score = -999999i32;

            let mut relaxed_best_model: Option<&BakedModel> = None;
            let mut relaxed_best_score = -999999i32;

            for (key, model) in &self.models {
                if key == target_base_id {
                    let score = if target_props.is_empty() { 1000 } else { 0 };
                    if score > best_score {
                        best_score = score;
                        best_model = Some(model);
                    }
                    if score > relaxed_best_score {
                        relaxed_best_score = score;
                        relaxed_best_model = Some(model);
                    }
                    continue;
                }

                if key.starts_with(&prefix) && key.ends_with(']') {
                    let cand_props_str = &key[prefix.len()..key.len() - 1];
                    let mut cand_props: HashMap<&str, &str> = HashMap::new();
                    for pair in cand_props_str.split(',') {
                        if let Some((k, v)) = pair.split_once('=') {
                            cand_props.insert(k.trim(), v.trim());
                        }
                    }

                    // Compatibility check:
                    // Any property specified in target_props MUST match candidate's value if candidate defines it
                    let mut compatible = true;
                    let mut matched_keys = 0i32;
                    let mut relaxed_score = 0i32;

                    for (k, v) in target_props {
                        if let Some(&cand_v) = cand_props.get(k.as_str()) {
                            if cand_v == v.as_str() {
                                matched_keys += 1;
                                relaxed_score += if k == "facing" || k == "axis" { 500 } else { 100 };
                            } else {
                                compatible = false;
                                if k == "facing" || k == "axis" {
                                    relaxed_score -= 300;
                                }
                            }
                        }
                    }

                    if compatible {
                        let mut score = matched_keys * 100;
                        if cand_props.len() == target_props.len() {
                            score += 1000;
                        }

                        // Score candidate properties that were NOT specified in the query:
                        // Prefer canonical vanilla default values!
                        for (cand_k, cand_v) in &cand_props {
                            if !target_props.contains_key(*cand_k) {
                                if *cand_k == "up" && (target_base_id.ends_with("_wall") || target_base_id == "wall") {
                                    // In vanilla Minecraft, wall blocks default to up=true (post enabled)
                                    if *cand_v == "true" {
                                        score += 10;
                                    } else {
                                        score -= 10;
                                    }
                                } else if matches!(
                                    *cand_v,
                                    "false" | "0" | "none" | "straight" | "bottom" | "lower" | "single"
                                        | "foot" | "normal" | "side" | "y" | "north"
                                ) {
                                    score += 10;
                                } else if matches!(
                                    *cand_v,
                                    "true" | "1" | "top" | "upper" | "head" | "inner" | "outer" | "double"
                                        | "x" | "z" | "south" | "east" | "west"
                                ) {
                                    score -= 10;
                                }
                            }
                        }

                        if score > best_score {
                            best_score = score;
                            best_model = Some(model);
                        }
                    }

                    if relaxed_score > relaxed_best_score {
                        relaxed_best_score = relaxed_score;
                        relaxed_best_model = Some(model);
                    }
                }
            }

            best_model.or(relaxed_best_model)
        };

        // Tier 3: Cross-category block mappings for directional variants
        // If torch has a horizontal facing (north/south/east/west), it is a wall torch
        if let Some(facing) = filtered_props.get("facing").map(|s| s.as_str()) {
            if matches!(facing, "north" | "south" | "east" | "west") {
                let wall_id = match parsed.name.as_str() {
                    "torch" => Some(format!("{}:wall_torch", parsed.namespace)),
                    "soul_torch" => Some(format!("{}:soul_wall_torch", parsed.namespace)),
                    "redstone_torch" => Some(format!("{}:redstone_wall_torch", parsed.namespace)),
                    _ => None,
                };
                if let Some(wid) = wall_id {
                    if let Some(m) = find_best_variant(&wid, &filtered_props) {
                        return Some(m);
                    }
                }
            }
        }

        // Tier 3.5: Compatibility match on base_id with filtered_props
        if let Some(m) = find_best_variant(&base_id, &filtered_props) {
            return Some(m);
        }

        // Also try stripped short name (e.g. without "minecraft:" namespace)
        if base_id != parsed.name {
            if let Some(m) = find_best_variant(&parsed.name, &filtered_props) {
                return Some(m);
            }
        }

        // Check wall_torch -> torch if facing is up or not found
        let standing_id = match parsed.name.as_str() {
            "wall_torch" => Some(format!("{}:torch", parsed.namespace)),
            "soul_wall_torch" => Some(format!("{}:soul_torch", parsed.namespace)),
            "redstone_wall_torch" => Some(format!("{}:redstone_torch", parsed.namespace)),
            _ => None,
        };
        if let Some(sid) = standing_id {
            let mut standing_props = filtered_props.clone();
            standing_props.remove("facing");
            if let Some(m) = find_best_variant(&sid, &standing_props) {
                return Some(m);
            }
        }

        // Tier 5: Base block ID fallback (e.g. "minecraft:chest")
        if let Some(m) = self.models.get(&base_id) {
            return Some(m);
        }

        // Also try stripped short name (e.g. "chest")
        self.models.get(&parsed.name)
    }

    pub fn len(&self) -> usize {
        self.models.len()
    }

    pub fn is_empty(&self) -> bool {
        self.models.is_empty()
    }

    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.models.keys()
    }
}

use std::collections::{BTreeMap, HashMap};

use super::definition::{
    BlockStateDefinition, MultipartCondition, VariantEntry, VariantMatch, VariantModel,
};
use super::state::BlockState;

/// Resolver for matching BlockState properties against `BlockStateDefinition`.
pub struct BlockStateResolver;

impl BlockStateResolver {
    /// Resolves active model variants from a BlockState definition.
    pub fn resolve(
        definition: &BlockStateDefinition,
        blockstate: &BlockState,
    ) -> Vec<VariantMatch> {
        let namespace = &blockstate.namespace;
        let props = &blockstate.properties;

        // 1. Check variants
        if let Some(ref variants) = definition.variants {
            if let Some(m) = Self::match_variants(variants, props, namespace) {
                return vec![m];
            }
            // Fallback to empty string "" variant
            if let Some(v) = variants.get("") {
                return vec![Self::model_to_match(v.select_primary(), namespace, props)];
            }
            // Fallback to arbitrary first variant
            if let Some((_, v)) = variants.iter().next() {
                return vec![Self::model_to_match(v.select_primary(), namespace, props)];
            }
        }

        // 2. Check multipart
        if let Some(ref multipart) = definition.multipart {
            let mut matches = Vec::new();
            for part in multipart {
                let applies = match &part.when {
                    None => true,
                    Some(cond) => Self::evaluate_condition(cond, props),
                };
                if applies {
                    let model = part.apply.select_primary();
                    matches.push(Self::model_to_match(model, namespace, props));
                }
            }
            if !matches.is_empty() {
                return matches;
            }

            // Fallback to first multipart rule if none matched
            if let Some(first_part) = multipart.first() {
                let model = first_part.apply.select_primary();
                return vec![Self::model_to_match(model, namespace, props)];
            }
        }

        // 3. Heuristic fallback when definition is empty
        vec![Self::heuristic_match(blockstate)]
    }

    /// Evaluates a `MultipartCondition` against the given properties.
    pub fn evaluate_condition(
        condition: &MultipartCondition,
        props: &BTreeMap<String, String>,
    ) -> bool {
        match condition {
            MultipartCondition::Or { or } => {
                or.iter().any(|sub_cond| Self::evaluate_condition(sub_cond, props))
            }
            MultipartCondition::And { and } => {
                and.iter().all(|sub_cond| Self::evaluate_condition(sub_cond, props))
            }
            MultipartCondition::Properties(dict) => {
                for (k, expected_val) in dict {
                    let mut actual_v = props.get(k.as_str()).map(|s| s.as_str());
                    if actual_v.is_none() {
                        if k == "flower_amount" {
                            actual_v = props.get("flowers").map(|s| s.as_str());
                        } else if k == "flowers" {
                            actual_v = props.get("flower_amount").map(|s| s.as_str());
                        } else if matches!(k.as_str(), "east" | "north" | "south" | "west") {
                            actual_v = Some("none");
                        }
                    }
                    let actual = actual_v.unwrap_or("");
                    let matched = match expected_val {
                        serde_json::Value::Bool(b) => {
                            let expected_str = if *b { "true" } else { "false" };
                            expected_str.eq_ignore_ascii_case(actual)
                        }
                        serde_json::Value::Number(num) => {
                            let num_str = num.to_string();
                            num_str.eq_ignore_ascii_case(actual)
                        }
                        serde_json::Value::String(s) => {
                            s.split('|').any(|option| option.trim().eq_ignore_ascii_case(actual))
                        }
                        _ => false,
                    };
                    if !matched {
                        return false;
                    }
                }
                true
            }
        }
    }

    fn match_variants(
        variants: &HashMap<String, VariantEntry>,
        props: &BTreeMap<String, String>,
        namespace: &str,
    ) -> Option<VariantMatch> {
        // Fast path: exact sorted key
        let exact_key = props
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join(",");
        if let Some(entry) = variants.get(&exact_key) {
            return Some(Self::model_to_match(entry.select_primary(), namespace, props));
        }

        // Match variants by scoring compatibility against props matching Python blockstate_resolver
        let mut best_entry: Option<&VariantEntry> = None;
        let mut best_score = -999999i32;

        for (v_key, entry) in variants {
            if v_key.is_empty() {
                if props.is_empty() && best_score < 0 {
                    best_score = 0;
                    best_entry = Some(entry);
                }
                continue;
            }

            let mut v_props: HashMap<&str, &str> = HashMap::new();
            for pair in v_key.split(',') {
                if let Some((k, v)) = pair.split_once('=') {
                    v_props.insert(k.trim(), v.trim());
                }
            }

            // Compatibility: every property in props must match v_props if present in v_props
            let mut compatible = true;
            let mut matched_keys = 0i32;
            for (k, v) in props {
                if let Some(&vv) = v_props.get(k.as_str()) {
                    if vv != v.as_str() {
                        compatible = false;
                        break;
                    }
                    matched_keys += 1;
                }
            }

            if !compatible {
                continue;
            }

            let mut score = matched_keys * 100;
            if v_props.len() == props.len() {
                score += 1000;
            }

            // Score keys in v_props that are not specified in props: prefer standard default states
            for (vk, vv) in &v_props {
                if !props.contains_key(*vk) {
                    if matches!(
                        *vv,
                        "false" | "0" | "none" | "straight" | "bottom" | "lower" | "single"
                            | "foot" | "normal" | "side" | "y" | "north"
                    ) {
                        score += 10;
                    } else if matches!(
                        *vv,
                        "true" | "1" | "top" | "upper" | "head" | "inner" | "outer" | "double"
                            | "x" | "z" | "south" | "east" | "west"
                    ) {
                        score -= 10;
                    }
                }
            }

            if score > best_score {
                best_score = score;
                best_entry = Some(entry);
            }
        }

        best_entry.map(|entry| Self::model_to_match(entry.select_primary(), namespace, props))
    }

    fn model_to_match(
        model: &VariantModel,
        default_namespace: &str,
        props: &BTreeMap<String, String>,
    ) -> VariantMatch {
        let mut model_id = model.model.clone();
        if !model_id.contains(':') {
            model_id = format!("{}:{}", default_namespace, model_id);
        }
        VariantMatch {
            model_id,
            rot_x: model.x,
            rot_y: model.y,
            uvlock: model.uvlock,
            weight: model.weight,
            variant_props: Some(props.clone()),
        }
    }

    /// Directional and axis rotation heuristic when no JSON definition is provided.
    pub fn heuristic_match(blockstate: &BlockState) -> VariantMatch {
        let mut rot_x = 0.0;
        let mut rot_y = 0.0;

        let facing = blockstate.properties.get("facing").map(|s| s.as_str());
        let axis = blockstate.properties.get("axis").map(|s| s.as_str());

        if let Some(a) = axis {
            match a {
                "x" => {
                    rot_x = 90.0;
                    rot_y = 90.0;
                }
                "z" => {
                    rot_x = 90.0;
                }
                _ => {}
            }
        } else if let Some(f) = facing {
            match f {
                "north" => rot_y = 0.0,
                "east" => rot_y = 90.0,
                "south" => rot_y = 180.0,
                "west" => rot_y = 270.0,
                "up" => rot_x = 270.0,
                "down" => rot_x = 90.0,
                _ => {}
            }
        }

        VariantMatch {
            model_id: format!("{}:block/{}", blockstate.namespace, blockstate.name),
            rot_x,
            rot_y,
            uvlock: false,
            weight: 1,
            variant_props: Some(blockstate.properties.clone()),
        }
    }
}

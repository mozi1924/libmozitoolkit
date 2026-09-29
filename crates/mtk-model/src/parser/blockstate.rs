use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::ModelError;

/// Parsed Minecraft BlockState representation.
/// Separates namespace, block identifier, and sorted key-value properties.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BlockState {
    /// Namespace (defaults to "minecraft" if omitted).
    pub namespace: String,
    /// Block identifier name (e.g. "observer", "stone", "oak_stairs").
    pub name: String,
    /// Sorted properties map (e.g. {"facing": "north", "powered": "false"}).
    pub properties: BTreeMap<String, String>,
}

impl BlockState {
    /// Parses a BlockState string into a structured `BlockState`.
    ///
    /// # Examples
    /// - `"stone"` -> `minecraft:stone`
    /// - `"minecraft:observer[facing=north,powered=false]"`
    pub fn parse(s: &str) -> Result<Self, ModelError> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(ModelError::InvalidBlockStateSyntax(
                "Empty string".to_string(),
            ));
        }

        let (base_part, prop_part) = if let Some(open_bracket) = trimmed.find('[') {
            if !trimmed.ends_with(']') {
                return Err(ModelError::InvalidBlockStateSyntax(format!(
                    "Missing closing bracket in '{}'",
                    trimmed
                )));
            }
            let base = &trimmed[..open_bracket];
            let props = &trimmed[open_bracket + 1..trimmed.len() - 1];
            (base, Some(props))
        } else {
            (trimmed, None)
        };

        // Parse namespace and name
        let (namespace, name) = if let Some(colon) = base_part.find(':') {
            let ns = &base_part[..colon];
            let n = &base_part[colon + 1..];
            (ns.to_string(), n.to_string())
        } else {
            ("minecraft".to_string(), base_part.to_string())
        };

        let mut properties = BTreeMap::new();
        if let Some(props_str) = prop_part {
            let props_trimmed = props_str.trim();
            if !props_trimmed.is_empty() {
                for item in props_trimmed.split(',') {
                    let pair = item.trim();
                    if pair.is_empty() {
                        continue;
                    }
                    if let Some(eq_idx) = pair.find('=') {
                        let k = pair[..eq_idx].trim().to_ascii_lowercase();
                        let v = pair[eq_idx + 1..].trim().to_ascii_lowercase();
                        properties.insert(k, v);
                    } else {
                        return Err(ModelError::InvalidBlockStateSyntax(format!(
                            "Invalid property item '{}' in '{}'",
                            pair, trimmed
                        )));
                    }
                }
            }
        }

        Ok(Self {
            namespace,
            name,
            properties,
        })
    }

    /// Full qualified identifier `namespace:name` without properties.
    #[inline]
    pub fn block_id(&self) -> String {
        format!("{}:{}", self.namespace, self.name)
    }

    /// Returns canonical representation where properties are deterministically sorted.
    pub fn to_canonical_string(&self) -> String {
        if self.properties.is_empty() {
            format!("{}:{}", self.namespace, self.name)
        } else {
            let props_str: Vec<String> = self
                .properties
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect();
            format!("{}:{}[{}]", self.namespace, self.name, props_str.join(","))
        }
    }

    /// Checks if a subset of required properties matches this blockstate.
    pub fn matches_properties<'a, I>(&self, required_props: I) -> bool
    where
        I: IntoIterator<Item = (&'a str, &'a str)>,
    {
        for (k, v) in required_props {
            match self.properties.get(k) {
                Some(curr_v) if curr_v == v => continue,
                _ => return false,
            }
        }
        true
    }
}

impl FromStr for BlockState {
    type Err = ModelError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl fmt::Display for BlockState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_canonical_string())
    }
}

/// A matched model variant with required transforms.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VariantMatch {
    /// Target model JSON path, e.g. "minecraft:block/oak_stairs".
    pub model_id: String,
    /// X rotation angle in degrees: 0, 90, 180, 270.
    pub rot_x: f32,
    /// Y rotation angle in degrees: 0, 90, 180, 270.
    pub rot_y: f32,
    /// Whether UVLock is enabled for this variant.
    pub uvlock: bool,
    /// Variant selection weight.
    pub weight: u32,
    /// Additional variant properties if applicable.
    pub variant_props: Option<BTreeMap<String, String>>,
}

impl Default for VariantMatch {
    fn default() -> Self {
        Self {
            model_id: String::new(),
            rot_x: 0.0,
            rot_y: 0.0,
            uvlock: false,
            weight: 1,
            variant_props: None,
        }
    }
}

/// Single model application entry in a variant or multipart definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VariantModel {
    /// Model identifier path.
    pub model: String,
    /// X rotation in degrees (0, 90, 180, 270).
    #[serde(default)]
    pub x: f32,
    /// Y rotation in degrees (0, 90, 180, 270).
    #[serde(default)]
    pub y: f32,
    /// UV lock boolean.
    #[serde(default)]
    pub uvlock: bool,
    /// Selection weight (defaults to 1).
    #[serde(default = "default_weight")]
    pub weight: u32,
}

fn default_weight() -> u32 {
    1
}

/// Variant entry which can be a single model or a list of weighted models.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum VariantEntry {
    Single(VariantModel),
    List(Vec<VariantModel>),
}

impl VariantEntry {
    /// Chooses the primary model (highest weight or first entry).
    pub fn select_primary(&self) -> &VariantModel {
        match self {
            VariantEntry::Single(m) => m,
            VariantEntry::List(list) => {
                let mut best = &list[0];
                for item in list.iter().skip(1) {
                    if item.weight > best.weight {
                        best = item;
                    }
                }
                best
            }
        }
    }
}

/// Condition specification in a multipart rule.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MultipartCondition {
    /// OR condition: any item in the list matching satisfies the rule.
    Or {
        #[serde(alias = "or", rename = "OR")]
        or: Vec<MultipartCondition>,
    },
    /// AND condition: all items in the list matching satisfies the rule.
    And {
        #[serde(alias = "and", rename = "AND")]
        and: Vec<MultipartCondition>,
    },
    /// Flat dictionary: ALL keys must match (AND condition).
    /// Values can be boolean, integer, or pipe-separated multiple choices (e.g. "north|south").
    Properties(HashMap<String, serde_json::Value>),
}

/// Multipart rule definition containing an optional condition and model to apply.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MultipartRule {
    /// Activation condition. If None, always applied.
    #[serde(default)]
    pub when: Option<MultipartCondition>,
    /// Model variant to apply.
    pub apply: VariantEntry,
}

/// Representation of a Minecraft BlockState definition JSON (assets/<namespace>/blockstates/<name>.json).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BlockStateDefinition {
    /// Standard key-value variants map.
    #[serde(default)]
    pub variants: Option<HashMap<String, VariantEntry>>,
    /// Multipart rules array for composite blocks (fences, walls, redstone wires, etc.).
    #[serde(default)]
    pub multipart: Option<Vec<MultipartRule>>,
}

impl BlockStateDefinition {
    /// Enumerates all canonical blockstate strings described by this definition.
    ///
    /// # Arguments
    /// - `base_id`: Canonical block id, e.g. `"minecraft:oak_stairs"` or `"minecraft:stone"`
    pub fn enumerate_all_states(&self, base_id: &str) -> Vec<String> {
        let mut results = Vec::new();
        let mut seen = BTreeSet::new();

        if let Some(ref variants) = self.variants {
            if !variants.is_empty() {
                if variants.len() == 1 && variants.contains_key("") {
                    if let Some(expanded) = Self::expand_known_entity_states(base_id) {
                        for st in expanded {
                            if let Ok(bs) = BlockState::parse(&st) {
                                let canon = bs.to_canonical_string();
                                if seen.insert(canon.clone()) {
                                    results.push(canon);
                                }
                            }
                        }
                        if seen.insert(base_id.to_string()) {
                            results.push(base_id.to_string());
                        }
                        return results;
                    }
                }

                for key in variants.keys() {
                    let state_str = if key.is_empty() {
                        base_id.to_string()
                    } else {
                        format!("{}[{}]", base_id, key)
                    };
                    if let Ok(bs) = BlockState::parse(&state_str) {
                        let canon = bs.to_canonical_string();
                        if seen.insert(canon.clone()) {
                            results.push(canon);
                        }
                    } else if seen.insert(state_str.clone()) {
                        results.push(state_str);
                    }
                }
                return results;
            }
        }

        if let Some(ref multipart) = self.multipart {
            if !multipart.is_empty() {
                let mut prop_values: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

                for rule in multipart {
                    if let Some(ref when) = rule.when {
                        Self::extract_condition_prop_values(when, &mut prop_values);
                    }
                }

                // If boolean properties only had true or false, expand to both
                for (k, vals) in prop_values.iter_mut() {
                    if vals.contains("true") || vals.contains("false") {
                        vals.insert("true".to_string());
                        vals.insert("false".to_string());
                    }
                    if k == "flower_amount" || k == "flowers" || k == "pickles" || k == "candles" {
                        vals.insert("1".to_string());
                    }
                }

                if prop_values.is_empty() {
                    results.push(base_id.to_string());
                } else {
                    let keys: Vec<String> = prop_values.keys().cloned().collect();
                    let value_lists: Vec<Vec<String>> = keys
                        .iter()
                        .map(|k| prop_values[k].iter().cloned().collect())
                        .collect();

                    let mut total_combos = 1usize;
                    for list in &value_lists {
                        total_combos = total_combos.saturating_mul(list.len());
                    }

                    if total_combos <= 512 {
                        let mut combos: Vec<Vec<(String, String)>> = vec![Vec::new()];
                        for (key, values) in keys.iter().zip(value_lists.iter()) {
                            let mut next_combos = Vec::new();
                            for existing in combos {
                                for val in values {
                                    let mut cloned = existing.clone();
                                    cloned.push((key.clone(), val.clone()));
                                    next_combos.push(cloned);
                                }
                            }
                            combos = next_combos;
                        }

                        for combo in combos {
                            let props_str: Vec<String> = combo
                                .into_iter()
                                .map(|(k, v)| format!("{}={}", k, v))
                                .collect();
                            let state_str = format!("{}[{}]", base_id, props_str.join(","));
                            if let Ok(bs) = BlockState::parse(&state_str) {
                                let canon = bs.to_canonical_string();
                                if seen.insert(canon.clone()) {
                                    results.push(canon);
                                }
                            } else if seen.insert(state_str.clone()) {
                                results.push(state_str);
                            }
                        }
                    } else {
                        // Fallback to base id if combinatorial explosion
                        results.push(base_id.to_string());
                    }
                }
                return results;
            }
        }

        if seen.insert(base_id.to_string()) {
            results.push(base_id.to_string());
        }
        results
    }

    fn extract_condition_prop_values(
        condition: &MultipartCondition,
        prop_values: &mut BTreeMap<String, BTreeSet<String>>,
    ) {
        match condition {
            MultipartCondition::Or { or } => {
                for sub in or {
                    Self::extract_condition_prop_values(sub, prop_values);
                }
            }
            MultipartCondition::And { and } => {
                for sub in and {
                    Self::extract_condition_prop_values(sub, prop_values);
                }
            }
            MultipartCondition::Properties(map) => {
                for (k, val) in map {
                    let set = prop_values.entry(k.clone()).or_default();
                    match val {
                        serde_json::Value::Bool(b) => {
                            set.insert(b.to_string());
                        }
                        serde_json::Value::Number(n) => {
                            set.insert(n.to_string());
                        }
                        serde_json::Value::String(s) => {
                            for item in s.split('|') {
                                set.insert(item.trim().to_string());
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    fn expand_known_entity_states(base_id: &str) -> Option<Vec<String>> {
        let short_name = base_id.strip_prefix("minecraft:").unwrap_or(base_id);

        if short_name == "chest" || short_name == "trapped_chest" || short_name.ends_with("_chest") {
            if short_name == "chest_boat" || short_name.ends_with("_chest_boat") {
                return None;
            }
            let mut list = Vec::new();
            let facings = ["north", "south", "east", "west"];
            if short_name == "ender_chest" {
                for f in facings {
                    list.push(format!("{}[facing={}]", base_id, f));
                }
            } else {
                let types = ["single", "left", "right"];
                for f in facings {
                    for t in types {
                        list.push(format!("{}[facing={},type={}]", base_id, f, t));
                    }
                }
            }
            return Some(list);
        }

        if short_name.ends_with("_head") || short_name.ends_with("_skull") {
            let mut list = Vec::new();
            if short_name.contains("_wall_") {
                for f in ["north", "south", "east", "west"] {
                    list.push(format!("{}[facing={}]", base_id, f));
                }
            } else {
                for r in 0..16 {
                    list.push(format!("{}[rotation={}]", base_id, r));
                }
            }
            return Some(list);
        }

        if short_name == "shulker_box" || short_name.ends_with("_shulker_box") {
            let mut list = Vec::new();
            for f in ["down", "up", "north", "south", "west", "east"] {
                list.push(format!("{}[facing={}]", base_id, f));
            }
            return Some(list);
        }

        if short_name.ends_with("_banner") {
            let mut list = Vec::new();
            if short_name.contains("_wall_") {
                for f in ["north", "south", "east", "west"] {
                    list.push(format!("{}[facing={}]", base_id, f));
                }
            } else {
                for r in 0..16 {
                    list.push(format!("{}[rotation={}]", base_id, r));
                }
            }
            return Some(list);
        }

        if short_name.ends_with("_sign") || short_name.contains("hanging_sign") {
            let mut list = Vec::new();
            if short_name.contains("_wall_") {
                for f in ["north", "south", "east", "west"] {
                    list.push(format!("{}[facing={}]", base_id, f));
                }
            } else {
                for r in 0..16 {
                    list.push(format!("{}[rotation={}]", base_id, r));
                }
            }
            return Some(list);
        }

        if short_name == "decorated_pot" {
            let mut list = Vec::new();
            for f in ["north", "south", "east", "west"] {
                list.push(format!("{}[facing={}]", base_id, f));
            }
            return Some(list);
        }

        None
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let bs = BlockState::parse("stone").unwrap();
        assert_eq!(bs.namespace, "minecraft");
        assert_eq!(bs.name, "stone");
        assert!(bs.properties.is_empty());
        assert_eq!(bs.to_canonical_string(), "minecraft:stone");
    }

    #[test]
    fn test_parse_with_properties() {
        let input = "minecraft:observer[powered=false,facing=north]";
        let bs = BlockState::parse(input).unwrap();
        assert_eq!(bs.block_id(), "minecraft:observer");
        assert_eq!(bs.properties.get("facing").unwrap(), "north");
        assert_eq!(bs.properties.get("powered").unwrap(), "false");
        assert_eq!(
            bs.to_canonical_string(),
            "minecraft:observer[facing=north,powered=false]"
        );
    }

    #[test]
    fn test_matches_properties() {
        let bs =
            BlockState::parse("minecraft:oak_stairs[facing=east,half=bottom,shape=straight]").unwrap();
        assert!(bs.matches_properties([("facing", "east"), ("half", "bottom")]));
        assert!(!bs.matches_properties([("facing", "west")]));
    }

    #[test]
    fn test_resolve_variants() {
        let json_data = r#"{
            "variants": {
                "facing=north": { "model": "minecraft:block/furnace" },
                "facing=south": { "model": "minecraft:block/furnace", "y": 180 },
                "facing=west":  { "model": "minecraft:block/furnace", "y": 270 },
                "facing=east":  { "model": "minecraft:block/furnace", "y": 90 }
            }
        }"#;
        let def: BlockStateDefinition = serde_json::from_str(json_data).unwrap();
        let bs = BlockState::parse("minecraft:furnace[facing=east]").unwrap();
        let matches = BlockStateResolver::resolve(&def, &bs);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].model_id, "minecraft:block/furnace");
        assert_eq!(matches[0].rot_y, 90.0);
    }

    #[test]
    fn test_resolve_multipart_conditions() {
        let json_data = r#"{
            "multipart": [
                {
                    "apply": { "model": "minecraft:block/oak_fence_post" }
                },
                {
                    "when": { "north": "true" },
                    "apply": { "model": "minecraft:block/oak_fence_side", "uvlock": true }
                },
                {
                    "when": { "OR": [{ "south": "true" }, { "east": "true" }] },
                    "apply": { "model": "minecraft:block/oak_fence_side", "y": 90 }
                }
            ]
        }"#;
        let def: BlockStateDefinition = serde_json::from_str(json_data).unwrap();
        let bs = BlockState::parse("minecraft:oak_fence[north=true,south=false,east=true]").unwrap();
        let matches = BlockStateResolver::resolve(&def, &bs);
        // Should match post (unconditional), north=true, and OR (east=true) -> 3 models
        assert_eq!(matches.len(), 3);
        assert_eq!(matches[0].model_id, "minecraft:block/oak_fence_post");
        assert_eq!(matches[1].model_id, "minecraft:block/oak_fence_side");
        assert!(matches[1].uvlock);
        assert_eq!(matches[2].rot_y, 90.0);
    }

    #[test]
    fn test_resolve_chiseled_bookshelf_and_condition() {
        let json_data = r#"{
            "multipart": [
                {
                    "apply": { "model": "minecraft:block/chiseled_bookshelf", "y": 180 },
                    "when": { "facing": "south" }
                },
                {
                    "apply": { "model": "minecraft:block/chiseled_bookshelf_occupied_slot_top_left", "y": 180 },
                    "when": {
                        "AND": [
                            { "facing": "south" },
                            { "slot_0_occupied": "true" }
                        ]
                    }
                },
                {
                    "apply": { "model": "minecraft:block/chiseled_bookshelf_empty_slot_top_left", "y": 180 },
                    "when": {
                        "AND": [
                            { "facing": "south" },
                            { "slot_0_occupied": "false" }
                        ]
                    }
                }
            ]
        }"#;
        let def: BlockStateDefinition = serde_json::from_str(json_data).unwrap();
        let bs_occupied = BlockState::parse("minecraft:chiseled_bookshelf[facing=south,slot_0_occupied=true]").unwrap();
        let matches_occ = BlockStateResolver::resolve(&def, &bs_occupied);
        assert_eq!(matches_occ.len(), 2);
        assert_eq!(matches_occ[0].model_id, "minecraft:block/chiseled_bookshelf");
        assert_eq!(matches_occ[1].model_id, "minecraft:block/chiseled_bookshelf_occupied_slot_top_left");

        let bs_empty = BlockState::parse("minecraft:chiseled_bookshelf[facing=south,slot_0_occupied=false]").unwrap();
        let matches_emp = BlockStateResolver::resolve(&def, &bs_empty);
        assert_eq!(matches_emp.len(), 2);
        assert_eq!(matches_emp[0].model_id, "minecraft:block/chiseled_bookshelf");
        assert_eq!(matches_emp[1].model_id, "minecraft:block/chiseled_bookshelf_empty_slot_top_left");
    }

    #[test]
    fn test_resolve_wildflowers_flower_amount_and_flowers_alias() {
        let json_data = r#"{
            "multipart": [
                {
                    "apply": { "model": "minecraft:block/wildflowers_1" },
                    "when": { "facing": "north" }
                },
                {
                    "apply": { "model": "minecraft:block/wildflowers_2" },
                    "when": { "facing": "north", "flower_amount": "2|3|4" }
                }
            ]
        }"#;
        let def: BlockStateDefinition = serde_json::from_str(json_data).unwrap();
        
        // 1 flower -> only wildflowers_1
        let bs1 = BlockState::parse("minecraft:wildflowers[facing=north,flower_amount=1]").unwrap();
        let m1 = BlockStateResolver::resolve(&def, &bs1);
        assert_eq!(m1.len(), 1);
        assert_eq!(m1[0].model_id, "minecraft:block/wildflowers_1");

        // 2 flowers -> wildflowers_1 and wildflowers_2
        let bs2 = BlockState::parse("minecraft:wildflowers[facing=north,flower_amount=2]").unwrap();
        let m2 = BlockStateResolver::resolve(&def, &bs2);
        assert_eq!(m2.len(), 2);
        assert_eq!(m2[0].model_id, "minecraft:block/wildflowers_1");
        assert_eq!(m2[1].model_id, "minecraft:block/wildflowers_2");

        // Backward compatibility alias: "flowers=2" matches "flower_amount: 2|3|4"
        let bs_alias = BlockState::parse("minecraft:wildflowers[facing=north,flowers=2]").unwrap();
        let m_alias = BlockStateResolver::resolve(&def, &bs_alias);
        assert_eq!(m_alias.len(), 2);
        assert_eq!(m_alias[1].model_id, "minecraft:block/wildflowers_2");
    }
}

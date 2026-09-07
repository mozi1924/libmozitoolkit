use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::ModelError;

/// Parsed Minecraft BlockState representation.
/// Separates namespace, block identifier, and sorted key-value properties.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
                list.iter().max_by_key(|m| m.weight).unwrap_or(&list[0])
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
        #[serde(rename = "OR")]
        or: Vec<HashMap<String, String>>,
    },
    /// Flat dictionary: ALL keys must match (AND condition).
    /// Values can contain pipe characters for multiple choices (e.g. "north|south").
    And(HashMap<String, String>),
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
                or.iter().any(|sub_cond| Self::match_and_dict(sub_cond, props))
            }
            MultipartCondition::And(dict) => Self::match_and_dict(dict, props),
        }
    }

    fn match_and_dict(dict: &HashMap<String, String>, props: &BTreeMap<String, String>) -> bool {
        for (k, expected_v) in dict {
            let actual_v = props.get(k.as_str()).map(|s| s.as_str()).unwrap_or("");
            // Handle pipe-separated multiple possible values: "north|south"
            let matched = expected_v
                .split('|')
                .any(|option| option.trim().eq_ignore_ascii_case(actual_v));
            if !matched {
                return false;
            }
        }
        true
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

        // Scored match: find variant that matches all its defined criteria and has maximum matched properties
        let mut best_entry: Option<&VariantEntry> = None;
        let mut best_score = -1i32;

        for (v_key, entry) in variants {
            if v_key.is_empty() {
                continue;
            }
            let mut matches_all = true;
            let mut count = 0;
            for pair in v_key.split(',') {
                if let Some((k, v)) = pair.split_once('=') {
                    let k = k.trim();
                    let v = v.trim();
                    if props.get(k).map(|s| s.as_str()) != Some(v) {
                        matches_all = false;
                        break;
                    }
                    count += 1;
                }
            }
            if matches_all && count > best_score {
                best_score = count;
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
}

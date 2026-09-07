use std::collections::BTreeMap;
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
                        let k = pair[..eq_idx].trim().to_string();
                        let v = pair[eq_idx + 1..].trim().to_string();
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

        // Canonical string must be sorted: facing before powered
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
}

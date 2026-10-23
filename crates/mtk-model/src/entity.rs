use serde::{Deserialize, Serialize};

/// High-level typed BlockEntity representation decoupled from BlockState strings.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "id")]
pub enum BlockEntityData {
    /// Decorated Pot with up to 4 sherd patterns.
    #[serde(rename = "minecraft:decorated_pot", alias = "decorated_pot")]
    DecoratedPot {
        /// Sherd patterns on [back, left, right, front].
        /// `None` indicates default undecorated brick side.
        sherds: [Option<String>; 4],
    },
    /// Banner with base color and pattern layers.
    #[serde(rename = "minecraft:banner", alias = "banner")]
    Banner {
        base_color: String,
        patterns: Vec<(String, String)>,
    },
    /// Raw generic NBT payload for unsupported or custom block entities.
    #[serde(rename = "raw")]
    Raw { data: Vec<u8> },
}

impl BlockEntityData {
    /// Normalizes a Minecraft Pottery Sherd item ID or pattern name into a standard pattern key.
    ///
    /// E.g. `"minecraft:angler_pottery_sherd"` -> `Some("angler")`
    ///      `"minecraft:brick"` -> `None`
    pub fn normalize_sherd_pattern(item_id: &str) -> Option<&'static str> {
        let clean = item_id.strip_prefix("minecraft:").unwrap_or(item_id);
        let stem = clean
            .strip_suffix("_pottery_sherd")
            .or_else(|| clean.strip_suffix("_pottery_pattern"))
            .unwrap_or(clean);

        match stem {
            "angler" => Some("angler"),
            "archer" => Some("archer"),
            "arms_up" => Some("arms_up"),
            "blade" => Some("blade"),
            "brewer" => Some("brewer"),
            "burn" => Some("burn"),
            "danger" => Some("danger"),
            "explorer" => Some("explorer"),
            "flow" => Some("flow"),
            "friend" => Some("friend"),
            "guster" => Some("guster"),
            "heart" => Some("heart"),
            "heartbreak" => Some("heartbreak"),
            "howl" => Some("howl"),
            "miner" => Some("miner"),
            "mourner" => Some("mourner"),
            "plenty" => Some("plenty"),
            "prize" => Some("prize"),
            "scrape" => Some("scrape"),
            "sheaf" => Some("sheaf"),
            "shelter" => Some("shelter"),
            "skull" => Some("skull"),
            "snort" => Some("snort"),
            "brick" | "blank" | "" => None,
            _ => None,
        }
    }

    /// Resolves the texture identifier for a specific sherd side.
    pub fn sherd_texture(pattern: Option<&str>) -> String {
        match pattern.and_then(Self::normalize_sherd_pattern) {
            Some(p) => format!("minecraft:entity/decorated_pot/{}_pottery_pattern", p),
            None => "minecraft:entity/decorated_pot/decorated_pot_side".to_string(),
        }
    }
}

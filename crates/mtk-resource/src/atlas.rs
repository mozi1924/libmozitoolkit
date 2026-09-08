use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::identifier::ResourceLocation;

/// Unstitch region slice definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnstitchRegion {
    pub sprite: ResourceLocation,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Pattern filter for atlas sources.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AtlasFilterPattern {
    #[serde(default)]
    pub namespace: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
}

/// All data-driven Sprite Source types supported by vanilla Minecraft 1.19.3+.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AtlasSource {
    #[serde(rename = "minecraft:directory", alias = "directory")]
    Directory {
        source: String,
        prefix: String,
    },

    #[serde(rename = "minecraft:single", alias = "single")]
    Single {
        resource: ResourceLocation,
        #[serde(default)]
        sprite: Option<ResourceLocation>,
    },

    #[serde(rename = "minecraft:filter", alias = "filter")]
    Filter {
        #[serde(flatten)]
        pattern: AtlasFilterPattern,
    },

    #[serde(rename = "minecraft:unstitch", alias = "unstitch")]
    Unstitch {
        resource: ResourceLocation,
        #[serde(default = "default_divisor")]
        divisor_x: f64,
        #[serde(default = "default_divisor")]
        divisor_y: f64,
        regions: Vec<UnstitchRegion>,
    },

    #[serde(rename = "minecraft:paletted_permutations", alias = "paletted_permutations")]
    PalettedPermutations {
        palette_key: ResourceLocation,
        permutations: HashMap<String, ResourceLocation>,
        textures: Vec<ResourceLocation>,
    },
}

fn default_divisor() -> f64 {
    1.0
}

/// Top-level container for atlas JSON configuration (`assets/<namespace>/atlases/*.json`).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AtlasDefinition {
    pub sources: Vec<AtlasSource>,
}

impl AtlasDefinition {
    pub fn parse_json(json_str: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json_str)
    }
}

/// Standardized Atlas categories corresponding to vanilla Minecraft atlases.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AtlasCategory {
    #[serde(rename = "blocks")]
    Blocks,
    #[serde(rename = "items")]
    Items,
    #[serde(rename = "particles")]
    Particles,
    #[serde(rename = "chests")]
    Chests,
    #[serde(rename = "shulker_boxes")]
    ShulkerBoxes,
    #[serde(rename = "banner_patterns")]
    BannerPatterns,
    #[serde(rename = "shield_patterns")]
    ShieldPatterns,
    #[serde(rename = "armor_trims")]
    ArmorTrims,
    #[serde(rename = "decorated_pot")]
    DecoratedPot,
    #[serde(rename = "paintings")]
    Paintings,
    #[serde(rename = "celestials")]
    Celestials,
    #[serde(rename = "gui")]
    Gui,
    #[serde(rename = "map_decorations")]
    MapDecorations,
    #[serde(rename = "entities")]
    Entities,
    #[serde(rename = "misc")]
    Misc,
    #[serde(untagged)]
    Custom(ResourceLocation),
}

impl AtlasCategory {
    pub const ALL_STANDARD: [AtlasCategory; 14] = [
        AtlasCategory::Blocks,
        AtlasCategory::Items,
        AtlasCategory::Particles,
        AtlasCategory::Chests,
        AtlasCategory::ShulkerBoxes,
        AtlasCategory::BannerPatterns,
        AtlasCategory::ShieldPatterns,
        AtlasCategory::ArmorTrims,
        AtlasCategory::DecoratedPot,
        AtlasCategory::Paintings,
        AtlasCategory::Celestials,
        AtlasCategory::Gui,
        AtlasCategory::MapDecorations,
        AtlasCategory::Entities,
    ];

    /// Canonical name string (e.g. `"blocks"`, `"items"`).
    pub fn as_str(&self) -> &str {
        match self {
            AtlasCategory::Blocks => "blocks",
            AtlasCategory::Items => "items",
            AtlasCategory::Particles => "particles",
            AtlasCategory::Chests => "chests",
            AtlasCategory::ShulkerBoxes => "shulker_boxes",
            AtlasCategory::BannerPatterns => "banner_patterns",
            AtlasCategory::ShieldPatterns => "shield_patterns",
            AtlasCategory::ArmorTrims => "armor_trims",
            AtlasCategory::DecoratedPot => "decorated_pot",
            AtlasCategory::Paintings => "paintings",
            AtlasCategory::Celestials => "celestials",
            AtlasCategory::Gui => "gui",
            AtlasCategory::MapDecorations => "map_decorations",
            AtlasCategory::Entities => "entities",
            AtlasCategory::Misc => "misc",
            AtlasCategory::Custom(loc) => &loc.path,
        }
    }

    /// Associated ResourceLocation for the atlas definition (e.g. `"minecraft:blocks"`).
    pub fn atlas_location(&self) -> ResourceLocation {
        match self {
            AtlasCategory::Blocks => ResourceLocation::vanilla("blocks"),
            AtlasCategory::Items => ResourceLocation::vanilla("items"),
            AtlasCategory::Particles => ResourceLocation::vanilla("particles"),
            AtlasCategory::Chests => ResourceLocation::vanilla("chests"),
            AtlasCategory::ShulkerBoxes => ResourceLocation::vanilla("shulker_boxes"),
            AtlasCategory::BannerPatterns => ResourceLocation::vanilla("banner_patterns"),
            AtlasCategory::ShieldPatterns => ResourceLocation::vanilla("shield_patterns"),
            AtlasCategory::ArmorTrims => ResourceLocation::vanilla("armor_trims"),
            AtlasCategory::DecoratedPot => ResourceLocation::vanilla("decorated_pot"),
            AtlasCategory::Paintings => ResourceLocation::vanilla("paintings"),
            AtlasCategory::Celestials => ResourceLocation::vanilla("celestials"),
            AtlasCategory::Gui => ResourceLocation::vanilla("gui"),
            AtlasCategory::MapDecorations => ResourceLocation::vanilla("map_decorations"),
            AtlasCategory::Entities => ResourceLocation::vanilla("entities"),
            AtlasCategory::Misc => ResourceLocation::vanilla("misc"),
            AtlasCategory::Custom(loc) => loc.clone(),
        }
    }

    /// Parse category name string into AtlasCategory.
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "blocks" | "block" => AtlasCategory::Blocks,
            "items" | "item" => AtlasCategory::Items,
            "particles" | "particle" => AtlasCategory::Particles,
            "chests" | "chest" => AtlasCategory::Chests,
            "shulker_boxes" | "shulker" => AtlasCategory::ShulkerBoxes,
            "banner_patterns" | "banners" => AtlasCategory::BannerPatterns,
            "shield_patterns" | "shields" => AtlasCategory::ShieldPatterns,
            "armor_trims" | "trims" => AtlasCategory::ArmorTrims,
            "decorated_pot" | "pot" => AtlasCategory::DecoratedPot,
            "paintings" | "painting" => AtlasCategory::Paintings,
            "celestials" | "environment" => AtlasCategory::Celestials,
            "gui" => AtlasCategory::Gui,
            "map_decorations" | "map" => AtlasCategory::MapDecorations,
            "entities" | "entity" => AtlasCategory::Entities,
            other => {
                if let Ok(loc) = ResourceLocation::parse(other) {
                    AtlasCategory::Custom(loc)
                } else {
                    AtlasCategory::Misc
                }
            }
        }
    }

    /// Classify any texture path string into its authoritative category.
    pub fn classify_texture_path(path: &str) -> Self {
        let clean = path
            .strip_prefix("assets/minecraft/textures/")
            .or_else(|| path.strip_prefix("textures/"))
            .unwrap_or(path)
            .trim_start_matches('/');

        if clean.starts_with("block/") {
            AtlasCategory::Blocks
        } else if clean.starts_with("item/") {
            AtlasCategory::Items
        } else if clean.starts_with("particle/") {
            AtlasCategory::Particles
        } else if clean.starts_with("painting/") {
            AtlasCategory::Paintings
        } else if clean.starts_with("trims/") {
            AtlasCategory::ArmorTrims
        } else if clean.starts_with("entity/chest/") {
            AtlasCategory::Chests
        } else if clean.starts_with("entity/shulker/") {
            AtlasCategory::ShulkerBoxes
        } else if clean.starts_with("entity/shield/") {
            AtlasCategory::ShieldPatterns
        } else if clean.starts_with("entity/banner/") {
            AtlasCategory::BannerPatterns
        } else if clean.starts_with("entity/decorated_pot/") {
            AtlasCategory::DecoratedPot
        } else if clean.starts_with("environment/") {
            AtlasCategory::Celestials
        } else if clean.starts_with("gui/") {
            AtlasCategory::Gui
        } else if clean.starts_with("map/") {
            AtlasCategory::MapDecorations
        } else if clean.starts_with("entity/") || clean.starts_with("models/armor/") {
            AtlasCategory::Entities
        } else {
            AtlasCategory::Misc
        }
    }

    /// Generates the standard default AtlasDefinition for this category if no atlases/*.json exists.
    pub fn default_definition(&self) -> AtlasDefinition {
        match self {
            AtlasCategory::Blocks => AtlasDefinition {
                sources: vec![
                    AtlasSource::Directory {
                        source: "block".to_string(),
                        prefix: "block/".to_string(),
                    },
                    AtlasSource::Directory {
                        source: "entity/conduit".to_string(),
                        prefix: "entity/conduit/".to_string(),
                    },
                    AtlasSource::Single {
                        resource: ResourceLocation::vanilla("entity/bell/bell_body"),
                        sprite: None,
                    },
                    AtlasSource::Single {
                        resource: ResourceLocation::vanilla("entity/enchantment/enchanting_table_book"),
                        sprite: None,
                    },
                ],
            },
            AtlasCategory::Items => AtlasDefinition {
                sources: vec![
                    AtlasSource::Directory {
                        source: "item".to_string(),
                        prefix: "item/".to_string(),
                    },
                ],
            },
            AtlasCategory::Particles => AtlasDefinition {
                sources: vec![
                    AtlasSource::Directory {
                        source: "particle".to_string(),
                        prefix: "particle/".to_string(),
                    },
                ],
            },
            AtlasCategory::Chests => AtlasDefinition {
                sources: vec![
                    AtlasSource::Directory {
                        source: "entity/chest".to_string(),
                        prefix: "entity/chest/".to_string(),
                    },
                ],
            },
            AtlasCategory::ShulkerBoxes => AtlasDefinition {
                sources: vec![
                    AtlasSource::Directory {
                        source: "entity/shulker".to_string(),
                        prefix: "entity/shulker/".to_string(),
                    },
                ],
            },
            AtlasCategory::BannerPatterns => AtlasDefinition {
                sources: vec![
                    AtlasSource::Directory {
                        source: "entity/banner".to_string(),
                        prefix: "entity/banner/".to_string(),
                    },
                ],
            },
            AtlasCategory::ShieldPatterns => AtlasDefinition {
                sources: vec![
                    AtlasSource::Directory {
                        source: "entity/shield".to_string(),
                        prefix: "entity/shield/".to_string(),
                    },
                ],
            },
            AtlasCategory::ArmorTrims => AtlasDefinition {
                sources: vec![
                    AtlasSource::Directory {
                        source: "trims".to_string(),
                        prefix: "trims/".to_string(),
                    },
                ],
            },
            AtlasCategory::DecoratedPot => AtlasDefinition {
                sources: vec![
                    AtlasSource::Directory {
                        source: "entity/decorated_pot".to_string(),
                        prefix: "entity/decorated_pot/".to_string(),
                    },
                ],
            },
            AtlasCategory::Paintings => AtlasDefinition {
                sources: vec![
                    AtlasSource::Directory {
                        source: "painting".to_string(),
                        prefix: "painting/".to_string(),
                    },
                ],
            },
            AtlasCategory::Celestials => AtlasDefinition {
                sources: vec![
                    AtlasSource::Directory {
                        source: "environment".to_string(),
                        prefix: "environment/".to_string(),
                    },
                ],
            },
            AtlasCategory::Gui => AtlasDefinition {
                sources: vec![
                    AtlasSource::Directory {
                        source: "gui".to_string(),
                        prefix: "gui/".to_string(),
                    },
                ],
            },
            AtlasCategory::MapDecorations => AtlasDefinition {
                sources: vec![
                    AtlasSource::Directory {
                        source: "map".to_string(),
                        prefix: "map/".to_string(),
                    },
                ],
            },
            AtlasCategory::Entities => AtlasDefinition {
                sources: vec![
                    AtlasSource::Directory {
                        source: "entity".to_string(),
                        prefix: "entity/".to_string(),
                    },
                    AtlasSource::Directory {
                        source: "models/armor".to_string(),
                        prefix: "models/armor/".to_string(),
                    },
                ],
            },
            AtlasCategory::Misc | AtlasCategory::Custom(_) => AtlasDefinition::default(),
        }
    }
}

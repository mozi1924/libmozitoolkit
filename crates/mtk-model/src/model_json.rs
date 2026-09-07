use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// High-level representation of a Minecraft Block Model JSON.
///
/// Follows Minecraft 1.21+ format, supporting parent template inheritance,
/// `#texture` variable resolutions, and custom elements.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BlockModelJson {
    /// Optional parent model path (e.g. "minecraft:block/cube_all" or "block/cube").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,

    /// Ambient occlusion flag (defaults to true in Minecraft).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ambientocclusion: Option<bool>,

    /// Texture mappings dictionary (e.g. {"all": "minecraft:block/stone", "particle": "#all"}).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub textures: Option<HashMap<String, TextureValue>>,

    /// 3D box elements definition.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub elements: Option<Vec<ElementJson>>,
}

/// Represents texture entries in Minecraft model JSON, which can be either a simple path
/// or a 1.21+ modern sprite object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TextureValue {
    /// String path, e.g. "minecraft:block/oak_planks" or variable ref "#all"
    Path(String),
    /// Modern Minecraft 1.21+ object format with extra attributes
    SpriteObject {
        sprite: String,
        #[serde(default)]
        force_translucent: Option<bool>,
    },
}

impl TextureValue {
    /// Returns the target sprite string path or reference.
    pub fn as_str(&self) -> &str {
        match self {
            TextureValue::Path(s) => s.as_str(),
            TextureValue::SpriteObject { sprite, .. } => sprite.as_str(),
        }
    }
}

/// 3D box element within a model definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElementJson {
    /// Starting coordinate in [0, 16] space.
    pub from: [f32; 3],

    /// Ending coordinate in [0, 16] space.
    pub to: [f32; 3],

    /// Optional local element rotation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotation: Option<RotationJson>,

    /// Whether to render shadows/shading on this element.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shade: Option<bool>,

    /// Faces dictionary mapping direction ("down", "up", "north", "south", "west", "east") to face data.
    pub faces: HashMap<String, FaceJson>,
}

/// Rotation specification for an element around a fixed origin point.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RotationJson {
    /// Origin pivot point in [0, 16] space.
    pub origin: [f32; 3],

    /// Rotation axis: "x", "y", or "z".
    pub axis: String,

    /// Rotation angle in degrees: typically -45.0, -22.5, 0.0, 22.5, or 45.0.
    pub angle: f32,

    /// Whether to rescale the face to stretch to the cube bounds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rescale: Option<bool>,
}

/// Face properties of an element.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FaceJson {
    /// UV coordinates [min_u, min_v, max_u, max_v] in [0, 16] space.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uv: Option<[f32; 4]>,

    /// Texture reference, e.g. "#side", "#all", or "minecraft:block/stone".
    pub texture: String,

    /// Optional cullface direction tag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cullface: Option<String>,

    /// Clockwise UV rotation: 0, 90, 180, or 270 degrees.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotation: Option<u32>,

    /// Biome / colormap tint index (-1 if none).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tintindex: Option<i16>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_block_model_cube() {
        let json_data = r##"{
            "parent": "minecraft:block/block",
            "textures": {
                "particle": "#all",
                "all": "minecraft:block/stone"
            },
            "elements": [
                {
                    "from": [0, 0, 0],
                    "to": [16, 16, 16],
                    "faces": {
                        "down":  { "uv": [0, 0, 16, 16], "texture": "#all", "cullface": "down" },
                        "up":    { "uv": [0, 0, 16, 16], "texture": "#all", "cullface": "up" },
                        "north": { "uv": [0, 0, 16, 16], "texture": "#all", "cullface": "north" },
                        "south": { "uv": [0, 0, 16, 16], "texture": "#all", "cullface": "south" },
                        "west":  { "uv": [0, 0, 16, 16], "texture": "#all", "cullface": "west" },
                        "east":  { "uv": [0, 0, 16, 16], "texture": "#all", "cullface": "east" }
                    }
                }
            ]
        }"##;

        let model: BlockModelJson = serde_json::from_str(json_data).unwrap();
        assert_eq!(model.parent, Some("minecraft:block/block".to_string()));
        let textures = model.textures.unwrap();
        assert_eq!(textures.get("all").unwrap().as_str(), "minecraft:block/stone");
        assert_eq!(model.elements.unwrap().len(), 1);
    }
}

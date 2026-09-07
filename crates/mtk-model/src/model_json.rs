use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::error::ModelError;

const MAX_PARENT_DEPTH: usize = 32;

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

impl BlockModelJson {
    /// Fully resolves the inheritance tree using a parent loader callback.
    ///
    /// Merges textures (child overrides parent) and inherits elements (first model in the chain providing elements wins).
    /// Resolves all `#var` texture variables to final canonical texture identifiers.
    pub fn resolve_hierarchy<F>(
        &self,
        current_model_id: &str,
        mut parent_loader: F,
    ) -> Result<ResolvedBlockModel, ModelError>
    where
        F: FnMut(&str) -> Option<BlockModelJson>,
    {
        let mut visited = HashSet::new();
        visited.insert(current_model_id.to_string());

        let mut raw_textures = self.textures.clone().unwrap_or_default();
        let mut elements = self.elements.clone();
        let mut ambientocclusion = self.ambientocclusion;

        let mut curr_parent = self.parent.clone();
        let mut depth = 0;

        while let Some(parent_id) = curr_parent {
            depth += 1;
            if depth > MAX_PARENT_DEPTH || visited.contains(&parent_id) {
                return Err(ModelError::CircularParentHierarchy(parent_id));
            }
            visited.insert(parent_id.clone());

            let parent_model = parent_loader(&parent_id).unwrap_or_default();

            // 1. Merge textures: child textures take priority; add parent's only if not present
            if let Some(ref p_textures) = parent_model.textures {
                for (k, v) in p_textures {
                    raw_textures.entry(k.clone()).or_insert_with(|| v.clone());
                }
            }

            // 2. Inherit elements if not defined yet
            if elements.is_none() && parent_model.elements.is_some() {
                elements = parent_model.elements.clone();
            }

            // 3. Inherit ambient occlusion if not defined
            if ambientocclusion.is_none() && parent_model.ambientocclusion.is_some() {
                ambientocclusion = parent_model.ambientocclusion;
            }

            curr_parent = parent_model.parent;
        }

        // Resolve all `#texture` variable references
        let mut resolved_textures = HashMap::new();
        for (k, val) in &raw_textures {
            let final_tex = Self::resolve_texture_value(val.as_str(), &raw_textures)?;
            resolved_textures.insert(k.clone(), final_tex);
        }

        // Bake texture values into elements
        let resolved_elements = if let Some(elems) = elements {
            let mut res_elems = Vec::with_capacity(elems.len());
            for elem in elems {
                let mut res_faces = HashMap::with_capacity(elem.faces.len());
                for (dir, face) in elem.faces {
                    let final_tex =
                        Self::resolve_texture_value(&face.texture, &raw_textures).unwrap_or_else(
                            |_| {
                                face.texture.trim_start_matches('#').to_string()
                            },
                        );
                    res_faces.insert(
                        dir,
                        ResolvedFace {
                            uv: face.uv,
                            texture: final_tex,
                            cullface: face.cullface,
                            rotation: face.rotation.unwrap_or(0),
                            tintindex: face.tintindex.unwrap_or(-1),
                        },
                    );
                }
                res_elems.push(ResolvedElement {
                    from: elem.from,
                    to: elem.to,
                    rotation: elem.rotation,
                    shade: elem.shade.unwrap_or(true),
                    faces: res_faces,
                });
            }
            res_elems
        } else {
            Vec::new()
        };

        Ok(ResolvedBlockModel {
            ambientocclusion: ambientocclusion.unwrap_or(true),
            textures: resolved_textures,
            elements: resolved_elements,
        })
    }

    /// Recursively resolves a `#texture` variable to its terminal path.
    pub fn resolve_texture_value(
        target: &str,
        textures: &HashMap<String, TextureValue>,
    ) -> Result<String, ModelError> {
        let mut curr = target;
        let mut visited = HashSet::new();

        while let Some(var_name) = curr.strip_prefix('#') {
            if visited.contains(var_name) {
                return Err(ModelError::CircularParentHierarchy(format!(
                    "Cyclic texture variable '#{}'",
                    var_name
                )));
            }
            visited.insert(var_name.to_string());

            if let Some(next_val) = textures.get(var_name) {
                curr = next_val.as_str();
            } else {
                return Err(ModelError::UnresolvedTextureVariable(var_name.to_string()));
            }
        }

        Ok(curr.to_string())
    }
}

/// Fully canonical resolved Block Model with parent hierarchy and texture variables flattened.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ResolvedBlockModel {
    /// Ambient occlusion.
    pub ambientocclusion: bool,
    /// Resolved texture dictionary mapping variable names to canonical texture IDs.
    pub textures: HashMap<String, String>,
    /// Fully resolved elements.
    pub elements: Vec<ResolvedElement>,
}

/// Resolved element with concrete textures.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedElement {
    pub from: [f32; 3],
    pub to: [f32; 3],
    pub rotation: Option<RotationJson>,
    pub shade: bool,
    pub faces: HashMap<String, ResolvedFace>,
}

/// Resolved element face with concrete texture identifier.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedFace {
    pub uv: Option<[f32; 4]>,
    pub texture: String,
    pub cullface: Option<String>,
    pub rotation: u32,
    pub tintindex: i16,
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
        let textures = model.textures.as_ref().unwrap();
        assert_eq!(textures.get("all").unwrap().as_str(), "minecraft:block/stone");
        assert_eq!(model.elements.as_ref().unwrap().len(), 1);

        // Test hierarchy and texture resolution
        let resolved = model.resolve_hierarchy("test:model", |_| None).unwrap();
        assert_eq!(resolved.textures.get("all").unwrap(), "minecraft:block/stone");
        assert_eq!(resolved.textures.get("particle").unwrap(), "minecraft:block/stone");
        assert_eq!(resolved.elements.len(), 1);
        assert_eq!(
            resolved.elements[0].faces.get("up").unwrap().texture,
            "minecraft:block/stone"
        );
    }

    #[test]
    fn test_parent_inheritance() {
        let parent_json = r##"{
            "textures": {
                "base": "minecraft:block/dirt"
            },
            "elements": [
                {
                    "from": [0, 0, 0],
                    "to": [16, 8, 16],
                    "faces": {
                        "up": { "texture": "#base" }
                    }
                }
            ]
        }"##;

        let child_json = r##"{
            "parent": "minecraft:block/slab_base",
            "textures": {
                "base": "minecraft:block/oak_planks"
            }
        }"##;

        let child: BlockModelJson = serde_json::from_str(child_json).unwrap();
        let parent: BlockModelJson = serde_json::from_str(parent_json).unwrap();

        let resolved = child
            .resolve_hierarchy("minecraft:block/oak_slab", |id| {
                if id == "minecraft:block/slab_base" {
                    Some(parent.clone())
                } else {
                    None
                }
            })
            .unwrap();

        assert_eq!(resolved.elements.len(), 1);
        assert_eq!(resolved.elements[0].to, [16.0, 8.0, 16.0]);
        // Child's texture should override parent's
        assert_eq!(
            resolved.elements[0].faces.get("up").unwrap().texture,
            "minecraft:block/oak_planks"
        );
    }
}

use std::collections::HashMap;

use glam::{Vec2, Vec3};
use mtk_core::direction::Direction;
use mtk_core::mesh::MeshData;
use serde::{Deserialize, Serialize};

use crate::cull_volume::clip_face_excluding_hidden_volume;
use crate::obj::BakedObjFace;

/// Represents a baked quad face with calculated geometry, UVs, and texture bindings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BakedFace {
    pub direction: Direction,
    pub texture: String,
    pub uv_rot: f32,
    pub uv_bounds: [f32; 4],
    pub tint_index: i16,
    pub cullface: Option<Direction>,
    pub vertices: [Vec3; 4],
    pub uvs: [Vec2; 4],
    pub normal: Vec3,
}

impl Default for BakedFace {
    fn default() -> Self {
        Self {
            direction: Direction::Up,
            texture: String::new(),
            uv_rot: 0.0,
            uv_bounds: [0.0, 0.0, 1.0, 1.0],
            tint_index: -1,
            cullface: None,
            vertices: [Vec3::ZERO; 4],
            uvs: [Vec2::ZERO; 4],
            normal: Vec3::Y,
        }
    }
}

/// Represents a 3D box element within a baked model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BakedElement {
    pub from_pos: [f32; 3],
    pub to_pos: [f32; 3],
    pub faces: HashMap<Direction, BakedFace>,
}

/// Fully baked model containing all elements, directional faces summary, and metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BakedModel {
    pub block_state: String,
    pub elements: Vec<BakedElement>,
    #[serde(default)]
    pub obj_faces: Vec<BakedObjFace>,
    pub faces: [BakedFace; 6],
    pub is_cube: bool,
    pub is_opaque: bool,
    pub is_emissive: bool,
    pub emissive_level: f32,
}

impl BakedModel {
    /// Returns the standard face for a given direction (from 6-face summary).
    pub fn get_face(&self, dir: Direction) -> &BakedFace {
        &self.faces[dir.to_index()]
    }

    /// Converts the baked model geometry into a platform-agnostic, contiguous `MeshData` buffer.
    ///
    /// If `exclude_hidden_volume` is true, internal overlapping faces between adjacent elements
    /// (such as within stairs or multi-element blocks) are clipped out via 2D boolean difference.
    pub fn to_mesh(&self, exclude_hidden_volume: bool) -> MeshData {
        self.to_mesh_with_textures(exclude_hidden_volume).0
    }

    /// Converts geometry into a `MeshData` buffer along with the ordered list of unique texture names.
    pub fn to_mesh_with_textures(&self, exclude_hidden_volume: bool) -> (MeshData, Vec<String>) {
        let mut mesh = MeshData::new();
        let mut texture_to_slot: HashMap<String, u16> = HashMap::new();
        let mut texture_list: Vec<String> = Vec::new();

        // Prepare bounding boxes for hidden volume clipping from actual transformed vertices
        let mut element_bounds = Vec::new();
        if exclude_hidden_volume && self.elements.len() > 1 {
            for el in &self.elements {
                let mut min_pos = Vec3::splat(f32::INFINITY);
                let mut max_pos = Vec3::splat(f32::NEG_INFINITY);
                for face in el.faces.values() {
                    for v in &face.vertices {
                        min_pos = min_pos.min(*v);
                        max_pos = max_pos.max(*v);
                    }
                }
                element_bounds.push(([min_pos.x, min_pos.y, min_pos.z], [max_pos.x, max_pos.y, max_pos.z]));
            }
        }

        // 1. Process JSON elements
        for (el_idx, el) in self.elements.iter().enumerate() {
            let other_bounds: Vec<_> = element_bounds
                .iter()
                .enumerate()
                .filter(|(idx, _)| *idx != el_idx)
                .map(|(_, b)| *b)
                .collect();

            for face in el.faces.values() {
                let slot = *texture_to_slot
                    .entry(face.texture.clone())
                    .or_insert_with(|| {
                        let id = texture_list.len() as u16;
                        texture_list.push(face.texture.clone());
                        id
                    });

                let pieces = if exclude_hidden_volume && !other_bounds.is_empty() {
                    clip_face_excluding_hidden_volume(
                        &face.vertices,
                        &face.uvs,
                        face.direction,
                        &other_bounds,
                    )
                } else {
                    vec![crate::cull_volume::ClippedQuadPiece {
                        vertices: face.vertices,
                        uvs: face.uvs,
                    }]
                };

                for piece in pieces {
                    let base_idx = mesh.positions.len() as u32;
                    let norm = [face.normal.x, face.normal.y, face.normal.z];

                    for i in 0..4 {
                        let v = piece.vertices[i];
                        let uv = piece.uvs[i];
                        mesh.positions.push([v.x, v.y, v.z]);
                        mesh.normals.push(norm);
                        mesh.uvs.push([uv.x, 1.0 - uv.y]);
                    }

                    // Triangulate CCW quad: 0-1-2 and 0-2-3
                    mesh.indices.push(base_idx);
                    mesh.indices.push(base_idx + 1);
                    mesh.indices.push(base_idx + 2);

                    mesh.indices.push(base_idx);
                    mesh.indices.push(base_idx + 2);
                    mesh.indices.push(base_idx + 3);

                    mesh.face_materials.push(slot);
                    mesh.face_tint_indices.push(face.tint_index);
                }
            }
        }

        // 2. Process OBJ faces
        for obj_f in &self.obj_faces {
            let slot = *texture_to_slot
                .entry(obj_f.texture.clone())
                .or_insert_with(|| {
                    let id = texture_list.len() as u16;
                    texture_list.push(obj_f.texture.clone());
                    id
                });

            let base_idx = mesh.positions.len() as u32;
            let norm = [obj_f.normal.x, obj_f.normal.y, obj_f.normal.z];

            for (&v, &uv) in obj_f.vertices.iter().zip(obj_f.uvs.iter()) {
                mesh.positions.push([v.x, v.y, v.z]);
                mesh.normals.push(norm);
                mesh.uvs.push([uv.x, 1.0 - uv.y]);
            }

            let n_verts = obj_f.vertices.len();
            if n_verts == 3 {
                mesh.indices.push(base_idx);
                mesh.indices.push(base_idx + 1);
                mesh.indices.push(base_idx + 2);
                mesh.face_materials.push(slot);
                mesh.face_tint_indices.push(obj_f.tint_index);
            } else if n_verts == 4 {
                mesh.indices.push(base_idx);
                mesh.indices.push(base_idx + 1);
                mesh.indices.push(base_idx + 2);

                mesh.indices.push(base_idx);
                mesh.indices.push(base_idx + 2);
                mesh.indices.push(base_idx + 3);
                mesh.face_materials.push(slot);
                mesh.face_tint_indices.push(obj_f.tint_index);
            } else {
                for i in 1..n_verts - 1 {
                    mesh.indices.push(base_idx);
                    mesh.indices.push(base_idx + i as u32);
                    mesh.indices.push(base_idx + (i + 1) as u32);
                    mesh.face_materials.push(slot);
                    mesh.face_tint_indices.push(obj_f.tint_index);
                }
            }
        }

        (mesh, texture_list)
    }

    /// Converts the baked model into standard Wavefront OBJ text.
    pub fn to_obj_string(&self, object_name: &str, exclude_hidden_volume: bool) -> String {
        let (mesh, textures) = self.to_mesh_with_textures(exclude_hidden_volume);
        crate::obj::mesh_to_obj_string(&mesh, object_name, &textures)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_baked_model_to_mesh() {
        let mut faces = HashMap::new();
        faces.insert(
            Direction::Up,
            BakedFace {
                direction: Direction::Up,
                texture: "minecraft:block/stone".to_string(),
                vertices: [
                    Vec3::new(0.0, 1.0, 0.0),
                    Vec3::new(0.0, 1.0, 1.0),
                    Vec3::new(1.0, 1.0, 1.0),
                    Vec3::new(1.0, 1.0, 0.0),
                ],
                uvs: [
                    Vec2::new(0.0, 0.0),
                    Vec2::new(0.0, 1.0),
                    Vec2::new(1.0, 1.0),
                    Vec2::new(1.0, 0.0),
                ],
                normal: Vec3::Y,
                ..Default::default()
            },
        );

        let model = BakedModel {
            block_state: "minecraft:stone".to_string(),
            elements: vec![BakedElement {
                from_pos: [0.0, 0.0, 0.0],
                to_pos: [16.0, 16.0, 16.0],
                faces,
            }],
            obj_faces: Vec::new(),
            faces: [
                BakedFace::default(),
                BakedFace::default(),
                BakedFace::default(),
                BakedFace::default(),
                BakedFace::default(),
                BakedFace::default(),
            ],
            is_cube: true,
            is_opaque: true,
            is_emissive: false,
            emissive_level: 0.0,
        };

        let mesh = model.to_mesh(false);
        assert_eq!(mesh.vertex_count(), 4);
        assert_eq!(mesh.triangle_count(), 2);
        assert_eq!(mesh.face_count(), 1);
        assert_eq!(mesh.face_materials[0], 0);
    }
}

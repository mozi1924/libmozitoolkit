use std::collections::HashMap;

use glam::Vec3;
use mtk_core::mesh::MeshData;

use crate::cull_volume::clip_face_excluding_hidden_volume;
use super::model::{BakedModel, ModelMeshOptions};

impl BakedModel {
    /// Converts the baked model geometry into a platform-agnostic, contiguous `MeshData` buffer.
    ///
    /// If `exclude_hidden_volume` is true, internal overlapping faces between adjacent elements
    /// (such as within stairs or multi-element blocks) are clipped out via 2D boolean difference.
    pub fn to_mesh(&self, exclude_hidden_volume: bool) -> MeshData {
        self.to_mesh_with_textures(exclude_hidden_volume).0
    }

    /// Converts geometry into a `MeshData` buffer along with the ordered list of unique texture names,
    /// using specified `ModelMeshOptions` for clipping and de-overlapping.
    pub fn to_mesh_with_options(&self, options: &ModelMeshOptions) -> (MeshData, Vec<String>) {
        let mut mesh = MeshData::new();
        let mut texture_to_slot: HashMap<String, u16> = HashMap::new();
        let mut texture_list: Vec<String> = Vec::new();

        // Prepare bounding boxes for hidden volume clipping from actual transformed vertices
        let mut element_bounds = Vec::new();
        if options.clip_hidden_volume && self.elements.len() > 1 {
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

        let mut face_attributes_list: Vec<mtk_core::attributes::FaceAttributes> = Vec::new();

        // 1. Process JSON elements
        for (el_idx, el) in self.elements.iter().enumerate() {
            let other_bounds: Vec<_> = element_bounds
                .iter()
                .enumerate()
                .filter(|(idx, _)| *idx != el_idx)
                .map(|(_, b)| *b)
                .collect();

            for face in el.faces.values() {
                // Skip standalone overlay decal faces: in MoziToolKit/libmtk, overlays are
                // composited directly onto base faces in the shader via companion atlas (_overlay.png).
                if face.texture.ends_with("_overlay")
                    || face.texture.ends_with("_OVERLAY")
                    || face.texture.contains("side_overlay")
                {
                    continue;
                }

                let slot = *texture_to_slot
                    .entry(face.texture.clone())
                    .or_insert_with(|| {
                        let id = texture_list.len() as u16;
                        texture_list.push(face.texture.clone());
                        id
                    });

                let pieces = if options.clip_hidden_volume && !other_bounds.is_empty() {
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

                let emission = if self.is_emissive { self.emissive_level } else { 0.0 };
                let scale_u = (face.uv_bounds[2] - face.uv_bounds[0]).abs();
                let scale_v = (face.uv_bounds[3] - face.uv_bounds[1]).abs();
                let uv_trans = [
                    if scale_u > 0.0 { scale_u } else { 1.0 },
                    if scale_v > 0.0 { scale_v } else { 1.0 },
                    face.uv_bounds[0],
                    face.uv_bounds[1],
                ];

                for piece in pieces {
                    let base_idx = mesh.positions.len() as u32;
                    let norm = [face.normal.x, face.normal.y, face.normal.z];

                    for i in 0..4 {
                        let v = piece.vertices[i];
                        mesh.positions.push([v.x, v.y, v.z]);
                        mesh.normals.push(norm);
                        if let Some(ref atlas_uvs) = face.atlas_uvs {
                            mesh.uvs.push([atlas_uvs[i].x, atlas_uvs[i].y]);
                        } else {
                            let uv = piece.uvs[i];
                            mesh.uvs.push([uv.x, 1.0 - uv.y]);
                        }
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

                    let (tint_color, tint_data) = ([1.0, 1.0, 1.0, 1.0], [1.0, 1.0, 0.0, 0.0]);

                    face_attributes_list.push(mtk_core::attributes::FaceAttributes {
                        texture_key: face.texture.clone(),
                        material_slot: slot,
                        tint_index: face.tint_index,
                        emission,
                        is_overlay: false,
                        uv_mode: 0,
                        atlas_chunk_id: face.atlas_chunk_id.map(|c| c as u32),
                        atlas_texture_id: face.atlas_texture_id,
                        uv_transform: uv_trans,
                        uv_rotation: face.uv_rot,
                        face_dir: face.direction.to_index() as u8,
                        material_props: [emission, 1.0, 0.0, 0.0],
                        biome_tint_color: tint_color,
                        biome_tint_data: tint_data,
                        ..Default::default()
                    });
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

            let emission = if self.is_emissive { self.emissive_level } else { 0.0 };
            let (tint_color, tint_data) = ([1.0, 1.0, 1.0, 1.0], [1.0, 1.0, 0.0, 0.0]);

            let face_attr = mtk_core::attributes::FaceAttributes {
                texture_key: obj_f.texture.clone(),
                material_slot: slot,
                tint_index: obj_f.tint_index,
                emission,
                is_overlay: false,
                uv_mode: 0,
                atlas_chunk_id: None,
                atlas_texture_id: None,
                uv_transform: [1.0, 1.0, 0.0, 0.0],
                uv_rotation: 0.0,
                face_dir: obj_f.direction.to_index() as u8,
                material_props: [emission, 1.0, 0.0, 0.0],
                biome_tint_color: tint_color,
                biome_tint_data: tint_data,
                ..Default::default()
            };

            let n_verts = obj_f.vertices.len();
            if n_verts == 3 {
                mesh.indices.push(base_idx);
                mesh.indices.push(base_idx + 1);
                mesh.indices.push(base_idx + 2);
                mesh.face_materials.push(slot);
                mesh.face_tint_indices.push(obj_f.tint_index);
                face_attributes_list.push(face_attr);
            } else if n_verts == 4 {
                mesh.indices.push(base_idx);
                mesh.indices.push(base_idx + 1);
                mesh.indices.push(base_idx + 2);

                mesh.indices.push(base_idx);
                mesh.indices.push(base_idx + 2);
                mesh.indices.push(base_idx + 3);
                mesh.face_materials.push(slot);
                mesh.face_tint_indices.push(obj_f.tint_index);
                face_attributes_list.push(face_attr);
            } else {
                for i in 1..n_verts - 1 {
                    mesh.indices.push(base_idx);
                    mesh.indices.push(base_idx + i as u32);
                    mesh.indices.push(base_idx + (i + 1) as u32);
                    mesh.face_materials.push(slot);
                    mesh.face_tint_indices.push(obj_f.tint_index);
                    face_attributes_list.push(face_attr.clone());
                }
            }
        }

        if !face_attributes_list.is_empty() {
            mesh.populate_standard_face_attributes(&face_attributes_list);
        }

        // 3. De-overlapping and Duplicate/Opposite Face Culling (prevents DCC renderer Z-fighting)
        if (options.cull_duplicates || options.cull_coplanar_opposite) && mesh.face_count() > 0 {
            let cull_cfg = mtk_cull::MeshCullConfig {
                tolerance: options.tolerance,
                cull_coplanar_opposite: options.cull_coplanar_opposite,
                cull_duplicates: options.cull_duplicates,
            };
            mesh = mtk_cull::cull_mesh_faces(&mesh, &cull_cfg).mesh;
        }

        (mesh, texture_list)
    }

    /// Converts geometry into a `MeshData` buffer along with the ordered list of unique texture names.
    ///
    /// If `exclude_hidden_volume` is true, clips internal volume overlaps and culls duplicate/opposite faces.
    pub fn to_mesh_with_textures(&self, exclude_hidden_volume: bool) -> (MeshData, Vec<String>) {
        let options = ModelMeshOptions {
            clip_hidden_volume: exclude_hidden_volume,
            cull_duplicates: exclude_hidden_volume,
            cull_coplanar_opposite: exclude_hidden_volume,
            tolerance: 1e-3,
        };
        self.to_mesh_with_options(&options)
    }

    /// Converts the baked model into standard Wavefront OBJ text.
    pub fn to_obj_string(&self, object_name: &str, exclude_hidden_volume: bool) -> String {
        let (mesh, textures) = self.to_mesh_with_textures(exclude_hidden_volume);
        crate::obj::mesh_to_obj_string(&mesh, object_name, &textures)
    }
}

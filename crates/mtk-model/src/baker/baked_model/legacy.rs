use std::collections::HashMap;

use glam::{Vec2, Vec3};
use mtk_core::direction::Direction;
use serde::Deserialize;

use super::database::BakedModelDatabase;
use super::model::{BakedElement, BakedFace, BakedModel};

#[cfg(feature = "std")]
impl BakedModelDatabase {
    pub fn to_bincode(&self) -> Result<Vec<u8>, bincode::Error> {
        bincode::serialize(self)
    }

    pub fn from_bincode(bytes: &[u8]) -> Result<Self, bincode::Error> {
        // 1. Current full format with variant_groups
        if let Ok(mut db) = bincode::deserialize::<BakedModelDatabase>(bytes) {
            for bm in db.models.values_mut() {
                if bm.culled_faces.iter().all(|v| v.is_empty()) && bm.unculled_faces.is_empty() && !bm.elements.is_empty() {
                    bm.rebuild_face_buckets();
                }
            }
            for vg in db.variant_groups.values_mut() {
                for bm in &mut vg.models {
                    if bm.culled_faces.iter().all(|v| v.is_empty()) && bm.unculled_faces.is_empty() && !bm.elements.is_empty() {
                        bm.rebuild_face_buckets();
                    }
                }
            }
            return Ok(db);
        }

        // 2. Legacy models-only HashMap fallback
        if let Ok(mut models) = bincode::deserialize::<HashMap<String, BakedModel>>(bytes) {
            for bm in models.values_mut() {
                if bm.culled_faces.iter().all(|v| v.is_empty()) && bm.unculled_faces.is_empty() && !bm.elements.is_empty() {
                    bm.rebuild_face_buckets();
                }
            }
            return Ok(Self {
                models,
                variant_groups: HashMap::new(),
            });
        }

        #[derive(Deserialize)]
        struct LegacyModelV2 {
            block_state: String,
            elements: Vec<BakedElement>,
            #[serde(default)]
            obj_faces: Vec<crate::obj::BakedObjFace>,
            faces: [BakedFace; 6],
            is_cube: bool,
            is_opaque: bool,
            is_emissive: bool,
            emissive_level: f32,
            cull_meta: Option<mtk_cull::BlockCullMeta>,
        }

        if let Ok(legacy_v2) = bincode::deserialize::<HashMap<String, LegacyModelV2>>(bytes) {
            let mut models = HashMap::with_capacity(legacy_v2.len());
            for (st, lm) in legacy_v2 {
                let mut bm = BakedModel {
                    block_state: lm.block_state,
                    elements: lm.elements,
                    obj_faces: lm.obj_faces,
                    faces: lm.faces,
                    is_cube: lm.is_cube,
                    is_opaque: lm.is_opaque,
                    is_emissive: lm.is_emissive,
                    emissive_level: lm.emissive_level,
                    cull_meta: lm.cull_meta,
                    culled_faces: Default::default(),
                    unculled_faces: Default::default(),
                };
                if bm.cull_meta.is_none() {
                    bm.cull_meta = Some(bm.get_or_compute_cull_meta());
                }
                bm.rebuild_face_buckets();
                models.insert(st, bm);
            }
            return Ok(Self {
                models,
                variant_groups: HashMap::new(),
            });
        }

        #[derive(Clone, Deserialize)]
        struct LegacyFaceV0 {
            direction: Direction,
            texture: String,
            uv_rot: f32,
            uv_bounds: [f32; 4],
            tint_index: i16,
            cullface: Option<Direction>,
            vertices: [Vec3; 4],
            uvs: [Vec2; 4],
            normal: Vec3,
        }

        #[derive(Deserialize)]
        struct LegacyElementV0 {
            from_pos: [f32; 3],
            to_pos: [f32; 3],
            faces: HashMap<Direction, LegacyFaceV0>,
        }

        #[derive(Deserialize)]
        struct LegacyModelV0 {
            block_state: String,
            elements: Vec<LegacyElementV0>,
            #[serde(default)]
            obj_faces: Vec<crate::obj::BakedObjFace>,
            faces: [LegacyFaceV0; 6],
            is_cube: bool,
            is_opaque: bool,
            is_emissive: bool,
            emissive_level: f32,
        }

        #[derive(Deserialize)]
        struct LegacyModelV1 {
            block_state: String,
            elements: Vec<LegacyElementV0>,
            #[serde(default)]
            obj_faces: Vec<crate::obj::BakedObjFace>,
            faces: [LegacyFaceV0; 6],
            is_cube: bool,
            is_opaque: bool,
            is_emissive: bool,
            emissive_level: f32,
            cull_meta: Option<mtk_cull::BlockCullMeta>,
        }

        let convert_face = |f: LegacyFaceV0| BakedFace {
            direction: f.direction,
            texture: f.texture,
            uv_rot: f.uv_rot,
            uv_bounds: f.uv_bounds,
            tint_index: f.tint_index,
            cullface: f.cullface,
            vertices: f.vertices,
            uvs: f.uvs,
            normal: f.normal,
            atlas_uvs: None,
            atlas_chunk_id: None,
            atlas_texture_id: None,
        };

        if let Ok(legacy_v1) = bincode::deserialize::<HashMap<String, LegacyModelV1>>(bytes) {
            let mut models = HashMap::with_capacity(legacy_v1.len());
            for (st, lm) in legacy_v1 {
                let elements = lm
                    .elements
                    .into_iter()
                    .map(|el| BakedElement {
                        from_pos: el.from_pos,
                        to_pos: el.to_pos,
                        faces: el
                            .faces
                            .into_iter()
                            .map(|(d, f)| (d, convert_face(f)))
                            .collect(),
                    })
                    .collect();
                let faces = [
                    convert_face(lm.faces[0].clone()),
                    convert_face(lm.faces[1].clone()),
                    convert_face(lm.faces[2].clone()),
                    convert_face(lm.faces[3].clone()),
                    convert_face(lm.faces[4].clone()),
                    convert_face(lm.faces[5].clone()),
                ];
                let mut bm = BakedModel {
                    block_state: lm.block_state,
                    elements,
                    obj_faces: lm.obj_faces,
                    faces,
                    is_cube: lm.is_cube,
                    is_opaque: lm.is_opaque,
                    is_emissive: lm.is_emissive,
                    emissive_level: lm.emissive_level,
                    cull_meta: lm.cull_meta,
                    culled_faces: Default::default(),
                    unculled_faces: Default::default(),
                };
                if bm.cull_meta.is_none() {
                    bm.cull_meta = Some(bm.get_or_compute_cull_meta());
                }
                bm.rebuild_face_buckets();
                models.insert(st, bm);
            }
            return Ok(Self {
                models,
                variant_groups: HashMap::new(),
            });
        }

        if let Ok(legacy_v0) = bincode::deserialize::<HashMap<String, LegacyModelV0>>(bytes) {
            let mut models = HashMap::with_capacity(legacy_v0.len());
            for (st, lm) in legacy_v0 {
                let elements = lm
                    .elements
                    .into_iter()
                    .map(|el| BakedElement {
                        from_pos: el.from_pos,
                        to_pos: el.to_pos,
                        faces: el
                            .faces
                            .into_iter()
                            .map(|(d, f)| (d, convert_face(f)))
                            .collect(),
                    })
                    .collect();
                let faces = [
                    convert_face(lm.faces[0].clone()),
                    convert_face(lm.faces[1].clone()),
                    convert_face(lm.faces[2].clone()),
                    convert_face(lm.faces[3].clone()),
                    convert_face(lm.faces[4].clone()),
                    convert_face(lm.faces[5].clone()),
                ];
                let mut bm = BakedModel {
                    block_state: lm.block_state,
                    elements,
                    obj_faces: lm.obj_faces,
                    faces,
                    is_cube: lm.is_cube,
                    is_opaque: lm.is_opaque,
                    is_emissive: lm.is_emissive,
                    emissive_level: lm.emissive_level,
                    cull_meta: None,
                    culled_faces: Default::default(),
                    unculled_faces: Default::default(),
                };
                bm.cull_meta = Some(bm.get_or_compute_cull_meta());
                bm.rebuild_face_buckets();
                models.insert(st, bm);
            }
            return Ok(Self {
                models,
                variant_groups: HashMap::new(),
            });
        }

        let mut models: HashMap<String, BakedModel> = bincode::deserialize(bytes)?;
        for bm in models.values_mut() {
            if bm.culled_faces.iter().all(|v| v.is_empty()) && bm.unculled_faces.is_empty() && !bm.elements.is_empty() {
                bm.rebuild_face_buckets();
            }
        }
        Ok(Self {
            models,
            variant_groups: HashMap::new(),
        })
    }
}

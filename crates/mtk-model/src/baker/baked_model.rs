use std::collections::{BTreeMap, HashMap};

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
    #[serde(default)]
    pub atlas_uvs: Option<[Vec2; 4]>,
    #[serde(default)]
    pub atlas_chunk_id: Option<u16>,
    #[serde(default)]
    pub atlas_texture_id: Option<u32>,
}

impl BakedFace {
    /// Remaps this face's local UVs to absolute Atlas UV coordinates given frame 0 UV bounds.
    pub fn remap_to_atlas_bounds(
        &mut self,
        frame_0_uv_bounds: [f32; 4],
        chunk_id: u16,
        texture_id: u32,
    ) {
        let u_min = frame_0_uv_bounds[0];
        let v_min = frame_0_uv_bounds[1];
        let u_span = frame_0_uv_bounds[2] - u_min;
        let v_span = frame_0_uv_bounds[3] - v_min;
        self.atlas_uvs = Some([
            Vec2::new(u_min + self.uvs[0].x * u_span, v_min + (1.0 - self.uvs[0].y) * v_span),
            Vec2::new(u_min + self.uvs[1].x * u_span, v_min + (1.0 - self.uvs[1].y) * v_span),
            Vec2::new(u_min + self.uvs[2].x * u_span, v_min + (1.0 - self.uvs[2].y) * v_span),
            Vec2::new(u_min + self.uvs[3].x * u_span, v_min + (1.0 - self.uvs[3].y) * v_span),
        ]);
        self.atlas_chunk_id = Some(chunk_id);
        self.atlas_texture_id = Some(texture_id);
    }
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
            atlas_uvs: None,
            atlas_chunk_id: None,
            atlas_texture_id: None,
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

/// Options for configuring mesh generation and face de-overlapping from baked models.
#[derive(Debug, Clone)]
pub struct ModelMeshOptions {
    /// Whether to clip faces occluded by interior element geometry via 2D boolean difference.
    pub clip_hidden_volume: bool,
    /// Whether to cull duplicate overlapping faces with the same normal (eliminating Z-fighting).
    pub cull_duplicates: bool,
    /// Whether to cull contacting interior faces with opposite normals.
    pub cull_coplanar_opposite: bool,
    /// Numerical tolerance for geometric equality.
    pub tolerance: f32,
}

impl Default for ModelMeshOptions {
    fn default() -> Self {
        Self {
            clip_hidden_volume: true,
            cull_duplicates: true,
            cull_coplanar_opposite: true,
            tolerance: 1e-3,
        }
    }
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
    #[serde(default)]
    pub cull_meta: Option<mtk_cull::BlockCullMeta>,
    #[serde(default)]
    pub culled_faces: [Vec<BakedFace>; 6],
    #[serde(default)]
    pub unculled_faces: Vec<BakedFace>,
}

impl BakedModel {
    /// Rebuilds `culled_faces` (indexed by cull direction) and `unculled_faces` from `elements`.
    /// Automatically filters out standalone overlay decal faces (`_overlay`, `side_overlay`).
    pub fn rebuild_face_buckets(&mut self) {
        let mut culled: [Vec<BakedFace>; 6] = Default::default();
        let mut unculled = Vec::new();

        for el in &self.elements {
            for face in el.faces.values() {
                if face.texture.ends_with("_overlay")
                    || face.texture.ends_with("_OVERLAY")
                    || face.texture.contains("side_overlay")
                {
                    continue;
                }
                if let Some(cull_dir) = face.cullface {
                    culled[cull_dir.to_index()].push(face.clone());
                } else {
                    unculled.push(face.clone());
                }
            }
        }
        self.culled_faces = culled;
        self.unculled_faces = unculled;
    }

    /// Eliminates overlapping, duplicate, and interior coplanar contacting faces from the model's elements.
    ///
    /// Automatically rebuilds `culled_faces` and `unculled_faces` buckets.
    /// Returns the number of removed faces.
    pub fn deduplicate_faces(&mut self) -> usize {
        if self.elements.len() <= 1 && self.elements.first().map(|e| e.faces.len()).unwrap_or(0) <= 6 {
            return 0;
        }

        let mut face_list: Vec<(usize, Direction, [Vec3; 4], Vec3, Vec3)> = Vec::new();
        for (el_idx, el) in self.elements.iter().enumerate() {
            for (&dir, f) in &el.faces {
                let center = (f.vertices[0] + f.vertices[1] + f.vertices[2] + f.vertices[3]) * 0.25;
                face_list.push((el_idx, dir, f.vertices, f.normal, center));
            }
        }

        let mut to_remove: std::collections::HashSet<(usize, Direction)> = std::collections::HashSet::new();
        let tol = 1e-3f32;

        for i in 0..face_list.len() {
            if to_remove.contains(&(face_list[i].0, face_list[i].1)) {
                continue;
            }
            let (el_a, dir_a, verts_a, norm_a, center_a) = &face_list[i];

            for j in (i + 1)..face_list.len() {
                if to_remove.contains(&(face_list[j].0, face_list[j].1)) {
                    continue;
                }
                let (el_b, dir_b, verts_b, norm_b, center_b) = &face_list[j];

                let dot = norm_a.dot(*norm_b);
                let is_same_dir = dot > 0.99;
                let is_opp_dir = dot < -0.99;
                if !is_same_dir && !is_opp_dir {
                    continue;
                }

                // Check coplanar distance
                let plane_dist = ((center_b - center_a).dot(*norm_a)).abs();
                if plane_dist > tol {
                    continue;
                }

                // Compute 2D tangent basis (u_axis, v_axis)
                let up = if norm_a.y.abs() > 0.9 { Vec3::Z } else { Vec3::Y };
                let mut u_axis = up.cross(*norm_a);
                let u_len = u_axis.length();
                if u_len < 1e-5 {
                    u_axis = Vec3::X;
                } else {
                    u_axis /= u_len;
                }
                let v_axis = norm_a.cross(u_axis).normalize();

                // Project verts_a to 2D
                let (mut u_min_a, mut u_max_a) = (f32::INFINITY, f32::NEG_INFINITY);
                let (mut v_min_a, mut v_max_a) = (f32::INFINITY, f32::NEG_INFINITY);
                for v in verts_a {
                    let u = v.dot(u_axis);
                    let vc = v.dot(v_axis);
                    u_min_a = u_min_a.min(u);
                    u_max_a = u_max_a.max(u);
                    v_min_a = v_min_a.min(vc);
                    v_max_a = v_max_a.max(vc);
                }
                let area_a = (u_max_a - u_min_a) * (v_max_a - v_min_a);
                if area_a <= 1e-6 {
                    continue;
                }

                // Project verts_b to 2D
                let (mut u_min_b, mut u_max_b) = (f32::INFINITY, f32::NEG_INFINITY);
                let (mut v_min_b, mut v_max_b) = (f32::INFINITY, f32::NEG_INFINITY);
                for v in verts_b {
                    let u = v.dot(u_axis);
                    let vc = v.dot(v_axis);
                    u_min_b = u_min_b.min(u);
                    u_max_b = u_max_b.max(u);
                    v_min_b = v_min_b.min(vc);
                    v_max_b = v_max_b.max(vc);
                }
                let area_b = (u_max_b - u_min_b) * (v_max_b - v_min_b);
                if area_b <= 1e-6 {
                    continue;
                }

                let inter_u_min = u_min_a.max(u_min_b);
                let inter_u_max = u_max_a.min(u_max_b);
                let inter_v_min = v_min_a.max(v_min_b);
                let inter_v_max = v_max_a.min(v_max_b);

                if inter_u_max > inter_u_min + tol && inter_v_max > inter_v_min + tol {
                    let inter_area = (inter_u_max - inter_u_min) * (inter_v_max - inter_v_min);
                    let is_exact = (u_min_a - u_min_b).abs() <= tol
                        && (u_max_a - u_max_b).abs() <= tol
                        && (v_min_a - v_min_b).abs() <= tol
                        && (v_max_a - v_max_b).abs() <= tol;

                    if is_exact {
                        if is_same_dir {
                            // Exact duplicate face: remove B
                            to_remove.insert((*el_b, *dir_b));
                        } else if is_opp_dir {
                            // Back-to-back contacting faces: remove both
                            to_remove.insert((*el_a, *dir_a));
                            to_remove.insert((*el_b, *dir_b));
                            break;
                        }
                    } else if inter_area >= area_b * 0.99 - tol {
                        // Face B is completely covered/contained within Face A
                        to_remove.insert((*el_b, *dir_b));
                    } else if inter_area >= area_a * 0.99 - tol {
                        // Face A is completely covered/contained within Face B
                        to_remove.insert((*el_a, *dir_a));
                        break;
                    }
                }
            }
        }

        let culled_count = to_remove.len();
        if culled_count > 0 {
            for (el_idx, dir) in to_remove {
                if let Some(el) = self.elements.get_mut(el_idx) {
                    el.faces.remove(&dir);
                }
            }
        }
        self.rebuild_face_buckets();

        culled_count
    }

    /// Returns references to face buckets. If `culled_faces` and `unculled_faces` are empty
    /// but `elements` is non-empty (e.g. manually constructed in tests), computes them on the fly.
    pub fn get_face_buckets(&self) -> ([Vec<BakedFace>; 6], Vec<BakedFace>) {
        if self.culled_faces.iter().any(|v| !v.is_empty()) || !self.unculled_faces.is_empty() {
            return (self.culled_faces.clone(), self.unculled_faces.clone());
        }
        let mut culled: [Vec<BakedFace>; 6] = Default::default();
        let mut unculled = Vec::new();
        for el in &self.elements {
            for face in el.faces.values() {
                if face.texture.ends_with("_overlay")
                    || face.texture.ends_with("_OVERLAY")
                    || face.texture.contains("side_overlay")
                {
                    continue;
                }
                if let Some(cull_dir) = face.cullface {
                    culled[cull_dir.to_index()].push(face.clone());
                } else {
                    unculled.push(face.clone());
                }
            }
        }
        (culled, unculled)
    }

    /// Returns pre-baked culling metadata, or dynamically computes it if missing.
    pub fn get_or_compute_cull_meta(&self) -> mtk_cull::BlockCullMeta {
        if let Some(ref meta) = self.cull_meta {
            return meta.clone();
        }
        let mut quads: Vec<([Vec3; 4], Direction)> = Vec::new();
        if !self.elements.is_empty() {
            for elem in &self.elements {
                for (&dir, face) in &elem.faces {
                    quads.push((face.vertices, dir));
                }
            }
        } else if self.is_cube {
            for dir in Direction::ALL {
                quads.push((self.faces[dir.to_index()].vertices, dir));
            }
        }
        let quads_slice = if quads.is_empty() { None } else { Some(quads.as_slice()) };
        mtk_cull::compute_block_cull_meta(&self.block_state, quads_slice, Some(self.is_opaque))
    }

    /// Returns the standard face for a given direction (from 6-face summary).
    pub fn get_face(&self, dir: Direction) -> &BakedFace {
        &self.faces[dir.to_index()]
    }

    /// Remaps all elements, faces, and OBJ faces in this model to atlas coordinates using a lookup closure.
    pub fn remap_to_atlas_with<F>(&mut self, mut lookup_fn: F)
    where
        F: FnMut(&str) -> Option<([f32; 4], u16, u32)>,
    {
        for elem in &mut self.elements {
            for face in elem.faces.values_mut() {
                if let Some((bounds, chunk_id, tex_id)) = lookup_fn(&face.texture) {
                    face.remap_to_atlas_bounds(bounds, chunk_id, tex_id);
                }
            }
        }
        for bucket in &mut self.culled_faces {
            for face in bucket {
                if let Some((bounds, chunk_id, tex_id)) = lookup_fn(&face.texture) {
                    face.remap_to_atlas_bounds(bounds, chunk_id, tex_id);
                }
            }
        }
        for face in &mut self.unculled_faces {
            if let Some((bounds, chunk_id, tex_id)) = lookup_fn(&face.texture) {
                face.remap_to_atlas_bounds(bounds, chunk_id, tex_id);
            }
        }
        for face in &mut self.faces {
            if let Some((bounds, chunk_id, tex_id)) = lookup_fn(&face.texture) {
                face.remap_to_atlas_bounds(bounds, chunk_id, tex_id);
            }
        }
        for obj_face in &mut self.obj_faces {
            if let Some((bounds, _chunk_id, _tex_id)) = lookup_fn(&obj_face.texture) {
                let u_min = bounds[0];
                let v_min = bounds[1];
                let u_span = bounds[2] - u_min;
                let v_span = bounds[3] - v_min;
                for uv in &mut obj_face.uvs {
                    uv.x = u_min + uv.x * u_span;
                    uv.y = v_min + (1.0 - uv.y) * v_span;
                }
            }
        }
    }

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

/// In-memory database of baked models keyed by canonical BlockState strings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BakedModelDatabase {
    pub models: HashMap<String, BakedModel>,
}

impl BakedModelDatabase {
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
        }
    }

    pub fn insert(&mut self, state: String, model: BakedModel) {
        self.models.insert(state, model);
    }

    /// Eliminates overlapping, duplicate, and interior coplanar contacting faces
    /// across all baked models in the database, returning total faces removed.
    pub fn deduplicate_all(&mut self) -> usize {
        let mut total = 0;
        for model in self.models.values_mut() {
            total += model.deduplicate_faces();
        }
        total
    }

    /// Remaps all baked models in the database to atlas coordinates using a lookup closure.
    pub fn remap_to_atlas_with<F>(&mut self, mut lookup_fn: F)
    where
        F: FnMut(&str) -> Option<([f32; 4], u16, u32)>,
    {
        for model in self.models.values_mut() {
            model.remap_to_atlas_with(&mut lookup_fn);
        }
    }

    /// Resolves a baked model using multi-tiered smart BlockState resolution:
    /// 1. Exact string match in `self.models`.
    /// 2. Normalized match with non-geometric properties stripped (e.g. `waterlogged`, `occupied`).
    /// 3. Best compatibility subset match against variant keys for this block ID.
    /// 4. Base unparameterized block ID fallback (e.g. `minecraft:chest`).
    pub fn get(&self, state: &str) -> Option<&BakedModel> {
        // Tier 1: Fast exact match
        if let Some(model) = self.models.get(state) {
            return Some(model);
        }

        let parsed = match crate::parser::blockstate::BlockState::parse(state) {
            Ok(p) => p,
            Err(_) => return None,
        };

        let base_id = parsed.block_id();
        let canon_str = parsed.to_canonical_string();

        // Tier 1.5: Canonical match (handles whitespace and property ordering)
        if let Some(model) = self.models.get(&canon_str) {
            return Some(model);
        }

        // Tier 2: Strip known non-geometric properties that never affect block model geometry in vanilla
        const NON_GEOMETRIC_PROPS: &[&str] = &[
            "waterlogged",
            "occupied",
            "distance",
            "persistent",
            "stage",
            "unstable",
            "conditional",
            "disarmed",
        ];

        let mut has_non_geom = false;
        let mut filtered_props = parsed.properties.clone();
        for &prop in NON_GEOMETRIC_PROPS {
            if filtered_props.remove(prop).is_some() {
                has_non_geom = true;
            }
        }

        if has_non_geom {
            let canon_filtered = if filtered_props.is_empty() {
                base_id.clone()
            } else {
                let props_str: Vec<String> = filtered_props
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, v))
                    .collect();
                format!("{}[{}]", base_id, props_str.join(","))
            };

            if let Some(model) = self.models.get(&canon_filtered) {
                return Some(model);
            }
        }

        // Helper closure to find the best compatible variant for a given base_id and property map
        let find_best_variant = |target_base_id: &str, target_props: &BTreeMap<String, String>| -> Option<&BakedModel> {
            let prefix = format!("{}[", target_base_id);
            let mut best_model: Option<&BakedModel> = None;
            let mut best_score = -999999i32;

            let mut relaxed_best_model: Option<&BakedModel> = None;
            let mut relaxed_best_score = -999999i32;

            for (key, model) in &self.models {
                if key == target_base_id {
                    let score = if target_props.is_empty() { 1000 } else { 0 };
                    if score > best_score {
                        best_score = score;
                        best_model = Some(model);
                    }
                    if score > relaxed_best_score {
                        relaxed_best_score = score;
                        relaxed_best_model = Some(model);
                    }
                    continue;
                }

                if key.starts_with(&prefix) && key.ends_with(']') {
                    let cand_props_str = &key[prefix.len()..key.len() - 1];
                    let mut cand_props: HashMap<&str, &str> = HashMap::new();
                    for pair in cand_props_str.split(',') {
                        if let Some((k, v)) = pair.split_once('=') {
                            cand_props.insert(k.trim(), v.trim());
                        }
                    }

                    // Compatibility check:
                    // Any property specified in target_props MUST match candidate's value if candidate defines it
                    let mut compatible = true;
                    let mut matched_keys = 0i32;
                    let mut relaxed_score = 0i32;

                    for (k, v) in target_props {
                        if let Some(&cand_v) = cand_props.get(k.as_str()) {
                            if cand_v == v.as_str() {
                                matched_keys += 1;
                                relaxed_score += if k == "facing" || k == "axis" { 500 } else { 100 };
                            } else {
                                compatible = false;
                                if k == "facing" || k == "axis" {
                                    relaxed_score -= 300;
                                }
                            }
                        }
                    }

                    if compatible {
                        let mut score = matched_keys * 100;
                        if cand_props.len() == target_props.len() {
                            score += 1000;
                        }

                        // Score candidate properties that were NOT specified in the query:
                        // Prefer canonical vanilla default values!
                        for (cand_k, cand_v) in &cand_props {
                            if !target_props.contains_key(*cand_k) {
                                if matches!(
                                    *cand_v,
                                    "false" | "0" | "none" | "straight" | "bottom" | "lower" | "single"
                                        | "foot" | "normal" | "side" | "y" | "north"
                                ) {
                                    score += 10;
                                } else if matches!(
                                    *cand_v,
                                    "true" | "1" | "top" | "upper" | "head" | "inner" | "outer" | "double"
                                        | "x" | "z" | "south" | "east" | "west"
                                ) {
                                    score -= 10;
                                }
                            }
                        }

                        if score > best_score {
                            best_score = score;
                            best_model = Some(model);
                        }
                    }

                    if relaxed_score > relaxed_best_score {
                        relaxed_best_score = relaxed_score;
                        relaxed_best_model = Some(model);
                    }
                }
            }

            best_model.or(relaxed_best_model)
        };

        // Tier 3: Cross-category block mappings for directional variants
        // If torch has a horizontal facing (north/south/east/west), it is a wall torch
        if let Some(facing) = filtered_props.get("facing").map(|s| s.as_str()) {
            if matches!(facing, "north" | "south" | "east" | "west") {
                let wall_id = match parsed.name.as_str() {
                    "torch" => Some(format!("{}:wall_torch", parsed.namespace)),
                    "soul_torch" => Some(format!("{}:soul_wall_torch", parsed.namespace)),
                    "redstone_torch" => Some(format!("{}:redstone_wall_torch", parsed.namespace)),
                    _ => None,
                };
                if let Some(wid) = wall_id {
                    if let Some(m) = find_best_variant(&wid, &filtered_props) {
                        return Some(m);
                    }
                }
            }
        }

        // Tier 3.5: Compatibility match on base_id with filtered_props
        if let Some(m) = find_best_variant(&base_id, &filtered_props) {
            return Some(m);
        }

        // Also try stripped short name (e.g. without "minecraft:" namespace)
        if base_id != parsed.name {
            if let Some(m) = find_best_variant(&parsed.name, &filtered_props) {
                return Some(m);
            }
        }

        // Check wall_torch -> torch if facing is up or not found
        let standing_id = match parsed.name.as_str() {
            "wall_torch" => Some(format!("{}:torch", parsed.namespace)),
            "soul_wall_torch" => Some(format!("{}:soul_torch", parsed.namespace)),
            "redstone_wall_torch" => Some(format!("{}:redstone_torch", parsed.namespace)),
            _ => None,
        };
        if let Some(sid) = standing_id {
            let mut standing_props = filtered_props.clone();
            standing_props.remove("facing");
            if let Some(m) = find_best_variant(&sid, &standing_props) {
                return Some(m);
            }
        }

        // Tier 5: Base block ID fallback (e.g. "minecraft:chest")
        if let Some(m) = self.models.get(&base_id) {
            return Some(m);
        }

        // Also try stripped short name (e.g. "chest")
        self.models.get(&parsed.name)
    }

    pub fn len(&self) -> usize {
        self.models.len()
    }

    pub fn is_empty(&self) -> bool {
        self.models.is_empty()
    }

    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.models.keys()
    }

    #[cfg(feature = "std")]
    pub fn to_bincode(&self) -> Result<Vec<u8>, bincode::Error> {
        bincode::serialize(&self.models)
    }

    #[cfg(feature = "std")]
    pub fn from_bincode(bytes: &[u8]) -> Result<Self, bincode::Error> {
        if let Ok(mut models) = bincode::deserialize::<HashMap<String, BakedModel>>(bytes) {
            for bm in models.values_mut() {
                if bm.culled_faces.iter().all(|v| v.is_empty()) && bm.unculled_faces.is_empty() && !bm.elements.is_empty() {
                    bm.rebuild_face_buckets();
                }
            }
            return Ok(Self { models });
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
            return Ok(Self { models });
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
            return Ok(Self { models });
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
            return Ok(Self { models });
        }

        let mut models: HashMap<String, BakedModel> = bincode::deserialize(bytes)?;
        for bm in models.values_mut() {
            if bm.culled_faces.iter().all(|v| v.is_empty()) && bm.unculled_faces.is_empty() && !bm.elements.is_empty() {
                bm.rebuild_face_buckets();
            }
        }
        Ok(Self { models })
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
            cull_meta: None,
            culled_faces: Default::default(),
            unculled_faces: Default::default(),
        };

        let mesh = model.to_mesh(false);
        assert_eq!(mesh.vertex_count(), 4);
        assert_eq!(mesh.triangle_count(), 2);
        assert_eq!(mesh.face_count(), 1);
        assert_eq!(mesh.face_materials[0], 0);

        // Verify standard attributes
        use mtk_core::attributes::constants::*;
        assert!(mesh.has_custom_attribute(ATTR_SOURCE_TEXTURE));
        assert!(mesh.has_custom_attribute(ATTR_FACE_DIR));
        assert!(mesh.has_custom_attribute(ATTR_EMISSION));
        assert!(mesh.has_custom_attribute(ATTR_MATERIAL_PROPS));
        assert!(mesh.has_custom_attribute(ATTR_UV_TRANSFORM));

        let tex_attr = mesh.get_custom_attribute(ATTR_SOURCE_TEXTURE).unwrap();
        if let mtk_core::attributes::AttributeData::String(ref vals) = tex_attr.data {
            assert_eq!(vals[0], "minecraft:block/stone");
        } else {
            panic!("Expected String attribute data");
        }
    }

    #[test]
    fn test_smart_blockstate_lookup() {
        let mut db = BakedModelDatabase::new();

        let dummy_model = |state: &str| BakedModel {
            block_state: state.to_string(),
            elements: Vec::new(),
            obj_faces: Vec::new(),
            faces: [
                BakedFace::default(),
                BakedFace::default(),
                BakedFace::default(),
                BakedFace::default(),
                BakedFace::default(),
                BakedFace::default(),
            ],
            is_cube: false,
            is_opaque: false,
            is_emissive: false,
            emissive_level: 0.0,
            cull_meta: None,
            culled_faces: Default::default(),
            unculled_faces: Default::default(),
        };

        db.insert(
            "minecraft:oak_stairs[facing=east,half=bottom,shape=straight]".to_string(),
            dummy_model("minecraft:oak_stairs[facing=east,half=bottom,shape=straight]"),
        );
        db.insert(
            "minecraft:smooth_stone_slab[type=bottom]".to_string(),
            dummy_model("minecraft:smooth_stone_slab[type=bottom]"),
        );
        db.insert(
            "minecraft:chest".to_string(),
            dummy_model("minecraft:chest"),
        );
        db.insert(
            "minecraft:red_bed[facing=north,part=foot]".to_string(),
            dummy_model("minecraft:red_bed[facing=north,part=foot]"),
        );

        // Tier 1: Exact match
        assert!(db.get("minecraft:chest").is_some());
        assert!(db.get("minecraft:smooth_stone_slab[type=bottom]").is_some());

        // Tier 2: Stripping non-geometric properties (waterlogged, occupied)
        let stairs_query = "minecraft:oak_stairs[facing=east,half=bottom,shape=straight,waterlogged=false]";
        let slab_query = "minecraft:smooth_stone_slab[type=bottom,waterlogged=false]";
        let bed_query = "minecraft:red_bed[facing=north,occupied=false,part=foot]";

        let stairs_found = db.get(stairs_query);
        assert!(stairs_found.is_some(), "Stairs with waterlogged=false must match");
        assert_eq!(stairs_found.unwrap().block_state, "minecraft:oak_stairs[facing=east,half=bottom,shape=straight]");

        let slab_found = db.get(slab_query);
        assert!(slab_found.is_some(), "Slab with waterlogged=false must match");
        assert_eq!(slab_found.unwrap().block_state, "minecraft:smooth_stone_slab[type=bottom]");

        let bed_found = db.get(bed_query);
        assert!(bed_found.is_some(), "Bed with occupied=false must match");
        assert_eq!(bed_found.unwrap().block_state, "minecraft:red_bed[facing=north,part=foot]");

        // Tier 3/4: Fallback for chest with state properties to base chest model
        let chest_query = "minecraft:chest[facing=south,type=single,waterlogged=false]";
        let chest_found = db.get(chest_query);
        assert!(chest_found.is_some(), "Chest with properties must fallback to base chest model");
        assert_eq!(chest_found.unwrap().block_state, "minecraft:chest");

        // Relaxed fallback: flower_amount=1 falling back to closest variant
        db.insert(
            "minecraft:wildflowers[flower_amount=2]".to_string(),
            dummy_model("minecraft:wildflowers[flower_amount=2]"),
        );
        let wf_found = db.get("minecraft:wildflowers[flower_amount=1]");
        assert!(wf_found.is_some(), "Wildflowers flower_amount=1 must fallback to closest variant");
        assert_eq!(wf_found.unwrap().block_state, "minecraft:wildflowers[flower_amount=2]");
    }
}


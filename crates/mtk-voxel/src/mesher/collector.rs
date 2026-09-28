//! # Face Attributes Collector
//!
//! High-throughput collector for all 15 per-face attributes needed for Atlas materials,
//! UV tiling transforms, labPBR provenance, and dynamic Biome Tinting.

use glam::IVec3;
use mtk_core::attributes::{AttributeData, AttributeDomain, MeshAttribute};
use mtk_core::mesh::MeshData;

#[derive(Debug, Clone, Default)]
pub struct FaceAttributesCollector {
    pub source_texture_keys: Vec<String>,
    pub material_slots: Vec<i32>,
    pub atlas_chunk_ids: Vec<i32>,
    pub atlas_texture_ids: Vec<u32>,
    pub uv_tiling_transforms: Vec<[f32; 4]>,
    pub uv_transforms: Vec<[f32; 4]>,
    pub uv_rotations: Vec<f32>,
    pub uv_modes: Vec<u8>,
    pub biome_tint_data: Vec<[f32; 4]>,
    pub biome_tint_colors: Vec<[f32; 4]>,
    pub colormap_uvs: Vec<[f32; 3]>,
    pub block_xs: Vec<i32>,
    pub block_ys: Vec<i32>,
    pub block_zs: Vec<i32>,
    pub face_dirs: Vec<u8>,
}

impl FaceAttributesCollector {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            source_texture_keys: Vec::with_capacity(capacity),
            material_slots: Vec::with_capacity(capacity),
            atlas_chunk_ids: Vec::with_capacity(capacity),
            atlas_texture_ids: Vec::with_capacity(capacity),
            uv_tiling_transforms: Vec::with_capacity(capacity),
            uv_transforms: Vec::with_capacity(capacity),
            uv_rotations: Vec::with_capacity(capacity),
            uv_modes: Vec::with_capacity(capacity),
            biome_tint_data: Vec::with_capacity(capacity),
            biome_tint_colors: Vec::with_capacity(capacity),
            colormap_uvs: Vec::with_capacity(capacity),
            block_xs: Vec::with_capacity(capacity),
            block_ys: Vec::with_capacity(capacity),
            block_zs: Vec::with_capacity(capacity),
            face_dirs: Vec::with_capacity(capacity),
        }
    }

    #[inline]
    pub fn push_face(
        &mut self,
        source_texture_key: String,
        material_slot: i32,
        atlas_chunk_id: i32,
        atlas_texture_id: u32,
        uv_tiling_transform: [f32; 4],
        uv_rotation: f32,
        uv_mode: u8,
        tint_data: [f32; 4],
        tint_color: [f32; 4],
        colormap_uv: [f32; 3],
        block_pos: IVec3,
        face_dir: u8,
    ) {
        self.source_texture_keys.push(source_texture_key);
        self.material_slots.push(material_slot);
        self.atlas_chunk_ids.push(atlas_chunk_id);
        self.atlas_texture_ids.push(atlas_texture_id);
        self.uv_tiling_transforms.push(uv_tiling_transform);
        self.uv_transforms.push(uv_tiling_transform);
        self.uv_rotations.push(uv_rotation);
        self.uv_modes.push(uv_mode);
        self.biome_tint_data.push(tint_data);
        self.biome_tint_colors.push(tint_color);
        self.colormap_uvs.push(colormap_uv);
        self.block_xs.push(block_pos.x);
        self.block_ys.push(block_pos.y);
        self.block_zs.push(block_pos.z);
        self.face_dirs.push(face_dir);
    }

    pub fn attach_to_mesh(self, mesh: &mut MeshData) {
        mesh.add_custom_attribute(MeshAttribute::new(
            "mtk_source_texture_key",
            AttributeDomain::Face,
            AttributeData::String(self.source_texture_keys),
        ));
        mesh.add_custom_attribute(MeshAttribute::new(
            "mtk_material_slot",
            AttributeDomain::Face,
            AttributeData::Int32(self.material_slots),
        ));
        mesh.add_custom_attribute(MeshAttribute::new(
            "mtk_atlas_chunk_id",
            AttributeDomain::Face,
            AttributeData::Int32(self.atlas_chunk_ids),
        ));
        mesh.add_custom_attribute(MeshAttribute::new(
            "mtk_atlas_texture_id",
            AttributeDomain::Face,
            AttributeData::UInt32(self.atlas_texture_ids),
        ));
        mesh.add_custom_attribute(MeshAttribute::new(
            "mtk_uv_tiling_transform",
            AttributeDomain::Face,
            AttributeData::Float4(self.uv_tiling_transforms),
        ));
        mesh.add_custom_attribute(MeshAttribute::new(
            "mtk_uv_transform",
            AttributeDomain::Face,
            AttributeData::Float4(self.uv_transforms),
        ));
        mesh.add_custom_attribute(MeshAttribute::new(
            "mtk_uv_rotation",
            AttributeDomain::Face,
            AttributeData::Float(self.uv_rotations),
        ));
        mesh.add_custom_attribute(MeshAttribute::new(
            "mtk_uv_mode",
            AttributeDomain::Face,
            AttributeData::UInt8(self.uv_modes),
        ));
        mesh.add_custom_attribute(MeshAttribute::new(
            "mtk_biome_tint_data",
            AttributeDomain::Face,
            AttributeData::Float4(self.biome_tint_data),
        ));
        mesh.add_custom_attribute(MeshAttribute::new(
            "mtk_biome_tint_color",
            AttributeDomain::Face,
            AttributeData::Float4(self.biome_tint_colors),
        ));
        mesh.add_custom_attribute(MeshAttribute::new(
            "mtk_colormap_uv",
            AttributeDomain::Face,
            AttributeData::Float3(self.colormap_uvs),
        ));
        mesh.add_custom_attribute(MeshAttribute::new(
            "mtk_block_x",
            AttributeDomain::Face,
            AttributeData::Int32(self.block_xs),
        ));
        mesh.add_custom_attribute(MeshAttribute::new(
            "mtk_block_y",
            AttributeDomain::Face,
            AttributeData::Int32(self.block_ys),
        ));
        mesh.add_custom_attribute(MeshAttribute::new(
            "mtk_block_z",
            AttributeDomain::Face,
            AttributeData::Int32(self.block_zs),
        ));
        mesh.add_custom_attribute(MeshAttribute::new(
            "mtk_face_dir",
            AttributeDomain::Face,
            AttributeData::UInt8(self.face_dirs),
        ));
    }
}

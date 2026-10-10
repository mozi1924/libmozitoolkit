//! # Standard Face Attributes Population
//!
//! Converts slices of `FaceAttributes` into strongly-typed `MeshAttribute` layers on `MeshData`.

use super::MeshData;
use crate::attributes::constants::*;
use crate::attributes::{AttributeData, AttributeDomain, FaceAttributes, MeshAttribute};

impl MeshData {
    /// Populates all standard Face attributes from an ordered slice of `FaceAttributes`.
    pub fn populate_standard_face_attributes(&mut self, face_attrs: &[FaceAttributes]) {
        let face_count = face_attrs.len();
        if face_count == 0 {
            return;
        }

        let mut textures = Vec::with_capacity(face_count);
        let mut face_dirs = Vec::with_capacity(face_count);
        let mut emissions = Vec::with_capacity(face_count);
        let mut mat_props = Vec::with_capacity(face_count);
        let mut uv_transforms = Vec::with_capacity(face_count);
        let mut uv_rotations = Vec::with_capacity(face_count);
        let mut uv_modes = Vec::with_capacity(face_count);
        let mut anim_timings = Vec::with_capacity(face_count);
        let mut anim_sizes = Vec::with_capacity(face_count);
        let mut tint_colors = Vec::with_capacity(face_count);
        let mut tint_datas = Vec::with_capacity(face_count);
        let mut colormap_uvs = Vec::with_capacity(face_count);
        let mut block_x = Vec::with_capacity(face_count);
        let mut block_y = Vec::with_capacity(face_count);
        let mut block_z = Vec::with_capacity(face_count);
        let mut chunk_ids = Vec::with_capacity(face_count);
        let mut has_chunk_id = false;
        let mut texture_ids = Vec::with_capacity(face_count);
        let mut has_texture_id = false;

        for attr in face_attrs {
            textures.push(attr.texture_key.clone());
            face_dirs.push(attr.face_dir as i32);
            emissions.push(attr.emission);
            mat_props.push(attr.material_props);
            uv_transforms.push(attr.uv_transform);
            uv_rotations.push(attr.uv_rotation);
            uv_modes.push(attr.uv_mode);
            anim_timings.push(attr.anim_timing);
            anim_sizes.push(attr.anim_frame_size);
            tint_colors.push(attr.biome_tint_color);
            tint_datas.push(attr.biome_tint_data);
            colormap_uvs.push(attr.colormap_uv);
            block_x.push(attr.block_pos[0]);
            block_y.push(attr.block_pos[1]);
            block_z.push(attr.block_pos[2]);

            if let Some(cid) = attr.atlas_chunk_id {
                has_chunk_id = true;
                chunk_ids.push(cid as i32);
            } else {
                chunk_ids.push(0);
            }

            if let Some(tid) = attr.atlas_texture_id {
                has_texture_id = true;
                texture_ids.push(tid);
            } else {
                texture_ids.push(0);
            }
        }

        // Modern canonical attributes
        self.add_custom_attribute(MeshAttribute::new(
            ATTR_SOURCE_TEXTURE,
            AttributeDomain::Face,
            AttributeData::String(textures.clone()),
        ));
        // Backwards-compatible alias for existing shaders/scripts
        self.add_custom_attribute(MeshAttribute::new(
            ATTR_SOURCE_TEXTURE_KEY,
            AttributeDomain::Face,
            AttributeData::String(textures),
        ));
        self.add_custom_attribute(MeshAttribute::new(
            ATTR_FACE_DIR,
            AttributeDomain::Face,
            AttributeData::Int32(face_dirs),
        ));
        self.add_custom_attribute(MeshAttribute::new(
            ATTR_EMISSION,
            AttributeDomain::Face,
            AttributeData::Float(emissions),
        ));
        self.add_custom_attribute(MeshAttribute::new(
            ATTR_MATERIAL_PROPS,
            AttributeDomain::Face,
            AttributeData::Float4(mat_props),
        ));
        self.add_custom_attribute(MeshAttribute::new(
            ATTR_UV_TRANSFORM,
            AttributeDomain::Face,
            AttributeData::Float4(uv_transforms.clone()),
        ));
        // Legacy alias mtk_uv_tiling_transform
        self.add_custom_attribute(MeshAttribute::new(
            "mtk_uv_tiling_transform",
            AttributeDomain::Face,
            AttributeData::Float4(uv_transforms),
        ));
        self.add_custom_attribute(MeshAttribute::new(
            ATTR_UV_ROTATION,
            AttributeDomain::Face,
            AttributeData::Float(uv_rotations),
        ));
        self.add_custom_attribute(MeshAttribute::new(
            ATTR_UV_MODE,
            AttributeDomain::Face,
            AttributeData::UInt8(uv_modes),
        ));
        self.add_custom_attribute(MeshAttribute::new(
            ATTR_ANIM_TIMING,
            AttributeDomain::Face,
            AttributeData::Float3(anim_timings),
        ));
        self.add_custom_attribute(MeshAttribute::new(
            ATTR_ANIM_FRAME_SIZE,
            AttributeDomain::Face,
            AttributeData::Float3(anim_sizes),
        ));
        self.add_custom_attribute(MeshAttribute::new(
            ATTR_BIOME_TINT_COLOR,
            AttributeDomain::Face,
            AttributeData::Float4(tint_colors),
        ));
        self.add_custom_attribute(MeshAttribute::new(
            ATTR_BIOME_TINT_DATA,
            AttributeDomain::Face,
            AttributeData::Float4(tint_datas),
        ));
        self.add_custom_attribute(MeshAttribute::new(
            ATTR_COLORMAP_UV,
            AttributeDomain::Face,
            AttributeData::Float3(colormap_uvs),
        ));
        self.add_custom_attribute(MeshAttribute::new(
            ATTR_BLOCK_X,
            AttributeDomain::Face,
            AttributeData::Int32(block_x),
        ));
        self.add_custom_attribute(MeshAttribute::new(
            ATTR_BLOCK_Y,
            AttributeDomain::Face,
            AttributeData::Int32(block_y),
        ));
        self.add_custom_attribute(MeshAttribute::new(
            ATTR_BLOCK_Z,
            AttributeDomain::Face,
            AttributeData::Int32(block_z),
        ));

        if has_chunk_id {
            self.add_custom_attribute(MeshAttribute::new(
                ATTR_ATLAS_CHUNK_ID,
                AttributeDomain::Face,
                AttributeData::Int32(chunk_ids),
            ));
        }

        if has_texture_id {
            self.add_custom_attribute(MeshAttribute::new(
                ATTR_ATLAS_TEXTURE_ID,
                AttributeDomain::Face,
                AttributeData::UInt32(texture_ids),
            ));
        }
    }
}

//! # Face Attributes Collector
//!
//! High-throughput collector for canonical per-face attributes needed for Atlas
//! materials, UV tiling transforms, and dynamic Biome Tinting.

use glam::IVec3;
use mtk_core::attributes::constants::*;
use mtk_core::attributes::{AttributeData, AttributeDomain, MeshAttribute};
use mtk_core::mesh::MeshData;

#[derive(Debug, Clone, Default)]
pub struct FaceAttributesCollector {
    pub source_texture_keys: Vec<String>,
    pub atlas_chunk_ids: Vec<i32>,
    pub uv_transforms: Vec<[f32; 4]>,
    pub uv_rotations: Vec<f32>,
    pub biome_tint_data: Vec<[f32; 4]>,
    pub biome_tint_colors: Vec<[f32; 4]>,
    pub block_xs: Vec<i32>,
    pub block_ys: Vec<i32>,
    pub block_zs: Vec<i32>,
    pub anim_timings: Vec<[f32; 3]>,
    pub anim_frame_sizes: Vec<[f32; 3]>,
}

impl FaceAttributesCollector {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            source_texture_keys: Vec::with_capacity(capacity),
            atlas_chunk_ids: Vec::with_capacity(capacity),
            uv_transforms: Vec::with_capacity(capacity),
            uv_rotations: Vec::with_capacity(capacity),
            biome_tint_data: Vec::with_capacity(capacity),
            biome_tint_colors: Vec::with_capacity(capacity),
            block_xs: Vec::with_capacity(capacity),
            block_ys: Vec::with_capacity(capacity),
            block_zs: Vec::with_capacity(capacity),
            anim_timings: Vec::with_capacity(capacity),
            anim_frame_sizes: Vec::with_capacity(capacity),
        }
    }

    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub fn push_face(
        &mut self,
        source_texture_key: String,
        atlas_chunk_id: i32,
        uv_transform: [f32; 4],
        uv_rotation: f32,
        tint_data: [f32; 4],
        tint_color: [f32; 4],
        block_pos: IVec3,
        anim_timing: [f32; 3],
        anim_frame_size: [f32; 3],
    ) {
        self.source_texture_keys.push(source_texture_key);
        self.atlas_chunk_ids.push(atlas_chunk_id);
        self.uv_transforms.push(uv_transform);
        self.uv_rotations.push(uv_rotation);
        self.biome_tint_data.push(tint_data);
        self.biome_tint_colors.push(tint_color);
        self.block_xs.push(block_pos.x);
        self.block_ys.push(block_pos.y);
        self.block_zs.push(block_pos.z);
        self.anim_timings.push(anim_timing);
        self.anim_frame_sizes.push(anim_frame_size);
    }

    pub fn attach_to_mesh(self, mesh: &mut MeshData) {
        mesh.add_custom_attribute(MeshAttribute::new(
            ATTR_SOURCE_TEXTURE_KEY,
            AttributeDomain::Face,
            AttributeData::String(self.source_texture_keys),
        ));
        mesh.add_custom_attribute(MeshAttribute::new(
            ATTR_ATLAS_CHUNK_ID,
            AttributeDomain::Face,
            AttributeData::Int32(self.atlas_chunk_ids),
        ));
        mesh.add_custom_attribute(MeshAttribute::new(
            ATTR_UV_TRANSFORM,
            AttributeDomain::Face,
            AttributeData::Float4(self.uv_transforms),
        ));
        mesh.add_custom_attribute(MeshAttribute::new(
            ATTR_UV_ROTATION,
            AttributeDomain::Face,
            AttributeData::Float(self.uv_rotations),
        ));
        mesh.add_custom_attribute(MeshAttribute::new(
            ATTR_BIOME_TINT_DATA,
            AttributeDomain::Face,
            AttributeData::Float4(self.biome_tint_data),
        ));
        mesh.add_custom_attribute(MeshAttribute::new(
            ATTR_BIOME_TINT_COLOR,
            AttributeDomain::Face,
            AttributeData::Float4(self.biome_tint_colors),
        ));
        mesh.add_custom_attribute(MeshAttribute::new(
            ATTR_BLOCK_X,
            AttributeDomain::Face,
            AttributeData::Int32(self.block_xs),
        ));
        mesh.add_custom_attribute(MeshAttribute::new(
            ATTR_BLOCK_Y,
            AttributeDomain::Face,
            AttributeData::Int32(self.block_ys),
        ));
        mesh.add_custom_attribute(MeshAttribute::new(
            ATTR_BLOCK_Z,
            AttributeDomain::Face,
            AttributeData::Int32(self.block_zs),
        ));
        // Keep the attribute set homogeneous across every Section. Static
        // sections carry default timing values; the merged world mesh prunes
        // these attributes once if nothing in the world is animated.
        mesh.add_custom_attribute(MeshAttribute::new(
            ATTR_ANIM_TIMING,
            AttributeDomain::Face,
            AttributeData::Float3(self.anim_timings),
        ));
        mesh.add_custom_attribute(MeshAttribute::new(
            ATTR_ANIM_FRAME_SIZE,
            AttributeDomain::Face,
            AttributeData::Float3(self.anim_frame_sizes),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mtk_core::attributes::constants::{
        ATTR_ANIM_FRAME_SIZE, ATTR_ANIM_TIMING, ATTR_BLOCK_X, ATTR_BLOCK_Y, ATTR_BLOCK_Z,
    };

    fn collector_with(anim_timing: [f32; 3]) -> FaceAttributesCollector {
        let mut collector = FaceAttributesCollector::with_capacity(1);
        collector.push_face(
            "minecraft:block/stone".to_string(),
            0,
            [1.0, 1.0, 0.0, 0.0],
            0.0,
            [1.0, 0.0, 0.0, 0.0],
            [1.0, 1.0, 1.0, 1.0],
            glam::IVec3::new(1, 2, 3),
            anim_timing,
            [16.0, 16.0, 0.0],
        );
        collector
    }

    #[test]
    fn static_faces_keep_animation_attributes_for_merge() {
        let mut mesh = MeshData::new();
        collector_with([1.0, 1.0, 0.0]).attach_to_mesh(&mut mesh);

        assert_eq!(
            mesh.get_custom_attribute(ATTR_ANIM_TIMING)
                .expect("static section must carry default timing")
                .len(),
            1
        );
        assert!(mesh.get_custom_attribute(ATTR_ANIM_FRAME_SIZE).is_some());
        assert!(mesh.get_custom_attribute(ATTR_BLOCK_X).is_some());
        assert!(mesh.get_custom_attribute(ATTR_BLOCK_Y).is_some());
        assert!(mesh.get_custom_attribute(ATTR_BLOCK_Z).is_some());
    }

    #[test]
    fn static_faces_keep_animation_attributes() {
        let mut mesh = MeshData::new();
        collector_with([4.0, 2.0, 1.0]).attach_to_mesh(&mut mesh);

        assert!(mesh.get_custom_attribute(ATTR_ANIM_TIMING).is_some());
        assert!(mesh.get_custom_attribute(ATTR_ANIM_FRAME_SIZE).is_some());
    }
}

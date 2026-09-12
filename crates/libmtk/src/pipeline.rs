//! # High-Level Geometry & Mesh Pipeline
//!
//! Provides an end-to-end, host-agnostic, data-in/data-out mesh processing engine.
//! In a single invocation, performs:
//! 1. External alias matching & material string resolution
//! 2. Multi-threaded Atlas / Standalone UV remapping
//! 3. Secondary normalized [0, 1] UV generation for PBR
//! 4. Structured result mesh & material summary generation

use std::collections::HashMap;
use mtk_core::mesh::MeshData;
use mtk_material::{remap_mesh_multi_uvs_parallel, GridAtlasSpec, MeshMultiUvRemapResult};
use mtk_texture::AtlasAddressMap;

/// Configuration options for the unified mesh processing pipeline.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MeshPipelineConfig {
    /// Optional external material alias map (raw_name -> candidate list).
    pub custom_aliases: Option<HashMap<String, Vec<String>>>,
    /// Whether to generate secondary [0, 1] UV coordinates for PBR shader channels.
    pub generate_secondary_uv: bool,
    /// Optional grid atlas specification for UV decoding.
    pub grid_atlas_spec: Option<GridAtlasSpec>,
}

/// Information describing a resolved material slot in the output mesh.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedMaterialInfo {
    /// Original raw material slot name before resolution.
    pub raw_name: String,
    /// Canonical Minecraft resource location (e.g. `minecraft:block/stone`).
    pub canonical_name: Option<String>,
    /// Chunk index in the baked atlas (-1 if standalone or unmapped).
    pub atlas_chunk_id: i32,
    /// Whether this material maps to an animated sprite strip.
    pub is_animated: bool,
}

/// Statistics returned after executing the mesh pipeline.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MeshProcessStats {
    /// Total input vertex count.
    pub input_vertices: usize,
    /// Total input polygon face count.
    pub input_faces: usize,
    /// Total output vertex count.
    pub output_vertices: usize,
    /// Total output polygon face count.
    pub output_faces: usize,
    /// Number of material slots successfully resolved to the atlas.
    pub resolved_slots: usize,
    /// Number of material slots that fell back to generic / unmapped.
    pub unmapped_slots: usize,
}

/// Result returned from the unified mesh processing pipeline.
#[derive(Debug, Clone)]
pub struct ProcessMeshOutput {
    /// Fully transformed, UV-remapped mesh data ready for immediate GPU or DCC injection.
    pub mesh: MeshData,
    /// Resolved material metadata indexed by material slot ID.
    pub materials: Vec<ResolvedMaterialInfo>,
    /// Processing performance and geometry metrics.
    pub stats: MeshProcessStats,
}

/// Unified entrypoint for processing any imported mesh in a single pass.
///
/// # Arguments
/// - `mesh`: The input mesh buffer.
/// - `material_names`: Slice of raw material names matching `mesh.face_materials` IDs.
/// - `address_map`: Optional baked AtlasAddressMap for UV remapping.
/// - `config`: Pipeline configuration settings.
pub fn process_mesh(
    mesh: &MeshData,
    material_names: &[String],
    address_map: Option<&AtlasAddressMap>,
    config: &MeshPipelineConfig,
) -> ProcessMeshOutput {
    let input_faces = mesh.face_count();
    let input_vertices = mesh.vertex_count();

    if input_faces == 0 || input_vertices == 0 {
        return ProcessMeshOutput {
            mesh: mesh.clone(),
            materials: Vec::new(),
            stats: MeshProcessStats::default(),
        };
    }

    let mut output_mesh = mesh.clone();
    let mut resolved_materials = Vec::with_capacity(material_names.len());
    let mut resolved_count = 0;
    let mut unmapped_count = 0;

    // 1. Resolve material names
    if let Some(addr_map) = address_map {
        let aliases_ref = config.custom_aliases.as_ref();
        for raw_name in material_names {
            let resolved = mtk_material::MaterialResolver::resolve(raw_name, aliases_ref, addr_map);
            if let Some((res_loc, sprite_loc)) = resolved {
                resolved_materials.push(ResolvedMaterialInfo {
                    raw_name: raw_name.clone(),
                    canonical_name: Some(res_loc.to_string()),
                    atlas_chunk_id: sprite_loc.chunk_id as i32,
                    is_animated: sprite_loc.is_animated,
                });
                resolved_count += 1;
            } else {
                resolved_materials.push(ResolvedMaterialInfo {
                    raw_name: raw_name.clone(),
                    canonical_name: None,
                    atlas_chunk_id: -1,
                    is_animated: false,
                });
                unmapped_count += 1;
            }
        }

        // Build per-face loop ranges based on vertex/face proportions
        let face_count = output_mesh.face_materials.len();
        let total_uvs = output_mesh.uvs.len();
        let mut face_loop_ranges: Vec<(u32, u32)> = Vec::with_capacity(face_count);
        let mut per_face_mat_names: Vec<String> = Vec::with_capacity(face_count);

        let loops_per_face = if face_count > 0 && total_uvs % face_count == 0 {
            (total_uvs / face_count) as u32
        } else {
            4 // default quad assumption
        };

        for (i, &mat_id) in output_mesh.face_materials.iter().enumerate() {
            let start = (i as u32) * loops_per_face;
            face_loop_ranges.push((start, loops_per_face));
            let name = material_names
                .get(mat_id as usize)
                .cloned()
                .unwrap_or_default();
            per_face_mat_names.push(name);
        }

        // 2. Parallel UV remapping & Secondary UV generation
        let remap_result: MeshMultiUvRemapResult = remap_mesh_multi_uvs_parallel(
            &output_mesh.uvs,
            &per_face_mat_names,
            &face_loop_ranges,
            addr_map,
            aliases_ref,
            config.grid_atlas_spec.as_ref(),
        );

        output_mesh.uvs = remap_result.atlas_uvs;
        if config.generate_secondary_uv {
            output_mesh.secondary_uvs = Some(remap_result.local_uvs);
        }

        use mtk_core::attributes::constants::*;
        use mtk_core::attributes::{AttributeData, AttributeDomain, MeshAttribute};

        let chunk_ids: Vec<i32> = remap_result.face_chunk_ids.iter().map(|&c| c as i32).collect();
        output_mesh.add_custom_attribute(MeshAttribute::new(
            ATTR_ATLAS_CHUNK_ID,
            AttributeDomain::Face,
            AttributeData::Int32(chunk_ids),
        ));
        output_mesh.add_custom_attribute(MeshAttribute::new(
            ATTR_ATLAS_TEXTURE_ID,
            AttributeDomain::Face,
            AttributeData::UInt32(remap_result.face_texture_ids),
        ));
        output_mesh.add_custom_attribute(MeshAttribute::new(
            ATTR_UV_TRANSFORM,
            AttributeDomain::Face,
            AttributeData::Float4(remap_result.face_uv_transforms.clone()),
        ));
        output_mesh.add_custom_attribute(MeshAttribute::new(
            "mtk_uv_tiling_transform",
            AttributeDomain::Face,
            AttributeData::Float4(remap_result.face_uv_transforms),
        ));
        output_mesh.add_custom_attribute(MeshAttribute::new(
            ATTR_UV_MODE,
            AttributeDomain::Face,
            AttributeData::UInt8(remap_result.face_uv_modes),
        ));

        // Inject resolved canonical texture keys if available
        let mut source_textures: Vec<String> = Vec::with_capacity(output_mesh.face_materials.len());
        for &mat_id in &output_mesh.face_materials {
            if let Some(mat_info) = resolved_materials.get(mat_id as usize) {
                if let Some(ref canon) = mat_info.canonical_name {
                    source_textures.push(canon.clone());
                    continue;
                }
            }
            source_textures.push(String::new());
        }
        if source_textures.iter().any(|s| !s.is_empty()) {
            output_mesh.add_custom_attribute(MeshAttribute::new(
                ATTR_SOURCE_TEXTURE,
                AttributeDomain::Face,
                AttributeData::String(source_textures.clone()),
            ));
            output_mesh.add_custom_attribute(MeshAttribute::new(
                ATTR_SOURCE_TEXTURE_KEY,
                AttributeDomain::Face,
                AttributeData::String(source_textures),
            ));
        }
    } else {
        // No atlas provided - keep original materials
        for raw_name in material_names {
            resolved_materials.push(ResolvedMaterialInfo {
                raw_name: raw_name.clone(),
                canonical_name: None,
                atlas_chunk_id: -1,
                is_animated: false,
            });
            unmapped_count += 1;
        }
    }

    let output_faces = output_mesh.face_count();
    let output_vertices = output_mesh.vertex_count();

    ProcessMeshOutput {
        mesh: output_mesh,
        materials: resolved_materials,
        stats: MeshProcessStats {
            input_vertices,
            input_faces,
            output_vertices,
            output_faces,
            resolved_slots: resolved_count,
            unmapped_slots: unmapped_count,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mtk_core::direction::Direction;
    use mtk_core::geometry::Quad;
    use mtk_texture::AtlasAddressMap;

    #[test]
    fn test_process_mesh_pipeline_end_to_end() {
        let mut mesh = MeshData::new();
        let q = Quad::unit_cube_face(Direction::North);
        mesh.append_quad(&q, &Default::default());

        let mapping_json = r#"{
            "chunks": [],
            "sprites": {
                "minecraft:block/stone": {
                    "chunk_id": 0,
                    "category": "blocks",
                    "is_animated": false,
                    "sprite_kind": "static_atlas",
                    "texture_id": 1,
                    "uv_bounds": [0.0, 0.0, 0.25, 0.25],
                    "frame_0_uv_bounds": [0.0, 0.0, 0.25, 0.25],
                    "local_uv_bounds": [0.0, 0.0, 1.0, 1.0],
                    "pixel_rect": [0, 0, 16, 16],
                    "frame_size": [16, 16],
                    "frame_count": 1,
                    "has_normal": false,
                    "has_specular": false
                }
            }
        }"#;
        let addr_map = AtlasAddressMap::from_json(mapping_json).unwrap();

        let materials = vec!["Tile_Stone".to_string()];
        let mut custom_aliases = HashMap::new();
        custom_aliases.insert("tile_stone".to_string(), vec!["block/stone".to_string()]);

        let cfg = MeshPipelineConfig {
            generate_secondary_uv: true,
            grid_atlas_spec: None,
            custom_aliases: Some(custom_aliases),
        };

        let out = process_mesh(&mesh, &materials, Some(&addr_map), &cfg);

        assert_eq!(out.stats.input_faces, 1);
        assert_eq!(out.stats.output_faces, 1);
        assert_eq!(out.stats.resolved_slots, 1);
        assert_eq!(out.materials[0].atlas_chunk_id, 0);
        assert_eq!(out.materials[0].canonical_name.as_deref(), Some("minecraft:block/stone"));
        assert!(out.mesh.secondary_uvs.is_some());
    }
}

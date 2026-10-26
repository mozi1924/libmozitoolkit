//! Conversion between `VoxelPointCloud` and `mtk_core::mesh::MeshData`.

use mtk_core::attributes::{AttributeData, AttributeDomain, MeshAttribute};
use mtk_core::mesh::MeshData;

use crate::types::VoxelError;

use super::{validate_entity_indices, validate_palette_indices, VoxelPointCloud};

impl VoxelPointCloud {
    /// Converts this `VoxelPointCloud` into a pure-point `MeshData` with standard
    /// attributes attached on the `Point` domain (without edges or polygons).
    pub fn to_mesh_data(&self) -> MeshData {
        let count = self.len();
        let mut mesh = MeshData::with_capacity(count, 0, 0);

        for i in 0..count {
            let p_off = i * 3;
            mesh.positions.push([
                self.positions[p_off],
                self.positions[p_off + 1],
                self.positions[p_off + 2],
            ]);
        }

        mesh.add_custom_attribute(MeshAttribute::new(
            "mtk_block_x",
            AttributeDomain::Point,
            AttributeData::Int32(self.block_x.clone()),
        ));
        mesh.add_custom_attribute(MeshAttribute::new(
            "mtk_block_y",
            AttributeDomain::Point,
            AttributeData::Int32(self.block_y.clone()),
        ));
        mesh.add_custom_attribute(MeshAttribute::new(
            "mtk_block_z",
            AttributeDomain::Point,
            AttributeData::Int32(self.block_z.clone()),
        ));
        mesh.add_custom_attribute(MeshAttribute::new(
            "mtk_state_idx",
            AttributeDomain::Point,
            AttributeData::Int32(self.state_indices.clone()),
        ));
        mesh.add_custom_attribute(MeshAttribute::new(
            "mtk_palette",
            AttributeDomain::Mesh,
            AttributeData::String(self.state_palette.clone()),
        ));
        mesh.add_custom_attribute(MeshAttribute::new(
            "mtk_biome_idx",
            AttributeDomain::Point,
            AttributeData::Int32(self.biome_indices.clone()),
        ));
        mesh.add_custom_attribute(MeshAttribute::new(
            "mtk_biome_palette",
            AttributeDomain::Mesh,
            AttributeData::String(self.biome_palette.clone()),
        ));

        if !self.entity_palette.is_empty() && self.entity_indices.len() == count {
            mesh.add_custom_attribute(MeshAttribute::new(
                "mtk_entity_idx",
                AttributeDomain::Point,
                AttributeData::Int32(self.entity_indices.clone()),
            ));
            mesh.add_custom_attribute(MeshAttribute::new(
                "mtk_entity_palette",
                AttributeDomain::Mesh,
                AttributeData::String(self.entity_palette.clone()),
            ));
        }

        mesh.add_custom_attribute(MeshAttribute::new(
            "mtk_light",
            AttributeDomain::Point,
            AttributeData::UInt8(self.light_levels.clone()),
        ));

        if let Some(b) = self.bounds {
            mesh.add_custom_attribute(MeshAttribute::new(
                "mtk_bounds",
                AttributeDomain::Mesh,
                AttributeData::Int32(b.to_vec()),
            ));
        }

        mesh
    }

    /// Reconstructs a `VoxelPointCloud` from a `MeshData` containing Point-domain attributes.
    pub fn from_mesh_data(mesh: &MeshData) -> Result<Self, VoxelError> {
        let count = mesh.positions.len();
        if count == 0 {
            return Ok(Self::new());
        }

        let mut positions = Vec::with_capacity(count * 3);
        for p in &mesh.positions {
            positions.extend_from_slice(p);
        }

        let block_x = mesh
            .get_custom_attribute("mtk_block_x")
            .and_then(|a| match &a.data {
                AttributeData::Int32(v) if v.len() == count => Some(v.clone()),
                _ => None,
            })
            .ok_or_else(|| {
                VoxelError::InvalidPointCloud(
                    "Missing or malformed mtk_block_x attribute".to_string(),
                )
            })?;

        let block_y = mesh
            .get_custom_attribute("mtk_block_y")
            .and_then(|a| match &a.data {
                AttributeData::Int32(v) if v.len() == count => Some(v.clone()),
                _ => None,
            })
            .ok_or_else(|| {
                VoxelError::InvalidPointCloud(
                    "Missing or malformed mtk_block_y attribute".to_string(),
                )
            })?;

        let block_z = mesh
            .get_custom_attribute("mtk_block_z")
            .and_then(|a| match &a.data {
                AttributeData::Int32(v) if v.len() == count => Some(v.clone()),
                _ => None,
            })
            .ok_or_else(|| {
                VoxelError::InvalidPointCloud(
                    "Missing or malformed mtk_block_z attribute".to_string(),
                )
            })?;

        let state_indices = mesh
            .get_custom_attribute("mtk_state_idx")
            .and_then(|a| match &a.data {
                AttributeData::Int32(v) if v.len() == count => Some(v.clone()),
                _ => None,
            })
            .ok_or_else(|| {
                VoxelError::InvalidPointCloud(
                    "Missing or malformed mtk_state_idx attribute".to_string(),
                )
            })?;

        let state_palette = mesh
            .get_custom_attribute("mtk_palette")
            .and_then(|a| match &a.data {
                AttributeData::String(v) if !v.is_empty() => Some(v.clone()),
                _ => None,
            })
            .ok_or_else(|| {
                VoxelError::InvalidPointCloud(
                    "Missing or malformed mtk_palette attribute".to_string(),
                )
            })?;
        validate_palette_indices("state", &state_indices, &state_palette)?;

        let biome_indices = mesh
            .get_custom_attribute("mtk_biome_idx")
            .and_then(|a| match &a.data {
                AttributeData::Int32(v) if v.len() == count => Some(v.clone()),
                _ => None,
            })
            .unwrap_or_else(|| vec![0; count]);
        let biome_palette = mesh
            .get_custom_attribute("mtk_biome_palette")
            .and_then(|a| match &a.data {
                AttributeData::String(v) if !v.is_empty() => Some(v.clone()),
                _ => None,
            })
            .unwrap_or_else(|| vec!["minecraft:plains".to_string()]);
        validate_palette_indices("biome", &biome_indices, &biome_palette)?;

        let (entity_palette, entity_indices) =
            if let Some(p_attr) = mesh.get_custom_attribute("mtk_entity_palette") {
                if let AttributeData::String(p) = &p_attr.data {
                    let indices = mesh
                        .get_custom_attribute("mtk_entity_idx")
                        .and_then(|a| match &a.data {
                            AttributeData::Int32(v) if v.len() == count => Some(v.clone()),
                            _ => None,
                        })
                        .unwrap_or_else(|| vec![-1; count]);
                    validate_entity_indices(&indices, p)?;
                    (p.clone(), indices)
                } else {
                    (Vec::new(), Vec::new())
                }
            } else {
                (Vec::new(), Vec::new())
            };

        let light_levels = mesh
            .get_custom_attribute("mtk_light")
            .and_then(|a| match &a.data {
                AttributeData::UInt8(v) if v.len() == count => Some(v.clone()),
                _ => None,
            })
            .unwrap_or_else(|| vec![0u8; count]);

        let bounds = mesh
            .get_custom_attribute("mtk_bounds")
            .and_then(|a| match &a.data {
                AttributeData::Int32(v) if v.len() == 6 => {
                    Some([v[0], v[1], v[2], v[3], v[4], v[5]])
                }
                _ => None,
            });

        Ok(Self {
            positions,
            block_x,
            block_y,
            block_z,
            state_palette,
            state_indices,
            biome_palette,
            biome_indices,
            entity_palette,
            entity_indices,
            light_levels,
            bounds,
        })
    }
}

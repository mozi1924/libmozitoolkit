//! Quad emission helpers shared by the mesh-level pixel splitter.

use crate::mesh::MeshData;

/// Accumulates output quads (indices, loop UVs, material, tint) before emission.
#[derive(Default)]
pub(crate) struct QuadAccumulator {
    pub(crate) indices: Vec<[u32; 4]>,
    pub(crate) uvs: Vec<[[f32; 2]; 4]>,
    pub(crate) materials: Vec<u16>,
    pub(crate) tints: Vec<i16>,
}

impl QuadAccumulator {
    pub(crate) fn push(&mut self, indices: [u32; 4], uvs: [[f32; 2]; 4], material: u16, tint: i16) {
        self.indices.push(indices);
        self.uvs.push(uvs);
        self.materials.push(material);
        self.tints.push(tint);
    }
}

/// Emits an existing quad unchanged, copying its vertices and interpolated attributes.
pub(crate) fn emit_quad_copy(
    out: &mut MeshData,
    quads: &mut QuadAccumulator,
    positions: [[f32; 3]; 4],
    normals: [[f32; 3]; 4],
    secondary: Option<[[f32; 2]; 4]>,
    colors: Option<[[f32; 4]; 4]>,
    uvs: [[f32; 2]; 4],
    material: u16,
    tint: i16,
) {
    let base = out.positions.len() as u32;
    out.positions.extend_from_slice(&positions);
    out.normals.extend_from_slice(&normals);
    if let Some(sec) = secondary {
        out.secondary_uvs.as_mut().unwrap().extend_from_slice(&sec);
    }
    if let Some(col) = colors {
        out.colors.as_mut().unwrap().extend_from_slice(&col);
    }
    quads.push([base, base + 1, base + 2, base + 3], uvs, material, tint);
}

//! # Mesh Vertex Spatial Welding
//!
//! Provides spatial vertex deduplication and index remapping for general 3D meshes.

use super::MeshData;

impl MeshData {
    /// Welds co-located vertices within `tolerance` distance into shared topology,
    /// remapping indices while preserving per-corner loop UVs and colors.
    pub fn weld_spatial_vertices(&mut self, tolerance: f32) {
        if self.positions.is_empty() || tolerance <= 0.0 {
            return;
        }

        let inv_dist = 1.0 / tolerance;
        let mut coord_map = rustc_hash::FxHashMap::<[i32; 3], u32>::with_capacity_and_hasher(
            self.positions.len(),
            Default::default(),
        );
        let mut remap: Vec<u32> = Vec::with_capacity(self.positions.len());
        let mut new_positions: Vec<[f32; 3]> = Vec::with_capacity(self.positions.len());
        let mut new_normals: Vec<[f32; 3]> = Vec::with_capacity(self.normals.len());

        for (i, &p) in self.positions.iter().enumerate() {
            let key = [
                (p[0] * inv_dist).round() as i32,
                (p[1] * inv_dist).round() as i32,
                (p[2] * inv_dist).round() as i32,
            ];

            if let Some(&existing_idx) = coord_map.get(&key) {
                remap.push(existing_idx);
            } else {
                let new_idx = new_positions.len() as u32;
                coord_map.insert(key, new_idx);
                remap.push(new_idx);
                new_positions.push(p);
                if let Some(&norm) = self.normals.get(i) {
                    new_normals.push(norm);
                }
            }
        }

        // Fast-path: If all vertices were already unique, no indices need modification
        if new_positions.len() == self.positions.len() {
            return;
        }

        // Remap triangle/polygon indices
        #[cfg(feature = "parallel")]
        {
            use rayon::prelude::*;
            self.indices.par_iter_mut().for_each(|idx| {
                if let Some(&new_i) = remap.get(*idx as usize) {
                    *idx = new_i;
                }
            });

            if let Some(ref mut quads) = self.quad_indices {
                quads.par_iter_mut().for_each(|idx| {
                    if let Some(&new_i) = remap.get(*idx as usize) {
                        *idx = new_i;
                    }
                });
            }
        }

        #[cfg(not(feature = "parallel"))]
        {
            for idx in &mut self.indices {
                if let Some(&new_i) = remap.get(*idx as usize) {
                    *idx = new_i;
                }
            }

            if let Some(ref mut quads) = self.quad_indices {
                for idx in quads {
                    if let Some(&new_i) = remap.get(*idx as usize) {
                        *idx = new_i;
                    }
                }
            }
        }

        self.positions = new_positions;
        if !self.normals.is_empty() {
            self.normals = new_normals;
        }
    }
}

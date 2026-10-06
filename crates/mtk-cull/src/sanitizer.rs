//! # Mesh Sanitizer
//!
//! Unified geometry sanitization tools for 3D meshes and quad face collections:
//! - Eliminates duplicate, redundant, and back-to-back contacting coplanar faces
//! - Clips quad faces against adjacent solid bounding boxes (difference clipping)
//! - Sanitizes arbitrary `MeshData` buffers

use alloc::vec::Vec;
use glam::{Vec2, Vec3};
use mtk_core::direction::Direction;
use mtk_core::mesh::MeshData;
use std::collections::HashSet;

use crate::geometry::coplanar::{check_coplanar_overlap, CoplanarOverlap, FaceAlignment};
use crate::geometry::volume_cull::{clip_face_excluding_hidden_volume, ClippedQuadPiece};
use crate::mesh_cull::{cull_mesh_faces, MeshCullConfig, MeshCullResult};

/// Unified mesh sanitizer for geometric mesh cleanup and deduplication.
pub struct MeshSanitizer;

impl MeshSanitizer {
    /// Detects and eliminates overlapping, duplicate, and interior coplanar contacting faces
    /// from an indexed collection of quads.
    ///
    /// Each quad is identified by an ID `T`, 4 vertex positions, and a normal vector.
    /// Returns the set of quad identifiers that should be removed.
    pub fn deduplicate_quads<T: Copy + Eq + core::hash::Hash>(
        quads: &[(T, [Vec3; 4], Vec3)],
        tolerance: f32,
    ) -> HashSet<T> {
        let mut to_remove = HashSet::new();

        for i in 0..quads.len() {
            if to_remove.contains(&quads[i].0) {
                continue;
            }
            let (id_a, verts_a, norm_a) = &quads[i];

            for j in (i + 1)..quads.len() {
                if to_remove.contains(&quads[j].0) {
                    continue;
                }
                let (id_b, verts_b, norm_b) = &quads[j];

                if let Some(rel) =
                    check_coplanar_overlap(verts_a, *norm_a, verts_b, *norm_b, tolerance)
                {
                    match rel.overlap {
                        CoplanarOverlap::Exact => {
                            if rel.alignment == FaceAlignment::SameDirection {
                                // Exact duplicate face: remove B
                                to_remove.insert(*id_b);
                            } else {
                                // Back-to-back contacting faces: remove both
                                to_remove.insert(*id_a);
                                to_remove.insert(*id_b);
                                break;
                            }
                        }
                        CoplanarOverlap::ContainedInA => {
                            // Face B is completely covered/contained within Face A
                            to_remove.insert(*id_b);
                        }
                        CoplanarOverlap::ContainedInB => {
                            // Face A is completely covered/contained within Face B
                            to_remove.insert(*id_a);
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }

        to_remove
    }

    /// Clips an axis-aligned quad face excluding regions embedded inside neighbouring volumes.
    pub fn clip_quad_excluding_hidden_volume(
        vertices: &[Vec3; 4],
        uvs: &[Vec2; 4],
        direction: Direction,
        neighbour_bounds: &[([f32; 3], [f32; 3])],
    ) -> Vec<ClippedQuadPiece> {
        clip_face_excluding_hidden_volume(vertices, uvs, direction, neighbour_bounds)
    }

    /// Sanitizes an arbitrary `MeshData` buffer by eliminating coplanar and duplicate faces.
    pub fn sanitize_mesh(mesh: &MeshData, config: &MeshCullConfig) -> MeshCullResult {
        cull_mesh_faces(mesh, config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitizer_deduplicate_quads() {
        let q1 = (
            1,
            [
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(1.0, 1.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            ],
            Vec3::new(0.0, 0.0, 1.0),
        );
        // Duplicate facing same direction
        let q2 = (
            2,
            [
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(1.0, 1.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            ],
            Vec3::new(0.0, 0.0, 1.0),
        );

        let quads = vec![q1, q2];
        let removed = MeshSanitizer::deduplicate_quads(&quads, 1e-3);
        assert_eq!(removed.len(), 1);
        assert!(removed.contains(&2));
    }
}

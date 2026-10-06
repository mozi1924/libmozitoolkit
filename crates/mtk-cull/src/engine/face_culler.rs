use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
#[cfg(not(feature = "std"))]
use core::cell::RefCell;
use std::collections::HashMap;

use mtk_core::direction::Direction;
use mtk_core::geometry::Aabb2d;
use mtk_core::{IVec3, Vec3};

use crate::rect_ops::is_face_completely_occluded;
use crate::rules::should_skip_rendering;
use crate::types::{BlockCullMeta, CullCategory, GlassCullMode, LeavesCullMode, FULL_FACE_RECT};

pub use crate::rules::categories::*;

/// High-Performance Unified Face Culling Engine.
#[derive(Debug)]
pub struct FaceCuller {
    pub leaves_cull_mode: LeavesCullMode,
    pub glass_cull_mode: GlassCullMode,
    #[cfg(feature = "std")]
    meta_cache: std::sync::RwLock<HashMap<String, Arc<BlockCullMeta>>>,
    #[cfg(not(feature = "std"))]
    meta_cache: RefCell<HashMap<String, Arc<BlockCullMeta>>>,
}

impl Clone for FaceCuller {
    fn clone(&self) -> Self {
        #[cfg(feature = "std")]
        {
            let cache_clone = self
                .meta_cache
                .read()
                .map(|g| g.clone())
                .unwrap_or_default();
            Self {
                leaves_cull_mode: self.leaves_cull_mode,
                glass_cull_mode: self.glass_cull_mode,
                meta_cache: std::sync::RwLock::new(cache_clone),
            }
        }
        #[cfg(not(feature = "std"))]
        {
            Self {
                leaves_cull_mode: self.leaves_cull_mode,
                glass_cull_mode: self.glass_cull_mode,
                meta_cache: RefCell::new(self.meta_cache.borrow().clone()),
            }
        }
    }
}

impl Default for FaceCuller {
    fn default() -> Self {
        Self::new(LeavesCullMode::SingleFace, GlassCullMode::Group)
    }
}

impl FaceCuller {
    /// Creates a new `FaceCuller` engine with custom leaves and glass modes.
    pub fn new(leaves_cull_mode: LeavesCullMode, glass_cull_mode: GlassCullMode) -> Self {
        Self {
            leaves_cull_mode,
            glass_cull_mode,
            #[cfg(feature = "std")]
            meta_cache: std::sync::RwLock::new(HashMap::new()),
            #[cfg(not(feature = "std"))]
            meta_cache: RefCell::new(HashMap::new()),
        }
    }

    /// Clears the cached block culling metadata.
    pub fn clear_cache(&self) {
        #[cfg(feature = "std")]
        {
            if let Ok(mut cache) = self.meta_cache.write() {
                cache.clear();
            }
        }
        #[cfg(not(feature = "std"))]
        {
            self.meta_cache.borrow_mut().clear();
        }
    }

    /// Number of cached metadata entries.
    pub fn cache_len(&self) -> usize {
        #[cfg(feature = "std")]
        {
            self.meta_cache.read().map(|g| g.len()).unwrap_or(0)
        }
        #[cfg(not(feature = "std"))]
        {
            self.meta_cache.borrow().len()
        }
    }

    /// Retrieves or computes `BlockCullMeta` for a given blockstate string.
    pub fn get_meta(
        &self,
        state_str: &str,
        element_quads: Option<&[([Vec3; 4], Direction)]>,
        is_opaque_hint: Option<bool>,
    ) -> Arc<BlockCullMeta> {
        #[cfg(feature = "std")]
        {
            if let Ok(cache) = self.meta_cache.read() {
                if let Some(meta) = cache.get(state_str) {
                    if element_quads.is_some() && !meta.has_baked_model {
                        // Needs re-bake with detailed quads
                    } else {
                        return Arc::clone(meta);
                    }
                }
            }

            let meta = Arc::new(compute_block_cull_meta(
                state_str,
                element_quads,
                is_opaque_hint,
            ));

            if let Ok(mut cache) = self.meta_cache.write() {
                if cache.len() >= 8192 && !cache.contains_key(state_str) {
                    if let Some(first_key) = cache.keys().next().cloned() {
                        cache.remove(&first_key);
                    }
                }
                cache.insert(state_str.to_string(), Arc::clone(&meta));
            }
            meta
        }
        #[cfg(not(feature = "std"))]
        {
            let mut cache = self.meta_cache.borrow_mut();
            if let Some(meta) = cache.get(state_str) {
                if element_quads.is_some() && !meta.has_baked_model {
                    // Refresh
                } else {
                    return Arc::clone(meta);
                }
            }

            let meta = Arc::new(compute_block_cull_meta(
                state_str,
                element_quads,
                is_opaque_hint,
            ));

            if cache.len() >= 8192 && !cache.contains_key(state_str) {
                if let Some(first_key) = cache.keys().next().cloned() {
                    cache.remove(&first_key);
                }
            }

            cache.insert(state_str.to_string(), Arc::clone(&meta));
            meta
        }
    }

    /// Evaluates Minecraft 1.21+ canonical face visibility test.
    ///
    /// Returns `true` if this face SHOULD be rendered, `false` if culled.
    pub fn should_render_face(
        &self,
        state_meta: &BlockCullMeta,
        neighbor_meta: Option<&BlockCullMeta>,
        direction: Direction,
        quad_face_shape: Option<&[Aabb2d]>,
        block_pos: Option<IVec3>,
        neighbor_pos: Option<IVec3>,
    ) -> bool {
        if state_meta.is_air {
            return false;
        }

        let neighbor_meta = match neighbor_meta {
            Some(n) if !n.is_air => n,
            _ => return true,
        };

        let opp_dir = direction.opposite();

        // 1. Solid / glass blocks must never have external faces culled by adjacent non-full / partial blocks
        let is_snow_cover = direction == Direction::Up
            && (neighbor_meta.block_name == "snow" || neighbor_meta.block_name.ends_with(":snow"))
            && neighbor_meta.has_full_face(Direction::Down);

        if (state_meta.category == CullCategory::SolidOpaque
            || state_meta.category == CullCategory::GlassTranslucent)
            && !is_snow_cover
            && (neighbor_meta.category == CullCategory::NonOccluding
                || (neighbor_meta.category == CullCategory::PartialShape
                    && !(neighbor_meta.block_name.ends_with("_slab")
                        || neighbor_meta.block_name.ends_with("_stairs"))))
        {
            return true;
        }

        // 2. Neighbor full solid face check (neighborFaceShape == Shapes.block())
        if neighbor_meta.has_full_face(opp_dir) {
            return false;
        }

        // 3. Custom skipRendering check (glass, leaves, fluid, snow, roots)
        if should_skip_rendering(
            state_meta,
            neighbor_meta,
            direction,
            self.leaves_cull_mode,
            self.glass_cull_mode,
            block_pos,
            neighbor_pos,
        ) {
            return false;
        }

        // 4. Neighbor empty face check (neighborFaceShape == Shapes.empty())
        if neighbor_meta.has_empty_face(opp_dir) {
            return true;
        }

        // 5. State empty face check (stateFaceShape == Shapes.empty())
        if quad_face_shape.is_none() && state_meta.has_empty_face(direction) {
            return true;
        }

        // 6. 2D Boolean shape occlusion check
        let default_quad_shape = [FULL_FACE_RECT];
        let target_shapes: &[Aabb2d] = if let Some(shapes) = quad_face_shape {
            shapes
        } else {
            let s = state_meta.get_face_shapes(direction);
            if s.is_empty() {
                &default_quad_shape
            } else {
                s
            }
        };
        let neighbor_shapes = neighbor_meta.get_face_shapes(opp_dir);

        if is_face_completely_occluded(target_shapes, neighbor_shapes) {
            return false;
        }

        true
    }
}

/// Compute all visible face directions for a single voxel block against surrounding neighbors.
pub fn get_visible_face_directions<F>(
    x: i32,
    y: i32,
    z: i32,
    state_str: &str,
    culler: &FaceCuller,
    get_neighbor_state: F,
) -> Vec<Direction>
where
    F: Fn(i32, i32, i32) -> Option<String>,
{
    if state_str.is_empty() {
        return Vec::new();
    }
    let meta = culler.get_meta(state_str, None, None);
    if meta.is_air {
        return Vec::new();
    }

    let mut visible = Vec::with_capacity(6);
    let block_pos = IVec3::new(x, y, z);

    for dir in Direction::ALL {
        let offset = dir.offset();
        let n_pos = block_pos + offset;
        let n_state_opt = get_neighbor_state(n_pos.x, n_pos.y, n_pos.z);
        let n_meta_opt = n_state_opt
            .as_deref()
            .map(|s| culler.get_meta(s, None, None));

        if culler.should_render_face(
            &meta,
            n_meta_opt.as_deref(),
            dir,
            None,
            Some(block_pos),
            Some(n_pos),
        ) {
            visible.push(dir);
        }
    }

    visible
}

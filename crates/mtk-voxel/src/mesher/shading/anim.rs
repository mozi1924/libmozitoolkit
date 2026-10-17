use mtk_texture::{AtlasAddressMap, AtlasSpriteLocation};
use std::collections::HashMap;

/// Extracts animation metrics: `anim_timing = [total_frames, frametime, interpolate]`,
/// and `anim_frame_size = [frame_width, frame_height, 0.0]`.
///
/// Delegates to the shared `mtk-material` policy so voxel meshing and external-mesh
/// material replacement cannot drift apart.
#[inline]
pub fn extract_anim_metrics(loc: Option<&AtlasSpriteLocation>) -> ([f32; 3], [f32; 3]) {
    mtk_material::extract_animation_metrics(loc)
}

/// Probes the AtlasAddressMap animated sprite cache for a given texture string.
/// Returns the target `AtlasSpriteLocation` if and only if the sprite exists in `anim_sprites`.
#[inline]
pub fn lookup_animated_from_cache<'a>(
    texture: &str,
    custom_aliases: Option<&HashMap<String, Vec<String>>>,
    atlas: &'a AtlasAddressMap,
) -> Option<&'a AtlasSpriteLocation> {
    mtk_material::resolve_animated_sprite(texture, custom_aliases, atlas).map(|(_, sprite)| sprite)
}

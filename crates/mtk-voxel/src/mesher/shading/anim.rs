use mtk_texture::atlas::{AtlasAddressMap, AtlasSpriteLocation};
use std::collections::HashMap;

/// Extracts animation metrics: `anim_timing = [total_frames, frametime, interpolate]`,
/// and `anim_frame_size = [frame_width, frame_height, 0.0]`.
#[inline]
pub fn extract_anim_metrics(loc: Option<&AtlasSpriteLocation>) -> ([f32; 3], [f32; 3]) {
    if let Some(loc) = loc {
        if loc.is_animated {
            let total_frames = loc.frame_count as f32;
            let frametime = loc
                .animation
                .as_ref()
                .map(|a| a.frametime as f32)
                .unwrap_or(1.0)
                .max(1.0);
            let interpolate = if loc
                .animation
                .as_ref()
                .map(|a| a.interpolate)
                .unwrap_or(false)
            {
                1.0
            } else {
                0.0
            };
            return (
                [total_frames, frametime, interpolate],
                [loc.frame_size[0] as f32, loc.frame_size[1] as f32, 0.0],
            );
        }
    }
    ([1.0, 1.0, 0.0], [16.0, 16.0, 0.0])
}

/// Probes the AtlasAddressMap animated sprite cache for a given texture string.
/// Returns the target `AtlasSpriteLocation` if and only if the sprite exists in `anim_sprites`.
#[inline]
pub fn lookup_animated_from_cache<'a>(
    texture: &str,
    custom_aliases: Option<&HashMap<String, Vec<String>>>,
    atlas: &'a AtlasAddressMap,
) -> Option<&'a AtlasSpriteLocation> {
    if texture.is_empty() {
        return None;
    }
    // 1. Direct O(1) string lookup (handles exact path e.g. "minecraft:block/kelp" or "block/kelp")
    if let Some(anim_sp) = atlas.lookup_animated_str(texture) {
        return Some(anim_sp);
    }
    // 2. Parsed ResourceLocation
    if let Ok(loc) = mtk_resource::ResourceLocation::parse(texture) {
        if let Some(anim_sp) = atlas.lookup_animated(&loc) {
            return Some(anim_sp);
        }
    }
    // 3. Clean identifier lookup
    let cleaned = mtk_material::clean_identifier(texture);
    if cleaned != texture {
        if let Some(anim_sp) = atlas.lookup_animated_str(&cleaned) {
            return Some(anim_sp);
        }
        if let Ok(loc) = mtk_resource::ResourceLocation::parse(&cleaned) {
            if let Some(anim_sp) = atlas.lookup_animated(&loc) {
                return Some(anim_sp);
            }
        }
    }
    // 4. Custom alias table
    if let Some(aliases) = custom_aliases {
        if let Some(cands) = aliases.get(texture).or_else(|| aliases.get(&cleaned)) {
            for cand in cands {
                if let Some(anim_sp) = atlas.lookup_animated_str(cand) {
                    return Some(anim_sp);
                }
            }
        }
    }
    None
}

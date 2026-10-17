//! Face-level atlas routing and animation re-routing.
//!
//! These functions are deliberately UV-independent where possible: a host can
//! resolve material routes once, then decide whether it needs to re-project an
//! existing atlas UV layout (for example when the user toggles animated textures
//! after materials have already been replaced).

use std::collections::HashMap;

use mtk_texture::{AtlasAddressMap, AtlasSpriteLocation};

use crate::remap::remap_sprite_to_sprite;
use crate::resolver::{
    extract_animation_metrics, resolve_animated_sprite, resolve_routed_sprite, MaterialResolver,
};
use crate::types::{MeshAnimationRerouteResult, MeshMaterialRoute};

/// Default route for a face whose material could not be resolved.
fn default_route() -> MeshMaterialRoute {
    MeshMaterialRoute {
        chunk_id: 0,
        texture_id: 0,
        is_animated: false,
        anim_timing: [1.0, 1.0, 0.0],
        anim_frame_size: [16.0, 16.0, 0.0],
        mapped: false,
    }
}

fn route_from_sprite(sprite: &AtlasSpriteLocation) -> MeshMaterialRoute {
    let (anim_timing, anim_frame_size) = extract_animation_metrics(Some(sprite));
    MeshMaterialRoute {
        chunk_id: sprite.chunk_id,
        texture_id: sprite.texture_id,
        is_animated: sprite.is_animated,
        anim_timing,
        anim_frame_size,
        mapped: true,
    }
}

/// Resolve atlas routes for a list of face material / texture names.
///
/// This is the allocation-light path used by provenance restores: it never
/// touches UVs and returns one compact record per face.
pub fn resolve_mesh_material_routes(
    face_texture_keys: &[String],
    aliases: Option<&HashMap<String, Vec<String>>>,
    address_map: &AtlasAddressMap,
    enable_animated_textures: bool,
) -> Vec<MeshMaterialRoute> {
    face_texture_keys
        .iter()
        .map(|key| {
            resolve_routed_sprite(key, aliases, address_map, enable_animated_textures)
                .map(|(_, sprite)| route_from_sprite(sprite))
                .unwrap_or_else(default_route)
        })
        .collect()
}

/// Locate the sprite that was previously used for a face.
///
/// The old sprite is identified by the stored chunk id plus the source texture
/// key. Checking both the static and animated address maps makes the lookup
/// robust across animation-policy changes.
fn find_old_sprite<'a>(
    key: &str,
    chunk_id: u16,
    aliases: Option<&HashMap<String, Vec<String>>>,
    address_map: &'a AtlasAddressMap,
) -> Option<&'a AtlasSpriteLocation> {
    if let Some(sprite) = address_map.lookup_static_str(key) {
        if sprite.chunk_id == chunk_id {
            return Some(sprite);
        }
    }
    if let Some(sprite) = address_map.lookup_animated_str(key) {
        if sprite.chunk_id == chunk_id {
            return Some(sprite);
        }
    }
    if let Some((_, sprite)) = MaterialResolver::resolve(key, aliases, address_map) {
        if sprite.chunk_id == chunk_id {
            return Some(sprite);
        }
    }
    if let Some((_, sprite)) = resolve_animated_sprite(key, aliases, address_map) {
        if sprite.chunk_id == chunk_id {
            return Some(sprite);
        }
    }
    None
}

/// Re-route an already atlas-mapped mesh to the current animation policy.
///
/// `source_uvs` must be the mesh's existing atlas UV loops. For every face the
/// old sprite is located by `face_old_chunk_ids` + source key, then each UV is
/// decoded from the old sprite and projected into the new target sprite. This
/// keeps the operation idempotent and avoids a second pass over the original
/// (pre-atlas) UVs, which are no longer available after material replacement.
pub fn reroute_mesh_animation(
    source_uvs: &[[f32; 2]],
    face_loop_ranges: &[(u32, u32)],
    face_texture_keys: &[String],
    face_old_chunk_ids: &[u16],
    address_map: &AtlasAddressMap,
    aliases: Option<&HashMap<String, Vec<String>>>,
    enable_animated_textures: bool,
) -> MeshAnimationRerouteResult {
    let face_count = face_texture_keys.len();
    let mut atlas_uvs = source_uvs.to_vec();
    let mut face_chunk_ids = vec![0u16; face_count];
    let mut face_texture_ids = vec![0u32; face_count];
    let mut face_is_animated = vec![false; face_count];
    let mut face_anim_timing = vec![[1.0f32, 1.0f32, 0.0f32]; face_count];
    let mut face_anim_frame_size = vec![[16.0f32, 16.0f32, 0.0f32]; face_count];
    let mut unmapped_faces = 0usize;
    let mut uv_unmapped_faces = 0usize;

    if face_loop_ranges.len() != face_count {
        return MeshAnimationRerouteResult {
            atlas_uvs,
            face_chunk_ids,
            face_texture_ids,
            face_is_animated,
            face_anim_timing,
            face_anim_frame_size,
            unmapped_faces: face_count,
            uv_unmapped_faces: 0,
        };
    }

    for (face_idx, &(start, count)) in face_loop_ranges.iter().enumerate() {
        let key = &face_texture_keys[face_idx];
        let Some((_, target_sprite)) =
            resolve_routed_sprite(key, aliases, address_map, enable_animated_textures)
        else {
            unmapped_faces += 1;
            continue;
        };

        let (timing, frame_size) = extract_animation_metrics(Some(target_sprite));
        face_chunk_ids[face_idx] = target_sprite.chunk_id;
        face_texture_ids[face_idx] = target_sprite.texture_id;
        face_is_animated[face_idx] = target_sprite.is_animated;
        face_anim_timing[face_idx] = timing;
        face_anim_frame_size[face_idx] = frame_size;

        let old_chunk_id = face_old_chunk_ids.get(face_idx).copied().unwrap_or(0);
        if old_chunk_id == target_sprite.chunk_id {
            continue;
        }

        let Some(old_sprite) = find_old_sprite(key, old_chunk_id, aliases, address_map) else {
            uv_unmapped_faces += 1;
            continue;
        };

        let start_idx = start as usize;
        let end_idx = start_idx.saturating_add(count as usize);
        if start_idx >= atlas_uvs.len() || end_idx > atlas_uvs.len() {
            uv_unmapped_faces += 1;
            continue;
        }

        for uv in &mut atlas_uvs[start_idx..end_idx] {
            *uv = remap_sprite_to_sprite(uv[0], uv[1], old_sprite, target_sprite);
        }
    }

    MeshAnimationRerouteResult {
        atlas_uvs,
        face_chunk_ids,
        face_texture_ids,
        face_is_animated,
        face_anim_timing,
        face_anim_frame_size,
        unmapped_faces,
        uv_unmapped_faces,
    }
}

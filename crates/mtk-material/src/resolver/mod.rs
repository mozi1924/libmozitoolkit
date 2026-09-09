use mtk_resource::ResourceLocation;
use mtk_texture::{AtlasAddressMap, AtlasSpriteLocation};

use crate::icecube::{clean_icecube_name, resolve_icecube_candidates};
use crate::jmc2obj::{clean_jmc2obj_name, resolve_jmc2obj_candidates};
use crate::mineways::{decode_mineways_uv, is_mineways_atlas_name};
use crate::types::ImporterOrigin;

/// Auto-detect the likely importer origin from a material name.
pub fn detect_importer_origin(material_name: &str) -> ImporterOrigin {
    let lower = material_name.to_lowercase();
    if is_mineways_atlas_name(&lower) || lower.starts_with("mineways") || lower.starts_with("mw_") {
        ImporterOrigin::Mineways
    } else if lower.starts_with("jmc2obj") || lower.starts_with("minecraft_block") || lower.starts_with("pattern_") {
        ImporterOrigin::Jmc2Obj
    } else if lower.starts_with("ice_cube") || lower.starts_with("icecube") || lower.starts_with("m_") {
        ImporterOrigin::IceCube
    } else {
        ImporterOrigin::Generic
    }
}

/// Unified resolver for mapping raw DCC material names to canonical AtlasSpriteLocations.
pub struct MaterialResolver;

impl MaterialResolver {
    /// Resolve a raw material name to a target Atlas Sprite Location.
    pub fn resolve<'a>(
        raw_material_name: &str,
        origin: ImporterOrigin,
        address_map: &'a AtlasAddressMap,
    ) -> Option<(ResourceLocation, &'a AtlasSpriteLocation)> {
        let actual_origin = match origin {
            ImporterOrigin::Auto => detect_importer_origin(raw_material_name),
            other => other,
        };

        // 1. Direct O(1) lookup attempt
        if let Some(loc) = address_map.lookup_str(raw_material_name) {
            if let Ok(res_loc) = ResourceLocation::parse(raw_material_name) {
                return Some((res_loc, loc));
            }
        }

        // 2. Importer specific candidate resolution
        let candidates = match actual_origin {
            ImporterOrigin::Jmc2Obj => {
                let cleaned = clean_jmc2obj_name(raw_material_name);
                resolve_jmc2obj_candidates(&cleaned)
            }
            ImporterOrigin::IceCube => {
                let cleaned = clean_icecube_name(raw_material_name);
                resolve_icecube_candidates(&cleaned)
            }
            ImporterOrigin::Mineways | ImporterOrigin::Generic | ImporterOrigin::Auto => {
                let cleaned = clean_jmc2obj_name(raw_material_name);
                resolve_jmc2obj_candidates(&cleaned)
            }
        };

        // 3. Search candidates in address_map
        for cand in candidates {
            if let Some(sprite_loc) = address_map.lookup_str(&cand) {
                let res_loc = ResourceLocation::parse(&cand)
                    .unwrap_or_else(|_| ResourceLocation::new("minecraft", cand));
                return Some((res_loc, sprite_loc));
            }
        }

        None
    }

    /// Resolve Mineways atlas face UV to canonical sprite location.
    pub fn resolve_mineways_face<'a>(
        u: f32,
        v: f32,
        image_width: u32,
        image_height: u32,
        address_map: &'a AtlasAddressMap,
    ) -> Option<(ResourceLocation, &'a AtlasSpriteLocation, [f32; 2])> {
        let (pri, alt, local_uv) = decode_mineways_uv(u, v, image_width, image_height);

        if let Some(pri_name) = pri {
            let candidates = resolve_jmc2obj_candidates(pri_name);
            for cand in candidates {
                if let Some(sprite_loc) = address_map.lookup_str(&cand) {
                    let res_loc = ResourceLocation::parse(&cand)
                        .unwrap_or_else(|_| ResourceLocation::new("minecraft", cand));
                    return Some((res_loc, sprite_loc, local_uv));
                }
            }
        }

        if let Some(alt_name) = alt {
            let candidates = resolve_jmc2obj_candidates(alt_name);
            for cand in candidates {
                if let Some(sprite_loc) = address_map.lookup_str(&cand) {
                    let res_loc = ResourceLocation::parse(&cand)
                        .unwrap_or_else(|_| ResourceLocation::new("minecraft", cand));
                    return Some((res_loc, sprite_loc, local_uv));
                }
            }
        }

        None
    }
}

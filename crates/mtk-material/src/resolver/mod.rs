use std::collections::HashMap;
use mtk_resource::ResourceLocation;
use mtk_texture::{AtlasAddressMap, AtlasSpriteLocation};

use crate::types::GridAtlasSpec;

/// Generic helper to clean a raw material or texture name into a normalized identifier stem.
pub fn clean_identifier(raw: &str) -> String {
    let mut s = raw.trim().to_lowercase();

    // 1. Strip file extensions
    if let Some(stripped) = s.strip_suffix(".png").or_else(|| s.strip_suffix(".jpg")) {
        s = stripped.to_string();
    }

    // 2. Strip blender duplicate suffixes (".001", "_001")
    if let Some(idx) = s.rfind('.') {
        if s[idx + 1..].chars().all(|c| c.is_ascii_digit()) {
            s = s[..idx].to_string();
        }
    }

    // 3. Strip standard namespace / path / exporter prefixes
    let prefixes = [
        "tex/minecraft/",
        "textures/block/",
        "textures/entity/",
        "textures/item/",
        "textures/banner/",
        "textures/",
        "minecraft_block-",
        "minecraft_entity-",
        "minecraft_item-",
        "minecraft_banner-",
        "minecraft_block_",
        "minecraft_entity_",
        "minecraft_item_",
        "jmc2obj_block-",
        "jmc2obj_block_",
        "jmc2obj_entity-",
        "jmc2obj_item-",
        "jmc2obj_",
        "minecraft:",
        "minecraft-",
        "mw_",
        "mwo_",
        "tile_",
        "tile-",
        "tile.",
        "block_",
        "block-",
        "block.",
        "block/",
        "entity/",
        "item/",
    ];

    for prefix in prefixes {
        if s.starts_with(prefix) {
            s = s[prefix.len()..].to_string();
            break;
        }
    }

    // 4. Strip exporter biome suffixes
    let biome_suffixes = [
        "-desert", "-forest", "-swamp", "-taiga", "-snow", "-ocean", "-jungle",
        "-badlands", "-savanna", "-dark_forest", "-birch_forest", "-plains",
        "-meadow", "-mangrove", "-cherry_grove", "-cold_ocean", "-warm_ocean",
        "_desert", "_forest", "_swamp", "_taiga", "_snow", "_ocean", "_jungle",
        "_badlands", "_savanna", "_dark_forest", "_birch_forest", "_plains",
        "_meadow", "_mangrove", "_cherry_grove", "_cold_ocean", "_warm_ocean",
        "_y",
    ];

    for suffix in biome_suffixes {
        if s.ends_with(suffix) {
            s = s[..s.len() - suffix.len()].to_string();
            break;
        }
    }

    if !s.contains('/') {
        s = s.replace(' ', "_").replace('-', "_");
    }

    s
}

/// Convert a grid atlas UV coordinate (u, v) to its local [0, 1] swatch coordinate based on GridAtlasSpec.
#[inline]
pub fn remap_grid_atlas_uv_to_local(
    u: f32,
    v: f32,
    spec: &GridAtlasSpec,
) -> [f32; 2] {
    let tex_w = spec.image_width.max(1) as f32;
    let tex_h = spec.image_height.max(1) as f32;

    let px = u * tex_w;
    let py = (1.0 - v) * tex_h;

    let atlas_col = (px / spec.swatch_size).floor();
    let atlas_row = (py / spec.swatch_size).floor();

    let lu = (px - (atlas_col * spec.swatch_size + spec.border)) / spec.tile_size;
    let lv = 1.0 - ((py - (atlas_row * spec.swatch_size + spec.border)) / spec.tile_size);

    [lu, lv]
}

/// Decode a polygon UV coordinate on a grid atlas to its candidate identifiers and local [0, 1] UVs.
pub fn decode_grid_atlas_uv<'a>(
    u: f32,
    v: f32,
    spec: &'a GridAtlasSpec,
) -> (Option<&'a Vec<String>>, [f32; 2]) {
    let tex_w = spec.image_width.max(1) as f32;
    let tex_h = spec.image_height.max(1) as f32;
    let swatches_per_row = (spec.image_width as f32 / spec.swatch_size).floor().max(1.0) as usize;

    let px = u * tex_w;
    let py = (1.0 - v) * tex_h;

    let atlas_col = (px / spec.swatch_size).floor() as usize;
    let atlas_row = (py / spec.swatch_size).floor() as usize;
    let swatch_id = atlas_col + atlas_row * swatches_per_row;

    let candidates = spec.swatch_to_candidates.get(&swatch_id);
    let local_uv = remap_grid_atlas_uv_to_local(u, v, spec);
    (candidates, local_uv)
}

#[inline]
fn get_fluid_candidates(name: &str) -> Option<&'static [&'static str]> {
    match name {
        "water" | "minecraft:water" | "stationary_water" | "water_still" => {
            Some(&["block/water_still", "block/water_flow"])
        }
        "flowing_water" | "minecraft:flowing_water" | "water_flow" => {
            Some(&["block/water_flow", "block/water_still"])
        }
        "lava" | "minecraft:lava" | "stationary_lava" | "lava_still" => {
            Some(&["block/lava_still", "block/lava_flow"])
        }
        "flowing_lava" | "minecraft:flowing_lava" | "lava_flow" => {
            Some(&["block/lava_flow", "block/lava_still"])
        }
        _ => None,
    }
}

/// Unified data-driven resolver for mapping raw DCC material names to canonical AtlasSpriteLocations.
pub struct MaterialResolver;

impl MaterialResolver {
    /// Resolve a raw material name to a target Atlas Sprite Location using optional external alias table.
    pub fn resolve<'a>(
        raw_material_name: &str,
        custom_aliases: Option<&HashMap<String, Vec<String>>>,
        address_map: &'a AtlasAddressMap,
    ) -> Option<(ResourceLocation, &'a AtlasSpriteLocation)> {
        let cleaned = clean_identifier(raw_material_name);

        // 0. Built-in fluid alias prioritization (prevents "water" / "lava" from colliding with paintings or particles)
        if let Some(candidates) = get_fluid_candidates(raw_material_name).or_else(|| get_fluid_candidates(&cleaned)) {
            for &cand in candidates {
                if let Some(sprite_loc) = address_map.lookup_str(cand) {
                    let res_loc = ResourceLocation::parse(cand)
                        .unwrap_or_else(|_| ResourceLocation::new("minecraft", cand));
                    return Some((res_loc, sprite_loc));
                }
            }
        }

        // 1. Direct O(1) lookup attempt on raw name
        if let Some(loc) = address_map.lookup_str(raw_material_name) {
            if let Ok(res_loc) = ResourceLocation::parse(raw_material_name) {
                return Some((res_loc, loc));
            }
        }

        // 2. Direct O(1) lookup attempt on cleaned name
        if let Some(loc) = address_map.lookup_str(&cleaned) {
            if let Ok(res_loc) = ResourceLocation::parse(&cleaned) {
                return Some((res_loc, loc));
            }
        }

        // 3. Search external / user-provided alias table
        if let Some(alias_map) = custom_aliases {
            // Check raw name
            if let Some(candidates) = alias_map.get(raw_material_name) {
                for cand in candidates {
                    if let Some(sprite_loc) = address_map.lookup_str(cand) {
                        let res_loc = ResourceLocation::parse(cand)
                            .unwrap_or_else(|_| ResourceLocation::new("minecraft", cand));
                        return Some((res_loc, sprite_loc));
                    }
                }
            }

            // Check cleaned name
            if cleaned != raw_material_name {
                if let Some(candidates) = alias_map.get(&cleaned) {
                    for cand in candidates {
                        if let Some(sprite_loc) = address_map.lookup_str(cand) {
                            let res_loc = ResourceLocation::parse(cand)
                                .unwrap_or_else(|_| ResourceLocation::new("minecraft", cand));
                            return Some((res_loc, sprite_loc));
                        }
                    }
                }
            }
        }

        // 3.5. Built-in legacy redstone material alias fallbacks (Mineways / Jmc2Obj)
        if cleaned.starts_with("redstone_dust") || cleaned.starts_with("redstone_wire") {
            let base = cleaned
                .strip_suffix("_on")
                .or_else(|| cleaned.strip_suffix("_off"))
                .unwrap_or(&cleaned);
            let candidates: &[&str] = match base {
                "redstone_dust_line0" | "redstone_wire_line0" => &[
                    "block/redstone_dust_line0",
                    "block/redstone_dust_overlay",
                    "block/redstone_dust_dot",
                ],
                "redstone_dust_line1" | "redstone_wire_line1" => &[
                    "block/redstone_dust_line1",
                    "block/redstone_dust_overlay",
                    "block/redstone_dust_dot",
                ],
                "redstone_dust_dot" | "redstone_wire_dot" => &[
                    "block/redstone_dust_dot",
                    "block/redstone_dust_overlay",
                ],
                "redstone_dust_angled" | "redstone_wire_angled" => &[
                    "block/redstone_dust_line0",
                    "block/redstone_dust_line1",
                    "block/redstone_dust_overlay",
                ],
                "redstone_dust_three_way" | "redstone_wire_three_way" => &[
                    "block/redstone_dust_line1",
                    "block/redstone_dust_overlay",
                    "block/redstone_dust_dot",
                ],
                "redstone_dust_four_way" | "redstone_dust_cross" | "redstone_wire_four_way" => &[
                    "block/redstone_dust_dot",
                    "block/redstone_dust_line1",
                    "block/redstone_dust_overlay",
                ],
                "redstone_dust_overlay" | "redstone_wire_overlay" => &[
                    "block/redstone_dust_overlay",
                ],
                _ => &[],
            };

            for &cand in candidates {
                if let Some(sprite_loc) = address_map.lookup_str(cand) {
                    let res_loc = ResourceLocation::parse(cand)
                        .unwrap_or_else(|_| ResourceLocation::new("minecraft", cand));
                    return Some((res_loc, sprite_loc));
                }
            }
        }

        // 4. Default standard Minecraft category fallbacks
        let fallbacks = [
            format!("block/{}", cleaned),
            format!("entity/{}", cleaned),
            format!("item/{}", cleaned),
            cleaned.clone(),
        ];

        for cand in &fallbacks {
            if let Some(sprite_loc) = address_map.lookup_str(cand) {
                let res_loc = ResourceLocation::parse(cand)
                    .unwrap_or_else(|_| ResourceLocation::new("minecraft", cand));
                return Some((res_loc, sprite_loc));
            }
        }

        None
    }

    /// Resolve a face on a grid-based atlas to canonical sprite location and local UVs.
    pub fn resolve_grid_atlas_face<'a>(
        u: f32,
        v: f32,
        spec: &GridAtlasSpec,
        custom_aliases: Option<&HashMap<String, Vec<String>>>,
        address_map: &'a AtlasAddressMap,
    ) -> Option<(ResourceLocation, &'a AtlasSpriteLocation, [f32; 2])> {
        let (candidates_opt, local_uv) = decode_grid_atlas_uv(u, v, spec);

        if let Some(candidates) = candidates_opt {
            for cand in candidates {
                if let Some((res_loc, sprite_loc)) = Self::resolve(cand, custom_aliases, address_map) {
                    return Some((res_loc, sprite_loc, local_uv));
                }
            }
        }

        None
    }
}

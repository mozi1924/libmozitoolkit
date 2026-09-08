use std::collections::HashSet;
use crate::atlas::{AtlasDefinition, AtlasSource};
use crate::error::ResourceError;
use crate::identifier::ResourceLocation;
use crate::meta::{TextureMetadata, AnimationMetadata};
use crate::pack::source::ResourcePack;

/// Discovered single sprite entry ready for decoding and stitching.
#[derive(Debug, Clone, PartialEq)]
pub struct DiscoveredSprite {
    /// Canonical sprite identifier, e.g. `"minecraft:block/stone"`.
    pub sprite_id: ResourceLocation,
    /// Texture file location in resource pack, e.g. `"minecraft:block/stone"`.
    pub texture_location: ResourceLocation,
    /// Direct raw PNG bytes if already loaded (e.g. from unstitch or memory).
    pub raw_albedo: Option<Vec<u8>>,
    pub raw_normal: Option<Vec<u8>>,
    pub raw_specular: Option<Vec<u8>>,
    pub metadata: Option<AnimationMetadata>,
}

/// Composite PBR companion data extracted for a texture resource.
#[derive(Debug, Clone, Default)]
pub struct PbrCompanions {
    pub albedo: Option<Vec<u8>>,
    pub normal: Option<Vec<u8>>,
    pub specular: Option<Vec<u8>>,
    pub mcmeta: Option<AnimationMetadata>,
}

/// Ordered hierarchy of resource packs evaluated from top (highest priority) to bottom (vanilla fallback).
pub struct ResourcePackStack {
    packs: Vec<Box<dyn ResourcePack>>,
}

impl Default for ResourcePackStack {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourcePackStack {
    pub fn new() -> Self {
        Self { packs: Vec::new() }
    }

    /// Push a resource pack to the top of the stack (highest priority).
    pub fn push_pack(&mut self, pack: Box<dyn ResourcePack>) {
        self.packs.insert(0, pack);
    }

    /// Append a resource pack to the bottom of the stack (lowest priority fallback anchor).
    pub fn append_pack(&mut self, pack: Box<dyn ResourcePack>) {
        self.packs.push(pack);
    }

    /// Number of packs in the stack.
    pub fn len(&self) -> usize {
        self.packs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.packs.is_empty()
    }

    /// Open a file by its explicit asset path, returning the first match from top to bottom.
    pub fn open_asset_raw(&self, asset_path: &str) -> Option<Vec<u8>> {
        for pack in &self.packs {
            if let Some(bytes) = pack.open(asset_path) {
                return Some(bytes);
            }
        }
        None
    }

    /// Open a specific categorized asset (e.g. category `"textures"`, ext `"png"`).
    pub fn open_texture_raw(&self, location: &ResourceLocation) -> Option<Vec<u8>> {
        let path = location.to_asset_path("textures", "png");
        self.open_asset_raw(&path)
    }

    /// Read and parse an atlas definition from `assets/<namespace>/atlases/<name>.json`.
    pub fn load_atlas_definition(&self, location: &ResourceLocation) -> Result<AtlasDefinition, ResourceError> {
        let path = location.to_asset_path("atlases", "json");
        let bytes = self.open_asset_raw(&path).ok_or_else(|| {
            ResourceError::NotFound(format!("Atlas definition not found: {}", path))
        })?;
        let json_str = std::str::from_utf8(&bytes).map_err(|e| {
            ResourceError::AtlasConfig(format!("Invalid UTF-8 in atlas JSON: {}", e))
        })?;
        AtlasDefinition::parse_json(json_str).map_err(ResourceError::from)
    }

    /// Read and parse an atlas definition for a category, falling back to standard default if not found.
    pub fn load_atlas_category(&self, category: &crate::atlas::AtlasCategory) -> AtlasDefinition {
        let loc = category.atlas_location();
        self.load_atlas_definition(&loc)
            .unwrap_or_else(|_| category.default_definition())
    }

    /// Resolve all PBR companions (`_n`, `_s`, `.mcmeta`) using granular per-channel fallback.
    pub fn resolve_pbr_companions(&self, location: &ResourceLocation) -> PbrCompanions {
        let mut companions = PbrCompanions::default();

        // Try standard textures/ prefix as well as raw path (for optifine/ctm/ assets)
        let candidates = [
            location.to_asset_path("textures", "png"),
            format!("assets/{}/{}.png", location.namespace, location.path),
        ];

        // 1. Albedo
        for path in &candidates {
            if let Some(bytes) = self.open_asset_raw(path) {
                companions.albedo = Some(bytes);
                break;
            }
        }

        // 2. Normal (try _n.png then _N.png)
        let normal_candidates = [
            location.with_suffix("_n").to_asset_path("textures", "png"),
            location.with_suffix("_N").to_asset_path("textures", "png"),
            format!("assets/{}/{}_n.png", location.namespace, location.path),
            format!("assets/{}/{}_N.png", location.namespace, location.path),
        ];
        for path in &normal_candidates {
            if let Some(bytes) = self.open_asset_raw(path) {
                companions.normal = Some(bytes);
                break;
            }
        }

        // 3. Specular (try _s.png then _S.png)
        let spec_candidates = [
            location.with_suffix("_s").to_asset_path("textures", "png"),
            location.with_suffix("_S").to_asset_path("textures", "png"),
            format!("assets/{}/{}_s.png", location.namespace, location.path),
            format!("assets/{}/{}_S.png", location.namespace, location.path),
        ];
        for path in &spec_candidates {
            if let Some(bytes) = self.open_asset_raw(path) {
                companions.specular = Some(bytes);
                break;
            }
        }

        // 4. MCMETA animation
        let meta_candidates = [
            location.to_mcmeta_asset_path("textures", "png"),
            format!("assets/{}/{}.png.mcmeta", location.namespace, location.path),
        ];
        for path in &meta_candidates {
            if let Some(meta_bytes) = self.open_asset_raw(path) {
                if let Ok(meta_str) = std::str::from_utf8(&meta_bytes) {
                    if let Ok(tex_meta) = TextureMetadata::parse_json(meta_str) {
                        companions.mcmeta = tex_meta.animation;
                        break;
                    }
                }
            }
        }

        companions
    }

    /// Scan all active resource packs and load all OptiFine / Continuity CTM rules.
    pub fn load_ctm_rules(&self) -> Vec<crate::ctm::CtmRule> {
        let mut rules = Vec::new();
        let mut seen_paths = HashSet::new();

        for pack in &self.packs {
            for file in pack.list_files("assets/") {
                if !file.ends_with(".properties") {
                    continue;
                }
                if !file.contains("/optifine/ctm/") && !file.contains("/textures/") {
                    continue;
                }
                if seen_paths.contains(&file) {
                    continue;
                }
                seen_paths.insert(file.clone());

                if let Some(bytes) = pack.open(&file) {
                    if let Ok(content) = std::str::from_utf8(&bytes) {
                        let parts: Vec<&str> = file.split('/').collect();
                        let namespace = if parts.len() > 1 { parts[1] } else { crate::DEFAULT_NAMESPACE };
                        let rel_path = parts[2..].join("/");
                        if let Some(rule) = crate::ctm::CtmRule::parse_properties(&rel_path, namespace, content) {
                            rules.push(rule);
                        }
                    }
                }
            }
        }

        rules.sort_by_key(|r| std::cmp::Reverse(r.priority));
        rules
    }

    /// Collect and expand all Sprite references declared in an AtlasDefinition into discrete `DiscoveredSprite`s.
    pub fn collect_sprites_for_atlas(&self, definition: &AtlasDefinition) -> Result<Vec<DiscoveredSprite>, ResourceError> {
        let mut results = Vec::new();
        let mut registered_sprites: HashSet<ResourceLocation> = HashSet::new();

        for source in &definition.sources {
            match source {
                AtlasSource::Directory { source: dir_src, prefix } => {
                    let mut found_paths = HashSet::new();
                    // Scan all packs in stack
                    for pack in &self.packs {
                        for file in pack.list_files("assets/") {
                            if !file.ends_with(".png") {
                                continue;
                            }
                            // Companion textures (_n, _s) must never become standalone sprites
                            if file.ends_with("_n.png") || file.ends_with("_N.png")
                                || file.ends_with("_s.png") || file.ends_with("_S.png") {
                                continue;
                            }

                            if let Some(loc) = ResourceLocation::from_asset_path(&file, "textures", "png") {
                                if loc.path == *dir_src || loc.path.starts_with(&format!("{}/", dir_src)) {
                                    found_paths.insert(loc);
                                }
                            }
                        }
                    }

                    // Convert to canonical Sprite locations with prefix
                    for tex_loc in found_paths {
                        let short = if tex_loc.path == *dir_src {
                            ""
                        } else {
                            tex_loc.path.strip_prefix(&format!("{}/", dir_src)).unwrap_or(&tex_loc.path)
                        };
                        let sprite_path = format!("{}{}", prefix, short);
                        let sprite_id = ResourceLocation::new(&tex_loc.namespace, sprite_path);

                        if !registered_sprites.contains(&sprite_id) {
                            let companions = self.resolve_pbr_companions(&tex_loc);
                            if companions.albedo.is_some() {
                                registered_sprites.insert(sprite_id.clone());
                                results.push(DiscoveredSprite {
                                    sprite_id,
                                    texture_location: tex_loc,
                                    raw_albedo: companions.albedo,
                                    raw_normal: companions.normal,
                                    raw_specular: companions.specular,
                                    metadata: companions.mcmeta,
                                });
                            }
                        }
                    }
                }

                AtlasSource::Single { resource, sprite } => {
                    let sprite_id = sprite.clone().unwrap_or_else(|| resource.clone());
                    if !registered_sprites.contains(&sprite_id) {
                        let companions = self.resolve_pbr_companions(resource);
                        if companions.albedo.is_some() {
                            registered_sprites.insert(sprite_id.clone());
                            results.push(DiscoveredSprite {
                                sprite_id,
                                texture_location: resource.clone(),
                                raw_albedo: companions.albedo,
                                raw_normal: companions.normal,
                                raw_specular: companions.specular,
                                metadata: companions.mcmeta,
                            });
                        }
                    }
                }

                AtlasSource::PalettedPermutations { palette_key: _, permutations, textures } => {
                    // Collect permutations declarations (will be baked by mtk-texture)
                    for tex_loc in textures {
                        for perm_name in permutations.keys() {
                            let sprite_path = format!("{}_{}", tex_loc.path, perm_name);
                            let sprite_id = ResourceLocation::new(&tex_loc.namespace, sprite_path);
                            if !registered_sprites.contains(&sprite_id) {
                                registered_sprites.insert(sprite_id.clone());
                                // We record base texture for permutation baking
                                let companions = self.resolve_pbr_companions(tex_loc);
                                results.push(DiscoveredSprite {
                                    sprite_id,
                                    texture_location: tex_loc.clone(),
                                    raw_albedo: companions.albedo,
                                    raw_normal: companions.normal,
                                    raw_specular: companions.specular,
                                    metadata: companions.mcmeta,
                                });
                            }
                        }
                    }
                }

                AtlasSource::Unstitch { resource, divisor_x: _, divisor_y: _, regions } => {
                    let companions = self.resolve_pbr_companions(resource);
                    for region in regions {
                        if !registered_sprites.contains(&region.sprite) {
                            registered_sprites.insert(region.sprite.clone());
                            results.push(DiscoveredSprite {
                                sprite_id: region.sprite.clone(),
                                texture_location: resource.clone(),
                                raw_albedo: companions.albedo.clone(),
                                raw_normal: companions.normal.clone(),
                                raw_specular: companions.specular.clone(),
                                metadata: None,
                            });
                        }
                    }
                }

                AtlasSource::Filter { pattern } => {
                    // Filter matching entries
                    results.retain(|s| {
                        let ns_match = pattern.namespace.as_ref().is_none_or(|ns_pat| s.sprite_id.namespace.contains(ns_pat));
                        let path_match = pattern.path.as_ref().is_none_or(|p_pat| s.sprite_id.path.contains(p_pat));
                        !(ns_match && path_match)
                    });
                }
            }
        }

        Ok(results)
    }

    /// Collect atlas sprites including all sub-tiles referenced by active CTM rules.
    pub fn collect_sprites_including_ctm(
        &self,
        definition: &AtlasDefinition,
        ctm_rules: &[crate::ctm::CtmRule],
    ) -> Result<Vec<DiscoveredSprite>, ResourceError> {
        let mut results = self.collect_sprites_for_atlas(definition)?;
        let mut registered_sprites: HashSet<ResourceLocation> = results.iter().map(|s| s.sprite_id.clone()).collect();

        for rule in ctm_rules {
            for tile_loc in rule.tiles.iter().flatten() {
                if !registered_sprites.contains(tile_loc) {
                    let companions = self.resolve_pbr_companions(tile_loc);
                    if companions.albedo.is_some() {
                        registered_sprites.insert(tile_loc.clone());
                        results.push(DiscoveredSprite {
                            sprite_id: tile_loc.clone(),
                            texture_location: tile_loc.clone(),
                            raw_albedo: companions.albedo,
                            raw_normal: companions.normal,
                            raw_specular: companions.specular,
                            metadata: companions.mcmeta,
                        });
                    }
                }
            }
        }

        Ok(results)
    }
}

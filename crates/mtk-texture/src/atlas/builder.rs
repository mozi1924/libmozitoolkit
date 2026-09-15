use std::collections::HashMap;
use mtk_resource::{AtlasDefinition, AtlasSource, ResourceLocation, ResourcePackStack};
use crate::atlas::address_map::{AtlasAddressMap, AtlasChunkMeta, AtlasSpriteLocation, SpriteKind};
use crate::error::TextureError;
use crate::image::buffer::RgbaBuffer;
use crate::image::loader::DecodedSprite;
use crate::image::padding::apply_edge_clamping_padding;
use crate::image::palette::bake_paletted_permutation;
use crate::stitcher::stitcher::Stitcher;

/// Configuration parameters for atlas generation.
#[derive(Debug, Clone)]
pub struct AtlasBuilderConfig {
    pub max_width: u32,
    pub max_height: u32,
    pub mip_level: u32,
    pub padding: u32,
}

impl Default for AtlasBuilderConfig {
    fn default() -> Self {
        Self {
            max_width: 4096,
            max_height: 4096,
            mip_level: 0,
            padding: 0,
        }
    }
}

/// A complete baked atlas sheet (chunk) with all active PBR buffers.
#[derive(Debug, Clone)]
pub struct BakedAtlasChunk {
    pub chunk_id: u16,
    pub category: String,
    pub is_animated: bool,
    pub category_chunk_index: usize,
    pub width: u32,
    pub height: u32,
    pub albedo: RgbaBuffer,
    pub normal: Option<RgbaBuffer>,
    pub specular: Option<RgbaBuffer>,
    pub overlay: Option<RgbaBuffer>,
}

impl BakedAtlasChunk {
    /// Canonical file stem for this atlas sheet, e.g. `"blocks_chunk_001"` or `"blocks_anim_chunk_001"`.
    pub fn file_stem(&self) -> String {
        if self.is_animated {
            format!("{}_anim_chunk_{:03}", self.category, self.category_chunk_index)
        } else {
            format!("{}_chunk_{:03}", self.category, self.category_chunk_index)
        }
    }
}

/// Complete output containing all baked atlas sheets and authoritative address table.
#[derive(Debug, Clone)]
pub struct BakedAtlas {
    pub chunks: Vec<BakedAtlasChunk>,
    pub address_map: AtlasAddressMap,
}

/// Top-level coordinator for building vanilla Minecraft atlases with PBR sync.
pub struct AtlasBuilder {
    config: AtlasBuilderConfig,
}

impl AtlasBuilder {
    pub fn new(config: AtlasBuilderConfig) -> Self {
        Self { config }
    }

    /// Bake an atlas given a ResourcePackStack and an AtlasDefinition (defaults to "blocks" category).
    pub fn build(
        &self,
        stack: &ResourcePackStack,
        definition: &AtlasDefinition,
    ) -> Result<BakedAtlas, TextureError> {
        self.build_with_category(stack, definition, "blocks")
    }

    /// Bake an atlas for a specific category (e.g. `AtlasCategory::Blocks`, `AtlasCategory::Items`).
    pub fn build_category(
        &self,
        stack: &ResourcePackStack,
        category: &mtk_resource::AtlasCategory,
    ) -> Result<BakedAtlas, TextureError> {
        let definition = stack.load_atlas_category(category);
        self.build_with_category(stack, &definition, category.as_str())
    }

    /// Bake multiple atlas categories into a single unified BakedAtlas with unified AddressMap.
    pub fn build_categories(
        &self,
        stack: &ResourcePackStack,
        definitions: &[(String, AtlasDefinition)],
    ) -> Result<BakedAtlas, TextureError> {
        let mut baked_chunks = Vec::new();
        let mut address_map = AtlasAddressMap::new();
        let mut global_chunk_counter = 0u16;
        let mut texture_id_counter = 0u32;

        for (category_name, definition) in definitions {
            // 1. Discover raw sprites
            let discovered = stack.collect_sprites_for_atlas(definition)?;
            if discovered.is_empty() && definition.sources.is_empty() {
                continue;
            }

            // 2. Decode discovered sprites in parallel
            let mut decoded = DecodedSprite::decode_batch(discovered)?;

            // 3. Process PalettedPermutations sources if any
            for source in &definition.sources {
                if let AtlasSource::PalettedPermutations { palette_key, permutations, textures } = source {
                    let key_bytes = match stack.open_texture_raw(palette_key) {
                        Some(b) => b,
                        None => continue,
                    };
                    let key_img = match RgbaBuffer::from_png_bytes(&key_bytes) {
                        Ok(img) => img,
                        Err(_) => continue,
                    };

                    for (perm_name, perm_loc) in permutations {
                        let perm_bytes = match stack.open_texture_raw(perm_loc) {
                            Some(b) => b,
                            None => continue,
                        };
                        let perm_img = match RgbaBuffer::from_png_bytes(&perm_bytes) {
                            Ok(img) => img,
                            Err(_) => continue,
                        };

                        for tex_loc in textures {
                            let companions = stack.resolve_pbr_companions(tex_loc);
                            if let Some(bytes) = companions.albedo {
                                if let Ok(base_buf) = RgbaBuffer::from_png_bytes(&bytes) {
                                    if let Ok(baked_perm) = bake_paletted_permutation(&base_buf, &key_img, &perm_img) {
                                        let sprite_path = format!("{}_{}", tex_loc.path, perm_name);
                                        let sprite_id = ResourceLocation::new(&tex_loc.namespace, sprite_path);
                                        let fw = baked_perm.width;
                                        let fh = baked_perm.height;
                                        let normal = companions.normal
                                            .and_then(|b| RgbaBuffer::from_png_bytes(&b).ok())
                                            .map(|n| n.align_companion_to_albedo(fw, fh, 1));
                                        let specular = companions.specular
                                            .and_then(|b| RgbaBuffer::from_png_bytes(&b).ok())
                                            .map(|s| s.align_companion_to_albedo(fw, fh, 1));
                                        let overlay = companions.overlay
                                            .and_then(|b| RgbaBuffer::from_png_bytes(&b).ok())
                                            .map(|o| o.align_companion_to_albedo(fw, fh, 1));
                                        decoded.push(DecodedSprite {
                                            sprite_id,
                                            albedo: baked_perm,
                                            normal,
                                            specular,
                                            overlay,
                                            frame_width: fw,
                                            frame_height: fh,
                                            frame_count: 1,
                                            metadata: None,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if decoded.is_empty() {
                continue;
            }

            // 4. Separate static and anim sprites
            let mut static_sprites = Vec::new();
            let mut anim_sprites = Vec::new();

            for sp in decoded {
                let static_sp = if sp.frame_count > 1 {
                    DecodedSprite {
                        sprite_id: sp.sprite_id.clone(),
                        albedo: sp.albedo.crop(0, 0, sp.frame_width, sp.frame_height),
                        normal: sp.normal.as_ref().map(|n| {
                            let h = n.height.min(sp.frame_height);
                            n.crop(0, 0, n.width.min(sp.frame_width), h)
                        }),
                        specular: sp.specular.as_ref().map(|s| {
                            let h = s.height.min(sp.frame_height);
                            s.crop(0, 0, s.width.min(sp.frame_width), h)
                        }),
                        overlay: sp.overlay.as_ref().map(|o| {
                            let h = o.height.min(sp.frame_height);
                            o.crop(0, 0, o.width.min(sp.frame_width), h)
                        }),
                        frame_width: sp.frame_width,
                        frame_height: sp.frame_height,
                        frame_count: 1,
                        metadata: None,
                    }
                } else {
                    sp.clone()
                };
                static_sprites.push(static_sp);

                if sp.frame_count > 1 {
                    anim_sprites.push(sp);
                }
            }

            // 5. Build Static Atlas Chunks
            if !static_sprites.is_empty() {
                let mut stitcher = Stitcher::new(self.config.max_width, self.config.max_height, self.config.mip_level);
                let mut sprite_map: HashMap<ResourceLocation, DecodedSprite> = HashMap::new();

                for sprite in static_sprites {
                    let fw = sprite.frame_width + self.config.padding * 2;
                    let fh = sprite.frame_height + self.config.padding * 2;
                    stitcher.register_sprite(sprite.sprite_id.clone(), fw, fh, sprite.sprite_id.as_string());
                    sprite_map.insert(sprite.sprite_id.clone(), sprite);
                }

                let stitched = stitcher.stitch()?;

                for (idx, chunk) in stitched.chunks.into_iter().enumerate() {
                    let chunk_id = global_chunk_counter;
                    global_chunk_counter += 1;
                    let category_chunk_index = idx + 1;
                    let mut has_normal = false;
                    let mut has_specular = false;
                    let mut has_overlay = false;

                    for slot in &chunk.slots {
                        if let Some(sp) = sprite_map.get(&slot.entry) {
                            if sp.normal.is_some() { has_normal = true; }
                            if sp.specular.is_some() { has_specular = true; }
                            if sp.overlay.is_some() { has_overlay = true; }
                        }
                    }

                    let mut albedo_buf = RgbaBuffer::new(chunk.width, chunk.height);
                    let mut normal_buf = if has_normal {
                        Some(RgbaBuffer::solid(chunk.width, chunk.height, 128, 128, 255, 255))
                    } else {
                        None
                    };
                    let mut specular_buf = if has_specular {
                        Some(RgbaBuffer::new(chunk.width, chunk.height))
                    } else {
                        None
                    };
                    let mut overlay_buf = if has_overlay {
                        Some(RgbaBuffer::new(chunk.width, chunk.height))
                    } else {
                        None
                    };

                    for slot in chunk.slots {
                        if let Some(sp) = sprite_map.get(&slot.entry) {
                            let inner_x = slot.x + self.config.padding;
                            let inner_y = slot.y + self.config.padding;
                            let fw = sp.frame_width;
                            let fh = sp.frame_height;

                            albedo_buf.blit(&sp.albedo, 0, 0, inner_x, inner_y, fw, fh);
                            if self.config.padding > 0 {
                                apply_edge_clamping_padding(&mut albedo_buf, inner_x, inner_y, fw, fh, self.config.padding);
                            }

                            let slot_has_normal = sp.normal.is_some();
                            if let (Some(ref mut n_buf), Some(ref norm_src)) = (&mut normal_buf, &sp.normal) {
                                n_buf.blit(norm_src, 0, 0, inner_x, inner_y, fw, fh);
                                if self.config.padding > 0 {
                                    apply_edge_clamping_padding(n_buf, inner_x, inner_y, fw, fh, self.config.padding);
                                }
                            }

                            let slot_has_specular = sp.specular.is_some();
                            if let (Some(ref mut s_buf), Some(ref spec_src)) = (&mut specular_buf, &sp.specular) {
                                s_buf.blit(spec_src, 0, 0, inner_x, inner_y, fw, fh);
                                if self.config.padding > 0 {
                                    apply_edge_clamping_padding(s_buf, inner_x, inner_y, fw, fh, self.config.padding);
                                }
                            }

                            let slot_has_overlay = sp.overlay.is_some();
                            if let (Some(ref mut o_buf), Some(ref over_src)) = (&mut overlay_buf, &sp.overlay) {
                                o_buf.blit(over_src, 0, 0, inner_x, inner_y, fw, fh);
                                if self.config.padding > 0 {
                                    apply_edge_clamping_padding(o_buf, inner_x, inner_y, fw, fh, self.config.padding);
                                }
                            }

                            let u_min = (inner_x as f32) / (chunk.width as f32);
                            let u_max = ((inner_x + fw) as f32) / (chunk.width as f32);
                            let v_min = 1.0 - ((inner_y + fh) as f32) / (chunk.height as f32);
                            let v_max = 1.0 - (inner_y as f32) / (chunk.height as f32);

                            address_map.sprites.insert(
                                sp.sprite_id.clone(),
                                AtlasSpriteLocation {
                                    chunk_id,
                                    category: category_name.to_string(),
                                    is_animated: false,
                                    sprite_kind: SpriteKind::StaticAtlas,
                                    texture_id: texture_id_counter,
                                    uv_bounds: [u_min, v_min, u_max, v_max],
                                    frame_0_uv_bounds: [u_min, v_min, u_max, v_max],
                                    local_uv_bounds: [0.0, 0.0, 1.0, 1.0],
                                    frame_uv_step: [0.0, 0.0],
                                    pixel_rect: [inner_x, inner_y, fw, fh],
                                    strip_pixel_rect: [inner_x, inner_y, fw, fh],
                                    frame_size: [fw, fh],
                                    frame_count: 1,
                                    animation: None,
                                    has_normal: slot_has_normal,
                                    has_specular: slot_has_specular,
                                    has_overlay: slot_has_overlay,
                                },
                            );

                            texture_id_counter += 1;
                        }
                    }

                    address_map.chunks.push(AtlasChunkMeta {
                        chunk_id,
                        category: category_name.to_string(),
                        is_animated: false,
                        category_chunk_index,
                        width: chunk.width,
                        height: chunk.height,
                        has_normal,
                        has_specular,
                        has_overlay,
                    });

                    baked_chunks.push(BakedAtlasChunk {
                        chunk_id,
                        category: category_name.to_string(),
                        is_animated: false,
                        category_chunk_index,
                        width: chunk.width,
                        height: chunk.height,
                        albedo: albedo_buf,
                        normal: normal_buf,
                        specular: specular_buf,
                        overlay: overlay_buf,
                    });
                }
            }

            // 6. Build Dedicated Animated Atlas Chunks (Full Strips + PBR Tiling)
            if !anim_sprites.is_empty() {
                let mut anim_stitcher = Stitcher::new(self.config.max_width, self.config.max_height, self.config.mip_level);
                let mut anim_sprite_map: HashMap<ResourceLocation, DecodedSprite> = HashMap::new();

                for sprite in anim_sprites {
                    let strip_w = sprite.frame_width + self.config.padding * 2;
                    let strip_h = sprite.albedo.height + self.config.padding * 2;
                    anim_stitcher.register_sprite(sprite.sprite_id.clone(), strip_w, strip_h, sprite.sprite_id.as_string());
                    anim_sprite_map.insert(sprite.sprite_id.clone(), sprite);
                }

                let anim_stitched = anim_stitcher.stitch()?;

                for (idx, chunk) in anim_stitched.chunks.into_iter().enumerate() {
                    let chunk_id = global_chunk_counter;
                    global_chunk_counter += 1;
                    let category_chunk_index = idx + 1;
                    let mut has_normal = false;
                    let mut has_specular = false;
                    let mut has_overlay = false;

                    for slot in &chunk.slots {
                        if let Some(sp) = anim_sprite_map.get(&slot.entry) {
                            if sp.normal.is_some() { has_normal = true; }
                            if sp.specular.is_some() { has_specular = true; }
                            if sp.overlay.is_some() { has_overlay = true; }
                        }
                    }

                    let mut albedo_buf = RgbaBuffer::new(chunk.width, chunk.height);
                    let mut normal_buf = if has_normal {
                        Some(RgbaBuffer::solid(chunk.width, chunk.height, 128, 128, 255, 255))
                    } else {
                        None
                    };
                    let mut specular_buf = if has_specular {
                        Some(RgbaBuffer::new(chunk.width, chunk.height))
                    } else {
                        None
                    };
                    let mut overlay_buf = if has_overlay {
                        Some(RgbaBuffer::new(chunk.width, chunk.height))
                    } else {
                        None
                    };

                    for slot in chunk.slots {
                        if let Some(sp) = anim_sprite_map.get(&slot.entry) {
                            let inner_x = slot.x + self.config.padding;
                            let inner_y = slot.y + self.config.padding;
                            let fw = sp.frame_width;
                            let strip_h = sp.albedo.height;

                            albedo_buf.blit(&sp.albedo, 0, 0, inner_x, inner_y, fw, strip_h);
                            if self.config.padding > 0 {
                                apply_edge_clamping_padding(&mut albedo_buf, inner_x, inner_y, fw, strip_h, self.config.padding);
                            }

                            let slot_has_normal = sp.normal.is_some();
                            if let (Some(ref mut n_buf), Some(ref norm_src)) = (&mut normal_buf, &sp.normal) {
                                n_buf.blit(norm_src, 0, 0, inner_x, inner_y, fw, strip_h);
                                if self.config.padding > 0 {
                                    apply_edge_clamping_padding(n_buf, inner_x, inner_y, fw, strip_h, self.config.padding);
                                }
                            }

                            let slot_has_specular = sp.specular.is_some();
                            if let (Some(ref mut s_buf), Some(ref spec_src)) = (&mut specular_buf, &sp.specular) {
                                s_buf.blit(spec_src, 0, 0, inner_x, inner_y, fw, strip_h);
                                if self.config.padding > 0 {
                                    apply_edge_clamping_padding(s_buf, inner_x, inner_y, fw, strip_h, self.config.padding);
                                }
                            }

                            let slot_has_overlay = sp.overlay.is_some();
                            if let (Some(ref mut o_buf), Some(ref over_src)) = (&mut overlay_buf, &sp.overlay) {
                                o_buf.blit(over_src, 0, 0, inner_x, inner_y, fw, strip_h);
                                if self.config.padding > 0 {
                                    apply_edge_clamping_padding(o_buf, inner_x, inner_y, fw, strip_h, self.config.padding);
                                }
                            }

                            let u_min = (inner_x as f32) / (chunk.width as f32);
                            let u_max = ((inner_x + fw) as f32) / (chunk.width as f32);
                            let v_min = 1.0 - ((inner_y + strip_h) as f32) / (chunk.height as f32);
                            let v_max = 1.0 - (inner_y as f32) / (chunk.height as f32);

                            let v_frame_step = (sp.frame_height as f32) / (chunk.height as f32);
                            let frame_0_v_min = v_max - v_frame_step;

                            address_map.anim_sprites.insert(
                                sp.sprite_id.clone(),
                                AtlasSpriteLocation {
                                    chunk_id,
                                    category: category_name.to_string(),
                                    is_animated: true,
                                    sprite_kind: SpriteKind::AnimatedAtlas,
                                    texture_id: texture_id_counter,
                                    uv_bounds: [u_min, v_min, u_max, v_max],
                                    frame_0_uv_bounds: [u_min, frame_0_v_min, u_max, v_max],
                                    local_uv_bounds: [0.0, 0.0, 1.0, 1.0],
                                    frame_uv_step: [0.0, v_frame_step],
                                    pixel_rect: [inner_x, inner_y, fw, sp.frame_height],
                                    strip_pixel_rect: [inner_x, inner_y, fw, strip_h],
                                    frame_size: [fw, sp.frame_height],
                                    frame_count: sp.frame_count,
                                    animation: sp.metadata.clone(),
                                    has_normal: slot_has_normal,
                                    has_specular: slot_has_specular,
                                    has_overlay: slot_has_overlay,
                                },
                            );

                            texture_id_counter += 1;
                        }
                    }

                    address_map.chunks.push(AtlasChunkMeta {
                        chunk_id,
                        category: category_name.to_string(),
                        is_animated: true,
                        category_chunk_index,
                        width: chunk.width,
                        height: chunk.height,
                        has_normal,
                        has_specular,
                        has_overlay,
                    });

                    baked_chunks.push(BakedAtlasChunk {
                        chunk_id,
                        category: category_name.to_string(),
                        is_animated: true,
                        category_chunk_index,
                        width: chunk.width,
                        height: chunk.height,
                        albedo: albedo_buf,
                        normal: normal_buf,
                        specular: specular_buf,
                        overlay: overlay_buf,
                    });
                }
            }
        }

        Ok(BakedAtlas {
            chunks: baked_chunks,
            address_map,
        })
    }

    /// Bake an atlas with an explicit category name string.
    pub fn build_with_category(
        &self,
        stack: &ResourcePackStack,
        definition: &AtlasDefinition,
        category_name: &str,
    ) -> Result<BakedAtlas, TextureError> {
        self.build_categories(stack, &[(category_name.to_string(), definition.clone())])
    }

    /// Build directly from pre-decoded sprites (defaults to "blocks" category).
    pub fn build_from_sprites(&self, sprites: Vec<DecodedSprite>) -> Result<BakedAtlas, TextureError> {
        self.build_from_sprites_with_category(sprites, "blocks")
    }

    /// Build directly from pre-decoded sprites with an explicit category name,
    /// separating static and animated sprites into isolated chunks.
    pub fn build_from_sprites_with_category(
        &self,
        sprites: Vec<DecodedSprite>,
        category_name: &str,
    ) -> Result<BakedAtlas, TextureError> {
        let mut static_sprites = Vec::new();
        let mut anim_sprites = Vec::new();

        for sp in sprites {
            let static_sp = if sp.frame_count > 1 {
                DecodedSprite {
                    sprite_id: sp.sprite_id.clone(),
                    albedo: sp.albedo.crop(0, 0, sp.frame_width, sp.frame_height),
                    normal: sp.normal.as_ref().map(|n| {
                        let h = n.height.min(sp.frame_height);
                        n.crop(0, 0, n.width.min(sp.frame_width), h)
                    }),
                    specular: sp.specular.as_ref().map(|s| {
                        let h = s.height.min(sp.frame_height);
                        s.crop(0, 0, s.width.min(sp.frame_width), h)
                    }),
                    overlay: sp.overlay.as_ref().map(|o| {
                        let h = o.height.min(sp.frame_height);
                        o.crop(0, 0, o.width.min(sp.frame_width), h)
                    }),
                    frame_width: sp.frame_width,
                    frame_height: sp.frame_height,
                    frame_count: 1,
                    metadata: None,
                }
            } else {
                sp.clone()
            };
            static_sprites.push(static_sp);

            if sp.frame_count > 1 {
                anim_sprites.push(sp);
            }
        }

        let mut baked_chunks = Vec::new();
        let mut address_map = AtlasAddressMap::new();
        let mut global_chunk_counter = 0u16;
        let mut texture_id_counter = 0u32;

        if !static_sprites.is_empty() {
            let mut stitcher = Stitcher::new(self.config.max_width, self.config.max_height, self.config.mip_level);
            let mut sprite_map: HashMap<ResourceLocation, DecodedSprite> = HashMap::new();

            for sprite in static_sprites {
                let fw = sprite.frame_width + self.config.padding * 2;
                let fh = sprite.frame_height + self.config.padding * 2;
                stitcher.register_sprite(sprite.sprite_id.clone(), fw, fh, sprite.sprite_id.as_string());
                sprite_map.insert(sprite.sprite_id.clone(), sprite);
            }

            let stitched = stitcher.stitch()?;

            for (idx, chunk) in stitched.chunks.into_iter().enumerate() {
                let chunk_id = global_chunk_counter;
                global_chunk_counter += 1;
                let category_chunk_index = idx + 1;
                let mut has_normal = false;
                let mut has_specular = false;
                let mut has_overlay = false;

                for slot in &chunk.slots {
                    if let Some(sp) = sprite_map.get(&slot.entry) {
                        if sp.normal.is_some() { has_normal = true; }
                        if sp.specular.is_some() { has_specular = true; }
                        if sp.overlay.is_some() { has_overlay = true; }
                    }
                }

                let mut albedo_buf = RgbaBuffer::new(chunk.width, chunk.height);
                let mut normal_buf = if has_normal {
                    Some(RgbaBuffer::solid(chunk.width, chunk.height, 128, 128, 255, 255))
                } else {
                    None
                };
                let mut specular_buf = if has_specular {
                    Some(RgbaBuffer::new(chunk.width, chunk.height))
                } else {
                    None
                };
                let mut overlay_buf = if has_overlay {
                    Some(RgbaBuffer::new(chunk.width, chunk.height))
                } else {
                    None
                };

                for slot in chunk.slots {
                    if let Some(sp) = sprite_map.get(&slot.entry) {
                        let inner_x = slot.x + self.config.padding;
                        let inner_y = slot.y + self.config.padding;
                        let fw = sp.frame_width;
                        let fh = sp.frame_height;

                        albedo_buf.blit(&sp.albedo, 0, 0, inner_x, inner_y, fw, fh);
                        if self.config.padding > 0 {
                            apply_edge_clamping_padding(&mut albedo_buf, inner_x, inner_y, fw, fh, self.config.padding);
                        }

                        let slot_has_normal = sp.normal.is_some();
                        if let (Some(ref mut n_buf), Some(ref norm_src)) = (&mut normal_buf, &sp.normal) {
                            n_buf.blit(norm_src, 0, 0, inner_x, inner_y, fw, fh);
                            if self.config.padding > 0 {
                                apply_edge_clamping_padding(n_buf, inner_x, inner_y, fw, fh, self.config.padding);
                            }
                        }

                        let slot_has_specular = sp.specular.is_some();
                        if let (Some(ref mut s_buf), Some(ref spec_src)) = (&mut specular_buf, &sp.specular) {
                            s_buf.blit(spec_src, 0, 0, inner_x, inner_y, fw, fh);
                            if self.config.padding > 0 {
                                apply_edge_clamping_padding(s_buf, inner_x, inner_y, fw, fh, self.config.padding);
                            }
                        }

                        let slot_has_overlay = sp.overlay.is_some();
                        if let (Some(ref mut o_buf), Some(ref over_src)) = (&mut overlay_buf, &sp.overlay) {
                            o_buf.blit(over_src, 0, 0, inner_x, inner_y, fw, fh);
                            if self.config.padding > 0 {
                                apply_edge_clamping_padding(o_buf, inner_x, inner_y, fw, fh, self.config.padding);
                            }
                        }

                        let u_min = (inner_x as f32) / (chunk.width as f32);
                        let u_max = ((inner_x + fw) as f32) / (chunk.width as f32);
                        let v_min = 1.0 - ((inner_y + fh) as f32) / (chunk.height as f32);
                        let v_max = 1.0 - (inner_y as f32) / (chunk.height as f32);

                        address_map.sprites.insert(
                            sp.sprite_id.clone(),
                            AtlasSpriteLocation {
                                chunk_id,
                                category: category_name.to_string(),
                                is_animated: false,
                                sprite_kind: SpriteKind::StaticAtlas,
                                texture_id: texture_id_counter,
                                uv_bounds: [u_min, v_min, u_max, v_max],
                                frame_0_uv_bounds: [u_min, v_min, u_max, v_max],
                                local_uv_bounds: [0.0, 0.0, 1.0, 1.0],
                                frame_uv_step: [0.0, 0.0],
                                pixel_rect: [inner_x, inner_y, fw, fh],
                                strip_pixel_rect: [inner_x, inner_y, fw, fh],
                                frame_size: [fw, fh],
                                frame_count: 1,
                                animation: None,
                                has_normal: slot_has_normal,
                                has_specular: slot_has_specular,
                                has_overlay: slot_has_overlay,
                            },
                        );

                        texture_id_counter += 1;
                    }
                }

                address_map.chunks.push(AtlasChunkMeta {
                    chunk_id,
                    category: category_name.to_string(),
                    is_animated: false,
                    category_chunk_index,
                    width: chunk.width,
                    height: chunk.height,
                    has_normal,
                    has_specular,
                    has_overlay,
                });

                baked_chunks.push(BakedAtlasChunk {
                    chunk_id,
                    category: category_name.to_string(),
                    is_animated: false,
                    category_chunk_index,
                    width: chunk.width,
                    height: chunk.height,
                    albedo: albedo_buf,
                    normal: normal_buf,
                    specular: specular_buf,
                    overlay: overlay_buf,
                });
            }
        }

        if !anim_sprites.is_empty() {
            let mut anim_stitcher = Stitcher::new(self.config.max_width, self.config.max_height, self.config.mip_level);
            let mut anim_sprite_map: HashMap<ResourceLocation, DecodedSprite> = HashMap::new();

            for sprite in anim_sprites {
                let strip_w = sprite.frame_width + self.config.padding * 2;
                let strip_h = sprite.albedo.height + self.config.padding * 2;
                anim_stitcher.register_sprite(sprite.sprite_id.clone(), strip_w, strip_h, sprite.sprite_id.as_string());
                anim_sprite_map.insert(sprite.sprite_id.clone(), sprite);
            }

            let anim_stitched = anim_stitcher.stitch()?;

            for (idx, chunk) in anim_stitched.chunks.into_iter().enumerate() {
                let chunk_id = global_chunk_counter;
                global_chunk_counter += 1;
                let category_chunk_index = idx + 1;
                let mut has_normal = false;
                let mut has_specular = false;
                let mut has_overlay = false;

                for slot in &chunk.slots {
                    if let Some(sp) = anim_sprite_map.get(&slot.entry) {
                        if sp.normal.is_some() { has_normal = true; }
                        if sp.specular.is_some() { has_specular = true; }
                        if sp.overlay.is_some() { has_overlay = true; }
                    }
                }

                let mut albedo_buf = RgbaBuffer::new(chunk.width, chunk.height);
                let mut normal_buf = if has_normal {
                    Some(RgbaBuffer::solid(chunk.width, chunk.height, 128, 128, 255, 255))
                } else {
                    None
                };
                let mut specular_buf = if has_specular {
                    Some(RgbaBuffer::new(chunk.width, chunk.height))
                } else {
                    None
                };
                let mut overlay_buf = if has_overlay {
                    Some(RgbaBuffer::new(chunk.width, chunk.height))
                } else {
                    None
                };

                for slot in chunk.slots {
                    if let Some(sp) = anim_sprite_map.get(&slot.entry) {
                        let inner_x = slot.x + self.config.padding;
                        let inner_y = slot.y + self.config.padding;
                        let fw = sp.frame_width;
                        let strip_h = sp.albedo.height;

                        albedo_buf.blit(&sp.albedo, 0, 0, inner_x, inner_y, fw, strip_h);
                        if self.config.padding > 0 {
                            apply_edge_clamping_padding(&mut albedo_buf, inner_x, inner_y, fw, strip_h, self.config.padding);
                        }

                        let slot_has_normal = sp.normal.is_some();
                        if let (Some(ref mut n_buf), Some(ref norm_src)) = (&mut normal_buf, &sp.normal) {
                            n_buf.blit(norm_src, 0, 0, inner_x, inner_y, fw, strip_h);
                            if self.config.padding > 0 {
                                apply_edge_clamping_padding(n_buf, inner_x, inner_y, fw, strip_h, self.config.padding);
                            }
                        }

                        let slot_has_specular = sp.specular.is_some();
                        if let (Some(ref mut s_buf), Some(ref spec_src)) = (&mut specular_buf, &sp.specular) {
                            s_buf.blit(spec_src, 0, 0, inner_x, inner_y, fw, strip_h);
                            if self.config.padding > 0 {
                                apply_edge_clamping_padding(s_buf, inner_x, inner_y, fw, strip_h, self.config.padding);
                            }
                        }

                        let slot_has_overlay = sp.overlay.is_some();
                        if let (Some(ref mut o_buf), Some(ref over_src)) = (&mut overlay_buf, &sp.overlay) {
                            o_buf.blit(over_src, 0, 0, inner_x, inner_y, fw, strip_h);
                            if self.config.padding > 0 {
                                apply_edge_clamping_padding(o_buf, inner_x, inner_y, fw, strip_h, self.config.padding);
                            }
                        }

                        let u_min = (inner_x as f32) / (chunk.width as f32);
                        let u_max = ((inner_x + fw) as f32) / (chunk.width as f32);
                        let v_min = 1.0 - ((inner_y + strip_h) as f32) / (chunk.height as f32);
                        let v_max = 1.0 - (inner_y as f32) / (chunk.height as f32);

                        let v_frame_step = (sp.frame_height as f32) / (chunk.height as f32);
                        let frame_0_v_min = v_max - v_frame_step;

                        address_map.anim_sprites.insert(
                            sp.sprite_id.clone(),
                            AtlasSpriteLocation {
                                chunk_id,
                                category: category_name.to_string(),
                                is_animated: true,
                                sprite_kind: SpriteKind::AnimatedAtlas,
                                texture_id: texture_id_counter,
                                uv_bounds: [u_min, v_min, u_max, v_max],
                                frame_0_uv_bounds: [u_min, frame_0_v_min, u_max, v_max],
                                local_uv_bounds: [0.0, 0.0, 1.0, 1.0],
                                frame_uv_step: [0.0, v_frame_step],
                                pixel_rect: [inner_x, inner_y, fw, sp.frame_height],
                                strip_pixel_rect: [inner_x, inner_y, fw, strip_h],
                                frame_size: [fw, sp.frame_height],
                                frame_count: sp.frame_count,
                                animation: sp.metadata.clone(),
                                has_normal: slot_has_normal,
                                has_specular: slot_has_specular,
                                has_overlay: slot_has_overlay,
                            },
                        );

                        texture_id_counter += 1;
                    }
                }

                address_map.chunks.push(AtlasChunkMeta {
                    chunk_id,
                    category: category_name.to_string(),
                    is_animated: true,
                    category_chunk_index,
                    width: chunk.width,
                    height: chunk.height,
                    has_normal,
                    has_specular,
                    has_overlay,
                });

                baked_chunks.push(BakedAtlasChunk {
                    chunk_id,
                    category: category_name.to_string(),
                    is_animated: true,
                    category_chunk_index,
                    width: chunk.width,
                    height: chunk.height,
                    albedo: albedo_buf,
                    normal: normal_buf,
                    specular: specular_buf,
                    overlay: overlay_buf,
                });
            }
        }

        Ok(BakedAtlas {
            chunks: baked_chunks,
            address_map,
        })
    }
}


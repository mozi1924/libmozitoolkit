use std::collections::HashMap;
use mtk_resource::{AtlasDefinition, AtlasSource, ResourceLocation, ResourcePackStack};
use crate::atlas::address_map::{AtlasAddressMap, AtlasChunkMeta, AtlasSpriteLocation};
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
    pub width: u32,
    pub height: u32,
    pub albedo: RgbaBuffer,
    pub normal: Option<RgbaBuffer>,
    pub specular: Option<RgbaBuffer>,
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

    /// Bake an atlas given a ResourcePackStack and an AtlasDefinition (e.g. `blocks.json`).
    pub fn build(
        &self,
        stack: &ResourcePackStack,
        definition: &AtlasDefinition,
    ) -> Result<BakedAtlas, TextureError> {
        // 1. Discover raw sprites
        let discovered = stack.collect_sprites_for_atlas(definition)?;

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
                        if let Some(bytes) = stack.open_texture_raw(tex_loc) {
                            if let Ok(base_buf) = RgbaBuffer::from_png_bytes(&bytes) {
                                if let Ok(baked_perm) = bake_paletted_permutation(&base_buf, &key_img, &perm_img) {
                                    let sprite_path = format!("{}_{}", tex_loc.path, perm_name);
                                    let sprite_id = ResourceLocation::new(&tex_loc.namespace, sprite_path);
                                    let fw = baked_perm.width;
                                    let fh = baked_perm.height;
                                    decoded.push(DecodedSprite {
                                        sprite_id,
                                        albedo: baked_perm,
                                        normal: None,
                                        specular: None,
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

        self.build_from_sprites(decoded)
    }

    /// Build directly from pre-decoded sprites.
    pub fn build_from_sprites(&self, sprites: Vec<DecodedSprite>) -> Result<BakedAtlas, TextureError> {
        let mut stitcher = Stitcher::new(self.config.max_width, self.config.max_height, self.config.mip_level);

        // Map sprites by canonical ID for fast retrieval during rasterization
        let mut sprite_map: HashMap<ResourceLocation, DecodedSprite> = HashMap::new();

        for sprite in sprites {
            let fw = sprite.frame_width + self.config.padding * 2;
            let fh = sprite.frame_height + self.config.padding * 2;
            stitcher.register_sprite(sprite.sprite_id.clone(), fw, fh, sprite.sprite_id.as_string());
            sprite_map.insert(sprite.sprite_id.clone(), sprite);
        }

        // Run vanilla stitcher layout
        let stitched = stitcher.stitch()?;

        let mut baked_chunks = Vec::new();
        let mut address_map = AtlasAddressMap::new();
        let mut texture_id_counter = 0u32;

        for chunk in stitched.chunks {
            let mut has_normal = false;
            let mut has_specular = false;

            // Check if this chunk needs companion sheets
            for slot in &chunk.slots {
                if let Some(sp) = sprite_map.get(&slot.entry) {
                    if sp.normal.is_some() {
                        has_normal = true;
                    }
                    if sp.specular.is_some() {
                        has_specular = true;
                    }
                }
            }

            let mut albedo_buf = RgbaBuffer::new(chunk.width, chunk.height);
            let mut normal_buf = if has_normal {
                // Default flat normal in tangent space: (128, 128, 255, 255)
                Some(RgbaBuffer::solid(chunk.width, chunk.height, 128, 128, 255, 255))
            } else {
                None
            };
            let mut specular_buf = if has_specular {
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

                    // 1. Blit Frame 0 Albedo
                    albedo_buf.blit(&sp.albedo, 0, 0, inner_x, inner_y, fw, fh);
                    if self.config.padding > 0 {
                        apply_edge_clamping_padding(&mut albedo_buf, inner_x, inner_y, fw, fh, self.config.padding);
                    }

                    // 2. Blit Normal companion
                    let slot_has_normal = sp.normal.is_some();
                    if let (Some(ref mut n_buf), Some(ref norm_src)) = (&mut normal_buf, &sp.normal) {
                        n_buf.blit(norm_src, 0, 0, inner_x, inner_y, fw, fh);
                        if self.config.padding > 0 {
                            apply_edge_clamping_padding(n_buf, inner_x, inner_y, fw, fh, self.config.padding);
                        }
                    }

                    // 3. Blit Specular companion
                    let slot_has_specular = sp.specular.is_some();
                    if let (Some(ref mut s_buf), Some(ref spec_src)) = (&mut specular_buf, &sp.specular) {
                        s_buf.blit(spec_src, 0, 0, inner_x, inner_y, fw, fh);
                        if self.config.padding > 0 {
                            apply_edge_clamping_padding(s_buf, inner_x, inner_y, fw, fh, self.config.padding);
                        }
                    }

                    // Compute normalized UV bounds [u_min, v_min, u_max, v_max]
                    let u_min = (inner_x as f32) / (chunk.width as f32);
                    let v_min = (inner_y as f32) / (chunk.height as f32);
                    let u_max = ((inner_x + fw) as f32) / (chunk.width as f32);
                    let v_max = ((inner_y + fh) as f32) / (chunk.height as f32);

                    address_map.sprites.insert(
                        sp.sprite_id.clone(),
                        AtlasSpriteLocation {
                            chunk_id: chunk.chunk_id,
                            texture_id: texture_id_counter,
                            uv_bounds: [u_min, v_min, u_max, v_max],
                            pixel_rect: [inner_x, inner_y, fw, fh],
                            frame_size: [fw, fh],
                            frame_count: sp.frame_count,
                            animation: sp.metadata.clone(),
                            has_normal: slot_has_normal,
                            has_specular: slot_has_specular,
                        },
                    );

                    texture_id_counter += 1;
                }
            }

            address_map.chunks.push(AtlasChunkMeta {
                chunk_id: chunk.chunk_id,
                width: chunk.width,
                height: chunk.height,
                has_normal,
                has_specular,
            });

            baked_chunks.push(BakedAtlasChunk {
                chunk_id: chunk.chunk_id,
                width: chunk.width,
                height: chunk.height,
                albedo: albedo_buf,
                normal: normal_buf,
                specular: specular_buf,
            });
        }

        Ok(BakedAtlas {
            chunks: baked_chunks,
            address_map,
        })
    }
}

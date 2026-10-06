use super::types::BiomeResolver;

impl BiomeResolver {
    /// Seed authoritative vanilla texture tint categories and overlay pairings (SSOT).
    pub fn seed_defaults(&mut self) {
        self.overlay_pairs.insert(
            "grass_block_side".to_string(),
            "grass_block_side_overlay".to_string(),
        );
        self.overlay_pairs.insert(
            "grass_block_snow".to_string(),
            "grass_block_side_overlay".to_string(),
        );
        self.overlay_pairs
            .insert("grass_side".to_string(), "grass_side_overlay".to_string());
        self.overlay_pairs.insert(
            "grass_side_snowed".to_string(),
            "grass_side_overlay".to_string(),
        );

        // 1. Grass colormap (grass.png)
        for t in &[
            "grass_block_top",
            "grass_block_side_overlay",
            "grass_side_overlay",
            "short_grass",
            "grass",
            "tall_grass_top",
            "tall_grass_bottom",
            "fern",
            "potted_fern",
            "large_fern_top",
            "large_fern_bottom",
            "bush", // Vanilla 1.21.4 bush uses grass colormap
            "pink_petals_stem",
            "wildflowers_stem",
            "sugar_cane",
        ] {
            self.texture_tint_categories
                .insert(t.to_string(), "grass".to_string());
        }

        // 2. Foliage colormap (foliage.png)
        for t in &[
            "oak_leaves",
            "jungle_leaves",
            "acacia_leaves",
            "dark_oak_leaves",
            "mangrove_leaves",
            "vine",
            "bamboo_large_leaves",
            "bamboo_small_leaves",
        ] {
            self.texture_tint_categories
                .insert(t.to_string(), "foliage".to_string());
        }

        // 3. Dry foliage colormap (dry_foliage.png)
        for t in &[
            "leaf_litter",
            "pale_hanging_moss",
            "pale_hanging_moss_tip",
            "short_dry_grass",
            "tall_dry_grass",
        ] {
            self.texture_tint_categories
                .insert(t.to_string(), "dry_foliage".to_string());
        }

        // 4. Explicit non-tinted blocks (retain their natural textures)
        for t in &[
            "dead_bush",
            "potted_dead_bush",
            "firefly_bush",
            "azalea_leaves",
            "flowering_azalea_leaves",
            "potted_azalea_bush_plant",
            "potted_flowering_azalea_bush_plant",
            "cherry_leaves",
            "pale_oak_leaves",
            "dirt",
            "coarse_dirt",
            "rooted_dirt",
            "seagrass",
            "tall_seagrass",
            "kelp",
            "kelp_plant",
        ] {
            self.texture_tint_categories
                .insert(t.to_string(), "none".to_string());
        }

        // 5. Hardcoded non-colormap colors
        for t in &[
            "spruce_leaves",
            "birch_leaves",
            "lily_pad",
            "attached_melon_stem",
            "attached_pumpkin_stem",
            "melon_stem",
            "pumpkin_stem",
            "redstone_wire",
        ] {
            self.texture_tint_categories
                .insert(t.to_string(), "hardcoded".to_string());
        }
    }
}

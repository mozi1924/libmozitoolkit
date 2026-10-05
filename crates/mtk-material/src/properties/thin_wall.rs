//! # Foliage and Vegetation Thin Wall Evaluation

pub const THIN_WALL_KEYWORDS: &[&str] = &[
    "leaves", "sapling", "crop", "flower", "tulip", "orchid", "daisy",
    "dandelion", "poppy", "allium", "bluet", "rose", "peony", "lilac",
    "sunflower", "petals", "wildflowers", "vine", "vines", "grass", "fern",
    "wheat", "carrot", "potatoes", "potato", "beetroot", "sprout", "roots",
    "lichen", "moss", "fungus", "mushroom", "stem", "propagule", "azalea",
    "dripleaf", "lily_pad", "seagrass", "kelp", "coral", "coral_fan",
    "sugar_cane", "bamboo", "sweet_berry", "bush", "eyeblossom",
];

pub const THIN_WALL_EXACT: &[&str] = &[
    "oak_leaves", "spruce_leaves", "birch_leaves", "jungle_leaves",
    "acacia_leaves", "dark_oak_leaves", "pale_oak_leaves",
    "mangrove_leaves", "cherry_leaves", "azalea_leaves", "flowering_azalea_leaves",
    "wheat", "carrots", "potatoes", "beetroots",
    "melon_stem", "attached_melon_stem", "pumpkin_stem", "attached_pumpkin_stem",
    "torchflower_crop", "pitcher_crop", "sweet_berry_bush", "cocoa",
    "dandelion", "poppy", "blue_orchid", "allium", "azure_bluet",
    "red_tulip", "orange_tulip", "white_tulip", "pink_tulip",
    "oxeye_daisy", "cornflower", "lily_of_the_valley", "wither_rose",
    "torchflower", "sunflower", "lilac", "rose_bush", "peony",
    "pitcher_plant", "pink_petals", "wildflowers", "cactus_flower",
    "spore_blossom", "open_eyeblossom", "closed_eyeblossom", "firefly_bush",
    "oak_sapling", "spruce_sapling", "birch_sapling", "jungle_sapling",
    "acacia_sapling", "dark_oak_sapling", "pale_oak_sapling",
    "cherry_sapling", "azalea", "flowering_azalea", "mangrove_propagule",
    "vine", "weeping_vines", "weeping_vines_plant",
    "twisting_vines", "twisting_vines_plant",
    "cave_vines", "cave_vines_plant", "glow_lichen", "hanging_roots",
    "short_grass", "tall_grass", "grass", "fern", "large_fern",
    "nether_sprouts", "crimson_roots", "warped_roots",
    "crimson_fungus", "warped_fungus", "brown_mushroom", "red_mushroom",
    "lily_pad", "seagrass", "tall_seagrass", "kelp", "kelp_plant",
    "sugar_cane", "bamboo", "big_dripleaf", "big_dripleaf_stem", "small_dripleaf",
    "tube_coral", "brain_coral", "bubble_coral", "fire_coral", "horn_coral",
    "tube_coral_fan", "brain_coral_fan", "bubble_coral_fan", "fire_coral_fan", "horn_coral_fan",
    "tube_coral_wall_fan", "brain_coral_wall_fan", "bubble_coral_wall_fan", "fire_coral_wall_fan", "horn_coral_wall_fan",
    "dead_tube_coral", "dead_brain_coral", "dead_bubble_coral", "dead_fire_coral", "dead_horn_coral",
    "dead_tube_coral_fan", "dead_brain_coral_fan", "dead_bubble_coral_fan", "dead_fire_coral_fan", "dead_horn_coral_fan",
    "dead_tube_coral_wall_fan", "dead_brain_coral_wall_fan", "dead_bubble_coral_wall_fan", "dead_fire_coral_wall_fan", "dead_horn_coral_wall_fan",
];

fn clean_name(s: &str) -> &str {
    let mut cur = s.trim();
    if let Some(rest) = cur.strip_prefix("minecraft:") {
        cur = rest;
    }
    if let Some(rest) = cur.strip_prefix("block/") {
        cur = rest;
    }
    if let Some(rest) = cur.strip_suffix(".png") {
        cur = rest;
    }
    cur
}

/// Checks if a block or texture represents thin-wall foliage or vegetation.
pub fn is_thin_wall_block(block_name: &str, texture_name: Option<&str>) -> bool {
    let clean_b = clean_name(block_name);
    let clean_t = texture_name.map(clean_name).unwrap_or("");

    for &ex in THIN_WALL_EXACT {
        if clean_b == ex || clean_t == ex {
            return true;
        }
    }

    for &kw in THIN_WALL_KEYWORDS {
        if clean_b.contains(kw) || (!clean_t.is_empty() && clean_t.contains(kw)) {
            if clean_b == "moss_block" || clean_t == "moss_block" || clean_b == "mushroom_stem" || clean_t == "mushroom_stem" {
                return false;
            }
            return true;
        }
    }

    false
}

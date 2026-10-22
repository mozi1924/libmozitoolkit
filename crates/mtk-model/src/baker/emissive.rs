//! Emissive light-emission detection and intensity calculation for Minecraft BlockStates.

use crate::parser::blockstate::BlockState;

/// White-list of natively light-emitting blocks in Minecraft.
pub const EMISSIVE_BLOCKS: &[&str] = &[
    "glowstone",
    "sea_lantern",
    "shroomlight",
    "magma_block",
    "magma",
    "crying_obsidian",
    "jack_o_lantern",
    "beacon",
    "end_rod",
    "lantern",
    "soul_lantern",
    "torch",
    "soul_torch",
    "wall_torch",
    "soul_wall_torch",
    "lava",
    "flowing_lava",
    "fire",
    "soul_fire",
    "conduit",
    "sculk_catalyst",
    "ochre_froglight",
    "pearlescent_froglight",
    "verdant_froglight",
    "end_portal",
    "end_gateway",
];

/// Checks if a blockstate is emissive based on identifier and properties.
pub fn is_block_emissive(blockstate: &BlockState) -> bool {
    let short_name = blockstate.name.as_str();
    if EMISSIVE_BLOCKS.contains(&short_name) || short_name.ends_with("_froglight") {
        return true;
    }
    let p = &blockstate.properties;
    let is_lit = p.get("lit").map(|s| s == "true").unwrap_or(false);
    if is_lit
        && matches!(
            short_name,
            "furnace"
                | "blast_furnace"
                | "smoker"
                | "redstone_lamp"
                | "campfire"
                | "soul_campfire"
                | "redstone_ore"
                | "deepslate_redstone_ore"
        )
    {
        return true;
    }
    if matches!(short_name, "redstone_torch" | "redstone_wall_torch") {
        return p.get("lit").map(|s| s == "true").unwrap_or(true);
    }
    if short_name == "respawn_anchor" {
        if let Some(charges) = p.get("charges").and_then(|s| s.parse::<i32>().ok()) {
            return charges > 0;
        }
    }
    if short_name == "redstone_wire" {
        if let Some(power) = p.get("power").and_then(|s| s.parse::<i32>().ok()) {
            return power > 0;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emissive_detection() {
        let glowstone = BlockState::parse("minecraft:glowstone").unwrap();
        assert!(is_block_emissive(&glowstone));

        let dirt = BlockState::parse("minecraft:dirt").unwrap();
        assert!(!is_block_emissive(&dirt));

        let lit_furnace = BlockState::parse("minecraft:furnace[lit=true]").unwrap();
        assert!(is_block_emissive(&lit_furnace));

        let unlit_furnace = BlockState::parse("minecraft:furnace[lit=false]").unwrap();
        assert!(!is_block_emissive(&unlit_furnace));

        let redstone_wire = BlockState::parse("minecraft:redstone_wire[power=15]").unwrap();
        assert!(is_block_emissive(&redstone_wire));
    }
}

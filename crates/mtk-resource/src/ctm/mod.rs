pub mod algorithms;
pub mod parser;
pub mod solver;
pub mod tables;
pub mod types;

pub use parser::parse_ctm_properties;
pub use solver::CtmSolver;
pub use tables::{
    build_ctm_47_lookup, build_overlay_17_lookup, CTM_47_LOOKUP, OVERLAY_17_LOOKUP,
    TILE_TO_CONNECTION_DATA, TILE_TO_OVERLAY_DATA,
};
pub use types::{
    coordinate_random, extract_block_name, get_face_tangents, BlockMatch, ConnectLogic, CtmMethod,
    CtmRule, CtmSymmetry,
};

impl CtmRule {
    /// Parse an OptiFine / Continuity `.properties` file into a `CtmRule`.
    #[inline]
    pub fn parse_properties(source_path: &str, namespace: &str, content: &str) -> Option<Self> {
        parse_ctm_properties(source_path, namespace, content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ctm_47_lookup_table() {
        assert_eq!(CTM_47_LOOKUP[0b00000000], 0);
        assert_eq!(CTM_47_LOOKUP[0b11111111], 26);
        assert_eq!(CTM_47_LOOKUP[0b01010101], 46);
        // Corner degeneration: top-left corner isolated without top or left -> should map to 0
        assert_eq!(CTM_47_LOOKUP[0b10000000], 0);
    }

    #[test]
    fn test_ctm_horizontal_and_vertical() {
        let content_h = r#"
matchBlocks=minecraft:bookshelf
method=horizontal
tiles=0 1 2 3
"#;
        let rule_h = CtmRule::parse_properties("optifine/ctm/bookshelf.properties", "minecraft", content_h).unwrap();
        let solver = CtmSolver::new(vec![rule_h]);

        // Query bookshelf face
        use mtk_core::direction::Direction;
        use glam::IVec3;
        let pos = IVec3::new(0, 0, 0);
        let get_block = |p: IVec3| {
            if p == IVec3::new(1, 0, 0) {
                Some("minecraft:bookshelf")
            } else {
                None
            }
        };

        let tile = solver.resolve_face("minecraft:bookshelf", Direction::North, pos, None, None, get_block);
        assert_eq!(tile.unwrap().path, "optifine/ctm/0");
    }

    #[test]
    fn test_ctm_repeat_method() {
        let content_repeat = r#"
matchBlocks=sandstone
method=repeat
width=2
height=2
tiles=0 1 2 3
"#;
        let rule_repeat = CtmRule::parse_properties("optifine/ctm/sandstone.properties", "minecraft", content_repeat).unwrap();
        let solver = CtmSolver::new(vec![rule_repeat]);

        use mtk_core::direction::Direction;
        use glam::IVec3;
        let pos = IVec3::new(1, 0, 0);
        let dummy = |_: IVec3| None::<&str>;
        let tile = solver.resolve_face("minecraft:sandstone", Direction::North, pos, None, None, dummy);
        assert!(tile.is_some());
    }

    #[test]
    fn test_ctm_custom_mod_namespace() {
        let content = r#"
matchBlocks=create:cogwheel
method=horizontal
tiles=0 1 2 3
"#;
        let rule = CtmRule::parse_properties("assets/create/optifine/ctm/cogwheel.properties", "create", content).unwrap();
        assert_eq!(rule.match_blocks[0].block, "create:cogwheel");
        assert_eq!(rule.tiles[0].as_ref().unwrap().namespace, "create");
        assert_eq!(rule.tiles[0].as_ref().unwrap().path, "assets/create/optifine/ctm/0");

        let solver = CtmSolver::new(vec![rule]);
        let pos = glam::IVec3::new(0, 0, 0);
        let get_block = |p: glam::IVec3| {
            if p == glam::IVec3::new(1, 0, 0) {
                Some("create:cogwheel")
            } else {
                None
            }
        };
        let tile = solver.resolve_face("create:cogwheel", mtk_core::direction::Direction::North, pos, None, None, get_block);
        assert_eq!(tile.unwrap().to_string(), "create:assets/create/optifine/ctm/0");
    }
}

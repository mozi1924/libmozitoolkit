use std::collections::HashMap;
use glam::IVec3;
use mtk_core::direction::Direction;
use crate::identifier::{DEFAULT_NAMESPACE, ResourceLocation};
use crate::ctm::types::{extract_block_name, ConnectLogic, CtmMethod, CtmRule};
use crate::ctm::algorithms::{
    compact::solve_compact_ctm,
    directional::{solve_horizontal, solve_horizontal_vertical, solve_top, solve_vertical, solve_vertical_horizontal},
    full::solve_full_ctm,
    patterns::{solve_overlay, solve_random, solve_repeat},
};

impl CtmRule {
    /// Solves the final CTM sub-tile ResourceLocation for this face.
    pub fn solve_tile<F, S>(
        &self,
        state: &str,
        face: Direction,
        world_pos: IVec3,
        get_block: &F,
    ) -> Option<ResourceLocation>
    where
        F: Fn(IVec3) -> Option<S>,
        S: AsRef<str>,
    {
        let check_connect = |offset: IVec3| -> bool {
            let n_pos = world_pos + offset;
            if let Some(neighbor_state) = get_block(n_pos) {
                let n_str = neighbor_state.as_ref();
                match &self.connect_logic {
                    ConnectLogic::SameBlock => {
                        extract_block_name(state) == extract_block_name(n_str)
                    }
                    ConnectLogic::SameState => state == n_str,
                    ConnectLogic::BlockNames(names) => {
                        let n_name = extract_block_name(n_str);
                        let canonical = if n_name.contains(':') {
                            n_name.to_string()
                        } else {
                            format!("{}:{}", DEFAULT_NAMESPACE, n_name)
                        };
                        names.contains(&canonical)
                    }
                    ConnectLogic::SameTile | ConnectLogic::Textures(_) => {
                        extract_block_name(state) == extract_block_name(n_str)
                    }
                }
            } else {
                false
            }
        };

        match &self.method {
            CtmMethod::Full { inner_seams } => {
                let tile_idx = solve_full_ctm(face, *inner_seams, &check_connect);
                self.tiles.get(tile_idx).and_then(|t| t.clone())
            }
            CtmMethod::Compact { .. } => {
                let tile_idx = solve_compact_ctm(face, &check_connect);
                self.tiles.get(tile_idx).and_then(|t| t.clone())
            }
            CtmMethod::Horizontal => {
                let tile_idx = solve_horizontal(face, &check_connect);
                self.tiles.get(tile_idx).and_then(|t| t.clone())
            }
            CtmMethod::Vertical => {
                let tile_idx = solve_vertical(face, &check_connect);
                self.tiles.get(tile_idx).and_then(|t| t.clone())
            }
            CtmMethod::HorizontalVertical => {
                let tile_idx = solve_horizontal_vertical(face, &check_connect);
                self.tiles.get(tile_idx).and_then(|t| t.clone())
            }
            CtmMethod::VerticalHorizontal => {
                let tile_idx = solve_vertical_horizontal(face, &check_connect);
                self.tiles.get(tile_idx).and_then(|t| t.clone())
            }
            CtmMethod::Top => {
                if solve_top(face, &check_connect) {
                    self.tiles.first().and_then(|t| t.clone())
                } else {
                    None
                }
            }
            CtmMethod::Repeat { width, height } => {
                let tile_idx = solve_repeat(face, world_pos, *width, *height)?;
                self.tiles.get(tile_idx).and_then(|t| t.clone())
            }
            CtmMethod::Random {
                weights,
                total_weight,
                symmetry,
                linked,
            } => {
                if self.tiles.is_empty() {
                    return None;
                }
                let chosen_idx = solve_random(
                    state,
                    face,
                    world_pos,
                    weights,
                    *total_weight,
                    *symmetry,
                    *linked,
                    self.tiles.len(),
                    get_block,
                );
                self.tiles.get(chosen_idx).and_then(|t| t.clone())
            }
            CtmMethod::Fixed => self.tiles.first().and_then(|t| t.clone()),
            CtmMethod::Overlay => {
                let tile_idx = solve_overlay(face, &check_connect)?;
                self.tiles.get(tile_idx).and_then(|t| t.clone())
            }
        }
    }
}

/// High-performance CTM Rule Solver indexed for fast per-face resolution during chunk meshing.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CtmSolver {
    pub rules: Vec<CtmRule>,
    pub rules_by_block: HashMap<String, Vec<usize>>,
    pub rules_by_tile: HashMap<ResourceLocation, Vec<usize>>,
}

impl CtmSolver {
    /// Creates a new `CtmSolver` from an array of loaded `CtmRule`s, sorting by priority.
    pub fn new(mut rules: Vec<CtmRule>) -> Self {
        rules.sort_by_key(|r| std::cmp::Reverse(r.priority));

        let mut rules_by_block: HashMap<String, Vec<usize>> = HashMap::new();
        let mut rules_by_tile: HashMap<ResourceLocation, Vec<usize>> = HashMap::new();

        for (idx, rule) in rules.iter().enumerate() {
            for mb in &rule.match_blocks {
                rules_by_block
                    .entry(mb.block.clone())
                    .or_default()
                    .push(idx);
            }
            for mt in &rule.match_tiles {
                rules_by_tile.entry(mt.clone()).or_default().push(idx);
            }
        }

        Self {
            rules,
            rules_by_block,
            rules_by_tile,
        }
    }

    /// Resolves the final CTM tile for a block face given its state, orientation, position, and neighbor query closure.
    pub fn resolve_face<F, S>(
        &self,
        state: &str,
        face: Direction,
        world_pos: IVec3,
        base_tile: Option<&ResourceLocation>,
        biome: Option<&str>,
        get_block: F,
    ) -> Option<ResourceLocation>
    where
        F: Fn(IVec3) -> Option<S>,
        S: AsRef<str>,
    {
        if self.rules.is_empty() {
            return None;
        }

        let block_name = extract_block_name(state);
        let canonical_name = if block_name.contains(':') {
            block_name.to_string()
        } else {
            format!("{}:{}", DEFAULT_NAMESPACE, block_name)
        };

        // 1. Try match by block
        if let Some(rule_indices) = self.rules_by_block.get(&canonical_name) {
            for &idx in rule_indices {
                let rule = &self.rules[idx];
                if rule.matches_block(state, face, world_pos, biome) {
                    if let Some(tile) = rule.solve_tile(state, face, world_pos, &get_block) {
                        return Some(tile);
                    }
                }
            }
        }

        // 2. Try match by base tile
        if let Some(tile_loc) = base_tile {
            if let Some(rule_indices) = self.rules_by_tile.get(tile_loc) {
                for &idx in rule_indices {
                    let rule = &self.rules[idx];
                    if rule.matches_block(state, face, world_pos, biome) {
                        if let Some(tile) = rule.solve_tile(state, face, world_pos, &get_block) {
                            return Some(tile);
                        }
                    }
                }
            }
        }

        None
    }
}

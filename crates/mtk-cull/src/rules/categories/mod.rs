//! # Minecraft Block Categories & Parametric Shape Heuristics
//!
//! Provides catalogued name registries and parametric face occlusion resolvers
//! for standard vanilla Minecraft blocks, slabs, stairs, fluids, leaves, and partial blocks.

pub mod catalog;
pub mod meta;
pub mod parametric;

pub use catalog::*;
pub use meta::*;
pub use parametric::*;

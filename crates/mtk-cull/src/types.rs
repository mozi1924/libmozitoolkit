use mtk_core::geometry::Aabb2d;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// High-level classification of blocks for face culling decision trees.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum CullCategory {
    /// Completely opaque full cubes (e.g. Stone, Dirt, Cobblestone).
    #[default]
    SolidOpaque,
    /// Transparent glass blocks (renders same-type or all-glass culling).
    Glass,
    /// Leaves blocks (supports Fancy and Fast culling).
    Leaves,
    /// Partial blocks with non-full faces (e.g. Slabs, Stairs, Trapdoors, Panes).
    Partial,
    /// Fluid blocks (Water, Lava).
    Fluid,
    /// Air or non-occluding pass-through blocks (e.g. Torches, Flowers, Air).
    Air,
}

/// Rendering modes for leaves culling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum LeavesCullMode {
    /// Leaves faces are culled against adjacent leaves of same or all types.
    #[default]
    Fancy,
    /// Leaves are treated as opaque solids for extreme optimization.
    Fast,
    /// Only outermost envelope faces are retained.
    SingleFace,
}

/// Rendering modes for glass culling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum GlassCullMode {
    /// Cull only if the adjacent block is the exact same glass type.
    #[default]
    SameType,
    /// Cull against any glass block regardless of color.
    AllGlass,
    /// Never cull interior faces between glass blocks.
    Never,
}

/// Canonical 2D unit square representing a fully occluding face.
pub const FULL_FACE_RECT: Aabb2d = Aabb2d {
    min: mtk_core::Vec2::ZERO,
    max: mtk_core::Vec2::ONE,
};

/// 2D zero-area rectangle representing no occlusion.
pub const EMPTY_FACE_RECT: Aabb2d = Aabb2d {
    min: mtk_core::Vec2::ZERO,
    max: mtk_core::Vec2::ZERO,
};

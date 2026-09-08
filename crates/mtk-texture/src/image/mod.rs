pub mod buffer;
pub mod loader;
pub mod padding;
pub mod palette;

pub use buffer::RgbaBuffer;
pub use loader::DecodedSprite;
pub use padding::apply_edge_clamping_padding;
pub use palette::{bake_paletted_permutation, extract_palette_colors};

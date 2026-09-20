pub mod alpha;
pub mod buffer;
pub mod loader;
pub mod padding;
pub mod palette;

pub use alpha::{
    batch_analyze_transparent_faces_f32, batch_analyze_transparent_faces_u8,
    is_face_transparent_f32, is_face_transparent_u8, sample_alpha_f32, sample_alpha_u8,
    uv_to_pixel_coord, SampleMode,
};
pub use buffer::RgbaBuffer;
pub use loader::DecodedSprite;
pub use padding::apply_edge_clamping_padding;
pub use palette::{bake_paletted_permutation, extract_palette_colors};

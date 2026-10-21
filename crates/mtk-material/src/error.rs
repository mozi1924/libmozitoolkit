use thiserror::Error;

#[derive(Debug, Error)]
pub enum MaterialError {
    #[error("Texture resolution error: {0}")]
    Texture(#[from] mtk_texture::TextureError),

    #[error("Invalid UV coordinate: ({0}, {1})")]
    InvalidUv(f32, f32),

    #[error("Unknown texture '{0}' for material '{1}'")]
    UnresolvedTexture(String, String),

    #[error("Buffer length mismatch: expected {expected}, got {actual}")]
    BufferLengthMismatch { expected: usize, actual: usize },

    #[error(
        "PBR buffer size mismatch: expected {expected_width}x{expected_height}, got {actual_width}x{actual_height}"
    )]
    PbrSizeMismatch {
        expected_width: u32,
        expected_height: u32,
        actual_width: u32,
        actual_height: u32,
    },
}

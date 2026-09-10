use thiserror::Error;
use mtk_resource::ResourceError;

#[derive(Error, Debug)]
pub enum TextureError {
    #[error("Resource error: {0}")]
    Resource(#[from] ResourceError),

    #[error("Image decoding/encoding error: {0}")]
    Image(#[from] image::ImageError),

    #[error("Stitcher error: {0}")]
    Stitcher(String),

    #[error("Atlas baking error: {0}")]
    Baking(String),

    #[error("Paletted permutation error: {0}")]
    Palette(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

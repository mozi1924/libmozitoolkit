use thiserror::Error;

#[derive(Error, Debug)]
pub enum ResourceError {
    #[error("Invalid resource location format: '{0}'")]
    InvalidLocation(String),

    #[error("Resource not found: '{0}'")]
    NotFound(String),

    #[error("IO error while accessing resource pack: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[cfg(feature = "zip")]
    #[error("Zip archive error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("Atlas configuration error: {0}")]
    AtlasConfig(String),
}

use thiserror::Error;

/// Errors that can occur during lightweight NBT deserialization.
#[derive(Debug, Error)]
pub enum NbtError {
    #[error("Unexpected end of NBT byte stream")]
    UnexpectedEof,

    #[error("Invalid NBT root tag ID: expected 10 (Compound) or 0 (End), found {0}")]
    InvalidRootType(u8),

    #[error("Unknown or invalid NBT tag ID: {0}")]
    UnknownTagId(u8),

    #[error("Malformed NBT data: {0}")]
    InvalidData(String),
}

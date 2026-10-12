//! Error types for Blender DNA memory direct operations.

use thiserror::Error;

/// Errors that can occur during Blender DNA direct memory operations.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum DnaError {
    /// Destination pointer is null (0).
    #[error("Destination pointer cannot be null (0)")]
    NullPointer,

    /// Destination buffer is too small to receive the written data.
    #[error(
        "Direct copy buffer overflow protection: required {required} bytes, but destination max_bytes is {limit}"
    )]
    BufferOverflow {
        /// Number of bytes required.
        required: usize,
        /// Maximum allowed bytes specified by the caller.
        limit: usize,
    },

    /// Destination pointer alignment does not match target type requirements.
    #[error("Invalid memory alignment: pointer 0x{ptr:x} is not aligned to {align} bytes")]
    MisalignedPointer {
        /// Memory pointer address.
        ptr: usize,
        /// Required byte alignment.
        align: usize,
    },
}

/// Convenience Result type for DNA operations.
pub type DnaResult<T> = Result<T, DnaError>;

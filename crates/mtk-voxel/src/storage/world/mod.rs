//! # Voxel World Storage
//!
//! Provides sparse 3D chunk-section world management, snapshot diffing,
//! boundary seam dirty tracking, CRC caching, and padded boundary extraction.

pub mod container;
pub mod manifest;
pub mod padded;
pub mod snapshot;

pub use container::*;

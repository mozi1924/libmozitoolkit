//! # Biome Resolver and Model Tint Scanning
//!
//! Provides resource pack block model parsing, `tintindex` discovery,
//! and authoritative SSOT vanilla biome mapping metadata.

pub mod defaults;
pub mod lookup;
pub mod scanner;
pub mod types;

pub use types::*;

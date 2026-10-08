//! # Headless Asset Prebaking and Cache Engine
//!
//! Performs unified end-to-end asset precompilation (Atlas stitching, Standalone PBR alignment,
//! and multi-threaded Model baking) directly into `.mtkcache` binary package files.

pub mod engine;
pub mod models;
pub mod reader;

pub use engine::{
    precompile_all_assets, precompile_all_assets_with_progress, PrecompileConfig, PrecompileResult,
};
pub use models::{prebake_all_models, prebake_all_models_with_progress};
pub use reader::{
    fingerprint_str_to_bytes16, AssetCacheReader, CacheManifest, ASSET_CACHE_FORMAT_VERSION,
};

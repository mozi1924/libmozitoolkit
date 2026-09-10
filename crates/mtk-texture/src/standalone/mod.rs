//! # Standalone Material Asset Library Module
//!
//! Provides high-performance synthesis and precompilation of standalone single-block textures,
//! multi-channel PBR frame alignment, UV scaling metadata computation, and atomic cache publishing.

pub mod aligner;
pub mod builder;

pub use aligner::{
    align_standalone_channels, ChannelData, ChannelType, StandaloneAlignResult,
    StandaloneAnimationMeta,
};
pub use builder::{
    StandaloneBuilder, StandaloneConfig, StandaloneFilePaths, StandaloneMapping,
    StandaloneResult, StandaloneTextureRecord, STANDALONE_FORMAT_VERSION,
};

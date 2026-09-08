//! # mtk-resource
//!
//! Minecraft resource pack virtual file system, asset identifier specification,
//! `.png.mcmeta` animation metadata parser, and vanilla `atlases/*.json` data models.

pub mod atlas;
pub mod ctm;
pub mod error;
pub mod identifier;
pub mod meta;
pub mod pack;

pub use atlas::{AtlasDefinition, AtlasFilterPattern, AtlasSource, UnstitchRegion};
pub use ctm::{BlockMatch, ConnectLogic, CtmMethod, CtmRule, CtmSymmetry};
pub use error::ResourceError;
pub use identifier::{DEFAULT_NAMESPACE, ResourceLocation};
pub use meta::{AnimationFrame, AnimationMetadata, TextureMetadata};
pub use pack::{DirectoryPack, DiscoveredSprite, MemoryPack, PbrCompanions, ResourcePack, ResourcePackStack};
#[cfg(feature = "zip")]
pub use pack::ZipPack;

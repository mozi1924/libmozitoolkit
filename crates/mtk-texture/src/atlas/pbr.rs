//! # PBR Atlas Metadata
//!
//! Canonical PBR channel metadata shared by the atlas cache (`mtk-texture`) and the
//! source-format decoders (`mtk-material`). The decoder itself lives in
//! `mtk-material::pbr`; these types live here so `AtlasChunkMeta` can serialize them
//! without creating a `mtk-texture -> mtk-material` dependency cycle.
//!
//! See `MoziToolKit/docs/materials/pbr_channel_spec.md`.

use serde::{Deserialize, Serialize};

/// Packed texture layout written into the asset cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PbrLayout {
    /// albedo / normal+height / ORM+emission / aux RGBA8 sheets.
    MtkPackedV1,
}

/// Source PBR format that produced the canonical channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PbrSourceFormat {
    #[serde(rename = "labpbr13")]
    LabPbr13,
    BedrockMer,
    MetallicRoughness,
    Unknown,
}

/// Tangent-space normal convention stored in the canonical normal channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NormalConvention {
    /// +Y up; Blender, OpenPBR, UsdPreviewSurface, Bedrock.
    Opengl,
    /// −Y up; LabPBR source, Unreal.
    Directx,
}

/// Semantics of the canonical roughness channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoughnessSpace {
    /// Blender / OpenPBR / glTF perceptual roughness (squared internally).
    Perceptual,
    /// GGX alpha / linear roughness.
    Linear,
}

/// 8-bit encoding used by the `aux.R` specular-IOR channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IorEncoding {
    /// `encoded = (ior - 1) / 7`, ior clamped to `[1, 8]`.
    #[serde(rename = "range_1_8")]
    Range1To8,
}

/// Metadata describing how a chunk's canonical PBR channels were produced.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PbrMeta {
    pub layout: PbrLayout,
    pub source_format: PbrSourceFormat,
    pub normal_convention: NormalConvention,
    pub roughness_space: RoughnessSpace,
    pub height_scale: f32,
    pub height_midlevel: f32,
    pub ior_encoding: IorEncoding,
    pub decoder_version: u32,
}

impl PbrMeta {
    /// LabPBR 1.3 canonical metadata.
    pub fn labpbr13() -> Self {
        Self {
            layout: PbrLayout::MtkPackedV1,
            source_format: PbrSourceFormat::LabPbr13,
            normal_convention: NormalConvention::Opengl,
            roughness_space: RoughnessSpace::Perceptual,
            height_scale: 0.25,
            height_midlevel: 1.0,
            ior_encoding: IorEncoding::Range1To8,
            decoder_version: 1,
        }
    }
}

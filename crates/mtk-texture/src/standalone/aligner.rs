//! # Standalone Texture Aligner
//!
//! Synchronizes multi-channel PBR textures (Albedo, Normal, Specular, Overlay) by detecting
//! animations and vertically tiling static/shorter channels to match the target frame count.

use serde::{Deserialize, Serialize};
use mtk_resource::{AnimationFrame, AnimationMetadata};
use crate::image::RgbaBuffer;

/// Available texture channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChannelType {
    Albedo,
    Normal,
    Specular,
    Overlay,
}

impl ChannelType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ChannelType::Albedo => "albedo",
            ChannelType::Normal => "normal",
            ChannelType::Specular => "specular",
            ChannelType::Overlay => "overlay",
        }
    }
}

/// Single channel data containing its RGBA image buffer and optional MCMETA animation.
#[derive(Debug, Clone)]
pub struct ChannelData {
    pub channel_type: ChannelType,
    pub buffer: RgbaBuffer,
    pub metadata: Option<AnimationMetadata>,
}

/// Animation metadata serialized to `standalone_mapping.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StandaloneAnimationMeta {
    pub frame_width: u32,
    pub frame_height: u32,
    pub image_width: u32,
    pub image_height: u32,
    pub total_frames: u32,
    pub frametime: u32,
    pub interpolate: bool,
    pub frames: Vec<u32>,
    pub v_scale: f32,
    pub v_offset: f32,
}

/// Result of aligning multi-channel textures.
#[derive(Debug, Clone)]
pub struct StandaloneAlignResult {
    pub is_animated: bool,
    pub channels: Vec<ChannelData>,
    pub animation: Option<StandaloneAnimationMeta>,
}

/// Check if an individual channel has animation properties (multiple frames).
pub fn is_channel_animated(buffer: &RgbaBuffer, metadata: Option<&AnimationMetadata>) -> bool {
    if buffer.width == 0 || buffer.height == 0 {
        return false;
    }

    if let Some(meta) = metadata {
        let frame_w = meta.width.unwrap_or(buffer.width);
        let frame_h = meta.height.unwrap_or(frame_w);
        let total_frames = if frame_h > 0 { buffer.height / frame_h } else { 1 };
        if total_frames > 1 {
            return true;
        }
        if let Some(ref frames) = meta.frames {
            if frames.len() > 1 {
                return true;
            }
        }
    } else if buffer.height > buffer.width && buffer.height % buffer.width == 0 {
        return true;
    }

    false
}

/// Inspect all provided channels and align animated/static channels to matching frame counts.
pub fn align_standalone_channels(mut channels: Vec<ChannelData>) -> StandaloneAlignResult {
    if channels.is_empty() {
        return StandaloneAlignResult {
            is_animated: false,
            channels,
            animation: None,
        };
    }

    // 1. Identify which channels are animated
    let mut animated_indices = Vec::new();
    let mut albedo_idx = None;

    for (idx, ch) in channels.iter().enumerate() {
        if ch.channel_type == ChannelType::Albedo {
            albedo_idx = Some(idx);
        }
        if is_channel_animated(&ch.buffer, ch.metadata.as_ref()) {
            animated_indices.push(idx);
        }
    }

    // If no channel has animation, return as-is
    if animated_indices.is_empty() {
        return StandaloneAlignResult {
            is_animated: false,
            channels,
            animation: None,
        };
    }

    // 2. Select reference channel: prefer Albedo if animated, else first animated channel
    let ref_idx = if let Some(a_idx) = albedo_idx {
        if animated_indices.contains(&a_idx) {
            a_idx
        } else {
            animated_indices[0]
        }
    } else {
        animated_indices[0]
    };

    let ref_ch = &channels[ref_idx];
    let ref_w = ref_ch.buffer.width;
    let ref_h = ref_ch.buffer.height;
    let ref_meta = ref_ch.metadata.as_ref();

    let ref_frame_w = ref_meta.and_then(|m| m.width).unwrap_or(ref_w);
    let ref_frame_h = ref_meta.and_then(|m| m.height).unwrap_or(ref_frame_w);
    let target_frame_count = if ref_frame_h > 0 {
        (ref_h / ref_frame_h).max(1)
    } else {
        1
    };
    let target_frametime = ref_meta.map_or(1, |m| m.frametime.max(1));
    let target_interpolate = ref_meta.map_or(false, |m| m.interpolate);

    let target_frames: Vec<u32> = if let Some(ref frames) = ref_meta.and_then(|m| m.frames.as_ref()) {
        frames.iter().map(|f| f.index()).collect()
    } else {
        (0..target_frame_count).collect()
    };

    if target_frame_count <= 1 && target_frames.len() <= 1 {
        return StandaloneAlignResult {
            is_animated: false,
            channels,
            animation: None,
        };
    }

    // 3. Align each channel vertically to target_frame_count
    for ch in &mut channels {
        let src_w = ch.buffer.width;
        let src_h = ch.buffer.height;
        if src_w == 0 || src_h == 0 {
            continue;
        }

        let ch_meta = ch.metadata.as_ref();
        let ch_frame_w = ch_meta.and_then(|m| m.width).unwrap_or(src_w);
        let ch_frame_h = ch_meta.and_then(|m| m.height).unwrap_or(ch_frame_w);
        let ch_frame_count = if ch_frame_h > 0 {
            (src_h / ch_frame_h).max(1)
        } else {
            1
        };

        let aligned_h = ch_frame_h * target_frame_count;

        if ch_frame_count != target_frame_count || src_h != aligned_h {
            let new_buf = if src_h >= aligned_h {
                ch.buffer.crop(0, 0, src_w, aligned_h)
            } else {
                ch.buffer.tile_vertical(aligned_h)
            };
            ch.buffer = new_buf;
        }

        // Update metadata with synchronized animation timing
        ch.metadata = Some(AnimationMetadata {
            frametime: target_frametime,
            interpolate: target_interpolate,
            width: Some(ch_frame_w),
            height: Some(ch_frame_h),
            frames: Some(target_frames.iter().map(|&idx| AnimationFrame::Index(idx)).collect()),
        });
    }

    // 4. Compute UV scaling metadata
    let v_scale = if ref_h > 0 {
        (ref_frame_h as f32) / (ref_h as f32)
    } else {
        1.0
    };
    let v_offset = 1.0 - v_scale;

    let anim_meta = StandaloneAnimationMeta {
        frame_width: ref_frame_w,
        frame_height: ref_frame_h,
        image_width: ref_w,
        image_height: ref_h,
        total_frames: target_frame_count,
        frametime: target_frametime,
        interpolate: target_interpolate,
        frames: target_frames,
        v_scale,
        v_offset,
    };

    StandaloneAlignResult {
        is_animated: true,
        channels,
        animation: Some(anim_meta),
    }
}

//! High-speed third-party skin mirror clients (Crafthead & Minotar) with strict validation.
//!
//! Mirrors shield official Mojang rate limits (600 req/10min, 200 req/1min).
//! However, mirrors return HTTP 200 with fallback skins (Alex/Steve) for invalid or offline UUIDs.
//! This module strictly inspects response headers (`x-crafthead-profile-cache-hit`, `x-crafthead-texture-id`,
//! `etag`) and content hashes to ensure fallback skins are rejected and not poisoned into cache.

use super::index::{compute_sha256_hex, is_mirror_fallback_hash};
use super::resolver::is_valid_png_header;
use std::io::Read;

/// Reads up to 1 MB PNG image from response body, checking PNG magic bytes.
pub fn read_png_body(resp: ureq::Response) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    resp.into_reader()
        .take(1024 * 1024)
        .read_to_end(&mut bytes)
        .ok()?;

    if is_valid_png_header(&bytes) {
        Some(bytes)
    } else {
        None
    }
}

/// Queries Crafthead (Cloudflare Worker CDN).
///
/// Returns `Some((canonical_mojang_texture_url, png_bytes))` if the skin was found.
/// Returns `None` if the profile is invalid or if Crafthead returned a fallback default skin.
pub fn query_crafthead(agent: &ureq::Agent, identifier: &str) -> Option<(String, Vec<u8>)> {
    let url = format!("https://crafthead.net/skin/{}", identifier);
    let resp = agent
        .get(&url)
        .set("User-Agent", "MoziToolKit/2.0")
        .call()
        .ok()?;

    if resp.status() != 200 {
        return None;
    }

    // 1. Check profile cache hit header: Crafthead returns "invalid-profile" for non-existent profiles.
    if let Some(hit) = resp.header("x-crafthead-profile-cache-hit") {
        if hit.eq_ignore_ascii_case("invalid-profile") {
            return None;
        }
    }

    // 2. Texture ID header must be present and be a 64-char hex string (Mojang texture hash).
    let texture_id = resp.header("x-crafthead-texture-id")?.trim();
    if texture_id.len() != 64 || !texture_id.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let texture_id = texture_id.to_string();

    let bytes = read_png_body(resp)?;

    // 3. Verify content hash matches texture_id and is not a known fallback skin
    let hash = compute_sha256_hex(&bytes);
    if !hash.eq_ignore_ascii_case(&texture_id) || is_mirror_fallback_hash(&hash) {
        return None;
    }

    let canonical_url = format!(
        "http://textures.minecraft.net/texture/{}",
        texture_id.to_ascii_lowercase()
    );
    Some((canonical_url, bytes))
}

/// Queries Minotar skin API.
///
/// Returns `Some((canonical_mojang_texture_url, png_bytes))` if the skin was found.
/// Returns `None` if the profile was not found or fell back to a default skin.
pub fn query_minotar(agent: &ureq::Agent, identifier: &str) -> Option<(String, Vec<u8>)> {
    let url = format!("https://minotar.net/skin/{}", identifier);
    let resp = agent
        .get(&url)
        .set("User-Agent", "MoziToolKit/2.0")
        .call()
        .ok()?;

    if resp.status() != 200 {
        return None;
    }

    // Minotar returns etag header:
    // If skin exists, etag is 64-char hex SHA256 (the texture hash).
    // If skin is a fallback (e.g. invalid UUID), etag is a 32-char MD5 (e.g. "98903c1609352e11552dca79eb1ce3d6").
    let etag = resp.header("etag")?;
    let clean_etag = etag.trim().trim_matches('"');
    if clean_etag.len() != 64 || !clean_etag.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let clean_etag = clean_etag.to_string();

    let bytes = read_png_body(resp)?;

    // Content hash verification
    let hash = compute_sha256_hex(&bytes);
    if !hash.eq_ignore_ascii_case(&clean_etag) || is_mirror_fallback_hash(&hash) {
        return None;
    }

    let canonical_url = format!(
        "http://textures.minecraft.net/texture/{}",
        clean_etag.to_ascii_lowercase()
    );
    Some((canonical_url, bytes))
}

/// Queries third-party mirrors in priority order (Crafthead -> Minotar).
pub fn query_mirror_skin(agent: &ureq::Agent, identifier: &str) -> Option<(String, Vec<u8>)> {
    if let Some(res) = query_crafthead(agent, identifier) {
        return Some(res);
    }
    if let Some(res) = query_minotar(agent, identifier) {
        return Some(res);
    }
    None
}

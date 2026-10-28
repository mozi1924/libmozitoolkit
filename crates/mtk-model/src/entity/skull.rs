//! Skull and player head entity definitions, SkullKind, and player profile texture extraction.

use serde::{Deserialize, Serialize};

/// Skull and Head species in Minecraft.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkullKind {
    Skeleton,
    WitherSkeleton,
    Zombie,
    Player,
    Creeper,
    Dragon,
    Piglin,
}

impl SkullKind {
    /// Maps vanilla legacy `SkullType` integer (0..6) to `SkullKind`.
    pub fn from_type_id(type_id: u8) -> Self {
        match type_id {
            0 => Self::Skeleton,
            1 => Self::WitherSkeleton,
            2 => Self::Zombie,
            3 => Self::Player,
            4 => Self::Creeper,
            5 => Self::Dragon,
            6 => Self::Piglin,
            _ => Self::Skeleton,
        }
    }

    /// Resolves `SkullKind` from a blockstate or item name (e.g. "player_head", "skeleton_skull").
    pub fn from_block_name(name: &str) -> Option<Self> {
        let clean = name.strip_prefix("minecraft:").unwrap_or(name);
        if clean.contains("wither_skeleton") {
            Some(Self::WitherSkeleton)
        } else if clean.contains("skeleton") {
            Some(Self::Skeleton)
        } else if clean.contains("zombie") {
            Some(Self::Zombie)
        } else if clean.contains("player") {
            Some(Self::Player)
        } else if clean.contains("creeper") {
            Some(Self::Creeper)
        } else if clean.contains("dragon") {
            Some(Self::Dragon)
        } else if clean.contains("piglin") {
            Some(Self::Piglin)
        } else {
            None
        }
    }

    /// Resolves default vanilla entity texture path in standard assets.
    pub fn default_texture_path(&self) -> &'static str {
        match self {
            Self::Skeleton => "minecraft:entity/skeleton/skeleton",
            Self::WitherSkeleton => "minecraft:entity/skeleton/wither_skeleton",
            Self::Zombie => "minecraft:entity/zombie/zombie",
            Self::Player => "minecraft:entity/player/wide/steve",
            Self::Creeper => "minecraft:entity/creeper/creeper",
            Self::Dragon => "minecraft:entity/enderdragon/dragon",
            Self::Piglin => "minecraft:entity/piglin/piglin",
        }
    }
}

/// Player Profile Data extracted from `SkullOwner` or `components."minecraft:profile"`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct PlayerProfileData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skin_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skin_base64: Option<String>,
}

impl PlayerProfileData {
    pub fn new(
        name: Option<String>,
        id: Option<String>,
        skin_url: Option<String>,
        skin_base64: Option<String>,
    ) -> Self {
        let mut data = Self {
            name,
            id,
            skin_url,
            skin_base64,
        };
        // Auto-extract skin_url from skin_base64 if not already populated
        if data.skin_url.is_none() {
            if let Some(ref b64) = data.skin_base64 {
                data.skin_url = extract_skin_url_from_base64(b64);
            }
        }
        data
    }

    /// Derives the canonical cache key for this profile.
    pub fn skin_key(&self) -> String {
        derive_skin_key(
            self.skin_url.as_deref(),
            self.id.as_deref(),
            self.name.as_deref(),
        )
    }

    /// Returns the unique skin hash (e.g. from `http://textures.minecraft.net/texture/<hash>`).
    pub fn skin_hash(&self) -> Option<&str> {
        self.skin_url.as_deref().and_then(extract_skin_hash)
    }

    /// Returns the resolved texture resource identifier or fallback.
    pub fn texture_identifier(&self) -> String {
        let key = self.skin_key();
        if key != "default_steve" {
            format!("minecraft:skin/{}", key)
        } else {
            "minecraft:entity/player/wide/steve".to_string()
        }
    }
}

/// Derives a stable, filesystem-safe canonical cache key for a skin reference.
pub fn derive_skin_key(skin_url: Option<&str>, uuid: Option<&str>, name: Option<&str>) -> String {
    if let Some(url) = skin_url {
        let clean = url.trim_end_matches('/');
        if let Some(last_seg) = clean.rsplit('/').next() {
            // Standard Mojang 64-hex SHA256 texture hash
            if last_seg.len() == 64 && last_seg.chars().all(|c| c.is_ascii_hexdigit()) {
                return last_seg.to_ascii_lowercase();
            }
        }
        // Custom CDN or URL with query parameters
        let h = xxhash_rust::xxh64::xxh64(clean.as_bytes(), 0);
        return format!("cdn_{:016x}", h);
    }

    if let Some(u) = uuid {
        let s = u
            .strip_prefix("uuid_")
            .or_else(|| u.strip_prefix("uuid:"))
            .unwrap_or(u);
        let clean_u = s.replace('-', "").to_ascii_lowercase();
        if !clean_u.is_empty() {
            return format!("uuid_{}", clean_u);
        }
    }

    if let Some(n) = name {
        let s = n
            .strip_prefix("player_")
            .or_else(|| n.strip_prefix("player:"))
            .unwrap_or(n);
        let clean_n = s.trim().to_ascii_lowercase();
        if !clean_n.is_empty() {
            return format!("player_{}", clean_n);
        }
    }

    "default_steve".to_string()
}

/// Extracts the final URL segment hash from a skin URL.
pub fn extract_skin_hash(url: &str) -> Option<&str> {
    let clean = url.trim_end_matches('/');
    clean.rsplit('/').next().filter(|s| !s.is_empty())
}

fn decode_base64_char(c: u8) -> Option<u8> {
    match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'+' | b'-' => Some(62),
        b'/' | b'_' => Some(63),
        _ => None,
    }
}

/// Zero-dependency URL-safe / standard Base64 decoder.
pub fn decode_base64(input: &str) -> Option<Vec<u8>> {
    let clean = input.trim();
    let bytes = clean.as_bytes();
    let mut out = Vec::with_capacity((bytes.len() * 3) / 4);
    let mut buf = 0u32;
    let mut bits = 0;

    for &b in bytes {
        if b == b'=' {
            break;
        }
        if b.is_ascii_whitespace() {
            continue;
        }
        let val = decode_base64_char(b)?;
        buf = (buf << 6) | (val as u32);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
        }
    }
    Some(out)
}

/// Extracts a skin URL from a Base64-encoded Minecraft textures property JSON.
pub fn extract_skin_url_from_base64(b64: &str) -> Option<String> {
    let decoded = decode_base64(b64)?;
    let val: serde_json::Value = serde_json::from_slice(&decoded).ok()?;
    let url = val
        .get("textures")
        .and_then(|t| t.get("SKIN"))
        .and_then(|s| s.get("url"))
        .and_then(|u| u.as_str())?;
    Some(url.to_string())
}

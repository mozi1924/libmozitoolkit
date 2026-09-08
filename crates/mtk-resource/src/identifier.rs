use std::fmt;
use std::str::FromStr;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use crate::error::ResourceError;

pub const DEFAULT_NAMESPACE: &str = "minecraft";

/// A standardized Minecraft Resource Location (Identifier), such as `minecraft:block/stone`.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ResourceLocation {
    pub namespace: String,
    pub path: String,
}

impl ResourceLocation {
    /// Create a new ResourceLocation with an explicit namespace and path.
    pub fn new(namespace: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            namespace: namespace.into(),
            path: path.into(),
        }
    }

    /// Create a new ResourceLocation in the `minecraft` namespace.
    pub fn vanilla(path: impl Into<String>) -> Self {
        Self::new(DEFAULT_NAMESPACE, path)
    }

    /// Parse a resource string such as `"minecraft:block/stone"` or `"block/stone"`.
    /// Missing namespace defaults to `"minecraft"`.
    pub fn parse(s: &str) -> Result<Self, ResourceError> {
        let s = s.trim();
        if s.is_empty() {
            return Err(ResourceError::InvalidLocation("Empty resource string".to_string()));
        }

        if let Some((ns, path)) = s.split_once(':') {
            if ns.is_empty() {
                return Err(ResourceError::InvalidLocation(format!("Empty namespace in '{}'", s)));
            }
            if path.is_empty() {
                return Err(ResourceError::InvalidLocation(format!("Empty path in '{}'", s)));
            }
            Ok(Self::new(ns, path))
        } else {
            Ok(Self::new(DEFAULT_NAMESPACE, s))
        }
    }

    /// Canonical string format: `"namespace:path"`.
    pub fn as_string(&self) -> String {
        format!("{}:{}", self.namespace, self.path)
    }

    /// Convert to a physical file path within standard Minecraft `assets/` structure.
    /// E.g. `location.to_asset_path("textures", "png")` -> `"assets/minecraft/textures/block/stone.png"`
    pub fn to_asset_path(&self, category_dir: &str, extension: &str) -> String {
        let ext = if extension.starts_with('.') {
            &extension[1..]
        } else {
            extension
        };
        if ext.is_empty() {
            format!("assets/{}/{}/{}", self.namespace, category_dir, self.path)
        } else {
            format!("assets/{}/{}/{}.{}", self.namespace, category_dir, self.path, ext)
        }
    }

    /// Try to construct a ResourceLocation from a physical relative asset path.
    /// E.g. `"assets/minecraft/textures/block/stone.png"` with prefix `"textures"` and ext `"png"`
    /// -> `Some(ResourceLocation("minecraft", "block/stone"))`
    pub fn from_asset_path(asset_path: &str, category_dir: &str, extension: &str) -> Option<Self> {
        let path = asset_path.replace('\\', "/");
        let prefix = format!("assets/");
        if !path.starts_with(&prefix) {
            return None;
        }
        let remainder = &path[prefix.len()..];
        let (ns, rest) = remainder.split_once('/')?;

        let cat_prefix = format!("{}/", category_dir);
        if !rest.starts_with(&cat_prefix) {
            return None;
        }
        let inner_path = &rest[cat_prefix.len()..];

        let ext_suffix = if extension.starts_with('.') {
            extension.to_string()
        } else if !extension.is_empty() {
            format!(".{}", extension)
        } else {
            String::new()
        };

        let trimmed_path = if !ext_suffix.is_empty() && inner_path.ends_with(&ext_suffix) {
            &inner_path[..inner_path.len() - ext_suffix.len()]
        } else {
            inner_path
        };

        Some(Self::new(ns, trimmed_path))
    }

    /// Return a new ResourceLocation with a PBR companion suffix (e.g. `_n` or `_s`).
    pub fn with_suffix(&self, suffix: &str) -> Self {
        Self::new(&self.namespace, format!("{}{}", self.path, suffix))
    }

    /// Return the `.mcmeta` asset location.
    pub fn to_mcmeta_asset_path(&self, category_dir: &str, extension: &str) -> String {
        format!("{}.mcmeta", self.to_asset_path(category_dir, extension))
    }

    /// Get the short file stem without directory prefixes.
    pub fn short_name(&self) -> &str {
        self.path.rsplit('/').next().unwrap_or(&self.path)
    }
}

impl fmt::Display for ResourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.namespace, self.path)
    }
}

impl fmt::Debug for ResourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.namespace, self.path)
    }
}

impl FromStr for ResourceLocation {
    type Err = ResourceError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl Serialize for ResourceLocation {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.as_string())
    }
}

impl<'de> Deserialize<'de> for ResourceLocation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::parse(&s).map_err(serde::de::Error::custom)
    }
}

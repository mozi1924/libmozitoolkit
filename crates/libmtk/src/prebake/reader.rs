//! Asset cache reader and manifest verification for `.mtkcache` packages.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use mtk_material::BiomeResolver;
use mtk_model::BakedModelDatabase;
use mtk_package::{MtkPackageReader, CURRENT_VERSION_MAJOR};
use mtk_texture::atlas::AtlasAddressMap;
use mtk_texture::StandaloneMapping;

use crate::MtkError;

/// Format version for compiled cache compatibility checking.
pub const ASSET_CACHE_FORMAT_VERSION: u32 = 2;

/// Converts a string fingerprint (e.g. `"mtk_fp_1234abcd5678ef00"`) to a 16-byte array.
pub fn fingerprint_str_to_bytes16(fp: &str) -> [u8; 16] {
    let mut out = [0u8; 16];
    let bytes = fp.as_bytes();
    if let Some(hex_part) = fp.strip_prefix("mtk_fp_") {
        if let Ok(val) = u64::from_str_radix(hex_part, 16) {
            out[..8].copy_from_slice(&val.to_le_bytes());
            return out;
        }
    }
    let copy_len = bytes.len().min(16);
    out[..copy_len].copy_from_slice(&bytes[..copy_len]);
    out
}

/// Metadata manifest stored in `.mtkcache` container for integrity and invalidation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheManifest {
    pub format_version: u32,
    pub fingerprint: String,
    pub pack_count: usize,
    pub atlas_chunks: usize,
    pub standalone_textures: usize,
    pub baked_models: usize,
    pub created_at_epoch_secs: u64,
}

impl CacheManifest {
    /// Read manifest directly from a `.mtkcache` package file.
    pub fn read_from_package(path: &Path) -> Option<Self> {
        let reader = MtkPackageReader::open_file(path).ok()?;
        reader.read_chunk_json("manifest").ok()
    }

    /// Read manifest from a package file or by locating `<fingerprint>.mtkcache` in `cache_dir`.
    pub fn read_from_dir(cache_dir: &Path) -> Option<Self> {
        if cache_dir.is_file() {
            return Self::read_from_package(cache_dir);
        }
        if let Ok(entries) = fs::read_dir(cache_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().is_some_and(|ext| ext == "mtkcache") {
                    if let Some(manifest) = Self::read_from_package(&p) {
                        return Some(manifest);
                    }
                }
            }
        }
        None
    }

    /// Check if manifest matches a given stack fingerprint and format version.
    pub fn is_valid_for(&self, fingerprint: &str) -> bool {
        self.format_version == ASSET_CACHE_FORMAT_VERSION && self.fingerprint == fingerprint
    }
}

/// High-performance reader for precompiled asset cache packages (`.mtkcache`).
pub struct AssetCacheReader {
    reader: MtkPackageReader,
}

impl AssetCacheReader {
    /// Opens an asset cache package file using memory mapping.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, MtkError> {
        let reader = MtkPackageReader::open_file(path)?;
        Ok(Self { reader })
    }

    /// Checks if a package at `path` is valid for the given fingerprint using only the 64-byte header.
    pub fn is_valid_cache_file(path: impl AsRef<Path>, fingerprint: &str) -> bool {
        let p = path.as_ref();
        if !p.exists() || !p.is_file() {
            return false;
        }
        if let Ok(hdr) = MtkPackageReader::read_header_only(p) {
            let fp_bytes = fingerprint_str_to_bytes16(fingerprint);
            hdr.fingerprint == fp_bytes && hdr.version_major == CURRENT_VERSION_MAJOR
        } else {
            false
        }
    }

    /// Returns package header reference.
    pub fn header(&self) -> &mtk_package::PackageHeader {
        self.reader.header()
    }

    /// Returns 16-byte fingerprint.
    pub fn fingerprint(&self) -> &[u8; 16] {
        self.reader.fingerprint()
    }

    /// Reads cache manifest metadata.
    pub fn manifest(&self) -> Result<CacheManifest, MtkError> {
        let manifest: CacheManifest = self.reader.read_chunk_json("manifest")?;
        Ok(manifest)
    }

    /// Loads baked model database directly from package.
    pub fn load_models(&self) -> Result<BakedModelDatabase, MtkError> {
        let raw = self.reader.read_chunk_decompressed("models/database")?;
        let db = BakedModelDatabase::from_bincode(&raw)?;
        Ok(db)
    }

    /// Loads atlas address mapping directly from package.
    pub fn load_atlas_mapping(&self) -> Result<AtlasAddressMap, MtkError> {
        let json_str = self.reader.read_chunk_str("atlas/mapping")?;
        let map = AtlasAddressMap::from_json(&json_str)?;
        Ok(map)
    }

    /// Loads biome resolver directly from package.
    pub fn load_biome_resolver(&self) -> Result<BiomeResolver, MtkError> {
        let json_str = self.reader.read_chunk_str("biome/mapping")?;
        let resolver = BiomeResolver::from_json(&json_str)?;
        Ok(resolver)
    }

    /// Loads standalone mapping table directly from package.
    pub fn load_standalone_mapping(&self) -> Result<StandaloneMapping, MtkError> {
        let json_str = self.load_standalone_mapping_json()?;
        let mapping = serde_json::from_str(&json_str)?;
        Ok(mapping)
    }

    /// Loads raw standalone mapping JSON string directly from package.
    pub fn load_standalone_mapping_json(&self) -> Result<String, MtkError> {
        let json_str = self.reader.read_chunk_str("standalone/mapping")?;
        Ok(json_str)
    }

    /// Extracts a specific atlas texture chunk to disk.
    pub fn extract_atlas_texture(
        &self,
        filename: &str,
        output_path: impl AsRef<Path>,
    ) -> Result<(), MtkError> {
        let chunk_id = format!("atlas/textures/{}", filename);
        self.reader.extract_chunk_to_file(&chunk_id, output_path)?;
        Ok(())
    }

    /// Extracts all atlas textures (`atlas/textures/*`) and `atlas_mapping.json` into the target directory.
    pub fn extract_all_atlas_textures(
        &self,
        output_dir: impl AsRef<Path>,
    ) -> Result<Vec<PathBuf>, MtkError> {
        let out_dir = output_dir.as_ref();
        fs::create_dir_all(out_dir)?;
        let mut extracted = Vec::new();

        for entry in self.reader.toc().iter_by_type(*b"ATLS") {
            if let Some(filename) = entry.identifier.strip_prefix("atlas/textures/") {
                let target = out_dir.join(filename);
                self.reader
                    .extract_chunk_to_file(&entry.identifier, &target)?;
                extracted.push(target);
            }
        }
        if let Ok(mapping_str) = self.reader.read_chunk_str("atlas/mapping") {
            let target = out_dir.join("atlas_mapping.json");
            fs::write(&target, mapping_str)?;
            extracted.push(target);
        }
        Ok(extracted)
    }

    /// Extracts a specific standalone texture to disk.
    pub fn extract_standalone_texture(
        &self,
        rel_path: &str,
        output_path: impl AsRef<Path>,
    ) -> Result<(), MtkError> {
        let chunk_id = format!("standalone/{}", rel_path);
        let out_p = output_path.as_ref();
        if let Some(parent) = out_p.parent() {
            fs::create_dir_all(parent)?;
        }
        self.reader.extract_chunk_to_file(&chunk_id, out_p)?;
        Ok(())
    }

    /// Extracts all standalone textures (`standalone/*`) and `standalone_mapping.json` into the target directory.
    pub fn extract_all_standalone_textures(
        &self,
        output_dir: impl AsRef<Path>,
    ) -> Result<Vec<PathBuf>, MtkError> {
        let out_dir = output_dir.as_ref();
        fs::create_dir_all(out_dir)?;
        let mut extracted = Vec::new();

        for entry in self.reader.toc().iter_by_type(*b"TXTR") {
            if let Some(rel_path) = entry.identifier.strip_prefix("standalone/") {
                if rel_path == "mapping" {
                    continue;
                }
                let target = out_dir.join(rel_path);
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent)?;
                }
                self.reader
                    .extract_chunk_to_file(&entry.identifier, &target)?;
                extracted.push(target);
            }
        }
        if let Ok(mapping_str) = self.load_standalone_mapping_json() {
            let target = out_dir.join("standalone_mapping.json");
            fs::write(&target, mapping_str)?;
            extracted.push(target);
        }
        Ok(extracted)
    }

    /// Extracts all colormap textures into the target directory.
    pub fn extract_all_colormaps(
        &self,
        output_dir: impl AsRef<Path>,
    ) -> Result<Vec<PathBuf>, MtkError> {
        let out_dir = output_dir.as_ref();
        fs::create_dir_all(out_dir)?;
        let mut extracted = Vec::new();

        for entry in self.reader.toc().iter_by_type(*b"BIOM") {
            if let Some(name) = entry.identifier.strip_prefix("biome/colormap/") {
                let target = out_dir.join(format!("{}.png", name));
                self.reader
                    .extract_chunk_to_file(&entry.identifier, &target)?;
                extracted.push(target);
            }
        }
        Ok(extracted)
    }

    /// Raw package reader accessor.
    pub fn package_reader(&self) -> &MtkPackageReader {
        &self.reader
    }
}

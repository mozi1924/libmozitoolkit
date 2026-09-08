use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use crate::error::ResourceError;

/// Trait defining a readable Minecraft resource pack source.
pub trait ResourcePack: Send + Sync {
    /// Friendly name or identifier of this pack.
    fn name(&self) -> &str;

    /// Open and read the raw bytes of a file inside the pack given its relative path (e.g. `"assets/minecraft/textures/block/stone.png"`).
    fn open(&self, relative_path: &str) -> Option<Vec<u8>>;

    /// List all relative paths starting with the given prefix.
    fn list_files(&self, prefix: &str) -> Vec<String>;

    /// Check if a relative path exists in this pack.
    fn has_file(&self, relative_path: &str) -> bool {
        self.open(relative_path).is_some()
    }
}

/// A resource pack backed by an unzipped directory on the local filesystem.
pub struct DirectoryPack {
    name: String,
    root_dir: PathBuf,
}

impl DirectoryPack {
    pub fn new(name: impl Into<String>, root_dir: impl AsRef<Path>) -> Self {
        Self {
            name: name.into(),
            root_dir: root_dir.as_ref().to_path_buf(),
        }
    }

    fn normalize_path(path: &str) -> PathBuf {
        let clean = path.replace('\\', "/");
        let path_obj = Path::new(&clean);
        // Prevent path traversal attacks
        let mut normalized = PathBuf::new();
        for comp in path_obj.components() {
            if let std::path::Component::Normal(c) = comp {
                normalized.push(c);
            }
        }
        normalized
    }
}

impl ResourcePack for DirectoryPack {
    fn name(&self) -> &str {
        &self.name
    }

    fn open(&self, relative_path: &str) -> Option<Vec<u8>> {
        let norm = Self::normalize_path(relative_path);
        let full_path = self.root_dir.join(norm);
        if full_path.is_file() {
            fs::read(full_path).ok()
        } else {
            None
        }
    }

    fn list_files(&self, prefix: &str) -> Vec<String> {
        let norm_prefix = prefix.replace('\\', "/");
        let mut results = Vec::new();
        let target_dir = self.root_dir.join(Self::normalize_path(&norm_prefix));
        
        let search_root = if target_dir.is_dir() {
            target_dir
        } else {
            self.root_dir.clone()
        };

        fn walk_dir(root: &Path, current: &Path, prefix_filter: &str, results: &mut Vec<String>) {
            if let Ok(entries) = fs::read_dir(current) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        walk_dir(root, &path, prefix_filter, results);
                    } else if path.is_file() {
                        if let Ok(rel) = path.strip_prefix(root) {
                            let rel_str = rel.to_string_lossy().replace('\\', "/");
                            if rel_str.starts_with(prefix_filter) || prefix_filter.is_empty() {
                                results.push(rel_str);
                            }
                        }
                    }
                }
            }
        }

        walk_dir(&self.root_dir, &search_root, &norm_prefix, &mut results);
        results
    }
}

/// An in-memory resource pack, ideal for unit tests and synthetic assets.
#[derive(Default, Clone)]
pub struct MemoryPack {
    name: String,
    files: HashMap<String, Vec<u8>>,
}

impl MemoryPack {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            files: HashMap::new(),
        }
    }

    pub fn insert(&mut self, path: impl Into<String>, data: impl Into<Vec<u8>>) {
        let clean = path.into().replace('\\', "/");
        self.files.insert(clean, data.into());
    }
}

impl ResourcePack for MemoryPack {
    fn name(&self) -> &str {
        &self.name
    }

    fn open(&self, relative_path: &str) -> Option<Vec<u8>> {
        let clean = relative_path.replace('\\', "/");
        self.files.get(&clean).cloned()
    }

    fn list_files(&self, prefix: &str) -> Vec<String> {
        let norm_prefix = prefix.replace('\\', "/");
        self.files
            .keys()
            .filter(|k| k.starts_with(&norm_prefix))
            .cloned()
            .collect()
    }
}

/// A resource pack backed by a `.zip` or `.jar` archive file.
#[cfg(feature = "zip")]
pub struct ZipPack {
    name: String,
    file_map: HashMap<String, Vec<u8>>,
}

#[cfg(feature = "zip")]
impl ZipPack {
    /// Load and cache all files from a ZIP or JAR archive into memory.
    pub fn from_file(name: impl Into<String>, path: impl AsRef<Path>) -> Result<Self, ResourceError> {
        let file = fs::File::open(path)?;
        let mut archive = zip::ZipArchive::new(file)?;
        let mut file_map = HashMap::new();

        for i in 0..archive.len() {
            let mut file_entry = archive.by_index(i)?;
            if file_entry.is_file() {
                let name = file_entry.name().replace('\\', "/");
                // Zip-Slip attack protection
                if name.contains("..") {
                    continue;
                }
                let mut buf = Vec::with_capacity(file_entry.size() as usize);
                file_entry.read_to_end(&mut buf)?;
                file_map.insert(name, buf);
            }
        }

        Ok(Self {
            name: name.into(),
            file_map,
        })
    }

    /// Load and cache from a raw byte slice.
    pub fn from_bytes(name: impl Into<String>, bytes: &[u8]) -> Result<Self, ResourceError> {
        let cursor = std::io::Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(cursor)?;
        let mut file_map = HashMap::new();

        for i in 0..archive.len() {
            let mut file_entry = archive.by_index(i)?;
            if file_entry.is_file() {
                let name = file_entry.name().replace('\\', "/");
                if name.contains("..") {
                    continue;
                }
                let mut buf = Vec::with_capacity(file_entry.size() as usize);
                file_entry.read_to_end(&mut buf)?;
                file_map.insert(name, buf);
            }
        }

        Ok(Self {
            name: name.into(),
            file_map,
        })
    }
}

#[cfg(feature = "zip")]
impl ResourcePack for ZipPack {
    fn name(&self) -> &str {
        &self.name
    }

    fn open(&self, relative_path: &str) -> Option<Vec<u8>> {
        let clean = relative_path.replace('\\', "/");
        self.file_map.get(&clean).cloned()
    }

    fn list_files(&self, prefix: &str) -> Vec<String> {
        let norm_prefix = prefix.replace('\\', "/");
        self.file_map
            .keys()
            .filter(|k| k.starts_with(&norm_prefix))
            .cloned()
            .collect()
    }
}

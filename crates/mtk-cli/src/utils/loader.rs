use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

use mtk_model::{BlockModelJson, BlockStateDefinition};
use zip::ZipArchive;

pub enum AssetSource {
    Jar {
        zip: ZipArchive<BufReader<File>>,
        file_names: HashMap<String, usize>,
        all_blockstate_names: Vec<String>,
    },
    Directory {
        base_dir: PathBuf,
    },
}

pub struct UniversalAssetLoader {
    source: AssetSource,
    model_cache: HashMap<String, Option<BlockModelJson>>,
    blockstate_cache: HashMap<String, Option<BlockStateDefinition>>,
}

impl UniversalAssetLoader {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let path_ref = path.as_ref();
        if path_ref.is_dir() {
            Ok(Self {
                source: AssetSource::Directory {
                    base_dir: path_ref.to_path_buf(),
                },
                model_cache: HashMap::new(),
                blockstate_cache: HashMap::new(),
            })
        } else {
            let file = File::open(path_ref)?;
            let mut zip = ZipArchive::new(BufReader::new(file))?;
            let mut file_names = HashMap::new();
            let mut all_blockstate_names = Vec::new();

            for i in 0..zip.len() {
                let name = zip.by_index(i)?.name().to_string();
                if name.starts_with("assets/minecraft/blockstates/") && name.ends_with(".json") {
                    if let Some(stem) = name
                        .strip_prefix("assets/minecraft/blockstates/")
                        .and_then(|s| s.strip_suffix(".json"))
                    {
                        all_blockstate_names.push(format!("minecraft:{}", stem));
                    }
                }
                file_names.insert(name, i);
            }
            all_blockstate_names.sort();

            Ok(Self {
                source: AssetSource::Jar {
                    zip,
                    file_names,
                    all_blockstate_names,
                },
                model_cache: HashMap::new(),
                blockstate_cache: HashMap::new(),
            })
        }
    }

    pub fn read_entry_bytes(&mut self, rel_path: &str) -> Option<Vec<u8>> {
        match &mut self.source {
            AssetSource::Jar {
                zip, file_names, ..
            } => {
                let idx = *file_names.get(rel_path)?;
                let mut file = zip.by_index(idx).ok()?;
                let mut data = Vec::new();
                file.read_to_end(&mut data).ok()?;
                Some(data)
            }
            AssetSource::Directory { base_dir } => {
                let full_path = base_dir.join(rel_path);
                fs::read(full_path).ok()
            }
        }
    }

    pub fn read_entry_string(&mut self, rel_path: &str) -> Option<String> {
        self.read_entry_bytes(rel_path)
            .and_then(|bytes| String::from_utf8(bytes).ok())
    }

    pub fn list_all_blockstates(&self) -> Vec<String> {
        match &self.source {
            AssetSource::Jar {
                all_blockstate_names,
                ..
            } => all_blockstate_names.clone(),
            AssetSource::Directory { base_dir } => {
                let mut list = Vec::new();
                let bs_dir = base_dir.join("assets/minecraft/blockstates");
                if let Ok(entries) = fs::read_dir(bs_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.extension().is_some_and(|e| e == "json") {
                            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                                list.push(format!("minecraft:{}", stem));
                            }
                        }
                    }
                }
                list.sort();
                list
            }
        }
    }

    pub fn load_blockstate(&mut self, blockstate_id: &str) -> Option<BlockStateDefinition> {
        let clean_id = blockstate_id.strip_prefix("minecraft:").unwrap_or(blockstate_id);
        if let Some(cached) = self.blockstate_cache.get(clean_id) {
            return cached.clone();
        }

        let (ns, name) = if let Some((ns, n)) = blockstate_id.split_once(':') {
            (ns, n)
        } else {
            ("minecraft", blockstate_id)
        };

        let path = format!("assets/{}/blockstates/{}.json", ns, name);
        let res = self
            .read_entry_string(&path)
            .and_then(|text| serde_json::from_str::<BlockStateDefinition>(&text).ok());

        self.blockstate_cache
            .insert(clean_id.to_string(), res.clone());
        res
    }

    pub fn load_model(&mut self, model_id: &str) -> Option<BlockModelJson> {
        let (ns, raw_path) = if let Some((ns, n)) = model_id.split_once(':') {
            (ns, n)
        } else {
            ("minecraft", model_id)
        };

        let cache_key = format!("{}:{}", ns, raw_path);
        if let Some(cached) = self.model_cache.get(&cache_key) {
            return cached.clone();
        }

        let clean_path = raw_path.strip_suffix(".json").unwrap_or(raw_path);
        let candidates = [
            format!("assets/{}/models/{}.json", ns, clean_path),
            format!("assets/{}/models/block/{}.json", ns, clean_path),
            format!("assets/{}/models/item/{}.json", ns, clean_path),
            format!("assets/{}/{}.json", ns, clean_path),
        ];

        for path in candidates {
            if let Some(text) = self.read_entry_string(&path) {
                if let Ok(model) = serde_json::from_str::<BlockModelJson>(&text) {
                    self.model_cache.insert(cache_key, Some(model.clone()));
                    return Some(model);
                }
            }
        }

        self.model_cache.insert(cache_key, None);
        None
    }
}

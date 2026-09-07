use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use mtk_model::{BlockModelJson, BlockState, BlockStateDefinition, ModelBaker};

struct DiskResourceLoader {
    base_dir: PathBuf,
    model_cache: RwLock<HashMap<String, Option<BlockModelJson>>>,
}

impl DiskResourceLoader {
    fn new(base_dir: impl AsRef<Path>) -> Self {
        Self {
            base_dir: base_dir.as_ref().to_path_buf(),
            model_cache: RwLock::new(HashMap::new()),
        }
    }

    fn load_blockstate(&self, block_id: &str) -> Option<BlockStateDefinition> {
        let (ns, name) = if let Some((ns, n)) = block_id.split_once(':') {
            (ns, n)
        } else {
            ("minecraft", block_id)
        };

        let path = self
            .base_dir
            .join("assets")
            .join(ns)
            .join("blockstates")
            .join(format!("{}.json", name));

        let content = fs::read_to_string(path).ok()?;
        serde_json::from_str(&content).ok()
    }

    fn load_model(&self, model_id: &str) -> Option<BlockModelJson> {
        let (ns, path) = if let Some((ns, p)) = model_id.split_once(':') {
            (ns, p)
        } else {
            ("minecraft", model_id)
        };

        let cache_key = format!("{}:{}", ns, path);
        {
            let cache = self.model_cache.read().unwrap();
            if let Some(cached) = cache.get(&cache_key) {
                return cached.clone();
            }
        }

        let clean_path = path.strip_suffix(".json").unwrap_or(path);
        let rel_json = if clean_path.starts_with("models/") {
            format!("assets/{}/{}.json", ns, clean_path)
        } else {
            format!("assets/{}/models/{}.json", ns, clean_path)
        };

        let full_path = self.base_dir.join(rel_json);
        let result = fs::read_to_string(full_path)
            .ok()
            .and_then(|s| serde_json::from_str::<BlockModelJson>(&s).ok());

        let mut cache = self.model_cache.write().unwrap();
        cache.insert(cache_key, result.clone());
        result
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: dump_baked <mc_resource_dir> <blockstate_str>");
        std::process::exit(1);
    }

    let mc_dir = &args[1];
    let state_str = &args[2];

    let loader = DiskResourceLoader::new(mc_dir);
    let blockstate = match BlockState::parse(state_str) {
        Ok(bs) => bs,
        Err(e) => {
            eprintln!("Error parsing blockstate: {}", e);
            std::process::exit(1);
        }
    };

    let block_id = format!("{}:{}", blockstate.namespace, blockstate.name);
    let def = loader.load_blockstate(&block_id);

    let mut baker = ModelBaker::new();
    match baker.bake_blockstate(state_str, def.as_ref(), |m_id| loader.load_model(m_id)) {
        Ok(baked) => {
            let json = serde_json::to_string_pretty(&baked).expect("Failed to serialize baked model");
            println!("{}", json);
        }
        Err(e) => {
            eprintln!("Error baking blockstate: {}", e);
            std::process::exit(1);
        }
    }
}

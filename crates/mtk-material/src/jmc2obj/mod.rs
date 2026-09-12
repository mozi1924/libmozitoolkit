pub mod aliases;

use aliases::{
    EXPLICIT_MATERIAL_ALIASES, JMC2OBJ_BANNER_SHORT_ALIASES, JMC2OBJ_BIOME_SUFFIXES,
};

/// Clean a raw jmc2obj material or texture name into a normalized identifier.
pub fn clean_jmc2obj_name(raw: &str) -> String {
    let mut s = raw.trim().to_lowercase();

    // 1. Strip file extensions
    if let Some(stripped) = s.strip_suffix(".png").or_else(|| s.strip_suffix(".jpg")) {
        s = stripped.to_string();
    }

    // 2. Strip blender numerical duplicate suffixes (".001", "_001")
    if let Some(idx) = s.rfind('.') {
        if s[idx + 1..].chars().all(|c| c.is_ascii_digit()) {
            s = s[..idx].to_string();
        }
    }

    // 3. Strip standard exporter texture path prefixes
    let prefixes = [
        "tex/minecraft/",
        "textures/block/",
        "textures/entity/",
        "textures/",
        "minecraft_block-",
        "minecraft_entity-",
        "minecraft_item-",
        "minecraft_block_",
        "minecraft_entity_",
        "jmc2obj_block-",
        "jmc2obj_block_",
        "jmc2obj_entity-",
        "jmc2obj_",
        "minecraft:",
        "minecraft-",
        "tile_",
        "tile-",
        "tile.",
        "block_",
        "block-",
        "block.",
    ];

    for prefix in prefixes {
        if s.starts_with(prefix) {
            s = s[prefix.len()..].to_string();
            break;
        }
    }

    // 4. Strip biome suffixes
    for &suffix in JMC2OBJ_BIOME_SUFFIXES {
        if s.ends_with(suffix) {
            s = s[..s.len() - suffix.len()].to_string();
            break;
        }
    }

    // Replace spaces and hyphens with underscores in block stems if not in a namespace path
    if !s.contains('/') {
        s = s.replace(' ', "_").replace('-', "_");
    }

    s
}

/// Resolve cleaned jmc2obj name to a list of candidate texture paths (e.g. `["block/stone"]`).
pub fn resolve_jmc2obj_candidates(cleaned_name: &str) -> Vec<String> {
    let mut candidates = Vec::new();

    // 1. Check Banner short aliases
    for &(short_code, mapped_path) in JMC2OBJ_BANNER_SHORT_ALIASES {
        if cleaned_name == short_code || cleaned_name.ends_with(short_code) {
            candidates.push(mapped_path.to_string());
            return candidates;
        }
    }

    // 2. Check explicit block/entity aliases
    for &(key, paths) in EXPLICIT_MATERIAL_ALIASES {
        if cleaned_name == key {
            for &p in paths {
                candidates.push(p.to_string());
            }
            return candidates;
        }
    }

    // 3. If it already contains a category slash (e.g. "entity/chest/normal" or "block/stone")
    if cleaned_name.contains('/') {
        candidates.push(cleaned_name.to_string());
        return candidates;
    }

    // 4. Strip common face/export suffixes (_all, _pattern, _side, _top, _bottom)
    let suffixes = [
        "_all",
        "_pattern",
        "_side",
        "_top",
        "_bottom",
        "_front",
        "_back",
        "_end",
        "_inner",
        "_base",
    ];

    let mut stripped_stem = cleaned_name.to_string();
    for suffix in suffixes {
        if stripped_stem.ends_with(suffix) {
            stripped_stem = stripped_stem[..stripped_stem.len() - suffix.len()].to_string();
            break;
        }
    }

    // 5. Default prefix guessing: try "block/{name}", then "entity/{name}", "item/{name}"
    candidates.push(format!("block/{}", cleaned_name));
    if stripped_stem != cleaned_name {
        candidates.push(format!("block/{}", stripped_stem));
    }
    candidates.push(format!("entity/{}", cleaned_name));
    if stripped_stem != cleaned_name {
        candidates.push(format!("entity/{}", stripped_stem));
    }
    candidates.push(format!("item/{}", cleaned_name));

    candidates
}

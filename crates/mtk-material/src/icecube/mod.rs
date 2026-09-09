/// Clean an Ice-Cube material identifier.
pub fn clean_icecube_name(raw: &str) -> String {
    let mut s = raw.trim().to_lowercase().replace(' ', "_");

    // 1. Strip file extensions
    if let Some(stripped) = s.strip_suffix(".png").or_else(|| s.strip_suffix(".jpg")) {
        s = stripped.to_string();
    }

    // 2. Strip Blender numerical suffixes (".001", "_001")
    if let Some(idx) = s.rfind('.') {
        if s[idx + 1..].chars().all(|c| c.is_ascii_digit()) {
            s = s[..idx].to_string();
        }
    }

    // 3. Strip Ice Cube specific prefixes
    let prefixes = [
        "ice_cube_block_",
        "ice_cube_entity_",
        "ice_cube_item_",
        "icecube_block_",
        "icecube_entity_",
        "icecube_item_",
        "icecube_",
        "ice_cube_",
        "m_block_",
        "m_entity_",
        "m_item_",
        "m_",
    ];

    for prefix in prefixes {
        if s.starts_with(prefix) {
            s = s[prefix.len()..].to_string();
            break;
        }
    }

    s.replace('-', "_")
}

/// Resolve cleaned Ice-Cube name to candidate texture paths.
pub fn resolve_icecube_candidates(cleaned: &str) -> Vec<String> {
    let mut candidates = Vec::new();

    // 1. Direct path if already structured
    if cleaned.contains('/') {
        candidates.push(cleaned.to_string());
        return candidates;
    }

    // 2. Common Ice Cube specific entity aliases
    let entity_aliases = [
        ("zombified_piglin", "entity/piglin/zombified_piglin"),
        ("zombie_villager", "entity/zombie_villager/zombie_villager"),
        ("zombie_horse", "entity/horse/horse_zombie"),
        ("zombie_head", "entity/zombie/zombie"),
        ("zombie", "entity/zombie/zombie"),
        ("creeper_head", "entity/creeper/creeper"),
        ("creeper", "entity/creeper/creeper"),
        ("skeleton_head", "entity/skeleton/skeleton"),
        ("skeleton", "entity/skeleton/skeleton"),
        ("wither_skeleton_head", "entity/skeleton/wither_skeleton"),
        ("wither_skeleton", "entity/skeleton/wither_skeleton"),
        ("piglin_head", "entity/piglin/piglin"),
        ("piglin", "entity/piglin/piglin"),
        ("dragon_head", "entity/enderdragon/dragon"),
        ("dragon", "entity/enderdragon/dragon"),
        ("player_head", "entity/player/wide/steve"),
        ("steve", "entity/player/wide/steve"),
        ("alex", "entity/player/slim/alex"),
        ("zoglin", "entity/hoglin/zoglin"),
        ("hoglin", "entity/hoglin/hoglin"),
        ("pig", "entity/pig/pig"),
        ("cow", "entity/cow/cow"),
        ("sheep", "entity/sheep/sheep"),
        ("chicken", "entity/chicken/chicken"),
    ];

    for &(src, dst) in &entity_aliases {
        if cleaned == src {
            candidates.push(dst.to_string());
            return candidates;
        }
    }

    // 3. Strip Ice Cube face/variant suffixes (_all, _pattern, _top, _bottom, _side)
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

    let mut stripped_stem = cleaned.to_string();
    for suffix in suffixes {
        if stripped_stem.ends_with(suffix) {
            stripped_stem = stripped_stem[..stripped_stem.len() - suffix.len()].to_string();
            break;
        }
    }

    // 4. Default resolution candidates: try block/{cleaned}, block/{stripped_stem}, entity/{...}, item/{...}
    candidates.push(format!("block/{}", cleaned));
    if stripped_stem != cleaned {
        candidates.push(format!("block/{}", stripped_stem));
    }
    candidates.push(format!("entity/{}", cleaned));
    if stripped_stem != cleaned {
        candidates.push(format!("entity/{}", stripped_stem));
    }
    candidates.push(format!("item/{}", cleaned));

    candidates
}

/// Clean an Ice-Cube material identifier.
pub fn clean_icecube_name(raw: &str) -> String {
    let mut s = raw.trim().to_lowercase();

    if let Some(stripped) = s.strip_suffix(".png").or_else(|| s.strip_suffix(".jpg")) {
        s = stripped.to_string();
    }

    if let Some(idx) = s.rfind('.') {
        if s[idx + 1..].chars().all(|c| c.is_ascii_digit()) {
            s = s[..idx].to_string();
        }
    }

    let prefixes = [
        "ice_cube_block_",
        "ice_cube_entity_",
        "icecube_block_",
        "icecube_",
        "ice_cube_",
        "m_block_",
        "m_entity_",
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
    if cleaned.contains('/') {
        vec![cleaned.to_string()]
    } else {
        vec![
            format!("block/{}", cleaned),
            format!("entity/{}", cleaned),
            format!("item/{}", cleaned),
        ]
    }
}

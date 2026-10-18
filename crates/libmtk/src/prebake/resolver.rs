//! Host-agnostic active `.mtkcache` resolver.
//!
//! Blender and C4D both need to pick the cache package that matches the active
//! resource-pack stack fingerprint. Keeping the scan/validation policy here
//! avoids re-implementing glob/mtime/header rules in every host.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::reader::AssetCacheReader;

fn mtime(path: &Path) -> SystemTime {
    path.metadata()
        .and_then(|m| m.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH)
}

fn newest(mut candidates: Vec<PathBuf>) -> Option<PathBuf> {
    candidates.sort_by_key(|p| mtime(p));
    candidates.pop()
}

/// Resolve the active cache package in `cache_dir`.
///
/// * `fingerprint = Some(fp)`: prefer exact direct names, then scan all
///   `.mtkcache` files and validate their package header fingerprint. Returns
///   the newest matching package, or `None` if none match.
/// * `fingerprint = None`: returns the newest `.mtkcache` package.
pub fn resolve_active_cache_package(
    cache_dir: impl AsRef<Path>,
    fingerprint: Option<&str>,
) -> Option<PathBuf> {
    let dir = cache_dir.as_ref();
    if !dir.is_dir() {
        return None;
    }

    if let Some(fp) = fingerprint.filter(|value| !value.trim().is_empty()) {
        for name in [
            format!("{fp}.mtkcache"),
            format!("mtk_fp_{fp}.mtkcache"),
            fp.to_string(),
        ] {
            let candidate = dir.join(name);
            if AssetCacheReader::is_valid_cache_file(&candidate, fp) {
                return Some(candidate);
            }
        }

        let matching: Vec<PathBuf> = std::fs::read_dir(dir)
            .ok()?
            .filter_map(|entry| entry.ok().map(|e| e.path()))
            .filter(|path| path.extension().is_some_and(|ext| ext == "mtkcache"))
            .filter(|path| AssetCacheReader::is_valid_cache_file(path, fp))
            .collect();
        return newest(matching);
    }

    let all: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "mtkcache"))
        .filter(|path| path.is_file())
        .collect();
    newest(all)
}

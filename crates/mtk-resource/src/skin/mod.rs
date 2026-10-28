//! Minecraft player skin resolution, multi-tier caching, and prebaked entity fallback system.
//!
//! Handles official Mojang CDNs, custom CDNs (e.g. LittleSkin, Ely.by),
//! negative caching for 404/invalid URLs, non-blocking disk/memory checks,
//! background HTTP fetching, and 0-second indexed local cache hits.

pub mod index;
pub mod mod_api;
pub mod resolver;

pub use index::{
    clean_player_name, clean_uuid, compute_sha256_hex, current_unix_timestamp, SkinIndex,
    SkinRecord,
};
pub use mod_api::{
    default_player_skin_texture, derive_skin_key, VANILLA_ALEX_TEXTURE, VANILLA_STEVE_TEXTURE,
};
pub use resolver::{extract_skin_url_from_property_base64, is_valid_png_header, SkinResolver};

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_derive_skin_key() {
        // Mojang hash
        let hash = "3c3fa837d5e9ae5d06150a699495387d068fb9e682572b191e6868959f51bc52";
        let url = format!("http://textures.minecraft.net/texture/{}", hash);
        assert_eq!(derive_skin_key(Some(&url), None, None), hash);

        // Custom CDN
        let custom_url = "https://littleskin.cn/textures/custom_user_123.png?v=1";
        let cdn_key = derive_skin_key(Some(custom_url), None, None);
        assert!(cdn_key.starts_with("cdn_"));

        // UUID fallback (with and without uuid_ prefix)
        assert_eq!(
            derive_skin_key(None, Some("4382c7a9-11f7-41c4-81fd-1592971e9eac"), None),
            "uuid_4382c7a911f741c481fd1592971e9eac"
        );
        assert_eq!(
            derive_skin_key(
                None,
                Some("uuid_4382c7a9-11f7-41c4-81fd-1592971e9eac"),
                None
            ),
            "uuid_4382c7a911f741c481fd1592971e9eac"
        );

        // Player name fallback (with and without player_ prefix)
        assert_eq!(
            derive_skin_key(None, None, Some("JaxLocke")),
            "player_jaxlocke"
        );
        assert_eq!(
            derive_skin_key(None, None, Some("player_JaxLocke")),
            "player_jaxlocke"
        );
    }

    #[test]
    fn test_default_player_skin_texture() {
        assert_eq!(
            default_player_skin_texture(None),
            "minecraft:entity/player/wide/steve"
        );
        let path = default_player_skin_texture(Some("00000000-0000-0000-0000-000000000001"));
        assert!(path == VANILLA_STEVE_TEXTURE || path == VANILLA_ALEX_TEXTURE);
    }

    #[test]
    fn test_memory_cache_registration() {
        let resolver = SkinResolver::new(None);
        let fake_png = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x01];
        let key = derive_skin_key(None, None, Some("custom_key_1"));
        resolver.register_memory_skin(&key, fake_png.clone());
        let fetched = resolver.try_resolve_cached(None, None, Some("custom_key_1"));
        assert_eq!(fetched, Some(fake_png));
    }

    #[test]
    fn test_negative_cache_and_resolution() {
        let resolver = SkinResolver::new(None).with_negative_cache_ttl(1);
        let invalid_url = "http://127.0.0.1:59996/invalid_skin.png";
        let png = resolver.resolve_skin_png(Some(invalid_url), None, None);
        assert!(png.is_none());

        // Negative cache hit within TTL
        assert!(resolver
            .try_resolve_cached(Some(invalid_url), None, None)
            .is_none());

        // Sleep to exceed 1s negative cache TTL
        std::thread::sleep(std::time::Duration::from_millis(1100));

        // After TTL expired, negative cache allows retry
        assert!(resolver
            .try_resolve_cached(Some(invalid_url), None, None)
            .is_none());
    }

    #[test]
    fn test_index_mapping_and_zero_second_cache_hit() {
        let temp_dir =
            std::env::temp_dir().join(format!("mtk_test_skin_idx_{}", current_unix_timestamp()));
        let _ = fs::create_dir_all(&temp_dir);

        let fake_png = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0xAA, 0xBB];
        let hash = compute_sha256_hex(&fake_png);
        let png_path = temp_dir.join(format!("{}.png", hash));
        fs::write(&png_path, &fake_png).unwrap();

        // 1. Resolver opens directory, should record hash into index
        let resolver = SkinResolver::new(Some(temp_dir.clone()));
        assert!(resolver.index.read().unwrap().skins.contains_key(&hash));

        // 2. Associate player and UUID with this hash
        {
            let mut idx = resolver.index.write().unwrap();
            idx.record_mapping(
                &hash,
                None,
                Some("eea5f812-2bde-3362-93f3-688fdbf6ab6e"),
                Some("JaxLocke"),
                1000,
            );
            idx.save_to_disk(&temp_dir).unwrap();
        }

        // 3. New resolver in another session: 0-second cache hit by name and offline UUID
        let resolver2 = SkinResolver::new(Some(temp_dir.clone()));
        let resolved_by_name = resolver2.try_resolve_cached(None, None, Some("jaxlocke"));
        assert_eq!(resolved_by_name, Some(fake_png.clone()));

        let resolved_by_uuid =
            resolver2.try_resolve_cached(None, Some("uuid_eea5f8122bde336293f3688fdbf6ab6e"), None);
        assert_eq!(resolved_by_uuid, Some(fake_png));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_legacy_file_automatic_migration() {
        let temp_dir =
            std::env::temp_dir().join(format!("mtk_test_skin_mig_{}", current_unix_timestamp()));
        let _ = fs::create_dir_all(&temp_dir);

        let fake_png = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x11, 0x22];
        let hash = compute_sha256_hex(&fake_png);

        // Create legacy files
        let legacy_player = temp_dir.join("player_jaxlocke.png");
        let legacy_uuid = temp_dir.join("uuid_eea5f8122bde336293f3688fdbf6ab6e.png");
        fs::write(&legacy_player, &fake_png).unwrap();
        fs::write(&legacy_uuid, &fake_png).unwrap();

        // Instantiate resolver: load_and_migrate runs
        let resolver = SkinResolver::new(Some(temp_dir.clone()));

        // Legacy files should be removed and migrated to <hash>.png
        assert!(!legacy_player.exists());
        assert!(!legacy_uuid.exists());
        assert!(temp_dir.join(format!("{}.png", hash)).exists());

        // Check index contains both mappings
        let idx = resolver.index.read().unwrap();
        assert_eq!(idx.players.get("jaxlocke"), Some(&hash));
        assert_eq!(
            idx.uuids.get("eea5f8122bde336293f3688fdbf6ab6e"),
            Some(&hash)
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    #[cfg(all(feature = "net", not(target_arch = "wasm32")))]
    fn test_resolve_real_mojang_skin_texture() {
        let resolver = SkinResolver::new(None);
        let real_url = "http://textures.minecraft.net/texture/3c3fa837d5e9ae5d06150a699495387d068fb9e682572b191e6868959f51bc52";
        let png = resolver.resolve_skin_png(Some(real_url), None, None);
        if let Some(bytes) = png {
            assert!(is_valid_png_header(&bytes));
            assert_eq!(bytes.len(), 3648);
        }
    }
}

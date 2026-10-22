//! Verifies that cached compatibility overrides are installed when a cache opens.

use std::io::Cursor;

use libmtk::AssetCacheReader;
use mtk_package::{ChunkWriteOptions, MtkPackageWriter, PackageProfile};

#[test]
fn cache_resolver_overrides_are_installed_on_open() {
    let mut cursor = Cursor::new(Vec::new());
    let mut writer =
        MtkPackageWriter::new(&mut cursor, PackageProfile::AssetCache, [0x43; 16]).unwrap();
    writer
        .add_str_chunk(
            *b"COMP",
            "resolver/overrides",
            r#"{"strip_biome_suffixes":["_custom"],"material_fallbacks":[]}"#,
            &ChunkWriteOptions::raw(),
        )
        .unwrap();
    writer.finish().unwrap();

    let path = std::env::temp_dir().join(format!(
        "mtk_resolver_overrides_{}_{}.mtkcache",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::write(&path, cursor.into_inner()).unwrap();

    let _reader = AssetCacheReader::open(&path).expect("open cache");
    assert_eq!(
        mtk_material::resolver::clean_identifier("stone_custom"),
        "stone"
    );

    mtk_material::resolver::clear_material_overrides();
    let _ = std::fs::remove_file(path);
}

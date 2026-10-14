use mtk_package::{
    ChunkWriteOptions, MtkPackageReader, MtkPackageWriter, PackageError, PackageHeader,
    PackageProfile, TableOfContents, HEADER_SIZE,
};
use std::io::Cursor;

#[test]
fn test_package_roundtrip_memory() {
    let mut cursor = Cursor::new(Vec::new());
    let fingerprint = [0xAB; 16];
    let mut writer =
        MtkPackageWriter::new(&mut cursor, PackageProfile::AssetCache, fingerprint).unwrap();

    // 1. Zstd compressed chunk
    let sample_model_data = b"MODEL_DATA_BLOCK_1234567890_REPEAT_FOR_COMPRESSION_".repeat(20);
    writer
        .add_chunk(
            *b"MODL",
            "models/database",
            &sample_model_data,
            &ChunkWriteOptions::zstd_fast(),
        )
        .unwrap();

    // 2. Raw uncompressed chunk (e.g. PNG texture)
    let sample_png = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 1, 2, 3, 4];
    writer
        .add_chunk(
            *b"ATLS",
            "atlas/chunk_001.png",
            &sample_png,
            &ChunkWriteOptions::raw(),
        )
        .unwrap();

    // 3. JSON chunk
    #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
    struct TestManifest {
        packs: Vec<String>,
        count: u32,
    }
    let manifest = TestManifest {
        packs: vec!["vanilla".to_string(), "faithful".to_string()],
        count: 42,
    };
    writer
        .add_json_chunk(
            *b"META",
            "manifest",
            &manifest,
            &ChunkWriteOptions::zstd_fast(),
        )
        .unwrap();

    let header = writer.finish().unwrap();
    assert_eq!(header.chunk_count, 3);
    assert_eq!(header.fingerprint, fingerprint);

    let bytes = cursor.into_inner();
    assert!(bytes.len() > HEADER_SIZE);

    // Read back
    let reader = MtkPackageReader::from_bytes(bytes).unwrap();
    assert_eq!(reader.header().profile, PackageProfile::AssetCache);
    assert_eq!(reader.fingerprint(), &fingerprint);
    assert_eq!(reader.toc().entries.len(), 3);

    // Verify 64-byte alignment of all entries
    for entry in &reader.toc().entries {
        assert_eq!(
            entry.offset % 64,
            0,
            "Chunk '{}' offset {} is not 64-byte aligned",
            entry.identifier,
            entry.offset
        );
    }
    assert_eq!(
        reader.header().toc_offset % 64,
        0,
        "TOC offset {} is not 64-byte aligned",
        reader.header().toc_offset
    );

    // Verify data integrity
    let read_models = reader.read_chunk_decompressed("models/database").unwrap();
    assert_eq!(read_models, sample_model_data);

    let read_png = reader
        .read_chunk_decompressed("atlas/chunk_001.png")
        .unwrap();
    assert_eq!(read_png, sample_png);

    let read_manifest: TestManifest = reader.read_chunk_json("manifest").unwrap();
    assert_eq!(read_manifest, manifest);
}

#[test]
fn test_package_file_and_header_only() {
    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("test.mtkcache");

    let fingerprint = [0x55; 16];
    let mut writer =
        MtkPackageWriter::create(&file_path, PackageProfile::AssetCache, fingerprint).unwrap();

    writer
        .add_str_chunk(
            *b"BIOM",
            "biome/mapping",
            r#"{"biomes":["plains","forest"]}"#,
            &ChunkWriteOptions::zstd_fast(),
        )
        .unwrap();

    writer.finish().unwrap();

    // Verify header-only reading (microsecond check)
    let header_only = MtkPackageReader::read_header_only(&file_path).unwrap();
    assert_eq!(header_only.fingerprint, fingerprint);
    assert_eq!(header_only.chunk_count, 1);

    // Verify mmap reader
    let reader = MtkPackageReader::open_file(&file_path).unwrap();
    assert!(reader.has_chunk("biome/mapping"));
    let str_val = reader.read_chunk_str("biome/mapping").unwrap();
    assert_eq!(str_val, r#"{"biomes":["plains","forest"]}"#);

    // Verify buffered reader (safe path)
    let buffered_reader = MtkPackageReader::open_file_buffered(&file_path).unwrap();
    assert!(buffered_reader.has_chunk("biome/mapping"));
    let str_val_buf = buffered_reader.read_chunk_str("biome/mapping").unwrap();
    assert_eq!(str_val_buf, r#"{"biomes":["plains","forest"]}"#);
}

#[test]
fn test_toc_forged_count_rejected() {
    // A forged chunk count must not trigger a huge pre-allocation; it is rejected
    // because the payload cannot possibly hold that many entries.
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&u32::MAX.to_le_bytes());
    bytes.extend_from_slice(&[0u8; 8]);

    match TableOfContents::decode_bytes(&bytes) {
        Err(PackageError::InvalidToc(_)) => {}
        other => panic!("Expected InvalidToc, got {:?}", other.is_ok()),
    }
}

#[test]
fn test_forged_toc_offsets_rejected() {
    let mut header = PackageHeader::new_asset_cache([0x11; 16], 0);
    header.toc_offset = u64::MAX;
    header.toc_length = 2;

    let bytes = header.to_bytes().to_vec();
    match MtkPackageReader::from_bytes(bytes) {
        Err(PackageError::OutOfBounds { .. }) => {}
        other => panic!("Expected OutOfBounds, got {:?}", other.is_ok()),
    }
}

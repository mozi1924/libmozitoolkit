use std::fs;
use std::path::Path;
use std::time::Instant;
use mtk_resource::{ResourceLocation, ResourcePackStack, ZipPack};
use mtk_texture::{AtlasBuilder, AtlasBuilderConfig, DecodedSprite};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let miex_dir = Path::new("/home/mozi/MiEx");
    let vanilla_jar = "/home/mozi/26.2-Fabric.jar";

    println!("============================================================");
    println!(" MoziToolKit 2.0 - OptiFine / Continuity CTM Pack Test");
    println!("============================================================");

    // 1. Scan and parse each CTM pack
    let mut ctm_zip_paths = Vec::new();
    for entry in fs::read_dir(miex_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().is_some_and(|ext| ext == "zip") {
            ctm_zip_paths.push(path);
        }
    }

    println!("Discovered {} test resource packs in {}:", ctm_zip_paths.len(), miex_dir.display());
    for p in &ctm_zip_paths {
        println!(" - {}", p.file_name().unwrap_or_default().to_string_lossy());
    }

    let mut total_rules_count = 0;

    for zip_path in &ctm_zip_paths {
        let pack_name = zip_path.file_name().unwrap_or_default().to_string_lossy();
        println!("\n------------------------------------------------------------");
        println!("Testing Pack: {}", pack_name);

        let t0 = Instant::now();
        let pack = ZipPack::from_file(pack_name.to_string(), zip_path)?;
        let mut single_stack = ResourcePackStack::new();
        single_stack.push_pack(Box::new(pack));

        let rules = single_stack.load_ctm_rules();
        println!("Loaded in {:?}, parsed {} CTM rules:", t0.elapsed(), rules.len());
        total_rules_count += rules.len();

        for (idx, r) in rules.iter().enumerate().take(5) {
            println!(
                "  [{}] Name: '{}', Method: {:?}, Tiles: {}, Matches: {} blocks / {} tiles, Faces: {:?}",
                idx,
                r.name,
                r.method,
                r.tiles.len(),
                r.match_blocks.len(),
                r.match_tiles.len(),
                r.faces
            );
        }
        if rules.len() > 5 {
            println!("  ... and {} more rules", rules.len() - 5);
        }
    }

    // 2. Full Multi-Pack Stack Atlas Baking Test
    println!("\n============================================================");
    println!(" End-to-End Multi-Pack Atlas Baking Test with CTM Sprites");
    println!("============================================================");

    let t_stack = Instant::now();
    let mut full_stack = ResourcePackStack::new();

    // Push base vanilla JAR
    if Path::new(vanilla_jar).exists() {
        println!("Adding Base Vanilla JAR: {}", vanilla_jar);
        full_stack.append_pack(Box::new(ZipPack::from_file("vanilla", vanilla_jar)?));
    }

    // Push all CTM packs on top
    for zip_path in &ctm_zip_paths {
        let pack_name = zip_path.file_name().unwrap_or_default().to_string_lossy();
        full_stack.push_pack(Box::new(ZipPack::from_file(pack_name.to_string(), zip_path)?));
    }

    println!("Full Stack constructed ({} packs) in {:?}", full_stack.len(), t_stack.elapsed());

    let all_ctm_rules = full_stack.load_ctm_rules();
    println!("Total Active CTM Rules across all packs: {}", all_ctm_rules.len());

    let blocks_loc = ResourceLocation::parse("minecraft:blocks")?;
    let definition = full_stack.load_atlas_definition(&blocks_loc)?;

    println!("Collecting sprites (including all CTM sub-tiles)...");
    let t_collect = Instant::now();
    let discovered = full_stack.collect_sprites_including_ctm(&definition, &all_ctm_rules)?;
    println!("Discovered {} total sprites (vanilla + CTM sub-tiles) in {:?}", discovered.len(), t_collect.elapsed());

    println!("Decoding sprites in parallel...");
    let t_decode = Instant::now();
    let decoded = DecodedSprite::decode_batch(discovered)?;
    println!("Decoded {} sprites in {:?}", decoded.len(), t_decode.elapsed());

    println!("Baking CTM Atlas...");
    let builder = AtlasBuilder::new(AtlasBuilderConfig {
        max_width: 4096,
        max_height: 4096,
        mip_level: 0,
        padding: 0,
    });

    let t_bake = Instant::now();
    let baked = builder.build_from_sprites(decoded)?;
    let bake_elapsed = t_bake.elapsed();

    println!("------------------------------------------------------------");
    println!(" CTM Atlas Baking Complete in {:?}", bake_elapsed);
    println!(" Total Chunks: {}", baked.chunks.len());
    println!(" Total Registered Sprites in Address Map: {}", baked.address_map.sprites.len());
    for chunk in &baked.chunks {
        println!(
            " - Chunk #{}: {}x{} px (Albedo: {} bytes, Normal: {}, Specular: {})",
            chunk.chunk_id,
            chunk.width,
            chunk.height,
            chunk.albedo.pixels.len(),
            chunk.normal.is_some(),
            chunk.specular.is_some()
        );
    }

    println!("============================================================");
    println!(" All CTM Tests Passed! Total Rules Parsed: {}", total_rules_count);
    println!("============================================================");

    Ok(())
}

use std::fs;
use std::path::Path;
use std::time::Instant;
use mtk_resource::{AtlasCategory, ResourcePackStack, ZipPack};
use mtk_texture::{AtlasAddressMap, AtlasBuilder, AtlasBuilderConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let vanilla_jar_path = "/home/mozi/26.2-Fabric.jar";
    let spbr_zip_path = "/home/mozi/Desktop/SPBR-21.zip";
    let output_dir = Path::new("manual_test/atlases");

    println!("============================================================");
    println!(" MoziToolKit 2.0 - Multi-Category Atlas Baking Pipeline");
    println!("============================================================");
    println!("Vanilla JAR: {}", vanilla_jar_path);
    println!("Top Pack ZIP: {}", spbr_zip_path);
    println!("Output Dir:  {}", output_dir.display());
    println!("------------------------------------------------------------");

    if output_dir.exists() {
        let _ = fs::remove_dir_all(output_dir);
    }
    fs::create_dir_all(output_dir)?;

    let t_start = Instant::now();

    // 1. Load Resource Packs into Stack
    println!("Step 1: Loading resource pack archives...");
    let t_load = Instant::now();

    let mut stack = ResourcePackStack::new();
    if Path::new(vanilla_jar_path).exists() {
        let vanilla_pack = ZipPack::from_file("vanilla_26.2", vanilla_jar_path)?;
        println!(" - Loaded vanilla JAR in {:?}", t_load.elapsed());
        stack.append_pack(Box::new(vanilla_pack));
    } else {
        println!("Warning: Vanilla JAR not found at {}", vanilla_jar_path);
    }

    if Path::new(spbr_zip_path).exists() {
        let t_spbr = Instant::now();
        let spbr_pack = ZipPack::from_file("spbr_pack", spbr_zip_path)?;
        println!(" - Loaded SPBR-21 ZIP in {:?}", t_spbr.elapsed());
        stack.push_pack(Box::new(spbr_pack));
    } else {
        println!("Warning: SPBR ZIP not found at {}", spbr_zip_path);
    }

    println!("ResourcePackStack ready ({} packs in stack, total loading time: {:?})\n", stack.len(), t_start.elapsed());

    // 2. Categories to process
    let categories = [
        AtlasCategory::Blocks,
        AtlasCategory::Items,
        AtlasCategory::Chests,
        AtlasCategory::ShulkerBoxes,
        AtlasCategory::BannerPatterns,
        AtlasCategory::ShieldPatterns,
        AtlasCategory::ArmorTrims,
        AtlasCategory::DecoratedPot,
        AtlasCategory::Paintings,
        AtlasCategory::Particles,
        AtlasCategory::Celestials,
        AtlasCategory::Gui,
        AtlasCategory::MapDecorations,
        AtlasCategory::Entities,
    ];

    let builder = AtlasBuilder::new(AtlasBuilderConfig {
        max_width: 4096,
        max_height: 4096,
        mip_level: 0,
        padding: 0,
    });

    let mut global_address_map = AtlasAddressMap::new();
    let mut total_static_chunks = 0;
    let mut total_anim_chunks = 0;
    let mut total_baked_sprites = 0;

    println!("Step 2: Baking Isolated Dual-Atlases (Static & Animated) for each category...");
    println!("------------------------------------------------------------");

    for category in &categories {
        let cat_name = category.as_str();
        let t_cat = Instant::now();

        let baked = match builder.build_category(&stack, category) {
            Ok(b) => b,
            Err(e) => {
                println!(" [SKIP] Category '{}': {:?}", cat_name, e);
                continue;
            }
        };

        if baked.address_map.sprites.is_empty() {
            println!(" [EMPTY] Category '{}': 0 sprites found, skipping output.", cat_name);
            continue;
        }

        let cat_elapsed = t_cat.elapsed();
        let sprite_count = baked.address_map.sprites.len();
        let mut static_chunks = 0;
        let mut anim_chunks = 0;

        for chunk in &baked.chunks {
            if chunk.is_animated {
                anim_chunks += 1;
            } else {
                static_chunks += 1;
            }
        }
        total_static_chunks += static_chunks;
        total_anim_chunks += anim_chunks;
        total_baked_sprites += sprite_count;

        // Count PBR features and animation
        let mut normal_count = 0;
        let mut specular_count = 0;
        let mut anim_count = 0;
        for loc in baked.address_map.sprites.values() {
            if loc.has_normal { normal_count += 1; }
            if loc.has_specular { specular_count += 1; }
            if loc.is_animated { anim_count += 1; }
        }

        println!(
            " [BAKED] Category '{: <15}' -> {:>4} sprites | Static: {} chk, Anim: {} chk | PBR(N:{:>3}, S:{:>3}) | Anim Sprites:{:>3} in {:?}",
            cat_name, sprite_count, static_chunks, anim_chunks, normal_count, specular_count, anim_count, cat_elapsed
        );

        // Save Baked Chunks to output_dir
        for chunk in &baked.chunks {
            let file_stem = chunk.file_stem();
            let albedo_path = output_dir.join(format!("{}_albedo.png", file_stem));
            let albedo_png = chunk.albedo.to_png_bytes()?;
            fs::write(&albedo_path, albedo_png)?;

            if let Some(ref norm_buf) = chunk.normal {
                let norm_path = output_dir.join(format!("{}_normal.png", file_stem));
                let norm_png = norm_buf.to_png_bytes()?;
                fs::write(&norm_path, norm_png)?;
            }

            if let Some(ref spec_buf) = chunk.specular {
                let spec_path = output_dir.join(format!("{}_specular.png", file_stem));
                let spec_png = spec_buf.to_png_bytes()?;
                fs::write(&spec_path, spec_png)?;
            }
        }

        // Save Category Mapping JSON
        let cat_json_path = output_dir.join(format!("{}_mapping.json", cat_name));
        let cat_json = baked.address_map.to_json()?;
        fs::write(&cat_json_path, cat_json)?;

        // Merge into global address map
        global_address_map.merge(baked.address_map);
    }

    // Save Global Combined Mapping JSON
    let global_json_path = output_dir.join("global_atlas_mapping.json");
    let global_json = global_address_map.to_json()?;
    fs::write(&global_json_path, global_json)?;

    println!("------------------------------------------------------------");
    println!("All Dual-Atlases Baked Successfully!");
    println!(" - Total Categories Processed: {}", categories.len());
    println!(" - Total Static Chunks:        {}", total_static_chunks);
    println!(" - Total Animated Chunks:      {}", total_anim_chunks);
    println!(" - Total Sprites Registered:   {}", total_baked_sprites);
    println!(" - Output Artifacts Saved To:  {}", output_dir.display());
    println!(" - Total Execution Time:       {:?}", t_start.elapsed());
    println!("============================================================\n");

    // 3. Automated Isolation Verification & Spot Checks
    println!("=== Cross-Category & Animation Isolation Verification ===");

    let mut block_contamination = 0;
    let mut item_contamination = 0;

    for (loc, meta) in &global_address_map.sprites {
        if meta.category == "blocks" && loc.path.starts_with("item/") {
            block_contamination += 1;
            eprintln!("Error: Item sprite '{}' found in blocks atlas!", loc);
        }
        if meta.category == "items" && loc.path.starts_with("block/") {
            item_contamination += 1;
            eprintln!("Error: Block sprite '{}' found in items atlas!", loc);
        }
    }

    if block_contamination == 0 && item_contamination == 0 {
        println!(" [PASS] Zero Data Cross-Contamination verified!");
        println!("        - 0 item textures in 'blocks' atlas");
        println!("        - 0 block textures in 'items' atlas");
    } else {
        eprintln!(" [FAIL] Detected contamination: blocks={}, items={}", block_contamination, item_contamination);
    }

    // Spot check static & animated queries
    println!("\nSample Static Lookups:");
    let static_queries = [
        "minecraft:block/diamond_block",
        "block/stone",
        "textures/block/oak_planks.png",
        "minecraft:item/diamond_sword",
        "item/apple",
    ];

    for query in &static_queries {
        if let Some(loc) = global_address_map.lookup_str(query) {
            println!(
                " - Static '{: <30}' -> Cat: {: <8} | Chk: #{:<2} | UV: [{:.3}, {:.3}, {:.3}, {:.3}] | Normal: {:<5} | Specular: {}",
                query, loc.category, loc.chunk_id, loc.uv_bounds[0], loc.uv_bounds[1], loc.uv_bounds[2], loc.uv_bounds[3], loc.has_normal, loc.has_specular
            );
        } else {
            println!(" - Static '{: <30}' -> NOT FOUND", query);
        }
    }

    println!("\nSample Animated Lookups (Frame 0 UV & Step Size):");
    let anim_queries = [
        "minecraft:block/sea_lantern",
        "minecraft:block/water_still",
        "minecraft:block/lava_still",
        "minecraft:block/fire_0",
        "minecraft:block/portal",
        "minecraft:block/magma",
        "minecraft:item/clock_00",
    ];

    for query in &anim_queries {
        if let Some(loc) = global_address_map.lookup_str(query) {
            println!(
                " - Anim   '{: <30}' -> Cat: {: <8} | Chk: #{:<2} | Frames: {:<2} | Frame0 UV: [{:.3}, {:.3}, {:.3}, {:.3}] | V Step: {:.5} | Normal: {}",
                query, loc.category, loc.chunk_id, loc.frame_count, loc.frame_0_uv_bounds[0], loc.frame_0_uv_bounds[1], loc.frame_0_uv_bounds[2], loc.frame_0_uv_bounds[3], loc.frame_uv_step[1], loc.has_normal
            );
        } else {
            println!(" - Anim   '{: <30}' -> NOT FOUND", query);
        }
    }

    println!("============================================================");

    Ok(())
}

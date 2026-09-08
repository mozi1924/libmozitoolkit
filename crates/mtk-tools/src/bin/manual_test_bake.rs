use std::fs;
use std::path::Path;
use std::time::Instant;
use mtk_resource::{ResourceLocation, ResourcePackStack, ZipPack};
use mtk_texture::{AtlasBuilder, AtlasBuilderConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let vanilla_jar_path = "/home/mozi/26.2-Fabric.jar";
    let spbr_zip_path = "/home/mozi/Desktop/SPBR-21.zip";
    let output_dir = Path::new("manual_test");

    println!("============================================================");
    println!(" MoziToolKit 2.0 - Resource Pack & Atlas Baking Test");
    println!("============================================================");
    println!("Vanilla JAR: {}", vanilla_jar_path);
    println!("Top Pack ZIP: {}", spbr_zip_path);
    println!("Output Dir:  {}", output_dir.display());
    println!("------------------------------------------------------------");

    fs::create_dir_all(output_dir)?;

    let t_start = Instant::now();

    // 1. Load Resource Packs into Stack
    println!("Step 1: Loading resource pack archives...");
    let t_load = Instant::now();

    let vanilla_pack = ZipPack::from_file("vanilla_26.2", vanilla_jar_path)?;
    println!(" - Loaded vanilla JAR in {:?}", t_load.elapsed());

    let t_spbr = Instant::now();
    let spbr_pack = ZipPack::from_file("spbr_pack", spbr_zip_path)?;
    println!(" - Loaded SPBR-21 ZIP in {:?}", t_spbr.elapsed());

    let mut stack = ResourcePackStack::new();
    // Hierarchy: SPBR (Top, Priority 0) -> Vanilla JAR (Bottom, Fallback)
    stack.append_pack(Box::new(vanilla_pack));
    stack.push_pack(Box::new(spbr_pack));
    println!("ResourcePackStack ready ({} packs in stack, total loading time: {:?})", stack.len(), t_start.elapsed());

    // 2. Load Atlas Definition
    let blocks_loc = ResourceLocation::parse("minecraft:blocks")?;
    let definition = stack.load_atlas_definition(&blocks_loc)?;
    println!("\nStep 2: Loaded atlas definition '{}' with {} sources", blocks_loc, definition.sources.len());

    // 3. Bake Atlas with PBR & Vanilla-style Stitching
    println!("\nStep 3: Baking atlas with vanilla single-frame stitching & PBR sync...");
    let builder = AtlasBuilder::new(AtlasBuilderConfig {
        max_width: 4096,
        max_height: 4096,
        mip_level: 0,
        padding: 0,
    });

    let t_bake = Instant::now();
    let baked = builder.build(&stack, &definition)?;
    let bake_duration = t_bake.elapsed();

    println!("------------------------------------------------------------");
    println!(" Atlas Baking Succeeded in {:?}", bake_duration);
    println!(" Total Chunks: {}", baked.chunks.len());
    println!(" Total Registered Sprites: {}", baked.address_map.sprites.len());

    // 4. Analyze PBR Coverage & Output Files
    let mut normal_count = 0;
    let mut specular_count = 0;
    let mut anim_count = 0;

    for loc in baked.address_map.sprites.values() {
        if loc.has_normal {
            normal_count += 1;
        }
        if loc.has_specular {
            specular_count += 1;
        }
        if loc.frame_count > 1 {
            anim_count += 1;
        }
    }

    println!("\nCoverage Statistics:");
    println!(" - Sprites with Normal maps (_n):   {} ({:.1}%)", normal_count, (normal_count as f32 / baked.address_map.sprites.len() as f32) * 100.0);
    println!(" - Sprites with Specular maps (_s): {} ({:.1}%)", specular_count, (specular_count as f32 / baked.address_map.sprites.len() as f32) * 100.0);
    println!(" - Animated Sprites:                {} ({:.1}%)", anim_count, (anim_count as f32 / baked.address_map.sprites.len() as f32) * 100.0);

    // 5. Save Artifacts to manual_test/
    println!("\nStep 4: Writing output artifacts to {}/...", output_dir.display());
    let t_write = Instant::now();

    for chunk in &baked.chunks {
        let albedo_path = output_dir.join(format!("atlas_chunk_{}_albedo.png", chunk.chunk_id));
        let albedo_png = chunk.albedo.to_png_bytes()?;
        fs::write(&albedo_path, albedo_png)?;
        println!(" - Saved Albedo Chunk #{}: {} ({}x{} px)", chunk.chunk_id, albedo_path.display(), chunk.width, chunk.height);

        if let Some(ref norm_buf) = chunk.normal {
            let norm_path = output_dir.join(format!("atlas_chunk_{}_normal.png", chunk.chunk_id));
            let norm_png = norm_buf.to_png_bytes()?;
            fs::write(&norm_path, norm_png)?;
            println!(" - Saved Normal Chunk #{}: {} ({}x{} px)", chunk.chunk_id, norm_path.display(), chunk.width, chunk.height);
        }

        if let Some(ref spec_buf) = chunk.specular {
            let spec_path = output_dir.join(format!("atlas_chunk_{}_specular.png", chunk.chunk_id));
            let spec_png = spec_buf.to_png_bytes()?;
            fs::write(&spec_path, spec_png)?;
            println!(" - Saved Specular Chunk #{}: {} ({}x{} px)", chunk.chunk_id, spec_path.display(), chunk.width, chunk.height);
        }
    }

    // Save JSON Address Mapping
    let json_path = output_dir.join("atlas_mapping.json");
    let json_data = baked.address_map.to_json()?;
    fs::write(&json_path, json_data)?;
    println!(" - Saved Address Mapping: {} ({:.2} KB)", json_path.display(), fs::metadata(&json_path)?.len() as f64 / 1024.0);

    println!("File writing finished in {:?}", t_write.elapsed());
    println!("============================================================");
    println!(" Total Pipeline Execution Time: {:?}", t_start.elapsed());
    println!("============================================================");

    Ok(())
}

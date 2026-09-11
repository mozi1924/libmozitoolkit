use std::fs;
use std::path::Path;
use std::time::Instant;
use mtk_resource::{ResourceLocation, ResourcePackStack, ZipPack};
use mtk_texture::{AtlasBuilder, AtlasBuilderConfig, StandaloneBuilder, StandaloneConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let vanilla_jar_path = "/home/mozi/26.2-Fabric.jar";
    let spbr_zip_path = "/home/mozi/Desktop/SPBR-21.zip";
    let output_root = Path::new("manual_test");
    let atlas_output_dir = output_root.join("atlas");
    let standalone_output_dir = output_root.join("standalone");

    println!("============================================================");
    println!(" MoziToolKit 2.0 - Atlas & Standalone Dual-Bake Manual Test");
    println!("============================================================");
    println!("Vanilla JAR: {}", vanilla_jar_path);
    println!("Top Pack ZIP: {}", spbr_zip_path);
    println!("Atlas Output:      {}", atlas_output_dir.display());
    println!("Standalone Output: {}", standalone_output_dir.display());
    println!("------------------------------------------------------------");

    fs::create_dir_all(&atlas_output_dir)?;
    fs::create_dir_all(&standalone_output_dir)?;

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

    // 3. Bake Atlas with 100% Static Coverage + Dedicated Anim Chunks
    println!("\nStep 3: Baking Atlas (Static 100% + Animated Strips)...");
    let atlas_builder = AtlasBuilder::new(AtlasBuilderConfig {
        max_width: 4096,
        max_height: 4096,
        mip_level: 0,
        padding: 0,
    });

    let t_bake = Instant::now();
    let baked = atlas_builder.build(&stack, &definition)?;
    let bake_duration = t_bake.elapsed();

    println!(" - Atlas Baking Succeeded in {:?}", bake_duration);
    println!(" - Total Chunks: {}", baked.chunks.len());
    println!(" - Total Static Sprites (100% coverage): {}", baked.address_map.sprites.len());
    println!(" - Total Animated Sprites (Dedicated strips): {}", baked.address_map.anim_sprites.len());

    for chunk in &baked.chunks {
        let albedo_path = atlas_output_dir.join(format!("{}_albedo.png", chunk.file_stem()));
        let albedo_png = chunk.albedo.to_png_bytes()?;
        fs::write(&albedo_path, albedo_png)?;
        println!(
            "   [Chunk #{:03}] {} ({}x{} px, anim={})",
            chunk.chunk_id,
            albedo_path.file_name().unwrap().to_string_lossy(),
            chunk.width,
            chunk.height,
            chunk.is_animated
        );

        if let Some(ref norm_buf) = chunk.normal {
            let norm_path = atlas_output_dir.join(format!("{}_normal.png", chunk.file_stem()));
            let norm_png = norm_buf.to_png_bytes()?;
            fs::write(&norm_path, norm_png)?;
        }

        if let Some(ref spec_buf) = chunk.specular {
            let spec_path = atlas_output_dir.join(format!("{}_specular.png", chunk.file_stem()));
            let spec_png = spec_buf.to_png_bytes()?;
            fs::write(&spec_path, spec_png)?;
        }
    }

    let json_path = atlas_output_dir.join("atlas_mapping.json");
    let json_data = baked.address_map.to_json()?;
    fs::write(&json_path, json_data)?;
    println!(" - Saved Atlas Mapping: {}", json_path.display());

    // 4. Bake Standalone Material Asset Library (Dual-Mode Static Square + Anim Strip)
    println!("\nStep 4: Baking Standalone Material Library (Dual-Mode Static/Anim)...");
    let standalone_builder = StandaloneBuilder::new(StandaloneConfig {
        stack_hash: Some("manual_test_hash".to_string()),
        filter_prefix: Some("block".to_string()),
    });

    let t_stand = Instant::now();
    let stand_res = standalone_builder.build_to_dir(&stack, &standalone_output_dir)?;
    println!(" - Standalone Baking Succeeded in {:?}", t_stand.elapsed());
    println!(" - Total Standalone Textures: {}", stand_res.texture_count);
    println!(" - Standalone Mapping: {}", stand_res.mapping_path.display());

    println!("============================================================");
    println!(" Total Pipeline Execution Time: {:?}", t_start.elapsed());
    println!("============================================================");

    Ok(())
}

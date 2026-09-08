use std::path::Path;
use std::time::Instant;
use mtk_resource::{DirectoryPack, ResourceLocation, ResourcePackStack};
use mtk_texture::{AtlasBuilder, AtlasBuilderConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mc_dir = "/home/mozi/mc";
    println!("=== Testing Vanilla Atlas Baking on {} ===", mc_dir);

    if !Path::new(mc_dir).exists() {
        println!("Warning: {} does not exist, skipping live test.", mc_dir);
        return Ok(());
    }

    let t0 = Instant::now();
    let mut stack = ResourcePackStack::new();
    stack.push_pack(Box::new(DirectoryPack::new("vanilla_client", mc_dir)));

    let blocks_loc = ResourceLocation::parse("minecraft:blocks")?;
    println!("Loading atlas definition: {}", blocks_loc);
    let definition = stack.load_atlas_definition(&blocks_loc)?;
    println!("Loaded atlas definition with {} sources in {:?}", definition.sources.len(), t0.elapsed());

    let builder = AtlasBuilder::new(AtlasBuilderConfig {
        max_width: 4096,
        max_height: 4096,
        mip_level: 0,
        padding: 0,
    });

    let t_bake = Instant::now();
    let baked = builder.build(&stack, &definition)?;
    let bake_elapsed = t_bake.elapsed();

    println!("--------------------------------------------------");
    println!("Atlas Baking Complete in {:?}", bake_elapsed);
    println!("Total Chunks: {}", baked.chunks.len());
    println!("Total Registered Sprites: {}", baked.address_map.sprites.len());
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
    println!("--------------------------------------------------");

    // Spot-check some well known vanilla blocks in address map
    for sample in &["minecraft:block/stone", "minecraft:block/diamond_ore", "minecraft:block/water_still", "minecraft:block/oak_planks"] {
        if let Some(loc) = baked.address_map.lookup_str(sample) {
            println!("Sample [{}]: rect={:?}, uv={:?}, frame_size={:?}, frames={}", sample, loc.pixel_rect, loc.uv_bounds, loc.frame_size, loc.frame_count);
        } else {
            println!("Warning: Sample [{}] not found in atlas!", sample);
        }
    }

    Ok(())
}

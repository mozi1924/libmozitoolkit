use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

use libmtk::model::{
    blockstate::{BlockState, BlockStateDefinition},
    model_json::BlockModelJson,
    ModelBaker,
};
use zip::ZipArchive;

struct JarAssetLoader {
    zip: ZipArchive<BufReader<File>>,
    file_names: HashMap<String, usize>,
}

impl JarAssetLoader {
    fn new<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let file = File::open(path)?;
        let mut zip = ZipArchive::new(BufReader::new(file))?;
        let mut file_names = HashMap::new();

        for i in 0..zip.len() {
            let name = zip.by_index(i)?.name().to_string();
            file_names.insert(name, i);
        }

        Ok(Self { zip, file_names })
    }

    fn read_entry(&mut self, path: &str) -> Option<String> {
        let idx = *self.file_names.get(path)?;
        let mut file = self.zip.by_index(idx).ok()?;
        let mut text = String::new();
        file.read_to_string(&mut text).ok()?;
        Some(text)
    }

    fn load_blockstate(&mut self, blockstate_id: &str) -> Option<BlockStateDefinition> {
        let (ns, name) = if let Some((ns, n)) = blockstate_id.split_once(':') {
            (ns, n)
        } else {
            ("minecraft", blockstate_id)
        };
        let path = format!("assets/{}/blockstates/{}.json", ns, name);
        let text = self.read_entry(&path)?;
        serde_json::from_str(&text).ok()
    }

    fn load_model(&mut self, model_id: &str) -> Option<BlockModelJson> {
        let (ns, raw_path) = if let Some((ns, n)) = model_id.split_once(':') {
            (ns, n)
        } else {
            ("minecraft", model_id)
        };

        // Standard vanilla model path: assets/<namespace>/models/<raw_path>.json
        let candidates = [
            format!("assets/{}/models/{}.json", ns, raw_path),
            format!("assets/{}/models/block/{}.json", ns, raw_path),
            format!("assets/{}/models/item/{}.json", ns, raw_path),
        ];

        for path in candidates {
            if let Some(text) = self.read_entry(&path) {
                if let Ok(model) = serde_json::from_str::<BlockModelJson>(&text) {
                    return Some(model);
                }
            }
        }
        None
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let jar_path = args
        .get(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/mozi/26.2-Fabric.jar"));
    let output_dir = args
        .get(2)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("manual_test"));

    println!("==================================================");
    println!(" MoziToolKit 2.0 - Universal Model Baker Verification");
    println!(" Loading Jar: {:?}", jar_path);
    println!(" Output Dir : {:?}", output_dir);
    println!("==================================================");

    if !jar_path.exists() {
        eprintln!("Error: Minecraft jar not found at {:?}", jar_path);
        std::process::exit(1);
    }

    fs::create_dir_all(&output_dir)?;

    let mut loader = JarAssetLoader::new(&jar_path)?;
    let mut baker = ModelBaker::new();

    // Representative test palette covering different geometry, rotations, and multi-part rules
    let test_states = [
        // 1. Simple full cube
        "minecraft:stone",
        "minecraft:oak_planks",
        "minecraft:glass",
        // 2. Directional block with unique faces and state properties
        "minecraft:furnace[facing=north,lit=false]",
        "minecraft:furnace[facing=east,lit=true]",
        "minecraft:observer[facing=up,powered=false]",
        "minecraft:observer[facing=south,powered=true]",
        // 3. Stairs (rotation, UVLock, internal volume culling)
        "minecraft:oak_stairs[facing=east,half=bottom,shape=straight]",
        "minecraft:oak_stairs[facing=north,half=top,shape=outer_left]",
        "minecraft:oak_stairs[facing=south,half=bottom,shape=inner_right]",
        // 4. Multipart blocks
        "minecraft:oak_fence[north=true,east=true,south=false,west=false,waterlogged=false]",
        "minecraft:cobblestone_wall[up=true,north=low,east=none,south=low,west=none,waterlogged=false]",
        // 5. Non-full complex elements and rotations
        "minecraft:lantern[hanging=false,waterlogged=false]",
        "minecraft:lantern[hanging=true,waterlogged=false]",
        "minecraft:iron_chain[axis=y,waterlogged=false]",
        "minecraft:iron_chain[axis=x,waterlogged=false]",
        "minecraft:torch",
        "minecraft:redstone_torch[lit=true]",
        "minecraft:oak_trapdoor[facing=north,half=bottom,open=false,powered=false,waterlogged=false]",
        "minecraft:oak_trapdoor[facing=north,half=bottom,open=true,powered=false,waterlogged=false]",
    ];

    println!("\nBaking {} blockstate models...", test_states.len());

    for &state_str in &test_states {
        let bs = BlockState::parse(state_str)?;
        let bs_def = loader.load_blockstate(&bs.name);

        let baked = baker.bake_blockstate(state_str, bs_def.as_ref(), |model_id| {
            loader.load_model(model_id)
        })?;

        // Format clean filename from blockstate
        let sanitized = state_str
            .replace("minecraft:", "")
            .replace('[', "_")
            .replace(']', "")
            .replace(',', "_")
            .replace('=', "-");

        let obj_filename = format!("{}.obj", sanitized);
        let mtl_filename = format!("{}.mtl", sanitized);

        let obj_path = output_dir.join(&obj_filename);
        let mtl_path = output_dir.join(&mtl_filename);

        // Generate mesh with hidden volume clipping enabled
        let (mesh, textures) = baked.to_mesh_with_textures(true);

        // Build OBJ with mtllib reference
        let mut obj_content = String::new();
        obj_content.push_str(&format!("# BlockState: {}\n", state_str));
        obj_content.push_str(&format!("mtllib {}\n", mtl_filename));
        obj_content.push_str(&libmtk::model::obj::mesh_to_obj_string(
            &mesh, &sanitized, &textures,
        ));

        // Build simple companion MTL file
        let mut mtl_content = String::new();
        mtl_content.push_str("# Materials generated for manual inspection in Blender\n");
        for tex in &textures {
            mtl_content.push_str(&format!("newmtl {}\n", tex));
            mtl_content.push_str("Kd 0.8 0.8 0.8\n");
            mtl_content.push_str("d 1.0\n");
            mtl_content.push_str("illum 1\n\n");
        }

        fs::write(&obj_path, obj_content)?;
        fs::write(&mtl_path, mtl_content)?;

        println!(
            " ✓ Baked {:<65} -> {:>3} verts, {:>3} tris, {:>2} textures [cube: {:<5}]",
            state_str,
            mesh.vertex_count(),
            mesh.triangle_count(),
            textures.len(),
            baked.is_cube
        );
    }

    println!("\n==================================================");
    println!(" All models successfully exported to: {:?}", output_dir);
    println!(" You can now open Blender and import any of the .obj files");
    println!(" to inspect 3D geometry, normals, and UV layouts.");
    println!("==================================================");

    Ok(())
}

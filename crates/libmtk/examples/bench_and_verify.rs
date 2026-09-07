use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::time::Instant;

use libmtk::model::{
    blockstate::{BlockState, BlockStateDefinition},
    model_json::BlockModelJson,
    ModelBaker,
};
use serde::{Deserialize, Serialize};
use zip::ZipArchive;

pub struct JarAssetLoader {
    zip: ZipArchive<BufReader<File>>,
    file_names: HashMap<String, usize>,
    all_blockstate_names: Vec<String>,
}

impl JarAssetLoader {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let file = File::open(path)?;
        let mut zip = ZipArchive::new(BufReader::new(file))?;
        let mut file_names = HashMap::new();
        let mut all_blockstate_names = Vec::new();

        for i in 0..zip.len() {
            let name = zip.by_index(i)?.name().to_string();
            if name.starts_with("assets/minecraft/blockstates/") && name.ends_with(".json") {
                let stem = name
                    .strip_prefix("assets/minecraft/blockstates/")
                    .unwrap()
                    .strip_suffix(".json")
                    .unwrap();
                all_blockstate_names.push(format!("minecraft:{}", stem));
            }
            file_names.insert(name, i);
        }

        all_blockstate_names.sort();

        Ok(Self {
            zip,
            file_names,
            all_blockstate_names,
        })
    }

    pub fn read_entry(&mut self, path: &str) -> Option<String> {
        let idx = *self.file_names.get(path)?;
        let mut file = self.zip.by_index(idx).ok()?;
        let mut text = String::new();
        file.read_to_string(&mut text).ok()?;
        Some(text)
    }

    pub fn load_blockstate(&mut self, blockstate_id: &str) -> Option<BlockStateDefinition> {
        let (ns, name) = if let Some((ns, n)) = blockstate_id.split_once(':') {
            (ns, n)
        } else {
            ("minecraft", blockstate_id)
        };
        let path = format!("assets/{}/blockstates/{}.json", ns, name);
        let text = self.read_entry(&path)?;
        serde_json::from_str(&text).ok()
    }

    pub fn load_model(&mut self, model_id: &str) -> Option<BlockModelJson> {
        let (ns, raw_path) = if let Some((ns, n)) = model_id.split_once(':') {
            (ns, n)
        } else {
            ("minecraft", model_id)
        };

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

#[derive(Serialize, Deserialize)]
struct RustBakeResult {
    block_state: String,
    element_count: usize,
    mesh_vertex_count: usize,
    mesh_tri_count: usize,
    is_cube: bool,
    is_opaque: bool,
    is_emissive: bool,
    textures: Vec<String>,
    elements: Vec<RustElementInfo>,
    six_faces: Vec<RustFaceInfo>,
}

#[derive(Serialize, Deserialize)]
struct RustElementInfo {
    from: [f32; 3],
    to: [f32; 3],
    faces: HashMap<String, RustFaceInfo>,
}

#[derive(Serialize, Deserialize)]
struct RustFaceInfo {
    direction: String,
    texture: String,
    uv_rot: f32,
    uv_bounds: [f32; 4],
    tint_index: i16,
    cullface: Option<String>,
    vertices: [[f32; 3]; 4],
    uvs: [[f32; 2]; 4],
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("bench");
    let jar_path = args
        .get(2)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/mozi/26.2-Fabric.jar"));

    let mut loader = JarAssetLoader::new(&jar_path)?;

    if mode == "dump_json" {
        // Mode: Dump baked states to stdout/file for consistency verification
        let states_file = args.get(3).expect("Provide states input JSON file");
        let states_json = std::fs::read_to_string(states_file)?;
        let states: Vec<String> = serde_json::from_str(&states_json)?;

        let mut results = HashMap::new();
        let mut baker = ModelBaker::new();

        for state_str in &states {
            let bs = BlockState::parse(state_str)?;
            let bs_def = loader.load_blockstate(&bs.name);
            match baker.bake_blockstate(state_str, bs_def.as_ref(), |id| loader.load_model(id)) {
                Ok(baked) => {
                    let (mesh, textures) = baked.to_mesh_with_textures(true);
                    let mut elements = Vec::new();
                    for el in &baked.elements {
                        let mut faces = HashMap::new();
                        for (d, f) in &el.faces {
                            faces.insert(
                                d.as_str().to_string(),
                                RustFaceInfo {
                                    direction: f.direction.as_str().to_string(),
                                    texture: f.texture.clone(),
                                    uv_rot: f.uv_rot,
                                    uv_bounds: f.uv_bounds,
                                    tint_index: f.tint_index,
                                    cullface: f.cullface.map(|c| c.as_str().to_string()),
                                    vertices: [
                                        [f.vertices[0].x, f.vertices[0].y, f.vertices[0].z],
                                        [f.vertices[1].x, f.vertices[1].y, f.vertices[1].z],
                                        [f.vertices[2].x, f.vertices[2].y, f.vertices[2].z],
                                        [f.vertices[3].x, f.vertices[3].y, f.vertices[3].z],
                                    ],
                                    uvs: [
                                        [f.uvs[0].x, f.uvs[0].y],
                                        [f.uvs[1].x, f.uvs[1].y],
                                        [f.uvs[2].x, f.uvs[2].y],
                                        [f.uvs[3].x, f.uvs[3].y],
                                    ],
                                },
                            );
                        }
                        elements.push(RustElementInfo {
                            from: el.from_pos,
                            to: el.to_pos,
                            faces,
                        });
                    }

                    let mut six_faces = Vec::new();
                    for f in &baked.faces {
                        six_faces.push(RustFaceInfo {
                            direction: f.direction.as_str().to_string(),
                            texture: f.texture.clone(),
                            uv_rot: f.uv_rot,
                            uv_bounds: f.uv_bounds,
                            tint_index: f.tint_index,
                            cullface: f.cullface.map(|c| c.as_str().to_string()),
                            vertices: [
                                [f.vertices[0].x, f.vertices[0].y, f.vertices[0].z],
                                [f.vertices[1].x, f.vertices[1].y, f.vertices[1].z],
                                [f.vertices[2].x, f.vertices[2].y, f.vertices[2].z],
                                [f.vertices[3].x, f.vertices[3].y, f.vertices[3].z],
                            ],
                            uvs: [
                                [f.uvs[0].x, f.uvs[0].y],
                                [f.uvs[1].x, f.uvs[1].y],
                                [f.uvs[2].x, f.uvs[2].y],
                                [f.uvs[3].x, f.uvs[3].y],
                            ],
                        });
                    }

                    results.insert(
                        state_str.clone(),
                        RustBakeResult {
                            block_state: state_str.clone(),
                            element_count: baked.elements.len(),
                            mesh_vertex_count: mesh.vertex_count(),
                            mesh_tri_count: mesh.triangle_count(),
                            is_cube: baked.is_cube,
                            is_opaque: baked.is_opaque,
                            is_emissive: baked.is_emissive,
                            textures,
                            elements,
                            six_faces,
                        },
                    );
                }
                Err(e) => {
                    eprintln!("Error baking {}: {}", state_str, e);
                }
            }
        }

        let out_file = args.get(4).expect("Provide output JSON file");
        let json_data = serde_json::to_string_pretty(&results)?;
        std::fs::write(out_file, json_data)?;
        println!("Exported {} baked states to {}", results.len(), out_file);
        return Ok(());
    }

    // Benchmark Mode
    println!("============================================================");
    println!(" Rust mtk-model Benchmark Runner");
    println!(" Jar path: {:?}", jar_path);
    println!(" Total blockstate JSONs in jar: {}", loader.all_blockstate_names.len());
    println!("============================================================");

    // 1. Collect all blockstate variants across the entire vanilla jar
    let mut all_test_states = Vec::new();
    let scan_start = Instant::now();

    for bs_name in &loader.all_blockstate_names.clone() {
        if let Some(def) = loader.load_blockstate(bs_name) {
            if let Some(ref variants) = def.variants {
                for k in variants.keys() {
                    if k.is_empty() {
                        all_test_states.push(bs_name.clone());
                    } else {
                        all_test_states.push(format!("{}[{}]", bs_name, k));
                    }
                }
            } else if def.multipart.is_some() {
                all_test_states.push(bs_name.clone());
                all_test_states.push(format!("{}[waterlogged=false]", bs_name));
                all_test_states.push(format!("{}[waterlogged=true]", bs_name));
            }
        }
    }
    let scan_duration = scan_start.elapsed();
    println!("Collected {} unique blockstate variants in {:?}", all_test_states.len(), scan_duration);

    // 2. Full Jar Cold Bake (Single-threaded)
    let mut baker = ModelBaker::new();
    let cold_start = Instant::now();
    let mut successful_bakes = 0;
    let mut total_tris = 0;

    for state_str in &all_test_states {
        if let Ok(bs) = BlockState::parse(state_str) {
            let bs_def = loader.load_blockstate(&bs.name);
            if let Ok(baked) = baker.bake_blockstate(state_str, bs_def.as_ref(), |id| loader.load_model(id)) {
                successful_bakes += 1;
                let mesh = baked.to_mesh(true);
                total_tris += mesh.triangle_count();
            }
        }
    }
    let cold_duration = cold_start.elapsed();
    println!("\n[Benchmark 1: Full Jar Cold Bake]");
    println!(" Successfully baked: {} / {} states", successful_bakes, all_test_states.len());
    println!(" Total triangles generated: {}", total_tris);
    println!(" Total time: {:?}", cold_duration);
    println!(" Average time per model: {:.3} µs", (cold_duration.as_secs_f64() * 1_000_000.0) / successful_bakes as f64);
    println!(" Throughput: {:.2} models/sec", successful_bakes as f64 / cold_duration.as_secs_f64());

    // 3. Preloaded In-Memory Model Baking (Raw Computation Speed, 100,000 iterations)
    println!("\n[Benchmark 2: In-Memory Raw Baker Computation]");
    let sample_states = [
        "minecraft:stone",
        "minecraft:furnace[facing=north,lit=false]",
        "minecraft:oak_stairs[facing=east,half=bottom,shape=straight]",
        "minecraft:oak_fence[north=true,east=true,south=false,west=false,waterlogged=false]",
        "minecraft:cobblestone_wall[up=true,north=low,east=none,south=low,west=none,waterlogged=false]",
        "minecraft:lantern[hanging=false,waterlogged=false]",
        "minecraft:iron_chain[axis=y,waterlogged=false]",
        "minecraft:observer[facing=south,powered=true]",
    ];

    // Pre-cache definitions
    let mut preloaded_defs = HashMap::new();
    let mut preloaded_models = HashMap::new();
    for &state_str in &sample_states {
        let bs = BlockState::parse(state_str)?;
        if let Some(def) = loader.load_blockstate(&bs.name) {
            preloaded_defs.insert(bs.name.clone(), def);
        }
    }
    // Load needed models
    let model_names = [
        "minecraft:block/stone", "minecraft:block/cube_all", "minecraft:block/furnace",
        "minecraft:block/orientable_with_bottom", "minecraft:block/oak_stairs",
        "minecraft:block/stairs", "minecraft:block/oak_fence_post", "minecraft:block/oak_fence_side",
        "minecraft:block/cobblestone_wall_post", "minecraft:block/cobblestone_wall_side",
        "minecraft:block/lantern", "minecraft:block/chain", "minecraft:block/observer",
    ];
    for name in model_names {
        if let Some(m) = loader.load_model(name) {
            preloaded_models.insert(name.to_string(), m);
        }
    }

    let iters = 100_000;
    let mut mem_baker = ModelBaker::new();
    let mem_start = Instant::now();

    for i in 0..iters {
        let state_str = sample_states[i % sample_states.len()];
        let bs = BlockState::parse(state_str).unwrap();
        let def = preloaded_defs.get(&bs.name);
        // Clear bake cache every 1000 to test raw baking path without memoization lookup
        let _ = mem_baker.bake_blockstate(state_str, def, |id| preloaded_models.get(id).cloned());
        if i % 1000 == 0 {
            mem_baker.clear_cache();
        }
    }
    let mem_duration = mem_start.elapsed();
    println!(" Iterations: {}", iters);
    println!(" Total time: {:?}", mem_duration);
    println!(" Average time per bake: {:.3} µs", (mem_duration.as_secs_f64() * 1_000_000.0) / iters as f64);
    println!(" Throughput: {:.2} bakes/sec", iters as f64 / mem_duration.as_secs_f64());

    // 4. Mesh Generation with 2D Hidden Volume Clipping
    println!("\n[Benchmark 3: Mesh Generation & Hidden Volume Culling]");
    let stairs_state = "minecraft:oak_stairs[facing=east,half=bottom,shape=straight]";
    let bs = BlockState::parse(stairs_state)?;
    let def = loader.load_blockstate(&bs.name);
    let stairs_baked = baker.bake_blockstate(stairs_state, def.as_ref(), |id| loader.load_model(id))?;

    let mesh_iters = 100_000;
    let mesh_start = Instant::now();
    for _ in 0..mesh_iters {
        let _ = stairs_baked.to_mesh(true);
    }
    let mesh_duration = mesh_start.elapsed();
    println!(" Stairs to_mesh(true) iterations: {}", mesh_iters);
    println!(" Total time: {:?}", mesh_duration);
    println!(" Average time per mesh gen: {:.3} µs", (mesh_duration.as_secs_f64() * 1_000_000.0) / mesh_iters as f64);
    println!(" Throughput: {:.2} meshes/sec", mesh_iters as f64 / mesh_duration.as_secs_f64());

    Ok(())
}

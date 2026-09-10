use std::collections::HashMap;

use glam::{Vec2, Vec3};
use mtk_core::direction::Direction;

use crate::baked::{BakedElement, BakedFace, BakedModel};
use crate::blockstate::{BlockState, BlockStateDefinition, BlockStateResolver};
use crate::builtin::{apply_bell_patches, BuiltinModelRegistry};
use crate::error::ModelError;
use crate::math::{bake_face_exact, rotate_direction};
use crate::model_json::BlockModelJson;
use crate::obj::{ModObjLoader, WavefrontObjParser};

/// White-list of natively light-emitting blocks in Minecraft.
const EMISSIVE_BLOCKS: &[&str] = &[
    "glowstone",
    "sea_lantern",
    "shroomlight",
    "magma_block",
    "magma",
    "crying_obsidian",
    "jack_o_lantern",
    "beacon",
    "end_rod",
    "lantern",
    "soul_lantern",
    "torch",
    "soul_torch",
    "wall_torch",
    "soul_wall_torch",
    "lava",
    "flowing_lava",
    "fire",
    "soul_fire",
    "conduit",
    "sculk_catalyst",
    "ochre_froglight",
    "pearlescent_froglight",
    "verdant_froglight",
];

const KNOWN_NON_CUBES: &[&str] = &[
    "glass_pane", "pane", "fence", "door", "trapdoor", "bars", "chain", "lantern",
    "stairs", "slab", "chest", "banner", "bed", "carpet", "pot", "sign", "hanging_sign",
    "head", "skull", "rod", "hook", "lever", "rail", "torch", "candle", "flower",
    "plant", "sapling", "vine", "bush", "wire", "repeater", "comparator", "cauldron",
    "hopper", "bell", "anvil", "stand", "frame", "portal", "conduit", "grindstone",
    "stonecutter", "scaffolding", "dripstone", "amethyst", "sensor", "shrieker",
];

const NON_OPAQUE_SUBSTRINGS: &[&str] = &[
    "glass", "leaves", "ice", "water", "air", "pane", "fence", "door",
    "trapdoor", "bars", "chain", "lantern", "stairs", "slab", "chest",
    "banner", "bed", "carpet", "pot", "sign", "hanging_sign", "head",
    "skull", "rod", "hook", "lever", "rail", "torch", "candle",
    "flower", "plant", "sapling", "vine", "bush", "wire", "repeater",
    "comparator", "cauldron", "hopper", "bell", "anvil", "stand",
    "frame", "portal", "conduit", "grindstone", "cutter", "piston",
];

/// Checks if a blockstate is emissive based on identifier and properties.
pub fn is_block_emissive(blockstate: &BlockState) -> bool {
    let short_name = blockstate.name.as_str();
    if EMISSIVE_BLOCKS.contains(&short_name) || short_name.ends_with("_froglight") {
        return true;
    }
    let p = &blockstate.properties;
    let is_lit = p.get("lit").map(|s| s == "true").unwrap_or(false);
    if is_lit
        && matches!(
            short_name,
            "furnace"
                | "blast_furnace"
                | "smoker"
                | "redstone_lamp"
                | "campfire"
                | "soul_campfire"
                | "redstone_ore"
                | "deepslate_redstone_ore"
        )
    {
        return true;
    }
    if matches!(short_name, "redstone_torch" | "redstone_wall_torch") {
        return p.get("lit").map(|s| s == "true").unwrap_or(true);
    }
    if short_name == "respawn_anchor" {
        if let Some(charges) = p.get("charges").and_then(|s| s.parse::<i32>().ok()) {
            return charges > 0;
        }
    }
    if short_name == "redstone_wire" {
        if let Some(power) = p.get("power").and_then(|s| s.parse::<i32>().ok()) {
            return power > 0;
        }
    }
    false
}

/// Universal, headless Minecraft Model Baker.
///
/// Bakes arbitrary BlockStates and models (both native Minecraft JSON models and Mod Wavefront OBJ meshes)
/// into fully realized geometry completely independent of Blender or any external 3D runtime.
#[derive(Default)]
pub struct ModelBaker {
    /// In-memory cache for baked models keyed by canonical BlockState string.
    bake_cache: HashMap<String, BakedModel>,
}

impl ModelBaker {
    /// Creates a new `ModelBaker` instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Clears the internal bake cache.
    pub fn clear_cache(&mut self) {
        self.bake_cache.clear();
    }

    /// Bakes a BlockState into a `BakedModel` using supplied JSON definitions and model loaders.
    ///
    /// - `state_str`: E.g. `"minecraft:oak_stairs[facing=east,half=bottom,shape=straight]"`
    /// - `blockstate_def`: Optional `BlockStateDefinition` (variants or multipart). If None, uses heuristic.
    /// - `model_loader`: Callback retrieving raw `BlockModelJson` given a model ID path.
    pub fn bake_blockstate<F>(
        &mut self,
        state_str: &str,
        blockstate_def: Option<&BlockStateDefinition>,
        mut model_loader: F,
    ) -> Result<BakedModel, ModelError>
    where
        F: FnMut(&str) -> Option<BlockModelJson>,
    {
        let blockstate = BlockState::parse(state_str)?;
        let canonical_str = blockstate.to_canonical_string();

        if let Some(cached) = self.bake_cache.get(&canonical_str) {
            return Ok(cached.clone());
        }

        let variant_matches = if let Some(def) = blockstate_def {
            BlockStateResolver::resolve(def, &blockstate)
        } else {
            vec![BlockStateResolver::heuristic_match(&blockstate)]
        };

        let mut baked_elements = Vec::new();
        let mut six_faces: [Option<BakedFace>; 6] = [None, None, None, None, None, None];

        for variant in &variant_matches {
            let mut root_model = model_loader(&variant.model_id).unwrap_or_default();
            apply_bell_patches(&variant.model_id, &mut root_model);

            let mut resolved = root_model.resolve_hierarchy(&variant.model_id, &mut model_loader)?;
            if resolved.elements.is_empty() {
                if let Some(builtin) = BuiltinModelRegistry::get_builtin_model(&blockstate) {
                    resolved = builtin.resolve_hierarchy(&variant.model_id, &mut model_loader)?;
                }
            }

            for elem in &resolved.elements {
                let from_pos = elem.from;
                let to_pos = elem.to;
                let elem_rot = elem.rotation.as_ref();

                // Prevent internal overlapping zero-thickness faces
                let is_zero_x = (from_pos[0] - to_pos[0]).abs() < 1e-5;
                let is_zero_y = (from_pos[1] - to_pos[1]).abs() < 1e-5;
                let is_zero_z = (from_pos[2] - to_pos[2]).abs() < 1e-5;

                let is_full_cuboid = from_pos == [0.0, 0.0, 0.0]
                    && to_pos == [16.0, 16.0, 16.0]
                    && elem_rot.is_none();

                let mut elem_faces = HashMap::new();

                const CANONICAL_DIRS: [&str; 6] = ["down", "up", "north", "south", "west", "east"];
                for orig_dir_str in CANONICAL_DIRS {
                    let face_data = match elem.faces.get(orig_dir_str) {
                        Some(f) => f,
                        None => continue,
                    };
                    let orig_dir = match Direction::parse_loose(orig_dir_str) {
                        Some(d) => d,
                        None => continue,
                    };

                    if is_zero_z && orig_dir == Direction::South && elem.faces.contains_key("north")
                    {
                        continue;
                    }
                    if is_zero_x && orig_dir == Direction::East && elem.faces.contains_key("west") {
                        continue;
                    }
                    if is_zero_y && orig_dir == Direction::Down && elem.faces.contains_key("up") {
                        continue;
                    }

                    let cullface_dir = face_data
                        .cullface
                        .as_deref()
                        .and_then(Direction::parse_loose)
                        .or(if is_full_cuboid { Some(orig_dir) } else { None });

                    let uv_base = if let Some(uv) = face_data.uv {
                        if uv.iter().any(|&c| c > 16.0) {
                            64.0
                        } else {
                            16.0
                        }
                    } else {
                        16.0
                    };

                    let baked_geom = bake_face_exact(
                        orig_dir,
                        from_pos,
                        to_pos,
                        face_data.uv,
                        face_data.rotation as f32,
                        variant.rot_x,
                        variant.rot_y,
                        elem_rot,
                        variant.uvlock,
                        uv_base,
                    );

                    let rotated_cullface = cullface_dir
                        .map(|cd| rotate_direction(cd, variant.rot_x, variant.rot_y));

                    let v1 = baked_geom.positions[1] - baked_geom.positions[0];
                    let v2 = baked_geom.positions[2] - baked_geom.positions[0];
                    let normal = v1.cross(v2).normalize_or_zero();

                    let baked_face = BakedFace {
                        direction: baked_geom.direction,
                        texture: face_data.texture.clone(),
                        uv_rot: baked_geom.detected_rotation,
                        uv_bounds: baked_geom.uv_bounds,
                        tint_index: face_data.tintindex,
                        cullface: rotated_cullface,
                        vertices: baked_geom.positions,
                        uvs: baked_geom.uvs,
                        normal,
                    };

                    let idx = baked_geom.direction.to_index();
                    if six_faces[idx].is_none() {
                        six_faces[idx] = Some(baked_face.clone());
                    }

                    elem_faces.insert(orig_dir, baked_face);
                }

                baked_elements.push(BakedElement {
                    from_pos,
                    to_pos,
                    faces: elem_faces,
                });
            }
        }

        // Fill missing 6-face summary entries
        let mut fallback_tex = "minecraft:block/dirt";
        'find_fallback: for el in &baked_elements {
            const CANONICAL_DIRS: [Direction; 6] = [
                Direction::Down,
                Direction::Up,
                Direction::North,
                Direction::South,
                Direction::West,
                Direction::East,
            ];
            for dir in CANONICAL_DIRS {
                if let Some(f) = el.faces.get(&dir) {
                    if !f.texture.is_empty() {
                        fallback_tex = f.texture.as_str();
                        break 'find_fallback;
                    }
                }
            }
        }

        let mut final_six_faces = [
            BakedFace::default(),
            BakedFace::default(),
            BakedFace::default(),
            BakedFace::default(),
            BakedFace::default(),
            BakedFace::default(),
        ];

        for dir in Direction::ALL {
            let idx = dir.to_index();
            if let Some(ref bf) = six_faces[idx] {
                final_six_faces[idx] = bf.clone();
            } else {
                final_six_faces[idx] = BakedFace {
                    direction: dir,
                    texture: fallback_tex.to_string(),
                    uv_rot: 0.0,
                    uv_bounds: [0.0, 0.0, 1.0, 1.0],
                    tint_index: -1,
                    cullface: None,
                    vertices: [Vec3::ZERO; 4],
                    uvs: [Vec2::ZERO; 4],
                    normal: dir.normal(),
                };
            }
        }

        let short_name = blockstate.name.as_str();
        let is_double_slab = short_name.ends_with("_slab")
            && blockstate.properties.get("type").map(|s| s.as_str()) == Some("double");

        let is_known_non_cube = KNOWN_NON_CUBES.iter().any(|&w| short_name.contains(w)) && !is_double_slab;

        let is_cube = !is_known_non_cube
            && !baked_elements.is_empty()
            && baked_elements.iter().all(|el| {
                el.from_pos == [0.0, 0.0, 0.0] && el.to_pos == [16.0, 16.0, 16.0]
            });

        let is_opaque = !NON_OPAQUE_SUBSTRINGS.iter().any(|&w| short_name.contains(w));

        let emissive = is_block_emissive(&blockstate);

        let baked_model = BakedModel {
            block_state: canonical_str.clone(),
            elements: baked_elements,
            obj_faces: Vec::new(),
            faces: final_six_faces,
            is_cube,
            is_opaque,
            is_emissive: emissive,
            emissive_level: if emissive { 1.0 } else { 0.0 },
        };

        self.bake_cache.insert(canonical_str, baked_model.clone());
        Ok(baked_model)
    }

    /// Bakes a Wavefront OBJ mesh into a `BakedModel`.
    #[allow(clippy::too_many_arguments)]
    pub fn bake_obj_model(
        &mut self,
        block_state: &str,
        obj_text: &str,
        texture_mappings: &HashMap<String, String>,
        fallback_texture: &str,
        rot_x: f32,
        rot_y: f32,
        rot_z: f32,
        offset: Vec3,
        object_filter: Option<&[&str]>,
    ) -> Result<BakedModel, ModelError> {
        let raw_faces = WavefrontObjParser::parse_str(obj_text, object_filter);
        let baked_obj_faces = ModObjLoader::process_raw_faces(
            &raw_faces,
            texture_mappings,
            fallback_texture,
            rot_x,
            rot_y,
            rot_z,
            offset,
        );

        let mut six_faces = [
            BakedFace::default(),
            BakedFace::default(),
            BakedFace::default(),
            BakedFace::default(),
            BakedFace::default(),
            BakedFace::default(),
        ];

        for dir in Direction::ALL {
            let idx = dir.to_index();
            if let Some(matching) = baked_obj_faces.iter().find(|f| f.direction == dir) {
                let mut v4 = [Vec3::ZERO; 4];
                let mut u4 = [Vec2::ZERO; 4];
                let count = matching.vertices.len().min(4);
                v4[..count].copy_from_slice(&matching.vertices[..count]);
                u4[..count].copy_from_slice(&matching.uvs[..count]);
                six_faces[idx] = BakedFace {
                    direction: dir,
                    texture: matching.texture.clone(),
                    uv_rot: 0.0,
                    uv_bounds: [0.0, 0.0, 1.0, 1.0],
                    tint_index: matching.tint_index,
                    cullface: None,
                    vertices: v4,
                    uvs: u4,
                    normal: matching.normal,
                };
            } else if let Some(first) = baked_obj_faces.first() {
                let mut v4 = [Vec3::ZERO; 4];
                let mut u4 = [Vec2::ZERO; 4];
                let count = first.vertices.len().min(4);
                v4[..count].copy_from_slice(&first.vertices[..count]);
                u4[..count].copy_from_slice(&first.uvs[..count]);
                six_faces[idx] = BakedFace {
                    direction: dir,
                    texture: first.texture.clone(),
                    uv_rot: 0.0,
                    uv_bounds: [0.0, 0.0, 1.0, 1.0],
                    tint_index: first.tint_index,
                    cullface: None,
                    vertices: v4,
                    uvs: u4,
                    normal: dir.normal(),
                };
            } else {
                six_faces[idx] = BakedFace {
                    direction: dir,
                    texture: fallback_texture.to_string(),
                    normal: dir.normal(),
                    ..Default::default()
                };
            }
        }

        let blockstate = BlockState::parse(block_state).unwrap_or_else(|_| BlockState {
            namespace: "minecraft".to_string(),
            name: "custom_obj".to_string(),
            properties: std::collections::BTreeMap::new(),
        });
        let emissive = is_block_emissive(&blockstate);

        Ok(BakedModel {
            block_state: block_state.to_string(),
            elements: Vec::new(),
            obj_faces: baked_obj_faces,
            faces: six_faces,
            is_cube: false,
            is_opaque: false,
            is_emissive: emissive,
            emissive_level: if emissive { 1.0 } else { 0.0 },
        })
    }

    /// Calculates a conservative default thread count for parallel operations.
    ///
    /// Strategy: min(4, max(1, available_parallelism / 2)).
    /// This leaves at least half of the CPU cores free for Blender UI, viewport rendering,
    /// or host system responsiveness.
    pub fn determine_conservative_threads() -> usize {
        mtk_core::constants::concurrency::determine_conservative_threads(4)
    }

    /// Bakes a batch of BlockStates.
    ///
    /// - When `feature = "parallel"` is enabled: parallelizes across worker threads.
    /// - When compiled for WASM or single-threaded mode: falls back to sequential iteration.
    pub fn bake_batch<S, SF, MF>(
        states: &[S],
        state_loader: SF,
        model_loader: MF,
    ) -> Result<Vec<(String, Result<BakedModel, ModelError>)>, ModelError>
    where
        S: AsRef<str> + Sync,
        SF: Fn(&str) -> Option<BlockStateDefinition> + Sync + Send,
        MF: Fn(&str) -> Option<BlockModelJson> + Sync + Send,
    {
        #[cfg(feature = "parallel")]
        {
            use rayon::prelude::*;
            let results = states
                .par_iter()
                .map(|state_ref| {
                    let state_str = state_ref.as_ref();
                    let mut local_baker = ModelBaker::new();
                    let res = (|| -> Result<BakedModel, ModelError> {
                        let bs = BlockState::parse(state_str)?;
                        let bs_def = state_loader(&bs.name);
                        local_baker.bake_blockstate(state_str, bs_def.as_ref(), |id| model_loader(id))
                    })();
                    (state_str.to_string(), res)
                })
                .collect();
            Ok(results)
        }

        #[cfg(not(feature = "parallel"))]
        {
            let results = states
                .iter()
                .map(|state_ref| {
                    let state_str = state_ref.as_ref();
                    let mut local_baker = ModelBaker::new();
                    let res = (|| -> Result<BakedModel, ModelError> {
                        let bs = BlockState::parse(state_str)?;
                        let bs_def = state_loader(&bs.name);
                        local_baker.bake_blockstate(state_str, bs_def.as_ref(), |id| model_loader(id))
                    })();
                    (state_str.to_string(), res)
                })
                .collect();
            Ok(results)
        }
    }

    /// Backwards-compatible batch baker with optional explicit thread pool configuration.
    pub fn bake_batch_parallel<S, SF, MF>(
        states: &[S],
        num_threads: Option<usize>,
        state_loader: SF,
        model_loader: MF,
    ) -> Result<Vec<(String, Result<BakedModel, ModelError>)>, ModelError>
    where
        S: AsRef<str> + Sync,
        SF: Fn(&str) -> Option<BlockStateDefinition> + Sync + Send,
        MF: Fn(&str) -> Option<BlockModelJson> + Sync + Send,
    {
        mtk_core::constants::concurrency::execute_parallel(num_threads, || {
            Self::bake_batch(states, state_loader, model_loader)
        })
        .map_err(ModelError::ThreadPoolError)?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bake_cube_blockstate() {
        let mut baker = ModelBaker::new();

        let model_json = r##"{
            "textures": {
                "all": "minecraft:block/stone"
            },
            "elements": [
                {
                    "from": [0, 0, 0],
                    "to": [16, 16, 16],
                    "faces": {
                        "down":  { "texture": "#all", "cullface": "down" },
                        "up":    { "texture": "#all", "cullface": "up" },
                        "north": { "texture": "#all", "cullface": "north" },
                        "south": { "texture": "#all", "cullface": "south" },
                        "west":  { "texture": "#all", "cullface": "west" },
                        "east":  { "texture": "#all", "cullface": "east" }
                    }
                }
            ]
        }"##;

        let parsed_model: BlockModelJson = serde_json::from_str(model_json).unwrap();

        let baked = baker
            .bake_blockstate("minecraft:stone", None, |_id| {
                Some(parsed_model.clone())
            })
            .unwrap();

        assert!(baked.is_cube);
        assert_eq!(baked.elements.len(), 1);
        assert_eq!(baked.get_face(Direction::Up).texture, "minecraft:block/stone");

        let mesh = baked.to_mesh(false);
        assert_eq!(mesh.face_count(), 6);
        assert_eq!(mesh.triangle_count(), 12);
        assert_eq!(mesh.vertex_count(), 24);
    }

    #[test]
    fn test_bake_obj_model() {
        let mut baker = ModelBaker::new();
        let obj_text = r#"
v 0 0 0
v 16 0 0
v 16 16 0
vt 0 0
vt 1 0
vt 1 1
f 1/1 2/2 3/3
"#;
        let baked = baker
            .bake_obj_model(
                "mod:custom_gear",
                obj_text,
                &HashMap::new(),
                "mod:block/gear",
                0.0,
                0.0,
                0.0,
                Vec3::ZERO,
                None,
            )
            .unwrap();

        assert!(!baked.is_cube);
        assert_eq!(baked.obj_faces.len(), 1);
        let mesh = baked.to_mesh(false);
        assert_eq!(mesh.triangle_count(), 1);
        assert_eq!(mesh.vertex_count(), 3);
    }

    #[test]
    #[cfg(feature = "parallel")]
    fn test_bake_batch_parallel() {
        let model_json = r##"{
            "textures": { "all": "minecraft:block/stone" },
            "elements": [{
                "from": [0, 0, 0], "to": [16, 16, 16],
                "faces": {
                    "down":  { "texture": "#all" }, "up":    { "texture": "#all" },
                    "north": { "texture": "#all" }, "south": { "texture": "#all" },
                    "west":  { "texture": "#all" }, "east":  { "texture": "#all" }
                }
            }]
        }"##;
        let model: BlockModelJson = serde_json::from_str(model_json).unwrap();

        let states = vec![
            "minecraft:stone".to_string(),
            "minecraft:stone[variant=smooth]".to_string(),
            "minecraft:stone[variant=rough]".to_string(),
        ];

        let results = ModelBaker::bake_batch_parallel(
            &states,
            Some(2),
            |_name| None,
            |_id| Some(model.clone()),
        )
        .unwrap();

        assert_eq!(results.len(), 3);
        for (st, res) in results {
            let baked = res.unwrap();
            assert!(baked.is_cube);
            assert_eq!(baked.elements.len(), 1);
            assert!(states.contains(&st));
        }
    }
}

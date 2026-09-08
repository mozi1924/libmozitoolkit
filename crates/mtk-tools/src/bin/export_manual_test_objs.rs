use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use mtk_model::{BlockModelJson, BlockState, BlockStateDefinition, ModelBaker};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mc_assets_dir = Path::new("/home/mozi/mc/assets");
    let output_dir = Path::new("manual_test/models");
    let textures_out_dir = Path::new("manual_test/textures");

    println!("============================================================");
    println!(" MoziToolKit 2.0 - Exporting Builtin Models to Wavefront OBJ");
    println!("============================================================");
    println!("MC Assets Directory:  {}", mc_assets_dir.display());
    println!("Models Output Dir:    {}", output_dir.display());
    println!("Textures Output Dir:  {}", textures_out_dir.display());
    println!("------------------------------------------------------------");

    fs::create_dir_all(output_dir)?;
    fs::create_dir_all(textures_out_dir)?;

    let mut baker = ModelBaker::new();

    // Model loader that reads vanilla block models if present in mc_assets_dir
    let load_model_from_mc = |id: &str| -> Option<BlockModelJson> {
        let clean_id = id.strip_prefix("minecraft:").unwrap_or(id);
        let path = mc_assets_dir.join("minecraft/models").join(format!("{}.json", clean_id));
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(json) = serde_json::from_str::<BlockModelJson>(&content) {
                    return Some(json);
                }
            }
        }
        None
    };

    // BlockState definition loader
    let load_blockstate_def = |name: &str| -> Option<BlockStateDefinition> {
        let clean_name = name.strip_prefix("minecraft:").unwrap_or(name);
        let path = mc_assets_dir.join("minecraft/blockstates").join(format!("{}.json", clean_name));
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(def) = serde_json::from_str::<BlockStateDefinition>(&content) {
                    return Some(def);
                }
            }
        }
        None
    };

    let test_cases = vec![
        // Chests
        ("chest_single", "minecraft:chest[facing=north,type=single]"),
        ("chest_left", "minecraft:chest[facing=north,type=left]"),
        ("chest_right", "minecraft:chest[facing=north,type=right]"),
        ("trapped_chest_single", "minecraft:trapped_chest[facing=north,type=single]"),
        ("ender_chest", "minecraft:ender_chest[facing=north]"),

        // Shulker Boxes
        ("shulker_box_default", "minecraft:shulker_box[facing=up]"),
        ("shulker_box_red", "minecraft:red_shulker_box[facing=up]"),
        ("shulker_box_white_north", "minecraft:white_shulker_box[facing=north]"),

        // Skulls & Heads
        ("skeleton_skull", "minecraft:skeleton_skull[rotation=0]"),
        ("skeleton_wall_skull", "minecraft:skeleton_wall_skull[facing=north]"),
        ("wither_skeleton_skull", "minecraft:wither_skeleton_skull[rotation=0]"),
        ("zombie_head", "minecraft:zombie_head[rotation=0]"),
        ("creeper_head", "minecraft:creeper_head[rotation=0]"),
        ("player_head", "minecraft:player_head[rotation=0]"),

        // Bell
        ("bell_floor", "minecraft:bell[attachment=floor,facing=north]"),
        ("bell_ceiling", "minecraft:bell[attachment=ceiling,facing=north]"),
        ("bell_single_wall", "minecraft:bell[attachment=single_wall,facing=north]"),

        // End Portal
        ("end_portal", "minecraft:end_portal"),

        // Signs & Hanging Signs
        ("oak_sign", "minecraft:oak_sign[rotation=0]"),
        ("oak_wall_sign", "minecraft:oak_wall_sign[facing=north]"),
        ("oak_hanging_sign", "minecraft:oak_hanging_sign[attached=false,rotation=0,waterlogged=false]"),
        ("oak_wall_hanging_sign", "minecraft:oak_wall_hanging_sign[facing=north,waterlogged=false]"),

        // Beds
        ("red_bed_foot", "minecraft:red_bed[facing=north,part=foot]"),
        ("red_bed_head", "minecraft:red_bed[facing=north,part=head]"),
        ("blue_bed_foot", "minecraft:blue_bed[facing=north,part=foot]"),

        // Standard comparison models
        ("oak_stairs", "minecraft:oak_stairs[facing=east,half=bottom,shape=straight]"),
        ("oak_door_lower", "minecraft:oak_door[facing=north,half=lower,hinge=left,open=false]"),
    ];

    let mut copied_textures: HashMap<String, PathBuf> = HashMap::new();

    for (label, state_str) in test_cases {
        let bs = BlockState::parse(state_str)?;
        let def = load_blockstate_def(&bs.name);

        let baked = baker.bake_blockstate(state_str, def.as_ref(), load_model_from_mc)?;
        let (mesh, textures) = baked.to_mesh_with_textures(false);

        println!("Exporting '{}' ({} elements, {} vertices, {} faces)...", label, baked.elements.len(), mesh.vertex_count(), mesh.triangle_count());

        // 1. Resolve and copy textures
        let mut mat_map: Vec<String> = Vec::new();
        for (idx, tex_name) in textures.iter().enumerate() {
            let mat_name = format!("mat_{}_{}", label, idx);
            mat_map.push(mat_name.clone());

            let clean_tex = tex_name.strip_prefix("minecraft:").unwrap_or(tex_name);
            let tex_source_paths = vec![
                mc_assets_dir.join("minecraft/textures").join(format!("{}.png", clean_tex)),
                mc_assets_dir.join("minecraft/textures/entity").join(format!("{}.png", clean_tex)),
                mc_assets_dir.join("minecraft/textures/block").join(format!("{}.png", clean_tex)),
                mc_assets_dir.join("minecraft/textures/item").join(format!("{}.png", clean_tex)),
            ];

            let mut found_src = None;
            for p in &tex_source_paths {
                if p.exists() {
                    found_src = Some(p.clone());
                    break;
                }
            }

            let file_safe_name = clean_tex.replace('/', "_") + ".png";
            let dest_path = textures_out_dir.join(&file_safe_name);

            if let Some(src) = found_src {
                if !dest_path.exists() {
                    let _ = fs::copy(&src, &dest_path);
                }
                copied_textures.insert(tex_name.clone(), dest_path);
            } else {
                if !dest_path.exists() {
                    create_fallback_png(&dest_path, idx)?;
                }
                copied_textures.insert(tex_name.clone(), dest_path);
            }
        }

        // 2. Write MTL
        let mtl_filename = format!("{}.mtl", label);
        let mtl_path = output_dir.join(&mtl_filename);
        let mut mtl_content = String::new();
        for (idx, tex_name) in textures.iter().enumerate() {
            let mat_name = &mat_map[idx];
            let clean_tex = tex_name.strip_prefix("minecraft:").unwrap_or(tex_name);
            let file_safe_name = clean_tex.replace('/', "_") + ".png";
            mtl_content.push_str(&format!("newmtl {}\n", mat_name));
            mtl_content.push_str("Kd 1.0 1.0 1.0\n");
            mtl_content.push_str("Ka 0.2 0.2 0.2\n");
            mtl_content.push_str("Ks 0.0 0.0 0.0\n");
            mtl_content.push_str("d 1.0\n");
            mtl_content.push_str("illum 1\n");
            mtl_content.push_str(&format!("map_Kd ../textures/{}\n\n", file_safe_name));
        }
        fs::write(&mtl_path, mtl_content)?;

        // 3. Write OBJ
        let obj_filename = format!("{}.obj", label);
        let obj_path = output_dir.join(&obj_filename);
        let mut obj_content = String::new();
        obj_content.push_str(&format!("# MoziToolKit 2.0 Exported OBJ - {}\n", label));
        obj_content.push_str(&format!("mtllib {}\n", mtl_filename));
        obj_content.push_str(&format!("o {}\n", label));

        for p in &mesh.positions {
            obj_content.push_str(&format!("v {:.6} {:.6} {:.6}\n", p[0], p[1], p[2]));
        }
        for uv in &mesh.uvs {
            obj_content.push_str(&format!("vt {:.6} {:.6}\n", uv[0], uv[1]));
        }
        for n in &mesh.normals {
            obj_content.push_str(&format!("vn {:.6} {:.6} {:.6}\n", n[0], n[1], n[2]));
        }

        let mut current_slot = None;
        let num_triangles = mesh.indices.len() / 3;
        for tri_idx in 0..num_triangles {
            let quad_face_idx = tri_idx / 2;
            let slot = mesh.face_materials.get(quad_face_idx).copied().unwrap_or(0);
            if current_slot != Some(slot) {
                current_slot = Some(slot);
                let mat_name = mat_map.get(slot as usize).map(|s| s.as_str()).unwrap_or("default");
                obj_content.push_str(&format!("usemtl {}\n", mat_name));
            }

            let i0 = mesh.indices[tri_idx * 3] + 1;
            let i1 = mesh.indices[tri_idx * 3 + 1] + 1;
            let i2 = mesh.indices[tri_idx * 3 + 2] + 1;

            obj_content.push_str(&format!("f {}/{}/{} {}/{}/{} {}/{}/{}\n", i0, i0, i0, i1, i1, i1, i2, i2, i2));
        }

        fs::write(&obj_path, obj_content)?;
        println!(" -> Saved: {}", obj_path.display());
    }

    println!("------------------------------------------------------------");
    println!("Export completed successfully! Total textures copied: {}", copied_textures.len());
    println!("You can now import the .obj files from {} in Blender.", output_dir.display());

    Ok(())
}

fn create_fallback_png(path: &Path, color_idx: usize) -> Result<(), Box<dyn std::error::Error>> {
    let colors = [
        image::Rgba([200u8, 100, 100, 255]),
        image::Rgba([100, 200, 100, 255]),
        image::Rgba([100, 100, 200, 255]),
        image::Rgba([200, 200, 100, 255]),
        image::Rgba([200, 100, 200, 255]),
        image::Rgba([100, 200, 200, 255]),
    ];
    let c = colors[color_idx % colors.len()];

    let mut img = image::RgbaImage::new(16, 16);
    for y in 0..16 {
        for x in 0..16 {
            let is_border = x == 0 || y == 0 || x == 15 || y == 15;
            if is_border {
                img.put_pixel(x, y, image::Rgba([50, 50, 50, 255]));
            } else {
                img.put_pixel(x, y, c);
            }
        }
    }

    img.save(path)?;
    Ok(())
}


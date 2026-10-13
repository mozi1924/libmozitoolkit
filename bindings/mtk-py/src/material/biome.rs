//! Standalone Biome, Colormap, and Material attribute functions.

use super::resolver::PyBiomeResolver;
use crate::voxel::PyVoxelStorage;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::collections::HashMap;

/// Retrieve canonical metadata and colors for a single biome.
#[pyfunction]
#[pyo3(signature = (biome_name))]
pub fn get_biome_meta(py: Python<'_>, biome_name: &str) -> PyResult<PyObject> {
    if biome_name.eq_ignore_ascii_case("custom") {
        let dict = PyDict::new(py);
        dict.set_item("id", "custom")?;
        dict.set_item("name", "Custom")?;
        dict.set_item("temperature", 0.8f32)?;
        dict.set_item("humidity", 0.4f32)?;
        dict.set_item("grass_hex", "#91BD59")?;
        dict.set_item("foliage_hex", "#77AB2F")?;
        dict.set_item("dry_foliage_hex", "#A37546")?;
        dict.set_item("water_hex", "#3F76E4")?;
        dict.set_item("grass_linear", mtk_material::hex_to_linear_rgba("#91BD59"))?;
        dict.set_item(
            "foliage_linear",
            mtk_material::hex_to_linear_rgba("#77AB2F"),
        )?;
        dict.set_item(
            "dry_foliage_linear",
            mtk_material::hex_to_linear_rgba("#A37546"),
        )?;
        dict.set_item("water_linear", mtk_material::hex_to_linear_rgba("#3F76E4"))?;
        dict.set_item("colormap_uv", mtk_material::get_colormap_uv(0.8, 0.4))?;
        dict.set_item("has_custom_grass", false)?;
        dict.set_item("has_custom_foliage", false)?;
        dict.set_item("has_custom_dry_foliage", false)?;
        return Ok(dict.into());
    }

    let pal = mtk_material::get_biome_palette(biome_name);
    let dict = PyDict::new(py);
    dict.set_item("id", pal.id)?;
    dict.set_item("name", pal.name)?;
    dict.set_item("temperature", pal.temperature)?;
    dict.set_item("humidity", pal.humidity)?;
    dict.set_item("grass_hex", pal.grass_hex)?;
    dict.set_item("foliage_hex", pal.foliage_hex)?;
    dict.set_item("dry_foliage_hex", pal.dry_foliage_hex)?;
    dict.set_item("water_hex", pal.water_hex)?;
    dict.set_item("grass_linear", pal.grass_linear())?;
    dict.set_item("foliage_linear", pal.foliage_linear())?;
    dict.set_item("dry_foliage_linear", pal.dry_foliage_linear())?;
    dict.set_item("water_linear", pal.water_linear())?;
    dict.set_item("colormap_uv", pal.colormap_uv())?;
    dict.set_item("has_custom_grass", pal.has_custom_grass)?;
    dict.set_item("has_custom_foliage", pal.has_custom_foliage)?;
    dict.set_item("has_custom_dry_foliage", pal.has_custom_dry_foliage)?;
    Ok(dict.into())
}

/// Retrieve a list of all canonical vanilla biomes.
#[pyfunction]
pub fn get_all_biomes(py: Python<'_>) -> PyResult<PyObject> {
    let list = pyo3::types::PyList::empty(py);
    for pal in mtk_material::CANONICAL_BIOMES {
        let dict = PyDict::new(py);
        dict.set_item("id", pal.id)?;
        dict.set_item("name", pal.name)?;
        dict.set_item("temperature", pal.temperature)?;
        dict.set_item("humidity", pal.humidity)?;
        dict.set_item("grass_hex", pal.grass_hex)?;
        dict.set_item("foliage_hex", pal.foliage_hex)?;
        dict.set_item("dry_foliage_hex", pal.dry_foliage_hex)?;
        dict.set_item("water_hex", pal.water_hex)?;
        dict.set_item("grass_linear", pal.grass_linear())?;
        dict.set_item("foliage_linear", pal.foliage_linear())?;
        dict.set_item("dry_foliage_linear", pal.dry_foliage_linear())?;
        dict.set_item("water_linear", pal.water_linear())?;
        dict.set_item("colormap_uv", pal.colormap_uv())?;
        dict.set_item("has_custom_grass", pal.has_custom_grass)?;
        dict.set_item("has_custom_foliage", pal.has_custom_foliage)?;
        dict.set_item("has_custom_dry_foliage", pal.has_custom_dry_foliage)?;
        list.append(dict)?;
    }
    Ok(list.into())
}

/// Compute colormap UV coordinates from temperature and humidity.
#[pyfunction]
#[pyo3(signature = (temperature, humidity))]
pub fn get_colormap_uv(temperature: f32, humidity: f32) -> [f32; 2] {
    mtk_material::get_colormap_uv(temperature, humidity)
}

/// Convert sRGB [r, g, b, a] color to Linear RGBA.
#[pyfunction]
#[pyo3(signature = (color))]
pub fn srgb_to_linear(color: [f32; 4]) -> [f32; 4] {
    [
        mtk_material::srgb_to_linear(color[0]),
        mtk_material::srgb_to_linear(color[1]),
        mtk_material::srgb_to_linear(color[2]),
        color[3],
    ]
}

/// Convert Linear RGBA [r, g, b, a] color to sRGB.
#[pyfunction]
#[pyo3(signature = (color))]
pub fn linear_to_srgb(color: [f32; 4]) -> [f32; 4] {
    [
        mtk_material::linear_to_srgb(color[0]),
        mtk_material::linear_to_srgb(color[1]),
        mtk_material::linear_to_srgb(color[2]),
        color[3],
    ]
}

/// Standalone batch function to compute mesh biome attributes in Rust parallel Rayon.
#[pyfunction]
#[pyo3(signature = (
    face_texture_keys,
    biome_name="PLAINS",
    multi_biomes=None,
    resolver=None,
    custom_temp=None,
    custom_humidity=None,
    custom_grass=None,
    custom_foliage=None,
    custom_dry_foliage=None,
    custom_water=None,
    has_custom_grass=false,
    has_custom_foliage=false,
    has_custom_dry_foliage=false,
    face_biomes=None,
))]
pub fn compute_biome_tint_attributes(
    py: Python<'_>,
    face_texture_keys: Vec<String>,
    biome_name: &str,
    multi_biomes: Option<Vec<(String, f32)>>,
    resolver: Option<&PyBiomeResolver>,
    custom_temp: Option<f32>,
    custom_humidity: Option<f32>,
    custom_grass: Option<[f32; 4]>,
    custom_foliage: Option<[f32; 4]>,
    custom_dry_foliage: Option<[f32; 4]>,
    custom_water: Option<[f32; 4]>,
    has_custom_grass: bool,
    has_custom_foliage: bool,
    has_custom_dry_foliage: bool,
    face_biomes: Option<Vec<String>>,
) -> PyResult<PyObject> {
    let default_resolver = mtk_material::BiomeResolver::new();
    let res_ref = resolver.map(|r| &r.inner).unwrap_or(&default_resolver);

    let res = py.allow_threads(|| {
        if let Some(ref biomes) = face_biomes {
            mtk_material::compute_mesh_biome_attributes_per_face(
                &face_texture_keys,
                biomes,
                res_ref,
            )
        } else if biome_name.eq_ignore_ascii_case("custom")
            || custom_temp.is_some()
            || custom_humidity.is_some()
            || custom_grass.is_some()
            || custom_foliage.is_some()
            || custom_water.is_some()
        {
            let settings = mtk_material::CustomBiomeSettings {
                temperature: custom_temp.unwrap_or(0.8),
                humidity: custom_humidity.unwrap_or(0.4),
                grass_color: custom_grass,
                foliage_color: custom_foliage,
                dry_foliage_color: custom_dry_foliage,
                water_color: custom_water,
                has_custom_grass,
                has_custom_foliage,
                has_custom_dry_foliage,
            };
            mtk_material::compute_mesh_biome_attributes_custom(
                &face_texture_keys,
                &settings,
                res_ref,
            )
        } else {
            mtk_material::compute_mesh_biome_attributes(
                &face_texture_keys,
                biome_name,
                multi_biomes.as_deref(),
                res_ref,
            )
        }
    });

    let dict = PyDict::new(py);
    dict.set_item("packed_tint_data", res.packed_tint_data)?;
    dict.set_item("tint_colors", res.tint_colors)?;
    dict.set_item("colormap_uvs", res.colormap_uvs)?;
    Ok(dict.into())
}

/// Compute 3D coordinate-aware biome tinting attributes using a VoxelStorage container.
#[pyfunction]
#[pyo3(signature = (
    face_texture_keys,
    block_xs,
    block_ys,
    block_zs,
    storage,
    resolver=None,
    smooth=true,
))]
pub fn compute_mesh_biome_attributes_from_storage(
    py: Python<'_>,
    face_texture_keys: Vec<String>,
    block_xs: Vec<i32>,
    block_ys: Vec<i32>,
    block_zs: Vec<i32>,
    storage: &PyVoxelStorage,
    resolver: Option<&PyBiomeResolver>,
    smooth: bool,
) -> PyResult<PyObject> {
    let face_count = face_texture_keys.len();
    let default_resolver = mtk_material::BiomeResolver::new();
    let res_ref = resolver.map(|r| &r.inner).unwrap_or(&default_resolver);

    let (packed_tint_data, tint_colors, colormap_uvs) = py.allow_threads(|| {
        let mut packed = Vec::with_capacity(face_count);
        let mut cols = Vec::with_capacity(face_count);
        let mut uvs = Vec::with_capacity(face_count);

        for i in 0..face_count {
            let key = &face_texture_keys[i];
            let bx = block_xs.get(i).copied().unwrap_or(0);
            let by = block_ys.get(i).copied().unwrap_or(0);
            let bz = block_zs.get(i).copied().unwrap_or(0);

            let (col, uv, has_cg, has_cf, has_cdf) = if smooth {
                let sm = mtk_voxel::biome::get_smoothed_column_biome(
                    |x, y, z| storage.inner.get_biome(x, y, z),
                    bx,
                    by,
                    bz,
                );
                (
                    sm,
                    [sm.colormap_uv[0], sm.colormap_uv[1], 0.0],
                    sm.has_custom_grass,
                    sm.has_custom_foliage,
                    sm.has_custom_dry_foliage,
                )
            } else {
                let b_name = storage.inner.get_biome(bx, by, bz);
                let sm = mtk_voxel::biome::get_point_column_biome(b_name, bx, bz);
                (
                    sm,
                    [sm.colormap_uv[0], sm.colormap_uv[1], 0.0],
                    sm.has_custom_grass,
                    sm.has_custom_foliage,
                    sm.has_custom_dry_foliage,
                )
            };

            let tint_info = res_ref.get_tint_info(key, None, None);
            let tt = tint_info.tint_type;
            let (c, has_custom) = match tt {
                mtk_material::TINT_TYPE_GRASS => (col.grass_color, has_cg),
                mtk_material::TINT_TYPE_FOLIAGE => (col.foliage_color, has_cf),
                mtk_material::TINT_TYPE_DRY_FOLIAGE => (col.dry_foliage_color, has_cdf),
                mtk_material::TINT_TYPE_WATER => (col.water_color, false),
                mtk_material::TINT_TYPE_HARDCODED => (
                    tint_info.hardcoded_color.unwrap_or([1.0, 1.0, 1.0, 1.0]),
                    true,
                ),
                _ => (
                    if tint_info.is_hardcoded {
                        tint_info.hardcoded_color.unwrap_or([1.0, 1.0, 1.0, 1.0])
                    } else {
                        [1.0, 1.0, 1.0, 1.0]
                    },
                    false,
                ),
            };

            let tint_type_val = if tint_info.is_hardcoded || has_custom {
                mtk_material::TINT_TYPE_HARDCODED as f32
            } else {
                tt as f32
            };

            packed.push([
                tint_info.base_tint_weight,
                tint_info.overlay_tint_weight,
                tint_info.tint_weight,
                tint_type_val,
            ]);
            cols.push(c);
            uvs.push(uv);
        }

        (packed, cols, uvs)
    });

    let dict = PyDict::new(py);
    dict.set_item("packed_tint_data", packed_tint_data)?;
    dict.set_item("tint_colors", tint_colors)?;
    dict.set_item("colormap_uvs", colormap_uvs)?;
    Ok(dict.into())
}

/// Classify a texture stem and/or block name into an authoritative tint category:
/// "grass", "foliage", "dry_foliage", "water", "hardcoded", or "none".
#[pyfunction]
#[pyo3(signature = (clean_stem, block_name=None, tint_index=None))]
pub fn classify_tint_category(
    clean_stem: &str,
    block_name: Option<&str>,
    tint_index: Option<i32>,
) -> &'static str {
    mtk_material::classify_tint_category(clean_stem, block_name, tint_index)
}

/// Compute a smooth blended Linear RGBA color across multiple weighted biomes.
#[pyfunction]
#[pyo3(signature = (biome_weights, tint_type="grass"))]
pub fn blend_biome_colors(biome_weights: Vec<(String, f32)>, tint_type: &str) -> [f32; 4] {
    let weights_ref: Vec<(&str, f32)> = biome_weights
        .iter()
        .map(|(s, w)| (s.as_str(), *w))
        .collect();
    mtk_material::blend_biome_colors(&weights_ref, tint_type)
}

/// Sample an sRGB color [r, g, b] from 2D colormap image bytes using canonical Minecraft coordinates.
#[pyfunction]
#[pyo3(signature = (image_pixels, width, height, temperature, downfall, channels=4))]
pub fn sample_colormap_pixel(
    image_pixels: &[u8],
    width: u32,
    height: u32,
    temperature: f32,
    downfall: f32,
    channels: usize,
) -> [f32; 3] {
    mtk_material::sample_colormap_pixel(
        image_pixels,
        width,
        height,
        temperature,
        downfall,
        channels,
    )
}

/// Compute packed material properties `[emission, thin_wall, transmission, sticker_threshold]`
/// for a list of face texture keys in parallel Rayon in Rust.
#[pyfunction]
#[pyo3(signature = (face_texture_keys, block_names=None))]
pub fn compute_mesh_material_props(
    py: Python<'_>,
    face_texture_keys: Vec<String>,
    block_names: Option<Vec<String>>,
) -> PyResult<Vec<[f32; 4]>> {
    let res = py.allow_threads(|| {
        mtk_material::compute_mesh_material_props(&face_texture_keys, block_names.as_deref())
    });
    Ok(res)
}

/// Compute flat packed material properties (float array of length len * 4)
/// for zero-copy memoryview / foreach_set injection into Blender Mesh.
#[pyfunction]
#[pyo3(signature = (face_texture_keys, block_names=None))]
pub fn compute_flat_material_props(
    py: Python<'_>,
    face_texture_keys: Vec<String>,
    block_names: Option<Vec<String>>,
) -> PyResult<Vec<f32>> {
    let res = py.allow_threads(|| {
        let props =
            mtk_material::compute_mesh_material_props(&face_texture_keys, block_names.as_deref());
        let mut flat = Vec::with_capacity(props.len() * 4);
        for p in props {
            flat.extend_from_slice(&p);
        }
        flat
    });
    Ok(res)
}

/// Query individual block material physical properties `[emission, thin_wall, transmission, sticker_threshold]`.
#[pyfunction]
#[pyo3(signature = (block_name="", texture_name=None))]
pub fn get_material_props(block_name: &str, texture_name: Option<&str>) -> [f32; 4] {
    mtk_material::get_material_props(block_name, texture_name)
}

/// Query canonical emission strength for a block / texture (0.0 .. 15.0).
#[pyfunction]
#[pyo3(signature = (block_name="", properties=None, texture_name=None))]
pub fn get_block_emission_strength(
    block_name: &str,
    properties: Option<HashMap<String, String>>,
    texture_name: Option<&str>,
) -> f32 {
    mtk_material::properties::get_block_emission_strength(
        block_name,
        properties.as_ref(),
        texture_name,
    )
}

/// Query whether a block or texture is thin wall foliage / vegetation.
#[pyfunction]
#[pyo3(signature = (block_name="", texture_name=None))]
pub fn is_thin_wall_block(block_name: &str, texture_name: Option<&str>) -> bool {
    mtk_material::properties::is_thin_wall_block(block_name, texture_name)
}

/// Query whether a block or texture is a dielectric transmissive medium (glass, water, ice, etc.).
#[pyfunction]
#[pyo3(signature = (block_name="", texture_name=None))]
pub fn is_transmissive_block(block_name: &str, texture_name: Option<&str>) -> bool {
    mtk_material::properties::is_transmissive_block(block_name, texture_name)
}

/// Query transmission weight (1.0 for glass/water/ice, 0.0 otherwise).
#[pyfunction]
#[pyo3(signature = (block_name="", texture_name=None))]
pub fn get_block_transmission_weight(block_name: &str, texture_name: Option<&str>) -> f32 {
    mtk_material::properties::get_block_transmission_weight(block_name, texture_name)
}

/// Query alpha sticker threshold (0.55 for glass, 0.95 for water/ice/slime/honey).
#[pyfunction]
#[pyo3(signature = (block_name="", texture_name=None))]
pub fn get_block_sticker_threshold(block_name: &str, texture_name: Option<&str>) -> f32 {
    mtk_material::properties::get_block_sticker_threshold(block_name, texture_name)
}

/// Register / merge external material properties into the Rust global registry.
/// Accepts either a JSON string or a Python dict.
#[pyfunction]
#[pyo3(signature = (properties))]
pub fn register_material_properties(py: Python<'_>, properties: &Bound<'_, PyAny>) -> PyResult<()> {
    let json_str = if let Ok(s) = properties.extract::<String>() {
        s
    } else {
        let json_mod = py.import("json")?;
        let dumps = json_mod.getattr("dumps")?;
        dumps.call1((properties,))?.extract::<String>()?
    };

    mtk_material::register_material_properties_json(&json_str).map_err(|e| {
        pyo3::exceptions::PyValueError::new_err(format!("Invalid material properties JSON: {}", e))
    })?;
    Ok(())
}

/// Fully replace the global material properties registry from a JSON string or Python dict.
#[pyfunction]
#[pyo3(signature = (properties))]
pub fn load_material_properties_replace(
    py: Python<'_>,
    properties: &Bound<'_, PyAny>,
) -> PyResult<()> {
    let json_str = if let Ok(s) = properties.extract::<String>() {
        s
    } else {
        let json_mod = py.import("json")?;
        let dumps = json_mod.getattr("dumps")?;
        dumps.call1((properties,))?.extract::<String>()?
    };

    mtk_material::load_material_properties_json_replace(&json_str).map_err(|e| {
        pyo3::exceptions::PyValueError::new_err(format!("Invalid material properties JSON: {}", e))
    })?;
    Ok(())
}

/// Reset the global material properties registry to built-in vanilla defaults.
#[pyfunction]
pub fn reset_material_properties_to_default() {
    mtk_material::reset_material_properties_to_default();
}

//! # `mtk-py` (Python Bindings for libmozitoolkit)
//!
//! Exposes `libmtk` data structures and high-performance algorithms to Python via PyO3.

pub mod cull;
pub mod material;
pub mod mesh;
pub mod mesher;
pub mod model;
pub mod protocol;
pub mod resource;
pub mod sync;
pub mod texture;
pub mod uv;
pub mod voxel;

use pyo3::prelude::*;

pub use cull::PyFaceCuller;
pub use material::{
    compute_biome_tint_attributes, get_all_biomes, get_biome_meta, PyBiomeResolver,
    PyGridAtlasSpec, PyMaterialResolver,
};
pub use mesh::{PyAttributeDomain, PyMeshData};
pub use mesher::PySectionMesher;
pub use model::{PyBakedModelDatabase, PyModelBaker};
pub use protocol::{decode_packet, encode_full_sync_request, encode_repair_requests, encode_sync_config};
pub use resource::{precompile_all_assets, PyPrecompileResult, PyResourcePackStack};
pub use sync::PyLiveSyncSession;
pub use texture::{PyAtlasBuilder, PyBakedAtlas, PyStandaloneBuilder, PyStandaloneResult};
pub use voxel::{PyMesherConfig, PyVoxelStorage};

/// Unified high-performance mesh processing pipeline entrypoint.
///
/// Takes input raw mesh, material names, optional baked atlas, and optional grid atlas spec.
/// Computes multi-threaded status resolution, UV remapping, secondary [0, 1] PBR UVs,
/// and returns the transformed MeshData along with structured material assignments.
#[pyfunction]
#[pyo3(signature = (mesh, material_names, atlas=None, aliases=None, generate_secondary_uv=true, grid_atlas_spec=None))]
pub fn process_mesh<'py>(
    py: Python<'py>,
    mesh: &PyMeshData,
    material_names: Vec<String>,
    atlas: Option<&PyBakedAtlas>,
    aliases: Option<std::collections::HashMap<String, Vec<String>>>,
    generate_secondary_uv: bool,
    grid_atlas_spec: Option<&PyGridAtlasSpec>,
) -> PyResult<(PyMeshData, Bound<'py, pyo3::types::PyList>, Bound<'py, pyo3::types::PyDict>)> {
    let cfg = libmtk::MeshPipelineConfig {
        custom_aliases: aliases,
        generate_secondary_uv,
        grid_atlas_spec: grid_atlas_spec.map(|s| s.inner.clone()),
    };

    let addr_map = atlas.map(|a| &a.inner.address_map);
    let output = libmtk::process_mesh(&mesh.inner, &material_names, addr_map, &cfg);

    // Build material metadata list
    let mat_list = pyo3::types::PyList::empty(py);
    for mat in output.materials {
        let dict = pyo3::types::PyDict::new(py);
        dict.set_item("raw_name", mat.raw_name)?;
        dict.set_item("canonical_name", mat.canonical_name)?;
        dict.set_item("atlas_chunk_id", mat.atlas_chunk_id)?;
        dict.set_item("is_animated", mat.is_animated)?;
        mat_list.append(dict)?;
    }

    // Build stats dictionary
    let stats_dict = pyo3::types::PyDict::new(py);
    stats_dict.set_item("input_vertices", output.stats.input_vertices)?;
    stats_dict.set_item("input_faces", output.stats.input_faces)?;
    stats_dict.set_item("output_vertices", output.stats.output_vertices)?;
    stats_dict.set_item("output_faces", output.stats.output_faces)?;
    stats_dict.set_item("resolved_slots", output.stats.resolved_slots)?;
    stats_dict.set_item("unmapped_slots", output.stats.unmapped_slots)?;

    Ok((PyMeshData { inner: output.mesh }, mat_list, stats_dict))
}

/// Returns libmtk version string.
#[pyfunction]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Main entrypoint for Python module `libmtk_py`.
#[pymodule]
fn libmtk_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // 1. Mesh & Core
    m.add_class::<PyAttributeDomain>()?;
    m.add_class::<PyMeshData>()?;
    m.add_function(wrap_pyfunction!(process_mesh, m)?)?;
    m.add_function(wrap_pyfunction!(mesh::calculate_face_target_grid, m)?)?;
    m.add_function(wrap_pyfunction!(mesh::adaptive_pixel_split_mesh, m)?)?;

    // 2. Culling
    m.add_class::<PyFaceCuller>()?;
    m.add_function(wrap_pyfunction!(cull::cull_mesh_faces, m)?)?;

    // 3. Voxel Storage & Config
    m.add_class::<PyVoxelStorage>()?;
    m.add_class::<PyMesherConfig>()?;

    // 4. Meshing Generator
    m.add_class::<PySectionMesher>()?;

    // 5. Live Sync & Protocol
    m.add_class::<PyLiveSyncSession>()?;
    m.add_function(wrap_pyfunction!(decode_packet, m)?)?;
    m.add_function(wrap_pyfunction!(encode_full_sync_request, m)?)?;
    m.add_function(wrap_pyfunction!(encode_repair_requests, m)?)?;
    m.add_function(wrap_pyfunction!(encode_sync_config, m)?)?;

    // 6. Resource Pack & Textures
    m.add_class::<PyResourcePackStack>()?;
    m.add_class::<PyAtlasBuilder>()?;
    m.add_class::<PyBakedAtlas>()?;
    m.add_class::<PyStandaloneBuilder>()?;
    m.add_class::<PyStandaloneResult>()?;
    m.add_class::<PyPrecompileResult>()?;
    m.add_function(wrap_pyfunction!(precompile_all_assets, m)?)?;
    m.add_function(wrap_pyfunction!(texture::sample_uv_alpha_f32, m)?)?;
    m.add_function(wrap_pyfunction!(texture::is_face_transparent_f32, m)?)?;
    m.add_function(wrap_pyfunction!(texture::batch_analyze_transparent_faces_f32, m)?)?;
    m.add_function(wrap_pyfunction!(texture::batch_analyze_transparent_faces_u8, m)?)?;

    // 7. Material & UV Remapper & Biome
    m.add_class::<PyGridAtlasSpec>()?;
    m.add_class::<PyMaterialResolver>()?;
    m.add_class::<PyBiomeResolver>()?;
    m.add_function(wrap_pyfunction!(compute_biome_tint_attributes, m)?)?;
    m.add_function(wrap_pyfunction!(get_biome_meta, m)?)?;
    m.add_function(wrap_pyfunction!(get_all_biomes, m)?)?;
    m.add_function(wrap_pyfunction!(material::get_colormap_uv, m)?)?;
    m.add_function(wrap_pyfunction!(material::srgb_to_linear, m)?)?;
    m.add_function(wrap_pyfunction!(material::linear_to_srgb, m)?)?;

    // 8. Model Baker
    m.add_class::<PyModelBaker>()?;
    m.add_class::<PyBakedModelDatabase>()?;

    // 9. UV Geometry Algorithms & Extrude
    m.add_function(wrap_pyfunction!(uv::calculate_uv_area, m)?)?;
    m.add_function(wrap_pyfunction!(uv::get_uv_bounds, m)?)?;
    m.add_function(wrap_pyfunction!(uv::get_uv_center, m)?)?;
    m.add_function(wrap_pyfunction!(uv::is_uv_collapsed, m)?)?;
    m.add_function(wrap_pyfunction!(uv::is_orthogonal_angle, m)?)?;
    m.add_function(wrap_pyfunction!(uv::detect_uv_rotation, m)?)?;
    m.add_function(wrap_pyfunction!(uv::straighten_uv, m)?)?;
    m.add_function(wrap_pyfunction!(uv::scale_uv, m)?)?;
    m.add_function(wrap_pyfunction!(uv::normalize_uv_for_atlas_tiling, m)?)?;
    m.add_function(wrap_pyfunction!(uv::uv_requires_atlas_tiling, m)?)?;
    m.add_function(wrap_pyfunction!(uv::restore_atlas_tiling_uv, m)?)?;
    m.add_function(wrap_pyfunction!(uv::repair_quad_fluid_uv, m)?)?;
    m.add_function(wrap_pyfunction!(uv::batch_repair_fluid_uv, m)?)?;
    m.add_function(wrap_pyfunction!(uv::get_fluid_top_uvs, m)?)?;
    m.add_function(wrap_pyfunction!(uv::get_fluid_side_uvs, m)?)?;
    m.add_function(wrap_pyfunction!(uv::repair_extruded_side_uv, m)?)?;
    m.add_function(wrap_pyfunction!(uv::generate_random_extrude_heights, m)?)?;

    // 10. Metadata
    m.add_function(wrap_pyfunction!(version, m)?)?;

    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_python_mesh_data_inner() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let mut mesh = PyMeshData::new();
            assert_eq!(mesh.vertex_count(), 0);
            assert!(mesh.is_empty());

            let res = mesh.append_unit_cube_face(1, 7, -1);
            assert!(res.is_ok());
            assert_eq!(mesh.vertex_count(), 4);
            assert_eq!(mesh.triangle_count(), 2);
            assert_eq!(mesh.face_count(), 1);

            let memview = mesh.positions_memoryview(py).unwrap();
            assert_eq!(memview.len().unwrap(), 4 * 3 * 4); // 4 verts * 3 floats * 4 bytes

            let indices_mem = mesh.indices_memoryview(py).unwrap();
            assert_eq!(indices_mem.len().unwrap(), 6 * 4); // 6 indices * 4 bytes

            mesh.clear();
            assert_eq!(mesh.vertex_count(), 0);
            assert!(mesh.is_empty());
        });
    }

    #[test]
    fn test_python_voxel_storage_and_mesher() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let mut storage = PyVoxelStorage::new();
            storage.set_bounds(0, 0, 0, 16, 16, 16);
            storage.set_block(0, 0, 0, "minecraft:stone", None);
            assert_eq!(storage.get_block(0, 0, 0), "minecraft:stone");
            assert_eq!(storage.dirty_section_count(), 1);

            let config = PyMesherConfig::new(true, true, true, None);
            let mesh = PySectionMesher::mesh_world(&storage, Some(&config), None).unwrap();
            assert!(!mesh.is_empty());
            assert_eq!(mesh.vertex_count(), 24); // 6 faces * 4 verts for isolated block

            let dict = PySectionMesher::mesh_sections_split(py, &storage, Some(&config), None).unwrap();
            assert_eq!(dict.len(), 1);

            let rebuilt = PySectionMesher::rebuild_dirty_sections(py, &mut storage, Some(&config), None).unwrap();
            assert_eq!(rebuilt.len(), 1);
            assert_eq!(storage.dirty_section_count(), 0);
        });
    }

    #[test]
    fn test_python_live_sync_session_and_protocol() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            // Test packet decode
            let pkt_bytes = [
                0x4D, 0x43, 0x01, 0x01,
                0x00, 0x00, 0x00, 0x00,
                0x40, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00,
                0x10, 0x00, 0x00, 0x00,
                0x10, 0x00, 0x00, 0x00,
                0x10, 0x00, 0x00, 0x00,
            ];
            let dict = decode_packet(py, &pkt_bytes).unwrap();
            assert_eq!(dict.get_item("type").unwrap().unwrap().extract::<String>().unwrap(), "SELECTION_INFO");

            // Test session
            let session = PyLiveSyncSession::new(None, None);
            let events = session.poll_events(py).unwrap();
            assert_eq!(events.len(), 0);
        });
    }

    #[test]
    fn test_python_process_mesh_end_to_end() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let positions = vec![
                0.0, 0.0, 0.0,
                1.0, 0.0, 0.0,
                1.0, 1.0, 0.0,
                0.0, 1.0, 0.0,
            ];
            let uvs = vec![
                0.0, 0.0,
                1.0, 0.0,
                1.0, 1.0,
                0.0, 1.0,
            ];
            let indices = vec![0, 1, 2, 0, 2, 3];
            let mesh = PyMeshData::from_raw_buffers(positions, uvs, indices, None, None).unwrap();
            assert_eq!(mesh.vertex_count(), 4);
            assert_eq!(mesh.face_count(), 2);

            let mapping_json = r#"{
                "version": 1,
                "sprites": {
                    "minecraft:block/stone": {
                        "chunk_id": 0,
                        "uv_bounds": [0.0, 0.0, 0.5, 0.5],
                        "frame_0_uv_bounds": [0.0, 0.0, 0.5, 0.5],
                        "local_uv_bounds": [0.0, 0.0, 1.0, 1.0],
                        "is_animated": false,
                        "frame_count": 1,
                        "has_normal": false,
                        "has_specular": false
                    }
                }
            }"#;
            let atlas = PyBakedAtlas::from_mapping_json(mapping_json).unwrap();
            let mat_names = vec!["Tile_Stone".to_string(), "Tile_Stone".to_string()];

            let (out_mesh, mats, stats) = process_mesh(
                py,
                &mesh,
                mat_names,
                Some(&atlas),
                None,
                true,
                None,
            ).unwrap();

            assert_eq!(out_mesh.vertex_count(), 4);
            assert_eq!(mats.len(), 2);
            assert_eq!(stats.get_item("resolved_slots").unwrap().unwrap().extract::<usize>().unwrap(), 2);
            let sec_uvs = out_mesh.secondary_uvs_memoryview(py).unwrap();
            assert!(sec_uvs.is_some());
        });
    }

    #[test]
    fn test_python_version_string() {
        assert_eq!(version(), env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn test_python_uv_functions() {
        let quad = vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
        assert!((uv::calculate_uv_area(quad.clone()) - 1.0).abs() < 1e-6);

        let bounds = uv::get_uv_bounds(quad.clone());
        assert_eq!(bounds, (0.0, 0.0, 1.0, 1.0, 1.0, 1.0));

        let center = uv::get_uv_center(quad.clone());
        assert_eq!(center, (0.5, 0.5));

        assert!(!uv::is_uv_collapsed(quad.clone(), None, None, None));
        assert!(uv::is_orthogonal_angle(0.0, 1e-3));
        assert!((uv::detect_uv_rotation(quad.clone(), 1e-3)).abs() < 1e-6);

        let (ang, straightened, new_uvs) = uv::straighten_uv(quad.clone(), None);
        assert!(!straightened);
        assert_eq!(ang, 0.0);
        assert_eq!(new_uvs.len(), 4);

        let scaled = uv::scale_uv(quad.clone(), 0.5);
        assert_eq!(scaled[0], (0.25, 0.25));

        let (norm, scale, loc) = uv::normalize_uv_for_atlas_tiling(quad.clone(), 1e-6);
        assert_eq!(norm.len(), 4);
        assert_eq!(scale, (1.0, 1.0, 1.0));
        assert_eq!(loc, (0.0, 0.0, 0.0));

        assert!(!uv::uv_requires_atlas_tiling(quad, 1e-4));
        assert_eq!(uv::restore_atlas_tiling_uv(0.5, 0.5, (1.0, 1.0, 1.0), (0.0, 0.0, 0.0), 0.0), (0.5, 0.5));

        let verts = vec![
            (0.0, 0.0, 1.0),
            (0.0, 0.0, 0.0),
            (0.0, 0.2, 0.0),
            (0.0, 0.8, 1.0),
        ];
        let inv_uvs = vec![
            (1.0, 0.0),
            (0.0, 0.0),
            (0.0, 0.8),
            (1.0, 0.2),
        ];
        let (repaired, out_uvs) = uv::repair_quad_fluid_uv(verts, inv_uvs, None, false, 0.005).unwrap();
        assert!(repaired);
        assert!((out_uvs[2].1 - 0.2).abs() < 1e-5);
        assert!((out_uvs[3].1 - 0.8).abs() < 1e-5);

        let top_uvs = uv::get_fluid_top_uvs(true, 0.0);
        assert_eq!(top_uvs.len(), 4);
        assert_eq!(top_uvs[0], (0.25, 0.25));

        let side_uvs = uv::get_fluid_side_uvs(0.8, 0.2);
        assert_eq!(side_uvs.len(), 4);
        assert!((side_uvs[0].1 - 0.1).abs() < 1e-6);
    }
}


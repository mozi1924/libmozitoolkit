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
pub mod voxel;

use pyo3::prelude::*;

pub use cull::PyFaceCuller;
pub use material::PyMaterialResolver;
pub use mesh::PyMeshData;
pub use mesher::PySectionMesher;
pub use model::{PyBakedModelDatabase, PyModelBaker};
pub use protocol::{decode_packet, encode_full_sync_request, encode_repair_requests, encode_sync_config};
pub use resource::PyResourcePackStack;
pub use sync::PyLiveSyncSession;
pub use texture::{PyAtlasBuilder, PyBakedAtlas, PyStandaloneBuilder, PyStandaloneResult};
pub use voxel::{PyMesherConfig, PyVoxelStorage};

/// Unified high-performance mesh processing pipeline entrypoint.
///
/// Takes input raw mesh, material names, optional baked atlas, and origin type.
/// Computes multi-threaded status resolution, UV remapping, secondary [0, 1] PBR UVs,
/// and returns the transformed MeshData along with structured material assignments.
#[pyfunction]
#[pyo3(signature = (mesh, material_names, atlas=None, origin="auto", generate_secondary_uv=true, mineways_size=None))]
pub fn process_mesh<'py>(
    py: Python<'py>,
    mesh: &PyMeshData,
    material_names: Vec<String>,
    atlas: Option<&PyBakedAtlas>,
    origin: &str,
    generate_secondary_uv: bool,
    mineways_size: Option<(u32, u32)>,
) -> PyResult<(PyMeshData, Bound<'py, pyo3::types::PyList>, Bound<'py, pyo3::types::PyDict>)> {
    let orig = mtk_material::ImporterOrigin::parse(origin);
    let (mw_w, mw_h) = mineways_size.unwrap_or((1024, 1024));

    let cfg = libmtk::MeshPipelineConfig {
        origin: orig,
        generate_secondary_uv,
        mineways_width: mw_w,
        mineways_height: mw_h,
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
    m.add_class::<PyMeshData>()?;
    m.add_function(wrap_pyfunction!(process_mesh, m)?)?;

    // 2. Culling
    m.add_class::<PyFaceCuller>()?;

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

    // 7. Material & UV Remapper
    m.add_class::<PyMaterialResolver>()?;

    // 8. Model Baker
    m.add_class::<PyModelBaker>()?;
    m.add_class::<PyBakedModelDatabase>()?;

    // 9. Metadata
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
                "auto",
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
}

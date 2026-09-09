//! # `mtk-py` (Python Bindings for libmozitoolkit)
//!
//! Exposes `libmtk` data structures and high-performance algorithms to Python via PyO3.

pub mod cull;
pub mod material;
pub mod mesh;
pub mod mesher;
pub mod resource;
pub mod texture;
pub mod voxel;

use pyo3::prelude::*;

pub use cull::PyFaceCuller;
pub use material::PyMaterialResolver;
pub use mesh::PyMeshData;
pub use mesher::PySectionMesher;
pub use resource::PyResourcePackStack;
pub use texture::{PyAtlasBuilder, PyBakedAtlas};
pub use voxel::{PyMesherConfig, PyVoxelStorage};

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

    // 2. Culling
    m.add_class::<PyFaceCuller>()?;

    // 3. Voxel Storage & Config
    m.add_class::<PyVoxelStorage>()?;
    m.add_class::<PyMesherConfig>()?;

    // 4. Meshing Generator
    m.add_class::<PySectionMesher>()?;

    // 5. Resource Pack & Textures
    m.add_class::<PyResourcePackStack>()?;
    m.add_class::<PyAtlasBuilder>()?;
    m.add_class::<PyBakedAtlas>()?;

    // 6. Material & UV Remapper
    m.add_class::<PyMaterialResolver>()?;

    // 7. Metadata
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
    fn test_python_version_string() {
        assert_eq!(version(), env!("CARGO_PKG_VERSION"));
    }
}

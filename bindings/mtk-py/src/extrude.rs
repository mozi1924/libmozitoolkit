//! # `mtk-py` Extrusion & Side UV Repair Bindings
//!
//! Exposes batch discrete face extrusion, UV reconstruction, and random noise displacement to Python.

use pyo3::prelude::*;

/// Performs high-performance batch Data In, Data Out UV repair across an entire mesh.
#[pyfunction]
#[pyo3(signature = (
    positions,
    face_vertices,
    face_uvs,
    face_materials,
    selected_faces,
    pixel_steps,
    uv_mode="SMART",
    repair_uv=true,
    add_crease=false,
    crease_val=1.0,
    only_collapsed=false,
    smart_side_faces=None
))]
pub fn process_mesh_extrude_repair(
    py: Python<'_>,
    positions: Vec<[f32; 3]>,
    face_vertices: Vec<Vec<u32>>,
    face_uvs: Vec<Vec<[f32; 2]>>,
    face_materials: Vec<u32>,
    selected_faces: Vec<u32>,
    pixel_steps: Vec<[f32; 2]>,
    uv_mode: &str,
    repair_uv: bool,
    add_crease: bool,
    crease_val: f32,
    only_collapsed: bool,
    smart_side_faces: Option<Vec<u32>>,
) -> (
    Vec<(u32, Vec<[f32; 2]>)>,
    Vec<(u32, u32)>,
    Vec<((u32, u32), f32)>,
    usize,
) {
    let mode = match uv_mode.to_uppercase().as_str() {
        "INWARD" => mtk_core::extrude::ExtrudeUvMode::Inward,
        "OUTWARD" => mtk_core::extrude::ExtrudeUvMode::Outward,
        _ => mtk_core::extrude::ExtrudeUvMode::Smart,
    };

    let input = mtk_core::extrude_mesh::ExtrudeMeshInput {
        positions,
        face_vertices,
        face_uvs,
        face_materials,
        selected_faces,
        pixel_steps,
        smart_side_faces,
        config: mtk_core::extrude_mesh::MeshExtrudeRepairConfig {
            uv_mode: mode,
            repair_uv,
            add_crease,
            crease_val,
            only_collapsed,
        },
    };

    let out = py.allow_threads(|| mtk_core::extrude_mesh::process_mesh_extrude_repair(&input));
    (
        out.modified_face_uvs,
        out.modified_face_materials,
        out.modified_edge_creases,
        out.repaired_count,
    )
}

/// Performs high-performance batch Data In, Data Out UV repair across an entire mesh
/// using flat contiguous 1D buffers for zero-copy memory ingestion.
#[pyfunction]
#[pyo3(signature = (
    positions,
    loop_vertices,
    loop_uvs,
    face_loop_starts,
    face_loop_totals,
    face_materials,
    selected_faces,
    pixel_steps,
    uv_mode="SMART",
    repair_uv=true,
    add_crease=false,
    crease_val=1.0,
    only_collapsed=false,
    smart_side_faces=None
))]
pub fn process_flat_mesh_extrude_repair(
    py: Python<'_>,
    positions: Vec<f32>,
    loop_vertices: Vec<u32>,
    loop_uvs: Vec<f32>,
    face_loop_starts: Vec<u32>,
    face_loop_totals: Vec<u32>,
    face_materials: Vec<u32>,
    selected_faces: Vec<u32>,
    pixel_steps: Vec<[f32; 2]>,
    uv_mode: &str,
    repair_uv: bool,
    add_crease: bool,
    crease_val: f32,
    only_collapsed: bool,
    smart_side_faces: Option<Vec<u32>>,
) -> (
    Vec<(u32, Vec<[f32; 2]>)>,
    Vec<(u32, u32)>,
    Vec<((u32, u32), f32)>,
    usize,
) {
    let mode = match uv_mode.to_uppercase().as_str() {
        "INWARD" => mtk_core::extrude::ExtrudeUvMode::Inward,
        "OUTWARD" => mtk_core::extrude::ExtrudeUvMode::Outward,
        _ => mtk_core::extrude::ExtrudeUvMode::Smart,
    };

    let flat_mesh = mtk_core::polygon::FlatPolygonMesh::from_flat_buffers(
        &positions,
        loop_vertices,
        &loop_uvs,
        face_loop_starts,
        face_loop_totals,
        face_materials,
    );

    let config = mtk_core::extrude_mesh::MeshExtrudeRepairConfig {
        uv_mode: mode,
        repair_uv,
        add_crease,
        crease_val,
        only_collapsed,
    };

    let out = py.allow_threads(|| {
        mtk_core::extrude_mesh::process_flat_mesh_extrude_repair(
            &flat_mesh,
            &selected_faces,
            &pixel_steps,
            &config,
            smart_side_faces.as_deref(),
        )
    });

    (
        out.modified_face_uvs,
        out.modified_face_materials,
        out.modified_edge_creases,
        out.repaired_count,
    )
}

/// Performs complete batch discrete face extrusion, 3D noise vertex displacement, topology rebuilding,
/// and automatic side UV repair in a single batch pass (Data In, Data Out).
#[pyfunction]
#[pyo3(signature = (
    positions,
    face_vertices,
    face_uvs,
    face_materials,
    selected_faces,
    pixel_steps,
    min_height=0.0,
    max_height=0.1,
    seed=0,
    noise_type="RANDOM",
    noise_scale=1.0,
    repair_uv=true,
    uv_mode="SMART",
    add_crease=false,
    crease_val=1.0
))]
pub fn process_random_extrude_mesh(
    py: Python<'_>,
    positions: Vec<[f32; 3]>,
    face_vertices: Vec<Vec<u32>>,
    face_uvs: Vec<Vec<[f32; 2]>>,
    face_materials: Vec<u32>,
    selected_faces: Vec<u32>,
    pixel_steps: Vec<[f32; 2]>,
    min_height: f32,
    max_height: f32,
    seed: u32,
    noise_type: &str,
    noise_scale: f32,
    repair_uv: bool,
    uv_mode: &str,
    add_crease: bool,
    crease_val: f32,
) -> (
    Vec<[f32; 3]>,
    Vec<Vec<u32>>,
    Vec<Vec<[f32; 2]>>,
    Vec<u32>,
    Vec<u32>,
    usize,
) {
    let n_type = match noise_type.to_uppercase().as_str() {
        "PERLIN" => mtk_core::extrude::ExtrudeNoiseType::Perlin,
        "CELL" | "CELLULAR" | "VORONOI" => mtk_core::extrude::ExtrudeNoiseType::Cellular,
        _ => mtk_core::extrude::ExtrudeNoiseType::UniformRandom,
    };

    let mode = match uv_mode.to_uppercase().as_str() {
        "INWARD" => mtk_core::extrude::ExtrudeUvMode::Inward,
        "OUTWARD" => mtk_core::extrude::ExtrudeUvMode::Outward,
        _ => mtk_core::extrude::ExtrudeUvMode::Smart,
    };

    let input = mtk_core::extrude_mesh::RandomExtrudeMeshInput {
        positions,
        face_vertices,
        face_uvs,
        face_materials,
        selected_faces,
        pixel_steps,
        min_height,
        max_height,
        seed,
        noise_type: n_type,
        noise_scale,
        repair_uv,
        uv_mode: mode,
        add_crease,
        crease_val,
    };

    let out = py.allow_threads(|| mtk_core::extrude_mesh::process_random_extrude_mesh(&input));
    (
        out.new_positions,
        out.new_face_vertices,
        out.new_face_uvs,
        out.new_face_materials,
        out.extruded_face_indices,
        out.repaired_count,
    )
}

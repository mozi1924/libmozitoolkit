//! # `mtk-wasm` (WebAssembly and JavaScript Bindings)
//!
//! Provides WebAssembly bindings for `libmtk` via `wasm-bindgen`, exposing
//! high-performance geometry buffers directly to JavaScript/TypeScript/WebGPU runtimes.

use wasm_bindgen::prelude::*;
use js_sys::{Float32Array, Uint32Array, Uint16Array};

use mtk_core::mesh::MeshData;
use mtk_core::direction::Direction;
use mtk_core::geometry::Quad;
use mtk_core::attributes::FaceAttributes;

/// Sets panic hook for readable console error stacks in browser devtools.
#[wasm_bindgen]
pub fn init_panic_hook() {
    console_error_panic_hook::set_once();
}

/// Returns libmtk version.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// JavaScript/WASM wrapper for `MeshData`.
#[wasm_bindgen]
pub struct WasmMeshData {
    inner: MeshData,
    cached_flat_positions: Vec<f32>,
    cached_flat_normals: Vec<f32>,
    cached_flat_uvs: Vec<f32>,
}

#[wasm_bindgen]
impl WasmMeshData {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: MeshData::new(),
            cached_flat_positions: Vec::new(),
            cached_flat_normals: Vec::new(),
            cached_flat_uvs: Vec::new(),
        }
    }

    #[wasm_bindgen(getter)]
    pub fn vertex_count(&self) -> usize {
        self.inner.vertex_count()
    }

    #[wasm_bindgen(getter)]
    pub fn triangle_count(&self) -> usize {
        self.inner.triangle_count()
    }

    #[wasm_bindgen(getter)]
    pub fn face_count(&self) -> usize {
        self.inner.face_count()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn clear(&mut self) {
        self.inner.clear();
        self.cached_flat_positions.clear();
        self.cached_flat_normals.clear();
        self.cached_flat_uvs.clear();
    }

    /// Appends a unit cube face in given direction (0: Down, 1: Up, 2: North, 3: South, 4: West, 5: East).
    pub fn append_unit_cube_face(&mut self, direction_id: u8, material_slot: u16) -> Result<(), JsValue> {
        let dir = match direction_id {
            0 => Direction::Down,
            1 => Direction::Up,
            2 => Direction::North,
            3 => Direction::South,
            4 => Direction::West,
            5 => Direction::East,
            _ => return Err(JsValue::from_str("Invalid direction index")),
        };
        let quad = Quad::unit_cube_face(dir);
        let attr = FaceAttributes {
            material_slot,
            ..Default::default()
        };
        self.inner.append_quad(&quad, &attr);
        Ok(())
    }

    /// Returns a copied Float32Array of vertex positions `[x0, y0, z0, x1, y1, z1, ...]`.
    pub fn get_flat_positions(&mut self) -> Float32Array {
        let v_count = self.inner.vertex_count();
        self.cached_flat_positions.clear();
        self.cached_flat_positions.reserve(v_count * 3);
        for p in &self.inner.positions {
            self.cached_flat_positions.push(p[0]);
            self.cached_flat_positions.push(p[1]);
            self.cached_flat_positions.push(p[2]);
        }
        Float32Array::from(self.cached_flat_positions.as_slice())
    }

    /// Returns a copied Float32Array of vertex normals `[nx0, ny0, nz0, ...]`.
    pub fn get_flat_normals(&mut self) -> Float32Array {
        let v_count = self.inner.vertex_count();
        self.cached_flat_normals.clear();
        self.cached_flat_normals.reserve(v_count * 3);
        for n in &self.inner.normals {
            self.cached_flat_normals.push(n[0]);
            self.cached_flat_normals.push(n[1]);
            self.cached_flat_normals.push(n[2]);
        }
        Float32Array::from(self.cached_flat_normals.as_slice())
    }

    /// Returns a copied Float32Array of vertex UVs `[u0, v0, u1, v1, ...]`.
    pub fn get_flat_uvs(&mut self) -> Float32Array {
        let v_count = self.inner.vertex_count();
        self.cached_flat_uvs.clear();
        self.cached_flat_uvs.reserve(v_count * 2);
        for uv in &self.inner.uvs {
            self.cached_flat_uvs.push(uv[0]);
            self.cached_flat_uvs.push(uv[1]);
        }
        Float32Array::from(self.cached_flat_uvs.as_slice())
    }

    /// Returns a copied Uint32Array of triangle indices.
    pub fn get_indices(&self) -> Uint32Array {
        Uint32Array::from(self.inner.indices.as_slice())
    }

    /// Returns a copied Uint16Array of face material slots.
    pub fn get_face_materials(&self) -> Uint16Array {
        Uint16Array::from(self.inner.face_materials.as_slice())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wasm_mesh_data() {
        let mut mesh = WasmMeshData::new();
        assert_eq!(mesh.vertex_count(), 0);
        assert!(mesh.is_empty());

        let res = mesh.append_unit_cube_face(1, 10);
        assert!(res.is_ok());
        assert_eq!(mesh.vertex_count(), 4);
        assert_eq!(mesh.triangle_count(), 2);
        assert_eq!(mesh.face_count(), 1);

        mesh.clear();
        assert_eq!(mesh.vertex_count(), 0);
        assert!(mesh.is_empty());
    }

    #[test]
    fn test_wasm_version() {
        assert_eq!(version(), env!("CARGO_PKG_VERSION"));
    }
}


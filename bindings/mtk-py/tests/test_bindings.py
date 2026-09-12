import unittest
import libmtk_py


class TestLibMtkPy(unittest.TestCase):
    def test_version(self):
        v = libmtk_py.version()
        self.assertTrue(len(v) > 0)

    def test_mesh_data_buffers(self):
        positions = [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0]
        uvs = [0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0]
        indices = [0, 1, 2, 0, 2, 3]

        mesh = libmtk_py.MeshData.from_raw_buffers(positions, uvs, indices)
        self.assertEqual(mesh.vertex_count, 4)
        self.assertEqual(mesh.triangle_count, 2)
        self.assertEqual(mesh.face_count, 2)

        pos_mv = mesh.positions_memoryview()
        self.assertEqual(len(pos_mv), 4 * 3 * 4)  # 4 verts * 3 floats * 4 bytes

        uv_mv = mesh.uvs_memoryview()
        self.assertEqual(len(uv_mv), 4 * 2 * 4)

        idx_mv = mesh.indices_memoryview()
        self.assertEqual(len(idx_mv), 6 * 4)

    def test_atlas_mapping_and_process_mesh(self):
        positions = [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0]
        uvs = [0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0]
        indices = [0, 1, 2, 0, 2, 3]
        mesh = libmtk_py.MeshData.from_raw_buffers(positions, uvs, indices)

        mapping_json = """{
            "chunks": [],
            "sprites": {
                "minecraft:block/stone": {
                    "chunk_id": 0,
                    "category": "blocks",
                    "is_animated": false,
                    "sprite_kind": "static_atlas",
                    "texture_id": 1,
                    "uv_bounds": [0.0, 0.0, 0.25, 0.25],
                    "frame_0_uv_bounds": [0.0, 0.0, 0.25, 0.25],
                    "local_uv_bounds": [0.0, 0.0, 1.0, 1.0],
                    "pixel_rect": [0, 0, 16, 16],
                    "frame_size": [16, 16],
                    "frame_count": 1,
                    "has_normal": false,
                    "has_specular": false
                }
            }
        }"""
        atlas = libmtk_py.BakedAtlas.from_mapping_json(mapping_json)

        out_mesh, mat_info, stats = libmtk_py.process_mesh(
            mesh,
            ["Tile_Stone"],
            atlas=atlas,
            origin="auto",
            generate_secondary_uv=True,
        )

        self.assertEqual(out_mesh.vertex_count, 4)
        self.assertEqual(stats["resolved_slots"], 1)
        self.assertEqual(mat_info[0]["canonical_name"], "minecraft:block/stone")
        self.assertEqual(mat_info[0]["atlas_chunk_id"], 0)

        flat_uvs = out_mesh.get_flat_uvs()
        self.assertEqual(flat_uvs, [0.0, 0.0, 0.25, 0.0, 0.25, 0.25, 0.0, 0.25])

        flat_sec_uvs = out_mesh.get_flat_secondary_uvs()
        self.assertEqual(flat_sec_uvs, [0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0])


if __name__ == "__main__":
    unittest.main()

import unittest

import libmtk_py


def quad_mesh():
    return libmtk_py.MeshData.from_flat_polygons(
        [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0],
        [0, 1, 2, 3],
        [0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0],
        [0],
        [4],
        None,
        [0],
    )


class TestLibMtkPy(unittest.TestCase):
    def test_version(self):
        self.assertTrue(libmtk_py.version())
        self.assertEqual(libmtk_py.api_version(), 1)

    def test_mesh_data_buffers(self):
        mesh = quad_mesh()
        self.assertEqual(mesh.vertex_count, 4)
        self.assertEqual(mesh.triangle_count, 2)
        self.assertEqual(mesh.face_count, 1)
        self.assertEqual(mesh.quad_count, 1)

        self.assertEqual(len(mesh.positions_memoryview()), 4 * 3 * 4)
        self.assertEqual(len(mesh.uvs_memoryview()), 4 * 2 * 4)
        self.assertEqual(len(mesh.indices_memoryview()), 6 * 4)
        self.assertEqual(len(mesh.quad_indices_memoryview()), 4 * 4)

    def test_custom_attributes_memoryview_and_list(self):
        mesh = quad_mesh()
        mesh.add_string_attribute("mtk_source_texture", "face", ["minecraft:block/dirt"])
        mesh.add_attribute_from_buffer(
            "mtk_emission", "face", "float", bytes([0, 0, 128, 63])
        )

        self.assertTrue(mesh.has_attribute("mtk_source_texture"))
        self.assertTrue(mesh.has_attribute("mtk_emission"))
        self.assertEqual(
            mesh.get_string_attribute("mtk_source_texture"),
            ["minecraft:block/dirt"],
        )
        self.assertAlmostEqual(mesh.get_attribute_data("mtk_emission")[0], 1.0)

    def test_mesh_data_memoryviews(self):
        import numpy as np

        mesh = quad_mesh()
        pos = np.frombuffer(mesh.positions_memoryview(), dtype=np.float32)
        self.assertEqual(len(pos), 12)
        np.testing.assert_allclose(
            pos.reshape(-1, 3)[1], [1.0, 0.0, 0.0]
        )

        idx = np.frombuffer(mesh.indices_memoryview(), dtype=np.uint32)
        np.testing.assert_array_equal(idx, [0, 1, 2, 0, 2, 3])

        loop_uv = np.frombuffer(mesh.loop_uvs_memoryview(), dtype=np.float32)
        np.testing.assert_array_equal(
            loop_uv, [0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0]
        )
        self.assertEqual(len(mesh.loop_starts_memoryview()), 4)
        self.assertEqual(len(mesh.loop_totals_memoryview()), 4)

    def test_build_material_palette(self):
        import numpy as np

        palette, indices = libmtk_py.build_material_palette(
            ["Grass", "default"],
            [False, True],
            ["", "minecraft:block/grass_block_top"],
            np.array([1, 0, 1], dtype=np.int32),
        )
        self.assertEqual(
            palette, ["minecraft:block/grass_block_top", "Grass"]
        )
        np.testing.assert_array_equal(
            np.frombuffer(indices, dtype=np.uint32), [0, 1, 0]
        )

    def test_point_cloud_palette_indices(self):
        import numpy as np

        cloud = libmtk_py.VoxelPointCloud()
        cloud.push([0.5, 0.5, 0.5], [0, 0, 0], "minecraft:stone", "minecraft:plains", 0)
        cloud.push([1.5, 0.5, 0.5], [1, 0, 0], "minecraft:dirt", "minecraft:desert", 0)
        cloud.push([2.5, 0.5, 0.5], [2, 0, 0], "minecraft:stone", "minecraft:plains", 0)

        self.assertEqual(
            cloud.get_state_palette(), ["minecraft:stone", "minecraft:dirt"]
        )
        self.assertEqual(
            cloud.get_biome_palette(), ["minecraft:plains", "minecraft:desert"]
        )
        np.testing.assert_array_equal(
            np.frombuffer(cloud.state_indices_memoryview(), dtype=np.int32),
            [0, 1, 0],
        )
        np.testing.assert_array_equal(
            np.frombuffer(cloud.biome_indices_memoryview(), dtype=np.int32),
            [0, 1, 0],
        )

        restored = libmtk_py.VoxelPointCloud.from_mesh_data(cloud.to_mesh_data())
        self.assertEqual(restored.get_state_palette(), cloud.get_state_palette())
        self.assertEqual(restored.get_biome_palette(), cloud.get_biome_palette())

    def test_unstable_blender_dna_submodule(self):
        import numpy as np

        if not hasattr(libmtk_py, "unstable") or not hasattr(
            libmtk_py.unstable, "blender_dna"
        ):
            self.skipTest("libmtk_py built without blender feature or unstable submodule")

        dna = libmtk_py.unstable.blender_dna
        mesh = quad_mesh()

        dst_pos = np.zeros(12, dtype=np.float32)
        copied = dna.copy_positions_to_ptr(mesh, dst_pos.ctypes.data)
        self.assertEqual(copied, 12 * 4)
        np.testing.assert_allclose(
            dst_pos.reshape(-1, 3)[0], [0.0, 0.0, 0.0]
        )

        dst_uv = np.zeros(8, dtype=np.float32)
        copied = dna.copy_loop_uvs_to_ptr(mesh, dst_uv.ctypes.data)
        self.assertEqual(copied, 8 * 4)
        np.testing.assert_array_equal(
            dst_uv, [0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0]
        )


if __name__ == "__main__":
    unittest.main()

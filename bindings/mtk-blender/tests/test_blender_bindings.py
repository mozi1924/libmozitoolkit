import ctypes
import unittest
import numpy as np

try:
    import libmtk_blender
except ImportError:
    libmtk_blender = None


class TestBlenderBindings(unittest.TestCase):
    def setUp(self):
        if libmtk_blender is None:
            self.skipTest("libmtk_blender is not compiled or available in test environment")

    def test_copy_positions_to_ptr(self):
        positions = np.array([0.0, 1.0, 2.0, 3.0, 4.0, 5.0], dtype=np.float32)
        dst = np.zeros(6, dtype=np.float32)
        ptr = dst.ctypes.data
        copied = libmtk_blender.copy_positions_to_ptr(positions, ptr, max_bytes=6 * 4)
        self.assertEqual(copied, 6 * 4)
        np.testing.assert_allclose(dst, positions)

    def test_null_pointer_rejected(self):
        positions = np.array([0.0, 1.0, 2.0], dtype=np.float32)
        with self.assertRaises(ValueError):
            libmtk_blender.copy_positions_to_ptr(positions, 0)

    def test_bounds_overflow_rejected(self):
        positions = np.array([0.0, 1.0, 2.0, 3.0, 4.0, 5.0], dtype=np.float32)
        dst = np.zeros(6, dtype=np.float32)
        ptr = dst.ctypes.data
        # Attempt to copy 24 bytes into a buffer limited to 12 bytes
        with self.assertRaises(ValueError):
            libmtk_blender.copy_positions_to_ptr(positions, ptr, max_bytes=12)

    def test_copy_loop_starts_and_totals(self):
        dst_starts = np.zeros(4, dtype=np.int32)
        dst_totals = np.zeros(4, dtype=np.int32)

        libmtk_blender.copy_loop_starts_to_ptr(4, True, dst_starts.ctypes.data)
        libmtk_blender.copy_loop_totals_to_ptr(4, True, dst_totals.ctypes.data)

        # Quads stride = 4
        np.testing.assert_array_equal(dst_starts, [0, 4, 8, 12])
        np.testing.assert_array_equal(dst_totals, [4, 4, 4, 4])


if __name__ == "__main__":
    unittest.main()

use glam::{Vec2, Vec3};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::direction::Direction;

/// 2D Axis-Aligned Bounding Box (AABB) in normalized [0, 1] or pixel space.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Aabb2d {
    pub min: Vec2,
    pub max: Vec2,
}

impl Aabb2d {
    pub const ZERO: Self = Self {
        min: Vec2::ZERO,
        max: Vec2::ZERO,
    };

    pub const UNIT: Self = Self {
        min: Vec2::ZERO,
        max: Vec2::ONE,
    };

    #[inline]
    pub const fn new(min: Vec2, max: Vec2) -> Self {
        Self { min, max }
    }

    #[inline]
    pub fn from_min_max(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> Self {
        Self {
            min: Vec2::new(min_x.min(max_x), min_y.min(max_y)),
            max: Vec2::new(min_x.max(max_x), min_y.max(max_y)),
        }
    }

    #[inline]
    pub fn width(&self) -> f32 {
        (self.max.x - self.min.x).max(0.0)
    }

    #[inline]
    pub fn height(&self) -> f32 {
        (self.max.y - self.min.y).max(0.0)
    }

    #[inline]
    pub fn area(&self) -> f32 {
        self.width() * self.height()
    }

    #[inline]
    pub fn is_empty(&self, eps: f32) -> bool {
        self.width() <= eps || self.height() <= eps
    }

    #[inline]
    pub fn contains_point(&self, p: Vec2) -> bool {
        p.x >= self.min.x && p.x <= self.max.x && p.y >= self.min.y && p.y <= self.max.y
    }

    #[inline]
    pub fn contains_rect(&self, other: &Self, eps: f32) -> bool {
        self.min.x <= other.min.x + eps
            && self.max.x >= other.max.x - eps
            && self.min.y <= other.min.y + eps
            && self.max.y >= other.max.y - eps
    }

    #[inline]
    pub fn intersects(&self, other: &Self) -> bool {
        self.min.x < other.max.x
            && self.max.x > other.min.x
            && self.min.y < other.max.y
            && self.max.y > other.min.y
    }

    #[inline]
    pub fn intersection(&self, other: &Self) -> Option<Self> {
        let min_x = self.min.x.max(other.min.x);
        let min_y = self.min.y.max(other.min.y);
        let max_x = self.max.x.min(other.max.x);
        let max_y = self.max.y.min(other.max.y);

        if min_x < max_x && min_y < max_y {
            Some(Self {
                min: Vec2::new(min_x, min_y),
                max: Vec2::new(max_x, max_y),
            })
        } else {
            None
        }
    }
}

/// 3D Axis-Aligned Bounding Box (AABB) in normalized block space [0, 1] or Minecraft [0, 16] space.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Aabb3d {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb3d {
    pub const ZERO: Self = Self {
        min: Vec3::ZERO,
        max: Vec3::ZERO,
    };

    pub const UNIT: Self = Self {
        min: Vec3::ZERO,
        max: Vec3::ONE,
    };

    #[inline]
    pub const fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    #[inline]
    pub fn from_min_max(
        min_x: f32,
        min_y: f32,
        min_z: f32,
        max_x: f32,
        max_y: f32,
        max_z: f32,
    ) -> Self {
        Self {
            min: Vec3::new(min_x.min(max_x), min_y.min(max_y), min_z.min(max_z)),
            max: Vec3::new(min_x.max(max_x), min_y.max(max_y), min_z.max(max_z)),
        }
    }

    #[inline]
    pub fn size(&self) -> Vec3 {
        (self.max - self.min).max(Vec3::ZERO)
    }

    #[inline]
    pub fn volume(&self) -> f32 {
        let s = self.size();
        s.x * s.y * s.z
    }
}

/// A 3D planar Quad composed of 4 vertices and corresponding UVs.
/// Follows Minecraft standard winding order for counter-clockwise face rendering.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Quad {
    pub vertices: [Vec3; 4],
    pub uvs: [Vec2; 4],
    pub normal: Vec3,
}

impl Quad {
    /// Constructs a quad and calculates normal using cross product.
    pub fn new(vertices: [Vec3; 4], uvs: [Vec2; 4]) -> Self {
        let edge1 = vertices[1] - vertices[0];
        let edge2 = vertices[2] - vertices[0];
        let normal = edge1.cross(edge2).normalize_or_zero();
        Self {
            vertices,
            uvs,
            normal,
        }
    }

    /// Generates standard 4 vertices for a unit cube [0, 1]^3 for the given face direction.
    /// Aligns strictly with Minecraft 1.21+ FaceInfo winding convention:
    /// - Down (-Y):  (0,0,1), (0,0,0), (1,0,0), (1,0,1)
    /// - Up (+Y):    (0,1,0), (0,1,1), (1,1,1), (1,1,0)
    /// - North (-Z): (1,1,0), (1,0,0), (0,0,0), (0,1,0)
    /// - South (+Z): (0,1,1), (0,0,1), (1,0,1), (1,1,1)
    /// - West (-X):  (0,1,0), (0,0,0), (0,0,1), (0,1,1)
    /// - East (+X):  (1,1,1), (1,0,1), (1,0,0), (1,1,0)
    pub fn unit_cube_face(dir: Direction) -> Self {
        let (vertices, normal) = match dir {
            Direction::Down => (
                [
                    Vec3::new(0.0, 0.0, 1.0),
                    Vec3::new(0.0, 0.0, 0.0),
                    Vec3::new(1.0, 0.0, 0.0),
                    Vec3::new(1.0, 0.0, 1.0),
                ],
                Vec3::new(0.0, -1.0, 0.0),
            ),
            Direction::Up => (
                [
                    Vec3::new(0.0, 1.0, 0.0),
                    Vec3::new(0.0, 1.0, 1.0),
                    Vec3::new(1.0, 1.0, 1.0),
                    Vec3::new(1.0, 1.0, 0.0),
                ],
                Vec3::new(0.0, 1.0, 0.0),
            ),
            Direction::North => (
                [
                    Vec3::new(1.0, 1.0, 0.0),
                    Vec3::new(1.0, 0.0, 0.0),
                    Vec3::new(0.0, 0.0, 0.0),
                    Vec3::new(0.0, 1.0, 0.0),
                ],
                Vec3::new(0.0, 0.0, -1.0),
            ),
            Direction::South => (
                [
                    Vec3::new(0.0, 1.0, 1.0),
                    Vec3::new(0.0, 0.0, 1.0),
                    Vec3::new(1.0, 0.0, 1.0),
                    Vec3::new(1.0, 1.0, 1.0),
                ],
                Vec3::new(0.0, 0.0, 1.0),
            ),
            Direction::West => (
                [
                    Vec3::new(0.0, 1.0, 0.0),
                    Vec3::new(0.0, 0.0, 0.0),
                    Vec3::new(0.0, 0.0, 1.0),
                    Vec3::new(0.0, 1.0, 1.0),
                ],
                Vec3::new(-1.0, 0.0, 0.0),
            ),
            Direction::East => (
                [
                    Vec3::new(1.0, 1.0, 1.0),
                    Vec3::new(1.0, 0.0, 1.0),
                    Vec3::new(1.0, 0.0, 0.0),
                    Vec3::new(1.0, 1.0, 0.0),
                ],
                Vec3::new(1.0, 0.0, 0.0),
            ),
        };

        // Standard canonical UVs [0..1]
        let uvs = [
            Vec2::new(0.0, 1.0),
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 1.0),
        ];

        Self {
            vertices,
            uvs,
            normal,
        }
    }
}

/// Transforms a canonical local Minecraft voxel corner vertex to a centered Z-up coordinate space (origin at block center: [-0.5..0.5]).
/// Minecraft: +X East, +Y Up, +Z South
/// Z-Up Right-Hand: +X East, +Y North (-Z_mc), +Z Up (+Y_mc)
#[inline]
pub fn mc_local_to_centered_z_up(lx: f32, ly: f32, lz: f32) -> Vec3 {
    Vec3::new(lx - 0.5, -(lz - 0.5), ly - 0.5)
}

/// Transforms a 3D vertex from Minecraft world coordinates (+X East, +Y Up, +Z South)
/// to standard Z-up right-hand coordinates (+X East, +Y North, +Z Up).
#[inline]
pub fn mc_world_to_z_up(wx: f32, wy: f32, wz: f32) -> Vec3 {
    Vec3::new(wx, -wz, wy)
}

/// Backward compatibility alias for `mc_local_to_centered_z_up`.
#[inline]
pub fn mc_local_to_blender(lx: f32, ly: f32, lz: f32) -> Vec3 {
    mc_local_to_centered_z_up(lx, ly, lz)
}

/// Backward compatibility alias for `mc_world_to_z_up`.
#[inline]
pub fn mc_world_to_blender(wx: f32, wy: f32, wz: f32) -> Vec3 {
    mc_world_to_z_up(wx, wy, wz)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aabb2d_intersection() {
        let r1 = Aabb2d::from_min_max(0.0, 0.0, 1.0, 1.0);
        let r2 = Aabb2d::from_min_max(0.5, 0.5, 1.5, 1.5);
        let inter = r1.intersection(&r2).unwrap();
        assert_eq!(inter.min, Vec2::new(0.5, 0.5));
        assert_eq!(inter.max, Vec2::new(1.0, 1.0));
        assert_eq!(inter.area(), 0.25);
    }

    #[test]
    fn test_unit_cube_normals() {
        for dir in Direction::ALL {
            let quad = Quad::unit_cube_face(dir);
            assert!(
                quad.normal.dot(dir.normal()) > 0.99,
                "Face normal mismatch for {:?}",
                dir
            );
        }
    }

    #[test]
    fn test_mc_to_blender_transforms() {
        let b = mc_local_to_blender(0.5, 0.5, 0.5);
        assert_eq!(b, Vec3::ZERO);
        let w = mc_world_to_blender(10.0, 64.0, -20.0);
        assert_eq!(w, Vec3::new(10.0, 20.0, 64.0));
    }
}

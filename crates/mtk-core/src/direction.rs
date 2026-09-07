use bitflags::bitflags;
use glam::{IVec3, Vec3};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Standard 6 cardinal directions in Minecraft 3D coordinate space.
///
/// Coordinate conventions:
/// - East:  +X (Index 0)
/// - West:  -X (Index 1)
/// - Up:    +Y (Index 2)
/// - Down:  -Y (Index 3)
/// - South: +Z (Index 4)
/// - North: -Z (Index 5)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
#[repr(u8)]
pub enum Direction {
    East = 0,
    West = 1,
    Up = 2,
    Down = 3,
    South = 4,
    North = 5,
}

impl Direction {
    /// Array containing all 6 cardinal directions in canonical index order.
    pub const ALL: [Direction; 6] = [
        Direction::East,
        Direction::West,
        Direction::Up,
        Direction::Down,
        Direction::South,
        Direction::North,
    ];

    /// Returns the integer index corresponding to the direction (0 to 5).
    #[inline]
    pub const fn to_index(self) -> usize {
        self as usize
    }

    /// Converts an integer index (0..6) back into a Direction.
    #[inline]
    pub const fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(Direction::East),
            1 => Some(Direction::West),
            2 => Some(Direction::Up),
            3 => Some(Direction::Down),
            4 => Some(Direction::South),
            5 => Some(Direction::North),
            _ => None,
        }
    }

    /// Returns the opposite direction.
    #[inline]
    pub const fn opposite(self) -> Self {
        match self {
            Direction::East => Direction::West,
            Direction::West => Direction::East,
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
            Direction::South => Direction::North,
            Direction::North => Direction::South,
        }
    }

    /// Returns the unit normal vector as float Vec3.
    #[inline]
    pub const fn normal(self) -> Vec3 {
        match self {
            Direction::East => Vec3::new(1.0, 0.0, 0.0),
            Direction::West => Vec3::new(-1.0, 0.0, 0.0),
            Direction::Up => Vec3::new(0.0, 1.0, 0.0),
            Direction::Down => Vec3::new(0.0, -1.0, 0.0),
            Direction::South => Vec3::new(0.0, 0.0, 1.0),
            Direction::North => Vec3::new(0.0, 0.0, -1.0),
        }
    }

    /// Returns the integer step offset IVec3.
    #[inline]
    pub const fn offset(self) -> IVec3 {
        match self {
            Direction::East => IVec3::new(1, 0, 0),
            Direction::West => IVec3::new(-1, 0, 0),
            Direction::Up => IVec3::new(0, 1, 0),
            Direction::Down => IVec3::new(0, -1, 0),
            Direction::South => IVec3::new(0, 0, 1),
            Direction::North => IVec3::new(0, 0, -1),
        }
    }

    /// Returns the single bitmask for this direction.
    #[inline]
    pub const fn mask(self) -> DirMask {
        match self {
            Direction::East => DirMask::EAST,
            Direction::West => DirMask::WEST,
            Direction::Up => DirMask::UP,
            Direction::Down => DirMask::DOWN,
            Direction::South => DirMask::SOUTH,
            Direction::North => DirMask::NORTH,
        }
    }

    /// Parses string representation (e.g. "east", "East", "+x", "+X").
    pub fn parse_loose(s: &str) -> Option<Self> {
        let trimmed = s.trim();
        if trimmed.eq_ignore_ascii_case("east") || trimmed.eq_ignore_ascii_case("+x") {
            Some(Direction::East)
        } else if trimmed.eq_ignore_ascii_case("west") || trimmed.eq_ignore_ascii_case("-x") {
            Some(Direction::West)
        } else if trimmed.eq_ignore_ascii_case("up") || trimmed.eq_ignore_ascii_case("+y") {
            Some(Direction::Up)
        } else if trimmed.eq_ignore_ascii_case("down") || trimmed.eq_ignore_ascii_case("-y") {
            Some(Direction::Down)
        } else if trimmed.eq_ignore_ascii_case("south") || trimmed.eq_ignore_ascii_case("+z") {
            Some(Direction::South)
        } else if trimmed.eq_ignore_ascii_case("north") || trimmed.eq_ignore_ascii_case("-z") {
            Some(Direction::North)
        } else {
            None
        }
    }

    /// Canonical lowercase name string ("east", "west", etc.).
    #[inline]
    pub const fn as_str(self) -> &'static str {
        match self {
            Direction::East => "east",
            Direction::West => "west",
            Direction::Up => "up",
            Direction::Down => "down",
            Direction::South => "south",
            Direction::North => "north",
        }
    }
}

bitflags! {
    /// 6-bit direction mask set.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct DirMask: u8 {
        const EAST  = 1 << 0;
        const WEST  = 1 << 1;
        const UP    = 1 << 2;
        const DOWN  = 1 << 3;
        const SOUTH = 1 << 4;
        const NORTH = 1 << 5;

        const ALL = Self::EAST.bits() | Self::WEST.bits() | Self::UP.bits()
                  | Self::DOWN.bits() | Self::SOUTH.bits() | Self::NORTH.bits();
    }
}

impl DirMask {
    /// Checks if a given direction is set in this mask.
    #[inline]
    pub const fn contains_dir(self, dir: Direction) -> bool {
        (self.bits() & dir.mask().bits()) != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_directions_basic() {
        for (i, dir) in Direction::ALL.iter().enumerate() {
            assert_eq!(dir.to_index(), i);
            assert_eq!(Direction::from_index(i), Some(*dir));
            assert_eq!(dir.opposite().opposite(), *dir);
        }
    }

    #[test]
    fn test_direction_masks() {
        let mut mask = DirMask::empty();
        mask |= Direction::East.mask();
        mask |= Direction::Up.mask();

        assert!(mask.contains_dir(Direction::East));
        assert!(mask.contains_dir(Direction::Up));
        assert!(!mask.contains_dir(Direction::Down));
        assert!(!mask.contains_dir(Direction::West));
    }

    #[test]
    fn test_parse_loose() {
        assert_eq!(Direction::parse_loose("+x"), Some(Direction::East));
        assert_eq!(Direction::parse_loose("Down"), Some(Direction::Down));
        assert_eq!(Direction::parse_loose("-Z"), Some(Direction::North));
        assert_eq!(Direction::parse_loose("invalid"), None);
    }
}

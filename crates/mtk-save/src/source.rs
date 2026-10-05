use std::collections::HashMap;
use std::path::{Path, PathBuf};

use glam::IVec3;
use mtk_voxel::source::VoxelSource;
use mtk_voxel::storage::SectionStorage;
use mtk_voxel::types::VoxelError;

use crate::chunk::ChunkParser;
use crate::region::RegionFile;

/// Abstract voxel source streaming chunk sections directly from Anvil `.mca` region files.
pub struct AnvilWorldSource {
    region_dir: PathBuf,
    open_regions: HashMap<(i32, i32), RegionFile>,
    bounds: Option<(IVec3, IVec3)>,
}

impl AnvilWorldSource {
    /// Creates a new `AnvilWorldSource` pointing to a region folder.
    pub fn new(region_dir: &Path) -> Self {
        Self {
            region_dir: region_dir.to_path_buf(),
            open_regions: HashMap::new(),
            bounds: None,
        }
    }

    /// Sets explicit bounding box in section coordinates.
    pub fn with_bounds(mut self, min_sec: IVec3, max_sec: IVec3) -> Self {
        self.bounds = Some((min_sec, max_sec));
        self
    }

    /// Helper to get or open a region file for region coordinates `(rx, rz)`.
    fn get_or_open_region(&mut self, rx: i32, rz: i32) -> Option<&mut RegionFile> {
        if !self.open_regions.contains_key(&(rx, rz)) {
            let mca_path = self.region_dir.join(format!("r.{}.{}.mca", rx, rz));
            if mca_path.exists() {
                if let Ok(reg) = RegionFile::open(&mca_path) {
                    self.open_regions.insert((rx, rz), reg);
                } else {
                    return None;
                }
            } else {
                return None;
            }
        }
        self.open_regions.get_mut(&(rx, rz))
    }
}

impl VoxelSource for AnvilWorldSource {
    fn source_name(&self) -> &str {
        "Minecraft-Anvil-MCA"
    }

    fn has_section(&self, sx: i32, _sy: i32, sz: i32) -> bool {
        let rx = sx >> 5;
        let rz = sz >> 5;
        let mca_path = self.region_dir.join(format!("r.{}.{}.mca", rx, rz));
        if !mca_path.exists() {
            return false;
        }

        // Fast check if chunk exists in location table
        if let Ok(reg) = RegionFile::open(&mca_path) {
            reg.has_chunk(sx, sz)
        } else {
            false
        }
    }

    fn load_section(
        &mut self,
        sx: i32,
        sy: i32,
        sz: i32,
    ) -> Result<Option<SectionStorage>, VoxelError> {
        let rx = sx >> 5;
        let rz = sz >> 5;

        let region = match self.get_or_open_region(rx, rz) {
            Some(r) => r,
            None => return Ok(None),
        };

        let decompressed = match region.read_chunk_decompressed(sx, sz) {
            Ok(Some(bytes)) => bytes,
            Ok(None) => return Ok(None),
            Err(e) => return Err(VoxelError::MalformedSnapshot(e.to_string())),
        };

        let sections = ChunkParser::parse_chunk_sections(&decompressed, sy, sy)
            .map_err(|e| VoxelError::MalformedSnapshot(e.to_string()))?;

        for sec in sections {
            if sec.coord.y == sy {
                return Ok(Some(sec));
            }
        }

        Ok(None)
    }

    fn section_bounds(&self) -> Option<(IVec3, IVec3)> {
        self.bounds
    }
}

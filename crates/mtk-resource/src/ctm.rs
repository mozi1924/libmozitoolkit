use std::collections::HashMap;
use mtk_core::direction::DirMask;
use crate::identifier::{DEFAULT_NAMESPACE, ResourceLocation};

/// Symmetry modes for random CTM.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CtmSymmetry {
    #[default]
    None,
    Opposite,
    All,
}

impl CtmSymmetry {
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "opposite" => CtmSymmetry::Opposite,
            "all" => CtmSymmetry::All,
            _ => CtmSymmetry::None,
        }
    }
}

/// The CTM connection algorithm method.
#[derive(Debug, Clone, PartialEq)]
pub enum CtmMethod {
    /// 47-tile full connected textures
    Full { inner_seams: bool },
    /// 5-tile compact connected textures
    Compact { inner_seams: bool },
    /// 4-tile horizontal connection (e.g. bookshelves, logs)
    Horizontal,
    /// 4-tile vertical connection (e.g. pillars, pillars)
    Vertical,
    /// Grid horizontal + vertical connection
    HorizontalVertical,
    /// Grid vertical + horizontal connection
    VerticalHorizontal,
    /// Top/bottom capping connection
    Top,
    /// NxM large texture repeat pattern
    Repeat { width: u32, height: u32 },
    /// Weighted pseudo-random variant selection
    Random {
        weights: Vec<f32>,
        total_weight: f32,
        symmetry: CtmSymmetry,
        linked: bool,
    },
    /// Direct fixed texture replacement
    Fixed,
    /// Overlay alpha-transition edge blending
    Overlay,
}

/// Connection logic determining which neighboring blocks connect.
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectLogic {
    SameBlock,
    SameTile,
    SameState,
    BlockNames(Vec<String>),
    Textures(Vec<ResourceLocation>),
}

/// A block pattern match with optional BlockState property constraints (e.g. `grass_block:snowy=false`).
#[derive(Debug, Clone, PartialEq)]
pub struct BlockMatch {
    pub block: String,
    pub properties: HashMap<String, Vec<String>>,
}

impl BlockMatch {
    pub fn parse(token: &str) -> Self {
        let parts: Vec<&str> = token.split(':').collect();
        let mut block_ns = DEFAULT_NAMESPACE;
        let mut block_name = parts[0];
        let mut prop_start = 1;

        if parts.len() > 1 && !parts[1].contains('=') {
            block_ns = parts[0];
            block_name = parts[1];
            prop_start = 2;
        }

        let canonical_name = format!("{}:{}", block_ns, block_name);
        let mut properties = HashMap::new();

        for &prop_token in &parts[prop_start..] {
            if let Some((k, v)) = prop_token.split_once('=') {
                let values: Vec<String> = v.split(',').map(|s| s.trim().to_string()).collect();
                properties.insert(k.trim().to_string(), values);
            }
        }

        Self {
            block: canonical_name,
            properties,
        }
    }
}

/// Fully parsed OptiFine / Continuity CTM rule definition.
#[derive(Debug, Clone, PartialEq)]
pub struct CtmRule {
    pub name: String,
    pub source_path: String,
    pub priority: i32,
    pub method: CtmMethod,
    /// Ordered list of referenced sprite tiles. None represents `<skip>` or `<default>`.
    pub tiles: Vec<Option<ResourceLocation>>,
    pub match_blocks: Vec<BlockMatch>,
    pub match_tiles: Vec<ResourceLocation>,
    pub connect_logic: ConnectLogic,
    pub faces: DirMask,
    pub biomes: Option<Vec<String>>,
    pub height_ranges: Option<Vec<(i32, i32)>>,
    pub tint_index: Option<i32>,
    pub tint_block: Option<String>,
}

impl CtmRule {
    /// Parse an OptiFine / Continuity `.properties` file into a `CtmRule`.
    pub fn parse_properties(source_path: &str, namespace: &str, content: &str) -> Option<Self> {
        let mut props: HashMap<String, String> = HashMap::new();

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
                continue;
            }
            if let Some((k, v)) = trimmed.split_once('=') {
                props.insert(k.trim().to_lowercase(), v.trim().to_string());
            }
        }

        let parent_dir = if let Some(idx) = source_path.rfind('/') {
            &source_path[..idx + 1]
        } else {
            ""
        };

        // 1. Method
        let method_str = props.get("method").map(|s| s.as_str()).unwrap_or("ctm");
        let inner_seams = props.get("innerseams").map(|s| s.eq_ignore_ascii_case("true")).unwrap_or(false);

        let method = match method_str {
            "ctm" | "overlay_ctm" => CtmMethod::Full { inner_seams },
            "ctm_compact" => CtmMethod::Compact { inner_seams },
            "horizontal" => CtmMethod::Horizontal,
            "vertical" => CtmMethod::Vertical,
            "horizontal+vertical" => CtmMethod::HorizontalVertical,
            "vertical+horizontal" => CtmMethod::VerticalHorizontal,
            "top" => CtmMethod::Top,
            "repeat" | "overlay_repeat" => {
                let width = props.get("width").and_then(|w| w.parse().ok()).unwrap_or(2);
                let height = props.get("height").and_then(|h| h.parse().ok()).unwrap_or(2);
                CtmMethod::Repeat { width, height }
            }
            "random" | "overlay_random" => {
                let symmetry = props.get("symmetry").map(|s| CtmSymmetry::parse(s)).unwrap_or_default();
                let linked = props.get("linked").map(|s| s.eq_ignore_ascii_case("true")).unwrap_or(false);
                let mut weights = Vec::new();
                let mut total_weight = 0.0f32;
                if let Some(w_str) = props.get("weights") {
                    for token in w_str.split_whitespace() {
                        if let Ok(val) = token.parse::<f32>() {
                            weights.push(val);
                            total_weight += val;
                        }
                    }
                }
                CtmMethod::Random {
                    weights,
                    total_weight,
                    symmetry,
                    linked,
                }
            }
            "fixed" | "overlay_fixed" => CtmMethod::Fixed,
            "overlay" => CtmMethod::Overlay,
            _ => CtmMethod::Full { inner_seams: false },
        };

        // 2. Tiles
        let mut tiles: Vec<Option<ResourceLocation>> = Vec::new();
        if let Some(tiles_str) = props.get("tiles") {
            for token in tiles_str.split_whitespace() {
                if token.eq_ignore_ascii_case("<skip>") || token.eq_ignore_ascii_case("<default>") {
                    tiles.push(None);
                } else if token.contains('-') {
                    // Range, e.g. 0-46
                    if let Some((start_s, end_s)) = token.split_once('-') {
                        if let (Ok(start), Ok(end)) = (start_s.parse::<u32>(), end_s.parse::<u32>()) {
                            for i in start..=end {
                                let tile_path = format!("{}{}", parent_dir, i);
                                tiles.push(Some(ResourceLocation::new(namespace, tile_path)));
                            }
                        }
                    }
                } else if token.contains('/') {
                    let clean = token.strip_prefix("assets/").unwrap_or(token);
                    tiles.push(Some(ResourceLocation::parse(clean).unwrap_or_else(|_| ResourceLocation::new(namespace, clean))));
                } else {
                    let clean_token = token.strip_suffix(".png").unwrap_or(token);
                    let tile_path = format!("{}{}", parent_dir, clean_token);
                    tiles.push(Some(ResourceLocation::new(namespace, tile_path)));
                }
            }
        }

        // 3. Match Blocks / Match Tiles
        let mut match_blocks = Vec::new();
        if let Some(mb) = props.get("matchblocks") {
            for token in mb.split_whitespace() {
                match_blocks.push(BlockMatch::parse(token));
            }
        }

        let mut match_tiles = Vec::new();
        if let Some(mt) = props.get("matchtiles") {
            for token in mt.split_whitespace() {
                let clean = token.strip_suffix(".png").unwrap_or(token);
                if clean.contains(':') {
                    if let Ok(loc) = ResourceLocation::parse(clean) {
                        match_tiles.push(loc);
                    }
                } else {
                    match_tiles.push(ResourceLocation::new(namespace, format!("block/{}", clean)));
                }
            }
        }

        // Default match from filename if neither specified
        let file_stem = source_path.rsplit('/').next().unwrap_or(source_path).strip_suffix(".properties").unwrap_or(source_path);
        if match_blocks.is_empty() && match_tiles.is_empty() {
            if file_stem.starts_with("block_") {
                match_blocks.push(BlockMatch::parse(&file_stem["block_".len()..]));
            } else {
                match_tiles.push(ResourceLocation::new(namespace, format!("block/{}", file_stem)));
            }
        }

        // 4. Connect Logic
        let connect_logic = if let Some(ct) = props.get("connecttiles") {
            let list = ct.split_whitespace().map(|s| {
                let clean = s.strip_suffix(".png").unwrap_or(s);
                ResourceLocation::parse(clean).unwrap_or_else(|_| ResourceLocation::new(namespace, format!("block/{}", clean)))
            }).collect();
            ConnectLogic::Textures(list)
        } else if let Some(cb) = props.get("connectblocks") {
            let list = cb.split_whitespace().map(|s| {
                if s.contains(':') { s.to_string() } else { format!("minecraft:{}", s) }
            }).collect();
            ConnectLogic::BlockNames(list)
        } else if let Some(conn) = props.get("connect") {
            match conn.as_str() {
                "tile" => ConnectLogic::SameTile,
                "state" => ConnectLogic::SameState,
                _ => ConnectLogic::SameBlock,
            }
        } else if !match_tiles.is_empty() {
            ConnectLogic::SameTile
        } else {
            ConnectLogic::SameBlock
        };

        // 5. Faces
        let mut faces = DirMask::all();
        if let Some(faces_str) = props.get("faces") {
            let mut custom_faces = DirMask::empty();
            for f in faces_str.split_whitespace() {
                match f.to_lowercase().as_str() {
                    "all" => custom_faces |= DirMask::all(),
                    "sides" => custom_faces |= DirMask::NORTH | DirMask::SOUTH | DirMask::EAST | DirMask::WEST,
                    "north" => custom_faces |= DirMask::NORTH,
                    "south" => custom_faces |= DirMask::SOUTH,
                    "east" => custom_faces |= DirMask::EAST,
                    "west" => custom_faces |= DirMask::WEST,
                    "up" | "top" => custom_faces |= DirMask::UP,
                    "down" | "bottom" => custom_faces |= DirMask::DOWN,
                    _ => {}
                }
            }
            if !custom_faces.is_empty() {
                faces = custom_faces;
            }
        }

        // 6. Weight / Priority
        let priority = props.get("weight").and_then(|w| w.parse().ok()).unwrap_or(0);

        // 7. Biomes & Heights
        let biomes = props.get("biomes").map(|b| {
            b.split_whitespace().map(|s| {
                if s.contains(':') { s.to_string() } else { format!("minecraft:{}", s) }
            }).collect()
        });

        let height_ranges = props.get("heights").map(|h| {
            let mut ranges = Vec::new();
            for token in h.split_whitespace() {
                if let Some((min_s, max_s)) = token.split_once('-') {
                    if let (Ok(min), Ok(max)) = (min_s.parse(), max_s.parse()) {
                        ranges.push((min, max));
                    }
                } else if let Ok(val) = token.parse() {
                    ranges.push((val, val));
                }
            }
            ranges
        });

        let tint_index = props.get("tintindex").and_then(|t| t.parse().ok());
        let tint_block = props.get("tintblock").cloned();

        Some(Self {
            name: file_stem.to_string(),
            source_path: source_path.to_string(),
            priority,
            method,
            tiles,
            match_blocks,
            match_tiles,
            connect_logic,
            faces,
            biomes,
            height_ranges,
            tint_index,
            tint_block,
        })
    }
}

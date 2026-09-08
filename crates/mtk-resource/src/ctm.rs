use std::collections::HashMap;
use glam::IVec3;
use mtk_core::direction::{Direction, DirMask};
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

    /// Checks if this rule applies to the given block state, face, world coordinates, and biome.
    pub fn matches_block(
        &self,
        state: &str,
        face: Direction,
        world_pos: IVec3,
        biome: Option<&str>,
    ) -> bool {
        if !self.faces.contains_dir(face) {
            return false;
        }
        if let Some(ref heights) = self.height_ranges {
            let y = world_pos.y;
            if !heights.iter().any(|&(min, max)| y >= min && y <= max) {
                return false;
            }
        }
        if let Some(ref biomes) = self.biomes {
            if let Some(b) = biome {
                let canonical_b = if b.contains(':') {
                    b.to_string()
                } else {
                    format!("{}:{}", DEFAULT_NAMESPACE, b)
                };
                if !biomes.contains(&canonical_b) {
                    return false;
                }
            }
        }
        if !self.match_blocks.is_empty() {
            if !self.match_blocks.iter().any(|m| m.matches(state)) {
                return false;
            }
        }
        true
    }

    /// Solves the final CTM sub-tile ResourceLocation for this face.
    pub fn solve_tile<F, S>(
        &self,
        state: &str,
        face: Direction,
        world_pos: IVec3,
        get_block: &F,
    ) -> Option<ResourceLocation>
    where
        F: Fn(IVec3) -> Option<S>,
        S: AsRef<str>,
    {
        let check_connect = |offset: IVec3| -> bool {
            let n_pos = world_pos + offset;
            if let Some(neighbor_state) = get_block(n_pos) {
                let n_str = neighbor_state.as_ref();
                match &self.connect_logic {
                    ConnectLogic::SameBlock => {
                        extract_block_name(state) == extract_block_name(n_str)
                    }
                    ConnectLogic::SameState => state == n_str,
                    ConnectLogic::BlockNames(names) => {
                        let n_name = extract_block_name(n_str);
                        let canonical = if n_name.contains(':') {
                            n_name.to_string()
                        } else {
                            format!("{}:{}", DEFAULT_NAMESPACE, n_name)
                        };
                        names.contains(&canonical)
                    }
                    ConnectLogic::SameTile | ConnectLogic::Textures(_) => {
                        extract_block_name(state) == extract_block_name(n_str)
                    }
                }
            } else {
                false
            }
        };

        match &self.method {
            CtmMethod::Full { inner_seams } => {
                let (up, left) = get_face_tangents(face);
                let down = up.opposite();
                let right = left.opposite();
                let forward = face;

                let mut bits = 0u8;
                if check_connect(up.offset() + left.offset()) {
                    bits |= 1 << 7;
                }
                if check_connect(up.offset()) {
                    bits |= 1 << 6;
                }
                if check_connect(up.offset() + right.offset()) {
                    bits |= 1 << 5;
                }
                if check_connect(right.offset()) {
                    bits |= 1 << 4;
                }
                if check_connect(down.offset() + right.offset()) {
                    bits |= 1 << 3;
                }
                if check_connect(down.offset()) {
                    bits |= 1 << 2;
                }
                if check_connect(down.offset() + left.offset()) {
                    bits |= 1 << 1;
                }
                if check_connect(left.offset()) {
                    bits |= 1;
                }

                if !inner_seams {
                    if (bits & (1 << 7)) == 0
                        && check_connect(up.offset() + left.offset() + forward.offset())
                    {
                        bits |= 1 << 7;
                    }
                    if (bits & (1 << 6)) == 0 && check_connect(up.offset() + forward.offset()) {
                        bits |= 1 << 6;
                    }
                    if (bits & (1 << 5)) == 0
                        && check_connect(up.offset() + right.offset() + forward.offset())
                    {
                        bits |= 1 << 5;
                    }
                    if (bits & (1 << 4)) == 0 && check_connect(right.offset() + forward.offset()) {
                        bits |= 1 << 4;
                    }
                    if (bits & (1 << 3)) == 0
                        && check_connect(down.offset() + right.offset() + forward.offset())
                    {
                        bits |= 1 << 3;
                    }
                    if (bits & (1 << 2)) == 0 && check_connect(down.offset() + forward.offset()) {
                        bits |= 1 << 2;
                    }
                    if (bits & (1 << 1)) == 0
                        && check_connect(down.offset() + left.offset() + forward.offset())
                    {
                        bits |= 1 << 1;
                    }
                    if (bits & 1) == 0 && check_connect(left.offset() + forward.offset()) {
                        bits |= 1;
                    }
                }

                let tile_idx = CTM_47_LOOKUP[bits as usize] as usize;
                self.tiles.get(tile_idx).and_then(|t| t.clone())
            }
            CtmMethod::Compact { .. } => {
                let (up, left) = get_face_tangents(face);
                let down = up.opposite();
                let right = left.opposite();

                let up_c = check_connect(up.offset());
                let down_c = check_connect(down.offset());
                let left_c = check_connect(left.offset());
                let right_c = check_connect(right.offset());

                let tile_idx = match (up_c || down_c, left_c || right_c) {
                    (true, true) => 4,
                    (true, false) => 2,
                    (false, true) => 3,
                    (false, false) => 0,
                };
                self.tiles.get(tile_idx).and_then(|t| t.clone())
            }
            CtmMethod::Horizontal => {
                let (_up, left) = get_face_tangents(face);
                let right = left.opposite();
                let left_c = check_connect(left.offset());
                let right_c = check_connect(right.offset());

                let tile_idx = match (left_c, right_c) {
                    (true, true) => 1,
                    (true, false) => 2,
                    (false, true) => 0,
                    (false, false) => 3,
                };
                self.tiles.get(tile_idx).and_then(|t| t.clone())
            }
            CtmMethod::Vertical => {
                let (up, _left) = get_face_tangents(face);
                let down = up.opposite();
                let up_c = check_connect(up.offset());
                let down_c = check_connect(down.offset());

                let tile_idx = match (up_c, down_c) {
                    (true, true) => 1,
                    (true, false) => 0,
                    (false, true) => 2,
                    (false, false) => 3,
                };
                self.tiles.get(tile_idx).and_then(|t| t.clone())
            }
            CtmMethod::HorizontalVertical => {
                let (up, left) = get_face_tangents(face);
                let down = up.opposite();
                let right = left.opposite();

                let left_c = check_connect(left.offset());
                let right_c = check_connect(right.offset());
                let mut tile_idx = match (left_c, right_c) {
                    (true, true) => 1,
                    (true, false) => 2,
                    (false, true) => 0,
                    (false, false) => 3,
                };
                if tile_idx == 3 {
                    let up_c = check_connect(up.offset());
                    let down_c = check_connect(down.offset());
                    tile_idx = match (up_c, down_c) {
                        (true, true) => 5,
                        (true, false) => 4,
                        (false, true) => 6,
                        (false, false) => 3,
                    };
                }
                self.tiles.get(tile_idx).and_then(|t| t.clone())
            }
            CtmMethod::VerticalHorizontal => {
                let (up, left) = get_face_tangents(face);
                let down = up.opposite();
                let right = left.opposite();

                let up_c = check_connect(up.offset());
                let down_c = check_connect(down.offset());
                let mut tile_idx = match (up_c, down_c) {
                    (true, true) => 1,
                    (true, false) => 0,
                    (false, true) => 2,
                    (false, false) => 3,
                };
                if tile_idx == 3 {
                    let left_c = check_connect(left.offset());
                    let right_c = check_connect(right.offset());
                    tile_idx = match (left_c, right_c) {
                        (true, true) => 5,
                        (true, false) => 6,
                        (false, true) => 4,
                        (false, false) => 3,
                    };
                }
                self.tiles.get(tile_idx).and_then(|t| t.clone())
            }
            CtmMethod::Top => {
                let (up, _left) = get_face_tangents(face);
                if check_connect(up.offset()) {
                    self.tiles.first().and_then(|t| t.clone())
                } else {
                    None
                }
            }
            CtmMethod::Repeat { width, height } => {
                if *width == 0 || *height == 0 || self.tiles.is_empty() {
                    return None;
                }
                let (up, left) = get_face_tangents(face);
                let u = match left {
                    Direction::East | Direction::West => world_pos.x,
                    Direction::Up | Direction::Down => world_pos.y,
                    Direction::North | Direction::South => world_pos.z,
                }
                .rem_euclid(*width as i32) as u32;

                let v = match up {
                    Direction::East | Direction::West => world_pos.x,
                    Direction::Up | Direction::Down => world_pos.y,
                    Direction::North | Direction::South => world_pos.z,
                }
                .rem_euclid(*height as i32) as u32;

                let tile_idx = (v * width + u) as usize;
                self.tiles.get(tile_idx).and_then(|t| t.clone())
            }
            CtmMethod::Random {
                weights,
                total_weight,
                symmetry,
                linked,
            } => {
                if self.tiles.is_empty() {
                    return None;
                }
                let (rx, ry, rz) = match symmetry {
                    CtmSymmetry::None => (
                        face.offset().x * 26,
                        face.offset().y * 26,
                        face.offset().z * 26,
                    ),
                    CtmSymmetry::Opposite => {
                        let rand_dir = match face {
                            Direction::South => Direction::North,
                            Direction::West => Direction::East,
                            Direction::Down => Direction::Up,
                            other => other,
                        };
                        (
                            rand_dir.offset().x * 26,
                            rand_dir.offset().y * 26,
                            rand_dir.offset().z * 26,
                        )
                    }
                    CtmSymmetry::All => (0, 0, 0),
                };
                let mut qy = world_pos.y + ry;
                if *linked {
                    if let Some(below_state) = get_block(world_pos + IVec3::new(0, -1, 0)) {
                        if extract_block_name(state) == extract_block_name(below_state.as_ref()) {
                            qy -= 1;
                        }
                    }
                }
                let rand_val = coordinate_random(world_pos.x + rx, qy, world_pos.z + rz);
                if weights.is_empty() || *total_weight <= 0.0 {
                    let idx = ((rand_val * (self.tiles.len() as f32)) as usize)
                        .min(self.tiles.len() - 1);
                    return self.tiles.get(idx).and_then(|t| t.clone());
                }
                let target = rand_val * total_weight;
                let mut accum = 0.0f32;
                let mut chosen_idx = 0;
                for (i, &w) in weights.iter().enumerate().take(self.tiles.len()) {
                    accum += w;
                    if target < accum {
                        chosen_idx = i;
                        break;
                    }
                }
                self.tiles.get(chosen_idx).and_then(|t| t.clone())
            }
            CtmMethod::Fixed => self.tiles.first().and_then(|t| t.clone()),
            CtmMethod::Overlay => {
                let (up, left) = get_face_tangents(face);
                let down = up.opposite();
                let right = left.opposite();

                let mut bits = 0u8;
                if check_connect(up.offset() + left.offset()) {
                    bits |= 1 << 7;
                }
                if check_connect(up.offset()) {
                    bits |= 1 << 6;
                }
                if check_connect(up.offset() + right.offset()) {
                    bits |= 1 << 5;
                }
                if check_connect(right.offset()) {
                    bits |= 1 << 4;
                }
                if check_connect(down.offset() + right.offset()) {
                    bits |= 1 << 3;
                }
                if check_connect(down.offset()) {
                    bits |= 1 << 2;
                }
                if check_connect(down.offset() + left.offset()) {
                    bits |= 1 << 1;
                }
                if check_connect(left.offset()) {
                    bits |= 1;
                }

                let tile_idx = OVERLAY_17_LOOKUP[bits as usize];
                if tile_idx >= 0 {
                    self.tiles.get(tile_idx as usize).and_then(|t| t.clone())
                } else {
                    None
                }
            }
        }
    }
}

impl BlockMatch {
    /// Tests if a block state string (e.g. `minecraft:grass_block[snowy=false]`) satisfies this match condition.
    pub fn matches(&self, state: &str) -> bool {
        let block_name = extract_block_name(state);
        let canonical_name = if block_name.contains(':') {
            block_name.to_string()
        } else {
            format!("{}:{}", DEFAULT_NAMESPACE, block_name)
        };
        if self.block != canonical_name {
            return false;
        }
        if self.properties.is_empty() {
            return true;
        }
        if let Some(prop_start) = state.find('[') {
            let prop_str = state[prop_start + 1..]
                .strip_suffix(']')
                .unwrap_or(&state[prop_start + 1..]);
            for part in prop_str.split(',') {
                if let Some((k, v)) = part.split_once('=') {
                    let k = k.trim();
                    let v = v.trim();
                    if let Some(allowed_vals) = self.properties.get(k) {
                        if !allowed_vals.iter().any(|val| val == v) {
                            return false;
                        }
                    }
                }
            }
        }
        true
    }
}

/// Helper to extract the block identifier before any property brackets `[...]`.
#[inline]
pub fn extract_block_name(state: &str) -> &str {
    if let Some(idx) = state.find('[') {
        &state[..idx]
    } else {
        state
    }
}

/// Computes the tangent `(up, left)` directions for a standard cardinal cube face.
#[inline]
pub const fn get_face_tangents(face: Direction) -> (Direction, Direction) {
    match face {
        Direction::North => (Direction::Up, Direction::East),
        Direction::South => (Direction::Up, Direction::West),
        Direction::East => (Direction::Up, Direction::South),
        Direction::West => (Direction::Up, Direction::North),
        Direction::Up => (Direction::North, Direction::West),
        Direction::Down => (Direction::South, Direction::West),
    }
}

/// Deterministic 3D spatial pseudorandom number generator matching standard Minecraft / OptiFine.
#[inline]
pub fn coordinate_random(x: i32, y: i32, z: i32) -> f32 {
    let mut l = (x as i64)
        .wrapping_mul(3129871)
        ^ (z as i64).wrapping_mul(116129781)
        ^ (y as i64);
    l = l
        .wrapping_mul(l)
        .wrapping_mul(42317861)
        .wrapping_add(l.wrapping_mul(11));
    let hash = (l >> 16) as u32;
    (hash & 0x00ff_ffff) as f32 / 16777216.0
}

/// 47 Full CTM tile index to connection bit pattern.
pub const TILE_TO_CONNECTION_DATA: [u8; 47] = [
    0b00000000, 0b00010000, 0b00010001, 0b00000001, 0b00010100, 0b00000101, 0b01010100, 0b00010101,
    0b01110101, 0b01011101, 0b11010111, 0b11110101, 0b00000100, 0b00011100, 0b00011111, 0b00000111,
    0b01010000, 0b01000001, 0b01010001, 0b01000101, 0b11010101, 0b01010111, 0b01011111, 0b01111101,
    0b01000100, 0b01111100, 0b11111111, 0b11000111, 0b01011100, 0b00010111, 0b01110100, 0b00011101,
    0b11110111, 0b11111101, 0b01110111, 0b11011101, 0b01000000, 0b01110000, 0b11110001, 0b11000001,
    0b01110001, 0b11000101, 0b11010001, 0b01000111, 0b11011111, 0b01111111, 0b01010101,
];

/// Precomputed 256-entry lookup table for Full 47-tile CTM.
pub const fn build_ctm_47_lookup() -> [u8; 256] {
    let mut table = [0u8; 256];
    let mut mapped = [false; 256];

    let mut i = 0;
    while i < 47 {
        let pattern = TILE_TO_CONNECTION_DATA[i] as usize;
        table[pattern] = i as u8;
        mapped[pattern] = true;
        i += 1;
    }

    let mut pattern = 0;
    while pattern < 256 {
        if !mapped[pattern] {
            let mut tile_idx = pattern;
            let mut corner_bit = 1;
            while corner_bit < 8 {
                let left_side_bit = if corner_bit == 0 { 7 } else { corner_bit - 1 };
                let right_side_bit = if corner_bit + 1 >= 8 { 0 } else { corner_bit + 1 };

                let left_side = tile_idx & (1 << left_side_bit);
                let right_side = tile_idx & (1 << right_side_bit);

                if left_side == 0 || right_side == 0 {
                    tile_idx &= !(1 << corner_bit);
                }
                corner_bit += 2;
            }

            table[pattern] = table[tile_idx];
        }
        pattern += 1;
    }

    table
}

pub const CTM_47_LOOKUP: [u8; 256] = build_ctm_47_lookup();

/// 17 Overlay tile index to connection bit pattern.
pub const TILE_TO_OVERLAY_DATA: [u8; 17] = [
    0b00001000, 0b00001110, 0b00000010, 0b00111110, 0b10001111, 0b10111111, 0b11101111,
    0b00111000, 0b11111111, 0b10000011, 0b11111000, 0b11100011, 0b11111110, 0b11111011,
    0b00100000, 0b11100000, 0b10000000,
];

/// Precomputed 256-entry lookup table for Overlay CTM.
pub const fn build_overlay_17_lookup() -> [i8; 256] {
    let mut table = [-2i8; 256];
    table[0b00000000] = -1;

    let mut i = 0;
    while i < 17 {
        table[TILE_TO_OVERLAY_DATA[i] as usize] = i as i8;
        i += 1;
    }

    table[0b11101110] = 1;
    table[0b10111011] = 7;

    let mut pattern = 0;
    while pattern < 256 {
        if table[pattern] < -1 {
            let mut tile_idx = pattern;
            let mut corner_bit = 1;
            while corner_bit < 8 {
                let left_side_bit = if corner_bit == 0 { 7 } else { corner_bit - 1 };
                let right_side_bit = if corner_bit + 1 >= 8 { 0 } else { corner_bit + 1 };

                let left_side = tile_idx & (1 << left_side_bit);
                let right_side = tile_idx & (1 << right_side_bit);

                if left_side > 0 || right_side > 0 {
                    tile_idx |= 1 << corner_bit;
                }
                if left_side == 0 && right_side == 0 {
                    tile_idx &= !(1 << corner_bit);
                }
                corner_bit += 2;
            }
            table[pattern] = table[tile_idx];
        }
        pattern += 1;
    }

    table
}

pub const OVERLAY_17_LOOKUP: [i8; 256] = build_overlay_17_lookup();

/// High-performance CTM Rule Solver indexed for fast per-face resolution during chunk meshing.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CtmSolver {
    pub rules: Vec<CtmRule>,
    pub rules_by_block: HashMap<String, Vec<usize>>,
    pub rules_by_tile: HashMap<ResourceLocation, Vec<usize>>,
}

impl CtmSolver {
    /// Creates a new `CtmSolver` from an array of loaded `CtmRule`s, sorting by priority.
    pub fn new(mut rules: Vec<CtmRule>) -> Self {
        rules.sort_by(|a, b| b.priority.cmp(&a.priority));

        let mut rules_by_block: HashMap<String, Vec<usize>> = HashMap::new();
        let mut rules_by_tile: HashMap<ResourceLocation, Vec<usize>> = HashMap::new();

        for (idx, rule) in rules.iter().enumerate() {
            for mb in &rule.match_blocks {
                rules_by_block
                    .entry(mb.block.clone())
                    .or_default()
                    .push(idx);
            }
            for mt in &rule.match_tiles {
                rules_by_tile.entry(mt.clone()).or_default().push(idx);
            }
        }

        Self {
            rules,
            rules_by_block,
            rules_by_tile,
        }
    }

    /// Resolves the final CTM tile for a block face given its state, orientation, position, and neighbor query closure.
    pub fn resolve_face<F, S>(
        &self,
        state: &str,
        face: Direction,
        world_pos: IVec3,
        base_tile: Option<&ResourceLocation>,
        biome: Option<&str>,
        get_block: F,
    ) -> Option<ResourceLocation>
    where
        F: Fn(IVec3) -> Option<S>,
        S: AsRef<str>,
    {
        if self.rules.is_empty() {
            return None;
        }

        let block_name = extract_block_name(state);
        let canonical_name = if block_name.contains(':') {
            block_name.to_string()
        } else {
            format!("{}:{}", DEFAULT_NAMESPACE, block_name)
        };

        // 1. Try match by block
        if let Some(rule_indices) = self.rules_by_block.get(&canonical_name) {
            for &idx in rule_indices {
                let rule = &self.rules[idx];
                if rule.matches_block(state, face, world_pos, biome) {
                    if let Some(tile) = rule.solve_tile(state, face, world_pos, &get_block) {
                        return Some(tile);
                    }
                }
            }
        }

        // 2. Try match by base tile
        if let Some(tile_loc) = base_tile {
            if let Some(rule_indices) = self.rules_by_tile.get(tile_loc) {
                for &idx in rule_indices {
                    let rule = &self.rules[idx];
                    if rule.matches_block(state, face, world_pos, biome) {
                        if let Some(tile) = rule.solve_tile(state, face, world_pos, &get_block) {
                            return Some(tile);
                        }
                    }
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ctm_47_lookup_table() {
        assert_eq!(CTM_47_LOOKUP[0b00000000], 0);
        assert_eq!(CTM_47_LOOKUP[0b11111111], 26);
        assert_eq!(CTM_47_LOOKUP[0b01010101], 46);
        // Corner degeneration: top-left corner isolated without top or left -> should map to 0
        assert_eq!(CTM_47_LOOKUP[0b10000000], 0);
    }

    #[test]
    fn test_ctm_horizontal_and_vertical() {
        let content_h = r#"
matchBlocks=minecraft:bookshelf
method=horizontal
tiles=0 1 2 3
"#;
        let rule_h = CtmRule::parse_properties("optifine/ctm/bookshelf.properties", "minecraft", content_h).unwrap();
        assert_eq!(rule_h.method, CtmMethod::Horizontal);

        let solver = CtmSolver::new(vec![rule_h]);

        // Left and right are connected -> tile 1
        let res = solver.resolve_face(
            "minecraft:bookshelf",
            Direction::North,
            IVec3::new(0, 64, 0),
            None,
            None,
            |pos| {
                if pos.x == 1 || pos.x == -1 {
                    Some("minecraft:bookshelf")
                } else {
                    Some("minecraft:air")
                }
            },
        );
        assert_eq!(res, Some(ResourceLocation::new("minecraft", "optifine/ctm/1")));
    }

    #[test]
    fn test_ctm_repeat_method() {
        let content_repeat = r#"
matchBlocks=minecraft:sandstone
method=repeat
width=2
height=2
tiles=0 1 2 3
"#;
        let rule_repeat = CtmRule::parse_properties("optifine/ctm/sandstone.properties", "minecraft", content_repeat).unwrap();
        let solver = CtmSolver::new(vec![rule_repeat]);

        let res0 = solver.resolve_face("minecraft:sandstone", Direction::North, IVec3::new(0, 0, 0), None, None, |_| None::<&str>);
        assert_eq!(res0, Some(ResourceLocation::new("minecraft", "optifine/ctm/0")));

        let res1 = solver.resolve_face("minecraft:sandstone", Direction::North, IVec3::new(1, 0, 0), None, None, |_| None::<&str>);
        assert_eq!(res1, Some(ResourceLocation::new("minecraft", "optifine/ctm/1")));

        let res2 = solver.resolve_face("minecraft:sandstone", Direction::North, IVec3::new(0, 1, 0), None, None, |_| None::<&str>);
        assert_eq!(res2, Some(ResourceLocation::new("minecraft", "optifine/ctm/2")));
    }
}


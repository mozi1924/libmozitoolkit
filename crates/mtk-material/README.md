# mtk-material

High-performance, host-agnostic material name resolution, external alias mapping, grid-based atlas unscrambling, canonical 66-biome palettes & color math (SSOT), and multi-threaded Rayon UV re-addressing to libmtk AtlasAddressMaps.

---

## 1. 架构定位与职责边界 (Architecture & Responsibilities)

`mtk-material` 是 `libmozitoolkit` 体系中材质解析、UV 空间重映射与生物群系色彩着色的**权威事实源 (Single Source of Truth, SSOT)**，具备以下核心职责与边界：

- **66 原版生物群系调色板唯一权威源 (Biome SSOT)**：内置 66 种 Minecraft 官方生物群系参数表（草方块、树叶、干枯植被、水体颜色、温度、湿度及 Colormap 三角形采样坐标），统一管理 sRGB 与 Linear RGBA 线性色彩转换数学。
- **方块分层染色注册表与分类器**：精准支持原版方块染色规则（如 `pink_petals` / `wildflowers` 的花瓣不染色与茎叶染色分层、`dead_bush` / `azalea_leaves` 永不染色保护、硬编码固定色方块如云杉/白桦树叶）。
- **多导出器材质与网格图集解混淆**：
  - 数据驱动支持 Mineways 等网格图集 (`GridAtlasSpec`，18x18 包含 1px 保护边框像素解码)；
  - 自动识别并纠正非正交旋转 UV（如 jmc2obj 的 45 度流动液体菱形 UV `straighten_diamond_quad_uv`）；
  - 自动归一化超限平铺 UV 并提取仿射变换矩阵（适配 `MC_Atlas_UV_Tiling` 着色器节点）。
- **Rayon 并行多 UV 批量重映射**：同时输出图集 UV (`atlas_uvs`) 与局部标准 [0, 1] UV (`local_uvs`)，输出逐面 Chunk ID、Texture ID、UV 模式与 Overlay 标记。

---

## 2. 核心数据结构与枚举 (Core Structures & Enums)

### 2.1 生物群系与调色板 (`biome/palettes.rs` & `hardcoded.rs`)
```rust
pub static CANONICAL_BIOMES: &[BiomePalette];

#[derive(Debug, Clone, PartialEq)]
pub struct BiomePalette {
    pub id: &'static str,
    pub name: &'static str,
    pub grass_hex: &'static str,
    pub foliage_hex: &'static str,
    pub dry_foliage_hex: &'static str,
    pub water_hex: &'static str,
    pub temperature: f32,
    pub humidity: f32,
    pub has_custom_grass: bool,
    pub has_custom_foliage: bool,
    pub has_custom_dry_foliage: bool,
}
```
- `pal.grass_linear() -> [f32; 4]`: 返回 Linear RGBA 线性草方块颜色。
- `pal.foliage_linear() -> [f32; 4]`: 返回 Linear RGBA 树叶颜色。
- `pal.dry_foliage_linear() -> [f32; 4]`: 返回 Linear RGBA 干枯植被颜色。
- `pal.water_linear() -> [f32; 4]`: 返回 Linear RGBA 水体颜色。
- `pal.colormap_uv() -> [f32; 2]`: 返回原版 256x256 三角形 Colormap 采样坐标 `[1.0 - t, h * t]`。

**染色分类常量**：
- `TINT_TYPE_NONE = 0`: 不染色
- `TINT_TYPE_GRASS = 1`: 草本染色（取 grass colormap 或调色板）
- `TINT_TYPE_FOLIAGE = 2`: 树叶染色（取 foliage colormap 或调色板）
- `TINT_TYPE_WATER = 3`: 水体染色
- `TINT_TYPE_HARDCODED = 4`: 硬编码固定颜色（云杉树叶、睡莲、红石线等）
- `TINT_TYPE_DRY_FOLIAGE = 5`: 苍白/干枯植被染色

---

### 2.2 生物群系解析器与网格面属性生成 (`biome/resolver/` & `batch.rs`)
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TintInfo {
    pub tint_type: u8,
    pub tint_category: String,
    pub tint_weight: f32,
    pub base_tint_weight: f32,
    pub overlay_tint_weight: f32,
    pub has_overlay: bool,
    pub overlay_texture: Option<String>,
    pub is_hardcoded: bool,
    pub hardcoded_color: Option<[f32; 4]>,
    pub hardcoded_hex: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MeshBiomeAttributesResult {
    /// mtk_biome_tint_data: (base_tint_weight, overlay_tint_weight, tint_weight, tint_type)
    pub packed_tint_data: Vec<[f32; 4]>,
    /// mtk_biome_tint_color: (r, g, b, a) Linear RGBA
    pub tint_colors: Vec<[f32; 4]>,
    /// mtk_colormap_uv: (u, v, 0.0)
    pub colormap_uvs: Vec<[f32; 3]>,
}
```

---

### 2.3 材质解析器与网格图集规范 (`resolver/` & `types.rs`)
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GridAtlasSpec {
    pub swatch_size: f32,
    pub tile_size: f32,
    pub border: f32,
    pub image_width: u32,
    pub image_height: u32,
    pub atlas_name_patterns: Vec<String>,
    pub atlas_suffix_patterns: Vec<String>,
    pub swatch_to_candidates: HashMap<usize, Vec<String>>,
}

pub struct MaterialResolver;
```

---

### 2.4 多通道 UV 批量重映射结果 (`types.rs`)
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeshMultiUvRemapResult {
    pub face_count: usize,
    pub loop_count: usize,
    pub atlas_uvs: Vec<[f32; 2]>,
    pub local_uvs: Vec<[f32; 2]>,
    pub face_chunk_ids: Vec<u16>,
    pub face_texture_ids: Vec<u32>,
    pub face_uv_transforms: Vec<[f32; 4]>,
    pub face_uv_rotations: Vec<f32>,
    pub face_uv_modes: Vec<u8>,
    pub face_is_overlay: Vec<bool>,
    pub unmapped_faces: usize,
}
```

---

## 3. 核心公共 API 清单 (Public APIs)

### 3.1 色彩数学与生物群系工具
| 函数签名 | 描述 |
| :--- | :--- |
| `srgb_to_linear(c: f32) -> f32` | 标准 sRGB 标量分量转线性空间。 |
| `linear_to_srgb(c: f32) -> f32` | 线性空间标量分量转 sRGB。 |
| `hex_to_linear_rgba(hex: &str) -> [f32; 4]` | 十六进制色彩字符串（如 `"#91BD59"`）转 Linear RGBA。 |
| `get_colormap_uv(temp: f32, hum: f32) -> [f32; 2]` | 计算原版三角形 Colormap 采样 UV。 |
| `get_biome_palette(name: &str) -> &'static BiomePalette` | 获取权威生物群系调色板（默认回退 `plains`）。 |
| `classify_tint_category(clean_stem, block_name, tint_index) -> &'static str` | 权威判定方块/贴图染色语义类别。 |
| `get_redstone_wire_color(power: u8) -> [f32; 4]` | 获取红石引线对应信号强度（0..15）的权威 Linear RGBA 线性颜色。 |
| `get_redstone_wire_srgb(power: u8) -> [f32; 4]` | 获取红石引线对应信号强度（0..15）的标准 sRGB 浮点色彩。 |
| `get_redstone_wire_hex(power: u8) -> &'static str` | 获取红石引线对应信号强度（0..15）的十六进制颜色代码。 |
| `blend_biome_colors(weights: &[(&str, f32)], category: &str) -> [f32; 4]` | 多生物群系加权线性混合色彩。 |
| `compute_mesh_biome_attributes(texture_keys, biome, multi_biomes, resolver)` | Rayon 并行计算全网格染色面属性数组。 |

### 3.2 材质与 UV 重映射
| 函数签名 | 描述 |
| :--- | :--- |
| `remap_local_to_atlas(u, v, sprite) -> [f32; 2]` | 将 [0, 1] 局部 UV 映射到图集 Sprite 的 Frame 0 区域。 |
| `remap_atlas_to_local(u, v, sprite) -> [f32; 2]` | 将图集 UV 逆变换还原为 [0, 1] 局部 UV。 |
| `is_quad_uv_diamond(uvs: &[[f32; 2]; 4]) -> bool` | 检测是否为 45 度旋转倾斜的流动液体 UV。 |
| `straighten_diamond_quad_uv(uvs: &mut [[f32; 2]; 4]) -> bool` | 将 45 度菱形 UV 扶正为标准正交四边形。 |
| `MaterialResolver::resolve(mat_name, aliases, address_map)` | 多级匹配解析材质名称到 `AtlasSpriteLocation`。 |
| `remap_mesh_multi_uvs_parallel(...) -> MeshMultiUvRemapResult` | 批量并行生成图集 UV、局部 UV 与着色器路由模式。 |

---

## 4. 快速上手示例 (Quick Start)

### 示例 1：查询权威生物群系调色板与色彩空间转换
```rust
use mtk_material::biome::{get_biome_palette, hex_to_linear_rgba, srgb_to_linear};

fn main() {
    // 1. 查询樱花树林 (Cherry Grove) 调色板
    let cherry = get_biome_palette("cherry_grove");
    println!("Biome: {}", cherry.name);
    println!("Grass Hex: {}", cherry.grass_hex);
    println!("Grass Linear RGBA: {:?}", cherry.grass_linear());
    println!("Colormap UV: {:?}", cherry.colormap_uv());

    // 2. 自定义十六进制色彩转 Linear RGBA (用于 Blender 着色器注入)
    let linear_color = hex_to_linear_rgba("#80A755");
    println!("Spruce leaves linear color: {:?}", linear_color);
}
```

### 示例 2：Rayon 并行计算网格面生物群系着色属性
```rust
use mtk_material::biome::{compute_mesh_biome_attributes, BiomeResolver};

fn main() {
    let resolver = BiomeResolver::new();

    // 模拟来自网格各面的材质/纹理标识符
    let face_texture_keys = vec![
        "minecraft:block/grass_block_top".to_string(),
        "minecraft:block/oak_leaves".to_string(),
        "minecraft:block/stone".to_string(),
        "minecraft:block/dead_bush".to_string(),
    ];

    // 计算平原生物群系下的面属性
    let attrs = compute_mesh_biome_attributes(
        &face_texture_keys,
        "plains",
        None,
        &resolver,
    );

    for i in 0..face_texture_keys.len() {
        println!(
            "Face {}: Tint Data: {:?}, Color: {:?}",
            face_texture_keys[i],
            attrs.packed_tint_data[i],
            attrs.tint_colors[i],
        );
    }
}
```

### 示例 3：全网格多通道 UV 并行重映射 (Atlas UV + Local UV)
```rust
use mtk_material::remap_mesh_multi_uvs_parallel;
use mtk_texture::AtlasAddressMap;

fn main() {
    let address_map = AtlasAddressMap::new(); // 从 atlas_mapping.json 加载...

    let mut source_uvs = vec![
        [0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0], // Face 0 (Quad)
    ];
    let face_materials = vec!["block/oak_planks".to_string()];
    let face_loop_ranges = vec![(0u32, 4u32)];

    let result = remap_mesh_multi_uvs_parallel(
        &source_uvs,
        &face_materials,
        &face_loop_ranges,
        &address_map,
        None,
        None,
    );

    println!("Remapped {} faces ({} loops)", result.face_count, result.loop_count);
    println!("Atlas UVs: {:?}", result.atlas_uvs);
    println!("Local UVs: {:?}", result.local_uvs);
}
```

### 示例 4：查询红石引线信号强度动态硬编码染色
```rust
use mtk_material::biome::hardcoded::{get_redstone_wire_color, get_redstone_wire_hex};
use mtk_material::BiomeResolver;

fn main() {
    // 1. 直接查询 0 与 15 级红石线性色彩
    println!("Off state hex: {}", get_redstone_wire_hex(0));  // -> "#4B0000"
    println!("Max signal hex: {}", get_redstone_wire_hex(15)); // -> "#FF2600"
    println!("Max signal Linear: {:?}", get_redstone_wire_color(15));

    // 2. 通过 BiomeResolver 动态解算方块状态中的信号等级
    let resolver = BiomeResolver::new();
    let tint = resolver.get_tint_info(
        "redstone_dust_line0",
        Some("minecraft:redstone_wire[power=15]"),
        Some(0),
    );
    assert_eq!(tint.tint_type, 4); // TINT_TYPE_HARDCODED
    assert_eq!(tint.hardcoded_hex.as_deref(), Some("#FF2600"));
}
```


# mtk-model

Headless Minecraft BlockState parser, 1.21+ Block Model JSON hierarchy baking engine, BakedModelDatabase, and Wavefront OBJ loader for libmtk.

---

## 1. 架构定位与职责边界 (Architecture & Responsibilities)

`mtk-model` 是 `libmozitoolkit` 体系中方块状态机 (BlockState) 解析、原版 1.21+ 模型 JSON 展开与几何面烘焙的核心引擎，具备以下核心职责与边界：

- **BlockState 状态机与变体解算**：解析带属性方块标识符（如 `minecraft:oak_stairs[facing=east,half=bottom,shape=straight]`），支持原版 `variants` 规则匹配与 `multipart` 组合条件树评估（`OR`, `AND`, 属性正则匹配）。
- **递归模型继承树展开 (Hierarchy Resolution)**：支持多达 32 层的父模型继承（`parent`），合并子父级纹理字典，精准展开 `#texture` 变量引用并将纹理绑定写入三维要素 (Element)。
- **微观几何烘焙与 6 向快速分桶**：
  - 将方块要素变换为局部三维空间四边形 (`BakedFace`)，精准维护 UV 旋转、UVLock 纹理锁定算法与法线朝向；
  - 自动将几何面划分至 6 向邻域剔除桶 (`culled_faces: [Vec<BakedFace>; 6]`) 与免剔除桶 (`unculled_faces`)，支撑体素网格化热循环极速发射。
- **红石引线状态机与连接解算 (Redstone Wire State & Geometry)**：
  - 内置原版 1.21+ `redstone_wire.json` Multipart 组合定义与全部变体模型（dot, side0/1, side_alt0/1, up 等）；
  - `normalize_redstone_wire_properties` 支持单臂直连自动拉直（如只有 north 则自动补齐 south=side 形成通线）、点状与交叉别名规范化；
  - `resolve_redstone_wire_connections` 自动针对 3D 体素邻域解算平地邻接、下行斜坡与上行贴墙导线；
  - 针对贴墙引线实现空间坐标感知的单面薄片剔除（North/South/East/West/Up/Down 贴墙面保留室内朝向，剔除贴墙背面）；
  - 动态计算红石信号发光等级 `get_block_emissive_level` (`power / 15.0`)。
- **叠面消重与接触面剔除**：直接调用 `mtk-cull::MeshSanitizer` 在模型要素粒度消除同向重合面（根除 DCC 视口 Z-fighting）与内部反向贴合接触面。
- **多级智能模型数据库 (`BakedModelDatabase`)**：
  - **Tier 1 (Exact Match)**: 极速精确哈希查询；
  - **Tier 1.5 (Canonical)**: 规范化键查询（消除属性书写顺序与空格差异）；
  - **Tier 2 (Canonical Filter)**: 剥离世界运行时非几何属性（如 `waterlogged`, `occupied`, `distance`, `stage`, `power`）进行几何查询；
  - **Tier 3 (Subset Match)**: 核心几何变体属性子集模糊降级匹配；
  - **Tier 4 (Base ID Fallback)**: 回退至基础方块默认模型。
- **Wavefront OBJ 模组模型加载**：支持 Mod 与第三方导出工具的 Wavefront OBJ 模型解析与面材质属性提取。

---

## 2. 核心数据结构与枚举 (Core Structures & Enums)

### 2.1 BlockState 状态表示 (`parser/blockstate/state.rs`)
```rust
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BlockState {
    pub namespace: String,
    pub name: String,
    pub properties: BTreeMap<String, String>,
}
```
- `BlockState::parse(s: &str) -> Result<Self, ModelError>`: 解析方块状态字符串。
- `state.block_id() -> String`: 返回 `namespace:name`（无属性）。
- `state.to_canonical_string() -> String`: 返回属性确定性升序排列的标准字符串。
- `state.matches_properties(req_props) -> bool`: 检查是否包含指定的属性子集。

---

### 2.2 状态机定义与变体匹配 (`parser/blockstate/definition.rs`)
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VariantMatch {
    pub model_id: String,
    pub rot_x: f32,
    pub rot_y: f32,
    pub uvlock: bool,
    pub weight: u32,
    pub variant_props: Option<BTreeMap<String, String>>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BlockStateDefinition {
    pub variants: Option<HashMap<String, VariantEntry>>,
    pub multipart: Option<Vec<MultipartRule>>,
}
```
- `def.enumerate_all_states(base_id: &str) -> Vec<String>`: 枚举该定义下所有合法的规范状态字符串。
- `BlockStateResolver::resolve(def, state) -> Vec<VariantMatch>`: 评估匹配当前方块状态的所有激活模型变体。

---

### 2.3 原版 Model JSON 结构 (`parser/model_json.rs`)
```rust
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BlockModelJson {
    pub parent: Option<String>,
    pub ambientocclusion: Option<bool>,
    pub textures: Option<HashMap<String, TextureValue>>,
    pub elements: Option<Vec<ElementJson>>,
}
```
- `model.resolve_hierarchy(model_id, parent_loader) -> Result<ResolvedBlockModel, ModelError>`: 递归解析父模型并展开所有 `#var` 纹理变量。

---

### 2.4 烘焙几何结构体 (`baker/baked_model/`)
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BakedFace {
    pub direction: Direction,
    pub texture: String,
    pub uv_rot: f32,
    pub uv_bounds: [f32; 4],
    pub tint_index: i16,
    pub cullface: Option<Direction>,
    pub vertices: [Vec3; 4],
    pub uvs: [Vec2; 4],
    pub normal: Vec3,
    pub atlas_uvs: Option<[Vec2; 4]>,
    pub atlas_chunk_id: Option<u16>,
    pub atlas_texture_id: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BakedModel {
    pub block_state: String,
    pub elements: Vec<BakedElement>,
    pub obj_faces: Vec<BakedObjFace>,
    pub faces: [BakedFace; 6],
    pub is_cube: bool,
    pub is_opaque: bool,
    pub is_emissive: bool,
    pub emissive_level: f32,
    pub cull_meta: Option<mtk_cull::BlockCullMeta>,
    pub culled_faces: [Vec<BakedFace>; 6],
    pub unculled_faces: Vec<BakedFace>,
}
```
- `baked.deduplicate_faces() -> usize`: 消除自身内部同向叠面与反向接触面，并重构 6 向分桶。
- `baked.to_mesh_with_options(opts: &ModelMeshOptions) -> MeshData`: 带叠面消除与隐藏体裁剪导出为 `mtk_core::MeshData`。
- `face.remap_to_atlas_bounds(bounds, chunk_id, texture_id)`: 离线注入图集 UV 坐标。

---

### 2.5 烘焙模型数据库 (`baker/baked_model/database.rs`)
```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BakedModelDatabase {
    pub models: HashMap<String, BakedModel>,
}
```
- `db.get(state: &str) -> Option<&BakedModel>`: 多级智能回退寻址查询。
- `db.deduplicate_all() -> usize`: 批量对库中所有模型执行面消重。
- `db.remap_to_atlas_with(lookup_fn)`: 离线对库中所有模型注入图集坐标。

---

## 3. 核心公共 API 清单 (Public APIs)

### 3.1 模型烘焙与状态机工具
| API 签名 | 描述 |
| :--- | :--- |
| `BlockState::parse(s: &str) -> Result<BlockState, ModelError>` | 解析方块状态字符串。 |
| `BlockStateResolver::resolve(def, state) -> Vec<VariantMatch>` | 评估匹配方块状态对应的模型变体。 |
| `normalize_redstone_wire_properties(props)` | 规范化红石引线方向连接、单臂拉直与别名。 |
| `resolve_redstone_wire_connections(pos, connectable_fn)` | 依据 3D 体素邻域自动计算红石引线四向连接状态（none/side/up）。 |
| `map_legacy_redstone_name(name) -> Option<(&'static str, BTreeMap)>` | 将 Mineways/Jmc2Obj 材质名映射为红石引线规范状态。 |
| `get_builtin_blockstate_def(name) -> Option<BlockStateDefinition>` | 获取内置原版 BlockState 组合定义（含 `redstone_wire`）。 |
| `get_builtin_model_by_id(model_id) -> Option<BlockModelJson>` | 获取内置原版模型 JSON（含红石引线全套模型）。 |
| `ModelBaker::new() -> Self` | 创建通用模型烘焙器。 |
| `baker.bake_blockstate(state_str, def, model_loader) -> Result<BakedModel, ModelError>` | 端到端烘焙指定方块状态为 `BakedModel`。 |
| `is_block_emissive(state: &BlockState) -> bool` | 判断方块是否为自发光方块。 |
| `get_block_emissive_level(state: &BlockState) -> f32` | 计算发光方块与红石引线（`power / 15.0`）的发光强度等级。 |
| `WavefrontObjParser::parse_str(text, filter) -> Vec<ObjRawFace>` | 解析通用 Wavefront OBJ 文本。 |

---

## 4. 快速上手示例 (Quick Start)

### 示例 1：解析 BlockState 字符串并规范化
```rust
use mtk_model::BlockState;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let state = BlockState::parse("minecraft:oak_stairs[shape=straight,facing=east,half=bottom]")?;
    assert_eq!(state.namespace, "minecraft");
    assert_eq!(state.name, "oak_stairs");
    assert_eq!(state.properties.get("facing").unwrap(), "east");

    // 输出确定性升序规范键
    println!("Canonical string: {}", state.to_canonical_string());
    // -> "minecraft:oak_stairs[facing=east,half=bottom,shape=straight]"

    Ok(())
}
```

### 示例 2：使用 `ModelBaker` 烘焙楼梯方块几何模型
```rust
use mtk_model::{BlockModelJson, BlockStateDefinition, ModelBaker};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut baker = ModelBaker::new();

    let blockstate_json = r#"{
        "variants": {
            "facing=east,half=bottom,shape=straight": {
                "model": "minecraft:block/oak_stairs",
                "y": 90,
                "uvlock": true
            }
        }
    }"#;

    let model_json = r#"{
        "textures": {
            "bottom": "minecraft:block/oak_planks",
            "top": "minecraft:block/oak_planks",
            "side": "minecraft:block/oak_planks"
        },
        "elements": [
            {
                "from": [0, 0, 0],
                "to": [16, 8, 16],
                "faces": {
                    "down":  { "texture": "#bottom", "cullface": "down" },
                    "up":    { "texture": "#top" },
                    "north": { "texture": "#side", "cullface": "north" },
                    "south": { "texture": "#side", "cullface": "south" },
                    "west":  { "texture": "#side", "cullface": "west" },
                    "east":  { "texture": "#side", "cullface": "east" }
                }
            },
            {
                "from": [8, 8, 0],
                "to": [16, 16, 16],
                "faces": {
                    "down":  { "texture": "#bottom" },
                    "up":    { "texture": "#top", "cullface": "up" },
                    "north": { "texture": "#side", "cullface": "north" },
                    "south": { "texture": "#side", "cullface": "south" },
                    "west":  { "texture": "#side" },
                    "east":  { "texture": "#side", "cullface": "east" }
                }
            }
        ]
    }"#;

    let def: BlockStateDefinition = serde_json::from_str(blockstate_json)?;
    let model: BlockModelJson = serde_json::from_str(model_json)?;

    let baked = baker.bake_blockstate(
        "minecraft:oak_stairs[facing=east,half=bottom,shape=straight]",
        Some(&def),
        |_model_id| Some(model.clone()),
    )?;

    println!("Baked model elements: {}", baked.elements.len());
    println!("Culled faces count: {:?}", baked.culled_faces.iter().map(|v| v.len()).collect::<Vec<_>>());
    println!("Unculled faces count: {}", baked.unculled_faces.len());

    Ok(())
}
```

### 示例 3：构建 `BakedModelDatabase` 并利用多级回退智能寻址
```rust
use mtk_model::{BakedModel, BakedModelDatabase};

fn main() {
    let mut db = BakedModelDatabase::new();

    let mut model = BakedModel::default();
    model.block_state = "minecraft:chest[facing=north]".to_string();
    db.insert("minecraft:chest[facing=north]".to_string(), model);

    // 1. 精确查询
    assert!(db.get("minecraft:chest[facing=north]").is_some());

    // 2. 剥离运行时非几何属性 waterlogged=true 查询成功
    assert!(db.get("minecraft:chest[facing=north,waterlogged=true]").is_some());

    // 3. 批量消重
    let removed = db.deduplicate_all();
    println!("Removed {} overlapping faces from database", removed);
}
```

### 示例 4：烘焙内置红石引线并提取发光与染色属性
```rust
use mtk_model::baker::ModelBaker;
use mtk_core::attributes::constants::{ATTR_EMISSION, ATTR_BIOME_TINT_COLOR};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut baker = ModelBaker::new();

    // 烘焙带信号强度的红石引线（自动使用内置 1.21+ fallback 模型与状态机）
    let baked = baker.bake_blockstate(
        "minecraft:redstone_wire[power=15,north=up,south=side]",
        None,
        |_| None,
    )?;

    assert!(baked.is_emissive);
    assert_eq!(baked.emissive_level, 1.0);

    let (mesh, _) = baked.to_mesh_with_textures(false);
    assert!(mesh.has_custom_attribute(ATTR_EMISSION));
    assert!(mesh.has_custom_attribute(ATTR_BIOME_TINT_COLOR));

    Ok(())
}
```


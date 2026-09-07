pub mod baked;
pub mod baker;
pub mod blockstate;
pub mod cull_volume;
pub mod error;
pub mod math;
pub mod model_json;
pub mod obj;

pub use baked::{BakedElement, BakedFace, BakedModel};
pub use baker::{is_block_emissive, ModelBaker};
pub use blockstate::{
    BlockState, BlockStateDefinition, BlockStateResolver, MultipartCondition, MultipartRule,
    VariantEntry, VariantMatch, VariantModel,
};
pub use cull_volume::{clip_face_excluding_hidden_volume, ClippedQuadPiece};
pub use error::ModelError;
pub use math::{
    apply_uvlock_to_uvs, bake_face_exact, calculate_facing, default_face_uv,
    get_face_canonical_vertex, get_face_uvlock_transform, recalculate_winding, rotate_direction,
    rotate_element_point, rotate_point, BakedFaceGeometry,
};
pub use model_json::{
    BlockModelJson, ElementJson, FaceJson, ResolvedBlockModel, ResolvedElement, ResolvedFace,
    RotationJson, TextureValue,
};
pub use obj::{BakedObjFace, ModObjLoader, ObjRawFace, WavefrontObjParser};

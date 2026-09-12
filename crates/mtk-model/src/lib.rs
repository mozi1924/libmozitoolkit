pub mod baker;
pub mod builtin;
pub mod culling;
pub mod error;
pub mod parser;

// Top-level re-exports
pub use baker::{
    apply_uvlock_to_uvs, bake_face_exact, calculate_facing, default_face_uv,
    get_face_canonical_vertex, get_face_uvlock_transform, is_block_emissive, recalculate_winding,
    rotate_direction, rotate_element_point, rotate_point, BakedElement, BakedFace,
    BakedFaceGeometry, BakedModel, BakedModelDatabase, ModelBaker,
};
pub use culling::{clip_face_excluding_hidden_volume, ClippedQuadPiece};
pub use error::ModelError;
pub use parser::{
    mesh_to_obj_string, BakedObjFace, BlockModelJson, BlockState, BlockStateDefinition,
    BlockStateResolver, ElementJson, FaceJson, ModObjLoader, MultipartCondition, MultipartRule,
    ObjRawFace, ResolvedBlockModel, ResolvedElement, ResolvedFace, RotationJson, TextureValue,
    VariantEntry, VariantMatch, VariantModel, WavefrontObjParser,
};

// Backward-compatibility module aliases
pub mod baked {
    pub use crate::baker::baked_model::*;
}
pub mod blockstate {
    pub use crate::parser::blockstate::*;
}
pub mod cull_volume {
    pub use crate::culling::cull_volume::*;
}
pub mod math {
    pub use crate::baker::math::*;
}
pub mod model_json {
    pub use crate::parser::model_json::*;
}
pub mod obj {
    pub use crate::parser::obj::*;
}

// BSD 3-Clause License
//
// Copyright (c) 2024, Bram Stout Productions
// Upstream: https://github.com/BramStoutProductions/MiEx
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions are met:
//
// 1. Redistributions of source code must retain the above copyright notice, this
//    list of conditions and the following disclaimer.
//
// 2. Redistributions in binary form must reproduce the above copyright notice,
//    this list of conditions and the following disclaimer in the documentation
//    and/or other materials provided with the distribution.
//
// 3. Neither the name of the copyright holder nor the names of its
//    contributors may be used to endorse or promote products derived from
//    this software without specific prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
// AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
// IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
// DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
// FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
// DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
// SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
// CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
// OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
// OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

pub mod evaluator;
pub mod loader;
pub mod patch;

pub use evaluator::entity_uv_to_faces;
pub use loader::MiExModelLoader;
pub use patch::apply_bell_patches;

use crate::parser::blockstate::BlockState;
use crate::parser::model_json::BlockModelJson;

/// Universal Builtin Model Registry driving dynamic models from raw upstream MiEx JSON definitions.
pub struct BuiltinModelRegistry;

impl BuiltinModelRegistry {
    /// Attempts to provide a builtin fallback `BlockModelJson` using upstream MiEx JSON templates.
    pub fn get_builtin_model(blockstate: &BlockState) -> Option<BlockModelJson> {
        MiExModelLoader::load_for_blockstate(blockstate)
    }
}

//! # Live Sync Protocol Module (RFC v3.1)
//!
//! Binary serialization, packet definitions, xxHash64 hasher, and network codecs for Minecraft Yefira Live Sync.

pub mod codec;
pub mod constants;
pub mod error;
pub mod hasher;
pub mod packet;

pub use codec::{compute_delta_batch_hash, decode_packet, encode_packet};
pub use constants::*;
pub use error::ProtocolError;
pub use hasher::{compute_canonical_state_hash, normalize_canonical_blockstate};
pub use packet::*;

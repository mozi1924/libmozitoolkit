//! # Live Sync Protocol Module
//!
//! Binary serialization, packet definitions, and network codecs for Minecraft Yefira Live Sync.

pub mod codec;
pub mod constants;
pub mod error;
pub mod packet;

pub use codec::{decode_packet, encode_full_sync_request, encode_repair_requests, encode_sync_config};
pub use constants::{PacketType, StreamStatus, PROTOCOL_MAGIC, PROTOCOL_VERSION};
pub use error::ProtocolError;
pub use packet::{DeltaChange, ManifestSectionEntry, Packet};

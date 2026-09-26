//! # mtk-sync
//!
//! **mtk-sync** provides native WebSocket Live Sync client implementations,
//! binary network protocol codecs, and session lifecycle controllers for libmtk.
//!
//! It connects directly to Minecraft streaming servers, deserializes block delta changes,
//! and synchronizes chunk data into `mtk-voxel` storage containers.

pub mod client;
pub mod events;
pub mod protocol;
pub mod session;

pub use client::{ClientCommand, ClientMessage, SyncClient};
pub use events::SyncEvent;
pub use session::LiveSyncSession;
pub use protocol::{
    decode_packet, encode_full_sync_request, encode_repair_requests, encode_sync_config,
    DeltaChange, ManifestSectionEntry, Packet, PacketType, ProtocolError, StreamStatus,
};

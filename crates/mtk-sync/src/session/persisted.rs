//! Persisted, host-agnostic Live Sync session state.
//!
//! The host stores this opaque JSON string in its own scene/document container.
//! Parsing, validation, UUID hex decoding and HELLO construction live here so
//! every DCC (Blender, C4D, standalone viewers) resumes a session identically.

use glam::IVec3;
use serde::{Deserialize, Serialize};

use crate::protocol::{Packet, PROTOCOL_VERSION};

use super::LiveSyncSession;

/// Serializable subset of the session state required for a zero-rebuild reconnect.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedSessionState {
    /// Server instance UUID as 32 lowercase hex characters.
    #[serde(default)]
    pub server_uuid: String,
    /// Server epoch counter.
    #[serde(default)]
    pub epoch: u64,
    /// Last confirmed monotonic sequence id.
    #[serde(default)]
    pub sequence: u64,
    /// Selection minimum block coordinate `[x, y, z]`.
    #[serde(default)]
    pub min: [i32; 3],
    /// Selection size in blocks `[x, y, z]`.
    #[serde(default)]
    pub size: [i32; 3],
    /// Dimension identifier, e.g. `minecraft:overworld`.
    #[serde(default)]
    pub dimension: String,
    /// xxHash64 canonical state hash (seed 0).
    #[serde(default)]
    pub state_hash: u64,
    /// Set when the user edited the scene after the snapshot; disables cached HELLO.
    #[serde(default)]
    pub user_modified: bool,
}

impl PersistedSessionState {
    /// Parse persisted state from JSON.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Serialize persisted state to JSON.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }

    /// Whether this state can be used for a cached HELLO.
    pub fn is_reconnect_candidate(&self) -> bool {
        if self.user_modified {
            return false;
        }
        let has_uuid = !self.server_uuid.trim().is_empty();
        let has_hash = self.state_hash != 0;
        let has_bounds = self.size != [0, 0, 0];
        (has_uuid || has_hash) && has_bounds
    }

    /// Build the RFC v3.1 HELLO packet for this cached state.
    pub fn to_hello_packet(&self, auth_token: &str) -> Option<Packet> {
        if !self.is_reconnect_candidate() {
            return None;
        }
        Some(Packet::Hello {
            min_version: PROTOCOL_VERSION as u16,
            max_version: PROTOCOL_VERSION as u16,
            capabilities: 0x03, // Zstd + DNA Direct Memory
            flags: 0,
            cached_world_uuid: parse_uuid_hex(&self.server_uuid),
            cached_epoch: self.epoch,
            cached_seq: self.sequence,
            cached_min: IVec3::new(self.min[0], self.min[1], self.min[2]),
            cached_size: IVec3::new(self.size[0], self.size[1], self.size[2]),
            cached_state_hash: self.state_hash,
            auth_token: auth_token.to_string(),
            cached_dimension: if self.dimension.trim().is_empty() {
                "minecraft:overworld".to_string()
            } else {
                self.dimension.clone()
            },
        })
    }
}

/// Decode a 32-character UUID hex string (dashes/`0x` tolerated) into bytes.
/// Invalid input yields the all-zero UUID instead of panicking.
fn parse_uuid_hex(raw: &str) -> [u8; 16] {
    let mut out = [0u8; 16];
    let clean: Vec<u8> = raw
        .trim()
        .trim_start_matches("0x")
        .bytes()
        .filter(|b| *b != b'-')
        .collect();
    if clean.len() != 32 {
        return out;
    }
    for (i, slot) in out.iter_mut().enumerate() {
        let hi = hex_nibble(clean[i * 2]);
        let lo = hex_nibble(clean[i * 2 + 1]);
        if let (Some(hi), Some(lo)) = (hi, lo) {
            *slot = (hi << 4) | lo;
        }
    }
    out
}

#[inline]
fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn uuid_hex(bytes: &[u8; 16]) -> String {
    let mut out = String::with_capacity(32);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

impl LiveSyncSession {
    /// Export the current persisted session state for host storage.
    pub fn export_persisted_state(&self, user_modified: bool) -> PersistedSessionState {
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        PersistedSessionState {
            server_uuid: uuid_hex(&state.server_uuid),
            epoch: state.epoch,
            sequence: state.sequence,
            min: [state.min_pos.x, state.min_pos.y, state.min_pos.z],
            size: [state.size.x, state.size.y, state.size.z],
            dimension: state.dimension.clone(),
            state_hash: state.current_state_hash,
            user_modified,
        }
    }

    /// Export the current persisted session state as opaque JSON.
    pub fn export_persisted_state_json(&self, user_modified: bool) -> String {
        self.export_persisted_state(user_modified).to_json()
    }

    /// Start a session, restoring a cached HELLO from persisted JSON when possible.
    pub fn start_with_persisted_state_json(
        &mut self,
        url: &str,
        auto_reconnect: bool,
        max_reconnect_attempts: usize,
        state_json: Option<&str>,
        auth_token: Option<&str>,
    ) -> Result<(), String> {
        let hello = state_json
            .filter(|s| !s.trim().is_empty())
            .and_then(|s| PersistedSessionState::from_json(s).ok())
            .and_then(|state| state.to_hello_packet(auth_token.unwrap_or("")));
        self.start_with_hello(url, auto_reconnect, max_reconnect_attempts, hello)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persisted_state_roundtrip_and_hello() {
        let state = PersistedSessionState {
            server_uuid: "00112233445566778899aabbccddeeff".to_string(),
            epoch: 7,
            sequence: 42,
            min: [-16, -64, 32],
            size: [64, 128, 64],
            dimension: "minecraft:overworld".to_string(),
            state_hash: 0xDEAD_BEEF,
            user_modified: false,
        };
        let json = state.to_json();
        let decoded = PersistedSessionState::from_json(&json).unwrap();
        assert_eq!(decoded, state);

        match state.to_hello_packet("token").unwrap() {
            Packet::Hello {
                cached_world_uuid,
                cached_epoch,
                cached_seq,
                cached_min,
                cached_size,
                cached_state_hash,
                auth_token,
                cached_dimension,
                ..
            } => {
                assert_eq!(cached_world_uuid[0], 0x00);
                assert_eq!(cached_world_uuid[15], 0xff);
                assert_eq!(cached_epoch, 7);
                assert_eq!(cached_seq, 42);
                assert_eq!(cached_min, IVec3::new(-16, -64, 32));
                assert_eq!(cached_size, IVec3::new(64, 128, 64));
                assert_eq!(cached_state_hash, 0xDEAD_BEEF);
                assert_eq!(auth_token, "token");
                assert_eq!(cached_dimension, "minecraft:overworld");
            }
            _ => panic!("expected HELLO"),
        }
    }

    #[test]
    fn user_modified_state_is_not_a_reconnect_candidate() {
        let mut state = PersistedSessionState {
            server_uuid: "00112233445566778899aabbccddeeff".to_string(),
            size: [16, 16, 16],
            ..Default::default()
        };
        assert!(state.is_reconnect_candidate());
        state.user_modified = true;
        assert!(!state.is_reconnect_candidate());
        assert!(state.to_hello_packet("").is_none());
    }
}

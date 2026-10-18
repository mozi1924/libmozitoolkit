//! HELLO construction helpers shared by every session start path (RFC v3.1 §4.1).
//!
//! The auth token must ride on *every* HELLO, including the no-cache default one:
//! Blender never uses the cached fast path (`VoxelStorage` is not persisted into
//! `.blend`), so omitting the token here makes a token-protected Yefira server
//! reject an otherwise identical secret with `Authentication failed`.

use glam::IVec3;

use crate::protocol::{Packet, PROTOCOL_VERSION};

/// Builds the HELLO used when the host has no persisted session state.
pub(crate) fn default_hello(auth_token: &str) -> Packet {
    Packet::Hello {
        min_version: PROTOCOL_VERSION as u16,
        max_version: PROTOCOL_VERSION as u16,
        capabilities: 0x03, // Zstd + DNA Direct Memory
        flags: 0,
        cached_world_uuid: [0u8; 16],
        cached_epoch: 0,
        cached_seq: 0,
        cached_min: IVec3::ZERO,
        cached_size: IVec3::ZERO,
        cached_state_hash: 0,
        auth_token: auth_token.to_string(),
        cached_dimension: "minecraft:overworld".to_string(),
    }
}

/// Applies the session auth token to a host-built (cached) HELLO.
///
/// The session-level token wins when it is non-empty so a cached HELLO cannot
/// silently ship a stale secret; when the session has no token the host-provided
/// value is preserved for hosts that manage per-connection secrets themselves.
pub(crate) fn with_auth_token(mut hello: Packet, auth_token: &str) -> Packet {
    if let Packet::Hello {
        auth_token: hello_token,
        ..
    } = &mut hello
    {
        if !auth_token.is_empty() {
            *hello_token = auth_token.to_string();
        }
    }
    hello
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hello_token(packet: &Packet) -> &str {
        match packet {
            Packet::Hello { auth_token, .. } => auth_token,
            other => panic!("expected HELLO, got {other:?}"),
        }
    }

    #[test]
    fn default_hello_carries_auth_token() {
        let hello = default_hello("s3cret");
        assert_eq!(hello_token(&hello), "s3cret");

        match &hello {
            Packet::Hello {
                cached_state_hash,
                cached_min,
                ..
            } => {
                assert_eq!(
                    *cached_state_hash, 0,
                    "no-cache HELLO must not claim a cache"
                );
                assert_eq!(*cached_min, IVec3::ZERO);
            }
            other => panic!("expected HELLO, got {other:?}"),
        }
    }

    #[test]
    fn cached_hello_inherits_session_token_and_preserves_cache() {
        let mut cached = default_hello("");
        if let Packet::Hello {
            cached_state_hash,
            cached_min,
            ..
        } = &mut cached
        {
            *cached_state_hash = 0xABCD;
            *cached_min = IVec3::new(1, 2, 3);
        }

        let filled = with_auth_token(cached, "token-from-session");
        assert_eq!(hello_token(&filled), "token-from-session");
        if let Packet::Hello {
            cached_state_hash,
            cached_min,
            ..
        } = &filled
        {
            assert_eq!(*cached_state_hash, 0xABCD, "cached fields must survive");
            assert_eq!(*cached_min, IVec3::new(1, 2, 3));
        }

        let host_managed = with_auth_token(default_hello("host-token"), "");
        assert_eq!(
            hello_token(&host_managed),
            "host-token",
            "an unset session token must not clobber a host-provided one"
        );

        let overridden = with_auth_token(default_hello("stale-token"), "session-token");
        assert_eq!(
            hello_token(&overridden),
            "session-token",
            "the session token is authoritative once set"
        );
    }
}

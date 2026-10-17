//! Regression tests for baseline assembly alignment, sequence handling and
//! state-hash validation robustness.

use glam::IVec3;
use mtk_sync::protocol::constants::*;
use mtk_sync::protocol::hasher::compute_canonical_state_hash;
use mtk_sync::protocol::packet::*;
use mtk_sync::{LiveSyncSession, SyncLifecycle};

/// Builds a single-chunk baseline payload for `size` voxels plus optional biome indices.
fn build_chunk(voxel_indices: &[u8], biome_indices: &[u8]) -> (Vec<u8>, u64) {
    let mut payload = Vec::with_capacity(voxel_indices.len() + biome_indices.len());
    payload.extend_from_slice(voxel_indices);
    payload.extend_from_slice(biome_indices);

    let mut hasher = xxhash_rust::xxh64::Xxh64::new(0);
    hasher.update(&payload);
    (payload, hasher.digest())
}

fn send_baseline(
    session: &LiveSyncSession,
    min_pos: IVec3,
    size: IVec3,
    voxel_indices: &[u8],
    biome_indices: &[u8],
    biome_bits: u8,
    state_hash: u64,
) {
    let total_voxels = (size.x * size.y * size.z) as usize;
    assert_eq!(total_voxels, voxel_indices.len());

    let (payload, chunk_hash) = build_chunk(voxel_indices, biome_indices);
    let block_palette = vec!["minecraft:air".to_string(), "minecraft:stone".to_string()];
    let biome_palette = vec![
        "minecraft:plains".to_string(),
        "minecraft:desert".to_string(),
    ];

    session.process_packet_direct(Packet::Welcome {
        negotiated_version: 3,
        server_uuid: [7u8; 16],
        epoch: 1,
        server_seq: 10,
        current_min: min_pos,
        current_size: size,
        current_state_hash: state_hash,
        decision: SyncDecision::FullBaselineRequired,
        message_or_dimension: "minecraft:overworld".to_string(),
    });

    session.process_packet_direct(Packet::BaselineBegin {
        epoch: 1,
        snapshot_uuid: [9u8; 16],
        snapshot_seq: 10,
        min_pos,
        size,
        state_hash,
        palette_revision: 1,
        total_chunks: 1,
        total_encoded_bytes: payload.len() as u64,
        total_decoded_bytes: payload.len() as u64,
        compression_mode: 0,
        palette_bits_per_block: 8,
        biome_bits_per_quart: biome_bits,
        dimension: "minecraft:overworld".to_string(),
        block_palette,
        biome_palette,
    });

    session.process_packet_direct(Packet::BaselineChunk {
        epoch: 1,
        snapshot_uuid: [9u8; 16],
        chunk_index: 0,
        total_chunks: 1,
        first_voxel_index: 0,
        voxel_count: total_voxels as u32,
        payload_len: payload.len() as u32,
        decoded_target_bytes: payload.len() as u32,
        chunk_xxhash64: chunk_hash,
        payload,
    });

    session.process_packet_direct(Packet::BaselineEnd {
        epoch: 1,
        snapshot_uuid: [9u8; 16],
        snapshot_seq: 10,
        total_chunks_sent: 1,
        final_state_hash: state_hash,
        status: 0,
    });
}

#[test]
fn test_unaligned_selection_with_multi_biomes_validates_state_hash() {
    let session = LiveSyncSession::new(None, None, None, true);

    // Deliberately NOT 16-aligned on X (4 % 16 != 0).
    let min_pos = IVec3::new(4, 0, 0);
    let size = IVec3::new(8, 1, 1);
    let voxels = [0u8; 8];
    // Two quart columns: qx=0 -> plains (wx=4), qx=1 -> desert (wx=8)
    let biomes = [0u8, 1u8];

    let expected_hash = compute_canonical_state_hash(
        min_pos,
        size,
        "minecraft:overworld",
        |_, _, _| "minecraft:air".to_string(),
        |qx, _, _| {
            if qx == 0 {
                "minecraft:plains".to_string()
            } else {
                "minecraft:desert".to_string()
            }
        },
    );

    send_baseline(&session, min_pos, size, &voxels, &biomes, 8, expected_hash);

    assert_eq!(
        session.lifecycle(),
        SyncLifecycle::Live,
        "Baseline on an unaligned selection must validate its canonical state hash"
    );
}

#[test]
fn test_superseded_baseline_frames_are_ignored() {
    let session = LiveSyncSession::new(None, None, None, true);

    let min_pos = IVec3::new(0, 0, 0);
    let size = IVec3::new(8, 1, 1);
    let voxels = [0u8; 8];
    let expected_hash = compute_canonical_state_hash(
        min_pos,
        size,
        "minecraft:overworld",
        |_, _, _| "minecraft:air".to_string(),
        |_, _, _| "minecraft:plains".to_string(),
    );
    let (payload, chunk_hash) = build_chunk(&voxels, &[]);
    let block_palette = vec!["minecraft:air".to_string(), "minecraft:stone".to_string()];
    let biome_palette = vec!["minecraft:plains".to_string()];

    session.process_packet_direct(Packet::Welcome {
        negotiated_version: 3,
        server_uuid: [7u8; 16],
        epoch: 1,
        server_seq: 10,
        current_min: min_pos,
        current_size: size,
        current_state_hash: expected_hash,
        decision: SyncDecision::FullBaselineRequired,
        message_or_dimension: "minecraft:overworld".to_string(),
    });

    let begin = |uuid: [u8; 16], seq: u64| Packet::BaselineBegin {
        epoch: 1,
        snapshot_uuid: uuid,
        snapshot_seq: seq,
        min_pos,
        size,
        state_hash: expected_hash,
        palette_revision: 1,
        total_chunks: 1,
        total_encoded_bytes: payload.len() as u64,
        total_decoded_bytes: payload.len() as u64,
        compression_mode: 0,
        palette_bits_per_block: 8,
        biome_bits_per_quart: 8,
        dimension: "minecraft:overworld".to_string(),
        block_palette: block_palette.clone(),
        biome_palette: biome_palette.clone(),
    };

    // Stream A starts, then is superseded by stream B (the user moved the selection).
    session.process_packet_direct(begin([1u8; 16], 10));
    session.process_packet_direct(begin([2u8; 16], 11));

    // A late chunk from stream A must be ignored, not corrupt stream B.
    session.process_packet_direct(Packet::BaselineChunk {
        epoch: 1,
        snapshot_uuid: [1u8; 16],
        chunk_index: 0,
        total_chunks: 1,
        first_voxel_index: 0,
        voxel_count: 8,
        payload_len: payload.len() as u32,
        decoded_target_bytes: payload.len() as u32,
        chunk_xxhash64: chunk_hash,
        payload: payload.clone(),
    });

    // A late END from stream A must be ignored as well (previously left the
    // lifecycle stuck in ValidatingHash / progress at 75%).
    session.process_packet_direct(Packet::BaselineEnd {
        epoch: 1,
        snapshot_uuid: [1u8; 16],
        snapshot_seq: 10,
        total_chunks_sent: 1,
        final_state_hash: expected_hash,
        status: 0,
    });
    assert_ne!(session.lifecycle(), SyncLifecycle::Live);

    // Stream B completes normally.
    session.process_packet_direct(Packet::BaselineChunk {
        epoch: 1,
        snapshot_uuid: [2u8; 16],
        chunk_index: 0,
        total_chunks: 1,
        first_voxel_index: 0,
        voxel_count: 8,
        payload_len: payload.len() as u32,
        decoded_target_bytes: payload.len() as u32,
        chunk_xxhash64: chunk_hash,
        payload,
    });
    session.process_packet_direct(Packet::BaselineEnd {
        epoch: 1,
        snapshot_uuid: [2u8; 16],
        snapshot_seq: 11,
        total_chunks_sent: 1,
        final_state_hash: expected_hash,
        status: 0,
    });

    assert_eq!(session.lifecycle(), SyncLifecycle::Live);
}

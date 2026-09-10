use glam::IVec3;
use mtk_voxel::protocol::*;

#[test]
fn test_selection_info_codec() {
    let mut data = vec![0x4D, 0x43, 0x01, 0x01];
    data.extend_from_slice(&(-10i32).to_le_bytes());
    data.extend_from_slice(&(64i32).to_le_bytes());
    data.extend_from_slice(&(-20i32).to_le_bytes());
    data.extend_from_slice(&(16i32).to_le_bytes());
    data.extend_from_slice(&(32i32).to_le_bytes());
    data.extend_from_slice(&(16i32).to_le_bytes());

    let packet = decode_packet(&data).expect("Failed to decode SelectionInfo");
    match packet {
        Packet::SelectionInfo { min_pos, size } => {
            assert_eq!(min_pos, IVec3::new(-10, 64, -20));
            assert_eq!(size, IVec3::new(16, 32, 16));
        }
        _ => panic!("Unexpected packet type"),
    }
}

#[test]
fn test_delta_update_codec() {
    let mut data = vec![0x4D, 0x43, 0x01, 0x03];
    data.extend_from_slice(&1001u32.to_le_bytes()); // seq_id
    data.extend_from_slice(&0i32.to_le_bytes()); // min_x
    data.extend_from_slice(&0i32.to_le_bytes()); // min_y
    data.extend_from_slice(&0i32.to_le_bytes()); // min_z
    data.extend_from_slice(&2u16.to_le_bytes()); // count

    // change 1
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&2u16.to_le_bytes());
    data.extend_from_slice(&3u16.to_le_bytes());
    let st1 = "minecraft:stone";
    data.extend_from_slice(&(st1.len() as u16).to_le_bytes());
    data.extend_from_slice(st1.as_bytes());

    // change 2
    data.extend_from_slice(&4u16.to_le_bytes());
    data.extend_from_slice(&5u16.to_le_bytes());
    data.extend_from_slice(&6u16.to_le_bytes());
    let st2 = "minecraft:dirt";
    data.extend_from_slice(&(st2.len() as u16).to_le_bytes());
    data.extend_from_slice(st2.as_bytes());

    let packet = decode_packet(&data).expect("Failed to decode DeltaUpdate");
    match packet {
        Packet::DeltaUpdate {
            seq_id,
            min_pos,
            changes,
        } => {
            assert_eq!(seq_id, 1001);
            assert_eq!(min_pos, IVec3::new(0, 0, 0));
            assert_eq!(changes.len(), 2);
            assert_eq!(changes[0].rel_pos, IVec3::new(1, 2, 3));
            assert_eq!(changes[0].state, "minecraft:stone");
            assert_eq!(changes[1].rel_pos, IVec3::new(4, 5, 6));
            assert_eq!(changes[1].state, "minecraft:dirt");
        }
        _ => panic!("Unexpected packet type"),
    }
}

#[test]
fn test_section_manifest_codec() {
    let mut data = vec![0x4D, 0x43, 0x01, 0x05];
    data.extend_from_slice(&42u32.to_le_bytes()); // seq_id
    data.extend_from_slice(&2u32.to_le_bytes()); // count = 2

    // entry 1: (0, 4, 0, 0x12345678)
    data.extend_from_slice(&0i32.to_le_bytes());
    data.extend_from_slice(&4i32.to_le_bytes());
    data.extend_from_slice(&0i32.to_le_bytes());
    data.extend_from_slice(&0x12345678u32.to_le_bytes());

    // entry 2: (1, 4, 0, 0xABCDEF01)
    data.extend_from_slice(&1i32.to_le_bytes());
    data.extend_from_slice(&4i32.to_le_bytes());
    data.extend_from_slice(&0i32.to_le_bytes());
    data.extend_from_slice(&0xABCDEF01u32.to_le_bytes());

    let packet = decode_packet(&data).expect("Failed to decode SectionManifest");
    match packet {
        Packet::SectionManifest { seq_id, sections } => {
            assert_eq!(seq_id, 42);
            assert_eq!(sections.len(), 2);
            assert_eq!(sections[0].coord, IVec3::new(0, 4, 0));
            assert_eq!(sections[0].crc32, 0x12345678);
            assert_eq!(sections[1].coord, IVec3::new(1, 4, 0));
            assert_eq!(sections[1].crc32, 0xABCDEF01);
        }
        _ => panic!("Unexpected packet type"),
    }
}

#[test]
fn test_client_request_encoders() {
    let full_req = encode_full_sync_request();
    assert_eq!(full_req, vec![0x4D, 0x43, 0x01, 0x80]);

    let config_req = encode_sync_config(1, 60, true);
    assert_eq!(config_req, vec![0x4D, 0x43, 0x01, 0x82, 1, 60, 1]);

    let repair_reqs = encode_repair_requests(&[IVec3::new(0, 1, 2), IVec3::new(3, 4, 5)], 64);
    assert_eq!(repair_reqs.len(), 1);
    assert_eq!(repair_reqs[0][0..4], [0x4D, 0x43, 0x01, 0x81]);
}

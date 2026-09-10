use mtk_voxel::sync::LiveSyncSession;
use mtk_voxel::types::MesherConfig;

#[test]
fn test_sync_session_initialization() {
    let session = LiveSyncSession::new(Some(MesherConfig::default()), None);
    let events = session.poll_events();
    assert!(events.is_empty());
}

#[test]
fn test_sync_session_packet_flow() {
    let session = LiveSyncSession::new(Some(MesherConfig::default()), None);

    // 1. Manually test storage update and meshing on session storage
    {
        let mut st = session.storage.write().unwrap();
        st.set_bounds(0, 0, 0, 16, 16, 16);
        let palette = vec!["minecraft:air".to_string(), "minecraft:stone".to_string()];
        let mut grid = vec![0u16; 4096];
        grid[0] = 1;
        st.set_section_snapshot(0, 0, 0, 0, 0, 0, 16, 16, 16, &palette, &grid, None, None);
        assert_eq!(st.get_block(0, 0, 0), "minecraft:stone");
    }
}

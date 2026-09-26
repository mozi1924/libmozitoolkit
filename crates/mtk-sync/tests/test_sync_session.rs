use mtk_sync::LiveSyncSession;
use mtk_voxel::types::MesherConfig;

#[test]
fn test_sync_session_initialization() {
    let session = LiveSyncSession::new(Some(MesherConfig::default()), None, None, true);
    let events = session.poll_events();
    assert!(events.is_empty());
    assert!(session.unified_mesh);
}

#[test]
fn test_sync_session_packet_flow() {
    let session = LiveSyncSession::new(Some(MesherConfig::default()), None, None, true);

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

    // 2. Test get_world_mesh on session
    let world_mesh = session.get_world_mesh();
    assert!(!world_mesh.is_empty());
    assert_eq!(world_mesh.positions.len(), 24); // 1 cube face count (6 faces * 4 verts)
}

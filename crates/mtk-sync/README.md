# mtk-sync

Native WebSocket Live Sync client, binary network protocol codecs, and session controller for libmtk.

This crate manages the bi-directional streaming connection between Minecraft (via companion mod/plugin) and DCC hosts (such as Blender MoziToolKit), orchestrating block updates, chunk delta streaming, and event pumping into `mtk-voxel`.

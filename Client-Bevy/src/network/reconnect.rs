use bevy::prelude::*;
use super::*;

#[derive(Resource)]
pub struct NetServerAddr(pub String);

/// 启动网络（按模式：mock 或真实 TCP）
pub(crate) fn setup_network(
    mut net: ResMut<NetConnection>,
    mut auth: ResMut<AuthFeedback>,
    mode: Res<NetMode>,
    addr: Res<NetServerAddr>,
) {
    match mode.0 {
        NetworkMode::Mock => {
            let (to_server, from_client) = crossbeam_channel::bounded::<Vec<u8>>(1024);
            let (to_client, from_server) = crossbeam_channel::bounded::<Vec<u8>>(1024);
            net.to_server = Some(to_server);
            net.from_server = Some(from_server);
            mock::spawn_mock(to_client, from_client);
            tracing::info!("🌐 Mock 网络已启动（本地模拟服务器）");
        }
        NetworkMode::Real => match tcp::connect(&addr.0, net.client_version_hash) {
            Ok(conn) => {
                net.to_server = Some(conn.to_server);
                net.tcp_events = Some(conn.from_server);
                net.mode = NetworkMode::Real;
                tracing::info!("🌐 真实 TCP 已连接: {}", addr.0);
            }
            Err(e) => {
                tracing::error!("🔌 连接服务器 {} 失败: {}", addr.0, e);
                auth.login_error = Some(format!("无法连接服务器 {}：{}", addr.0, e));
                net.disconnected = Some(format!("{}", e));
            }
        },
    }
}

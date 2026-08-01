// ============================================================================
// mir2 bevy - 主入口
// ============================================================================
// 传奇2 (Legend of Mir 2) 客户端 Bevy 移植版
//
// 用法:
//   cargo run --bin client_bevy                     # 默认地图 n0 + 演示角色
//   cargo run --bin client_bevy -- --map n0
//   cargo run --bin client_bevy -- --map 11yearvilliage
//   cargo run --bin client_bevy -- --no-actors      # 只渲染地图（截图验证用）

use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy::window::WindowResolution;
use client_bevy::actor::ActorPlugin;
use client_bevy::event_bus::EventBusPlugin;
use client_bevy::map_renderer::MapRenderPlugin;
use client_bevy::network::NetworkPlugin;
use client_bevy::scenes::AppState;
use client_bevy::ui::login::LoginPlugin;
use client_bevy::ui::select::SelectPlugin;

fn main() {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(LogPlugin {
                filter:
                    "info,bevy_render=warn,bevy_asset=warn,bevy_log=warn,bevy_diagnostic=warn,wgpu_hal=warn,naga=warn"
                        .into(),
                ..default()
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Mir2 (Bevy) — 传奇2 客户端移植".to_string(),
                    // 游戏窗口强制 1 世界单位 = 1 像素（忽略系统 DPI 缩放）
                    resolution: WindowResolution::new(1280u32, 800u32)
                        .with_scale_factor_override(1.0),
                    ..default()
                }),
                ..default()
            }),
    );
    app.insert_resource(ClearColor(Color::srgb(0.07, 0.08, 0.12)));
    app.init_state::<AppState>();
    app.add_plugins(EventBusPlugin);
    app.add_plugins((NetworkPlugin, LoginPlugin, SelectPlugin));
    // --auto-enter: 自动从登录界面进入游戏（自动化验证用）
    if std::env::args().any(|a| a == "--auto-enter") {
        // auto_enter 需要覆盖 Login 和 Select 两个状态（内部自行判断）
        app.add_systems(Update, auto_enter);
    }
    // --no-actors: 只渲染地图（用于纯地图截图验证）
    if std::env::args().any(|a| a == "--no-actors") {
        app.add_plugins(MapRenderPlugin);
    } else {
        app.add_plugins((MapRenderPlugin, ActorPlugin));
    }
    app.run();
}

/// --auto-enter：自动驱动 mock 登录流程（Login→Select→Game，验证网络管道）
fn auto_enter(
    mut net: ResMut<client_bevy::network::NetworkContext>,
    state: Res<State<AppState>>,
    mut frames: Local<u32>,
) {
    use mir2_shared::packets::client::account::{Login, StartGame};
    *frames += 1;
    if *frames == 5 {
        net.state = client_bevy::network::NetState::LoggingIn;
        net.send_packet(&Login {
            account_id: "test".to_string(),
            password: "123456".to_string(),
        });
    }
    if *state == AppState::Select && net.selected_index.is_none() {
        let first_index = net.characters.first().map(|c| c.index);
        if let Some(idx) = first_index {
            net.selected_index = Some(idx);
            net.send_packet(&StartGame {
                character_index: idx,
            });
        }
    }
}

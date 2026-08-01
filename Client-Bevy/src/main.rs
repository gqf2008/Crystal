// ============================================================================
// mir2 bevy - 主入口
// ============================================================================
// 里程碑 1: 加载 Data/*.Lib + Map/*.map，用 Bevy 渲染地图
//
// 用法:
//   cargo run --bin client_bevy                 # 默认地图 0100
//   cargo run --bin client_bevy -- --map 11yearvilliage
//   cargo run --bin client_bevy -- --map n0

use bevy::log::LogPlugin;
use bevy::prelude::*;
use client_bevy::map_renderer::MapRenderPlugin;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(LogPlugin {
                    filter: "info,bevy_render=warn,bevy_asset=warn,bevy_log=warn,bevy_diagnostic=warn,wgpu_hal=warn,naga=warn".into(),
                    ..default()
                })
                .set(WindowPlugin {
            primary_window: Some(Window {
                title: "Mir2 (Bevy) — 传奇2 客户端移植".to_string(),
                resolution: (1280.0, 800.0).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(MapRenderPlugin)
        .run();
}

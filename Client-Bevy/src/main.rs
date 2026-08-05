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
use bevy::render::settings::{Backends, RenderCreation, WgpuSettings};
use bevy::render::RenderPlugin;
use client_bevy::actor::ActorPlugin;
use client_bevy::event_bus::EventBusPlugin;
use client_bevy::map_renderer::MapRenderPlugin;
use client_bevy::network::NetworkPlugin;
use client_bevy::scenes::AppState;
use client_bevy::ui::intro::IntroPlugin;
use client_bevy::ui::login::LoginPlugin;
use client_bevy::ui::modal_box::ModalBoxPlugin;
use client_bevy::ui::new_character::NewCharacterPlugin;
use client_bevy::ui::pinyin_ime::PinyinImePlugin;
use client_bevy::ui::select::SelectPlugin;

mod auto;

// #71：全局给 UI 实体打 RenderLayers layer 1（由独立 UI 相机渲染，地图相机不重画 UI）
use client_bevy::ui::sprite_ui::mark_ui_render_layers;

fn main() {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            // 资源目录固定到源码 assets/（Bevy 0.19 默认按 exe 目录解析，跨 target 运行会找不到 shader）
            .set(AssetPlugin {
                file_path: concat!(env!("CARGO_MANIFEST_DIR"), "/assets").to_string(),
                ..default()
            })
            // 使用 DX12 后端（Vulkan 的 swapchain present 在此机器上会冻结）
            .set(RenderPlugin {
                render_creation: RenderCreation::Automatic(Box::new(WgpuSettings {
                    backends: Some(Backends::DX12),
                    ..default()
                })),
                ..default()
            })
            .set(LogPlugin {
                filter:
                    "info,bevy_render=warn,bevy_asset=warn,bevy_log=warn,bevy_diagnostic=warn,wgpu_hal=warn,naga=warn,icu4x=error,icu_segmenter=error"
                        .into(),
                ..default()
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Mir2 (Bevy) — 传奇2 客户端移植".to_string(),
                    resolution: (1024u32, 768u32).into(),
                    // 禁用系统 IME：用游戏内置拼音输入法（src/ui/pinyin_ime.rs）。
                    // winit 用 IACE_CHILDREN 解关联 IME 上下文，字母键作为原始
                    // KeyboardInput 到达，不被手心等系统输入法拦截。
                    ime_enabled: false,
                    // 无 vsync：避免会话中 vblank 缺失导致 present 永久阻塞（画面冻结）
                    present_mode: bevy::window::PresentMode::Immediate,
                    ..default()
                }),
                ..default()
            }),
    );
    app.insert_resource(ClearColor(Color::srgb(0.07, 0.08, 0.12)));
    // 性能（#112）：PresentMode::Immediate 无 vsync 会无限刷帧烧 CPU（基线 ~150% 单核）。
    // 用 winit Reactive 60Hz 限帧：动画/输入照常（事件唤醒 + 16.6ms 心跳），CPU 大幅下降，
    // 且不引入 vsync 阻塞（此前 DX12+Vulkan 均出现过 present 冻结）。
    use std::time::Duration;
    app.insert_resource(bevy::winit::WinitSettings {
        focused_mode: bevy::winit::UpdateMode::reactive(Duration::from_secs_f64(1.0 / 60.0)),
        ..default()
    });
    app.init_state::<AppState>();
    // --skip-login: 直接从登录界面进入游戏（诊断呈现问题用）
    if std::env::args().any(|a| a == "--skip-login") {
        app.add_systems(Update, |mut next: ResMut<NextState<AppState>>| {
            next.set(AppState::Game)
        });
    }
    app.add_plugins(EventBusPlugin);
    app.add_plugins(PinyinImePlugin);
    app.add_plugins((
        NetworkPlugin,
        IntroPlugin,
        LoginPlugin,
        SelectPlugin,
        NewCharacterPlugin,
        ModalBoxPlugin,
        client_bevy::game::GamePlugin,
    ));
    app.add_systems(Update, mark_ui_render_layers);
    // #91 UI 按钮交互音效（全场景：登录/选角/游戏）
    app.add_systems(Update, client_bevy::ui::sprite_ui::ui_button_sound_system);
    auto::register(&mut app);
    // --no-actors: 只渲染地图（用于纯地图截图验证）
    if std::env::args().any(|a| a == "--no-actors") {
        app.add_plugins(MapRenderPlugin);
    } else {
        app.add_plugins((MapRenderPlugin, ActorPlugin));
    }
    app.run();
}

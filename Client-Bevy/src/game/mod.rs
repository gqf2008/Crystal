// ============================================================================
// 游戏场景模块（M8：HUD + 聊天 + 玩家控制 + 移动）
// UI 交互参考：Client/MirScenes/GameScene.cs + Dialogs/MainDialogs.cs
// 绘制/网络参考：Client-Macroquad/src（main_dialog / player_control / network）
// ============================================================================

pub mod chat;
pub mod hud;
pub mod movement;
pub mod pathfinding;
pub mod player_control;

use bevy::prelude::*;

use crate::scenes::AppState;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<hud::HudState>();
        app.init_resource::<chat::ChatState>();
        app.init_resource::<movement::NetMotions>();
        app.init_resource::<player_control::ControlState>();
        // 游戏场景 UI 相机（HUD/聊天共用，先于各 UI 插件创建）
        app.add_systems(OnEnter(AppState::Game), setup_game_ui_camera);
        app.add_plugins((
            hud::HudPlugin,
            chat::ChatPlugin,
            movement::MovementPlugin,
            player_control::PlayerControlPlugin,
        ));
    }
}

fn setup_game_ui_camera(mut commands: Commands) {
    // 与 login 的 UI 相机不同：游戏场景有地图相机（order 0），UI 相机不能清屏
    // 否则会把地图整个盖掉（Bevy 后渲染的相机默认用 ClearColor 清屏）
    use bevy::camera::ScalingMode;
    commands.spawn((
        crate::ui::sprite_ui::UiEntity,
        Camera2d,
        Transform::from_xyz(512.0, -384.0, 100.0),
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::Fixed {
                width: 1024.0,
                height: 768.0,
            },
            ..OrthographicProjection::default_2d()
        }),
        Camera {
            order: 1,
            clear_color: ClearColorConfig::None,
            ..default()
        },
    ));
}

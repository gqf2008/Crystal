// UI System - 用户界面系统
use bevy::prelude::*;
use crate::bevy::components::{Player, GridPosition, Movement};

/// FPS 显示组件
#[derive(Component)]
pub struct FpsText;

/// 玩家信息显示组件
#[derive(Component)]
pub struct PlayerInfoText;

/// 初始化调试 UI
pub fn setup_debug_ui(
    mut commands: Commands,
) {
    // 创建 FPS 文本
    commands.spawn((
        Text::new("FPS: --"),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
        FpsText,
    ));
    
    // 创建玩家信息文本
    commands.spawn((
        Text::new("玩家: --"),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(30.0),
            left: Val::Px(10.0),
            ..default()
        },
        PlayerInfoText,
    ));
    
    println!("🖥️ 调试 UI 初始化完成");
}

/// 更新 FPS 显示
pub fn update_fps_system(
    time: Res<Time>,
    mut query: Query<&mut Text, With<FpsText>>,
) {
    for mut text in query.iter_mut() {
        let fps = 1.0 / time.delta_secs();
        text.0 = format!("FPS: {:.0}", fps);
    }
}

/// 更新玩家信息显示
pub fn update_player_info_system(
    player_query: Query<(&GridPosition, &Movement), With<Player>>,
    mut text_query: Query<&mut Text, With<PlayerInfoText>>,
) {
    for mut text in text_query.iter_mut() {
        // 使用 iter() 而不是 get_single()
        for (pos, movement) in player_query.iter().take(1) {
            text.0 = format!(
                "玩家: ({}, {}) {:?} {}",
                pos.x, pos.y,
                movement.direction,
                if movement.is_running { "跑" } else { "走" }
            );
        }
    }
}

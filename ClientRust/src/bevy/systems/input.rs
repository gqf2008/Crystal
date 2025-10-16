// Input System - 输入处理系统
use bevy::prelude::*;
use crate::bevy::components::{Player, Movement};
use crate::mir2_shared::protocol::MirDirection;

/// 鼠标输入系统 - 处理鼠标点击移动
pub fn mouse_input_system(
    mouse_button: Res<ButtonInput<MouseButton>>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    windows: Query<&Window>,
    mut player_query: Query<&mut Movement, With<Player>>,
) {
    let Ok((camera, camera_transform)) = camera_query.get_single() else {
        return;
    };
    
    let Ok(window) = windows.get_single() else {
        return;
    };
    
    // 检查鼠标按键
    let is_running = if mouse_button.pressed(MouseButton::Right) {
        Some(true) // 右键跑步
    } else if mouse_button.pressed(MouseButton::Left) {
        Some(false) // 左键走路
    } else {
        None
    };
    
    if let Some(running) = is_running {
        if let Some(cursor_pos) = window.cursor_position() {
            // 将屏幕坐标转换为世界坐标
            if let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) {
                // TODO: 计算方向并更新玩家移动组件
                // 这里需要玩家当前位置才能计算方向
                // 后续实现
                
                for mut movement in player_query.iter_mut() {
                    movement.set_running(running);
                }
            }
        }
    }
}

/// 键盘输入系统 - 处理键盘快捷键
pub fn keyboard_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        // TODO: 打开菜单或退出游戏
    }
}

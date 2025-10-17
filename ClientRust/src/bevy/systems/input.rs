// Input System - 输入处理系统
use bevy::prelude::*;
use crate::bevy::components::{Player, Movement, GridPosition};
use mir2_shared::MirDirection;

/// 计算从 source 到 dest 的方向
fn calculate_direction(source_x: i32, source_y: i32, dest_x: i32, dest_y: i32) -> MirDirection {
    use mir2_shared::MirDirection::*;
    
    if source_x < dest_x {
        if source_y < dest_y {
            return DownRight;
        }
        if source_y > dest_y {
            return UpRight;
        }
        return Right;
    }

    if source_x > dest_x {
        if source_y < dest_y {
            return DownLeft;
        }
        if source_y > dest_y {
            return UpLeft;
        }
        return Left;
    }

    if source_y < dest_y {
        Down
    } else {
        Up
    }
}

/// 鼠标输入系统 - 处理鼠标点击移动
pub fn mouse_input_system(
    mouse_button: Res<ButtonInput<MouseButton>>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    windows: Query<&Window>,
    mut player_query: Query<(&GridPosition, &mut Movement), With<Player>>,
) {
    // 获取第一个摄像机
    let Some((camera, camera_transform)) = camera_query.iter().next() else {
        return;
    };
    
    // 获取第一个窗口
    let Some(window) = windows.iter().next() else {
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
                // 世界坐标转网格坐标 (48×32 格子)
                const CELL_WIDTH: f32 = 48.0;
                const CELL_HEIGHT: f32 = 32.0;
                let dest_x = (world_pos.x / CELL_WIDTH) as i32;
                let dest_y = (world_pos.y / CELL_HEIGHT) as i32;
                
                // 更新玩家移动状态
                for (grid_pos, mut movement) in player_query.iter_mut() {
                    // 计算方向
                    let direction = calculate_direction(
                        grid_pos.x, grid_pos.y,
                        dest_x, dest_y
                    );
                    
                    // 更新移动组件
                    movement.direction = direction;
                    movement.set_running(running);
                    
                    println!("🎯 目标:({}, {}) 方向:{:?} {}", 
                        dest_x, dest_y, direction,
                        if running { "跑步" } else { "走路" }
                    );
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

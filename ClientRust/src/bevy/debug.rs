// 调试辅助系统
// 
// 提供快捷键快速测试各个场景

use bevy::prelude::*;

/// 调试快捷键系统
/// 
/// F1 - 跳转到登录场景
/// F2 - 跳转到选择场景
/// F3 - 跳转到游戏场景
/// F5 - 重新加载地图
/// ESC - 返回登录场景
pub fn debug_shortcuts_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    current_state: Res<State<crate::bevy::GameState>>,
    mut next_state: ResMut<NextState<crate::bevy::GameState>>,
    mut map_load_request: Option<ResMut<crate::bevy::scenes::MapLoadRequest>>,
) {
    use crate::bevy::GameState;
    
    // F1 - 登录场景
    if keyboard.just_pressed(KeyCode::F1) {
        info!("🔧 调试: 切换到登录场景");
        next_state.set(GameState::Login);
    }
    
    // F2 - 选择场景
    if keyboard.just_pressed(KeyCode::F2) {
        info!("🔧 调试: 切换到选择场景");
        next_state.set(GameState::Select);
    }
    
    // F3 - 游戏场景
    if keyboard.just_pressed(KeyCode::F3) {
        info!("🔧 调试: 切换到游戏场景");
        next_state.set(GameState::Game);
    }
    
    // F5 - 重新加载地图 (仅在游戏场景)
    if keyboard.just_pressed(KeyCode::F5) && current_state.get() == &GameState::Game {
        if let Some(mut load_request) = map_load_request {
            info!("🔧 调试: 重新加载地图");
            load_request.request("0".to_string());
        }
    }
    
    // ESC - 返回登录
    if keyboard.just_pressed(KeyCode::Escape) && current_state.get() != &GameState::Login {
        info!("🔧 调试: 返回登录场景");
        next_state.set(GameState::Login);
    }
}

/// 显示调试信息
pub fn debug_info_overlay_system(
    mut gizmos: Gizmos,
    current_state: Res<State<crate::bevy::GameState>>,
    map_data: Option<Res<crate::bevy::scenes::MapRenderData>>,
    camera_query: Query<(&Transform, &crate::bevy::scenes::GameCamera)>,
) {
    // 在屏幕顶部显示当前状态
    // (使用 gizmos 绘制调试信息)
    
    // 如果在游戏场景,显示地图信息
    if let Some(map_data) = map_data {
        if map_data.width > 0 {
            // 地图已加载
            if let Ok((transform, camera)) = camera_query.single() {
                // 显示摄像机位置
                let cam_pos = transform.translation;
                info!("📷 摄像机: ({:.1}, {:.1})", cam_pos.x, cam_pos.y);
            }
        }
    }
}

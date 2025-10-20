// Bevy States - 游戏状态定义
use bevy::prelude::*;

/// 游戏状态枚举
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum GameState {
    #[default]
    Loading,  // 加载资源
    Login,    // 登录界面
    Select,   // 选择角色
    Game,     // 游戏中
}

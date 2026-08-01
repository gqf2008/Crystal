// 场景状态机（对应 macroquad 的 SceneKind：Login/Select/Game）
use bevy::prelude::*;

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    /// 启动画面（logo）
    #[default]
    Intro,
    Login,
    Select,
    Game,
}

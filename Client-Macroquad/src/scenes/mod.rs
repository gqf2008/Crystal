// ============================================================================
// Scene System - 游戏场景管理系统（enum_dispatch 静态分发）
// ============================================================================

pub mod login_scene;
pub mod character_select_scene;
pub mod game_scene;
pub mod loading_scene;

pub use login_scene::LoginScene;
pub use character_select_scene::CharacterSelectScene;
pub use game_scene::GameScene;
pub use loading_scene::LoadingScene;

use crate::compat::GameResult;
use enum_dispatch::enum_dispatch;

/// 场景切换请求
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneTransition {
    None,
    Login,
    CharacterSelect,
    Game,
    Loading,
    Exit,
}

/// 场景 trait（由 enum_dispatch 自动分发）
#[enum_dispatch]
pub trait SceneHandler {
    fn name(&self) -> &str;
    fn on_enter(&mut self) -> GameResult;
    fn on_exit(&mut self) -> GameResult;
    fn update(&mut self, dt: f32) -> GameResult<SceneTransition>;
    fn render(&mut self) -> GameResult;
    fn handle_input(&mut self) -> GameResult;
}

/// 场景枚举（enum_dispatch 自动生成分发代码）
#[enum_dispatch(SceneHandler)]
pub enum Scene {
    Login(LoginScene),
    CharacterSelect(CharacterSelectScene),
    Game(GameScene),
    Loading(LoadingScene),
}


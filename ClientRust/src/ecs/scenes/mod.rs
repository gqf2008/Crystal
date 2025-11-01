// ============================================================================
// 场景系统 - 模块定义
// ============================================================================
//
// 注意: 这是 ECS 版本的简化场景系统
// 完整的 OOP 版本在 src/scenes/ 目录（包含完整UI、纹理、网络等）
// ============================================================================

mod game_scene;
pub mod login_scene;  // LoginScene模块（包含ECS组件、系统、对话框等）
mod select_scene;
pub mod ui;  // 共享UI组件（Button, TextInput等）

use std::sync::Arc;
use crate::network::NetContext;
use ggez::graphics::Canvas;
use ggez::input::keyboard::KeyInput;
use ggez::{Context, GameResult};
use hecs::World;

pub use game_scene::GameScene;
pub use login_scene::LoginScene;
pub use select_scene::SelectScene;

/// 场景类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneType {
    /// 登录场景
    Login,
    /// 角色选择场景
    Select,
    /// 游戏场景
    Game,
}

/// 场景 Trait
pub trait Scene {
    /// 向下转型支持（用于访问具体场景类型）
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
    fn update(
        &mut self,
        ctx: &mut Context,
        world: &mut World,
    ) -> GameResult<Option<SceneType>>;

    fn draw(&mut self, ctx: &mut Context, canvas: &mut Canvas, world: &World) -> GameResult;

}
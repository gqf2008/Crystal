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
use crate::ecs::{GameContext, GameWorld};
use crate::network::NetContext;
use ggez::graphics::{Canvas, GraphicsContext};
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
/// 
/// ✅ 使用 GameContext 统一访问游戏状态
/// - 零拷贝访问：直接引用 ggez Context 和 ECS World
/// - 事件访问：通过 GameContext API 而非 WorldExt
/// - 简化接口：单一参数代替多个分离的参数
pub trait Scene {
    /// 向下转型支持（用于访问具体场景类型）
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
    
    /// 更新场景逻辑
    /// 
    /// # 参数
    /// - `game_ctx`: 统一的游戏上下文，包含 ggez Context、ECS World、事件等
    /// 
    /// # 返回
    /// - `Ok(Some(SceneType))`: 需要切换到新场景
    /// - `Ok(None)`: 继续当前场景
    fn update(
        &mut self,
        ctx: &mut GameContext,
    ) -> GameResult<Option<SceneType>>;

    /// 渲染场景
    /// 
    /// # 参数
    /// - `ctx`: ggez 上下文（用于绘制）
    /// - `world`: ECS World（只读访问）
    fn draw(&mut self, ctx: &mut GraphicsContext, world: &GameWorld) -> GameResult;
}
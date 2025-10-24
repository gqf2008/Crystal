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

use crate::network::NetworkCommand;
use ggez::graphics::Canvas;
use ggez::input::keyboard::KeyInput;
use ggez::{Context, GameResult};
use hecs::World;
use tokio::sync::mpsc;

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
    /// 更新场景逻辑
    ///
    /// 返回 Some(SceneType) 表示需要切换到新场景
    fn update(
        &mut self,
        ctx: &mut Context,
        world: &mut World,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) -> GameResult<Option<SceneType>>;

    /// 绘制场景
    fn draw(&mut self, ctx: &mut Context, canvas: &mut Canvas, world: &World) -> GameResult;

    /// 鼠标按下事件
    fn on_mouse_down(
        &mut self,
        _ctx: &mut Context,
        _world: &mut World,
        _button: ggez::winit::event::MouseButton,
        _x: f32,
        _y: f32,
        _network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) -> GameResult {
        Ok(())
    }

    /// 鼠标抬起事件
    fn on_mouse_up(
        &mut self,
        _ctx: &mut Context,
        _world: &mut World,
        _button: ggez::winit::event::MouseButton,
        _x: f32,
        _y: f32,
        _network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) -> GameResult {
        Ok(())
    }

    /// 鼠标移动事件
    fn on_mouse_move(
        &mut self,
        _ctx: &mut Context,
        _world: &mut World,
        _x: f32,
        _y: f32,
    ) -> GameResult {
        Ok(())
    }

    
    /// 键盘按下事件
    fn on_key_down(
        &mut self,
        _ctx: &mut Context,
        _world: &mut World,
        _input: KeyInput,
        _network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) -> GameResult<Option<SceneType>> {
        Ok(None)
    }
    
    /// 文本输入事件 (IME)
    fn on_text_input(
        &mut self,
        _ctx: &mut Context,
        _world: &mut World,
        _character: String,
    ) -> GameResult {
        Ok(())
    }
    
    /// 鼠标滚轮事件
    fn on_mouse_wheel(
        &mut self,
        _ctx: &mut Context,
        _world: &mut World,
        _x: f32,
        _y: f32,
    ) -> GameResult {
        Ok(())
    }
    
    /// 窗口大小调整事件
    fn on_resize(
        &mut self,
        _ctx: &mut Context,
        _world: &mut World,
        _width: f32,
        _height: f32,
    ) -> GameResult {
        Ok(())
    }
}
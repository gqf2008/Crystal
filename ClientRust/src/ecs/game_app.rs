// ============================================================================
// 游戏主应用 - 基于 ECS 架构
// ============================================================================
//
// 管理整个游戏的生命周期：
// - 场景切换（登录 → 选择角色 → 游戏）
// - ECS World 管理
// - 网络连接管理
// - 资源加载
//
// ============================================================================

use ggez::event::EventHandler;
use ggez::graphics::{Canvas, Color};
use ggez::GameResult;

use crate::ecs::components::InputEvent;
use crate::ecs::scenes::{GameScene, LoginScene, Scene, SceneType, SelectScene};
use crate::ecs::GameContext;

/// 游戏主应用
pub struct GameState {
    /// 当前场景
    current_scene: Box<dyn Scene>,
    /// 场景类型
    scene_type: SceneType,
}
impl GameState {
    /// 创建新的游戏应用
    ///
    /// # 参数
    /// - `ctx`: ggez 上下文
    /// - `settings`: 客户端配置（由 ClientRuntime 加载）
    pub fn new(_ctx: &mut GameContext) -> GameResult<Self> {
        // 创建初始场景（登录场景）
        let login_scene = LoginScene::new();
        Ok(Self {
            current_scene: Box::new(login_scene),
            scene_type: SceneType::Login,
        })
    }
}

impl GameState {
    /// 切换场景
    pub fn switch_scene(&mut self, ctx: &mut GameContext, scene_type: SceneType) -> GameResult {
        tracing::info!("切换场景: {:?} -> {:?}", self.scene_type, scene_type);
        self.scene_type = scene_type;
        self.current_scene = match scene_type {
            SceneType::Login => {
                // 🧹 返回登录场景时发送断开连接命令
                if let Err(e) = ctx
                    .network()
                    .send(crate::network::handlers::GameEvent::DisconnectRequest)
                {
                    tracing::error!("❌ 发送断开连接命令失败: {:?}", e);
                }
                tracing::info!("🔌 已发送断开连接命令");

                Box::new(LoginScene::new())
            }
            SceneType::Select => {
                // SelectScene 将从 World 查询角色数据 (ECS 架构)
                // 📡 SelectScene直接使用NetContext发送命令，不需要command_sender
                println!("🎭 创建SelectScene (角色数据在 World 中)");
                Box::new(SelectScene::new())
            }
            SceneType::Game => {
                // 🧹 在切换到游戏场景之前,清理旧的游戏对象
                // 这对于切换账号/角色时非常重要,避免旧角色数据残留
                ctx.clear_game_objects();
                // GameScene是纯粹的场景编排器，构造函数不需要参数
                Box::new(GameScene::spawn(ctx))
            }
        };

        Ok(())
    }

    /// 获取当前场景类型
    pub fn current_scene_type(&self) -> SceneType {
        self.scene_type
    }
}

impl EventHandler<GameContext> for GameState {
    fn update(&mut self, ctx: &mut GameContext) -> GameResult {
        ctx.collect_network_events();
        if let Some(next_scene) = self.current_scene.update(ctx)? {
            self.switch_scene(ctx, next_scene)?;
        }
        ctx.clear_frame_events();
        Ok(())
    }

    fn draw(&mut self, ctx: &mut GameContext) -> GameResult {
        let mut canvas = Canvas::from_frame(ctx, Color::BLACK);
        // 绘制当前场景
        self.current_scene.draw(ctx, &mut canvas)?;
        canvas.finish(ctx)?;

        Ok(())
    }

    fn mouse_enter_or_leave(
        &mut self,
        ctx: &mut GameContext,
        entered: bool,
    ) -> Result<(), ggez::GameError> {
        ctx.push_input_event(InputEvent::MouseEnterOrLeave { entered });
        tracing::trace!(
            "🖱️ 鼠标{}",
            if entered {
                "进入窗口"
            } else {
                "离开窗口"
            }
        );
        Ok(())
    }

    fn mouse_wheel_event(
        &mut self,
        ctx: &mut GameContext,
        x: f32,
        y: f32,
    ) -> Result<(), ggez::GameError> {
        ctx.push_input_event(InputEvent::MouseWheel { x, y });
        tracing::trace!("🖱️ 鼠标滚轮: ({:.1}, {:.1})", x, y);
        Ok(())
    }

    fn mouse_motion_event(
        &mut self,
        ctx: &mut GameContext,
        x: f32,
        y: f32,
        dx: f32,
        dy: f32,
    ) -> Result<(), ggez::GameError> {
        ctx.push_input_event(InputEvent::MouseMove { x, y, dx, dy });
        tracing::trace!("🖱️ 鼠标移动: ({:.1}, {:.1}), Δ({:.1}, {:.1})", x, y, dx, dy);
        Ok(())
    }

    fn text_input_event(
        &mut self,
        ctx: &mut GameContext,
        character: char,
    ) -> Result<(), ggez::GameError> {
        ctx.push_input_event(InputEvent::Ime {
            character,
            timestamp: std::time::Instant::now(),
        });
        tracing::trace!("⌨️ 输入字符: '{}'", character);
        Ok(())
    }
}

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

use super::WorldExt; // 用于 network(), spawn_settings() 等方法
use ggez::event::EventHandler;
use ggez::graphics::{Canvas, Color};
use ggez::{Context, GameResult};
use hecs::World;

use crate::ecs::components::InputEvent;
use crate::ecs::scenes::{GameScene, LoginScene, Scene, SceneType, SelectScene};
use crate::settings::ClientSettings;

/// 游戏主应用
pub struct GameState {
    /// ECS World（包含 NetContext, ClientSettings 等单例组件）
    world: World,

    /// 当前场景
    current_scene: Box<dyn Scene>,

    /// 场景类型
    scene_type: SceneType,

    /// 本帧收集的输入事件（在 EventHandler 回调中收集，在 update 中使用）
    frame_input_events: Vec<InputEvent>,
}
impl GameState {
    /// 创建新的游戏应用
    ///
    /// # 参数
    /// - `ctx`: ggez 上下文
    /// - `settings`: 客户端配置（由 ClientRuntime 加载）
    pub fn new(ctx: &mut Context, settings: ClientSettings) -> GameResult<Self> {
        tracing::info!("🎮 游戏应用初始化中...");
        let net_ctx = crate::network::NetworkBuilder::new(settings.network.clone())
            .build()
            .expect("Failed to initialize network");

        let mut world = World::new();
        world.spawn_settings(settings).spawn_network(net_ctx);
        // 创建初始场景（登录场景）
        let login_scene = LoginScene::new();
        Ok(Self {
            world,
            current_scene: Box::new(login_scene),
            scene_type: SceneType::Login,
            frame_input_events: Vec::new(),
        })
    }
}

/// 优雅关闭
impl Drop for GameState {
    fn drop(&mut self) {
        tracing::info!("🛑 Shutting down GameState...");
        if let Err(e) = self
            .world
            .network()
            .send(crate::network::handlers::GameEvent::DisconnectRequest)
        {
            tracing::error!("Failed to send disconnect: {:?}", e);
        }
        tracing::info!("✅ GameState shutdown complete");
    }
}

impl GameState {
    /// 切换场景
    pub fn switch_scene(&mut self, ctx: &mut Context, scene_type: SceneType) -> GameResult {
        tracing::info!("🔄 切换场景: {:?} -> {:?}", self.scene_type, scene_type);
        self.scene_type = scene_type;
        self.current_scene = match scene_type {
            SceneType::Login => {
                // 🧹 返回登录场景时发送断开连接命令
                if let Err(e) = self
                    .world
                    .network()
                    .send(crate::network::handlers::GameEvent::DisconnectRequest)
                {
                    tracing::error!("❌ 发送断开连接命令失败: {}", e);
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
                self.clear_game_objects();
                // GameScene是纯粹的场景编排器，构造函数不需要参数
                Box::new(GameScene::spawn(ctx, &mut self.world))
            }
        };

        Ok(())
    }

    /// 清理所有游戏对象实体
    ///
    /// 在切换到游戏场景之前调用,确保旧角色数据不会残留
    /// 清理的对象包括: 玩家、怪物、NPC、物品掉落、地图瓦片等
    #[inline]
    fn clear_game_objects(&mut self) {
        use crate::ecs::components::*;

        println!("🧹 开始清理旧游戏对象...");

        let mut to_despawn = Vec::new();

        // 1. 清理玩家实体 (包括本地玩家和其他玩家)
        for (entity, _) in self.world.query::<&PlayerData>().iter() {
            to_despawn.push(entity);
        }

        // 2. 清理怪物实体
        for (entity, _) in self.world.query::<&MonsterData>().iter() {
            to_despawn.push(entity);
        }

        // 3. 清理NPC实体
        for (entity, _) in self.world.query::<&NPCData>().iter() {
            to_despawn.push(entity);
        }

        // 4. 清理物品掉落实体
        for (entity, _) in self.world.query::<&ItemDrop>().iter() {
            to_despawn.push(entity);
        }

        // 5. 清理地图瓦片实体
        for (entity, _) in self.world.query::<&MapTile>().iter() {
            to_despawn.push(entity);
        }

        // 6. 清理地图数据实体
        for (entity, _) in self.world.query::<&MapData>().iter() {
            to_despawn.push(entity);
        }

        // 7. 清理动画瓦片实体
        for (entity, _) in self.world.query::<&AnimatedTile>().iter() {
            to_despawn.push(entity);
        }

        // 8. 清理门实体
        for (entity, _) in self.world.query::<&Door>().iter() {
            to_despawn.push(entity);
        }

        // 删除所有收集的实体
        let count = to_despawn.len();
        for entity in to_despawn {
            if let Err(e) = self.world.despawn(entity) {
                println!("⚠️ 删除实体失败: {:?}", e);
            }
        }

        println!("✅ 已清理 {} 个游戏对象和地图实体", count);
    }

    /// 获取当前场景类型
    pub fn current_scene_type(&self) -> SceneType {
        self.scene_type
    }
}

impl EventHandler for GameState {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        // 创建 GameContext 并传入本帧收集的输入事件
        let mut game_ctx = crate::ecs::GameContext::new(ctx, &mut self.world);
        game_ctx.input_events = std::mem::take(&mut self.frame_input_events);

        // 🎮 更新当前场景（Scene 使用 GameContext 访问所有状态）
        if let Some(next_scene) = self.current_scene.update(&mut game_ctx)? {
            // 场景请求切换
            self.switch_scene(ctx, next_scene)?;
        }

        // ✅ 清空输入事件，为下一帧准备
        self.frame_input_events.clear();
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        let mut canvas = Canvas::from_frame(ctx, Color::BLACK);
        let mut ctx = crate::ecs::GameContext::new(ctx, &mut self.world);
        // 绘制当前场景
        self.current_scene.draw(&mut ctx, &mut canvas)?;
        canvas.finish(ctx.ctx)?;

        Ok(())
    }

    fn mouse_enter_or_leave(
        &mut self,
        _ctx: &mut Context,
        entered: bool,
    ) -> Result<(), ggez::GameError> {
        self.frame_input_events
            .push(InputEvent::MouseEnterOrLeave { entered });
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
        _ctx: &mut Context,
        x: f32,
        y: f32,
    ) -> Result<(), ggez::GameError> {
        self.frame_input_events
            .push(InputEvent::MouseWheel { x, y });
        tracing::trace!("🖱️ 鼠标滚轮: ({:.1}, {:.1})", x, y);
        Ok(())
    }

    fn mouse_motion_event(
        &mut self,
        _ctx: &mut Context,
        x: f32,
        y: f32,
        dx: f32,
        dy: f32,
    ) -> Result<(), ggez::GameError> {
        self.frame_input_events
            .push(InputEvent::MouseMove { x, y, dx, dy });
        Ok(())
    }

    fn text_input_event(&mut self, _ctx: &mut Context, character: char) -> Result<(), ggez::GameError> {
        self.frame_input_events
            .push(InputEvent::Ime { character, timestamp: std::time::Instant::now() });
        Ok(())
    }
}

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
use ggez::{Context, GameResult};
use hecs::World;
use std::sync::Arc;

use crate::ecs::scenes::{GameScene, LoginScene, Scene, SceneType, SelectScene};
use crate::network::NetContext;
use crate::settings::ClientSettings;

/// 游戏主应用
pub struct GameState {
    /// ECS World
    world: World,

    /// 当前场景
    current_scene: Box<dyn Scene>,

    /// 场景类型
    scene_type: SceneType,

    /// 网络上下文（✨ 唯一的网络接口）
    net_ctx: Arc<crate::network::NetContext>,
}
impl GameState {
    /// 创建新的游戏应用
    ///
    /// # 参数
    /// - `ctx`: ggez 上下文
    /// - `settings`: 客户端配置（由 ClientRuntime 加载）
    pub fn new(_ctx: &mut Context, settings: ClientSettings) -> GameResult<Self> {
        println!("🎮 游戏应用初始化中...");
        // ✨ 使用 Builder 模式创建网络模块（隐藏所有内部细节）
        let net_ctx = crate::network::NetworkBuilder::new(settings.network)
            .build()
            .expect("Failed to initialize network");

        let net_ctx = Arc::new(net_ctx);

        // 创建 ECS World
        let world = World::new();

        // 创建初始场景（登录场景）
        let login_scene = LoginScene::new();

        Ok(Self {
            world,
            current_scene: Box::new(login_scene),
            scene_type: SceneType::Login,
            net_ctx,
        })
    }

    /// 获取网络上下文引用（供场景使用）
    #[inline]
    pub fn net_ctx(&self) -> Arc<NetContext> {
        self.net_ctx.clone()
    }
}

/// 优雅关闭
impl Drop for GameState {
    fn drop(&mut self) {
        tracing::info!("🛑 Shutting down GameState...");

        // 发送断开连接请求
        if let Err(e) = self
            .net_ctx
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
        println!("🔄 切换场景: {:?} -> {:?}", self.scene_type, scene_type);

        self.scene_type = scene_type;

        self.current_scene = match scene_type {
            SceneType::Login => {
                // 🧹 返回登录场景时发送断开连接命令
                if let Err(e) = self
                    .net_ctx
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
                Box::new(GameScene::new())
            }
        };

        Ok(())
    }

    /// 清理所有游戏对象实体
    ///
    /// 在切换到游戏场景之前调用,确保旧角色数据不会残留
    /// 清理的对象包括: 玩家、怪物、NPC、物品掉落、地图瓦片等
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
        // 更新当前场景（Scene 自己会处理需要的网络事件）
        if let Some(next_scene) = self
            .current_scene
            .update(ctx, &mut self.world, &self.net_ctx)?
        {
            // 场景请求切换
            self.switch_scene(ctx, next_scene)?;
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        let mut canvas = Canvas::from_frame(ctx, Color::BLACK);
        // 绘制当前场景
        self.current_scene.draw(ctx, &mut canvas, &self.world)?;

        canvas.finish(ctx)?;
        Ok(())
    }

    fn mouse_button_down_event(
        &mut self,
        ctx: &mut Context,
        button: ggez::winit::event::MouseButton,
        x: f32,
        y: f32,
    ) -> GameResult {
        self.current_scene
            .on_mouse_down(ctx, &mut self.world, button, x, y, &self.net_ctx)
    }

    fn mouse_button_up_event(
        &mut self,
        ctx: &mut Context,
        button: ggez::winit::event::MouseButton,
        x: f32,
        y: f32,
    ) -> GameResult {
        self.current_scene
            .on_mouse_up(ctx, &mut self.world, button, x, y, &self.net_ctx)
    }

    fn mouse_motion_event(
        &mut self,
        ctx: &mut Context,
        x: f32,
        y: f32,
        _dx: f32,
        _dy: f32,
    ) -> GameResult {
        self.current_scene.on_mouse_move(ctx, &mut self.world, x, y)
    }

    fn key_down_event(
        &mut self,
        ctx: &mut Context,
        input: ggez::input::keyboard::KeyInput,
        _repeated: bool,
    ) -> GameResult {
        // 处理场景切换
        if let Some(new_scene_type) =
            self.current_scene
                .on_key_down(ctx, &mut self.world, input, &self.net_ctx)?
        {
            self.switch_scene(ctx, new_scene_type)?;
        }
        Ok(())
    }

    fn mouse_wheel_event(&mut self, ctx: &mut Context, x: f32, y: f32) -> GameResult {
        self.current_scene
            .on_mouse_wheel(ctx, &mut self.world, x, y)
    }

    fn text_input_event(&mut self, ctx: &mut Context, character: char) -> GameResult {
        // 转发 IME 输入到当前场景
        // 将 char 转换为 String
        tracing::debug!("🔥 GameState::text_input_event 被调用: '{}'", character);
        self.current_scene
            .on_text_input(ctx, &mut self.world, character.to_string())
    }

    fn resize_event(&mut self, ctx: &mut Context, width: f32, height: f32) -> GameResult {
        // 在高 DPI 显示器上，ggez 传递的是物理像素，需要转换为逻辑像素
        let scale_factor = ctx.gfx.window().scale_factor() as f32;
        let logical_width = width / scale_factor;
        let logical_height = height / scale_factor;
        self.current_scene
            .on_resize(ctx, &mut self.world, logical_width, logical_height)
    }
}

// ============================================================================
// NetEventListener 实现
// ============================================================================
// 注意：NetEventListener 实现已移除
// 新架构中，NetworkEventSystem 直接处理 GameEvent 并更新 ECS 组件
// ============================================================================

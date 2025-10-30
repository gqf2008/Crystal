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

use ggez::{Context, GameResult};
use ggez::event::EventHandler;
use ggez::graphics::{Canvas, Color};
use hecs::World;
use std::sync::Arc;
use std::thread;

use crate::ecs::scenes::{Scene, SceneType, LoginScene, SelectScene, GameScene};
use crate::network::NetContext;
use crate::settings::ClientSettings;
use mir2_shared::packets::CharacterSummary;

/// 游戏主应用
pub struct GameState {
    /// ECS World
    world: World,
    
    /// 当前场景
    current_scene: Box<dyn Scene>,
    
    /// 场景类型
    scene_type: SceneType,
    
    /// 网络线程句柄（🔄 改用 std::thread）
    network_thread: Option<thread::JoinHandle<()>>,
    
    /// 网络上下文（✨ 唯一的网络接口）
    net_ctx: Arc<crate::network::NetContext>,
    
    /// 客户端设置
    settings: Arc<parking_lot::RwLock<ClientSettings>>,
    
    /// 场景间临时数据 - 角色列表（LoginScene → SelectScene）
    pending_characters: Option<Vec<CharacterSummary>>,
    
    /// 场景间临时数据 - 选中的角色索引（SelectScene → GameScene）
    selected_character_index: Option<i32>,
    
    /// 🆕 网络事件系统（处理 GameEvent 并更新 ECS 组件）
    network_event_system: crate::ecs::systems::update::NetworkEventSystem,
}
impl GameState {
    /// 创建新的游戏应用
    /// 
    /// # 参数
    /// - `ctx`: ggez 上下文
    /// - `settings`: 客户端配置（由 ClientRuntime 加载）
    pub fn new(
        _ctx: &mut Context,
        settings: ClientSettings,
    ) -> GameResult<Self> {
        println!("🎮 游戏应用初始化中...");
        
        let settings_arc = Arc::new(parking_lot::RwLock::new(settings));
        
        // ✨ 使用 Builder 模式创建网络模块（隐藏所有内部细节）
        let net_ctx = crate::network::NetworkBuilder::new(settings_arc.clone())
            .build()
            .expect("Failed to initialize network");
        let net_ctx = Arc::new(net_ctx);
        
        // 🆕 创建网络事件系统（处理 GameEvent 并更新 ECS 组件）
        let network_event_system = crate::ecs::systems::update::NetworkEventSystem::new(net_ctx.clone());
        
        println!("✅ 网络系统初始化完成");
        
        // 创建 ECS World
        let world = World::new();
        
        // 创建初始场景（登录场景）
        let login_scene = LoginScene::new();
        
        Ok(Self {
            world,
            current_scene: Box::new(login_scene),
            scene_type: SceneType::Login,
            network_thread: None,
            net_ctx,
            settings: settings_arc,
            pending_characters: None,
            selected_character_index: None,
            network_event_system,
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
        if let Err(e) = self.net_ctx.send(crate::network::handlers::GameEvent::DisconnectRequest) {
            tracing::error!("Failed to send disconnect: {:?}", e);
        }
        
        tracing::info!("✅ GameState shutdown complete");
    }
}

impl GameState {
    
    // 注意：process_network_events() 方法已移除
    // 新架构中，NetworkEventSystem 直接处理网络事件并更新 ECS 组件
    // 场景切换相关的事件应通过 ECS 的全局资源或事件队列处理
    
    /// 切换场景
    pub fn switch_scene(&mut self, ctx: &mut Context, scene_type: SceneType) -> GameResult {
        println!("🔄 切换场景: {:?} -> {:?}", self.scene_type, scene_type);
        
        self.scene_type = scene_type;
        
        self.current_scene = match scene_type {
            SceneType::Login => {
                // 🧹 返回登录场景时发送断开连接命令
                if let Err(e) = self.command_tx.send(crate::network::NetworkCommand::Disconnect) {
                    tracing::error!("❌ 发送断开连接命令失败: {}", e);
                }
                tracing::info!("🔌 已发送断开连接命令");
                
                Box::new(LoginScene::new())
            },
            SceneType::Select => {
                // 从LoginScene传递角色列表
                let characters = self.pending_characters.take().unwrap_or_else(Vec::new);
                println!("🎭 创建SelectScene，角色数: {}", characters.len());
                let mut scene = SelectScene::new(characters);
                // 🆕 设置网络命令发送器
                scene.set_command_sender(self.command_tx.clone());
                Box::new(scene)
            },
            SceneType::Game => {
                // 🧹 在切换到游戏场景之前,清理旧的游戏对象
                // 这对于切换账号/角色时非常重要,避免旧角色数据残留
                self.clear_game_objects();
                
                // 🆕 新架构：GameClient 不再存储状态
                // 玩家位置、地图信息将通过 GameEvent (MapInformation, UserLocation) 获取
                // GameEventSystem 会创建对应的 ECS 实体
                println!("⏳ 等待服务器发送地图信息和玩家位置...");
                
                // GameScene 使用固定的 UI 设计分辨率 1024×768
                // 不需要传递配置的窗口分辨率
                Box::new(GameScene::new(ctx, &mut self.world, None, None)?)
            },
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
        // 🆕 运行网络事件系统（处理 GameEvent 并更新 ECS 组件）
        // 创建临时的 GameWorld 来访问 world
        let mut game_world = crate::ecs::world::GameWorld {
            world: std::mem::take(&mut self.world),
            start_time: std::time::Instant::now(),
        };
        
        self.network_event_system.update(&mut game_world);
        
        // 归还 world
        self.world = game_world.world;
        
        // 处理网络事件并分发到场景（可能触发场景切换）
        if let Some(next_scene) = self.process_network_events(ctx)? {
            self.switch_scene(ctx, next_scene)?;
        }
        
        // 更新当前场景
        if let Some(next_scene) = self.current_scene.update(ctx, &mut self.world, &self.net_ctx)? {
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
        self.current_scene.on_mouse_down(ctx, &mut self.world, button, x, y, &self.net_ctx)
    }
    
    fn mouse_button_up_event(
        &mut self,
        ctx: &mut Context,
        button: ggez::winit::event::MouseButton,
        x: f32,
        y: f32,
    ) -> GameResult {
        self.current_scene.on_mouse_up(ctx, &mut self.world, button, x, y, &self.net_ctx)
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
        if let Some(new_scene_type) = self.current_scene.on_key_down(ctx, &mut self.world, input, &self.net_ctx)? {
            self.switch_scene(ctx, new_scene_type)?;
        }
        Ok(())
    }
    
    fn mouse_wheel_event(&mut self, ctx: &mut Context, x: f32, y: f32) -> GameResult {
        self.current_scene.on_mouse_wheel(ctx, &mut self.world, x, y)
    }
    
    fn text_input_event(&mut self, ctx: &mut Context, character: char) -> GameResult {
        // 转发 IME 输入到当前场景
        // 将 char 转换为 String
        tracing::debug!("🔥 GameState::text_input_event 被调用: '{}'", character);
        self.current_scene.on_text_input(ctx, &mut self.world, character.to_string())
    }
    
    fn resize_event(&mut self, ctx: &mut Context, width: f32, height: f32) -> GameResult {
        // 在高 DPI 显示器上，ggez 传递的是物理像素，需要转换为逻辑像素
        let scale_factor = ctx.gfx.window().scale_factor() as f32;
        let logical_width = width / scale_factor;
        let logical_height = height / scale_factor;
        self.current_scene.on_resize(ctx, &mut self.world, logical_width, logical_height)
    }
}

// ============================================================================
// NetEventListener 实现
// ============================================================================
// 注意：NetEventListener 实现已移除
// 新架构中，NetworkEventSystem 直接处理 GameEvent 并更新 ECS 组件
// ============================================================================

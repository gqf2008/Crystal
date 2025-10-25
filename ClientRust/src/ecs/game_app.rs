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
use tokio::sync::mpsc;

use crate::ecs::scenes::{Scene, SceneType, LoginScene, SelectScene, GameScene};
use crate::network::{NetworkManager, GameEvent, NetworkCommand, GameObject};
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
    
    /// 网络管理器（可选，使用 tokio::sync::RwLock 以支持跨 await 的 Send）
    network_manager: Option<Arc<tokio::sync::RwLock<NetworkManager>>>,
    
    /// 事件接收器（从网络线程接收）
    event_rx: mpsc::UnboundedReceiver<GameEvent>,
    
    /// 命令发送器（发送到网络线程）
    command_tx: mpsc::UnboundedSender<NetworkCommand>,
    
    /// 客户端设置
    settings: Arc<parking_lot::RwLock<ClientSettings>>,
    
    /// 场景间临时数据 - 角色列表（LoginScene → SelectScene）
    pending_characters: Option<Vec<CharacterSummary>>,
    
    /// 场景间临时数据 - 选中的角色索引（SelectScene → GameScene）
    selected_character_index: Option<i32>,
}
impl GameState {
    /// 创建新的游戏应用
    /// 
    /// # 参数
    /// - `ctx`: ggez 上下文
    /// - `settings`: 客户端配置（由 ClientRuntime 加载）
    pub fn new(
        ctx: &mut Context,
        settings: ClientSettings,
    ) -> GameResult<Self> {
        println!("🎮 游戏应用初始化中...");
        
        let settings_arc = Arc::new(parking_lot::RwLock::new(settings));
        
        // 创建事件和命令通道
        let (event_tx, event_rx) = mpsc::unbounded_channel::<GameEvent>();
        let (command_tx, command_rx) = mpsc::unbounded_channel::<NetworkCommand>();
        
        // 创建网络管理器
        let network_manager = NetworkManager::new(
            settings_arc.clone(),
            event_tx.clone(),
            command_rx,
        );
        
        let network_manager_arc = Arc::new(tokio::sync::RwLock::new(network_manager));
        
        // 启动网络任务（使用 tokio::spawn，类似 CrystalGame）
        {
            let nm_clone = network_manager_arc.clone();
            tokio::spawn(async move {
                tracing::info!("🌐 网络任务启动");
                
                // 自动连接到服务器
                {
                    let mut nm = nm_clone.write().await;
                    if let Err(e) = nm.connect().await {
                        tracing::error!("❌ 初始连接失败: {}", e);
                    } else {
                        tracing::info!("✅ 已连接到服务器");
                    }
                }
                
                // 进入网络处理循环
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_millis(16)).await;
                    
                    let mut nm = nm_clone.write().await;
                    if let Err(e) = nm.process().await {
                        tracing::error!("⚠️ 网络处理错误: {}", e);
                        // 发生错误后等待一段时间再重试
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    }
                }
            });
        }
        
        println!("✅ 网络系统初始化完成");
        
        // 创建 ECS World
        let world = World::new();
        
        // 创建初始场景（登录场景）
        let login_scene = LoginScene::new();
        
        Ok(Self {
            world,
            current_scene: Box::new(login_scene),
            scene_type: SceneType::Login,
            network_manager: Some(network_manager_arc),
            event_rx,
            command_tx,
            settings: settings_arc,
            pending_characters: None,
            selected_character_index: None,
        })
    }
    
    /// 发送网络命令
    pub fn send_command(&self, command: NetworkCommand) {
        if let Err(e) = self.command_tx.send(command) {
            eprintln!("❌ 发送网络命令失败: {}", e);
        }
    }
    
    /// 处理网络事件并分发到当前场景
    fn process_network_events(&mut self, ctx: &mut Context) -> GameResult<Option<SceneType>> {
        let mut next_scene = None;
        let mut events_to_process = Vec::new();
        
        // 收集所有待处理的网络事件
        while let Ok(event) = self.event_rx.try_recv() {
            events_to_process.push(event);
        }
        
        // 分发事件到当前场景
        for event in events_to_process {
            // 先处理全局事件（连接/断开）
            match &event {
                GameEvent::Connected => {
                    println!("✅ 已连接到服务器");
                }
                GameEvent::Disconnected { reason } => {
                    println!("⚠️ 与服务器断开连接: {}", reason);
                    // 断开连接时返回登录场景
                    if self.scene_type != SceneType::Login {
                        next_scene = Some(SceneType::Login);
                    }
                }
                _ => {}
            }
            
            // 将事件分发到当前场景
            match self.scene_type {
                SceneType::Login => {
                    // LoginScene特殊处理：登录成功需要切换场景并传递角色列表
                    if let GameEvent::LoginSuccess { ref characters } = event {
                        println!("✅ 登录成功！收到 {} 个角色", characters.len());
                        // 保存角色列表，准备切换到SelectScene
                        self.pending_characters = Some(characters.clone());
                        next_scene = Some(SceneType::Select);
                    }
                    
                    // 将事件传递给LoginScene处理
                    if let Some(login_scene) = self.current_scene.as_mut().as_any_mut().downcast_mut::<LoginScene>() {
                        login_scene.handle_network_event(&event);
                    }
                }
                SceneType::Select => {
                    // SelectScene特殊处理：开始游戏成功
                    if let GameEvent::StartGameResponse { result } = event {
                        if result == 0 {
                            println!("🎮 开始游戏成功");
                            // 获取SelectScene中选中的角色索引
                            if let Some(select_scene) = self.current_scene.as_mut().as_any_mut().downcast_mut::<SelectScene>() {
                                self.selected_character_index = Some(select_scene.get_selected_character_index());
                            }
                            next_scene = Some(SceneType::Game);
                        }
                    }
                    
                    // 将事件传递给SelectScene处理
                    if let Some(select_scene) = self.current_scene.as_mut().as_any_mut().downcast_mut::<SelectScene>() {
                        select_scene.handle_network_event(&event);
                    }
                }
                SceneType::Game => {
                    // 将事件传递给GameScene处理
                    if let Some(game_scene) = self.current_scene.as_mut().as_any_mut().downcast_mut::<GameScene>() {
                        let event_desc = match &event {
                            GameEvent::PlayerMoved { location } => format!("PlayerMoved({}, {})", location.x, location.y),
                            GameEvent::MapInformation { map_index, file_name, .. } => format!("MapInformation({}:{})", map_index, file_name),
                            GameEvent::UserInformation { .. } => "UserInformation".to_string(),
                            GameEvent::ObjectSpawned { object } => match object {
                                GameObject::Player { id, name, .. } => format!("ObjectSpawned::Player({}:{})", id, name),
                                GameObject::Monster { id, name, .. } => format!("ObjectSpawned::Monster({}:{})", id, name),
                                GameObject::Npc { id, name, .. } => format!("ObjectSpawned::Npc({}:{})", id, name),
                                GameObject::Item { id, .. } => format!("ObjectSpawned::Item({})", id),
                            },
                            _ => "其他事件".to_string(),
                        };
                        tracing::info!("🎮 GameApp: 传递事件到GameScene: {}", event_desc);
                        game_scene.handle_network_event(&mut self.world, &event);
                    }
                }
            }
        }
        
        Ok(next_scene)
    }
    
    /// 切换场景
    pub fn switch_scene(&mut self, ctx: &mut Context, scene_type: SceneType) -> GameResult {
        println!("🔄 切换场景: {:?} -> {:?}", self.scene_type, scene_type);
        
        self.scene_type = scene_type;
        
        self.current_scene = match scene_type {
            SceneType::Login => {
                // 🧹 返回登录场景时:
                // 1. 先断开网络连接(服务器端清理会话)
                // 2. 再重置GameClient状态(客户端清理数据)
                // 这对于重新登录非常重要,避免旧账号数据干扰
                
                // 1️⃣ 发送断开连接命令
                if let Err(e) = self.command_tx.send(crate::network::NetworkCommand::Disconnect) {
                    tracing::error!("❌ 发送断开连接命令失败: {}", e);
                }
                tracing::info!("🔌 已发送断开连接命令");
                
                // 2️⃣ 重置GameClient状态
                if let Some(network_manager) = &self.network_manager {
                    let nm = network_manager.blocking_read();
                    let gc = nm.game_client();
                    let mut client = gc.write();
                    client.reset_to_login();
                }
                
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
                
                // 从网络客户端获取玩家初始位置和地图信息
                let (player_location, map_file_name) = if let Some(network_manager) = &self.network_manager {
                    let nm = network_manager.blocking_read();
                    let gc = nm.game_client();
                    let client = gc.read();
                    let location = client.player.as_ref().map(|p| (p.location.x, p.location.y));
                    let map_name = client.map_info.as_ref().map(|m| m.file_name.clone());
                    (location, map_name)
                } else {
                    (None, None)
                };
                
                if let Some((x, y)) = player_location {
                    println!("✅ 从服务器获取玩家位置: ({}, {})", x, y);
                } else {
                    println!("⚠️ 未找到玩家位置，使用默认值(0, 0)");
                }
                
                if let Some(ref map_name) = map_file_name {
                    println!("✅ 从服务器获取地图名称: {}", map_name);
                } else {
                    println!("⚠️ 未找到地图信息，使用默认地图 '0'");
                }
                
                // GameScene 使用固定的 UI 设计分辨率 1024×768
                // 不需要传递配置的窗口分辨率
                Box::new(GameScene::new(ctx, &mut self.world, player_location, map_file_name)?)
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
        // 处理网络事件并分发到场景（可能触发场景切换）
        if let Some(next_scene) = self.process_network_events(ctx)? {
            self.switch_scene(ctx, next_scene)?;
        }
        
        // 更新当前场景
        if let Some(next_scene) = self.current_scene.update(ctx, &mut self.world, &self.command_tx)? {
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
        self.current_scene.on_mouse_down(ctx, &mut self.world, button, x, y, &self.command_tx)
    }
    
    fn mouse_button_up_event(
        &mut self,
        ctx: &mut Context,
        button: ggez::winit::event::MouseButton,
        x: f32,
        y: f32,
    ) -> GameResult {
        self.current_scene.on_mouse_up(ctx, &mut self.world, button, x, y, &self.command_tx)
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
        if let Some(new_scene_type) = self.current_scene.on_key_down(ctx, &mut self.world, input, &self.command_tx)? {
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

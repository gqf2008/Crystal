use anyhow::{Context, Result};
use tokio::runtime::{Builder, Runtime};
use std::sync::Arc;
use parking_lot::RwLock;

use crate::settings::ClientSettings;
use crate::version;
use crate::graphics::{self, GgezManager};
use crate::scenes::{Scene, SceneManager, SceneType, LoginScene, SelectScene};

// TODO: Implement these modules
// use crate::key_bind_settings::KeyBindSettings;  // TODO: 需要实现
// use crate::audio;  // Audio engine - not yet implemented
// use crate::ui;     // UI layer - not yet implemented
use crate::network as net;  // Use network module as 'net'

pub struct ClientRuntime {
    pub settings: ClientSettings,
    // pub keybinds: KeyBindSettings,  // TODO: 需要实现
    pub tokio: Runtime,
}

impl ClientRuntime {
    /// 初始化日志系统
    pub fn init_logging(log_level: &str) {
        tracing_subscriber::fmt()
            .with_env_filter(log_level)
            .with_target(false)
            .init();
    }

    /// 加载客户端配置
    pub fn load_config(use_test_config: bool) -> Result<ClientSettings> {
        let settings =
            ClientSettings::load(use_test_config, None).context("loading client settings")?;
        Ok(settings)
    }

    /// 创建 Tokio runtime
    pub fn create_tokio_runtime() -> Result<Runtime> {
        Builder::new_multi_thread()
            .enable_all()
            .thread_name("mir2-client")
            .build()
            .context("building tokio runtime")
    }

    /// 初始化图像库系统（包括 MapLibs）
    pub fn init_graphics_libraries(data_path: &str) -> Result<()> {
        tracing::info!("=== 初始化图像库系统 ===");
        graphics::initialize_all_libraries(data_path)
            .context("initializing graphics libraries")?;
        tracing::info!("✅ 图像库初始化完成");
        Ok(())
    }

    /// 加载核心图形库（Data.lib, Prguse.lib等）
    pub fn load_core_libraries() -> Result<()> {
        tracing::info!("📦 正在加载核心图形库...");
        graphics::libraries::load_core_libraries()
            .context("loading core libraries")?;
        tracing::info!("✅ 核心图形库加载成功");
        Ok(())
    }

    /// 完整的 bootstrap 流程（原有方法，用于传统启动）
    pub fn bootstrap(use_test_config: bool) -> Result<()> {
        Self::init_logging("info");

        let settings = Self::load_config(use_test_config)?;
        let tokio = Self::create_tokio_runtime()?;

        let runtime = Self {
            settings,
            tokio,
        };

        runtime.run()
    }

    /// 创建 ClientRuntime 实例（供 ggez 等新架构使用）
    pub fn new(use_test_config: bool) -> Result<Self> {
        let settings = Self::load_config(use_test_config)?;
        let tokio = Self::create_tokio_runtime()?;

        Ok(Self {
            settings,
            tokio,
        })
    }

    fn run(self) -> Result<()> {
        let Self {
            settings,
            tokio,
        } = self;

        tokio.block_on(async move {
            // TODO: Initialize audio engine (not yet implemented)
            // let audio = audio::AudioEngine::new(&settings.sound).context("initializing audio")?;
            
            let mut net = net::NetworkStack::new(&settings.network);
            net.connect(&settings.network)
                .await
                .context("initializing network")?;

            let _version_hash = match version::client_binary_hash() {
                Ok(hash) => {
                    tracing::info!(
                        hash = %version::hash_to_hex(&hash),
                        "computed client version hash"
                    );
                    hash
                }
                Err(err) => {
                    tracing::warn!(
                        error = %err,
                        "failed to compute client version hash, falling back to empty hash"
                    );
                    Vec::new()
                }
            };

            // TODO: Launch UI (Forms-based windows)
            // let launch_result = crate::ui::launch(&settings)
            //     .await
            //     .context("running ui")?;
            
            // Save settings
            settings.save().context("saving settings")?;
            
            tracing::info!("Client completed");
            
            Ok(())
        })
    }
}

/// 游戏主状态 - 实现 ggez::EventHandler
/// 
/// 对应 C# 原版: Client/Forms/CMain.cs
pub struct CrystalGame {
    pub settings: ClientSettings,
    pub ggez_manager: GgezManager,
    pub scene_manager: Arc<RwLock<SceneManager>>,
    pub last_update_time: std::time::Instant,
    pub scale_factor: f32,  // 窗口缩放因子
    
    // 网络系统
    pub game_client: crate::network::game_client::SharedGameClient,
    pub event_rx: tokio::sync::mpsc::UnboundedReceiver<crate::network::game_client::GameEvent>,
    pub command_tx: tokio::sync::mpsc::UnboundedSender<crate::network::NetworkCommand>,
    pub network_task: Option<tokio::task::JoinHandle<()>>,
    
    // 缓存的 MapInformation 事件(用于场景切换时不丢失)
    pub cached_map_info: Option<(i32, String, String)>, // (map_index, file_name, title)
    
    // 缓存的 UserInformation 事件(用于场景切换时不丢失) - ⭐ 关键修复!
    pub cached_user_info: Option<Box<mir2_shared::packets::server::UserInformation>>,
    
    // Tokio runtime (保持网络任务运行)
    #[allow(dead_code)]
    pub runtime: tokio::runtime::Runtime,
}

impl CrystalGame {
    pub fn new(settings: ClientSettings, runtime: tokio::runtime::Runtime) -> Result<Self> {
        // 确保runtime可用
        let _guard = runtime.enter();
        println!("\n========================================");
        println!("🎮 Crystal Mir2 Client - Ggez版本");
        println!("========================================\n");
        tracing::info!("初始化游戏状态...");
        
        // 创建网络系统
        println!("🌐 初始化网络系统...");
        let game_client = crate::network::game_client::new_shared_client();
        let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel();
        let (command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel();
        
        // 启动网络任务 (Tokio runtime)
        println!("🚀 启动网络管理器...");
        let settings_arc = Arc::new(RwLock::new(settings.clone()));
        let gc = game_client.clone();
        
        let network_task = Some(tokio::spawn(async move {
            // 设置事件通道 (tokio::sync::RwLock 是异步的,需要await)
            gc.write().await.set_event_channel(event_tx.clone());
            
            use crate::network::NetworkManager;
            let mut network_manager = NetworkManager::new(settings_arc, event_tx, command_rx);
            
            // 自动连接到服务器
            tracing::info!("🔌 正在连接服务器...");
            if let Err(e) = network_manager.connect().await {
                tracing::error!("❌ 连接服务器失败: {}", e);
            } else {
                tracing::info!("✅ 已连接到服务器");
            }
            
            // 进入消息循环 (不返回Result)
            crate::network::network_task(network_manager).await;
        }));
        
        println!("✓ 网络系统初始化完成");
        
        // 创建渲染管理器
        let res = settings.resolution();
        let screen_width = res.width as f32;
        let screen_height = res.height as f32;
        let ggez_manager = GgezManager::new(screen_width, screen_height);
        println!("✓ Ggez 渲染管理器已创建: {}x{}", screen_width, screen_height);
        
        // 加载核心图形库 (Data.lib, Prguse.lib 等)
        println!("📦 正在加载图形库...");
        tracing::info!("加载图形库...");
        if let Err(e) = graphics::libraries::load_core_libraries() {
            println!("⚠ 加载图形库失败: {}", e);
            tracing::warn!("加载图形库失败: {}, 将在需要时按需加载", e);
        } else {
            println!("✓ 所有图形库加载成功!");
        }
        
        // 注意: 地图库 (MapLibs) 已在 initialize_all_libraries() 中加载
        
        // 创建场景管理器
        let mut scene_manager = SceneManager::new();
        
        // 根据启动模式选择初始场景
        let initial_scene = if settings.launcher.enabled {
            tracing::info!("启动器模式: 显示 LauncherWindow");
            // TODO: 创建 LauncherScene
            SceneType::Login  // 暂时使用 LoginScene
        } else {
            tracing::info!("直接登录模式: 显示 LoginScene");
            SceneType::Login
        };
        
        scene_manager.switch_scene(initial_scene)?;
        
        let scene_manager = Arc::new(RwLock::new(scene_manager));
        
        // 将 GameClient 和 CommandTx 传递给场景
        {
            let mut mgr = scene_manager.write();
            mgr.set_game_client(game_client.clone());
            mgr.set_command_sender(command_tx.clone());
        }
        
        tracing::info!("游戏初始化完成");
        println!("\n✅ 所有系统启动完毕!\n");
        
        Ok(Self {
            settings,
            ggez_manager,
            scene_manager,
            last_update_time: std::time::Instant::now(),
            scale_factor: 1.0,  // 暂时不缩放
            game_client,
            event_rx,
            command_tx,
            network_task,
            cached_map_info: None,  // 初始化为 None
            cached_user_info: None, // 初始化为 None
            runtime,
        })
    }
    
    /// 处理网络事件
    pub fn process_network_events(&mut self) {
        while let Ok(event) = self.event_rx.try_recv() {
            // 🐛 DEBUG: 强制打印所有网络事件到控制台 (使用 println! 确保一定能看到)
            println!("╔════════════════════════════════════════════════════════════════");
            println!("║ 📦 收到服务器数据包!");
            println!("╚════════════════════════════════════════════════════════════════");
            
            match &event {
                crate::network::game_client::GameEvent::Connected => {
                    println!("📡 事件类型: Connected (连接成功)");
                    tracing::info!("📡 Connected");
                }
                crate::network::game_client::GameEvent::Disconnected { reason } => {
                    println!("❌ 事件类型: Disconnected (断开连接)");
                    println!("   原因: {}", reason);
                    tracing::info!("❌ Disconnected: {}", reason);
                }
                crate::network::game_client::GameEvent::LoginSuccess { characters } => {
                    println!("🎉 事件类型: LoginSuccess (登录成功)");
                    println!("   角色数量: {}", characters.len());
                    for (i, ch) in characters.iter().enumerate() {
                        println!("   - 角色{}: {} (等级{})", i+1, ch.name, ch.level);
                    }
                    tracing::info!("🎉 LoginSuccess, 角色数量: {}", characters.len());
                }
                crate::network::game_client::GameEvent::MapInformation { map_index, ref file_name, ref title } => {
                    println!("🗺️  事件类型: MapInformation (地图信息)");
                    println!("   地图索引: {}", map_index);
                    println!("   地图文件: {}", file_name);
                    println!("   地图名称: {}", title);
                    tracing::info!("🗺️  MapInformation: {} ({})", title, file_name);
                    self.cached_map_info = Some((*map_index, file_name.clone(), title.clone()));
                }
                crate::network::game_client::GameEvent::UserInformation { user_info } => {
                    println!("👤 事件类型: UserInformation (角色信息) ⭐ 关键!");
                    println!("   ObjectID: {}", user_info.object_id);
                    println!("   玩家名称: {}", user_info.name);
                    println!("   职业: {:?}", user_info.class);
                    println!("   性别: {:?}", user_info.gender);
                    println!("   等级: {}", user_info.level);
                    println!("   位置: ({}, {})", user_info.location_x, user_info.location_y);
                    // ⭐ 缓存 UserInformation - 关键修复!
                    self.cached_user_info = Some(user_info.clone());
                    println!("   朝向: {:?}", user_info.direction);
                    println!("   发型: {}", user_info.hair);
                    println!("   金币: {}", user_info.gold);
                    println!("   声望: {}", user_info.credit);
                    tracing::info!("👤 UserInformation: {} 位置=({},{})", 
                        user_info.name, user_info.location_x, user_info.location_y);
                }
                crate::network::game_client::GameEvent::PlayerSpawned { player } => {
                    println!("👥 事件类型: PlayerSpawned (玩家出生)");
                    println!("   玩家名称: {}", player.name);
                    println!("   位置: ({}, {})", player.location.x, player.location.y);
                    tracing::info!("👥 PlayerSpawned: {} 位置=({}, {})", 
                        player.name, player.location.x, player.location.y);
                }
                crate::network::game_client::GameEvent::SystemMessage { message } => {
                    println!("💬 事件类型: SystemMessage (系统消息)");
                    println!("   消息: {}", message);
                    tracing::info!("💬 SystemMessage: {}", message);
                }
                crate::network::game_client::GameEvent::StartGameResponse { result } => {
                    println!("🎮 事件类型: StartGameResponse (开始游戏响应)");
                    println!("   结果: {:?}", result);
                    tracing::info!("🎮 StartGameResponse: {:?}", result);
                }
                _ => {
                    // 其他事件
                    println!("📨 事件类型: {:?}", std::mem::discriminant(&event));
                    tracing::info!("📨 收到网络事件: {:?}", std::mem::discriminant(&event));
                }
            }
            
            println!("════════════════════════════════════════════════════════════════\n");
            
            let mut scene_manager = self.scene_manager.write();
            scene_manager.process_event(&event);
        }
    }
    
    /// 检查并处理场景切换 (Login -> Select)
    pub fn check_login_to_select_transition(&mut self, ctx: &mut ggez::Context) {
        let (should_switch_to_select, ready_flag) = {
            let scene_manager = self.scene_manager.read();
            if scene_manager.current_scene_type() == Some(SceneType::Login) {
                if let Some(scene) = scene_manager.current_scene() {
                    if let Some(login_scene) = scene.as_any().downcast_ref::<LoginScene>() {
                        let ready = login_scene.ready_for_character_select;
                        (ready, Some(ready))
                    } else {
                        (false, None)
                    }
                } else {
                    (false, None)
                }
            } else {
                (false, None)
            }
        };
        
        // 🐛 DEBUG: 每隔一段时间打印状态
        static mut CHECK_COUNTER: u32 = 0;
        unsafe {
            CHECK_COUNTER += 1;
            if CHECK_COUNTER % 120 == 1 {  // 每2秒打印一次(60fps)
                tracing::debug!("🔍 检查场景切换: ready_for_character_select={:?}, should_switch={}", 
                    ready_flag, should_switch_to_select);
            }
        }
        
        if should_switch_to_select {
            // 获取角色列表
            let characters = {
                let scene_manager = self.scene_manager.read();
                if let Some(scene) = scene_manager.current_scene() {
                    if let Some(login_scene) = scene.as_any().downcast_ref::<LoginScene>() {
                        login_scene.characters.clone()
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                }
            };
            
            tracing::info!("✅ 登录成功,切换到角色选择场景 ({} 个角色)", characters.len());
            
            // 🎨 预加载 SelectScene 所需的纹理 (不持有任何锁)
            tracing::info!("📦 预加载 SelectScene 纹理...");
            self.load_select_scene_textures(ctx);
            
            // 创建 SelectScene
            let mut select_scene = Box::new(SelectScene::new(characters));
            
            // 设置网络命令发送器
            select_scene.set_command_sender(self.command_tx.clone());
            
            // 初始化并替换当前场景
            select_scene.initialize();
            
            let mut scene_manager = self.scene_manager.write();
            scene_manager.set_current_scene(select_scene);
            tracing::info!("场景切换完成: Select");
        }
    }
    
    /// 检查并处理场景切换 (Select -> Game)
    pub fn check_select_to_game_transition(&mut self) {
        let should_switch_to_game = {
            let scene_manager = self.scene_manager.read();
            if scene_manager.current_scene_type() == Some(SceneType::Select) {
                if let Some(scene) = scene_manager.current_scene() {
                    if let Some(select_scene) = scene.as_any().downcast_ref::<SelectScene>() {
                        select_scene.pending_scene_change == Some(SceneType::Game)
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            }
        };
        
        if should_switch_to_game {
            tracing::info!("✅ 开始游戏,切换到 GameScene");
            
            // 检查是否有缓存的地图信息
            let cached_map = self.cached_map_info.clone();
            
            // 切换到 GameScene
            let mut scene_manager = self.scene_manager.write();
            if let Err(e) = scene_manager.switch_scene(SceneType::Game) {
                tracing::error!("❌ 切换到 GameScene 失败: {}", e);
            } else {
                tracing::info!("🎮 场景切换完成: Game");
                
                // 1️⃣ 如果有缓存的地图信息,立即发送到 GameScene
                if let Some((map_index, file_name, title)) = cached_map {
                    println!("╔════════════════════════════════════════════════════════════════");
                    println!("║ 🔄 重发缓存的 MapInformation");
                    println!("╚════════════════════════════════════════════════════════════════");
                    println!("   地图: {} ({})", title, file_name);
                    println!("════════════════════════════════════════════════════════════════\n");
                    
                    tracing::info!("🔄 Resending cached MapInformation to GameScene: {} ({})", title, file_name);
                    
                    // 重新创建 MapInformation 事件并发送到 GameScene
                    let event = crate::network::game_client::GameEvent::MapInformation {
                        map_index,
                        file_name: file_name.clone(),
                        title: title.clone(),
                    };
                    
                    // 直接在持有锁的情况下发送事件
                    scene_manager.process_event(&event);
                    tracing::info!("✅ MapInformation event resent to GameScene");
                } else {
                    tracing::warn!("⚠️  No cached map information available");
                }
                
                // 2️⃣ ⭐ 关键修复: 重发缓存的 UserInformation
                if let Some(ref user_info) = self.cached_user_info {
                    println!("╔════════════════════════════════════════════════════════════════");
                    println!("║ 🔄 重发缓存的 UserInformation ⭐⭐⭐");
                    println!("╚════════════════════════════════════════════════════════════════");
                    println!("   玩家: {}", user_info.name);
                    println!("   位置: ({}, {})", user_info.location_x, user_info.location_y);
                    println!("════════════════════════════════════════════════════════════════\n");
                    
                    tracing::info!("🔄 Resending cached UserInformation to GameScene: {}", user_info.name);
                    
                    // 重新创建 UserInformation 事件并发送到 GameScene
                    let event = crate::network::game_client::GameEvent::UserInformation {
                        user_info: user_info.clone(),
                    };
                    
                    scene_manager.process_event(&event);
                    tracing::info!("✅ UserInformation event resent to GameScene");
                } else {
                    tracing::warn!("⚠️  No cached user information available");
                }
            }
        }
    }
    
    /// 预加载 SelectScene 所需的纹理
    fn load_select_scene_textures(&mut self, ctx: &mut ggez::Context) {
        use crate::graphics::libraries::{get_library, LibraryName};
        
        // SelectScene 需要的纹理列表
        let mut textures_to_load = vec![
            (LibraryName::Prguse, 65),   // 背景
            (LibraryName::Title, 40),     // 标题
            (LibraryName::Prguse, 44),   // 空角色槽位
        ];
        
        // 添加按钮纹理 Title_340-354 (StartGame, NewCharacter, DeleteCharacter, Credits, ExitGame)
        for i in 340..=354 {
            textures_to_load.push((LibraryName::Title, i));
        }
        
        // 添加角色槽位纹理 Title_660-669 (职业槽位: 660-664未选中, 665-669选中)
        for i in 660..=669 {
            textures_to_load.push((LibraryName::Title, i));
        }
        
        // 添加角色预览纹理 ChrSel_220 和混合纹理 ChrSel_780 (220+560)
        textures_to_load.push((LibraryName::ChrSel, 220));
        textures_to_load.push((LibraryName::ChrSel, 780));  // 220 + 560 混合效果
        
        // NewCharacterDialog 纹理
        textures_to_load.push((LibraryName::Prguse, 73));   // 对话框背景
        textures_to_load.push((LibraryName::Title, 20));    // 标题标签
        textures_to_load.push((LibraryName::Title, 280));   // 取消按钮 (Normal)
        textures_to_load.push((LibraryName::Title, 281));   // 取消按钮 (Hover)
        textures_to_load.push((LibraryName::Title, 282));   // 取消按钮 (Pressed)
        textures_to_load.push((LibraryName::Title, 360));   // 确认按钮 (Normal)
        textures_to_load.push((LibraryName::Title, 361));   // 确认按钮 (Hover)
        textures_to_load.push((LibraryName::Title, 362));   // 确认按钮 (Pressed)
        
        // DeleteCharacterDialog / MessageBox / InputBox 纹理
        textures_to_load.push((LibraryName::Prguse, 360));  // MessageBox 背景
        textures_to_load.push((LibraryName::Prguse, 660));  // InputBox 背景
        // OK 按钮 (200-202)
        textures_to_load.push((LibraryName::Title, 200));   // OK (Normal)
        textures_to_load.push((LibraryName::Title, 201));   // OK (Hover)
        textures_to_load.push((LibraryName::Title, 202));   // OK (Pressed)
        // Cancel 按钮 (203-205)
        textures_to_load.push((LibraryName::Title, 203));   // Cancel (Normal)
        textures_to_load.push((LibraryName::Title, 204));   // Cancel (Hover)
        textures_to_load.push((LibraryName::Title, 205));   // Cancel (Pressed)
        // Yes 按钮 (206-208)
        textures_to_load.push((LibraryName::Title, 206));   // Yes (Normal)
        textures_to_load.push((LibraryName::Title, 207));   // Yes (Hover)
        textures_to_load.push((LibraryName::Title, 208));   // Yes (Pressed)
        // No 按钮 (210-212)
        textures_to_load.push((LibraryName::Title, 210));   // No (Normal)
        textures_to_load.push((LibraryName::Title, 211));   // No (Hover)
        textures_to_load.push((LibraryName::Title, 212));   // No (Pressed)
        
        // 职业按钮 (战士、法师、道士、刺客、弓箭手)
        for i in 2426..=2440 {  // 2426-2428 (战士), 2429-2431 (法师), 2432-2434 (道士), 2435-2437 (刺客), 2438-2440 (弓箭手)
            textures_to_load.push((LibraryName::Prguse, i));
        }
        
        // 性别按钮 (男、女)
        for i in 2420..=2425 {  // 2420-2422 (男), 2423-2425 (女)
            textures_to_load.push((LibraryName::Prguse, i));
        }
        
        // 角色预览动画 (所有职业和性别组合)
        // 战士男 20-35, 法师男 40-55, 道士男 60-75, 刺客男 80-95, 弓箭手男 100-115
        // 战士女 300-315, 法师女 320-335, 道士女 340-355, 刺客女 360-375, 弓箭手女 140-155 (注意:不是380!)
        for base in [20, 40, 60, 80, 100, 300, 320, 340, 360, 140] {
            for i in base..base+16 {  // 16帧动画
                textures_to_load.push((LibraryName::ChrSel, i));
            }
        }
        
        // 法师混合效果纹理 (基础索引 + 560)
        // 法师男: 40-55 + 560 = 600-615
        // 法师女: 320-335 + 560 = 880-895
        for base in [40, 320] {
            for i in (base+560)..(base+560+16) {
                textures_to_load.push((LibraryName::ChrSel, i));
            }
        }
        
        for (lib_name, index) in textures_to_load {
            // 从 MLibrary 预加载纹理（MLibrary 内部会缓存）
            if let Some(lib_arc) = get_library(lib_name) {
                let mut lib = lib_arc.lock().unwrap();
                // 调用 get_or_create_texture 会自动创建并缓存纹理
                match lib.get_or_create_texture(ctx, index) {
                    Ok(_info) => {
                        tracing::debug!("✓ 预加载纹理: {}_{}", lib_name.default_path(), index);
                    }
                    Err(e) => {
                        tracing::warn!("⚠️ 预加载纹理失败 {}_{}: {}", lib_name.default_path(), index, e);
                    }
                }
            } else {
                tracing::warn!("⚠️ 库未加载: {:?}", lib_name);
            }
        }
        
        tracing::info!("✓ SelectScene 纹理预加载完成");
    }
}

/// 为 CrystalGame 实现 ggez::EventHandler trait
impl ggez::event::EventHandler for CrystalGame {
    fn update(&mut self, ctx: &mut ggez::Context) -> ggez::GameResult {
        // 计算 delta_time
        let now = std::time::Instant::now();
        let delta_time = now.duration_since(self.last_update_time).as_secs_f32();
        self.last_update_time = now;
        
        // 限制 delta_time (防止卡顿时跳跃过大)
        let delta_time = delta_time.min(0.1);
        
        // 处理网络事件
        self.process_network_events();
        
        // 检查 LoginScene -> SelectScene 转换
        self.check_login_to_select_transition(ctx);
        
        // 检查 SelectScene -> GameScene 转换
        self.check_select_to_game_transition();
        
        // 处理场景切换
        {
            let mut scene_manager = self.scene_manager.write();
            if let Err(e) = scene_manager.process_transitions() {
                tracing::error!("场景切换错误: {}", e);
                ctx.request_quit();
                return Ok(());
            }
            
            // 更新当前场景
            scene_manager.update(delta_time);
        }
        
        Ok(())
    }

    fn draw(&mut self, ctx: &mut ggez::Context) -> ggez::GameResult {
        // 开始帧
        self.ggez_manager.begin_frame();
        
        // 🔧 根据当前场景选择背景色
        use ggez::graphics::Color;
        let bg_color = {
            let scene_manager = self.scene_manager.read();
            match scene_manager.current_scene_type() {
                Some(crate::scenes::SceneType::Login) | Some(crate::scenes::SceneType::Select) => {
                    Color::from_rgb(0, 0, 0) // 登录和选择场景使用黑色背景
                },
                Some(crate::scenes::SceneType::Game) => {
                    Color::from_rgb(0, 32, 0) // 游戏场景使用深绿色背景
                },
                None => Color::from_rgb(0, 0, 0), // 默认黑色
            }
        };
        
        // 创建 canvas (ggez会用bg_color清除framebuffer)
        let mut canvas = ggez::graphics::Canvas::from_frame(ctx, bg_color);
        
        // 绘制当前场景
        {
            let mut scene_manager = self.scene_manager.write();
            scene_manager.draw(ctx, &mut canvas);
        }
        
        // 结束帧
        canvas.finish(ctx)?;
        self.ggez_manager.end_frame();
        
        Ok(())
    }
    
    fn key_down_event(
        &mut self,
        ctx: &mut ggez::Context,
        input: ggez::input::keyboard::KeyInput,
        _repeated: bool,
    ) -> ggez::GameResult {
        use ggez::winit::keyboard::{KeyCode as WinitKeyCode, PhysicalKey};
        use crate::scenes::ModifiersState;
        
        tracing::info!("🔑 key_down_event 被调用!");
        
        let mut key_consumed = false;
        
        if let PhysicalKey::Code(keycode) = input.event.physical_key {
            // 检查 Ctrl+Q 退出
            if keycode == WinitKeyCode::KeyQ && input.mods.control_key() {
                tracing::info!("用户请求退出 (Ctrl+Q)");
                ctx.request_quit();
                return Ok(());
            }
            
            let modifiers = input.mods;
            
            // 转换为 Scene 自定义 ModifiersState
            let modifiers_state = ModifiersState {
                shift: modifiers.shift_key(),
                ctrl: modifiers.control_key(),
                alt: modifiers.alt_key(),
            };
            
            // 转换 ggez KeyCode 到 Scene KeyCode
            if let Some(scene_keycode) = ggez_keycode_to_scene(keycode) {
                let mut scene_manager = self.scene_manager.write();
                key_consumed = scene_manager.handle_key_press(scene_keycode, modifiers_state);
            }
        }
        
        // 只有在按键未被消费时才处理文本输入
        if !key_consumed {
            tracing::debug!("KeyEvent - text field: {:?}", input.event.text);
            
            if let Some(text) = &input.event.text {
                tracing::info!("✓ 收到文本: '{}'", text);
                for ch in text.chars() {
                    // 过滤掉特殊控制字符
                    if ch != '\r' && ch != '\n' && ch != '\t' && ch != '\x08' && !ch.is_control() {
                        tracing::info!("→ 处理字符: '{}'", ch);
                        let mut scene_manager = self.scene_manager.write();
                        scene_manager.handle_text_input(ch);
                    } else {
                        tracing::debug!("✗ 过滤控制字符: {:?}", ch);
                    }
                }
            } else {
                tracing::debug!("✗ text 字段为 None");
            }
        } else {
            tracing::debug!("✗ 按键被消费,跳过文本输入");
        }
        
        Ok(())
    }
    
    fn mouse_button_down_event(
        &mut self,
        _ctx: &mut ggez::Context,
        button: ggez::input::mouse::MouseButton,
        x: f32,
        y: f32,
    ) -> ggez::GameResult {
        use crate::scenes::MouseButton as SceneMouseButton;
        
        // 转换 ggez MouseButton 到 Scene MouseButton
        let scene_button = match button {
            ggez::input::mouse::MouseButton::Left => SceneMouseButton::Left,
            ggez::input::mouse::MouseButton::Right => SceneMouseButton::Right,
            ggez::input::mouse::MouseButton::Middle => SceneMouseButton::Middle,
            ggez::input::mouse::MouseButton::Back | ggez::input::mouse::MouseButton::Forward => SceneMouseButton::Other(0),
            ggez::input::mouse::MouseButton::Other(v) => SceneMouseButton::Other(v),
        };
        
        // 将实际窗口坐标转换为逻辑坐标
        let logical_x = x / self.scale_factor;
        let logical_y = y / self.scale_factor;
        
        let mut scene_manager = self.scene_manager.write();
        scene_manager.handle_mouse_button(scene_button, true, logical_x as i32, logical_y as i32);
        
        Ok(())
    }
    
    fn mouse_button_up_event(
        &mut self,
        _ctx: &mut ggez::Context,
        button: ggez::input::mouse::MouseButton,
        x: f32,
        y: f32,
    ) -> ggez::GameResult {
        use crate::scenes::MouseButton as SceneMouseButton;
        
        let scene_button = match button {
            ggez::input::mouse::MouseButton::Left => SceneMouseButton::Left,
            ggez::input::mouse::MouseButton::Right => SceneMouseButton::Right,
            ggez::input::mouse::MouseButton::Middle => SceneMouseButton::Middle,
            ggez::input::mouse::MouseButton::Back | ggez::input::mouse::MouseButton::Forward => SceneMouseButton::Other(0),
            ggez::input::mouse::MouseButton::Other(v) => SceneMouseButton::Other(v),
        };
        
        // 将实际窗口坐标转换为逻辑坐标
        let logical_x = x / self.scale_factor;
        let logical_y = y / self.scale_factor;
        
        let mut scene_manager = self.scene_manager.write();
        scene_manager.handle_mouse_button(scene_button, false, logical_x as i32, logical_y as i32);
        
        Ok(())
    }
    
    fn mouse_motion_event(
        &mut self,
        _ctx: &mut ggez::Context,
        x: f32,
        y: f32,
        _dx: f32,
        _dy: f32,
    ) -> ggez::GameResult {
        // 将实际窗口坐标转换为逻辑坐标
        let logical_x = x / self.scale_factor;
        let logical_y = y / self.scale_factor;
        
        let mut scene_manager = self.scene_manager.write();
        scene_manager.handle_mouse_move(logical_x as i32, logical_y as i32);
        
        Ok(())
    }
    
    fn mouse_wheel_event(&mut self, _ctx: &mut ggez::Context, x: f32, y: f32) -> ggez::GameResult {
        let mut scene_manager = self.scene_manager.write();
        scene_manager.handle_mouse_wheel(x, y);
        
        Ok(())
    }
    
    fn text_input_event(&mut self, _ctx: &mut ggez::Context, character: char) -> ggez::GameResult {
        tracing::debug!("收到文本输入: '{}'", character);
        
        let mut scene_manager = self.scene_manager.write();
        scene_manager.handle_text_input(character);
        
        Ok(())
    }
}

/// 辅助函数: 转换 winit KeyCode 到 Scene KeyCode
fn ggez_keycode_to_scene(key: ggez::winit::keyboard::KeyCode) -> Option<crate::scenes::KeyCode> {
    use ggez::winit::keyboard::KeyCode as GK;
    use crate::scenes::KeyCode as SceneKeyCode;
    
    Some(match key {
        GK::KeyA => SceneKeyCode::KeyA,
        GK::KeyB => SceneKeyCode::KeyB,
        GK::KeyC => SceneKeyCode::KeyC,
        GK::KeyD => SceneKeyCode::KeyD,
        GK::KeyE => SceneKeyCode::KeyE,
        GK::KeyF => SceneKeyCode::KeyF,
        GK::KeyG => SceneKeyCode::KeyG,
        GK::KeyH => SceneKeyCode::KeyH,
        GK::KeyI => SceneKeyCode::KeyI,
        GK::KeyJ => SceneKeyCode::KeyJ,
        GK::KeyK => SceneKeyCode::KeyK,
        GK::KeyL => SceneKeyCode::KeyL,
        GK::KeyM => SceneKeyCode::KeyM,
        GK::KeyN => SceneKeyCode::KeyN,
        GK::KeyO => SceneKeyCode::KeyO,
        GK::KeyP => SceneKeyCode::KeyP,
        GK::KeyQ => SceneKeyCode::KeyQ,
        GK::KeyR => SceneKeyCode::KeyR,
        GK::KeyS => SceneKeyCode::KeyS,
        GK::KeyT => SceneKeyCode::KeyT,
        GK::KeyU => SceneKeyCode::KeyU,
        GK::KeyV => SceneKeyCode::KeyV,
        GK::KeyW => SceneKeyCode::KeyW,
        GK::KeyX => SceneKeyCode::KeyX,
        GK::KeyY => SceneKeyCode::KeyY,
        GK::KeyZ => SceneKeyCode::KeyZ,
        GK::Digit1 => SceneKeyCode::Digit1,
        GK::Digit2 => SceneKeyCode::Digit2,
        GK::Digit3 => SceneKeyCode::Digit3,
        GK::Digit4 => SceneKeyCode::Digit4,
        GK::Digit5 => SceneKeyCode::Digit5,
        GK::Digit6 => SceneKeyCode::Digit6,
        GK::Digit7 => SceneKeyCode::Digit7,
        GK::Digit8 => SceneKeyCode::Digit8,
        GK::Digit9 => SceneKeyCode::Digit9,
        GK::Digit0 => SceneKeyCode::Digit0,
        GK::Enter => SceneKeyCode::Enter,
        GK::Escape => SceneKeyCode::Escape,
        GK::Backspace => SceneKeyCode::Backspace,
        GK::Tab => SceneKeyCode::Tab,
        GK::Space => SceneKeyCode::Space,
        GK::ArrowLeft => SceneKeyCode::ArrowLeft,
        GK::ArrowRight => SceneKeyCode::ArrowRight,
        GK::ArrowUp => SceneKeyCode::ArrowUp,
        GK::ArrowDown => SceneKeyCode::ArrowDown,
        GK::F1 => SceneKeyCode::F1,
        GK::F2 => SceneKeyCode::F2,
        GK::F3 => SceneKeyCode::F3,
        GK::F4 => SceneKeyCode::F4,
        GK::F5 => SceneKeyCode::F5,
        GK::F6 => SceneKeyCode::F6,
        GK::F7 => SceneKeyCode::F7,
        GK::F8 => SceneKeyCode::F8,
        GK::F9 => SceneKeyCode::F9,
        GK::F10 => SceneKeyCode::F10,
        GK::F11 => SceneKeyCode::F11,
        GK::F12 => SceneKeyCode::F12,
        _ => return None,
    })
}

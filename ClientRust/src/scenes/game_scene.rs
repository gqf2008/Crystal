// GameScene V2 - Refactored to mirror C# GameScene.cs architecture
// Reference: Client/MirScenes/GameScene.cs
//
// ARCHITECTURE PRINCIPLES:
// 1. GameScene is the central hub managing ALL game state
// 2. MapControl handles map rendering (nested functionality)
// 3. UI controls managed through control tree (Parent = this)
// 4. Network packets processed centrally via process_packet()
// 5. Rendering phases: MapControl.draw() → UI tree → Top layer

use ggez::graphics::Canvas;
use ggez::GameResult;
use std::collections::{HashMap, VecDeque};

use mir2_shared::{
    data::client_data::{
        ClientBuff,      // ✅ SharedRust/src/data/client_data.rs line 764
        ClientFriend, // ✅ SharedRust/src/data/client_data.rs line 885 (Shared/Data/ClientData.cs line 122)
        ClientMagic,  // ✅ SharedRust/src/data/client_data.rs line 70
        ClientMail, // ✅ SharedRust/src/data/client_data.rs line 922 (Shared/Data/ClientData.cs line 154)
        ClientQuestInfo, // ✅ SharedRust/src/data/client_data.rs line 392
    },
    data::shared_data::{
        RankCharacterInfo, // ✅ SharedRust/src/data/shared_data.rs line 92 (Shared/Data/SharedData.cs line 43)
    },
    enums::*,
    Point,
    UserItem, // ✅ Shared/Data/ItemData.cs line 277
};

use crate::controls::Control;
use crate::objects::{HeroObject, MapObject, UserObject};
use crate::scenes::{GameEvent, KeyCode, ModifiersState, Scene, SceneType};

// 导入 Camera (摄像机系统)
pub mod camera;
pub use camera::Camera;

// 导入 MapRenderer (纯渲染层)
pub mod map_renderer;
pub use map_renderer::MapRenderer;

// ==================== 架构说明 ====================
//
// 数据结构来源:
// - UserItem, ClientMagic, ClientBuff, ClientQuestInfo → mir2_shared (Shared 项目)
// - MirItemCell, MirLabel 等 UI 控件 → controls 模块 (Client/MirControls)
// - MapObject, UserObject 等游戏对象 → objects 模块 (Client/MirObjects)
//
// 参考:
// - Shared/Data/ItemData.cs line 277 (UserItem)
// - SharedRust/src/data/client_data.rs line 70 (ClientMagic)
// - SharedRust/src/data/client_data.rs line 764 (ClientBuff)
// - SharedRust/src/data/client_data.rs line 392 (ClientQuestInfo)
// - Client/MirControls/MirItemCell.cs line 11 (MirItemCell - UI 控件)
//
// ==================== 客户端专有数据结构 ====================

/// 用户位置（用于绘制玩家）
#[derive(Debug, Clone, Copy)]
pub struct UserPosition {
    pub x: i32,
    pub y: i32,
    pub offset_x: i32,
    pub offset_y: i32,
}

//
// 以下结构是客户端 UI 层的数据,不属于 mir2_shared:

#[allow(dead_code)]
/// 任务跟踪 UI 状态 (对应 C# QuestTrackingDialog 的内部状态)
///
/// 注意: C# 中 QuestTrackingDialog 维护任务追踪状态,没有单独的 QuestTracker 类
/// TODO: 后续应迁移到 controls::QuestTrackingDialog 作为内部状态
#[derive(Debug, Clone)]
pub struct QuestTracker {
    pub quest_id: u32,
    pub active: bool,
}

/// 输出消息 (对应 C# OutPutMessage)
///
/// 屏幕左上角的滚动提示文本 (系统消息/任务消息/公会消息)
/// 这是客户端 UI 层的数据,不需要与服务器通信
#[derive(Debug, Clone)]
pub struct OutputMessage {
    pub message: String,
    pub expire_time: i64,
    pub message_type: OutputMessageType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMessageType {
    Normal,
    Quest,
    Guild,
}

// ==================== 已删除的重复定义 ====================
//
// 以下结构已从 game_scene_v2.rs 删除,改用 mir2_shared:
//
// ❌ Friend           → ✅ 使用 mir2_shared::data::client_data::ClientFriend
// ❌ Mail             → ✅ 使用 mir2_shared::data::client_data::ClientMail
// ❌ Rank             → ✅ 使用 mir2_shared::data::shared_data::RankCharacterInfo
// ❌ Relationship     → ✅ 不需要,使用 ClientFriend 即可
// ❌ RelationshipType → ✅ 不需要,使用 ClientFriend 的字段表示
// ❌ GuildObject      → ✅ 不需要完整对象,只存公会名称/等级字段

// ==================== GameScene 状态机 ====================

/// GameScene 加载状态
///
/// 用于管理从 SelectScene 切换到 GameScene 的加载流程:
/// 1. WaitingForData - 等待地图信息和玩家信息
/// 2. LoadingMap - 正在加载地图文件
/// 3. Ready - 地图和玩家都已准备好,可以渲染
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameSceneState {
    /// 初始状态,等待地图和玩家数据
    WaitingForData,
    /// 正在加载地图 (map_name)
    LoadingMap(String),
    /// 等待玩家信息
    WaitingForPlayer,
    /// 所有数据就绪,可以正常渲染
    Ready,
}

// ==================== GameScene 主结构 ====================

/// GameScene - 游戏场景中枢
///
/// 对应 C# Client/MirScenes/GameScene.cs (line 27-10207)
///
/// 这是游戏运行期的核心类,管理:
/// - 所有游戏状态数据
/// - MapControl (地图渲染)
/// - UI 控件树
/// - 网络协议处理
/// - 输入处理
#[allow(dead_code)]
pub struct GameScene {
    // ==================== 子系统架构 (NEW) ====================
    /// 输入系统 - 统一处理鼠标键盘
    input_system: crate::systems::InputSystem,
    
    /// 对象管理系统 - 管理玩家、怪物、NPC、掉落物
    object_manager: crate::systems::ObjectManager,
    
    /// 渲染管线 - 整合 MapRenderer 和 Camera
    rendering_pipeline: crate::systems::RenderingPipeline,
    
    // ==================== 场景状态 ====================
    /// 当前场景加载状态
    state: GameSceneState,

    // ==================== 玩家与英雄 (保留用于向后兼容) ====================
    /// 当前玩家对象 (向后兼容字段，实际数据在 object_manager 中)
    user: Option<UserObject>,

    /// 英雄对象 (向后兼容字段，实际数据在 object_manager 中)
    hero: Option<HeroObject>,

    /// 是否拥有英雄 (C#: public bool HasHero)
    has_hero: bool,

    /// 英雄召唤状态 (C#: public HeroSpawnState HeroSpawnState)
    hero_spawn_state: HeroSpawnState,

    // ==================== 对象管理 (保留用于向后兼容) ====================
    /// 所有地图对象 (向后兼容字段)
    objects: HashMap<u32, MapObject>,

    /// 被选中的格子 (C#: public static MapObject SelectedCell)
    selected_cell: Option<MapObject>,

    // ==================== 物品与经济 ====================
    // C# line 160-169
    /// 背包 46 格 (C#: 通过 GameScene.Inventory 静态字段)
    /// 注意: C# 中 Inventory 是 MirItemCell[] (UI控件数组)
    /// Rust 中暂用 Option<UserItem> 存储数据,UI 由 controls 模块负责
    inventory: [Option<UserItem>; 46],

    /// 仓库 80 格 (C#: public static UserItem[] Storage)
    storage: [Option<UserItem>; 80],

    /// 腰带 6 格 (C#: 通过 BeltIdx)
    belt: [Option<UserItem>; 6],

    /// 装备槽 14 格 (C#: 通过 Equipment)
    equipment: [Option<UserItem>; 14],

    /// 公会仓库 112 格 (C#: public static UserItem[] GuildStorage)
    guild_storage: [Option<UserItem>; 112],

    /// 精炼仓库 16 格 (C#: public static UserItem[] Refine)
    refine_storage: [Option<UserItem>; 16],

    /// 金币 (C#: public static uint Gold)
    gold: u32,

    /// 点数 (C#: public static uint Credit)
    credit: u32,

    /// 悬浮物品 (C#: public static UserItem HoverItem)
    hover_item: Option<UserItem>,

    /// 选中物品 (C#: public static UserItem SelectedItem)
    selected_item: Option<UserItem>,

    /// 选中的物品格子 (C#: public static MirItemCell SelectedCell)
    /// 注意: C# 中是 MirItemCell (UI 控件引用)
    /// Rust 中暂存物品数据,UI 控件由 controls 模块管理
    selected_cell_item: Option<UserItem>,

    /// 是否拾取了金币 (C#: public static bool PickedUpGold)
    picked_up_gold: bool,

    // ==================== 技能与 Buff ====================
    // C# 通过 User.Magics/User.Buffs
    /// 技能列表 (C#: User.Magics)
    /// 使用 mir2_shared::data::client_data::ClientMagic
    magics: Vec<ClientMagic>,

    /// Buff 列表 (C#: User.Buffs)
    /// 使用 mir2_shared::data::client_data::ClientBuff
    buffs: Vec<ClientBuff>,

    // ==================== 任务系统 ====================
    // C# line 153-156
    /// 任务列表 (C#: public static List<ClientQuestInfo> QuestInfoList)
    /// 使用 mir2_shared::data::client_data::ClientQuestInfo
    quests: Vec<ClientQuestInfo>,

    /// 跟踪的任务 (C#: QuestTrackingDialog.TrackedQuests)
    tracked_quests: Vec<QuestTracker>,

    // ==================== 社交系统 ====================
    // C# 通过各个 Dialog 管理
    /// 好友列表 (C#: FriendDialog)
    /// 使用 mir2_shared::data::client_data::ClientFriend (C# Shared/Data/ClientData.cs line 122)
    friends: Vec<ClientFriend>,

    /// 公会名称 (C#: Scene.Guild.Name)
    /// 注意: 客户端不需要完整的 GuildObject,只存储公会名称和等级
    guild_name: Option<String>,

    /// 公会等级 (C#: Scene.Guild.Rank)
    guild_rank: Option<String>,

    // ==================== 邮件系统 ====================
    // C# line 106-110
    /// 邮件列表 (C#: MailListDialog)
    /// 使用 mir2_shared::data::client_data::ClientMail (C# Shared/Data/ClientData.cs line 154)
    mail_list: Vec<ClientMail>,

    /// 是否有新邮件 (C#: public bool NewMail)
    new_mail: bool,

    /// 新邮件计数器 (C#: public int NewMailCounter)
    new_mail_counter: i32,

    // ==================== 排行榜 ====================
    // C# line 157
    /// 排行榜列表 (C#: public static Dictionary<long, RankCharacterInfo> RankingList)
    /// 使用 mir2_shared::data::shared_data::RankCharacterInfo (C# Shared/Data/SharedData.cs line 43)
    rankings: Vec<RankCharacterInfo>,

    // ==================== MapRenderer (地图渲染) ====================
    // 对应 C# MapControl 嵌套类 (line 10209-11241)
    // 重构后：MapRenderer 直接拥有地图数据（采用 map_viewer.rs 设计）
    /// 地图渲染器 - 拥有地图数据并负责渲染
    map_renderer: MapRenderer,

    // ==================== Camera (摄像机系统) ====================
    // 🎥 用于坐标转换和视野管理
    /// 摄像机（跟随玩家，处理世界坐标↔屏幕坐标转换）
    camera: Camera,

    // ==================== UI 控件树 ====================
    // 对应 C# line 59-149 (所有 Dialog)
    /// 子控件列表 (C#: 通过 Parent = this 建立)
    controls: Vec<Box<dyn Control>>,

    // 主要对话框 (C#: public XXXDialog XXXDialog)
    // 注意: 暂时用 Option,后续实现时创建
    main_dialog: Option<()>,           // MainDialog
    chat_dialog: Option<()>,           // ChatDialog
    chat_control: Option<()>,          // ChatControlBar
    inventory_dialog: Option<()>,      // InventoryDialog
    character_dialog: Option<()>,      // CharacterDialog
    hero_dialog: Option<()>,           // HeroDialog (CharacterDialog for hero)
    hero_inventory_dialog: Option<()>, // HeroInventoryDialog
    hero_manage_dialog: Option<()>,    // HeroManageDialog
    craft_dialog: Option<()>,          // CraftDialog
    storage_dialog: Option<()>,        // StorageDialog
    belt_dialog: Option<()>,           // BeltDialog
    minimap_dialog: Option<()>,        // MiniMapDialog
    inspect_dialog: Option<()>,        // InspectDialog
    option_dialog: Option<()>,         // OptionDialog
    menu_dialog: Option<()>,           // MenuDialog
    npc_dialog: Option<()>,            // NPCDialog
    // ... 40+ 其他对话框

    // ==================== 输入与渲染 ====================
    // C# line 171-182
    /// 鼠标位置 (C#: 通过 CMain.MPoint)
    mouse_location: Point,

    /// 输出消息列表 (C#: public List<OutPutMessage> OutputMessages)
    output_messages: VecDeque<OutputMessage>,

    /// 输出行 (C#: public MirLabel[] OutputLines)
    output_lines: Vec<String>,

    /// 最大输出消息数 (C#: 默认 10)
    max_output_messages: usize,

    // ==================== 模式与状态 ====================
    // C# line 184-186
    /// 攻击模式 (C#: public AttackMode AMode)
    attack_mode: AttackMode,

    /// 宠物模式 (C#: public PetMode PMode)
    pet_mode: PetMode,

    /// 光照设置 (C#: public LightSetting Lights)
    lights: LightSetting,

    /// 观察模式 (C#: public static bool Observing)
    observing: bool,

    /// 允许观察 (C#: public static bool AllowObserve)
    allow_observe: bool,

    // ==================== 时间戳 ====================
    // C# line 29-30
    /// 移动时间 (C#: public static long MoveTime)
    move_time: i64,

    /// 攻击时间 (C#: public static long AttackTime)
    attack_time: i64,

    /// 下次奔跑时间 (C#: public static long NextRunTime)
    next_run_time: i64,

    /// 登出时间 (C#: public static long LogTime)
    log_time: i64,

    /// 上次奔跑时间 (C#: public static long LastRunTime)
    last_run_time: i64,

    /// 切换宠物模式时间 (C#: public static long ChangePModeTime)
    change_pmode_time: i64,

    /// 切换攻击模式时间 (C#: public static long ChangeAModeTime)
    change_amode_time: i64,

    /// 英雄技能时间 (C#: public static long HeroSpellTime)
    hero_spell_time: i64,

    /// 智能生物拾取时间 (C#: public static long IntelligentCreaturePickupTime)
    intelligent_creature_pickup_time: i64,

    /// 使用物品时间 (C#: public static long UseItemTime)
    use_item_time: i64,

    /// 拾取时间 (C#: public static long PickUpTime)
    pickup_time: i64,

    /// 掉落视图时间 (C#: public static long DropViewTime)
    drop_view_time: i64,

    /// 目标死亡时间 (C#: public static long TargetDeadTime)
    target_dead_time: i64,

    /// 检查时间 (C#: public static long InspectTime)
    inspect_time: i64,

    /// 法术时间 (C#: public static long SpellTime)
    spell_time: i64,

    /// 切换时间 (C#: public long ToggleTime)
    toggle_time: i64,

    /// 输出延迟 (C#: public long OutputDelay)
    output_delay: i64,

    // ==================== 标志位 ====================
    // C# line 31
    /// 可以移动 (C#: public static bool CanMove)
    can_move: bool,

    /// 可以奔跑 (C#: public static bool CanRun)
    can_run: bool,

    /// 显示复活消息 (C#: public bool ShowReviveMessage)
    show_revive_message: bool,

    // ==================== NPC 相关 ====================
    // C# line 188-193
    /// NPC 时间 (C#: public static long NPCTime)
    npc_time: i64,

    /// NPC ID (C#: public static uint NPCID)
    npc_id: u32,

    /// NPC 费率 (C#: public static float NPCRate)
    npc_rate: f32,

    /// NPC 面板类型 (C#: public static PanelType NPCPanelType)
    npc_panel_type: PanelType,

    /// 默认 NPC ID (C#: public static uint DefaultNPCID)
    default_npc_id: u32,

    /// 隐藏添加的商店属性 (C#: public static bool HideAddedStoreStats)
    hide_added_store_stats: bool,

    // ==================== 粒子引擎 ====================
    // C# line 53
    /// 粒子引擎列表 (C#: public List<ParticleEngine> ParticleEngines)
    particle_engines: Vec<()>, // TODO: 实现 ParticleEngine

    // ==================== 调试控制 ====================
    /// 是否显示玩家角色 (调试用，U键控制)
    show_player: bool,
    
    // ==================== 调试信息 ====================
    /// 碰到的障碍物格子 (用于调试绘制红色标记)
    blocked_cell: Option<Point>,
    
    // ==================== 网络通信 ====================
    /// 网络命令发送通道
    command_tx: Option<tokio::sync::mpsc::UnboundedSender<crate::network::NetworkCommand>>,
}

/// HeroSpawnState - 英雄召唤状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeroSpawnState {
    None,
    Spawning,
    Spawned,
    Unsummoning,
}

/// PanelType - 面板类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelType {
    Buy,
    BuySub,
    Craft,
    Sell,
    // ... 更多类型
}

// ==================== GameScene 实现 ====================

impl GameScene {
    /// 创建新的 GameScene
    ///
    /// 对应 C# GameScene 构造函数 (line 242-461)
    pub fn new() -> Self {
        tracing::info!("🎮 ========================================");
        tracing::info!("🎮 GameScene::new() 创建游戏场景 (子系统架构)");
        tracing::info!("🎮   使用 InputSystem + ObjectManager + RenderingPipeline");
        tracing::info!("🎮   初始摄像机位置: (0, 0)");
        tracing::info!("🎮 ========================================");

        // 创建子系统
        let input_system = crate::systems::InputSystem::new();
        let object_manager = crate::systems::ObjectManager::new();
        let rendering_pipeline = crate::systems::RenderingPipeline::new();

        Self {
            // 子系统
            input_system,
            object_manager,
            rendering_pipeline,
            
            // 场景状态
            state: GameSceneState::WaitingForData,
            
            // 玩家与英雄 (向后兼容字段)
            user: None,
            hero: None,
            has_hero: false,
            hero_spawn_state: HeroSpawnState::None,
            
            // 对象管理 (向后兼容字段)
            objects: HashMap::new(),
            selected_cell: None,

            // 物品与经济
            inventory: std::array::from_fn(|_| None),
            storage: std::array::from_fn(|_| None),
            belt: std::array::from_fn(|_| None),
            equipment: std::array::from_fn(|_| None),
            guild_storage: std::array::from_fn(|_| None),
            refine_storage: std::array::from_fn(|_| None),
            gold: 0,
            credit: 0,
            hover_item: None,
            selected_item: None,
            selected_cell_item: None,
            picked_up_gold: false,

            // 技能与 Buff
            magics: Vec::new(),
            buffs: Vec::new(),

            // 任务系统
            quests: Vec::new(),
            tracked_quests: Vec::new(),

            // 社交系统
            friends: Vec::new(),
            guild_name: None,
            guild_rank: None,

            // 邮件系统
            mail_list: Vec::new(),
            new_mail: false,
            new_mail_counter: 0,

            // 排行榜
            rankings: Vec::new(),

            // MapRenderer（拥有地图数据，初始化为空，load_map 时从 MapReader 构造）
            map_renderer: MapRenderer::default(),

            // Camera（初始化为 1024x768，后续在 initialize 中更新）
            camera: Camera::new(1024.0, 768.0),

            // UI 控件树
            controls: Vec::new(),
            main_dialog: None,
            chat_dialog: None,
            chat_control: None,
            inventory_dialog: None,
            character_dialog: None,
            hero_dialog: None,
            hero_inventory_dialog: None,
            hero_manage_dialog: None,
            craft_dialog: None,
            storage_dialog: None,
            belt_dialog: None,
            minimap_dialog: None,
            inspect_dialog: None,
            option_dialog: None,
            menu_dialog: None,
            npc_dialog: None,

            // 输入与渲染
            mouse_location: Point { x: 0, y: 0 },
            output_messages: VecDeque::new(),
            output_lines: Vec::new(),
            max_output_messages: 10,

            // 模式与状态
            attack_mode: AttackMode::Peace,
            pet_mode: PetMode::Both,
            lights: LightSetting::Day,
            observing: false,
            allow_observe: false,

            // 时间戳
            move_time: 0,
            attack_time: 0,
            next_run_time: 0,
            log_time: 0,
            last_run_time: 0,
            change_pmode_time: 0,
            change_amode_time: 0,
            hero_spell_time: 0,
            intelligent_creature_pickup_time: 0,
            use_item_time: 0,
            pickup_time: 0,
            drop_view_time: 0,
            target_dead_time: 0,
            inspect_time: 0,
            spell_time: 0,
            toggle_time: 0,
            output_delay: 0,

            // 标志位
            can_move: true,
            can_run: true,
            show_revive_message: false,

            // NPC 相关
            npc_time: 0,
            npc_id: 0,
            npc_rate: 1.0,
            npc_panel_type: PanelType::Buy,
            default_npc_id: 0,
            hide_added_store_stats: false,

            // 粒子引擎
            particle_engines: Vec::new(),

            // 调试控制
            show_player: true, // 默认显示玩家
            
            blocked_cell: None, // 障碍物调试
            command_tx: None, // 网络命令通道（后续通过 set_command_sender 设置）
        }
    }

    /// 设置网络命令发送通道
    pub fn set_command_sender(&mut self, tx: tokio::sync::mpsc::UnboundedSender<crate::network::NetworkCommand>) {
        self.command_tx = Some(tx);
        tracing::info!("✅ GameScene 网络命令通道已设置");
    }

    /// 绘制 UI 控件树
    ///
    /// 等价于 C# base.DrawControl()
    fn draw_controls(&mut self, canvas: &mut Canvas) -> GameResult<()> {
        for control in &mut self.controls {
            if control.visible() {
                // TODO: control.draw(canvas)?;
            }
        }
        Ok(())
    }

    /// 绘制顶层元素
    fn draw_top_layer(&mut self, canvas: &mut Canvas) -> GameResult<()> {
        // 1) 拖拽物品图标
        if self.picked_up_gold || self.selected_cell_item.is_some() {
            // TODO: 绘制拖拽图标
        }

        // 2) 输出消息 (左上角滚动提示)
        self.draw_output_messages(canvas)?;

        Ok(())
    }

    /// 绘制输出消息
    fn draw_output_messages(&mut self, canvas: &mut Canvas) -> GameResult<()> {
        // TODO: 实现输出行绘制
        Ok(())
    }

    /// 清理纹理缓存
    ///
    /// 对应 C# DXManager.CleanUp() 方法
    ///
    /// 清理所有 MapLibs 中超过指定时间未使用的纹理
    /// 清理纹理缓存 (对应 C# DXManager.CleanUp)
    ///
    /// C# 参考:
    /// ```csharp
    /// // DXManager.cs - CleanUp()
    /// for (int i = TextureList.Count - 1; i >= 0; i--) {
    ///     if (CMain.Time >= TextureList[i].CleanTime)
    ///         TextureList[i].DisposeTexture();
    /// }
    /// ```
    fn cleanup_texture_cache(&mut self) {
        use crate::graphics::get_all_map_libraries;
        use std::time::Duration;

        // 清理超过 30 秒未使用的纹理 (C# 使用 CleanDelay = 600000ms = 10分钟)
        // Rust 使用更激进的清理策略以节省内存
        let max_age = Duration::from_secs(30);
        let libs = get_all_map_libraries();
        let mut total_cleaned = 0;

        for (idx, lib) in libs.iter().enumerate() {
            if let Ok(mut library) = lib.lock() {
                let (before, _) = library.get_cache_stats();
                library.cleanup_old_textures(max_age);
                let (after, _) = library.get_cache_stats();

                let cleaned = before.saturating_sub(after);
                if cleaned > 0 {
                    total_cleaned += cleaned;
                    tracing::debug!(
                        "🧹 MapLib[{}]: cleaned {} textures ({} → {})",
                        idx,
                        cleaned,
                        before,
                        after
                    );
                }
            }
        }

        if total_cleaned > 0 {
            tracing::info!(
                "🧹 Texture cache cleanup: removed {} old textures",
                total_cleaned
            );
        }
    }

    // ==================== 网络协议处理 ====================
    // 对应 C# ProcessPacket (line 1384-5976)

    /// 绘制玩家角色（旧版本，已废弃，使用 draw_player_with_camera 代替）
    ///
    /// 简化版本：绘制一个简单的角色图像用于测试位置
    #[allow(dead_code)]
    fn draw_player(
        &self,
        ctx: &mut ggez::Context,
        canvas: &mut ggez::graphics::Canvas,
        user_pos: &UserPosition,
    ) -> ggez::GameResult<()> {
        use crate::graphics::libraries::{get_library, LibraryName};
        use ggez::graphics::{Color, DrawMode, DrawParam, Mesh, Rect};
        use mir2_shared::enums::{MirClass, MirGender};

        if let Some(ref user) = self.user {
            // 计算角色在屏幕上的绘制位置（屏幕中心）
            // 注意：offset_x/offset_y 已被摄像机系统取代
            let screen_center_x = (self.camera.screen_width / 2.0) as f32;
            let screen_center_y = (self.camera.screen_height / 2.0) as f32;

            // 🎨 先绘制一个简单的矩形作为角色占位符，确认位置
            let player_rect = Rect::new(screen_center_x - 20.0, screen_center_y - 40.0, 40.0, 60.0);
            let rect_mesh = Mesh::new_rectangle(
                ctx,
                DrawMode::stroke(2.0),
                player_rect,
                Color::from_rgb(0, 255, 0), // 绿色边框
            )?;
            canvas.draw(&rect_mesh, DrawParam::default());

            // 绘制一个填充的圆形表示角色位置
            let circle_mesh = Mesh::new_circle(
                ctx,
                DrawMode::fill(),
                [screen_center_x, screen_center_y],
                5.0,
                0.1,
                Color::from_rgb(255, 255, 0), // 黄色圆点
            )?;
            canvas.draw(&circle_mesh, DrawParam::default());

            // 🎨 使用 ChrSel 库绘制角色
            // 这个库包含角色选择界面的角色预览，可以用于测试
            // 帧索引计算：class_index * 40 + gender * 20 + direction
            let class_base = match user.player.class {
                MirClass::Warrior => 0,
                MirClass::Wizard => 40,
                MirClass::Taoist => 80,
                MirClass::Assassin => 120,
                MirClass::Archer => 160,
            };

            let gender_offset = match user.player.gender {
                MirGender::Male => 0,
                MirGender::Female => 20,
            };

            let direction = user.player.map_object.direction as usize;
            let frame_index = class_base + gender_offset + direction;

            // 🐛 调试：打印角色绘制信息
            static mut FIRST_PLAYER_DRAW: bool = true;
            unsafe {
                if FIRST_PLAYER_DRAW {
                    println!("\n👤 === 角色绘制调试 ===");
                    println!(
                        "角色位置: ({}, {})",
                        user.player.map_object.movement.x, user.player.map_object.movement.y
                    );
                    println!(
                        "屏幕中心位置: ({:.1}, {:.1})",
                        screen_center_x, screen_center_y
                    );
                    println!(
                        "绿色矩形: x={:.1}, y={:.1}, 宽=40, 高=60",
                        screen_center_x - 20.0,
                        screen_center_y - 40.0
                    );
                    println!("黄色圆点: ({:.1}, {:.1})", screen_center_x, screen_center_y);
                    println!("方向: {:?}", user.player.map_object.direction);
                    println!(
                        "性别: {:?}, 职业: {:?}",
                        user.player.gender, user.player.class
                    );
                    FIRST_PLAYER_DRAW = false;
                }
            }

            // 🎨 尝试使用 ChrSel 库绘制角色预览图
            let class_base = match user.player.class {
                MirClass::Warrior => 0,
                MirClass::Wizard => 40,
                MirClass::Taoist => 80,
                MirClass::Assassin => 120,
                MirClass::Archer => 160,
            };

            let gender_offset = match user.player.gender {
                MirGender::Male => 0,
                MirGender::Female => 20,
            };

            let direction = user.player.map_object.direction as usize;
            let frame_index = class_base + gender_offset + direction;

            if let Some(lib_arc) = get_library(LibraryName::ChrSel) {
                if let Ok(mut lib) = lib_arc.try_lock() {
                    // 检查图像数量
                    let image_count = lib.count();
                    unsafe {
                        if FIRST_PLAYER_DRAW {
                            println!("ChrSel 库图像数量: {}", image_count);
                            println!("====================\n");
                        }
                    }

                    // 确保索引有效
                    if frame_index >= image_count {
                        tracing::warn!("❌ 角色帧索引越界: {} >= {}", frame_index, image_count);
                        return Ok(());
                    }

                    // 绘制角色（使用偏移量居中显示）
                    match lib.draw_with_color(
                        ctx,
                        canvas,
                        frame_index,
                        screen_center_x,
                        screen_center_y - 20.0, // 稍微往上偏移，因为角色图片底部应该在脚下
                        Color::WHITE,
                        true, // use_offset = true，使用图像的偏移信息
                    ) {
                        Ok(_) => {}
                        Err(e) => {
                            tracing::error!("❌ 角色绘制失败 (索引 {}): {:?}", frame_index, e);
                        }
                    }
                } else {
                    tracing::warn!("❌ 无法锁定 ChrSel 库");
                }
            } else {
                tracing::error!("❌ ChrSel 库未加载！");
            }
        }

        Ok(())
    }

    /// 🎥 使用摄像机系统绘制玩家角色
    ///
    /// 与 draw_player 的区别：
    /// - 使用摄像机的世界坐标转屏幕坐标
    /// - 角色始终在屏幕中心
    /// - 支持缩放
    /// 绘制玩家角色 (使用摄像机坐标系统)
    ///
    /// ════════════════════════════════════════════════════════════
    /// 📝 玩家绘制流程详解
    /// ════════════════════════════════════════════════════════════
    ///
    /// 1. 计算玩家世界坐标 (像素)
    ///    world_x = grid_x * CELL_WIDTH + offset_x
    ///    world_y = grid_y * CELL_HEIGHT + offset_y
    ///
    /// 2. 世界坐标转屏幕坐标 (通过摄像机)
    ///    screen_x = world_x - camera.x + screen_width/2
    ///    screen_y = world_y - camera.y + screen_height/2
    ///
    /// 3. 绘制角色纹理
    ///    - 绿色矩形框 (占位符)
    ///    - 黄色圆点 (中心点标记)
    ///    - ChrSel 库的角色纹理 (实际角色图像)
    ///
    /// 4. 计算纹理索引
    ///    frame_index = class_base + gender_offset + direction
    ///    - Warrior: 0-19 (Male: 0-7, Female: 20-27)
    ///    - Wizard: 40-59
    ///    - Taoist: 80-99
    ///    - Assassin: 120-139
    ///    - Archer: 160-179
    /// ════════════════════════════════════════════════════════════
    fn draw_player_with_camera(
        &self,
        ctx: &mut ggez::Context,
        canvas: &mut ggez::graphics::Canvas,
        _user_pos: &UserPosition,
    ) -> ggez::GameResult<()> {
        use crate::graphics::libraries::{get_library, LibraryName};
        use ggez::graphics::{Color, DrawMode, DrawParam, Mesh, Rect};
        use mir2_shared::enums::{MirClass, MirGender};

        if let Some(ref user) = self.user {
            // ════════════════════════════════════════════════════════
            // 步骤 1: 计算玩家世界坐标（像素）
            // ════════════════════════════════════════════════════════
            let player_world_x = (user.player.map_object.movement.x as f32
                * MapRenderer::CELL_WIDTH as f32)
                + user.player.map_object.offset_move.x as f32;
            let player_world_y = (user.player.map_object.movement.y as f32
                * MapRenderer::CELL_HEIGHT as f32)
                + user.player.map_object.offset_move.y as f32;

            // ════════════════════════════════════════════════════════
            // 步骤 2: 世界坐标转屏幕坐标
            // ════════════════════════════════════════════════════════
            let (screen_x, screen_y) = self.camera.world_to_screen(player_world_x, player_world_y);

            // 🐛 DEBUG: 首帧详细打印玩家绘制信息
            static mut FIRST_DRAW_WITH_CAMERA: bool = true;
            unsafe {
                if FIRST_DRAW_WITH_CAMERA {
                    println!("╔════════════════════════════════════════════════════════════════");
                    println!("║ 👤 玩家角色绘制详细信息");
                    println!("╚════════════════════════════════════════════════════════════════");
                    println!(
                        "   📍 格子位置: ({}, {})",
                        user.player.map_object.movement.x, user.player.map_object.movement.y
                    );
                    println!(
                        "   📐 移动偏移: ({}, {})",
                        user.player.map_object.offset_move.x, user.player.map_object.offset_move.y
                    );
                    println!(
                        "   🌍 世界坐标: ({:.1}, {:.1}) 像素",
                        player_world_x, player_world_y
                    );
                    println!(
                        "   🎥 摄像机位置: ({:.1}, {:.1})",
                        self.camera.x, self.camera.y
                    );
                    println!("   🖥️  屏幕坐标: ({:.1}, {:.1})", screen_x, screen_y);
                    println!(
                        "   📺 屏幕尺寸: ({:.0}, {:.0})",
                        self.camera.get_screen_size().0,
                        self.camera.get_screen_size().1
                    );
                    println!("   🧭 朝向: {:?}", user.player.map_object.direction);
                    println!(
                        "   👤 性别: {:?}, 职业: {:?}",
                        user.player.gender, user.player.class
                    );
                    println!("════════════════════════════════════════════════════════════════\n");
                    FIRST_DRAW_WITH_CAMERA = false;
                }
            }

            // ════════════════════════════════════════════════════════
            // 步骤 3: 计算角色动画帧索引 (使用正确的帧计算)
            // ════════════════════════════════════════════════════════
            // CArmours 库帧布局:
            //   - Standing: 0-31   (8方向 * 4帧)
            //   - Walking:  32-79  (8方向 * 6帧)
            //   - Running:  80-127 (8方向 * 6帧)
            //   - Attack1:  128-175 (8方向 * 6帧)
            //   - Male: 0-xxx
            //   - Female: +808 offset
            //
            // 计算公式: DrawFrame + ArmourOffSet
            //   DrawFrame = action_frame_start + direction * frames_per_direction + frame_index
            //   ArmourOffSet = gender_offset (Male=0, Female=808)
            
            let final_frame = user.player.get_final_frame() as usize;

            tracing::trace!(
                "🎨 角色帧索引: {} (动作:{:?}, 方向:{}, 性别:{:?})",
                final_frame,
                user.player.current_action,
                user.player.map_object.direction as usize,
                user.player.gender
            );

            // ════════════════════════════════════════════════════════
            // 步骤 5: 绘制角色纹理 (CArmours 库 - 正确!)
            // ════════════════════════════════════════════════════════
            // 选择装备库:
            //   - Warrior/Wizard/Taoist: CArmours[armour_id]
            //   - Assassin: AArmours[armour_id]
            //   - Archer: CArmours or ARArmours (取决于动作)
            
            // 获取装备ID (如果为负数则使用默认值0)
            let armour_id = if user.player.armour < 0 {
                tracing::warn!("⚠️ 装备ID为负数 ({}), 使用默认值0", user.player.armour);
                0
            } else {
                user.player.armour as usize
            };
            
            let library_name = match user.player.class {
                MirClass::Warrior | MirClass::Wizard | MirClass::Taoist => {
                    // 通用装备库 CArmours
                    // 当前使用装备0 (默认服装)
                    LibraryName::CArmours(armour_id)
                }
                MirClass::Assassin => {
                    // 刺客专用库 AArmours
                    LibraryName::AArmours(armour_id)
                }
                MirClass::Archer => {
                    // 弓箭手: 根据动作选择库
                    // Walking/Running/Attack1 用 ARArmours, 其他用 CArmours
                    let alt_anim = matches!(
                        user.player.current_action,
                        MirAction::Walking | MirAction::Running | MirAction::Attack1
                    );
                    
                    if alt_anim {
                        LibraryName::ARArmours(armour_id)
                    } else {
                        LibraryName::CArmours(armour_id)
                    }
                }
            };
            
            if let Some(lib_arc) = get_library(library_name.clone()) {
                if let Ok(mut lib) = lib_arc.try_lock() {
                    let image_count = lib.count();

                    // 检查索引是否有效
                    if final_frame < image_count {
                        tracing::trace!("🎨 开始绘制 {:?}[{}] 纹理...", library_name, final_frame);

                        // 绘制角色身体 (使用摄像机缩放)
                        match lib.draw_with_scale(
                            ctx,
                            canvas,
                            final_frame,
                            screen_x,
                            screen_y,
                            Color::WHITE,
                            true, // use_offset (使用图像偏移量)
                            self.camera.zoom, // 应用摄像机缩放
                        ) {
                            Ok(_) => {
                                tracing::trace!("✅ {:?}[{}] 纹理绘制成功 (缩放: {:.2}x)", library_name, final_frame, self.camera.zoom);
                            }
                            Err(e) => {
                                tracing::error!("❌ {:?}[{}] 纹理绘制失败: {:?}", library_name, final_frame, e);
                            }
                        }
                    } else {
                        tracing::error!(
                            "❌ 角色纹理索引越界: {} >= {} (总图像数)",
                            final_frame,
                            image_count
                        );
                    }
                } else {
                    tracing::error!("❌ 无法锁定装备库 {:?}", library_name);
                }
            } else {
                tracing::error!("❌ 装备库 {:?} 未加载", library_name);
            }
        }

        Ok(())
    }

    /// 加载地图文件 (从 game_scene_old.rs 迁移)
    ///
    /// 对应 C# LoadMap 方法
    fn load_map_file(map_name: &str) -> std::io::Result<MapRenderer> {
        use crate::objects::MapReader;
        use std::path::PathBuf;

        // 尝试不同路径 - 优先 ClientRust/Map
        let paths = [
            PathBuf::from(format!("Map/{}.map", map_name)), // ClientRust/Map
            PathBuf::from(format!("./Map/{}.map", map_name)),
            PathBuf::from(format!("Data/Map/{}.map", map_name)),
            PathBuf::from(format!("./Data/Map/{}.map", map_name)),
            PathBuf::from(format!("../Data/Map/{}.map", map_name)),
            PathBuf::from(format!("../../Data/Map/{}.map", map_name)),
        ];

        for path in &paths {
            if path.exists() {
                tracing::info!("🗺️  Found map file: {:?}", path);
                match MapReader::new(path.to_str().unwrap()) {
                    Ok(reader) => {
                        // 🎨 新架构：直接返回 MapRenderer
                        return Ok(MapRenderer::from_reader(reader));
                    }
                    Err(e) => {
                        tracing::warn!("⚠️  Failed to parse map file {:?}: {}", path, e);
                        continue;
                    }
                }
            }
        }

        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Map file not found: {}", map_name),
        ))
    }

    // ==================== 输入处理 ====================
    // 对应 C# GameScene_KeyDown (line 1148-801)

    /// 处理键盘输入
    ///
    /// 对应 C# GameScene_KeyDown
    #[allow(unused_variables)]
    pub fn on_key_down(&mut self, key: ggez::input::keyboard::KeyCode) {
        // TODO: 映射 KeybindOptions
        // match keybind_option {
        //     KeybindOptions::Bar1Skill1 => self.use_spell(1),
        //     KeybindOptions::Inventory => self.toggle_inventory(),
        //     ...
        // }
    }


    /// 将屏幕坐标转换为地图坐标
    /// 
    /// 对应 C# ToMapLocation
    fn screen_to_map_location(&self, screen_pos: Point) -> Point {
        // 1. 屏幕坐标 -> 世界坐标（像素）
        let (world_x, world_y) = self.camera.screen_to_world(screen_pos.x as f32, screen_pos.y as f32);
        
        // 2. 世界坐标（像素）-> 地图格子坐标
        use crate::scenes::game_scene::map_renderer::MapRenderer;
        
        // 传奇2使用48x32的瓦片大小
        // 但渲染时使用了48x48的格子（CELL_WIDTH x CELL_HEIGHT）
        // 这里使用CELL_WIDTH和CELL_HEIGHT来转换
        let map_x = (world_x / MapRenderer::CELL_WIDTH as f32) as i32;
        let map_y = (world_y / MapRenderer::CELL_HEIGHT as f32) as i32;
        
        Point { x: map_x, y: map_y }
    }

    // ==================== 游戏逻辑 ====================

    /// 使用技能
    ///
    /// 对应 C# UseSpell (line 878-1021)
    #[allow(unused_variables)]
    pub fn use_spell(&mut self, key: i32) {
        // TODO: 实现技能释放逻辑
    }

    /// 输出消息
    ///
    /// 对应 C# OutputMessage (line 504-508)
    pub fn output_message(&mut self, message: String, message_type: OutputMessageType) {
        let expire_time = 0; // TODO: CMain.Time + 5000
        self.output_messages.push_back(OutputMessage {
            message,
            expire_time,
            message_type,
        });

        if self.output_messages.len() > self.max_output_messages {
            self.output_messages.pop_front();
        }
    }
    
    // ==================== 移动相关方法 ====================
    
    /// 检查是否可以向指定方向移动
    /// 
    /// 对应 C# GameScene.cs CanWalk(MirDirection dir)
    /// Reference: Client/MirScenes/GameScene.cs line 13174
    pub fn can_walk(&self, dir: MirDirection) -> bool {
        if let Some(ref user) = self.user {
            // 🔧 修复: 应该使用 current_location 而不是 movement
            // current_location 是逻辑位置，movement 是渲染位置
            let current_loc = user.player.map_object.current_location;
            let target_loc = self.point_move(current_loc, dir, 1);
            
            // 检查目标位置是否为空
            self.empty_cell(target_loc) && !user.player.map_object.in_trap_rock
        } else {
            false
        }
    }
    
    /// 检查指定格子是否可以移动到
    pub fn can_walk_to(&self, target: Point) -> bool {
        self.empty_cell(target)
    }
    
    /// 检查是否可以向指定方向移动，如果不行尝试相邻方向
    /// 
    /// 对应 C# GameScene.cs CanWalk(MirDirection dir, out MirDirection outDir)
    /// Reference: Client/MirScenes/GameScene.cs line 13179
    pub fn can_walk_adjust(&self, dir: MirDirection) -> Option<MirDirection> {
        if let Some(ref user) = self.user {
            if user.player.map_object.in_trap_rock {
                return None;
            }
            
            let current_loc = user.player.map_object.movement;
            
            // 首先尝试原方向
            let target_loc = self.point_move(current_loc, dir, 1);
            if self.empty_cell(target_loc) {
                return Some(dir);
            }
            
            // 尝试下一个方向 (顺时针)
            let next_dir = self.next_dir(dir);
            let target_loc = self.point_move(current_loc, next_dir, 1);
            if self.empty_cell(target_loc) {
                return Some(next_dir);
            }
            
            // 尝试上一个方向 (逆时针)
            let prev_dir = self.previous_dir(dir);
            let target_loc = self.point_move(current_loc, prev_dir, 1);
            if self.empty_cell(target_loc) {
                return Some(prev_dir);
            }
            
            None
        } else {
            None
        }
    }
    
    /// 检查格子是否为空 (没有障碍物)
    /// 
    /// 对应 C# GameScene.cs EmptyCell(Point p)
    fn empty_cell(&self, p: Point) -> bool {
        // 直接使用MapRenderer的is_walkable方法
        self.map_renderer.is_walkable(p.x, p.y)
    }
    
    /// 计算从某点向指定方向移动 n 格后的位置
    /// 
    /// 对应 C# Functions.PointMove(Point p, MirDirection d, int count)
    fn point_move(&self, p: Point, d: MirDirection, count: i32) -> Point {
        use mir2_shared::enums::MirDirection::*;
        
        match d {
            Up => Point { x: p.x, y: p.y - count },
            UpRight => Point { x: p.x + count, y: p.y - count },
            Right => Point { x: p.x + count, y: p.y },
            DownRight => Point { x: p.x + count, y: p.y + count },
            Down => Point { x: p.x, y: p.y + count },
            DownLeft => Point { x: p.x - count, y: p.y + count },
            Left => Point { x: p.x - count, y: p.y },
            UpLeft => Point { x: p.x - count, y: p.y - count },
        }
    }
    
    /// 根据两点计算方向
    /// 
    /// 对应 C# Functions.DirectionFromPoint(Point source, Point dest)
    /// 使用 SharedRust 的标准实现(基于象限判断,比角度计算更准确)
    fn direction_from_point(&self, source: Point, dest: Point) -> MirDirection {
        use mir2_shared::enums::MirDirection::*;
        
        if source.x < dest.x {
            if source.y < dest.y {
                return DownRight;
            }
            if source.y > dest.y {
                return UpRight;
            }
            return Right;
        }

        if source.x > dest.x {
            if source.y < dest.y {
                return DownLeft;
            }
            if source.y > dest.y {
                return UpLeft;
            }
            return Left;
        }

        if source.y < dest.y {
            Down
        } else {
            Up
        }
    }
    
    /// 获取下一个方向 (顺时针)
    /// 
    /// 对应 C# Functions.NextDir(MirDirection d)
    fn next_dir(&self, d: MirDirection) -> MirDirection {
        use mir2_shared::enums::MirDirection::*;
        
        match d {
            Up => UpRight,
            UpRight => Right,
            Right => DownRight,
            DownRight => Down,
            Down => DownLeft,
            DownLeft => Left,
            Left => UpLeft,
            UpLeft => Up,
        }
    }
    
    /// 计算从当前方向到目标方向的最近转向
    /// 
    /// 返回下一步应该朝向的方向(最多转一格)
    /// 这样可以实现平滑的方向转换,而不是直接跳转
    fn smooth_direction_change(&self, current: MirDirection, target: MirDirection) -> MirDirection {
        if current == target {
            return current;
        }
        
        // 计算顺时针和逆时针到目标的步数
        let mut clockwise_steps = 0;
        let mut dir = current;
        while dir != target && clockwise_steps < 8 {
            dir = self.next_dir(dir);
            clockwise_steps += 1;
        }
        
        let counter_clockwise_steps = 8 - clockwise_steps;
        
        // 选择最短路径
        if clockwise_steps <= counter_clockwise_steps {
            // 顺时针转一格
            self.next_dir(current)
        } else {
            // 逆时针转一格
            self.previous_dir(current)
        }
    }
    
    /// 获取上一个方向 (逆时针)
    /// 
    /// 对应 C# Functions.PreviousDir(MirDirection d)
    fn previous_dir(&self, d: MirDirection) -> MirDirection {
        use mir2_shared::enums::MirDirection::*;
        
        match d {
            Up => UpLeft,
            UpLeft => Left,
            Left => DownLeft,
            DownLeft => Down,
            Down => DownRight,
            DownRight => Right,
            Right => UpRight,
            UpRight => Up,
        }
    }

    /// 绘制加载屏幕（私有辅助方法）
    /// 会先绘制黑色背景覆盖游戏内容,然后显示加载文本
    fn draw_loading_screen(&self, canvas: &mut crate::graphics::Canvas, message: &str) {
        use ggez::glam::Vec2;
        use ggez::graphics::{Color, DrawParam, PxScale, Rect};

        tracing::info!(
            "📺 绘制加载屏幕: \"{}\" (屏幕: {:.0}x{:.0})",
            message,
            self.camera.screen_width,
            self.camera.screen_height
        );

        // 先绘制黑色背景覆盖整个屏幕 (覆盖绿色底色)
        canvas.draw(
            &ggez::graphics::Quad,
            DrawParam::default()
                .dest(Vec2::new(0.0, 0.0))
                .scale(Vec2::new(
                    self.camera.screen_width,
                    self.camera.screen_height,
                ))
                .color(Color::BLACK),
        );

        // 在屏幕中心绘制加载文本 (使用更大的字体)
        let mut text = ggez::graphics::Text::new(message);
        text.set_font("AlibabaPuHuiTi"); // 🔧 关键修复: 设置中文字体!
        text.set_scale(PxScale::from(32.0)); // 32px 大字体,更明显

        // 简单居中：估算文本宽度约为字符数 * 24px (32px字体的中文字符), 高度约 40px
        let estimated_width = message.chars().count() as f32 * 24.0;
        let text_x = (self.camera.screen_width - estimated_width) / 2.0;
        let text_y = (self.camera.screen_height - 40.0) / 2.0;

        tracing::info!(
            "📺   文本位置: ({:.0}, {:.0}), 估算宽度: {:.0}px",
            text_x,
            text_y,
            estimated_width
        );

        canvas.draw(
            &text,
            DrawParam::default()
                .dest(Vec2::new(text_x.max(0.0), text_y.max(0.0)))
                .color(Color::WHITE),
        );
    }
}

// MapControl 的实现在 map_control.rs 中

// ==================== Scene trait 实现 ====================

impl Scene for GameScene {
    fn scene_type(&self) -> SceneType {
        SceneType::Game
    }

    fn initialize(&mut self) {
        tracing::info!("🎮 GameScene V2 initializing...");

        // 🎥 初始化摄像机（使用默认屏幕尺寸 1024x768）
        // 实际尺寸会在 draw() 方法中根据窗口大小更新
        self.camera = Camera::new(1024.0, 768.0);
        tracing::info!("📷 Camera initialized: 1024x768");

        // TODO: 创建所有 UI 对话框
        // self.main_dialog = Some(MainDialog::new());
        // self.chat_dialog = Some(ChatDialog::new());
        // ... 40+ dialogs

        // TODO: 设置控件树 (Parent = this)
        // self.controls.push(Box::new(self.main_dialog));
        // self.controls.push(Box::new(self.chat_dialog));
        // ...

        tracing::info!("✅ GameScene V2 initialized!");
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn update(&mut self, ctx: &mut ggez::Context) {
        // 定期清理纹理缓存 (每 5 秒检查一次,清理超过 30 秒未使用的纹理)
        static mut LAST_CLEANUP_TIME: Option<std::time::Instant> = None;
        unsafe {
            let now = std::time::Instant::now();
            if LAST_CLEANUP_TIME.is_none()
                || now.duration_since(LAST_CLEANUP_TIME.unwrap())
                    > std::time::Duration::from_secs(5)
            {
                self.cleanup_texture_cache();
                LAST_CLEANUP_TIME = Some(now);
            }
        }
        
        // 1. 更新屏幕尺寸
        let (screen_width, screen_height) = ctx.gfx.drawable_size();
        self.camera.update_screen_size(screen_width, screen_height);
        
        // 2. 🆕 更新输入系统 (读取鼠标键盘状态)
        self.input_system.update(ctx);
        
        // 3. 🆕 处理玩家移动输入 (通过 ObjectManager)
        self.object_manager.handle_move_input(
            &self.input_system,
            &self.camera,
            &self.map_renderer,
            &self.command_tx,
        );
        
        // 4. 🆕 更新对象管理器 (FSM、动画、网络同步)
        let delta_time = ctx.time.delta().as_secs_f32();
        self.object_manager.update(delta_time, &self.command_tx);
        
        // 5. 同步玩家对象 (向后兼容)
        if let Some(user) = self.object_manager.user() {
            self.user = Some(user.clone());
        }
        
        // ==================== 角色移动状态机 (旧版逻辑 - 暂时禁用) ====================
        // ⚠️ 临时注释: 此代码与 ObjectManager.update() 冲突,导致 FSM 被更新两次
        // TODO: 将此逻辑完整迁移到 ObjectManager::handle_move_input()
        /*
        use ggez::input::mouse::MouseButton;
        let mouse_right_down = ctx.mouse.button_pressed(MouseButton::Right);
        let mouse_left_down = ctx.mouse.button_pressed(MouseButton::Left);
        let mouse_pos = ctx.mouse.position();
        let mouse_pos_point = Point { 
            x: mouse_pos.x as i32, 
            y: mouse_pos.y as i32 
        };
        
        // ... (旧版鼠标处理和移动逻辑)
        // 已被 ObjectManager.update() 替代
        */
        
        // TODO: 实现更新逻辑 (对应 C# Process)
        // 更新动画计数器
        // 更新对象
        // 更新 UI
    }

    /// 渲染场景 (Scene trait 要求的签名)
    ///
    /// ============================================================
    /// 🎨 GameScene 绘制流程详解
    /// ============================================================
    ///
    /// 📝 **绘制顺序**:
    /// 1. 清除整个屏幕 (深绿色背景) ← 防止登录场景残留
    /// 2. 检查状态机 (WaitingForData/LoadingMap/WaitingForPlayer/Ready)
    /// 3. 如果不是 Ready 状态，显示加载提示并返回
    /// 4. 如果是 Ready 状态:
    ///    a. 更新摄像机屏幕尺寸
    ///    b. 让摄像机跟随玩家 (带地图边界限制)
    ///    c. 绘制地图 (MapRenderer)
    ///    d. 绘制玩家角色
    ///    e. 绘制 UI 控件
    ///    f. 绘制顶层元素 (鼠标提示等)
    ///
    /// 🐛 **常见问题**:
    /// - 登录背景残留: Canvas 没清除 → 已修复 (第1步)
    /// - 地图不显示: 检查状态是否为 Ready
    /// - 玩家不显示: 检查 self.user 是否为 Some
    /// - 摄像机不跟随: 检查 follow_target_clamped 调用
    /// ============================================================
    fn draw(&mut self, ctx: &mut ggez::Context, canvas: &mut crate::graphics::Canvas) {
        let (screen_width, screen_height) = ctx.gfx.drawable_size();

        // 绘制帧计数 (调试时可打印)

        // ════════════════════════════════════════════════════════════
        // 步骤 3: 状态机检查 - 只有 Ready 状态才渲染游戏
        // ════════════════════════════════════════════════════════════
        // 📝 状态转换流程:
        //    WaitingForData → LoadingMap → WaitingForPlayer → Ready
        //    ↑                ↑             ↑                  ↑
        //    场景初始化       收到MapInfo   收到UserInfo       可以渲染游戏
        match &self.state {
            GameSceneState::WaitingForData => {
                // 显示 "等待游戏数据..." 提示
                self.draw_loading_screen(canvas, "等待服务器数据...");
                return;
            }
            GameSceneState::LoadingMap(map_name) => {
                // 显示 "正在加载地图: XXX" 提示
                let msg = format!("正在加载地图: {}", map_name);
                self.draw_loading_screen(canvas, &msg);
                return;
            }
            GameSceneState::WaitingForPlayer => {
                // 显示 "等待角色数据..." 提示
                self.draw_loading_screen(canvas, "等待角色数据...");
                return;
            }
            GameSceneState::Ready => {
                // ✅ 状态正常，继续渲染游戏
            }
        }

        // ════════════════════════════════════════════════════════════
        // 🔧 关键修复: 清空画布 - 防止其他场景背景残留!
        // ════════════════════════════════════════════════════════════
        // 📝 问题: 从 LoginScene/SelectScene 切换到 GameScene 时,
        //         旧场景的背景纹理会残留在画布上,因为 ggez 的 Canvas
        //         不会自动清空。
        //
        // 📝 解决方案: 在每帧开始时用黑色矩形覆盖整个屏幕。
        //             这样即使之前场景有背景,也会被清除干净。
        //
        // 📝 参考: SelectScene.rs 第795-802行也使用了相同的技巧
        use ggez::graphics::{Color as GgezColor, DrawMode, DrawParam, Mesh, Rect};
        let clear_rect = Rect::new(0.0, 0.0, screen_width, screen_height);
        let clear_color = GgezColor::from_rgb(0, 0, 0); // 黑色背景
        if let Ok(clear_mesh) = Mesh::new_rectangle(ctx, DrawMode::fill(), clear_rect, clear_color)
        {
            canvas.draw(&clear_mesh, DrawParam::default());
        }

        // ════════════════════════════════════════════════════════════
        // 步骤 4: 更新摄像机 (只有 Ready 状态才执行)
        // ════════════════════════════════════════════════════════════

        // 4a. 更新摄像机屏幕尺寸
        self.camera.update_screen_size(screen_width, screen_height);
        tracing::trace!(
            "🎥 摄像机屏幕尺寸已更新: {:.0}x{:.0}",
            screen_width,
            screen_height
        );

        // 4b. 更新摄像机跟随玩家（带地图边界限制）
        // 📝 摄像机跟随原理:
        //    - 使用 FSM 计算的平滑世界坐标
        //    - 摄像机居中对准玩家
        //    - 边界限制防止摄像机超出地图范围
        //    - 平滑插值防止抖动
        if let Some(ref user) = self.user {
            // 🔧 使用 FSM 的平滑世界坐标,而不是手动计算
            // 这样可以避免摄像机跳跃和抖动
            let (player_world_x, player_world_y) = user.movement_fsm.get_world_position(
                MapRenderer::CELL_WIDTH,
                MapRenderer::CELL_HEIGHT,
            );

            // 计算地图的像素尺寸
            let map_width_px = self.map_renderer.width as f32 * MapRenderer::CELL_WIDTH as f32;
            let map_height_px = self.map_renderer.height as f32 * MapRenderer::CELL_HEIGHT as f32;

            // 使用带边界限制和平滑插值的摄像机跟随
            self.camera.follow_target_clamped(
                player_world_x,
                player_world_y,
                map_width_px,
                map_height_px,
            );
        } else {
            tracing::warn!(
                "⚠️ GameScene::draw() 但 self.user 为 None，摄像机保持在: ({:.1}, {:.1})",
                self.camera.x,
                self.camera.y
            );
        }

        // ════════════════════════════════════════════════════════════
        // 步骤 5: 绘制地图与游戏对象
        // ════════════════════════════════════════════════════════════
        // 📝 绘制顺序:
        //    1. 地图 (MapRenderer) - 分层绘制 (Tiles, Front)
        //    2. 玩家角色 (draw_player_with_camera)
        //    3. 其他对象 (NPC, 怪物等) - TODO

        // 5a. 准备玩家位置数据
        let user_pos = if let Some(ref user) = self.user {
            UserPosition {
                x: user.player.map_object.movement.x,
                y: user.player.map_object.movement.y,
                offset_x: user.player.map_object.offset_move.x,
                offset_y: user.player.map_object.offset_move.y,
            }
        } else {
            // 没有用户时,使用地图中心
            UserPosition {
                x: self.map_renderer.width / 2,
                y: self.map_renderer.height / 2,
                offset_x: 0,
                offset_y: 0,
            }
        };

        // ════════════════════════════════════════════════════════════
        // 🆕 使用 RenderingPipeline 进行渲染
        // ════════════════════════════════════════════════════════════
        tracing::trace!("🎨 使用 RenderingPipeline 渲染场景...");
        if let Err(e) = self.rendering_pipeline.render(
            ctx,
            canvas,
            &mut self.map_renderer,
            &self.camera,
            &self.object_manager
        ) {
            tracing::error!("❌ RenderingPipeline 渲染失败: {:?}", e);
        } else {
            tracing::trace!("✅ RenderingPipeline 渲染成功");
        }
        
        // ════════════════════════════════════════════════════════════
        // 🔧 旧版玩家绘制 (暂时保留用于向后兼容)
        // TODO: 逐步迁移到 RenderingPipeline
        // ════════════════════════════════════════════════════════════
        
        // 5c. 绘制玩家角色 (使用摄像机转换坐标)
        if self.user.is_some() && self.show_player {
            tracing::trace!("👤 开始绘制玩家角色 (旧版)...");
            if let Err(e) = self.draw_player_with_camera(ctx, canvas, &user_pos) {
                tracing::error!("❌ 玩家绘制失败: {:?}", e);
            } else {
                tracing::trace!("✅ 玩家绘制成功");
            }
        } else if !self.show_player {
            tracing::trace!("👤 玩家显示已关闭 (U键控制)");
        } else {
            tracing::warn!("⚠️  没有玩家数据，跳过玩家绘制");
        }

        // 5d. 绘制障碍物标记 (调试用)
        if let Some(blocked_pos) = self.blocked_cell {
            // 将地图坐标转换为世界坐标（像素）
            let world_x = blocked_pos.x as f32 * MapRenderer::CELL_WIDTH as f32;
            let world_y = blocked_pos.y as f32 * MapRenderer::CELL_HEIGHT as f32;
            
            // 转换为屏幕坐标
            let screen_pos = self.camera.world_to_screen(world_x, world_y);
            
            // 绘制红色半透明矩形标记障碍物
            use ggez::graphics::{Color as GgezColor, DrawMode, DrawParam, Mesh, Rect};
            let obstacle_rect = Rect::new(
                screen_pos.0,
                screen_pos.1,
                MapRenderer::CELL_WIDTH as f32,
                MapRenderer::CELL_HEIGHT as f32,
            );
            let obstacle_color = GgezColor::from_rgba(255, 0, 0, 128); // 半透明红色
            if let Ok(obstacle_mesh) = Mesh::new_rectangle(ctx, DrawMode::fill(), obstacle_rect, obstacle_color) {
                canvas.draw(&obstacle_mesh, DrawParam::default());
            }
            
            // 绘制边框
            let border_color = GgezColor::from_rgb(255, 0, 0); // 纯红色
            if let Ok(border_mesh) = Mesh::new_rectangle(ctx, DrawMode::stroke(2.0), obstacle_rect, border_color) {
                canvas.draw(&border_mesh, DrawParam::default());
            }
        }

        // ════════════════════════════════════════════════════════════
        // 步骤 6: 绘制 UI 控件树 (TODO)
        // ════════════════════════════════════════════════════════════
        // TODO: 遍历 self.controls 并调用 draw
        tracing::trace!("🎨 UI 控件绘制 (暂未实现)");

        // ════════════════════════════════════════════════════════════
        // 步骤 7: 绘制顶层元素 (TODO)
        // ════════════════════════════════════════════════════════════
        // TODO: 绘制鼠标提示、输出消息、对话框等
        tracing::trace!("✨ 顶层元素绘制 (暂未实现)");

        // ════════════════════════════════════════════════════════════
        // 步骤 8: 绘制FPS显示
        // ════════════════════════════════════════════════════════════
        {
            use ggez::graphics::{Text, Color as GgezColor, DrawParam};
            
            // 计算FPS
            let fps = ctx.time.fps();
            
            // 创建FPS文本
            let fps_text = format!("FPS: {:.0}", fps);
            let mut text = Text::new(fps_text);
            text.set_scale(24.0);
            
            // 绘制在左上角
            let draw_param = DrawParam::default()
                .dest([10.0, 10.0])
                .color(GgezColor::from_rgb(255, 255, 0)); // 黄色
            
            canvas.draw(&text, draw_param);
        }

        // ════════════════════════════════════════════════════════════
        // 绘制完成
        // ════════════════════════════════════════════════════════════
        tracing::trace!("🎬 GameScene::draw() 完成");
    }

    /// 处理游戏事件 (从 game_scene_old.rs 迁移)
    ///
    /// 对应 C# ProcessPacket 的各个分支
    fn process_event(&mut self, event: &GameEvent) {
        // 🐛 DEBUG: 强制打印所有收到的事件
        println!("╔════════════════════════════════════════════════════════════════");
        println!("║ 🎮 GameScene.process_event() 被调用!");
        println!("╚════════════════════════════════════════════════════════════════");
        println!("   事件类型: {:?}", std::mem::discriminant(event));
        println!("   当前状态: {:?}", self.state);
        println!("════════════════════════════════════════════════════════════════\n");

        tracing::debug!("📨 GameScene 收到事件: {:?}", std::mem::discriminant(event));

        match event {
            GameEvent::MapInformation {
                map_index: _,
                file_name,
                title,
            } => {
                tracing::info!("🗺️  ========================================");
                tracing::info!("🗺️  收到服务器地图信息:");
                tracing::info!("🗺️    地图名称: {}", title);
                tracing::info!("🗺️    文件名: {}", file_name);
                tracing::info!("🗺️  ========================================");

                // 🔄 状态转换: WaitingForData → LoadingMap
                self.state = GameSceneState::LoadingMap(file_name.clone());
                tracing::info!("🔄 状态切换: WaitingForData → LoadingMap({})", file_name);

                // 🎨 加载地图到 MapRenderer
                match Self::load_map_file(file_name) {
                    Ok(map_renderer) => {
                        tracing::info!("✅ 地图加载成功:");
                        tracing::info!(
                            "   - 地图尺寸: {} x {} 格子",
                            map_renderer.width,
                            map_renderer.height
                        );
                        tracing::info!(
                            "   - 像素尺寸: {:.1} x {:.1} 像素",
                            map_renderer.width as f32 * MapRenderer::CELL_WIDTH as f32,
                            map_renderer.height as f32 * MapRenderer::CELL_HEIGHT as f32
                        );
                        self.map_renderer = map_renderer;
                        
                        // 🆕 地图已经加载到 self.map_renderer
                        // RenderingPipeline 通过引用访问，无需复制

                        // 🔧 状态转换: 地图加载完成
                        println!(
                            "╔════════════════════════════════════════════════════════════════"
                        );
                        println!("║ 🔄 状态转换检查 - MapInformation");
                        println!(
                            "╚════════════════════════════════════════════════════════════════"
                        );
                        println!("   当前状态: {:?}", self.state);
                        println!("   地图已加载: {}", self.map_renderer.width > 0);
                        println!("   玩家已创建: {}", self.user.is_some());

                        if self.user.is_some() {
                            // 玩家数据已存在 → Ready
                            self.state = GameSceneState::Ready;
                            println!("   ✅ 状态切换: LoadingMap → Ready (玩家数据已存在) ⭐⭐⭐");
                            tracing::info!("🔄 状态切换: LoadingMap → Ready (玩家数据已存在)");
                        } else {
                            // 等待玩家数据
                            self.state = GameSceneState::WaitingForPlayer;
                            println!("   ⏳ 状态切换: LoadingMap → WaitingForPlayer");
                            tracing::info!("🔄 状态切换: LoadingMap → WaitingForPlayer");
                        }
                        println!("   切换后状态: {:?}", self.state);
                        println!(
                            "════════════════════════════════════════════════════════════════\n"
                        );

                        // 🔧 如果用户已经存在，更新摄像机到玩家位置
                        if let Some(ref user) = self.user {
                            let player_world_x = (user.player.map_object.movement.x as f32
                                * MapRenderer::CELL_WIDTH as f32)
                                + user.player.map_object.offset_move.x as f32;
                            let player_world_y = (user.player.map_object.movement.y as f32
                                * MapRenderer::CELL_HEIGHT as f32)
                                + user.player.map_object.offset_move.y as f32;

                            let map_width_px =
                                self.map_renderer.width as f32 * MapRenderer::CELL_WIDTH as f32;
                            let map_height_px =
                                self.map_renderer.height as f32 * MapRenderer::CELL_HEIGHT as f32;

                            tracing::info!("🎥 地图加载后更新摄像机:");
                            tracing::info!(
                                "   玩家位置: ({:.1}, {:.1})",
                                player_world_x,
                                player_world_y
                            );
                            tracing::info!(
                                "   摄像机更新前: ({:.1}, {:.1})",
                                self.camera.x,
                                self.camera.y
                            );

                            self.camera.follow_target_clamped(
                                player_world_x,
                                player_world_y,
                                map_width_px,
                                map_height_px,
                            );

                            tracing::info!(
                                "   摄像机更新后: ({:.1}, {:.1})",
                                self.camera.x,
                                self.camera.y
                            );
                        }
                    }
                    Err(e) => {
                        tracing::error!("❌ Failed to load map {}: {}", file_name, e);
                        // 创建空白地图作为后备
                        self.map_renderer = MapRenderer::default();
                    }
                }
            }

            GameEvent::UserInformation { user_info } => {
                tracing::info!("👤 ========================================");
                tracing::info!("👤 收到服务器玩家信息:");
                tracing::info!("👤   玩家名称: {}", user_info.name);
                tracing::info!("👤   ObjectID: {}", user_info.object_id);
                tracing::info!(
                    "👤   位置: ({}, {}) ⭐ 这是玩家的初始位置!",
                    user_info.location_x,
                    user_info.location_y
                );
                tracing::info!("👤   方向: {:?}", user_info.direction);
                tracing::info!(
                    "👤   职业: {:?}, 等级: {}",
                    user_info.class,
                    user_info.level
                );
                tracing::info!("👤   金币: {}, 点券: {}", user_info.gold, user_info.credit);
                tracing::info!("👤 ========================================");

                // 🔧 CRITICAL FIX: Create user object from UserInformation packet
                // This is the PRIMARY way to create the player object
                use mir2_shared::packets::server::ObjectPlayer;

                let object_player = ObjectPlayer {
                    object_id: user_info.object_id,
                    name: user_info.name.clone(),
                    guild_name: user_info.guild_name.clone(),
                    guild_rank_name: user_info.guild_rank.clone(),
                    name_colour: user_info.name_colour,
                    class: user_info.class,
                    gender: user_info.gender,
                    level: user_info.level,
                    location_x: user_info.location_x,
                    location_y: user_info.location_y,
                    direction: user_info.direction,
                    hair: user_info.hair,
                    light: 0,
                    weapon: -1, // Will be set from equipment
                    weapon_effect: 0,
                    armour: -1, // Will be set from equipment
                    poison: mir2_shared::PoisonType::empty(),
                    dead: false,
                    hidden: false,
                    effect: mir2_shared::enums::SpellEffect::None,
                    wing_effect: 0,
                    extra: false,
                    mount_type: -1,
                    riding_mount: false,
                    fishing: false,
                    transform_type: 0,
                    element_orb_effect: 0,
                    element_orb_lvl: 0,
                    element_orb_max: 0,
                    buffs: Vec::new(),
                    level_effects: user_info.level_effects,
                };

                // 使用 ObjectFactory 创建玩家对象
                let user_obj = crate::objects::ObjectFactory::create_player(&object_player);

                tracing::info!("✅ ========================================");
                tracing::info!("✅ 玩家对象创建成功:");
                tracing::info!("✅   ObjectID: {}", user_obj.player.map_object.object_id());
                tracing::info!("✅   玩家名称: {}", user_obj.player.map_object.name);
                tracing::info!(
                    "✅   CurrentLocation: ({}, {})",
                    user_obj.player.map_object.current_location.x,
                    user_obj.player.map_object.current_location.y
                );
                tracing::info!(
                    "✅   Movement: ({}, {}) ⭐ 这个位置用于摄像机跟随!",
                    user_obj.player.map_object.movement.x,
                    user_obj.player.map_object.movement.y
                );
                tracing::info!(
                    "✅   Offset: ({}, {})",
                    user_obj.player.map_object.offset_move.x,
                    user_obj.player.map_object.offset_move.y
                );
                tracing::info!(
                    "✅   职业: {:?}, 等级: {}",
                    user_obj.player.class,
                    user_obj.player.level
                );

                // 计算玩家的世界坐标（像素）
                let player_world_x = (user_obj.player.map_object.movement.x as f32
                    * MapRenderer::CELL_WIDTH as f32)
                    + user_obj.player.map_object.offset_move.x as f32;
                let player_world_y = (user_obj.player.map_object.movement.y as f32
                    * MapRenderer::CELL_HEIGHT as f32)
                    + user_obj.player.map_object.offset_move.y as f32;
                tracing::info!(
                    "✅   世界坐标 (像素): ({:.1}, {:.1})",
                    player_world_x,
                    player_world_y
                );
                tracing::info!("✅ ========================================");

                // 🆕 同步玩家到 ObjectManager
                self.object_manager.set_user(user_obj.clone());
                self.user = Some(user_obj);

                // 🔧 状态转换: 玩家数据到达
                println!("╔════════════════════════════════════════════════════════════════");
                println!("║ 🔄 状态转换检查 - UserInformation");
                println!("╚════════════════════════════════════════════════════════════════");
                println!("   当前状态: {:?}", self.state);
                println!("   地图已加载: {}", self.map_renderer.width > 0);
                println!("   玩家已创建: {}", self.user.is_some());
                println!("   ObjectManager 玩家: {}", self.object_manager.user().is_some());

                match self.state {
                    GameSceneState::WaitingForData => {
                        // 地图还未加载 → WaitingForData (不变)
                        println!("   ❌ 玩家数据到达,但地图还未加载 (保持 WaitingForData)");
                        tracing::info!("🔄 玩家数据到达,但地图还未加载 (保持 WaitingForData)");
                    }
                    GameSceneState::LoadingMap(_) => {
                        // 地图正在加载 → 等待地图加载完成
                        println!("   ⏳ 玩家数据到达,等待地图加载完成");
                        tracing::info!("🔄 玩家数据到达,等待地图加载完成");
                    }
                    GameSceneState::WaitingForPlayer => {
                        // 地图已加载 → Ready
                        self.state = GameSceneState::Ready;
                        println!("   ✅ 状态切换: WaitingForPlayer → Ready ⭐⭐⭐");
                        tracing::info!("🔄 状态切换: WaitingForPlayer → Ready");
                    }
                    GameSceneState::Ready => {
                        // 已就绪,不变
                        println!("   ✅ 已经是 Ready 状态");
                    }
                }
                println!("   切换后状态: {:?}", self.state);
                println!("════════════════════════════════════════════════════════════════\n");

                // 🔧 立即更新摄像机到玩家位置
                let map_width_px = self.map_renderer.width as f32 * MapRenderer::CELL_WIDTH as f32;
                let map_height_px =
                    self.map_renderer.height as f32 * MapRenderer::CELL_HEIGHT as f32;

                tracing::info!("🎥 ========================================");
                tracing::info!("🎥 初始化摄像机位置:");
                tracing::info!(
                    "🎥   地图尺寸 (像素): {:.1} x {:.1}",
                    map_width_px,
                    map_height_px
                );
                tracing::info!(
                    "🎥   摄像机更新前: ({:.1}, {:.1})",
                    self.camera.x,
                    self.camera.y
                );

                if map_width_px > 0.0 && map_height_px > 0.0 {
                    // 地图已加载，使用带边界限制的跟随
                    self.camera.follow_target_clamped(
                        player_world_x,
                        player_world_y,
                        map_width_px,
                        map_height_px,
                    );
                    tracing::info!("🎥   使用带边界限制的跟随");
                } else {
                    // 地图还未加载，直接设置摄像机位置（不限制边界）
                    self.camera.follow_target(player_world_x, player_world_y);
                    tracing::info!("🎥   地图未加载，直接设置摄像机位置");
                }

                tracing::info!(
                    "🎥   摄像机更新后: ({:.1}, {:.1})",
                    self.camera.x,
                    self.camera.y
                );
                tracing::info!("🎥 ========================================");

                // Update inventory, equipment, gold, etc.
                self.gold = user_info.gold;
                self.credit = user_info.credit;
                if let Some(ref inv) = user_info.inventory {
                    // Convert Vec to array, fallback to empty array if length mismatch
                    if inv.len() == 46 {
                        self.inventory = inv.clone().try_into().unwrap();
                    }
                }
                if let Some(ref equip) = user_info.equipment {
                    if equip.len() == 14 {
                        self.equipment = equip.clone().try_into().unwrap();
                    }
                }

                tracing::info!("✅ User state fully initialized!");
            }

            GameEvent::PlayerSpawned { player } => {
                tracing::info!(
                    "👤 Player spawned: {} at ({}, {})",
                    player.name,
                    player.location.x,
                    player.location.y
                );

                // 🔧 CRITICAL: Update user object position if it exists
                if let Some(ref mut user) = self.user {
                    tracing::info!(
                        "✅ Updating user position from PlayerSpawned: ({}, {})",
                        player.location.x,
                        player.location.y
                    );
                    user.player.map_object.set_current_location(player.location);
                    tracing::info!(
                        "✅ User object synced: current_location=({}, {}), movement=({}, {})",
                        user.player.map_object.current_location.x,
                        user.player.map_object.current_location.y,
                        user.player.map_object.movement.x,
                        user.player.map_object.movement.y
                    );
                } else {
                    tracing::warn!("⚠️  PlayerSpawned received but user object doesn't exist yet (will be created from UserInformation)");
                }
            }

            GameEvent::PlayerMoved { location } => {
                if let Some(ref mut user) = self.user {
                    tracing::debug!("🚶 Player moved to: ({}, {})", location.x, location.y);
                    // 🔧 CRITICAL FIX: Update both current_location AND movement
                    // This synchronizes the rendering position with the actual map position
                    user.player.map_object.set_current_location(*location);
                    tracing::debug!(
                        "✅ User position synced: current_location={:?}, movement={:?}",
                        user.player.map_object.current_location,
                        user.player.map_object.movement
                    );
                }
            }

            GameEvent::ObjectSpawned { object } => {
                use crate::network::game_client::GameObject;

                match object {
                    GameObject::Player { id, name, .. } => {
                        tracing::info!("👤 Player spawned: {} ({})", name, id);
                    }
                    GameObject::Monster { id, name, .. } => {
                        tracing::info!("👹 Monster spawned: {} ({})", name, id);
                    }
                    GameObject::Npc { id, name, .. } => {
                        tracing::info!("🧙 NPC spawned: {} ({})", name, id);
                    }
                    GameObject::Item { id, .. } => {
                        tracing::info!("💎 Item spawned: {}", id);
                    }
                }
            }

            GameEvent::ObjectRemoved { object_id } => {
                tracing::debug!("🗑️  Object removed: {}", object_id);
                // TODO: self.remove_object(*object_id);
            }

            GameEvent::ChatReceived { message } => {
                self.output_message(message.text.clone(), OutputMessageType::Normal);
            }

            GameEvent::GoldChanged { gold } => {
                self.gold = *gold;
                tracing::debug!("💰 Gold changed: {}", gold);
            }

            GameEvent::SystemMessage { message } => {
                self.output_message(message.clone(), OutputMessageType::Normal);
            }

            GameEvent::ItemGained { item, grid_type } => {
                tracing::info!("🎁 Item gained: {:?} in {}", item, grid_type);
            }

            GameEvent::MagicCast { spell, target_id } => {
                tracing::debug!("✨ Magic cast: {:?} on target {}", spell, target_id);
            }

            _ => {
                tracing::warn!("⚠️  Unhandled game event: {:?}", event);
            }
        }
    }

    /// 处理鼠标移动事件 - 覆盖Scene trait的默认实现
    fn handle_mouse_move(&mut self, x: i32, y: i32) {
        // 更新鼠标位置
        self.mouse_location = Point { x, y };
    }
    
    /// 处理鼠标按钮事件
    /// 注意：游戏一般不使用事件驱动，而是在 update() 中主动轮询鼠标状态
    fn handle_mouse_button(&mut self, _button: super::MouseButton, _pressed: bool, _x: i32, _y: i32) {
        // 不需要实现，鼠标状态在 update() 中主动查询
    }
    
    /// 处理鼠标滚轮事件（用于地图和窗口缩放）
    fn handle_mouse_wheel(&mut self, _delta_x: f32, delta_y: f32) {
        // delta_y > 0: 向上滚动 (放大)
        // delta_y < 0: 向下滚动 (缩小)

        // 获取当前缩放级别
        let current_zoom = self.camera.zoom;

        // 缩放速度：每次滚动改变 10%
        let zoom_factor = if delta_y > 0.0 { 1.1 } else { 0.9 };
        let new_zoom = current_zoom * zoom_factor;

        // 限制缩放范围：0.5x ~ 3.0x
        let clamped_zoom = new_zoom.max(0.5).min(3.0);

        // 应用新的缩放级别
        self.camera.set_zoom(clamped_zoom);

        tracing::debug!(
            "🔍 Camera zoom changed: {:.2}x -> {:.2}x (wheel delta: {:.1})",
            current_zoom,
            clamped_zoom,
            delta_y
        );
    }

    /// 🎮 处理键盘按键事件 - MapRenderer 显示控制
    ///
    /// 快捷键列表:
    /// - G键: 切换地图网格
    /// - B键: 切换纹理边框
    /// - 1键: 切换 Back 层
    /// - 2键: 切换 Middle 层
    /// - 3键: 切换 Front 层
    /// - O键: 切换障碍层
    /// - A键: 切换动画效果
    fn handle_key_press(&mut self, key: KeyCode, _modifiers: ModifiersState) -> bool {
        use crate::scenes::KeyCode;

        match key {
            // G键: 切换地图网格
            KeyCode::KeyG => {
                self.map_renderer.show_grid = !self.map_renderer.show_grid;
                println!(
                    "🔍 地图网格: {}",
                    if self.map_renderer.show_grid {
                        "开启"
                    } else {
                        "关闭"
                    }
                );
                true // 已处理
            }

            // B键: 切换纹理边框
            KeyCode::KeyB => {
                self.map_renderer.show_borders = !self.map_renderer.show_borders;
                println!(
                    "🔍 纹理边框: {}",
                    if self.map_renderer.show_borders {
                        "开启"
                    } else {
                        "关闭"
                    }
                );
                true
            }

            // 1键: 切换 Back 层
            KeyCode::Digit1 => {
                self.map_renderer.show_layer_back = !self.map_renderer.show_layer_back;
                println!(
                    "🎨 Back层: {}",
                    if self.map_renderer.show_layer_back {
                        "开启"
                    } else {
                        "关闭"
                    }
                );
                true
            }

            // 2键: 切换 Middle 层
            KeyCode::Digit2 => {
                self.map_renderer.show_layer_middle = !self.map_renderer.show_layer_middle;
                println!(
                    "🎨 Middle层: {}",
                    if self.map_renderer.show_layer_middle {
                        "开启"
                    } else {
                        "关闭"
                    }
                );
                true
            }

            // 3键: 切换 Front 层
            KeyCode::Digit3 => {
                self.map_renderer.show_layer_front = !self.map_renderer.show_layer_front;
                println!(
                    "🎨 Front层: {}",
                    if self.map_renderer.show_layer_front {
                        "开启"
                    } else {
                        "关闭"
                    }
                );
                true
            }

            // O键: 切换障碍层
            KeyCode::KeyO => {
                self.map_renderer.show_obstacles = !self.map_renderer.show_obstacles;
                println!(
                    "🚧 障碍层: {}",
                    if self.map_renderer.show_obstacles {
                        "开启"
                    } else {
                        "关闭"
                    }
                );
                true
            }

            // A键: 切换动画效果
            KeyCode::KeyA => {
                self.map_renderer.show_animations = !self.map_renderer.show_animations;
                println!(
                    "🎬 动画效果: {}",
                    if self.map_renderer.show_animations {
                        "开启"
                    } else {
                        "关闭"
                    }
                );
                true
            }

            // U键: 切换玩家显示
            KeyCode::KeyU => {
                self.show_player = !self.show_player;
                println!(
                    "👤 玩家显示: {}",
                    if self.show_player {
                        "开启"
                    } else {
                        "关闭"
                    }
                );
                true
            }

            // 其他按键不处理
            _ => false,
        }
    }
}

// ==================== 辅助函数 ====================

/// 获取当前时间戳 (毫秒)
#[allow(dead_code)]
fn current_time_millis() -> i64 {
    // TODO: 实现
    0
}

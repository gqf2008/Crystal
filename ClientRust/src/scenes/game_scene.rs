// GameScene V2 - Refactored to mirror C# GameScene.cs architecture
// Reference: Client/MirScenes/GameScene.cs
//
// ARCHITECTURE PRINCIPLES:
// 1. GameScene is the central hub managing ALL game state
// 2. MapControl handles map rendering (nested functionality)
// 3. UI controls managed through control tree (Parent = this)
// 4. Network packets processed centrally via process_packet()
// 5. Rendering phases: MapControl.draw() → UI tree → Top layer

use std::collections::{HashMap, VecDeque};
use ggez::GameResult;
use ggez::graphics::Canvas;

use mir2_shared::{
    enums::*,
    Point,
    UserItem,                         // ✅ Shared/Data/ItemData.cs line 277
    data::client_data::{
        ClientMagic,                  // ✅ SharedRust/src/data/client_data.rs line 70
        ClientBuff,                   // ✅ SharedRust/src/data/client_data.rs line 764
        ClientQuestInfo,              // ✅ SharedRust/src/data/client_data.rs line 392
        ClientFriend,                 // ✅ SharedRust/src/data/client_data.rs line 885 (Shared/Data/ClientData.cs line 122)
        ClientMail,                   // ✅ SharedRust/src/data/client_data.rs line 922 (Shared/Data/ClientData.cs line 154)
    },
    data::shared_data::{
        RankCharacterInfo,            // ✅ SharedRust/src/data/shared_data.rs line 92 (Shared/Data/SharedData.cs line 43)
    },
};

use crate::controls::Control;
use crate::objects::{
    UserObject, HeroObject, MapObject,
};
use crate::scenes::{Scene, SceneType, GameEvent};

// 导入 MapControl (从 game_scene 子模块)
pub mod map_control;
pub use map_control::MapControl;

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
    // ==================== 玩家与英雄 ====================
    // C# line 27-48
    
    /// 当前玩家对象 (C#: public static UserObject User)
    user: Option<UserObject>,
    
    /// 英雄对象 (C#: public static UserHeroObject Hero)
    hero: Option<HeroObject>,
    
    /// 是否拥有英雄 (C#: public bool HasHero)
    has_hero: bool,
    
    /// 英雄召唤状态 (C#: public HeroSpawnState HeroSpawnState)
    hero_spawn_state: HeroSpawnState,
    
    // ==================== 对象管理 ====================
    // C# line 50-58
    
    /// 所有地图对象 (C#: 通过 MapObject.User/MapObject.Objects 管理)
    /// 注意: C# 使用静态字典,Rust 用实例字段
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
    
    // ==================== MapControl (嵌套功能) ====================
    // 对应 C# MapControl 嵌套类 (line 10209-11241)
    
    /// 地图控制器 (C#: public MapControl MapControl)
    map_control: Option<MapControl>,
    
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
}

// ==================== MapControl 已迁移到子模块 ====================
//
// MapControl 现在位于 src/scenes/game_scene/map_control.rs
// 
// 对应 C# MapControl 嵌套类 (line 10209-11241)
// 
// 虽然 C# 中 MapControl 是 GameScene 的嵌套类,
// 但 Rust 中我们将其作为独立模块实现,便于:
// - 代码组织和维护
// - 单元测试
// - 避免单文件过大
//
// 导入: use crate::scenes::game_scene::MapControl;
//
// 同样,M2CellInfo 使用 objects::CellInfo (已在 objects 模块中实现)

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
        Self {
            // 玩家与英雄
            user: None,
            hero: None,
            has_hero: false,
            hero_spawn_state: HeroSpawnState::None,
            
            // 对象管理
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
            
            // MapControl
            map_control: None,
            
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
        }
    }
    
    
    /// 加载地图
    /// 
    /// 对应 C# LoadMap 方法
    /// 
    /// 参数:
    /// - map_path: 地图文件路径 (如 "Maps/0.map")
    pub fn load_map(&mut self, map_path: &str) -> GameResult<()> {
        use crate::objects::MapReader;
        
        tracing::info!("🗺️  Loading map: {}", map_path);
        
        // 加载地图数据
        match MapReader::new(map_path) {
            Ok(reader) => {
                // 创建 MapControl
                let map_control = MapControl::from_map_reader(reader);
                
                tracing::info!("✅ Map loaded: {} ({}x{})", 
                    map_control.filename, 
                    map_control.width, 
                    map_control.height
                );
                
                self.map_control = Some(map_control);
                Ok(())
            }
            Err(e) => {
                tracing::error!("❌ Failed to load map: {}", e);
                Err(ggez::GameError::CustomError(format!("Failed to load map: {}", e)))
            }
        }
    }

    
    // ==================== 主渲染方法 ====================
    // 对应 C# DrawControl (line 1062-1086)
    
    /// 绘制场景
    /// 
    /// 渲染分三个阶段:
    /// 1. MapControl.draw() - 地图与对象
    /// 2. UI 控件树 - 所有对话框
    /// 3. 顶层元素 - 拖拽物品/输出消息
    /// 
    /// 注意: 此方法已弃用,使用 Scene trait 的 draw 方法
    #[allow(dead_code)]
    pub fn draw_old(&mut self, ctx: &mut ggez::Context, canvas: &mut Canvas) -> GameResult<()> {
        // 定期清理纹理缓存 (每 5 分钟清理一次超过 10 分钟未使用的纹理)
        // 对应 C# DXManager.CleanUp() 
        static mut LAST_CLEANUP_TIME: Option<std::time::Instant> = None;
        unsafe {
            let now = std::time::Instant::now();
            if LAST_CLEANUP_TIME.is_none() || now.duration_since(LAST_CLEANUP_TIME.unwrap()) > std::time::Duration::from_secs(300) {
                self.cleanup_texture_cache();
                LAST_CLEANUP_TIME = Some(now);
            }
        }
        
        // Phase 1: 地图与对象
        if let Some(map_control) = &mut self.map_control {
            // 从 user 对象获取实际位置
            let user_pos = if let Some(user) = &self.user {
                map_control::UserPosition {
                    x: user.player.map_object.current_location.x,
                    y: user.player.map_object.current_location.y,
                    offset_x: user.player.map_object.offset_move.x,
                    offset_y: user.player.map_object.offset_move.y,
                }
            } else {
                // 如果没有 user 对象,使用默认中心位置
                map_control::UserPosition {
                    x: map_control.width / 2,
                    y: map_control.height / 2,
                    offset_x: 0,
                    offset_y: 0,
                }
            };
            
            map_control.draw(ctx, canvas, &user_pos)?;
        }
        
        // Phase 2: UI 控件树 (等价于 C# base.DrawControl())
        self.draw_controls(canvas)?;
        
        // Phase 3: 顶层元素
        self.draw_top_layer(canvas)?;
        
        Ok(())
    }
    
    /// 绘制 UI 控件树
    /// 
    /// 等价于 C# base.DrawControl()
    #[allow(unused_variables)]
    fn draw_controls(&mut self, canvas: &mut Canvas) -> GameResult<()> {
        for control in &mut self.controls {
            if control.visible() {
                // TODO: control.draw(canvas)?;
            }
        }
        Ok(())
    }
    
    /// 绘制顶层元素
    /// 
    /// 对应 C# DrawControl 后半部分 (line 1070-1085)
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
    #[allow(unused_variables)]
    fn draw_output_messages(&mut self, canvas: &mut Canvas) -> GameResult<()> {
        // TODO: 实现输出行绘制
        Ok(())
    }
    
    /// 清理纹理缓存
    /// 
    /// 对应 C# DXManager.CleanUp() 方法
    /// 
    /// 清理所有 MapLibs 中超过指定时间未使用的纹理
    fn cleanup_texture_cache(&mut self) {
        use crate::graphics::get_all_map_libraries;
        use std::time::Duration;
        
        let max_age = Duration::from_secs(600); // 10 分钟
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
                    tracing::debug!("🧹 MapLib[{}]: cleaned {} textures ({} → {})", 
                        idx, cleaned, before, after);
                }
            }
        }
        
        if total_cleaned > 0 {
            tracing::info!("🧹 Texture cache cleanup complete: removed {} old textures", total_cleaned);
        }
    }
    
    // ==================== 网络协议处理 ====================
    // 对应 C# ProcessPacket (line 1384-5976)
    
    /// 加载地图文件 (从 game_scene_old.rs 迁移)
    /// 
    /// 对应 C# LoadMap 方法
    fn load_map_file(map_name: &str) -> std::io::Result<MapControl> {
        use std::path::PathBuf;
        use crate::objects::MapReader;
        
        // 尝试不同路径 - 优先 ClientRust/Map
        let paths = [
            PathBuf::from(format!("Map/{}.map", map_name)),         // ClientRust/Map
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
                        return Ok(MapControl::from_map_reader(reader));
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
    
    /// 处理鼠标移动
    pub fn on_mouse_move(&mut self, location: Point) {
        self.mouse_location = location;
        // TODO: 更新物品悬浮提示
    }
    
    /// 处理鼠标点击
    #[allow(unused_variables)]
    pub fn on_mouse_down(&mut self, button: ggez::input::mouse::MouseButton, location: Point) {
        // 1) 检查 UI 控件命中
        for control in &mut self.controls {
            // TODO: if control.contains(location) && control.on_mouse_down(button, location) { return; }
        }
        
        // 2) 委托给 MapControl
        if let Some(_map_control) = &mut self.map_control {
            // TODO: map_control.on_mouse_down(button, location);
        }
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
}

// MapControl 的实现在 map_control.rs 中

// ==================== Scene trait 实现 ====================

impl Scene for GameScene {
    fn scene_type(&self) -> SceneType {
        SceneType::Game
    }
    
    fn initialize(&mut self) {
       tracing::info!("🎮 GameScene V2 initializing...");
        
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
    
    fn update(&mut self, _delta_time: f32) {
        // TODO: 实现更新逻辑 (对应 C# Process)
        // 更新动画计数器
        // 更新对象
        // 更新 UI
    }
    
    /// 渲染场景 (Scene trait 要求的签名)
    fn draw(&mut self, ctx: &mut ggez::Context, canvas: &mut crate::graphics::Canvas, _ggez_manager: &mut crate::graphics::GgezManager) {
        // Phase 1: 地图与对象渲染
        if let Some(ref mut map_control) = self.map_control {
            tracing::debug!("🎨 Drawing map: {} ({}x{})", map_control.title, map_control.width, map_control.height);
            // 构造用户位置
            let user_pos = if let Some(ref user) = self.user {
                map_control::UserPosition {
                    x: user.player.map_object.current_location.x,
                    y: user.player.map_object.current_location.y,
                    offset_x: user.player.map_object.offset_move.x,
                    offset_y: user.player.map_object.offset_move.y,
                }
            } else {
                // 没有用户时,使用地图中心
                map_control::UserPosition {
                    x: map_control.width / 2,
                    y: map_control.height / 2,
                    offset_x: 0,
                    offset_y: 0,
                }
            };
            
            // 调用 draw
            if let Err(e) = map_control.draw(ctx, canvas, &user_pos) {
                tracing::error!("❌ Failed to draw map: {:?}", e);
            }
        } else {
            tracing::warn!("⚠️  GameScene.draw() called but map_control is None!");
        }
        
        // Phase 2: UI 控件树渲染
        // TODO: 遍历 self.controls 并调用 draw
        
        // Phase 3: 顶层元素渲染
        // TODO: 绘制鼠标提示、输出消息等
    }
    
    /// 处理游戏事件 (从 game_scene_old.rs 迁移)
    /// 
    /// 对应 C# ProcessPacket 的各个分支
    fn process_event(&mut self, event: &GameEvent) {
        match event {
            GameEvent::MapInformation { map_index: _, file_name, title } => {
                tracing::info!("🗺️  Received MapInformation: {} ({})", title, file_name);
                
                // 加载地图文件
                match Self::load_map_file(file_name) {
                    Ok(mut map) => {
                        map.title = title.clone();
                        map.filename = file_name.clone();
                        tracing::info!("✅ Map loaded: {} ({}x{})", map.title, map.width, map.height);
                        self.map_control = Some(map);
                    }
                    Err(e) => {
                        tracing::error!("❌ Failed to load map {}: {}", file_name, e);
                        // 创建空白地图作为后备
                        let mut fallback = MapControl::new(100, 100);
                        fallback.title = title.clone();
                        fallback.filename = file_name.clone();
                        self.map_control = Some(fallback);
                    }
                }
            }
            
            GameEvent::PlayerSpawned { player } => {
                tracing::info!("👤 Player spawned: {}", player.name);
                // TODO: 创建 UserObject
            }
            
            GameEvent::PlayerMoved { location } => {
                if let Some(ref mut _user) = self.user {
                    tracing::debug!("🚶 Player moved to: ({}, {})", location.x, location.y);
                    // TODO: 更新玩家位置
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
}

// ==================== 辅助函数 ====================

/// 获取当前时间戳 (毫秒)
#[allow(dead_code)]
fn current_time_millis() -> i64 {
    // TODO: 实现
    0
}

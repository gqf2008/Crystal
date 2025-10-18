// GameScene Components - 游戏场景的组件和资源定义

use bevy::prelude::*;
use std::collections::HashMap;
use super::constants::*; // 导入常量定义

/// 游戏场景的全局状态资源
#[derive(Resource, Debug, Clone)]
pub struct GameSceneState {
    /// 当前加载的地图名称
    pub current_map: String,
    
    /// 玩家的 Entity ID (在 Bevy 中)
    pub player_entity: Option<Entity>,
    
    /// 玩家属性缓存
    pub player_level: u16,
    pub player_experience: i64,
    pub player_health: u16,
    pub player_max_health: u16,
    pub player_mana: u16,
    pub player_max_mana: u16,
    
    /// 游戏场景是否已初始化
    pub is_initialized: bool,
    
    /// 当前游戏时间
    pub game_time: f32,
    
    /// 是否暂停游戏
    pub is_paused: bool,
    
    /// HUD 显示状态
    pub show_hud: bool,
    pub show_chat: bool,
    pub show_inventory: bool,
    pub show_skills: bool,
}

impl Default for GameSceneState {
    fn default() -> Self {
        Self {
            current_map: "Map001".to_string(),
            player_entity: None,
            player_level: 1,
            player_experience: 0,
            player_health: 100,
            player_max_health: 100,
            player_mana: 50,
            player_max_mana: 50,
            is_initialized: false,
            game_time: 0.0,
            is_paused: false,
            show_hud: true,
            show_chat: true,
            show_inventory: false,
            show_skills: false,
        }
    }
}

// ============================================================================
// 玩家相关组件
// ============================================================================

/// 角色属性 - 玩家的基础属性数据
#[derive(Debug, Clone, Copy)]
pub struct CharacterStats {
    pub attack: u16,           // 物理攻击力
    pub defense: u16,          // 物理防御力
    pub magic_attack: u16,     // 魔法攻击力
    pub magic_defense: u16,    // 魔法防御力
    pub speed: u16,            // 移动速度
}

impl Default for CharacterStats {
    fn default() -> Self {
        Self {
            attack: 10,
            defense: 5,
            magic_attack: 8,
            magic_defense: 4,
            speed: 100,
        }
    }
}

/// 增益效果 - 临时状态效果
#[derive(Debug, Clone)]
pub struct BuffEffect {
    pub buff_id: u32,
    pub name: String,
    pub duration: f32,
    pub effect_type: u8,  // 0=治疗, 1=伤害, 2=速度, 3=防御
}

/// 玩家组件 - 完整的玩家数据
#[derive(Component, Debug, Clone)]
pub struct Player {
    /// 玩家网络 ID
    pub character_id: i32,
    /// 玩家名称
    pub name: String,
    /// 玩家职业
    pub class: u8,
    /// 玩家性别
    pub gender: u8,
    /// 玩家等级
    pub level: u16,
    /// 发型 (新增)
    pub hair: u8,
    /// 脸型 (新增)
    pub face: u8,
    /// 角色属性 (新增)
    pub stats: CharacterStats,
    /// 增益效果列表 (新增)
    pub buffs: Vec<BuffEffect>,
}

/// 玩家移动组件
#[derive(Component, Debug, Clone)]
pub struct PlayerMovement {
    pub speed: f32,
    pub direction: Vec3,
    pub is_moving: bool,
}

impl Default for PlayerMovement {
    fn default() -> Self {
        Self {
            speed: 100.0,  // pixels per second
            direction: Vec3::ZERO,
            is_moving: false,
        }
    }
}

// ============================================================================
// UI 相关组件
// ============================================================================

/// 游戏场景根节点
#[derive(Component)]
pub struct GameSceneRoot;

/// HUD (Heads-Up Display) 根节点
#[derive(Component)]
pub struct HudRoot;

/// 玩家信息 HUD (等级、经验、HP/MP)
#[derive(Component)]
pub struct PlayerInfoHud;

/// 聊天面板
#[derive(Component)]
pub struct ChatPanel;

/// 聊天输入框
#[derive(Component)]
pub struct ChatInput;

/// 聊天消息列表
#[derive(Component)]
pub struct ChatMessageList;

/// 技能栏
#[derive(Component)]
pub struct SkillBar;

/// 快捷栏按钮
#[derive(Component)]
pub struct QuickSlotButton {
    pub slot_index: usize,
}

/// 小地图
#[derive(Component)]
pub struct MiniMap;

/// 人物属性面板
#[derive(Component)]
pub struct CharacterPanel;

/// 背包面板
#[derive(Component)]
pub struct InventoryPanel;

/// 队伍面板
#[derive(Component)]
pub struct PartyPanel;

// ============================================================================
// 游戏对象相关组件
// ============================================================================

/// 可交互对象
#[derive(Component, Debug, Clone)]
pub struct InteractiveObject {
    pub object_id: i32,
    pub name: String,
    pub object_type: String,  // NPC, Item, Door, etc
    pub interaction_range: f32,
}

/// NPC 组件
#[derive(Component, Debug, Clone)]
pub struct NPC {
    pub npc_id: i32,
    pub name: String,
    pub dialogue_id: Option<i32>,
}

/// 游戏对象位置追踪
#[derive(Component, Debug, Clone)]
pub struct GameObjectTracker {
    pub object_id: i32,
    pub last_known_position: Vec3,
    pub last_update_time: f32,
}

// ============================================================================
// 地图相关组件
// ============================================================================

/// 地图加载器
#[derive(Component)]
pub struct MapLoader {
    pub map_name: String,
    pub is_loading: bool,
}

/// 地图渲染层
#[derive(Component)]
pub struct MapLayer {
    pub layer_index: u32,
}

/// 地图瓦片数据
#[derive(Debug, Clone, Copy)]
pub struct MapTile {
    pub tile_x: u16,
    pub tile_y: u16,
    pub layer: u8,           // 0=地面, 1=物体, 2=顶层
    pub tile_id: u32,        // 瓦片 ID
    pub walkable: bool,      // 是否可行走
}

impl Default for MapTile {
    fn default() -> Self {
        Self {
            tile_x: 0,
            tile_y: 0,
            layer: 0,
            tile_id: 0,
            walkable: true,
        }
    }
}

/// 地图对象（NPC、物品、传送点等）
#[derive(Debug, Clone)]
pub struct MapObject {
    pub object_id: u32,
    pub object_type: u8,     // 1=NPC, 2=物品, 3=传送点, 4=怪物, 5=其他
    pub x: u16,
    pub y: u16,
    pub name: String,
    pub properties: HashMap<String, String>,
}

/// 完整的地图数据资源
#[derive(Resource, Debug, Clone)]
pub struct MapData {
    pub map_id: u32,
    pub map_name: String,
    pub width: u16,
    pub height: u16,
    
    // 地图瓦片数据 (按图层存储)
    pub layers: Vec<Vec<MapTile>>,  // layers[layer_index][y*width+x]
    
    // 地图对象
    pub objects: Vec<MapObject>,
    
    // 环境属性
    pub ambient_light: [f32; 3],
    pub background_music: String,
    
    // 地图加载状态
    pub is_loaded: bool,
}

impl Default for MapData {
    fn default() -> Self {
        Self {
            map_id: 0,
            map_name: "Map001".to_string(),
            width: 100,
            height: 100,
            layers: vec![Vec::new(); 3],  // 3 个图层
            objects: Vec::new(),
            ambient_light: [1.0, 1.0, 1.0],
            background_music: String::new(),
            is_loaded: false,
        }
    }
}

impl MapData {
    /// 创建一个空地图
    pub fn new(map_id: u32, map_name: String, width: u16, height: u16) -> Self {
        let tile_count = (width as usize) * (height as usize);
        let mut layers = Vec::new();
        
        for layer_idx in 0..3 {
            let mut layer = vec![MapTile::default(); tile_count];
            
            // 底层是地面瓦片
            if layer_idx == 0 {
                for i in 0..tile_count {
                    layer[i].tile_id = 1;  // 地面瓦片 ID
                    layer[i].layer = 0;
                }
            }
            
            layers.push(layer);
        }
        
        Self {
            map_id,
            map_name,
            width,
            height,
            layers,
            objects: Vec::new(),
            ambient_light: [1.0, 1.0, 1.0],
            background_music: String::new(),
            is_loaded: false,
        }
    }
    
    /// 获取瓦片
    pub fn get_tile(&self, x: u16, y: u16, layer: u8) -> Option<MapTile> {
        if x >= self.width || y >= self.height || layer as usize >= self.layers.len() {
            return None;
        }
        
        let index = (y as usize) * (self.width as usize) + (x as usize);
        self.layers.get(layer as usize).and_then(|l| l.get(index)).copied()
    }
    
    /// 设置瓦片
    pub fn set_tile(&mut self, x: u16, y: u16, layer: u8, tile: MapTile) {
        if x >= self.width || y >= self.height || layer as usize >= self.layers.len() {
            return;
        }
        
        let index = (y as usize) * (self.width as usize) + (x as usize);
        if let Some(layer_data) = self.layers.get_mut(layer as usize) {
            if let Some(t) = layer_data.get_mut(index) {
                *t = tile;
            }
        }
    }
    
    /// 添加地图对象
    pub fn add_object(&mut self, object: MapObject) {
        self.objects.push(object);
    }
    
    /// 检查位置是否可行走
    pub fn is_walkable(&self, x: u16, y: u16) -> bool {
        self.get_tile(x, y, 0).map(|t| t.walkable).unwrap_or(false)
    }
}

// ============================================================================
// 对话系统相关
// ============================================================================

/// 对话选项
#[derive(Debug, Clone)]
pub struct DialogueOption {
    pub option_id: u32,
    pub text: String,           // 选项文本
    pub next_dialogue_id: Option<u32>,  // 下一个对话 ID
    pub action: String,         // 执行的动作 (如 "给予物品", "开启任务")
    pub conditions: Vec<String>, // 显示条件
}

/// 对话节点 - 一个对话框的内容
#[derive(Debug, Clone)]
pub struct DialogueNode {
    pub node_id: u32,
    pub npc_id: i32,            // 哪个 NPC 说这句话
    pub text: String,           // 对话文本
    pub speaker: String,        // 说话者名称
    pub options: Vec<DialogueOption>,  // 可选的回应
    pub auto_next: Option<u32>, // 自动进行到下一个对话 (无选项时)
}

/// 对话树 - 完整的对话脚本
#[derive(Resource, Debug, Clone)]
pub struct DialogueTree {
    pub tree_id: u32,
    pub npc_id: i32,
    pub nodes: std::collections::HashMap<u32, DialogueNode>,
    pub start_node_id: u32,     // 开始对话节点 ID
}

impl DialogueTree {
    /// 创建新的对话树
    pub fn new(tree_id: u32, npc_id: i32, start_node_id: u32) -> Self {
        Self {
            tree_id,
            npc_id,
            nodes: Default::default(),
            start_node_id,
        }
    }
    
    /// 添加对话节点
    pub fn add_node(&mut self, node: DialogueNode) {
        self.nodes.insert(node.node_id, node);
    }
    
    /// 获取对话节点
    pub fn get_node(&self, node_id: u32) -> Option<&DialogueNode> {
        self.nodes.get(&node_id)
    }
    
    /// 获取起始节点
    pub fn get_start_node(&self) -> Option<&DialogueNode> {
        self.get_node(self.start_node_id)
    }
}

/// 当前对话会话状态
#[derive(Resource, Debug, Clone)]
pub struct DialogueState {
    pub is_in_dialogue: bool,
    pub current_npc_id: Option<i32>,
    pub current_node_id: u32,
    pub tree_id: u32,
}

impl Default for DialogueState {
    fn default() -> Self {
        Self {
            is_in_dialogue: false,
            current_npc_id: None,
            current_node_id: 0,
            tree_id: 0,
        }
    }
}

/// 交互状态追踪
#[derive(Resource, Debug)]
pub struct InteractionState {
    pub can_interact: bool,
    pub nearby_objects: Vec<i32>,  // 附近可交互对象的 ID
    pub selected_object_id: Option<i32>,  // 当前选中的对象
}

impl Default for InteractionState {
    fn default() -> Self {
        Self {
            can_interact: false,
            nearby_objects: Vec::new(),
            selected_object_id: None,
        }
    }
}

// ============================================================================
// 聊天系统相关
// ============================================================================

/// 聊天消息结构
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub sender: String,
    pub content: String,
    pub timestamp: f32,
    pub message_type: u8,  // 0=普通, 1=系统, 2=私聊, 3=公告
}

/// 聊天管理器 - 管理聊天历史和输入
#[derive(Resource, Debug)]
pub struct ChatManager {
    pub history: std::collections::VecDeque<ChatMessage>,
    pub max_history: usize,
    pub input_buffer: String,
}

impl Default for ChatManager {
    fn default() -> Self {
        Self {
            history: std::collections::VecDeque::new(),
            max_history: MAX_CHAT_HISTORY,
            input_buffer: String::new(),
        }
    }
}

/// 聊天过滤器配置
#[derive(Resource, Debug, Clone)]
pub struct ChatFilterConfig {
    pub show_system: bool,          // 显示系统消息
    pub show_whisper: bool,         // 显示私聊消息
    pub show_broadcast: bool,       // 显示公告消息
    pub max_message_length: usize,  // 最大消息长度
    pub word_filter: Vec<String>,   // 屏蔽词列表
}

impl Default for ChatFilterConfig {
    fn default() -> Self {
        Self {
            show_system: true,
            show_whisper: true,
            show_broadcast: true,
            max_message_length: MAX_CHAT_MESSAGE_LENGTH,
            word_filter: Vec::new(),
        }
    }
}

/// 聊天快捷命令
#[derive(Debug, Clone)]
pub struct ChatCommand {
    pub name: String,              // 命令名称 (如 "help", "emote")
    pub description: String,       // 命令描述
    pub prefix: char,              // 命令前缀 (如 '/')
}

/// 聊天快捷命令管理器
#[derive(Resource, Debug)]
pub struct ChatCommandManager {
    pub commands: Vec<ChatCommand>,
    pub enabled: bool,
}

impl Default for ChatCommandManager {
    fn default() -> Self {
        Self {
            commands: vec![
                ChatCommand {
                    name: "help".to_string(),
                    description: "显示帮助信息".to_string(),
                    prefix: '/',
                },
                ChatCommand {
                    name: "emote".to_string(),
                    description: "执行表情动作".to_string(),
                    prefix: '/',
                },
                ChatCommand {
                    name: "whisper".to_string(),
                    description: "私聊玩家".to_string(),
                    prefix: '/',
                },
                ChatCommand {
                    name: "party".to_string(),
                    description: "队伍聊天".to_string(),
                    prefix: '/',
                },
            ],
            enabled: true,
        }
    }
}

/// 聊天显示设置
#[derive(Resource, Debug, Clone)]
pub struct ChatDisplaySettings {
    pub max_visible_messages: usize,    // 最多显示的消息数
    pub message_fade_time: f32,         // 消息淡出时间（秒）
    pub show_timestamps: bool,          // 显示时间戳
    pub show_sender_names: bool,        // 显示发送者名称
    pub font_size: f32,                 // 字体大小
}

impl Default for ChatDisplaySettings {
    fn default() -> Self {
        Self {
            max_visible_messages: 20,
            message_fade_time: 30.0,
            show_timestamps: true,
            show_sender_names: true,
            font_size: 14.0,
        }
    }
}

// ============================================================================
// 消息类型 - 游戏交互
// ============================================================================

/// 玩家移动消息
#[derive(Message, Clone, Default)]
pub struct PlayerMoveMessage {
    pub x: i32,
    pub y: i32,
}

/// 玩家停止消息
#[derive(Message, Clone, Default)]
pub struct PlayerStopMessage;

/// 打开聊天消息
#[derive(Message, Clone, Default)]
pub struct OpenChatMessage;

/// 关闭聊天消息
#[derive(Message, Clone, Default)]
pub struct CloseChatMessage;

/// 发送聊天消息
#[derive(Message, Clone, Default)]
pub struct SendChatMessage {
    pub text: String,
}

/// 打开背包消息
#[derive(Message, Clone, Default)]
pub struct OpenInventoryMessage;

/// 关闭背包消息
#[derive(Message, Clone, Default)]
pub struct CloseInventoryMessage;

/// 打开技能面板消息
#[derive(Message, Clone, Default)]
pub struct OpenSkillsMessage;

/// 关闭技能面板消息
#[derive(Message, Clone, Default)]
pub struct CloseSkillsMessage;

/// 打开角色面板消息
#[derive(Message, Clone, Default)]
pub struct OpenCharacterMessage;

/// 关闭角色面板消息
#[derive(Message, Clone, Default)]
pub struct CloseCharacterMessage;

/// 退出游戏消息
#[derive(Message, Clone, Default)]
pub struct ExitGameMessage;

/// 与 NPC 交互消息
#[derive(Message, Clone, Default)]
pub struct InteractWithNpcMessage {
    pub npc_id: i32,
}

/// 使用技能消息
#[derive(Message, Clone, Default)]
pub struct UseSkillMessage {
    pub skill_id: i32,
    pub target_x: f32,
    pub target_y: f32,
}

/// 暂停/恢复游戏消息
#[derive(Message, Clone, Default)]
pub struct PauseGameMessage {
    pub is_paused: bool,
}

/// 开始对话消息
#[derive(Message, Clone, Default)]
pub struct StartDialogueMessage {
    pub npc_id: i32,
}

/// 选择对话选项消息
#[derive(Message, Clone, Default)]
pub struct SelectDialogueOptionMessage {
    pub option_id: u32,
}

/// 关闭对话消息
#[derive(Message, Clone, Default)]
pub struct CloseDialogueMessage;

/// 进行交互消息
#[derive(Message, Clone, Default)]
pub struct PerformInteractionMessage {
    pub object_id: i32,
    pub interaction_type: u8,  // 1=对话, 2=传送, 3=获取物品
}

// ============================================================================
// 常量定义
// ============================================================================

/// HUD 背景颜色
pub const HUD_BG_COLOR: Color = Color::srgba(0.0, 0.0, 0.0, 0.7);

/// HUD 文本颜色
pub const HUD_TEXT_COLOR: Color = Color::srgba(1.0, 1.0, 1.0, 1.0);

/// HUD 高亮颜色
pub const HUD_HIGHLIGHT_COLOR: Color = Color::srgba(1.0, 1.0, 0.0, 1.0);

/// 血条颜色 - 满血
pub const HP_COLOR: Color = Color::srgba(0.0, 1.0, 0.0, 1.0);

/// 血条颜色 - 低血
pub const HP_LOW_COLOR: Color = Color::srgba(1.0, 0.0, 0.0, 1.0);

/// 蓝条颜色
pub const MANA_COLOR: Color = Color::srgba(0.0, 0.5, 1.0, 1.0);

/// 聊天消息颜色 - 普通
pub const CHAT_TEXT_COLOR: Color = Color::srgba(1.0, 1.0, 1.0, 1.0);

/// 聊天消息颜色 - 系统
pub const CHAT_SYSTEM_COLOR: Color = Color::srgba(1.0, 1.0, 0.0, 1.0);

/// 聊天消息颜色 - 私聊
pub const CHAT_WHISPER_COLOR: Color = Color::srgba(1.0, 0.5, 1.0, 1.0);

/// 聊天消息颜色 - 公告
pub const CHAT_BROADCAST_COLOR: Color = Color::srgba(0.5, 1.0, 1.0, 1.0);

/// 快捷键提示
pub const QUICKSLOT_TOOLTIP_COLOR: Color = Color::srgba(0.8, 0.8, 0.8, 1.0);

/// 游戏世界缩放因子 (地图像素到屏幕像素)
pub const MAP_SCALE: f32 = 2.0;

/// 地图加载范围 (角色周围多少像素内加载地图)
pub const MAP_LOAD_RADIUS: f32 = 800.0;

/// 最大视野范围 (相机显示范围)
pub const MAX_VIEW_RANGE: f32 = 600.0;

/// 默认玩家移动速度 (像素/秒)
pub const DEFAULT_MOVE_SPEED: f32 = 100.0;

/// 快捷栏最多快捷键数量
pub const QUICKSLOT_COUNT: usize = 12;

/// 最大聊天消息历史
// 注意: 所有常量已移动到 constants.rs

// ============================================================================
// Phase 5: 网络同步系统
// ============================================================================

/// 网络连接状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,    // 未连接
    Connecting,      // 连接中
    Connected,       // 已连接
    Reconnecting,    // 重新连接中
    Disconnecting,   // 断开连接中
}

/// 网络同步状态资源
#[derive(Resource, Debug, Clone)]
pub struct NetworkState {
    pub connection_state: ConnectionState,
    pub last_sync_time: f32,            // 上次同步时间
    pub sync_interval: f32,             // 同步间隔（秒）
    pub player_id: Option<i32>,         // 当前玩家网络 ID
    pub server_address: String,         // 服务器地址
    pub is_syncing: bool,               // 是否正在同步
    pub pending_updates: usize,         // 待发送的更新数
}

impl Default for NetworkState {
    fn default() -> Self {
        Self {
            connection_state: ConnectionState::Disconnected,
            last_sync_time: 0.0,
            sync_interval: 0.1,  // 每 0.1 秒同步一次
            player_id: None,
            server_address: "127.0.0.1:8888".to_string(),
            is_syncing: false,
            pending_updates: 0,
        }
    }
}

/// 远端玩家数据缓存
#[derive(Component, Debug, Clone)]
pub struct RemotePlayer {
    pub player_id: i32,
    pub character_id: i32,
    pub name: String,
    pub position: Vec3,
    pub level: u16,
    pub health: u16,
    pub max_health: u16,
    pub last_update_time: f32,
}

/// 玩家位置同步消息
#[derive(Message, Clone, Default)]
pub struct PlayerSyncMessage {
    pub character_id: i32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub direction: u8,  // 0-7 表示 8 个方向
}

/// 玩家属性同步消息
#[derive(Message, Clone, Default)]
pub struct PlayerStatsSyncMessage {
    pub character_id: i32,
    pub level: u16,
    pub experience: i64,
    pub health: u16,
    pub max_health: u16,
    pub mana: u16,
    pub max_mana: u16,
    pub stats_hash: u32,  // 用于检测变化
}

/// 远端玩家同步消息（接收其他玩家位置/状态）
#[derive(Message, Clone, Default)]
pub struct RemotePlayerSyncMessage {
    pub character_id: i32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub level: u16,
    pub health: u16,
    pub max_health: u16,
}

/// NPC 状态同步消息
#[derive(Message, Clone, Default)]
pub struct NPCSyncMessage {
    pub npc_id: i32,
    pub x: f32,
    pub y: f32,
    pub health: u16,
    pub max_health: u16,
    pub state: u8,  // 0=空闲, 1=战斗, 2=移动
}

/// 地图对象同步消息（物品、门等）
#[derive(Message, Clone, Default)]
pub struct MapObjectSyncMessage {
    pub object_id: i32,
    pub object_type: u8,
    pub x: u16,
    pub y: u16,
    pub state: u8,
}

/// 聊天广播同步消息
#[derive(Message, Clone, Default)]
pub struct ChatSyncMessage {
    pub sender_id: i32,
    pub sender_name: String,
    pub content: String,
    pub chat_type: u8,  // 0=普通, 1=系统, 2=私聊, 3=公告
}

/// 物品掉落同步消息
#[derive(Message, Clone, Default)]
pub struct ItemSpawnMessage {
    pub item_id: i32,
    pub item_type: u16,
    pub x: f32,
    pub y: f32,
    pub quantity: u16,
}

/// 物品消失同步消息
#[derive(Message, Clone, Default)]
pub struct ItemDespawnMessage {
    pub item_id: i32,
}

/// 网络连接事件
#[derive(Message, Clone, Default)]
pub struct ConnectionEvent {
    pub event_type: u8,  // 0=连接成功, 1=连接失败, 2=断开连接, 3=超时
    pub message: String,
}

/// 网络错误事件
#[derive(Message, Clone, Default)]
pub struct NetworkErrorMessage {
    pub error_code: u16,
    pub error_message: String,
}

/// 服务器时间同步消息
#[derive(Message, Clone, Default)]
pub struct ServerTimeSyncMessage {
    pub server_time: u64,
    pub server_tick: u32,
}

// ============================================================================
// 网络同步常量
// ============================================================================

/// 默认网络同步间隔（秒）
pub const DEFAULT_SYNC_INTERVAL: f32 = 0.1;

/// 网络超时时间（秒）
pub const NETWORK_TIMEOUT: f32 = 30.0;

/// 最大待处理更新数
pub const MAX_PENDING_UPDATES: usize = 1000;

/// 最大远端玩家数
pub const MAX_REMOTE_PLAYERS: usize = 100;

// ============================================================================
// Phase 6: 完整事件循环系统
// ============================================================================

/// 游戏循环状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameLoopState {
    Idle,           // 空闲
    Running,        // 运行中
    Paused,         // 暂停中
    Slowing,        // 减速中（检查帧率）
    SpeedUp,        // 加速中
    Error,          // 错误状态
}

/// 帧统计信息
#[derive(Resource, Debug, Clone)]
pub struct FrameStats {
    pub current_fps: f32,              // 当前FPS
    pub average_fps: f32,              // 平均FPS
    pub frame_count: u32,              // 累计帧数
    pub total_time: f32,               // 总运行时间（秒）
    pub last_frame_time: f32,          // 最后一帧时间（毫秒）
    pub min_frame_time: f32,           // 最小帧时间
    pub max_frame_time: f32,           // 最大帧时间
    pub frame_time_history: Vec<f32>,  // 帧时间历史记录
    pub history_size: usize,           // 历史记录大小
}

impl Default for FrameStats {
    fn default() -> Self {
        Self {
            current_fps: 60.0,
            average_fps: 60.0,
            frame_count: 0,
            total_time: 0.0,
            last_frame_time: 0.0,
            min_frame_time: f32::MAX,
            max_frame_time: 0.0,
            frame_time_history: Vec::new(),
            history_size: 60,
        }
    }
}

/// 游戏计时器
#[derive(Resource, Debug, Clone)]
pub struct GameTimer {
    pub elapsed_time: f32,             // 游戏已运行时间
    pub delta_time: f32,               // 上一帧的增量时间
    pub game_speed: f32,               // 游戏速度倍数（1.0=正常）
    pub is_running: bool,              // 是否运行中
    pub frame_skip_threshold: f32,     // 帧跳过阈值（毫秒）
}

impl Default for GameTimer {
    fn default() -> Self {
        Self {
            elapsed_time: 0.0,
            delta_time: 0.0,
            game_speed: 1.0,
            is_running: true,
            frame_skip_threshold: 33.33,  // 30FPS 阈值
        }
    }
}

/// 游戏事件队列
#[derive(Resource, Debug)]
pub struct EventQueue {
    pub events: Vec<String>,  // 事件日志
    pub max_events: usize,    // 最大事件数
}

impl Default for EventQueue {
    fn default() -> Self {
        Self {
            events: Vec::new(),
            max_events: 1000,
        }
    }
}

impl EventQueue {
    pub fn push_event(&mut self, event: String) {
        self.events.push(event);
        if self.events.len() > self.max_events {
            self.events.remove(0);
        }
    }
}

/// 系统健康检查
#[derive(Resource, Debug, Clone)]
pub struct SystemHealthCheck {
    pub player_system_ok: bool,        // 玩家系统
    pub map_system_ok: bool,           // 地图系统
    pub dialogue_system_ok: bool,      // 对话系统
    pub chat_system_ok: bool,          // 聊天系统
    pub network_system_ok: bool,       // 网络系统
    pub render_system_ok: bool,        // 渲染系统
    pub all_systems_ok: bool,          // 所有系统
    pub last_check_time: f32,          // 最后检查时间
}

impl Default for SystemHealthCheck {
    fn default() -> Self {
        Self {
            player_system_ok: true,
            map_system_ok: true,
            dialogue_system_ok: true,
            chat_system_ok: true,
            network_system_ok: true,
            render_system_ok: true,
            all_systems_ok: true,
            last_check_time: 0.0,
        }
    }
}

/// 游戏循环控制消息
#[derive(Message, Clone, Default)]
pub struct GameLoopMessage {
    pub loop_state: u8,  // 0=normal, 1=pause, 2=resume, 3=speed_up, 4=slow_down
}

/// 帧统计请求消息
#[derive(Message, Clone, Default)]
pub struct RequestFrameStatsMessage;

/// 系统健康检查请求消息
#[derive(Message, Clone, Default)]
pub struct RequestSystemHealthMessage;

/// 性能报告消息
#[derive(Message, Clone, Default)]
pub struct PerformanceReportMessage {
    pub report_type: u8,  // 0=fps, 1=memory, 2=network, 3=all
}

// ============================================================================
// Phase 6 相关常量
// ============================================================================

/// 目标 FPS
pub const TARGET_FPS: f32 = 60.0;

/// 最大 FPS
pub const MAX_FPS: f32 = 144.0;

/// 最小 FPS
pub const MIN_FPS: f32 = 30.0;

/// 帧时间历史大小
pub const FRAME_TIME_HISTORY_SIZE: usize = 60;

/// 系统健康检查间隔（秒）
pub const HEALTH_CHECK_INTERVAL: f32 = 5.0;

/// 性能采样间隔（秒）
pub const PERFORMANCE_SAMPLE_INTERVAL: f32 = 1.0;

// ============================================================================
// Components - ECS 组件定义
// 参考 C# Client/MirObjects/ 的对象属性
// ============================================================================

use mir2_shared::Point;
pub use mir2_shared::{MirDirection, MirAction, MirClass, MirGender};
use std::time::Instant;
use crate::objects::CellInfo;

// 导入效果混合模式
pub use crate::objects::SpriteBlendMode;

// ============================================================================
// 核心组件 (所有实体都有)
// ============================================================================

/// 位置组件 - 世界坐标（像素级，支持浮点）
/// 统一使用 f32 坐标系统，支持平滑移动和精确渲染
#[derive(Debug, Clone, Copy)]
pub struct Position {
    pub x: f32,      // 世界坐标 X（像素）
    pub y: f32,      // 世界坐标 Y（像素）
}

impl Position {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
    
    /// 从整数格子坐标创建（48x32像素单元格）
    pub fn from_grid(grid_x: i32, grid_y: i32) -> Self {
        Self {
            x: grid_x as f32 * 48.0,
            y: grid_y as f32 * 32.0,
        }
    }
}

/// 速度组件 - 移动实体必备
#[derive(Debug, Clone, Copy)]
pub struct Velocity {
    pub dx: f32,
    pub dy: f32,
}

impl Velocity {
    pub fn new(dx: f32, dy: f32) -> Self {
        Self { dx, dy }
    }

    pub fn zero() -> Self {
        Self { dx: 0.0, dy: 0.0 }
    }
}

/// 方向组件
#[derive(Debug, Clone, Copy)]
pub struct Direction {
    pub current: MirDirection,
    pub target: MirDirection,
}

impl Direction {
    pub fn new(dir: MirDirection) -> Self {
        Self { current: dir, target: dir }
    }
}

/// 精灵渲染组件 - 可渲染实体必备
#[derive(Debug, Clone)]
pub struct Sprite {
    pub library: i32,      // MLibrary 索引 (0=Tiles, 1=SmTiles, 2=Objects, etc.)
    pub index: i32,        // 贴图索引
    pub frame: i32,        // 当前帧
    pub blend_mode: SpriteBlendMode, // 混合模式
}

impl Sprite {
    pub fn new(library: i32, index: i32) -> Self {
        Self {
            library,
            index,
            frame: 0,
            blend_mode: SpriteBlendMode::Alpha,
        }
    }

    pub fn with_blend(library: i32, index: i32, blend_mode: SpriteBlendMode) -> Self {
        Self { library, index, frame: 0, blend_mode }
    }
}

// ============================================================================
// 动画组件
// ============================================================================

/// 动画状态组件
#[derive(Debug, Clone)]
pub struct Animation {
    pub action: MirAction,
    pub direction: u8,       // 方向 0-7
    pub frame_count: u8,
    pub frame_index: u8,
    pub frame_interval: u32, // 毫秒
    pub frame_timer: u32,
    pub loop_animation: bool,
}

impl Animation {
    pub fn new(action: MirAction, frame_count: u8, frame_interval: u32) -> Self {
        Self {
            action,
            direction: 0,    // 默认朝右
            frame_count,
            frame_index: 0,
            frame_interval,
            frame_timer: 0,
            loop_animation: true,
        }
    }

    /// 更新动画 (返回 true 表示播放完成)
    pub fn update(&mut self, delta_ms: u32) -> bool {
        self.frame_timer += delta_ms;
        if self.frame_timer >= self.frame_interval {
            self.frame_timer = 0;
            self.frame_index += 1;

            if self.frame_index >= self.frame_count {
                if self.loop_animation {
                    self.frame_index = 0;
                } else {
                    self.frame_index = self.frame_count - 1;
                    return true; // 动画完成
                }
            }
        }
        false
    }
}

// ============================================================================
// 战斗相关组件
// ============================================================================

/// 生命值组件
#[derive(Debug, Clone, Copy)]
pub struct Health {
    pub current: i32,
    pub max: i32,
}

impl Health {
    pub fn new(max: i32) -> Self {
        Self { current: max, max }
    }

    pub fn is_alive(&self) -> bool {
        self.current > 0
    }

    pub fn take_damage(&mut self, damage: i32) {
        self.current = (self.current - damage).max(0);
    }

    pub fn heal(&mut self, amount: i32) {
        self.current = (self.current + amount).min(self.max);
    }
}

/// 魔法值组件
#[derive(Debug, Clone, Copy)]
pub struct Mana {
    pub current: i32,
    pub max: i32,
}

impl Mana {
    pub fn new(max: i32) -> Self {
        Self { current: max, max }
    }

    pub fn has_enough(&self, cost: i32) -> bool {
        self.current >= cost
    }

    pub fn consume(&mut self, cost: i32) -> bool {
        if self.current >= cost {
            self.current -= cost;
            true
        } else {
            false
        }
    }

    pub fn restore(&mut self, amount: i32) {
        self.current = (self.current + amount).min(self.max);
    }

    pub fn percent(&self) -> f32 {
        if self.max > 0 {
            self.current as f32 / self.max as f32
        } else {
            0.0
        }
    }
}

/// 战斗属性组件 (玩家/怪物)
#[derive(Debug, Clone)]
pub struct CombatStats {
    pub level: u16,
    pub attack_min: i32,
    pub attack_max: i32,
    pub defense: i32,
    pub magic_defense: i32,
    pub accuracy: u8,
    pub agility: u8,
}

// ============================================================================
// 玩家专用组件
// ============================================================================

/// 玩家数据组件 (标记这是玩家实体)
#[derive(Debug, Clone)]
pub struct PlayerData {
    pub id: u32,
    pub name: String,
    pub class: MirClass,
    pub gender: MirGender,
    pub exp: i64,
    pub gold: u32,
}

/// 本地玩家标记 (只有一个)
#[derive(Debug, Clone, Copy)]
pub struct LocalPlayer;

/// 远程玩家标记 (网络同步)
#[derive(Debug, Clone, Copy)]
pub struct RemotePlayer {
    pub id: u32,
}

// ============================================================================
// 怪物专用组件
// ============================================================================

/// 怪物数据组件
#[derive(Debug, Clone)]
pub struct MonsterData {
    pub id: u32,
    pub name: String,
    pub monster_index: u16,
    pub ai_mode: u8,
    pub ai_type: u8,         // AI 类型 (0=无, 1=近战, 2=远程, 3=巡逻)
    pub spawn_x: f32,        // 出生点 X
    pub spawn_y: f32,        // 出生点 Y
}

/// AI 状态组件
#[derive(Debug, Clone)]
pub struct AIState {
    pub mode: AIMode,
    pub current_action: AIAction,
    pub target_entity: Option<hecs::Entity>, // 目标实体
    pub target_pos: Option<(f32, f32)>,      // 目标位置
    pub last_action_time: u64,
    pub patrol_points: Vec<(f32, f32)>,      // 巡逻路径点
    pub current_patrol_index: usize,         // 当前巡逻点索引
}

impl Default for AIState {
    fn default() -> Self {
        Self {
            mode: AIMode::Idle,
            current_action: AIAction::Idle,
            target_entity: None,
            target_pos: None,
            last_action_time: 0,
            patrol_points: Vec::new(),
            current_patrol_index: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AIMode {
    Idle,
    Patrol,
    Chase,
    Attack,
    Retreat,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AIAction {
    Idle,      // 闲置
    Patrol,    // 巡逻
    Chase,     // 追击
    Attack,    // 攻击
    Retreat,   // 后退
}

// ============================================================================
// NPC 组件
// ============================================================================

/// NPC 数据组件
#[derive(Debug, Clone)]
pub struct NPCData {
    pub id: u32,
    pub name: String,
    pub npc_index: u16,
    pub dialogue_id: u32,
    pub colour: i32,  // NPC颜色染色 (ARGB格式)
    pub action_timer: u32,  // 动作切换计时器(毫秒)
    pub next_action_delay: u32,  // 下次切换延迟(毫秒)
}

/// NPC任务标识
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QuestIcon {
    None,           // 无任务
    Available,      // 可接任务(黄色感叹号)
    Complete,       // 可交任务(黄色问号)
    Incomplete,     // 进行中任务(灰色问号)
}

/// 任务标记组件
#[derive(Debug, Clone, Copy)]
pub struct QuestMarker {
    pub icon: QuestIcon,
}

impl QuestMarker {
    pub fn new(icon: QuestIcon) -> Self {
        Self { icon }
    }
}

// ============================================================================
// 技能/特效组件
// ============================================================================

/// 技能数据组件
#[derive(Debug, Clone)]
pub struct SpellData {
    pub spell_id: u16,
    pub caster_id: u32,
    pub target_pos: Point,
    pub power: i32,
}

/// 生命周期组件 (技能特效/掉落物等有时间限制的实体)
#[derive(Debug, Clone, Copy)]
pub struct Lifetime {
    pub remaining_ms: u32,
}

impl Lifetime {
    pub fn new(duration_ms: u32) -> Self {
        Self { remaining_ms: duration_ms }
    }

    pub fn update(&mut self, delta_ms: u32) -> bool {
        if self.remaining_ms > delta_ms {
            self.remaining_ms -= delta_ms;
            false
        } else {
            self.remaining_ms = 0;
            true // 生命周期结束
        }
    }
}

// ============================================================================
// 物品组件
// ============================================================================

/// 地面物品组件
#[derive(Debug, Clone)]
pub struct ItemDrop {
    pub item_id: u32,
    pub item_index: u16,
    pub count: u32,
    pub owner_id: Option<u32>, // 归属玩家 (拾取保护)
}

// ============================================================================
// 网络同步组件
// ============================================================================

/// 网络同步标记 (需要同步的实体)
#[derive(Debug, Clone)]
pub struct NetworkSync {
    /// 服务器对象ID
    pub object_id: u32,
    /// 最后更新时间
    pub last_update: Instant,
    /// 对象类型
    pub object_type: NetworkObjectType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkObjectType {
    Player,      // 其他玩家
    NPC,         // NPC
    Monster,     // 怪物
    Item,        // 地面物品
    Spell,       // 技能特效
}

impl NetworkSync {
    pub fn new(object_id: u32, object_type: NetworkObjectType) -> Self {
        Self {
            object_id,
            last_update: Instant::now(),
            object_type,
        }
    }
}

// ============================================================================
// 渲染层级组件
// ============================================================================

/// 渲染层级 (用于排序)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RenderLayer {
    Ground = 0,      // 地面层
    GroundItem = 1,  // 地面物品
    Shadow = 2,      // 阴影
    Object = 3,      // 游戏对象 (玩家/怪物/NPC)
    Effect = 4,      // 特效 (技能/爆炸)
    UI = 5,          // UI元素
}

#[derive(Debug, Clone, Copy)]
pub struct RenderOrder {
    pub layer: RenderLayer,
    pub z_order: i32, // 同层内的排序 (Y坐标)
}

impl RenderOrder {
    pub fn new(layer: RenderLayer, z_order: i32) -> Self {
        Self { layer, z_order }
    }
}

// ============================================================================
// 地图查看器/工具专用组件
// ============================================================================

/// 相机组件 - 视口控制
#[derive(Debug, Clone)]
pub struct Camera {
    pub zoom: f32,
    pub screen_width: f32,
    pub screen_height: f32,
}

impl Camera {
    pub fn new(screen_width: f32, screen_height: f32) -> Self {
        Self {
            zoom: 1.0,
            screen_width,
            screen_height,
        }
    }
}

/// 拖拽组件 - 鼠标拖拽状态
#[derive(Debug, Clone)]
pub struct Draggable {
    pub is_dragging: bool,
    pub drag_start_x: f32,
    pub drag_start_y: f32,
    pub drag_start_pos_x: f32,
    pub drag_start_pos_y: f32,
}

impl Default for Draggable {
    fn default() -> Self {
        Self {
            is_dragging: false,
            drag_start_x: 0.0,
            drag_start_y: 0.0,
            drag_start_pos_x: 0.0,
            drag_start_pos_y: 0.0,
        }
    }
}

/// 角色组件 - 查看器中的可控角色
#[derive(Debug, Clone)]
pub struct Player {
    pub direction: u8,  // 0-7 八方向
    pub action: PlayerAction,
    pub frame_index: i32,
    pub frame_time: i32,
    pub speed: f32,
    pub target_x: f32,
    pub target_y: f32,
    pub is_moving: bool,
    pub path: Vec<(i32, i32)>,
    pub path_index: usize,
    pub move_mode: MoveMode,
    pub last_move_time: std::time::Instant,  // 上次发送移动命令的时间
    pub move_delay: std::time::Duration,     // 移动命令间隔(服务器MoveDelay=600ms)
    pub waiting_server_confirm: bool,        // 🎯 等待服务器确认移动
}

/// 角色动作
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlayerAction {
    Stand = 0,
    Walk = 1,
    Run = 2,
}

impl PlayerAction {
    pub fn frame_count(&self) -> i32 {
        match self {
            PlayerAction::Stand => 4,
            PlayerAction::Walk => 6,
            PlayerAction::Run => 6,
        }
    }
    
    pub fn frame_interval(&self) -> i32 {
        match self {
            PlayerAction::Stand => 30,
            PlayerAction::Walk => 6,
            PlayerAction::Run => 5,
        }
    }
    
    pub fn frame_start(&self) -> i32 {
        match self {
            PlayerAction::Stand => 0,
            PlayerAction::Walk => 32,
            PlayerAction::Run => 80,
        }
    }
}

/// 移动模式状态机
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MoveMode {
    Idle,
    DirectFollow,
    AutoPathfinding,
}

/// 玩家外观组件
#[derive(Debug, Clone)]
pub struct PlayerAppearance {
    pub class: mir2_shared::enums::MirClass,
    pub gender: mir2_shared::enums::MirGender,
    pub hair: u8,
    pub weapon: i16,
    pub armour: i16,
    pub weapon_effect: i16,
    pub wing_effect: u8,
}

impl Default for PlayerAppearance {
    fn default() -> Self {
        Self {
            class: mir2_shared::enums::MirClass::Warrior,
            gender: mir2_shared::enums::MirGender::Male,
            hair: 0,
            weapon: -1,  // -1 表示无武器
            armour: 0,   // 默认盔甲索引
            weapon_effect: 0,
            wing_effect: 0,
        }
    }
}

/// 地面物品组件
#[derive(Debug, Clone)]
pub struct GroundItem {
    pub object_id: u32,
    pub item: mir2_shared::data::item::UserItem,
    pub gold_amount: u32,  // 如果是金币，这里是数量
}

/// 背包组件 - 存储玩家的物品
#[derive(Debug, Clone)]
pub struct Inventory {
    /// 背包物品列表（索引对应格子位置）
    /// None 表示空格子
    pub items: Vec<Option<mir2_shared::data::item::UserItem>>,
    
    /// 背包容量（默认40格）
    pub capacity: usize,
    
    /// 金币数量
    pub gold: u32,
    
    /// 当前负重
    pub current_weight: u16,
    
    /// 最大负重
    pub max_weight: u16,
}

impl Default for Inventory {
    fn default() -> Self {
        Self::new(40) // 默认40格背包
    }
}

impl Inventory {
    pub fn new(capacity: usize) -> Self {
        Self {
            items: vec![None; capacity],
            capacity,
            gold: 0,
            current_weight: 0,
            max_weight: 100, // 默认最大负重100
        }
    }
    
    /// 添加物品到背包
    pub fn add_item(&mut self, item: mir2_shared::data::item::UserItem) -> bool {
        // 查找空格子
        for slot in &mut self.items {
            if slot.is_none() {
                *slot = Some(item);
                return true;
            }
        }
        false // 背包已满
    }
    
    /// 移除指定格子的物品
    pub fn remove_item(&mut self, slot_index: usize) -> Option<mir2_shared::data::item::UserItem> {
        if slot_index < self.items.len() {
            self.items[slot_index].take()
        } else {
            None
        }
    }
    
    /// 获取指定格子的物品引用
    pub fn get_item(&self, slot_index: usize) -> Option<&mir2_shared::data::item::UserItem> {
        if slot_index < self.items.len() {
            self.items[slot_index].as_ref()
        } else {
            None
        }
    }
    
    /// 设置金币数量
    pub fn set_gold(&mut self, gold: u32) {
        self.gold = gold;
    }
    
    /// 添加金币
    pub fn add_gold(&mut self, amount: u32) {
        self.gold = self.gold.saturating_add(amount);
    }
    
    /// 减少金币
    pub fn remove_gold(&mut self, amount: u32) -> bool {
        if self.gold >= amount {
            self.gold -= amount;
            true
        } else {
            false
        }
    }
}

/// 鼠标输入状态组件
#[derive(Debug, Clone)]
pub struct MouseInput {
    pub left_pressed: bool,
    pub right_pressed: bool,
    pub left_double_clicked: bool,
    pub right_double_clicked: bool,
    pub left_press_time: i32,
    pub right_press_time: i32,
    pub left_last_click_time: Instant,
    pub right_last_click_time: Instant,
    pub x: f32,
    pub y: f32,
}

impl Default for MouseInput {
    fn default() -> Self {
        Self {
            left_pressed: false,
            right_pressed: false,
            left_double_clicked: false,
            right_double_clicked: false,
            left_press_time: 0,
            right_press_time: 0,
            left_last_click_time: Instant::now(),
            right_last_click_time: Instant::now(),
            x: 0.0,
            y: 0.0,
        }
    }
}

/// 装备栏组件
#[derive(Debug, Clone)]
pub struct Equipment {
    pub weapon: Option<mir2_shared::data::item::UserItem>,       // 武器
    pub armour: Option<mir2_shared::data::item::UserItem>,       // 衣服
    pub helmet: Option<mir2_shared::data::item::UserItem>,       // 头盔
    pub necklace: Option<mir2_shared::data::item::UserItem>,     // 项链
    pub bracelet_l: Option<mir2_shared::data::item::UserItem>,   // 左手镯
    pub bracelet_r: Option<mir2_shared::data::item::UserItem>,   // 右手镯
    pub ring_l: Option<mir2_shared::data::item::UserItem>,       // 左戒指
    pub ring_r: Option<mir2_shared::data::item::UserItem>,       // 右戒指
    pub amulet: Option<mir2_shared::data::item::UserItem>,       // 护身符
    pub belt: Option<mir2_shared::data::item::UserItem>,         // 腰带
    pub boots: Option<mir2_shared::data::item::UserItem>,        // 鞋子
    pub stone: Option<mir2_shared::data::item::UserItem>,        // 宝石
    pub torch: Option<mir2_shared::data::item::UserItem>,        // 火把
    pub mount: Option<mir2_shared::data::item::UserItem>,        // 坐骑
}

impl Default for Equipment {
    fn default() -> Self {
        Self {
            weapon: None,
            armour: None,
            helmet: None,
            necklace: None,
            bracelet_l: None,
            bracelet_r: None,
            ring_l: None,
            ring_r: None,
            amulet: None,
            belt: None,
            boots: None,
            stone: None,
            torch: None,
            mount: None,
        }
    }
}

impl Equipment {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// 根据装备类型获取对应槽位
    pub fn get_slot_for_type(&self, item_type: mir2_shared::enums::ItemType) -> Option<u8> {
        use mir2_shared::enums::ItemType;
        match item_type {
            ItemType::Weapon => Some(0),
            ItemType::Armour => Some(1),
            ItemType::Helmet => Some(2),
            ItemType::Necklace => Some(3),
            ItemType::Bracelet => Some(4), // 默认左手镯
            ItemType::Ring => Some(6),     // 默认左戒指
            ItemType::Amulet => Some(8),
            ItemType::Belt => Some(9),
            ItemType::Boots => Some(10),
            ItemType::Stone => Some(11),
            ItemType::Torch => Some(12),
            ItemType::Mount => Some(13),
            _ => None,
        }
    }
    
    /// 装备物品到指定槽位
    pub fn equip(&mut self, slot: u8, item: mir2_shared::data::item::UserItem) -> Option<mir2_shared::data::item::UserItem> {
        let slot_ref = match slot {
            0 => &mut self.weapon,
            1 => &mut self.armour,
            2 => &mut self.helmet,
            3 => &mut self.necklace,
            4 => &mut self.bracelet_l,
            5 => &mut self.bracelet_r,
            6 => &mut self.ring_l,
            7 => &mut self.ring_r,
            8 => &mut self.amulet,
            9 => &mut self.belt,
            10 => &mut self.boots,
            11 => &mut self.stone,
            12 => &mut self.torch,
            13 => &mut self.mount,
            _ => return None,
        };
        
        // 返回旧装备
        slot_ref.replace(item)
    }
    
    /// 卸下指定槽位的装备
    pub fn unequip(&mut self, slot: u8) -> Option<mir2_shared::data::item::UserItem> {
        let slot_ref = match slot {
            0 => &mut self.weapon,
            1 => &mut self.armour,
            2 => &mut self.helmet,
            3 => &mut self.necklace,
            4 => &mut self.bracelet_l,
            5 => &mut self.bracelet_r,
            6 => &mut self.ring_l,
            7 => &mut self.ring_r,
            8 => &mut self.amulet,
            9 => &mut self.belt,
            10 => &mut self.boots,
            11 => &mut self.stone,
            12 => &mut self.torch,
            13 => &mut self.mount,
            _ => return None,
        };
        
        slot_ref.take()
    }
}

/// 地图瓦片组件
#[derive(Debug, Clone)]
pub struct MapTile {
    pub grid_x: i32,
    pub grid_y: i32,
    pub layer: TileLayer,
    pub library_index: i16,
    pub image_index: i32,
    pub use_blend: bool,
    pub brightness: f32,
    pub z_order: i32,
}

/// 瓦片层级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TileLayer {
    Back = 0,
    Middle = 1,
    Front = 2,
}

/// 动画瓦片组件
#[derive(Debug, Clone)]
pub struct AnimatedTile {
    pub frame_count: u8,
    pub frame_interval: u8,
    pub base_image_index: i32,
}

/// 门组件
#[derive(Debug, Clone)]
pub struct Door {
    pub door_index: u8,
    pub door_offset: i32,
    pub state: DoorState,
    pub current_frame: i32,
    pub last_tick: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DoorState {
    Closed = 0,
    Opening = 1,
    Open = 2,
    Closing = 3,
}

/// 地图数据组件
#[derive(Clone)]
pub struct MapData {
    pub cells: Vec<Vec<CellInfo>>,
    pub width: i32,
    pub height: i32,
}

/// 渲染配置组件
#[derive(Debug, Clone)]
pub struct RenderConfig {
    pub show_back: bool,
    pub show_middle: bool,
    pub show_front: bool,
    pub show_grid: bool,
    pub show_obstacles: bool,
    pub show_animations: bool,
    pub show_borders: bool,
    pub show_npc_borders: bool,      // NPC边框调试
    pub show_monster_borders: bool,  // Monster边框调试
    pub show_effect_borders: bool,   // 特效边框调试
    pub show_path: bool,
    pub max_fps: u32,
    pub enable_lod: bool,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            show_back: true,
            show_middle: true,
            show_front: true,
            show_grid: false,
            show_obstacles: false,
            show_animations: true,
            show_borders: false,
            show_npc_borders: false,
            show_monster_borders: false,
            show_effect_borders: false,
            show_path: false,
            max_fps: 60,
            enable_lod: false,
        }
    }
}

/// 时间跟踪组件
#[derive(Debug, Clone)]
pub struct TimeTracker {
    pub animation_count: i32,
    pub frame_count: u64,
    pub fps: f32,
    pub last_fps_update: Instant,
    pub last_frame_time: Instant,
}

impl Default for TimeTracker {
    fn default() -> Self {
        Self {
            animation_count: 0,
            frame_count: 0,
            fps: 0.0,
            last_fps_update: Instant::now(),
            last_frame_time: Instant::now(),
        }
    }
}

/// 可见区域缓存
#[derive(Debug, Clone)]
pub struct VisibleArea {
    pub start_x: i32,
    pub end_x: i32,
    pub start_y: i32,
    pub end_y: i32,
    pub front_end_y: i32,
    pub zoom: f32,
    pub camera_x: f32,
    pub camera_y: f32,
    pub visible_entities: Vec<hecs::Entity>,
    pub last_update: Instant,
}

impl Default for VisibleArea {
    fn default() -> Self {
        Self {
            start_x: -999999,
            end_x: -999999,
            start_y: -999999,
            end_y: -999999,
            front_end_y: -999999,
            zoom: -1.0,
            camera_x: -999999.0,
            camera_y: -999999.0,
            visible_entities: Vec::new(),
            last_update: Instant::now(),
        }
    }
}

// ============================================================================
// 其他网络对象组件
// ============================================================================

/// 其他玩家组件（区别于本地玩家Player）
#[derive(Debug, Clone)]
pub struct OtherPlayer {
    pub name: String,
    pub class: MirClass,
    pub gender: MirGender,
    pub level: u16,
    pub guild_name: Option<String>,
}

impl OtherPlayer {
    pub fn new(name: String, class: MirClass, gender: MirGender, level: u16) -> Self {
        Self {
            name,
            class,
            gender,
            level,
            guild_name: None,
        }
    }
}

/// NPC组件
#[derive(Debug, Clone)]
pub struct NPC {
    pub name: String,
    pub npc_type: String,
    pub can_interact: bool,
}

impl NPC {
    pub fn new(name: String, npc_type: String) -> Self {
        Self {
            name,
            npc_type,
            can_interact: true,
        }
    }
}

/// 怪物组件
#[derive(Debug, Clone)]
pub struct Monster {
    pub name: String,
    pub monster_type: u16,
    pub ai_state: MonsterAIState,
    pub target_id: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonsterAIState {
    Idle,
    Patrol,
    Chase,
    Attack,
    Dead,
}

impl Monster {
    pub fn new(name: String, monster_type: u16) -> Self {
        Self {
            name,
            monster_type,
            ai_state: MonsterAIState::Idle,
            target_id: None,
        }
    }
}

// ============================================================================
// 技能/魔法系统组件
// ============================================================================

/// 技能类型 (对应 C# Spell 枚举)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum SpellType {
    None = 0,
    
    // Warrior (战士)
    Fencing = 1,
    Slaying = 2,
    Thrusting = 3,
    HalfMoon = 4,
    ShoulderDash = 5,
    TwinDrakeBlade = 6,
    Entrapment = 7,
    FlamingSword = 8,
    LionRoar = 9,
    CrossHalfMoon = 10,
    BladeAvalanche = 11,
    ProtectionField = 12,
    Rage = 13,
    CounterAttack = 14,
    SlashingBurst = 15,
    Fury = 16,
    ImmortalSkin = 17,
    
    // Wizard (法师)
    FireBall = 31,
    Repulsion = 32,
    ElectricShock = 33,
    GreatFireBall = 34,
    HellFire = 35,
    ThunderBolt = 36,
    Teleport = 37,
    FireBang = 38,
    FireWall = 39,
    Lightning = 40,
    FrostCrunch = 41,
    ThunderStorm = 42,
    MagicShield = 43,
    TurnUndead = 44,
    Vampirism = 45,
    IceStorm = 46,
    FlameDisruptor = 47,
    Mirroring = 48,
    FlameField = 49,
    Blizzard = 50,
    MagicBooster = 51,
    MeteorStrike = 52,
    IceThrust = 53,
    FastMove = 54,
    StormEscape = 55,
    
    // Taoist (道士)
    Healing = 61,
    SpiritSword = 62,
    Poisoning = 63,
    SoulFireBall = 64,
    SummonSkeleton = 65,
    Hiding = 67,
    MassHiding = 68,
    SoulShield = 69,
    Revelation = 70,
    BlessedArmour = 71,
    EnergyRepulsor = 72,
    TrapHexagon = 73,
    Purification = 74,
    MassHealing = 75,
    Hallucination = 76,
    UltimateEnhancer = 77,
    SummonShinsu = 78,
    Reincarnation = 79,
    SummonHolyDeva = 80,
    Curse = 81,
    Plague = 82,
    PoisonCloud = 83,
    EnergyShield = 84,
    PetEnhancer = 85,
    HealingCircle = 86,
    
    // Assassin (刺客)
    FatalSword = 91,
    DoubleSlash = 92,
    Haste = 93,
    FlashDash = 94,
    LightBody = 95,
    HeavenlySword = 96,
    FireBurst = 97,
    Trap = 98,
    PoisonSword = 99,
    MoonLight = 100,
    MPEater = 101,
    SwiftFeet = 102,
    DarkBody = 103,
    Hemorrhage = 104,
    CrescentSlash = 105,
    MoonMist = 106,
    CatTongue = 107,
    
    // Archer (弓箭手)
    Focus = 121,
    StraightShot = 122,
    DoubleShot = 123,
    ExplosiveTrap = 124,
    DelayedExplosion = 125,
    Meditation = 126,
    BackStep = 127,
    ElementalShot = 128,
    Concentration = 129,
    Stonetrap = 130,
    ElementalBarrier = 131,
    SummonVampire = 132,
    VampireShot = 133,
    SummonToad = 134,
    PoisonShot = 135,
    CrippleShot = 136,
    SummonSnakes = 137,
    NapalmShot = 138,
    OneWithNature = 139,
    BindingShot = 140,
    MentalState = 141,
}

impl SpellType {
    /// 获取技能名称
    pub fn name(&self) -> &'static str {
        match self {
            Self::None => "无",
            // Warrior
            Self::Fencing => "基本剑术",
            Self::Slaying => "攻杀剑术",
            Self::Thrusting => "刺杀剑术",
            Self::HalfMoon => "半月弯刀",
            Self::ShoulderDash => "野蛮冲撞",
            Self::LionRoar => "狮子吼",
            // Wizard
            Self::FireBall => "火球术",
            Self::GreatFireBall => "大火球",
            Self::HellFire => "地狱火",
            Self::ThunderBolt => "雷电术",
            Self::Teleport => "瞬息移动",
            Self::Lightning => "疾光电影",
            Self::MagicShield => "魔法盾",
            // Taoist
            Self::Healing => "治愈术",
            Self::SpiritSword => "精神力战法",
            Self::Poisoning => "施毒术",
            Self::SummonSkeleton => "召唤骷髅",
            Self::Hiding => "隐身术",
            Self::SoulShield => "幽灵盾",
            // Assassin
            Self::FatalSword => "致命剑术",
            Self::DoubleSlash => "双倍斩",
            Self::Haste => "加速",
            Self::FlashDash => "闪避",
            // Archer
            Self::Focus => "集中",
            Self::StraightShot => "直射",
            Self::DoubleShot => "双射",
            Self::Meditation => "冥想",
            _ => "未知技能"
        }
    }
    
    /// 获取技能所需职业
    pub fn required_class(&self) -> MirClass {
        let id = *self as u8;
        if id >= 1 && id <= 17 { MirClass::Warrior }
        else if id >= 31 && id <= 55 { MirClass::Wizard }
        else if id >= 61 && id <= 86 { MirClass::Taoist }
        else if id >= 91 && id <= 107 { MirClass::Assassin }
        else if id >= 121 && id <= 141 { MirClass::Archer }
        else { MirClass::Warrior } // 默认
    }
}

/// 已学会的技能数据
#[derive(Debug, Clone)]
pub struct LearnedMagic {
    pub spell: SpellType,
    pub level: u8,        // 技能等级 (0-3)
    pub experience: u32,  // 技能经验
    pub key_slot: Option<u8>, // 绑定的快捷键槽位 (F1-F8)
}

impl LearnedMagic {
    pub fn new(spell: SpellType) -> Self {
        Self {
            spell,
            level: 0,
            experience: 0,
            key_slot: None,
        }
    }
}

/// 玩家已学技能列表组件
#[derive(Debug, Clone)]
pub struct MagicList {
    pub magics: Vec<LearnedMagic>,
}

impl MagicList {
    pub fn new() -> Self {
        Self { magics: Vec::new() }
    }
    
    /// 学会新技能
    pub fn learn(&mut self, spell: SpellType) -> bool {
        if self.has_learned(spell) {
            return false;
        }
        self.magics.push(LearnedMagic::new(spell));
        true
    }
    
    /// 是否已学会某技能
    pub fn has_learned(&self, spell: SpellType) -> bool {
        self.magics.iter().any(|m| m.spell == spell)
    }
    
    /// 获取技能
    pub fn get_mut(&mut self, spell: SpellType) -> Option<&mut LearnedMagic> {
        self.magics.iter_mut().find(|m| m.spell == spell)
    }
    
    /// 获取绑定到某槽位的技能
    pub fn get_by_slot(&self, slot: u8) -> Option<&LearnedMagic> {
        self.magics.iter().find(|m| m.key_slot == Some(slot))
    }
}

impl Default for MagicList {
    fn default() -> Self {
        Self::new()
    }
}

/// 可学习技能列表组件 (NPC 提供或职业默认)
#[derive(Debug, Clone)]
pub struct LearnableMagicList {
    pub spells: Vec<(SpellType, u16)>, // (技能, 所需等级)
}

impl LearnableMagicList {
    pub fn new() -> Self {
        Self { spells: Vec::new() }
    }
    
    /// 添加可学技能
    pub fn add(&mut self, spell: SpellType, required_level: u16) {
        self.spells.push((spell, required_level));
    }
    
    /// 获取玩家当前可学习的技能
    pub fn get_available(&self, player_level: u16, learned: &MagicList) -> Vec<SpellType> {
        self.spells.iter()
            .filter(|(spell, req_level)| {
                *req_level <= player_level && !learned.has_learned(*spell)
            })
            .map(|(spell, _)| *spell)
            .collect()
    }
    
    /// 为职业初始化默认可学技能
    pub fn init_for_class(class: MirClass) -> Self {
        let mut list = Self::new();
        match class {
            MirClass::Warrior => {
                list.add(SpellType::Fencing, 7);
                list.add(SpellType::Slaying, 15);
                list.add(SpellType::Thrusting, 22);
                list.add(SpellType::HalfMoon, 28);
                list.add(SpellType::ShoulderDash, 30);
                list.add(SpellType::LionRoar, 36);
            },
            MirClass::Wizard => {
                list.add(SpellType::FireBall, 7);
                list.add(SpellType::Repulsion, 12);
                list.add(SpellType::ElectricShock, 13);
                list.add(SpellType::GreatFireBall, 15);
                list.add(SpellType::HellFire, 19);
                list.add(SpellType::ThunderBolt, 22);
                list.add(SpellType::Teleport, 25);
                list.add(SpellType::Lightning, 29);
                list.add(SpellType::MagicShield, 31);
            },
            MirClass::Taoist => {
                list.add(SpellType::Healing, 7);
                list.add(SpellType::SpiritSword, 9);
                list.add(SpellType::Poisoning, 14);
                list.add(SpellType::SoulFireBall, 18);
                list.add(SpellType::SummonSkeleton, 19);
                list.add(SpellType::Hiding, 20);
                list.add(SpellType::SoulShield, 24);
            },
            MirClass::Assassin => {
                list.add(SpellType::FatalSword, 7);
                list.add(SpellType::DoubleSlash, 15);
                list.add(SpellType::Haste, 20);
                list.add(SpellType::FlashDash, 25);
            },
            MirClass::Archer => {
                list.add(SpellType::Focus, 7);
                list.add(SpellType::StraightShot, 9);
                list.add(SpellType::DoubleShot, 15);
                list.add(SpellType::Meditation, 20);
            },
        }
        list
    }
}

impl Default for LearnableMagicList {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 目标选择组件
// ============================================================================

/// 目标类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetType {
    None,
    Monster(u32),      // 怪物实体 ID
    Player(u32),       // 玩家实体 ID
    NPC(u32),          // NPC 实体 ID
    Location(i32, i32), // 地面位置 (x, y)
}

/// 当前选中的目标组件
#[derive(Debug, Clone, Copy)]
pub struct TargetSelection {
    pub current: TargetType,
    pub last_update: std::time::Instant,
}

impl TargetSelection {
    pub fn new() -> Self {
        Self {
            current: TargetType::None,
            last_update: std::time::Instant::now(),
        }
    }
    
    pub fn select_monster(&mut self, id: u32) {
        self.current = TargetType::Monster(id);
        self.last_update = std::time::Instant::now();
    }
    
    pub fn select_player(&mut self, id: u32) {
        self.current = TargetType::Player(id);
        self.last_update = std::time::Instant::now();
    }
    
    pub fn select_location(&mut self, x: i32, y: i32) {
        self.current = TargetType::Location(x, y);
        self.last_update = std::time::Instant::now();
    }
    
    pub fn clear(&mut self) {
        self.current = TargetType::None;
        self.last_update = std::time::Instant::now();
    }
    
    pub fn has_target(&self) -> bool {
        !matches!(self.current, TargetType::None)
    }
    
    pub fn get_monster_id(&self) -> Option<u32> {
        match self.current {
            TargetType::Monster(id) => Some(id),
            _ => None,
        }
    }
    
    pub fn get_location(&self) -> Option<(i32, i32)> {
        match self.current {
            TargetType::Location(x, y) => Some((x, y)),
            _ => None,
        }
    }
}

impl Default for TargetSelection {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 任务组件 (从 quest_system.rs 重新导出)
// ============================================================================

// QuestLog 组件在 quest_system.rs 中定义，在这里重新导出
pub use crate::ecs::systems::quest_system::QuestLog;

// ============================================================================
// 交易组件 (从 trade_system.rs 重新导出)
// ============================================================================

// TradeWindow 组件在 trade_system.rs 中定义，在这里重新导出
pub use crate::ecs::systems::trade_system::TradeWindow;

// ============================================================================
// 常量
// ============================================================================

pub const CELL_WIDTH: i32 = 48;
pub const CELL_HEIGHT: i32 = 32;




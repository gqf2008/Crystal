// ============================================================================
// Components - ECS 组件定义
// 参考 C# Client/MirObjects/ 的对象属性
// ============================================================================

use mir2_shared::Point;
pub use mir2_shared::{MirDirection, MirAction, MirClass, MirGender};
use std::time::Instant;
use crate::objects::CellInfo;

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
    
    /// 转换为格子坐标
    pub fn to_grid(&self) -> (i32, i32) {
        ((self.x / 48.0) as i32, (self.y / 32.0) as i32)
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
pub struct DirectionComp {
    pub current: MirDirection,
    pub target: MirDirection,
}

impl DirectionComp {
    pub fn new(dir: MirDirection) -> Self {
        Self { current: dir, target: dir }
    }
}

/// 精灵渲染组件 - 可渲染实体必备
#[derive(Debug, Clone)]
pub struct SpriteComp {
    pub library: i32,      // MLibrary 索引 (0=Tiles, 1=SmTiles, 2=Objects, etc.)
    pub index: i32,        // 贴图索引
    pub frame: i32,        // 当前帧
    pub blend_mode: BlendModeComp, // 混合模式
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BlendModeComp {
    Alpha,
    Add,    // ⭐ ADD 混合 (技能特效)
    Multiply,
}

impl SpriteComp {
    pub fn new(library: i32, index: i32) -> Self {
        Self {
            library,
            index,
            frame: 0,
            blend_mode: BlendModeComp::Alpha,
        }
    }

    pub fn with_blend(library: i32, index: i32, blend_mode: BlendModeComp) -> Self {
        Self { library, index, frame: 0, blend_mode }
    }
}

// ============================================================================
// 动画组件
// ============================================================================

/// 动画状态组件
#[derive(Debug, Clone)]
pub struct AnimationComp {
    pub action: MirAction,
    pub direction: u8,       // 方向 0-7
    pub frame_count: u8,
    pub frame_index: u8,
    pub frame_interval: u32, // 毫秒
    pub frame_timer: u32,
    pub loop_animation: bool,
}

impl AnimationComp {
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
pub struct PlayerComp {
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
pub struct MonsterComp {
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
pub struct NPCComp {
    pub id: u32,
    pub name: String,
    pub npc_index: u16,
    pub dialogue_id: u32,
}

// ============================================================================
// 技能/特效组件
// ============================================================================

/// 技能数据组件
#[derive(Debug, Clone)]
pub struct SpellComp {
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
// 常量
// ============================================================================

pub const CELL_WIDTH: i32 = 48;
pub const CELL_HEIGHT: i32 = 32;

# objects - 游戏对象系统

**对应C#代码**: `Client/MirObjects/`  
**文件数**: 19  
**代码行数**: 10,842  
**状态**: ✅ 核心完成，部分功能待完善

---

## 📚 目录

1. [模块概述](#-模块概述)
2. [架构设计](#-架构设计)
3. [对象类型](#-对象类型)
4. [核心系统](#-核心系统)
5. [使用指南](#-使用指南)
6. [开发状态](#-开发状态)

---

## 📖 模块概述

`objects` 模块是游戏世界中所有可见对象的数据表示层，负责：

- **对象管理**: 定义游戏中所有对象的数据结构和行为
- **状态维护**: 管理对象的状态（位置、动画、属性等）
- **动画控制**: 处理对象的动画帧序列
- **行为逻辑**: 实现对象的移动、攻击、交互等行为
- **数据同步**: 提供与服务器同步的接口

> ⚠️ **重要**: 此模块仅包含**数据和逻辑**，不包含渲染代码。实际渲染由 ECS 系统负责。

### 与C#版本的对应关系

| C# 类 | Rust 文件 | 说明 |
|-------|----------|------|
| `MapObject.cs` | `map_object.rs` | 地图对象基类 |
| `PlayerObject.cs` | `player_object.rs` | 玩家对象基类 |
| `UserObject.cs` | `user_object.rs` | 用户对象（本地玩家） |
| `MonsterObject.cs` | `monster_object.rs` | 怪物对象 |
| `NPCObject.cs` | `npc_object.rs` | NPC对象 |
| `ItemObject.cs` | `item_object.rs` | 物品对象 |
| `HeroObject.cs` | `hero_object.rs` | 英雄对象（宠物） |
| `SpellObject.cs` | `spell_object.rs` | 技能对象 |
| `Effect.cs` | `effect.rs` | 特效对象 |
| `Damage.cs` | `damage.rs` | 伤害显示 |
| `Frame.cs` | `frames.rs` | 动画帧 |
| `MapCode.cs` | `map_code.rs` | 地图数据 |

---

## 🏗 架构设计

### 对象层次结构

```
MapObject (基类)
├── PlayerObject (玩家基类)
│   ├── UserObject (本地玩家)
│   └── HeroObject (英雄/宠物)
├── MonsterObject (怪物)
├── NPCObject (NPC)
├── ItemObject (物品)
├── SpellObject (技能)
└── Effect (特效)

Damage (独立，用于显示伤害数字)
```

### 核心特性

#### 1. **DrawableMapObject Trait** - 可绘制接口

所有需要渲染的对象都实现此 trait：

```rust
pub trait DrawableMapObject {
    // 获取对象ID
    fn object_id(&self) -> u32;
    
    // 获取渲染位置
    fn draw_location(&self) -> Point;
    
    // 获取渲染偏移
    fn draw_offset(&self) -> Point;
    
    // 获取当前帧
    fn current_frame(&self) -> Option<&Frame>;
    
    // 获取当前动作
    fn current_action(&self) -> MirAction;
    
    // 获取当前方向
    fn current_direction(&self) -> MirDirection;
    
    // 是否可见
    fn is_visible(&self) -> bool;
    
    // 绘制顺序 (Y-sorting)
    fn draw_y(&self) -> i32;
}
```

#### 2. **状态机设计** - 分离的行为模式

**PlayerMovementFSM** - 玩家移动状态机：

```rust
pub enum MovementState {
    Idle,                    // 站立
    Walking { start_time: Instant },  // 行走
    Running { start_time: Instant },  // 奔跑
}

impl PlayerMovementFSM {
    // 状态转换
    pub fn start_walking(&mut self) -> bool;
    pub fn start_running(&mut self) -> bool;
    pub fn stop(&mut self) -> bool;
    
    // 查询
    pub fn is_idle(&self) -> bool;
    pub fn is_walking(&self) -> bool;
    pub fn is_running(&self) -> bool;
}
```

#### 3. **动画系统** - Frame Based Animation

```rust
pub struct Frame {
    pub library: LibraryId,    // 图像库ID
    pub index: u32,            // 图像索引
    pub offset: Point,         // 渲染偏移
    pub blend_mode: SpriteBlendMode,  // 混合模式
    pub delay: u32,            // 帧延迟
    pub sound: Option<SoundId>,  // 音效
}

// 动画播放结果
pub struct AnimationAdvanceSummary {
    pub animation_completed: bool,  // 动画是否完成
    pub sound_id: Option<SoundId>,  // 触发的音效
}
```

#### 4. **对象工厂** - 从服务器包创建对象

```rust
pub struct ObjectFactory;

impl ObjectFactory {
    // 从服务器数据包创建对象
    pub fn create_from_packet(
        packet: &ObjectDataPacket
    ) -> Result<Box<dyn DrawableMapObject>>;
    
    // 更新现有对象
    pub fn update_from_packet(
        object: &mut dyn DrawableMapObject,
        packet: &ObjectUpdatePacket
    ) -> Result<()>;
}
```

---

## 📦 对象类型

### 1. MapObject - 地图对象基类

**文件**: `map_object.rs` (~1,500行)  
**职责**: 所有地图对象的基类，定义共有属性和行为

#### 核心属性

```rust
pub struct MapObject {
    // 基础信息
    pub object_id: u32,
    pub name: String,
    pub name_color: Color,
    pub object_type: MapObjectType,
    
    // 位置信息
    pub current_location: Point,
    pub map_location: Point,
    
    // 动画信息
    pub current_action: MirAction,
    pub current_direction: MirDirection,
    pub frames: Vec<Frame>,
    pub frame_index: usize,
    pub frame_interval: u32,
    pub frame_time: Instant,
    
    // 状态
    pub dead: bool,
    pub skeleton: bool,
    pub poison: PoisonType,
    pub hidden: bool,
    
    // 移动
    pub movement_queue: VecDeque<Point>,
    pub movement_speed: f32,
}
```

#### 主要方法

- `update()` - 更新对象状态
- `advance_frame()` - 推进动画帧
- `set_action()` - 设置动作
- `move_to()` - 移动到目标位置
- `attacked()` - 受到攻击
- `die()` - 死亡处理

#### 已实现功能

- ✅ 基础属性管理
- ✅ 动画播放
- ✅ 移动逻辑
- ✅ 状态转换
- ✅ 名字颜色管理
- ✅ 毒药效果
- ✅ 死亡处理

### 2. PlayerObject - 玩家对象基类

**文件**: `player_object.rs` (~800行)  
**职责**: 所有玩家类对象的基类（包括其他玩家和英雄）

#### 扩展属性

```rust
pub struct PlayerObject {
    pub base: MapObject,  // 继承 MapObject
    
    // 玩家特有
    pub gender: MirGender,
    pub class: MirClass,
    pub hair: u8,
    pub level: u16,
    
    // 装备
    pub weapon: Option<i32>,
    pub armor: Option<i32>,
    pub helmet: Option<i32>,
    
    // 状态
    pub light: u8,
    pub攻击速度: i16,
    pub mount: Option<MountType>,
    pub wings: Option<WingType>,
    
    // 动作队列
    pub queued_actions: VecDeque<QueuedAction>,
}

pub struct QueuedAction {
    pub action: MirAction,
    pub direction: MirDirection,
    pub location: Point,
}
```

#### 特殊功能

- ✅ **装备显示**: 武器、护甲、头盔等
- ✅ **坐骑系统**: 骑乘状态管理
- ✅ **翅膀系统**: 翅膀显示
- ✅ **动作队列**: 动作排队执行
- ✅ **等级显示**: 等级和职业

### 3. UserObject - 用户对象（本地玩家）

**文件**: `user_object.rs` (~2,500行)  
**职责**: 本地玩家对象，包含完整的玩家状态

#### 完整属性

```rust
pub struct UserObject {
    pub player: PlayerObject,  // 继承 PlayerObject
    
    // 属性
    pub hp: u32,
    pub max_hp: u32,
    pub mp: u32,
    pub max_mp: u32,
    pub ac: u16,
    pub mac: u16,
    pub dc: u16,
    pub mc: u16,
    pub sc: u16,
    
    // 经验
    pub experience: u64,
    pub max_experience: u64,
    
    // 背包和装备
    pub inventory: Vec<UserItem>,
    pub equipment: HashMap<EquipmentSlot, UserItem>,
    pub storage: Vec<UserItem>,
    
    // 技能
    pub magics: Vec<ClientMagic>,
    
    // 任务
    pub quests: Vec<ClientQuestProgress>,
    
    // 宠物
    pub intelligent_creatures: Vec<ClientIntelligentCreature>,
    
    // 货币
    pub gold: u64,
    pub credit: u64,
}
```

#### 主要功能

- ✅ **属性管理**: HP/MP/攻击/防御等
- ✅ **背包系统**: 物品增删改查
- ✅ **装备系统**: 穿戴卸下装备
- ✅ **技能系统**: 学习使用技能
- ✅ **任务系统**: 任务进度跟踪
- ✅ **宠物系统**: 宠物管理
- 🚧 **交易系统**: 与其他玩家交易
- 🚧 **邮件系统**: 收发邮件

### 4. MonsterObject - 怪物对象

**文件**: `monster_object.rs` (~1,200行)  
**职责**: 怪物对象，包含AI行为

#### 怪物属性

```rust
pub struct MonsterObject {
    pub base: MapObject,
    
    // 怪物信息
    pub monster: Monster,  // 怪物配置数据
    pub ai_mode: AIMode,
    pub pet_owner: Option<u32>,
    
    // 状态
    pub extra_attack_flag: bool,
    pub attack_speed: i16,
}

pub struct Monster {
    pub name: String,
    pub image: u16,
    pub level: u16,
    pub hp: u32,
    pub view_range: u8,
    pub cool_eye: u8,
    pub attack_speed: i16,
    pub move_speed: i16,
}
```

#### AI模式

```rust
pub enum AIMode {
    None,          // 无AI
    Guard,         // 守卫模式（固定范围）
    Aggressive,    // 主动攻击
    Passive,       // 被动（不主动攻击）
    Patrol,        // 巡逻模式
}
```

#### 已实现功能

- ✅ 基础属性
- ✅ AI模式切换
- ✅ 宠物标识
- ✅ 攻击动画
- ✅ 死亡动画
- ✅ 音效播放
- 🚧 复杂AI行为（由ECS系统实现）

### 5. NPCObject - NPC对象

**文件**: `npc_object.rs` (~600行)  
**职责**: NPC对象，处理对话和服务

#### NPC属性

```rust
pub struct NPCObject {
    pub base: MapObject,
    
    // NPC信息
    pub image: NpcImage,
    pub turn_time: Instant,
    pub can_turn: bool,
    
    // 交互
    pub has_shop: bool,
    pub has_quest: bool,
    pub has_craft: bool,
}

pub enum NpcImage {
    Normal(u16),       // 普通NPC图像
    Special(u16),      // 特殊NPC
    Monster(u16),      // 怪物NPC
}
```

#### 主要功能

- ✅ NPC显示
- ✅ 自动转向
- ✅ 对话系统接口
- ✅ 商店标识
- ✅ 任务标识
- 🚧 完整对话树（由UI系统实现）

### 6. ItemObject - 物品对象

**文件**: `item_object.rs` (~400行)  
**职责**: 地面掉落物品

#### 物品属性

```rust
pub struct ItemObject {
    pub base: MapObject,
    
    // 物品信息
    pub item: UserItem,
    pub item_image: u16,
    pub owner: Option<u32>,  // 归属玩家
    pub expire_time: Option<Instant>,
}
```

#### 主要功能

- ✅ 物品显示
- ✅ 归属权限
- ✅ 过期时间
- ✅ 物品图标
- ✅ 名字颜色（品质）
- ✅ 拾取交互

### 7. HeroObject - 英雄对象（宠物）

**文件**: `hero_object.rs` (~500行)  
**职责**: 玩家的英雄/宠物

#### 英雄属性

```rust
pub struct HeroObject {
    pub player: PlayerObject,  // 继承 PlayerObject
    
    // 英雄特有
    pub owner_id: u32,
    pub hero_state: HeroState,
    
    // 属性
    pub hp: u32,
    pub max_hp: u32,
    pub level: u16,
    pub experience: u64,
}

pub enum HeroState {
    Follow,      // 跟随
    Attack,      // 攻击
    Guard,       // 守卫
    Standby,     // 待命
}
```

#### 主要功能

- ✅ 基础属性
- ✅ 跟随逻辑
- ✅ 状态切换
- 🚧 完整AI（攻击、守卫等）
- 🚧 装备系统
- 🚧 技能系统

### 8. SpellObject - 技能对象

**文件**: `spell_object.rs` (~600行)  
**职责**: 魔法技能特效

#### 技能属性

```rust
pub struct SpellObject {
    pub base: MapObject,
    
    // 技能信息
    pub spell: Spell,
    pub caster_id: u32,
    pub target_id: Option<u32>,
    pub target_location: Point,
    
    // 特效
    pub effect_type: EffectType,
    pub repeat: bool,
}

pub enum EffectType {
    Projectile,    // 飞行物（火球等）
    Immediate,     // 即时（闪电等）
    Area,          // 范围（爆炸等）
    Buff,          // 增益效果
    Debuff,        // 减益效果
}
```

#### 主要功能

- ✅ 飞行物轨迹
- ✅ 碰撞检测
- ✅ 特效播放
- ✅ 目标追踪
- 🚧 复杂特效链
- 🚧 Buff/Debuff可视化

### 9. Effect - 特效对象

**文件**: `effect.rs` (~800行)  
**职责**: 视觉特效

#### 特效属性

```rust
pub struct Effect {
    // 基础信息
    pub effect_id: u32,
    pub location: Point,
    pub layer: EffectLayer,
    
    // 动画
    pub frames: Vec<Frame>,
    pub frame_index: usize,
    pub repeat: bool,
    pub repeat_until: Option<Instant>,
    
    // 渲染
    pub blend_mode: SpriteBlendMode,
    pub light: u8,
}

pub enum EffectLayer {
    BelowObject,   // 在对象下方
    AboveObject,   // 在对象上方
    GroundItem,    // 地面物品层
}

pub enum SpriteBlendMode {
    Normal,        // 正常
    Additive,      // 叠加
    Multiply,      // 相乘
    Screen,        // 滤色
}
```

#### 特效类型

- ✅ **击中特效**: 攻击命中
- ✅ **技能特效**: 技能释放
- ✅ **环境特效**: 火焰、水流等
- ✅ **状态特效**: Buff/Debuff图标
- 🚧 **粒子特效**: 复杂粒子系统

### 10. Damage - 伤害显示

**文件**: `damage.rs` (~300行)  
**职责**: 伤害数字显示

#### 伤害属性

```rust
pub struct Damage {
    // 信息
    pub damage_value: i32,
    pub damage_type: DamageType,
    pub location: Point,
    
    // 显示
    pub color: Color,
    pub create_time: Instant,
    pub duration: Duration,
    pub offset_y: f32,  // 上升偏移
}

pub enum DamageType {
    Hit,           // 普通伤害
    Critical,      // 暴击
    Miss,          // 未命中
    Heal,          // 治疗
    Mana,          // 魔法值
    Poison,        // 毒药
}
```

#### 显示效果

- ✅ 数字上浮动画
- ✅ 颜色区分类型
- ✅ 暴击放大
- ✅ 渐隐效果
- ✅ 多伤害堆叠

---

## 🔧 核心系统

### 1. 动画系统 (frames.rs)

**职责**: 管理所有对象的动画帧序列

#### Frame结构

```rust
pub struct Frame {
    pub library: LibraryId,    // 使用哪个图像库
    pub index: u32,            // 图像索引
    pub offset: Point,         // 渲染偏移
    pub blend_mode: SpriteBlendMode,
    pub delay: u32,            // 帧延迟 (ms)
    pub sound: Option<SoundId>,
}
```

#### 动画播放

```rust
impl Frame {
    // 推进动画
    pub fn advance(
        frames: &[Frame],
        frame_index: &mut usize,
        last_frame_time: &mut Instant,
        loop_animation: bool,
    ) -> AnimationAdvanceSummary;
}

pub struct AnimationAdvanceSummary {
    pub animation_completed: bool,
    pub sound_id: Option<SoundId>,
}
```

#### 已实现功能

- ✅ 帧序列管理
- ✅ 循环/单次播放
- ✅ 帧延迟控制
- ✅ 音效触发
- ✅ 混合模式
- ✅ 渲染偏移

### 2. 移动状态机 (player_movement_fsm.rs)

**职责**: 管理玩家的移动状态

#### 状态定义

```rust
pub enum MovementState {
    Idle,
    Walking { start_time: Instant },
    Running { start_time: Instant },
}
```

#### 状态转换

```rust
impl PlayerMovementFSM {
    pub fn new() -> Self;
    
    // 转换到行走
    pub fn start_walking(&mut self) -> bool;
    
    // 转换到奔跑
    pub fn start_running(&mut self) -> bool;
    
    // 停止移动
    pub fn stop(&mut self) -> bool;
    
    // 状态查询
    pub fn is_idle(&self) -> bool;
    pub fn is_walking(&self) -> bool;
    pub fn is_running(&self) -> bool;
    pub fn current_state(&self) -> &MovementState;
}
```

#### 特性

- ✅ 清晰的状态转换
- ✅ 时间戳记录
- ✅ 状态查询
- ✅ 线程安全

### 3. 寻路系统 (pathfinder.rs)

**职责**: A*寻路算法实现

#### 主要接口

```rust
pub struct PathFinder {
    // 寻路配置
    max_distance: u32,
    diagonal_cost: u32,
}

impl PathFinder {
    pub fn new() -> Self;
    
    // 寻找路径
    pub fn find_path(
        &self,
        start: Point,
        end: Point,
        is_valid_cell: impl Fn(Point) -> bool,
    ) -> Option<Vec<Point>>;
    
    // 计算启发式
    fn heuristic(&self, a: Point, b: Point) -> u32;
    
    // 获取相邻格子
    fn get_neighbors(&self, pos: Point) -> Vec<(Point, u32)>;
}
```

#### 特性

- ✅ A*算法
- ✅ 曼哈顿距离启发式
- ✅ 对角线移动支持
- ✅ 障碍物检测
- ✅ 路径平滑
- 🚧 跳点搜索(JPS)优化

### 4. 地图数据 (map_code.rs)

**职责**: 地图数据读取和碰撞检测（对应C# MapCode.cs）

#### 核心结构

```rust
pub struct MapReader {
    width: i32,
    height: i32,
    cells: Vec<CellInfo>,
}

pub struct CellInfo {
    pub back_image: i32,
    pub middle_image: i32,
    pub front_image: i32,
    pub back_index: i32,
    pub middle_index: i32,
    pub front_index: i32,
    pub door_index: u8,
    pub door_offset: u8,
    pub front_anim_frame: u8,
    pub middle_anim_frame: u8,
    pub front_anim_tick: u16,
    pub middle_anim_tick: u16,
    pub tile_anim_frame_index: u8,
    pub object_flags: CellAttribute,
}

bitflags! {
    pub struct CellAttribute: u8 {
        const NONE = 0;
        const HIGH_WALL = 1;
        const LOW_WALL = 2;
        const UNKNOWN = 4;
    }
}
```

#### 主要功能

- ✅ 读取.map文件
- ✅ 获取格子信息
- ✅ 碰撞检测
- ✅ 格子属性查询
- ✅ 地图边界检查

### 5. 属性扩展 (stats_ext.rs)

**职责**: 扩展Stats结构，提供计算方法

```rust
pub trait StatsExt {
    // 属性计算
    fn calculate_attack_power(&self) -> u16;
    fn calculate_defense(&self) -> u16;
    fn calculate_magic_defense(&self) -> u16;
    
    // 战斗计算
    fn calculate_damage(&self, target: &Stats) -> u32;
    fn calculate_hit_rate(&self, target: &Stats) -> f32;
    fn calculate_crit_rate(&self) -> f32;
}

impl StatsExt for Stats {
    // 实现细节...
}
```

### 6. 对象工厂 (object_factory.rs)

**职责**: 从服务器数据包创建对象

```rust
pub struct ObjectFactory;

impl ObjectFactory {
    // 创建对象
    pub fn create_player(data: &PlayerData) -> PlayerObject;
    pub fn create_monster(data: &MonsterData) -> MonsterObject;
    pub fn create_npc(data: &NPCData) -> NPCObject;
    pub fn create_item(data: &ItemData) -> ItemObject;
    
    // 更新对象
    pub fn update_object(
        object: &mut dyn DrawableMapObject,
        data: &ObjectUpdate,
    ) -> Result<()>;
}
```

---

## 📖 使用指南

### 创建玩家对象

```rust
use crate::objects::*;

// 创建玩家
let mut player = PlayerObject::new(
    1001,                    // object_id
    "玩家名字".to_string(),
    Point::new(100, 100),    // 位置
    MirGender::Male,
    MirClass::Warrior,
);

// 设置装备
player.weapon = Some(1);
player.armor = Some(2);

// 设置动作
player.base.set_action(MirAction::Standing);

// 移动
player.base.move_to(Point::new(105, 105));

// 更新（每帧调用）
player.base.update(delta_time);
```

### 创建怪物

```rust
// 怪物配置
let monster_config = Monster {
    name: "猎鹰".to_string(),
    image: 1,
    level: 5,
    hp: 100,
    view_range: 7,
    cool_eye: 0,
    attack_speed: 1200,
    move_speed: 1800,
};

// 创建怪物对象
let mut monster = MonsterObject::new(
    2001,
    monster_config,
    Point::new(120, 120),
);

// 设置AI模式
monster.ai_mode = AIMode::Aggressive;

// 更新
monster.base.update(delta_time);
```

### 播放动画

```rust
// 设置动作（自动加载对应的帧序列）
object.set_action(MirAction::Attack1);

// 手动推进动画
let summary = Frame::advance(
    &object.frames,
    &mut object.frame_index,
    &mut object.frame_time,
    true,  // 循环播放
);

// 检查动画完成
if summary.animation_completed {
    println!("动画播放完毕");
}

// 播放音效
if let Some(sound_id) = summary.sound_id {
    sound_manager.play(sound_id);
}
```

### 寻路

```rust
let pathfinder = PathFinder::new();

// 定义障碍物检测
let is_valid = |pos: Point| {
    // 检查地图边界
    if pos.x < 0 || pos.y < 0 || pos.x >= map_width || pos.y >= map_height {
        return false;
    }
    
    // 检查碰撞
    let cell = map_reader.get_cell(pos);
    !cell.object_flags.intersects(CellAttribute::HIGH_WALL | CellAttribute::LOW_WALL)
};

// 寻找路径
if let Some(path) = pathfinder.find_path(
    current_pos,
    target_pos,
    is_valid,
) {
    // 沿路径移动
    for waypoint in path {
        object.move_to(waypoint);
    }
}
```

### 显示伤害

```rust
// 创建伤害显示
let damage = Damage::new(
    123,                      // 伤害值
    DamageType::Critical,     // 暴击
    target_location,
);

// 渲染伤害数字（在渲染系统中）
if !damage.is_expired() {
    let pos = damage.get_display_position();
    let color = damage.color;
    draw_text(damage_text, pos, color);
}
```

---

## 📊 开发状态

### 完成度统计

| 功能模块 | 完成度 | 说明 |
|---------|--------|------|
| **MapObject** | 95% | 基础完成，部分特效待完善 |
| **PlayerObject** | 90% | 核心完成，高级功能待完善 |
| **UserObject** | 85% | 主要功能完成，交易/邮件待完善 |
| **MonsterObject** | 90% | 基础完成，复杂AI由ECS实现 |
| **NPCObject** | 85% | 显示完成，对话树待完善 |
| **ItemObject** | 95% | 完成 |
| **HeroObject** | 60% | 基础框架，AI和系统待完善 |
| **SpellObject** | 70% | 基础特效完成，复杂特效待完善 |
| **Effect** | 80% | 基础特效完成，粒子系统待实现 |
| **Damage** | 95% | 完成 |
| **Frames** | 100% | 完成 |
| **PathFinder** | 80% | A*完成，优化待进行 |
| **MapReader** | 100% | 完成 |

### 已实现功能清单

#### ✅ 核心功能

- [x] 对象层次结构
- [x] DrawableMapObject trait
- [x] 动画系统
- [x] 移动系统
- [x] 状态管理
- [x] 对象工厂

#### ✅ 玩家系统

- [x] 玩家对象
- [x] 用户对象（本地玩家）
- [x] 属性系统
- [x] 背包系统
- [x] 装备系统
- [x] 技能系统（数据层）
- [x] 移动状态机

#### ✅ 对象类型

- [x] 怪物对象
- [x] NPC对象
- [x] 物品对象
- [x] 英雄对象（基础）
- [x] 技能对象
- [x] 特效对象
- [x] 伤害显示

#### ✅ 工具系统

- [x] 寻路算法
- [x] 地图数据读取
- [x] 属性计算
- [x] 碰撞检测

### 未实现功能清单

#### ⏳ 高级功能

- [ ] **完整的宠物系统**: HeroObject AI和装备
- [ ] **骑乘系统**: 坐骑数据和动画
- [ ] **变身系统**: 角色变身
- [ ] **称号系统**: 称号显示

#### ⏳ 玩家功能

- [ ] **交易系统**: 玩家间交易数据
- [ ] **邮件系统**: 邮件数据
- [ ] **好友系统**: 好友列表
- [ ] **公会数据**: 公会成员信息

#### ⏳ 对象功能

- [ ] **复杂AI**: 更多AI模式（逃跑、徘徊等）
- [ ] **对话树**: 完整的NPC对话系统
- [ ] **任务链**: 复杂任务系统

#### ⏳ 优化

- [ ] **对象池**: 对象复用
- [ ] **寻路缓存**: 路径缓存
- [ ] **JPS算法**: 跳点搜索优化
- [ ] **LOD系统**: 远距离对象简化

---

## 🚀 未来规划

### 短期目标 (1-2周)

1. **完善HeroObject** 🔴 高优先级
   - 实现完整的跟随AI
   - 添加装备系统
   - 实现技能系统
   - 添加状态切换逻辑

2. **优化寻路系统** 🟡 中优先级
   - 实现路径缓存
   - 添加JPS优化
   - 优化性能

3. **对象池管理** 🟡 中优先级
   - 实现对象池
   - 优化创建和销毁
   - 减少内存分配

### 中期目标 (3-4周)

4. **交易系统数据层** 🟡 中优先级
   - 交易状态管理
   - 物品锁定
   - 交易验证

5. **邮件系统数据层** 🟢 低优先级
   - 邮件数据结构
   - 附件管理
   - 过期处理

6. **完善特效系统** 🟡 中优先级
   - 更多特效类型
   - 特效链
   - Buff/Debuff可视化

### 长期目标 (1-2月)

7. **性能优化**
   - 对象池
   - 空间索引（四叉树/网格）
   - 视野裁剪
   - LOD系统

8. **扩展性改进**
   - 插件化对象系统
   - 脚本驱动的对象行为
   - 自定义对象类型
   - 数据驱动的动画

---

## 🐛 已知问题

### 高优先级

- [ ] 多个对象同时移动时偶尔碰撞
- [ ] 动画在快速切换动作时有时跳帧
- [ ] 寻路在复杂地形中性能较低

### 中优先级

- [ ] HeroObject 跟随逻辑不够平滑
- [ ] SpellObject 碰撞检测不够精确
- [ ] 对象创建时内存分配较多

### 低优先级

- [ ] 部分动画偏移不够准确
- [ ] 伤害数字堆叠时显示拥挤
- [ ] 对象名字过长时显示溢出

---

## 📝 代码规范

### 对象命名

- 对象类型名: `XxxObject` (例如: `PlayerObject`)
- 数据结构: `XxxData` (例如: `MonsterData`)
- 配置: `XxxConfig` (例如: `MonsterConfig`)

### 方法命名

- 更新: `update()`
- 设置状态: `set_xxx()`
- 获取状态: `get_xxx()` 或属性访问
- 动作: `action_verb()` (例如: `move_to()`, `attack()`)

### 错误处理

```rust
// 使用 Result 返回错误
pub fn create_from_packet(packet: &ObjectData) -> Result<PlayerObject> {
    // 验证数据
    if packet.id == 0 {
        return Err(anyhow!("Invalid object ID"));
    }
    
    // 创建对象
    Ok(PlayerObject::new(/* ... */))
}
```

### 注释规范

```rust
/// 玩家对象基类
/// 
/// 所有玩家类型对象的基类，包括：
/// - 其他玩家
/// - 英雄/宠物
/// 
/// # Examples
/// 
/// ```
/// let player = PlayerObject::new(1001, "玩家".to_string(), Point::new(100, 100));
/// player.set_action(MirAction::Standing);
/// ```
pub struct PlayerObject {
    // ...
}
```

---

## 🔗 相关文档

### 内部文档

- **ECS系统**: `../ecs/systems/README.md` - 对象的渲染和更新逻辑
- **网络模块**: `../network/README.md` - 对象的网络同步
- **图形模块**: `../graphics/README.md` - 对象的图像资源

### 外部资源

- **C#原版**: `Client/MirObjects/` - C#版本参考
- **共享代码**: `mir2_shared` - 共享数据结构

---

## 💡 设计原则

### 1. 数据与渲染分离

对象模块**只负责数据和逻辑**，不包含任何渲染代码：

```rust
// ✅ 正确：数据层
pub struct PlayerObject {
    pub current_location: Point,
    pub current_action: MirAction,
}

// ❌ 错误：不在对象中进行渲染
impl PlayerObject {
    pub fn draw(&self, ctx: &mut Context) {  // ❌ 不应该在这里
        // 渲染代码...
    }
}
```

**渲染由ECS的RenderSystem负责**。

### 2. 可测试性

所有逻辑都应该易于测试：

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_player_movement() {
        let mut player = PlayerObject::new(/* ... */);
        player.move_to(Point::new(10, 10));
        assert_eq!(player.current_location, Point::new(10, 10));
    }
}
```

### 3. 类型安全

使用强类型而非魔法数字：

```rust
// ✅ 正确
pub enum MirAction {
    Standing,
    Walking,
    Running,
    Attack1,
}

// ❌ 错误
pub const ACTION_STAND: u8 = 0;
pub const ACTION_WALK: u8 = 1;
```

### 4. 所有权清晰

明确对象的所有权关系：

```rust
// 主对象拥有子对象
pub struct UserObject {
    pub player: PlayerObject,  // 拥有
    pub inventory: Vec<UserItem>,  // 拥有
}

// 对象引用其他对象
pub struct HeroObject {
    pub owner_id: u32,  // 引用（通过ID）
}
```

---

**文档版本**: v1.0  
**最后更新**: 2025-10-28  
**维护者**: Crystal Mir2 Team

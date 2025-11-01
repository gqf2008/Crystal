# ecs/components - ECS组件定义

**文件数**: 17  
**代码行数**: ~2,800  
**状态**: ✅ 完成

---

## 📚 目录

1. [模块概述](#-模块概述)
2. [组件分类](#-组件分类)
3. [核心组件](#-核心组件)
4. [组件详解](#-组件详解)
5. [使用指南](#-使用指南)

---

## 📖 模块概述

`components` 目录包含所有 ECS 组件的定义。组件是纯数据结构，不包含逻辑。

### 设计原则

1. **数据导向**: 组件只包含数据，不包含方法
2. **组合优于继承**: 通过组合组件实现不同实体
3. **高内聚低耦合**: 每个组件职责单一
4. **性能优先**: 结构紧凑，缓存友好

### 文件结构

```
components/
├── mod.rs                  # 模块入口，统一导出
├── core.rs                 # 核心组件（Position, Velocity等）
├── movement.rs             # 移动组件
├── animation_state.rs      # 动画状态组件
├── player.rs               # 玩家组件
├── actor.rs                # 角色组件（Monster, NPC）
├── combat.rs               # 战斗组件
├── spell.rs                # 技能组件
├── item.rs                 # 物品组件
├── map.rs                  # 地图组件
├── render.rs               # 渲染组件
├── input.rs                # 输入组件
├── network.rs              # 网络组件
├── prediction.rs           # 预测组件
├── sound.rs                # 音效组件
└── debug.rs                # 调试组件
```

---

## 📦 组件分类

### 1. 核心组件 (core.rs)

最基础的组件，几乎所有实体都需要：

| 组件 | 用途 | 必需性 |
|------|------|--------|
| **Entity** | 实体ID标记 | ✅ 必需 |
| **Position** | 世界坐标（f32） | ✅ 必需 |
| **Velocity** | 速度（移动实体） | 移动实体必需 |
| **MovementAnimation** | 动画帧插值 | 移动实体必需 |
| **Direction** | 朝向（8方向） | ✅ 必需 |
| **CurrentAction** | 当前动作 | ✅ 必需 |
| **Name** | 显示名称 | 显示实体必需 |
| **LocalPlayer** | 本地玩家标记 | 仅本地玩家 |

### 0. 全局事件组件 (events.rs) ⭐ 重要

**GlobalEvents** - 全局事件总线（单例组件）：

| 字段 | 类型 | 用途 |
|------|------|------|
| **input_events** | `Vec<InputEvent>` | 键盘、鼠标、IME事件 |
| **net_events** | `CategorizedEvents` | 分类网络事件（11个类别） |
| **frame_event_count** | `usize` | 当前帧事件计数 |
| **total_event_count** | `u64` | 总事件计数 |
| **enable_logging** | `bool` | 是否启用事件日志 |

**使用方式**:
```rust
// 读取事件（场景/系统）
let events = world.global_events();
for event in &events.input_events {
    // 处理事件
}

// 清理事件（GameState）
world.global_events_mut().clear_frame_events();
```

**生命周期**: 由 `GameState` 管理
- 收集阶段：`collect_network_events()`, ggez 事件回调
- 处理阶段：`Scene::update()`, 各个 ECS 系统
- 清理阶段：`clear_global_events()` 每帧结束时调用

### 2. 移动组件 (movement.rs)

处理移动和寻路：

| 组件 | 用途 |
|------|------|
| **VelocityComponent** | 速度控制 |
| **PathComponent** | 寻路路径 |
| **MovementStateComponent** | 移动状态 |

### 3. 动画组件 (animation_state.rs)

管理动画状态：

| 组件 | 用途 |
|------|------|
| **AnimationState** | 当前动画状态 |
| **FrameIndex** | 当前帧索引 |
| **FrameTimer** | 帧计时器 |

### 4. 玩家组件 (player.rs)

玩家特有属性：

| 组件 | 用途 |
|------|------|
| **Player** | 玩家标记 |
| **Level** | 等级 |
| **Experience** | 经验值 |
| **Stats** | 属性（HP/MP/攻击等） |
| **Equipment** | 装备 |
| **Inventory** | 背包 |
| **MagicList** | 技能列表 |

### 5. 角色组件 (actor.rs)

怪物和NPC：

| 组件 | 用途 |
|------|------|
| **Monster** | 怪物标记 |
| **MonsterAI** | 怪物AI |
| **NPC** | NPC标记 |
| **Merchant** | 商人标记 |

### 6. 战斗组件 (combat.rs)

战斗相关：

| 组件 | 用途 |
|------|------|
| **Health** | 生命值 |
| **Mana** | 魔法值 |
| **Attack** | 攻击力 |
| **Defense** | 防御力 |
| **AttackTarget** | 攻击目标 |
| **Buff** | 增益效果 |
| **Debuff** | 减益效果 |

### 7. 技能组件 (spell.rs)

技能系统：

| 组件 | 用途 |
|------|------|
| **Magic** | 技能数据 |
| **Spell** | 技能实例 |
| **SpellCooldown** | 技能冷却 |
| **CastingSpell** | 正在施放的技能 |

### 8. 物品组件 (item.rs)

物品相关：

| 组件 | 用途 |
|------|------|
| **Item** | 物品数据 |
| **GroundItem** | 地面物品 |
| **ItemOwner** | 物品归属 |

### 9. 地图组件 (map.rs)

地图和场景：

| 组件 | 用途 |
|------|------|
| **MapData** | 地图数据 |
| **MapTile** | 地图瓦片 |
| **CellInfo** | 格子信息 |
| **MapObject** | 地图物件 |

### 10. 渲染组件 (render.rs)

渲染相关：

| 组件 | 用途 |
|------|------|
| **Sprite** | 精灵图像 |
| **SpriteLayer** | 渲染层级 |
| **Camera** | 相机 |
| **RenderConfig** | 渲染配置 |
| **Visible** | 可见性 |

### 11. 输入组件 (input.rs)

玩家输入：

| 组件 | 用途 |
|------|------|
| **PlayerInputComponent** | 玩家输入 |
| **MouseInput** | 鼠标输入 |
| **KeyboardInput** | 键盘输入 |

### 12. 网络组件 (network.rs)

网络同步：

| 组件 | 用途 |
|------|------|
| **NetworkId** | 服务器对象ID |
| **ServerStateComponent** | 服务器状态 |
| **SyncComponent** | 同步组件 |

### 13. 预测组件 (prediction.rs)

客户端预测：

| 组件 | 用途 |
|------|------|
| **PredictionComponent** | 预测状态 |
| **InterpolationComponent** | 插值组件 |

### 14. 音效组件 (sound.rs)

音效触发：

| 组件 | 用途 |
|------|------|
| **SoundTrigger** | 音效触发器 |
| **SpatialSound** | 空间音效 |

### 15. 调试组件 (debug.rs)

调试信息：

| 组件 | 用途 |
|------|------|
| **DebugInfo** | 调试信息 |
| **DebugDraw** | 调试绘制 |

---

## 🔍 核心组件

### Position - 位置组件

```rust
/// 位置组件 - 世界坐标（像素级，支持浮点）
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
```

**用途**:
- 存储实体在世界中的位置
- 使用浮点数支持平滑移动
- 与格子坐标相互转换

### Velocity - 速度组件

```rust
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
```

**用途**:
- 表示实体的移动速度
- 由移动系统更新位置

### MovementAnimation - 动画帧插值组件

```rust
/// 动画帧插值组件 - 实现原版C#的OffSetMove机制
#[derive(Debug, Clone, Copy)]
pub struct MovementAnimation {
    /// 当前格子位置（服务器确认的）
    pub current_grid: (i32, i32),
    
    /// 移动目标格子位置
    pub movement_grid: (i32, i32),
    
    /// 动画帧插值偏移（像素）
    pub offset_move: (f32, f32),
    
    /// 移动距离（格子数）: Walk=1, Run=2, Mount=3
    pub move_distance: i32,
}
```

**用途**:
- 实现平滑的移动动画
- 同步服务器位置和客户端动画
- 支持行走、奔跑、骑乘不同速度

### Direction - 方向组件

```rust
/// 方向组件
#[derive(Debug, Clone, Copy)]
pub struct Direction(pub MirDirection);

pub enum MirDirection {
    Up,
    UpRight,
    Right,
    DownRight,
    Down,
    DownLeft,
    Left,
    UpLeft,
}
```

**用途**:
- 表示实体朝向
- 影响动画帧选择
- 影响移动方向

### CurrentAction - 动作组件

```rust
/// 当前动作组件
#[derive(Debug, Clone, Copy)]
pub struct CurrentAction(pub MirAction);

pub enum MirAction {
    Standing,
    Walking,
    Running,
    Attack1,
    Attack2,
    Attack3,
    Spell,
    Harvest,
    Die,
    Dead,
    // ... 更多动作
}
```

**用途**:
- 表示当前动作状态
- 决定播放哪个动画
- 影响行为逻辑

---

## 📋 组件详解

### 玩家组件 (player.rs)

#### Stats - 属性组件

```rust
pub struct Stats {
    // 基础属性
    pub hp: u32,
    pub max_hp: u32,
    pub mp: u32,
    pub max_mp: u32,
    
    // 攻击属性
    pub dc: u16,        // 物理攻击
    pub mc: u16,        // 魔法攻击
    pub sc: u16,        // 道术攻击
    
    // 防御属性
    pub ac: u16,        // 物理防御
    pub mac: u16,       // 魔法防御
    
    // 其他属性
    pub accuracy: u16,  // 准确
    pub agility: u16,   // 敏捷
    pub luck: u8,       // 幸运
}
```

#### Equipment - 装备组件

```rust
pub struct Equipment {
    pub weapon: Option<UserItem>,
    pub armor: Option<UserItem>,
    pub helmet: Option<UserItem>,
    pub necklace: Option<UserItem>,
    pub bracelet_l: Option<UserItem>,
    pub bracelet_r: Option<UserItem>,
    pub ring_l: Option<UserItem>,
    pub ring_r: Option<UserItem>,
    pub boots: Option<UserItem>,
    pub belt: Option<UserItem>,
}
```

#### Inventory - 背包组件

```rust
pub struct Inventory {
    pub items: Vec<Option<UserItem>>,
    pub max_slots: usize,
    pub gold: u64,
}
```

### 战斗组件 (combat.rs)

#### Health - 生命值组件

```rust
pub struct Health {
    pub current: u32,
    pub maximum: u32,
}

impl Health {
    pub fn percentage(&self) -> f32 {
        self.current as f32 / self.maximum as f32
    }
    
    pub fn is_alive(&self) -> bool {
        self.current > 0
    }
}
```

#### AttackTarget - 攻击目标组件

```rust
pub struct AttackTarget {
    pub target_entity: Entity,
    pub last_attack_time: Instant,
}
```

#### Buff - 增益效果组件

```rust
pub struct Buff {
    pub buff_type: BuffType,
    pub start_time: Instant,
    pub duration: Duration,
    pub stats_modifier: StatsModifier,
}
```

### 移动组件 (movement.rs)

#### PathComponent - 路径组件

```rust
pub struct PathComponent {
    /// 路径点列表（格子坐标）
    pub waypoints: Vec<(i32, i32)>,
    /// 当前路径点索引
    pub current_index: usize,
    /// 是否有效
    pub is_valid: bool,
}

impl PathComponent {
    pub fn set_path(&mut self, waypoints: Vec<(i32, i32)>) {
        self.waypoints = waypoints;
        self.current_index = 0;
        self.is_valid = !self.waypoints.is_empty();
    }
    
    pub fn current_waypoint(&self) -> Option<(i32, i32)> {
        if self.current_index < self.waypoints.len() {
            Some(self.waypoints[self.current_index])
        } else {
            None
        }
    }
}
```

#### MovementStateComponent - 移动状态组件

```rust
pub struct MovementStateComponent {
    pub state: MovementState,
    pub last_state_change: Instant,
}

pub enum MovementState {
    Idle,
    Walking,
    Running,
}
```

### 渲染组件 (render.rs)

#### Sprite - 精灵组件

```rust
pub struct Sprite {
    pub library: LibraryName,
    pub index: u32,
    pub offset: (i16, i16),
    pub blend_mode: SpriteBlendMode,
}
```

#### Camera - 相机组件

```rust
pub struct Camera {
    pub position: Position,
    pub viewport_width: f32,
    pub viewport_height: f32,
    pub zoom: f32,
}
```

### 网络组件 (network.rs)

#### ServerStateComponent - 服务器状态组件

```rust
pub struct ServerStateComponent {
    pub server_position: Position,
    pub server_direction: MirDirection,
    pub server_action: MirAction,
    pub last_update_time: Instant,
}
```

#### PredictionComponent - 预测组件

```rust
pub struct PredictionComponent {
    pub predicted_position: Position,
    pub predicted_velocity: Velocity,
    pub prediction_error: f32,
}
```

---

## 📖 使用指南

### 创建实体

```rust
use hecs::World;
use crate::ecs::components::*;

// 创建玩家实体
fn create_player(world: &mut World, x: f32, y: f32) -> Entity {
    world.spawn((
        Entity::new(),
        Position::new(x, y),
        Velocity::zero(),
        Direction(MirDirection::Down),
        CurrentAction(MirAction::Standing),
        LocalPlayer,
        Player,
        Stats {
            hp: 100,
            max_hp: 100,
            mp: 50,
            max_mp: 50,
            // ... 其他属性
        },
        Equipment::default(),
        Inventory::new(40),
    ))
}

// 创建怪物实体
fn create_monster(world: &mut World, x: f32, y: f32) -> Entity {
    world.spawn((
        Entity::new(),
        Position::new(x, y),
        Direction(MirDirection::Down),
        CurrentAction(MirAction::Standing),
        Monster {
            monster_type: 1,
            ai_mode: AIMode::Aggressive,
        },
        Health { current: 50, maximum: 50 },
    ))
}
```

### 查询组件

```rust
// 查询所有有位置和速度的实体
for (entity, (pos, vel)) in world.query::<(&Position, &Velocity)>().iter() {
    println!("Entity at ({}, {}) with velocity ({}, {})", 
             pos.x, pos.y, vel.dx, vel.dy);
}

// 查询本地玩家
for (entity, (pos, stats)) in world.query::<(&Position, &Stats)>()
    .with::<LocalPlayer>()
    .iter() 
{
    println!("Player HP: {}/{}", stats.hp, stats.max_hp);
}
```

### 修改组件

```rust
// 修改玩家生命值
for (entity, stats) in world.query::<&mut Stats>()
    .with::<LocalPlayer>()
    .iter() 
{
    stats.hp = stats.hp.saturating_sub(10);
    if stats.hp == 0 {
        // 玩家死亡
    }
}

// 修改位置
for (entity, (pos, vel)) in world.query::<(&mut Position, &Velocity)>().iter() {
    pos.x += vel.dx;
    pos.y += vel.dy;
}
```

### 添加/移除组件

```rust
// 添加组件
world.insert_one(entity, Health { current: 100, maximum: 100 })?;

// 移除组件
world.remove_one::<Health>(entity)?;

// 检查组件是否存在
if world.get::<Health>(entity).is_ok() {
    println!("实体有生命值组件");
}
```

---

## 💡 设计模式

### 组合模式

不同实体通过组合不同组件实现：

```rust
// 玩家 = 核心组件 + 玩家组件 + 战斗组件 + 渲染组件
Player: Position + Direction + Player + Stats + Equipment + Sprite

// 怪物 = 核心组件 + 怪物组件 + 战斗组件 + 渲染组件
Monster: Position + Direction + Monster + Health + Sprite

// NPC = 核心组件 + NPC组件 + 渲染组件
NPC: Position + Direction + NPC + Sprite

// 物品 = 核心组件 + 物品组件 + 渲染组件
Item: Position + GroundItem + Sprite
```

### 标记组件

某些组件只是标记，不包含数据：

```rust
pub struct LocalPlayer;    // 标记本地玩家
pub struct Monster;        // 标记怪物
pub struct NPC;            // 标记NPC
pub struct Dead;           // 标记死亡
```

### 可选组件

根据需要添加可选组件：

```rust
// 移动的实体需要
+ Velocity
+ PathComponent

// 有AI的实体需要
+ MonsterAI
+ AttackTarget

// 需要网络同步的实体需要
+ NetworkId
+ ServerStateComponent
```

---

## 🎯 最佳实践

### 1. 保持组件简单

```rust
// ✅ 正确：简单的数据结构
pub struct Health {
    pub current: u32,
    pub maximum: u32,
}

// ❌ 错误：包含逻辑
pub struct Health {
    pub current: u32,
    pub maximum: u32,
    pub regeneration: f32,
    
    pub fn update(&mut self, delta: f32) {
        // 组件不应该有复杂逻辑
    }
}
```

### 2. 使用类型安全

```rust
// ✅ 正确：使用枚举
pub struct Direction(pub MirDirection);

pub enum MirDirection {
    Up, UpRight, Right, //...
}

// ❌ 错误：使用魔法数字
pub struct Direction(pub u8);  // 0=Up, 1=UpRight, ...
```

### 3. 合理的粒度

```rust
// ✅ 正确：适当的组件粒度
pub struct Position { pub x: f32, pub y: f32 }
pub struct Velocity { pub dx: f32, pub dy: f32 }

// ❌ 错误：过度细分
pub struct PositionX(pub f32);
pub struct PositionY(pub f32);
```

### 4. 使用 Copy trait

对于小型组件，实现 Copy trait：

```rust
// ✅ 正确：小型组件实现 Copy
#[derive(Debug, Clone, Copy)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

// ❌ 错误：大型组件实现 Copy
#[derive(Debug, Clone, Copy)]  // Copy 代价太大
pub struct Inventory {
    pub items: [Option<UserItem>; 100],
}
```

---

## 📊 统计信息

### 组件数量

| 分类 | 数量 | 说明 |
|------|------|------|
| 核心组件 | 8 | Position, Velocity等 |
| 移动组件 | 3 | 移动和寻路 |
| 玩家组件 | 6 | 玩家特有属性 |
| 战斗组件 | 7 | 战斗相关 |
| 其他组件 | 20+ | 各种功能组件 |
| **总计** | **44+** | - |

### 内存占用估算

| 组件 | 大小 | 备注 |
|------|------|------|
| Position | 8 bytes | 2×f32 |
| Velocity | 8 bytes | 2×f32 |
| Direction | 1 byte | enum |
| Stats | ~40 bytes | 多个u16/u32 |
| Equipment | ~100 bytes | 10个Option |
| Inventory | ~4KB | 40×100字节 |

---

## 🔗 相关文档

- **ECS系统**: `../systems/README.md` - 系统如何使用组件
- **对象系统**: `../../objects/README.md` - 组件与对象的对应关系

---

**文档版本**: v1.0  
**最后更新**: 2025-10-28  
**维护者**: Crystal Mir2 Team

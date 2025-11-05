# systems/ 目录说明

存放游戏逻辑系统（Systems）。系统负责处理组件数据并驱动游戏行为。

## 系统分层架构

本项目采用分层系统设计，按优先级（Priority）从低到高执行：

### 第 0 层：基础设施 (0-99)
- 资源预加载、场景管理、保存系统

### 第 1 层：输入与网络 (100-199)
- `InputSystem` - 输入处理
- `NetworkSystem` - 网络事件接收与分发
- `PlayerControlSystem` - 玩家控制输入转换

### 第 2 层：游戏逻辑 (200-599)

#### 决策系统 (200-299)
- `MonsterAISystem` - 怪物 AI 行为
- `NpcAISystem` - NPC AI 行为
- `NpcDialogueSystem` - NPC 对话逻辑

#### 战斗系统 (300-399)
- `CombatSystem` - 战斗逻辑与伤害计算
- `SkillSystem` - 技能释放与效果
- `HealthRegenSystem` - 生命回复

#### 物理与移动 (500-599)
- `PathfindingSystem` - 寻路计算
- `MovementSystem` - 实体位移（Position += Velocity * dt）
- `CollisionSystem` - 碰撞检测与位置修正
- `MapUpdateSystem` - 地图更新（瓦片动画、地图切换）

### 第 3 层：表现层 (600-899)
- `AnimationSystem` - 动画状态更新
- `ParticleSystem` - 粒子效果
- `CameraFollowSystem` - 相机跟随
- `CameraSystem` - 相机特效（震动等）

### 第 4 层：渲染层 (900-1999)
- `MapRenderSystem` - 地图渲染
- `EntityRenderSystem` - 实体渲染（玩家/怪物）
- `EffectRenderSystem` - 特效渲染
- `UIRenderSystem` - UI 渲染

### 第 5 层：调试工具 (9000+)
- `DebugSystem` - 调试信息显示
- `ProfileSystem` - 性能分析

## 系统类型（重要）

项目定义了三种系统类型：

1. **System** - 纯逻辑系统
   - 实现 `update()` 方法
   - 用于 AI、物理、战斗等逻辑处理

2. **DrawSystem** - 纯渲染系统
   - 实现 `draw()` 方法
   - 用于地图、UI 渲染

3. **HybridSystem** - 混合系统
   - 同时实现 `update()` 和 `draw()`
   - 用于粒子、调试等需要逻辑+渲染的系统

## 关键系统依赖组件速查

### MovementSystem (优先级 500)
**依赖组件**：
- 读取：`Velocity`, `Position`
- 写入：`Position`

**职责**：纯物理移动，应用速度到位置

### CollisionSystem (优先级 510)
**依赖组件**：
- 读取：`Position`, `Collider`, `MapData`
- 写入：`Position` (碰撞修正)

**职责**：碰撞检测与位置修正

### CombatSystem (优先级 300)
**依赖组件**：
- 读取：`Position`, `CombatStats`, `Target`
- 写入：`Health`, `CombatState`

**职责**：战斗逻辑、伤害计算、状态更新

### MapUpdateSystem (优先级 500)
**依赖组件**：
- 读取：`MapData`, `AnimatedTile`
- 写入：`AnimatedTile` (帧索引)

**职责**：地图瓦片动画更新、地图切换处理

### EntityRenderSystem (优先级 920)
**依赖组件**：
- 读取：`Position`, `Sprite`, `Camera`, `Player`, `Monster`
- 写入：无（纯渲染）

**职责**：渲染玩家和怪物实体

## 约定与最佳实践

1. **组件依赖声明**：每个系统应在文档或模块注释中明确依赖的组件列表。
2. **优先级常量**：使用 `systems::priority` 模块中的常量，不要硬编码数字。
3. **线程安全**：并行系统应在文档中标注线程安全注意点。
4. **单元测试**：复杂系统应编写单元测试（Mock World + 必要组件）。

## 使用示例

```rust
use crate::ecs::systems::{System, priority};

struct MyLogicSystem;

impl System for MyLogicSystem {
    fn priority(&self) -> u32 { 
        priority::MOVEMENT 
    }

    fn update(&mut self, world: &mut hecs::World, dt: f32) -> GameResult {
        // 处理逻辑
        for (id, (pos, vel)) in world.query_mut::<(&mut Position, &Velocity)>() {
            pos.x += vel.x * dt;
            pos.y += vel.y * dt;
        }
        Ok(())
    }
}
```

## 查看更多

- 完整的系统架构说明：`systems/mod.rs` 顶部注释
- 各子系统详细文档：查看对应目录（`logic/`, `rendering/`, `input/` 等）
# ECS Systems 架构文档 v4.0

**版本**: v4.0 (两类系统架构)  
**最后更新**: 2025-11-05  
**状态**: ✅ 架构重构完成

---

## 📚 快速导航

- [架构概览](#-架构概览)
- [系统清单](#-系统清单)
- [数据流](#-数据流)
- [使用指南](#-使用指南)
- [架构审查报告](./ARCHITECTURE_REVIEW.md)

---

## 🏗️ 架构概览

### 目录结构

```
systems/
├── mod.rs                      # 主模块：系统类型定义 (LogicSystem/RenderSystem)
├── README.md                   # 本文档
│
├── logic/                      # 纯逻辑系统 (只有 update，优先级 50-599)
│   ├── input/                  # Layer 1: 输入层 (50-199)
│   │   └── player_control_system.rs    - 玩家控制
│   │
│   ├── decision/               # Layer 2: 决策层 (200-299)
│   │   ├── monster_ai_system.rs        - 怪物AI
│   │   ├── npc_ai_system.rs            - NPC AI
│   │   └── npc_dialogue_system.rs      - NPC对话
│   │
│   ├── combat_skill/           # Layer 3: 战斗技能层 (300-399)
│   │   ├── skill_system.rs             - 技能系统
│   │   └── combat_system.rs            - 战斗系统
│   │
│   ├── physics/                # Layer 4: 物理移动层 (400-499)
│   │   ├── movement_system.rs          - 移动系统
│   │   ├── collision_system.rs         - 碰撞检测
│   │   ├── pathfinding_system.rs       - 寻路系统
│   │   └── camera_follow_system.rs     - 相机跟随
│   │
│   └── update/                 # Layer 5: 状态更新层 (500-599)
│       ├── particle_system.rs          - 粒子特效
│       ├── health_regen_system.rs      - 生命恢复
│       ├── sound_system.rs             - 音效系统
│       └── camera_system.rs            - 相机系统
│
└── render/                     # 渲染系统 (有 update + draw，优先级 1000-1999)
    ├── map_system.rs           - 地图渲染 (混合系统: 瓦片动画更新 + 地图绘制)
    └── entity_render_system.rs - 实体渲染 (玩家/怪物/NPC)
```

**统计数据**:
- **逻辑系统** (LogicSystem): 11 个 - 只实现 `update()` 方法
- **渲染系统** (RenderSystem): 2 个 - 实现 `update()` + `draw()` 方法
- **总系统数**: 13 个系统
- **事件清理**: 由 GameContext 在帧结束时自动清理

---

## 🎯 设计原则

### 两类系统架构

```
┌──────────────────────────────────────────────────────┐
│ LogicSystem (纯逻辑系统) - logic/                     │
│ - 只实现 update(&mut GameContext, dt) -> GameResult  │
│ - 用于: AI、物理、战斗等游戏逻辑处理                   │
│ - 示例: MovementSystem, CombatSystem, AISystem       │
│ - 优先级: 50-599                                      │
└──────────────────────────────────────────────────────┘
              ↓ 数据流 (组件读写)
┌──────────────────────────────────────────────────────┐
│ RenderSystem (渲染系统) - render/                     │
│ - 实现 update() + draw() 两个方法                     │
│ - update(): 更新渲染相关状态(如动画帧、视锥裁剪)      │
│ - draw(): 绘制到 Canvas                               │
│ - 示例: MapRenderSystem (瓦片动画 + 地图渲染)        │
│ - 优先级: 1000-1999                                   │
└──────────────────────────────────────────────────────┘
```

**关键设计变化**:
- ❌ **移除** HybridSystem trait (过度设计)
- ✅ **简化** 为两类系统: LogicSystem / RenderSystem
- ✅ **RenderSystem** 可以同时有 update() 和 draw() 方法
- ✅ **update() 可选**: 如果不需要逻辑更新,提供空实现即可

### 核心设计原则

1. ✅ **职责分离**: logic/ 处理游戏逻辑，render/ 负责渲染
2. ✅ **单向数据流**: Layer 1 → Layer 2 → ... → Layer 5 → Render
3. ✅ **组件驱动**: 系统通过读写组件通信，不直接调用
4. ✅ **最小化系统状态**: 优先使用组件存储状态，系统状态仅用于缓存
5. ✅ **优先级排序**: 通过注册顺序和优先级参数控制执行顺序

---

## 📊 系统清单

### Logic Systems (优先级 50-900)

#### Layer 1: Input & Network (50-199)

| 系统 | 优先级 | 职责 | 状态 |
|------|--------|------|------|
| PlayerControlSystem | 110 | 处理玩家输入、移动控制、攻击逻辑 | ✅ 就绪 |

**输入处理**:
- 输入事件由 GameContext 统一管理
- PlayerControlSystem 从 GameContext 读取键盘/鼠标事件
- 支持移动、攻击、技能释放等玩家操作

---

#### Layer 2: AI & Decision (200-299)

| 系统 | 优先级 | 职责 | 状态 |
|------|--------|------|------|
| MonsterAISystem | 200 | 怪物AI逻辑 | ✅ 就绪 |
| NpcAISystem | 210 | NPC AI逻辑 | ✅ 就绪 |
| NpcDialogueSystem | 220 | NPC对话逻辑 | ✅ 就绪 |

**设计评价**: ⭐⭐⭐⭐⭐ 职责最清晰的层

---

#### Layer 3: Combat & Skills (300-399)

| 系统 | 优先级 | 职责 | 状态 |
|------|--------|------|------|
| SkillSystem | 300 | 技能施放、冷却管理、MP消耗 | ✅ 就绪 |
| CombatSystem | 310 | 伤害计算、命中判定、暴击 | ✅ 就绪 |

**输入依赖**: Layer 1 的玩家输入, Layer 2 的AI决策  
**输出影响**: 修改 Health/Mana 组件, 发布网络命令, 触发特效

---

#### Layer 4: Physics & Movement (400-499)

| 系统 | 优先级 | 职责 | 状态 |
|------|--------|------|------|
| PathfindingSystem | 350 | A*寻路算法、路径计算 | ✅ 就绪 |
| MovementSystem | 400 | 根据路径更新实体位置、处理移动动画 | ✅ 就绪 |
| CollisionSystem | 410 | 碰撞检测与位置修正 | ✅ 就绪 |

---

#### Layer 5: State Update (500-599)

| 系统 | 优先级 | 职责 | 状态 |
|------|--------|------|------|
| ParticleSystem | 510 | 粒子生命周期管理、位置速度更新 | ✅ 就绪 |
| HealthRegenSystem | 515 | 生命值/魔法值自动恢复 | ✅ 就绪 |
| SoundSystem | 520 | 音效播放管理 | ✅ 就绪 |
| CameraSystem | 530 | 相机控制（跟随、边缘滚动、缩放） | ✅ 就绪 |

---

---

### Render Systems (优先级 1000-1999)

| 系统 | 优先级 | 职责 | 状态 |
|------|--------|------|------|
| MapRenderSystem | 1000 | **update()**: 更新瓦片动画帧索引<br>**draw()**: 渲染地图三层(地面/遮罩/前景) | ✅ 已实现 |
| EntityRenderSystem | 1020 | **update()**: 视锥裁剪、深度排序<br>**draw()**: 渲染玩家/怪物/NPC精灵 | ✅ 已实现 |

**设计说明**:
- 所有 RenderSystem 都实现 `update()` + `draw()` 方法
- `update()` 在逻辑阶段执行,用于更新渲染相关状态
- `draw()` 在渲染阶段执行,负责实际绘制
- 示例: MapRenderSystem 的 update() 更新瓦片动画,draw() 绘制地图

---

## 🔄 数据流

### GameContext 事件管理

```
用户输入 (键盘/鼠标)
    ↓
GameContext
├─ input_events       (输入事件队列)
├─ game_events        (游戏逻辑事件)
└─ network_packets    (网络数据包)
    ↓
PlayerControlSystem (读取 input_events)
AISystem (读取 game_events)
CombatSystem (处理战斗逻辑)
... (其他系统读取并处理)
    ↓
GameContext::clear_frame_events()
(每帧结束自动清理)
```

### 系统执行顺序

```
每帧循环:

Update 阶段 (所有系统的 update 方法):
  ────────── Logic Systems ──────────
  110 → PlayerControlSystem   (玩家控制)
  ────────────────────────────
  200 → MonsterAISystem       (怪物AI)
  210 → NpcAISystem           (NPC AI)
  220 → NpcDialogueSystem     (对话逻辑)
  ────────────────────────────
  300 → SkillSystem           (技能施放)
  310 → CombatSystem          (战斗计算)
  ────────────────────────────
  350 → PathfindingSystem     (寻路计算)
  400 → MovementSystem        (实体移动)
  410 → CollisionSystem       (碰撞检测)
  ────────────────────────────
  510 → ParticleSystem        (粒子更新)
  515 → HealthRegenSystem     (生命恢复)
  520 → SoundSystem           (音效播放)
  530 → CameraSystem          (相机控制)
  
  ────────── Render Systems (update) ──────────
  1000 → MapRenderSystem::update()      (瓦片动画帧更新)
  1020 → EntityRenderSystem::update()   (视锥裁剪、深度排序)

Draw 阶段 (RenderSystem 的 draw 方法):
  1000 → MapRenderSystem::draw()        (绘制地图三层)
  1020 → EntityRenderSystem::draw()     (绘制实体精灵)

⚠️ 事件清理: 由 GameContext::clear_frame_events() 在帧结束时自动执行
```

---

## 🛠️ 使用指南

### 如何添加新系统

#### 1. 创建逻辑系统

```rust
// logic/combat_skill/damage_system.rs

use crate::ecs::systems::LogicSystem;
use crate::ecs::GameContext;
use ggez::GameResult;

pub struct DamageSystem;

impl DamageSystem {
    pub fn new() -> Self {
        Self
    }
}

impl LogicSystem for DamageSystem {
    fn update(&mut self, ctx: &mut GameContext, dt: f32) -> GameResult {
        // 从 ctx.world 查询实体和组件
        // 实现伤害计算逻辑
        Ok(())
    }
}
```

#### 2. 创建渲染系统

```rust
// render/effect_system.rs

use crate::ecs::systems::RenderSystem;
use crate::ecs::GameContext;
use ggez::graphics::{Canvas, GraphicsContext};
use ggez::GameResult;

pub struct EffectRenderSystem {
    frame_counter: u32,
}

impl EffectRenderSystem {
    pub fn new() -> Self {
        Self { frame_counter: 0 }
    }
}

impl RenderSystem for EffectRenderSystem {
    // 可选的逻辑更新
    fn update(&mut self, ctx: &mut GameContext, dt: f32) -> GameResult {
        self.frame_counter += 1;
        // 更新特效动画状态
        Ok(())
    }

    // 必须实现的渲染方法
    fn draw(
        &mut self,
        gfx_ctx: &mut GraphicsContext,
        canvas: &mut Canvas,
        world: &hecs::World,
    ) -> GameResult {
        // 绘制特效
        Ok(())
    }
}
```

#### 3. 在模块中导出

```rust
// logic/combat_skill/mod.rs
pub mod damage_system;
pub use damage_system::DamageSystem;
```

#### 4. 实现 IntoSystemKind trait

使用派生宏自动实现:

```rust
// 对于逻辑系统
#[derive(LogicSystem)]
pub struct DamageSystem;

// 对于渲染系统
#[derive(RenderSystem)]
pub struct EffectRenderSystem;
```

或者使用声明宏批量实现:

```rust
// logic/mod.rs
logic_system!(
    combat_skill::DamageSystem,
    physics::CustomPhysicsSystem,
);

// render/mod.rs
render_system!(
    EffectRenderSystem,
    UIRenderSystem,
);
```

#### 5. 在 SystemScheduler 中注册

```rust
// src/bin/map_viewer/scene.rs 或 game_scene.rs

let mut scheduler = SystemScheduler::new();

// 逻辑系统 (优先级通过第二个参数指定)
scheduler.add_system(DamageSystem::new(), priority::COMBAT + 5);

// 渲染系统
scheduler.add_system(EffectRenderSystem::new(), priority::EFFECT_RENDER);
```

### 如何调试系统

#### 1. 启用日志

在系统中添加日志:

```rust
impl LogicSystem for MySystem {
    fn update(&mut self, ctx: &mut GameContext, dt: f32) -> GameResult {
        tracing::debug!("MySystem::update() 开始执行");
        // ... 逻辑
        tracing::debug!("MySystem::update() 执行完毕");
        Ok(())
    }
}
```

#### 2. 使用 GameContext 查询

```rust
impl LogicSystem for MySystem {
    fn update(&mut self, ctx: &mut GameContext, dt: f32) -> GameResult {
        // 查询实体数量
        let entity_count = ctx.world.len();
        tracing::info!("当前实体数: {}", entity_count);
        
        // 查询特定组件
        for (id, (pos, vel)) in &mut ctx.world.query::<(&Position, &Velocity)>() {
            tracing::debug!("实体 {:?}: pos={:?}, vel={:?}", id, pos, vel);
        }
        
        Ok(())
    }
}
```

---

## ✅ 架构优势

### 设计改进

1. **简化系统类型**
   - ✅ 从三类系统(System/DrawSystem/HybridSystem)简化为两类(LogicSystem/RenderSystem)
   - ✅ RenderSystem 可以同时有 update() 和 draw() 方法,无需单独的 HybridSystem
   - ✅ 减少概念复杂度,提高代码可维护性

2. **统一事件管理**
   - ✅ GameContext 统一管理所有输入/游戏/网络事件
   - ✅ 自动清理机制,防止事件污染
   - ✅ 零拷贝事件访问,提高性能

3. **优先级系统优化**
   - ✅ 通过 priority 常量明确系统执行顺序
   - ✅ 支持优先级参数传递,灵活调整顺序
   - ✅ 清晰的分层架构(输入→决策→战斗→物理→状态→渲染)

### 待优化项

1. **网络同步**
   - 当前网络事件处理分散在各个场景中
   - 建议: 统一到 GameContext 或专用网络层

2. **文档完善**
   - 部分模块缺少详细文档
   - 建议: 统一文档风格和格式

---

## 📚 相关文档

- **[systems/mod.rs](./mod.rs)**: 主模块,系统类型定义和优先级常量
- **[logic/](./logic/)**: 所有逻辑系统实现
- **[render/](./render/)**: 所有渲染系统实现

---

## 🎯 总体评价

| 维度 | 评分 | 说明 |
|------|------|------|
| **架构设计** | ⭐⭐⭐⭐⭐ (10/10) | 两类系统设计简洁清晰,易于理解 |
| **职责边界** | ⭐⭐⭐⭐⭐ (10/10) | LogicSystem 和 RenderSystem 职责明确 |
| **ECS原则** | ⭐⭐⭐⭐⭐ (10/10) | 严格遵守 ECS 设计思想 |
| **文档质量** | ⭐⭐⭐⭐☆ (8/10) | 主要文档已更新,部分细节待完善 |
| **可维护性** | ⭐⭐⭐⭐⭐ (10/10) | 宏注册优雅,优先级系统清晰 |

**最终评分**: 📊 **9.6/10** (优秀)

---

## 📝 更新日志

### v4.0 (2025-11-05) 🎉
- ✅ **重大重构**: 简化为两类系统 (LogicSystem / RenderSystem)
- ✅ 移除 HybridSystem trait (过度设计)
- ✅ RenderSystem 支持 update() + draw() 方法
- ✅ 统一事件管理到 GameContext
- ✅ 实现 MapRenderSystem 和 EntityRenderSystem
- ✅ 优化优先级系统和系统注册流程
- ✅ 更新所有文档以反映新架构

### v3.0 (2025-11-01)
- 重构为 logic/render 双模块架构
- 引入三类系统 (System/DrawSystem/HybridSystem)
- 添加 GlobalEvents 事件总线
- 已废弃 ❌

### v2.0 (2025-10-28)
- 五层架构 (layer1-5/)
- 32+ 系统
- 已废弃 ❌

---

**维护者**: ECS 架构团队  
**最后审查**: 2025-11-05  
**架构状态**: ✅ 稳定

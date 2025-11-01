# ECS Systems 架构文档 v3.0

**版本**: v3.0 (重构完成版)  
**最后更新**: 2025-01-XX  
**状态**: ✅ 架构就绪，实现进行中

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
├── mod.rs                      # 主模块：系统类型定义、Schedulable trait
├── README_v3.md                # 本文档
├── ARCHITECTURE_REVIEW.md      # 架构审查报告
│
├── logic/                      # 游戏逻辑系统 (优先级 50-900)
│   ├── input/                  # Layer 1: 输入层 (50-199)
│   │   ├── input_system.rs             - 输入收集
│   │   ├── player_control_system.rs    - 玩家控制
│   │   └── game_event_system.rs        - 事件分发
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
│   │   └── camera_follow_system.rs     - 相机跟随
│   │
│   └── update/                 # Layer 5: 状态更新层 (500-599)
    ├── animation_system.rs         - 动画更新
    ├── particle_system.rs          - 粒子特效
    ├── health_regen_system.rs      - 生命恢复
    ├── sound_system.rs             - 音效系统
    ├── camera_system.rs            - 相机系统
    └── map_update_system.rs        - 地图更新
│
└── render/                     # 渲染系统 (优先级 1000-1999)
    ├── map_system.rs           - 地图渲染
    ├── sprite_system.rs        - 精灵渲染
    ├── effect_system.rs        - 特效渲染
    ├── ui_system.rs            - UI渲染
    └── debug_system.rs         - 调试渲染（混合系统）
```

**统计数据**:
- **总系统数**: 15 个系统
- **逻辑系统**: 10 个（纯 System trait）
- **渲染系统**: 4 个（纯 DrawSystem trait）
- **混合系统**: 1 个（DebugSystem，HybridSystem trait）
- **事件清理**: 由 GameState 统一处理（非独立系统）

---

## 🎯 设计原则

### 三类系统架构

```
┌──────────────────────────────────────────────────────┐
│ System (纯逻辑系统)                                    │
│ - 只实现 update(&mut World, dt) -> GameResult        │
│ - 用于: AI、物理、战斗、网络等逻辑处理                 │
│ - 示例: MovementSystem, CombatSystem, AISystem       │
└──────────────────────────────────────────────────────┘
              ↓ 数据流 (组件读写)
┌──────────────────────────────────────────────────────┐
│ DrawSystem (纯渲染系统)                                │
│ - 只实现 draw(&mut Canvas, &World) -> GameResult     │
│ - 用于: 地图、精灵、UI等纯渲染任务                     │
│ - 示例: MapRenderSystem, SpriteRenderSystem          │
└──────────────────────────────────────────────────────┘
              ↓ 特殊需求
┌──────────────────────────────────────────────────────┐
│ HybridSystem (混合系统)                                │
│ - 同时实现 update() 和 draw()                         │
│ - 用于: 粒子系统、调试系统等需要双重逻辑的场景         │
│ - 示例: DebugSystem (收集性能数据 + 渲染)             │
│ - ⚠️ 谨慎使用，大部分系统应该是纯系统                 │
└──────────────────────────────────────────────────────┘
```

### 核心设计原则

1. ✅ **职责分离**: logic/ 处理游戏逻辑，render/ 负责渲染
2. ✅ **单向数据流**: Layer 1 → Layer 2 → ... → Layer 6 → Render
3. ✅ **组件驱动**: 系统通过读写组件通信，不直接调用
4. ✅ **无状态系统**: 系统本身不保存状态，所有状态在 World 中
5. ✅ **优先级排序**: 通过 `priority()` 明确执行顺序

---

## 📊 系统清单

### Logic Systems (优先级 50-900)

#### Layer 1: Input & Network (50-199)

| 系统 | 优先级 | 职责 | 状态 |
|------|--------|------|------|
| ~~NetworkSyncSystem~~ | ~~50~~ | ~~从网络线程接收数据包~~ | ❌ **已废弃** |
| InputSystem | 100 | 收集键盘/鼠标输入 → GlobalEvents | ✅ 就绪 |
| PlayerControlSystem | 110 | 处理玩家控制逻辑 | ✅ 就绪 |
| GameEventDispatcher | 120 | 分发游戏事件 | ✅ 就绪 |

**输出组件**: GlobalEvents（keyboard_events, mouse_events, game_events）

**网络架构说明**: 
- ❌ NetworkSyncSystem 已废弃
- 当前网络事件由 Scene 直接从 `NetContext.try_recv()` 读取
- GameScene 的网络同步需要重新设计

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
| MovementSystem | 400 | 纯物理移动: Position += Velocity * dt | ✅ 就绪 |
| CollisionSystem | 410 | 碰撞检测与位置修正 | ✅ 就绪 |
| CameraFollowSystem | 420 | 相机跟随玩家移动 | ⚠️ 与CameraSystem重叠 |

---

#### Layer 5: State Update (500-599)

| 系统 | 优先级 | 职责 | 状态 |
|------|--------|------|------|
| AnimationSystem | 500 | 动画帧更新、状态切换 | ✅ 就绪 |
| ParticleSystem | 510 | 粒子特效更新 | ✅ 就绪 |
| HealthRegenSystem | 515 | 生命值/魔法值自动恢复 | ✅ 就绪 |
| SoundSystem | 520 | 音效播放 | ✅ 就绪 |
| CameraSystem | 530 | 相机控制（边缘滚动、缩放） | ⚠️ 与CameraFollowSystem重叠 |
| MapUpdateSystem | 540 | 地图动画瓦片、光照更新 | ✅ 就绪 |

---

#### Layer 6: 事件清理

**说明**: 
- ❌ **没有独立的 EventCleanupSystem**
- ✅ 事件清理由 `GameState::update()` 在每帧结束时统一处理
- ✅ 调用 `world.global_events_mut().clear_frame_events()` 清理
- ✅ 确保所有场景/系统处理完事件后再清理，防止事件污染

---

### Render Systems (优先级 1000-1999)

| 系统 | 优先级 | 类型 | 职责 | 状态 |
|------|--------|------|------|------|
| MapRenderSystem | 1000 | DrawSystem | 渲染地图瓦片 | 🚧 空实现 |
| SpriteRenderSystem | 1100 | DrawSystem | 渲染精灵（玩家/怪物/NPC） | 🚧 空实现 |
| EffectRenderSystem | 1200 | DrawSystem | 渲染粒子特效 | 🚧 空实现 |
| UIRenderSystem | 1300 | DrawSystem | 渲染UI界面 | 🚧 空实现 |
| DebugSystem | u32::MAX-1 | HybridSystem | 渲染调试信息 | 🚧 空实现 |

**注意**: 所有渲染系统当前都是空实现（框架已就位）

---

## 🔄 数据流

### GlobalEvents 事件总线

```
用户输入/网络包
    ↓
InputSystem (Layer 1)
    ↓
GlobalEvents 组件
├─ keyboard_events    (键盘事件队列)
├─ mouse_events       (鼠标事件队列)
├─ ime_events         (IME输入事件)
├─ game_events        (游戏事件队列)
└─ network_incoming   (网络数据包队列)
    ↓
PlayerControlSystem (Layer 1)
AISystem (Layer 2)
CombatSystem (Layer 3)
... (其他系统读取并处理)
    ↓
GameState::clear_global_events()
清理所有事件队列
```

### 系统执行顺序

```
每帧循环:

Update 阶段 (logic 系统):
  50  → NetworkSyncSystem     (接收网络包)
  100 → InputSystem           (收集输入)
  110 → PlayerControlSystem   (玩家控制)
  120 → GameEventSystem       (事件分发)
  ────────────────────────────
  200 → MonsterAISystem       (怪物AI)
  210 → NpcAISystem           (NPC AI)
  220 → NpcDialogueSystem     (对话逻辑)
  ────────────────────────────
  300 → SkillSystem           (技能施放)
  310 → CombatSystem          (战斗计算)
  ────────────────────────────
  400 → MovementSystem        (物理移动)
  410 → CollisionSystem       (碰撞检测)
  420 → CameraFollowSystem    (相机跟随)
  ────────────────────────────
  500 → AnimationSystem       (动画更新)
  510 → ParticleSystem        (粒子更新)
  515 → HealthRegenSystem     (生命恢复)
  520 → SoundSystem           (音效播放)
  530 → CameraSystem          (相机控制)
  540 → MapUpdateSystem       (地图更新)

⚠️ 事件清理: 由 GameState::update() 在所有系统执行完后统一清理

Draw 阶段 (render 系统):
  1000 → MapRenderSystem      (地图渲染)
  1100 → SpriteRenderSystem   (精灵渲染)
  1200 → EffectRenderSystem   (特效渲染)
  1300 → UIRenderSystem       (UI渲染)
  MAX  → DebugSystem          (调试渲染)
```

---

## 🛠️ 使用指南

### 如何添加新系统

#### 1. 创建系统文件

选择合适的目录：
- 逻辑系统 → `logic/<layer>/`
- 渲染系统 → `render/`

```rust
// logic/combat_skill/damage_system.rs

use crate::ecs::systems::System;
use ggez::GameResult;

pub struct DamageSystem;

impl DamageSystem {
    pub fn new() -> Self {
        Self
    }
}

impl System for DamageSystem {
    fn priority(&self) -> u32 {
        315 // 在 CombatSystem 之后执行
    }

    fn update(&mut self, world: &mut hecs::World, dt: f32) -> GameResult {
        // 实现伤害计算逻辑
        Ok(())
    }
}
```

#### 2. 在模块中注册

编辑对应的 `mod.rs`:

```rust
// logic/combat_skill/mod.rs

pub mod damage_system;  // 添加模块声明
pub use damage_system::DamageSystem;  // 导出系统
```

#### 3. 使用宏批量注册

在 `logic/mod.rs` 中添加到宏调用:

```rust
logic_system!(
    // ... 现有系统
    super::combat_skill::DamageSystem,  // 添加新系统
);
```

#### 4. 在 SystemScheduler 中初始化

编辑 `game_scene.rs`:

```rust
fn create_system_scheduler() -> SystemScheduler {
    let mut scheduler = SystemScheduler::new();
    
    // ... 现有系统
    
    // Layer 3: Combat & Skills
    scheduler.add_system(SkillSystem);
    scheduler.add_system(CombatSystem);
    scheduler.add_system(DamageSystem::new());  // 添加新系统
    
    // ...
}
```

### 如何调试系统

#### 1. 启用日志

在系统中添加日志:

```rust
impl System for MySystem {
    fn update(&mut self, world: &mut World, dt: f32) -> GameResult {
        tracing::debug!("MySystem::update() 开始执行");
        // ... 逻辑
        tracing::debug!("MySystem::update() 执行完毕");
        Ok(())
    }
}
```

#### 2. 使用 GlobalEvents 日志

```rust
if let Some((_, events)) = world.query::<&GlobalEvents>().iter().next() {
    if events.enable_logging {
        tracing::info!("当前事件数: {}", events.frame_event_count);
    }
}
```

#### 3. 使用 DebugSystem

DebugSystem 可以显示:
- FPS
- 实体数量
- 碰撞框
- 网格
- 坐标轴

---

## ⚠️ 已知问题

### 🔴 严重问题

1. **NetworkSyncSystem 缺失**
   - 状态: 已禁用（依赖旧协议）
   - 影响: 网络数据包谁负责接收？
   - 解决方案: 见 [ARCHITECTURE_REVIEW.md](./ARCHITECTURE_REVIEW.md)

2. **README.md 严重过时**
   - 状态: 描述 v2.0 的五层架构（已废弃）
   - 影响: 误导新开发者
   - 解决方案: 使用本文档（README_v3.md）

### ⚠️ 中等问题

3. **GameEventSystem 职责不清**
   - 问题: 与 GlobalEvents 功能重叠
   - 建议: 明确职责或重命名为 `GameEventDispatcher`

4. **Camera 系统职责重复**
   - 问题: `CameraFollowSystem` (Layer 4) vs `CameraSystem` (Layer 5)
   - 建议: 合并或明确划分职责

### 💡 改进建议

6. **渲染系统未实现**
   - 状态: 所有 render/ 系统都是空实现
   - 优先级: P3（实现后再评估设计）

7. **模块文档不一致**
   - 问题: 部分 mod.rs 缺少详细文档
   - 建议: 参考 `logic/combat_skill/mod.rs` 的文档风格

详细分析见: [ARCHITECTURE_REVIEW.md](./ARCHITECTURE_REVIEW.md)

---

## 📚 相关文档

- **[ARCHITECTURE_REVIEW.md](./ARCHITECTURE_REVIEW.md)**: 完整的架构审查报告
- **[systems/mod.rs](./mod.rs)**: 主模块，系统类型定义
- **[logic/combat_skill/mod.rs](./logic/combat_skill/mod.rs)**: 最佳文档示例

---

## 🎯 总体评价

| 维度 | 评分 | 说明 |
|------|------|------|
| **架构设计** | ⭐⭐⭐⭐☆ (8/10) | logic/render 分离清晰，子层划分合理 |
| **职责边界** | ⭐⭐⭐☆☆ (6/10) | 大部分清晰，但事件系统设计混乱 |
| **ECS原则** | ⭐⭐⭐⭐⭐ (10/10) | 严格遵守 ECS 设计思想 |
| **文档质量** | ⭐⭐☆☆☆ (4/10) | 旧文档过时，新文档正在建设中 |
| **可维护性** | ⭐⭐⭐⭐☆ (8/10) | 宏注册优雅，但缺少部分文档 |

**最终评分**: 📊 **7.2/10** (良好，有改进空间)

---

## 📝 更新日志

### v3.0 (2025-11-01)
- ✅ 重构为 logic/render 双模块架构
- ✅ 引入三类系统 (System/DrawSystem/HybridSystem)
- ✅ 添加 GlobalEvents 事件总线
- ✅ 事件清理由 GameState 统一处理
- ✅ 添加宏注册机制
- ✅ 完成架构审查和文档重写

### v2.0 (2025-10-28)
- 五层架构 (layer1-5/)
- 32+ 系统
- 9,243 行代码
- 已废弃 ❌

---

**维护者**: ECS 架构团队  
**最后审查**: 2025-11-01  
**下次审查**: 实现渲染系统后

---

## 🗑️ 已删除的过时文档

以下早期设计文档已删除（包含错误的 EventCleanupSystem 信息）：
- ❌ `docs/EVENT_SYSTEM.md`
- ❌ `docs/NETWORK_EVENT_ARCHITECTURE.md`
- ❌ `docs/GLOBAL_EVENTS_REFACTOR_SUMMARY.md`

最新的事件系统说明请参考：[docs/事件清理机制说明.md](../../../docs/事件清理机制说明.md)

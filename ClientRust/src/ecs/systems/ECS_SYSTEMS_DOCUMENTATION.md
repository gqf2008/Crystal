# ECS系统架构完整文档
**创建日期**: 2025-10-28  
**版本**: v2.0 (五层架构重构版)  
**目的**: 指导下一步迭代开发工作

---

## 📚 目录

1. [架构概览](#架构概览)
2. [五层架构详解](#五层架构详解)
3. [系统清单](#系统清单)
4. [数据流与依赖关系](#数据流与依赖关系)
5. [关键设计模式](#关键设计模式)
6. [性能优化指南](#性能优化指南)
7. [下一步迭代计划](#下一步迭代计划)
8. [常见问题](#常见问题)

---

## 📐 架构概览

### 五层架构设计

本项目采用**严格分层的ECS架构**，数据单向流动，职责清晰分离：

```
┌─────────────────────────────────────────────────────────────┐
│ Layer 5: UI层                                                │
│ - UI事件处理、对话框管理、物品/任务/交易系统                  │
│ - 不负责UI渲染（渲染由Layer 4完成）                           │
└─────────────────────────────────────────────────────────────┘
                              ↑
┌─────────────────────────────────────────────────────────────┐
│ Layer 4: 渲染层                                              │
│ - 纯渲染逻辑、相机变换、Y-sorting、遮挡透明度、音效播放       │
│ - 只读组件，不修改游戏逻辑状态                                │
└─────────────────────────────────────────────────────────────┘
                              ↑
┌─────────────────────────────────────────────────────────────┐
│ Layer 3: 表现状态层                                           │
│ - 动画状态决策、音效触发决策、怪物动画决策                    │
│ - 根据游戏逻辑状态决定表现效果                                │
└─────────────────────────────────────────────────────────────┘
                              ↑
┌─────────────────────────────────────────────────────────────┐
│ Layer 2: 核心逻辑层                                           │
│ - 客户端预测、物理移动、服务器校正、平滑插值                  │
│ - 游戏核心规则（战斗、魔法、怪物AI、NPC交互）                 │
└─────────────────────────────────────────────────────────────┘
                              ↑
┌─────────────────────────────────────────────────────────────┐
│ Layer 1: 输入与网络层                                         │
│ - 捕获原始输入（鼠标/键盘）、接收网络数据包、转换为游戏命令   │
└─────────────────────────────────────────────────────────────┘
```

### 核心设计原则

1. **单向数据流**: Layer 1 → Layer 2 → Layer 3 → Layer 4 → Layer 5
2. **职责分离**: 每层只负责特定功能，不越界
3. **组件驱动**: 系统通过读写组件通信，不直接调用
4. **无状态系统**: 系统本身不保存状态，所有状态存储在组件中
5. **可测试性**: 每层可独立测试，易于单元测试

---

## 🔍 五层架构详解

### Layer 1: 输入与网络层
**文件位置**: `src/ecs/systems/layer1_input/`

#### 职责
- 捕获原始输入（鼠标、键盘、触摸）
- 接收网络数据包
- 转换为游戏命令
- 双击/长按检测

#### 系统列表

| 系统 | 文件 | 行数 | 职责 |
|------|------|------|------|
| **InputCollectingSystem** | `input_collecting_system.rs` | 205 | 输入收集、双击检测、写入PlayerInputComponent |
| **ClientNetworkSystem** | `client_network_system.rs` | 263 | 接收网络包、写入ServerStateComponent |

#### 输出组件
- `PlayerInputComponent`: 玩家输入意图（移动目标、按键）
- `ServerStateComponent`: 服务器权威状态（位置校正、服务器事件）

#### 代码示例
```rust
// InputCollectingSystem 核心逻辑
impl InputCollectingSystem {
    pub fn process_mouse_down(world: &mut World, button: MouseButton, x: f32, y: f32) {
        // 1. 更新鼠标状态
        // 2. 检测双击
        // 3. 写入 PlayerInputComponent
    }
}
```

---

### Layer 2: 核心逻辑层
**文件位置**: `src/ecs/systems/layer2_logic/`

#### 职责
- **客户端预测**: 零延迟响应玩家输入
- **物理移动**: 纯物理运动，应用速度到位置
- **服务器校正**: 比较预测与服务器状态，校正误差
- **平滑插值**: 对其他玩家/怪物应用平滑移动
- **游戏核心规则**: 战斗系统、魔法系统、怪物AI、NPC交互

#### 系统列表

| 系统 | 文件 | 行数 | 职责 |
|------|------|------|------|
| **LocalPredictionSystem** | `local_prediction_system.rs` | 125 | 客户端预测移动，调用寻路算法 |
| **MovementSystemV2** | `movement_system.rs` | 64 | 纯物理运动，应用速度到位置 |
| **ReconciliationSystem** | `reconciliation_system.rs` | 122 | 服务器校正，修正预测误差 |
| **InterpolationSystem** | `interpolation_system.rs` | 79 | 其他实体平滑插值移动 |
| **MonsterSystem** | `monster_system.rs` | 326 | 怪物AI、攻击逻辑、死亡处理 |
| **NPCSystem** | `npc_system.rs` | 158 | NPC对话、任务触发、商店交互 |
| **CombatSystem** | `combat_system.rs` | 350 | 战斗计算、伤害系统、技能效果 |
| **MagicCastSystem** | `magic_cast_system.rs` | 421 | 魔法施放、MP消耗、冷却管理 |

#### 核心工作流：客户端预测 + 服务器校正

```
┌────────────────┐
│ 玩家点击地面    │
└────────┬───────┘
         │
         ↓
┌────────────────────────────┐
│ LocalPredictionSystem       │
│ 1. 读取 PlayerInputComponent│
│ 2. 调用寻路算法              │
│ 3. 立即写入 Velocity        │
│ 4. 记录 PredictionComponent │
└────────┬───────────────────┘
         │
         ↓
┌────────────────────────────┐
│ MovementSystemV2            │
│ 应用速度 → 更新 Position    │
└────────┬───────────────────┘
         │
         ↓
┌────────────────────────────┐
│ ClientNetworkSystem         │
│ 发送移动命令到服务器         │
└────────┬───────────────────┘
         │
         ↓ (100ms 网络延迟)
┌────────────────────────────┐
│ 服务器返回权威位置           │
└────────┬───────────────────┘
         │
         ↓
┌────────────────────────────┐
│ ReconciliationSystem        │
│ 1. 比较预测 vs 服务器状态   │
│ 2. 如果误差 > 阈值，平滑校正│
└────────────────────────────┘
```

#### 关键组件
- **输入**: `PlayerInputComponent`, `ServerStateComponent`
- **输出**: `VelocityComponent`, `PathComponent`, `MovementStateComponent`, `PredictionComponent`

---

### Layer 3: 表现状态层
**文件位置**: `src/ecs/systems/layer3_presentation/`

#### 职责
- **动画状态决策**: 根据移动状态决定播放什么动画（Idle/Walk/Run/Attack）
- **音效触发决策**: 根据游戏事件决定播放什么音效
- **怪物动画决策**: 根据怪物AI状态决定动画
- **NPC动作决策**: 根据对话状态决定NPC动画
- **粒子特效创建**: (未来扩展)

#### 系统列表

| 系统 | 文件 | 行数 | 职责 |
|------|------|------|------|
| **AnimationStateSystem** | `animation_state_system.rs` | 166 | 玩家动画状态决策 |
| **MonsterAnimationStateSystem** | `monster_animation_state_system.rs` | 68 | 怪物动画状态决策 |
| **NPCActionSystem** | `npc_action_system.rs` | 90 | NPC动作状态决策 |
| **SoundTriggerSystem** | `sound_trigger_system.rs` | 154 | 音效触发决策 |

#### 关键设计
- **只决策，不执行**: Layer 3 只决定"应该播放什么"，不实际播放
- **状态机模式**: 使用状态机管理动画切换逻辑
- **事件驱动**: 读取 GameEvent，触发对应的音效/特效

#### 关键组件
- **输入**: `MovementStateComponent`, `Player`, `GameEvent`, `AIAction`, `Velocity`
- **输出**: `AnimationStateComponent`, `SoundTriggerComponent`, `Animation`

---

### Layer 4: 渲染层
**文件位置**: `src/ecs/systems/layer4_rendering/`

#### 职责
- **纯渲染逻辑**: 从组件读取数据，绘制到屏幕
- **Y-sorting**: 深度排序，确保正确遮挡关系
- **相机变换**: 世界坐标 → 屏幕坐标
- **遮挡透明度**: 玩家前方物体半透明
- **音效播放**: 读取Layer 3的音效触发决策，实际播放
- **HUD渲染**: 生命值、魔法值、经验条、小地图
- **UI渲染**: 对话框、背包、技能栏（数据来自Layer 5）

#### 系统列表

| 系统 | 文件 | 行数 | 职责 |
|------|------|------|------|
| **RenderSystem** | `render_system/mod.rs` | 524 | 主渲染系统，Y-sorting，地图/角色渲染 |
| └─ `tiles.rs` | | 396 | 地图瓦片渲染（Back/Middle/Front三层） |
| └─ `player.rs` | | 684 | 玩家角色渲染、装备显示 |
| └─ `monster.rs` | | 424 | 怪物渲染、名字/血条显示 |
| └─ `npc.rs` | | 299 | NPC渲染、对话图标显示 |
| └─ `item.rs` | | 67 | 地面物品渲染 |
| └─ `debug.rs` | | 323 | 调试信息渲染（网格、坐标、碰撞框） |
| └─ `ui.rs` | | 211 | UI渲染（对话框、背包等） |
| **CameraSystem** | `camera_system.rs` | 133 | 相机边缘滚动、跟随玩家、平滑移动 |
| **OcclusionSystem** | `occlusion_system.rs` | 134 | 计算遮挡透明度（玩家前方物体半透明） |
| **AnimationPlaybackSystem** | `animation_playback_system.rs` | 40 | 动画帧播放（读取Layer 3的AnimationState） |
| **TileAnimationSystem** | `tile_animation_system.rs` | 53 | 地图动画瓦片更新 |
| **MovementInterpolationSystem** | `movement_interpolation_system.rs` | 101 | 渲染插值（平滑移动显示） |
| **SoundPlaybackSystem** | `sound_playback_system.rs` | 243 | 音效播放（读取Layer 3的SoundTrigger） |
| **HUDRenderSystem** | `hud_render_system.rs` | 345 | HUD渲染（血条、MP条、经验条、小地图） |
| **UIRenderSystem** | `ui_render_system.rs` | 186 | UI渲染（对话框UI渲染，数据来自Layer 5） |

#### 渲染流程

```
RenderSystem::draw_game_world()
│
├─ 1. 渲染地面层 (Back + Middle)
│  └─ draw_tiles() [TileAnimationSystem 更新动画瓦片]
│
├─ 2. 渲染实体层 (玩家、怪物、NPC、物品)
│  ├─ 收集所有实体
│  ├─ Y-sorting（按Y坐标排序）
│  ├─ draw_player() [读取 AnimationStateComponent]
│  ├─ draw_monster() [读取 Animation]
│  ├─ draw_npc()
│  └─ draw_item()
│
├─ 3. 渲染前景层 (Front tiles)
│  └─ OcclusionSystem 计算透明度
│
├─ 4. 渲染 HUD
│  └─ HUDRenderSystem::draw()
│
└─ 5. 渲染 UI
   └─ UIRenderSystem::draw() [读取 Layer 5 的对话框数据]
```

#### 关键设计
- **只读组件**: Layer 4 不修改游戏逻辑状态，只读取渲染
- **Y-sorting**: 确保正确的深度排序
- **分批渲染**: Back/Middle → 实体 → Front，优化性能

---

### Layer 5: UI层
**文件位置**: `src/ecs/systems/layer5_ui/`

#### 职责
- **UI事件处理**: 按钮点击、输入框、对话框交互
- **UI数据更新**: 背包、任务列表、交易界面
- **对话框管理**: 打开/关闭对话框、层级管理
- **键盘快捷键**: F1-F12快捷键处理
- **鼠标事件**: 鼠标悬停、拖拽、右键菜单

#### 系统列表

| 系统 | 文件 | 行数 | 职责 |
|------|------|------|------|
| **DialogManagerSystem** | `dialog_manager_system.rs` | 303 | 对话框管理（打开/关闭/层级） |
| **UIEventDispatcher** | `ui_event_dispatcher.rs` | 183 | UI事件分发（点击/悬停/输入） |
| **KeyboardShortcutSystem** | `keyboard_shortcut_system.rs` | 205 | 键盘快捷键处理（F1-F12） |
| **MouseEventSystem** | `mouse_event_system.rs` | 212 | 鼠标事件处理（点击/拖拽） |
| **ItemSystem** | `item_system.rs` | 326 | 背包系统、装备穿戴、物品使用 |
| **QuestSystem** | `quest_system.rs` | 430 | 任务系统、任务进度追踪 |
| **TradeSystem** | `trade_system.rs` | 385 | 交易系统、商店系统 |
| **MagicLearningSystem** | `magic_learning_system.rs` | 164 | 技能学习、技能升级 |
| **UISystem** | `ui_system.rs` | 68 | 向后兼容入口（实际功能已拆分） |

#### 重要说明
- **不负责UI渲染**: UI渲染由 Layer 4 的 `UIRenderSystem` 完成
- **事件驱动**: 系统响应用户输入，更新UI数据
- **对话框架构**: 使用 Dialog 组件存储UI状态，Render System 读取并渲染

#### UI系统重构历史
```
旧架构 (已废弃):
  UISystem (470行) - 包含所有UI逻辑

新架构 (当前):
  ├─ DialogManagerSystem (303行) - 对话框管理
  ├─ UIEventDispatcher (183行) - 事件分发
  ├─ KeyboardShortcutSystem (205行) - 快捷键
  └─ MouseEventSystem (212行) - 鼠标事件
  
重构完成时间: 2025-10-28
```

---

## 📊 系统清单

### 系统总览

| 层级 | 系统数量 | 总行数 | 职责概述 |
|------|---------|--------|---------|
| **Layer 1** | 2 | 468 | 输入与网络 |
| **Layer 2** | 8 | 1,645 | 核心逻辑 |
| **Layer 3** | 4 | 510 | 表现决策 |
| **Layer 4** | 9 + 子模块 | 4,144 | 渲染与音效 |
| **Layer 5** | 9 | 2,476 | UI逻辑 |
| **总计** | **32+** | **9,243** | 完整游戏系统 |

### 完整系统列表（按执行顺序）

#### 游戏主循环执行顺序

```rust
// 1️⃣ Layer 1: 输入与网络层
InputCollectingSystem::update();        // 捕获输入
ClientNetworkSystem::update();          // 接收网络包

// 2️⃣ Layer 2: 核心逻辑层
LocalPredictionSystem::update();        // 客户端预测
MovementSystemV2::update();             // 物理移动
ReconciliationSystem::update();         // 服务器校正
InterpolationSystem::update();          // 平滑插值
MonsterSystem::update();                // 怪物AI
NPCSystem::update();                    // NPC交互
CombatSystem::update();                 // 战斗系统
MagicCastSystem::update();              // 魔法系统

// 3️⃣ Layer 3: 表现状态层
AnimationStateSystem::update();         // 玩家动画决策
MonsterAnimationStateSystem::update();  // 怪物动画决策
NPCActionSystem::update();              // NPC动作决策
SoundTriggerSystem::update();           // 音效触发决策

// 4️⃣ Layer 4: 渲染层
CameraSystem::update();                 // 相机更新
OcclusionSystem::update();              // 遮挡计算
TileAnimationSystem::update();          // 地图动画
AnimationPlaybackSystem::update();      // 动画播放
MovementInterpolationSystem::update();  // 渲染插值
RenderSystem::draw_game_world();        // 渲染游戏世界
HUDRenderSystem::draw();                // 渲染HUD
UIRenderSystem::draw();                 // 渲染UI
SoundPlaybackSystem::update();          // 音效播放

// 5️⃣ Layer 5: UI层
KeyboardShortcutSystem::update();       // 快捷键处理
MouseEventSystem::update();             // 鼠标事件
DialogManagerSystem::update();          // 对话框管理
UIEventDispatcher::update();            // UI事件分发
ItemSystem::update();                   // 背包系统
QuestSystem::update();                  // 任务系统
TradeSystem::update();                  // 交易系统
MagicLearningSystem::update();          // 技能学习
```

---

## 🔄 数据流与依赖关系

### 核心组件依赖图

```
PlayerInputComponent (Layer 1 写入)
         │
         ↓
LocalPredictionSystem (Layer 2 读取)
         │
         ├─→ VelocityComponent ────┐
         ├─→ PathComponent         │
         └─→ PredictionComponent   │
                                   │
ServerStateComponent (Layer 1 写入)│
         │                         │
         ↓                         │
ReconciliationSystem (Layer 2 读取)│
         │                         │
         └─→ 校正 VelocityComponent│
                                   │
                                   ↓
MovementSystemV2 (Layer 2 读取 Velocity)
         │
         └─→ Position (更新位置)
                  │
                  ↓
AnimationStateSystem (Layer 3 读取 Position/Velocity)
         │
         └─→ AnimationStateComponent (写入动画状态)
                  │
                  ↓
RenderSystem (Layer 4 读取 AnimationStateComponent)
         │
         └─→ 屏幕渲染
```

### 组件读写权限表

| 组件 | Layer 1 | Layer 2 | Layer 3 | Layer 4 | Layer 5 |
|------|---------|---------|---------|---------|---------|
| **PlayerInputComponent** | ✍️ 写 | 📖 读 | - | - | - |
| **ServerStateComponent** | ✍️ 写 | 📖 读 | - | - | - |
| **VelocityComponent** | - | ✍️ 写 | 📖 读 | 📖 读 | - |
| **PathComponent** | - | ✍️ 写 | 📖 读 | - | - |
| **MovementStateComponent** | - | ✍️ 写 | 📖 读 | - | - |
| **PredictionComponent** | - | ✍️ 写 | - | - | - |
| **AnimationStateComponent** | - | - | ✍️ 写 | 📖 读 | - |
| **SoundTriggerComponent** | - | - | ✍️ 写 | 📖 读 | - |
| **Position** | - | ✍️ 写 | 📖 读 | 📖 读 | 📖 读 |
| **Camera** | - | - | - | ✍️ 写 | - |
| **Dialog** | - | - | - | 📖 读 | ✍️ 写 |

**注意**: 严格遵守读写权限，避免跨层写入导致的数据竞争！

---

## 🎨 关键设计模式

### 1. 客户端预测 + 服务器校正模式

**问题**: 网络延迟导致操作不流畅  
**解决方案**: 客户端立即响应，服务器事后校正

```rust
// 客户端预测（LocalPredictionSystem）
pub fn update(world: &mut World, map_data: &MapData, _dt: f32) {
    // 1. 读取玩家输入
    let input = world.get::<PlayerInputComponent>(player_entity);
    
    // 2. 立即计算路径并移动（不等服务器）
    let path = Pathfinding::find_path(...);
    velocity.set(path.next_velocity());
    
    // 3. 记录预测状态
    prediction.record(position, velocity, sequence_number);
}

// 服务器校正（ReconciliationSystem）
pub fn update(world: &mut World, _dt: f32) {
    // 1. 读取服务器权威位置
    let server_state = world.get::<ServerStateComponent>(player_entity);
    
    // 2. 比较预测 vs 服务器
    let error = server_state.position - prediction.position;
    
    // 3. 如果误差过大，平滑校正
    if error.length() > THRESHOLD {
        position.smooth_correct(server_state.position, LERP_FACTOR);
    }
}
```

### 2. 状态机模式（动画状态管理）

**问题**: 复杂的动画切换逻辑难以维护  
**解决方案**: 使用状态机管理动画转换

```rust
pub enum AnimationState {
    Idle,
    Walk,
    Run,
    Attack,
    Spell,
    Die,
}

impl AnimationStateSystem {
    pub fn update(world: &mut World, _dt: f32) {
        for (movement_state, mut animation_state) in world.query_mut::<...>() {
            let desired_state = match movement_state.state {
                MovementState::Idle => AnimationState::Idle,
                MovementState::Walking => AnimationState::Walk,
                MovementState::Running => AnimationState::Run,
            };
            
            // 状态切换逻辑
            if animation_state.current != desired_state {
                animation_state.transition_to(desired_state);
            }
        }
    }
}
```

### 3. 事件驱动模式（音效/UI）

**问题**: 直接调用导致耦合严重  
**解决方案**: 使用事件队列解耦

```rust
// Layer 3: 触发音效事件
SoundTriggerSystem::update(world, events) {
    for event in events {
        match event {
            GameEvent::Attack => {
                // 写入音效触发组件
                world.insert_one(entity, SoundTrigger {
                    sound_id: "attack_sword.wav",
                    volume: 1.0,
                });
            }
        }
    }
}

// Layer 4: 播放音效
SoundPlaybackSystem::update(world, audio_engine) {
    for (entity, sound_trigger) in world.query::<&SoundTrigger>() {
        audio_engine.play(sound_trigger.sound_id, sound_trigger.volume);
        // 播放后移除触发组件
        world.remove_one::<SoundTrigger>(entity);
    }
}
```

### 4. Y-Sorting 渲染模式

**问题**: 2D游戏需要正确的深度排序  
**解决方案**: 按Y坐标排序渲染

```rust
impl RenderSystem {
    pub fn draw_game_world(...) {
        // 1. 收集所有实体
        let mut entities = Vec::new();
        for (entity, (pos, _)) in world.query::<(&Position, &Player)>() {
            entities.push((entity, pos.y));
        }
        
        // 2. Y-sorting（Y值越大越靠前）
        entities.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        
        // 3. 按顺序渲染
        for (entity, _) in entities {
            Self::draw_entity(entity, ...);
        }
    }
}
```

---

## ⚡ 性能优化指南

### 1. 系统执行频率优化

不是所有系统都需要每帧执行：

```rust
// 高频系统（每帧执行）
- InputCollectingSystem           // 60 FPS
- MovementSystemV2                // 60 FPS
- RenderSystem                    // 60 FPS

// 中频系统（每 100ms 执行）
- MonsterSystem                   // 10 FPS (怪物AI)
- NPCSystem                       // 10 FPS (NPC逻辑)
- ReconciliationSystem            // 10 FPS (服务器校正)

// 低频系统（按需执行）
- TradeSystem                     // 仅在交易时
- QuestSystem                     // 仅在任务更新时
- DialogManagerSystem             // 仅在UI事件时
```

### 2. 组件查询优化

```rust
// ❌ 坏：每次查询所有组件
for (entity, (pos, vel, anim, player, ...)) in world.query::<(
    &Position, &Velocity, &Animation, &Player, ...
)>() {
    // ...
}

// ✅ 好：只查询需要的组件
for (entity, (pos, vel)) in world.query::<(&Position, &Velocity)>() {
    // ...
}

// ✅ 更好：使用 with 过滤
for (entity, pos) in world.query::<&Position>()
    .with::<&LocalPlayer>()  // 只查询本地玩家
{
    // ...
}
```

### 3. 避免重复计算

```rust
// ❌ 坏：每次都计算
for (entity, pos) in world.query::<&Position>() {
    let screen_pos = world_to_screen(pos.x, pos.y, camera);  // 重复计算
}

// ✅ 好：缓存计算结果
let camera_transform = camera.get_transform();  // 计算一次
for (entity, pos) in world.query::<&Position>() {
    let screen_pos = camera_transform.apply(pos);  // 直接使用
}
```

### 4. 渲染批处理

```rust
// ❌ 坏：每个瓦片单独绘制
for tile in tiles {
    canvas.draw(&tile.image, tile.pos);  // 1000 次 draw call
}

// ✅ 好：批量绘制
let mut instances = Vec::new();
for tile in tiles {
    instances.push(DrawParam::new().dest(tile.pos));
}
canvas.draw_instance_array(&tile_image, instances);  // 1 次 draw call
```

### 5. 视锥剔除

```rust
// 只渲染屏幕可见的实体
let visible_rect = camera.get_visible_rect();
for (entity, pos) in world.query::<&Position>() {
    if !visible_rect.contains(pos.x, pos.y) {
        continue;  // 跳过不可见实体
    }
    // 渲染...
}
```

---

## 🚀 下一步迭代计划

### 短期目标（1-2周）

#### 1. 完善网络同步
**优先级**: 🔴 高  
**负责系统**: `ClientNetworkSystem`, `ReconciliationSystem`

- [ ] 实现完整的客户端预测与服务器校正
- [ ] 添加网络丢包处理
- [ ] 优化插值算法（Hermite插值）
- [ ] 添加网络延迟显示（Ping值）

**参考文件**:
- `src/ecs/systems/layer1_input/client_network_system.rs`
- `src/ecs/systems/layer2_logic/reconciliation_system.rs`
- `src/ecs/systems/layer2_logic/interpolation_system.rs`

#### 2. 优化怪物AI系统
**优先级**: 🟡 中  
**负责系统**: `MonsterSystem`, `MonsterAnimationStateSystem`

- [ ] 实现多种怪物AI模式（巡逻、追击、逃跑）
- [ ] 添加怪物技能系统
- [ ] 优化寻路性能（使用 A* 缓存）
- [ ] 添加怪物群体行为（组队攻击）

**参考文件**:
- `src/ecs/systems/layer2_logic/monster_system.rs`
- `src/ecs/systems/layer3_presentation/monster_animation_state_system.rs`
- `src/algorithms/pathfinding.rs`

#### 3. 完善UI系统
**优先级**: 🟡 中  
**负责系统**: `DialogManagerSystem`, `ItemSystem`, `QuestSystem`

- [ ] 实现拖拽功能（物品拖拽）
- [ ] 添加右键菜单
- [ ] 优化对话框层级管理
- [ ] 实现背包自动整理

**参考文件**:
- `src/ecs/systems/layer5_ui/dialog_manager_system.rs`
- `src/ecs/systems/layer5_ui/item_system.rs`
- `src/ecs/systems/layer5_ui/mouse_event_system.rs`

### 中期目标（3-4周）

#### 4. 技能系统重构
**优先级**: 🟡 中  
**负责系统**: `MagicCastSystem`, `CombatSystem`

- [ ] 统一技能/魔法系统架构
- [ ] 添加技能冷却可视化
- [ ] 实现技能连招系统
- [ ] 添加 Buff/Debuff 系统

**参考文件**:
- `src/ecs/systems/layer2_logic/magic_cast_system.rs`
- `src/ecs/systems/layer2_logic/combat_system.rs`

#### 5. 粒子特效系统
**优先级**: 🟢 低  
**负责系统**: 新系统 `ParticleSystem`（Layer 3）

- [ ] 设计粒子特效架构
- [ ] 实现基础粒子系统（位置、速度、生命周期）
- [ ] 添加预设特效（爆炸、火焰、闪电）
- [ ] 集成到技能系统

**建议架构**:
```
Layer 3: ParticleEmissionSystem（创建粒子发射器）
         ↓
Layer 4: ParticleRenderSystem（渲染粒子）
```

#### 6. 地图编辑器集成
**优先级**: 🟢 低  
**负责系统**: `RenderSystem`, `MapData`

- [ ] 实时地图预览
- [ ] 地图动画播放
- [ ] 碰撞编辑可视化
- [ ] 导出优化

### 长期目标（1-2月）

#### 7. 多人游戏完整支持
- [ ] 实现完整的服务器架构
- [ ] 添加房间/频道系统
- [ ] 实现玩家间交互（组队、PK、交易）
- [ ] 添加反作弊机制

#### 8. 性能优化
- [ ] 实现 ECS 并行化（使用 Rayon）
- [ ] 优化渲染管线（合并 draw call）
- [ ] 添加性能分析工具
- [ ] 优化内存使用

#### 9. 可扩展性改进
- [ ] 插件系统（热加载模块）
- [ ] 脚本系统（Lua/Rhai）
- [ ] 配置热重载
- [ ] 模组支持

---

## 🐛 常见问题

### Q1: 为什么要分五层？三层不够吗？

**A**: 三层架构（输入-逻辑-渲染）在简单游戏中够用，但复杂游戏会遇到问题：
- **Layer 2-3 分离**: 游戏逻辑（移动）与表现逻辑（动画）分离，便于独立测试
- **Layer 3-4 分离**: 决策（播放什么动画）与执行（实际渲染）分离，便于换渲染器
- **Layer 5 独立**: UI逻辑复杂，独立成层便于管理

### Q2: 客户端预测会导致不同步吗？

**A**: 不会，因为有 `ReconciliationSystem` 校正：
1. 客户端预测是临时的，给玩家即时反馈
2. 服务器返回权威状态后，校正误差
3. 使用平滑插值，玩家感知不到跳跃

**关键代码**: `src/ecs/systems/layer2_logic/reconciliation_system.rs`

### Q3: 为什么渲染系统不能修改组件？

**A**: 渲染系统只负责显示，不应影响游戏逻辑：
- **测试性**: 可以禁用渲染系统，游戏逻辑仍正常运行
- **可移植性**: 可以替换渲染器（GGEZ → Bevy），不影响逻辑
- **性能**: 渲染可以在单独线程，不阻塞逻辑

### Q4: 系统之间如何通信？

**A**: 通过组件，不直接调用：
```rust
// ❌ 坏：直接调用
AnimationSystem::play_animation(entity, "walk");

// ✅ 好：写入组件
world.insert_one(entity, AnimationStateComponent {
    state: AnimationState::Walk,
});

// Layer 4 的 AnimationPlaybackSystem 读取组件并播放
```

### Q5: 如何调试系统执行顺序？

**A**: 添加日志：
```rust
tracing::debug!("[LayerX] SystemName::update() START");
// 系统逻辑
tracing::debug!("[LayerX] SystemName::update() END");
```

查看控制台输出，确认执行顺序是否正确。

### Q6: 新增一个系统应该放在哪一层？

**决策树**:
```
是否涉及输入/网络？
├─ 是 → Layer 1
└─ 否 → 是否涉及游戏逻辑（移动/战斗/AI）？
       ├─ 是 → Layer 2
       └─ 否 → 是否涉及表现决策（动画/音效选择）？
              ├─ 是 → Layer 3
              └─ 否 → 是否涉及渲染/音效播放？
                     ├─ 是 → Layer 4
                     └─ 否 → Layer 5 (UI)
```

### Q7: 为什么有两个 MonsterSystem？

**A**: 不是两个，是不同层的系统：
- `layer2_logic/monster_system.rs`: 怪物AI、攻击逻辑（游戏规则）
- `layer3_presentation/monster_animation_state_system.rs`: 怪物动画决策（表现逻辑）

两者职责不同，不要混淆！

### Q8: 系统可以跨层读取组件吗？

**A**: 可以读取，但不能写入：
- ✅ Layer 4 可以读取 Layer 2 的 `Position`
- ❌ Layer 4 不能写入 Layer 2 的 `Position`
- 原则：**只能读取底层组件，不能写入底层组件**

---

## 📚 参考资料

### 内部文档
- `SYSTEM_CALL_ORDER.rs`: 系统调用顺序示例
- `CODE_REVIEW_REPORT_2025-10-28.md`: 最新代码审查报告
- `CLEANUP_STATUS.md`: 废弃系统清理状态

### 外部资源
- [ECS 架构设计](https://github.com/SanderMertens/ecs-faq)
- [客户端预测与服务器校正](https://www.gabrielgambetta.com/client-side-prediction-server-reconciliation.html)
- [GGEZ 渲染优化](https://ggez.rs/docs/guides/performance/)
- [Hecs ECS 文档](https://docs.rs/hecs/)

---

## 🔧 维护指南

### 添加新系统时的检查清单

- [ ] 确定系统所属层级（Layer 1-5）
- [ ] 在对应 `mod.rs` 中声明模块
- [ ] 添加 `pub use` 导出
- [ ] 在主循环中按正确顺序调用
- [ ] 添加系统文档注释（职责、输入输出组件）
- [ ] 更新本文档的系统列表
- [ ] 编写单元测试
- [ ] 更新执行顺序图

### 删除系统时的检查清单

- [ ] 确认没有其他系统依赖
- [ ] 移除模块声明
- [ ] 移除 `pub use` 导出
- [ ] 从主循环中移除调用
- [ ] 删除相关组件（如果不再使用）
- [ ] 更新本文档
- [ ] 删除相关测试

---

**文档版本**: v2.0  
**最后更新**: 2025-10-28  
**维护者**: ECS架构团队  
**联系方式**: 见项目README

---

## 🎯 总结

本项目采用**五层ECS架构**，实现了清晰的职责分离和单向数据流：

1. **Layer 1 (输入层)**: 捕获输入 → 写入 `PlayerInputComponent`
2. **Layer 2 (逻辑层)**: 读取输入 → 客户端预测 → 物理移动 → 服务器校正
3. **Layer 3 (表现层)**: 读取逻辑状态 → 决定动画/音效
4. **Layer 4 (渲染层)**: 读取表现状态 → 实际渲染/播放
5. **Layer 5 (UI层)**: 处理UI事件 → 更新UI数据

**核心原则**:
- ✅ 单向数据流（Layer 1 → 2 → 3 → 4 → 5）
- ✅ 职责分离（每层只做一件事）
- ✅ 组件驱动（系统通过组件通信）
- ✅ 可测试性（每层独立测试）

**当前状态**: 架构重构完成，32+ 系统，9,243 行代码，编译通过 ✅

**下一步**: 完善网络同步、优化怪物AI、完善UI系统

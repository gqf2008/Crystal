# ECS系统目录结构与职责清单

**日期**: 2025-10-28  
**架构**: 五层架构 (Layer 1→2→3→4→5)  
**原则**: 单向数据流，组件通信，职责单一

---

## 📁 目录树状结构

```
src/ecs/systems/
├── 📂 layer1_input/              【Layer 1: 输入与网络层】
│   ├── input_collecting_system.rs
│   ├── client_network_system.rs
│   └── mod.rs
│
├── 📂 layer2_logic/              【Layer 2: 核心逻辑层】
│   ├── local_prediction_system.rs
│   ├── movement_system_v2.rs
│   ├── reconciliation_system.rs
│   ├── interpolation_system.rs
│   ├── monster_system.rs
│   ├── npc_system.rs
│   ├── combat_system.rs
│   ├── magic_cast_system.rs
│   └── mod.rs
│
├── 📂 layer3_presentation/       【Layer 3: 表现决策层】
│   ├── animation_state_system.rs
│   ├── monster_animation_state_system.rs
│   ├── npc_action_system.rs
│   ├── sound_trigger_system.rs
│   └── mod.rs
│
├── 📂 layer4_rendering/          【Layer 4: 渲染执行层】
│   ├── render_system/
│   │   ├── mod.rs
│   │   ├── entity.rs
│   │   └── map.rs
│   ├── camera_system.rs
│   ├── occlusion_system.rs
│   ├── animation_playback_system.rs
│   ├── tile_animation_system.rs
│   ├── movement_interpolation_system.rs
│   ├── sound_playback_system.rs
│   ├── hud_render_system.rs
│   ├── ui_render_system.rs
│   └── mod.rs
│
├── 📂 layer5_ui/                 【Layer 5: UI交互层】
│   ├── ui_system.rs
│   ├── item_system.rs
│   ├── quest_system.rs
│   ├── trade_system.rs
│   ├── magic_learning_system.rs
│   └── mod.rs
│
├── 📂 deprecated/                【废弃系统】
│   ├── animation_system.rs       (→ AnimationStateSystem + AnimationPlaybackSystem)
│   ├── movement_system.rs        (→ MovementSystemV2)
│   ├── pathfinding_system.rs     (→ LocalPredictionSystem)
│   ├── input_system.rs           (→ InputCollectingSystem)
│   ├── network_system.rs         (→ ClientNetworkSystem)
│   └── mod.rs
│
├── player_system.rs              【特殊: Layer 1+2 混合】
├── monster_system.rs             【旧位置，已迁移到layer2_logic】
├── mod.rs                        【总导出模块】
├── README.md
└── SYSTEM_CALL_ORDER.rs          【系统调用顺序文档】
```

---

## 🎯 Layer 1: 输入与网络层

**职责**: 捕获原始输入和网络数据，转换为组件

| 系统 | 文件 | 职责 | 读取组件 | 写入组件 | 调用频率 |
|------|------|------|----------|----------|----------|
| **InputCollectingSystem** | `input_collecting_system.rs` | 收集键盘/鼠标输入，转换为InputComponent | - | `MouseInput`, `KeyboardInput` | 每帧 |
| **ClientNetworkSystem** | `client_network_system.rs` | 发送网络命令，接收服务器事件 | `PlayerInput` | `ServerState` | 每帧 |

### 🔑 关键设计
- **只写不读**: Layer 1只负责采集数据，不处理游戏逻辑
- **原始数据**: 保持输入的原始状态，不做解释
- **网络缓冲**: 将网络事件缓存到组件，供Layer 2处理

---

## ⚙️ Layer 2: 核心逻辑层

**职责**: 游戏规则，物理模拟，AI决策

| 系统 | 文件 | 职责 | 读取组件 | 写入组件 | 调用频率 |
|------|------|------|----------|----------|----------|
| **LocalPredictionSystem** | `local_prediction_system.rs` | 本地玩家移动预测与寻路 | `PlayerInput`, `MapData` | `Position`, `Velocity`, `Path` | 每帧 |
| **MovementSystemV2** | `movement_system_v2.rs` | 纯物理运动（所有实体） | `Velocity` | `Position` | 每帧 |
| **ReconciliationSystem** | `reconciliation_system.rs` | 服务器校正（消除预测误差） | `ServerState`, `Position` | `Position` | 每帧 |
| **InterpolationSystem** | `interpolation_system.rs` | 其他玩家/怪物位置插值 | `ServerState` | `Position` | 每帧 |
| **MonsterSystem** | `monster_system.rs` | 怪物AI逻辑 | `MonsterData`, `Position`, `Health` | `AIState`, `Position`, `Velocity` | 每帧 |
| **NPCSystem** | `npc_system.rs` | NPC行为逻辑 | `NPCData`, `Position` | `NPCState` | 每帧 |
| **CombatSystem** | `combat_system.rs` | 战斗计算（伤害、命中） | `Attack`, `Defense`, `Health` | `Health`, `CombatEvent` | 每帧 |
| **MagicCastSystem** | `magic_cast_system.rs` | 魔法施放逻辑 | `MagicList`, `Mana` | `Spell`, `Mana`, `MagicEvent` | 每帧 |

### 🔑 关键设计
- **纯逻辑**: 只更新游戏状态，不触碰渲染/动画
- **确定性**: 相同输入必定产生相同输出（网络同步保证）
- **组件读写**: 读取Layer 1写入的组件，写入逻辑结果

### 📊 数据流示例
```
PlayerInput (Layer 1) 
    → LocalPredictionSystem 读取 
    → 写入 Position, Velocity (Layer 2)
    → AnimationStateSystem 读取 (Layer 3)
```

---

## 🎬 Layer 3: 表现决策层

**职责**: 根据逻辑状态决定"播什么"，不实际播放

| 系统 | 文件 | 职责 | 读取组件 | 写入组件 | 调用频率 |
|------|------|------|----------|----------|----------|
| **AnimationStateSystem** | `animation_state_system.rs` | 决定玩家动画状态 | `Velocity`, `Player.is_moving` | `Animation.action` | 每帧 |
| **MonsterAnimationStateSystem** | `monster_animation_state_system.rs` | 决定怪物动画状态 | `AIState`, `Velocity`, `MonsterData` | `Animation.action` | 每帧 |
| **NPCActionSystem** | `npc_action_system.rs` | 决定NPC动作切换 | `NPCState`, `TimeTracker` | `Animation.action` | 每帧 |
| **SoundTriggerSystem** | `sound_trigger_system.rs` | 决定应播放的音效 | `CombatEvent`, `MagicEvent` | `SoundTrigger` | 事件驱动 |

### 🔑 关键设计
- **决策不执行**: 只决定"应该播什么"，不更新帧索引
- **状态映射**: 将逻辑状态映射为表现状态
  ```
  AIState::Chase + Velocity != 0 → Animation.action = MirAction::Walking
  ```
- **单向依赖**: 读取Layer 2的逻辑结果，写入表现决策

### ✅ 正确示例
```rust
// Layer 3: MonsterAnimationStateSystem
if ai_state.action == AIAction::Chase && velocity.length() > 0.0 {
    animation.action = MirAction::Walking;  // ✅ 只设置应该播什么
}
```

### ❌ 错误示例（Layer越界）
```rust
// ❌ Layer 3不应该做这些
animation.frame_index += 1;  // ❌ 这是Layer 4的工作（AnimationPlaybackSystem）
position.x += velocity.dx;   // ❌ 这是Layer 2的工作（MovementSystemV2）
```

---

## 🖼️ Layer 4: 渲染执行层

**职责**: 纯粹的渲染、动画播放、音效播放

| 系统 | 文件 | 职责 | 读取组件 | 写入组件 | 调用频率 |
|------|------|------|----------|----------|----------|
| **RenderSystem** | `render_system/mod.rs` | 绘制地图和实体 | `Position`, `Animation`, `Sprite` | - | draw() |
| **CameraSystem** | `camera_system.rs` | 更新相机位置（跟随玩家） | `Player`, `Position` | `Camera.position` | 每帧 |
| **OcclusionSystem** | `occlusion_system.rs` | 计算遮挡透明度 | `Position`, `MapTile` | `TileOcclusion.alpha` | 每帧 |
| **AnimationPlaybackSystem** | `animation_playback_system.rs` | 推进动画帧 | `Animation.action`, `Animation.frame_count` | `Animation.frame_index` | 每帧 |
| **TileAnimationSystem** | `tile_animation_system.rs` | 更新地图瓦片动画 | `AnimatedTile`, `TimeTracker` | `MapTile.image_index` | 每帧 |
| **MovementInterpolationSystem** | `movement_interpolation_system.rs` | 计算移动偏移量 | `Velocity`, `Animation.frame_index` | `MovementAnimation.offset_move` | 每帧 |
| **SoundPlaybackSystem** | `sound_playback_system.rs` | 实际播放音效 | `SoundTrigger` | - | 每帧 |
| **HUDRenderSystem** | `hud_render_system.rs` | 渲染HUD（血条、地图） | `Health`, `Position` | - | draw() |
| **UIRenderSystem** | `ui_render_system.rs` | 渲染UI（对话框、背包） | `UIState` | - | draw() |

### 🔑 关键设计
- **只读不改逻辑**: 读取所有组件，但只写渲染相关组件
- **帧推进**: `AnimationPlaybackSystem` 负责 `frame_index++`
- **插值计算**: `MovementInterpolationSystem` 实现平滑移动

### 📊 渲染流程
```
Layer 3: Animation.action = Walking (决策)
    ↓
Layer 4: AnimationPlaybackSystem
    → frame_index: 0 → 1 → 2 → 3 → 0 (循环)
    ↓
Layer 4: RenderSystem
    → 根据 action + frame_index 绘制对应帧
```

---

## 🖱️ Layer 5: UI交互层

**职责**: UI事件处理，用户交互响应

| 系统 | 文件 | 职责 | 读取组件 | 写入组件 | 调用频率 |
|------|------|------|----------|----------|----------|
| **UISystem** | `ui_system.rs` | UI对话框状态管理 | `MouseInput`, `UIState` | `UIState` | 事件驱动 |
| **ItemSystem** | `item_system.rs` | 物品拖拽、使用 | `MouseInput`, `Inventory` | `Inventory`, `ItemEvent` | 事件驱动 |
| **QuestSystem** | `quest_system.rs` | 任务进度跟踪、UI交互 | `QuestLog`, `KillEvent` | `QuestLog` | 事件驱动 |
| **TradeSystem** | `trade_system.rs` | 交易窗口逻辑 | `TradeWindow`, `Inventory` | `TradeWindow` | 事件驱动 |
| **MagicLearningSystem** | `magic_learning_system.rs` | 魔法学习UI | `MagicList`, `LearnableMagicList` | `MagicList` | 事件驱动 |

### 🔑 关键设计
- **事件驱动**: 不是每帧调用，而是响应用户操作
- **读取输入**: 直接读取 `MouseInput` 判断点击
- **UI状态**: 管理对话框的打开/关闭/拖拽

### ⚠️ 已知问题
- **QuestSystem混合职责**: 同时包含Layer 2（进度跟踪）和Layer 5（UI交互）
  - **建议**: 拆分为 `QuestProgressSystem` (Layer 2) + `QuestUISystem` (Layer 5)

---

## 🗑️ 废弃系统 (deprecated/)

| 系统 | 替代者 | 废弃原因 |
|------|--------|----------|
| **AnimationSystem** | `AnimationStateSystem` + `AnimationPlaybackSystem` | 混合了Layer 3和Layer 4职责 |
| **MovementSystem** | `MovementSystemV2` | 未分离预测与物理 |
| **PathfindingSystem** | `LocalPredictionSystem` | 功能被整合 |
| **InputSystem** | `InputCollectingSystem` | 命名更清晰 |
| **NetworkSystem** | `ClientNetworkSystem` | 职责更明确 |

---

## 📋 组件读写矩阵

| Layer | 读取组件 | 写入组件 | 禁止操作 |
|-------|----------|----------|----------|
| **Layer 1** | - | `MouseInput`, `KeyboardInput`, `ServerState` | 读取其他组件 |
| **Layer 2** | Layer 1组件 | `Position`, `Velocity`, `AIState`, `Health` | 设置动画、渲染 |
| **Layer 3** | Layer 2组件 | `Animation.action`, `SoundTrigger` | 更新帧索引、物理 |
| **Layer 4** | 所有组件 | `Animation.frame_index`, `Camera.position`, `offset_move` | 修改逻辑状态 |
| **Layer 5** | Layer 1+2组件 | `UIState`, `Inventory`, `QuestLog` | 直接渲染 |

---

## 🔄 系统调用顺序（每帧）

```rust
// Layer 1: 输入与网络
InputCollectingSystem::update(world, ctx);
ClientNetworkSystem::send_commands(world, network_tx);

// Layer 2: 核心逻辑
LocalPredictionSystem::update(world, map_data, dt);
MovementSystemV2::update(world, dt);
ReconciliationSystem::update(world, dt);
InterpolationSystem::update(world, dt);
MonsterSystem::update(world, dt);
CombatSystem::update(world, dt);

// Layer 3: 表现决策
AnimationStateSystem::update(world, dt);
MonsterAnimationStateSystem::update(world);
NPCActionSystem::update(world, dt);

// Layer 4: 渲染准备
TileAnimationSystem::update(world, animation_count);
AnimationPlaybackSystem::update(world, dt);
MovementInterpolationSystem::update(world);
CameraSystem::update(world);

// Layer 4: 实际绘制（在draw()中）
RenderSystem::draw_game_world(ctx, canvas, world, ...);
HUDRenderSystem::render(ctx, canvas, world);
UIRenderSystem::render(ctx, canvas, world);

// Layer 5: UI事件（事件驱动，非每帧）
UISystem::handle_click(world, mouse_pos);
ItemSystem::handle_drag(world, mouse_pos);
```

---

## 🎯 职责边界检查清单

### ✅ 如何判断系统是否在正确的Layer？

| 问题 | Layer 1 | Layer 2 | Layer 3 | Layer 4 | Layer 5 |
|------|---------|---------|---------|---------|---------|
| 是否读取输入？ | ✅ | ❌ | ❌ | ❌ | ✅ |
| 是否修改Position？ | ❌ | ✅ | ❌ | ❌ | ❌ |
| 是否设置Animation.action？ | ❌ | ❌ | ✅ | ❌ | ❌ |
| 是否更新frame_index？ | ❌ | ❌ | ❌ | ✅ | ❌ |
| 是否实际绘制？ | ❌ | ❌ | ❌ | ✅ | ❌ |
| 是否响应UI点击？ | ❌ | ❌ | ❌ | ❌ | ✅ |

### ⚠️ 常见Layer越界错误

```rust
// ❌ Layer 2做Layer 3的事
anim.action = MirAction::Walking;  // MonsterSystem中（已修复）

// ❌ Layer 2做Layer 4的事
camera.position = player.position;  // PlayerSystem中（已修复）

// ❌ Layer 3做Layer 4的事
anim.frame_index += 1;  // AnimationStateSystem中（错误）
```

---

## 📚 相关文档

- `SYSTEM_CALL_ORDER.rs` - 详细的系统调用顺序和原则
- `LAYER_CLEANUP_REPORT.md` - Layer越界清理过程记录
- `REFACTOR_AUDIT_REPORT.md` - 重构审查报告
- `README.md` - 系统模块简介

---

## 🔮 未来优化建议

1. **网络系统完善**
   - 实现 `ClientNetworkSystem::receive_updates`
   - 完善服务器状态同步

2. **QuestSystem拆分**
   - `QuestProgressSystem` (Layer 2): 追踪任务进度
   - `QuestUISystem` (Layer 5): UI交互

3. **旧系统移除**
   - 逐步禁用 `PathfindingSystem`
   - 逐步禁用 `MovementSystem`

4. **性能优化**
   - ECS查询缓存
   - 系统并行执行（未来考虑）

---

**维护者**: Crystal开发团队  
**最后更新**: 2025-10-28  
**架构状态**: ✅ 稳定，Layer边界清晰

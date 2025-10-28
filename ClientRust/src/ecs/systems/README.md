# ECS Systems - 五层架构目录结构

## 📁 目录组织

```
systems/
├── layer1_input/          # Layer 1: 输入与网络层
│   ├── input_collecting_system.rs   - 输入收集（鼠标/键盘）
│   └── client_network_system.rs     - 网络通信（发送/接收）
│
├── layer2_logic/          # Layer 2: 核心逻辑层
│   ├── local_prediction_system.rs   - 客户端预测（寻路）
│   ├── movement_system_v2.rs        - 物理移动（速度计算）
│   ├── reconciliation_system.rs     - 服务器校正（误差修正）
│   ├── interpolation_system.rs      - 平滑插值（其他玩家）
│   ├── monster_system.rs            - 怪物AI
│   ├── npc_system.rs                - NPC交互
│   ├── combat_system.rs             - 战斗逻辑
│   └── magic_cast_system.rs         - 技能施法
│
├── layer3_presentation/   # Layer 3: 表现状态层
│   ├── animation_state_system.rs    - 动画状态决策
│   └── npc_action_system.rs         - NPC动作切换决策
│
├── layer4_rendering/      # Layer 4: 渲染层
│   ├── render_system/               - 渲染系统（模块化）
│   │   ├── mod.rs                   - Y-sorting 核心
│   │   ├── player.rs                - 角色渲染
│   │   ├── monster.rs               - 怪物渲染
│   │   ├── npc.rs                   - NPC + 特效渲染
│   │   ├── tiles.rs                 - 地图渲染
│   │   ├── item.rs                  - 物品渲染
│   │   ├── ui.rs                    - UI渲染
│   │   └── debug.rs                 - 调试渲染
│   ├── camera_system.rs             - 相机系统
│   ├── occlusion_system.rs          - 遮挡透明度
│   ├── animation_playback_system.rs - 动画帧播放
│   ├── tile_animation_system.rs     - 地图瓦片动画
│   └── movement_interpolation_system.rs - 移动插值
│
├── layer5_ui/             # Layer 5: UI 层
│   ├── ui_system.rs                 - UI事件处理
│   ├── item_system.rs               - 物品系统
│   ├── quest_system.rs              - 任务系统
│   ├── trade_system.rs              - 交易系统
│   └── magic_learning_system.rs     - 技能学习
│
└── deprecated/            # 废弃系统（兼容性保留）
    ├── animation_system.rs          - 旧动画系统（已完全替代）
    ├── movement_system.rs           - 旧移动系统
    ├── pathfinding_system.rs        - 旧寻路系统
    ├── input_system.rs              - 旧输入系统
    └── network_system.rs            - 旧网络系统
```

---

## 🎯 五层架构设计原则

### Layer 1: 输入与网络层
**职责**: 原始数据采集
- 捕获鼠标/键盘输入
- 接收网络数据包
- 转换为游戏命令

**输出组件**:
- `PlayerInputComponent` - 玩家输入意图
- `ServerStateComponent` - 服务器权威状态

---

### Layer 2: 核心逻辑层
**职责**: 游戏规则执行
- 客户端预测（零延迟）
- 物理模拟（移动、碰撞）
- 服务器校正（误差修正）
- 游戏逻辑（战斗、AI）

**输入组件**:
- `PlayerInputComponent` (Layer 1 写入)
- `ServerStateComponent` (Layer 1 写入)

**输出组件**:
- `MovementStateComponent` - 移动状态
- `VelocityComponent` - 速度向量
- `PathComponent` - 路径队列

---

### Layer 3: 表现状态层
**职责**: 表现决策（不实际渲染）
- 动画状态决策
- NPC动作切换决策
- 音效触发决策（未来）
- 粒子特效创建决策（未来）

**输入组件**:
- `MovementStateComponent` (Layer 2 写入)
- `Player` (方向、武器等)

**输出组件**:
- `AnimationStateComponent` - 动画状态
- `SoundTriggerComponent` - 音效触发（未来）
- `ParticleEmitterComponent` - 粒子发射器（未来）

---

### Layer 4: 渲染层
**职责**: 纯渲染与动画播放，不含逻辑
- 从组件读取数据
- Y-sorting（深度排序）
- 绘制到屏幕
- 动画帧更新
- 地图瓦片动画
- 移动插值计算

**输入组件（只读）**:
- `Position`
- `Animation` (Layer 3 写入动作，Layer 4 更新帧)
- `AnimationStateComponent` (Layer 3 写入)
- `Camera`
- `MapData`

**输出**: 屏幕图像 + Animation.frame_index 更新

---

### Layer 5: UI 层
**职责**: UI 交互和数据管理
- UI 事件处理
- 对话框状态管理
- 不负责 UI 渲染（由 Layer 4 负责）

**输入**: 游戏事件（GameEvent）

**输出**: UI 组件数据更新

---

## 📊 数据流向

```
用户输入/网络包
    ↓
Layer 1: InputCollectingSystem, ClientNetworkSystem
    ↓ (PlayerInputComponent, ServerStateComponent)
Layer 2: LocalPredictionSystem, MovementSystemV2, ReconciliationSystem
    ↓ (MovementStateComponent, VelocityComponent)
Layer 3: AnimationStateSystem
    ↓ (AnimationStateComponent)
Layer 4: RenderSystem, CameraSystem
    ↓ (屏幕图像)
Layer 5: UISystem
    ↓ (UI 更新)
```

---

## 🔄 系统调用顺序

### 更新阶段（game_scene.rs::update）

```rust
// Layer 1: 输入与网络
InputCollectingSystem::update(world, ctx);
ClientNetworkSystem::send_commands(world, network_tx);

// Layer 2: 核心逻辑
LocalPredictionSystem::update(world, map_data, dt);
MovementSystemV2::update(world, dt);
ReconciliationSystem::update(world, dt);
InterpolationSystem::update(world, dt);

// Layer 3: 表现决策
AnimationStateSystem::update(world, dt);
NPCActionSystem::update(world, delta_ms);

// Layer 4: 渲染/播放
TileAnimationSystem::update(world, animation_count);
AnimationPlaybackSystem::update(world, delta_ms);
MovementInterpolationSystem::update(world);
```

### 渲染阶段（game_scene.rs::draw）

```rust
// Layer 4: 渲染
RenderSystem::draw_game_world(ctx, canvas, world, ...);

// Layer 5: UI（事件驱动）
UISystem::process_event(world, event);
```

---

## 🚫 废弃系统（deprecated/）

这些系统已被五层架构完全替代：

| 旧系统 | 替代方案 | 状态 |
|--------|----------|------|
| `AnimationSystem::update_tiles` | `TileAnimationSystem` (Layer 4) | ✅ 已完全替代 |
| `AnimationSystem::update_entities` | `AnimationPlaybackSystem` (Layer 4) | ✅ 已完全替代 |
| `AnimationSystem::update_movement_animation` | `MovementInterpolationSystem` (Layer 4) | ✅ 已完全替代 |
| `AnimationSystem::NPCActionSystem` | `NPCActionSystem` (Layer 3) | ✅ 已完全替代 |
| `MovementSystem` | `MovementSystemV2` (Layer 2) | ⏳ 待废弃 |
| `PathfindingSystem` | `LocalPredictionSystem` (Layer 2) | ⏳ 待废弃 |
| `InputSystem` | `InputCollectingSystem` (Layer 1) | ⏳ 待废弃 |
| `NetworkSystem` | `ClientNetworkSystem` (Layer 1) | ⏳ 待废弃 |

**废弃计划**: AnimationSystem已完全迁移，其他系统验证稳定后逐步移除。

---

## 📝 使用指南

### 导入系统

```rust
use crate::ecs::systems::{
    // Layer 1
    InputCollectingSystem, ClientNetworkSystem,
    
    // Layer 2
    LocalPredictionSystem, MovementSystemV2,
    ReconciliationSystem, InterpolationSystem,
    MonsterSystem, CombatSystem,
    
    // Layer 3
    AnimationStateSystem, NPCActionSystem,
    
    // Layer 4
    RenderSystem, CameraSystem,
    AnimationPlaybackSystem, TileAnimationSystem,
    MovementInterpolationSystem,
    
    // Layer 5
    UISystem, ItemSystem, QuestSystem,
};
```

### 添加新系统

1. 确定系统属于哪一层
2. 在对应的 `layerN_xxx/` 目录创建文件
3. 在该层的 `mod.rs` 中添加导出
4. 在主 `mod.rs` 中重新导出（如需要）
5. 在 `game_scene.rs` 的正确位置调用

---

## 🎉 优势

1. **职责清晰**: 每层只做一件事
2. **易于测试**: 层与层之间通过组件解耦
3. **可维护性**: 文件平均 150 行，远低于 500 行限制
4. **可扩展性**: 新功能容易定位到对应层级
5. **数据流清晰**: Layer 1 → 2 → 3 → 4 → 5，单向数据流

---

**日期**: 2025-10-28  
**版本**: 2.0  
**状态**: ✅ 五层架构完整实现

**变更日志**:
- ✅ AnimationSystem 完全拆分为3个Layer 4系统
- ✅ NPCActionSystem 迁移到Layer 3
- ✅ 所有系统严格按照5层架构组织
- ✅ deprecated/仅保留DoorSystem和旧移动/寻路系统

# ECS 系统使用说明

## 当前启用的 ECS 系统

根据 `src/ecs/scenes/game_scene.rs` 的代码分析，以下系统正在被使用：

### 1. **UISystem** (UI系统)
- **位置**: `src/ecs/systems/ui_system.rs`
- **用途**: 渲染所有UI组件（MainDialog、InventoryDialog、CharacterDialog等）
- **调用**: `self.ui_system.draw(ctx, canvas, world, 0)`
- **说明**: ✅ **已启用** - 所有UI确实由UISystem统一处理

### 2. **AnimationSystem** (动画系统)
- **位置**: `src/ecs/systems/animation.rs`
- **用途**: 更新所有实体的动画状态
- **调用**: `AnimationSystem::update(world, animation_count)`
- **说明**: ✅ **已启用**

### 3. **CameraSystem** (相机系统)
- **位置**: `src/ecs/systems/camera.rs`
- **用途**: 更新相机位置，跟随玩家
- **调用**: `CameraSystem::update(world)`
- **说明**: ✅ **已启用**

### 4. **PlayerSystem** (玩家系统)
- **位置**: `src/ecs/systems/player.rs`
- **用途**: 更新玩家状态、移动、动画等
- **调用**: `PlayerSystem::update(world)`
- **说明**: ✅ **已启用**

### 5. **MonsterSystem** (怪物系统)
- **位置**: `src/ecs/systems/monster.rs`
- **用途**: 更新怪物AI、移动、攻击等
- **调用**: `MonsterSystem::update(world, delta_time)`
- **说明**: ✅ **已启用**

### 6. **RenderSystem** (渲染系统)
- **位置**: `src/ecs/systems/render.rs`
- **用途**: 渲染地图瓦片、玩家、怪物、调试信息等
- **调用**: 
  - `RenderSystem::draw_tiles(...)`
  - `RenderSystem::draw_monsters(...)`
  - `RenderSystem::draw_player_with_world(...)`
  - `RenderSystem::draw_monster_info(...)`
  - `RenderSystem::draw_grid(...)` (调试)
  - `RenderSystem::draw_obstacles(...)` (调试)
  - `RenderSystem::draw_path(...)` (调试)
- **说明**: ✅ **已启用** - 负责所有游戏世界的渲染

### 7. **NetworkSystem** (网络系统)
- **位置**: `src/ecs/systems/network.rs`
- **用途**: 处理网络事件，同步服务器数据到ECS
- **调用**: 在 `handle_network_event()` 中使用
- **说明**: ✅ **已启用**

### 8. **MagicLearningSystem** (魔法学习系统)
- **位置**: `src/ecs/systems/magic_learning_system.rs`
- **用途**: 处理技能学习相关逻辑
- **调用**: `MagicLearningSystem::update_available_magics(world)`
- **说明**: ✅ **已启用**

### 9. **QuestSystem** (任务系统)
- **位置**: `src/ecs/systems/quest_system.rs`
- **用途**: 处理任务相关逻辑
- **调用**: 在初始化和事件处理中使用
- **说明**: ✅ **已启用**

## 未使用但存在的系统

以下系统存在于代码中但当前未被调用：

### ❌ NPCSystem
- **位置**: `src/ecs/systems/npc_system.rs`
- **状态**: 未在 `game_scene.rs` 中调用
- **建议**: 如需NPC交互功能，需要在update循环中添加调用

### ❌ ItemSystem
- **位置**: `src/ecs/systems/item_system.rs`
- **状态**: 未在 `game_scene.rs` 中调用
- **建议**: 如需物品掉落/拾取功能，需要添加调用

### ❌ CombatSystem
- **位置**: `src/ecs/systems/combat_system.rs`
- **状态**: 未在 `game_scene.rs` 中调用
- **建议**: 如需战斗系统，需要添加调用

### ❌ TradeSystem / ShopSystem
- **位置**: `src/ecs/systems/trade_system.rs`
- **状态**: 未在 `game_scene.rs` 中调用
- **建议**: 如需交易/商店功能，需要添加调用

### ❌ MagicCastSystem
- **位置**: `src/ecs/systems/magic_cast_system.rs`
- **状态**: 未在 `game_scene.rs` 中调用
- **建议**: 如需技能施放功能，需要添加调用

## UI 渲染流程

```
GameScene::draw()
    ↓
self.ui_system.draw(ctx, canvas, world, 0)
    ↓
UISystem::draw()
    ↓
├─ 查询所有UI组件 (MainDialogComp, InventoryDialogComp, CharacterDialogComp等)
├─ 按z-order排序
└─ 调用各对话框的 draw() 方法
    ├─ MainDialog::draw()
    ├─ InventoryDialog::draw_with_z()
    ├─ CharacterDialog::draw()
    └─ ...其他对话框
```

**结论**: UI确实是由UISystem统一处理的，不存在重复渲染的问题。

## 系统更新顺序

1. **AnimationSystem** - 更新动画帧
2. **CameraSystem** - 更新相机位置
3. **PlayerSystem** - 更新玩家状态
4. **MonsterSystem** - 更新怪物AI
5. **ChatDialog** - 更新聊天输入框（光标闪烁）

## 渲染顺序

1. **RenderSystem** - 渲染游戏世界（地图、实体）
2. **UISystem** - 渲染UI层（对话框、按钮等）

---

**生成时间**: 2025年10月24日
**文件位置**: `ClientRust/docs/ECS_SYSTEMS_USAGE.md`

# ECS 代码清理计划

## 🗑️ 需要删除的文件

### 1. 未使用的系统
- `src/ecs/systems/logic/update/animation_system.rs` (CharacterAnimationSystem)
  - 只更新 AnimationControl.current_frame，但没有任何地方读取
  - 已从 scene.rs 移除注册

### 2. 未使用的组件
- `src/ecs/components/animation_state.rs`
  - AnimationState 枚举 - 未被渲染系统使用
  - AnimationControl 结构体 - 只被已删除的 CharacterAnimationSystem 使用
  - ActionType, QueuedAction - 似乎也未使用

- `src/ecs/components/prediction.rs` ⚠️ 需要确认
  - PredictionState, ServerState, Interpolation 等
  - 没有找到任何系统使用这些组件
  - 可能是为未来的网络预测准备的

### 3. 未使用的组件字段
- `src/ecs/components/state_machine.rs`
  - `PlayerState::frame_count()` - 已删除 ✅
  - `PlayerState::frame_interval()` - 已删除 ✅

## ✅ 实际使用的组件

### 核心组件 (core.rs)
- Position, Velocity, Health, Name - 被多个系统使用

### 玩家组件 (player.rs)
- Player, PlayerAction ✅ (正在使用)
- PlayerAppearance
- LocalPlayer

### 输入组件 (input.rs)
- PlayerInput - PlayerControlSystem 使用

### 移动组件 (movement.rs)
- MovementVelocity, Path - MovementSystem, CollisionSystem 使用
- MapBounds

### 状态机组件 (state_machine.rs)
- PlayerStateMachine, PlayerState - PlayerStateSystem 使用

### 地图组件 (map.rs)
- MapData, Camera, VisibleArea

### 渲染组件 (render.rs)
- Sprite, TimeTracker

## 📋 清理步骤

### 步骤 1: 删除 CharacterAnimationSystem
- [x] 从 scene.rs 移除注册
- [x] 从 imports 移除
- [ ] 删除文件 `animation_system.rs`
- [ ] 从 mod.rs 移除导出

### 步骤 2: 删除 AnimationControl 和 AnimationState
- [x] 从 scene.rs 移除组件添加
- [x] 从 PlayerStateSystem 移除同步代码
- [ ] 删除文件 `animation_state.rs`
- [ ] 从 components/mod.rs 移除导出

### 步骤 3: 评估 prediction.rs
- [ ] 搜索所有使用 PredictionState 的地方
- [ ] 如果确认未使用，删除文件
- [ ] 从 components/mod.rs 移除导出

### 步骤 4: 清理 mod.rs 导出
- [ ] 移除对已删除组件的 pub use
- [ ] 移除注释掉的代码 (QuestLog, TradeWindow)

### 步骤 5: 编译测试
- [ ] cargo build --bin map_viewer_v3
- [ ] cargo build --bin game_scene (如果存在)
- [ ] 确保没有编译错误

## ⚠️ 需要保留的组件（虽然看起来未使用）

### game_scene.rs 中使用的
- MonsterAISystem
- NpcDialogueSystem
- SkillSystem
- CombatSystem
- ParticleSystem
- HealthRegenSystem
- SoundSystem

这些系统在 game_scene.rs 中注册，但我们主要在 map_viewer 中测试。

### 未来可能使用的
- combat.rs - 战斗组件
- spell.rs - 技能组件
- item.rs - 物品组件
- network.rs - 网络组件
- particle.rs - 粒子组件
- sound.rs - 音效组件
- events.rs - 事件组件

## 🎯 当前状态

### ✅ 已完成
1. **系统移除**:
   - ✅ 移除 CharacterAnimationSystem 的注册（map_viewer/scene.rs, game_scene.rs）
   - ✅ 注释掉 systems/logic/update/mod.rs 中的 animation_system 模块
   - ✅ 注释掉 systems/logic/mod.rs 中的 CharacterAnimationSystem 导入
   - ✅ 注释掉 systems/mod.rs 中的 CharacterAnimationSystem 导出

2. **组件清理**:
   - ✅ 移除 AnimationControl 组件的添加
   - ✅ 注释掉 components/mod.rs 中的 animation_state 模块
   - ✅ 移除 PlayerStateSystem 中的 AnimationControl 同步代码

3. **代码删除**:
   - ✅ 删除 PlayerState.frame_count() 和 frame_interval() 方法

4. **依赖发现**:
   - ✅ 确认 prediction.rs **被 MapUpdateSystem 使用**（已保留）

5. **编译验证**:
   - ✅ **cargo build 成功**: `Finished dev profile [optimized + debuginfo]`
   - ✅ 所有编译错误已解决

### 📋 可选操作
- 物理删除文件: animation_system.rs, animation_state.rs（当前仅注释掉模块声明）
- 运行时测试: `cargo run --bin map_viewer_v3`
- ⏳ 清理 components/mod.rs
- ⏳ 编译测试

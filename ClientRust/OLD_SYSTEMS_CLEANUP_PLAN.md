# 旧系统清理计划

## 🗑️ 需要删除的系统（已被新系统替代）

### 1. movement_system.rs
**状态**: ❌ 待删除  
**原因**: 已被 `movement_system_v2.rs` (纯物理运动) + `local_prediction_system.rs` (预测寻路) 替代  
**功能**: 旧的移动系统，包含寻路、移动、网络发送等多个职责，违反单一职责原则  
**替代方案**: 
- 寻路逻辑 → PathfindingService
- 预测移动 → LocalPredictionSystem
- 纯物理 → MovementSystemV2

### 2. pathfinding_system.rs
**状态**: ❌ 待删除  
**原因**: 已被 `PathfindingService` 替代（无状态服务，更适合寻路逻辑）  
**功能**: 旧的寻路系统，但寻路本质上是无状态的纯函数，不应该是系统  
**替代方案**: PathfindingService::find_path()

### 3. input_system.rs
**状态**: ❌ 待删除  
**原因**: 已被 `input_collecting_system.rs` 替代  
**功能**: 旧的输入系统，职责不清晰  
**替代方案**: InputCollectingSystem

### 4. network_system.rs
**状态**: ❌ 待删除  
**原因**: 已被 `client_network_system.rs` 替代  
**功能**: 旧的网络系统，收发逻辑混在一起  
**替代方案**: ClientNetworkSystem（收发分离，process_event 处理接收，send_commands 处理发送）

---

## 🔄 需要重构的系统（保留但需要简化）

### 5. animation_system.rs
**状态**: 🔄 需要重构  
**当前问题**: 
- 可能包含动画状态决策逻辑（应该在 AnimationStateSystem 中）
- 应该只负责播放动画（更新帧索引、切换帧图像）

**重构方案**:
- 移除动画状态决策逻辑 → AnimationStateSystem
- 只保留动画播放逻辑：
  - 读取 AnimationStateComponent
  - 更新 current_frame
  - 处理循环/非循环动画

### 6. camera_system.rs
**状态**: ✅ 保留  
**职责**: 相机跟随玩家  
**检查**: 确保没有包含游戏逻辑，只负责相机位置计算

### 7. render_system/
**状态**: ✅ 保留  
**职责**: 纯渲染（Layer 4）  
**检查**: 确保没有包含游戏逻辑，只负责绘制

---

## ✅ 保留的系统（游戏逻辑，Layer 2）

### 8. monster_system.rs
**状态**: ✅ 保留  
**职责**: 怪物AI逻辑（Layer 2）  
**注意**: 应该写入 VelocityComponent，由 MovementSystemV2 执行物理运动

### 9. combat_system.rs
**状态**: ✅ 保留  
**职责**: 战斗逻辑（Layer 2）  
**注意**: 应该调用 AnimationStateSystem::trigger_attack()

### 10. magic_cast_system.rs
**状态**: ✅ 保留  
**职责**: 施法逻辑（Layer 2）  
**注意**: 应该调用 AnimationStateSystem::trigger_spell()

### 11. magic_learning_system.rs
**状态**: ✅ 保留  
**职责**: 技能学习（Layer 2）

### 12. item_system.rs
**状态**: ✅ 保留  
**职责**: 物品拾取、使用（Layer 2）

### 13. npc_system.rs
**状态**: ✅ 保留  
**职责**: NPC交互（Layer 2）

### 14. quest_system.rs
**状态**: ✅ 保留  
**职责**: 任务系统（Layer 2）

### 15. trade_system.rs
**状态**: ✅ 保留  
**职责**: 交易系统（Layer 2）

### 16. ui_system.rs
**状态**: ✅ 保留  
**职责**: UI系统（Layer 5）

### 17. occlusion_system.rs
**状态**: ✅ 保留  
**职责**: 遮挡剔除（Layer 4 优化）

---

## 📝 清理步骤

### 阶段1: 备份（可选）
```bash
# 创建备份分支
git checkout -b backup-before-cleanup
git commit -am "Backup before cleaning old systems"
git checkout ggez-game
```

### 阶段2: 删除旧系统文件

```bash
# 1. 删除已替代的系统
rm src/ecs/systems/movement_system.rs
rm src/ecs/systems/pathfinding_system.rs
rm src/ecs/systems/input_system.rs
rm src/ecs/systems/network_system.rs

# 2. 从 mod.rs 中移除这些模块
# 手动编辑 src/ecs/systems/mod.rs
```

### 阶段3: 更新 mod.rs

需要从 `mod.rs` 中移除：
```rust
pub mod movement_system;      // ❌ 删除
pub mod pathfinding_system;   // ❌ 删除
pub mod input_system;         // ❌ 删除
pub mod network_system;       // ❌ 删除
```

保留：
```rust
// === 新架构：五层系统 ===
pub mod input_collecting_system;   // ✅ Layer 1
pub mod client_network_system;     // ✅ Layer 1
pub mod local_prediction_system;   // ✅ Layer 2
pub mod movement_system_v2;        // ✅ Layer 2
pub mod reconciliation_system;     // ✅ Layer 2
pub mod interpolation_system;      // ✅ Layer 2
pub mod animation_state_system;    // ✅ Layer 3

// === 保留的旧系统 ===
pub mod camera_system;             // ✅ Layer 4
pub mod animation_system;          // ✅ Layer 4（需要重构）
pub mod render_system;             // ✅ Layer 4
pub mod monster_system;            // ✅ Layer 2
pub mod combat_system;             // ✅ Layer 2
pub mod magic_cast_system;         // ✅ Layer 2
pub mod magic_learning_system;     // ✅ Layer 2
pub mod item_system;               // ✅ Layer 2
pub mod npc_system;                // ✅ Layer 2
pub mod quest_system;              // ✅ Layer 2
pub mod trade_system;              // ✅ Layer 2
pub mod ui_system;                 // ✅ Layer 5
pub mod occlusion_system;          // ✅ Layer 4
```

### 阶段4: 检查编译

```bash
cargo check
```

### 阶段5: 更新 GameApp

在 `game_app.rs` 中移除对旧系统的引用：
- 删除 `MovementSystem` 的调用 → 使用 `MovementSystemV2`
- 删除 `PathfindingSystem` 的调用 → 在 `LocalPredictionSystem` 内部调用 `PathfindingService`
- 删除 `InputSystem` 的调用 → 使用 `InputCollectingSystem`
- 删除 `NetworkSystem` 的调用 → 使用 `ClientNetworkSystem`

### 阶段6: 集成测试

运行游戏，测试：
1. ✅ 点击移动是否立即响应
2. ✅ 服务器校正是否平滑
3. ✅ 其他玩家移动是否流畅
4. ✅ 动画切换是否正确
5. ✅ 战斗、施法、拾取等功能是否正常

---

## ⚠️ 注意事项

1. **逐步删除**: 一次删除一个系统，检查编译
2. **保留文档**: 在删除前记录旧系统的关键逻辑
3. **测试充分**: 删除后彻底测试所有功能
4. **可回滚**: 确保 git 可以回滚到删除前状态

---

## 📊 预期结果

**删除前**:
- 系统文件: ~20个
- 平均系统大小: 500-900行
- 职责混乱: 多个系统做类似的事

**删除后**:
- 系统文件: ~16个
- 新系统大小: 72-238行
- 职责清晰: 每个系统单一职责
- 代码总量: 减少 ~30%

**性能提升**:
- ECS 查询次数减少
- 系统调用顺序更清晰
- 数据流向更明确

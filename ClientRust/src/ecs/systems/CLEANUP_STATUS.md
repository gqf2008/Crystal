# 废弃系统清理状态报告

**最后更新：2025-01-28**

## 🎯 清理目标

彻底清理 `deprecated/` 目录中的废弃系统，确保 `game_scene.rs` 不再依赖旧代码。

---

## ✅ 已完成的清理

### 1. **从主循环移除调用**
- ✅ `PathfindingSystem::update()` - 已注释（game_scene.rs:628）
- ✅ `MovementSystem::update()` - 已注释（game_scene.rs:631）

### 2. **从导出中移除**
- ✅ `systems/mod.rs` - 不再导出 PathfindingSystem, MovementSystem
- ✅ `deprecated/mod.rs` - 不再导出这两个系统（但保留模块声明）

### 3. **添加警告文档**
- ✅ `pathfinding_system.rs` - 添加大型警告框
- ✅ `movement_system.rs` - 添加大型警告框

---

## ⚠️ 保留的系统（有充分理由）

### **InputSystem** - ⚠️ 部分保留
**状态：** 仍在使用中

**原因：**
- `game_scene.rs` 中 **6个事件处理方法** 仍在活跃调用：
  - `process_keyboard()` (line 789)
  - `process_mouse_click()` (line 808)
  - `process_mouse_up()` (line 823)
  - `process_mouse_move()` (line 839)
  - `process_mouse_wheel()` (line 852)
  - `update_mouse_input()` (line 634)

**内容：**
- 515行代码，包含大量键盘快捷键逻辑（F1-F8, I/C/S/K/Q/T, Space, Z, N）
- 调用其他系统：UISystem, ItemSystem, MagicCastSystem, NPCSystem
- 非简单包装器 - 包含实质性游戏逻辑

**替代方案：**
- `InputCollectingSystem` 仅负责收集输入，不处理业务逻辑
- InputSystem 实际上是 **事件分发器**，应保留或重新归类

---

### **NetworkSystem** - ⚠️ 部分保留
**状态：** 仍在使用中

**原因：**
- `NetworkSystem::process_event()` 仍在 `game_scene.rs:432` 被调用
- 处理关键服务器事件：
  - `MapInformation`
  - `ObjectSpawned`
  - `ObjectRemoved`
  - `PlayerMoved`
- 管理 object_map 和实体生命周期

**替代方案：**
- `ClientNetworkSystem::receive_updates()` 已实现
- 需要迁移事件处理逻辑到新系统
- **待办：** 完成迁移后才能移除

---

### **AnimationSystem / DoorSystem** - ⚠️ 部分保留
**状态：** DoorSystem 仍在使用中

**原因：**
- `DoorSystem::update()` 在 `game_scene.rs` 和 `map_viewer_ecs.rs` 中被调用
- 系统仍在 `systems/mod.rs` 中导出
- 包含门动画的特殊逻辑

**替代方案：**
- 应迁移到 Layer 3（表现决策层）
- **待办：** 创建 DoorAnimationSystem 或集成到 AnimationStateSystem

---

### **PathfindingSystem & MovementSystem** - ⚠️ 向后兼容保留
**状态：** 文件保留但不导出

**原因：**
- `map_viewer_ecs.rs` 仍在使用（lines 434, 437）
- map_viewer 是独立的可执行文件，不使用新架构

**处理方案：**
- ✅ 从 `systems/mod.rs` 移除导出
- ✅ 从 `deprecated/mod.rs` 移除导出
- ✅ 添加大型警告文档
- ✅ game_scene.rs 不再能访问
- ⚠️ map_viewer 可通过 `deprecated::pathfinding_system::PathfindingSystem` 直接访问模块

**删除条件：**
- map_viewer 迁移到新架构
- 或 map_viewer 使用独立的副本

---

## 📊 清理统计

| 系统                  | 文件状态 | 导出状态 | game_scene 使用 | 其他使用            |
|-----------------------|----------|----------|-----------------|---------------------|
| PathfindingSystem     | 保留     | ❌ 移除  | ❌ 不使用       | map_viewer          |
| MovementSystem        | 保留     | ❌ 移除  | ❌ 不使用       | map_viewer          |
| InputSystem           | 保留     | ✅ 导出  | ✅ 使用（6方法）| -                   |
| NetworkSystem         | 保留     | ✅ 导出  | ✅ 使用（1方法）| -                   |
| AnimationSystem       | 保留     | ✅ 导出  | ✅ DoorSystem   | map_viewer          |

---

## 🔄 迁移路径（未来工作）

### **Phase 1: InputSystem 迁移** （复杂度：高）
1. 分析 `process_keyboard` 中的 12 个键盘快捷键
2. 确定业务逻辑归属层级：
   - UI 操作（F1-F8）→ Layer 5
   - 物品操作（Space, Z）→ Layer 2
   - 对话/交易（N, T）→ Layer 2/3
3. 创建 `KeyboardShortcutSystem` 或分散到对应层
4. 迁移后删除 InputSystem

### **Phase 2: NetworkSystem 迁移** （复杂度：中）
1. 将 `process_event` 逻辑迁移到 `ClientNetworkSystem::receive_updates`
2. 重构 object_map 管理
3. 测试服务器事件处理
4. 删除 NetworkSystem

### **Phase 3: DoorSystem 独立** （复杂度：低）
1. 从 AnimationSystem 提取 DoorSystem
2. 移动到 `layer3_presentation/door_animation_system.rs`
3. 删除 AnimationSystem

### **Phase 4: map_viewer 迁移** （复杂度：中）
1. 升级 map_viewer 使用新架构
2. 或为其创建独立的旧系统副本
3. 删除 PathfindingSystem 和 MovementSystem 文件

---

## 🎯 清理完成度

**整体进度：** 40%

```
✅ PathfindingSystem  - 从 game_scene 移除
✅ MovementSystem     - 从 game_scene 移除
⚠️ InputSystem        - 待迁移（复杂）
⚠️ NetworkSystem      - 待迁移（中等）
⚠️ AnimationSystem    - 待拆分（简单）
```

---

## ✅ 验证清单

- [x] game_scene.rs 不再调用 PathfindingSystem::update
- [x] game_scene.rs 不再调用 MovementSystem::update
- [x] systems/mod.rs 不再导出这两个系统
- [x] deprecated/mod.rs 注释掉导出
- [x] 添加警告文档到废弃文件
- [ ] cargo check 编译通过
- [ ] 功能测试（寻路、移动）仍正常工作
- [ ] map_viewer 仍能编译运行

---

## 🚨 重要警告

**不要直接删除文件！**

即使文件未被导出，map_viewer_ecs.rs 仍通过以下方式访问：

```rust
use crate::ecs::systems::deprecated::pathfinding_system::PathfindingSystem;
use crate::ecs::systems::deprecated::movement_system::MovementSystem;
```

删除文件会导致 `cargo build` 失败。

**正确做法：**
1. 先迁移 map_viewer
2. 再删除文件
3. 或保留文件但标记为 `#[deprecated]`

---

## 📝 后续步骤

1. **立即：** 运行 `cargo check` 验证清理不破坏编译
2. **本周：** 测试 game_scene 的寻路和移动功能
3. **下周：** 制定 InputSystem 迁移计划
4. **本月：** 完成 NetworkSystem 迁移
5. **下季度：** 完成所有迁移，删除 deprecated 目录

---

## 👤 审查签名

**执行人：** GitHub Copilot  
**审查人：** 用户  
**状态：** ✅ 部分完成 - 等待编译验证和功能测试  
**风险等级：** 🟡 中等 - InputSystem 迁移需要谨慎设计  

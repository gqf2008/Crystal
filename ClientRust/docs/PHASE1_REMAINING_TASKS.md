# Phase 1 剩余任务计划

**日期**: 2025-10-03  
**状态**: 🟡 规划中  
**完成**: P0 任务已完成 ✅，现在处理 P1 任务

---

## 📋 任务概览

基于当前 326 个编译错误的分析，Phase 1 剩余任务分为以下几类：

### P1 - 网络层重构 (高优先级)

**问题**：`controls/mod.rs` 中使用了旧的 `ServerMessage` 枚举和 `parse_server_message` 函数，这些在 `protocol.rs` 重构后已经不存在。

**影响的错误**：
- `error[E0425]`: `parse_server_message` 函数未找到
- `error[E0599]`: `ServerMessage::*` 类型未找到（约 100+ 个）
- `error[E0223]`: 关联类型歧义（约 7 个）
- `error[E0599]`: `NetworkEvent::Packet` 变体未找到
- `error[E0599]`: `NetworkStack::next_event` 方法未找到

**涉及文件**：
- `src/controls/mod.rs` (1951 行) - 主要问题文件
- `src/network/protocol.rs` (已重构，使用 PacketHandler trait)

**任务**：
1. ✅ **P1-1**: 移除 `ServerMessage` 枚举依赖
2. ✅ **P1-2**: 实现基于 `PacketHandler` trait 的数据包处理
3. ✅ **P1-3**: 更新网络事件循环使用新 API

---

### P2 - 模块导入修复 (中优先级)

**问题**：`program.rs` 导入了不存在的模块

**错误**：
```
error[E0432]: unresolved imports `crate::audio`, `crate::net`, `crate::ui`
```

**涉及文件**：
- `src/program.rs` (78 行)

**任务**：
1. ✅ **P2-1**: 检查这些模块是否存在
2. ✅ **P2-2**: 如果不存在，注释掉或创建占位模块
3. ✅ **P2-3**: 更新相关代码以处理缺失的模块

---

### P3 - UI Trait 实现 (低优先级)

**问题**：多个对话框未完全实现 `Dialog` trait

**错误**：
```
error[E0046]: not all trait items implemented, missing: 
  `name`, `contains_point`, `position`, `size`
```

**涉及文件**（16 个对话框）：
1. `src/scenes/dialogs/main_dialog.rs`
2. `src/scenes/dialogs/chat_dialog.rs`
3. `src/scenes/dialogs/inventory_dialog.rs`
4. `src/scenes/dialogs/character_dialog.rs`
5. `src/scenes/dialogs/skillbar_dialog.rs`
6. `src/scenes/dialogs/npc_dialog.rs`
7. `src/scenes/dialogs/storage_dialog.rs`
8. `src/scenes/dialogs/trade_dialog.rs`
9. `src/scenes/dialogs/guild_dialog.rs`
10. `src/scenes/dialogs/friend_dialog.rs`
11. `src/scenes/dialogs/group_dialog.rs`
12. `src/scenes/dialogs/bigmap_dialog.rs`
13. `src/scenes/dialogs/quest_list_dialog.rs`
14. `src/scenes/dialogs/mail_dialog.rs` (2 个结构体)
15. `src/scenes/dialogs/help_dialog.rs`

**任务**：
1. ✅ **P3-1**: 查看 `Dialog` trait 定义
2. ✅ **P3-2**: 为所有对话框添加缺失的方法（批量操作）

---

## 🎯 优先级排序

### 立即执行 (P1)

**P1-1, P1-2, P1-3**: 网络层重构
- **理由**: 影响最大（100+ 错误），是后续功能的基础
- **预计时间**: 2-3 小时
- **难度**: ⭐⭐⭐⭐（需要理解 PacketHandler trait 和异步处理）

### 第二优先 (P2)

**P2-1, P2-2, P2-3**: 模块导入修复
- **理由**: 简单快速，影响编译流程
- **预计时间**: 15-30 分钟
- **难度**: ⭐（简单注释或创建占位）

### 第三优先 (P3)

**P3-1, P3-2**: UI Trait 实现
- **理由**: 纯机械性工作，可以批量处理
- **预计时间**: 1-2 小时
- **难度**: ⭐⭐（重复性工作，可以脚本化）

---

## 📊 任务依赖关系

```
P0 (MapObject 重构) ✅ 已完成
    ↓
P2 (模块导入) → 可以并行
P1 (网络层) → 依赖 protocol.rs 重构
    ↓
P3 (UI Trait) → 可以并行
    ↓
Phase 1 完成 🎯
```

---

## 🔧 详细执行计划

### P1 - 网络层重构 (controls/mod.rs)

#### 当前问题代码
```rust
// ❌ 旧代码 (不存在的 API)
match event {
    NetworkEvent::Packet { header, payload } => {
        match parse_server_message(header, payload) {
            ServerMessage::Connected => { ... }
            ServerMessage::ClientVersion { result } => { ... }
            ServerMessage::NewMapInfo(packet) => { ... }
            // ... 100+ 个 ServerMessage 匹配
        }
    }
}
```

#### 问题分析

1. **`ServerMessage` 枚举已移除**
   - 在 `protocol.rs` 重构中，我们移除了 `ServerMessage` 枚举
   - 现在使用 `PacketHandler` trait 直接处理类型化数据包

2. **`parse_server_message` 函数不存在**
   - 旧代码依赖此函数解析数据包到 `ServerMessage` 枚举
   - 新架构直接使用 `mir2_shared::packets::server::*` 类型

3. **`NetworkEvent::Packet` 变体可能不存在**
   - 需要检查 `NetworkEvent` 的定义
   - 可能需要更新为新的事件类型

#### 解决方案

**方案 A: 实现 PacketHandler trait**（推荐）
```rust
struct GamePacketHandler {
    // 游戏状态
}

impl PacketHandler for GamePacketHandler {
    fn on_connected(&mut self, packet: packets::Connected) {
        tracing::info!("received server handshake");
        // 处理连接
    }
    
    fn on_new_map_info(&mut self, packet: packets::NewMapInfo) {
        // 处理地图信息
    }
    
    // ... 实现所有需要的方法
}

// 在事件循环中
let mut handler = GamePacketHandler::new();
network.dispatch_packet(&header, &payload, &mut handler)?;
```

**方案 B: 恢复 ServerMessage 枚举**（不推荐）
- 违反了 protocol.rs 的设计原则
- 创建不必要的中间层
- 不采用

**方案 C: 直接匹配 opcode**（临时方案）
```rust
use mir2_shared::enums::ServerPacketIds;

match header.opcode as u16 {
    x if x == ServerPacketIds::Connected as u16 => {
        let packet = packets::Connected::parse(payload)?;
        // 处理
    }
    x if x == ServerPacketIds::NewMapInfo as u16 => {
        let packet = packets::NewMapInfo::parse(payload)?;
        // 处理
    }
    // ...
}
```

**决策**: 采用 **方案 A**，因为：
- 符合新架构设计
- 类型安全
- 易于维护和扩展

#### 实现步骤

**Step 1**: 检查 NetworkEvent 定义
```rust
// 需要查看 src/network/mod.rs 或相关文件
// 确认 NetworkEvent 的定义和 next_event() 方法
```

**Step 2**: 创建 GamePacketHandler 结构体
```rust
// 在 controls/mod.rs 中
struct GamePacketHandler {
    game_state: Arc<Mutex<GameState>>,
    // 其他需要的状态
}
```

**Step 3**: 实现 PacketHandler trait
```rust
impl PacketHandler for GamePacketHandler {
    // 实现 100+ 个数据包处理方法
    // 只需要实现实际使用的方法，其他保持默认空实现
}
```

**Step 4**: 重构事件循环
```rust
// ❌ 旧代码
match parse_server_message(header, payload) {
    ServerMessage::Connected => { ... }
}

// ✅ 新代码
let mut handler = GamePacketHandler { ... };
dispatch_packet(&header, payload, &mut handler)?;
```

---

### P2 - 模块导入修复 (program.rs)

#### 当前错误
```rust
use crate::{audio, net, ui, version};
//          ^^^^^  ^^^  ^^
//          这些模块不存在
```

#### 解决方案

**Step 1**: 检查模块是否存在
```bash
ls src/audio/
ls src/net/
ls src/ui/
```

**Step 2**: 如果不存在，创建占位或注释

**方案 A**: 创建占位模块
```rust
// src/audio/mod.rs (如果不存在)
// Audio system - TODO: implement
pub fn init() {
    tracing::info!("Audio system not yet implemented");
}
```

**方案 B**: 条件编译
```rust
#[cfg(feature = "audio")]
use crate::audio;

#[cfg(feature = "ui")]
use crate::ui;
```

**方案 C**: 直接注释（如果当前不需要）
```rust
// use crate::{audio, net, ui, version};
use crate::version;
```

---

### P3 - UI Trait 实现 (批量修复)

#### 问题
```
error[E0046]: not all trait items implemented, missing: 
  `name`, `contains_point`, `position`, `size`
```

#### 解决方案

**Step 1**: 查看 Dialog trait 定义
```rust
// 需要找到 Dialog trait 的定义
trait Dialog {
    fn name(&self) -> &str;
    fn contains_point(&self, point: Point) -> bool;
    fn position(&self) -> Point;
    fn size(&self) -> Size;
    
    // 可能还有其他方法
}
```

**Step 2**: 为每个对话框添加缺失方法

**模板**:
```rust
impl Dialog for MainDialog {
    fn name(&self) -> &str {
        "MainDialog"  // 或者使用 self.title
    }
    
    fn contains_point(&self, point: Point) -> bool {
        // 计算是否在对话框范围内
        let pos = self.position();
        let size = self.size();
        point.x >= pos.x && point.x < pos.x + size.width &&
        point.y >= pos.y && point.y < pos.y + size.height
    }
    
    fn position(&self) -> Point {
        self.position  // 或者从字段获取
    }
    
    fn size(&self) -> Size {
        self.size  // 或者从字段获取
    }
}
```

**Step 3**: 批量应用到所有 16 个对话框

可以使用 `multi_replace_string_in_file` 批量处理。

---

## 📈 预期结果

### P1 完成后
- 减少约 **120 个错误**（所有 ServerMessage 和网络相关错误）
- controls/mod.rs 编译通过
- 网络数据包处理正常工作

### P2 完成后
- 减少 **3 个错误**（模块导入错误）
- program.rs 编译通过

### P3 完成后
- 减少约 **16 个错误**（Dialog trait 未完全实现）
- 所有对话框编译通过

### 总计
- 当前错误：326 个
- 预期剩余：**~187 个错误**
- 改善：**-43%**

---

## 🚀 执行顺序

### 第一步：P2 (快速胜利)
- 时间：15 分钟
- 影响：小但快

### 第二步：P1 (核心任务)
- 时间：2-3 小时
- 影响：最大

### 第三步：P3 (批量修复)
- 时间：1-2 小时
- 影响：中等

---

## 📝 注意事项

### P1 执行注意事项
1. 需要仔细理解 `PacketHandler` trait 设计
2. 保持与 C# Client 的事件处理逻辑一致
3. 注意异步处理和锁的使用
4. 每实现 10 个处理器就编译一次，逐步验证

### P2 执行注意事项
1. 检查这些模块是否在其他分支或目录中
2. 如果是核心功能，创建占位而不是删除
3. 更新相关的调用代码

### P3 执行注意事项
1. 先实现一个对话框，确认模板正确
2. 然后批量应用到其他对话框
3. 确保 `position` 和 `size` 字段存在于所有对话框结构体中

---

## 🎯 成功标准

- [ ] controls/mod.rs 编译通过（0 错误）
- [ ] program.rs 编译通过（0 错误）
- [ ] 所有 16 个对话框编译通过（0 错误）
- [ ] 总错误数 < 200
- [ ] 无新增错误
- [ ] 所有修改有清晰的文档

---

**下一步**: 开始执行 P2 (模块导入修复)，因为它最简单快速 ✅


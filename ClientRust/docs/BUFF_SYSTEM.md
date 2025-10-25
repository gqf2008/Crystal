# Buff/Debuff 系统设计文档

## 概述

Buff/Debuff 系统用于显示玩家当前的增益和减益效果。

## 已完成的工作

### 1. **Buff 显示对话框** ✅
**文件**: `src/ecs/ui/buff_dialog.rs`

**功能**:
- 显示玩家当前的 Buff 和 Debuff 列表
- 图标显示在屏幕上方中央
- 显示剩余时间倒计时
- 鼠标悬停显示详细信息 (名称、描述、剩余时间)
- 支持右键点击取消可移除的 Buff
- 自动移除过期的 Buff

**位置**:
- Buffs: 屏幕顶部中央
- Debuffs: Buffs 下方一行

**UI 布局**:
```
┌─────────────────────────────────────┐
│  [Buff1] [Buff2] [Buff3] [Buff4]    │  ← 增益效果 (蓝色边框)
│   1:30    2:45    ∞      0:45       │
│                                      │
│  [Debuff1] [Debuff2]                │  ← 减益效果 (红色边框)
│    0:15      0:30                   │
└─────────────────────────────────────┘
```

**数据结构**:
```rust
pub struct BuffItem {
    pub buff_type: BuffType,     // Buff 类型
    pub visible: bool,            // 是否可见
    pub object_id: u32,           // 归属对象ID
    pub expire_time: i64,         // 过期时间 (毫秒)
    pub infinite: bool,           // 是否永久
    pub paused: bool,             // 是否暂停
    pub icon_index: u32,          // 图标索引
    pub name: String,             // Buff 名称
    pub description: String,      // Buff 描述
}
```

### 2. **Buff 类型定义** ✅
**文件**: `SharedRust/src/enums.rs`

**支持的 Buff 类型**:
- **魔法类**: 隐身、加速、狂怒、护盾、诅咒等 (25种)
- **怪物类**: 怪物特殊Buff和Debuff (8种)
- **特殊类**: GM、经验加成、金币加成等 (16种)
- **属性类**: 攻击、防御、生命、魔法加成等 (10种)

**Buff 属性**:
```rust
pub struct BuffProperty {
    REMOVE_ON_DEATH,      // 死亡时移除
    REMOVE_ON_EXIT,       // 下线时移除
    DEBUFF,               // 是否为减益
    PAUSE_IN_SAFE_ZONE,   // 安全区暂停
}
```

### 3. **网络协议** ✅
**文件**: `SharedRust/src/packets/server/buff.rs`

**数据包**:
- `AddBuff`: 添加 Buff
- `RemoveBuff`: 移除 Buff
- `PauseBuff`: 暂停/恢复 Buff
- `ColourChanged`: 玩家颜色变化 (Buff 效果)
- `ObjectColourChanged`: 对象颜色变化
- `Poisoned`: 中毒状态

## 待集成工作

### 1. **GameClient 处理 Buff 事件** ⏳
**文件**: `src/network/game_client.rs`
- 已有 Buff 处理框架 (1438行注释)
- 需要实现 `on_add_buff()` / `on_remove_buff()`
- 将 Buff 事件转发给 UI 层

**示例代码**:
```rust
fn on_add_buff(&mut self, packet: packets::AddBuff) {
    self.send_event(GameEvent::AddBuff {
        buff: packet.buff,
    });
}

fn on_remove_buff(&mut self, packet: packets::RemoveBuff) {
    self.send_event(GameEvent::RemoveBuff {
        buff_type: packet.buff_type,
        object_id: packet.object_id,
    });
}
```

### 2. **GameScene 集成 BuffDialog** ⏳
**文件**: `src/ecs/scenes/game_scene.rs`
- 在 `GameScene::new()` 中创建 `BuffDialogComp` 实体
- 在 `draw()` 中绘制 Buff 对话框
- 在 `handle_network_event()` 中处理 Buff 事件

**示例代码**:
```rust
// 创建 Buff 对话框实体
let buff_dialog_entity = world.spawn((
    BuffDialogComp::new(screen_width, screen_height),
));

// 绘制
if let Ok(buff_dialog) = world.get::<&BuffDialogComp>(buff_dialog_entity) {
    buff_dialog.dialog.draw(ctx, canvas)?;
}

// 处理事件
GameEvent::AddBuff { buff } => {
    if let Ok(mut buff_dialog) = world.get::<&mut BuffDialogComp>(buff_dialog_entity) {
        let item = BuffItem {
            buff_type: buff.buff_type,
            visible: buff.visible,
            // ... 转换 ClientBuff 到 BuffItem
        };
        buff_dialog.dialog.add_buff(item);
    }
}
```

### 3. **UISystem 处理 Buff 事件** ⏳
**文件**: `src/ecs/systems/ui_system.rs`
- 在 `process_event()` 中添加 Buff 事件处理
- 更新 BuffDialog 状态
- 处理鼠标交互 (悬停、点击)

### 4. **Buff 图标资源** ⏳
需要从图像库加载 Buff 图标:
- 路径: `Data/Buff.lib` (猜测)
- 每个 BuffType 对应一个图标索引
- 使用 `MLibrary` 系统加载和绘制

### 5. **Buff 名称本地化** ⏳
**文件**: `src/ecs/ui/buff_dialog.rs`
- `get_buff_name()` 方法已实现基础翻译
- 需要完善所有 Buff 类型的中文名称
- 可选: 从配置文件加载翻译

## 测试计划

### 1. 单元测试
```rust
#[test]
fn test_buff_item_remaining_time() {
    let buff = BuffItem {
        expire_time: future_time(),
        infinite: false,
        // ...
    };
    assert!(buff.remaining_seconds() > 0);
}

#[test]
fn test_buff_expiration() {
    let mut dialog = BuffDialog::new(1024.0, 768.0);
    dialog.add_buff(expired_buff());
    dialog.update();  // 应该移除过期 Buff
    assert_eq!(dialog.buffs.len(), 0);
}
```

### 2. 集成测试
1. 启动客户端并登录
2. 服务器发送 `AddBuff` 数据包
3. 检查屏幕顶部是否显示 Buff 图标
4. 检查倒计时是否正常
5. 测试鼠标悬停显示提示
6. 测试右键取消 Buff

## 性能优化

### 1. 更新频率
- Buff 剩余时间每秒更新一次
- 过期检测每帧执行 (轻量级)

### 2. 内存管理
- 使用 `Vec` 动态管理 Buff 列表
- 及时移除过期的 Buff

### 3. 渲染优化
- 只绘制可见的 Buff
- 缓存 Buff 图标纹理

## 扩展功能

### 1. Buff 分组
- 按类型分组显示 (魔法/特殊/属性)
- 支持折叠/展开

### 2. Buff 排序
- 按剩余时间排序
- 按重要性排序
- 用户自定义排序

### 3. Buff 通知
- Buff 即将过期提醒 (剩余10秒)
- 重要 Buff 添加音效
- Buff 叠加提示

### 4. Buff 管理
- 支持拖拽重新排列
- 支持隐藏特定 Buff
- 支持 Buff 历史记录

## 参考资料

- C# 原版实现: `Client/MirScenes/GameScene/BuffList.cs`
- 网络协议: `SharedRust/src/packets/server/buff.rs`
- Buff 枚举: `SharedRust/src/enums.rs` (BuffType)

## 开发日志

- **2025-10-25**: 创建 BuffDialog UI 组件
- **待定**: 集成到 GameScene
- **待定**: 实现网络事件处理
- **待定**: 添加 Buff 图标支持

# SharedRust 快速参考 (Quick Reference)

## 🚀 快速开始

### 添加依赖

```toml
# ClientRust/Cargo.toml
[dependencies]
mir2_shared = { path = "../SharedRust" }
```

### 基础导入

```rust
use mir2_shared::{
    // 枚举
    MirDirection, MirClass, MirGender, ItemType, Spell,
    // 数据结构
    Point, UserItem, ItemInfo, Stats, SelectInfo,
    // 常量
    MAX_LEVEL, MAX_BAG_WEIGHT,
};
```

---

## 📋 常用功能速查

### 1. Point 操作

```rust
use mir2_shared::Point;

// 创建
let p = Point::new(10, 20);

// 运算
let p1 = Point::new(10, 20);
let p2 = Point::new(5, 7);
let sum = p1 + p2;           // Point(15, 27)
let diff = p1 - p2;          // Point(5, 13)

// 字符串转换
let s = p.to_string();       // "10, 20"
let p: Point = "10, 20".parse().unwrap();
```

### 2. 方向计算

```rust
use mir2_shared::{MirDirection, Point, utils::*};

// 从两点计算方向
let dir = direction_from_point(
    Point::new(10, 10),
    Point::new(15, 10)
); // MirDirection::Right

// 旋转方向
let next = next_dir(MirDirection::Up);         // UpRight
let prev = previous_dir(MirDirection::Up);     // UpLeft
let reverse = reverse_direction(MirDirection::Up); // Down

// 移动点
let moved = point_move(Point::new(10, 10), MirDirection::Right, 5);
// Point(15, 10)

// 距离检查
let in_range = in_range(Point::new(10, 10), Point::new(12, 11), 2); // true
let distance = max_distance(Point::new(10, 10), Point::new(15, 13)); // 5
```

### 3. 数据包操作

```rust
use mir2_shared::packets::client::account::Login;
use mir2_shared::packets::base::PacketMessage;

// 创建数据包
let packet = Login {
    username: "player".to_string(),
    password: "pass123".to_string(),
};

// 序列化
let bytes = packet.write_packet()?;

// 反序列化
let packet = Login::read_packet(&bytes)?;
```

---

## 📚 模块速查表

| 功能 | 导入路径 | 示例 |
|-----|---------|------|
| 枚举 | `mir2_shared::MirDirection` | `MirDirection::Up` |
| 全局常量 | `mir2_shared::MAX_LEVEL` | `MAX_LEVEL` |
| Point | `mir2_shared::Point` | `Point::new(10, 20)` |
| 方向函数 | `mir2_shared::utils::*` | `direction_from_point(p1, p2)` |
| 数据包 | `mir2_shared::packets::client::*` | `Login { ... }` |
| 数据结构 | `mir2_shared::UserItem` | `UserItem { ... }` |

---

## 🎯 已实现的客户端数据包

### 连接 (3)
```rust
use mir2_shared::packets::client::connection::{
    ClientVersion, Disconnect, KeepAlive
};
```

### 账户 (4)
```rust
use mir2_shared::packets::client::account::{
    NewAccount, ChangePassword, Login, StartGame
};
```

### 角色 (3)
```rust
use mir2_shared::packets::client::character::{
    NewCharacter, DeleteCharacter, LogOut
};
```

### 移动 (3)
```rust
use mir2_shared::packets::client::movement::{
    Turn, Walk, Run
};
```

### 聊天 (3)
```rust
use mir2_shared::packets::client::chat::{
    Chat, Inspect, Observe
};
```

### 物品 (12)
```rust
use mir2_shared::packets::client::item::{
    MoveItem, StoreItem, TakeBackItem, MergeItem,
    EquipItem, RemoveItem, RemoveSlotItem, SplitItem,
    UseItem, DropItem, DropGold, PickUp
};
```

### 战斗 (6)
```rust
use mir2_shared::packets::client::combat::{
    Attack, RangeAttack, Harvest,
    Magic, SpellToggle, MagicKey
};
```

---

## 🔄 C# → Rust 对照

### 命名转换

| C# | Rust | 说明 |
|----|------|------|
| `DirectionFromPoint()` | `direction_from_point()` | 函数 snake_case |
| `UniqueID` | `unique_id` | 字段 snake_case |
| `ItemInfo` | `ItemInfo` | 类型 PascalCase |
| `MaxLevel` | `MAX_LEVEL` | 常量 UPPER_CASE |

### 类型对照

| C# | Rust |
|----|------|
| `int` | `i32` |
| `byte` | `u8` |
| `string` | `String` |
| `List<T>` | `Vec<T>` |
| `Point` | `Point` |

### 常见操作

| C# | Rust |
|----|------|
| `p1.Add(p2)` | `p1 + p2` |
| `Functions.DirectionFromPoint(p1, p2)` | `direction_from_point(p1, p2)` |
| `packet.ToArray()` | `packet.write_packet()?` |
| `Globals.MaxLevel` | `MAX_LEVEL` |

---

## 📖 详细文档

完整的模块映射和使用说明请参考:
- **移植指南**: `PORTING_GUIDE.md`
- **移植报告**: `MIGRATION_REPORT.md`
- **API 文档**: 运行 `cargo doc --open`

---

## ⚡ 常用代码片段

### 角色登录流程
```rust
use mir2_shared::packets::client::account::{Login, StartGame};
use mir2_shared::packets::base::PacketMessage;

// 1. 登录
let login = Login {
    username: "player".to_string(),
    password: "pass123".to_string(),
};
send_packet(login.write_packet()?);

// 2. 选择角色
let start = StartGame {
    character_index: 0,
};
send_packet(start.write_packet()?);
```

### 角色移动
```rust
use mir2_shared::packets::client::movement::{Turn, Walk};
use mir2_shared::{MirDirection, Point, utils::*};

// 计算目标方向
let player_pos = Point::new(100, 100);
let target_pos = Point::new(105, 100);
let direction = direction_from_point(player_pos, target_pos);

// 转向
let turn = Turn { direction };
send_packet(turn.write_packet()?);

// 行走
let walk = Walk { direction };
send_packet(walk.write_packet()?);
```

### 物品操作
```rust
use mir2_shared::packets::client::item::{MoveItem, UseItem};
use mir2_shared::MirGridType;

// 移动物品
let move_item = MoveItem {
    grid: MirGridType::Inventory,
    from: 0,
    to: 5,
};
send_packet(move_item.write_packet()?);

// 使用物品
let use_item = UseItem {
    unique_id: 12345,
};
send_packet(use_item.write_packet()?);
```

### 战斗
```rust
use mir2_shared::packets::client::combat::{Attack, Magic};
use mir2_shared::{MirDirection, Spell};

// 普通攻击
let attack = Attack {
    direction: MirDirection::Right,
    spell: Spell::None,
};
send_packet(attack.write_packet()?);

// 释放技能
let magic = Magic {
    spell: Spell::FireBall,
    direction: MirDirection::Right,
    target_id: Some(999),
    location: Point::new(105, 100),
};
send_packet(magic.write_packet()?);
```

---

**最后更新**: 2025年10月2日

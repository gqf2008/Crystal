# C# Shared → Rust SharedRust 移植指南

## 📋 文档说明

本文档提供 C# Shared 项目到 Rust SharedRust 项目的**完整模块映射关系**，帮助 ClientRust 和 ServerRust 项目开发者快速找到对应的 Rust 实现。

**项目路径**:
- C# Shared: `Shared/`
- Rust SharedRust: `SharedRust/`

**最后更新**: 2025年10月2日

---

## 🗂️ 模块对应关系总览

| C# Shared 模块 | SharedRust 模块 | 状态 | 说明 |
|---------------|----------------|------|------|
| `Enums.cs` | `enums.rs` | ✅ 完整移植 | 所有枚举类型 |
| `Globals.cs` | `globals.rs` | ✅ 完整移植 | 全局常量和函数 |
| `Packet.cs` | `packets/base.rs` | ✅ 完整移植 | 数据包基础设施 |
| `ClientPackets.cs` | `packets/client/` | ⚠️ 部分移植 | 客户端数据包 (34/100+) |
| `ServerPackets.cs` | `packets/server/` | ⚠️ 部分移植 | 服务器数据包 |
| `BaseStats.cs` | `data/stats.rs` | ✅ 完整移植 | 基础属性统计 |
| `Data/` | `data/` | ✅ 完整移植 | 数据结构 |
| `Functions/Functions.cs` | `utils/direction.rs` | ✅ 核心函数已移植 | 方向和几何函数 |
| `Point` (System.Drawing) | `map.rs` | ✅ 完整移植 | 2D 点结构 |
| `Extensions/` | - | ❌ 无需移植 | 使用 Rust 标准库 |
| `Functions/IniReader.cs` | - | ❌ 无需移植 | 使用 Rust 配置库 |
| `Helpers/FileIO.cs` | - | ❌ 无需移植 | 使用 Rust std::fs |
| `Language.cs` | - | ❌ 无需移植 | 客户端特定,使用 i18n 库 |

---

## 📚 详细模块映射

### 1. 枚举类型 (Enums)

#### C# → Rust 映射

| C# (Shared/Enums.cs) | Rust (SharedRust/src/enums.rs) |
|---------------------|-------------------------------|
| `MirDirection` | `MirDirection` |
| `MirClass` | `MirClass` |
| `MirGender` | `MirGender` |
| `MirAction` | `MirAction` |
| `Spell` | `Spell` |
| `ItemType` | `ItemType` |
| `ItemGrade` | `ItemGrade` |
| `MirGridType` | `MirGridType` |
| `EquipmentSlot` | `EquipmentSlot` |
| `AttackMode` | `AttackMode` |
| `PetMode` | `PetMode` |
| `ClientPacketIds` | `ClientPacketIds` |
| `ServerPacketIds` | `ServerPacketIds` |
| ... (100+ 枚举) | ... (100+ 枚举) |

#### 使用示例

```rust
// C#
// using Shared;
// MirDirection dir = MirDirection.Up;

// Rust
use mir2_shared::MirDirection;
let dir = MirDirection::Up;
```

**状态**: ✅ 完整移植,所有枚举已实现
**位置**: `SharedRust/src/enums.rs`

---

### 2. 全局常量和函数 (Globals)

#### C# → Rust 映射

| C# (Shared/Globals.cs) | Rust (SharedRust/src/globals.rs) |
|----------------------|----------------------------------|
| `MaxBagWeight` | `MAX_BAG_WEIGHT` |
| `MaxWearWeight` | `MAX_WEAR_WEIGHT` |
| `MaxHandWeight` | `MAX_HAND_WEIGHT` |
| `MaxLevel` | `MAX_LEVEL` |
| `MaxAttackRange` | `MAX_ATTACK_RANGE` |
| `DataRange` | `DATA_RANGE` |
| `IsRangedSpell()` | `is_ranged_spell()` |
| `IsFishingRod()` | `is_fishing_rod()` |

#### 使用示例

```rust
// C#
// int maxWeight = Globals.MaxBagWeight;
// bool isRanged = Globals.IsRangedSpell(spell);

// Rust
use mir2_shared::{MAX_BAG_WEIGHT, is_ranged_spell};
let max_weight = MAX_BAG_WEIGHT;
let is_ranged = is_ranged_spell(spell);
```

**状态**: ✅ 完整移植
**位置**: `SharedRust/src/globals.rs`

---

### 3. 数据包系统 (Packets)

#### 3.1 数据包基础设施

| C# | Rust | 说明 |
|----|------|------|
| `Packet` 类 | `packets::base::PacketMessage` trait | 数据包接口 |
| `Packet.ToArray()` | `PacketMessage::write_packet()` | 序列化 |
| `Packet.FromArray()` | `PacketMessage::read_packet()` | 反序列化 |
| `PacketHeader` | `PacketHeader` struct | 数据包头 |

#### 使用示例

```rust
// C#
// var packet = new ClientVersion { VersionHash = hash };
// byte[] data = packet.ToArray();

// Rust
use mir2_shared::packets::client::connection::ClientVersion;
use mir2_shared::packets::base::PacketMessage;

let packet = ClientVersion { version_hash: hash };
let data = packet.write_packet()?;
```

**状态**: ✅ 基础设施完整移植
**位置**: `SharedRust/src/packets/base.rs`

#### 3.2 客户端数据包 (ClientPackets)

| C# 类别 | C# 文件 | Rust 模块 | 状态 |
|--------|---------|----------|------|
| 连接管理 | `ClientPackets.cs` | `packets/client/connection.rs` | ✅ 完整 (3/3) |
| 账户管理 | `ClientPackets.cs` | `packets/client/account.rs` | ✅ 完整 (4/4) |
| 角色管理 | `ClientPackets.cs` | `packets/client/character.rs` | ✅ 完整 (3/3) |
| 移动系统 | `ClientPackets.cs` | `packets/client/movement.rs` | ✅ 完整 (3/3) |
| 聊天系统 | `ClientPackets.cs` | `packets/client/chat.rs` | ✅ 完整 (3/3) |
| 物品管理 | `ClientPackets.cs` | `packets/client/item.rs` | ✅ 完整 (12/12) |
| 战斗系统 | `ClientPackets.cs` | `packets/client/combat.rs` | ✅ 完整 (6/6) |
| NPC 交互 | `ClientPackets.cs` | `packets/client/npc.rs` | ❌ 待实现 (~10) |
| 交易系统 | `ClientPackets.cs` | `packets/client/trade.rs` | ❌ 待实现 (5) |
| 组队系统 | `ClientPackets.cs` | `packets/client/group.rs` | ❌ 待实现 (4) |
| 公会系统 | `ClientPackets.cs` | `packets/client/guild.rs` | ❌ 待实现 (9) |
| 英雄系统 | `ClientPackets.cs` | `packets/client/hero.rs` | ❌ 待实现 (6) |
| 其他系统 | `ClientPackets.cs` | 待实现 | ❌ 待实现 (~50) |

**已实现数据包 (34个)**:

```rust
// 连接管理 (3)
use mir2_shared::packets::client::connection::{
    ClientVersion,    // C# ClientVersion
    Disconnect,       // C# Disconnect
    KeepAlive,        // C# KeepAlive
};

// 账户管理 (4)
use mir2_shared::packets::client::account::{
    NewAccount,       // C# NewAccount
    ChangePassword,   // C# ChangePassword
    Login,            // C# Login
    StartGame,        // C# StartGame
};

// 角色管理 (3)
use mir2_shared::packets::client::character::{
    NewCharacter,     // C# NewCharacter
    DeleteCharacter,  // C# DeleteCharacter
    LogOut,           // C# LogOut
};

// 移动系统 (3)
use mir2_shared::packets::client::movement::{
    Turn,             // C# Turn
    Walk,             // C# Walk
    Run,              // C# Run
};

// 聊天系统 (3)
use mir2_shared::packets::client::chat::{
    Chat,             // C# Chat
    Inspect,          // C# Inspect
    Observe,          // C# Observe
};

// 物品管理 (12)
use mir2_shared::packets::client::item::{
    MoveItem,         // C# MoveItem
    StoreItem,        // C# StoreItem
    TakeBackItem,     // C# TakeBackItem
    MergeItem,        // C# MergeItem
    EquipItem,        // C# EquipItem
    RemoveItem,       // C# RemoveItem
    RemoveSlotItem,   // C# RemoveSlotItem
    SplitItem,        // C# SplitItem
    UseItem,          // C# UseItem
    DropItem,         // C# DropItem
    DropGold,         // C# DropGold
    PickUp,           // C# PickUp
};

// 战斗系统 (6)
use mir2_shared::packets::client::combat::{
    Attack,           // C# Attack
    RangeAttack,      // C# RangeAttack
    Harvest,          // C# Harvest
    Magic,            // C# Magic
    SpellToggle,      // C# SpellToggle
    MagicKey,         // C# MagicKey
};
```

**状态**: ⚠️ 部分移植 (34/100+)
**位置**: `SharedRust/src/packets/client/`

#### 3.3 服务器数据包 (ServerPackets)

| C# 类别 | Rust 模块 | 状态 |
|--------|----------|------|
| 连接响应 | `packets/server/connection.rs` | ✅ 部分实现 |
| 账户响应 | `packets/server/account.rs` | ✅ 部分实现 |
| 角色响应 | `packets/server/character.rs` | ✅ 部分实现 |
| 地图数据 | `packets/server/map.rs` | ✅ 部分实现 |
| 战斗响应 | `packets/server/combat.rs` | ✅ 部分实现 |
| 物品响应 | `packets/server/item.rs` | ✅ 部分实现 |
| ... | ... | ⚠️ 持续移植中 |

**状态**: ⚠️ 部分移植,核心数据包已实现
**位置**: `SharedRust/src/packets/server/`

---

### 4. 数据结构 (Data)

#### 4.1 物品相关

| C# (Shared/Data/) | Rust (SharedRust/src/data/) |
|------------------|---------------------------|
| `ItemInfo` | `item.rs::ItemInfo` |
| `UserItem` | `item.rs::UserItem` |
| `GameShopItem` | `item.rs::GameShopItem` |
| `ItemRentalInformation` | `item.rs::ItemRentalInformation` |

```rust
// C#
// var item = new UserItem { UniqueID = 123, ... };

// Rust
use mir2_shared::UserItem;
let item = UserItem {
    unique_id: 123,
    // ...
};
```

#### 4.2 客户端数据

| C# | Rust |
|----|------|
| `SelectInfo` | `client_data.rs::SelectInfo` |
| `ClientMagic` | `client_data.rs::ClientMagic` |
| `ClientBuff` | `client_data.rs::ClientBuff` |
| `ClientMapInfo` | `client_data.rs::ClientMapInfo` |
| `ClientFriend` | `client_data.rs::ClientFriend` |
| `ClientQuestInfo` | `client_data.rs::ClientQuestInfo` |
| `GuildMember` | `client_data.rs::GuildMember` |

#### 4.3 统计数据

| C# | Rust |
|----|------|
| `BaseStats` | `stats.rs::BaseStats` |
| `Stats` | `stats.rs::Stats` |

```rust
// C#
// var stats = new Stats();
// stats[Stat.HP] = 100;

// Rust
use mir2_shared::{Stats, Stat};
let mut stats = Stats::default();
stats.set(Stat::HP, 100);
```

**状态**: ✅ 完整移植
**位置**: `SharedRust/src/data/`

---

### 5. 工具函数 (Functions/Utilities)

#### 5.1 方向和几何函数

| C# (Functions/Functions.cs) | Rust (utils/direction.rs) |
|----------------------------|---------------------------|
| `DirectionFromPoint(source, dest)` | `direction_from_point(source, dest)` |
| `PreviousDir(dir)` | `previous_dir(dir)` |
| `NextDir(dir)` | `next_dir(dir)` |
| `ReverseDirection(dir)` | `reverse_direction(dir)` |
| `ShiftDirection(dir, steps)` | `shift_direction(dir, steps)` |
| `PointMove(p, dir, distance)` | `point_move(p, dir, distance)` |
| `Left(p, dir)` | `left_point(p, dir)` |
| `Right(p, dir)` | `right_point(p, dir)` |
| `MaxDistance(p1, p2)` | `max_distance(p1, p2)` |
| `InRange(a, b, range)` | `in_range(a, b, range)` |
| `FacingEachOther(...)` | `facing_each_other(...)` |

```rust
// C#
// MirDirection dir = Functions.DirectionFromPoint(source, target);
// Point moved = Functions.PointMove(p, dir, 5);

// Rust
use mir2_shared::utils::*;
let dir = direction_from_point(source, target);
let moved = point_move(p, dir, 5);
```

**状态**: ✅ 核心函数已移植 (11个)
**位置**: `SharedRust/src/utils/direction.rs`

#### 5.2 Point 扩展方法

| C# (Functions/Functions.cs) | Rust (map.rs) |
|----------------------------|---------------|
| `p1.Add(p2)` | `p1.add(p2)` 或 `p1 + p2` |
| `p1.Subtract(p2)` | `p1.subtract(p2)` 或 `p1 - p2` |
| `p.Offset(x, y)` | `p.offset(x, y)` |
| `PointToString(p)` | `p.to_string()` 或 `format!("{}", p)` |
| `TryParse(str, out p)` | `str.parse::<Point>()` |

```rust
// C#
// Point sum = p1.Add(p2);
// Point diff = p1.Subtract(p2);
// string s = Functions.PointToString(p);

// Rust
use mir2_shared::Point;
let sum = p1 + p2;          // 或 p1.add(p2)
let diff = p1 - p2;         // 或 p1.subtract(p2)
let s = p.to_string();      // 或 format!("{}", p)
let p: Point = "10, 20".parse().unwrap();
```

**状态**: ✅ 完整移植
**位置**: `SharedRust/src/map.rs`

---

### 6. 不需要移植的模块

#### 6.1 Extensions/ExtensionMethods.cs

**原因**: Rust 标准库已提供更好的替代

| C# | Rust 替代 |
|----|---------|
| `ValueOrDefault<T>()` | `Option::unwrap_or_default()` |
| `Shuffle<T>()` | `rand::seq::SliceRandom::shuffle()` |

```rust
// C# ValueOrDefault
// T value = obj.ValueOrDefault<T>();

// Rust
let value = option.unwrap_or_default();

// C# Shuffle
// list.Shuffle();

// Rust
use rand::seq::SliceRandom;
let mut rng = rand::thread_rng();
list.shuffle(&mut rng);
```

#### 6.2 Functions/IniReader.cs

**原因**: 使用 Rust 配置库

| C# | Rust 替代 |
|----|---------|
| `InIReader` | `ini` crate 或 `config` crate |

```rust
// Rust 使用 ini crate
use ini::Ini;
let conf = Ini::load_from_file("config.ini")?;
let value = conf.get_from(Some("section"), "key");
```

#### 6.3 Functions/RegexFunctions.cs

**原因**: 使用 Rust regex crate (优先级低,按需实现)

```rust
// Rust
use regex::Regex;
let re = Regex::new(r"<(.*?/.*?)>").unwrap();
```

#### 6.4 Helpers/FileIO.cs

**原因**: Rust 标准库已足够

```rust
// Rust
use std::fs;
use std::process::Command;
let contents = fs::read_to_string("file.txt")?;
Command::new("notepad.exe").arg("file.txt").spawn()?;
```

#### 6.5 Language.cs

**原因**: 客户端 UI 文本,不属于共享网络协议

**建议**: 在 ClientRust 中使用 `fluent` 或 `gettext` 实现本地化

```rust
// ClientRust 中使用 fluent-rs
use fluent::{FluentBundle, FluentResource};
// 实现本地化
```

---

## 🔧 二进制序列化工具 (Binary)

### C# → Rust 映射

| C# | Rust (binary.rs) |
|----|-----------------|
| `BinaryReader.ReadString()` | `read_dotnet_string()` |
| `BinaryWriter.Write(string)` | `write_dotnet_string()` |
| `BinaryReader.ReadBoolean()` | `ReadBytesExt::read_u8()` |
| `BinaryReader.ReadInt32()` | `ReadBytesExt::read_i32::<LittleEndian>()` |

```rust
// Rust
use mir2_shared::binary::{read_dotnet_string, write_dotnet_string};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

let s = read_dotnet_string(&mut reader)?;
write_dotnet_string(&mut writer, &s)?;
let value = reader.read_i32::<LittleEndian>()?;
```

**状态**: ✅ 完整移植
**位置**: `SharedRust/src/binary.rs`

---

## 📦 如何在 ClientRust 中使用

### 1. 添加依赖

在 `ClientRust/Cargo.toml` 中:

```toml
[dependencies]
mir2_shared = { path = "../SharedRust" }
```

### 2. 导入模块

```rust
// 导入常用类型
use mir2_shared::{
    // 枚举
    MirDirection, MirClass, MirGender, Spell, ItemType,
    // 数据结构
    Point, UserItem, ItemInfo, Stats,
    // 全局常量
    MAX_LEVEL, MAX_BAG_WEIGHT,
    // 工具函数
    utils::*,
};

// 导入数据包
use mir2_shared::packets::client::connection::ClientVersion;
use mir2_shared::packets::client::account::Login;
use mir2_shared::packets::base::PacketMessage;

// 导入数据类型
use mir2_shared::{SelectInfo, ClientMagic, ClientBuff};
```

### 3. 使用示例

```rust
// 创建和发送登录数据包
let login = Login {
    username: "player".to_string(),
    password: "pass123".to_string(),
};
let packet_data = login.write_packet()?;
// 发送 packet_data...

// 方向和移动计算
let player_pos = Point::new(100, 100);
let target_pos = Point::new(105, 105);
let direction = direction_from_point(player_pos, target_pos);
let new_pos = point_move(player_pos, direction, 1);

// 检查距离
if in_range(player_pos, target_pos, 10) {
    // 在攻击范围内
}

// 使用 Point 运算符
let offset = Point::new(5, 3);
let moved = player_pos + offset;
```

---

## 🧪 测试覆盖

SharedRust 包含全面的单元测试:

```bash
cd SharedRust
cargo test
```

**测试统计**:
- ✅ 53+ 单元测试
- ✅ 覆盖所有核心功能
- ✅ 包含边界情况测试

---

## 📊 移植进度

### 总体进度: ~70%

| 模块 | 进度 | 状态 |
|-----|------|------|
| 枚举 (Enums) | 100% | ✅ 完成 |
| 全局常量 (Globals) | 100% | ✅ 完成 |
| 数据结构 (Data) | 100% | ✅ 完成 |
| 工具函数 (Utils) | 100% | ✅ 完成 |
| Point 操作 | 100% | ✅ 完成 |
| 数据包基础 | 100% | ✅ 完成 |
| 客户端数据包 | ~34% | ⚠️ 进行中 |
| 服务器数据包 | ~40% | ⚠️ 进行中 |

### 待实现客户端数据包 (~70个)

- NPC 交互 (10+ packets)
- 交易系统 (5 packets)
- 组队系统 (4 packets)
- 公会系统 (9 packets)
- 英雄系统 (6 packets)
- 邮件系统 (7 packets)
- 好友系统 (4 packets)
- 任务系统 (4 packets)
- 市场系统 (7 packets)
- 其他系统 (14+ packets)

**按需实现**: 可根据 ClientRust 的开发进度按需实现缺失的数据包

---

## 🔄 命名约定对照

### C# → Rust 命名转换规则

| C# 约定 | Rust 约定 | 示例 |
|---------|----------|------|
| PascalCase (类型) | PascalCase (类型) | `ItemInfo` → `ItemInfo` |
| PascalCase (方法) | snake_case (函数) | `DirectionFromPoint` → `direction_from_point` |
| PascalCase (属性) | snake_case (字段) | `UniqueID` → `unique_id` |
| UPPER_CASE (常量) | UPPER_CASE (常量) | `MaxLevel` → `MAX_LEVEL` |
| camelCase (局部变量) | snake_case | `itemCount` → `item_count` |

### 类型对照

| C# 类型 | Rust 类型 |
|---------|----------|
| `byte` | `u8` |
| `short` | `i16` |
| `int` | `i32` |
| `long` | `i64` |
| `bool` | `bool` |
| `string` | `String` |
| `byte[]` | `Vec<u8>` |
| `List<T>` | `Vec<T>` |
| `Dictionary<K,V>` | `HashMap<K,V>` |

---

## 📝 常见问题 (FAQ)

### Q1: 如何判断某个 C# 类在 Rust 中的位置?

**A**: 参考上面的模块对应表,或使用以下规则:
- 枚举类型 → `enums.rs`
- 数据包 → `packets/client/` 或 `packets/server/`
- 数据结构 → `data/`
- 工具函数 → `utils/`
- Point 相关 → `map.rs`

### Q2: 某个 C# 数据包还没有 Rust 实现怎么办?

**A**: 
1. 查看本文档的"待实现数据包"列表
2. 参考已实现的数据包格式
3. 在相应模块中添加新数据包结构
4. 实现 `PacketMessage` trait
5. 添加到 `mod.rs` 导出

### Q3: 为什么有些 C# 模块没有移植?

**A**: 
- **Extensions**: Rust 标准库已有更好的实现
- **IniReader**: 使用 Rust 生态的配置库
- **Language**: 属于客户端 UI,不是网络协议共享部分
- **FileIO**: Rust std::fs 已足够

### Q4: 如何查找某个函数的 Rust 版本?

**A**: 
1. 在本文档搜索 C# 函数名
2. 查看"详细模块映射"章节
3. 参考使用示例代码

### Q5: 数据包序列化格式是否兼容?

**A**: ✅ 是的! Rust 实现完全兼容 C# 的二进制格式:
- 相同的字节序 (Little Endian)
- 相同的字符串编码 (.NET UTF-8 with length prefix)
- 相同的数据包结构

---

## 🔗 相关资源

- **SharedRust 源码**: `SharedRust/src/`
- **单元测试**: `SharedRust/src/` (各模块 `#[cfg(test)]` 部分)
- **移植报告**: `SharedRust/MIGRATION_REPORT.md`
- **Rust 文档**: 运行 `cargo doc --open` 查看 API 文档

---

## 📅 版本历史

- **v0.1.0** (2025-10-02): 初始版本
  - ✅ 核心枚举和数据结构
  - ✅ 基础数据包系统
  - ✅ 34 个客户端数据包
  - ✅ 工具函数 (方向、几何)
  - ✅ Point 扩展

---

## 🎯 下一步计划

1. **完善客户端数据包** (按 ClientRust 需求优先级):
   - NPC 交互数据包
   - 交易系统数据包
   - 组队/公会数据包

2. **完善服务器数据包** (按 ServerRust 需求):
   - 补充缺失的响应数据包

3. **文档改进**:
   - 添加更多使用示例
   - 提供迁移指南

4. **性能优化**:
   - 数据包零拷贝优化
   - 序列化性能提升

---

**如有问题或需要添加新功能,请参考已实现的代码模式或查阅本文档。** 🚀

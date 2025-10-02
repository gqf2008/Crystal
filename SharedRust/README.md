# SharedRust - Mir2 共享库 (Rust 版本)

![Build Status](https://img.shields.io/badge/build-passing-brightgreen)
![Tests](https://img.shields.io/badge/tests-53%20passed-success)
![Coverage](https://img.shields.io/badge/coverage-~70%25-yellow)

**SharedRust** 是传奇2 (Legend of Mir 2) 游戏的 Rust 共享库,移植自 C# Shared 项目。提供客户端和服务器之间通信所需的数据结构、数据包定义、枚举类型和工具函数。

## 📦 项目结构

```
SharedRust/
├── src/
│   ├── lib.rs              # 主模块导出
│   ├── binary.rs           # 二进制序列化工具
│   ├── enums.rs            # 枚举类型 (100+ 枚举)
│   ├── globals.rs          # 全局常量和函数
│   ├── map.rs              # 2D Point 结构体
│   ├── data/               # 数据结构
│   │   ├── client_data.rs  # 客户端数据
│   │   ├── item.rs         # 物品相关
│   │   ├── stats.rs        # 统计数据
│   │   ├── notice.rs       # 公告
│   │   └── shared_data.rs  # 共享数据
│   ├── packets/            # 网络数据包
│   │   ├── base.rs         # 数据包基础设施
│   │   ├── ids.rs          # 数据包 ID
│   │   ├── client/         # 客户端数据包 (34+ 已实现)
│   │   │   ├── connection.rs
│   │   │   ├── account.rs
│   │   │   ├── character.rs
│   │   │   ├── movement.rs
│   │   │   ├── chat.rs
│   │   │   ├── item.rs
│   │   │   └── combat.rs
│   │   └── server/         # 服务器数据包 (部分实现)
│   └── utils/              # 工具函数
│       ├── mod.rs
│       └── direction.rs    # 方向和几何计算
├── Cargo.toml
├── README.md               # 本文件
├── PORTING_GUIDE.md        # 📘 详细移植指南 ★★★
├── QUICK_REFERENCE.md      # ⚡ 快速参考 ★★★
└── MIGRATION_REPORT.md     # 📊 移植报告
```

## 🚀 快速开始

### 添加依赖

在您的 `Cargo.toml` 中:

```toml
[dependencies]
mir2_shared = { path = "../SharedRust" }
```

### 基础使用

```rust
use mir2_shared::{
    // 枚举
    MirDirection, MirClass, MirGender,
    // 数据结构
    Point, UserItem, Stats,
    // 常量
    MAX_LEVEL,
    // 工具函数
    utils::*,
};

// 创建点
let pos = Point::new(100, 100);

// 计算方向
let target = Point::new(105, 100);
let dir = direction_from_point(pos, target); // MirDirection::Right

// 移动
let new_pos = point_move(pos, dir, 5); // Point(105, 100)
```

### 数据包示例

```rust
use mir2_shared::packets::client::account::Login;
use mir2_shared::packets::base::PacketMessage;

// 创建登录数据包
let packet = Login {
    username: "player".to_string(),
    password: "pass123".to_string(),
};

// 序列化为字节
let bytes = packet.write_packet()?;

// 发送到服务器...
```

## 📚 文档

| 文档 | 说明 | 适合 |
|-----|------|------|
| **[PORTING_GUIDE.md](PORTING_GUIDE.md)** | 📘 完整的 C# → Rust 模块映射指南 | 所有开发者 ★★★ |
| **[QUICK_REFERENCE.md](QUICK_REFERENCE.md)** | ⚡ 快速参考和常用代码片段 | 快速查找 ★★★ |
| **[MIGRATION_REPORT.md](MIGRATION_REPORT.md)** | 📊 移植进度和技术细节 | 了解项目状态 |
| **API 文档** | 运行 `cargo doc --open` | API 细节查询 |

## ✨ 主要特性

### ✅ 已完整实现

- **枚举系统** (100+ 枚举类型)
  - `MirDirection`, `MirClass`, `MirGender`, `ItemType`, `Spell` 等

- **数据结构** (完整)
  - `Point`, `UserItem`, `ItemInfo`, `Stats`, `SelectInfo` 等

- **工具函数** (11个核心函数)
  - 方向计算: `direction_from_point()`, `next_dir()`, `previous_dir()`
  - 点移动: `point_move()`, `left_point()`, `right_point()`
  - 距离检查: `max_distance()`, `in_range()`, `facing_each_other()`

- **Point 操作** (完整)
  - 算术运算: `+`, `-`, `add()`, `subtract()`
  - 字符串转换: `to_string()`, `FromStr` trait
  - 二进制序列化: `read_from()`, `write_to()`

- **客户端数据包** (34个)
  - ✅ 连接管理 (3): ClientVersion, Disconnect, KeepAlive
  - ✅ 账户管理 (4): NewAccount, ChangePassword, Login, StartGame
  - ✅ 角色管理 (3): NewCharacter, DeleteCharacter, LogOut
  - ✅ 移动系统 (3): Turn, Walk, Run
  - ✅ 聊天系统 (3): Chat, Inspect, Observe
  - ✅ 物品管理 (12): MoveItem, EquipItem, UseItem, DropItem 等
  - ✅ 战斗系统 (6): Attack, RangeAttack, Magic, SpellToggle 等

### ⚠️ 部分实现

- **服务器数据包** (~40% 完成)
  - 核心响应数据包已实现
  - 按需继续添加

- **客户端数据包** (~34% 完成)
  - 待实现: NPC 交互、交易、公会、英雄等系统 (~70个)

## 🧪 测试

```bash
# 运行所有测试
cargo test

# 运行测试并显示输出
cargo test -- --nocapture

# 查看测试覆盖
cargo test --verbose
```

**测试统计**:
- ✅ **53+ 单元测试**
- ✅ 覆盖所有核心功能
- ✅ 包含边界情况测试

## 📖 使用指南

### 在 ClientRust 中使用

```rust
use mir2_shared::{
    MirDirection, Point, utils::*,
    packets::client::{
        account::Login,
        movement::Walk,
        combat::Attack,
    },
};

// 1. 登录
let login = Login {
    username: "player".to_string(),
    password: "pass".to_string(),
};
network.send(login.write_packet()?);

// 2. 移动
let walk = Walk {
    direction: MirDirection::Right,
};
network.send(walk.write_packet()?);

// 3. 攻击
let attack = Attack {
    direction: MirDirection::Right,
    spell: Spell::None,
};
network.send(attack.write_packet()?);
```

### 方向和几何计算

```rust
use mir2_shared::{Point, MirDirection, utils::*};

let player = Point::new(100, 100);
let monster = Point::new(105, 102);

// 计算方向
let direction = direction_from_point(player, monster);

// 检查距离
if in_range(player, monster, 3) {
    // 在攻击范围内
    let attack_dir = direction_from_point(player, monster);
}

// 预测移动
let next_pos = point_move(player, MirDirection::Right, 1);

// 检查是否面对面
if facing_each_other(
    MirDirection::Right, player,
    MirDirection::Left, monster
) {
    // 面对面交互
}
```

## 🔄 C# Shared 对应关系

| C# Shared | SharedRust | 状态 |
|-----------|-----------|------|
| `Enums.cs` | `enums.rs` | ✅ 100% |
| `Globals.cs` | `globals.rs` | ✅ 100% |
| `BaseStats.cs` | `data/stats.rs` | ✅ 100% |
| `Data/*` | `data/*` | ✅ 100% |
| `Packet.cs` | `packets/base.rs` | ✅ 100% |
| `ClientPackets.cs` | `packets/client/*` | ⚠️ ~34% |
| `ServerPackets.cs` | `packets/server/*` | ⚠️ ~40% |
| `Functions.cs` | `utils/direction.rs` | ✅ 核心完成 |
| `Point` | `map.rs` | ✅ 100% |

详细对应关系请参考 [PORTING_GUIDE.md](PORTING_GUIDE.md)

## 🛠️ 开发

### 构建

```bash
# 开发构建
cargo build

# 发布构建
cargo build --release

# 检查代码
cargo check

# 运行 clippy
cargo clippy
```

### 生成文档

```bash
# 生成并打开 API 文档
cargo doc --open

# 只生成文档
cargo doc
```

## 📊 项目状态

### 移植进度: ~70%

| 模块 | 进度 | 状态 |
|-----|------|------|
| 枚举 | 100% | ✅ 完成 |
| 数据结构 | 100% | ✅ 完成 |
| 工具函数 | 100% | ✅ 完成 |
| 数据包基础 | 100% | ✅ 完成 |
| 客户端数据包 | ~34% | ⚠️ 进行中 |
| 服务器数据包 | ~40% | ⚠️ 进行中 |

### 编译状态

```
✅ 编译: 成功 (0 错误)
⚠️ 警告: 21 个 (未使用函数)
✅ 测试: 53 个全部通过
```

## 🎯 下一步计划

1. **完善客户端数据包**
   - [ ] NPC 交互 (10+ packets)
   - [ ] 交易系统 (5 packets)
   - [ ] 组队/公会 (13 packets)
   - [ ] 英雄系统 (6 packets)

2. **完善服务器数据包**
   - [ ] 补充缺失的响应数据包

3. **性能优化**
   - [ ] 数据包零拷贝优化
   - [ ] 序列化性能提升

## 🤝 贡献

欢迎贡献! 特别是:
- 添加缺失的数据包实现
- 改进文档和示例
- 性能优化
- Bug 修复

## 📄 许可证

本项目遵循与原 C# 项目相同的许可证。

## 🔗 相关项目

- **C# Shared**: `../Shared/` - 原始 C# 共享库
- **ClientRust**: `../ClientRust/` - Rust 客户端
- **ServerRust**: 待开发

## 📞 联系方式

如有问题或建议,请:
1. 查阅 [PORTING_GUIDE.md](PORTING_GUIDE.md)
2. 查看 [QUICK_REFERENCE.md](QUICK_REFERENCE.md)
3. 运行 `cargo doc --open` 查看 API 文档

---

**最后更新**: 2025年10月2日
**版本**: 0.1.0
**状态**: 开发中 (Development)

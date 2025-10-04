# Crystal - Rust Client

一个 Rust 实现的传奇客户端 (Legend of Mir 2 Client in Rust)

## 📚 文档导航

### 核心文档 (推荐阅读顺序)
1. **[ARCHITECTURE_CORRECT.md](./ARCHITECTURE_CORRECT.md)** - ⭐⭐⭐ **必读!** 正确的架构对应关系
2. **[ARCHITECTURE_CORRECTION.md](./ARCHITECTURE_CORRECTION.md)** - ⭐⭐ 架构理解修正说明
3. **[PROTOCOL_QUICK_REFERENCE.md](./PROTOCOL_QUICK_REFERENCE.md)** - ⭐ 快速参考指南
4. **[PROTOCOL_MAPPING.md](./PROTOCOL_MAPPING.md)** - Rust 与 C# 详细对应关系 (部分需更新)
5. **[ARCHITECTURE_COMPARISON.md](./ARCHITECTURE_COMPARISON.md)** - 架构对比与可视化图表 (部分需更新)

### 开发文档
- **[PHASE_A_TESTING.md](./PHASE_A_TESTING.md)** - Phase A 测试验证报告 (已完成 ✅)
- **[PHASE_B_DEVELOPMENT_PLAN.md](./PHASE_B_DEVELOPMENT_PLAN.md)** - Phase B 开发计划 (进行中 ⏳)
- **[PHASE_A_B_STATUS.md](./PHASE_A_B_STATUS.md)** - 当前状态总结

### 重构历史
- **[REFACTORING_COMPLETE.md](./REFACTORING_COMPLETE.md)** - Phase 1B 重构完成报告
- **[SESSION_CHECKLIST.md](./SESSION_CHECKLIST.md)** - 项目总体进度清单

---

## 🎯 项目概述

### 什么是 ClientRust?

ClientRust 是传奇游戏 (Legend of Mir 2) 的 Rust 重写版客户端,是 Crystal 项目的一部分。

**重要**: Crystal 项目分为两层架构:
- **SharedRust** → 共享协议定义层 (对应 C# Shared 项目)
- **ClientRust** → 客户端实现层 (对应 C# Client 项目)

**核心对应关系**:
```
SharedRust/ (协议定义)     ↔  C# Shared/ (协议定义)
    ├─ packet_ids.rs           ├─ ServerPackets.cs
    ├─ client_packets.rs       ├─ ClientPackets.cs
    └─ enums.rs                └─ Enums.cs

ClientRust/ (客户端实现)   ↔  C# Client/ (客户端实现)
    ├─ protocol.rs             ├─ MirNetwork/Network.cs
    ├─ protocol_packets/       └─ MirScenes/GameScene.cs
    ├─ ui.rs (部分)
    └─ state.rs (部分)
```

### 为什么用 Rust 重写?

| 优势 | 说明 |
|------|------|
| 🔒 **类型安全** | 编译时保证协议解析正确,零运行时类型转换错误 |
| ⚡ **高性能** | 零成本抽象,无GC,游戏性能更稳定 |
| 🧩 **模块化** | 代码量减少 70%,可维护性大幅提升 |
| 🔧 **并行开发** | 模块独立,多人协作无冲突 |
| 📦 **现代化** | Cargo 包管理,生态系统丰富 |

---

## 📂 项目结构

```
ClientRust/
├── src/
│   ├── main.rs                    # 程序入口
│   ├── protocol.rs                # 协议路由 (4,366行,目标1,500行)
│   ├── protocol_packets/          # 模块化协议包
│   │   └── packets/
│   │       ├── account.rs         # 账户系统 (4个)
│   │       ├── npc.rs             # NPC系统 (9个)
│   │       ├── magic.rs           # 魔法系统 (4个)
│   │       ├── item.rs            # 物品系统 (10个)
│   │       ├── player.rs          # 玩家系统 (9个)
│   │       ├── object.rs          # 对象系统 (4个)
│   │       ├── group.rs           # 组队系统 (3个)
│   │       ├── guild.rs           # 公会系统 (3个)
│   │       ├── hero.rs            # 英雄系统 (5个)
│   │       └── quest.rs           # 任务系统 (2个)
│   ├── ui.rs                      # UI处理 (待完善)
│   ├── state.rs                   # 游戏状态 (待完善)
│   └── ...
└── docs/                          # 文档 (*.md)
```

---

## 🚀 快速开始

### 构建项目

```powershell
# 调试版本
cargo build

# 发布版本 (优化编译)
cargo build --release
```

### 运行项目

```powershell
# 调试模式
cargo run

# 发布模式
cargo run --release
```

### 开发工具

```powershell
# 代码检查
cargo check

# 运行测试
cargo test

# 代码格式化
cargo fmt

# Clippy 静态分析
cargo clippy
```

---

## 📊 开发进度

### Phase A: 重构验证 ✅ (已完成)

- ✅ 编译测试通过
- ✅ 路由完整性验证 (52个路由)
- ✅ 模块函数存在性验证 (53个函数)
- ✅ Clippy 检查通过 (零警告)
- ✅ 代码格式化完成

### Phase B: 协议模块化 ⏳ (进行中)

**当前状态** (2025年10月2日):
- ✅ 已模块化: 53个数据包 (18.6%)
- ⏳ 待模块化: 49个数据包 (17.2%) - 已实现在 protocol.rs 中
- ❌ 未实现: 183个数据包 (64.2%)
- **总目标**: 285个数据包

**代码统计**:
- protocol.rs: 4,366 行 (目标: 1,500 行)
- 模块文件: 10个,总计 ~1,400 行
- 平均模块大小: 140 行

### Phase C: UI/State 层 ❌ (未开始)

对应 C# 的 `GameScene.cs` (12,000 行),需要实现:
- 数据包处理逻辑 (100+ 个方法)
- UI 对话框管理
- 游戏对象管理
- 状态更新逻辑

---

## 📈 架构对比

### 共享层对比 (Shared ↔ SharedRust)

| 指标 | C# Shared | Rust SharedRust |
|------|-----------|-----------------|
| **总代码量** | ~17,261 行 | ~6,442 行 (-63%) |
| **文件数** | 22 个 | 11 个 |
| **协议定义** | ServerPackets.cs (6,708行) | packet_ids.rs (~300行) |
| **类型安全** | 运行时 | 编译时 ✅ |

### 客户端层对比 (Client ↔ ClientRust)

| 指标 | C# Client | Rust ClientRust |
|------|-----------|-----------------|
| **协议处理** | Network.cs (257行) | protocol.rs (4,366行,含102个解析函数) |
| **协议解析** | 嵌入在类中 | protocol_packets/ (10模块,53函数) |
| **游戏场景** | GameScene.cs (12,297行) | ui.rs + state.rs (部分) |
| **模块化** | 困难 ❌ | 容易 ✅ |
| **并行开发** | 冲突频繁 ❌ | 无冲突 ✅ |

详见 [ARCHITECTURE_CORRECT.md](./ARCHITECTURE_CORRECT.md)

---

## 🔧 开发指南

### 查找 Rust 与 C# 对应代码

参考 [PROTOCOL_QUICK_REFERENCE.md](./PROTOCOL_QUICK_REFERENCE.md) 的"查找对应关系"章节。

**快速示例**:

| C# 代码 | Rust 代码 | 说明 |
|---------|-----------|------|
| `ServerPackets.NPCSell` | `protocol_packets::packets::npc::NPCSell` | 数据包定义 |
| `GameScene::NPCSell(p)` | `ui.rs::handle_npc_sell()` (待实现) | 业务处理 |

### 添加新数据包

1. 在对应模块添加结构体和解析函数
2. 在 `protocol.rs` 添加路由
3. 在 `ui.rs` 添加处理逻辑

详见开发文档。

---

## 🤝 贡献

1. 阅读 [PROTOCOL_QUICK_REFERENCE.md](./PROTOCOL_QUICK_REFERENCE.md)
2. 查看 [PHASE_B_DEVELOPMENT_PLAN.md](./PHASE_B_DEVELOPMENT_PLAN.md)
3. 选择模块开发
4. 提交 PR

---

## 📝 相关项目

- **C# Client**: `Crystal/Client/` - 原始客户端
- **C# Server**: `Crystal/Server/` - 服务器端
- **Shared**: `Crystal/Shared/` - 共享协议

---

## 📄 许可证

详见 LICENSE 文件

---

**最后更新**: 2025年10月2日  
**项目状态**: 🟡 开发中 (协议层 35% 完成)

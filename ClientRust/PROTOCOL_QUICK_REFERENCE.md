# Protocol 模块快速参考指南

**快速理解 Rust ClientRust 的 protocol 模块与 C# Client 的对应关系**

---

## 🎯 一句话总结

> **Rust ClientRust 的 `protocol` 模块 = C# Client 的 `Shared/ServerPackets.cs` (协议定义) + `Client/MirNetwork/Network.cs` (协议解析)**

---

## 📂 文件对应速查表

| Rust 文件 | C# 文件 | 行数对比 | 说明 |
|-----------|---------|---------|------|
| `protocol.rs` | `ServerPackets.cs` | 4,366 vs 6,700 | 主协议路由 |
| `protocol_packets/packets/npc.rs` | `ServerPackets.cs` (NPC相关类) | 140 vs ~300 | NPC系统 |
| `protocol_packets/packets/item.rs` | `ServerPackets.cs` (Item相关类) | 280 vs ~600 | 物品系统 |
| `protocol_packets/packets/magic.rs` | `ServerPackets.cs` (Magic相关类) | 110 vs ~200 | 魔法系统 |
| `protocol_packets/packets/player.rs` | `ServerPackets.cs` (Player相关类) | 290 vs ~400 | 玩家系统 |
| `protocol_packets/packets/object.rs` | `ServerPackets.cs` (Object相关类) | 100 vs ~300 | 对象系统 |
| `protocol_packets/packets/group.rs` | `ServerPackets.cs` (Group相关类) | 70 vs ~100 | 组队系统 |
| `protocol_packets/packets/guild.rs` | `ServerPackets.cs` (Guild相关类) | 110 vs ~200 | 公会系统 |
| `protocol_packets/packets/hero.rs` | `ServerPackets.cs` (Hero相关类) | 130 vs ~250 | 英雄系统 |
| `protocol_packets/packets/quest.rs` | `ServerPackets.cs` (Quest相关类) | 40 vs ~100 | 任务系统 |
| `ui.rs` (待完善) | `GameScene.cs` | ? vs 12,000 | 业务逻辑 |

**总计**: Rust ~5,900 行 vs C# ~19,000 行 (减少 70%)

---

## 🔍 关键概念对应

### 1. 数据包定义

**C#**:
```csharp
// Shared/ServerPackets.cs
public sealed class NPCSell : Packet {
    public override short Index => (short)ServerPacketIds.NPCSell;
    
    protected override void ReadPacket(BinaryReader reader) {
        // 解析字段
    }
}
```

**Rust**:
```rust
// protocol_packets/packets/npc.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NPCSell;

pub(crate) fn parse_npc_sell(_payload: &[u8]) -> Result<NPCSell, String> {
    Ok(NPCSell)
}
```

### 2. 数据包路由

**C#**:
```csharp
// GameScene.cs
protected override void ProcessPacket(Packet p) {
    switch ((ServerPacketIds)p.Index) {
        case ServerPacketIds.NPCSell:
            NPCSell((S.NPCSell)p);  // 运行时类型转换
            break;
        // ... 100+ cases
    }
}
```

**Rust**:
```rust
// protocol.rs
pub fn parse_server_message(data: &[u8]) -> Result<ServerMessage, String> {
    match packet_id {
        Ok(ServerPacketId::NPCSell) => {
            let result = packets::npc::parse_npc_sell(&payload)?;
            Ok(ServerMessage::NPCSell(result))  // 编译时类型安全
        }
        // ... 50+ routes
    }
}
```

### 3. 数据包处理

**C#**:
```csharp
// GameScene.cs
private void NPCSell(S.NPCSell p) {
    // 打开商店对话框
    NPCDialog.ShowSellDialog();
}

private void ObjectPlayer(S.ObjectPlayer p) {
    // 创建玩家对象
    PlayerObject player = new PlayerObject(p.ObjectID);
    player.Load(p);
}
```

**Rust** (待实现):
```rust
// ui.rs (需要实现)
pub fn handle_server_message(msg: ServerMessage) {
    match msg {
        ServerMessage::NPCSell(_) => {
            // 打开商店UI
        }
        ServerMessage::ObjectPlayer(data) => {
            // 创建玩家对象
        }
        // ...
    }
}
```

---

## 📊 数据包分类速查

### 已模块化 (53个)

| 模块 | 数据包数量 | 对应 C# 类 |
|------|-----------|-----------|
| account.rs | 4 | ChangePassword, LoginSuccess, ... |
| npc.rs | 9 | NPCSell, NPCRepair, NPCStorage, ... |
| magic.rs | 4 | Magic, MagicDelay, NewMagic, ... |
| item.rs | 10 | ObjectItem, GainedItem, MoveItem, ... |
| player.rs | 9 | NewCharacter, StartGame, UserInformation, ... |
| object.rs | 4 | ObjectPlayer, ObjectMonster, ObjectRemove, ... |
| group.rs | 3 | GroupInvite, AddMember, DeleteMember |
| guild.rs | 3 | GuildInvite, GuildStatus, GuildStorage |
| hero.rs | 5 | NewHero, HeroInformation, HeroLevelChanged, ... |
| quest.rs | 2 | ShareQuest, CompleteQuest |

### 待模块化 (49个,已在 protocol.rs 中实现)

| 待创建模块 | 预计数据包 | 对应功能 |
|----------|-----------|---------|
| combat.rs | ~20 | 战斗、攻击、伤害、死亡 |
| trade.rs | ~6 | 交易请求、物品交换 |
| buff.rs | ~10 | Buff、中毒、颜色变化 |
| map.rs | ~10 | 地图切换、传送、移动 |
| chat.rs | ~3 | 聊天、私聊 |

### 未实现 (183个)

需要从零开始实现,参考 C# 的 `ServerPackets.cs` 中相应的类。

---

## 🚀 查找对应关系的方法

### 方法 1: 按数据包名称查找

**场景**: 你知道 C# 的数据包类名,想找 Rust 对应代码

**步骤**:
1. 在 C# 中找到类名,如 `ServerPackets.NPCSell`
2. 在 Rust 中搜索对应的 struct: `NPCSell`
3. 或搜索解析函数: `parse_npc_sell`

**示例**:
```bash
# 在 Rust 代码中搜索
$ grep -r "struct NPCSell" ClientRust/src/
ClientRust/src/protocol_packets/packets/npc.rs:pub struct NPCSell;

$ grep -r "fn parse_npc_sell" ClientRust/src/
ClientRust/src/protocol_packets/packets/npc.rs:pub(crate) fn parse_npc_sell
```

### 方法 2: 按功能查找

**场景**: 你知道功能(如"NPC商店"),想找对应代码

**步骤**:
1. 确定功能所属系统(NPC/物品/魔法等)
2. 在 Rust 中打开对应模块文件(如 `npc.rs`)
3. 查看该模块的所有数据包定义

**示例**:
```rust
// protocol_packets/packets/npc.rs
pub struct NPCSell;           // NPC商店
pub struct NPCRepair;         // NPC修理
pub struct NPCStorage;        // NPC仓库
// ...
```

### 方法 3: 按 PacketId 查找

**场景**: 你知道数据包的 ID,想找对应实现

**步骤**:
1. 在 Rust 的 `protocol.rs` 中搜索 `ServerPacketId::`
2. 找到对应的 match 分支
3. 查看调用的模块和函数

**示例**:
```rust
// protocol.rs
match packet_id {
    Ok(ServerPacketId::NPCSell) => 
        crate::protocol_packets::packets::npc::parse_npc_sell(&payload),
    // ...
}
```

---

## 💡 常见问题速答

### Q1: 为什么 Rust 代码行数更少?
**A**: 模块化 + 没有重复代码 + Rust 简洁的语法

### Q2: C# 的 GameScene 对应 Rust 的什么?
**A**: 对应 `ui.rs` 和 `state.rs`,但目前这两个文件功能还不完善

### Q3: Rust 如何处理运行时类型转换?
**A**: 不需要!Rust 使用枚举 `ServerMessage`,编译时保证类型安全

### Q4: 如何添加新的数据包?
**A**: 
1. 在对应模块(如 `npc.rs`)中添加 struct 和 parse 函数
2. 在 `protocol.rs` 中添加路由分支
3. 在 `ui.rs` 中添加处理逻辑

### Q5: protocol.rs 为什么还有 4,366 行?
**A**: 还有 49 个数据包未模块化 + ~100 个传统数据包,目标是减少到 ~1,500 行

---

## 📚 推荐阅读顺序

1. **新手**: 先看 `PROTOCOL_MAPPING.md` 了解整体对应关系
2. **开发**: 看本文档快速查找对应代码
3. **重构**: 看 `ARCHITECTURE_COMPARISON.md` 理解架构差异
4. **测试**: 看 `PHASE_A_TESTING.md` 了解测试方法
5. **规划**: 看 `PHASE_B_DEVELOPMENT_PLAN.md` 了解后续计划

---

## 🔗 相关文档

- [PROTOCOL_MAPPING.md](./PROTOCOL_MAPPING.md) - 详细对应关系
- [ARCHITECTURE_COMPARISON.md](./ARCHITECTURE_COMPARISON.md) - 架构对比图
- [PHASE_A_TESTING.md](./PHASE_A_TESTING.md) - 测试验证报告
- [PHASE_B_DEVELOPMENT_PLAN.md](./PHASE_B_DEVELOPMENT_PLAN.md) - 开发计划
- [REFACTORING_COMPLETE.md](./REFACTORING_COMPLETE.md) - 重构完成报告

---

**最后更新**: 2025年10月2日

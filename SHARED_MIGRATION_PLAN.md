# Shared → SharedRust 完整迁移计划

## 🔴 问题确认

**当前状态**: SharedRust 项目**不完整**，导致 ClientRust 被迫重复实现 Shared 中的模块。

**问题根源**:
- ServerPackets.cs (5,773行) - ❌ **未迁移**
- ClientPackets.cs (2,225行) - ⚠️ **部分迁移** (client_packets.rs 可能不完整)
- Enums.cs (1,835行) - ⚠️ **部分迁移** (enums.rs 可能不完整)
- Packet.cs (926行) - ✅ 已有 packet.rs
- Data/ItemData.cs (1,185行) - ⚠️ **部分迁移** (item.rs)
- Data/ClientData.cs (519行) - ✅ 已有 client_data.rs
- BaseStats.cs (171行) - ✅ 已有 stats.rs
- Language.cs (761行) - ❌ **未迁移**
- Globals.cs (50行) - ❌ **未迁移**

**影响范围**:
```
ClientRust/src/game/network/protocol/
├── game_packets.rs     ← 重复了 ServerPackets 中的部分定义
├── login_packets.rs    ← 重复了 ServerPackets 中的部分定义
├── world_packets.rs    ← 重复了 ServerPackets 中的部分定义
└── mod.rs
```

这些文件本应该直接使用 `use shared_rust::ServerPackets` 而不是重新定义！

---

## 📋 完整迁移任务清单

### **阶段1: ServerPackets (最高优先级) ⭐⭐⭐**

**目标文件**: `ServerPackets.cs` (5,773行)

**包含内容**:
- 200+ 服务器→客户端数据包定义
- 核心包类型:
  - 连接相关: Connected, ClientVersion, Disconnect, KeepAlive
  - 登录相关: LoginSuccess, LoginFailed, NewCharacter, NewCharacterSuccess
  - 玩家相关: UserInformation, UserLocation, ObjectPlayer, PlayerUpdate, PlayerInspect
  - 战斗相关: ObjectAttack, ObjectStruck, DamageIndicator, Death, ObjectDied
  - 物品相关: NewItemInfo, MoveItem, EquipItem, GainedItem, DeleteItem, **SellItem** (您当前选中的)
  - 交易相关: TradeRequest, TradeAccept, TradeGold, TradeItem, TradeConfirm, TradeCancel
  - NPC相关: ObjectNPC, NPCResponse, NPCGoods, NPCSell, NPCRepair, NPCStorage
  - 魔法相关: NewMagic, Magic, MagicDelay, MagicCast, ObjectMagic
  - 组队相关: GroupInvite, AddMember, DeleteMember, SendMemberLocation
  - 地图相关: MapChanged, TimeOfDay, ObjectTeleportOut, ObjectTeleportIn
  - 聊天相关: Chat, ObjectChat
  - 更多...

**迁移策略**:
```rust
// SharedRust/src/server_packets.rs (新建)
use crate::packet::Packet;
use crate::enums::*;
use crate::item::UserItem;
use std::net::SocketAddr;

// 1. 连接相关 (20个包)
pub struct Connected { /* ... */ }
pub struct ClientVersion { /* ... */ }
pub struct Disconnect { /* ... */ }

// 2. 登录相关 (30个包)
pub struct LoginSuccess { /* ... */ }
pub struct NewCharacter { /* ... */ }

// 3. 玩家相关 (40个包)
pub struct UserInformation { /* ... */ }
pub struct ObjectPlayer { /* ... */ }

// 4. 战斗相关 (30个包)
pub struct ObjectAttack { /* ... */ }
pub struct DamageIndicator { /* ... */ }

// 5. 物品相关 (40个包)
pub struct NewItemInfo { /* ... */ }
pub struct SellItem {  // ← 您选中的这个
    pub unique_id: u64,
    pub count: u16,
    pub success: bool,
}

// 6. NPC相关 (20个包)
// 7. 魔法相关 (20个包)
// 8-15. 其他分组...
```

**预计工作量**: 5,773行 → ~6,500行 Rust (2-3天)

---

### **阶段2: ClientPackets (高优先级) ⭐⭐**

**目标文件**: `ClientPackets.cs` (2,225行)

**包含内容**:
- 100+ 客户端→服务器数据包定义
- 核心包类型:
  - 登录: ClientVersion, ChangePassword, Login, NewCharacter, DeleteCharacter
  - 移动: Turn, Walk, Run, Jump
  - 攻击: Attack, RangeAttack
  - 聊天: Chat, ChatWhisper
  - 物品: PickUp, DropItem, MoveItem, EquipItem, RemoveItem, UseItem, SplitItem
  - NPC: CallNPC, BuyItem, SellItem, RepairItem, BuyItemBack
  - 魔法: SpellRequest, MagicKey
  - 交易: TradeRequest, TradeReply, TradeGold, TradeConfirm, TradeCancel
  - 组队: GroupSwitch, GroupInvite, GroupAcceptInvite, GroupKick
  - 公会: GuildCreate, GuildInvite, GuildKick
  - 更多...

**迁移策略**:
```rust
// SharedRust/src/client_packets.rs (扩展现有文件)
// 当前可能只有部分实现，需要完整补充
```

**预计工作量**: 2,225行 → ~2,800行 Rust (1-2天)

---

### **阶段3: Enums 完整化 (高优先级) ⭐⭐**

**目标文件**: `Enums.cs` (1,835行)

**包含内容**:
- 100+ 枚举定义
- 关键枚举:
  - MouseCursor, WeatherSetting, PanelType, BlendMode
  - DamageType, GMOptions, AwakeType, LevelEffects
  - ItemGrade, QuestType, QuestState, QuestAction
  - Monster (1000+ 怪物ID)
  - Spell (200+ 技能ID)
  - ItemType, RequiredType, RequiredClass
  - MirClass, MirGender, MirDirection
  - AttackMode, PetMode, BindMode
  - ChatType, OutputMessageType
  - ServerPacketIds (200+), ClientPacketIds (100+)
  - 更多...

**当前 enums.rs 检查**:
```bash
# 需要检查现有 enums.rs 是否完整
```

**预计工作量**: 1,835行 → ~2,200行 Rust (1天)

---

### **阶段4: 数据结构完整化 (中优先级) ⭐**

#### **4.1 ItemData.cs → item.rs** (1,185行)
- ItemInfo 结构
- UserItem 结构
- 物品属性、槽位、绑定等

#### **4.2 ClientData.cs → client_data.rs** (519行)
- SelectInfo (角色选择)
- ClientHeroInformation
- ClientObjectInformation
- ClientMagic
- 各种客户端数据结构

#### **4.3 Language.cs → language.rs** (761行) ❌ **缺失**
- GameLanguage 类
- 所有游戏文本本地化字符串
- 多语言支持

#### **4.4 Globals.cs → globals.rs** (50行) ❌ **缺失**
- 全局常量
- 版本号
- 配置常量

**预计工作量**: ~2,500行 Rust (1天)

---

### **阶段5: 辅助模块 (低优先级)**

#### **5.1 Data/GuildData.cs**
- 公会相关数据结构

#### **5.2 Data/IntelligentCreatureData.cs**
- 智能宠物数据

#### **5.3 Data/Notice.cs**
- 公告数据

#### **5.4 Functions/Functions.cs**
- 工具函数

#### **5.5 Extensions/ExtensionMethods.cs**
- 扩展方法

**预计工作量**: ~1,500行 Rust (1天)

---

## 🎯 迁移总计划

### **工作量估算**

```
阶段1: ServerPackets        ~6,500行  (2-3天)  ← 最关键
阶段2: ClientPackets        ~2,800行  (1-2天)
阶段3: Enums完整化          ~2,200行  (1天)
阶段4: 数据结构完整化       ~2,500行  (1天)
阶段5: 辅助模块             ~1,500行  (1天)
──────────────────────────────────────────
总计:                      ~15,500行  (6-8天)
```

### **推荐执行顺序**

```
第1天:  ServerPackets (1/3) - 连接、登录、玩家包
第2天:  ServerPackets (2/3) - 战斗、物品、NPC包
第3天:  ServerPackets (3/3) - 魔法、组队、剩余包
第4天:  ClientPackets (完整) - 所有客户端包
第5天:  Enums完整化 + Globals + Language
第6天:  数据结构完整化 + 测试
第7天:  辅助模块 + 集成测试
第8天:  清理ClientRust重复代码 + 文档
```

---

## 🔧 迁移后重构

### **ClientRust 清理任务**

迁移完成后，需要删除/重构 ClientRust 中的重复代码：

```rust
// ClientRust/src/game/network/protocol/ 目录重构

// ❌ 删除这些重复定义
game_packets.rs     → 使用 shared_rust::ServerPackets
login_packets.rs    → 使用 shared_rust::ServerPackets
world_packets.rs    → 使用 shared_rust::ServerPackets

// ✅ 保留这些客户端专属逻辑
mod.rs              → 保留网络协议处理逻辑
packet_handler.rs   → 保留包处理器
```

**重构方式**:
```rust
// 之前 (重复定义):
// ClientRust/src/game/network/protocol/game_packets.rs
pub struct ObjectPlayer { ... }  // ❌ 重复

// 之后 (使用SharedRust):
use shared_rust::server_packets::ObjectPlayer;  // ✅ 正确
```

---

## 📊 依赖关系分析

```
SharedRust (基础层)
    ├── Enums (枚举定义)
    ├── Packet (包基类)
    ├── Item (物品数据)
    ├── Stats (属性数据)
    ├── ServerPackets (服务器包) ← 依赖上面所有
    ├── ClientPackets (客户端包) ← 依赖上面所有
    ├── Language (本地化)
    └── Globals (全局常量)
    
ClientRust (应用层)
    └── network/protocol/
        ├── packet_handler.rs  → 使用 ServerPackets
        ├── connection.rs      → 使用 ClientPackets
        └── mod.rs             → 统一导出
```

**关键依赖**:
1. ServerPackets + ClientPackets 依赖 Enums (必须先迁移Enums)
2. 包定义依赖 Item, Stats 等数据结构
3. ClientRust 网络层依赖 SharedRust 所有包定义

---

## ✅ 验证标准

迁移完成后需满足：

### **1. 完整性验证**
```bash
# 检查所有C#包是否都有对应Rust实现
grep "public sealed class.*Packet" Shared/*.cs | wc -l  # C#包数量
grep "pub struct.*Packet" SharedRust/src/*.rs | wc -l   # Rust包数量
# 应该相等或Rust略多(因为有额外trait impl)
```

### **2. 功能验证**
- ✅ 所有 ServerPacketIds 有对应实现
- ✅ 所有 ClientPacketIds 有对应实现
- ✅ 所有枚举类型完整
- ✅ 二进制序列化/反序列化正确

### **3. 集成验证**
```rust
// ClientRust 可以直接使用
use shared_rust::server_packets::*;
use shared_rust::client_packets::*;
use shared_rust::enums::*;

// 不再有重复定义
```

### **4. 测试验证**
- ✅ 每个包有序列化测试
- ✅ 每个包有反序列化测试
- ✅ 二进制兼容性测试 (与C#服务器通信)

---

## 📖 迁移规范

### **命名规范**
```
C# PascalCase       → Rust snake_case (结构体字段)
C# PascalCase       → Rust PascalCase (类型名)
C# Enum             → Rust enum
C# sealed class     → Rust struct
```

### **类型映射**
```
C#                  → Rust
────────────────────────────────
byte                → u8
ushort              → u16
uint                → u32
ulong               → u64
short               → i16
int                 → i32
long                → i64
float               → f32
bool                → bool
string              → String
Point               → (i32, i32)
Color               → u32 (ARGB)
DateTime            → chrono::DateTime
List<T>             → Vec<T>
Dictionary<K,V>     → HashMap<K,V>
```

### **Packet实现模板**
```rust
use crate::packet::Packet;
use crate::binary::{Readable, Writable};

#[derive(Debug, Clone, PartialEq)]
pub struct SomePacket {
    pub field1: u32,
    pub field2: String,
}

impl Packet for SomePacket {
    fn packet_id() -> u16 {
        ServerPacketIds::SomePacket as u16
    }
}

impl Readable for SomePacket {
    fn read_from<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        Ok(SomePacket {
            field1: u32::read_from(reader)?,
            field2: String::read_from(reader)?,
        })
    }
}

impl Writable for SomePacket {
    fn write_to<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.field1.write_to(writer)?;
        self.field2.write_to(writer)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_some_packet_serialize() {
        // 测试序列化/反序列化
    }
}
```

---

## 🚀 开始行动

### **立即开始 - 第一步**

1. **暂停 ClientRust Stage 2 对话框开发**
2. **启动 SharedRust 完整迁移项目**
3. **从 ServerPackets.cs 开始** (您当前正在看的文件!)

### **第一天任务分解**

**上午 (4小时)**: ServerPackets 连接&登录包
- Connected, Disconnect, KeepAlive
- LoginSuccess, LoginFailed, NewAccount
- NewCharacter, NewCharacterSuccess, DeleteCharacter
- StartGame, UserInformation, UserLocation
- ~20个包, ~500行

**下午 (4小时)**: ServerPackets 玩家&移动包
- ObjectPlayer, ObjectRemove, ObjectTurn, ObjectWalk, ObjectRun
- PlayerUpdate, PlayerInspect
- MapChanged, TeleportIn, ObjectTeleportOut
- ~20个包, ~600行

---

## 📌 总结

**核心问题**: SharedRust 不完整 → ClientRust 被迫重复实现 → 架构混乱

**解决方案**: 
1. ✅ 先完整迁移 Shared → SharedRust (6-8天)
2. ✅ 然后清理 ClientRust 重复代码 (1天)
3. ✅ 最后继续 ClientRust Stage 2 开发

**优先级**: 
- **P0**: ServerPackets (5,773行) - 网络通信核心
- **P1**: ClientPackets (2,225行) - 客户端请求
- **P1**: Enums完整化 (1,835行) - 类型定义
- **P2**: 数据结构 (2,500行) - 支持结构
- **P3**: 辅助模块 (1,500行) - 工具函数

**时间线**: 6-8天完成完整迁移

---

**您确认这个计划吗？我们从 ServerPackets.cs 开始？** 🚀

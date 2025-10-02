# Shared → SharedRust 迁移执行计划 v2.0

## 📊 现状分析

### **C# Shared 项目** (源文件)
```
ServerPackets.cs        5,773行   ❌ 未迁移
ClientPackets.cs        2,225行   ⚠️  部分迁移 (243行)
Enums.cs                1,835行   ⚠️  部分迁移 (2,179行) - 可能完整
Packet.cs                 926行   ✅ 已迁移 (199行)
Data/ItemData.cs        1,185行   ⚠️  部分迁移 (1,541行)
Data/ClientData.cs        519行   ✅ 已迁移 (417行)
BaseStats.cs              171行   ✅ 已迁移 (626行)
Language.cs               761行   ❌ 未迁移
Globals.cs                 50行   ❌ 未迁移
───────────────────────────────
总计:                  13,445行
```

### **Rust SharedRust 项目** (当前状态)
```
enums.rs                2,179行   ✅ 可能完整
item.rs                 1,541行   ✅ 较完整
stats.rs                  626行   ✅ 完整
packet_ids.rs             473行   ✅ 完整
client_data.rs            417行   ✅ 完整
client_packets.rs         243行   ⚠️  仅10个包 (需要100+)
packet.rs                 199行   ✅ 完整
map.rs                    112行   ✅ 完整
binary.rs                  94行   ✅ 完整
world_map.rs               45行   ✅ 完整
lib.rs                     15行   ✅ 完整
server_packets.rs           0行   ❌ 不存在！
language.rs                 0行   ❌ 不存在！
globals.rs                  0行   ❌ 不存在！
───────────────────────────────
总计:                   5,944行
缺失:                   ~7,500行 (ServerPackets + Language + Globals + 完整ClientPackets)
```

### **ClientRust 重复代码** (需要清理)
```
protocol.rs             2,371行   ❌ 重复了 ServerPackets
protocol_packets/       2,204行   ❌ 重复了 ServerPackets
───────────────────────────────
总计重复:               4,575行   (应该全部删除,使用 SharedRust)
```

---

## 🎯 迁移目标

### **终极目标**
```
SharedRust (完整)       ~15,500行
    ├── server_packets.rs    ~6,500行  ← NEW (最关键)
    ├── client_packets.rs    ~2,800行  ← 扩展 (当前243→2,800)
    ├── language.rs          ~  900行  ← NEW
    ├── globals.rs           ~  100行  ← NEW
    ├── enums.rs              2,179行  ✅ (可能需要微调)
    ├── item.rs               1,541行  ✅
    ├── stats.rs                626行  ✅
    ├── client_data.rs          417行  ✅
    ├── packet_ids.rs           473行  ✅
    ├── packet.rs               199行  ✅
    ├── map.rs                  112行  ✅
    ├── binary.rs                94行  ✅
    ├── world_map.rs             45行  ✅
    └── lib.rs                   30行  ← 更新导出

ClientRust (清理后)
    ├── protocol.rs           ~200行  ← 缩减到仅网络处理逻辑
    └── protocol_packets/    删除整个目录 ❌
```

---

## 📋 详细任务清单

### **阶段0: 准备工作** (30分钟)

#### 0.1 验证 Enums 完整性
```bash
# 检查 C# Enums.cs vs Rust enums.rs
cd d:\Users\gxh\Documents\GitHub\Crystal\Shared
grep -E "^public enum" Enums.cs | wc -l

cd d:\Users\gxh\Documents\GitHub\Crystal\SharedRust\src
grep -E "^pub enum" enums.rs | wc -l
```

**任务**:
- [ ] 对比 C# 和 Rust 枚举数量
- [ ] 确认所有枚举都已迁移
- [ ] 补充缺失的枚举定义

---

### **阶段1: ServerPackets.cs → server_packets.rs** (3天)

**目标**: 将 5,773 行 C# 代码迁移为 ~6,500 行 Rust 代码

#### **1.1 第一天上午: 连接&登录包** (20个包, ~500行)

**包列表**:
```rust
// 连接相关 (5个)
Connected              // 空包
ClientVersion          // result: byte
Disconnect             // reason: byte
KeepAlive              // time: i64, 
NewAccount             // result: byte

// 登录相关 (15个)
ChangePassword         // result: byte
ChangePasswordBanned   // reason: String, expiry_ticks: i64
Login                  // result: byte
LoginSuccess           // characters: Vec<SelectInfo>
LoginBanned            // reason: String, expiry_ticks: i64
NewCharacter           // result: byte
NewCharacterSuccess    // char_info: SelectInfo
DeleteCharacter        // result: byte
DeleteCharacterSuccess // char_index: i32
StartGame              // result: byte, resolution: i32
StartGameSuccess       // 空包
StartGameBanned        // reason: String, expiry_ticks: i64
StartGameDelay         // milliseconds: i64
LogOutSuccess          // characters: Vec<SelectInfo>
LogOutFailed           // 空包
```

**任务**:
- [ ] 创建 `SharedRust/src/server_packets.rs`
- [ ] 实现基础模块结构
- [ ] 实现 20 个包的结构定义
- [ ] 实现每个包的 `Packet` trait
- [ ] 实现每个包的 `Readable` + `Writable` trait
- [ ] 为每个包添加测试 (序列化/反序列化)

**代码模板**:
```rust
// SharedRust/src/server_packets.rs

use crate::packet::Packet;
use crate::binary::{Readable, Writable};
use crate::enums::*;
use crate::client_data::SelectInfo;
use std::io::{Read, Write, Result};

//=============================================================================
// Connection Packets (5)
//=============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Connected;

impl Packet for Connected {
    fn packet_id() -> u16 { ServerPacketIds::Connected as u16 }
}

impl Readable for Connected {
    fn read_from<R: Read>(_reader: &mut R) -> Result<Self> {
        Ok(Connected)
    }
}

impl Writable for Connected {
    fn write_to<W: Write>(&self, _writer: &mut W) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Disconnect {
    pub reason: u8,
}

impl Packet for Disconnect {
    fn packet_id() -> u16 { ServerPacketIds::Disconnect as u16 }
}

impl Readable for Disconnect {
    fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        Ok(Disconnect {
            reason: u8::read_from(reader)?,
        })
    }
}

impl Writable for Disconnect {
    fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        self.reason.write_to(writer)
    }
}

// ... 继续实现其他 18 个包
```

---

#### **1.2 第一天下午: 玩家&地图包** (25个包, ~700行)

**包列表**:
```rust
// 玩家信息 (10个)
UserInformation        // 30+ 字段 (HP, MP, Level, Gold, etc.)
UserLocation           // location: Point, direction: MirDirection
UserSlotsRefresh       // belt_items: Vec<UserItem>, fisher_items: Vec<UserItem>
ObjectPlayer           // 20+ 字段 (玩家出现在视野)
ObjectHero             // 类似 ObjectPlayer (英雄)
ObjectRemove           // object_id: u32
PlayerUpdate           // 外观更新 (武器、盔甲等)
PlayerInspect          // 查看玩家装备
ColourChanged          // name_colour: Color
ObjectColourChanged    // object_id: u32, name_colour: Color

// 移动相关 (5个)
ObjectTurn             // object_id: u32, location: Point, direction: MirDirection
ObjectWalk             // 同上
ObjectRun              // 同上
Pushed                 // location: Point, direction: MirDirection
ObjectPushed           // object_id: u32, location: Point, direction: MirDirection

// 地图相关 (10个)
MapChanged             // 10+ 字段 (地图切换)
MapInformation         // map_index: i32, filename: String, title: String, ...
NewMapInfo             // map_info: ClientMapInfo
WorldMapSetup          // world_map: WorldMapSetup
SearchMapResult        // map_index: u32, location: Point
TimeOfDay              // lights: LightSetting
ObjectTeleportOut      // object_id: u32, type: u8
ObjectTeleportIn       // object_id: u32, type: u8
TeleportIn             // 空包
ObjectHide/ObjectShow  // object_id: u32
```

---

#### **1.3 第二天上午: 战斗包** (25个包, ~800行)

**包列表**:
```rust
// 攻击相关 (10个)
ObjectAttack           // object_id, location, direction, spell, level, type
Struck                 // attacker_id: u32
ObjectStruck           // object_id: u32, attacker_id: u32, location, direction
DamageIndicator        // damage: i32, type: DamageType, object_id: u32
Death                  // location: Point, direction: MirDirection
ObjectDied             // object_id: u32, location, direction, type: u8
ObjectLeveled          // object_id: u32
ObjectHarvest          // object_id: u32, location, direction
ObjectHarvested        // 同上
ObjectHealth           // object_id: u32, percent: u8, expire: u8

// 经验&等级 (8个)
GainExperience         // amount: u32
GainHeroExperience     // amount: u32
LevelChanged           // level: u16, experience: i64, max_experience: i64
HeroLevelChanged       // 同上
HealthChanged          // hp: i32, mp: i32
HeroHealthChanged      // 同上
DuraChanged            // unique_id: u64, current_dura: u16
DeleteItem             // unique_id: u64, count: u16

// 状态相关 (7个)
Poisoned               // poison: PoisonType
ObjectPoisoned         // object_id: u32, poison: PoisonType
Revived                // 空包
ObjectRevived          // object_id: u32, effect: bool
SpellToggle            // object_id: u32, spell: Spell, can_use: bool
AddBuff                // buff_type: BuffType, caster_id: u32, visible: bool, ...
RemoveBuff             // buff_type: BuffType
```

---

#### **1.4 第二天下午: 物品包** (30个包, ~900行)

**包列表**:
```rust
// 物品信息 (10个)
NewItemInfo            // info: ItemInfo
NewHeroInfo            // info: ClientHeroInformation, storage_index: i32
NewChatItem            // item: UserItem
ObjectItem             // object_id, name, location, image, grade
ObjectGold             // object_id, gold, location
GainedItem             // item: UserItem
GainedGold             // gold: u32
LoseGold               // gold: u32
GainedCredit           // credit: u32
LoseCredit             // credit: u32

// 物品操作 (15个)
MoveItem               // grid: MirGridType, from: i32, to: i32, success: bool
EquipItem              // grid, unique_id: u64, to: i32, success: bool
MergeItem              // grid_from, grid_to, id_from, id_to, success: bool
RemoveItem             // grid, unique_id, to: i32, success: bool
RemoveSlotItem         // grid, grid_to, unique_id, to, success: bool
TakeBackItem           // from: i32, to: i32, success: bool
StoreItem              // from, to, success: bool
SplitItem              // item: UserItem, grid: MirGridType
SplitItem1             // grid, unique_id, count: u16, success: bool
UseItem                // unique_id, success: bool, grid: MirGridType
DropItem               // unique_id, count: u16, hero_item: bool, success: bool
RepairItem             // unique_id: u64
ItemRepaired           // unique_id, max_dura: u16, current_dura: u16
ItemSlotSizeChanged    // unique_id, slot_size: i32
ItemSealChanged        // unique_id, expiry_date: DateTime

// 特殊操作 (5个)
TakeBackHeroItem       // from, to, success: bool
TransferHeroItem       // from, to, success: bool
DepositRefineItem      // from, to, success: bool
RetrieveRefineItem     // from, to, success: bool
RefineCancel           // unlock: bool
```

---

#### **1.5 第三天上午: NPC&交易包** (25个包, ~800行)

**包列表**:
```rust
// NPC相关 (15个)
ObjectNPC              // object_id, name, location, image, direction, quest_ids
NPCResponse            // page: Vec<String>
NPCGoods               // list: Vec<UserItem>, rate: f32, type: PanelType
NPCSell                // 空包
NPCRepair              // rate: f32
NPCSRepair             // rate: f32
NPCRefine              // rate: f32, refining: bool
NPCCheckRefine         // 空包
NPCCollectRefine       // success: bool
NPCReplaceWedRing      // rate: f32
NPCStorage             // 空包
UserStorage            // storage: Vec<UserItem>
SellItem               // unique_id: u64, count: u16, success: bool
RefineItem             // unique_id: u64
DepositTradeItem       // from, to, success: bool

// 交易相关 (10个)
TradeRequest           // name: String
TradeAccept            // name: String
TradeGold              // amount: u32
TradeItem              // trade_items: Vec<UserItem>
TradeConfirm           // 空包
TradeCancel            // unlock: bool
RetrieveTradeItem      // from, to, success: bool
MarriageRequest        // name: String
DivorceRequest         // name: String
MentorRequest          // name: String, level: u16
```

---

#### **1.6 第三天下午: 魔法&组队包** (30个包, ~900行)

**包列表**:
```rust
// 魔法相关 (15个)
NewMagic               // magic: ClientMagic, hero: bool
RemoveMagic            // place_id: i32
MagicLeveled           // object_id, spell: Spell, level: u8, experience: u16
Magic                  // spell, target_id, target: Point, cast: bool, level, secondary_targets
MagicDelay             // object_id, spell, delay: i64
MagicCast              // spell: Spell
ObjectMagic            // object_id, location, direction, spell, target_id, ...
ObjectEffect           // object_id, effect: SpellEffect, effect_type, delay_time, time
ObjectProjectile       // spell, source: u32, destination: u32
RangeAttack            // target_id, target: Point, spell
ObjectRangeAttack      // object_id, location, direction, target_id, target: Point, spell
ObjectMana             // object_id, percent: u8
FishingUpdate          // object_id, fishing: bool, progress: i32, bait_lost: bool
ChangeMusic            // music_index: u16
ChangeMapMusic         // music_index: u16

// 组队相关 (10个)
SwitchGroup            // allow_group: bool
DeleteGroup            // 空包
DeleteMember           // name: String
GroupInvite            // name: String
AddMember              // name: String
GroupMembersMap        // player_name: String, player_map: String
SendMemberLocation     // member_name: String, member_location: Point
GroupSwapCharacter     // swap_hero: bool
ChangeTeamSwitchType   // type: TeamSwitchType
LevelUp                // 空包

// 任务相关 (5个)
NewQuestInfo           // quest: ClientQuestInfo
UpdateQuestInfo        // quest_id: i32, quest_progress: ClientQuestProgress
CompleteQuest          // quest_id: i32
RemoveQuest            // quest_id: i32
ShareQuest             // quest_id: i32, share_info: String
```

---

#### **1.7 第四天: 公会&怪物&剩余包** (50个包, ~1500行)

**包列表**:
```rust
// 公会相关 (30个)
GuildNotice            // notice: Vec<String>
GuildMemberChange      // name: String, status: byte
GuildStatus            // guild_name, guild_rank, members: Vec<GuildMemberInfo>
GuildInvite            // name: String
GuildExpGain           // amount: u32
GuildNameRequest       // 空包
GuildStorageGoldChange // amount: u32, type: byte
GuildStorageItemChange // type: byte, index: i32, user: String
GuildStorageList       // items: Vec<UserItem>
GuildRequestWar        // guild_name: String
// ... 更多公会包

// 怪物相关 (10个)
ObjectMonster          // 15+ 字段 (怪物出现在视野)
ObjectAttack           // 怪物攻击
// ... 更多怪物包

// 其他 (10个)
ChangeAMode            // mode: AttackMode
ChangePMode            // mode: PetMode
ReturnToLogin          // 空包
ObjectName             // object_id, name: String
ObjectGuildNameChanged // object_id, guild_name: String
// ... 更多
```

---

### **阶段1 完成标准**

- [ ] 所有 200+ 服务器包定义完整
- [ ] 每个包实现 `Packet` + `Readable` + `Writable`
- [ ] 每个包有单元测试
- [ ] 代码编译通过 (`cargo build`)
- [ ] 测试全部通过 (`cargo test`)
- [ ] 文档注释完整

**预计输出**: `server_packets.rs` (~6,500行)

---

### **阶段2: ClientPackets 完整化** (1-2天)

**目标**: 从 243 行扩展到 ~2,800 行

#### **2.1 第五天上午: 基础客户端包** (30个包, ~800行)

**包列表**:
```rust
// 已有 (10个) ✅
ClientVersion          // version_hash: Vec<u8>
Disconnect             // 空包
KeepAlive              // time: i64
NewAccount             // ...
ChangePassword         // ...
Login                  // ...
NewCharacter           // ...
DeleteCharacter        // ...
StartGame              // ...
LogOut                 // ...

// 需要新增 (20个)
Turn                   // direction: MirDirection
Walk                   // direction: MirDirection
Run                    // direction: MirDirection
Chat                   // message: String, linked_items: Vec<ChatLinkedItem>
MoveItem               // grid: MirGridType, from: i32, to: i32
StoreItem              // from: i32, to: i32
TakeBackItem           // from: i32, to: i32
MergeItem              // grid_from, grid_to, id_from, id_to: u64
EquipItem              // grid: MirGridType, unique_id: u64, to: i32
RemoveItem             // grid, unique_id, to: i32
RemoveSlotItem         // grid, grid_to, unique_id, to: i32
SplitItem              // grid, unique_id, count: u16
UseItem                // unique_id: u64
DropItem               // unique_id, count: u16
PickUp                 // 空包
Inspect                // object_id: u32
ChangeAMode            // mode: AttackMode
ChangePMode            // mode: PetMode
Attack                 // direction: MirDirection, spell: Spell
Harvest                // direction: MirDirection
```

---

#### **2.2 第五天下午: NPC&魔法&交易包** (40个包, ~1200行)

**包列表**:
```rust
// NPC相关 (15个)
CallNPC                // object_id: u32, key: String
BuyItem                // item_index: usize, count: u16
SellItem               // unique_id: u64, count: u16
RepairItem             // unique_id: u64
BuyItemBack            // unique_id: u64, count: u16
SRepairItem            // unique_id: u64
RefineCancel           // 空包
RefineItem             // unique_id: u64
CheckRefine            // unique_id: u64
ReplaceWedRing         // unique_id: u64
DepositRefineItem      // from: i32, to: i32
RetrieveRefineItem     // from: i32, to: i32
DepositTradeItem       // from: i32, to: i32
RetrieveTradeItem      // from: i32, to: i32
RequestUserName        // user_id: u32

// 魔法相关 (10个)
Magic                  // spell: Spell, direction: MirDirection, target_id, location: Point
MagicKey               // spell: Spell, target_id: u32
SwitchCancel           // 空包
SpellToggle            // spell: Spell, can_use: bool
RangeAttack            // direction: MirDirection, location: Point, target_id: u32
Consume                // unique_id: u64
Fishing                // cast_out: bool
FishingCast            // location: Point
RequestMapInfo         // map_index: i32
TeleportToNPC          // object_id: u32

// 交易相关 (15个)
TradeRequest           // target_id: u32
TradeReply             // accept: bool
TradeGold              // amount: u32
TradeConfirm           // 空包
TradeCancel            // 空包
EquipSlotItem          // grid, unique_id, to: i32, grid_to: MirGridType
FishingChangeAutocast  // autocast: bool
AcceptQuest            // npc_index: u32, quest_index: i32
FinishQuest            // quest_id: i32, selected_item_index: i32
AbandonQuest           // quest_id: i32
ShareQuest             // quest_id: i32
AcceptReincarnation    // 空包
CancelReincarnation    // 空包
CombineItem            // grid: MirGridType, from: i32, to: i32
AwakeItem              // unique_id, type: AwakeType
```

---

#### **2.3 第六天上午: 组队&公会包** (30个包, ~800行)

**包列表**:
```rust
// 组队相关 (15个)
GroupSwitch            // allow_group: bool
GroupInvite            // name: String
GroupAcceptInvite      // name: String
GroupDeclineInvite     // name: String
GroupKick              // name: String
GroupLeave             // 空包
GroupSwapCharacter     // swap_hero: bool
ChangeTeamSwitchType   // type: TeamSwitchType
RequestGroupMembers    // 空包
SendGroupLocation      // 空包
JumpToGroup            // member_name: String
AcceptGroupInvite      // name: String
HeroCall               // 空包
TownRevive             // 空包
CancelRevive           // 空包

// 公会相关 (15个)
GuildBuffUpdate        // action: byte, id: i32
GuildCreateAlly        // guild_name: String
GuildDeleteAlly        // guild_name: String
RequestGuildInfo       // guild_id: i32
GuildNameReturn        // name: String
GuildNoticeChange      // notice: Vec<String>
GuildMemberChange      // name: String, status: byte
GuildCreate            // guild_name: String
GuildInvite            // name: String
GuildAcceptInvite      // name: String
GuildDeclineInvite     // name: String
GuildKick              // name: String
GuildLeave             // 空包
RequestGuildList       // offset: i32
RequestGuildMemberList // 空包
```

---

### **阶段2 完成标准**

- [ ] ClientPackets 从 10 个扩展到 100+ 个
- [ ] 所有客户端请求包完整
- [ ] 每个包实现 `Packet` + `Readable` + `Writable`
- [ ] 编译通过 + 测试通过

**预计输出**: `client_packets.rs` (~2,800行, 从 243 → 2,800)

---

### **阶段3: Language + Globals** (半天)

#### **3.1 Language.cs → language.rs** (761行 → ~900行)

**结构**:
```rust
// SharedRust/src/language.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameLanguage {
    // 所有游戏文本
    pub game_name: String,
    pub login_window_title: String,
    pub character_window_title: String,
    
    // 物品相关
    pub item_durability: String,
    pub item_weight: String,
    pub item_required_level: String,
    
    // 技能相关
    pub spell_mana_cost: String,
    pub spell_cast_time: String,
    
    // UI文本
    pub button_confirm: String,
    pub button_cancel: String,
    pub button_ok: String,
    
    // 聊天相关
    pub chat_all: String,
    pub chat_whisper: String,
    pub chat_guild: String,
    
    // ... 100+ 字段
}

impl Default for GameLanguage {
    fn default() -> Self {
        Self::english()
    }
}

impl GameLanguage {
    pub fn english() -> Self {
        GameLanguage {
            game_name: "Legend of Mir 2".to_string(),
            login_window_title: "Login".to_string(),
            // ... 所有默认英文文本
        }
    }
    
    pub fn chinese() -> Self {
        GameLanguage {
            game_name: "传奇".to_string(),
            login_window_title: "登录".to_string(),
            // ... 所有中文文本
        }
    }
    
    pub fn load_from_file(path: &str) -> std::io::Result<Self> {
        // 从配置文件加载
        todo!()
    }
}
```

---

#### **3.2 Globals.cs → globals.rs** (50行 → ~100行)

**结构**:
```rust
// SharedRust/src/globals.rs

pub const CRYSTAL_VERSION: u32 = 1_0_0;
pub const PROTOCOL_VERSION: u16 = 83;

pub const MAX_LEVEL: u16 = 400;
pub const MAX_BAG_SIZE: usize = 46;
pub const MAX_STORAGE_SIZE: usize = 160;
pub const MAX_GUILD_STORAGE_SIZE: usize = 400;

pub const MAX_WEIGHT: i32 = 50;
pub const MAX_HAND_WEIGHT: i32 = 12;

pub const MIN_USERNAME_LENGTH: usize = 3;
pub const MAX_USERNAME_LENGTH: usize = 15;
pub const MIN_PASSWORD_LENGTH: usize = 5;
pub const MAX_PASSWORD_LENGTH: usize = 15;
pub const MIN_CHAR_NAME_LENGTH: usize = 3;
pub const MAX_CHAR_NAME_LENGTH: usize = 15;

pub const GOLD_WEIGHT: f32 = 0.0001;

pub const MAX_INVENTORY_SLOTS: usize = 46;
pub const MAX_EQUIPMENT_SLOTS: usize = 14;

// ... 更多全局常量
```

---

### **阶段3 完成标准**

- [ ] language.rs 完整 (~900行)
- [ ] globals.rs 完整 (~100行)
- [ ] 编译通过
- [ ] lib.rs 更新导出

---

### **阶段4: 清理 ClientRust 重复代码** (1天)

#### **4.1 protocol.rs 重构** (2,371行 → ~200行)

**之前 (重复定义)**:
```rust
// ClientRust/src/protocol.rs (2,371行)
// 定义了大量 ServerMessage enum 和解析逻辑
pub enum ServerMessage {
    Connected,
    ClientVersion { result: ClientVersionResult },
    // ... 200+ 变体
}

pub struct PlayerObject { ... }  // ❌ 重复
pub struct ObjectMonster { ... } // ❌ 重复
// ... 大量重复定义
```

**之后 (使用 SharedRust)**:
```rust
// ClientRust/src/protocol.rs (~200行)
use shared_rust::server_packets::*;
use shared_rust::client_packets::*;
use shared_rust::packet::Packet;

// 仅保留网络层逻辑
pub struct NetworkProtocol {
    // 网络连接管理
}

impl NetworkProtocol {
    pub fn parse_server_packet(&self, data: &[u8]) -> Result<Box<dyn Packet>> {
        // 使用 SharedRust 中的包定义
        todo!()
    }
    
    pub fn encode_client_packet(&self, packet: &dyn Packet) -> Result<Vec<u8>> {
        // 使用 SharedRust 中的包定义
        todo!()
    }
}
```

---

#### **4.2 删除 protocol_packets/ 目录** (2,204行 → 0行)

```bash
# 删除整个目录
rm -rf ClientRust/src/protocol_packets
```

**原因**: 所有包定义现在都在 SharedRust 中

---

#### **4.3 更新 ClientRust 依赖**

```toml
# ClientRust/Cargo.toml
[dependencies]
shared_rust = { path = "../SharedRust" }  # 确保路径正确
```

---

### **阶段4 完成标准**

- [ ] protocol.rs 缩减到 ~200 行
- [ ] protocol_packets/ 目录删除
- [ ] 所有包使用 `use shared_rust::*`
- [ ] ClientRust 编译通过
- [ ] ClientRust 测试通过
- [ ] 减少 ~4,500 行重复代码

---

## 📊 总进度跟踪

### **代码行数变化**

```
迁移前:
  SharedRust:            5,944行
  ClientRust (protocol): 4,575行 (重复)
  ─────────────────────────────
  总计:                 10,519行

迁移后:
  SharedRust:           15,500行  (+9,556行)
  ClientRust (protocol):   200行  (-4,375行)
  ─────────────────────────────
  总计:                 15,700行  (+5,181行净增长)

重复代码消除: -4,375行 ✅
```

### **时间轴**

```
准备工作:   0.5天
阶段1:      3.0天  (ServerPackets)
阶段2:      2.0天  (ClientPackets)
阶段3:      0.5天  (Language + Globals)
阶段4:      1.0天  (ClientRust清理)
────────────────
总计:       7.0天
```

---

## ✅ 最终验证清单

### **SharedRust 验证**
- [ ] 所有 ServerPacketIds 有对应实现 (200+)
- [ ] 所有 ClientPacketIds 有对应实现 (100+)
- [ ] 所有枚举完整 (Enums.cs → enums.rs)
- [ ] 所有测试通过 (`cargo test`)
- [ ] 文档完整 (`cargo doc --open`)
- [ ] 无编译警告 (`cargo clippy`)
- [ ] 代码格式化 (`cargo fmt`)

### **ClientRust 验证**
- [ ] 删除所有重复包定义
- [ ] 使用 `shared_rust::*` 导入所有包
- [ ] protocol.rs 仅保留网络逻辑 (~200行)
- [ ] 编译通过
- [ ] 测试通过
- [ ] 运行时验证 (与服务器通信正常)

### **二进制兼容性验证**
- [ ] Rust 客户端可以连接 C# 服务器
- [ ] 所有包序列化格式与 C# 一致
- [ ] 所有包反序列化格式与 C# 一致
- [ ] 网络通信正常

---

## 🚀 开始执行

### **立即开始 - 第一个任务**

```bash
# 1. 进入 SharedRust 目录
cd d:\Users\gxh\Documents\GitHub\Crystal\SharedRust\src

# 2. 创建 server_packets.rs
# (Agent 将创建文件)

# 3. 验证 Enums 完整性
cd d:\Users\gxh\Documents\GitHub\Crystal\Shared
grep -E "^public enum" Enums.cs | wc -l

cd d:\Users\gxh\Documents\GitHub\Crystal\SharedRust\src
grep -E "^pub enum" enums.rs | wc -l
```

---

**准备好开始了吗？我们从 ServerPackets 的第一批 20 个包开始！** 🎯

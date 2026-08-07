# Crystal Mir2 Rust 移植状态

> 最后更新: 2026-08-08 | 测试: ServerRust 227 + protocol_conformance 5 | Client-Bevy 79 | 真实服 E2E 12/12

---

## 总体进度: ~90%

| 组件 | 文件 | 状态 |
|------|------|------|
| SharedRust (协议库) | 146 client + 273 server packets | ✅ 100% |
| Client-Macroquad | 228 .rs, 46 ECS 系统, 37 对话框 | ✅ ~99% |
| ServerRust | 47+ .rs, kameo actor 架构 | ✅ ~90% |

---

## 服务端功能对照

### 核心系统 ✅

| 功能 | C# 参考 | Rust 实现 | 状态 |
|------|---------|----------|------|
| 网络网关 | MirNetwork/ | `gate/actor.rs` (2782行) | ✅ |
| 账号认证 | AccountInfo.cs | `actors/account.rs` | ✅ |
| 角色管理 | CharacterInfo.cs | `actors/player.rs` (2600行) | ✅ |
| 背包/装备 | HumanObject.cs | `actors/inventory.rs` (787行) | ✅ |
| 移动 | HumanObject.cs | `world/session.rs` | ✅ |
| 聊天 | ChatSystem.cs | `social.rs` | ✅ |
| 战斗(物理) | MonsterObject.cs | `combat/attack.rs` | ✅ |
| 魔法/技能 | SpellObject.cs (639行) | `combat/magic.rs` + `world/spell.rs` | ✅ |
| 怪物 AI | 212 per-monster .cs | `world/mod.rs` + `world/tick.rs` (数据驱动) | ✅ |
| NPC | NPCObject.cs | `world/npc.rs` (994行) | ✅ |
| 任务 | QuestInfo.cs | `world/quest.rs` | ✅ |
| 组队 | PlayerObject.cs | `social.rs` | ✅ |
| 交易 | PlayerObject.cs | `social.rs` | ✅ |
| 好友 | PlayerObject.cs | `actors/friend.rs` + `social.rs` | ✅ |
| 邮件 | MailInfo.cs | `actors/mail.rs` + `world/mail.rs` | ✅ |
| 公会 | GuildObject.cs (1037行) | `actors/guild.rs` + `world/guild.rs` + `social.rs` | ✅ |
| 结婚/师徒 | PlayerObject.cs | `social.rs` | ✅ |
| 市场/拍卖 | AuctionInfo.cs | `world/market.rs` (1023行) | ✅ |
| 商城 | NPC system | `world/npc.rs` | ✅ |
| 仓库 | PlayerObject.cs | `world/item.rs` | ✅ |
| 精炼 | NPC system | `actors/refine.rs` + `world/awakening.rs` | ✅ |
| 觉醒 | NPC system | `world/awakening.rs` (624行) | ✅ |
| 英雄 | HeroObject.cs (1277行) | `world/hero.rs` (252行) | ✅ |
| 宠物 | IntelligentCreatureObject.cs | `actors/creature.rs` + `world/hero.rs` | ✅ |
| Buff | BuffInfo.cs | `combat/buff.rs` (199行) | ✅ |
| PK 系统 | PlayerObject.cs | `world/tick.rs` | ✅ |
| 日夜循环 | Envir | `world/tick.rs` | ✅ |
| 地图系统 | Map.cs (2549行) | `maps/loader.rs` + `world/mod.rs` | ✅ |
| 复活/重生 | PlayerObject.cs | `world/tick.rs` | ✅ |
| 租赁 | Trade system | `world/market.rs` (467行) | ✅ |
| 数据库 | MirDatabase/ (23 files) | `db/mod.rs` (2562行) | ✅ |
| 地图传送点 | Map.cs | `world/mod.rs` | ✅ |
| 环境伤害 | Map.cs | `world/tick.rs` | ✅ |
| 自动喝药 | PlayerObject.cs | `actors/player.rs` | ✅ |

### 辅助系统 ✅

| 功能 | C# 参考 | Rust 实现 | 状态 |
|------|---------|----------|------|
| 钓鱼 | PlayerObject.cs (280行) | `world/npc.rs` + `world/tick.rs` | ✅ |
| 采矿 | MineInfo.cs (101行) | `world/combat.rs` | ✅ |
| 排行 | Player system | `world/npc.rs` (DB查询) | ✅ |
| 举报 | Player system | `world/report.rs` (DB持久化) | ✅ |
| 机器人 | Robot.cs (114行) | `world/robot.rs` | ✅ |
| 抽奖 | PlayerObject.cs | `world/item.rs` | ✅ |
| 公会领地 | Guild system | `world/guild.rs` (接入征服数据) | ✅ |
| 坐骑 | PlayerObject.cs | `actors/player.rs` | ✅ |

### 终局系统 ✅

| 功能 | C# 参考 | Rust 实现 | 状态 |
|------|---------|----------|------|
| 龙 | Dragon.cs (232行) | `world/dragon.rs` | ✅ |
| 征服/攻城 | ConquestObject.cs (969行) | `world/conquest.rs` (215行) | ✅ |
| 城门/城墙 | Gate.cs, Wall.cs, CastleGate.cs | `world/conquest.rs` SiegeStructure | ✅ |

### 待补齐 ⚠️（2026-08 更新）

| 功能 | 阻塞原因 |
|------|----------|
| HTTP 后台 | 代码已写 (`http.rs`)，需 axum 依赖（可选，未启用） |
| 91 法术逐差异化效果 | 已大部分实现（FireWall/Blizzard/Meteor/HellFire/IceThrust/Curse/Lightning/FireBang/IceStorm/Poisoning/TrapHexagon/传送/召唤等均有 C# 专属逻辑）；剩余为个别法术细节 |
| 精灵 Atlas 性能优化 | 可选优化项（客户端 CPU/内存基线） |

### 2026-08-07/08 服务端/客户端对齐批次（已合入 master）

- 怪物 AI 对齐 C#：仇恨/巡逻/攻速移速/Boss 专属行为（#990）
- 掉落对齐 C#：Gold/QuestRequired/稀有度 Uncommon/Rare/Elite/掉落率（#999/#1140）
- 召唤系对齐 C#：NoPets/宠物上限/PetLevel/Shinsu 形态（#981）
- 经验发放对齐 C#：WinExp/GainExp/InRange 切比雪夫 16/死亡成员不计经验（#1160）
- 宠物/行会经验 + 经验曲线配置化（#1164）；行会战对齐 C#（宣战费用/时限自动结束，#1170）
- 英雄系统：HP 同步/阵亡/经验等级链路/LastHitter（#1129/#1137/#1143/#1165）
- 装备槽补 Torch/Belt/Stone + 双击 L/R 智能选择（#1172/#1175）
- 真实服 E2E 回归基建：严格标记判定 + 测试库自动准备（#1144/#1152/#1162/#1166/#1167/#1173）

---

## 已加载游戏数据

| 表 | 数量 | 来源 |
|----|------|------|
| 物品 | 1,348 | C# Server.MirDB |
| 怪物 | 506 | C# Server.MirDB |
| NPC | 293 | C# Server.MirDB |
| 魔法 | 103 | C# Server.MirDB |
| 任务 | 157 | C# Server.MirDB |
| 商城 | 106 | C# Server.MirDB |

---

## 测试覆盖

| 层 | 测试数 | 类型 |
|----|--------|------|
| SharedRust | 156 | 包体 roundtrip |
| ServerRust | 106 (含 7 E2E) | 单元 + 端到端 |
| Client-Macroquad | 10 E2E | Mock 网络 + ECS |

---

## 服务器启动验证

```
物品: 1348 ✅  怪物: 506 ✅  NPC: 293 ✅
魔法: 103 ✅   任务: 157 ✅  商城: 106 ✅
监听: 0.0.0.0:7000 ✅  崩溃: 0 ✅
```

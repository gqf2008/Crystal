# Crystal Server 移植计划

## 项目概况

**目标：** 将 OpenMir2 C# 服务器（.NET 8）移植为 Rust 实现，与现有 Rust 客户端（macroquad）无缝对接。

**现有基础设施：**
- `SharedRust` — 协议定义已完成（19 客户端包模块 + 34 服务端包模块）
- 客户端 — 250+ NetworkEvent 类型已定义，定义了服务器需要生产/消费的全部事件
- `mock.rs` — 已实现 mock 服务器逻辑，可作为真实服务器行为参考

**C# 原始架构：** 多进程 Gate + Service 模式，13 个服务项目。

## 架构决策

### Rust 服务器架构：单进程 + tokio async + kameo actor

与 C# 的多进程不同，Rust 版采用单进程 + tokio async + kameo actor 模型：

| C# 多进程 | Rust Actor | 理由 |
|-----------|------------|------|
| LoginGate + SelGate + GameGate | `GateActor` | TCP 接入，会话隔离 |
| LoginSrv | `AccountActor` | 账号认证，状态独占 |
| DBSrv | `StorageActor` | 数据持久化，串行化写入 |
| M2Server World | `WorldActor` + `MapActor` | 游戏主循环 + 地图网格 |
| PlayerObject | `PlayerActor`（每玩家一个） | 独立属性/装备/战斗状态 |
| MonsterObject | `MonsterActor`（每怪一个） | 独立 AI + 行为树 |
| GuildManager | `GuildActor` | 行会数据 + 成员管理 |

**Kameo 优势：**
- 每个 actor 独占状态，无需 `Mutex`/`RwLock`
- 内置 supervision 策略，actor 崩溃可自动重启（替代 C# 的手动 try-catch）
- 原生 `ask`/`tell` 消息模式，替代手写 `channel`
- 0.20.0 版本已迭代 20+ 个版本，背靠 Tokio 底层稳定

### 技术选型

| 用途 | 选型 | 理由 |
|------|------|------|
| 异步运行时 | `tokio` | 成熟生态，C# async/await 直接映射 |
| Actor 模型 | `kameo 0.20.0` | 基于 Tokio，状态隔离 + supervision 自动恢复 |
| 网络协议 | 复用 `mir2_shared` | 客户端已用，序列化/反序列化统一 |
| 数据库 | `sqlx` + SQLite | 轻量，C# DBSrv 多后端可选一先做 |
| ECS（可选） | `hecs` 或 `shipyard` | 与客户端一致，怪物/玩家/NPC 统一建模 |
| 日志 | `tracing` | 与客户端一致 |
| 配置 | `serde` + TOML | 替代 C# 的 XML/INI 配置 |

## 项目结构

```
Crystal/Server/
├── Cargo.toml                 # kameo = "0.20.0", tokio, mir2_shared, ...
├── config/
│   └── server.toml            # 服务器配置（端口、数据库路径等）
├── src/
│   ├── main.rs                # 启动入口：spawn actors → tokio::select!
│   ├── lib.rs                 # 库根
│   ├── gate/                  # TCP 接入层（替代 LoginGate/SelGate/GameGate）
│   │   ├── mod.rs
│   │   ├── actor.rs           # GateActor：接受连接，转发到 AccountActor
│   │   └── codec.rs           # 帧编解码（长度前缀 + XOR）
│   ├── actors/                # Actor 定义（核心业务逻辑）
│   │   ├── mod.rs
│   │   ├── account.rs         # AccountActor：账号认证（替代 LoginSrv）
│   │   ├── character.rs       # CharacterActor：角色创建/选择（替代 SelGate）
│   │   ├── world.rs           # WorldActor：游戏主循环（替代 GameSrv）
│   │   ├── player.rs          # PlayerActor：每玩家一个 actor（替代 CharacterObject）
│   │   ├── monster.rs         # MonsterActor：每怪一个 actor（替代 MonsterObject）
│   │   ├── npc.rs             # NpcActor：NPC 对话/商品/合成（替代 Merchant）
│   │   ├── map.rs             # MapActor：地图网格 + 对象广播（替代 Environment）
│   │   ├── guild.rs           # GuildActor：行会 + 攻城（替代 GuildManager）
│   │   └── storage.rs         # StorageActor：数据持久化（替代 DBSrv，串行写入）
│   ├── combat/                # 战斗计算（纯函数，actor 调用）
│   │   ├── mod.rs
│   │   ├── attack.rs          # 命中/伤害/暴击计算
│   │   ├── magic.rs           # 魔法系统
│   │   └── buff.rs            # Buff/Debuff
│   ├── maps/                  # 地图数据
│   │   ├── mod.rs
│   │   ├── loader.rs          # 地图加载（复用客户端 MapReader）
│   │   └── pathfind.rs        # 寻路（复用客户端 PathfindingSystem）
│   ├── systems/               # 子系统（纯函数/actor 混合）
│   │   ├── mod.rs
│   │   ├── chat.rs            # ChatActor
│   │   ├── trade.rs           # TradeActor
│   │   ├── group.rs           # GroupActor
│   │   ├── mail.rs            # MailActor
│   │   ├── market.rs          # MarketActor
│   │   └── quest.rs           # QuestActor
│   └── util/
│       ├── mod.rs
│       ├── config.rs          # 配置加载
│       └── crypto.rs          # 加密/解密（复用 SharedRust 的 TOTP/CRC）
```

## 移植阶段

### Phase 1：基础设施（可独立运行）

**目标：** 能接受 TCP 连接，处理登录/选角流程

| 步骤 | 内容 | 复用来源 | Actor 映射 | 预估难度 |
|------|------|----------|------------|----------|
| 1.1 | Cargo workspace + Cargo.toml | — | — | 低 |
| 1.2 | GateActor：TCP 监听 + 会话管理 | LoginGate/AppServer.cs | `GateActor` | 低 |
| 1.3 | GateActor：帧编解码（长度前缀 + XOR） | LoginGate/ClientSession.cs | — | 中 |
| 1.4 | AccountActor：账号注册/登录 | LoginSrv/AccountManager.cs | `AccountActor` | 低 |
| 1.5 | StorageActor：SQLite 持久化 | DBSrv/LocalFileStorage.cs | `StorageActor`（串行写入） | 低 |
| 1.6 | CharacterActor：角色创建/选择/删除 | SelGate + DBSrv 角色逻辑 | `CharacterActor` | 中 |

**验收标准：** `cargo run` → 客户端能完成登录 + 选角色 → 进入游戏场景

### Phase 2：地图 + 玩家（核心游戏循环）

**目标：** 玩家能在地图上移动、看到其他实体

| 步骤 | 内容 | 复用来源 | Actor 映射 | 预估难度 |
|------|------|----------|----------|----------|
| 2.1 | MapActor：地图加载 | GameSrv/MapManager + 客户端 MapReader | `MapActor` | 中 |
| 2.2 | MapActor：网格系统（视野/广播） | M2Server/Environment.cs | — | 中 |
| 2.3 | PlayerActor：属性/装备/状态 | M2Server/CharacterObject.cs | 每玩家一个 `PlayerActor` | 高 |
| 2.4 | PlayerActor：移动处理 | M2Server/CharacterObject.Operate.cs | — | 中 |
| 2.5 | WorldActor：游戏主循环 | GameSrv/WorldServer.cs | `WorldActor` | 中 |
| 2.6 | MapActor：实体广播 | M2Server/BaseObject.Message.cs | — | 中 |

**验收标准：** 多客户端连接，玩家可见彼此移动，地图正确加载

### Phase 3：战斗 + NPC

**目标：** 玩家能攻击怪物、与 NPC 交互

| 步骤 | 内容 | 复用来源 | Actor 映射 | 预估难度 |
|------|------|----------|----------|----------|
| 3.1 | 攻击计算（命中/伤害/暴击） | M2Server/CharacterObject.Attack.cs | 纯函数，PlayerActor 调用 | 高 |
| 3.2 | MonsterActor：AI + 生成器 | M2Server/MonsterObject + GameSrv/MonGenProcessor | 每怪一个 `MonsterActor` | 高 |
| 3.3 | 掉落系统 | M2Server/CharacterObject.Sale.cs | — | 中 |
| 3.4 | NpcActor：对话/商品/合成/仓库 | GameSrv/Merchant + NPCDialogs | `NpcActor` | 高 |
| 3.5 | Buff 系统 | M2Server/ActorBuffSystem.cs | PlayerActor 内嵌 | 中 |
| 3.6 | 魔法系统 | M2Server/MagicBase/MagicManager | 纯函数，PlayerActor 调用 | 高 |

**验收标准：** 打怪升级、掉落物品、NPC 对话/购买/合成/仓库

### Phase 4：社交系统

**目标：** 玩家间交互功能

| 步骤 | 内容 | 复用来源 | Actor 映射 | 预估难度 |
|------|------|----------|----------|----------|
| 4.1 | 组队系统 | M2Server Group 逻辑 | `GroupActor` | 中 |
| 4.2 | 交易系统 | M2Server/CharacterObject.Socket.cs | `TradeActor` | 高 |
| 4.3 | 聊天系统（世界/私聊/行会） | ChatSrv + M2Server/Chat | `ChatActor` | 中 |
| 4.4 | 好友系统 | 对应 NetworkEvent | — | 低 |
| 4.5 | 行会 + 攻城 | M2Server/GuildManager + CastleManager | `GuildActor` + `CastleActor` | 高 |
| 4.6 | 婚姻/师徒 | 对应 NetworkEvent | — | 低 |

**验收标准：** 组队打怪、玩家交易、行会创建

### Phase 5：经济系统

**目标：** 游戏内经济循环

| 步骤 | 内容 | 复用来源 | Actor 映射 | 预估难度 |
|------|------|----------|----------|----------|
| 5.1 | 邮件系统 | MailSrv 逻辑 | `MailActor` | 中 |
| 5.2 | 商城 | MarketSystem 模块 | `MarketActor` | 中 |
| 5.3 | 寄售行/拍卖行 | UserStallSystem | `MarketActor` | 高 |
| 5.4 | 物品租赁 | 对应 NetworkEvent | — | 中 |
| 5.5 | 英雄系统 | Hero 相关 NetworkEvent | `HeroActor` | 高 |

**验收标准：** 邮件收发、商城购买、寄售行上架/购买

### Phase 6：完善 + 优化

| 步骤 | 内容 | 预估难度 |
|------|------|----------|
| 6.1 | 性能优化（实体分片、网络缓冲） | 高 |
| 6.2 | 多数据库后端（MySQL/MongoDB） | 中 |
| 6.3 | GM 管理工具（Web API） | 中 |
| 6.4 | 机器人/BotSrv | 高 |
| 6.5 | 文档 + 部署脚本 | 低 |

## 复用清单

### 可直接复用（无需重写）

| 来源 | 内容 | 用途 |
|------|------|------|
| `SharedRust` | 全部客户端/服务端包定义 | 服务器协议层 |
| `SharedRust` | 枚举（Direction/AttackMode/PoisonState 等） | 数据类型 |
| `SharedRust` | CRC/加密工具 | 协议加密 |
| 客户端 `map_reader.rs` | 地图文件解析 | 服务端地图加载 |
| 客户端 `pathfinding_system.rs` | A* 寻路 | 服务端怪物/NPC 寻路 |
| 客户端 `mock.rs` | Mock 服务器行为 | 真实服务器行为参考 |
| 客户端 `network/handlers/` | 250+ 事件定义 | 服务器需要响应的事件清单 |

### 需要重写的（C# → Rust）

| C# 源文件 | 目标 Rust Actor | 复杂度 |
|-----------|-----------------|--------|
| LoginSrv/*.cs | `AccountActor` | 低（~500 行 C#） |
| DBSrv/*.cs | `StorageActor` | 低（~800 行 C#） |
| M2Server/BaseObject.cs | `BaseActor`（trait） | 中（核心抽象） |
| M2Server/CharacterObject.cs | `PlayerActor` | 高（~8000 行 C#，7 个 partial） |
| M2Server/MonsterObject.cs | `MonsterActor` | 高（~50 怪物子类） |
| M2Server/Environment.cs | `MapActor` | 中 |
| M2Server/MagicBase.cs | `MagicActor` | 高 |
| GameSrv/WorldServer.cs | `WorldActor` | 中 |
| GameSrv/Merchant.cs | `NpcActor` | 中 |
| M2Server/GuildManager.cs | `GuildActor` | 高 |
| M2Server/CastleManager.cs | `CastleActor` | 高 |

## 里程碑

| 里程碑 | 达成标志 | 风险 |
|--------|----------|------|
| M1：能登录 | 客户端完成登录流程 | 低 |
| M2：能进游戏 | 客户端进入游戏场景 | 中（地图加载） |
| M3：多人可见 | 多客户端互相看到移动 | 中（广播） |
| M4：能打怪 | 攻击怪物、掉落物品 | 高（战斗计算复杂） |
| M5：能交易 | 玩家间交易完成 | 高（状态同步） |
| M6：完整功能 | 所有 Phase 1-5 通过 | 取决于 3/4/5 阶段 |

## 风险与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| M2Server 代码量巨大（113 文件） | 移植周期长 | 分模块逐步移植，优先核心路径 |
| 战斗计算逻辑复杂（数值平衡） | 与 C# 不一致导致客户端异常 | 用 mock.rs 作为行为参考，写对比测试 |
| 50+ 怪物子类 | 维护成本高 | 用数据驱动（配置表）替代继承 |
| 地图文件格式不透明 | 加载失败 | 直接复用客户端已验证的 MapReader |
| 多进程到 actor 模型的架构差异 | 并发模型变化 | kameo actor 天然隔离状态，Tokio 处理并发 |

## 开发约定

- 与客户端保持一致的代码风格：简洁、无不必要的抽象、无过度设计
- 协议处理复用 `mir2_shared` 的 Packet trait
- 网络事件命名与客户端 `NetworkEvent` 保持一致
- Actor 之间通过 `ask`（同步响应）/ `tell`（异步投递）通信，避免手写 channel
- 战斗/伤害计算用纯函数，actor 调用，不内嵌到 actor 状态
- 每个 Phase 完成后用客户端连接验证，不写纯单元测试
- 不写 mock 数据，直接用真实逻辑（吸取客户端 HUD mock 数据的教训）
- Actor 崩溃时利用 kameo supervision 策略自动重启，不吞异常

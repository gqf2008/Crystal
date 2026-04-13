# Crystal 传奇2 Rust 客户端 — 移植计划

> 基于 C# 原版 (`Client/`) 与 Rust 移植 (`Client-Macroquad/src/`) 的全面对比分析。
> 目标：系统性完成剩余 ~55% 的移植工作。

---

## 项目现状

| 维度 | C# 原版 | Rust 移植 | 完成度 |
|------|---------|-----------|--------|
| 源文件数 | 97 | 184 | — |
| 代码行数 | ~73,500 | ~53,400 | 73% |
| 场景数 | 3 | 4 | 133% |
| Dialog 数 | ~35 | ~15 | 43% |
| 网络 Handler | 单体 | 10 模块 (4 完成 + 6 stub) | 40% |

### 功能模块完成度

| 模块 | 完成度 | 说明 |
|------|--------|------|
| 场景切换系统 | 100% | Login → Select → Game 全链路 |
| 登录/注册 | 100% | 含 ChangePassword、NewAccount 子对话框 |
| 网络层框架 | 90% | 客户端+Mock Server 完整，Handler 待补全 |
| 资源加载 (MLibrary) | 95% | .lib 解析器 + 纹理缓存完整 |
| 地图系统 | 90% | 地图读取 + 网格渲染 + 碰撞检测 |
| 角色/装备UI | 80% | 装备槽/属性面板 UI 完成 |
| 背包UI | 80% | 多标签页 UI 完成，拖拽待完善 |
| NPC 对话 | 70% | 基础对话 + 商店框架 |
| 战斗系统 | 50% | 伤害计算完成，技能/PK 未实现 |
| 音效 | 50% | 基础播放完成，循环/衰减未实现 |
| 粒子系统 | 40% | 基础框架完成 |
| 社交功能 | 20% | Handler stub 存在 |
| 任务系统 | 10% | 仅日志查看 UI |
| 交易系统 | 0% | Handler stub 存在 |
| 宠物/英雄 | 0% | 未开始 |
| 行会系统 | 0% | Handler stub 存在 |

---

## 移植原则

1. **先框架后功能** — 优先补齐核心游戏循环和基础交互
2. **先 UI 后逻辑** — UI 骨架搭好后再对接网络/战斗逻辑
3. **增量可用** — 每个阶段结束时游戏应该能跑起来玩
4. **ECS 优先** — 不复刻 C# 单例模式，新功能走 ECS 组件系统
5. **不做多余工作** — 如 Windows Forms (AMain/Config) 不需要移植

---

## 阶段一：基础游戏循环打通（~2 周）

> 让游戏从登录 → 选择角色 → 进入游戏 → 看到地图+角色 → 基础移动+战斗 完整跑通

### 1.1 补全核心 Network Handler

**目标**: 让服务器驱动的游戏逻辑能正确反映在客户端

| Handler | 当前状态 | 需要实现 | 优先级 | 预估 |
|---------|---------|---------|--------|------|
| `item.rs` | 4/37 opcodes | MoveItem, EquipItem, UseItem, DropItem, MergeItem, SplitItem | 🔴 P0 | 3h |
| `movement.rs` | 基础/15+ | Teleport, BackStep, Dash, SitDown | 🔴 P0 | 2h |
| `combat.rs` | 基础 | ObjectStruck, ObjectDied, PlayerDied | 🔴 P0 | 2h |
| `npc.rs` | 2/32 | NPCSell, NPCRepair, NPCStorage, CraftItem | 🟡 P1 | 2h |
| `quest.rs` | 0/全部 | ChangeQuest, CompleteQuest, NewQuestInfo | 🟡 P1 | 2h |
| `trade.rs` | 2/6 | TradeGold, TradeItem, TradeConfirm, TradeCancel | 🟡 P1 | 2h |

**产出**: 每个 opcode 至少正确解析为 NetworkEvent，即使游戏逻辑暂时用 TODO 占位

### 1.2 完善玩家属性同步

**文件**: `scenes/dialogs/game/main_dialog.rs`, `systems/infra/network_apply_system.rs`

- [ ] 等级变化时 MainDialog 更新
- [ ] 金币变化时 MainDialog 更新
- [ ] 经验变化时经验条更新
- [ ] 负重变化时负重条更新
- [ ] 背包空格数同步

### 1.3 完善怪物渲染

**文件**: `systems/infra/network_apply_system.rs`, `systems/rendering/sprite_system.rs`

- [ ] 怪物出现/消失网络事件处理
- [ ] 怪物动画正确播放 (Stand/Walk/Run/Attack)
- [ ] 怪物头顶名称显示
- [ ] 怪物头顶血条显示

### 1.4 完善战斗反馈

**文件**: `systems/infra/network_apply_system.rs`

- [ ] 受击动画 (Flinch)
- [ ] 死亡动画 (Die)
- [ ] 伤害数字浮动效果
- [ ] 技能特效显示

### 1.5 技能系统基础

**文件**: `systems/logic/combat/skill_system.rs`, `components/spell.rs`

- [ ] 技能释放请求 → 网络发送
- [ ] 技能冷却显示
- [ ] 已学技能列表同步
- [ ] 技能快捷键 (F1-F8) 释放

### 阶段一检查清单

- [ ] 能登录服务器
- [ ] 能看到角色列表中所有角色
- [ ] 能进入游戏场景
- [ ] 能看到地图、自己、其他玩家、怪物、NPC
- [ ] 能移动、攻击怪物、怪物会反击
- [ ] 拾取物品后背包格子减少
- [ ] 使用物品/技能有视觉反馈
- [ ] 死亡/复活流程正常

---

## 阶段二：核心 UI 完善（~2 周）

> 完善现有 UI 的交互和视觉效果

### 2.1 MainDialog 补全

**文件**: `scenes/dialogs/game/main_dialog.rs`

- [ ] 攻击模式显示 (AMode/PMode/SMode 切换)
- [ ] 角色状态图标 (中毒/隐身等)
- [ ] 快捷技能栏 (功能按钮右侧)
- [ ] 小地图缩略图 (MainDialog 右上角)

### 2.2 InventoryDialog 交互完善

**文件**: `scenes/dialogs/game/inventory_dialog.rs`

- [ ] 物品图标从 MLibrary 正确加载
- [ ] 物品名称 Tooltip
- [ ] 物品拖拽 (背包内移动)
- [ ] 物品拆分 (数量 > 1 时)
- [ ] 物品丢弃确认对话框
- [ ] 物品使用 (双击)

### 2.3 BeltDialog 交互完善

**文件**: `scenes/dialogs/game/belt_dialog.rs`

- [ ] 与背包的物品拖拽交互
- [ ] 物品数量显示
- [ ] 数字键 1-6 快捷使用

### 2.4 ChatDialog 完善

**文件**: `scenes/dialogs/game/chat_dialog.rs`

- [ ] 多频道消息过滤 (综合/私聊/队伍/行会)
- [ ] 消息中物品链接点击
- [ ] 消息中坐标链接点击
- [ ] 表情支持

### 2.5 QuestDialog 完善

**文件**: `scenes/dialogs/game/quest_log_dialog.rs`

- [ ] 任务追踪面板 (右侧)
- [ ] 任务详情显示
- [ ] 任务进度更新
- [ ] 任务完成提示

### 2.6 NPC 商店完善

**文件**: `scenes/dialogs/game/npc_goods_dialog.rs`

- [ ] 商品列表正确显示 (从服务器获取)
- [ ] 购买确认 (数量选择)
- [ ] 出售物品 (从背包拖入)
- [ ] 修理装备
- [ ] 仓库存取

---

## 阶段三：社交系统移植（~3 周）

> 移植组队、好友、行会等社交功能

### 3.1 组队系统

**C# 参考**: `MirScenes/Dialogs/GroupDialog.cs` (226 行)

- [ ] 网络 Handler 补全 (GroupInvite, GroupAccept, GroupLeave, etc.)
- [ ] `GroupDialog` 对话框
  - 队员列表
  - 队长标记
  - 在线状态
  - 邀请/踢出/退出按钮
- [ ] 组队经验分配显示
- [ ] 队员血条显示 (MainDialog 右侧)

### 3.2 好友系统

**C# 参考**: `MirScenes/Dialogs/FriendDialog.cs` (569 行)

- [ ] 网络 Handler
- [ ] `FriendDialog` 对话框
  - 好友列表
  - 添加/删除好友
  - 在线状态显示
  - 私聊入口
- [ ] 私聊消息处理

### 3.3 行会系统

**C# 参考**: `MirScenes/Dialogs/GuildDialog.cs` (2,232 行)

- [ ] 网络 Handler 补全
- [ ] `GuildDialog` 对话框
  - 行会信息
  - 成员列表
  - 行会公告
  - 行会通知
  - 申请/退出/弹劾
- [ ] 行会名称显示 (角色头顶)

### 3.4 师徒系统

**C# 参考**: `MirScenes/Dialogs/MentorDialog.cs` (314 行)

- [ ] 网络 Handler
- [ ] `MentorDialog` 对话框
- [ ] 师徒关系显示

### 3.5 关系系统 (婚姻)

**C# 参考**: `MirScenes/Dialogs/RelationshipDialog.cs` (247 行)

- [ ] 网络 Handler
- [ ] `RelationshipDialog` 对话框

---

## 阶段四：交易系统移植（~1.5 周）

### 4.1 玩家交易

**C# 参考**: `MirScenes/Dialogs/TradeDialogs.cs` (280 行)

- [ ] 网络 Handler 补全 (TradeRequest, TradeAccept, TradeItem, TradeGold, TradeConfirm, TradeCancel)
- [ ] `TradeDialog` 对话框
  - 双方交易区域
  - 物品拖入
  - 金币输入
  - 锁定/确认按钮
- [ ] 交易流程 (请求 → 双方确认 → 完成)

### 4.2 拍卖行

**C# 参考**: `MirScenes/Dialogs/TrustMerchantDialog.cs` (1,563 行)

- [ ] 网络 Handler
- [ ] `TrustMerchantDialog` 对话框
  - 搜索/过滤
  - 寄售/购买
  - 我的交易列表

### 4.3 物品租赁

**C# 参考**: `MirScenes/Dialogs/ItemRentalDialog.cs` + `ItemRentDialog.cs` + `ItemRentingDialog.cs` (819 行)

- [ ] 网络 Handler
- [ ] 租赁对话框

---

## 阶段五：特色系统移植（~3 周）

### 5.1 英雄系统

**C# 参考**: `MirScenes/Dialogs/HeroDialogs.cs` (896 行) + `HeroObject.cs` + `UserHeroObject.cs`

- [ ] 英雄对象 ECS 组件
- [ ] `HeroDialog` 对话框
  - 英雄信息
  - 英雄装备
  - 英雄技能
- [ ] 英雄行为面板 (跟随/攻击/休息)
- [ ] 英雄召唤/收回
- [ ] MainDialog 英雄按钮

### 5.2 宠物系统

**C# 参考**: `MirScenes/Dialogs/IntelligentCreatureDialogs.cs` (1,389 行)

- [ ] 宠物对象 ECS 组件
- [ ] `IntelligentCreatureDialog` 对话框
- [ ] 宠物 AI 行为树
- [ ] MainDialog 宠物按钮

### 5.3 坐骑系统

**C# 参考**: `MirScenes/Dialogs/MountDialog.cs` (263 行)

- [ ] 网络 Handler
- [ ] `MountDialog` 对话框
- [ ] 骑乘/下骑状态同步
- [ ] 坐骑移动速度加成

### 5.4 钓鱼系统

**C# 参考**: `MirScenes/Dialogs/FishingDialog.cs` (385 行)

- [ ] 网络 Handler
- [ ] `FishingDialog` 对话框
- [ ] 钓鱼动画

### 5.5 Buff 显示

**C# 参考**: `MirScenes/Dialogs/BuffDialog.cs` (899 行)

- [ ] `BuffDialog` 对话框
  - 当前 Buff 列表
  - Buff 倒计时
  - Buff 图标
- [ ] 角色状态图标 (中毒/隐身等已有部分)

---

## 阶段六：辅助功能移植（~2 周）

### 6.1 大地图

**C# 参考**: `MirScenes/Dialogs/BigMapDialog.cs` (858 行)

- [ ] `BigMapDialog` 对话框
- [ ] 世界地图显示
- [ ] 传送点标记
- [ ] 当前位置标记

### 6.2 邮件系统

**C# 参考**: `MirScenes/Dialogs/MailDialogs.cs` (1,274 行)

- [ ] 网络 Handler
- [ ] `MailDialog` 对话框
  - 收件箱/发件箱
  - 读取邮件
  - 附件提取
  - 金币/物品附件

### 6.3 公告系统

**C# 参考**: `MirScenes/Dialogs/NoticeDialog.cs` (373 行)

- [ ] `NoticeDialog` 对话框
- [ ] 服务器公告显示

### 6.4 排行榜

**C# 参考**: `MirScenes/Dialogs/RankingDialog.cs` (433 行)

- [ ] 网络 Handler
- [ ] `RankingDialog` 对话框

### 6.5 其他小功能

| C# 文件 | 行数 | 预估 |
|---------|------|------|
| CompassDialog.cs | 64 | 30min |
| HelpDialog.cs | 403 | 1h |
| KeyboardLayoutDialog.cs | 438 | 2h |
| ReportDialog.cs | 75 | 30min |
| RollDialog.cs | 199 | 1h |
| SocketDialog.cs | 123 | 1h |
| TimerDialog.cs | 246 | 1h |
| GuildTerritoryDialog.cs | 361 | 2h |

---

## 阶段七：技术债清理（持续）

### 7.1 UI 架构统一

- [ ] 统一所有 Dialog 的 open/close/toggle 接口
- [ ] 实现 Dialog z-ordering 和 Focus Manager
- [ ] 消除 `transparent_skin` 重复代码
- [ ] `ItemTextureCache` 添加淘汰机制
- [ ] 键盘输入作用域化 (只有顶层 Dialog 响应)

### 7.2 网络完善

- [ ] 优雅关闭 (`Network::shutdown()`)
- [ ] 心跳超时检测
- [ ] 断线重连
- [ ] 密码日志脱敏

### 7.3 性能优化

- [ ] UI 渲染每帧 `get_texture()` 缓存化
- [ ] 聊天消息可见索引缓存
- [ ] NPC 对话文本解析缓存
- [ ] `Health`/`Mana` 改为 `u32` 类型
- [ ] `EquipmentSlot` 枚举替换魔术数字
- [ ] `Inventory`/`Storage`/`QuestInventory` 去重

### 7.4 ECS 完善

- [ ] 拆分 `AnimationSystem` (God System)
- [ ] 拆分 `UIRenderSystem`
- [ ] 统一时间类型 (`f64` from `get_time()`)
- [ ] 提取行为树到独立模块
- [ ] 补全空系统 (ResourcePreloadSystem, SceneSystem, SaveSystem) 或删除

---

## 时间线预估

```
阶段一  基础游戏循环     ██░░░░░░░░░░░░░░░░░░  ~2 周
阶段二  核心 UI 完善     ████░░░░░░░░░░░░░░░░  ~2 周
阶段三  社交系统         ██████░░░░░░░░░░░░░░  ~3 周
阶段四  交易系统         ████████░░░░░░░░░░░░  ~1.5 周
阶段五  特色系统         ██████████░░░░░░░░░░  ~3 周
阶段六  辅助功能         ████████████░░░░░░░░  ~2 周
阶段七  技术债           ██████████████████████ 持续

总计约 13.5 周 (按单人每天投入估算)
```

---

## 风险与注意事项

1. **服务器依赖** — 阶段一需要服务器支持，建议先用 Mock Server 开发
2. **协议对齐** — 网络 Handler 补全需要对照 `mir2_shared` 中的协议定义
3. **资源依赖** — 部分 Dialog 需要特定纹理资源，需确认 `Data/*.Lib` 中包含
4. **UI 一致性** — 新功能应遵循现有 Dialog 风格 (Prguse 纹理索引)
5. **回归测试** — 每完成一个阶段，确保 `cargo run --bin test_login` 和 `cargo run --bin mir2` 正常运行

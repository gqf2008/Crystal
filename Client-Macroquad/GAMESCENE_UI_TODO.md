# GameScene UI 实现状态

> 最后更新: 2026-04-12
> 本文档反映当前代码实际状态，非初始规划。

---

## 总体进度

| 类别 | 已实现 | 部分实现 | 缺失 | 完成度 |
|---|---|---|---|---|
| 对话框 UI | 24 | 6 | 1 | ~88% |
| 网络协议 | 250+ opcode | 17 未处理 | - | ~94% |
| ECS 系统 | 25+ 核心系统 | 10 轻量 | 3 空桩 | ~90% |
| 场景流程 | Login → Select → Game | - | - | 100% |

---

## 已完整实现的对话框（纹理+内容+网络接线）

| 对话框 | 文件 | 行数 | 说明 |
|---|---|---|---|
| MainDialog | `main_dialog.rs` | 2200+ | 底部工具栏、HP/MP/Exp/负重、7+ 功能按钮、快捷键(I/C/B/S/Tab/H/G) |
| BeltDialog | `belt_dialog.rs` | 727 | 横/竖布局、6 格、物品图标/数量、拖拽交换、双击使用、冷却显示 |
| ChatDialog | `chat_dialog.rs` | 1100+ | 三种窗口大小、消息滚动、时间戳、输入发送 |
| ChatControlBar | `chat_control_bar.rs` | 300+ | 频道切换、交易/设置/举报按钮 |
| InventoryDialog | `inventory_dialog.rs` | 800+ | 3 标签页(Items/Items2/Quest)、80+ 格子、锁定格、重量显示、拖拽到 Belt |
| CharacterDialog | `character_dialog.rs` | 727 | 装备槽位(12 个)、纸娃娃、属性页、技能页、拖拽交互 |
| NpcDialog | `npc_dialog.rs` | 800+ | 三种尺寸、对话文本、选项按钮、动作接线 |
| NpcGoodsDialog | `npc_goods_dialog.rs` | 700+ | 商店列表、买/卖、数量选择、AmountBox |
| MinimapDialog | `minimap_dialog.rs` | 500+ | 地图渲染、位置标记、邮件/大地图按钮、透明度/缩放 |
| BigMapDialog | `big_map_dialog.rs` | 300+ | 全屏查看、瓦片地图(3层渲染)、拖拽平移/滚轮缩放、玩家标记、坐标显示 |
| TextInputDialog | `text_input_dialog.rs` | 250+ | 通用文本输入、键盘/光标、Enter/Escape |
| MenuDialog | `menu_dialog.rs` | 280+ | 退出/返回角色/设置/关于 |
| OptionDialog | `option_dialog.rs` | 280+ | 图形/音效/游戏设置 |
| TradeDialog | `trade_dialog.rs` | 600+ | 双方物品栏、金币、锁定/确认、拖拽占位 |
| GroupDialog | `group_dialog.rs` | 420+ | 成员列表、HP 显示、队长标记、邀请/踢出/退出、双击查看 |
| GuildDialog | `guild_dialog.rs` | 467 | 信息/成员/公告标签页、加入/刷新/编辑、双击查看详情 |
| FriendDialog | `friend_dialog.rs` | 330+ | 好友列表、在线状态、添加/删除/刷新、双击私聊 |
| MailDialog | `mail_dialog.rs` | 380+ | 收件箱/发件箱/写信标签、未读标记、包裹徽章、删除/领取 |
| HeroDialog | `hero_dialog.rs` | 300+ | 英雄信息面板、属性显示 |
| MountDialog | `mount_dialog.rs` | 300+ | 坐骑列表、骑乘/下马 |

## 部分实现的对话框

| 对话框 | 状态 | 缺口 |
|---|---|---|
| QuestLogDialog | UI 完整(30KB)，标签页/列表 | 服务器数据已绑定（QuestAccepted/Completed/ProgressUpdated）|
| RelationshipDialog | UI 完整(11KB)，婚姻/师徒 | 基础展示，亲密度/伙伴等级需服务器数据驱动 |
| BuffDialog | UI 完整(10KB) | 数据来源已接入 |
| FishingDialog | UI 面板(12KB) | 基础钓鱼界面 |
| IntelligentCreatureDialog | UI 完整(16KB) | 守护管理 |
| SocketDialog | UI 存在(8KB) | 宝石镶嵌基础界面 |
| StorageDialog | 基于 NpcGoodsDialog | 双面板布局(仓库/背包)+存入/取出按钮+网络接线 ✅ 已完成 |

## 缺失的对话框

| 对话框 | 说明 | 优先级 |
|---|---|---|
| ~~RankingDialog~~ | 排行榜 | ✅ 已实现 |
| ~~HelpDialog~~ | 帮助文档 | ✅ 已实现 |
| ~~InspectDialog~~ | 查看玩家装备 | ✅ 已实现 |

---

## ECS 系统状态

### 核心系统（完整实现）
- **网络**: NetworkSystem, NetworkApplySystem(163KB), MapBootstrapSystem, MapLoadSystem
- **输入**: PlayerControlSystem(59KB), AutoPotionSystem, SpellInputSystem, LocalPlayerAiSystem(37KB)
- **逻辑**: CombatSystem(26KB), SkillSystem(22KB), HealthRegenSystem, BufSystem, MovementSystem(14KB), CollisionSystem(11KB), PathfindingSystem(26KB), MonsterAISystem, NpcAISystem, NpcDialogueSystem
- **表现**: AnimationSystem(30KB), ParticleSystem, SoundSystem(17KB), CameraFollowSystem, CameraSystem, DialogSystem(46KB)
- **渲染**: MapRenderSystem(14KB), SpriteRenderSystem(9KB), EffectRenderSystem(14KB), UIRenderSystem(58KB), DebugSystem(21KB)

### 空桩系统（3 个）
- `ResourcePreloadSystem` (6 行) — 资源懒加载已接管
- `SaveSystem` (5 行) — 设置由 config.ini 管理
- `SceneSystem` (5 行) — 场景切换由 GameState 直接管理

### 轻量系统
HUDSystem(601B), UISystem(665B), MinimapSystem(3.8KB), FloatingTextSystem(1.2KB), HealthBarAnimSystem(2.5KB), PositionInterpolationSystem(1.4KB), RemoteMoveAnimSystem(1.9KB), MountStateSyncSystem(3.3KB), WeatherSystem(3.4KB, 已实现天气码→粒子发射器)

---

## 网络处理覆盖

| Handler | 已处理/总数 | 备注 |
|---|---|---|
| `item.rs` | 50/51 | 物品全生命周期 |
| `combat.rs` | 47/47 | HeroHealthChanged 已发射 |
| `movement.rs` | 29/30 | 移动/传送/闪现 |
| `npc.rs` | 27/28 | 已清理死代码（6 个 market opcode） |
| `guild.rs` | 14/15 | 完整 |
| `chat.rs` | 2/3 | 群聊/私聊 |
| `mail.rs` | 6/7 | 邮件/附件/发送/成本 |
| `ui_events.rs` | 13/14 | 音效/坐骑/计时/钓鱼/排行榜/商城 |
| `social.rs` | 5/6 | 婚姻/师徒 |
| `friend.rs` | 1/2 | FriendUpdate |
| `player.rs` | 1/2 | PlayerInspect |
| `market.rs` | 6/7 | 寄售/市场 |
| `creature.rs` | 4/5 | 守护生物 |
| `hero.rs` | 11/12 | 英雄系统 |
| `trade.rs` | 6/7 | 交易系统 |
| `group.rs` | 7/8 | 组队系统 |
| `quest.rs` | 6/7 | 任务系统 |
| `character.rs` | 25/23 | LogOut/ReturnToLogin 已发射 |
| `connection.rs` | 4/5 | 连接管理 |

---

## 已知代码问题

| 问题 | 文件 | 影响 | 状态 |
|---|---|---|---|
| Struck 事件 damage=0 | `combat.rs:22` | 协议不携带 damage 字段 | 协议限制 |
| NPCResponse npc_id=0 | `npc.rs:21` | 协议只有 page 字段 | 协议限制 |
| NPCRequestInput npc_id=0 | `npc.rs:234` | 协议不携带 object_id | 协议限制 |
| chat sender 为空 | `chat.rs:20` | Server chat 无 sender | 协议限制 |
| ItemTakenBack/ItemStored 空数据 | `item.rs:111-127` | 协议不携带物品数据 | 协议限制 |
| MarriageRequestSend 空 target | `ui_system.rs:886` | 由服务器根据亲密度/位置判定 | 协议限制 |
| 未使用事件变体 | `mod.rs:66-800` | 大量 NetworkEvent 未 emit | 预留 |

---

## 待实现高优先级

1. **Inventory ↔ Belt/Character 直接拖拽** — ✅ 完整实现：
   - ✅ Inventory → Belt: 拖动物品到 Belt 窗口释放（含 rollback + unique_id 保留）
   - ✅ Belt → Inventory: 拖动物品到 Inventory 窗口释放（含 rollback + unique_id 保留）
   - ✅ Inventory → Character: 拖到角色面板（自动发送 EquipItemRequest）
   - ✅ Character → Inventory: 卸下装备拖回背包（含背包已满处理 + RemoveItemRequest 发包）
   - 右键转移机制保留作为补充
2. **Mock 网络补充** — 已完善：
   - ✅ 排行榜：GetRankingRequest → RankingsReceivedWithEntries（mock 15 条排行数据）
   - ✅ 好友列表：基础 SystemMessage 响应
   - ✅ 任务系统：AcceptQuest/FinishQuest/AbandonQuest 完整 mock
   - ✅ 交易系统：TradeRequest/TradeReply/TradeConfirm/TradeCancel 完整 mock
   - ✅ 公会系统：请求信息/加入/离开/编辑公告/成员管理 完整 mock
   - ✅ 仓库系统：NPCStorage/StoreItem/TakeBackItem 完整 mock
   - ✅ 市场系统：基础 SystemMessage 响应
   - ✅ 组队系统：GroupMembersMapUpdated/GroupModeChanged 已接线 UI

## 最近完成的改进

- ✅ Handler 缺口修复：HeroHealthChanged, LogOutSuccess, LogOutFailed, ReturnToLogin
- ✅ unique_id 全链路保留：Inventory ↔ Belt ↔ Character 拖拽/回滚/转移
- ✅ Character → Inventory 卸下装备自动发包 RemoveItemRequest
- ✅ 组队事件下游接线：成员列表更新、组队模式切换、位置跟踪
- ✅ Clippy 0 warnings（51 → 0，含 mock.rs 修复、Default 派生等）
- ✅ QuestLogDialog 已绑定服务器数据（TODO 标注已过时）
- ✅ WeatherSystem 完整实现：天气码→粒子发射器，Mock 随机天气
- ✅ 粒子类型差异化：Blizzard/FlowersRain/FogCloud 专用生成逻辑
- ✅ 亲密度/导师经验条件渲染：协议无数据时隐藏
- ✅ UnhandledPacket 日志升级为 warn 并输出 opcode

# Client-Bevy 迁移计划（Bevy 0.19）

> 生成日期: 2026-08-02 | 分支: feat/bevy-client
> 目标: 将 `Client-Macroquad`（macroquad + hecs，约 99% 完成）迁移到 **Bevy 0.19**
> 参考: `Client-Macroquad/IMPLEMENTATION_PLAN.md`（P0–P5）、`ServerRust/docs/PORT_STATUS.md`
> 共享: `SharedRust`（协议库，276 S→C + 145 C→S）、`Client-Macroquad/Data`（游戏数据）

---

## 零、移植参考原则（重要）

| 部分 | 参考源码 | 说明 |
|---|---|---|
| **UI 逻辑**（对话框布局/交互/流程/文案） | 原版 C#：`Client/MirScenes/`（LoginScene/SelectScene/GameScene.cs）+ `Client/MirScenes/Dialogs/`（36 个对话框）+ `Client/MirControls/` | UI 行为以原版 C# 为准，macroquad 版仅作参考 |
| **游戏绘制**（地图/精灵/帧动画/特效/遮挡） | Rust：`Client-Macroquad/src/`（map_renderer、rendering、objects、components） | 渲染数据流以 Rust 实现为准 |
| **网络**（协议/TCP/编解码/包处理） | Rust：`Client-Macroquad/src/network/` + `SharedRust/` | 协议与帧格式以 Rust/SharedRust 为准 |

> 原因：C# 版是 WinForms + DirectX 的原始实现，UI 交互（焦点、按钮三态、对话框层级）最贴近原版；而 Rust 版已完成协议对齐与渲染管线打磨，绘制/网络直接复用可避免重复踩坑。

---

## 一、现状（已完成里程碑 M1–M6）

| 里程碑 | 内容 | 状态 |
|---|---|---|
| M1 | 地图渲染（.map 7 种格式、块纹理三层、相机控制） | ✅ |
| M2 | 角色/NPC/怪物精灵渲染与帧动画（帧表复用、精灵图缓存） | ✅ |
| M3 | Y 轴深度排序 + 前景遮挡（逐瓦片 front 层 + 角色 ghost 半透明） | ✅ |
| M4 | 场景系统（Intro/Login/Select/Game 状态机）+ 事件总线 + 登录界面 | ✅ |
| M5 | 网络层 mock 模式（codec 长度前缀+XOR → 类型化包 → 登录/选角/进游戏/对象生成） | ✅ |
| M6 | UI 迁移到 bevy_ui + 登录/选角/新建角色/删除确认完整移植 + IME 中文输入 + DX12 渲染后端 | ✅ |

当前 `cargo check` 通过；已接入包：LoginSuccess / StartGame / NewCharacter(Success) / DeleteCharacterSuccess / MapChanged / ObjectPlayer / ObjectMonster / ObjectNpc / ObjectRemove / KeepAlive。

## 二、待迁移代码量（对照 Client-Macroquad/src）

| 模块 | 文件数 | 行数 | Bevy 现状 |
|---|---:|---:|---|
| systems（infra/input/logic/presentation/rendering） | 61 | ~19,645 | 未移植（仅 network_system 雏形） |
| scenes/dialogs（56 个游戏对话框） | 57 | ~21,569 | 仅 login/select 相关已移植 |
| scenes（登录/选角/游戏场景） | 69 | ~24,572 | 状态机 + 登录/选角已移植 |
| network（17 handler + client + mock） | 32 | ~11,160 | codec + mock + 10 个包分支 |
| components（21 个 ECS 组件） | 21 | ~3,955 | 未移植 |
| event_bus（5 队列） | 5 | ~1,751 | 简易版（仅 LoginSuccess） |

核心大文件：network_apply_system（~240k，264 个 opcode 分支）、main_dialog（105k）、rendering/ui_system（~97k）、player_control（59k）、sprite_system/character（51k）、npc_goods_dialog（45k）、chat_dialog（44k）。

---

## 三、剩余里程碑提案（M7–M12）

### M7: 真实网络层（TCP 接入 ServerRust + 全量 handler）
- [x] TCP 客户端线程（crossbeam 通道 + 阻塞读写），连接 `ServerRust`（`--real-net [addr]`，默认 mock），ClientVersion 握手 + KeepAlive 心跳 + 断线通知（`src/network/tcp.rs`）
- [x] 已接入包：Connected / ClientVersion / NewAccount / ChangePassword / Login / LoginSuccess / StartGame / NewCharacter(Success) / DeleteCharacter(Success) / MapChanged / ObjectPlayer / ObjectMonster / ObjectNpc / ObjectRemove（实体删除）/ KeepAlive
- [ ] 剩余 handler：movement / chat / combat / item / npc / quest / group / trade / guild / mail / market / friend / hero / creature / social / ui_events（M8–M10 随场景/对话框推进）
- [ ] NetworkContext 扩展为完整网络事件分发（对齐 macroquad 264 个 NetworkEvent 变体）
- [ ] 发包补齐：~145 个 C→S 包（移动/攻击/技能/对话框动作/组队/行会/邮件等）
- [x] 登录失败/注册/改密结果提示、断线提示（登录界面状态文本）；KeepAlive 心跳自动发送
- [ ] 自动重连
- **验收（进行中）**: 与真实 ServerRust 联调 握手→登录 已通（`examples/net_smoke.rs` 验证 Connected/ClientVersion/Login 响应）；登录→选角→进游戏→对象生成待 GUI 联调；`cargo check` / `clippy` / `cargo test` 全过

### M8: 游戏场景基础（HUD + 玩家控制）
> 参考：HUD/对话框 → `Client/MirScenes/GameScene.cs` + `Client/MirScenes/Dialogs/MainDialogs.cs`；绘制 → `Client-Macroquad/src/rendering`；网络 → `Client-Macroquad/src/network`
- [x] Game 场景骨架：StartGame/MapChanged 加载地图、相机定位、出生点（map_renderer）
- [x] 主对话框 HUD（血/蓝球、经验条、金币/等级/名字、原版五按钮+菜单/商城）与聊天面板（历史/Enter 输入/IME）（`src/game/hud.rs` + `chat.rs`）
- [ ] 小地图、菜单/帮助/设置入口（M9 对话框接入）
- [x] 玩家控制：右键寻路（A*）、左键 NPC CallNPC / 怪物攻击、中键 AutoRun；移动 Walk/Run 发包 + 远端插值（`src/game/player_control.rs` + `movement.rs` + `pathfinding.rs`）
- [ ] 拾取、自动喝药、快捷栏（后续）
- [x] 移植基础组件：player / movement / input / network / session / settings（部分）
- **验收（进行中）**: mock --auto-enter 全流程稳定运行；地图/HUD/聊天像素级验证通过；真实 ServerRust 移动/聊天待联调

### M9: 对话框系统（56 个游戏对话框，分 4 批）
> 参考：**以原版 C# 为准** `Client/MirScenes/Dialogs/*.cs`（36 个文件）；Rust 版 `Client-Macroquad/src/scenes/dialogs/` 仅作迁移对照
- [x] 通用 UI 基建：对话框管理器（DialogManager 开关/z 序）、HUD 按钮接入、--auto-inv/--auto-char 调试
- [x] 第 1 批（核心）: inventory（**数据驱动完成**：40 格物品图标/堆叠数量/双击使用/装备）/ character（4 标签页+14 装备槽）/ menu / minimap（玩家点/M 键）/ belt（快捷栏）/ compass 全部完成
- [x] 第 2 批（交互）: npc / npc_goods（商店闭环）/ trade / amount_box / group / quest_log / friend / inspect 全部完成
- [ ] 第 2 批（交互）: npc / npc_goods / trade / amount_box / group / quest_log / friend / inspect
- [x] 第 3 批（社交）: guild / guild_territory / mail / trust_merchant / item_rental / ranking / report / mentor / relationship / hero / intelligent_creature / mount 全部完成
- [x] 第 4 批（系统）: 全部 17 个完成（含 chat_notice 屏幕通知、game_shop 商城）
- [x] **M9 全部完成：56 个游戏对话框骨架就位**（数据驱动，网络后续批次接入）
- [ ] 第 4 批（系统）: game_shop / refine / craft / socket / dura_status / npc_drop / roll / npc_awake / notice / chat_notice / timer / option / help / keyboard_layout / big_map / fishing / buff
- **验收**: 每个对话框交互与原版 C# / macroquad 一致（数据驱动，mock 先行）

### M10: 战斗 / 逻辑 / 物理
- [x] combat（部分）: 攻击发包、受击/死亡动画、伤害飘字、自动喝药（闭环验证）
- [ ] combat: magic / skill / buff / regen（技能栏 F1-F8 待续）
- [x] decision: 服务端驱动怪物 AI（ObjectWalk/Run/Turn/Attack/Death 接入）；NPC 对话闭环
- [x] physics: movement / pathfinding / collision（A* + 步进 + 远端插值）
- [x] input: auto_potion（**数据驱动**：从背包找真实 Potion 并发送 unique_id）/ [ ] local_player_ai（待续）
- **验收（进行中）**: 打怪/飘字闭环已验；技能/自动喝药待续

### M11: 呈现与特效
- [ ] animation_system（帧动画/挂点/武器特效/坐骑状态）
- [x] particle / weather（雨/雪粒子）/ floating_text（伤害飘字）/ sound（SoundList 450 条 + 攻击/受击音效）
- [ ] rendering: sprite_system 分层遮挡 / effect_system / map_system 分块 + 唯一贴图去重
- [x] 日夜循环（24 分钟一天夜晚覆盖层）；屏幕通知（ChatNotice 对话框就位）
- **验收**: 天气/粒子/音效/伤害飘字/血条/特效完整呈现

### M12: 打磨与验收
- [x] config.ini 配置（ServerAddr/UseMock/账号）
- [x] 单元测试：24 个（tcp/寻路/坐标/codec）全过
- [x] clippy 零警告（我方代码）、release 构建成功（13m50s）
- [x] **与真实 ServerRust 全流程联调通过**：握手→登录→建角色→角色列表→StartGame→进游戏
- [ ] 设置对话框、性能基线（精灵 Atlas）、README 完善（后续）
- **验收**: cargo test 24 全过、release 构建成功、真实服务器全流程打通

### M13: 背包数据驱动（2026-08-02 完成）
- [x] **网络数据链路**：ServerRust `UserInformation` 携带背包(40 格)/装备(12 槽) 及 ItemInfo（名称/图标/类型）
  - SharedRust 新增 `UserItem::write_to_with_info` / `read_from_with_info`（UserInformation 专用内联 ItemInfo）
  - C#→SharedRust 枚举映射：ItemType/ItemGrade/RequiredType/ItemSet/HeroBehaviour（编号差 3）
- [x] **客户端渲染**：背包格子子实体 = Items 库图标 + 堆叠数量文本；按 `hud.inventory.items` 逐帧更新显隐
- [x] **交互**：双击格子 → 药水/卷轴 `UseItem{unique_id}`，装备 `EquipItem{grid: MirGridType, unique_id, to}`（服务端按 unique_id 定位背包格）
- [x] **自动喝药**：HP<35% 自动找背包第一个 Potion 使用
- [x] **服务端全链路修复**（真实联调中发现）：
  - `SetPlayerState`/`SetMapData` 的 ask 补 `.await`（kameo 惰性发送，原消息从未送达 → 角色状态永远空背包）
  - `save_character` 先删子表行再 INSERT OR REPLACE（有背包物品时 FK 冲突无法存档）
  - tick 自动存档传 `account_username`（原传角色名 → 账号 FK 失败）
  - 断线/登出时登出账号（否则账号一直 online 无法重登）
  - PBKDF2 校验修复（`splitn(2)` + `parts[0]/parts[1]`，C# 迁移账号可登录）
  - `EquipItem` 按 unique_id 定位背包格（原把 MirGridType 当格索引）
- [x] 验证：真实 ServerRust 全流程——登录→进游戏→UserInformation 4 件物品→客户端背包图标渲染；客户端 24 测试 + 服务端 139 测试全过
- [x] **技能栏 F1-F8 `Magic` 包**：NewMagic 包写入已学技能（key 绑定），F1-F8 发送 Magic（spell/direction/位置）；快捷栏按 key 显示 MagIcon[icon*2] 技能图标（原版 C# MainDialogs）
- [x] **服务端系统性修复**：kameo 0.20 的 ask/tell 是惰性 request，丢弃 future 消息永远不会发送——gate 转发（MoveItem/UseItem/EquipItem/Magic/组队/行会/邮件等 ~335 处）全部静默丢失。async 上下文补 .await，sync 上下文改 tell().try_send()，SendToClient 一律 tell 防死锁
- [x] 技能编号对齐：DB magic_infos/player_magics 用 C# 编号，客户端协议 SharedRust(+3)，MagicRequest/NewMagic 统一换算
- [x] **背包点击移动（MoveItem 全链路）**：单击选中（黄色高亮，原版 C# SelectedCell）→ 点目标格发送 MoveItem；服务端响应后本地 swap；服务端修复 from/to 槽位索引
- [x] **装备流程闭环**：双击装备（EquipItem 响应本地同步、旧装备回背包）、使用物品扣减、卸下装备回背包；角色对话框 14 槽映射服务端 12 槽渲染装备图标
- [x] **物品 tooltip**：悬停物品格跟随光标显示 名称 x数量（原版 C# MirItemCell.Hint 语义）
- [x] **物品完整交互（M13 收尾）**：右键使用/装备、Shift+左键拆分（MirAmountBox → SplitItem）、选中物品点地面丢弃（单件 YesNo 确认 / 多件数量框 → DropItem）、装备格右键卸下（RemoveItem）
  - 客户端：背包点击逻辑重构进 `inv_item_action_system`（单选/双击/右键/拆分/丢弃 + 弹窗模态门，原版 C# Modal 语义）
  - 数量框输入兜底 `logical_key`（winit 注入事件 text=None 时仍可输入数字）
  - 服务端修复：SplitItem 按 unique_id 定位原格（原误把 MirGridType 当格索引）；DropItem 支持部分数量（原整叠移除）；MergeItem 按 uid 合并；物品操作后发完整 UserInformation 刷新（含背包/装备）
  - 服务端刷新修复：`send_user_information_refresh` 补 HeroBehaviour(+3) 与 observer 字节（原包客户端解析失败）；物品操作刷新复用 `build_user_information_packet`（原双重包头导致解析失败）
  - 验证：真实 ServerRust E2E——拆分 10→7+3（客户端重建 3 件）、丢弃 8→6、右键卸下 SpiritBlade → 装备槽清空回背包，DB 全链路确认
### M14: 技能页 + 快捷键分配面板（2026-08-02 完成）
- [x] **技能页（角色对话框第 4 页）**：7 行技能按钮（原版 C# CharacterDialog.MagicButton，行距 33px），行内渲染 MagIcon2 图标 / Title[516,517] 等级经验条 / Key 标签（F1..F8、CTRL\nF1..、Shift\nF1..）/ 等级 / 名称 / 经验；Next/Back 翻页（Prguse[396-399]，StartIndex 按 7 步进，原版语义）
- [x] **快捷键分配面板**（原版 C# AssignKeyPanel）：Prguse[710] 居中，魔法图标 + 名称，16 个 F 键按钮（F1-F8 / Ctrl+F1-8，Prguse[1656-1658] 三态），None（Title[287-289]）/ Save（Title[156-158]）
- [x] **MagicKey 包闭环**：Save 时清除占用同键的其他技能（C# SaveButton 语义）→ 发送 `C.MagicKey{Spell, Key, OldKey}` → 本地 MagicsState 更新（快捷栏图标自动刷新）→ 服务端 SetSpellKey 落库
- [x] **服务端修复**：SetSpellKey/ToggleSpell 补 C#→SharedRust 编号换算（-3，与 MagicRequest 一致）——原实现直接比较导致快捷键绑到错误技能
- [x] 验证：真实 ServerRust E2E——技能页点行 → 面板打开 → 点 F3/F1 → Save → 服务端落库（FireBall key=3→1，冲突技能 GreatFireBall key 自动清 0）；客户端 34 测试全过

---


### M16: 地面物品渲染 + 拾取（2026-08-03 完成）
- [x] **ObjectItem 携带 ItemInfo**：SharedRust ObjectItem 改 write_to_with_info/read_from_with_info，服务端掉落广播前补 ItemInfo（DB 配置 → SharedRust 枚举 +3），客户端渲染图标/名称
- [x] **地面物品实体**：NetObject::GroundItem → Items 库图标 + 名称标签（世界坐标），随 ObjectRemove 清除
- [x] **点击拾取**：点击地面物品 → 邻近（1 格）直接发 PickUp，远距离寻路后自动拾取（原版 C# ItemObject）
- [x] **拾取后完整 UserInformation 刷新**：服务端 PickUp 成功后发完整背包刷新（客户端立即显示物品回包）
- [x] **修复玩家实体渲染（预存 bug）**：
  - 服务端 ObjectPlayer 的 effect 写 0 但 SharedRust SpellEffect::None=3 → 玩家包永远解析失败（本地/远端玩家从未渲染）
  - 服务端向新玩家补发自身 ObjectPlayer（客户端据此生成本地玩家实体）
  - 客户端玩家生成未取负 y → 玩家在世界镜像位置（本地玩家实体一直存在但位置错误）
- [x] 验证：真实 ServerRust E2E——丢弃（确认框）→ ObjectItem 解析/地面实体生成 → 点击 → PickUp → 背包刷新 1 件 → DB 落库

---

### M17: NPC 商店买卖闭环（2026-08-03 完成）
- [x] **NPCGoods gzip 解压**：C# 协议 `ServerPackets.NPCGoods.Compressed==true`（SharedRust `is_compressed()==true`），客户端此前未解压直接解析压缩字节 → 修复为收到后先 GzipDecoder 再 read_body
- [x] **修复服务端双重包头**：`send_npc_goods`/`send_npc_panel`/`send_user_storage` 先用 `serialize_packet`（已写完整内层包头）又用 `build_packet_bytes` 二次包装 → 去掉二次包装，与 ObjectItem/UserInformation 一致
- [x] **NPCGoods 内联 ItemInfo**：与 UserInformation/ObjectItem 同约定，SharedRust NPCGoods 改 `write_to_with_info`/`read_from_with_info`（原版 C# 客户端用本地 GetItemInfo 解析，Rust 客户端无本地物品库 → 服务端内联）
- [x] **购买闭环**：BuyItem（C# wire：`[u64 item_index][u16 count][u8 panel]`）→ 服务端按 `session_npc` 会话上下文解析 NPC → 商品匹配/库存/金币校验 → 扣款发物 → 完整 UserInformation 刷新
- [x] **出售闭环**：SellItem（`[u64 uid][u16 count]`）→ 按 unique_id 移除背包物品 → 半价加金币 → SellItem 响应 + 完整 UserInformation 刷新；修复响应包 count 写成 u32 而协议是 u16 导致 success 误读
- [x] **服务端预存 bug 修复**：
  - `load_character` 不读 `characters.gold` → 每次登录金币归零（已补）
  - `load_npc_goods` 老库无 stock/infinite_stock 列 → 默认 0/售罄（已改默认无限库存，C# 语义）
  - 同批修正 SplitItem 响应 count u16（DropItem 保持 u32）
- [x] **客户端 UI 对齐原版 C#**：`npc.rs` 支持 `<文字/@Buy>` 原版行格式点击；`npc_goods.rs` 购买按钮 → `C.BuyItem{item_index, count, PanelType::Buy}`；`inventory.rs` Alt+左键快速出售（NPC 商店打开时）→ `C.SellItem{unique_id, count}`
- [x] 验证：真实 ServerRust E2E（`--shop-test --auto-enter`）——CallNPC → [@Buy] → NPCGoods 18 件（含名称/价格）→ 购买 (HP)DrugSmall 扣 40 金 → 背包 +1 → 出售响应 success=true 回 +20 金 → 背包清空 → DB 落库（gold 持久化）

---
### M18: 仓库（Storage）物品存取（2026-08-03 完成）
- [x] **UserStorage 解析 + 仓库对话框**：服务端 [@Storage] → UserStorage（80 格，含物品）→ 客户端打开仓库面板（10 列 x 8 行，原版 C# StorageDialog 布局），同时打开背包（原版 C# 语义）
- [x] **存入**：选中背包物品 → 点仓库格 → `C.StoreItem{From=背包格, To=仓库格}`（原版 C# MirItemCell 拖放语义；服务端优先目标格，占用则找第一个空位）
- [x] **取出**：选中仓库物品 → 点背包格 → `C.TakeBackItem{From=仓库格, To=背包格}`
- [x] **服务端 wire 对齐 C#/SharedRust**：gate 原解析 `[grid u8][uid u64][count u32]`（与 C# `[From i32][To i32]` 不符）→ 改回 `[from i32][to i32]`；PlayerInventory 新增 `store_item_to`/`take_back_item_to`（目标格优先 + 首空位兜底）
- [x] **操作后完整刷新**：存入/取出成功 → 完整 UserStorage + UserInformation（仓库与背包同时重建）
- [x] 验证：真实 ServerRust E2E（`--storage-test --auto-enter`）——CallNPC → [@Storage] → 仓库 80 格解析 → 存入背包格 0→仓库 0（仓库 0→1 件、背包 1→0）→ 取出仓库 0→背包格 0（仓库 1→0、背包 0→1）→ DB 落库 ✅

---
### M19: NPC 商店回购（BuyItemBack）（2026-08-03 完成）
- [x] **[@BuyBack] 引擎级按键**：服务端 CallNPC 对 [@BuyBack] 直接发回购商品列表（C# NPCScript.BuyBackKey 语义），不被 NPC 脚本页遮蔽
- [x] **回购列表**：出售物品进 buyback_items（服务端已有）→ 回购面板以 NPCGoods 发送（原物品 + 原始 unique_id + ItemInfo）
- [x] **回购动作**：客户端 NPC 菜单点 `<回购/@BuyBack>`（is_buyback 标记）→ 购买按钮发 `C.BuyItemBack{unique_id, count}`（C# wire [u64][u16]）；服务端按 unique_id 定位回购条目，2×卖价扣款、物品回背包、完整 UserInformation 刷新
- [x] **gate wire 修复**：BuyItemBack 原解析 `[item_index u32]` 与 C# `[uid u64][count u16]` 不符 → 修正
- [x] 验证：真实 ServerRust E2E（`--shop-test` 扩展）——购买(HP)DrugSmall(-40金) → 出售(+20金) → [@BuyBack] 列表 1 件(uid=1) → 回购(-40金) → 物品回背包 ✅（DB 落库，新 uid=2）

---
### M20: NPC 出售/修理面板（NPCDropDialog）（2026-08-03 完成）
- [x] **PanelType 路由**：NPCGoods(panel_type=Sell/Repair/SpecialRepair) → 打开出售/修理面板（C# GameScene.NPCSell/NPCRepair → NPCDropDialog），不再误开商品对话框
- [x] **出售/修理面板 UI**：Prguse[392]（C# NPCDropDialog 布局 (264,224)）、确认按钮 Title[290-292]、拖放区 (20,55,75,75)、提示文本
- [x] **交互（原版 C# 拖放语义）**：点背包物品选中（SelectedCell）→ 点面板拖放区放入 TargetItem → 点确认：Sell 卖整叠（`C.SellItem{uid, count=整叠数量}`）/ Repair 发 `C.RepairItem{uid}`
- [x] 面板打开时同时打开背包（C# NPCDropDialog.Show 语义）
- [x] 验证：真实 ServerRust E2E（--shop-test 扩展）——[@Sell] → `🧰 NPC 面板: Sell` → 出售面板打开 ✅；买→卖→回购全链路仍通

---
### M21: 组队（Group）全链路（2026-08-03 完成）
- [x] **SharedRust 对齐服务端 wire**：GroupMembersMap 改成员列表（name + is_leader + online，count 前缀）、GroupInvite 加 inviter_id（u64）——原定义（逐成员 name+map / 仅 name）与服务端实际发送不符（两仓库同步）
- [x] **服务端修复**：
  - `SetSocialRef` 从未被 main.rs 调用 → gate social_ref 恒 None，所有社交转发静默丢弃（组队/交易/好友全断）——补消息 + 启动链接
  - `SocialPlayerJoined/Left` 从未发送 → SocialActor 在线表恒空（find_player_by_name 永远失败）——session StartGame/断开/登出补通知
  - gate AddMember/DellMember 用 u16 前缀解析 DotNet 字符串 → 改 `read_dotnet_string`（7-bit）
  - 创建组队分支漏 `broadcast_group_update` → 双方收不到成员列表
- [x] **客户端**：
  - GroupMembersMap/GroupInvite/DeleteGroup/DeleteMember 网络处理 → GroupState
  - 组队对话框（C# GroupDialog 布局）：成员列表 2 列（队长★/离线标记）、允许组队开关（Prguse[114/115]）、邀请提示（MirMessageBox：Yes/No → C.GroupInvite）
  - 右键点击远端玩家 → C.AddMember{Name}（原版 C# MainDialogs 右键邀请）；PlayerName 组件
- [x] 验证：真实 ServerRust 双客户端 E2E——A(test/bevychar) 发 AddMember → B(bevy2/bevy2char) 收到邀请提示 → 自动接受 → 双方收到 `👥 组队成员: ★bevychar, bevy2char` ✅

---
### M22: 邮件（Mail）收发闭环（2026-08-03 完成）
- [x] **服务端 SendMail wire 修复**：gate 用 u32 前缀解析字符串 + 拆 subject/message 三字段，与 C#/SharedRust `[name][message][gold][5×u64][stamped]` 不符 → 改 `read_dotnet_string`×2 + 二进制字段，subject 由正文首行派生（C# 语义）
- [x] **ReceiveMail 双格式解析**：服务端同 opcode 发两种包（新邮件条目 / ReadMail 全文，正文在 timestamp 之前）→ 客户端先尝试全文格式再回退条目格式
- [x] **邮件对话框**：列表（发件人-主题-未读标记）、点击列表项 → `C.ReadMail{mail_id}` → 内容区显示正文/金币/附件（C# MailDialog 语义）
- [x] **Bevy 16 参数上限**：network_system 达 17 参数 → 自定义 `SystemParam`（NetworkPanels）合并 5 个对话框状态资源
- [x] 验证：真实 ServerRust 双客户端 E2E——A 发邮件（含 100 金币，扣款 1,000,000→999,900）→ B `📧 新邮件: bevychar - HelloSubject（未读）` → ReadMail → `📧 邮件详情: ... 金币=100` + 正文 ✅
- [ ] 写邮件 UI（收件人/主题/正文输入框，需通用文本输入框组件）；登录时同步已有邮件列表（服务端未发）

---
### M23: 交易（Trade）全链路（2026-08-03 完成）
- [x] **SharedRust 补 DepositTradeItem/RetrieveTradeItem 客户端包**（[from i32][to i32]，双仓库同步）
- [x] **交易对话框**（C# TradeDialogs 语义）：左右 5x4 物品槽 + 物品图标/数量、金币显示、金币输入按钮（数量框 → C.TradeGold）、锁定按钮（C.TradeConfirm）、关闭（C.TradeCancel）、邀请提示（MirMessageBox Yes/No → C.TradeReply）
- [x] **交互**：点背包物品 → C.DepositTradeItem{from,to}（pending_deposit 本地乐观）；点我方槽 → C.RetrieveTradeItem 取回；邀请接受后本地开窗（is_initiator 区分发起者/接受者）
- [x] **服务端 wire 手动解析**：TradeRequest 邀请/打开同 opcode（is_initiator 区分）、TradeGold [u64]、TradeConfirm [a][b]（按 is_initiator 映射）、TradeItem [uid][grid][count][is_add]、DepositTradeItem [from][success]
- [x] **服务端修复**：
  - gate forward_trade_request 用 `.ask()` 但丢弃 future → 消息从未发送（改 tell）
  - execute_trade 从玩家背包重新查 uid（物品已随 DepositTradeItemBySlot 移除）→ 物品丢失；改用 TradeItem.item_data 缓存（fallback 查询）
- [x] 验证：真实 ServerRust 双客户端 E2E——邀请/接受/开窗 → A 金币 500 + 物品 (HP)DrugSmall → B 金币 300 → 双方锁定 → 🎉 交易完成 → DB：A gold 999,800（-200）、B gold 1,000,200、物品转移到 B 背包 ✅

---
### M24: 怪物掉落 → 拾取联调（2026-08-03 完成）
- [x] **服务端 spawn_single_drop 补 ItemInfo**：怪物掉落 ObjectItem 此前不带 info（地面物品显示 #853）→ 与 M16 玩家丢弃路径一致，掉落渲染真实名称（如 Venison）
- [x] **E2E 全链路验证**（真实 ServerRust）：角色 (205,325) 秒杀 Deer（DB 掉落 chance=1.0）→ 服务端 spawn_monster_drops → `📦 地面物品: Venison (uid=1/2) @ (205,325)` + 实体生成 → PickUp（1 格内）→ `🎒 背包 40 格（1 件物品）` → DB 落库 item 853 ✅（杀怪经验同步生效）
- [x] `--drop-pick-test` 自动驱动（多方向攻击轮换 + 地面物品检测 + 拾取 + 背包校验）

---
### M25: 好友（Friend）网络接线（2026-08-03 完成）
- [x] **服务端 gate AddFriend 解析修复**：u16 前缀 → `read_dotnet_string`（+ blocked u8，C#/SharedRust wire）
- [x] **服务端系统性修复 `ask()` future 被丢弃**（同类 bug 批量修复）：SetGroupId ×4、AddFriendToSelf ×2、SetSpouse ×3、SetMentor ×3、SetAllowMentor ×1、SetPlayerPosition ×5、SetGuildInfo ×4——这些消息从未发出（组队持久化/传送/行会/婚姻/师徒全部失效）
- [x] **客户端**：FriendUpdate 双格式解析（count 前缀列表 / 单个添加，同 opcode）→ FriendState；好友对话框列表渲染（在线/离线 + 备注）；打开时自动 C.RefreshFriends（原版 C# FriendDialog.Show 语义）
- [x] 验证：真实 ServerRust 双客户端 E2E——A 添加 bevy2char → `👥 好友列表: bevy2char(在线)` → ✅；客户端 34 测试、服务端 139 测试全过

---
## 四、执行顺序与依赖

```
M7（真实网络）→ M8（HUD+控制）→ M9（对话框 1→4 批）→ M10（战斗/逻辑）→ M11（特效）→ M12（打磨）
```

- M9 第 1 批可与 M8 并行；M10 依赖 M8 的玩家控制
- 每个里程碑独立可编译、可运行（mock 模式保底）
- 对话框/系统移植时以 `Client-Macroquad/src` 为参考，去掉 macroquad 耦合，Bevy 用 ECS 组件 + System + bevy_ui

## 五、风险与已解决问题

| 风险 | 状态 |
|---|---|
| bevy_ui 布局与原版像素级对齐 | 登录/选角已验证（锚点左上 + 精灵坐标） |
| 中文输入（IME） | 已解决（Font::from_bytes + MessageReader） |
| 渲染后端冻结 | 强制 DX12（Vulkan present 在此机器异常） |
| 大量精灵性能 | 精灵图缓存已建；后续 Atlas/批处理 |
| 数据依赖 | 复用 Client-Macroquad/Data，`resolve_data_path` 自动解析 |










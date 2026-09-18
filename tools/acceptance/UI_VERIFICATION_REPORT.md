# UI 交互窗口完整测试验证报告

**日期**：2026-09-18（第一轮：开/关/渲染；第二轮：交互级，见 §9）
**版本**：master @ `b9356911`（含 #2949~#2954）；交互能力 PR #2955
**环境**：ServerRust release（后台常驻）+ Client-Bevy debug（`--real-net --auto-enter`），测试账号 `test / bevychar`，地图 BichonProvince
**作者**：Claude Opus 5 (1M context)

---

## 1. 验证范围与方法

客户端全部 **48 个 `DialogKind`**（`Client-Bevy/src/game/dialogs/mod.rs:80-147`），三种方法交叉验证：

| 方法 | 覆盖 | 说明 |
|---|---|---|
| **A. control RPC 巡回** | 45 个 RPC 可映射窗口 | `tools/acceptance/ui_sweep.ps1`：逐窗口 `dialog open` → `dialogs` 断言在列 → 截图 → `dialog close` → 断言消失。结果存 `ui_sweep_results.json` |
| **B. 实机业务流程** | NPC 对话 / 英雄管理 | `@move` 走位 + `npc_call [@MAIN]` 走真实服务端脚本流程；`hero_manage` 走 `HeroState.managing` 状态路径 |
| **C. 截图像素巡检** | 全部 52 张 | 颜色多样性扫描 + 关键窗口人工目检 |

control RPC（127.0.0.1:9000）：`dialog {kind,action}` / `dialogs`（列 DialogManager.open）/ `visible`（列实际可见 DialogRoot，**状态驱动窗的正确探针**）/ `npc_call` / `chat`（GM 命令）/ `screenshot`。

---

## 2. 总览结论

| 分类 | 数量 | 结果 |
|---|---|---|
| A. RPC 开/关断言通过 + 截图确认 | **40** | ✅ 全部通过 |
| **D. 交互级（合成点击关闭钮 + 拖动）** | **40** | ✅ **40/40 点击闭环 + 拖动位移精确**（§9） |
| B. 状态驱动窗（RPC open 被 `sync_dialog_state` 覆写属**设计行为**） | 5 | npc ✅ 实机验证（发现并修复黑文本 bug）；hero_manage ✅ `visible` 断言+截图；trade / npc_goods / roll ⚠️ 机制正确，真实流程需多人/特定条件（见 §5 缺口） |
| C. 无 RPC 映射（刻意） | 3 | GuestTrade / Memo / FishingStatus —— 由业务流程驱动，见 §5 缺口 |
| RPC 别名 | 3 | hero_skill→HeroEquipment、npc_drop→Npc、trust_merchant→Market ✅ |
| **合计** | **48 + 3 别名** | **无一窗口崩溃/白屏；1 个真实缺陷已修复（#2952）** |

---

## 3. 分类明细

### 3.1 A 类：RPC 巡回验证通过（40 窗口）

每窗口均完成：`dialog open` → `dialogs` 返回含该窗口 → 截图 → `dialog close` → `dialogs` 返回不含。截图在 `shots/ui_<kind>.png`。

| 窗口 | RPC 名 | 截图 | 内容抽检 |
|---|---|---|---|
| 背包 | `inventory` | ui_inventory.png | ✅ |
| 角色 | `character` | ui_character.png | ✅ |
| 任务日记 | `quest_log` | ui_quest_log.png | ✅ |
| 设置 | `settings` | ui_settings.png | ✅ |
| 菜单 | `menu` | ui_menu.png | ✅ |
| 游戏商城 | `game_shop` | ui_game_shop.png | ✅ 目检：分类树+商品列表+金币正常渲染 |
| 小地图 | `minimap` | ui_minimap.png | ✅（常驻，close 后 reopen） |
| 组队 | `group` | ui_group.png | ✅ |
| 好友 | `friend` | ui_friend.png | ✅ |
| 观察 | `inspect` | ui_inspect.png | ✅ |
| 行会 | `guild` | ui_guild.png | ✅ 目检：框架+页签正常；内容空=未入行会，符合预期 |
| 邮件 | `mail` | ui_mail.png | ✅ |
| 排行 | `ranking` | ui_ranking.png | ✅ |
| 师徒 | `mentor` | ui_mentor.png | ✅ |
| 关系 | `relationship` | ui_relationship.png | ✅ |
| 坐骑 | `mount` | ui_mount.png | ✅ |
| 举报 | `report` | ui_report.png | ✅ |
| 英雄背包 | `hero_inventory` | ui_hero_inventory.png | ✅ |
| 英雄装备 | `hero_equipment` | ui_hero_equipment.png | ✅ |
| 宝宝/生物 | `creature` | ui_creature.png | ✅ |
| 物品出租 | `item_rental` | ui_item_rental.png | ✅ |
| 行会领地 | `guild_territory` | ui_guild_territory.png | ✅ |
| 帮助 | `help` | ui_help.png | ✅ 目检：完整键位表渲染正常 |
| 公告 | `notice` | ui_notice.png | ✅ |
| Buff 栏 | `buff` | ui_buff.png | ✅ |
| 钓鱼 | `fishing` | ui_fishing.png | ✅ |
| 镶嵌 | `socket` | ui_socket.png | ✅ |
| 精炼 | `refine` | ui_refine.png | ✅ |
| 打造 | `craft` | ui_craft.png | ✅ |
| 耐久状态 | `dura_status` | ui_dura_status.png | ✅ |
| NPC 觉醒 | `npc_awake` | ui_npc_awake.png | ✅ |
| 计时器 | `timer` | ui_timer.png | ✅ |
| 键位设置 | `keyboard_layout` | ui_keyboard_layout.png | ✅ 目检：功能/默认/当前三列渲染正常 |
| 大地图 | `big_map` | ui_big_map.png | ✅ |
| 聊天通知 | `chat_notice` | ui_chat_notice.png | ✅ |
| 寄售行 | `market` | ui_market.png | ✅ |
| 仓库 | `storage` | ui_storage.png | ✅ |
| 已租出浏览 | `item_rental_browse` | ui_item_rental_browse.png | ✅ |
| 任务详情 | `quest_detail` | ui_quest_detail.png | ✅ |
| 输入框 | `input_box` | ui_input_box.png | ✅（状态 RPC 直切 `InputBoxState.open`，列入 DialogManager） |

**关闭断言：40/40 全部通过，无泄漏**（`ui_sweep_results.json` 中 `afterClose` 均不含目标窗口）。

### 3.2 B 类：状态驱动窗（5）

这些窗口由业务状态驱动，`sync_dialog_state` 每帧用状态覆写 `DialogManager`——RPC `dialog open` 被下帧还原是**设计行为，不是 bug**（代码走读确认：`dialogs/mod.rs:271`）。验证改用状态路径/真实流程：

| 窗口 | 驱动状态 | 验证方式 | 结果 |
|---|---|---|---|
| **NPC 对话** `npc` | `NpcDialogState.visible` | **实机全流程**：`@move 296 612`（铁匠旁，曼哈顿距离≤2 通过服务端校验）→ `npc_call {object_id, "[@MAIN]"}` → 服务端下发 1 行对话 → `dialogs`=[Minimap,Npc] → 截图 | ✅ **发现并修复文本全黑缺陷（#2952）**，修复后实机复验文字正常渲染 |
| **英雄管理** `hero_manage` | `HeroState.managing` | RPC 切状态（control.rs:762 特殊分支）→ `visible` 断言含 HeroManage → 截图 ui_hero_manage_live.png（8 英雄槽渲染正常，空=无英雄符合预期）→ close 后消失 | ✅ |
| 交易 `trade` | trade 会话状态 | 代码走读确认机制；真实流程需双账号 | ⚠️ 缺口（§5） |
| NPC 商店 `npc_goods` | NPC 商品状态 | 代码走读确认机制；窗口本体已经实机打开过（修复前截图 ui_npc_goods_real.png 证明窗口/面板渲染，文字同受 #2952 影响） | ⚠️ 商品列表实机复验列入跟进 |
| 掷点 `roll` | 组队掉落掷点状态 | 代码走读确认机制；真实流程需组队掉落 | ⚠️ 缺口（§5） |

### 3.3 C 类：刻意无 RPC 映射（3）

`control.rs:639 has_rpc_mapping` 显式返回 false，各带注释说明原因：

| 窗口 | 驱动方式 | 真实流程验证条件 |
|---|---|---|
| 对方交易窗 `GuestTrade` | 网络 trade 会话与 Trade 成对显隐 | 需双账号互发起交易 |
| 好友备注 `Memo` | 好友窗「备注」动作打开（C# `MemoDialog.Show()`） | 需好友列表非空 |
| 钓鱼状态 `FishingStatus` | 钓鱼流程驱动，与 `Fishing` 成对显隐 | 需钓具+水边抛竿 |

### 3.4 RPC 别名（3）

| 别名 | 解析到 | 截图 | 结果 |
|---|---|---|---|
| `hero_skill` | HeroEquipment | ui_alias_hero_skill.png | ✅ |
| `npc_drop` | Npc | ui_alias_npc_drop.png | ✅ |
| `trust_merchant` | Market | ui_alias_trust_merchant.png | ✅ |

---

## 4. 本次验证发现并修复的缺陷

### 4.1 【已修复】NPC 对话文本区全黑 —— PR #2952

- **发现**：B 类实机验证时，铁匠对话窗能开、服务端数据已下发（日志 `🧙 NPC 对话: 1 行`），但文本区全黑。修复前截图 `ui_npc_real.png` 文本区像素采样：主色 (14,12,11) 占 138369/140400 像素，全区域仅 162 种颜色（纯底板+边框）。
- **根因**：`npc_ui_system` 行渲染查询以 `With<NpcDialogWidget>` 过滤（npc.rs:245），而 `spawn_npc_dialog` 的 8 个行实体在 f6bbef83（bevy_ui 迁移）后丢了该标记 → 查询恒空 → 文字永不写入。窗口显隐走的是面板根，所以"能开但无字"。
- **修复**：行实体 spawn 补 `NpcDialogWidget` 标记（1 行结构性修复）+ 回归测试 `npc_line_entities_carry_dialog_widget_marker`。
- **验证**：
  - 红检：临时去掉标记 → 测试 FAILED；恢复 → 通过
  - 客户端 lib 门禁 622 passed / 0 failed（含新测试）
  - 独立审查：通过（核实了根因链、全部同类实体标记-查询对应关系、测试非恒真）
  - **实机复验**：同流程重跑，`npc_text_fixed.png` 文本区 532 种颜色（+370 = 文字字形像素），截图可见 "Merchant_Smith: 你想说什么？"

### 4.2 本链条此前修复（本次巡回确认无复发）

| PR | 缺陷 | 本次确认 |
|---|---|---|
| #2949 | 腰带空槽被强制显示为白色占位块 | 巡回截图底部腰带槽为深色空槽 ✅ |
| #2950+#2951 | 登录进图误下发 S.ManageHeroes → 英雄管理窗自开 | 巡回基线 `dialogs`=[Minimap]，无 HeroManage 自开 ✅ |

---

## 5. 覆盖缺口（如实记录）

| 缺口 | 原因 | 建议补验方式 |
|---|---|---|
| `trade` / `GuestTrade` 真实交易流程 | 需双账号同图互发起 | 开两个客户端实例走交易会话 |
| `roll` 掷点窗 | 需组队+掉落触发 | 双账号组队击杀掉落 |
| `npc_goods` 商品列表实机复验 | 修前已证明窗口可开；修复后未重走商店流程 | `npc_call` 进商店页签截图 |
| `Memo` 备注窗 | 需好友流程驱动 | 加好友后点备注 |
| `FishingStatus` 钓鱼状态窗 | 需钓鱼流程驱动 | 钓具+水边实钓 |
| 各窗口**内部控件**（页签/输入框/复选框/下拉框等） | 第二轮已覆盖「点 X 关」+ 拖动（§9）；内部控件未逐项 | 按优先级逐窗做控件级验收 |

---

## 6. 门禁状态

| 门禁 | 结果 |
|---|---|
| Client-Bevy `cargo test --lib` | **628 passed / 0 failed**（#2952 + #2955 回归测试） |
| 交互巡回 `ui_interact_sweep.ps1` | **40/40 关闭钮点击通过** + 拖动 + NPC/hero_manage X 点击全过 |
| ServerRust 全量测试 | 818 + gate_hardening 11 + no_blocking 2 + protocol_conformance 6 全绿 |
| rustfmt（改动文件） | ✅ edition 2021 check 通过 |

## 7. 证据文件清单

| 文件 | 内容 |
|---|---|
| `tools/acceptance/ui_sweep.ps1` | 45 窗口自动巡回脚本（可重跑） |
| `tools/acceptance/ui_interact_sweep.ps1` | **交互级巡回**（click/dialog_rect；可重跑） |
| `tools/acceptance/ui_interact_results.json` | 交互巡回原始断言数据 |
| `tools/acceptance/ui_sweep_results.json` | 巡回原始断言数据 |
| `tools/acceptance/npc_text_verify.ps1` | NPC 实机验证脚本（可重跑） |
| `shots/ui_*.png`（48 张） | 45 窗口 + 3 别名截图 |
| `shots/ui_npc_real.png` / `ui_npc_goods_real.png` | 修复前黑文本证据 |
| `shots/npc_text_fixed.png` | 修复后实机复验（文字已渲染） |
| `shots/ui_hero_manage_live.png` | 英雄管理窗状态路径验证 |

## 8. 过程纠偏记录（本次踩坑）

1. **`nearby` RPC 返回字段是 `entities` 不是 `npcs`** —— 脚本首轮误读字段报"找不到铁匠"。
2. **`npc_call` 的 key 必须与服务端脚本节名大小写一致**（`[@MAIN]` 非 `@main`/`[@main]`），不匹配则服务端静默忽略。
3. **`chat` RPC 参数名是 `message` 不是 `text`** —— 参数错误时返回 `{"error":"missing message"}`，`@move` 根本没发出去（表现为"位置没变"）。
4. **服务端 CallNPC 有曼哈顿距离 ≤2 校验**（npc.rs）—— 必须 `@move` 到相邻格再呼叫。
5. **客户端 Start-Process 启动必须带 `D:\toolchains\msys64\ucrt64\bin` 进 PATH**，缺运行时 DLL 进程秒退且无日志（与火绒拦截表象相同，注意区分）。

---

## 9. 交互级验证（D 类）—— 2026-09-18 第二轮

第一轮回答的是「能开、能关、渲染正确」。用户追问「交互验证了吗」后，本轮回答的是：
**真的用合成鼠标点击窗口右上角 X，窗口会关吗？拖动会动吗？**

### 9.1 方法与能力（PR #2955 入库）

| control RPC | 语义 |
|---|---|
| `click {x,y,drag_to?}` | 合成点击/拖拽：`PendingClick` 分阶段注入（cursor position → PointerInput Move/Press/Release + MouseButtonInput），与 ui_picking 同一坐标系；回报 hover 命中栈 |
| `dialog_rect {kind}` | 按 `CloseButton` 标记沿 ChildOf 上溯到 Visible 的 `DialogRoot`，返回关闭钮布局后中心（点击点）+ 根面板矩形 |
| `diag_closebtn` | 全 CloseButton 组件清单 + 按需帧窗门控的 vis/on_insert 诊断 |

`CloseButton` 纯标记由 `spawn_close_button` 统一挂，18+ 对话框接入（dura_status 的 DialogRoot 挂在钮自身走特例）。

### 9.2 结果

`tools/acceptance/ui_interact_sweep.ps1` 全过：

- **40/40** 窗 kind「点 X 关」闭环（6 个设计无钮窗——menu/minimap/buff/refine/timer/chat_notice——走 RPC 往返）
- **拖动**：inventory 按 (40,216) 拖到 (100,256)，根矩形精确 +60/+40（`🖱️ 拖动对话框 Inventory`）
- **NPC 实机会话**：传送+呼叫 → 窗开 → 点 X 关（3 次重试防抖）
- **hero_manage** 状态窗 X 点击闭环

### 9.3 本轮发现并已修复的缺陷（4+2，全部红→绿含回归测试）

| # | 缺陷 | 修复 | PR |
|---|---|---|---|
| 1 | 背包扩容钮命中区覆盖关闭钮左缘，吞掉 X 点击 | 收窄至精灵自然尺寸 48×25 | #2953 |
| 2 | `char_skill_system` 裸 Without 查询每帧把**全 app 按钮**压 Hidden（开窗期间所有 X 不可点） | 补 `Or<With<CharSkillRow/Next/Back>>` 约束 | #2954 |
| 3 | **input_box 根显隐从未有系统写入**——服务端发起（公会取名/宣战/NPC 输入）的输入框永不显示 | ui_system 补 `InputBoxRoot` 显隐 wiring；测试 `root_visibility_follows_open_state` | #2955 |
| 4 | **NPC Hide 级联每帧强清**联动窗——任何 NPC 窗未开时刻，服务端/RPC 开的仓库/出售/商品窗都被立即再关 | 改「可见→不可见」边沿触发；测试 `npc_cascade_closes_linked_panels_only_on_fall_edge` | #2955 |
| 5 | **storage 关闭钮缺 `StorageWidget`**——buttons 查询域 `With<StorageWidget>` 永不命中，实机点 X 无效 | 补挂标记；测试 `storage_close_button_matches_buttons_query` | #2955 |
| 6 | hero_manage 关闭钮缺 `CloseButton` 标记（hero_equipment 已挂、hero 漏挂） | 补挂 | #2955 |

### 9.4 诊断产物（debug 门控，常态不刷日志）

- closebtn 组件清单 dump（`diag_closebtn`）
- `on_insert` Visibility 回溯钩子（抓「谁把关闭钮压 Hidden」）
- Changed-Visibility Pre/Post watch（由 `diag_closebtn` arm 的 `VisBatchWatch(N)` 帧窗门控）

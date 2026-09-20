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

### 6.1 交互巡回已进「门禁」（2026-09-20 更新）

§6 那一行的「40/40」当时是**人工跑一遍的记录**，不是门禁：脚本硬编码主检出绝对路径
（在 worktree 里跑会静默测另一份二进制）、且无论红绿都 `exit 0`（结论只存在于打印里），
也没被任何脚本/CI 调用——**回归时没人拦得住**。现已改成门禁形态：

| 项 | 现状 |
|---|---|
| 退出码 | `0` 全过 / `1` 有用例 FAIL / `2` 前置失败；`-FailOnSkip` 可让 SKIP 也算失败 |
| 产物来源 | 按 `-RepoRoot`（默认脚本所在检出）解析；产物比源码旧 → `exit 2`；`CARGO_TARGET_DIR` 也认 |
| 前置自检 | Data 资产、服务端就绪、进图失败（含日志尾）各自 `exit 2`，不再"跑出一堆假 FAIL" |
| 覆盖清单 | `tools/acceptance/interact_sweep_manifest.json`（单一真源）；`Client-Bevy` 的 `control.rs::interact_sweep_manifest_covers_all_rpc_kinds` 与 RPC 窗口登记对账，漏登记 → `cargo test --lib` 红 |
| 计数口径 | 逐窗 40 + 拖动 + NPC 会话窗 + hero_manage 全部计入账本（旧版把 NPC/hero_manage 只打印、不计入「41/41」，红了也不影响结论） |
| 接入 | `pwsh scripts/run_real_e2e.ps1 -IncludeInteractSweep` 追加巡回并把它计入退出码；`AGENTS.md` 与 `docs/DELIVERY.md` §5.3 已列入 PR 门禁 |

**本报告里引用的其余实机脚本不在仓库内**（`.gitignore` 未放行，`git ls-files` 可验）：
`ui_sweep.ps1`、`ui_bugfix_verify.ps1`、`ime_rpc_verify.ps1`、`npc_text_verify.ps1`
——它们的 45/45、18/18、10/10 等数字**在干净 checkout 上不可复现**，只能算当时那台机器上的记录。
目前唯一入库的实机脚本是 `ui_interact_sweep.ps1`（即本节的这个门禁）。

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

> 本表是**当时**的产物清单。其中 `ui_sweep.ps1` / `npc_text_verify.ps1` 等**不在仓库内**
> （`.gitignore` 未放行），干净 checkout 上不可复现；入库且已门禁化的只有
> `ui_interact_sweep.ps1`（用法与退出码见 §6.1）。

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
| 6 | hero_manage 关闭钮缺 `CloseButton` 标记（hero_equipment 亦同批补挂） | 补挂 | #2955 |

### 9.4 诊断产物（debug 门控，常态不刷日志）

- closebtn 组件清单 dump（`diag_closebtn`）
- `on_insert` Visibility 回溯钩子（抓「谁把关闭钮压 Hidden」）
- Changed-Visibility Pre/Post watch（由 `diag_closebtn` arm 的 `VisBatchWatch(N)` 帧窗门控）

---

## 10. 用户实测 6 项缺陷验收（issue #2961）—— 2026-09-19

用户实测报出 6 个 UI 缺陷，逐项修复后做**实机复验**。本节是本批次的验收依据。

**版本**：master @ `7726c529`（本批**修复 PR 共 16 个**：#2962~#2973 与 #2976~#2981，另有报告自身 #2974/#2975；#2980 已撤回。本节数字均取自该 sha 重建的二进制）
**环境**：ServerRust release（后台常驻）+ Client-Bevy debug（`--real-net --auto-enter`），测试账号 `test / bevychar`，地图 BichonProvince
**方法**：每项都要「单元测试红→绿」+「实机可判真假的断言」，实机断言优先取 control RPC 的**真值字段**，像素只作字形类证据。

### 10.1 结论

| # | 用户原话（摘要） | 状态 | 实机判据 |
|---|---|---|---|
| 1 | 坐骑没有像英雄一样实现遮挡半透明 | ✅ 已修已验 | 骑乘走过树冠，坐骑以**半透明残影**显示而非消失（`shots/mount_ghost_evidence.png`） |
| 2 | 底部的信息输入/出框居然可以被拖动 | ✅ 已修已验 | 面板内起点 (400,700) 拖到 (700,250)：玩家 tile 不变 + 面板左边框锚定区像素原位 |
| 3 | 中文输入法候选词是乱码 | ✅ 已修已验 | 候选条 `nihao 1.你好 2.你 3.尼 4.呢 5.泥 6.妮 7.拟 8.逆 9.倪` 字形正常（`shots/ime_rpc_3_candidates.png`）；输入框 `> 你好`（`shots/ime_rpc_4_committed.png`）；聊天记录 `[bevychar]: 你好`（`shots/chat_sent_evidence.png`） |
| 4 | 商场窗口里的内容错位 | ✅ 已修已验 | 面板 @(164,146)；按 C# 坐标点格 0/格 3/格 4/分类行 → 4/4 命中 `root=GameShop` |
| 5 | 好多窗口滚动条好像都没实现 | ✅ 已修已验 | 滚轮注入 + `scroll` 读**真值**：邮件列表注入 1 格 → `offset 0 → 1`（恰为 `step=1`）；负控（所有列表之外滚）offset 不变。另有商城分类条/行会双列表截图存档 |
| 6 | 鼠标拖拽窗口事件会穿透到游戏中 | ✅ 已修已验 | 拖腰带/点拖后腰带：玩家不移动；正控制空白点仍可走（证明没被一刀切拦死） |

**6 项全部修复并实机验证通过。** 项 5 最初只有截图人工目视（独立审查据此判为弱证据），#2978 把滚轮/滚动条做成可注入、可读真值的 RPC 后升级为**真值断言**。实机脚本汇总：`ui_bugfix_verify.ps1` **18/18**、`ime_rpc_verify.ps1` **10/10**、`ui_interact_sweep.ps1` **41/41**。

### 10.2 逐项证据

**项1 坐骑遮挡半透明** — 根因：`attach_mount_layer` 只挂 `SpriteLayer`，未像 `attach_player_layers` 那样同挂 `GhostLayer`，遮挡系统查询不到残影层 → 走到建筑/树后被整个剔除。修 #2965（审查又抓出 `MountUpdated` 下马主路径 ghost 泄漏，一并修）。
实机复验（本轮补齐）：`@make LeatherBridle 1` + `@make Saddle 1` → `@ride`（**客户端**日志 `client_bevy::actor::spawn`：`🐴 玩家 24495 骑乘坐骑 type=0`）→ 骑乘走过树冠 → **骑手与虎体以半透明残影压在树叶之上**，不再消失。

**项2 聊天窗可拖** — 根因：曾把 C# `MainDialogs.cs:697` 的 `Movable = true` 误读成整窗可拖，实际那是滚动滑块 `PositionBar`。修 #2966（移除整窗拖动 + 补 `WindowDragState::unregister` 契约）。
实机复验：面板内拖动玩家不移动；面板左边框锚定区像素原位；正控制（面板外世界点）仍可走。

**项3 IME 候选乱码** — 三层根因逐层修：
① 候选条标签用 Arial + parley Han 回退（只在首次排版生效）→ 候选全豆腐，修 #2967；
② **聊天面板**同样豆腐——`chat.rs` 是最后一个把 Arial 当**中文正文**主字体的模块（`grep load_ui_font` 仍有约 50 处调用，多为纯拉丁/单次排版站点），输入框 `> □□`、聊天记录 `[玩家]: □□`，修 #2971；
③ Enter 开框当帧把回车符写进草稿（`opened_trigger` 漏登记 Enter 路径），光标多算一位，修 #2973。
实机判据全部取自 RPC 真值：`ime_composing == "nihao"`、`chat_input_text == "你好"`、`chat_input_text` 在中文输入过程中恒为空（无裸 ASCII 泄漏）。

**项4 商城内容错位** — 修 #2968 按 C# `GameshopDialog` 重写（面板 Title[749] 696x476 @(164,146)、8 格 125x146 @(152+i%4*132, 115/275)、分类栏原点 (120,117)）。
**过程中实机复验立刻抓到 P0**：`game_shop_ui_system` 把 9 个 `&mut Text` 拆成两个 ParamSet，B0001 在系统初始化期 panic，**一进游戏就退出**（exit 101）。修 #2970（合并为单个 8 项 ParamSet）+ 回归测试。
实机判据：C# 坐标 4 个点全部命中 `root=GameShop`。

**项5 滚动条缺失** — 修 #2968：`UiScrollList` 屏幕原点改为沿 `ChildOf` 链逐级累加（子列表不再错位）、隐藏列表不再吞滚轮、商城分类条与行会双列表各自绑定轨道。修 #2966 上游另有 `PositionBar` 链路修整。
实机判据（#2978 后为真值断言）：`wheel` 在邮件列表内注入 1 格 → `offset 0 → 1`（恰为 `step=1`，与 C# `Delta/MouseWheelScrollDelta` 的 1 行/格一致）；负控——在所有列表之外注入滚轮，`offset` 不变。截图（`bug5_shop_scrollbar.png` / `bug5_guild.png`）保留作渲染存档：商城分类条有滑块与上下箭头、行会 Members/Storage 两条轨道并排。

**项6 拖拽穿透** — 修 #2966：`WindowDragState::over_window` 世界点击闸门 + 隐藏窗口必须 `unregister`（否则残留矩形变死点击区，审查 P1）。
实机判据：拖腰带不移动玩家；点拖后腰带落点不穿透；**正控制**空白世界点仍可走。

### 10.3 本轮实机复验额外发现并修复的缺陷（4 项）

| # | 缺陷 | 影响 | 修复 |
|---|---|---|---|
| A | 商城系统跨 ParamSet 触发 B0001 | **P0 一进游戏即崩** | #2970（+ issue #2969） |
| B | 聊天面板中文全豆腐 | 用户报的「乱码」在聊天区没修干净 | #2971（+ 审查跟进 P2 光标 advance） |
| C | Enter 开框当帧写入回车符 | 草稿留不可见 CR、光标偏移一位 | #2973 |
| D | `ui_alignment::inventory_bigmap_aligned` 在 master 上长期 FAILED | 防漂移断言与 #2953 的刻意偏离不同步 | #2972 |

### 10.4 门禁与实机脚本结果

| 门禁 | 结果 |
|---|---|
| `cargo test --lib` | **662 passed / 0 failed**（本机 Windows；同 sha `7726c529` 的 CI（Linux）报 **661**，差 1。本机无留存日志，差异原因未核实、不做推断。重跑 `cargo test --lib` 即得本机数字） |
| `cargo test --test b0001_smoke` | 1 passed |
| `cargo test --test ui_alignment` | **51 passed / 0 failed**（修前 49/1） |
| `cargo fmt -- --check` | 干净 |
| `ui_bugfix_verify.ps1`（项 2/4/5/6 实机 + 基线正控制） | **18/18** |
| `ime_rpc_verify.ps1`（项 3 实机，RPC 真值） | **10/10** |
| `ui_interact_sweep.ps1`（40 窗交互回归） | **41/41**（40 窗中 34 窗点 X 关闭、6 窗无钮设计走 RPC 往返；另含 inventory 拖动与 npc/hero_manage 的 X 点击） |

本批次 PR：#2962 #2963 #2964 #2965 #2966 #2967 #2968 #2970 #2971 #2972 #2973（issue #2969 P0），以及后续跟进 #2976（`new_char_ui_system` 冒烟）／#2977（ServerRust CI 三层红）／#2978（wheel/scroll RPC）／#2979（常量断言拆出 `require_assets!`）／#2981（滚轮 1 行/格）；#2980 因审查证伪其前提而**撤回未合并**。

三条实机脚本的结果 JSON 均在 `tools/acceptance/`（`ui_bugfix_verify_results.json` / `ime_rpc_verify_results.json` / `ui_interact_results.json`），**均为在 `7726c529` 重建的同一份二进制上重跑的最新结果**；截图同目录 `shots/`。

### 10.5 残余缺口与跟进项（如实记录）

**本轮已处理**（原文留痕便于对照）：

- ~~ServerRust CI job 在 master 上红灯~~ → **已修 #2977**。不是一层而是**三层**，每层都被前一层挡着：fmt（120 处 diff / 17 文件）→ clippy（25 errors）→ 测试线程栈溢出（SIGABRT）。修后 CI 的 ServerRust job 四步全绿，并把 `cargo fmt -- --check` + `cargo clippy --lib -- -D warnings` 补进 `AGENTS.md` 本地门禁防复发。
- ~~`ui_alignment` 的常量断言被同函数 `require_assets!` 连带跳过~~ → **已拆 #2979**（`inventory_bigmap_constants`，`CRYSTAL_NO_DATA_ASSETS=1` 下确认真跑）。同类混排在其余测试里仍存在，未逐个拆。
- ~~`mail/npc/npc_goods` 列表滚轮 `step: 3`~~ → **已修 #2981**（三处均按 C# 原文改 1 行/格；实机确认邮件列表 `offset 0 → 1`）。
- ~~项 5 只有截图人工复核~~ → **已升级 #2978**（`wheel`/`scroll` RPC：真值断言 + 负控）。

**仍然开着**：

1. 背包扩容钮 `z=8`（全仓实测：17 处 `z=10`、`npc.rs` 一处 `z=9`、**仅背包 `z=8`**）——把 z 提到 10 才是结构性正解（届时 C# 的 72×23 命中区也不再吞关闭钮），本轮只改了断言，实现偏离保留。
2. 商城仍缺：物品图标、职业分区页签、Preview/Viewer 视图、`qty_up` 的 StackSize 上限（服务端会静默丢弃超量）。排行榜滚动为文档化的 no-op。
3. `market.rs` 列表 `step: 10`（1 格 = 1 页）——C# `TrustMerchantDialog` 同样没有滚轮处理，本端是**增补**的翻页语义；是否改成 1 行/格属产品判断，未动。
4. **【自我更正】商城 P0 不是防线盲区，是合并闸门被绕过。** 我先前在这里写「`b0001_smoke` 没登记 UI 插件、所以没拦住」——**错了**：`tests/b0001_smoke.rs:101` **登记了 `game_shop`**，该 PR 自身的 pull_request CI **三次 push 全红**（首推 `e7e8911c` 红于合并前约 8 小时；tip `3a7938bf` 的 run 与合并**同秒**创建、合并后 9 分钟才判红——即合并那一刻是"红已存在 8 小时、最新一次还 pending"），run [35411949422](https://github.com/gqf2008/Crystal/actions/runs/35411949422) 的 `Client-Bevy :: test (integration)` 步骤就是 **failure**（合并提交 `09b16ac8` 的 push run 35411951451 结论相同），日志即 `[game_shop] error[B0001]: Query<..., GameShopCat> ...::game_shop_ui_system`。**红线被看到了，但无人核对该结论就合了——本仓 master 未配 required status checks（`enforcement_level=off`），红灯本来就不拦人，普通 `gh pr merge` 同样会放行；`--admin` 只是让"没人看"没有代价。**
   → 整改方向随之改变：**不是**去补插件覆盖，而是**别在 CI 红着时 `--admin` 合并**（至少先看该 run 的结论）。
   （附带事实仍然成立但**与商城 P0 无关**：该冒烟里确实一个 `ui::` 插件都没登记，`ui::` 类插件的冲突目前只有 #2976 那条 `run_system_once` 冒烟覆盖。）
5. Enter 长按：本轮曾按「C# 是 no-op」提了 #2980，**被独立审查证伪**（`MainDialogs.cs:749-750` 的 `Visible=false; Text=""` 在 `!string.IsNullOrEmpty` 守卫**之外**，空文本同样关框），且该改法会让重复事件漏进第二个文本循环、把 CR 塞进草稿——**已撤回未合并**。即现状与 C# 一致，不再是缺口。
6. `ServerRust/.cargo/config.toml` 的 8 MiB 测试栈只有一次性实验（1 MiB 复现 / 8 MiB 通过），没有"守住阈值"的断言或基准（#2977 审查 P2-3，留给原作者判断）。
7. 生产调用点（handler 内联链）的**峰值栈未测量**（#2977 审查 P2-2 更正后的真实缺口；生产 actor 栈已是 32 MiB，测试 8 MiB 仍更严）。
8. 坐骑「装备」一步没有 RPC 原语（需界面操作），本轮先用界面把坐骑装好（BengalTiger + 鞍）再骑乘验证遮挡。
9. `chat_notice.rs` 的通知条不可达（`ChatNoticeState` 全仓无写入方）；`chat.rs` 之外仍有 Arial 文本站点，但已逐个核实为数字/符号，不构成中文正文豆腐风险。

### 10.6 复现方法（本机）

```powershell
# 0) 环境：客户端依赖 msys64/ucrt64 与 libpinyin 的 DLL，缺任一目录会以 0xC0000135 静默退出
$env:PATH = 'D:\toolchains\msys64\ucrt64\bin;D:\toolchains\libpinyin-install\bin;' + $env:PATH
$env:LIBPINYIN_DIR = 'D:/toolchains/libpinyin-install'

# 1) 服务端（后台常驻）
cd ServerRust; ./target/release/mir2_server.exe

# 2) 三项实机复验（脚本各自起停客户端）
powershell -File tools/acceptance/ui_bugfix_verify.ps1    # 项 2/4/5/6 -> 18/18
powershell -File tools/acceptance/ime_rpc_verify.ps1      # 项 3     -> 10/10（需先起客户端）
# 40 窗交互巡回（门禁：exit 0 才算过；-ManageServer 可连服务端一起代起停，见 §6.1）
pwsh tools/acceptance/ui_interact_sweep.ps1 -ManageServer
```

`wheel`/`scroll` 两个 RPC 是 #2978 入库的验收能力：`wheel {x,y,delta}` 在 UI 逻辑坐标注入一行滚轮（正=向下滚=offset 增），`scroll {}` 返回全部 `UiScrollList` 真值（轨道矩形、**列表矩形**（滚轮命中用这个）、`offset/total/visible/step/z`、`shown`）。滚轮命中读注入探针且**注入后 2 帧自动撤销**，所以判据要用 `offset` 变化而不是依赖探针常驻；列表 `shown=false`（如行会默认页）时**正确地**不吃滚轮，拿它做正控会得到假 FAIL。

实机脚本的三个已知陷阱（本轮踩过，已在脚本内注释）：正控制点必须落在**可走瓦片**（屏幕偏移对应的瓦片随玩家站位而变，改用 4 方向 8 点探测，任一可走即证通路）；像素锚定必须避开**动态内容**（聊天行实时刷新、输入光标 2Hz 闪烁、输入行随焦点变色），故项 2 改用面板左边框静态条并先自检两帧稳定性。

---

## 11. 用户实测第二批（issue #2985）+ 逐窗截图矩阵 —— 2026-09-19

用户报：「公会 任务 坐骑面板还有很多错位，好友 键盘 帮助面板里有乱码，宠物面板看上去也不对」。
按「**每个窗开→截图→与 C# 逐项比**」推进；数据不足处改为**自己给测试角色塞数据**。

### 11.1 数据塞入（前置条件，`tools/acceptance/seed_db.py`）

停服 → 直接写 `ServerRust/Data/crystal.db` → 重启（跑着写会被存档覆盖；行会是启动时整体载入缓存的）：

| 表 | 塞什么 | 目的 |
|---|---|---|
| `guild_members` | 28 名成员（1 会长 + 2 副会长 + 25 成员） | 越过 18 行视窗 → 触发滚动/翻页/行程 |
| `guilds.rank_defs_json` | 3 档职务 | 成员行下拉有内容 |
| `quests` | 5 个已接任务（跨 3 个 group） | 任务日记的已接段 + 组头 |
| `creatures.owned_json` | 5 只宠物（原版静态表里真有图标/规则的 5 种） | 宠物面板填充 |

原库备份 `tools/acceptance/crystal.db.bak`。

**这一步立刻炸出 3 个空数据时物理上看不见的缺陷**（见 11.2）。

### 11.2 修复清单

| # | 缺陷 | 根因 | 验证 |
|---|---|---|---|
| 1 | 行会成员只显示 1/28，翻页与滚动条行程归零 | `GuildState` 用 `#[derive(Default)]`，`bool` 默认 false；C# `MembersShowOfflinesetting = true` | `scroll` RPC：`total 1 → 28` |
| 2 | **滚动条滑块从来没动过** | `list_thumb` 返回 `co.0`——`co` 是 `&ChildOf`，取到的是**父实体**；`get_mut` 恒失败 | 滑块高 40 → **194**（=302×18/28），滚到底 `top` 76 → **184** |
| 3 | 滑块跟随被光标判定挡在 `return` 之后 | 无焦点窗口 `cursor_position()` 为 None，整段跳过 | 与 2 同批复验 |
| 4 | 隐藏页控件串页（名次/状态页浮着成员下拉框） | Bevy 的 `Visibility::Visible` **越过隐藏祖先渲染**；页容器漏挂 `UiRootDisplay` | 原下拉框位置像素 `(12,12,20) → (3,3,3)` |
| 5 | 状态页表头没右对齐、也不是灰的 | 缺 C# 的 `DrawFormat=Right` + `ForeColour=Gray` | 亮区 `[362..384] → [417.3..439.3]`（右对齐到 437） |
| 6 | 翻页箭头被 16x14 拉伸（艺术尺寸 12x12） | C# `MirImageControl.AutoSize` 让显式 `Size` 形同虚设 | 亮区 12.0×12.0 |
| 7 | 列表滚轮只有一小块区域生效 | 命中矩形写成 (125,30,200,270)；C# 是整页并集 | 页内右下角注滚轮 offset 1→2，页外负控不变 |
| 8 | 任务日记「任务：x/y」压住标题栏 | C# `_takenQuestsLabel @(210,7)`，本端写死 (18,20) | 亮区起于 panel x=211 |
| 9 | 宠物立绘偏右下 16/39px、压住信息行 | C# `CreatureImage.UseOffSet = true` → 绘制点 = `Location + GetOffSet`；本端漏叠 | 立绘回到框内（`p4_crop.png` → `p5_crop.png`） |
| 10 | 好友页签：文字标签压在 `FRIEND` 标题上 | C# 是贴图按钮 `Title[163]@(10,34)` / `Title[167]@(128,34)` | 贴图页签就位 |
| 11 | 师徒标题被拉伸 51% 且被「师徒」文字覆盖 | 按 103x17 画（艺术 68x15）+ 多画了一行文字 | `MENTOR` 单独、原尺寸 |
| 12 | **行会公告页永远空白** | 公告框写死 `Visibility::Hidden` 且全仓无人置显；正文也无人写入 | 公告 3 行 + 「第九行」正常渲染 |
| 13 | 中文豆腐扫尾：24 文件 35 处 + 裸 `.spawn((` 18 处 | Arial 句柄画中文一律 `.notdef`；Han 回退实测不生效 | 40 窗巡回 41/41 |
| 14 | 服务端：C# 迁移角色的宠物静默丢光 | `migrate.rs` 写数字、枚举 serde 默认认变体名；读端 `unwrap_or_default()` 吞掉 + 存档覆盖 | 旧库加载出 5 只，存档自愈为变体名 |

### 11.3 关键方法论（写进 LESSON）

1. **UI 的填充/滚动/比例类缺陷，空夹具下与正确实现渲染结果相同** —— 必须造数据并越过边界。
2. **测试要断言最终产物，不要只断言中间量**：滚动条 bug 活了很久，是因为测试只断言 `offset`（它一直是对的）。
3. **审计要按绑定表达式判类，不能按变量名**：`friend.rs` 里 `let font = shared_cjk_font(..)`，
   `hud.rs` 里 `let font = ui_font.0.clone()` —— 名字会骗人。
4. **扫中文豆腐别漏裸 `.spawn((`**：只匹配 `spawn_*(` 前缀会漏掉
   `ic.spawn((..., TextFont { font: FontSource::Handle(font.clone()) }))`（行会公告就是这么漏的）。

### 11.4 仍未处理（如实列出）

- `dura_status` 面板 `dialog_rect` 读出 **20x19**，与代码里的 64x85 不符，待查
- `craft` 的「学会配方 #0」直接打 recipe_id（观感）
- `report` 面板内容超出右边界、`item_rental_browse` 的 RENT 按钮压住下边框（待与 C# 核对）
- `settings`（键位设置的首个分页）标签是英文（SKILL MODE / EFFECTS …），C# 无此字面量，待核对
- **A3 键盘面板乱码**：默认状态复现不出，等用户指出具体状态

### 11.5 逐窗截图矩阵（本轮新增工具）

```
python tools/acceptance/ui_shot_matrix.py      # 48 窗逐个开→截图→关，产出 manifest.json
python tools/acceptance/contact_sheet.py       # 拼联络表逐张目检
python tools/acceptance/crop_kind.py <kind> <out.png> [scale]   # 按 manifest 矩形裁+放大
python tools/acceptance/font_audit.py          # Arial 句柄 × 可能含中文 的站点审计
python tools/acceptance/seed_db.py             # 测试数据塞入（须先停服）
```

---

## 12. 交互巡回门禁化（2026-09-20）

§9 的 40 窗交互巡回此前**只在开发机人工跑**：`ui_interact_sweep.ps1` 要「服务端 + 真实 `Data/` +
桌面窗口」，进不了 CI；脚本本身退出码还恒为 0（红了也无人能自动发现）。本轮把它拆成两道门禁：

**1. headless 门禁（进 CI）** —— `Client-Bevy/src/game/dialogs/interact_gate.rs`（`#[cfg(test)]`，
随 `cargo test --lib` 跑，CI 的 Client-Bevy job 就含这一步）：

- 名单与脚本 `$kinds` / `$noCloseByDesign` **逐项对齐**：另一支测试解析脚本原文比对，漂移即红；
- 每窗：开窗（语义逐条对齐 `ControlCommand::Dialog`，含 `hero_manage`/`input_box`/`storage` 状态窗特例）
  → 按 `dialog_rect` 同判据定位标准关闭钮（`theme::CloseButton` + 最近祖先 `DialogRoot` 且其 Visible）
  → 合成按压 → 断言已关栈；
- **自建合成最小 `.Lib`**（每库 2600 帧 1x1；`Data/` 不入库、CI 无资产，走 `require_assets!` 跳过就又是假绿）；
- 结果 **41/41**（34 窗按压真实标准关闭钮 + 6 窗「设计无 X」开关往返 + `hero_manage`）。
- **阳性对照（做过，做完即撤）**：抽掉 `storage` 关闭钮的 `StorageWidget` → 报
  `storage: 按压标准关闭钮后仍在 open 栈`（即 §9.3 那处历史缺陷会被拦住）；抽掉
  `spawn_close_button` 的 `CloseButton` 标记 → 15 窗报 `无 theme::CloseButton`。

**2. 脚本可当门禁用** —— `ui_interact_sweep.ps1` 已按 §6.1 门禁化：退出码 `0` 全过 / `1` 有用例 FAIL /
`2` 前置失败（未进图、产物陈旧、缺资产），失败项在总结里逐条列出，`-FailOnSkip` 可让 SKIP 也算失败；
并接入 `pwsh scripts/run_real_e2e.ps1 -IncludeInteractSweep`。
（合并说明：另一支实现用「非全过即 `throw`」表达同一语义，与 §6.1 的退出码机制重复，落库时取后者；
`throw` 会经脚本的 `catch` 记成「巡回中断」→ 同样 `exit 1`，故无行为差异。）

**它不覆盖什么（如实）**：像素级命中区/遮挡（#2953 扩容钮吞点击那类）——headless 无窗口无相机，
按压是直接写 `Interaction::Pressed` 后只跑 `Update` 调度（`ui_focus_system` 会在 `PreUpdate` 按真实光标复位）；
窗口内部控件（页签/输入框/滚动条）也不在巡回范围（同 §5 缺口表）。这些仍须实机跑脚本。

# Client-Bevy UI 功能/交互组件矩阵

本文是 [#2704](https://github.com/gqf2008/Crystal/issues/2704) 的全量盘点入口。原则：

- 以 C# `Client/MirScenes` + `Client/MirControls` 为 UI 交互与布局基准。
- 每项必须有明确实现文件、交互系统、C# 锚点和验证状态。
- 按批次修复，每批一个 worktree / PR；不把全量窗口混成不可审查的大 PR。
- 状态只表示“当前盘点结论”，不代表所有细节已验证。

当前进度（2026-09-12 收口）：

- 批1 通用交互基建：隐藏 `UiButton` 不再点击/悬停/发声/tooltip/拦截世界点击；重叠按钮只触发最上层。
- 批2 HUD/Chat/SkillBar/AssignKey：技能栏子树门控、模态 UI 世界输入锁。
- 批3 Inventory/Character/Hero 系列：补齐英雄技能 Shift+F1..F8 快捷键分配（client+server）。
- 批4 Social/Trade/Market/Mail：交互覆盖已核对；Mail 列表面板缺口在批7运行时审计中定位并修复。
- 批5 Quest/NPC/系统窗：已关闭的 #2535～#2538 均落地；Timer 按 `NotControl` 允许点击穿透。
- 批6 Login/Select/NewCharacter：补齐登录页 `InputKeyDialog` 安全键盘。
- 批7 状态窗/Mail：NPC、Trade/GuestTrade、NpcGoods、Roll、Buff 同步 `DialogManager`，世界输入锁与通用显隐使用同一真值；Mail 列表恢复 C# 312x444 面板、Title[7]、10x33 行和底部操作按钮。
- 批8 Center 布局：Friend/Fishing/Creature/Mentor/Relationship/Report/Guild 按窗口真实尺寸居中；Mentor/Relationship/Report 使用 C# 控件坐标，ChatNotice 使用 C# 顶部中心公式。
- 批9 Creature 内部：背景改 C# `Title[468]`（原生 452x376，原先用 `Prguse[170]` 244x207 拉伸变形）；10 个宠物槽按 C# 5x2 网格（44+81*col, 259+40*row，命中测试同一常量）；改名/召唤/解散/释放/选项/自动/半自动改用 C# `Title` 精灵与原生坐标/尺寸；自动/半自动按 C# `RefreshMode()` 依 `petMode` 互斥显示（原先两个叠在同一坐标）。

## 1. 通用交互控件

| 组件 | Bevy 实现 | C# 基准 | 主要交互 | 修复批次 |
|---|---|---|---|---|
| 三态按钮 | `ui/sprite_ui.rs::UiButton` | `MirButton.cs` | hover/pressed、点击、音效、tooltip | 批1 |
| 图像按钮 | `ui/theme.rs::ImageButton` | `MirButton.cs` | Bevy UI 三帧按钮 | 批1 |
| 动画按钮 | `ui/controls.rs::AnimatedButton` | `MirAnimatedButton.cs` | 帧循环、点击 | 批1 |
| 复选框 | `ui/controls.rs::CheckBox` | `MirCheckBox.cs` | 勾选状态、点击 | 批1 |
| 下拉框 | `ui/theme.rs::UiDropDown` / `ui/controls.rs::DropDown` | `MirDropDownBox.cs` | 展开、选项、滚动、外部关闭 | 批1 |
| 文本框 | `game/dialogs/text_input.rs` | `MirTextBox.cs` | 聚焦、编辑、提交、Esc | 批1 |
| 中文输入法 | `ui/pinyin_ime.rs` | GDI/IME 行为 | 候选、组合文本、焦点 | 批1 |
| 物品格 | `ui/theme.rs::UiItemCell` | `MirItemCell.cs` | 左/右键、双击、拖放、tooltip | 批3 |
| 商品格 | 各商店/市场模块 | `MirGoodsCell.cs` / `MirGameShopCell.cs` | 选择、购买、价格 | 批4 |
| 滚动列表 | `ui/scroll_list.rs::UiScrollList` | `MirControl` 滚动子控件 | 滚轮、拖动条、翻页 | 批1 |
| Tooltip | `ui/tooltip.rs` | `MirLabel` / Hint | 悬停、描边、避让屏幕 | 批1 |
| 消息框 | `ui/modal_box.rs` | `MirMessageBox.cs` / `MirInputBox.cs` | Yes/No/OK、输入、模态 | 批1 |
| 页签 | 各对话框 `*Tab*` 组件 | 各 C# Dialog 页签按钮 | 切换页、选中态 | 批2-6 |
| 对话框拖动/置顶 | `game/dialogs/mod.rs` | `MirControl.OnMouseMove` / BringToFront | 拖动、钳制、层级 | 批1 |
| Tab/Enter 键盘导航 | `ui/keyboard_nav.rs` | Windows 键鼠交互语义 | 焦点、Enter、高亮 | 批1 |

## 2. HUD 与常驻层

| 组件 | Bevy 实现 | C# 基准 | 修复批次 |
|---|---|---|---|
| HUD 按钮 | `game/hud.rs::HudButton` | `MainDialogs.cs` | 批2 |
| 血/蓝球、经验、金币、等级、名字 | `game/hud.rs` | `MainDialogs.cs` | 批2 |
| 英雄信息面板/按钮 | `game/hud.rs` | `MainDialogs.cs` | 批2 |
| 聊天面板、频道、输入、滚动、过滤 | `game/chat.rs` | `ChatDialog` / `ChatOptionDialog` | 批2 |
| 技能快捷栏 | `game/skills.rs::SkillBarRoot` | `MainDialogs.cs::SkillBarDialog` | 批2 |
| 技能快捷键分配 | `game/dialogs/assign_key.rs` | `AssignKeyPanel` | 批2 |
| 小地图 | `game/dialogs/minimap.rs` | `MiniMapDialog` | 批2 |
| 指南针 | `game/dialogs/compass.rs` | `CompassDialog` | 批2 |
| 任务追踪 | `game/dialogs/quest_tracking.rs` | `QuestTrackingDialog` | 批5 |
| Buff/毒 Buff | `game/dialogs/buff.rs` | `BuffDialog` / PoisonBuff | 批2 |
| 耐久切换面板 | `game/dialogs/dura_status.rs` | `DuraStatusPanel` | 批2 |
| 倒计时/公告/掷骰/聊天公告 | `timer.rs` / `notice.rs` / `roll.rs` / `chat_notice.rs` | 对应 C# Dialog | 批5 |
| 死亡选择框 | `game/hud.rs::DeathDialogState` | `MainDialogs` 死亡流程 | 批2 |

## 3. DialogKind 全量矩阵

状态列：`实现` = 已有组件；`批N` = 纳入对应批次逐项修复。

| DialogKind | Bevy 文件 | C# 主基准 | 状态 |
|---|---|---|---|
| Inventory | `dialogs/inventory.rs` | `InventoryDialog.cs` | 实现；批3 |
| Character | `dialogs/character.rs` | `CharacterDialog.cs` | 实现；批3 |
| QuestLog | `dialogs/quest_log.rs` | `QuestDialogs.cs` | 实现；批5 |
| Settings | `dialogs/option.rs` | `MainDialogs.cs` OptionDialog | 实现；批5 |
| Menu | `dialogs/menu.rs` | `MainDialogs.cs` MenuDialog | 实现；批5 |
| GameShop | `dialogs/game_shop.rs` | `GameshopDialog.cs` | 实现；批4 |
| Minimap | `dialogs/minimap.rs` | `BigMapDialog.cs` / MiniMap | 实现；批2 |
| Npc | `dialogs/npc.rs` | `NPCDialogs.cs` | 实现；批5 |
| Group | `dialogs/group.rs` | `GroupDialog.cs` | 实现；批4 |
| Friend | `dialogs/friend.rs` | `FriendDialog.cs` | 实现；批4/批8（Center） |
| Trade | `dialogs/trade.rs` | `TradeDialogs.cs` | 实现；批4 |
| GuestTrade | `dialogs/trade.rs` | `TradeDialogs.cs` | 实现；批4 |
| Inspect | `dialogs/inspect.rs` | `CharacterDialog.cs` Inspect | 实现；批3 |
| NpcGoods | `dialogs/npc_goods.rs` | `NPCDialogs.cs` | 实现；批4 |
| Guild | `dialogs/guild.rs` | `GuildDialog.cs` | 实现；批4/批8（Center） |
| Mail | `dialogs/mail.rs` | `MailDialogs.cs` | 实现；批4/批7（列表布局） |
| Ranking | `dialogs/ranking.rs` | `RankingDialog.cs` | 实现；批4 |
| Mentor | `dialogs/mentor.rs` | `MentorDialog.cs` | 实现；批4/批8（Center） |
| Relationship | `dialogs/relationship.rs` | `RelationshipDialog.cs` | 实现；批4/批8（Center/按钮） |
| Mount | `dialogs/mount.rs` | `MountDialog.cs` | 实现；批3 |
| Report | `dialogs/report.rs` | `ReportDialog.cs` | 实现；批5/批8（布局） |
| Hero | `dialogs/hero.rs` | `HeroDialogs.cs` | 实现；批3 |
| HeroInventory | `dialogs/hero_inventory.rs` | `HeroDialogs.cs` | 实现；批3 |
| HeroEquipment | `dialogs/hero_equipment.rs` | `HeroDialogs.cs` | 实现；批3 |
| HeroSkill | `dialogs/hero_skills.rs` | `HeroDialogs.cs` | 实现；批3 |
| Creature | `dialogs/creature.rs` | `IntelligentCreatureDialogs.cs` | 实现；批3/批8（Center）/批9（内部布局） |
| ItemRental | `dialogs/item_rental.rs` | `ItemRentalDialog.cs` 等 | 实现；批4 |
| GuildTerritory | `dialogs/guild_territory.rs` | `GuildTerritoryDialog .cs` | 实现；批4 |
| Help | `dialogs/help.rs` | `HelpDialog.cs` | 实现；批5 |
| Notice | `dialogs/notice.rs` | `NoticeDialog.cs` | 实现；批5 |
| Buff | `dialogs/buff.rs` | `BuffDialog.cs` | 实现；批2 |
| Fishing | `dialogs/fishing.rs` | `FishingDialog.cs` | 实现；批5/批8（Center） |
| Socket | `dialogs/socket.rs` | `SocketDialog.cs` | 实现；批3 |
| Refine | `dialogs/refine.rs` | `NPCDialogs.cs` refine 流 | 实现；批3 |
| Craft | `dialogs/craft.rs` | `NPCDialogs.cs` Craft | 实现；批3 |
| DuraStatus | `dialogs/dura_status.rs` | `MainDialogs.cs` DuraStatus | 实现；批2 |
| Roll | `dialogs/roll.rs` | `RollDialog.cs` | 实现；批5 |
| NpcAwake | `dialogs/npc_awake.rs` | `NPCDialogs.cs` NPCAwake | 实现；批5 |
| Timer | `dialogs/timer.rs` | `TimerDialog.cs` | 实现；批5 |
| KeyboardLayout | `dialogs/keyboard_layout.rs` | `KeyboardLayoutDialog.cs` | 实现；批5 |
| BigMap | `dialogs/big_map.rs` | `BigMapDialog.cs` | 实现；批2 |
| ChatNotice | `dialogs/chat_notice.rs` | `ChatNoticeDialog.cs` | 实现；批5/批8（位置） |
| Market | `dialogs/market.rs` | `TrustMerchantDialog.cs` | 实现；批4 |
| Storage | `dialogs/storage.rs` | `InventoryDialog.cs` Storage | 实现；批3 |
| Skills | `game/skills.rs` / Character SkillPage | `MainDialogs.cs` MagicWindow | 实现；批2 |

## 4. 登录、选角与账号流程

| 组件 | Bevy 文件 | C# 基准 | 修复批次 |
|---|---|---|---|
| 登录框 | `ui/login.rs` | `LoginScene.cs` / `LoginDialog` | 批6 |
| 新建账号 | `ui/login.rs` | `NewAccountDialog` | 批6 |
| 修改密码 | `ui/login.rs` | `ChangePasswordDialog` | 批6 |
| 查看密钥 | `ui/login.rs` | `LoginScene` KeyDialog | 批6 |
| 选角界面 | `ui/select.rs` | `SelectScene.cs` | 批6 |
| 新建角色 | `ui/new_character.rs` | `NewCharacterDialog.cs` | 批6 |
| 删除角色 | `ui/modal_box.rs` | `SelectScene` delete dialogs | 批6 |

## 5. 批次完成记录

| 批次 | 结果 | PR |
|---|---|---|
| 批1 通用交互基建 | 隐藏/遮挡按钮命中、层级、音效、tooltip、世界点击统一收口 | #2705 |
| 批2 HUD/Chat/SkillBar/AssignKey | SkillBar 整树门控；模态输入锁覆盖 AssignKey/数量框/丢弃确认 | #2707 |
| 批3 背包/角色/英雄 | 英雄技能行可点击；client+server 支持 Key 17..24 路由 `hero_magics` | #2709 |
| 批4 社交/交易/市场/邮件 | 对照代码和已合并批次，现有实现覆盖；无新增改动 | 已核实 |
| 批5 任务/NPC/系统窗 | #2535～#2538 已落地；Timer `NotControl` 点击穿透修复 | #2710 |
| 批6 登录/选角/建角 | 登录 ViewKey `InputKeyDialog` 安全键盘落地 | #2712 |
| 批7 状态窗/Mail | 状态驱动窗口同步 DialogManager；Mail 面板/10x33 行/按钮对齐 C# | #2714 |
| 批8 Center 布局 | Friend/Fishing/Creature/Mentor/Relationship/Report/Guild 居中；Report/ChatNotice 坐标收口 | #2716 |
| 批9 Creature 内部 | `Title[468]` 面板 + 5x2 宠物槽 + C# 操作按钮精灵/坐标 + 自动/半自动互斥 | #2718 |

## 6. 验证基线

- `cargo check --tests`（Client-Bevy）通过。
- `cargo test`（Client-Bevy）：385 lib + 2 bin + 1 smoke + 17 alignment 通过（批9后基线）。
- Report 的 C# `Prguse[1633]` 在当前本地 Data 包缺失；已使用按 C# 控件边界推导的 360x244 深色兜底面板并保留对应子控件坐标，待资源包更新后自动加载正确背景。
- ServerRust：665 lib + protocol integration 通过（批3后基线）。
- 关键实机/定向验证：UI 子树泄漏截图、Character 技能页、AssignKey 模态输入、Timer 穿透、登录安全键盘资源；批7 复验 Mail/Buff；批8 复验 Center 窗口。
- 批9 实机复验（2026-09-13，`--skip-login` + Control API `dialog`/`screenshot`，mock 3 宠物）：Creature 面板为 `Title[468]` 452x376 居中；槽位标签 `>小猪` / `很长的宠物名`（超宽截断）/ `#4`（空名字回退）互不压叠、无越界裁切；右侧操作列 RENAME / DISMISS（激活宠物）/ RELEASE / OPTIONS / ENABLE（Automatic 互斥，无半自动叠加）/ 刷新 均在 C# 坐标；信息行 (19,161)/(19,176) 显示「宠物: 3 个 ｜ 小猪 自动 饥饿:55」与操作反馈。

## 7. 已知有意偏差

- Creature：C# `CreatureRenameButton` 构造即 `Visible = false` 且再无置真处（原版死控件，改名入口点不到）；Bevy 保留可用的「改名」按钮（功能补齐见 #1281），仅坐标/精灵与 C# 对齐。
- Creature：C# `CreatureInfo`/`CreatureInfo1` 是选中宠物的能力文案（`CanPickupItems`/`CanProduceBlackStones`），`CreatureInfo2` @(19,191) 未实现；Bevy 用 (19,161) 一行承载「数量 + 选中宠物名/模式/饥饿度」，语义不同但坐标对齐。
- Creature：C# 未选中宠物时模式按钮是 `Enabled = false`（`RefreshMode()` 早返回、保留原可见性）；Bevy 无禁用态，未选中时直接 `Visibility::Hidden`。
- Creature：C# `HelpPetButton`（`Prguse2[257..259]` @ `Size.Width-48,3`）在原版无 Click 处理（死控件），Bevy 未实现该占位按钮，待有宠物帮助页时再补。
- Creature：`刷新` 是 Bevy 扩展按钮（C# 无此控件），用中文文本渲染；此前借用 MessageBox 的 `Title[206..208]`（原版是「YES」精灵），实机截图里会显示成「YES」。

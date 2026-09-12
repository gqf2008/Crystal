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
- 批10 Craft 外壳：面板改 C# `Prguse[1109]`（原生 337x215），位置按 C# `Show()` 相对背包窗 `(Inventory.X-12, Y+236)`；RecipeLabel(22,5)/PossibilityLabel(10,135)/GoldLabel(30,190) 与标题 `Title[18]`(28,8) 对齐；按钮改 C# `AutoFill Title[180..182]`(165,185)、`Craft Title[336..338]`(215,185)、Close `Prguse2[360..362]`(312,3) 原生 24x21（原先一个通用 `Title[206..208]` 合成按钮 + 文本行）。
- 批10 配方协议：`S.NewRecipeInfo` 由只带 `recipe_id` 扩成 C# `ClientRecipeInfo` 整份（Gold/Chance/Item/Tools/Ingredients，服务端按 DB 配方构造影子 UserItem），客户端缓存到 `CraftState.recipes` 并用于信息行（金币/成功率/工具数/材料数）；材料槽 UI 与 `CraftItem.slots` 待移植。
- 批10 Craft 材料槽：`RecipeRequirement`（item_index/count/image/name/min_dura）随包下发图标与名称；C# 3 工具格 `((x*44)+108,44)` + 6 材料格 `(52+(x-3)*40,86)` 影子格落地（未放入显示需求图标，放入显示背包物品图标+数量）；背包选中物点格放入（C# `Grid_Click` 索引/数量/工具耐久校验）、`AUTO` 按配方顺序自动填充（C# `AutoFill`）、`CRAFT` 全部就位后发 `CraftItem.slots`（C# `CraftItem`）；配方切换/关闭清空槽位（C# `ResetCells`）。
- 批10 Refine 材料格：面板改 C# `Prguse[1002]`（原生 164x207）@ (0,225)，标题 `Title[18]`(28,8)；4x4 材料格按 C# `((x*34)+12+x, (y*32)+37+y)`、34x32；格子 i ↔ 服务端材料槽 i+1（Rust `to=0` 为武器槽）；背包选中物点格存入、点已存入格取回到背包空格，服务端 `DepositRefineItem/RetrieveRefineItem` 确认包更新本地镜像；服务端 `REFINE_MATERIAL_SLOTS` 10 → 16 对齐 C# `CharacterInfo.Refine[16]`。
- 批10 Refine 入口：`S.NPCRefine/NPCCheckRefine/NPCCollectRefine` → 投放窗（`Prguse[392]`，`PanelType.Refine/CheckRefine`，提示「放入武器后点确认精炼」）+ 背包 + 材料窗同开（C# `NPCDialog` PanelType.Refine → `RefineDialog.Show()`）；确认 → 存入武器（`to=0`）后按 uid 发起 `RefineItem`（CheckRefine 模式发 `CheckRefine`）；`S.RefineItem/RefineCancel` → 清空材料格（C# `RefineReset`）；`refining=true`/`NPCCollectRefine` → 收起；`RefineDialog` 不再带关闭键（C# 没有，靠 NPC 窗联动）。顺带修掉投放窗提示用拉丁字体导致的中文豆腐块。
- 批10 ItemRent 浏览窗（`dialogs/item_rental_browse.rs`，新 `DialogKind::ItemRentalBrowse`）：C# `ItemRentalDialog` `Prguse3[1]`(400x174 居中)、标题 `Prguse3[0]`(22,8)、页签 `Prguse3[2]/[3]` (8,32)/(81,32)、`Prguse3[4..6]` RENT(295,144) 85x29、关闭(375,3)、3 行(0,78+i*21) 三列(5/137/264)；打开时按 C# 60s 节流发 `C.GetRentedItems`，`S.GetRentedItems` 线格式改成 C# `ItemRentalInformation`（ItemId/ItemName/RentingPlayerName/ItemReturnDate）→ 填行；RENT 按钮发 `C.ItemRentalRequest`。
- 批10 TrustMerchant 外壳：面板改 C# `Title[786]`(492x478 @ C# 默认(0,0))；四页签 `Title[789/788]`(9,35)/`[791/790]`(104,35)/`[817/816]`(199,35)/`[819/818]`(389,35)；关闭 `Prguse2[360..362]`(465,3)；底部栏 SearchTextBox(11,452) 110x18、Find `Title[480..482]`(124,448)、Refresh `Prguse[663..665]`(320,448)、Buy `Title[703..705]`(380,448)、翻页 `Prguse2[240..242]`(251,419)/`[243..245]`(320,419)、PageLabel(260,419)；列表移至 C# 列表区 (130,60) 行高 18（左列 x≤120 留给筛选树）；GameShop 页签开现有商城窗，寄售卖价/立即售出/取回暂用 C# 筛选按钮精灵放左列。
- 批10 ItemRent 出租流程（物主侧，`item_rental.rs` 重写）：C# 双窗同用 `Prguse[238]`(204x109)——费用窗 `(718,163)`（价格按钮 `Prguse[28]`(18,46) 32x17 → 数量框 → `ItemRentalFee`；锁定费用 `Prguse[250..252]`(22,76)）、物品窗 `(718,287)`（物品格(16,35)、锁定物品 `Prguse[250..252]`(18,76) 期限 1..30 校验、设置期限 `Prguse3[7..9]`(46,76) 84x28、确认 `Prguse3[10..12]`(130,76) 58x28 需 `can_confirm`）、两窗关闭 `Prguse2[360..362]`(180,3)、名称(30,8)/数值(60,42) 标签；发起租赁改由浏览窗 RENT（`C.ItemRentalRequest`）。
- 批10 ItemRent 对方镜像窗（`item_rental.rs` 四窗化）：`S.ItemRentalRequest` 按 C# 扩成 `{Name, Renting}`（两端各收一份定角色），角色由 `Renting` 定（false=物主=点 RENT 发起方，true=租客=被请求方）；每端只显示「1 自有窗 + 1 对方窗」——物主：自有物品窗(287)+对方费用窗(`GuestItemRentDialog` 163)，租客：自有费用窗(163)+对方物品窗(`GuestItemRentingDialog` 287)，坐标互补不重叠；对方窗控件全部 `Enabled=false`（单帧、无关闭键、物品格只读）。`S.ItemRentalPartnerLock`/`S.ItemRentalLock` 改成 C# 判别位（`GoldLocked`/`ItemLocked`），锁定后对应窗锁形换 `Prguse[253]`。服务端角色按 C# 反转：会话键=物主（存物/设期/锁物/确认/收租），partner=租客（设费/锁费/收物）；`UpdateRentalItem` 改 C# `HasData`+`LoanItem`（None 清空对方物品格）且只发租客，`CanConfirm` 只发物主。
- 批10 TrustMerchant 左列筛选树（新 `dialogs/market_filter.rs`）：C# `SetupFilters` 8 个主项（显示所有物品/武器类物品/衣服类物品/饰品类物品/消耗品/强化/书籍/制作材料）+ 17 个子项（类型 + 形状范围，Index 201..704）逐项移植，文案取 `Client/Localization/Chinese.json`；`DrawFilters` 布局 = 按钮 `Prguse2[920..923]`(100x22) @(7,60)、主项步进 20、展开 +2、子项步进 21、子标签缩进 10（主 2）、`MaxLines=19`；滚动条 `Prguse2[197..199]`(108,60)/`[207..209]`(108,429)/`[205/206]`(108,73) 可拖动（`PosMinY=73`/`PosMaxY=410`，不满一屏隐藏）；点击主项展开（C# 有子项时不发搜索）、点击子项/无子项主项发 **C# 规范 `C.MarketSearch{Match,Type,Usermode=false,MinShape,MaxShape,MarketType}`**（顺带修好 Find 按钮：原 `MarketSearchWire` 只写关键字，网关 `read_body` 失败静默丢弃）；页签语义按 `TMerchantDialog(type)`——Market/GameShop 显示筛选树并复位 `DrawFilters(0,-1)` 后发搜索，寄售/拍卖隐藏整列（Bevy 扩展的寄售/取回/售出按钮改为只在这两个页签露出）。
- 批10 TrustMerchant 寄售/拍卖页签面板：面板背景按页签换 `Title[786]`/`Title[787]`（均 492x478），`BuyButton` 换 `Title[703..705]`/`[706..708]`（84x25 @380,448）；新增 C# 面板控件 `HelpLabel`(8,237) 115x205（按 `Globals` 数值格式化规则文案，定宽换行）、`ItemCell`(47,104) 36x32（`MirItemCell` 空置态 `BackColour(255,255,125)`+0.5 透明）、`PriceTextBox`(15,165) 100x18、`SellItemButton Title[700..702]`(39,188) 52x25、`CollectSoldButton Title[680..682]`(300,448) 72x25（仅寄售）、`SellNowButton`(324,448)（仅拍卖），以及 5 个表头（出售价格/出售物品/物品/价格/到期，文案随页签）；售价三态（C# `TextBox_TextChanged`：寄售 5000..50,000,000、拍卖 0..50,000，低于下限红/有效绿/达上限橙 + 超上限钳制），提交键禁用态与 `ItemCell_Click` 选物流程（点背包选中物入格 + 聚焦售价框 + 再点取消）照 C#；Buy 键在寄售/拍卖页签按 C# UserMode 语义改发 `C.MarketGetBack{AuctionID}`。

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
| 批10 Craft 外壳 | `Prguse[1109]` 面板 + C# 标签/按钮坐标（材料槽待配方协议扩展） | #2720 |

## 6. 验证基线

- `cargo check --tests`（Client-Bevy）通过。
- `cargo test`（Client-Bevy）：409 lib + 2 bin + 1 smoke + 20 alignment 通过（批10 寄售/拍卖面板后基线）。
- Report 的 C# `Prguse[1633]` 在当前本地 Data 包缺失；已使用按 C# 控件边界推导的 360x244 深色兜底面板并保留对应子控件坐标，待资源包更新后自动加载正确背景。
- ServerRust：666 lib + 6 integration 通过（批10 租赁角色对齐后基线）；SharedRust 185 + 11（2 ignored）；`MapEditor/SharedRust` `cargo check` 通过（副本同步）。
- 关键实机/定向验证：UI 子树泄漏截图、Character 技能页、AssignKey 模态输入、Timer 穿透、登录安全键盘资源；批7 复验 Mail/Buff；批8 复验 Center 窗口。
- 批9 实机复验（2026-09-13，`--skip-login` + Control API `dialog`/`screenshot`，mock 3 宠物）：Creature 面板为 `Title[468]` 452x376 居中；槽位标签 `>小猪` / `很长的宠物名`（超宽截断）/ `#4`（空名字回退）互不压叠、无越界裁切；右侧操作列 RENAME / DISMISS（激活宠物）/ RELEASE / OPTIONS / ENABLE（Automatic 互斥，无半自动叠加）/ 刷新 均在 C# 坐标；信息行 (19,161)/(19,176) 显示「宠物: 3 个 ｜ 小猪 自动 饥饿:55」与操作反馈。
- 批10 实机复验（2026-09-13，Control API 打开 Craft 截图）：`Prguse[1109]` 面板（含 3 工具格 + 6 材料格轮廓）、RecipeLabel(22,5)、PossibilityLabel(10,135)、AUTO(165,185)/CRAFT(215,185) 精灵按钮、Close(312,3) 与 C# 坐标一致。
- 批10 租赁四窗实机复验（2026-09-13，`--skip-login` + `--rental-test` + Control API 截图；mock 侧 `ItemRentalRequest` 分别回 `Renting=false/true` 各截一次）：物主侧 = 自有物品窗(718,287：名称/期限/SET PERIOD/RENT/锁形) + 对方费用窗(718,163：对方名「bevy2char」/费用/金币价格钮/锁形)；租客侧 = 自有费用窗 + 对方物品窗（对方名 + 期限 + 只读物品格 + 禁用 SET PERIOD/RENT）。`visible` RPC 每端恰好 2 个 `ItemRental` 根（四窗中只显 2），确认角色分流生效。
- 批10 筛选树实机复验（2026-09-13，`--skip-login` + Control API `dialog trust_merchant open` 截图）：左列 8 个主项按 C# 文案与坐标渲染、首项「显示所有物品」为选中帧(`Prguse2[921]`)、右侧滚动条上下箭头就位、手柄因 `PossibleTotal(8) <= MaxLines(19)` 正确隐藏；临时把进入页签时的 `filter_index` 设为 2（衣服类物品）复截：主项高亮 + 5 个子项（护甲/头盔/腰带/靴子/宝石/石头）按 +2 间隔与 21 步进缩进排布，与 C# `DrawFilters` 一致（验证后已还原）。
- 批10 寄售页签面板实机复验（2026-09-13，`--skip-login` + Control API 打开 trust_merchant 截图；临时把进入页签时的 `panel` 设为 `Consign`，验证后已还原）：背景为 `Title[787]`（带寄售表格网格）、表头「出售物品/物品/价格/到期」+「出售价格」(15,142) 就位、物品格(47,104) 显示 C# 空置态浅黄半透明底、售价框(15,165) 按无效态显示红底、SELL(39,188) 呈禁用暗化、底栏 COLLECT(300,448) 与 BUY（`Title[706..708]` 用户模式帧，380,448）就位、说明文案按 115px 宽换行显示、筛选树整列隐藏；与 C# `TMerchantDialog(MarketPanelType.Consign)` 的显隐一致。

## 7. 已知有意偏差

- Creature：C# `CreatureRenameButton` 构造即 `Visible = false` 且再无置真处（原版死控件，改名入口点不到）；Bevy 保留可用的「改名」按钮（功能补齐见 #1281），仅坐标/精灵与 C# 对齐。
- Creature：C# `CreatureInfo`/`CreatureInfo1` 是选中宠物的能力文案（`CanPickupItems`/`CanProduceBlackStones`），`CreatureInfo2` @(19,191) 未实现；Bevy 用 (19,161) 一行承载「数量 + 选中宠物名/模式/饥饿度」，语义不同但坐标对齐。
- Creature：C# 未选中宠物时模式按钮是 `Enabled = false`（`RefreshMode()` 早返回、保留原可见性）；Bevy 无禁用态，未选中时直接 `Visibility::Hidden`。
- Creature：C# `HelpPetButton`（`Prguse2[257..259]` @ `Size.Width-48,3`）在原版无 Click 处理（死控件），Bevy 未实现该占位按钮，待有宠物帮助页时再补。
- Creature：`刷新` 是 Bevy 扩展按钮（C# 无此控件），用中文文本渲染；此前借用 MessageBox 的 `Title[206..208]`（原版是「YES」精灵），实机截图里会显示成「YES」。
- Craft：C# 用客户端本地 `ItemInfo` 库解析需求图标，Rust 客户端无本地物品库 —— 图标/名称改由 `RecipeRequirement.image/name` 随包下发（协议自洽偏离，已注释说明）。
- Craft：C# 放入材料后会锁定对应背包格（`SelectedCell.Locked`）直到关窗；Bevy 目前只记录背包槽号，未在背包侧锁定（后续可加 `LockedSlots` 资源）。
- Refine：C# 的待精炼武器走 NPCDialog 的 ItemCell（投放窗确认即 `C.RefineItem{UniqueID}`）；Rust 服务端语义是两步（`DepositRefineItem to=0` 存入 → `RefineItem{uid}` 发起），故 Bevy 的投放窗确认在收到存入确认后再发 `RefineItem`（对外行为等价，多一个包）。
- ItemRent：C# `KeybindOptions.Rental` 在 `KeyBindSettings.New()` 里**没有默认绑定行**（枚举成员存在但无 `KeyBind`，原版默认无键，键位面板也列不出）；Bevy 作扩展给「租赁」（界面组，默认 `T`）并在键位面板可重绑，热键 `ItemRentalDialog.Toggle()` 语义与 C# `GameScene.cs:779-781` 一致。
- ItemRent：`Prguse[238]` 面板位图**自带**右上角关闭图样（C# 四窗都画得到），但只有自有窗挂了可点关闭键（`Prguse2[360..362]`）——C# `GuestItemRentDialog`/`GuestItemRentingDialog` 本身没有 `closeButton`，Bevy 同样只在自有窗挂 `ItemRentalClose`，对方窗的 X 是装饰。
- ItemRent：`S.UpdateRentalItem` 除 C# 的 `HasData`+`LoanItem` 外仍带 `rental_fee`/`rental_period`（Rust 扩展，客户端只在 >0 时用作兜底刷新）；`S.CanConfirmItemRental`/`S.ConfirmItemRental` 在 C# 是空包，Rust 端口带 `can_confirm`/`success` 载荷（自洽偏离，双方均为 Rust 实现）。
- ItemRent：C# 租客侧的合计费用在服务端 `SetItemRentalFee` 里即时扣金币（`S.LoseGold`）；Rust 服务端到 `ConfirmItemRental` 成交时才扣，客户端费用标签两侧都由本地/对包数值驱动，显示一致。
- ItemRent：mock 单客户端下 `ItemRentalRequest` 固定回 `Renting=false`（物主侧），租客侧窗口与锁定帧的实机验证靠临时把 mock 回包改 `true` 截图（验证后已还原）。
- TrustMerchant：列表行仍是 Bevy 文本行（C# `AuctionRow` 为 `(127, 82+i*33)` 的 item cell + 三列文本 + `Selected` 边框，行高 33、10 行分页）、Mail 按钮（`Prguse[437..439]`）、价格排序（`PriceFilter` 三态 + `Prguse2[925/926]` 图标）尚未移植。
- TrustMerchant：售价框边框三态（C# `PriceTextBox.BorderColour` 只画 1px 边框色）在 Bevy 用输入框底色近似；`SellItemButton.Enabled = false` 的灰度用 `ImageNode.color` 暗化近似（无灰度着色器）；C# 寄售选物会锁定背包格（`tempCell.Locked`），Bevy 只记录选中物、未锁背包格。
- TrustMerchant：筛选树 `Prguse2[205/206]` 手柄的拖动用「按下时记录抓取偏移 → 按住按光标 y 反算 `Skip`」实现（C# 是 `MirControl.OnMoving`）；手柄高度固定为精灵原始 12x18（C# 也未按 `PossibleTotal` 缩放）。
- TrustMerchant：Find/筛选/页签搜索改发 C# 规范 `C.MarketSearch`（此前 Bevy `MarketSearchWire` 只写关键字，被网关 `read_body` 静默丢弃 → 搜索无效）；`MarketRefresh` 仅保留在刷新按钮（C# `RefreshButton.Click` 先清空搜索框）。
- Craft：C# `BeforeDraw` 在背包关闭时会隐藏合成窗，Bevy 未实现（挂机脚本会直开 Craft，保持现状以免回归）。

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
- 批10 TrustMerchant 列表行（C# `AuctionRow`）：10 行改 `(127, 82+i*33)` 354x32（原为 (130,60+18i) 文本行）；行内 34x32 图标区（有物品 `Items[Info.Image]`、零数量 `Prguse[540]` 占位，按 `(IconArea-Icon.Size)/2` 居中）+ `NameLabel`(38,8)/`PriceLabel`(170,8)/`SellerLabel`(256,0)/`ExpireLabel`(256,14) 四标签 + 选中 1px 橙框（`BorderColour=FromArgb(255,200,100,0)`，C# `BorderInfo` 外扩 1px；`SelectedImage`(Prguse[545]) 是死控件）；文本按 C# 规则：价格千分位 + 拍卖「出价」后缀 + 阈值着色（>10M 红/>1M 橙/>100k 草绿/>10k 天蓝）、名称按品质（`GradeNameColor`，黄→白）、卖家列 UserMode 状态串着色（Sold/Expired/Bid Met）、到期列 = 寄售日期 + 7 天按 `dd/MM/yy HH:mm:ss`；底栏按 C# `UpdateInterface` 联动启用态（选中→Buy 亮/CollectSold 灰，`Bid Met` 才亮 SellNow）。服务端补 `AuctionInfo.GetSellerLabel(userMatch)` 移植（UserMode 下发 Sold/Expired/Bid Met/No Bid/For Sale 标记）并把 `Usermode` 记进搜索缓存供翻页沿用。
- 批10 TrustMerchant 价格排序 + Mail 按钮：价格表头 `TitlePriceLabel`(295,60,88x21) 加 C# 点击层（`Click → CyclePriceFilter`），三态 `MarketPriceFilter` Normal→Low→High→Normal，图标 `Prguse2[925]`(低)/`[926]`(高) 12x11 @(371,65)（= `X+W-12, Y+(H-14)/2+2`，Normal 隐藏）；列表按 `GetOrderedListings()` 显示（Low 升序/High 降序，稳定排序，Normal 保持服务器顺序；Bevy 服务端按页下发，故排序作用于当前页——见 §7）；`MailButton` `Prguse[437..439]` 28x25 @(350,448)（仅市场页签）→ 选中行时以卖家为收件人、按 `InterestedInPurchase`（「我有意购买{0}，价格为{1}。」）预填正文开写信窗（`ComposeMail` 扩了 `message` 字段）。顺带修正行取数：原 `Page*10 + i` 在服务端按页下发模型下会让第 2 页起取不到行（`listings` 只存当前页），改为 `row_listing_index`（当前页 + 排序映射），行渲染/命中/选中三处同源。
- 批11（#2736）TrustMerchant 买/取回确认框：Buy 键按 C# `BuyButton.Click`（:360-440）分支改弹 `MirMessageBox` YesNo（`Prguse[360]` 456x190 居中 @(284,289)、文本 (35,35) 390x110、Yes `Title[206..208]`@(260,157)、No `Title[210..212]`@(360,157)，均 76x25）——UserMode 寄售 `For Sale`/拍卖 `No Bid` → 「{物品}尚未售出，确定要取回它吗？」（Yes = `MarketGetBack{AuctionID}`）；UserMode 其余状态直接取回；非 UserMode 寄售/商城 → 「确定要以{价格:#,##0} {金币|积分}购买{物品}吗？」（Yes = `MarketBuy{AuctionID}`）；非 UserMode 拍卖 → 「你确定要为{物品}出价{额:#,##0}金币吗？」（出价取价格输入框，缺省 `当前价+1`）。确认框为独立根节点（z=45，不在 TM 面板裁剪内），随市场窗关闭自动收起。
- 批11（#2736）Creature 操作按钮禁用态：按 C# `RefreshUI()`（IntelligentCreatureDialogs.cs:606-668）把「未选中宠物」从 Bevy 的 `Visibility::Hidden` 改为 C# 的 **`Enabled = false`**——RENAME/OPTIONS/AUTO/SEMI/RELEASE/SUMMON 保持可见但灰化（`ImageNode.color` 暗化近似 `GrayScale`）且不响应点击；`DISMISS` 仍按 C# 显式 `Visible = false`；召唤/释放仅在未召唤时可用、解散仅在已召唤时可用（C# :647 `ReleaseButton.Enabled = false`）。模式按钮在未选中时按 C# `RefreshMode()` 早返回 + 构造默认 `Visible = true` → 两个都可见（同坐标 (375,187)，后建的 SemiAuto 覆盖，与 C# 绘制顺序一致）且都禁用。

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
| 批11 §7 偏差收口 | TrustMerchant 买/取回确认框；Creature 未选中按钮灰化；Craft 放入后锁定来源背包格；TrustMerchant 跨页累积排序 | #2737 #2738 #2739 #2740 |

## 6. 验证基线

- `cargo check --tests`（Client-Bevy）通过。
- `cargo test`（Client-Bevy）：425 lib + 2 bin + 1 smoke + 24 alignment 通过（批11 TrustMerchant 跨页排序后基线）。
- Report 的 C# `Prguse[1633]` 在当前本地 Data 包缺失；已使用按 C# 控件边界推导的 360x244 深色兜底面板并保留对应子控件坐标，待资源包更新后自动加载正确背景。
- ServerRust：667 lib + 6 integration 通过（批10 列表行后基线）；SharedRust 185 + 11（2 ignored）；`MapEditor/SharedRust` `cargo check` 通过（副本同步）。
- 关键实机/定向验证：UI 子树泄漏截图、Character 技能页、AssignKey 模态输入、Timer 穿透、登录安全键盘资源；批7 复验 Mail/Buff；批8 复验 Center 窗口。
- 批9 实机复验（2026-09-13，`--skip-login` + Control API `dialog`/`screenshot`，mock 3 宠物）：Creature 面板为 `Title[468]` 452x376 居中；槽位标签 `>小猪` / `很长的宠物名`（超宽截断）/ `#4`（空名字回退）互不压叠、无越界裁切；右侧操作列 RENAME / DISMISS（激活宠物）/ RELEASE / OPTIONS / ENABLE（Automatic 互斥，无半自动叠加）/ 刷新 均在 C# 坐标；信息行 (19,161)/(19,176) 显示「宠物: 3 个 ｜ 小猪 自动 饥饿:55」与操作反馈。
- 批10 实机复验（2026-09-13，Control API 打开 Craft 截图）：`Prguse[1109]` 面板（含 3 工具格 + 6 材料格轮廓）、RecipeLabel(22,5)、PossibilityLabel(10,135)、AUTO(165,185)/CRAFT(215,185) 精灵按钮、Close(312,3) 与 C# 坐标一致。
- 批10 租赁四窗实机复验（2026-09-13，`--skip-login` + `--rental-test` + Control API 截图；mock 侧 `ItemRentalRequest` 分别回 `Renting=false/true` 各截一次）：物主侧 = 自有物品窗(718,287：名称/期限/SET PERIOD/RENT/锁形) + 对方费用窗(718,163：对方名「bevy2char」/费用/金币价格钮/锁形)；租客侧 = 自有费用窗 + 对方物品窗（对方名 + 期限 + 只读物品格 + 禁用 SET PERIOD/RENT）。`visible` RPC 每端恰好 2 个 `ItemRental` 根（四窗中只显 2），确认角色分流生效。
- 批10 筛选树实机复验（2026-09-13，`--skip-login` + Control API `dialog trust_merchant open` 截图）：左列 8 个主项按 C# 文案与坐标渲染、首项「显示所有物品」为选中帧(`Prguse2[921]`)、右侧滚动条上下箭头就位、手柄因 `PossibleTotal(8) <= MaxLines(19)` 正确隐藏；临时把进入页签时的 `filter_index` 设为 2（衣服类物品）复截：主项高亮 + 5 个子项（护甲/头盔/腰带/靴子/宝石/石头）按 +2 间隔与 21 步进缩进排布，与 C# `DrawFilters` 一致（验证后已还原）。
- 批10 寄售页签面板实机复验（2026-09-13，`--skip-login` + Control API 打开 trust_merchant 截图；临时把进入页签时的 `panel` 设为 `Consign`，验证后已还原）：背景为 `Title[787]`（带寄售表格网格）、表头「出售物品/物品/价格/到期」+「出售价格」(15,142) 就位、物品格(47,104) 显示 C# 空置态浅黄半透明底、售价框(15,165) 按无效态显示红底、SELL(39,188) 呈禁用暗化、底栏 COLLECT(300,448) 与 BUY（`Title[706..708]` 用户模式帧，380,448）就位、说明文案按 115px 宽换行显示、筛选树整列隐藏；与 C# `TMerchantDialog(MarketPanelType.Consign)` 的显隐一致。
- 批10 列表行实机复验（2026-09-13，`--skip-login --market-buy` + Control API 打开 trust_merchant 截图）：两行按 `(127,82)`/`(127,115)` 渲染，图标（`Items[853]` 书页图）+ 名称 `#853`（mock 无 ItemInfo 名称回退）+ 价格 `100` / `100 出价`（拍卖后缀）+ 卖家 `bevychar` + 到期 `20/09/26 04:20:50`（mock 用当前-1h + 7 天）与表头列对齐；临时置 `selected=Some(0)` 复截（已还原）：选中行出现 1px 橙框、底栏 BUY 由暗化转为亮态。
- 批10 #2732 漏项修复复验（2026-09-13）：`MarketPanelSprites` 资源此前**从未插入**（`market_panel_system` 拿不到 → 786/787 背景与 Buy 精灵切换实为死代码），本批次补插后重截寄售页签：左列出现 `Title[787]` 位图自带的说明/格子底纹（#2732 截图里是 786 的纯暗底），确认切换生效。
- 批10 价格排序/Mail 实机复验（2026-09-13，`--skip-login --market-buy` + Control API 截图；临时置 `price_filter=Low`、`selected=Some(0)`，验证后已还原）：价格表头右侧出现 `Prguse2[925]` 蓝色下三角（(371,65)）；底栏出现 Mail 键（`Prguse[437]` 信封图，(350,448)，有选中行时为亮态），位于 Refresh(320) 与 Buy(380) 之间。
- 批11 买/取回确认框实机复验（2026-09-13，`--skip-login --market-buy` + Control API 截图；临时把确认框设为可见并填 `ConfirmBuyItemWithPrice` 文案，验证后已还原）：`Prguse[360]` 456x190 居中面板 + 文本「确定要以12,345 金币购买屠龙吗？」(35,35) + YES(260,157)/NO(360,157) 76x25 精灵按钮，与 C# `MirMessageBox` YesNo 一致。
- 批11 Creature 禁用态实机复验（2026-09-13，`--skip-login` + Control API `dialog creature open` 截图；临时注释掉打开时的宠物列表请求以制造「无选中」状态，验证后已还原）：无选中时 RENAME/OPTIONS/DISABLE(=SemiAuto)/RELEASE/**SUMMON** 全部可见但灰化、DISMISS 隐藏；有选中（mock 小猪）时改为 RENAME/OPTIONS/DISABLE/RELEASE 亮态且 Dismiss 顶替 Summon——两态与 C# `RefreshUI` error/else 分支一致。
- 批11 Craft 背包格锁定实机复验（2026-09-13，`--auto-enter`（真实登录链路，mock 才有背包物品）+ Control API 打开 Inventory/Craft 截图；mock 不下发 `PanelType::Craft` 商品行，故临时注入一条与 mock 背包匹配的配方，并用 `Interaction::Pressed` 驱动真实放入分支，验证后均已还原）：放入「布衣」后 Craft 材料格显示该物品且提示「放入 布衣」，来源背包格（第 3 格）图标由均值 RGB(70,35,47) 压暗到 (29,13,19)（≈`Color.DimGray` 0.41 系数），同排其它格像素不变。
- 批11 Craft 锁定门禁（2026-09-13）：`craft_lock_sync_matches_placed_slots`（放入/AutoFill 锁定、幂等、取出一格解锁、`ResetCells` 全解锁）与 `inv_locked_slots_match_csharp` / `inv_locked_slot_is_not_clickable`（DimGray 图标色、锁定格不可命中）；红检：把 `sync_craft_locks` 改成只清空不加锁、`inv_clickable_slot` 改成恒真，两条断言分别如期失败。
- 批11 TrustMerchant 跨页排序实机复验（2026-09-13，`--skip-login --market-many` + Control API 打开 trust_merchant；mock 新增 3 页共 25 条、价格逐条递减 1000→760，临时驱动「排序→请求第 2 页→本地回第 1 页」，验证后已删除）：全量升序排序下第 1 页由 `1,000…910`（卖家0…9）变为 `810…900`（卖家19…10，全部来自服务器第 2 页）——证明排序跨页而非只排当前页；第 2 页（加载后自动跳转）显示全量第 11–20 名 `910…1000`；本地回第 1 页不触发服务器请求（mock 未收到 `MarketPage` 日志）。
- 批11 TrustMerchant 分页 mock 对齐（2026-09-13）：`MockNPCMarket` 改为按页数下发页名、`MarketSearch`/`MarketRefresh` 只回第 1 页、新增 `C.MarketPage` 分支按 10 条/页切片（此前 mock 把全部条数当一页回，与服务端 `start = page * 10` 不一致），并新增 `--market-many` 造多页数据供实机验证。
- 批11 收尾三窗复验（2026-09-13，合并后单一进程 `--skip-login --market-many` + Control API 依次开 Market/Creature/Inventory+Craft 截图）：Market = 10 行 + `第 1/3 页` + 右侧滚动条可滚范围=已累积页；Creature（mock 小猪为已召唤态）= RENAME/OPTIONS/DISABLE/DISMISS 亮态、RELEASE 灰化、无 SUMMON（与 C# `RefreshUI` 已召唤分支一致）；Craft = `Prguse[1109]` 面板 + 「未选择产物——点击左侧商品列表」提示 + AUTO/CRAFT 按钮就位。

## 7. 已知有意偏差

- Creature：C# `CreatureRenameButton` 构造即 `Visible = false` 且再无置真处（原版死控件，改名入口点不到）；Bevy 保留可用的「改名」按钮（功能补齐见 #1281），仅坐标/精灵与 C# 对齐。
- Creature：C# `CreatureInfo`/`CreatureInfo1` 是选中宠物的能力文案（`CanPickupItems`/`CanProduceBlackStones`），`CreatureInfo2` @(19,191) 未实现；Bevy 用 (19,161) 一行承载「数量 + 选中宠物名/模式/饥饿度」，语义不同但坐标对齐。
- Creature：未选中宠物时按 C# `RefreshUI` 保留按钮但灰化（`Enabled=false`）；禁用视觉仍用 `ImageNode.color` 暗化近似 C# `GrayScale`（无灰度着色器）。C# 「已召唤**其它种类**宠物」时 `SummonButton` 会切到 `Title[593..595]` 并禁用，Bevy 无该区分（按当前宠物的召唤状态处理）。
- Creature：C# `HelpPetButton`（`Prguse2[257..259]` @ `Size.Width-48,3`）在原版无 Click 处理（死控件），Bevy 未实现该占位按钮，待有宠物帮助页时再补。
- Creature：`刷新` 是 Bevy 扩展按钮（C# 无此控件），用中文文本渲染；此前借用 MessageBox 的 `Title[206..208]`（原版是「YES」精灵），实机截图里会显示成「YES」。
- Craft：C# 用客户端本地 `ItemInfo` 库解析需求图标，Rust 客户端无本地物品库 —— 图标/名称改由 `RecipeRequirement.image/name` 随包下发（协议自洽偏离，已注释说明）。
- Craft：C# 放入材料后会锁定对应背包格（`SelectedCell.Locked`）直到关窗（`AutoFill` 逐格锁定、`ResetCells()` 全解锁）；Bevy 已对齐（`InvLockedSlots` 资源 + 图标按 `Color.DimGray` 灰化 + 锁定格不响应点击/选择）。残留偏差：C# 还以 0.8 不透明度叠加绘制，Bevy 只改图标色（无 alpha 混合）；C# `MirItemCell.Locked` 的其它来源（装备/拆分/移入腰带/镶嵌等）Bevy 未逐个实现，当前仅 Craft 驱动锁定。
- Refine：C# 的待精炼武器走 NPCDialog 的 ItemCell（投放窗确认即 `C.RefineItem{UniqueID}`）；Rust 服务端语义是两步（`DepositRefineItem to=0` 存入 → `RefineItem{uid}` 发起），故 Bevy 的投放窗确认在收到存入确认后再发 `RefineItem`（对外行为等价，多一个包）。
- ItemRent：C# `KeybindOptions.Rental` 在 `KeyBindSettings.New()` 里**没有默认绑定行**（枚举成员存在但无 `KeyBind`，原版默认无键，键位面板也列不出）；Bevy 作扩展给「租赁」（界面组，默认 `T`）并在键位面板可重绑，热键 `ItemRentalDialog.Toggle()` 语义与 C# `GameScene.cs:779-781` 一致。
- ItemRent：`Prguse[238]` 面板位图**自带**右上角关闭图样（C# 四窗都画得到），但只有自有窗挂了可点关闭键（`Prguse2[360..362]`）——C# `GuestItemRentDialog`/`GuestItemRentingDialog` 本身没有 `closeButton`，Bevy 同样只在自有窗挂 `ItemRentalClose`，对方窗的 X 是装饰。
- ItemRent：`S.UpdateRentalItem` 除 C# 的 `HasData`+`LoanItem` 外仍带 `rental_fee`/`rental_period`（Rust 扩展，客户端只在 >0 时用作兜底刷新）；`S.CanConfirmItemRental`/`S.ConfirmItemRental` 在 C# 是空包，Rust 端口带 `can_confirm`/`success` 载荷（自洽偏离，双方均为 Rust 实现）。
- ItemRent：C# 租客侧的合计费用在服务端 `SetItemRentalFee` 里即时扣金币（`S.LoseGold`）；Rust 服务端到 `ConfirmItemRental` 成交时才扣，客户端费用标签两侧都由本地/对包数值驱动，显示一致。
- ItemRent：mock 单客户端下 `ItemRentalRequest` 固定回 `Renting=false`（物主侧），租客侧窗口与锁定帧的实机验证靠临时把 mock 回包改 `true` 截图（验证后已还原）。
- TrustMerchant：价格排序已对齐 C#（`Listings.AddRange` 累积后对全量排序，`UpdateInterface` 取 `orderedListings[Page*10+i]`）——客户端按页累积（`MarketState.loaded_pages`/`pending_page`），翻页与 C# 一致（Back 本地翻页、Next 已累积则本地否则发 `C.MarketPage`），滚轮范围 = 已累积页。残留偏差：`S.NPCMarketPage` 不带页号，客户端按「最近一次请求页（无未决请求 = 第 0 页）」归属累积位置，而 C# 靠 `NPCMarket`/`NPCMarketPage` 两种包区分。
- TrustMerchant：写邮件正文复用邮件窗的 `MirTextBox`（预填文本与 C# `ComposeMail(recipient, message)` 一致），换行/焦点细节未逐像素复刻。
- TrustMerchant：拍卖出价在 C# 走 `MirAmountBox`（带物品图标、默认 `Price+1`），Bevy 复用底栏价格输入框（缺省 `当前价+1`），确认框文案一致（批11 已补 `MirMessageBox`）。
- TrustMerchant：C# `AuctionRow.SelectedImage`（`Prguse[545]` 296x38）构造后 `Visible=false` 且全仓无置真处（原版死控件）；选中高亮只用 `Border`（1px 外扩橙框），Bevy 一致。
- TrustMerchant：`C.AuctionRow` 到期列在 C# 用本地墙钟 `DateTime`；Bevy 用 `chrono::Local` 格式化（同一时刻的本地显示），mock 侧写「当前-1h」便于核对格式。
- TrustMerchant：售价框边框三态（C# `PriceTextBox.BorderColour` 只画 1px 边框色）在 Bevy 用输入框底色近似；`Enabled = false` 的灰度统一用 `ImageNode.color` 暗化近似（无灰度着色器，`SellItemButton`/`BuyButton`/`CollectSoldButton`/`SellNowButton` 同）；C# 寄售选物会锁定背包格（`tempCell.Locked`），Bevy 只记录选中物、未锁背包格。
- TrustMerchant：筛选树 `Prguse2[205/206]` 手柄的拖动用「按下时记录抓取偏移 → 按住按光标 y 反算 `Skip`」实现（C# 是 `MirControl.OnMoving`）；手柄高度固定为精灵原始 12x18（C# 也未按 `PossibleTotal` 缩放）。
- TrustMerchant：Find/筛选/页签搜索改发 C# 规范 `C.MarketSearch`（此前 Bevy `MarketSearchWire` 只写关键字，被网关 `read_body` 静默丢弃 → 搜索无效）；`MarketRefresh` 仅保留在刷新按钮（C# `RefreshButton.Click` 先清空搜索框）。
- Craft：C# `BeforeDraw` 在背包关闭时会隐藏合成窗，Bevy 未实现（挂机脚本会直开 Craft，保持现状以免回归）。

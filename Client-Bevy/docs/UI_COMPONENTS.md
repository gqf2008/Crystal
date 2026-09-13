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
| Tooltip | `ui/tooltip.rs`（sprite-UI `TooltipHint` + bevy-UI `UiHint` 双通道） | `MirLabel` / Hint | 悬停、描边、避让屏幕 | 批1 / 批18 |
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
| 批12 §7 偏差收口（二） | 禁用态按 `grayscale.ps` 真灰度（含 Creature 自造灰化纠正）；背包格锁定覆盖装备/拆分/镶嵌/寄售 + 交易所推背包；拍卖出价改用 `MirAmountBox` | #2743 #2744 #2745 |
| 批13 §7 偏差收口（三） | 锁定格 `DimGray` × 0.8 不透明度；数量框补 1px 三态边框 + 售价框去掉自造三态底色（§7 描述更正）；Mail 两个灰度禁用占位键 | #2748 #2749 #2750 |
| 批14 背包锁定 per-grid 化 | 锁资源升为 `(reason, LockGrid, index)`；仓库存入/取出锁与解锁（含 C# 目标格选择）；交易存入/取回锁与解锁 + 交易槽灰化 | #2753 #2754 |
| 批15 宠物规则 + Creature 信息行 | 服务端移植 C# `IntelligentCreatureInfo` 静态表并随 `UpdateIntelligentCreatureList` 下发规则；客户端 `CreatureInfo`/`CreatureInfo1`/`CreatureInfo2` 三行按 C# 文案与 (19,161)/(19,176)/(19,191) 落地 | #2758 #2759 #2760 |
| 批16 Creature 面板剩余控件 | 列表包补 `icon/fullness/expire/blackstone` + C# 尾部三字段（召唤态/召唤种类/玩家珍珠数）；完整度条 + 黑石条 + 刻度与悬停；槽位图标/选中框；`CreatureName`/`CreatureDeadline`/`CreaturePearls`；面板宠物动画（帧表 + 8 秒交替）与「已召唤其它种类」按钮态 | #2762 #2763 #2764 #2765 |
| 批17 悬停提示闭环 + Hint 覆盖 | 控制接口加光标探针（`cursor` RPC）+ `nearby` 带视口坐标，把「自动化无光标」的复现障碍做进产品；修提示框 CJK 豆腐字（Arial → 共享宋体主字体）；补技能栏格 `SkillMpCooldownKey` 与大地图（搜索NPC/队友名）Hint，时间格式收进 `game::time_format` | #2768 #2769 #2770 |
| 批18 悬停提示长尾（一） | 玩家右键菜单 5 项 Hint + 新增 `KeybindOptions.Trade`（按 C# 默认 T，热键发 `C.TradeRequest`；租赁扩展键让位改 `;`）；修菜单字体豆腐与菜单项顺序；新增**通用按钮 Hint 通道**（`UiHint`/`ui_hint_system`，沿 `ChildOf` 累加求绝对原点）并补 Mail/Group/Friend 三窗按钮 Hint | #2772 #2773 |
| 批19 悬停提示长尾（二） | Ranking/Relationship/Mentor/GuildTerritory/Hero/腰带/耐久/音量/技能页 103 条描述等 Hint；**提示面板由 sprite 层迁到 bevy_ui 置顶**（旧实现被对话框整块盖住）；修空提示框与描边副本残留；菜单窗 13 项、小地图条 3 项、`char_page`/`player_menu` 控制接口 | #2776 #2777 #2778 #2779 |
| 批20 聊天控制栏 + 三档尺寸 | C# `ChatControlBar` 落地（9 按钮 + 发送前缀语义 + Hint + 交易/设置）；聊天窗口 0/1/2 档（面板 2221/2224/2227、行数 4/7/11、底边固定向上长高）；腰带随档位上移 + 滚动条轨道换图/滑块比例；新增 `chat_size` 控制接口 | #2782 #2783 #2784 |
| 批21 §7 剩余缺口（一） | 观察窗伴侣钮（`PlayerInspect` 协议补 `lover_name`：服务端两处写入 + mock + 客户端解析 + 两端字节契约单测）+ 观察窗 CJK 字体；Mail 回复钮 + GuildTerritory 邮件会长钮（复用写信链路）；伴侣钮已婚/未婚两态动态 Hint | #2787 #2788 #2789 |
| 批22 §7 剩余缺口（二） | ① 英雄管理窗（`Prguse[1688]` + 8 槽头像 + 当前头像 + `Hint = info.ToString()` + MakeActiveHero 确认 → `C.ChangeHero`；`ManageHeroes` 补 `max_count`：C# 语义 = `maximum_hero_count + 1`；新增独立 `DialogKind::HeroManage` + `dialog hero_manage` 控制接口）② 商城付款方式复选框 + 完整 `BuyProduct`（`Prguse[2086/2087]` 互斥 + 余额标签 + 未选/余额不足聊天提示 + MirMessageBox 确认；`GameShopInfo` 商品项补 `can_buy_credit/can_buy_gold` 两字节，SharedRust + MapEditor 副本 + 服务端 + 客户端解析同步）③ 世界头顶提示按「光标在对话框上」门控（`cursor_over_dialog_rect`）④ BuffDialog 图标行 + Hint（`Prguse2[20..30]` 右缘锚定 + `BuffIcon[BuffImage]` + `BuffString`/`CombinedBuffText` 文案表；`AddBuff` wire 补 `remaining_ms/paused/values`） | #2792 #2793 #2794 #2795 |
| 批23 §7 收尾（三） | ① Exp/Drop 加成进 Buff 窗（服务端 `SetExpMultiplier`/`SetDropMultiplier` 带显示载荷：生效发 `S.AddBuff`、到期发 `S.RemoveBuff`，tag 29/30 = C# `BuffType.Exp/Drop`；客户端显示表补图标 260/162 + `ExpRatePercent`/`ItemDropRatePercent` 属性行）② Buff 窗按 C# `Movable = false` 不可拖动（`NotDraggable` + `dialog_drag_system` 排除）③ §7 记录结论落档（`PoisonBuffDialog` = 原版 `//UNFINISHED` 死代码，不实现） | #2798 #2799 #2800 |
| 批24 任务详情窗（QuestDetailDialog） | 整窗对齐 C#（`QuestDialogs.cs:463-628 / 1003-1390 / 1396-1745`）：① `Prguse[960]` 面板 + `Title[16]` + `Prguse2[360..362]` 关闭键 + 独立 `DialogKind::QuestDetail` + **日记已接行左键打开**（`:1928-1935`）② `QuestMessage` 消息区（行模型 `UpdateQuest`/`AdjustDescription`、16 行槽、上下滚 `Prguse2[197..199]/[207..209]`、位置条 `Prguse2[205/206]` 拖动含原版 `Count-1` 钳位、标题圆点 `Prguse[919]`、首行黄、`{文本/颜色}` 去标记）③ 分享键→`C.ShareQuest`、取消键→`MirMessageBox(AskCancelQuest)`→Yes `C.AbandonQuest`+`Hide()`、`_pauseButton` 按 C# 死控件处理；奖励区 @(5,307)（`Title[17]` + `Prguse[966/965/2447]` 图标与数值偏移链 + 固定/可选各 5 格 + `Prguse[989]/[979]` 底 + 物品图居中 + `###0` 数量 + 多选一记未过滤下标 + `FilterRewards` 性别过滤）；**协议补齐** `QuestItemReward` 携带完整 `ItemInfo`（C# `SharedData.cs:75-93`，客户端无本地物品库）+ 顺带修 `close` 宽查询吞掉接受/完成键按下边沿（`#2535` 状态机整条不可用） | #2802 #2803 #2804 |

## 6. 验证基线

- `cargo check --tests`（Client-Bevy）通过。
- `cargo test`（Client-Bevy）：**513 lib** + 2 bin + 1 smoke + 24 alignment 通过（批24 单元③ 后基线）；ServerRust **680 lib** + 6 integration（批24 单元③ 后，含 `QuestItemReward` ItemInfo 协议）；SharedRust **187 + 11**（2 ignored，含 `QuestItemReward` 往返无损）。
- Report 的 C# `Prguse[1633]` 在当前本地 Data 包缺失；已使用按 C# 控件边界推导的 360x244 深色兜底面板并保留对应子控件坐标，待资源包更新后自动加载正确背景。
- ServerRust：680 lib + 6 integration 通过（批24 单元③ 后；批16 基线为 673 lib）；SharedRust 187 + 11（2 ignored）；`MapEditor/SharedRust` `cargo check` 通过（副本同步，批24 单元③ 改 `QuestItemReward` 时同步）。
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
- 批12 禁用态灰度实机复验（2026-09-13，`--skip-login --market-many` + Control API；「有选中」态用临时系统置 `market.selected = Some(0)`，验证后已删除）：Market 无选中时 BUY 区域平均 RGB `(85.3,85.3,85.3)`、色度 `max|R-G|+|G-B| = 0`（真灰度 = `grayscale.ps`），同屏可用的 REFRESH/FIND 保留色度（26/40）——即「灰度只作用于禁用的那一键」；临时选中第 1 行后 BUY 变为 `(101.1,82.7,57.1)`、色度 44（`GrayScale=false` 回原色）。Craft 无配方时 CRAFT 按钮 `(85.4,85.4,85.4)` 色度 0、同排 AUTO `(102.9,92.4,75.4)` 色度 28。
- 批12 灰度门禁（2026-09-13）：`gray_pixel_matches_csharp_shader`（0.3/0.59/0.11 逐通道 + alpha 保留）、`gray_rgba_keeps_size_and_alpha`、`gray_cache_round_trips_and_dedups`（同源去重 + 灰度→源回溯）、`craft_button_enabled_matches_refresh_craft_cells`、`market_bottom_buttons_gray_when_disabled`；红检：把 `GRAY_COEFFS[0]` 改成 0.25、把 `craft_button_enabled` 改成恒真，两条断言分别如期失败。
- 批12 背包锁定实机复验（2026-09-13，`--auto-enter --market-many` + Control API；寄售放入/切页签用临时驱动系统注入 `Interaction::Pressed` 与 `market.panel`，验证后已删除）：切到寄售页签并点寄售格后，来源背包格（第 3 格）图标均值 RGB `(73.7,36.6,49.6)` → `(30.2,13.7,19.3)`、色度 52.7 → 23.1（≈`Color.DimGray` 0.412 系数）；切回市场页签后恢复 `(73.7,36.6,49.6)`/52.7。同屏可见 C# `Show()` 的背包右移（`Size.Width + 5 = 497`，`InventoryPlaceAt`）。
- 批12 背包锁定门禁（2026-09-13）：`inv_lock_reasons_are_isolated`（Craft 收敛不动其它来源、同格多来源需全部解锁）、`inventory_events_release_source_locks`（`ItemEquipped`/`EquipSlotItemResult`/`SplitItem1Result` 分别解锁 Equip/Socket/Split，Craft/寄售不受影响）、`use_item_core_reports_source_lock_reason`（背包来源装备→Equip、仓库来源→不锁）、`inventory_shift_right_repositions_entities_and_origin`（新增 `InventoryPlaceAt` 复位断言）；红检：`unlock_all` 改空实现、`use_item_core` 不登记锁来源，两条断言分别如期失败。
- 批12 拍卖出价数量框实机复验（2026-09-13，`--skip-login --market-buy` + Control API；选中拍卖行与按 BUY 用临时驱动注入 `Interaction::Pressed`、出价金额用注入 `AmountBoxResult`，验证后已删除）：数量框为 `Prguse[238]` 居中 204x109 + 标题「出价金额」+ 物品图标（`Items[853]`，@(15,34) 38x34）+ 默认值 `151`（= 当前价 150 + 1）+ OKAY/CANCEL；注入 200 后弹确认框「你确定要为#853出价200金币吗？」+ YES/NO，与 C# `MirAmountBox` → `MirMessageBox` 两步一致。
- 批12 拍卖出价门禁（2026-09-13）：`market_buy_outcome_matches_csharp`（拍卖分支改为 `BidAmount`）、`market_bid_uses_amount_box_like_csharp`（默认/下限 = 当前价+1、金额确定后确认框文案与动作、低于下限钳制、取消不弹确认）；红检：把 `ask_with` 初值改回 `max` 忽略默认值 → 断言如期失败。
- 批12 收尾三窗复验（2026-09-13，合并后单一进程 `--auto-enter --market-buy` + Control API 依次开 Market/Creature/Craft 截图）：Market = 两行（寄售 100 / 拍卖「100 出价」）+ 无选中时 BUY 仍为真灰度（均值 `(85.3,85.3,85.3)`、色度 0）+ 背包按 C# 推到 x=497 且第 3 格图标为原色 `(73.7,36.6,49.6)`（无残留锁）；Creature = PET STATUS 面板与操作列正常；Craft = `Prguse[1109]` 面板 + `CRAFT` 按钮灰度（AUTO 保持原色）。
- 批13 锁定格透明度实机复验（2026-09-13，`--auto-enter --market-many` + Control API；寄售放入/切页签用临时驱动，验证后已删除）：同一来源格（第 3 格）未锁均值 `(73.7,36.6,49.6)`；锁定后 `(27.4,12.5,17.5)`，locked/unlocked 比例 **0.371/0.342/0.352**（批12 无 alpha 时为 0.410/0.374/0.389）——与 C# `0.412 × 0.8 = 0.330` 加单元格底色的 alpha 混合一致。
- 批13 输入框边框实机复验（2026-09-13，`--skip-login --market-buy` + Control API；选拍卖行/按 BUY/改数量/切页签用临时驱动，验证后已删除）：数量框 `Lime` 态 = 绿 1px 边框 + `OKAY` 可见（默认 151）；把值改成 100（低于下限 151）→ **红 1px 边框 + OKAY 隐藏**（只剩 CANCEL），与 C# `TextBox_TextChanged` 合法/非法分支一致；切到寄售页签后售价框为纯深色输入框（不再有红/绿/橙自造底色），SELL 键保持禁用灰。
- 批13 Mail 占位键实机复验（2026-09-13，`--skip-login` + Control API `dialog mail open` 截图）：底栏右侧出现 `Prguse[520]`/`[523]` 两个键，均值 RGB `(110.4,110.6,109.3)`/`(114.4,114.7,112.9)`、色度 `1.46`/`2.15`（≈0 = 批12 灰度公式），同排可用的发送/删除键保持棕色原色 —— 与 C# `GrayScale=true, Enabled=false` 占位态一致。
- 批14 仓储锁实机复验（2026-09-13，`--auto-enter` + Control API `npc_call{110,"[@STORAGE]"}` 打开仓库、`dialog npc close` 关掉 NPC 页；临时把 mock 仓储密码置空以到达面板，**验证后已还原**；锁定/解锁用临时驱动，**验证后已删除**）：临时锁 `(Storage,3)` + `(Inventory,3)` 后，仓储格 3 均值 RGB `(38.2,28.2,17.9)` 色度 20.6（对照：同行未锁的仓储格 4 `(89.1,69.6,46.1)` 色度 43.3 不变），背包格 3 `(26.3,12.2,16.9)` 色度 19.6；解锁后仓储格 3 恢复 `(93.6,73.2,48.3)` 色度 45.7、背包格 3 恢复 `(70.3,35.1,47.4)` 色度 50.1 —— 两侧网格同色规则、被锁格与对照格差异显著。
- 批14 仓储锁门禁（2026-09-13）：`store_target_slot_matches_csharp`（C# 目标格选择：空格用它/占用取首个空格/全满 None）、`store_receipt_releases_grid_locks`（`S.StoreItem` 回包清 `(Storage,·)` 两侧锁、Craft 来源不受影响）、`inv_lock_reasons_are_isolated` 扩充（同格号在 `LockGrid::Storage` 与 `Inventory` 互不影响 + 仓储锁定格同色）；红检：把 `store_target_slot` 改成恒返回点击格、把 `is_locked_in` 改成忽略网格，两条断言分别如期 FAILED。
- 批14 交易锁门禁（2026-09-13）：`trade_receipts_release_locks`（`S.DepositTradeItem` 失败也清 `Trade` 两侧锁且不动 Craft 来源；`S.RetrieveTradeItem` 失败保留槽内物品、成功才清空，两种都解锁）；红检：把成功分支改成恒真（`if true`）→ 断言「取回失败应保留槽内物品」如期 FAILED。实机验证：交易需双端会话，mock 单客户端下用系统级测试覆盖；被锁交易槽的灰化与背包侧共用同一 `LOCKED_ITEM_COLOR` 与 `color_at` 路径（同批仓储/背包已实机确认）。
- 批14 收尾复验（2026-09-13，合并后单一进程 `--auto-enter --market-many` + Control API 依次开 Mail/TrustMerchant/Craft 截图）：四窗渲染正常、既有灰度无回归 —— Market 无选中 BUY 均值 `(85.3,85.3,85.3)` 色度 **0.00**、Mail 占位键区域色度 5.36（≈灰，区域被同屏其它面板部分遮挡故均值与前次单开不同）；锁资源新增的网格维度不影响既有五类背包锁（438 lib 全绿覆盖）。
- 批13 收尾三窗复验（2026-09-13，合并后单一进程 `--skip-login` + Control API 依次开 Mail/TrustMerchant/Craft 截图）：Mail 底栏占位键灰度保持；TrustMerchant 底栏无选中时 BUY 灰度、售价框无自造底色；Craft `CRAFT` 灰度 / `AUTO` 原色、面板几何无变化。
- 批15 宠物规则协议实机复验（2026-09-13，`--auto-enter` + Control API `dialog creature open`，截图转 JPG）：mock 两条样本（C# `Chick` 行 + `BabyPig` 行，默认选中第 1 条）下三行依次为「可以拾取物品（7x7 auto/semi-auto, 11x11 mouse）。」/「可以产出黑石。」/「可以产出珍珠，用于购买召唤兽物品。」；真实点击第 2 条后第 1 行变为「可以拾取物品（0x0 semi-auto0x0 mouse）。」、后两行为空 —— 既证明规则逐宠物下发，也证明 `:729-730` 的原版拼接怪癖逐字复刻；Bevy 摘要行 (19,206) 与操作反馈行 (19,243) 与 C# 控件无重叠（面板内其余空档：槽下区域已进入面板底纹，x<113 仅 94px 宽）。
- 批15 宠物规则门禁（2026-09-13）：服务端 `creature_rules_mirror_csharp_table`（C# 静态表逐项 + 本端独有类型全禁用默认）；客户端 `creature_list_decodes_rules`（按 wire 字节序解码规则）、`creature_list_without_rules_falls_back_to_disabled`（老服务端缺字段仍可解析）、`mock_creature_list_matches_csharp_rules`（mock 与真实服务端同编码路径）；文案 `creature_info_pickup_text_matches_csharp` / `creature_info_empty_cases` / `creature_info_mouse_range_uses_its_own_range`。红检：`BabyChicken` 分支改全禁用、规则读取挪到 `filter/grade` 之前、`semi` 的「NxN」改用 `SemiAutoPickupRange` —— 三处断言分别如期 FAILED（`(0,0,0,false)` vs `(11,7,7,true)`；`(2,42,0)` vs `(2,42,3)`；`3x0 semi-auto0x0 mouse` vs `0x0 semi-auto0x0 mouse`）。
- 批16 跨端字节契约（2026-09-13）：服务端 `test_creature_list_body_matches_documented_wire` 断言 `build_creature_list_body` 输出等于按 wire 文档**手推**的 72 字节；客户端 `creature_list_wire_contract_matches_server_literal` 断言**同一串字节**解出 `icon 500 / fullness 4000 / expire 604800 / blackstone 3600` 与尾字段 `(true,2,777)` —— 两端各自按同一条文档手推，互为见证。红检：服务端把 `icon`/`fullness` 写出顺序对调 → FAILED（`…160,15,…244,1…` vs `…244,1,…160,15…`）；客户端把读取顺序对调 → FAILED（`(4000,500,…)` vs `(500,4000,…)`）。
- 批16 实机复验（2026-09-13，`--auto-enter` + Control API，截图转 JPG）：合并态面板一次截图核对全部新增项 —— NAME 框内居中「小鸡」、第二行「过期: 7d 00h 00m 00s」、完整度条 75%（Min 刻度 10% = `MinimalFullness 1000`、Now 刻度 75%）、黑石条 33%（`3600/10800` 蓝段 57px/172）、槽位 chick/pig 图标 + `Prguse2[535]` 选中框、珍珠图标 + `1234`、三行信息 / 摘要 (206) / 反馈 (333) 互不压叠；相隔 0.4s 两张截图在宠物框 190x140 区域内有 3589 px（13.5%）差异 → 面板动画在推进。
- 批16 悬停/按钮态实机复验（2026-09-13，同上）：悬停文案用**临时固定 cursor** 驱动一次真实分支（共享桌面 OS `SetCursorPos` 会被抢焦点、`cursor_position()` 读不到新值），截得 `7500 / 10000` 居中显示在条身上，验证后已还原（`Select-String TEMP` 复核为空）；「已召唤其它种类」用**临时 mock 尾字段**（声明小猪召唤中 + 小鸡未激活）截得原 DISMISS 位置改显 `Title[593..595]` 的 SUMMON 帧、Dismiss 隐藏，验证后已还原为 `6/1`。
- 批16 mock 编号修正（2026-09-13）：`MockCreatureList` 的宠物类型字节此前用共享枚举（C# 编号：小鸡 4 / 小猪 3），而真实服务端用 ServerRust `CreatureType`（小鸡 6 / 小猪 2）→ 实机里「小鸡」会按类型 4 播成 BabySkeleton 帧（已截图复现）。现改用 ServerRust 编号并同步尾部 `summoned_type`。属「mock 必须模拟线上格式」类缺陷：mock 只用「同一个包里自洽」不够，必须与服务端实际枚举一致。
- 批17 光标探针 + 悬停闭环（2026-09-13，`--auto-enter` + Control API，截图转 JPG）：控制接口新增 `cursor {x,y}` / `cursor {clear:true}`，`nearby` 每条实体带 `vp`（世界→逻辑视口）。用探针驱动真实悬停链路：`bevy2char` 视口 `(560,384)`、探针 `(570,354)` 命中 → 头顶显示「bevy2char」（黄标题 + 白行）；负控把探针移到 `(10,10)` → 同区域 65.9% 像素变化、均值亮度 57→92（提示消失）。
- 批17 提示框字体修正（2026-09-13）：提示框文本原用 `UiFont`(Arial)，parley 的 Hani 回退只在实体首次排版生效 → 悬停换文本后退化成豆腐字（#2599 同类），实机把「怪物5」渲染成「□□5」。改用共享宋体主字体（`shared_cjk_font`，与 NPC/公告等动态文本一致）后正确显示「怪物5」/「bevy2char」。
- 批17 Hint 补齐实机复验（2026-09-13，同上，截图转 JPG）：技能栏（`Mir2Config.ini` `Skillbar0X=220` → 格 0 中心 UI (247,14)）探针命中后显示「攻杀剑术 / 魔法值: 4 / 冷却时间: 0.0s / 键位: F1」；大地图搜索按钮（面板局部 (39,479)）显示「搜索NPC」；负控移开探针后技能栏提示区 36.9% 像素变化、均值亮度 78.3→91.3（提示消失）。队友名提示因 mock 无队伍数据未实机触发，命中半径与同图过滤由单测覆盖。
- 批18 玩家菜单 Hint 实机复验（2026-09-13，`--auto-enter` + 新增控制接口 `player_menu {object_id}` 打开右键菜单，截图转 JPG）：探针依次悬停显示「邀请加入队伍 / 添加到好友列表 / 发送邮件 / 交易 (T) / 观战」（末项键位按 C# `MainDialogs.cs:2296` 拼接）；负控移开探针后菜单提示消失。过程中实机发现并修掉两个真 bug：菜单项文案整体豆腐（与批17 同源：Arial 的 Hani 回退只在实体首次排版生效）与菜单项顺序错乱（原先按 ECS `Query` 迭代序自增索引，改为 `PlayerMenuOption{action,index}` 按生成序取值）。
- 批18 通用按钮 Hint 实机复验（2026-09-13，同上，截图转 JPG）：Mail「读取/发送/删除」、Group「允许/拒绝队伍请求/添加/移除」、Friend「添加/移除/备注/邮件/悄悄话」在真实悬停链路下均正确显示。首个版本「取同 `DialogKind` 的第一个根面板算绝对原点」在 Group 上整体错位（该 kind 同时存在邀请确认框与主面板），改沿 `ChildOf` 链累加各级 `Node.left/top` 后命中正确。
- 批18 门禁（2026-09-13）：`player_menu_hint`（5 项文案 + 键位拼接）、`ui_hint_hit_covers_rect_and_borders`（绝对矩形命中，边界含等号）、交易键位默认表 + 热键发 `C.TradeRequest`、租赁键位归还分号键；红检：把 `ui_hint_hit` 边界临时改成 `>`/`<` →「左上角含边界」断言如期 FAILED，还原 `>=`/`<=` 后 PASS。
- 批19 提示面板置顶 + 对话框 Hint 实机复验（2026-09-13，`--auto-enter` + 光标探针，截图转 JPG）：排行「总榜前 20/弓箭手前 20」、关系「允许阻止结婚/发送悄悄话给伴侣」、师徒「移除师徒关系」、领地「购买」、英雄「背包 (Ctrl + I)」「技能 (Ctrl + S)」、英雄/药水腰带「旋转」「关闭 (Z)」全部显示。关键发现与修复：提示框原画在 sprite 层，而同一相机里 bevy_ui 节点整体画在 sprite 之后 → 光标落在对话框内时提示必然被该对话框盖住（调试日志证明状态已置位、整屏无提示；批18 的 Mail/Group/Friend 只是恰好落在面板边界外）；迁到 `GlobalZIndex(90)` 的 bevy_ui 面板后对**所有**对话框可见。
- 批19 菜单窗/小地图条 Hint 实机复验（2026-09-13，同上）：菜单「退出 (Alt + Q)」「帮助 (H)」「师徒 ()」（C# `KeybindOptions.Mentor` 默认 `Keys.None` → 原版就渲染「师徒 ()」，逐字复刻）、「公会 (G)」；小地图条「小地图 (V)」「邮件」「大地图 (B)」。
- 批19 动态 Hint 实机复验（2026-09-13，同上）：设置窗音效条「80%」/音乐条「60%」（两值不同说明逐条绑定正确）、耐久钮「耐久面板」、英雄行为按钮「英雄行为：攻击」「英雄行为：自动」。实机暴露并修复两个提示框缺陷：① 命中空文案 `UiHint` 会写入 `lines=[""]` → 空提示框（队伍无成员时可见）；② 隐藏标题/行时只隐藏正文不清空 → 4 个黑描边副本残留暗字。
- 批19 技能页 Hint 复验 + 门禁（2026-09-13，`char_page {page:3}` + 光标探针）：第 1 行「攻杀剑术」显示「攻杀剑术／被动技能／…／当前技能等级 1／下一等级 2」，第 2 行「刺杀剑术」显示对应描述（含原版缺空行版式）；扩展点把正文 `\n` 计入行数后框体尺寸才够。红检：`skill_hint` 的 `{2}` 替换变异成空串 → FAILED；`binding_key_text` 的 `RequireCtrl == 1` 变异成 `!= 2` → FAILED；`behaviour_hint(3)` 变异成「自定义」→ FAILED。顺带修 mock：第二个魔法名字是「刺杀剑术」而 `spell` 写成 `Spell::Fencing`（应为 `Thrusting`），否则技能页 Hint 显示成另一技能（mock 枚举取值必须与线上一致，批16 CreatureType 同类）。
- 批20 聊天控制栏实机复验（2026-09-13，`--auto-enter` + 光标探针，截图转 JPG）：`Prguse[2034]` 控制栏 @(230,656) 按 C# 渲染 9 个按钮，「全部」呈选中（按下）帧，悬停依次显示「全部/喊话/情侣/交易 (T)/聊天设置」；负控无提示。控制栏此前**整条未实现**（`chat.rs` 的 `ChatTabBtn`/`ChatBarBtn`/`ChatSettingsBtn` 三个组件只有声明与系统分支、从未 spawn）。
- 批20 聊天三档尺寸实机复验（2026-09-13，`chat_size {size}` RPC 驱动，截图转 JPG）：0 档面板 632x68 紧贴控制栏；1/2 档面板向上长高 +48/+96（**底边固定**）、控制栏与 Home/Up 随之上移、行数 4→7→11；2 档下两条横向腰带（药水/英雄）随之上移到 522（= 控制栏顶边 560 − 腰带高 38），不再与聊天窗重叠；切回 0 档整体复位。C# `ChangeSize` 的「Down/End/输入框相对位置 +48*size」抵消长高 → 绝对坐标不变，Bevy 生成期即绝对值，无需处理（单测 `chat_size_tops_follow_bottom_anchor` 钉住）。
- 批21 观察窗伴侣钮实机复验（2026-09-13，`--auto-enter --inspect-test` + 光标探针，截图转 JPG）：观察窗左上角 (17,17) 出现 `Prguse[604]` 红心钮（配偶名非空才显示），悬停显示「示范伴侣」（= mock 下发的配偶名）；顺带修掉观察窗行会标签的整行豆腐（原用 Arial，改共享宋体）。协议侧：`PlayerInspect` 身份段补 `lover_name`（服务端 `inspect_identity_bytes` 两处调用 + 客户端 `parse_inspect_identity` + mock），两端用同一串手推字节互为见证。
- 批21 邮件回复/领地邮件 + 伴侣钮两态实机复验（2026-09-13，同上）：Mail 底栏 4 键（发送/回复/读取/删除）悬停回复键显示「回复」；GuildTerritory (262,208) 信封键显示「发送邮件给公会会长」；关系窗伴侣钮未婚态显示「允许/禁止传送」、已婚态（临时开关摆状态，验证后已删）显示「允许/阻止结婚」；负控均无提示。

- 批22 实机/JPG 复验（2026-09-13，`--auto-enter`（+`--buff-test`）与 Control API `cursor` 探针/真实光标点击；`DialogKind::HeroManage` 与 `dialog hero_manage` 控制入口）：① 英雄管理窗 `Prguse[1688]` @(350,350) + 8 槽头像（占用槽 36x30 @+5,+5、超名额槽 `Prguse[1689]` 空框）→ 悬停槽 0 得「英雄小刀 / Level 30 male warrior」，点槽 → MakeActiveHero 确认框 → YES 发 `C.ChangeHero` 且左侧当前头像同步；② 商城付款行 @(250/340,449) 金币/积分互斥勾选、余额标签 0 / 10,000，三种购买分支（金币成交、积分不可购、积分余额不足）日志逐条对账；③ 同一视口坐标下对话框开着无世界头顶提示、全关后恢复；④ Buff 图标行右缘 898（贴小地图左缘）+ 逐图标 Hint（`魔法盾/增加 伤害减免 ： 15%/过期: 19s` 等）+ 收起态「3」与 `当前增益效果` 合计。

- 批23 实机/JPG 复验（2026-09-13，`--auto-enter`（含 `--buff-test`）+ 真实鼠标右键/拖拽 + `cursor` 探针）：① 使用双倍经验药水/掉率加成药水（mock 按 shape 4/5 回发 AddBuff）→ Buff 窗出现 `BuffIcon[260]/[162]` 两枚图标，悬停分别得「经验加成 / 增加 经验 ： 50% / 过期: 29m 23s」与「掉率加成 / 增加 物品掉落 ： 120% / 过期: 29m 58s」；② 展开态 3 个 buff 下从面板内真实拖拽 → 面板仍锚右缘 898（无「拖动对话框 Buff」日志）。
- 批24 实机/JPG 复验（2026-09-14，`--auto-enter --quest-data-test` + Control API；机器全程停在 Windows 锁屏（前台 = `Windows 默认锁屏界面`），`SetCursorPos`/`SendInput`/`PostMessage` 三种注入到不了 winit，故渲染类证据走 **RPC 等价入口** `quest_detail {quest_id[,top_line][,confirm]}`（写同一份 `QuestDetailState`，与点日记已接行/点滚动键/点取消键同状态），点击类语义由系统级单测钉住）：
  - **单元①**（锁屏前取得，真实左键）：点任务日记「已接」行 → `QuestDetail` 进管理栈开窗；点关闭键 @(833,73) → 关窗；修复 `close` 宽查询后点 ACCEPT 实测收到 `C.AcceptQuest` → mock 回 `ChangeQuest` → 任务进入已接列表。
  - **单元②**：任务日记（「未分组」组头 + `Lv1 击杀 稻草人 0/3（进行中）` + 追踪钮）与详情窗 `Prguse[960]` @(532,60) + `Title[16]` + `Prguse2[360..362]` 并排；第 1 页 = 首行黄任务名 + 8 行描述 + 「任务」标题（圆点 `Prguse[919]` + 缩进 15）+ 3 条任务条目 + 「任务交付」标题；第 2 页（`top_line=16`）= `任务描述第 8 行` / 任务 / 任务交付 / **时间限制 1h 01m 01s** / **进度 击杀 稻草人 0/3**，右侧位置条随页下移；两页在消息区 (790,90)-(1270,510) 的像素差 bbox = `(0,54,466,420)`（非空即翻页生效）。
  - **单元③**：同一窗奖励区 = `Exp. 50` + 金币图标 `100`（信用 0 隐藏）+ `Title[17]`「SELECT ITEM」+ 固定排 2 格（`Prguse[989]` 40x34 底 + 物品图标）+ 可选排 3 格（未选中无底图）；底栏 `SHARE`/`CANCEL` 两键就位；`confirm=true` 弹取消询问框 = `Prguse[360]` 456x190 @(284,289) + 「你确定要取消这个任务吗？」+ YES(`Title[206..208]` @260,157)/NO(`Title[210..212]` @360,157)。
  - 未覆盖（输入注入被锁屏阻断，留待解锁后补真点击 JPG）：滚轮/位置条拖动、分享/取消/多选一真实点击、奖励格悬停物品说明。

## 7. 已知有意偏差

- Creature：C# `CreatureRenameButton` 构造即 `Visible = false` 且再无置真处（原版死控件，改名入口点不到）；Bevy 保留可用的「改名」按钮（功能补齐见 #1281），仅坐标/精灵与 C# 对齐。
- Creature：C# `CreatureInfo`(19,161)/`CreatureInfo1`(19,176)/`CreatureInfo2`(19,191) 三行（`CanPickupItems`/`CanProduceBlackStones`/`CanProducePearlsBuyCreatureItems`）批15 已落地（规则随 `S.UpdateIntelligentCreatureList` 下发，见下条）；Bevy 原本占用这两行的「数量 + 选中宠物名/模式/饥饿度」摘要改挂 C# 三行之后的同间距第 4 行 (19,206)，操作反馈行挂 (19,333)（第二行宠物槽图标底 331 与黑石条顶 348 之间的空档；批16 槽位名字标签移到 C# `NameLabel` 位置后原 (19,243) 与之冲突）——两者都是 Bevy 扩展行（C# 无对应控件），不再占用 C# 信息位。
- Creature：`CreatureInfo` 的 `semi`/`mouse` 两段按 C# `IntelligentCreatureDialogs.cs:729-730` **逐字复刻其原版拼接怪癖**：`semi` 的「NxN」取的是 `AutoPickupRange`（不是 `SemiAutoPickupRange`）、`mouse` 段只由 `SemiAutoPickupEnabled` 决定（与 `MousePickupEnabled` 无关）、未开 `MousePickupEnabled` 时两段之间没有 `, ` 分隔符 —— 因此只开 Semi 的 C# `BabyPig`/`Kitten` 行渲染为「可以拾取物品（0x0 semi-auto0x0 mouse）。」。这是原版行为（实机截图已核对），不是移植缺陷。
- Creature：未选中宠物时按 C# `RefreshUI` 保留按钮但 `Enabled=false`；**禁用态外观与可用态相同**（C# `MirButton` 无 `DisabledIndex` 时 `Index` 回落 `base.Index`，且本对话框从不置 `GrayScale`），Bevy 已按此回退原色（此前用 `ImageNode.color` 暗化属自造视觉，批12 已纠正）。C# 「已召唤**其它种类**宠物」时 `SummonButton` 切到 `Title[593..595]` 并禁用、`Dismiss` 隐藏（`:640-667`）——批16 已按列表包尾部三字段（召唤态/召唤种类）对齐。
- Creature：C# `HelpPetButton`（`Prguse2[257..259]` @ `Size.Width-48,3`）在原版无 Click 处理（死控件），Bevy 未实现该占位按钮，待有宠物帮助页时再补。
- Creature：`刷新` 是 Bevy 扩展按钮（C# 无此控件），用中文文本渲染；此前借用 MessageBox 的 `Title[206..208]`（原版是「YES」精灵），实机截图里会显示成「YES」。
- Creature：C# `CreatureMaintainFoodBuff`（食物增益时间）构造即 `Visible = false`（源码注释「FAR made invisible as position was wierd」，`IntelligentCreatureDialogs.cs:320-328`）且全仓无处置真 —— 原版死控件，Bevy 不实现。批16 已补齐同组的 `CreatureDeadline`(@140,85，`Expire`/`ExpireNever`) 与 `CreaturePearls`(@53,348，玩家珍珠数)：到期剩余秒与珍珠数随列表包下发（服务端 `expire_at` 持久化到 DB 列 `active_expire_at`，宠物蛋 `Effect` 天数 → `Expire`，对齐 C# `PlayerObject.cs:6231`）。
- Creature：面板宠物动画（`CreatureImage` @50,110）按 `SetCreatureFrames` 帧表播 default/ex 两套、每 8 秒交替；本端独有类型（Panda/Oma/Sheep/Gorilla/Custom）C# `switch` 无 case → 沿用 `CreatureButton` 构造默认帧表 `540/6/400 + 550/5/400`（Bevy 同）。槽位图标用服务端下发的 `icon`（`Prguse2[500..514]`），本端独有类型 icon=0 → 不绘制图标（C# `GetCreatureInfo` 对这些类型返回 null，行为未定义，属有意取「不绘制」）。
- Craft：C# 用客户端本地 `ItemInfo` 库解析需求图标，Rust 客户端无本地物品库 —— 图标/名称改由 `RecipeRequirement.image/name` 随包下发（协议自洽偏离，已注释说明）。
- 背包格锁定（`MirItemCell.Locked`，批12 单元2 + 批13 单元1 + 批14）：Craft `SelectedCell`（`AutoFill` 逐格锁定、`ResetCells()` 解锁）、装备 `C.EquipItem`（`S.EquipItem` 解锁）、拆分 `C.SplitItem`（`S.SplitItem1` 解锁）、镶嵌/钓具坐骑槽 `C.EquipSlotItem`（`S.EquipSlotItem` 解锁）、交易所寄售选物 `tempCell`（换物/切页签/关窗/`S.ConsignItem` 解锁）、仓库存入/取出（`C.StoreItem`/`C.TakeBackItem` → `S.StoreItem`/`S.TakeBackItem` 解锁，含 C# 的「点击格空则用它、否则首个空格」目标选择）、交易存入/取回（`C.DepositTradeItem`/`RetrieveTradeItem` → `S.*` 解锁；取回改为「回包成功才清槽」）七类已对齐；锁按 `(InvLockReason, LockGrid, index)` 三元组存放（`LockGrid::{Inventory,Belt,HeroInventory,HeroBelt,Storage,Trade}`），故「背包格 3」与「仓储格 3」互不影响，图标按 `Color::srgba_u8(105,105,105,204)`（= `Color.DimGray` × 0.8）灰化且锁定格不响应点击。
- 腰带来源的锁在 Bevy **无对应物**（不是未实现）：C# 治疗品消耗后走 `C.MoveItem` 从背包补入腰带格（`MirItemCell.cs:556-599`）并锁来源背包格；Bevy 的腰带是 `PotionBeltState` 里的 **unique_id 虚拟槽**（`belt_restock_events` 直接换 uid，不发包、不占背包格），既没有在途请求也就无需锁定。
- Refine：C# 的待精炼武器走 NPCDialog 的 ItemCell（投放窗确认即 `C.RefineItem{UniqueID}`）；Rust 服务端语义是两步（`DepositRefineItem to=0` 存入 → `RefineItem{uid}` 发起），故 Bevy 的投放窗确认在收到存入确认后再发 `RefineItem`（对外行为等价，多一个包）。
- ItemRent：C# `KeybindOptions.Rental` 在 `KeyBindSettings.New()` 里**没有默认绑定行**（枚举成员存在但无 `KeyBind`，原版默认无键，键位面板也列不出）；Bevy 作扩展给「租赁」（界面组，默认 `;`（分号））并在键位面板可重绑，热键 `ItemRentalDialog.Toggle()` 语义与 C# `GameScene.cs:779-781` 一致。批18 前该扩展键是 `T`，因批18 按 C# `KeyBindSettings.cs:340` 补上 `KeybindOptions.Trade`（默认 `T`）而让位到分号键。
- 键位：宠物模式——C# `KeybindOptions.PetmodeBoth`/`PetmodeMoveonly`/`PetmodeAttackonly`/`PetmodeNone`/`PetmodeFocusMasterTarget`（`KeyBindSettings.cs:361-369`）默认 `Keys.None`（无键），Bevy 同样未绑定这 5 个；只有 `ChangePetmode`（C# 默认 Ctrl+A，`GameScene.cs:782-784`）在 Bevy 为 Ctrl+T（#1562：A 已用于相机平移）。
- ItemRent：`Prguse[238]` 面板位图**自带**右上角关闭图样（C# 四窗都画得到），但只有自有窗挂了可点关闭键（`Prguse2[360..362]`）——C# `GuestItemRentDialog`/`GuestItemRentingDialog` 本身没有 `closeButton`，Bevy 同样只在自有窗挂 `ItemRentalClose`，对方窗的 X 是装饰。
- ItemRent：`S.UpdateRentalItem` 除 C# 的 `HasData`+`LoanItem` 外仍带 `rental_fee`/`rental_period`（Rust 扩展，客户端只在 >0 时用作兜底刷新）；`S.CanConfirmItemRental`/`S.ConfirmItemRental` 在 C# 是空包，Rust 端口带 `can_confirm`/`success` 载荷（自洽偏离，双方均为 Rust 实现）。
- ItemRent：C# 租客侧的合计费用在服务端 `SetItemRentalFee` 里即时扣金币（`S.LoseGold`）；Rust 服务端到 `ConfirmItemRental` 成交时才扣，客户端费用标签两侧都由本地/对包数值驱动，显示一致。
- ItemRent：mock 单客户端下 `ItemRentalRequest` 固定回 `Renting=false`（物主侧），租客侧窗口与锁定帧的实机验证靠临时把 mock 回包改 `true` 截图（验证后已还原）。
- TrustMerchant：价格排序已对齐 C#（`Listings.AddRange` 累积后对全量排序，`UpdateInterface` 取 `orderedListings[Page*10+i]`）——客户端按页累积（`MarketState.loaded_pages`/`pending_page`），翻页与 C# 一致（Back 本地翻页、Next 已累积则本地否则发 `C.MarketPage`），滚轮范围 = 已累积页。残留偏差：`S.NPCMarketPage` 不带页号，客户端按「最近一次请求页（无未决请求 = 第 0 页）」归属累积位置，而 C# 靠 `NPCMarket`/`NPCMarketPage` 两种包区分。
- TrustMerchant：写邮件正文复用邮件窗的 `MirTextBox`（预填文本与 C# `ComposeMail(recipient, message)` 一致），换行/焦点细节未逐像素复刻。
- TrustMerchant：拍卖出价已对齐 C#：按 BUY 先弹 `MirAmountBox`（`BidAmount` 标题 + `ItemImage` @(15,34) 38x34 物品图标 + 默认/下限 `Price + 1`、上限 `uint.MaxValue`），OK 后再弹 `MirMessageBox`（`ConfirmBidGoldForItem`）——批12 单元3 落地 `AmountBoxState::ask_with`（批11 已补确认框）。
- TrustMerchant：C# `AuctionRow.SelectedImage`（`Prguse[545]` 296x38）构造后 `Visible=false` 且全仓无置真处（原版死控件）；选中高亮只用 `Border`（1px 外扩橙框），Bevy 一致。
- TrustMerchant：`C.AuctionRow` 到期列在 C# 用本地墙钟 `DateTime`；Bevy 用 `chrono::Local` 格式化（同一时刻的本地显示），mock 侧写「当前-1h」便于核对格式。
- TrustMerchant：底栏 `BuyButton`/`CollectSoldButton`/`MailButton`/`SellNowButton` 与 `SellItemButton` 的禁用态已按 C# `GrayScale`（`:1005-1033` 与 `RefreshCraftCells` 同套语义）走真灰度（`ui::gray`，公式见 `Data/Shaders/grayscale.ps`）。C# 寄售选物锁定背包格（`tempCell.Locked`）已对齐（`InvLockReason::Consign`；放入锁、换物/切页签/关窗/`S.ConsignItem` 回包解锁）；C# `Show()` 把背包推到 `Size.Width + 5`、`Hide()` 复位 (0,0) 也已对齐（`InventoryPlaceAt`）。
- 交易（`C.DepositTradeItem`/`C.RetrieveTradeItem`，批14 单元2）：放入时锁来源背包格 + 目标交易槽（`InvLockReason::Trade`，`LockGrid::{Inventory,Trade}`），`S.DepositTradeItem`（GameScene.cs:2804-2820）回包解锁；取回时锁交易槽并**保留槽内物品**（此前 Bevy 乐观清空），`S.RetrieveTradeItem`（:2821-2836）回包按 `success` 决定清槽并解锁；被锁交易槽按同一 `LOCKED_ITEM_COLOR` 灰化且不响应点击，关窗（`trade_reset` 语义）清该来源全部锁。
- TrustMerchant / 数量框：售价框的「三态边框」§7 原描述有误——C# `PriceTextBox.BorderColour` 虽被 `TextBox_TextChanged` 赋值（:1333/1337/1345/1355/1359），但该控件从未置 `Border = true`，`MirControl.Draw()` 的 `DrawBorder()` 因 `_border` 默认 false 早返回 → **原版售价框没有三态视觉**；批13 已删掉 Bevy 自造的底色近似（三态仍驱动 `SellItemButton.Enabled`）。真正画 1px 边框三态的是 `MirAmountBox` 的输入框（`Border = true; BorderColour = Lime`，:80-87、`TextBox_TextChanged`:172-194 → 合法 Lime、`== MaxAmount` Orange、非法 Red 且隐藏 OK 键），批13 已按其坐标 (58,43) 132x19 补框（Bevy 容器取 (57,42) 134x21 使内容区一致）。
- TrustMerchant：筛选树 `Prguse2[205/206]` 手柄的拖动用「按下时记录抓取偏移 → 按住按光标 y 反算 `Skip`」实现（C# 是 `MirControl.OnMoving`）；手柄高度固定为精灵原始 12x18（C# 也未按 `PossibleTotal` 缩放）。
- TrustMerchant：Find/筛选/页签搜索改发 C# 规范 `C.MarketSearch`（此前 Bevy `MarketSearchWire` 只写关键字，被网关 `read_body` 静默丢弃 → 搜索无效）；`MarketRefresh` 仅保留在刷新按钮（C# `RefreshButton.Click` 先清空搜索框）。
- Craft：C# `BeforeDraw` 在背包关闭时会隐藏合成窗，Bevy 未实现（挂机脚本会直开 Craft，保持现状以免回归）。
- Craft：`CraftButton` 的 `Enabled`/`GrayScale` 已按 C# `RefreshCraftCells`（NPCDialogs.cs:2686-2723）对齐：未选配方或任一工具/材料槽未就位 → 按钮灰度且点击不触发（构造默认即 `GrayScale=true, Enabled=false`）；超出 3 工具格 / 6 材料格的额外需求按 C# `continue` 忽略。
- Mail：C# `BlockListButton`/`BugReportButton`（MailDialogs.cs:257-283，`Prguse[520]` @(183,414) / `Prguse[523]` @(210,414) 28x25）构造即 `GrayScale = true, Enabled = false` 且无 Click 处理（`AllowDisabledMouseOver` 默认 false，连 Hint 都不弹）；批13 单元3 已按原坐标补这两个灰度占位图（不含交互）。
- 世界悬停按「光标是否在控件上」门控（**批22 单元③ 已对齐**）：C# 的头顶名字画在 `MapControl` 图层、光标位于控件上时仍绘制但被对话框整块盖住（等价于「控件 Hint 优先」）；本端提示面板迁到 `GlobalZIndex(90)` 置顶层后该隐式遮盖消失，故用 `cursor_over_dialog_rect`（可见 `DialogRoot` 根面板的显式 `Px` 矩形，几何判定、可被 `cursor` 探针驱动）在 `actor_hover_tooltip_system` 里清自身 source=12 并 return。HUD 常驻元素（非 `DialogRoot`）不在门控范围，保持原样。
- 悬停提示（`MirControl.Hint`）覆盖长尾：C# 侧 `Hint = ` 共 **203 处**（`rg -c "Hint\s*=" Client/MirScenes`，其中 MainDialogs 149）。**已对齐**：HUD 主按钮 8 个（含键位）、小地图条 3 个（邮件/大地图/小地图）、菜单窗 13 个（含键位）、物品/商品/角色格（`item_tooltip_lines`）、头顶名字（玩家/怪物/NPC）、技能栏格（`SkillMpCooldownKey`）、大地图（搜索NPC/队友名）、玩家右键菜单 5 项、Mail（发送/读取/删除）、Group（允许拒绝队伍请求/添加/移除/成员名）、Friend（添加/移除/备注/邮件/悄悄话）、Ranking 6 页签、Relationship 5 项、Mentor 3 项、GuildTerritory 4 项（退出/翻页/购买）、Hero 行为 4 项 + 英雄/药水腰带 2 项、耐久面板钮、设置窗 2 条音量滑条、**技能页 7 行魔法格**（103 条技能描述，`dialogs/skill_desc.rs`，按 C# `Spell` 分派）。
  机制：sprite-UI `UiButton` 走 `TooltipHint`（source=1）；bevy UI `Button`/文本按钮走 `UiHint`（source=8，命中用「沿 `ChildOf` 链累加各级 `Node.left/top` 的绝对矩形」，`Auto` 尺寸回退 `ComputedNode`）；提示面板自身是 `GlobalZIndex(90)` 的 bevy_ui 根节点（批19 由 sprite 层迁移——旧实现会被所属对话框整块盖住）；带键位的文案按 C# `KeyBindSettings.GetKey` 拼接（`keyboard_layout::{binding_key_text, hint_with_key}`）。
  **批22 后已全部接上**（原「控件未实现」三项 + Buff 图标，见上）：`ChatControlBar`（批20）、`LoverButton`（批21）、GuildTerritory 邮件会长钮（批21）、Mail 回复（批21）、Relationship `AllowButton` 动态 Hint（批21）、`HeroManageAvatar`（批22 单元①）、Gameshop 付款复选框（批22 单元②）、`BuffDialog` 增益图标 Hint（批22 单元④）。**批23 单元① 已对齐 Exp/Drop**：C# `HumanObject.AddBuff` 对本人无条件 `Enqueue(S.AddBuff)`（`Visible` 只控制广播给他人），故 C# 客户端 Buff 窗会显示 `BuffImage(Exp)=260`/`(Drop)=162`；本端由 `SetExpMultiplier`/`SetDropMultiplier` 携带显示载荷（生效发 AddBuff tag 29/30 + `Luck` 百分比、到期发 RemoveBuff）补齐，安全区顺延不发包（与 C# 客户端本地倒计时一致）。**剩余长尾**：C# 的 `PoisonBuffDialog`（毒窗口，`Prguse2[40..]`）是**原版未完成功能**——源码自带 `//UNFINISHED`（`BuffDialog.cs:535`）且全仓零调用点（`S.Poisoned` 只用位掩码驱动致盲/清魔法），本端**不实现**；毒状态在本端走 `LocalPoisonChanged` 提示。
  **Buff 显示近似（批22 单元④）**：Rust 端 `BuffType` 比 C# 粗（`AttackBoost` 同时覆盖 C# `Rage` 与 Buff 药水 `Impact`；`Invisibility` 覆盖 `Hiding`/`MoonLight`/`DarkBody`），`dialogs/buff.rs::buff_display` 按**代表来源**取 C# 图标/文案；毒类（`Poison/Slow/Frozen/Stun`）取 `PoisonType` 的图标与名称；展开态面板宽度按 art 宽度布局（C# `Size.Width` 展开时被改写成 `count*23`，比 art 窄 ~21px）；渐隐动画（C# `Opacity 0→1`，0.2/55ms）简化为直接显隐；Buff 窗为状态驱动 `AlwaysVisible`；**批23 单元② 已按 C# `Movable = false` 不可拖动**（`NotDraggable` 标记 + `dialog_drag_system` 的 `Without<NotDraggable>`，既不拖动也不进 kind 包围盒）。
- QuestDetail（批24）：**信用图标缺失**——C# 奖励区画 `Prguse[2447]`（信用/点数图标），但本端 `Data/Prguse.Lib` 只有 2447 张（下标 0..2446）→ 该图越界取不到，本端按缺失跳过（数值与偏移链仍按 C# 计算）；待资源包更新后自动出现。
- QuestDetail（批24）：`_pauseButton`（`Title[270/271/272]` @(120,436)）在 C# 里**建了控件但 `Visible = false` 且无 Click**（`QuestDialogs.cs:577-584`，死控件）→ 本端保留同坐标实体并恒隐藏，不接线；同理 C# 注释掉的 `helpButton`（`Prguse2[257..259]`）不存在，本端不实现。
- QuestDetail（批24）：消息区标题行 C# 用 `Font(Settings.FontName, 10F, FontStyle.Bold)`（GDI 合成粗体），Bevy `TextFont.weight` 只对**可变字重**字体生效（宋体无可变轴）→ 本端以 **+1px 字号**近似（正文 12px / 标题 13px，`QuestMessage` 的 +5 行占位与 15 缩进仍逐字照抄）；行内 `{文本/颜色}` 只做**去标记取正文**（C# `:1321-1323`），`NewColour` 彩色叠加（`:1336-1353`）、`NewLink` 链接悬停（`:1355-1382`）与 `NPCDialog` 的怪物/NPC/物品链接名替换（`:1281-1319`）**未移植**——含这类标记的行按原文显示（代码注释已列清单，待后续单元）。
- QuestDetail（批24）：奖励区物品格按 C# `QuestCell` 画法（固定排 `Prguse[989]` @(x,y−1)、可选排选中 `Prguse[979]` @(x,y−5)、物品图居中 `(40−宽)/2,(32−高)/2`），但**格内物品说明悬停**（C# `QuestCell.OnMouseEnter` → `CreateItemLabel(ShowItem)`）未移植；`QuestRewards` 在 C# 里是 `static` 共享格数组（QuestListDialog 与 QuestDetailDialog 共用），本端按窗口各自渲染（视觉等价，选择态各窗独立）。

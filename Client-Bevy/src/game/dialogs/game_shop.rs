// ============================================================================
// 商城对话框（M35）
// 参考：C# GameshopDialog（Title[411] 背景）+ ServerRust npc.rs GameshopBuy
// 网络：
//   C: GameshopBuy{ item_id=0 → 请求目录；>0 → 购买 }（wire: [item_id u32][quantity u32]）
//   S: GameShopInfo(250) 商品列表 / GameShopStock(251) 库存变化
// 购买成功物品通过邮件送达（服务端 send_mail_received_packet）
// ============================================================================

use std::collections::HashMap;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::game::dialogs::text_input::{
    TextInputDisplay, TextInputField, TextInputRect, TextInputState,
};
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::ArrayLibType;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{shared_cjk_font, UiCjkFont, UiFont};
use crate::ui::theme::{
    load_lib_image, spawn_close_button, spawn_container, spawn_icon_button, spawn_image,
    spawn_label, spawn_label_center, spawn_panel, spawn_scroll_bar_ui, UiScrollList,
};

/// #2892 批B：面板精灵与 C# 原生尺寸（C# `GameShopDialog.Index = 749; Location = Center`）
pub const PANEL: (LibraryName, usize) = (LibraryName::Title, 749);
pub const PANEL_SIZE: (f32, f32) = (696.0, 476.0);
/// 关闭键 `Prguse2[360..362]` @(671,4)（`GameShopDialog.cs:67-76`，无 `Size` → 原生 24x21）
pub const CLOSE_POS: (f32, f32) = (671.0, 4.0);

/// P3-3：商品名解析与物品名表写入**共用 `crate::game::item_names` 的同一对纯函数**
/// （#782 验收判据 = 「仓库与商城走同一降级链」）——这里只保留商城侧的旧名字，
/// 免得调用点与既有测试全改一遍：
///
/// - `resolve_shop_name` = `item_names::resolve_item_name`
///   （线包自带名字 → 本地表 → 需要发一次 `RequestItemInfo` → 兜底 `#id`）
/// - `remember_item_name` = `item_names::remember_item_name`
pub use crate::game::item_names::{remember_item_name, resolve_item_name as resolve_shop_name};

/// 商城商品（GameShopInfo 写入）
#[derive(Debug, Clone, Default)]
pub struct ShopItem {
    pub item_index: i32,
    /// C# `Item.Info.Image`：格子图标用 `Libraries.Items[image]`（`MirGameShopCell.DrawControl`）
    pub image: i32,
    pub name: String,
    pub gold_price: u32,
    pub credit_price: u32,
    /// 分类（C# `GameShopItem.Category`）——即原版 `TypeFilter` 的取值
    pub category: String,
    /// 职业（C# `GameShopItem.Class`：`"All"`/`"Warrior"`/`"Wizard"`/`"Taoist"`/`"Assassin"`/`"Archer"`）
    /// ——即原版 `ClassFilter` 的取值（原版按**字符串**比，`"All"` 商品对所有职业可见）
    pub class: String,
    pub stock: i32,
    /// C# `Item.Count`（购买确认文案 `{3}` 用）
    pub count: i32,
    /// 特价（C# `SectionFilter == "DealItems"` 的判据）
    pub deal: bool,
    /// 置顶（C# `SectionFilter == "TopItems"` 的判据）
    pub top_item: bool,
    /// 上架时间（C# `SectionFilter == "NewItems"`：`Date > Now - 7 天`）
    pub date: i64,
    /// C# `GameShopItem.CanBuyGold/CanBuyCredit`（`ItemData.cs:793-794`）
    pub can_buy_gold: bool,
    pub can_buy_credit: bool,
}

/// 待确认的购买（C# `MirMessageBox` 弹起后 Yes 才发包）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShopPending {
    /// C# `C.GameshopBuy.PType`：0=积分 1=金币
    pub p_type: i32,
    pub quantity: u32,
    /// C# `C.GameshopBuy.GIndex`——格内购买钮按下时锁定，确认直接发包
    /// （不再回查「选中行」，C# 无选中语义：每格自带购买钮）
    pub g_index: i32,
}

/// 商城状态
#[derive(Resource)]
pub struct GameShopState {
    pub items: Vec<ShopItem>,
    pub gold: u32,
    pub item_names: HashMap<i32, String>,
    /// 按需请求回来的 `ItemInfo` 关键字段（试穿预览用；`MirGameShopCell.cs:278/505-547/566`）
    pub item_infos: HashMap<i32, ShopItemInfo>,
    /// P3-3：已发过 `RequestItemInfo` 的商品索引（按索引去重，避免每帧刷包）
    pub requested_item_info: std::collections::HashSet<i32>,
    /// 搜索关键词（C# GameshopDialog Search，本地按名称过滤）
    pub search: String,
    /// 分类列表（第 0 项 = 全部，C# Filters[22]；服务端 category 去重保序）
    pub categories: Vec<String>,
    /// 当前选中分类（空 = 全部）
    pub category: String,
    /// 当前职业筛选（C# `ClassFilter`；`"Show All"` = 不限）
    pub class_filter: String,
    /// 当前区段筛选（C# `SectionFilter`：`"Show All"`/`"TopItems"`/`"DealItems"`/`"NewItems"`）
    pub section_filter: String,
    /// 商品翻页（每页 8 格，C# `Page`/`maxPage`；过滤/搜索/换分类时归 0）
    pub page: usize,
    /// 各格选购数量（C# 每格自带 `Quantity`，`UpdateShop` 重建格子时归 1）
    pub qty: [u8; 8],
    /// 付款方式（C# `GameshopDialog.PaymentTypeGold/Credit.Checked`；0=积分 1=金币，
    /// 初值 1——C# 构造即 `PaymentTypeGold.Checked = true`，`GameshopDialog.cs:195`）
    pub pay_type: i32,
    /// 待确认购买（C# `MirMessageBox`；None = 未弹）
    pub pending: Option<ShopPending>,
    /// 确认框文案（C# `ConfirmPurchaseItemGold` / `ConfirmBuyItemCredits`）
    pub confirm_text: String,
    /// 试穿预览状态（None = 未打开；C# `GameShopDialog.Viewer`，`:124-132`）
    pub viewer: Option<ShopViewerState>,
}

/// 试穿预览需要的 `ItemInfo` 关键字段
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ShopItemInfo {
    /// `ItemInfo.item_type`（`ItemType` 数值）
    pub item_type: u8,
    /// `ItemInfo.shape`
    pub shape: i16,
    /// `ItemInfo.required_gender` 的 bits（`RequiredGender` 位标志）
    pub required_gender: u8,
}

/// 试穿预览状态（C# `GameShopViewer`：`ViewerItem` + `Direction`，`:317-319`）
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShopViewerState {
    /// 正在试穿的物品索引（C# `ViewerItem.ItemIndex`）
    pub item_index: i32,
    /// 朝向 1..8（C# `Direction`，初值 6）
    pub direction: u8,
    /// 面板相对商城对话框的位置（C# `:129` 按被点格子在左半/右半选）
    pub pos: (f32, f32),
}

impl Default for GameShopState {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            gold: 0,
            item_names: HashMap::new(),
            item_infos: HashMap::new(),
            requested_item_info: std::collections::HashSet::new(),
            search: String::new(),
            categories: Vec::new(),
            category: String::new(),
            // C# `Show()` 里 `ClassFilter = User.Class.ToString()`、`SectionFilter = "Show All"`
            // （`GameshopDialog.cs:508-509`）——开窗那帧会按玩家职业重设，这里只是初值
            class_filter: "Show All".to_string(),
            section_filter: "Show All".to_string(),
            page: 0,
            qty: [1; 8],
            // C# `GameshopDialog` 构造即 `PaymentTypeGold.Checked = true`（`:195`
            // 与 `PaymentTypeCredit` 默认未勾选）→ 默认金币付款
            pay_type: 1,
            pending: None,
            confirm_text: String::new(),
            viewer: None,
        }
    }
}

/// 商品格面板内坐标（C# `UpdateShop`：`i < 4 ? (152 + i*132, 115) : (152 + (i-4)*132, 275)`）
pub(crate) fn cell_pos(i: usize) -> (f32, f32) {
    if i < 4 {
        (152.0 + i as f32 * 132.0, 115.0)
    } else {
        (152.0 + (i - 4) as f32 * 132.0, 275.0)
    }
}

/// 总页数（C# `maxPage = Ceiling(filteredShop.Count / 8)`，< 1 归 1）
pub(crate) fn max_page(filtered_len: usize) -> usize {
    (filtered_len as f32 / 8.0).ceil().max(1.0) as usize
}

/// 数量加（C# `quantityUp.Click`：Shift +10；上限 99，有库存（>0）再压到 stock）
pub(crate) fn qty_up(qty: u8, stock: i32, shift: bool) -> u8 {
    let mut q = qty.saturating_add(if shift { 10 } else { 1 });
    if q >= 99 {
        q = 99;
    }
    if stock != 0 && q as i32 > stock {
        q = stock.clamp(1, 99) as u8;
    }
    q
}

/// 数量减（C# `quantityDown.Click`：Shift -10；下限 1）
pub(crate) fn qty_down(qty: u8, shift: bool) -> u8 {
    let q = qty as i32 - if shift { 10 } else { 1 };
    if q <= 1 || q > 99 {
        1
    } else {
        q as u8
    }
}

#[derive(Component)]
pub struct GameShopWidget;

#[derive(Component)]
pub struct GameShopClose;

/// 商品格根（C# `MirGameShopCell` 125x146 `Title[750]`，4列x2行）——空槽整格隐藏
/// （C# `UpdateShop` 只重建有商品的格子，其余不存在）
#[derive(Component)]
pub struct GameShopCell(pub usize);

/// 格内商品名（C# `nameLabel` 125x15 居中 @(0,13)）
#[derive(Component)]
pub struct GameShopCellName(pub usize);

/// 格内金币价（C# `goldLabel` 95x20 右对齐 @(2,102)，仅 `CanBuyGold` 显示）
#[derive(Component)]
pub struct GameShopCellGold(pub usize);

/// 格内积分价（C# `gpLabel` 95x20 右对齐 @(2,81)，仅 `CanBuyCredit` 显示）
#[derive(Component)]
pub struct GameShopCellCredit(pub usize);

/// 格内库存值（C# `stockLabel` 20x20 居中 @(93,37)；0=∞、>=99=99+）
#[derive(Component)]
pub struct GameShopCellStock(pub usize);

/// 格内每件数量（C# `countLabel` 30x20 右对齐 @(16,60)，`Item.Count`）
#[derive(Component)]
pub struct GameShopCellCount(pub usize);

/// 格内选购数量值（C# `quantity` 20x13 居中 @(74,56)）
#[derive(Component)]
pub struct GameShopCellQty(pub usize);

/// 格内数量减（C# `quantityDown` `Prguse2[240..242]` @(55,56)；Shift=±10）
#[derive(Component)]
pub struct GameShopCellQtyDown(pub usize);

/// 格内数量加（C# `quantityUp` `Prguse2[243..245]` @(97,56)；Shift=±10）
#[derive(Component)]
pub struct GameShopCellQtyUp(pub usize);

/// 格内购买钮（C# `BuyItem` `Title[778..780]` @(42,122) → `BuyProduct`）
#[derive(Component)]
pub struct GameShopCellBuy(pub usize);

/// 格内试穿钮（C# `PreviewItem` `Title[781..783]` @(8,122)；仅 Mount/Weapon/Armour/Transform 可见）
#[derive(Component)]
pub struct GameShopCellPreview(pub usize);

/// 试穿预览面板（C# `GameShopViewer`）
#[derive(Component)]
pub struct GameShopViewerPanel;

/// 试穿预览的四个图层（C# `PreviewImage` / `WeaponImage` / `WeaponImage2` / `MountImage`）
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum GameShopViewerLayer {
    /// 身体/护甲层（`PreviewImage`）
    Body,
    /// 主手武器层（`WeaponImage`）
    Weapon,
    /// 副手武器层（刺客专属，`WeaponImage2`）
    Weapon2,
    /// 坐骑层（`MountImage`）
    Mount,
}

/// 预览面板关闭钮（C# `CloseButton` `Prguse[361..363]` @(230,8)）
#[derive(Component)]
pub struct GameShopViewerClose;

/// 预览转身钮（C# `LeftDirection`/`RightDirection` @(81,282)/(160,282)）
#[derive(Component)]
pub struct GameShopViewerTurn {
    pub right: bool,
}

/// 页码标签（C# `PageNumberLabel` 83x17 居中 @(597,446)，"N / M"）
#[derive(Component)]
pub struct GameShopPageLabel;

/// 上一页（C# `PreviousButton` `Prguse2[240..242]` @(600,448)）
#[derive(Component)]
pub struct GameShopPagePrev;

/// 下一页（C# `NextButton` `Prguse2[243..245]` @(660,448)）
#[derive(Component)]
pub struct GameShopPageNext;

#[derive(Component)]
pub struct GameShopCat(pub usize);

#[derive(Component)]
pub struct GameShopCatUp;

#[derive(Component)]
pub struct GameShopCatDown;

/// 商城三段筛选里的按钮：职业（C# `ClassFilter`）与区段（C# `SectionFilter`）。
///
/// C# `GameshopDialog` 的按钮表（`GameshopDialog.cs:212-377`）：
/// - 职业：`Title[751..768]` 六档，`ALL@(539,37)`、其余 `@(568+23i,38)`；
///   选中态 = **hover 帧**（`ResetClass` 把 `ALL.Index` 设成 752）⇔ 本端用 `ImageButton.pressed`；
/// - 区段：`Title` 四档 @ `(138|209|280|351, 68)`，常态 770/776/772/774、选中 771/777/773/775；
///   `New` 在原版初始 `Visible = false` 且从未置 true（死按钮）——本端同样隐藏。
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum ShopFilterBtn {
    Class(usize),
    Section(usize),
}

/// 职业筛选按钮：`(C# ClassFilter 值, 常态帧, 选中/hover 帧, x)`，y 统一 37/38（C# 常量）
pub const CLASS_FILTERS: [(&str, usize, usize, f32); 6] = [
    ("Show All", 751, 752, 539.0),
    ("Warrior", 754, 755, 568.0),
    ("Assassin", 757, 758, 591.0),
    ("Taoist", 760, 761, 614.0),
    ("Wizard", 763, 764, 637.0),
    ("Archer", 766, 767, 660.0),
];
/// 职业按钮 y：`ALL` 用 37，其余 38（照抄 C# 常量，不做"对齐修正"）
pub const CLASS_BTN_Y: [f32; 6] = [37.0, 38.0, 38.0, 38.0, 38.0, 38.0];

/// 区段筛选按钮：`(C# SectionFilter 值, 常态帧, 选中帧, x)`，y 统一 68
pub const SECTION_FILTERS: [(&str, usize, usize, f32); 4] = [
    ("Show All", 770, 771, 138.0),
    ("TopItems", 776, 777, 209.0),
    ("DealItems", 772, 773, 280.0),
    // C# `New.Visible = false`（`:263`）且从未置 true → 本端同样隐藏（保持原版"死按钮"）
    ("NewItems", 774, 775, 351.0),
];
pub const SECTION_BTN_Y: f32 = 68.0;

// ---------------------------------------------------------------------------
// 试穿预览（C# `GameShopViewer`，`Client/MirControls/MirGameShopCell.cs:312-595`）
// ---------------------------------------------------------------------------

/// 预览面板：`Title[785]`，作为**商城对话框的子控件**（C# `Parent = GameShopDialog`，`:127`）。
/// 位置按被点格子在左半还是右半决定：`X < 350 ? (416,115) : (151,115)`（`:129`）。
pub const VIEWER_PANEL: (LibraryName, usize) = (LibraryName::Title, 785);
pub const VIEWER_RIGHT_SIDE: (f32, f32) = (416.0, 115.0);
pub const VIEWER_LEFT_SIDE: (f32, f32) = (151.0, 115.0);
/// 四个图层都在 (105,160)（`:353/366/379/393`）
pub const VIEWER_LAYER_POS: (f32, f32) = (105.0, 160.0);
/// 关闭 `Prguse[361..363]` @(230,8)（`:335-348`）
pub const VIEWER_CLOSE_POS: (f32, f32) = (230.0, 8.0);
/// 左转 `Prguse2[240..242]` @(81,282)、右转 `Prguse2[243..245]` @(160,282)（`:404-438`）
pub const VIEWER_PREV_POS: (f32, f32) = (81.0, 282.0);
pub const VIEWER_NEXT_POS: (f32, f32) = (160.0, 282.0);
/// `AnimationDelay = 150`（`:355/368/381/395`）
pub const VIEWER_ANIM_MS: f32 = 150.0;
/// `Direction` 初值 6、取值 1..8（`:318/416-417/434-435`）
pub const VIEWER_DIRECTION0: u8 = 6;

/// 预览面板放哪一侧：C# `Location = this.Location.X < 350 ? (416,115) : (151,115)`（`:129`）
pub fn viewer_side(cell_x: f32) -> (f32, f32) {
    if cell_x < 350.0 {
        VIEWER_RIGHT_SIDE
    } else {
        VIEWER_LEFT_SIDE
    }
}

/// 转身：`Direction` 在 1..8 环绕（`:414-438`）
pub fn viewer_turn(direction: u8, right: bool) -> u8 {
    let d = direction.clamp(1, 8);
    if right {
        if d >= 8 {
            1
        } else {
            d + 1
        }
    } else if d <= 1 {
        8
    } else {
        d - 1
    }
}

/// 哪些类型可试穿（C# `MirGameShopCell.cs:278`：Mount / Weapon / Armour / Transform）
pub fn viewer_previewable(item_type: u8) -> bool {
    matches!(
        item_type,
        ITEM_TYPE_WEAPON | ITEM_TYPE_ARMOUR | ITEM_TYPE_MOUNT | ITEM_TYPE_TRANSFORM
    )
}

/// 可试穿时购买钮右移让位（C# `:281` `BuyItem.Location = new Point(75, 122)`；常态 `(42,122)`）
pub fn viewer_buy_x(previewable: bool) -> f32 {
    if previewable {
        75.0
    } else {
        42.0
    }
}

/// 身体层（`PreviewImage`）帧号：
/// 武器/护甲/变形 = 男 `32 + 6*(Dir-1)` / 女 `840 + 6*(Dir-1)`（`:492/567/583`）；
/// 坐骑 = 男 `448 + 8*(Dir-1)` / 女 `1256 + 8*(Dir-1)`（`:466/468`）。
pub fn viewer_body_index(female: bool, direction: u8, mounted: bool) -> usize {
    let d = (direction.clamp(1, 8) - 1) as usize;
    match (mounted, female) {
        (true, false) => 448 + 8 * d,
        (true, true) => 1256 + 8 * d,
        (false, false) => 32 + 6 * d,
        (false, true) => 840 + 6 * d,
    }
}

/// 武器层帧号：`32 + 6*(Dir-1)`（`:510/550`）
pub fn viewer_weapon_index(direction: u8) -> usize {
    32 + 6 * (direction.clamp(1, 8) - 1) as usize
}

/// 坐骑层帧号：`32 + 8*(Dir-1)`（`:474`）
pub fn viewer_mount_index(direction: u8) -> usize {
    32 + 8 * (direction.clamp(1, 8) - 1) as usize
}

/// 武器层用哪个库 + 索引（C# `DrawWeapon`，`:505-547`）：
/// - `shape 100..199` → `AWeapon/{shape-100} R` + `AWeapon/{shape-100} L`（左右两层）
/// - `shape >= 200`   → `ARWeapon/{shape-200}`（单层）
/// - `shape < 100`    → `CWeapon/{shape}`（单层）
pub fn viewer_weapon_libs(shape: i16) -> (ArrayLibType, usize, Option<(ArrayLibType, usize)>) {
    if (100..=199).contains(&shape) {
        let i = (shape - 100) as usize;
        (
            ArrayLibType::AWeaponsR,
            i,
            Some((ArrayLibType::AWeaponsL, i)),
        )
    } else if shape >= 200 {
        (ArrayLibType::ARWeapons, (shape - 200) as usize, None)
    } else {
        (ArrayLibType::CWeapons, shape.max(0) as usize, None)
    }
}

/// 图层前后顺序（数值大者在前）——照抄 C# `BringToFront()` 调用序（**最后调用的在最前**，
/// `MirGameShopCell.cs:499-545`）：返回 `(weapon_z, weapon2_z, body_z)`；坐骑层固定 0（最底）。
pub fn viewer_layer_z(direction: u8, shape: i16, has_weapon2: bool) -> (i32, i32, i32) {
    let d = direction.clamp(1, 8);
    let weapon_front = (2..=4).contains(&d) || (shape >= 200 && (6..=8).contains(&d));
    if !has_weapon2 {
        return if weapon_front { (4, 0, 3) } else { (3, 0, 4) };
    }
    // 刺客双武器层（`:512-528`）
    if (2..=3).contains(&d) {
        (4, 2, 3) // weapon → body → weapon2（调用序），故 weapon 最前
    } else if (7..=8).contains(&d) {
        (2, 4, 3) // weapon → body → weapon2 调用序（`:524-526`）⇒ weapon2 > body > weapon
    } else {
        (3, 2, 4) // body 最前
    }
}

/// C# `ItemType` 数值（`Shared/Enums.cs` / `SharedRust::enums::ItemType`）
pub const ITEM_TYPE_WEAPON: u8 = 4;
pub const ITEM_TYPE_ARMOUR: u8 = 5;
pub const ITEM_TYPE_MOUNT: u8 = 22;
pub const ITEM_TYPE_TRANSFORM: u8 = 40;

/// 装备槽位：护甲（C# `EquipmentSlot.Armour`；本端 `Loadout.slots` 同序：武器 0、护甲 1、…、坐骑 10）
const EQUIP_SLOT_ARMOUR: usize = 1;
/// `RequiredGender::FEMALE` 的 bits（护甲试穿按**物品**的必需性别选帧，`MirGameShopCell.cs:566`）
const GENDER_FEMALE: u8 = 0x02;

/// 付款方式复选框：金币（C# `PaymentTypeGold`，`Prguse[2086/2087]` @(250,449)）
#[derive(Component)]
pub struct GameShopPayGold;

/// 付款方式复选框：积分（C# `PaymentTypeCredit` @(340,449)）
#[derive(Component)]
pub struct GameShopPayCredit;

/// 底部金币余额标签（C# `totalGold` @(123,449) 100x20 右对齐）
#[derive(Component)]
pub struct GameShopGoldLabel;

/// 底部积分余额标签（C# `totalCredits` @(5,449) 100x20 右对齐）
#[derive(Component)]
pub struct GameShopCreditLabel;

/// 购买确认框（C# `MirMessageBox`：`Prguse[360]` 456x190 居中 @(284,289)）
#[derive(Component)]
pub struct GameShopConfirm;

/// 商品格物品图标（C# `MirGameShopCell.DrawControl`：`Libraries.Items[Item.Info.Image]`）
#[derive(Component)]
pub struct GameShopCellIcon(pub usize);

#[derive(Component)]
pub struct GameShopConfirmText;

#[derive(Component)]
pub struct GameShopConfirmYes;

#[derive(Component)]
pub struct GameShopConfirmNo;

/// 付款复选框两帧（C# `UnTickedIndex = 2086` / `TickedIndex = 2087`，`Libraries.Prguse`）
#[derive(Resource, Default)]
pub struct GameShopPayFrames {
    pub unchecked: Option<Handle<Image>>,
    pub checked: Option<Handle<Image>>,
}

/// `NewItems` 段的时间窗（C# `Date > CMain.Now.AddDays(-7)`，`GameshopDialog.cs:670/723`）
pub const NEW_ITEM_WINDOW_SECS: i64 = 7 * 24 * 3600;

/// 当前 Unix 秒（只用于 `NewItems` 的 7 天窗；独立成函数便于门禁传固定时刻）
pub(crate) fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// `MirClass` → C# `GameShopItem.Class` 字符串（原版 `GameScene.User.Class.ToString()`，
/// 与库表 `game_shop_items.class_name` 同一套写法：`Warrior`/`Wizard`/`Taoist`/`Assassin`/`Archer`）
pub(crate) fn class_filter_name(class: u8) -> &'static str {
    match class {
        0 => "Warrior",
        1 => "Wizard",
        2 => "Taoist",
        3 => "Assassin",
        4 => "Archer",
        _ => "Show All",
    }
}

/// C# `GameshopDialog.UpdateShop` 的筛选谓词（`GameshopDialog.cs:718-726`，逐条对齐）：
/// `Class == ClassFilter || Class == "All" || ClassFilter == "Show All"`
/// × `Category == TypeFilter || TypeFilter == "Show All"`
/// ×（`Show All` / `TopItems && TopItem` / `DealItems && Deal` / `NewItems && Date > Now-7d`）。
/// 搜索是**先**用 `FriendlyName.ToLower().Contains(kw)` 缩表（`:704-707`）。
/// 本端 `category` 用空串表示"全部"（原版字面量是 `"Show All"`）。
pub(crate) fn shop_item_matches(
    it: &ShopItem,
    kw: &str,
    class_filter: &str,
    category: &str,
    section_filter: &str,
    now_unix: i64,
) -> bool {
    if !kw.is_empty() && !it.name.to_lowercase().contains(kw) {
        return false;
    }
    if class_filter != "Show All" && it.class != class_filter && it.class != "All" {
        return false;
    }
    if !category.is_empty() && it.category != category {
        return false;
    }
    match section_filter {
        "TopItems" => it.top_item,
        "DealItems" => it.deal,
        "NewItems" => it.date > now_unix - NEW_ITEM_WINDOW_SECS,
        _ => true,
    }
}

/// C# `UpdateShop` 的过滤 + **名称升序排序**（`filteredShop.OrderBy(e => e.Info.FriendlyName)`，
/// `GameshopDialog.cs:740`——排序在分页前，所以页内容也按名字排），返回 items 下标。
///
/// 排序口径差异（如实记录）：C# 的 `OrderBy` 用 `Comparer<string>.Default` ⇒
/// **文化敏感**比较（`String.CompareTo`）；这里用 Rust 的码点序（`str::cmp`）。
/// ASCII 名字两者一致；中文名（如"法师杖" vs "通用药"）可能排出不同次序——
/// 本端无 ICU 之外的文化排序表，按码点稳定排序，不假装与 .NET 逐字一致。
pub(crate) fn filter_shop_items(
    items: &[ShopItem],
    search: &str,
    class_filter: &str,
    category: &str,
    section_filter: &str,
    now_unix: i64,
) -> Vec<usize> {
    let kw = search.trim().to_lowercase();
    let mut v: Vec<usize> = items
        .iter()
        .enumerate()
        .filter(|(_, it)| {
            shop_item_matches(it, &kw, class_filter, category, section_filter, now_unix)
        })
        .map(|(i, _)| i)
        .collect();
    // 稳定排序，与 C# `OrderBy` 一致（同名保持目录序）
    v.sort_by(|a, b| items[*a].name.cmp(&items[*b].name));
    v
}

/// C# `GetCategories()`（`GameshopDialog.cs:648-680`）：切职业 / 切区段 / 改搜索后
/// `TypeFilter = "Show All"`、`Page/StartIndex = 0`，并按当前搜索 × 职业 × 区段重建分类表
/// （第 0 项 = 全部；列表里只留"这个筛选下真实存在的分类"）。
pub(crate) fn rebuild_categories(shop: &mut GameShopState, now_unix: i64) {
    shop.category.clear();
    shop.page = 0;
    let kw = shop.search.trim().to_lowercase();
    let mut cats: Vec<String> = vec![String::new()];
    for it in &shop.items {
        if !shop_item_matches(
            it,
            &kw,
            &shop.class_filter,
            "",
            &shop.section_filter,
            now_unix,
        ) {
            continue;
        }
        if !it.category.is_empty() && !cats.iter().any(|c| c == &it.category) {
            cats.push(it.category.clone());
        }
    }
    shop.categories = cats;
}

/// 取精灵原生尺寸（C# 这些按钮都没设 `Size` → 用 art 尺寸）
fn lib_img_size(images: &Assets<Image>, h: &Handle<Image>) -> (f32, f32) {
    images
        .get(h)
        .map(|img| {
            let s = img.size_f32();
            (s.x, s.y)
        })
        .unwrap_or((28.0, 20.0))
}

/// 第 i 格当前展示的商品（C# `UpdateShop`：`filteredShop[i + Page*8]`；空格 None）
/// 试穿预览帧缓存：`(库, 槽位, 帧号) → 句柄/艺术偏移/尺寸`。
///
/// C# 每帧直接从 `.Lib` 取图；Bevy 侧若每帧 `images.add` 会无界增长，故按三元组缓存
/// （图层 4 个 × 帧 6/8 ⇒ 上界很小）。
#[derive(Resource, Default)]
pub struct ShopViewerFrameCache(
    pub HashMap<(ArrayLibType, usize, usize), (Handle<Image>, (i16, i16), (f32, f32))>,
);

/// 取某个数组库的某一帧（带艺术偏移与尺寸；`UseOffSet` 规则与宠物/坐骑立绘同源）
fn viewer_frame(
    libs: &mut GameLibraries,
    images: &mut Assets<Image>,
    cache: &mut ShopViewerFrameCache,
    lib: ArrayLibType,
    slot: usize,
    frame: usize,
) -> Option<(Handle<Image>, (i16, i16), (f32, f32))> {
    if let Some(v) = cache.0.get(&(lib, slot, frame)) {
        return Some(v.clone());
    }
    let info = libs.0.get_array_image(lib, slot, frame)?;
    let rgba = info.rgba.clone()?;
    let (w, h) = (info.width.max(0) as u32, info.height.max(0) as u32);
    let handle = images.add(crate::map_renderer::make_image(rgba, w, h));
    let v = (handle, (info.offset_x, info.offset_y), (w as f32, h as f32));
    cache.0.insert((lib, slot, frame), v.clone());
    Some(v)
}

/// 某个图层这一帧要画什么：`(库, 槽位, 帧基址 + 动画帧, ZIndex)`
type ViewerLayerSpec = Option<(ArrayLibType, usize, usize, i32)>;

/// 试穿预览（C# `GameShopViewer`：`UpdateViewer` / `DrawWeapon` / `DrawArmour` / `DrawMount` /
/// `DrawTransform`，`Client/MirControls/MirGameShopCell.cs:442-595`）。
///
/// 身体层用**玩家自己**的护甲与性别（`GameScene.User.Equipment[Armour]` / `User.Gender`），
/// 护甲试穿则按**物品的** `RequiredGender` 选帧（`:566`）；武器层按 `shape` 选职业武器库
/// （`:505-547`）；坐骑层用 `Mounts[shape]`（`:473`）。图层前后照 `BringToFront()` 调用序。
#[allow(clippy::too_many_arguments)]
fn shop_viewer_system(
    mut shop: ResMut<GameShopState>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<ShopViewerFrameCache>,
    time: Res<Time>,
    loadout_q: Query<&crate::game::player_state::Loadout, With<crate::actor::LocalPlayer>>,
    appearance_q: Query<&crate::actor::ActorAppearance, With<crate::actor::LocalPlayer>>,
    mut panel: Query<
        (&mut Node, &mut Visibility, &GameShopViewerPanel),
        (
            Without<GameShopViewerLayer>,
            Without<GameShopViewerClose>,
            Without<GameShopViewerTurn>,
            Without<GameShopCellPreview>,
            Without<GameShopCellBuy>,
        ),
    >,
    mut layers: Query<
        (
            &GameShopViewerLayer,
            &mut ImageNode,
            &mut Node,
            &mut Visibility,
            &mut ZIndex,
        ),
        (
            Without<GameShopViewerPanel>,
            Without<GameShopViewerClose>,
            Without<GameShopViewerTurn>,
            Without<GameShopCellPreview>,
            Without<GameShopCellBuy>,
        ),
    >,
    // 格内试穿钮：显隐 + 点击（C# `PreviewItem.Visible` / `.Click`，`:111-133/278-282`）
    mut previews: Query<
        (Entity, &Interaction, &GameShopCellPreview, &mut Visibility),
        (
            Without<GameShopViewerPanel>,
            Without<GameShopViewerLayer>,
            Without<GameShopViewerClose>,
            Without<GameShopViewerTurn>,
            Without<GameShopCellBuy>,
        ),
    >,
    // 格内购买钮：可试穿时右移到 x=75（C# `BuyItem.Location`，`:281`）
    mut buy_nodes: Query<
        (&GameShopCellBuy, &mut Node),
        (
            Without<GameShopViewerPanel>,
            Without<GameShopViewerLayer>,
            Without<GameShopCellPreview>,
            Without<GameShopViewerClose>,
            Without<GameShopViewerTurn>,
        ),
    >,
    close: Query<(Entity, &Interaction), (With<GameShopViewerClose>, Without<GameShopViewerTurn>)>,
    turn: Query<(Entity, &Interaction, &GameShopViewerTurn), Without<GameShopViewerClose>>,
    mut prev_inter: Local<std::collections::HashMap<Entity, Interaction>>,
) {
    fn edge(
        e: Entity,
        inter: &Interaction,
        prev: &mut std::collections::HashMap<Entity, Interaction>,
    ) -> bool {
        let was = prev.insert(e, *inter);
        *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
    }
    // 关闭（C# `CloseButton.Click → Visible = false`，`:345-348`）
    for (e, inter) in &close {
        if edge(e, inter, &mut prev_inter) {
            shop.viewer = None;
        }
    }

    // 格内试穿钮：显隐 + 购买钮让位 + 点击开预览（C# `MirGameShopCell.UpdateText/Click`，`:111-133/278-282`）
    {
        let now = now_unix();
        let filtered = filter_shop_items(
            &shop.items,
            &shop.search,
            &shop.class_filter,
            &shop.category,
            &shop.section_filter,
            now,
        );
        let previewable: Vec<bool> = (0..8)
            .map(|i| {
                cell_item(&shop, &filtered, i)
                    .and_then(|it| shop.item_infos.get(&it.item_index))
                    .map(|info| viewer_previewable(info.item_type))
                    .unwrap_or(false)
            })
            .collect();
        let mut open_request: Option<(i32, f32)> = None;
        for (e, inter, p, mut vis) in &mut previews {
            let show = *previewable.get(p.0).unwrap_or(&false);
            let want = if show {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
            if *vis != want {
                *vis = want;
            }
            if show && edge(e, inter, &mut prev_inter) {
                if let Some(it) = cell_item(&shop, &filtered, p.0) {
                    open_request = Some((it.item_index, cell_pos(p.0).0));
                }
            }
        }
        for (b, mut node) in &mut buy_nodes {
            let x = viewer_buy_x(*previewable.get(b.0).unwrap_or(&false));
            if node.left != Val::Px(x) {
                node.left = Val::Px(x);
            }
        }
        if let Some((item_index, cell_x)) = open_request {
            shop.viewer = Some(ShopViewerState {
                item_index,
                // C# `PreviewItem.Click` 每次新建 `GameShopViewer` ⇒ `Direction` 回到初值 6（`:318`）
                direction: VIEWER_DIRECTION0,
                pos: viewer_side(cell_x),
            });
            tracing::info!(
                "🛒 试穿预览: item={} pos={:?}",
                item_index,
                viewer_side(cell_x)
            );
        }
    }
    // 转身（C# `Direction±1` 且 1..8 环绕，`:414-438`）
    for (e, inter, t) in &turn {
        if edge(e, inter, &mut prev_inter) {
            if let Some(v) = shop.viewer.as_mut() {
                v.direction = viewer_turn(v.direction, t.right);
            }
        }
    }

    let Some(viewer) = shop.viewer else {
        for (_, mut vis, _) in &mut panel {
            if *vis != Visibility::Hidden {
                *vis = Visibility::Hidden;
            }
        }
        return;
    };
    // 面板位置/显隐（C# `:129` 决定左/右半）
    for (mut node, mut vis, _) in &mut panel {
        if node.left != Val::Px(viewer.pos.0) {
            node.left = Val::Px(viewer.pos.0);
        }
        if node.top != Val::Px(viewer.pos.1) {
            node.top = Val::Px(viewer.pos.1);
        }
        if *vis != Visibility::Visible {
            *vis = Visibility::Visible;
        }
    }

    let info = shop
        .item_infos
        .get(&viewer.item_index)
        .copied()
        .unwrap_or_default();
    let dir = viewer.direction.clamp(1, 8);
    let female_player = appearance_q
        .single()
        .map(|a| a.gender == mir2_shared::enums::MirGender::Female)
        .unwrap_or(false);
    // 玩家自己穿的护甲 shape（身体层用它；C# `:461/487`）
    let equipped_armour = loadout_q
        .single()
        .ok()
        .and_then(|l| l.slots.get(EQUIP_SLOT_ARMOUR))
        .and_then(|s| s.as_ref())
        .and_then(|it| shop.item_infos.get(&(it.item_index)).copied())
        .map(|i| i.shape.max(0) as usize)
        .unwrap_or(0);

    let mounted = info.item_type == ITEM_TYPE_MOUNT;
    let frames = if mounted { 8usize } else { 6 };
    let anim = ((time.elapsed_secs() * 1000.0 / VIEWER_ANIM_MS).max(0.0) as usize) % frames;

    // 各层这一帧要画的东西
    let (body_shape_lib, body_base): (ArrayLibType, usize) = match info.item_type {
        ITEM_TYPE_ARMOUR => (
            ArrayLibType::CArmours,
            // 护甲预览按**物品的** RequiredGender 选帧（`:566-569`）
            viewer_body_index(info.required_gender == GENDER_FEMALE, dir, false),
        ),
        ITEM_TYPE_TRANSFORM => (
            ArrayLibType::Transform,
            viewer_body_index(false, dir, false),
        ),
        _ => (
            ArrayLibType::CArmours,
            viewer_body_index(female_player, dir, mounted),
        ),
    };
    let body_slot = if info.item_type == ITEM_TYPE_ARMOUR {
        info.shape.max(0) as usize
    } else {
        equipped_armour
    };
    let body: ViewerLayerSpec = Some((
        body_shape_lib,
        body_slot,
        body_base + anim,
        viewer_layer_z(dir, info.shape, false).2 + 41,
    ));
    let (weapon_lib, weapon_slot, weapon2) = viewer_weapon_libs(info.shape);
    let (wz, w2z, _) = viewer_layer_z(dir, info.shape, weapon2.is_some());
    let weapon: ViewerLayerSpec = if info.item_type == ITEM_TYPE_WEAPON {
        Some((
            weapon_lib,
            weapon_slot,
            viewer_weapon_index(dir) + anim,
            wz + 41,
        ))
    } else {
        None
    };
    let weapon2: ViewerLayerSpec = match (info.item_type, weapon2) {
        (ITEM_TYPE_WEAPON, Some((lib, slot))) => {
            Some((lib, slot, viewer_weapon_index(dir) + anim, w2z + 41))
        }
        _ => None,
    };
    let mount: ViewerLayerSpec = if mounted {
        Some((
            ArrayLibType::Mounts,
            info.shape.max(0) as usize,
            viewer_mount_index(dir) + anim,
            // 坐骑层在身体之下（C# 只设 Visible，不 BringToFront）
            41,
        ))
    } else {
        None
    };

    for (layer, mut img, mut node, mut vis, mut z) in &mut layers {
        let spec = match layer {
            GameShopViewerLayer::Body => body,
            GameShopViewerLayer::Weapon => weapon,
            GameShopViewerLayer::Weapon2 => weapon2,
            GameShopViewerLayer::Mount => mount,
        };
        let Some((lib, slot, frame, zval)) = spec else {
            if *vis != Visibility::Hidden {
                *vis = Visibility::Hidden;
            }
            continue;
        };
        let Some((h, off, (w, hh))) =
            viewer_frame(&mut libs, &mut images, &mut cache, lib, slot, frame)
        else {
            if *vis != Visibility::Hidden {
                *vis = Visibility::Hidden;
            }
            continue;
        };
        if img.image != h {
            img.image = h;
        }
        // `UseOffSet`：绘制点 = 图层 Location + 该帧艺术偏移（`MirImageControl.cs:7`）
        let dx = VIEWER_LAYER_POS.0 + off.0 as f32;
        let dy = VIEWER_LAYER_POS.1 + off.1 as f32;
        if node.left != Val::Px(dx) {
            node.left = Val::Px(dx);
        }
        if node.top != Val::Px(dy) {
            node.top = Val::Px(dy);
        }
        if node.width != Val::Px(w) {
            node.width = Val::Px(w);
        }
        if node.height != Val::Px(hh) {
            node.height = Val::Px(hh);
        }
        if z.0 != zval {
            z.0 = zval;
        }
        if *vis != Visibility::Visible {
            *vis = Visibility::Visible;
        }
    }
}

/// 格子图标索引（纯函数，门禁可测）：C# 用 `Libraries.Items[Item.Info.Image]`；
/// `image <= 0` 视为无图（空槽/未下发图号）。
pub(crate) fn shop_cell_icon_index(image: i32) -> Option<usize> {
    if image > 0 {
        Some(image as usize)
    } else {
        None
    }
}

/// 商品格图标刷新：按当前分类/搜索/页取该格的商品，拿 `image` 去 `Libraries.Items` 取图。
/// 单独成一个系统（现有渲染系统已经吃满 ParamSet 槽位），分类切换/翻页时同一份
/// `filter_shop_items` 口径 ⇒ 图标与文字同步换页。
fn shop_cell_icons_system(
    shop: Res<GameShopState>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut q: Query<(&GameShopCellIcon, &mut ImageNode, &mut Visibility)>,
) {
    let filtered = filter_shop_items(
        &shop.items,
        &shop.search,
        &shop.class_filter,
        &shop.category,
        &shop.section_filter,
        now_unix(),
    );
    for (icon, mut node, mut vis) in q.iter_mut() {
        let image = cell_item(&shop, &filtered, icon.0)
            .map(|it| it.image)
            .unwrap_or(0);
        match shop_cell_icon_index(image) {
            Some(idx) => match load_lib_image(&mut libs, &mut images, LibraryName::Items, idx) {
                Some(h) => {
                    node.image = h;
                    *vis = Visibility::Visible;
                }
                None => *vis = Visibility::Hidden,
            },
            None => *vis = Visibility::Hidden,
        }
    }
}

fn cell_item<'a>(shop: &'a GameShopState, filtered: &'a [usize], i: usize) -> Option<&'a ShopItem> {
    filtered
        .get(shop.page * 8 + i)
        .and_then(|&idx| shop.items.get(idx))
}

pub struct GameShopPlugin;

impl Plugin for GameShopPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameShopState>();
        app.init_resource::<GameShopPayFrames>();
        app.init_resource::<ShopViewerFrameCache>();
        app.add_systems(Update, shop_server_events.run_if(in_state(AppState::Game)));
        app.add_systems(
            Update,
            shop_cell_icons_system.run_if(in_state(AppState::Game)),
        );
        app.add_systems(OnEnter(AppState::Game), spawn_game_shop);
        app.add_systems(OnExit(AppState::Game), cleanup_game_shop);
        app.add_systems(
            Update,
            (game_shop_ui_system, game_shop_pay_system).run_if(in_state(AppState::Game)),
        );
        // 筛选按钮选中态（C# `ResetClass`/`ResetTabs` 换帧：职业选中=hover 帧、区段选中=第二帧）
        app.add_systems(
            Update,
            shop_filter_button_visuals_system.run_if(in_state(AppState::Game)),
        );
        // 试穿预览（C# `GameShopViewer`）
        app.add_systems(Update, shop_viewer_system.run_if(in_state(AppState::Game)));
    }
}

/// 职业/区段筛选按钮的选中态（C# `ResetClass` / `ResetTabs` 只换 `Index`，不重建按钮）。
///
/// 单独成一个系统：`game_shop_ui_system` 的参数表已经贴着 Bevy 上限，而这里只需要
/// 读筛选状态 + 写按钮贴图。`ShopFilterBtn` 单个 marker 类型 ⇒ 一个查询搞定，无 B0001 风险。
fn shop_filter_button_visuals_system(
    shop: Res<GameShopState>,
    mut q: Query<(
        &ShopFilterBtn,
        &crate::ui::theme::ImageButton,
        &mut ImageNode,
    )>,
) {
    for (kind, ib, mut node) in &mut q {
        let active = match kind {
            ShopFilterBtn::Class(i) => shop.class_filter == CLASS_FILTERS[*i].0,
            ShopFilterBtn::Section(i) => shop.section_filter == SECTION_FILTERS[*i].0,
        };
        let want = if active { &ib.pressed } else { &ib.normal };
        if node.image != *want {
            node.image = want.clone();
        }
    }
}

fn cleanup_game_shop(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_game_shop(
    mut commands: Commands,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut fonts: ResMut<Assets<Font>>,
    mut cjk_font: ResMut<UiCjkFont>,
    mut ui_font: ResMut<UiFont>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();
    let cjk = shared_cjk_font(&mut fonts, &mut cjk_font);

    // 付款复选框两帧（C# `MirCheckBox.UnTickedIndex/TickedIndex` = `Prguse[2086]/[2087]`）
    let pay_unchecked = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 2086);
    let pay_checked = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 2087);
    commands.insert_resource(GameShopPayFrames {
        unchecked: pay_unchecked.clone(),
        checked: pay_checked.clone(),
    });

    // 面板 Title[749]（C# GameshopDialog Index=749，696x476 居中 @(164,146)；
    // 旧 Bevy 用 Title[411] 259 宽占位，分类/搜索悬空面板外）
    let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Title, 749) else {
        return;
    };
    let (px, py) = ((1024.0 - 696.0) / 2.0, (768.0 - 476.0) / 2.0);
    let panel = spawn_panel(&mut commands, bg, px, py, PANEL_SIZE.0, PANEL_SIZE.1, 30);
    commands
        .entity(panel)
        .insert((DialogRoot(DialogKind::GameShop), GameShopWidget));

    // 分类滚动条（C# `PositionBar` `Prguse2[205]` @(120,117)，y 行程 117..401
    // = `UpdatePositionBar` 钳位，滑块高 20 → 轨道高 304）：
    // 共享 UiScrollList = 滚轮 + 滑块拖动 + 滑块跟随；轨道 (120,117,16,304)
    let mut cat_thumb = None;
    commands.entity(panel).with_children(|p| {
        let (_, thumb) = spawn_scroll_bar_ui(p, (120.0, 117.0, 16.0, 304.0), 10);
        cat_thumb = Some(thumb);
        // 标题 Title[26]（C# (18,9)）
        if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Title, 26) {
            spawn_image(p, h, 18.0, 9.0, 103.0, 17.0, 8);
        }
        // 关闭（C# (671,4)）
        if let Some(mut btn) =
            spawn_close_button(p, &mut libs, &mut images, CLOSE_POS.0, CLOSE_POS.1, 10)
        {
            btn.insert(GameShopClose);
        }
        // 分类底图（C# `FilterBackground` Title[769] @(11,102)，尺寸取精灵原生；
        // z=8 垫在页签 z=9 之下；同时承接 C# `FilterBackground.MouseWheel` 的
        // 滚动命中区——滚轮命中由 UiScrollList rect_rel 覆盖同区域）
        if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Title, 769) {
            let (w, hh) = images
                .get(&h)
                .map(|img| {
                    let sz = img.size_f32();
                    (sz.x, sz.y)
                })
                .unwrap_or((125.0, 336.0));
            spawn_image(p, h, 11.0, 102.0, w, hh, 8);
        }
        // 分类页签（C# `Filters[22]` 90x20 @(15, 103+15i)，行距 15——
        // C# 按钮高 20 行距 15 微叠，文本行不重叠）
        for i in 0..22usize {
            spawn_label(
                p,
                &cjk,
                "",
                15.0,
                103.0 + i as f32 * 15.0,
                12.0,
                Color::srgb(0.9, 0.9, 0.9),
                9,
            )
            .insert(GameShopCat(i));
        }
        // 分类翻页（C# `UpButton` `Prguse2[197..199]` @(120,103) /
        // `DownButton` `Prguse2[207..209]` @(120,421)；PositionBar 滚动条另见滚动条批次）
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 197),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 198),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 199),
        ) {
            spawn_icon_button(p, n, h, pr, 120.0, 103.0, 16.0, 14.0, 10).insert(GameShopCatUp);
        }
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 207),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 208),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 209),
        ) {
            spawn_icon_button(p, n, h, pr, 120.0, 421.0, 16.0, 14.0, 10).insert(GameShopCatDown);
        }
        // 职业筛选六档（C# `ClassFilter`：`Title[751..768]`，`ALL@(539,37)`、其余 `@(568+23i,38)`；
        // 选中态用 hover 帧——`ResetClass` 把 `ALL.Index` 设成 752，本端对应 `ImageButton.pressed`）
        for (i, (_val, n_i, a_i, x)) in CLASS_FILTERS.iter().enumerate() {
            if let (Some(n), Some(a)) = (
                load_lib_image(&mut libs, &mut images, LibraryName::Title, *n_i),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, *a_i),
            ) {
                let (w, h) = lib_img_size(&images, &n);
                spawn_icon_button(p, n, a.clone(), a, *x, CLASS_BTN_Y[i], w, h, 10)
                    .insert(ShopFilterBtn::Class(i));
            }
        }
        // 区段筛选四档（C# `SectionFilter` @(138|209|280|351, 68)，常态 770/776/772/774、
        // 选中 771/777/773/775；第 4 档 `New` 在原版初始 `Visible=false` 且从未置 true）
        for (i, (_val, n_i, a_i, x)) in SECTION_FILTERS.iter().enumerate() {
            if let (Some(n), Some(a)) = (
                load_lib_image(&mut libs, &mut images, LibraryName::Title, *n_i),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, *a_i),
            ) {
                let (w, h) = lib_img_size(&images, &n);
                let mut cmds = spawn_icon_button(p, n, a.clone(), a, *x, SECTION_BTN_Y, w, h, 10);
                cmds.insert(ShopFilterBtn::Section(i));
                if i == 3 {
                    // C# `New.Visible = false`（`GameshopDialog.cs:263`）——照原版保持不可见
                    cmds.insert(Visibility::Hidden);
                }
            }
        }
        // 商品格 8 = 4列x2行（C# `Grid` `MirGameShopCell` 125x146 `Title[750]`：
        // 上行 @(152+i*132,115)，下行 @(152+i*132,275)；空槽整格隐藏与 C# 不建格一致）
        let cell_bg = load_lib_image(&mut libs, &mut images, LibraryName::Title, 750);
        let buy_frames = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 778),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 779),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 780),
        );
        let qty_down_frames = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 240),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 241),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 242),
        );
        let qty_up_frames = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 243),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 244),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 245),
        );
        for i in 0..8usize {
            let (cx, cy) = cell_pos(i);
            let mut cell = spawn_container(p, cx, cy, 125.0, 146.0, 8);
            cell.insert(GameShopCell(i));
            cell.with_children(|cp| {
                // 格底图 `Title[750]`（C# `GameShopCell.Index = 750`）
                if let Some(bg) = cell_bg.clone() {
                    spawn_image(cp, bg, 0.0, 0.0, 125.0, 146.0, 0);
                }
                // 物品图标：C# `DrawControl` 把 `Libraries.Items[Image]` 居中画在 32×32 盒里，
                // 盒左上 = 格子本地 (12,40)（`offSet + DisplayLocation + (12,40)`）
                cp.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(12.0),
                        top: Val::Px(40.0),
                        width: Val::Px(32.0),
                        height: Val::Px(32.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    ZIndex(1),
                ))
                .with_children(|ib| {
                    ib.spawn((
                        ImageNode::default(),
                        GameShopCellIcon(i),
                        Visibility::Hidden,
                    ));
                });
                // 商品名（C# `nameLabel` 125x15 居中 @(0,13)；>17 字截断）
                spawn_label_center(cp, &cjk, "", 62.5, 13.0, 125.0, 12.0, Color::WHITE, 1)
                    .insert(GameShopCellName(i));
                // 「STOCK:」（C# `StockLabel` 40x20 @(53,37) 灰 7F）
                spawn_label(
                    cp,
                    &cjk,
                    "STOCK:",
                    53.0,
                    37.0,
                    10.0,
                    Color::srgb(0.5, 0.5, 0.5),
                    1,
                );
                // 库存值（C# `stockLabel` 20x20 居中 @(93,37)）
                spawn_label_center(cp, &cjk, "", 103.0, 37.0, 20.0, 10.0, Color::WHITE, 1)
                    .insert(GameShopCellStock(i));
                // 每件数量（C# `countLabel` 30x20 右对齐 @(16,60)）
                cp.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(16.0),
                        top: Val::Px(60.0),
                        width: Val::Px(30.0),
                        height: Val::Px(20.0),
                        ..default()
                    },
                    Text::new(String::new()),
                    TextFont {
                        font: FontSource::Handle(cjk.clone()),
                        font_size: FontSize::Px(10.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    TextLayout::justify(Justify::Right),
                    ZIndex(1),
                    GameShopCellCount(i),
                ));
                // 积分价（C# `gpLabel` 95x20 右对齐 @(2,81)）
                cp.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(2.0),
                        top: Val::Px(81.0),
                        width: Val::Px(95.0),
                        height: Val::Px(20.0),
                        ..default()
                    },
                    Text::new(String::new()),
                    TextFont {
                        font: FontSource::Handle(cjk.clone()),
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    TextLayout::justify(Justify::Right),
                    ZIndex(1),
                    GameShopCellCredit(i),
                ));
                // 金币价（C# `goldLabel` 95x20 右对齐 @(2,102)）
                cp.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(2.0),
                        top: Val::Px(102.0),
                        width: Val::Px(95.0),
                        height: Val::Px(20.0),
                        ..default()
                    },
                    Text::new(String::new()),
                    TextFont {
                        font: FontSource::Handle(cjk.clone()),
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(Color::srgb(1.0, 0.9, 0.3)),
                    TextLayout::justify(Justify::Right),
                    ZIndex(1),
                    GameShopCellGold(i),
                ));
                // 数量减/值/加（C# @(55,56) / 20x13 @(74,56) / @(97,56)；Shift=±10）
                if let (Some(n), Some(h), Some(pr)) = qty_down_frames.clone() {
                    spawn_icon_button(cp, n, h, pr, 55.0, 56.0, 16.0, 14.0, 2)
                        .insert(GameShopCellQtyDown(i));
                }
                spawn_label_center(cp, &cjk, "1", 84.0, 56.0, 20.0, 12.0, Color::WHITE, 1)
                    .insert(GameShopCellQty(i));
                if let (Some(n), Some(h), Some(pr)) = qty_up_frames.clone() {
                    spawn_icon_button(cp, n, h, pr, 97.0, 56.0, 16.0, 14.0, 2)
                        .insert(GameShopCellQtyUp(i));
                }
                // 购买钮（C# `BuyItem` `Title[778..780]` @(42,122)）
                if let (Some(n), Some(h), Some(pr)) = buy_frames.clone() {
                    spawn_icon_button(cp, n, h, pr, 42.0, 122.0, 42.0, 22.0, 2)
                        .insert(GameShopCellBuy(i));
                }
                // 试穿钮（C# `PreviewItem` `Title[781..783]` @(8,122)，仅 Mount/Weapon/Armour/Transform 可见；
                // 此时 BuyItem 右移到 x=75，`:111-133/278-282`）
                if let (Some(n), Some(h), Some(pr)) = (
                    load_lib_image(&mut libs, &mut images, LibraryName::Title, 781),
                    load_lib_image(&mut libs, &mut images, LibraryName::Title, 782),
                    load_lib_image(&mut libs, &mut images, LibraryName::Title, 783),
                ) {
                    let mut cmds = spawn_icon_button(cp, n, h, pr, 8.0, 122.0, 42.0, 22.0, 2);
                    cmds.insert(GameShopCellPreview(i));
                    cmds.insert(Visibility::Hidden);
                }
            });
        }
        // 页码 + 翻页（C# `PageNumberLabel` 83x17 @(597,446)、
        // `PreviousButton` `Prguse2[240]` @(600,448)、`NextButton` `Prguse2[243]` @(660,448)）
        spawn_label_center(p, &cjk, "", 638.5, 446.0, 83.0, 10.0, Color::WHITE, 9)
            .insert(GameShopPageLabel);
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 240),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 241),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 242),
        ) {
            spawn_icon_button(p, n, h, pr, 600.0, 448.0, 16.0, 14.0, 10).insert(GameShopPagePrev);
        }
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 243),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 244),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 245),
        ) {
            spawn_icon_button(p, n, h, pr, 660.0, 448.0, 16.0, 14.0, 10).insert(GameShopPageNext);
        }
        // 搜索（C# Search @(540,69) 140x16；TextInput 31；C# 无文字标签，面板图自带标识）
        spawn_container(p, 540.0, 69.0, 140.0, 16.0, 10)
            .insert((
                BackgroundColor(Color::srgba(0.05, 0.05, 0.08, 0.95)),
                crate::game::dialogs::text_input::TextInputField(31),
                crate::game::dialogs::text_input::TextInputRect(px + 540.0, py + 69.0, 140.0, 16.0),
            ))
            .with_children(|ic| {
                ic.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(4.0),
                        top: Val::Px(1.0),
                        ..default()
                    },
                    Text::new(String::new()),
                    TextFont {
                        font: FontSource::Handle(cjk.clone()),
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    ZIndex(11),
                    crate::game::dialogs::text_input::TextInputDisplay(31),
                ));
            });
        // ===== 付款方式复选框 + 余额标签（C# `GameshopDialog.cs:79-97/185-210`；#2791 单元②）=====
        // 复选框 @(250,449)/(340,449) 16x13，LabelText @(+15,-2)（`MirCheckBox.cs:89`）
        for (x, label, hint, is_gold) in [
            (250.0f32, "用金币购买", "用金币购买物品。", true),
            (340.0, "用积分购买", "用积分购买物品。", false),
        ] {
            if let Some(frame) = pay_unchecked.clone() {
                let mut e = spawn_image(p, frame, x, 449.0, 16.0, 13.0, 9);
                e.insert((
                    Button,
                    crate::ui::tooltip::UiHint {
                        text: hint.to_string(),
                    },
                ));
                if is_gold {
                    e.insert(GameShopPayGold);
                } else {
                    e.insert(GameShopPayCredit);
                }
            }
            // #2791：中文标签用共享宋体（原 C# `Settings.FontName` 支持 CJK；Arial 会豆腐）
            spawn_label(p, &cjk, label, x + 15.0, 447.0, 12.0, Color::WHITE, 9);
        }
        // 余额标签（C# `totalCredits` @(5,449) / `totalGold` @(123,449)，100x20 右对齐）
        for (x, is_gold) in [(5.0f32, false), (123.0, true)] {
            let mut e = p.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(x),
                    top: Val::Px(449.0),
                    width: Val::Px(100.0),
                    height: Val::Px(20.0),
                    ..default()
                },
                Text::new(String::new()),
                TextFont {
                    font: FontSource::Handle(cjk.clone()),
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(Color::WHITE),
                TextLayout::justify(Justify::Right),
                ZIndex(9),
            ));
            if is_gold {
                e.insert(GameShopGoldLabel);
            } else {
                e.insert(GameShopCreditLabel);
            }
        }
        // ===== 试穿预览面板（C# `GameShopViewer`，`MirGameShopCell.cs:312-440`）=====
        // 作为商城对话框的子控件（C# `Parent = GameShopDialog`），默认隐藏；
        // 四个图层都在 (105,160)、`UseOffSet=true`、`AnimationDelay=150`。
        if let Some(bg) = load_lib_image(&mut libs, &mut images, VIEWER_PANEL.0, VIEWER_PANEL.1) {
            let (vw, vh) = lib_img_size(&images, &bg);
            let mut viewer = spawn_container(p, VIEWER_LEFT_SIDE.0, VIEWER_LEFT_SIDE.1, vw, vh, 40);
            viewer.insert(GameShopViewerPanel);
            viewer.insert(Visibility::Hidden);
            viewer.with_children(|vp| {
                vp.spawn((ImageNode::new(bg.clone()), ZIndex(40)));
                // 四个图层（顺序无关，前后由 ZIndex 每帧按方向重排）
                for layer in [
                    GameShopViewerLayer::Mount,
                    GameShopViewerLayer::Weapon2,
                    GameShopViewerLayer::Weapon,
                    GameShopViewerLayer::Body,
                ] {
                    vp.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(VIEWER_LAYER_POS.0),
                            top: Val::Px(VIEWER_LAYER_POS.1),
                            ..default()
                        },
                        ImageNode::new(bg.clone()),
                        layer,
                        Visibility::Hidden,
                        ZIndex(41),
                    ));
                }
                // 关闭（`Prguse[361..363]` @(230,8)）
                if let (Some(n), Some(h), Some(pr)) = (
                    load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 361),
                    load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 362),
                    load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 363),
                ) {
                    spawn_icon_button(
                        vp,
                        n,
                        h,
                        pr,
                        VIEWER_CLOSE_POS.0,
                        VIEWER_CLOSE_POS.1,
                        24.0,
                        21.0,
                        45,
                    )
                    .insert(GameShopViewerClose);
                }
                // 左/右转身（`Prguse2[240..242]` @(81,282) / `[243..245]` @(160,282)）
                if let (Some(n), Some(h), Some(pr)) = (
                    load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 240),
                    load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 241),
                    load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 242),
                ) {
                    spawn_icon_button(
                        vp,
                        n,
                        h,
                        pr,
                        VIEWER_PREV_POS.0,
                        VIEWER_PREV_POS.1,
                        16.0,
                        14.0,
                        45,
                    )
                    .insert(GameShopViewerTurn { right: false });
                }
                if let (Some(n), Some(h), Some(pr)) = (
                    load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 243),
                    load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 244),
                    load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 245),
                ) {
                    spawn_icon_button(
                        vp,
                        n,
                        h,
                        pr,
                        VIEWER_NEXT_POS.0,
                        VIEWER_NEXT_POS.1,
                        16.0,
                        14.0,
                        45,
                    )
                    .insert(GameShopViewerTurn { right: true });
                }
            });
        }
    });
    commands.entity(panel).insert(UiScrollList {
        rect_rel: (11.0, 102.0, 125.0, 336.0),
        row_h: 15.0,
        visible: 22,
        total: 0,
        offset: 0,
        step: 1,
        // C# `UpdatePositionBar`/`PositionBar_OnMoving`：y 行程 117..(401)，滑块高 20
        // → 轨道高 304（`GameshopDialog.cs:586-601`）
        track_rel: (120.0, 117.0, 16.0, 304.0),
        thumb: cat_thumb,
        z: 30,
    });

    // 购买确认框（C# `MirMessageBox`：`Prguse[360]` 456x190 居中 @(284,289)，
    // Yes `Title[206..208]` @(260,157) / No `Title[210..212]` @(360,157)）
    if let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 360) {
        let confirm = spawn_panel(&mut commands, bg, 284.0, 289.0, 456.0, 190.0, 46);
        commands.entity(confirm).insert((
            GameShopConfirm,
            DialogRoot(DialogKind::GameShop),
            crate::game::dialogs::AlwaysVisible,
        ));
        commands.entity(confirm).with_children(|p| {
            spawn_label(p, &cjk, "", 35.0, 35.0, 12.0, Color::WHITE, 9).insert(GameShopConfirmText);
            if let (Some(n), Some(h), Some(pr)) = (
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 206),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 207),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 208),
            ) {
                spawn_icon_button(p, n, h, pr, 260.0, 157.0, 76.0, 25.0, 10)
                    .insert(GameShopConfirmYes);
            }
            if let (Some(n), Some(h), Some(pr)) = (
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 210),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 211),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 212),
            ) {
                spawn_icon_button(p, n, h, pr, 360.0, 157.0, 76.0, 25.0, 10)
                    .insert(GameShopConfirmNo);
            }
        });
    }
}

/// 商城按钮打包（全只读 (Entity, &Interaction)，无冲突；控系统参数个数）
#[derive(SystemParam)]
struct ShopButtons<'w, 's> {
    close: Query<'w, 's, (Entity, &'static Interaction), With<GameShopClose>>,
    cat_up: Query<'w, 's, (Entity, &'static Interaction), With<GameShopCatUp>>,
    cat_down: Query<'w, 's, (Entity, &'static Interaction), With<GameShopCatDown>>,
    page_prev: Query<'w, 's, (Entity, &'static Interaction), With<GameShopPagePrev>>,
    page_next: Query<'w, 's, (Entity, &'static Interaction), With<GameShopPageNext>>,
    /// 三段筛选按钮（职业 / 区段）
    /// （格内试穿钮的显隐/点击在 `shop_viewer_system` 里处理：`ui_set.p0()` 也写
    /// `Visibility`，放这里会 B0001）
    filters: Query<'w, 's, (Entity, &'static Interaction, &'static ShopFilterBtn)>,
}

/// 显隐 + 渲染 + 关闭/翻页/分类 + 打开时请求目录
#[allow(clippy::too_many_arguments)]
fn game_shop_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut shop: ResMut<GameShopState>,
    mut input: ResMut<TextInputState>,
    net: Res<NetConnection>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    mut widgets: Query<&mut Visibility, With<GameShopWidget>>,
    // 9 个 &mut Text/&mut Visibility 查询：**必须放进同一个 ParamSet** 分时借用。
    // B0001 只在单个 ParamSet **内部**被豁免——拆成两个 ParamSet 时，各自并集里
    // 的无过滤 `&mut Text` 会在 system_meta 上正面相撞（实机一进游戏即 panic）。
    mut ui_set: ParamSet<(
        Query<(&mut Visibility, &GameShopCell), Without<GameShopWidget>>,
        Query<(&mut Text, &GameShopCellName)>,
        Query<(&mut Text, &GameShopCellGold)>,
        Query<(&mut Text, &GameShopCellCredit)>,
        Query<(&mut Text, &GameShopCellStock)>,
        Query<
            (
                &mut Text,
                Option<&GameShopCellCount>,
                Option<&GameShopCellQty>,
            ),
            Or<(With<GameShopCellCount>, With<GameShopCellQty>)>,
        >,
        Query<(&mut Text, &GameShopCat)>,
        Query<&mut Text, With<GameShopPageLabel>>,
    )>,
    buttons: ShopButtons,
    mut cat_scroll: Query<&mut UiScrollList, (With<GameShopWidget>, Without<GameShopCell>)>,
    mut requested: Local<bool>,
    mut prev_inter: Local<std::collections::HashMap<Entity, Interaction>>,
    panel_origin: Query<&Node, With<GameShopWidget>>,
    // C# `Show()` 里 `ClassFilter = User.Class.ToString()`（`GameshopDialog.cs:508`）
    local_appearance: Query<&crate::actor::ActorAppearance, With<crate::actor::LocalPlayer>>,
) {
    fn edge(
        e: Entity,
        inter: &Interaction,
        prev: &mut std::collections::HashMap<Entity, Interaction>,
    ) -> bool {
        let was = prev.insert(e, *inter);
        *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
    }
    let open = mgr.is_open(DialogKind::GameShop);
    for mut vis in widgets.iter_mut() {
        *vis = if open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !open {
        *requested = false;
        shop.search.clear();
        if input.texts.len() > 31 {
            input.texts[31].clear();
        }
        return;
    }
    // 打开瞬间请求商城目录（C# GameshopDialog.Show → C.GameshopBuy{g_index=0}）
    if !*requested {
        *requested = true;
        // C# `Show()`（`GameshopDialog.cs:504-513`）：开窗即把职业筛选设成**自己的职业**、
        // 区段回 `Show All`，再 GetCategories（TypeFilter 归零 + 重建分类表）
        shop.class_filter = local_appearance
            .single()
            .map(|a| class_filter_name(a.class as u8).to_string())
            .unwrap_or_else(|_| "Show All".to_string());
        shop.section_filter = "Show All".to_string();
        rebuild_categories(&mut shop, now_unix());
        net.send_packet(&crate::network::GameshopBuyWire {
            g_index: 0,
            quantity: 0,
            p_type: 0,
        });
        tracing::info!("🛒 请求商城目录");
    }
    let Ok(mut cat_list) = cat_scroll.single_mut() else {
        return;
    };
    // 搜索同步（C# KeyUp 本地过滤 + ResetPage；texts 由 text_input_system 每帧回填）
    if let Some(t) = input.texts.get(31) {
        if shop.search != *t {
            // C# `Search.TextBox.KeyUp → GetCategories()`：TypeFilter="Show All"、
            // Page/StartIndex 归零、PositionBar 回 (120,117)（`GameshopDialog.cs:180-183/647-651`）
            shop.search = t.clone();
            rebuild_categories(&mut shop, now_unix());
            shop.qty = [1; 8];
            cat_list.offset = 0;
        }
    }
    let filtered = filter_shop_items(
        &shop.items,
        &shop.search,
        &shop.class_filter,
        &shop.category,
        &shop.section_filter,
        now_unix(),
    );
    for (e, inter) in &buttons.close {
        if edge(e, inter, &mut prev_inter) {
            mgr.close(DialogKind::GameShop);
        }
    }
    // 商品翻页（C# PreviousButton/NextButton：Page±1 后 UpdateShop 重建格子）
    let pages = max_page(filtered.len());
    if shop.page >= pages {
        shop.page = pages - 1;
    }
    for (e, inter) in &buttons.page_prev {
        if edge(e, inter, &mut prev_inter) && shop.page > 0 {
            shop.page -= 1;
            shop.qty = [1; 8];
        }
    }
    for (e, inter) in &buttons.page_next {
        if edge(e, inter, &mut prev_inter) && shop.page + 1 < pages {
            shop.page += 1;
            shop.qty = [1; 8];
        }
    }
    // 三段筛选点击：
    //   职业（C# `ClassFilter=X; TypeFilter="Show All"; GetCategories(); ResetClass();`）
    //   区段（C# `SectionFilter=X; ResetTabs(); GetCategories();`）
    // 两者都走 `GetCategories()` ⇒ 分类表按新筛选重建、回到第一页、分类滚动归零。
    for (e, inter, b) in &buttons.filters {
        if !edge(e, inter, &mut prev_inter) {
            continue;
        }
        let (want, kind) = match b {
            ShopFilterBtn::Class(i) => (CLASS_FILTERS[*i].0, "职业"),
            ShopFilterBtn::Section(i) => (SECTION_FILTERS[*i].0, "区段"),
        };
        let changed = match b {
            ShopFilterBtn::Class(_) => shop.class_filter != want,
            ShopFilterBtn::Section(_) => shop.section_filter != want,
        };
        if !changed {
            continue;
        }
        match b {
            ShopFilterBtn::Class(_) => shop.class_filter = want.to_string(),
            ShopFilterBtn::Section(_) => shop.section_filter = want.to_string(),
        }
        rebuild_categories(&mut shop, now_unix());
        shop.qty = [1; 8];
        cat_list.offset = 0;
        cat_list.set_total(shop.categories.len());
        tracing::info!("🛒 商城{}筛选: {}", kind, want);
    }
    for mut t in &mut ui_set.p7() {
        let s = format!("{} / {}", shop.page + 1, pages);
        if t.0 != s {
            t.0 = s;
        }
    }
    // 商品格渲染（C# `UpdateShop`：第 page 页 8 格；空槽整格隐藏；
    // 名称 >17 截断——`MirGameShopCell.UpdateText`）
    for (mut vis, cell) in &mut ui_set.p0() {
        *vis = if cell_item(&shop, &filtered, cell.0).is_some() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for (mut text, c) in &mut ui_set.p1() {
        text.0 = match cell_item(&shop, &filtered, c.0) {
            Some(it) => {
                // P3-3：it.name -> 本地物品名表 -> 发一次 RequestItemInfo（按索引去重）-> #id
                // 先把条目字段拷成局部量：`it` 借的是 `shop`，而下面要写 `shop.requested_item_info`
                let item_index = it.item_index;
                let item_name = it.name.clone();
                let (n, need_req) = resolve_shop_name(&item_name, &shop.item_names, item_index);
                if need_req && shop.requested_item_info.insert(item_index) {
                    net.send_packet(&mir2_shared::packets::client::info::RequestItemInfo {
                        item_index,
                    });
                    tracing::info!("🛒 商城缺物品名，请求 ItemInfo: idx={}", item_index);
                }
                // C# `UpdateText`：>17 字截断
                if n.chars().count() > 17 {
                    n.chars().take(17).collect()
                } else {
                    n
                }
            }
            None => String::new(),
        };
    }
    for (mut text, c) in &mut ui_set.p2() {
        text.0 = match cell_item(&shop, &filtered, c.0) {
            // C# `UpdateText`：仅 CanBuyGold 写 goldLabel；价格随选购数量联动
            Some(it) if it.can_buy_gold => {
                crate::game::hud::format_gold(it.gold_price.saturating_mul(shop.qty[c.0] as u32))
            }
            _ => String::new(),
        };
    }
    for (mut text, c) in &mut ui_set.p3() {
        text.0 = match cell_item(&shop, &filtered, c.0) {
            Some(it) if it.can_buy_credit => {
                crate::game::hud::format_gold(it.credit_price.saturating_mul(shop.qty[c.0] as u32))
            }
            _ => String::new(),
        };
    }
    for (mut text, c) in &mut ui_set.p4() {
        text.0 = match cell_item(&shop, &filtered, c.0) {
            // C# `UpdateText`：0=∞、>=99=99+、否则原值（原版顺序怪癖：>=99 先判）
            Some(it) if it.stock >= 99 => "99+".to_string(),
            Some(it) if it.stock == 0 => "∞".to_string(),
            Some(it) => it.stock.to_string(),
            None => String::new(),
        };
    }
    // 数量/单价两文本合并为一查询：`&mut Text` 查询必须全部塞进**单个** ParamSet
    // 才免 B0001，而 ParamSet 上限 8 个（见系统参数处注释）
    for (mut text, cnt, qty) in &mut ui_set.p5() {
        // spawn 侧不变量：数量与单价文本挂**不同实体**（否则 else-if 会让单价永不刷新）
        debug_assert!(
            !(cnt.is_some() && qty.is_some()),
            "GameShopCellCount 与 GameShopCellQty 不得同挂一个实体"
        );
        if let Some(c) = cnt {
            text.0 = match cell_item(&shop, &filtered, c.0) {
                Some(it) => it.count.to_string(),
                None => String::new(),
            };
        } else if let Some(q) = qty {
            let s = shop.qty[q.0].to_string();
            if text.0 != s {
                text.0 = s;
            }
        }
    }
    // 分类渲染（C# Filters[22]：第 0 项 = 全部；CStartIndex 行偏移 22 行窗，
    // ▶ 标记当前选中；偏移由共享 UiScrollList 驱动：滚轮 + PositionBar 拖动）
    cat_list.set_total(shop.categories.len());
    let cat_base = cat_list.offset;
    for (mut text, row) in &mut ui_set.p6() {
        let idx = cat_base + row.0;
        text.0 = match shop.categories.get(idx) {
            Some(c) => {
                let label = if c.is_empty() {
                    "全部".to_string()
                } else {
                    c.clone()
                };
                if *c == shop.category {
                    format!("▶ {}", label)
                } else {
                    label
                }
            }
            None => String::new(),
        };
    }
    // 分类翻页（C# UpButton/DownButton：步 1 行，下限 0、上限 Count-22）
    for (e, inter) in &buttons.cat_up {
        if edge(e, inter, &mut prev_inter) && cat_list.offset > 0 {
            cat_list.offset -= 1;
        }
    }
    for (e, inter) in &buttons.cat_down {
        if edge(e, inter, &mut prev_inter) && cat_list.offset < cat_list.max_offset() {
            cat_list.offset += 1;
        }
    }
    // 分类点击（C# `Filters.Click` 90x20 @(15, 103+15i) 行距 15；CStartIndex 翻页）
    if mouse.just_pressed(MouseButton::Left) {
        if let Ok(window) = windows.single() {
            if let Some(cursor) = window.cursor_position() {
                let (ox, oy) = panel_origin
                    .single()
                    .map(|n| crate::ui::theme::node_origin(n, (164.0, 146.0)))
                    .unwrap_or((164.0, 146.0));
                for i in 0..22usize {
                    let y = oy + 103.0 + i as f32 * 15.0;
                    if cursor.x >= ox + 15.0
                        && cursor.x <= ox + 105.0
                        && cursor.y >= y
                        && cursor.y <= y + 15.0
                    {
                        let idx = cat_base + i;
                        if let Some(c) = shop.categories.get(idx).cloned() {
                            if shop.category != c {
                                shop.category = c;
                                // C# `ResetPage`：换分类回第 1 页（分类滚动也归零）
                                shop.page = 0;
                                shop.qty = [1; 8];
                                cat_list.offset = 0;
                            }
                            tracing::info!(
                                "🛒 商城分类: {}",
                                if shop.category.is_empty() {
                                    "全部"
                                } else {
                                    &shop.category
                                }
                            );
                        }
                        break;
                    }
                }
            }
        }
    }
}

/// C# `ClientTextKeys.YouMustSelectPaymentType`（`Chinese.json` Text）
pub(crate) const GAME_SHOP_SELECT_PAYMENT: &str = "您必须选择一种支付方式！";

/// C# `ClientTextKeys.YouCantAffordSelectedItem`（`Chinese.json` Text）——
/// C# `BuyProduct` 的金币分支同样复用这条「点数不足」文案（`MirGameShopCell.cs:232`，原版怪癖）
pub(crate) const GAME_SHOP_CANT_AFFORD: &str = "您的点数不足，无法购买所选物品。";

/// 购买确认文案（C# `ConfirmPurchaseItemGold` / `ConfirmBuyItemCredits` 逐字：
/// 「您确定要购买 {1} 个 \n{0}（{3}）并花费 {2} 金币/点数吗？」，
/// `{0}`=物品名 `{1}`=数量 `{2}`=总价 `{3}`=`Item.Count`）
pub(crate) fn game_shop_confirm_text(
    is_gold: bool,
    name: &str,
    quantity: u32,
    cost: u32,
    item_count: i32,
) -> String {
    let currency = if is_gold { "金币" } else { "点数" };
    format!("您确定要购买 {quantity} 个 \n{name}（{item_count}）并花费 {cost} {currency}吗？")
}

/// 格内按钮打包（购买/数量±；全只读查询，控系统参数个数）
#[derive(SystemParam)]
struct ShopCellButtons<'w, 's> {
    buy: Query<'w, 's, (Entity, &'static Interaction, &'static GameShopCellBuy)>,
    qty_up: Query<'w, 's, (Entity, &'static Interaction, &'static GameShopCellQtyUp)>,
    qty_down: Query<'w, 's, (Entity, &'static Interaction, &'static GameShopCellQtyDown)>,
}

/// 付款方式 + 余额标签 + 数量± + 购买确认（C# `GameshopDialog.cs:185-210` 的复选框与
/// `MirGameShopCell.BuyProduct` :189-238 的整条购买流程；#2791 单元②）。
/// 独立系统：`game_shop_ui_system` 已 13 个参数（Bevy 上限 16）。
#[allow(clippy::too_many_arguments)]
fn game_shop_pay_system(
    mgr: Res<DialogManager>,
    mut shop: ResMut<GameShopState>,
    net: Res<NetConnection>,
    mut chat: ResMut<crate::game::chat::ChatState>,
    credit_q: Query<&crate::game::player_state::Credit, With<crate::actor::LocalPlayer>>,
    frames: Res<GameShopPayFrames>,
    keys: Res<ButtonInput<KeyCode>>,
    cell_btns: ShopCellButtons,
    mut checks: Query<
        (
            Entity,
            &Interaction,
            &mut ImageNode,
            Option<&GameShopPayGold>,
            Option<&GameShopPayCredit>,
        ),
        // 必须限定是付款复选框本身：否则「任何带图按钮」（购买/关闭/分类翻页…）都会被
        // 当成积分复选框（`gold/credit` 皆 None → `is_gold = false`），点购买键会误切付款方式
        Or<(With<GameShopPayGold>, With<GameShopPayCredit>)>,
    >,
    mut gold_label: Query<
        &mut Text,
        (
            With<GameShopGoldLabel>,
            Without<GameShopCreditLabel>,
            Without<GameShopConfirmText>,
        ),
    >,
    mut credit_label: Query<
        &mut Text,
        (
            With<GameShopCreditLabel>,
            Without<GameShopGoldLabel>,
            Without<GameShopConfirmText>,
        ),
    >,
    mut confirm_panel: Query<
        &mut Visibility,
        (With<GameShopConfirm>, Without<GameShopConfirmText>),
    >,
    mut confirm_text: Query<
        &mut Text,
        (
            With<GameShopConfirmText>,
            Without<GameShopGoldLabel>,
            Without<GameShopCreditLabel>,
        ),
    >,
    mut confirm_btns: Query<(
        Entity,
        &Interaction,
        Option<&GameShopConfirmYes>,
        Option<&GameShopConfirmNo>,
    )>,
    mut prev_inter: Local<std::collections::HashMap<Entity, Interaction>>,
) {
    fn edge(
        e: Entity,
        inter: &Interaction,
        prev: &mut std::collections::HashMap<Entity, Interaction>,
    ) -> bool {
        let was = prev.insert(e, *inter);
        *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
    }
    let open = mgr.is_open(DialogKind::GameShop);
    let credits = credit_q.single().map(|c| c.0).unwrap_or(0);
    // 复选框：点击即互斥切换（C# `PType_Clicked` → `RefreshPayType`），帧 `Prguse[2086]`/`[2087]`
    for (e, inter, mut image, gold, _credit) in &mut checks {
        let is_gold = gold.is_some();
        let checked = if is_gold {
            shop.pay_type == 1
        } else {
            shop.pay_type == 0
        };
        let target = if checked {
            frames.checked.clone()
        } else {
            frames.unchecked.clone()
        };
        if let Some(h) = target {
            if image.image != h {
                image.image = h;
            }
        }
        if open && edge(e, inter, &mut prev_inter) {
            shop.pay_type = if is_gold { 1 } else { 0 };
            tracing::info!("🛒 付款方式: {}", if is_gold { "金币" } else { "积分" });
        }
    }
    // 余额标签（C# `Process()`：`totalCredits/totalGold = ...ToString("###,###,##0")`）
    for mut t in &mut gold_label {
        let s = crate::game::hud::format_gold(shop.gold);
        if t.0 != s {
            t.0 = s;
        }
    }
    for mut t in &mut credit_label {
        let s = crate::game::hud::format_gold(credits);
        if t.0 != s {
            t.0 = s;
        }
    }
    // 格内数量±（C# `quantityUp/Down.Click`：Shift=±10；上限 99、有库存压库存，下限 1。
    // C# 另有按 `StackSize` 的 5 组封顶，本端无堆叠数数据，从略）
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let filtered = filter_shop_items(
        &shop.items,
        &shop.search,
        &shop.class_filter,
        &shop.category,
        &shop.section_filter,
        now_unix(),
    );
    for (e, inter, cell) in &cell_btns.qty_up {
        if !open || !edge(e, inter, &mut prev_inter) {
            continue;
        }
        let stock = cell_item(&shop, &filtered, cell.0)
            .map(|it| it.stock)
            .unwrap_or(0);
        shop.qty[cell.0] = qty_up(shop.qty[cell.0], stock, shift);
    }
    for (e, inter, cell) in &cell_btns.qty_down {
        if !open || !edge(e, inter, &mut prev_inter) {
            continue;
        }
        shop.qty[cell.0] = qty_down(shop.qty[cell.0], shift);
    }
    // 格内购买钮 → C# `MirGameShopCell.BuyProduct`（:189-238；空槽格整格隐藏，钮不可达）
    for (e, inter, cell) in &cell_btns.buy {
        if !open || !edge(e, inter, &mut prev_inter) {
            continue;
        }
        let Some(item) = cell_item(&shop, &filtered, cell.0).cloned() else {
            continue;
        };
        let quantity = shop.qty[cell.0] as u32;
        // C#：先看 Credit.Checked && CanBuyCredit，再看 Gold.Checked && CanBuyGold
        let p_type = if shop.pay_type == 0 && item.can_buy_credit {
            0
        } else if shop.pay_type == 1 && item.can_buy_gold {
            1
        } else {
            -1
        };
        if p_type == -1 {
            tracing::info!(
                "🛒 付款方式不可用（商品 #{} 金币可购={} 积分可购={}，当前={}）",
                item.item_index,
                item.can_buy_gold,
                item.can_buy_credit,
                if shop.pay_type == 1 {
                    "金币"
                } else {
                    "积分"
                }
            );
            chat.add_line(
                GAME_SHOP_SELECT_PAYMENT,
                crate::game::chat::chat_color(mir2_shared::enums::ChatType::System),
                crate::game::chat::ChatChannel::System,
            );
            continue;
        }
        let cost = if p_type == 0 {
            item.credit_price as u64 * quantity as u64
        } else {
            item.gold_price as u64 * quantity as u64
        };
        let balance = if p_type == 0 {
            credits as u64
        } else {
            shop.gold as u64
        };
        if cost > balance {
            tracing::info!(
                "🛒 余额不足（商品 #{} 需 {}，持有 {}）",
                item.item_index,
                cost,
                balance
            );
            chat.add_line(
                GAME_SHOP_CANT_AFFORD,
                crate::game::chat::chat_color(mir2_shared::enums::ChatType::System),
                crate::game::chat::ChatChannel::System,
            );
            continue;
        }
        shop.confirm_text =
            game_shop_confirm_text(p_type == 1, &item.name, quantity, cost as u32, item.count);
        shop.pending = Some(ShopPending {
            p_type,
            quantity,
            g_index: item.item_index,
        });
        tracing::info!(
            "🛒 确认购买 #{} {} x{} 付款方式={}",
            item.item_index,
            item.name,
            quantity,
            if p_type == 1 { "金币" } else { "积分" }
        );
    }
    // 确认框（C# `MirMessageBox`）：同一帧先算显隐再处理 Yes/No
    let confirm_visible = open && shop.pending.is_some();
    for mut vis in &mut confirm_panel {
        *vis = if confirm_visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    let text = if confirm_visible {
        shop.confirm_text.clone()
    } else {
        String::new()
    };
    for mut t in &mut confirm_text {
        if t.0 != text {
            t.0 = text.clone();
        }
    }
    for (e, inter, yes, no) in &mut confirm_btns {
        if !confirm_visible || !edge(e, inter, &mut prev_inter) {
            continue;
        }
        if yes.is_some() {
            if let Some(p) = shop.pending.take() {
                // 购买钮按下时已锁定 GIndex（C# 每格自带购买钮，无「选中行」回查）
                net.send_packet(&crate::network::GameshopBuyWire {
                    g_index: p.g_index,
                    quantity: p.quantity as u8,
                    p_type: p.p_type,
                });
                tracing::info!("🛒 购买商城商品 #{}（付款 {}）", p.g_index, p.p_type);
            }
        } else if no.is_some() {
            shop.pending = None;
        }
    }
}

/// 消费服务端商城事件（网络层只广播 ServerEvent；文案在此构造）
fn shop_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut shop: ResMut<GameShopState>,
    mut cat_scroll: Query<&mut UiScrollList, With<GameShopWidget>>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        match ev {
            ServerEvent::ShopCatalog { items, gold } => {
                shop.items = items
                    .iter()
                    .map(|it| ShopItem {
                        item_index: it.item_index,
                        image: it.image,
                        name: shop
                            .item_names
                            .get(&it.item_index)
                            .cloned()
                            .unwrap_or_default(),
                        gold_price: it.gold_price,
                        credit_price: it.credit_price,
                        category: it.category.clone(),
                        class: it.class.clone(),
                        stock: it.stock,
                        count: it.count,
                        deal: it.deal,
                        top_item: it.top_item,
                        date: it.date,
                        can_buy_gold: it.can_buy_gold,
                        can_buy_credit: it.can_buy_credit,
                    })
                    .collect();
                shop.gold = *gold;
                // #1334：分类列表 = 全部 + 服务端 category 去重保序（C# Filters）
                let mut cats: Vec<String> = vec![String::new()];
                for it in &shop.items {
                    if !it.category.is_empty() && !cats.iter().any(|c| c == &it.category) {
                        cats.push(it.category.clone());
                    }
                }
                shop.categories = cats;
                shop.category = String::new();
                // C# `UpdateShop` 重建全部格子 → 页码/选购数量/分类滚动归零
                shop.page = 0;
                shop.qty = [1; 8];
                for mut l in &mut cat_scroll {
                    l.offset = 0;
                }
            }
            ServerEvent::ShopStock { item_id, stock } => {
                // C# `S.GameShopStock`：就地更新格内 stockLabel，无额外提示
                if let Some(it) = shop.items.iter_mut().find(|i| i.item_index == *item_id) {
                    it.stock = *stock;
                }
            }
            ServerEvent::UserInformation { item_names, .. } => {
                for (idx, name) in item_names {
                    shop.item_names.insert(*idx, name.clone());
                }
            }
            ServerEvent::ItemInfoReceived {
                index,
                name,
                item_type,
                shape,
                required_gender,
                // #3120 ② 残余：商城不关心绑定位（邮件窗的 `MailState.item_bind` 才是消费者）
                ..
            } => {
                // P3-3：按需请求的回应——写进表，下一帧格子就会显示真名
                if remember_item_name(&mut shop.item_names, *index, name) {
                    shop.requested_item_info.remove(index);
                }
                // 试穿预览要用的 ItemInfo 三件套（类型 / shape / 需性别）
                shop.item_infos.insert(
                    *index,
                    ShopItemInfo {
                        item_type: *item_type,
                        shape: *shape,
                        required_gender: *required_gender,
                    },
                );
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(name: &str) -> ShopItem {
        ShopItem {
            image: 0,
            item_index: 0,
            name: name.to_string(),
            gold_price: 1,
            credit_price: 0,
            category: String::new(),
            // C# 目录里没有职业限制的商品一律 `Class = "All"`（库表实测 106/106 都是 "All"）
            class: "All".to_string(),
            stock: 1,
            count: 1,
            deal: false,
            top_item: false,
            date: 0,
            can_buy_gold: true,
            can_buy_credit: true,
        }
    }

    fn item_cat(name: &str, category: &str) -> ShopItem {
        ShopItem {
            image: 0,
            category: category.to_string(),
            ..item(name)
        }
    }

    /// 带职业/区段数据造物（门禁用）
    fn item_full(name: &str, cat: &str, class: &str, deal: bool, top: bool, date: i64) -> ShopItem {
        ShopItem {
            category: cat.to_string(),
            class: class.to_string(),
            deal,
            top_item: top,
            date,
            ..item(name)
        }
    }

    /// #2791 单元②：C# `GameshopDialog` 构造即 `PaymentTypeGold.Checked = true`（`:195`）
    #[test]
    fn shop_default_pay_type_is_gold() {
        assert_eq!(GameShopState::default().pay_type, 1);
    }

    /// #2968 实机回归 P0：`game_shop_ui_system` 必须在真实系统初始化时不 panic。
    /// B0001 只在单个 `ParamSet` 内部豁免——曾把 9 个 `&mut Text` 查询拆成
    /// `cell_set`/`page_set` 两个 ParamSet，两个并集里的无过滤 `&mut Text` 在
    /// system_meta 上相撞：一进游戏（`AppState::Game` 首次运行）即 panic 退出。
    /// 修复前本测试 panic（B0001），修复后通过。
    #[test]
    fn shop_ui_system_initializes_without_query_conflict() {
        use bevy::ecs::system::RunSystemOnce;

        let mut world = World::new();
        world.insert_resource(DialogManager::default());
        world.insert_resource(GameShopState::default());
        world.insert_resource(TextInputState::default());
        world.insert_resource(NetConnection::default());
        world.init_resource::<ButtonInput<MouseButton>>();
        // 系统初始化（B0001 在此触发）后走 `!open` 早退分支
        world.run_system_once(game_shop_ui_system).unwrap();
    }

    /// #2791 单元②：付款复选框查询必须限定标记——否则购买键等任意带图按钮会被当成
    /// 积分复选框（回归：实机点「购买」把付款方式切成积分，购买流程永不触发）
    #[test]
    fn shop_pay_system_ignores_other_image_buttons() {
        use bevy::ecs::system::RunSystemOnce;

        let mut world = World::new();
        world.insert_resource(GameShopState::default());
        world.insert_resource(NetConnection::default());
        world.init_resource::<crate::game::chat::ChatState>();
        world.init_resource::<GameShopPayFrames>();
        world.init_resource::<ButtonInput<KeyCode>>();
        let mut shop = DialogManager::default();
        shop.open(DialogKind::GameShop);
        world.insert_resource(shop);
        // 格内购买键（有 GameShopCellBuy 标记，不是付款复选框）；目录为空 → 点了不弹确认
        let buy = world
            .spawn((
                Button,
                GameShopCellBuy(0),
                Interaction::Pressed,
                Node::default(),
                ImageNode::default(),
            ))
            .id();
        // 金币/积分复选框
        let gold = world
            .spawn((
                Button,
                GameShopPayGold,
                Interaction::None,
                Node::default(),
                ImageNode::default(),
            ))
            .id();
        let credit = world
            .spawn((
                Button,
                GameShopPayCredit,
                Interaction::None,
                Node::default(),
                ImageNode::default(),
            ))
            .id();

        world
            .run_system_once(game_shop_pay_system)
            .expect("game_shop_pay_system 应成功");
        // 点购买键不应改付款方式（默认金币）；目录为空 → 不弹确认框
        assert_eq!(world.resource::<GameShopState>().pay_type, 1);
        assert_eq!(world.resource::<GameShopState>().pending, None);
        let _ = (buy, gold, credit);

        // 点积分复选框 → 切换为积分
        world.entity_mut(credit).insert(Interaction::Pressed);
        world.entity_mut(buy).insert(Interaction::None);
        world
            .run_system_once(game_shop_pay_system)
            .expect("game_shop_pay_system 应成功");
        assert_eq!(world.resource::<GameShopState>().pay_type, 0);
    }

    /// 格内购买钮：按下即锁定该格商品 GIndex 与选购数量进确认框
    /// （C# `MirGameShopCell.BuyItem.Click → BuyProduct`，无「选中行」语义）
    #[test]
    fn shop_cell_buy_locks_g_index_and_quantity() {
        use bevy::ecs::system::RunSystemOnce;

        let mut state = GameShopState {
            gold: 500,
            ..Default::default()
        };
        state.items.push(ShopItem {
            item_index: 42,
            name: "金创药(小)".to_string(),
            gold_price: 100,
            count: 1,
            can_buy_gold: true,
            ..Default::default()
        });
        state.qty[0] = 3;
        let mut world = World::new();
        world.insert_resource(state);
        world.insert_resource(NetConnection::default());
        world.init_resource::<crate::game::chat::ChatState>();
        world.init_resource::<GameShopPayFrames>();
        world.init_resource::<ButtonInput<KeyCode>>();
        let mut shop = DialogManager::default();
        shop.open(DialogKind::GameShop);
        world.insert_resource(shop);
        let buy = world
            .spawn((
                Button,
                GameShopCellBuy(0),
                Interaction::Pressed,
                Node::default(),
                ImageNode::default(),
            ))
            .id();
        world
            .run_system_once(game_shop_pay_system)
            .expect("game_shop_pay_system 应成功");
        let st = world.resource::<GameShopState>();
        assert_eq!(
            st.pending,
            Some(ShopPending {
                p_type: 1,
                quantity: 3,
                g_index: 42,
            })
        );
        assert_eq!(
            st.confirm_text,
            game_shop_confirm_text(true, "金创药(小)", 3, 300, 1)
        );
        let _ = buy;
    }

    /// 商品格坐标逐一对齐 C# `UpdateShop`：`i < 4 ? (152 + i*132, 115) : (152 + (i-4)*132, 275)`
    /// 门禁（2026-09-24，owner 队列「商城缺物品图标」）：格子图标的取图口径必须与 C# 一致——
    /// `Libraries.Items[Item.Info.Image]`，`image<=0` 视为无图（隐藏）。
    ///
    /// 阳性对照：把 `shop_cell_icon_index` 改成恒 `Some(image as usize)`（不排除 0/负数）→ 断言立即红。
    #[test]
    fn shop_cell_icon_index_matches_csharp_items_library() {
        assert_eq!(shop_cell_icon_index(2259), Some(2259)); // HoaSword 的 Image
        assert_eq!(shop_cell_icon_index(1), Some(1));
        assert_eq!(shop_cell_icon_index(0), None, "无图号（0）必须视为无图");
        assert_eq!(shop_cell_icon_index(-1), None, "负数同样无图");
    }

    #[test]
    fn shop_cell_positions_match_csharp() {
        let expect = [
            (152.0, 115.0),
            (284.0, 115.0),
            (416.0, 115.0),
            (548.0, 115.0),
            (152.0, 275.0),
            (284.0, 275.0),
            (416.0, 275.0),
            (548.0, 275.0),
        ];
        for (i, e) in expect.iter().enumerate() {
            assert_eq!(cell_pos(i), *e, "格 {i} 位置");
        }
    }

    /// 总页数 = C# `Ceiling(Count/8)`，< 1 归 1
    #[test]
    fn shop_max_page_ceils_by_8() {
        assert_eq!(max_page(0), 1);
        assert_eq!(max_page(1), 1);
        assert_eq!(max_page(8), 1);
        assert_eq!(max_page(9), 2);
        assert_eq!(max_page(16), 2);
        assert_eq!(max_page(17), 3);
    }

    /// 分类滚动边界 = C#（`DownButton`：`CStartIndex + 22 >= Count` 停；不足一屏不可滚）
    #[test]
    fn shop_category_scroll_bounds_match_csharp() {
        let mut l = UiScrollList {
            rect_rel: (11.0, 102.0, 125.0, 336.0),
            row_h: 15.0,
            visible: 22,
            total: 0,
            offset: 0,
            step: 1,
            track_rel: (120.0, 117.0, 16.0, 304.0),
            thumb: None,
            z: 30,
        };
        l.set_total(30);
        assert_eq!(
            l.max_offset(),
            8,
            "C# DownButton：CStartIndex+22 >= 30 停 → 上限 8"
        );
        l.offset = 99;
        l.set_total(30);
        assert_eq!(l.offset, 8, "超界由 set_total 夹紧");
        l.set_total(22);
        assert_eq!(l.max_offset(), 0, "恰好一屏不可滚");
        l.set_total(5);
        assert_eq!(l.max_offset(), 0, "不足一屏不可滚");
    }

    /// 数量加减（C# `quantityUp/Down.Click`：Shift=±10；上限 99、有库存压库存、下限 1）
    #[test]
    fn shop_qty_up_down_match_csharp() {
        // 加：无库存限制（stock=0 = 无限）
        assert_eq!(qty_up(1, 0, false), 2);
        assert_eq!(qty_up(1, 0, true), 11);
        assert_eq!(qty_up(98, 0, false), 99);
        assert_eq!(qty_up(95, 0, true), 99);
        // 加：有库存压库存
        assert_eq!(qty_up(3, 5, true), 5);
        assert_eq!(qty_up(3, 4, false), 4);
        // 减：下限 1
        assert_eq!(qty_down(5, false), 4);
        assert_eq!(qty_down(15, true), 5);
        assert_eq!(qty_down(1, false), 1);
        assert_eq!(qty_down(5, true), 1);
    }

    /// #2791 单元②：购买确认文案逐字对齐 C# `ConfirmPurchaseItemGold` /
    /// `ConfirmBuyItemCredits`（`Chinese.json`；`{0}`=物品名 `{1}`=数量 `{2}`=总价 `{3}`=`Item.Count`）
    #[test]
    fn shop_confirm_text_matches_csharp() {
        assert_eq!(
            game_shop_confirm_text(true, "金创药(小)", 1, 100, 10),
            "您确定要购买 1 个 \n金创药(小)（10）并花费 100 金币吗？"
        );
        assert_eq!(
            game_shop_confirm_text(false, "金创药(小)", 2, 120, 10),
            "您确定要购买 2 个 \n金创药(小)（10）并花费 120 点数吗？"
        );
    }

    #[test]
    fn shop_category_filters() {
        let items = vec![
            item_cat("金创药", "药品"),
            item_cat("太阳水", "药品"),
            item_cat("回城卷", "卷轴"),
        ];
        let f = |s: &str, c: &str| filter_shop_items(&items, s, "Show All", c, "Show All", 0).len();
        assert_eq!(f("", "药品"), 2);
        assert_eq!(f("", "卷轴"), 1);
        assert_eq!(f("", "不存在"), 0);
        // 分类 + 名称 叠加过滤
        assert_eq!(f("金创", "药品"), 1);
        assert_eq!(f("金创", "卷轴"), 0);
    }

    #[test]
    fn shop_search_filters_by_name() {
        let items = vec![item("金创药"), item("太阳水"), item("回城卷")];
        let f = |s: &str| filter_shop_items(&items, s, "Show All", "", "Show All", 0).len();
        assert_eq!(f(""), 3);
        assert_eq!(f("药"), 1);
        assert_eq!(f("水"), 1);
        assert_eq!(f("不存在"), 0);
        assert_eq!(f("  药  "), 1);
        assert_eq!(f("JINCHUANG"), 0);
    }

    #[test]
    fn shop_search_returns_original_indices() {
        let items = vec![
            item("金创药"),
            item("太阳水"),
            item("回城卷"),
            item("金创药·大"),
        ];
        let idx = filter_shop_items(&items, "金创药", "Show All", "", "Show All", 0);
        assert_eq!(idx, vec![0, 3]);
    }

    /// 门禁（owner 队列 `shop-class-tabs`）：**职业段**按 C# `ClassFilter` 语义筛选——
    /// `Class == ClassFilter` 或 `Class == "All"`，`"Show All"` 时全放行（`GameshopDialog.cs:720`）。
    ///
    /// 阳性对照：把 `it.class != "All"` 这一支删掉 → 第二条断言（通用商品在职业筛选下仍可见）立即红。
    #[test]
    fn shop_class_filter_matches_csharp() {
        let items = vec![
            item_full("战士刀", "武器", "Warrior", false, false, 0),
            item_full("法师杖", "武器", "Wizard", false, false, 0),
            item_full("通用药", "药品", "All", false, false, 0),
        ];
        let n = |cf: &str| filter_shop_items(&items, "", cf, "", "Show All", 0).len();
        assert_eq!(n("Show All"), 3, "不限职业 → 全部");
        assert_eq!(
            n("Wizard"),
            2,
            "法师筛选 = 法师专用 + 通用（Class==\"All\"）"
        );
        assert_eq!(
            filter_shop_items(&items, "", "Wizard", "", "Show All", 0),
            vec![1, 2],
            "结果按名称升序（码点序：法(U+6CD5) < 通(U+901A)；C# 用文化敏感比较，见函数注释）"
        );
        assert_eq!(n("Archer"), 1, "弓箭手筛选 = 只有通用商品");
    }

    /// 门禁：**区段段**按 C# `SectionFilter` 语义筛选——`TopItems`→`TopItem`、
    /// `DealItems`→`Deal`、`NewItems`→`Date > Now-7d`（`GameshopDialog.cs:723`）。
    ///
    /// 阳性对照：把 `"DealItems" => it.deal` 改成恒 `true` → 第三条断言立即红。
    #[test]
    fn shop_section_filter_matches_csharp() {
        const NOW: i64 = 1_800_000_000;
        let items = vec![
            item_full("普通", "药品", "All", false, false, 0),
            item_full("特价", "药品", "All", true, false, 0),
            item_full("置顶", "药品", "All", false, true, 0),
            item_full("新品", "药品", "All", false, false, NOW - 3600),
            item_full(
                "旧货",
                "药品",
                "All",
                false,
                false,
                NOW - NEW_ITEM_WINDOW_SECS - 3600,
            ),
        ];
        let n = |sf: &str| filter_shop_items(&items, "", "Show All", "", sf, NOW).len();
        assert_eq!(n("Show All"), 5);
        assert_eq!(n("DealItems"), 1, "只有特价");
        assert_eq!(n("TopItems"), 1, "只有置顶");
        assert_eq!(n("NewItems"), 1, "只有 7 天窗内上架的（旧货被挡）");
    }

    /// 门禁：结果**先排序再分页**——C# `filteredShop.OrderBy(FriendlyName)` 紧跟过滤
    /// （`GameshopDialog.cs:740`），所以页内容按名字升序。
    ///
    /// 阳性对照：把 `v.sort_by(...)` 删掉 → 本测试红（返回目录序）。
    #[test]
    fn shop_filter_sorts_by_name_like_csharp() {
        let items = vec![item("c"), item("a"), item("b")];
        let idx = filter_shop_items(&items, "", "Show All", "", "Show All", 0);
        assert_eq!(idx, vec![1, 2, 0], "按名称升序取原下标");
    }

    /// 门禁：`GetCategories()`（`GameshopDialog.cs:648-680`）重建分类表时必须**尊重当前
    /// 职业/区段**，并把分类筛选归零、页码归零，第 0 项是"全部"。
    ///
    /// 阳性对照：把 `rebuild_categories` 里的 `shop_item_matches` 判定改成恒 `true`
    /// → 第一条断言（法师筛选下不该出现"卷轴"）立即红。
    #[test]
    fn shop_rebuild_categories_respects_class_and_section() {
        let mut shop = GameShopState::default();
        shop.items = vec![
            item_full("战士刀", "武器", "Warrior", false, false, 0),
            item_full("法师杖", "武器", "Wizard", false, false, 0),
            item_full("战士卷", "卷轴", "Warrior", false, false, 0),
            item_full("法师药", "药品", "Wizard", false, false, 0),
            item_full("通用药", "药品", "All", false, false, 0),
        ];
        shop.class_filter = "Wizard".to_string();
        shop.category = "武器".to_string();
        shop.page = 3;
        rebuild_categories(&mut shop, 0);
        assert_eq!(
            shop.categories,
            vec![String::new(), "武器".to_string(), "药品".to_string()],
            "第 0 项=全部；法师筛选下没有战士专属的『卷轴』分类"
        );
        assert!(
            shop.category.is_empty(),
            "重建后 TypeFilter 归零（C# GetCategories）"
        );
        assert_eq!(shop.page, 0, "页码归零");

        // 区段切换同理：只有特价时，分类表只剩特价商品所在分类
        shop.items = vec![
            item_full("特价药", "药品", "All", true, false, 0),
            item_full("普通卷", "卷轴", "All", false, false, 0),
        ];
        shop.class_filter = "Show All".to_string();
        shop.section_filter = "DealItems".to_string();
        rebuild_categories(&mut shop, 0);
        assert_eq!(shop.categories, vec![String::new(), "药品".to_string()]);
    }

    /// 门禁：`MirClass` → C# 职业字符串（原版 `User.Class.ToString()`，`GameshopDialog.cs:508`）
    #[test]
    fn class_filter_name_matches_csharp() {
        assert_eq!(class_filter_name(0), "Warrior");
        assert_eq!(class_filter_name(1), "Wizard");
        assert_eq!(class_filter_name(2), "Taoist");
        assert_eq!(class_filter_name(3), "Assassin");
        assert_eq!(class_filter_name(4), "Archer");
        assert_eq!(class_filter_name(9), "Show All", "未知职业退回不限");
    }

    /// 门禁：三段筛选按钮的几何/帧表照 C# 常量（`GameshopDialog.cs:212-377`）——
    /// 位置、常态帧、选中帧一一对应；`New` 档在原版是 `Visible=false` 的死按钮。
    #[test]
    fn shop_filter_button_tables_match_csharp() {
        assert_eq!(
            CLASS_FILTERS,
            [
                ("Show All", 751, 752, 539.0),
                ("Warrior", 754, 755, 568.0),
                ("Assassin", 757, 758, 591.0),
                ("Taoist", 760, 761, 614.0),
                ("Wizard", 763, 764, 637.0),
                ("Archer", 766, 767, 660.0),
            ],
            "职业六档：C# (539,37)/(568+23i,38)，帧 751..768"
        );
        assert_eq!(CLASS_BTN_Y, [37.0, 38.0, 38.0, 38.0, 38.0, 38.0]);
        assert_eq!(
            SECTION_FILTERS,
            [
                ("Show All", 770, 771, 138.0),
                ("TopItems", 776, 777, 209.0),
                ("DealItems", 772, 773, 280.0),
                ("NewItems", 774, 775, 351.0),
            ],
            "区段四档：C# (138|209|280|351, 68)"
        );
        assert_eq!(SECTION_BTN_Y, 68.0);
    }
    /// P3-3 回归（2026-09-22）：商品名降级链必须是
    /// `it.name` → 本地物品名表 → （需要请求）→ `#id`。
    ///
    /// 阳性对照（落地时实做）：把中间那段查表删掉（直接回 `#id` + need_request=false）
    /// → 本测试立即红。
    #[test]

    /// P3-3 后半条回归（2026-09-22）：`NewItemInfo` 回包必须写进物品名表；
    /// 空名字不得覆盖已有名字。
    ///
    /// 阳性对照（落地时实做）：把 `remember_item_name` 改成直接 `return false;`（不写表）
    /// → 本测试立即红。
    #[test]
    fn new_item_info_reply_fills_item_names() {
        let mut names = std::collections::HashMap::new();
        assert!(remember_item_name(&mut names, 1268, "屠龙"));
        assert_eq!(names.get(&1268).map(String::as_str), Some("屠龙"));
        // 格子侧的降级链应当立刻吃到这个名字（不再回 #id）
        assert_eq!(
            resolve_shop_name("", &names, 1268),
            ("屠龙".to_string(), false)
        );
        // 空名字不得覆盖已有名字
        assert!(!remember_item_name(&mut names, 1268, ""));
        assert_eq!(names.get(&1268).map(String::as_str), Some("屠龙"));
    }
    fn resolve_shop_name_falls_back_in_order() {
        let mut names = std::collections::HashMap::new();
        names.insert(1269, "金创药（小）".to_string());

        // ① 条目自带名字：直接用，不需要请求
        assert_eq!(
            resolve_shop_name("屠龙", &names, 1268),
            ("屠龙".to_string(), false)
        );
        // ② 条目无名但本地表有：用本地表，不需要请求
        assert_eq!(
            resolve_shop_name("", &names, 1269),
            ("金创药（小）".to_string(), false)
        );
        // ③ 两处都没有：显示 #id 并**要求发起一次请求**（不是静默显示 #id 就算完）
        assert_eq!(
            resolve_shop_name("", &names, 1270),
            ("#1270".to_string(), true)
        );
        // ④ 表里存了空串同样视为「没有」，仍要请求
        names.insert(1271, String::new());
        assert_eq!(
            resolve_shop_name("", &names, 1271),
            ("#1271".to_string(), true)
        );
    }

    // -----------------------------------------------------------------------
    // 商城试穿预览（C# `GameShopViewer`，`Client/MirControls/MirGameShopCell.cs:312-595`）
    // 四条门禁：帧号 / 武器库 / 图层前后 / 类型门控与坐标。
    // 阳性对照做法见各条注释末尾（改回旧值或对调分支即应立红）。
    // -----------------------------------------------------------------------

    /// 帧号必须逐条等于 C#：身体层武器/护甲/变形 男 `32+6*(Dir-1)`／女 `840+6*(Dir-1)`
    /// （`:492/567/583`），坐骑 男 `448+8*(Dir-1)`／女 `1256+8*(Dir-1)`（`:466/468`）；
    /// 武器层与坐骑层 `32+6*(Dir-1)`／`32+8*(Dir-1)`（`:510/474`）；`Direction` 初值 6（`:318`）。
    /// 阳性对照：把 `6` 改成 `8`（或把男女基址对调）后本测试即红。
    #[test]
    fn viewer_frame_indices_match_csharp() {
        for d in 1..=8u8 {
            let i = (d - 1) as usize;
            assert_eq!(
                viewer_body_index(false, d, false),
                32 + 6 * i,
                "male body dir={d}"
            );
            assert_eq!(
                viewer_body_index(true, d, false),
                840 + 6 * i,
                "female body dir={d}"
            );
            assert_eq!(
                viewer_body_index(false, d, true),
                448 + 8 * i,
                "male mount-body dir={d}"
            );
            assert_eq!(
                viewer_body_index(true, d, true),
                1256 + 8 * i,
                "female mount-body dir={d}"
            );
            assert_eq!(viewer_weapon_index(d), 32 + 6 * i, "weapon layer dir={d}");
            assert_eq!(viewer_mount_index(d), 32 + 8 * i, "mount layer dir={d}");
        }
        // 越界方向按 1..8 夹取（C# 的 Direction 由按钮维护在 1..8 内）
        assert_eq!(viewer_body_index(false, 0, false), 32);
        assert_eq!(viewer_body_index(false, 9, false), 32 + 6 * 7);
        assert_eq!(VIEWER_DIRECTION0, 6);
    }

    /// 武器层取哪个库 + 索引，照抄 `DrawWeapon`（`:505-547`）：
    /// `100..=199` → `AWeapon/{shape-100} R`+`L` 双层；`>=200` → `ARWeapon/{shape-200}`；
    /// `<100` → `CWeapon/{shape}`。
    /// 阳性对照：把 `>=200` 分支删掉（落进 CWeapons）或把区间边界写成 `100..199` 即红。
    #[test]
    fn viewer_weapon_libs_match_csharp() {
        use ArrayLibType::*;
        assert_eq!(viewer_weapon_libs(0), (CWeapons, 0, None));
        assert_eq!(viewer_weapon_libs(19), (CWeapons, 19, None));
        assert_eq!(viewer_weapon_libs(99), (CWeapons, 99, None));
        assert_eq!(
            viewer_weapon_libs(100),
            (AWeaponsR, 0, Some((AWeaponsL, 0)))
        );
        assert_eq!(
            viewer_weapon_libs(150),
            (AWeaponsR, 50, Some((AWeaponsL, 50)))
        );
        assert_eq!(
            viewer_weapon_libs(199),
            (AWeaponsR, 99, Some((AWeaponsL, 99)))
        );
        assert_eq!(viewer_weapon_libs(200), (ARWeapons, 0, None));
        assert_eq!(viewer_weapon_libs(250), (ARWeapons, 50, None));
    }

    /// 复刻 C# `GameShopViewer` 的 `BringToFront()` 调用序（`DrawWeapon`，`:499-545`）。
    /// 子控件创建序：CloseButton → WeaponImage → WeaponImage2 → MountImage → PreviewImage
    /// （`:333-395`），Crystal 里**后创建者在上**，故初始层序（前→后）= body > weapon2 > weapon1。
    /// 返回**可见层**的前→后顺序（非刺客 shape 只有 body 与 weapon1）。
    fn csharp_weapon_layer_order(direction: u8, shape: i16) -> Vec<&'static str> {
        fn front(stack: &mut Vec<&'static str>, what: &'static str) {
            stack.retain(|x| *x != what);
            stack.insert(0, what);
        }
        let d = direction.clamp(1, 8);
        let mut stack: Vec<&'static str> = vec!["body", "weapon2", "weapon1"];

        // `MirGameShopCell.cs:506-509`
        if d > 1 && d < 5 {
            front(&mut stack, "weapon1");
        } else {
            front(&mut stack, "body");
        }
        // `:512-528`（刺客双武器层）
        if (100..=199).contains(&shape) {
            if (2..=3).contains(&d) {
                front(&mut stack, "weapon2");
                front(&mut stack, "body");
                front(&mut stack, "weapon1");
            } else if d == 7 || d == 8 {
                front(&mut stack, "weapon1");
                front(&mut stack, "body");
                front(&mut stack, "weapon2");
            } else {
                front(&mut stack, "weapon1");
                front(&mut stack, "body");
            }
        }
        // `:537-544`（弓箭手武器在 6..8 方向压到最前）
        if shape >= 200 && (6..=8).contains(&d) {
            front(&mut stack, "body");
            front(&mut stack, "weapon1");
        }
        stack.retain(|x| *x != "weapon2" || (100..=199).contains(&shape));
        stack
    }

    /// 图层前后必须与 C# 的 `BringToFront()` 调用序一致（数值大者在前）。
    /// 阳性对照：把双武器层 `7..=8` 分支改回 `(3,4,2)`（即只按调用先后排、漏掉 body 压中间）即红。
    #[test]
    fn viewer_layer_z_matches_csharp_bring_to_front_order() {
        for d in 1..=8u8 {
            for shape in [0i16, 49, 99, 100, 150, 199, 200, 250] {
                let expected = csharp_weapon_layer_order(d, shape);
                let (z_weapon1, z_weapon2, z_body) =
                    viewer_layer_z(d, shape, (100..=199).contains(&shape));
                let z_of = |name: &str| match name {
                    "weapon1" => z_weapon1,
                    "weapon2" => z_weapon2,
                    _ => z_body,
                };
                for i in 0..expected.len() {
                    for j in (i + 1)..expected.len() {
                        assert!(
                            z_of(expected[i]) > z_of(expected[j]),
                            "dir={d} shape={shape}: C# 层序 {expected:?} 要求 {} 在 {} 之前，\
                             但 z=(w1={z_weapon1}, w2={z_weapon2}, body={z_body})",
                            expected[i],
                            expected[j]
                        );
                    }
                }
            }
        }
    }

    /// 类型门控与坐标必须等于 C#：可试穿 `Type ∈ {Weapon, Armour, Mount, Transform}`（`:278`），
    /// 可试穿时购买钮右移到 `(75,122)`、否则 `(42,122)`（`:281`）；面板按格子左右半选边
    /// `X < 350 ? (416,115) : (151,115)`（`:129`）；转身 1..8 环绕；面板/图层/按钮坐标与
    /// `AnimationDelay=150`（`:335/353/404/424/355`）。
    /// 阳性对照：删掉 `Transform` 分支、或把 `viewer_side` 的阈值写成 `<= 350` 即红。
    #[test]
    fn viewer_gate_and_layout_match_csharp() {
        for t in [
            ITEM_TYPE_WEAPON,
            ITEM_TYPE_ARMOUR,
            ITEM_TYPE_MOUNT,
            ITEM_TYPE_TRANSFORM,
        ] {
            assert!(viewer_previewable(t), "type {t} 应可试穿");
        }
        for t in [0u8, 1, 2, 3, 6, 7, 20, 21, 23, 39, 41, 255] {
            assert!(!viewer_previewable(t), "type {t} 不应可试穿");
        }
        assert_eq!(viewer_buy_x(true), 75.0);
        assert_eq!(viewer_buy_x(false), 42.0);
        assert_eq!(viewer_side(0.0), VIEWER_RIGHT_SIDE);
        assert_eq!(viewer_side(349.9), VIEWER_RIGHT_SIDE);
        assert_eq!(viewer_side(350.0), VIEWER_LEFT_SIDE);
        assert_eq!(viewer_side(1024.0), VIEWER_LEFT_SIDE);
        assert_eq!(viewer_turn(6, true), 7);
        assert_eq!(viewer_turn(8, true), 1);
        assert_eq!(viewer_turn(1, false), 8);
        assert_eq!(viewer_turn(6, false), 5);
        assert_eq!(VIEWER_PANEL, (LibraryName::Title, 785));
        assert_eq!(VIEWER_LAYER_POS, (105.0, 160.0));
        assert_eq!(VIEWER_CLOSE_POS, (230.0, 8.0));
        assert_eq!(VIEWER_PREV_POS, (81.0, 282.0));
        assert_eq!(VIEWER_NEXT_POS, (160.0, 282.0));
        assert_eq!(VIEWER_ANIM_MS, 150.0);
    }
}

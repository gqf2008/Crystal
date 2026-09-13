// ============================================================================
// 市场/交易所对话框（M34）
// 参考：C# NPC 交易所（NPCDialogs.cs Consign 面板）+ ServerRust market.rs
// 网络（ServerRust gate 实际 wire，与 SharedRust 客户端包结构不一致，手动构造）：
//   C: MarketRefresh(空) / MarketSearch[u32 item_index] / MarketPage[u32]
//      MarketBuy[u32 listing_id] / MarketGetBack[u32 listing_id]
//      MarketSellNow[u32 uid][u32 price] / ConsignItem[u32 uid][u32 price][u32 0]
//   S: NPCMarket[页数] / NPCMarketPage[商品列表] / ConsignItem[uid u64][ok u8]
//      MarketSuccess[消息] / MarketFail[原因 u8]
// ============================================================================

use std::collections::HashMap;

use bevy::prelude::*;
use mir2_shared::enums::{ItemType, MarketPanelType};

use crate::actor::LocalPlayer;
use crate::game::dialogs::market_filter::{
    self, MarketFilterSprites,
};
use crate::game::dialogs::inventory::{InvClickState, InvItem, InvLockReason, InvLockedSlots};
use crate::game::dialogs::text_input::TextInputState;
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::game::player_state::Inventory;
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{shared_cjk_font, ui_image, UiCjkFont, UiFont, UiImageCache};
use crate::ui::gray::UiGray;
use crate::ui::theme::{
    load_lib_image, spawn_container, spawn_icon_button, spawn_item_cell_ui, spawn_label,
    spawn_label_center, spawn_panel, spawn_scroll_bar_ui, ImageButton, UiItemCellData,
    UiItemCellIcon, UiScrollList,
};

/// 市场商品条目（NPCMarketPage 写入）
#[derive(Debug, Clone, Default)]
pub struct MarketItem {
    pub auction_id: u64,
    pub unique_id: u64,
    pub name: String,
    pub item_index: i32,
    /// 物品图标帧（C# `Listing.Item.Info.Image`；0 或 count==0 时用 `Prguse[540]` 占位）
    pub image: u16,
    /// 物品品质（C# `Item.Info.Grade` → `GradeNameColor`）
    pub grade: u8,
    pub count: u16,
    pub seller: String,
    pub price: u32,
    /// 0=寄售 1=拍卖（C# MarketItemType）
    pub item_type: u8,
    /// 拍卖当前最高出价（寄售=0）
    pub current_bid: u32,
    /// C# `ClientAuction.ConsignmentDate`（Unix 秒；到期列显示 + `ConsignmentLength` 天）
    pub consignment_date: i64,
}

/// 市场状态
#[derive(Resource)]
pub struct MarketState {
    /// 已累积页的条目（C# `TrustMerchantDialog.Listings`：`NPCMarket` 赋值、`NPCMarketPage` `AddRange`）
    pub listings: Vec<MarketItem>,
    /// 已累积的页数（C# `(Listings.Count - 1) / 10`；0 = 尚未收到任何页）
    pub loaded_pages: usize,
    /// 最近一次 `C.MarketPage` 请求的页号（服务器回包不带页号，靠它定位累积位置）
    pub pending_page: Option<usize>,
    pub pages: usize,
    pub page: usize,
    /// 选中的列表行（购买/取回/立即售出目标）
    pub selected: Option<usize>,
    /// 最近寄售成功的物品 uid
    pub consign_ok: Option<u64>,
    /// 最近操作结果消息（MarketSuccess/Fail 或本地提示）
    pub message: String,
    /// 物品名缓存（item_index → name，来自 UserInformation）
    pub item_names: HashMap<i32, String>,
    /// 当前页签（C# `TrustMerchantDialog.MarketType`）：Market/Consign/Auction/GameShop
    pub panel: MarketPanelType,
    /// 筛选树选中主项（C# `SelectedIndex`，默认 0 = 显示所有物品）
    pub filter_index: i32,
    /// 筛选树选中子项（C# `SelectedSubIndex`，None = -1）
    pub filter_sub_index: Option<i32>,
    /// 筛选树滚动偏移（C# `Skip`）
    pub filter_skip: usize,
    /// 寄售/拍卖目标物品（C# `SellItemSlot`；点 ItemCell 从背包选中物放入）
    pub consign_item: Option<InvItem>,
    /// #2742：寄售目标物品的背包槽（C# `tempCell`；放入即 `Locked = true`，
    /// 换物/切页签/关窗/`S.ConsignItem` 回包时解锁）
    pub consign_slot: Option<usize>,
    /// 价格排序三态（C# `TrustMerchantDialog.PriceFilter`）
    pub price_filter: MarketPriceFilter,
}

impl Default for MarketState {
    fn default() -> Self {
        Self {
            listings: Vec::new(),
            loaded_pages: 0,
            pending_page: None,
            pages: 0,
            page: 0,
            selected: None,
            consign_ok: None,
            message: String::new(),
            item_names: HashMap::new(),
            // C# 进入市场页签即 `TMerchantDialog(Market)` → `DrawFilters(0, -1)`
            panel: MarketPanelType::Market,
            filter_index: 0,
            filter_sub_index: None,
            filter_skip: 0,
            consign_item: None,
            consign_slot: None,
            price_filter: MarketPriceFilter::Normal,
        }
    }
}

#[derive(Component)]
pub struct MarketWidget;

#[derive(Component)]
pub struct MarketClose;

/// 页签按钮（C# `TrustMerchantDialog`：Market/Consignment/Auction/GameShop）
#[derive(Component)]
pub struct MarketTabBtn(pub &'static str);

/// C# `TrustMerchantDialog`（`TrustMerchantDialog.cs:86-500`）面板与控件锚点
pub const TM_PANEL_W: f32 = 492.0;
pub const TM_PANEL_H: f32 = 478.0;
/// C# 未设 `Location`（MirControl 默认 (0,0)）
const TM_POS: (f32, f32) = (0.0, 0.0);
const TM_CLOSE: (f32, f32) = (465.0, 3.0);
/// (marker, x, y, normal/hover 帧, pressed 帧)
const TM_TABS: [(&str, f32, f32, usize, usize); 4] = [
    ("market", 9.0, 35.0, 789, 788),
    ("consign", 104.0, 35.0, 791, 790),
    ("auction", 199.0, 35.0, 817, 816),
    ("game_shop", 389.0, 35.0, 819, 818),
];
/// C# 列表区（左侧 x≤120 为筛选树，行高 18）
const TM_LIST_X: f32 = 130.0;
const TM_LIST_Y: f32 = 60.0;
/// C# 底部操作栏：搜索框 (11,452) 110x18、Find (124,448)、Refresh (320,448)、Buy (380,448)
const TM_SEARCH_POS: (f32, f32) = (11.0, 452.0);
const TM_FIND_POS: (f32, f32) = (124.0, 448.0);
const TM_REFRESH_POS: (f32, f32) = (320.0, 448.0);
const TM_BUY_POS: (f32, f32) = (380.0, 448.0);
/// C# 翻页：Back (251,419)、Next (320,419)、PageLabel (260,419) 70x18
const TM_BACK_POS: (f32, f32) = (251.0, 419.0);
const TM_NEXT_POS: (f32, f32) = (320.0, 419.0);
const TM_PAGE_POS: (f32, f32) = (260.0, 419.0);

#[derive(Component)]
pub struct MarketRefreshBtn;

#[derive(Component)]
pub struct MarketSearchBtn;

#[derive(Component)]
pub struct MarketBuyBtn;

#[derive(Component)]
pub struct MarketSellNowBtn;

#[derive(Component)]
pub struct MarketPrevBtn;

#[derive(Component)]
pub struct MarketNextBtn;

#[derive(Component)]
pub struct MarketLine(usize);

/// 搜索输入框（TextInput id 5）/ 寄售价格（id 6）
#[derive(Component)]
pub struct MarketSearchField;

#[derive(Component)]
pub struct MarketPriceField;

// ===== 寄售 / 拍卖页签面板（C# `TrustMerchantDialog` 的 787 面板，TrustMerchantDialog.cs:171-590）=====

/// 页签可见性（C# `TMerchantDialog(type)` 的 Visible 开关汇总，:1117-1314）
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum MarketForPanel {
    /// 寄售/拍卖：HelpLabel / PriceTextBox / ItemCell / SellItemButton
    ConsignOrAuction,
    /// 仅寄售：`CollectSoldButton`
    ConsignOnly,
    /// 仅拍卖：`SellNowButton`
    AuctionOnly,
    /// 仅市场：`MailButton`（C# 其余三个页签都 `Visible = false`）
    MarketOnly,
}

/// 列表表头标签（C# `TitleSalePriceLabel`/`TitleSellLabel`/`TitleItemLabel`/
/// `TitlePriceLabel`/`TitleExpiryLabel`，:610-669；文案随页签变化 :1144-1300）
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum MarketHeader {
    SalePrice,
    Sell,
    Item,
    Price,
    Expiry,
}

/// 表头锚点（C# `Location`/`Size`：居中绘制 → Bevy 用 `spawn_label_center`）
pub const TM_HEADERS: [(MarketHeader, f32, f32, f32); 5] = [
    (MarketHeader::SalePrice, 15.0, 142.0, 100.0),
    (MarketHeader::Sell, 10.0, 60.0, 110.0),
    (MarketHeader::Item, 127.0, 60.0, 166.0),
    (MarketHeader::Price, 295.0, 60.0, 88.0),
    (MarketHeader::Expiry, 384.0, 60.0, 98.0),
];

/// 表头文案（C# `TMerchantDialog(type)` 的 `Text = GetLocalization(...)`）
pub fn header_text(kind: MarketHeader, panel: MarketPanelType) -> &'static str {
    use MarketHeader::*;
    match (kind, panel) {
        (SalePrice, MarketPanelType::Consign) => "出售价格",
        (SalePrice, MarketPanelType::Auction) => "起始出价",
        (SalePrice, _) => "",
        (Sell, _) => "出售物品",
        (Item, _) => "物品",
        (Price, MarketPanelType::Market) => "价格 / 出价",
        (Price, MarketPanelType::Consign) => "价格",
        (Price, MarketPanelType::Auction) => "最高出价",
        (Price, _) => "价格",
        (Expiry, MarketPanelType::Market) => "卖家 / 到期",
        (Expiry, MarketPanelType::Consign) => "到期",
        (Expiry, MarketPanelType::Auction) => "结束日期",
        (Expiry, _) => "",
    }
}

/// C# `HelpLabel`（:171-180）@(8,237) 115x205
pub const TM_HELP_POS: (f32, f32) = (8.0, 237.0);
pub const TM_HELP_W: f32 = 115.0;
pub const TM_HELP_H: f32 = 205.0;
/// C# `ItemCell`（:544-553）@(47,104)，`MirItemCell` 默认尺寸 36x32
pub const TM_CONSIGN_CELL_POS: (f32, f32) = (47.0, 104.0);
pub const TM_CONSIGN_CELL_W: f32 = 36.0;
pub const TM_CONSIGN_CELL_H: f32 = 32.0;
/// C# `PriceTextBox`（:556-564）@(15,165) 100x18
pub const TM_PRICE_POS: (f32, f32) = (15.0, 165.0);
pub const TM_PRICE_W: f32 = 100.0;
pub const TM_PRICE_H: f32 = 18.0;
/// C# `SellItemButton`（:568-579）`Title[700..702]` 52x25 @(39,188)
pub const TM_SELL_ITEM_POS: (f32, f32) = (39.0, 188.0);
pub const TM_SELL_BTN_W: f32 = 52.0;
pub const TM_SELL_BTN_H: f32 = 25.0;
/// C# `CollectSoldButton`（:460-469）`Title[680..682]` 72x25 @(300,448)
pub const TM_COLLECT_SOLD_POS: (f32, f32) = (300.0, 448.0);
pub const TM_COLLECT_BTN_W: f32 = 72.0;
pub const TM_COLLECT_BTN_H: f32 = 25.0;
/// C# `SellNowButton`（:442-451）`Title[700..702]` 52x25 @(324,448)
pub const TM_SELL_NOW_POS: (f32, f32) = (324.0, 448.0);
/// C# `BuyButton` 用户模式精灵 `Title[706..708]`（:1171-1175/1222-1226，同为 84x25 @(380,448)）
pub const TM_BUY_USER_FRAMES: (usize, usize, usize) = (706, 707, 708);
/// C# `BuyButton` 市场/商城精灵 `Title[703..705]`（:1130-1131/1273-1274）
pub const TM_BUY_MARKET_FRAMES: (usize, usize, usize) = (703, 704, 705);

/// 载入一组三帧按钮精灵（任一缺失返回 `None`）
pub fn load_buy_frames(
    libs: &mut GameLibraries,
    images: &mut Assets<Image>,
    (n, h, p): (usize, usize, usize),
) -> Option<(Handle<Image>, Handle<Image>, Handle<Image>)> {
    Some((
        load_lib_image(libs, images, LibraryName::Title, n)?,
        load_lib_image(libs, images, LibraryName::Title, h)?,
        load_lib_image(libs, images, LibraryName::Title, p)?,
    ))
}
/// C# `Globals`：寄售 5000..50,000,000、拍卖起始价 0..50,000（Shared/Globals.cs:44-48）
pub const TM_MIN_CONSIGN_PRICE: u32 = 5000;
pub const TM_MAX_CONSIGN_PRICE: u32 = 50_000_000;
pub const TM_MAX_STARTING_BID: u32 = 50_000;
/// C# `Globals.ConsignmentLength`（天，到期列 = 寄售日期 + 7 天）
pub const TM_CONSIGNMENT_LENGTH_DAYS: i64 = 7;

// ===== 价格排序（C# `MarketPriceFilter` + `PriceFilterIcon`）与 Mail 按钮 =====

/// C# `PriceFilterIcon` 位置 = `(TitlePriceLabel.X + W - 12, Y + (H - 14)/2 + 2)` = (371, 65)
pub const TM_PRICE_ICON_POS: (f32, f32) = (371.0, 65.0);
/// `Prguse2[925]`（低价）与 `[926]`（高价）实测 12x11
pub const TM_PRICE_ICON_LOW: usize = 925;
pub const TM_PRICE_ICON_HIGH: usize = 926;
pub const TM_PRICE_ICON_W: f32 = 12.0;
pub const TM_PRICE_ICON_H: f32 = 11.0;
/// 价格表头点击层（C# `TitlePriceLabel.Click` 的 (295,60) 88x21 命中区）
pub const TM_PRICE_HEADER_POS: (f32, f32) = (295.0, 60.0);
pub const TM_PRICE_HEADER_W: f32 = 88.0;
pub const TM_PRICE_HEADER_H: f32 = 21.0;
/// C# `MailButton`：`Prguse[437..439]` 28x25 @(350,448)
pub const TM_MAIL_POS: (f32, f32) = (350.0, 448.0);
pub const TM_MAIL_W: f32 = 28.0;
pub const TM_MAIL_H: f32 = 25.0;

/// C# `MarketPriceFilter`（价格排序三态，Shared/Enums.cs:61）
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum MarketPriceFilter {
    #[default]
    Normal,
    Low,
    High,
}

impl MarketPriceFilter {
    /// C# `CyclePriceFilter()`：Normal → Low → High → Normal
    pub fn next(self) -> Self {
        match self {
            Self::Normal => Self::Low,
            Self::Low => Self::High,
            Self::High => Self::Normal,
        }
    }

    /// C# `UpdatePriceFilterIcon()`：Normal 隐藏，Low 用 `Prguse2[925]`，High 用 `[926]`
    pub fn icon_frame(self) -> Option<usize> {
        match self {
            Self::Normal => None,
            Self::Low => Some(TM_PRICE_ICON_LOW),
            Self::High => Some(TM_PRICE_ICON_HIGH),
        }
    }
}

/// C# `GetOrderedListings()`：Normal 保持服务器顺序，Low/High 按价格升/降序（稳定排序，
/// 同价保持服务器顺序；C# `OrderBy` 亦为稳定排序，`?? 0` 对应价格取 0）
pub fn display_order(prices: &[u32], filter: MarketPriceFilter) -> Vec<usize> {
    let mut order: Vec<usize> = (0..prices.len()).collect();
    match filter {
        MarketPriceFilter::Normal => {}
        MarketPriceFilter::Low => order.sort_by_key(|i| prices[*i]),
        MarketPriceFilter::High => order.sort_by_key(|i| std::cmp::Reverse(prices[*i])),
    }
    order
}

/// C# `MailButton.Click` 正文（`ClientTextKeys.InterestedInPurchase` =
/// 「我有意购买{0}，价格为{1}。」，价格不带千分位）
pub fn market_mail_message(item_name: &str, price: u32) -> String {
    format!("我有意购买{}，价格为{}。", item_name, price)
}

/// 价格排序触发区（C# `TitlePriceLabel.Click`）
#[derive(Component)]
pub struct MarketPriceFilterBtn;

/// 价格排序图标（C# `PriceFilterIcon`）
#[derive(Component)]
pub struct MarketPriceFilterIcon;

/// 写邮件按钮（C# `MailButton`）
#[derive(Component)]
pub struct MarketMailBtn;

/// C# `TMerchantDialog(type)` 的页签控件显隐（:1117-1314）：
/// 寄售/拍卖面板组、`CollectSoldButton`（仅寄售）、`SellNowButton`（仅拍卖）、`MailButton`（仅市场）
pub fn panel_part_visible(kind: MarketForPanel, panel: MarketPanelType) -> bool {
    match kind {
        MarketForPanel::ConsignOrAuction => {
            matches!(panel, MarketPanelType::Consign | MarketPanelType::Auction)
        }
        MarketForPanel::ConsignOnly => panel == MarketPanelType::Consign,
        MarketForPanel::AuctionOnly => panel == MarketPanelType::Auction,
        MarketForPanel::MarketOnly => panel == MarketPanelType::Market,
    }
}

/// 当前页第 `slot` 行对应的 `listings` 下标（按价格排序重排后）。
///
/// C# `UpdateInterface`（TrustMerchantDialog.cs:983-996）用 `orderedListings[Page * 10 + i]`
/// —— 索引打在**累积后的全量列表**上，故价格排序是跨页的；`listings` 即该累积列表。
pub fn row_listing_index(market: &MarketState, slot: usize) -> Option<usize> {
    let prices: Vec<u32> = market.listings.iter().map(|l| l.price).collect();
    display_order(&prices, market.price_filter)
        .get(market.page * 10 + slot)
        .copied()
}

/// 累积一页商品（C# `GameScene.NPCMarketPage`，GameScene.cs:5644-5654）：
/// `Listings.AddRange(p.Listings)` 后 `Page = (Listings.Count - 1) / 10`。
///
/// 服务器回包不带页号（`S.NPCMarketPage` 只有 listings），故由调用方给出该页页号：
/// - `page == 0`：新一轮搜索结果（`NPCMarket` / 搜索 / 刷新）→ 替换累积并复位选中；
/// - `page <= loaded_pages`：续接/覆盖已加载前缀内的该页（C# 顺序累积等价行为）；
/// - `page > loaded_pages`：缺页（正常交互不会发生，翻页只请求已加载前缀的下一页）→ 忽略。
pub fn accumulate_market_page(market: &mut MarketState, page: usize, items: Vec<MarketItem>) {
    if page == 0 {
        market.listings.clear();
        market.loaded_pages = 0;
        market.selected = None;
    } else if page > market.loaded_pages {
        tracing::warn!(
            "🏪 忽略缺页回包 page={}（已加载 {} 页）",
            page,
            market.loaded_pages
        );
        return;
    }
    let keep = (page * 10).min(market.listings.len());
    market.listings.truncate(keep);
    market.listings.extend(items);
    market.loaded_pages = page + 1;
    // C# `Page = (Listings.Count - 1) / 10`：顺序累积时即刚到的这一页
    market.page = page;
    market.pending_page = None;
}

/// C# `BackButton.Click`（:192-198）：`if (Page <= 0) return;` 后 `Page--` —— 已在
/// 累积列表里，纯本地翻页，不请求服务器。
pub fn back_page_action(page: usize) -> Option<usize> {
    (page > 0).then(|| page - 1)
}

/// C# `NextButton.Click`（:210-222）：`Page >= PageCount - 1` 不动作；
/// `Page < (Listings.Count - 1) / 10` → 本地翻页；否则发 `C.MarketPage{Page+1}`。
/// 返回 `(目标页, 是否需请求服务器)`。
pub fn next_page_action(
    page: usize,
    loaded_pages: usize,
    total_pages: usize,
) -> Option<(usize, bool)> {
    let next = page + 1;
    if next >= total_pages.max(1) {
        return None;
    }
    Some((next, next >= loaded_pages))
}

/// 发 `C.MarketPage{page}` 并登记待回包的页号（`pending_page` 只登记最早一次未回包的请求：
/// 服务端对翻页有 500ms 节流会静默丢包，登记最早页可让后续重试自愈）。
pub fn request_market_page(
    market: &mut MarketState,
    net: &crate::network::NetConnection,
    page: usize,
) {
    market.pending_page.get_or_insert(page);
    net.send_packet(&crate::network::MarketPageWire { page: page as u32 });
}

// ===== 买/取回确认框（C# `MirMessageBox` YesNo，TrustMerchantDialog.cs:360-440）=====

/// C# `MirMessageBox` YesNo：背景 `Prguse[360]` 456x190 居中 @(284,289)、
/// 文本 (35,35) 390x110、Yes `Title[206..208]` @(260,157)、No `Title[210..212]` @(360,157)
pub const TM_CONFIRM_POS: (f32, f32) = (284.0, 289.0);
pub const TM_CONFIRM_W: f32 = 456.0;
pub const TM_CONFIRM_H: f32 = 190.0;
pub const TM_CONFIRM_TEXT_POS: (f32, f32) = (35.0, 35.0);
pub const TM_CONFIRM_YES_POS: (f32, f32) = (260.0, 157.0);
pub const TM_CONFIRM_NO_POS: (f32, f32) = (360.0, 157.0);
pub const TM_CONFIRM_BTN_W: f32 = 76.0;
pub const TM_CONFIRM_BTN_H: f32 = 25.0;

/// 确认后要执行的动作
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MarketConfirmAction {
    /// `C.MarketBuy{AuctionID, BidPrice}`（寄售/商城一口价 → bid 0）
    Buy { auction_id: u64, bid_price: u32 },
    /// `C.MarketGetBack{AuctionID}`（取回物品/领取金币）
    GetBack { auction_id: u64 },
}

/// C# `BuyButton.Click` 的分支结果
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum MarketBuyOutcome {
    /// 直接执行（C# 未弹确认框的分支）
    Direct(MarketConfirmAction),
    /// 弹 YesNo 确认（文案 + Yes 时执行的动作）
    Confirm(String, MarketConfirmAction),
    /// C# 拍卖分支：先弹 `MirAmountBox`（`BidAmount`，带物品图标、默认 `Price + 1`），
    /// 金额确定后再弹确认框
    BidAmount,
}

/// C# `ItemNotSoldGetBack` / `ItemNotSoldConfirmRetrieve`（同文案）
pub fn market_retrieve_text(item_name: &str) -> String {
    format!("{}尚未售出，确定要取回它吗？", item_name)
}

/// C# `ConfirmBuyItemWithPrice`：`{1:#,##0}` 千分位 + 货币名（金币/积分）
pub fn market_buy_text(item_name: &str, price: u32, currency: &str) -> String {
    format!(
        "确定要以{} {}购买{}吗？",
        group_thousands(price),
        currency,
        item_name
    )
}

/// C# `ConfirmBidGoldForItem`：`你确定要为{物品}出价{额}金币吗？`（`{0:#,##0}` 千分位）
pub fn market_bid_text(item_name: &str, bid: u32) -> String {
    format!(
        "你确定要为{}出价{}金币吗？",
        item_name,
        group_thousands(bid)
    )
}

/// C# `BuyButton.Click` 分支（:360-440）：
/// - UserMode（寄售/拍卖页签）：`For Sale` / `No Bid` 弹「尚未售出，确定要取回它吗？」，其余直接 `MarketGetBack`
/// - 非 UserMode 寄售/商城：弹「确定要以 N 金币购买 X 吗？」；货币按页签取 金币/积分
/// - 非 UserMode 拍卖：返回 [`MarketBuyOutcome::BidAmount`]，由调用方弹 `MirAmountBox`
///   （C# `MirAmountBox(BidAmount, Item.Info.Image, uint.MaxValue, Price + 1, Price + 1)`），
///   金额确定后再走确认框（`ConfirmBidGoldForItem`）
pub fn market_buy_outcome(item: &MarketItem, panel: MarketPanelType) -> MarketBuyOutcome {
    let user_mode = matches!(panel, MarketPanelType::Consign | MarketPanelType::Auction);
    if user_mode {
        let action = MarketConfirmAction::GetBack {
            auction_id: item.auction_id,
        };
        let needs_confirm = (item.item_type == 0 && item.seller == "For Sale")
            || (item.item_type == 1 && item.seller == "No Bid");
        if needs_confirm {
            MarketBuyOutcome::Confirm(market_retrieve_text(&item.name), action)
        } else {
            MarketBuyOutcome::Direct(action)
        }
    } else if item.item_type == 1 {
        // 拍卖：先弹 `MirAmountBox`（出价金额）
        MarketBuyOutcome::BidAmount
    } else {
        let currency = if panel == MarketPanelType::GameShop {
            "积分"
        } else {
            "金币"
        };
        MarketBuyOutcome::Confirm(
            market_buy_text(&item.name, item.price, currency),
            MarketConfirmAction::Buy {
                auction_id: item.auction_id,
                bid_price: 0,
            },
        )
    }
}

/// 确认框状态（`visible` + 文案 + 待执行动作）
#[derive(Resource, Default)]
pub struct MarketConfirm {
    pub visible: bool,
    pub text: String,
    pub action: Option<MarketConfirmAction>,
}

#[derive(Component)]
pub struct MarketConfirmWidget;

#[derive(Component)]
pub struct MarketConfirmText;

#[derive(Component)]
pub struct MarketConfirmYes;

#[derive(Component)]
pub struct MarketConfirmNo;

// ===== 列表行（C# `AuctionRow`，TrustMerchantDialog.cs:1440-1625）=====

/// C# `Rows[i].Location = new Point(127, 82 + i * 33)`；`Size = (354, 32)`
pub const TM_ROW_X: f32 = 127.0;
pub const TM_ROW_Y: f32 = 82.0;
pub const TM_ROW_W: f32 = 354.0;
pub const TM_ROW_H: f32 = 32.0;
pub const TM_ROW_STEP: f32 = 33.0;
/// `IconArea = (34, 32)`（图标按 `(Area - Icon.Size)/2` 居中）
pub const TM_ROW_ICON_W: f32 = 34.0;
pub const TM_ROW_ICON_H: f32 = 32.0;
/// 行内标签（相对行原点）：`NameLabel`(38,8)/`PriceLabel`(170,8)/`SellerLabel`(256,0)/`ExpireLabel`(256,14)
pub const TM_ROW_NAME_POS: (f32, f32) = (38.0, 8.0);
pub const TM_ROW_PRICE_POS: (f32, f32) = (170.0, 8.0);
pub const TM_ROW_SELLER_POS: (f32, f32) = (256.0, 0.0);
pub const TM_ROW_EXPIRE_POS: (f32, f32) = (256.0, 14.0);
/// 选中边框（C# `BorderColour = Color.FromArgb(255, 200, 100, 0)`，`BorderInfo` 外扩 1px）
pub const TM_ROW_BORDER_COLOR: Color = Color::srgb_u8(200, 100, 0);
pub const TM_ROW_BORDER_INSET: f32 = 1.0;
/// 空/零数量物品的占位图标（C# `Prguse[540]`）
pub const TM_ROW_PLACEHOLDER_FRAME: usize = 540;

/// C# `AuctionRow` 行槽
#[derive(Component)]
pub struct MarketAuctionRow(pub usize);

/// 行图标（C# `IconImage`）
#[derive(Component)]
pub struct MarketRowIcon(pub usize);

/// 行文本种类（C# `NameLabel`/`PriceLabel`/`SellerLabel`/`ExpireLabel`）
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum MarketRowTextKind {
    Name,
    Price,
    Seller,
    Expire,
}

/// 行文本（槽位 + 种类）
#[derive(Component)]
pub struct MarketRowText(pub usize, pub MarketRowTextKind);

/// 行选中边框（C# `AuctionRow.Border`，`Rows[i].Border = Rows[i] == Selected`）
#[derive(Component)]
pub struct MarketRowBorder(pub usize);

/// 底栏按钮（C# `UpdateInterface` 的三个启用态开关，:1005-1033）
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum MarketBottomBtn {
    /// 选中行才可用（C# `BuyButton.Enabled`）
    Buy,
    /// 未选中时才可用（C# `CollectSoldButton`）
    CollectSold,
    /// 选中行卖家为 `Bid Met` 才可用（C# `SellNowButton`）
    SellNow,
    /// 选中行才可用（C# `MailButton.Enabled`）
    Mail,
}

/// C# `UpdateInterface`（TrustMerchantDialog.cs:1005-1033）底栏四键的 `Enabled`：
/// 有选中 → Buy/Mail 可用、CollectSold 不可用（反之亦然）；SellNow 仅当选中行卖家为
/// `Bid Met`（拍卖已有人出价）。禁用态同时是 `GrayScale = true` 的灰度绘制。
pub fn market_bottom_enabled(kind: MarketBottomBtn, has_sel: bool, bid_met: bool) -> bool {
    match kind {
        MarketBottomBtn::Buy => has_sel,
        MarketBottomBtn::CollectSold => !has_sel,
        // C# `Selected != null && Selected.Listing.Seller == "Bid Met"`
        MarketBottomBtn::SellNow => has_sel && bid_met,
        MarketBottomBtn::Mail => has_sel,
    }
}

/// 行内绝对定位节点（`border` = 四周 1px 描边，用于选中框）
fn row_node(x: f32, y: f32, w: f32, h: f32, border: bool) -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(x),
        top: Val::Px(y),
        width: Val::Px(w),
        height: Val::Px(h),
        border: if border {
            UiRect::all(Val::Px(1.0))
        } else {
            UiRect::default()
        },
        ..default()
    }
}

/// C# `{0:###,###,##0}` 千分位
pub fn group_thousands(n: u32) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// C# `AuctionRow.Update`：`String.Format("{0:###,###,##0} {1}", Price, 拍卖 ? "出价" : "")`
pub fn row_price_text(price: u32, item_type: u8) -> String {
    let amount = group_thousands(price);
    if item_type == 1 {
        format!("{} 出价", amount)
    } else {
        amount
    }
}

/// C# 价格颜色阈值（:1575-1584）：>10M 红 / >1M 橙 / >100k 草绿 / >10k 天蓝 / 其余白
pub fn row_price_color(price: u32) -> Color {
    if price > 10_000_000 {
        Color::srgb(1.0, 0.0, 0.0)
    } else if price > 1_000_000 {
        Color::srgb(1.0, 0.549, 0.0)
    } else if price > 100_000 {
        Color::srgb(0.486, 0.988, 0.0)
    } else if price > 10_000 {
        Color::srgb(0.0, 0.749, 1.0)
    } else {
        Color::WHITE
    }
}

/// C# `GradeNameColor`（GameScene.cs:6791-6808）+ 调用点「黄色→白色」：
/// None/Common 白、Rare 天蓝、Legendary 深橙、Mythical 梅红、Heroic 红
pub fn row_name_color(grade: u8) -> Color {
    match grade {
        5 => Color::srgb(0.0, 0.749, 1.0),     // Rare DeepSkyBlue
        6 => Color::srgb(1.0, 0.549, 0.0),     // Legendary DarkOrange
        7 => Color::srgb(0.867, 0.627, 0.867), // Mythical Plum
        8 => Color::srgb(1.0, 0.0, 0.0),       // Heroic Red
        _ => Color::WHITE,                     // None/Common（C# 黄 → 白）
    }
}

/// C# 卖家列颜色（:1587-1607）：UserMode 下 `Sold` 金 / `Expired` 红 / `Bid Met` 草绿 / 其余白
pub fn row_seller_color(seller: &str, user_mode: bool) -> Color {
    if !user_mode {
        return Color::WHITE;
    }
    match seller {
        "Sold" => Color::srgb(1.0, 0.843, 0.0),
        "Expired" => Color::srgb(1.0, 0.0, 0.0),
        "Bid Met" => Color::srgb(0.486, 0.988, 0.0),
        _ => Color::WHITE,
    }
}

/// C# `AuctionRow.UpdateInterface`：`ConsignmentDate.AddDays(ConsignmentLength)` 的
/// `{0:dd/MM/yy HH:mm:ss}` 文本（本地墙钟，与 C# `DateTime` 一致）
pub fn row_expire_text(consignment_date: i64) -> String {
    if consignment_date <= 0 {
        return String::new();
    }
    let expire = consignment_date + TM_CONSIGNMENT_LENGTH_DAYS * 86_400;
    chrono::DateTime::from_timestamp(expire, 0)
        .map(|t| t.with_timezone(&chrono::Local).format("%d/%m/%y %H:%M:%S").to_string())
        .unwrap_or_default()
}

/// 价格输入状态（C# `TextBox_TextChanged` 的 `PriceTextBox.BorderColour` 三态 + 上限钳制）
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MarketPriceState {
    /// 未输入或低于下限（C# Red + `SellItemButton.Enabled = false`）
    Invalid,
    /// 有效（C# Lime）
    Valid,
    /// 达到上限（C# Orange）；输入会被钳到上限后再提交
    Capped,
}

impl MarketPriceState {
    pub fn allowed(self) -> bool {
        !matches!(self, MarketPriceState::Invalid)
    }
}

/// C# `TextBox_TextChanged`：按页签判定价格区间（寄售 5000..50M、拍卖 0..50k）
pub fn price_state(panel: MarketPanelType, price: u32) -> MarketPriceState {
    match panel {
        MarketPanelType::Auction => {
            // C# 先 `if (Amount > MaxBidAmount) Amount = MaxBidAmount;` 再判 `== Max` → Orange
            if price >= TM_MAX_STARTING_BID {
                MarketPriceState::Capped
            } else {
                MarketPriceState::Valid
            }
        }
        _ => {
            if price < TM_MIN_CONSIGN_PRICE {
                MarketPriceState::Invalid
            } else if price >= TM_MAX_CONSIGN_PRICE {
                MarketPriceState::Capped
            } else {
                MarketPriceState::Valid
            }
        }
    }
}

/// C# `HelpLabel` 文案（:52-64，用 `Globals` 数值格式化）
pub fn help_text(panel: MarketPanelType) -> String {
    match panel {
        MarketPanelType::Auction => format!(
            "1. 拍卖费用为{0}金币，单件起拍价最高为{1}金币 \n\n2. 成交价的1%在拍卖结束时支付给信托商人\n\n3. 最长可登记拍卖{2}天，到期后物品将交给最高出价者\n\n4. 拍卖物品数量无限制\n\n",
            5000, TM_MAX_STARTING_BID, 7
        ),
        _ => format!(
            "1. 寄售费用为每件{0}金币 \n\n2. 成交价的1%在出售结束时支付给信托商人\n\n3. 最长可登记出售{1}天，超时物品将被移除\n\n4. 寄售物品数量无限制\n\n5. 售价可设定范围：{2} - {3}金币",
            5000, 7, TM_MIN_CONSIGN_PRICE, TM_MAX_CONSIGN_PRICE
        ),
    }
}

/// 寄售目标格（C# `ItemCell`，GridType.TrustMerchant）
#[derive(Component)]
pub struct MarketConsignCell;
/// 寄售/拍卖提交键（C# `SellItemButton`）
#[derive(Component)]
pub struct MarketSellItemBtn;
/// 领取已售金币（C# `CollectSoldButton`）
#[derive(Component)]
pub struct MarketCollectSoldBtn;
/// 寄售说明标签（C# `HelpLabel`）
#[derive(Component)]
pub struct MarketHelpLabel;

/// 页签相关精灵（面板背景 786/787 + Buy 703..705/706..708）
#[derive(Resource)]
pub struct MarketPanelSprites {
    pub bg_market: Handle<Image>,
    pub bg_consign: Handle<Image>,
    pub buy_market: (Handle<Image>, Handle<Image>, Handle<Image>),
    pub buy_user: (Handle<Image>, Handle<Image>, Handle<Image>),
}

pub struct MarketPlugin;

impl Plugin for MarketPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MarketState>();
        app.init_resource::<MarketConfirm>();
        app.init_resource::<MarketBidPending>();
                app.add_systems(
            Update,
            market_server_events.run_if(in_state(AppState::Game)),
        );
app.add_systems(OnEnter(AppState::Game), spawn_market);
        app.add_systems(OnExit(AppState::Game), cleanup_market);
        app.add_systems(
            Update,
            (market_ui_system, market_action_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
        app.add_systems(
            Update,
            market_tab_system
                .chain()
                .run_if(in_state(AppState::Game)),
        );
        app.add_systems(
            Update,
            market_filter::market_filter_system.run_if(in_state(AppState::Game)),
        );
        app.add_systems(
            Update,
            (
                market_panel_system,
                market_consign_system,
                market_row_system,
                market_consign_cell_system,
                market_price_filter_system,
                market_mail_system,
                market_confirm_system,
                market_bid_amount_system,
            )
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_market(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_market(
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

    // 面板 C# `TrustMerchantDialog`：Title[786]（Market/GameShop）与 Title[787]（寄售/拍卖）
    // 原生 492x478（C# 未设 Location → (0,0)）；页签切换时换背景
    let Some(bg_market) = load_lib_image(&mut libs, &mut images, LibraryName::Title, 786) else {
        return;
    };
    let bg_consign = load_lib_image(&mut libs, &mut images, LibraryName::Title, 787);
    let (Some(bg_consign), Some(buy_market), Some(buy_user)) = (
        bg_consign,
        load_buy_frames(&mut libs, &mut images, TM_BUY_MARKET_FRAMES),
        load_buy_frames(&mut libs, &mut images, TM_BUY_USER_FRAMES),
    ) else {
        return;
    };
    let panel = spawn_panel(
        &mut commands,
        bg_market.clone(),
        TM_POS.0,
        TM_POS.1,
        TM_PANEL_W,
        TM_PANEL_H,
        30,
    );
    commands.entity(panel).insert((
        DialogRoot(DialogKind::Market),
        MarketWidget,
        // #89 市场列表滚轮翻页：1 格 = 1 页（10 行）
        UiScrollList {
            rect_rel: (TM_LIST_X, TM_LIST_Y, 300.0, 180.0),
            row_h: 18.0,
            visible: 10,
            total: 0,
            offset: 0,
            step: 10,
            track_rel: (435.0, TM_LIST_Y, 4.0, 180.0),
            thumb: None,
            z: 9,
        },
    ));

    let mut filter_sprites: Option<MarketFilterSprites> = None;
    commands.entity(panel).with_children(|p| {
        // 滚动条（面板子节点）
        spawn_scroll_bar_ui(p, (435.0, TM_LIST_Y, 4.0, 180.0), 9);
        // 关闭 C# CloseButton Prguse2[360/361/362] @(465,3)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 360),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 361),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 362),
        ) {
            spawn_icon_button(p, n, h, pr, TM_CLOSE.0, TM_CLOSE.1, 24.0, 21.0, 10)
                .insert(MarketClose);
        }
        // C# 四个页签（Title[789/788]、[791/790]、[817/816]、[819/818]）
        for (name, x, y, normal, pressed) in TM_TABS {
            if let (Some(n), Some(h), Some(pr)) = (
                load_lib_image(&mut libs, &mut images, LibraryName::Title, normal),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, normal),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, pressed),
            ) {
                spawn_icon_button(p, n, h, pr, x, y, 92.0, 21.0, 10).insert(MarketTabBtn(name));
            }
        }
        // C# `AuctionRow` ×10（:1440-1522）：行 (127, 82+i*33) 354x32 = 34x32 图标区 +
        // 名称/价格/卖家/到期 4 标签 + 选中橙框（外扩 1px）
        for i in 0..10usize {
            let mut row = spawn_container(
                p,
                TM_ROW_X,
                TM_ROW_Y + i as f32 * TM_ROW_STEP,
                TM_ROW_W,
                TM_ROW_H,
                9,
            );
            row.insert((MarketAuctionRow(i), Visibility::Visible));
            let row_id = row.id();
            p.commands().entity(row_id).with_children(|r| {
                // 图标（C# `IconImage`）：尺寸/居中位置由 `market_row_system` 按图标实际尺寸写
                r.spawn((
                    MarketRowIcon(i),
                    ImageNode::new(Handle::default()),
                    row_node(0.0, 0.0, 1.0, 1.0, false),
                    ZIndex(1),
                ));
                for (kind, (lx, ly)) in [
                    (MarketRowTextKind::Name, TM_ROW_NAME_POS),
                    (MarketRowTextKind::Price, TM_ROW_PRICE_POS),
                    (MarketRowTextKind::Seller, TM_ROW_SELLER_POS),
                    (MarketRowTextKind::Expire, TM_ROW_EXPIRE_POS),
                ] {
                    spawn_label(r, &cjk, "", lx, ly, 12.0, Color::WHITE, 2)
                        .insert(MarketRowText(i, kind));
                }
                // 选中框（C# `BorderInfo` 外扩 1px；C# `SelectedImage`(Prguse[545]) 是死控件）
                r.spawn((
                    MarketRowBorder(i),
                    row_node(
                        -TM_ROW_BORDER_INSET,
                        -TM_ROW_BORDER_INSET,
                        TM_ROW_W + TM_ROW_BORDER_INSET * 2.0,
                        TM_ROW_H + TM_ROW_BORDER_INSET * 2.0,
                        true,
                    ),
                    BackgroundColor(Color::NONE),
                    BorderColor::all(TM_ROW_BORDER_COLOR),
                    ZIndex(3),
                    Visibility::Hidden,
                ));
            });
        }
        // C# `PageLabel` @(260,419) 70x18：行 10 承载「第 x/y 页」
        spawn_label(p, &cjk, "", TM_PAGE_POS.0, TM_PAGE_POS.1, 12.0, Color::srgb(1.0, 0.9, 0.5), 9)
            .insert(MarketLine(10));
        // 消息行（Bevy 扩展，放在列表下方）
        spawn_label(p, &cjk, "", TM_LIST_X, TM_LIST_Y + 190.0, 12.0, Color::srgb(1.0, 0.9, 0.5), 9)
            .insert(MarketLine(11));
        // C# 底部操作栏：Find Title[480..482] @(124,448)、Refresh Prguse[663..665] @(320,448)、
        // Buy Title[703..705] @(380,448)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 480),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 481),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 482),
        ) {
            spawn_icon_button(p, n, h, pr, TM_FIND_POS.0, TM_FIND_POS.1, 48.0, 25.0, 10)
                .insert(MarketSearchBtn);
        }
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 663),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 664),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 665),
        ) {
            spawn_icon_button(p, n, h, pr, TM_REFRESH_POS.0, TM_REFRESH_POS.1, 28.0, 25.0, 10)
                .insert(MarketRefreshBtn);
        }
        // Buy `Title[703..705]` @(380,448)：寄售/拍卖页签换成 `Title[706..708]`（运行时切换）
        {
            let (n, h, pr) = buy_market.clone();
            spawn_icon_button(p, n, h, pr, TM_BUY_POS.0, TM_BUY_POS.1, 84.0, 25.0, 10)
                .insert((
                    MarketBuyBtn,
                    MarketBottomBtn::Buy,
                    UiGray::default(),
                ));
        }
        // 表头标签（C# 5 个 Title*Label，居中；文案随页签变化）
        for (kind, x, y, w) in TM_HEADERS {
            spawn_label_center(p, &cjk, "", x + w / 2.0, y, w, 12.0, Color::WHITE, 9).insert(kind);
        }
        // #2720：价格排序（C# `TitlePriceLabel.Click` 命中区 + `PriceFilterIcon` Prguse2[925/926]）
        spawn_container(
            p,
            TM_PRICE_HEADER_POS.0,
            TM_PRICE_HEADER_POS.1,
            TM_PRICE_HEADER_W,
            TM_PRICE_HEADER_H,
            10,
        )
        .insert((Button, BackgroundColor(Color::NONE), MarketPriceFilterBtn));
        {
            let low = load_lib_image(
                &mut libs,
                &mut images,
                LibraryName::Prguse2,
                TM_PRICE_ICON_LOW,
            );
            let high = load_lib_image(
                &mut libs,
                &mut images,
                LibraryName::Prguse2,
                TM_PRICE_ICON_HIGH,
            );
            if let (Some(low), Some(high)) = (low, high) {
                spawn_icon_button(
                    p,
                    low.clone(),
                    low,
                    high,
                    TM_PRICE_ICON_POS.0,
                    TM_PRICE_ICON_POS.1,
                    TM_PRICE_ICON_W,
                    TM_PRICE_ICON_H,
                    11,
                )
                .insert((MarketPriceFilterIcon, Visibility::Hidden));
            }
        }
        // #2720：写邮件（C# `MailButton` Prguse[437..439] @(350,448)，仅市场页签）
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 437),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 438),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 439),
        ) {
            spawn_icon_button(
                p,
                n,
                h,
                pr,
                TM_MAIL_POS.0,
                TM_MAIL_POS.1,
                TM_MAIL_W,
                TM_MAIL_H,
                10,
            )
            .insert((
                MarketMailBtn,
                MarketBottomBtn::Mail,
                MarketForPanel::MarketOnly,
                UiGray::default(),
            ));
        }
        // C# 翻页：Back Prguse2[240..242] @(251,419)、Next Prguse2[243..245] @(320,419)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 240),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 241),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 242),
        ) {
            spawn_icon_button(p, n, h, pr, TM_BACK_POS.0, TM_BACK_POS.1, 16.0, 16.0, 10)
                .insert(MarketPrevBtn);
        }
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 243),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 244),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 245),
        ) {
            spawn_icon_button(p, n, h, pr, TM_NEXT_POS.0, TM_NEXT_POS.1, 16.0, 16.0, 10)
                .insert(MarketNextBtn);
        }
        // C# 搜索框 @(11,452) 110x18（Bevy TextInput id 5）
        if let Some(search_box) = spawn_market_input(
            p,
            &mut images,
            &font,
            5,
            TM_SEARCH_POS.0,
            TM_SEARCH_POS.1,
            110.0,
            11.0,
            452.0,
        ) {
            p.commands().entity(search_box).insert(MarketSearchField);
        }
        // C# 左列筛选树（`Prguse2[920..923]` 按钮 + `[197..209]` 滚动条），仅 Market/GameShop 页签可见
        filter_sprites = market_filter::spawn_filter_tree(p, &mut libs, &mut images, &cjk);
        // C# 寄售/拍卖页签面板（`Index=787` 背景 + HelpLabel/ItemCell/PriceTextBox/SellItemButton）
        spawn_label(
            p,
            &cjk,
            &help_text(MarketPanelType::Consign),
            TM_HELP_POS.0,
            TM_HELP_POS.1,
            12.0,
            Color::WHITE,
            9,
        )
        .insert((MarketHelpLabel, MarketForPanel::ConsignOrAuction));
        spawn_item_cell_ui(
            p,
            &mut images,
            &font,
            TM_CONSIGN_CELL_POS.0,
            TM_CONSIGN_CELL_POS.1,
            TM_CONSIGN_CELL_W,
            TM_CONSIGN_CELL_H,
            9,
            0,
        )
        .insert((MarketConsignCell, Button, MarketForPanel::ConsignOrAuction));
        if let Some(price_box) = spawn_market_input(
            p,
            &mut images,
            &font,
            6,
            TM_PRICE_POS.0,
            TM_PRICE_POS.1,
            TM_PRICE_W,
            TM_PRICE_POS.0,
            TM_PRICE_POS.1,
        ) {
            p.commands()
                .entity(price_box)
                .insert((MarketPriceField, MarketForPanel::ConsignOrAuction));
        }
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 700),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 701),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 702),
        ) {
            spawn_icon_button(
                p,
                n,
                h,
                pr,
                TM_SELL_ITEM_POS.0,
                TM_SELL_ITEM_POS.1,
                TM_SELL_BTN_W,
                TM_SELL_BTN_H,
                10,
            )
            .insert((
                MarketSellItemBtn,
                MarketForPanel::ConsignOrAuction,
                UiGray::default(),
            ));
        }
        // C# `CollectSoldButton`（仅寄售）/ `SellNowButton`（仅拍卖），都在底栏
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 680),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 681),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 682),
        ) {
            spawn_icon_button(
                p,
                n,
                h,
                pr,
                TM_COLLECT_SOLD_POS.0,
                TM_COLLECT_SOLD_POS.1,
                TM_COLLECT_BTN_W,
                TM_COLLECT_BTN_H,
                10,
            )
            .insert((
                MarketCollectSoldBtn,
                MarketBottomBtn::CollectSold,
                MarketForPanel::ConsignOnly,
                UiGray::default(),
            ));
        }
        {
            if let Some((n, h, pr)) = load_buy_frames(&mut libs, &mut images, (700, 701, 702)) {
                spawn_icon_button(
                    p,
                    n,
                    h,
                    pr,
                    TM_SELL_NOW_POS.0,
                    TM_SELL_NOW_POS.1,
                    TM_SELL_BTN_W,
                    TM_SELL_BTN_H,
                    10,
                )
                .insert((
                    MarketSellNowBtn,
                    MarketBottomBtn::SellNow,
                    MarketForPanel::AuctionOnly,
                    UiGray::default(),
                ));
            }
        }
    });
    if let Some(sp) = filter_sprites {
        commands.insert_resource(sp);
    }
    // #2733：此前漏了这条插入 → `market_panel_system` 拿不到 `MarketPanelSprites`，
    // 页签背景 786/787 与 Buy 精灵 703..708 的切换实为死代码（PR #2732 的漏项）
    commands.insert_resource(MarketPanelSprites {
        bg_market,
        bg_consign,
        buy_market,
        buy_user,
    });

    // #2720：买/取回确认框（C# `MirMessageBox` YesNo）——独立根节点（不在 TM 面板裁剪内，
    // C# 里也是全局模态框）
    if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 360) {
        let panel = spawn_panel(
            &mut commands,
            h,
            TM_CONFIRM_POS.0,
            TM_CONFIRM_POS.1,
            TM_CONFIRM_W,
            TM_CONFIRM_H,
            45,
        );
        commands
            .entity(panel)
            .insert((MarketConfirmWidget, Visibility::Hidden));
        commands.entity(panel).with_children(|p| {
            spawn_label(
                p,
                &cjk,
                "",
                TM_CONFIRM_TEXT_POS.0,
                TM_CONFIRM_TEXT_POS.1,
                12.0,
                Color::WHITE,
                9,
            )
            .insert((MarketConfirmWidget, MarketConfirmText));
            if let (Some(n), Some(h), Some(pr)) = (
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 206),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 207),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 208),
            ) {
                spawn_icon_button(
                    p,
                    n,
                    h,
                    pr,
                    TM_CONFIRM_YES_POS.0,
                    TM_CONFIRM_YES_POS.1,
                    TM_CONFIRM_BTN_W,
                    TM_CONFIRM_BTN_H,
                    10,
                )
                .insert(MarketConfirmYes);
            }
            if let (Some(n), Some(h), Some(pr)) = (
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 210),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 211),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 212),
            ) {
                spawn_icon_button(
                    p,
                    n,
                    h,
                    pr,
                    TM_CONFIRM_NO_POS.0,
                    TM_CONFIRM_NO_POS.1,
                    TM_CONFIRM_BTN_W,
                    TM_CONFIRM_BTN_H,
                    10,
                )
                .insert(MarketConfirmNo);
            }
        });
    }
}

/// 市场输入框（TextInputField(id) + 子 TextInputDisplay(id)）；面板子节点
#[allow(clippy::too_many_arguments)]
fn spawn_market_input(
    parent: &mut bevy::ecs::hierarchy::ChildSpawnerCommands,
    images: &mut Assets<Image>,
    font: &Handle<Font>,
    id: usize,
    x: f32,
    y: f32,
    w: f32,
    rect_x: f32,
    rect_y: f32,
) -> Option<Entity> {
    let container = spawn_container(parent, x, y, w, 20.0, 10)
        .insert((
            crate::game::dialogs::text_input::TextInputField(id),
            crate::game::dialogs::text_input::TextInputRect(rect_x, rect_y, w, 20.0),
            BackgroundColor(Color::srgba(0.2, 0.2, 0.25, 0.9)),
        ))
        .with_children(|ic| {
            ic.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(4.0),
                    top: Val::Px(2.0),
                    ..default()
                },
                Text::new(String::new()),
                TextFont {
                    font: FontSource::Handle(font.clone()),
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 1.0, 1.0)),
                ZIndex(11),
                crate::game::dialogs::text_input::TextInputDisplay(id),
            ));
        })
        .id();
    let _ = images;
    Some(container)
}

/// 显隐 + 渲染 + 按钮
#[allow(clippy::too_many_arguments)]
/// 商品行命中矩形（面板原点 ox/oy + 相对坐标；i 0..10）
fn market_row_rect(i: usize, ox: f32, oy: f32) -> (f32, f32, f32, f32) {
    // C# `AuctionRow`：行 (127, 82+i*33) 354x32（行矩形与绘制同源）
    (
        ox + TM_ROW_X,
        oy + TM_ROW_Y + i as f32 * TM_ROW_STEP,
        TM_ROW_W,
        TM_ROW_H,
    )
}

fn market_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut market: ResMut<MarketState>,
    net: Res<NetConnection>,
    mut input: ResMut<crate::game::dialogs::text_input::TextInputState>,
    // 五组按钮查询折叠成一个元组参数（系统参数上限 16）
    btns: (
        Query<(Entity, &Interaction), With<MarketClose>>,
        Query<(Entity, &Interaction), With<MarketRefreshBtn>>,
        Query<(Entity, &Interaction), With<MarketSearchBtn>>,
        Query<(Entity, &Interaction), With<MarketPrevBtn>>,
        Query<(Entity, &Interaction), With<MarketNextBtn>>,
    ),
    mouse: Res<ButtonInput<MouseButton>>,
    ui: (
        Query<&Window>,
        Query<&Node, With<MarketWidget>>,
    ),
    mut widgets: Query<&mut Visibility, With<MarketWidget>>,
    mut lines: Query<(&mut Text, &MarketLine)>,
    mut scroll: Query<&mut UiScrollList, With<MarketWidget>>,
    mut place_at: MessageWriter<crate::game::dialogs::inventory::InventoryPlaceAt>,
    mut requested: Local<bool>,
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
    let open = mgr.is_open(DialogKind::Market);
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        *requested = false;
        return;
    }
    // 打开瞬间按 C# `TMerchantDialog(MarketPanelType.Market)`：
    // `DrawFilters(0, -1)` 复位筛选树 + 发 `C.MarketSearch{Match="", Type=Nothing, Usermode=false}`
    if !*requested {
        *requested = true;
        // C# `TrustMerchantDialog.Show()`（:1435-1436）：背包推到 `Size.Width + 5` 并打开
        place_at.write(crate::game::dialogs::inventory::InventoryPlaceAt(
            TM_PANEL_W + 5.0,
        ));
        mgr.open(DialogKind::Inventory);
        market.panel = MarketPanelType::Market;
        market.filter_index = 0;
        market.filter_sub_index = None;
        market.filter_skip = 0;
        // 新一轮搜索：清掉上一次会话的未决翻页请求（页累积由第 0 页回包复位）
        market.pending_page = None;
        send_market_search(
            &net,
            "",
            ItemType::Nothing,
            false,
            0,
            0,
            MarketPanelType::Market,
        );
        tracing::info!("🏪 打开市场（C# MarketSearch 复位）");
    }
    for (e, inter) in &btns.0 {
        if edge(e, inter, &mut prev_inter) {
            mgr.close(DialogKind::Market);
            // C# `Hide()`（:1411）：背包复位到 (0,0)
            place_at.write(crate::game::dialogs::inventory::InventoryPlaceAt(0.0));
        }
    }
    // 渲染（#89 滚轮翻页：scroll.offset 行号 ↔ market.page 同步）
    // 可滚动范围 = 已累积页（C# 同为顺序翻页：未加载的页先请求、不跳页）
    {
        let mut sl = scroll.single_mut();
        if let Ok(sl) = sl.as_mut() {
            sl.set_total(market.loaded_pages.max(1) * 10);
            let want = market.page * 10;
            if sl.offset != want {
                sl.offset = want; // 翻页按钮驱动 → 同步滚动条
            }
            let new_page = sl.offset / 10;
            if new_page != market.page {
                // 滚轮驱动 → 本地翻页（仅已累积页）
                market.page = new_page;
            }
        }
    }
    for (mut text, line) in &mut lines {
        text.0 = match line.0 {
            10 => format!("第 {}/{} 页", market.page + 1, market.pages.max(1)),
            11 => market.message.clone(),
            _ => String::new(),
        };
    }
    // 行点击选中
    if mouse.just_pressed(MouseButton::Left) {
        if let Ok(window) = ui.0.single() {
            if let Some(cursor) = window.cursor_position() {
                let (ox, oy) = ui
                    .1
                    .single()
                    .map(|n| crate::ui::theme::node_origin(n, (280.0, 80.0)))
                    .unwrap_or((280.0, 80.0));
                for i in 0..10usize {
                    let (rx, ry, rw, rh) = market_row_rect(i, ox, oy);
                    if cursor.x >= rx && cursor.x <= rx + rw && cursor.y >= ry && cursor.y <= ry + rh {
                        // 按价格排序映射到 `listings` 下标（Bevy 服务端按页下发，见 `row_listing_index`）
                        if let Some(idx) = row_listing_index(&market, i) {
                            market.selected = Some(idx);
                            let it = &market.listings[idx];
                            tracing::info!(
                                "🏪 选中商品: {} {} 卖家={} 价格={}",
                                it.auction_id,
                                it.name,
                                it.seller,
                                it.price
                            );
                        }
                        break;
                    }
                }
            }
        }
    }
    // 刷新
    for (e, inter) in &btns.1 {
        if edge(e, inter, &mut prev_inter) {
            // C# `RefreshButton.Click`：清空搜索框 + `C.MarketRefresh`（保留筛选树选中）
            if let Some(t) = input.texts.get_mut(5) {
                t.clear();
            }
            input.active = None;
            net.send_packet(&mir2_shared::packets::client::market::MarketRefresh);
            tracing::info!("🏪 刷新市场");
        }
    }
    // 搜索（C# `FindButton.Click` → `C.MarketSearch{Match, MarketType}`，Type 默认 Nothing 不过滤）
    for (e, inter) in &btns.2 {
        if edge(e, inter, &mut prev_inter) {
            let kw = input.texts.get(5).cloned().unwrap_or_default().trim().to_string();
            if kw.is_empty() {
                continue;
            }
            send_market_search(&net, &kw, ItemType::Nothing, false, 0, 0, market.panel);
            tracing::info!("🏪 搜索市场: {}", kw);
        }
    }
    // 翻页
    for (e, inter) in &btns.3 {
        if edge(e, inter, &mut prev_inter) {
            // C# `BackButton.Click`：纯本地翻页（列表已累积）
            if let Some(prev) = back_page_action(market.page) {
                market.page = prev;
            }
        }
    }
    for (e, inter) in &btns.4 {
        if edge(e, inter, &mut prev_inter) {
            // C# `NextButton.Click`：已累积 → 本地翻页；否则请求下一页
            if let Some((next, need_request)) =
                next_page_action(market.page, market.loaded_pages, market.pages)
            {
                if need_request {
                    request_market_page(&mut market, &net, next);
                } else {
                    market.page = next;
                }
            }
        }
    }
}

/// C# 页签：`TMerchantDialog(type)`（TrustMerchantDialog.cs:1117-1314）——
/// Market：筛选树可见 + 复位到 index 0；GameShop：同上（本端开独立商城窗，见 §7 偏差）；
/// Consign/Auction：筛选树整列隐藏 + `Usermode=true` 搜索，本端暂用 Bevy 扩展按钮占位。
/// （独立系统避免 Bevy 16 参数上限）
fn market_tab_system(
    mut mgr: ResMut<DialogManager>,
    mut market: ResMut<MarketState>,
    net: Res<NetConnection>,
    tab_btns: Query<(Entity, &Interaction, &MarketTabBtn)>,
    mut panel_parts: Query<(&MarketForPanel, &mut Visibility)>,
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
    if !mgr.is_open(DialogKind::Market) {
        return;
    }
    let mut switched = false;
    for (e, inter, tab) in &tab_btns {
        if !edge(e, inter, &mut prev_inter) {
            continue;
        }
        match tab.0 {
            "market" => {
                market.panel = MarketPanelType::Market;
                market.filter_index = 0;
                market.filter_sub_index = None;
                market.filter_skip = 0;
                market.message = "当前：市场".to_string();
                switched = true;
            }
            // C# `GameShopButton.Click` → `TMerchantDialog(MarketPanelType.GameShop)`
            "game_shop" => {
                market.panel = MarketPanelType::GameShop;
                market.filter_index = 0;
                market.filter_sub_index = None;
                market.filter_skip = 0;
                mgr.open(DialogKind::GameShop);
                market.message = "打开游戏商城".to_string();
                switched = true;
            }
            "consign" => {
                market.panel = MarketPanelType::Consign;
                market.message = "寄售页签（面板待移植，可先用左列按钮）".to_string();
                switched = true;
            }
            "auction" => {
                market.panel = MarketPanelType::Auction;
                market.message = "拍卖页签（面板待移植，可先用左列按钮）".to_string();
                switched = true;
            }
            _ => {}
        }
    }
    // 页签切换：按 C# 补发搜索（Market/GameShop：Usermode=false；寄售/拍卖：Usermode=true）
    if switched {
        let (user_mode, item_type) = if matches!(
            market.panel,
            MarketPanelType::Consign | MarketPanelType::Auction
        ) {
            (true, ItemType::Nothing)
        } else {
            (false, ItemType::Nothing)
        };
        send_market_search(&net, "", item_type, user_mode, 0, 0, market.panel);
    }
    // 寄售/拍卖页签控件显隐（C# `TMerchantDialog(type)`：ItemCell/PriceTextBox/SellItem/Help 只在
    // 这两个页签；CollectSold 只寄售、SellNow 只拍卖）
    for (for_panel, mut vis) in &mut panel_parts {
        let show = match for_panel {
            kind => panel_part_visible(*kind, market.panel),
        };
        *vis = if show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

/// C# `C.MarketSearch` 发送（SharedRust 规范包：网关按此格式解析）
#[allow(clippy::too_many_arguments)]
fn send_market_search(
    net: &NetConnection,
    match_text: &str,
    item_type: ItemType,
    user_mode: bool,
    min_shape: i16,
    max_shape: i16,
    market_type: MarketPanelType,
) {
    net.send_packet(&mir2_shared::packets::client::market::MarketSearch {
        match_text: match_text.to_string(),
        item_type,
        user_mode,
        min_shape,
        max_shape,
        market_type,
    });
}

/// 页签面板：背景 `Title[786]`↔`[787]`、Buy 精灵 `703..705`↔`706..708`、
/// 表头文案/显隐、寄售说明文案（C# `TMerchantDialog(type)` :1117-1314）
fn market_panel_system(
    mgr: Res<DialogManager>,
    market: Res<MarketState>,
    sprites: Option<Res<MarketPanelSprites>>,
    mut bg: Query<&mut ImageNode, With<MarketWidget>>,
    mut buy: Query<&mut ImageButton, With<MarketBuyBtn>>,
    mut headers: Query<(&MarketHeader, &mut Text, &mut Visibility), Without<MarketHelpLabel>>,
    mut help: Query<(&mut Text, &mut Node), (With<MarketHelpLabel>, Without<MarketHeader>)>,
    mut consign_cell: Query<&mut BackgroundColor, With<MarketConsignCell>>,
) {
    if !mgr.is_open(DialogKind::Market) {
        return;
    }
    let consign_panel = matches!(
        market.panel,
        MarketPanelType::Consign | MarketPanelType::Auction
    );
    if let Some(sp) = sprites.as_ref() {
        for mut node in &mut bg {
            let want = if consign_panel {
                sp.bg_consign.clone()
            } else {
                sp.bg_market.clone()
            };
            if node.image != want {
                node.image = want;
            }
        }
        for mut btn in &mut buy {
            let (n, h, p) = if consign_panel {
                sp.buy_user.clone()
            } else {
                sp.buy_market.clone()
            };
            btn.normal = n;
            btn.hover = h;
            btn.pressed = p;
        }
    }
    for (kind, mut text, mut vis) in &mut headers {
        let want = header_text(*kind, market.panel);
        if text.0 != want {
            text.0 = want.to_string();
        }
        // C# `TMerchantDialog`：SalePrice/Sell 只在寄售/拍卖可见，其余页签恒可见
        let show = consign_panel || !matches!(kind, MarketHeader::SalePrice | MarketHeader::Sell);
        *vis = if show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for (mut text, mut node) in &mut help {
        let want = help_text(market.panel);
        if text.0 != want {
            text.0 = want;
        }
        // C# `HelpLabel` Size(115,205)：定宽换行 + 限高
        if node.width != Val::Px(TM_HELP_W) {
            node.width = Val::Px(TM_HELP_W);
        }
        if node.height != Val::Px(TM_HELP_H) {
            node.height = Val::Px(TM_HELP_H);
        }
    }
    // C# `MirItemCell` 空置态：`BackColour = (255,255,125)` + `Opacity = 0.5`（浅红半透明底）
    for mut bg in &mut consign_cell {
        let want = BackgroundColor(Color::srgba(1.0, 1.0, 0.49, 0.5));
        if *bg != want {
            *bg = want;
        }
    }
}

/// 寄售/拍卖交互：价格校验（边框三态 + 提交键启用态）、选物、提交、领取已售、立即售出
#[allow(clippy::too_many_arguments)]
fn market_consign_system(
    mgr: Res<DialogManager>,
    mut market: ResMut<MarketState>,
    net: Res<NetConnection>,
    mut input: ResMut<TextInputState>,
    mut inv_click: ResMut<InvClickState>,
    mut locked: ResMut<InvLockedSlots>,
    inv_q: Query<&Inventory, With<LocalPlayer>>,
    mut price_box: Query<&mut BackgroundColor, With<MarketPriceField>>,
    mut sell_btn: Query<(Entity, &Interaction, &mut UiGray), With<MarketSellItemBtn>>,
    cell: Query<(Entity, &Interaction), With<MarketConsignCell>>,
    collect_btn: Query<(Entity, &Interaction), With<MarketCollectSoldBtn>>,
    sellnow_btn: Query<(Entity, &Interaction), With<MarketSellNowBtn>>,
    mut place_at: MessageWriter<crate::game::dialogs::inventory::InventoryPlaceAt>,
    mut was_open: Local<bool>,
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
    if !mgr.is_open(DialogKind::Market) {
        // C# `Hide()`（TrustMerchantDialog.cs:1398-1411）：关窗解锁 `tempCell`
        if let Some(slot) = market.consign_slot.take() {
            locked.unlock(InvLockReason::Consign, slot);
        }
        // 任意关闭路径（关闭键/Control API/联动）都复位背包位置
        if *was_open {
            *was_open = false;
            place_at.write(crate::game::dialogs::inventory::InventoryPlaceAt(0.0));
        }
        return;
    }
    *was_open = true;
    let consign_panel = matches!(
        market.panel,
        MarketPanelType::Consign | MarketPanelType::Auction
    );
    if !consign_panel {
        // 离开寄售/拍卖页签：C# `Hide()`/切页签会清掉 `SellItemSlot` 与售价框
        // （:104-113 `MarketButton.Click` 同样解锁 `tempCell`）
        if let Some(slot) = market.consign_slot.take() {
            locked.unlock(InvLockReason::Consign, slot);
        }
        if market.consign_item.is_some() {
            market.consign_item = None;
            if let Some(t) = input.texts.get_mut(6) {
                t.clear();
            }
            input.active = None;
        }
        return;
    }
    // 价格：C# `TextBox_TextChanged`（只留数字 + 上限钳制 + 三态边框）
    let raw = input.texts.get(6).cloned().unwrap_or_default();
    let digits: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();
    let mut price: u32 = digits.parse().unwrap_or(0);
    let max = if market.panel == MarketPanelType::Auction {
        TM_MAX_STARTING_BID
    } else {
        TM_MAX_CONSIGN_PRICE
    };
    if price > max {
        price = max;
        // C# 回写钳制后的数值（`PriceTextBox.Text = ...`）
        input.texts.insert(6, price.to_string());
    }
    let state = price_state(market.panel, price);
    // C# `PriceTextBox.BorderColour`：Red / Lime / Orange（Bevy 用输入框底色近似）
    let fill = match state {
        MarketPriceState::Invalid => Color::srgba(0.45, 0.10, 0.10, 0.9),
        MarketPriceState::Valid => Color::srgba(0.10, 0.35, 0.12, 0.9),
        MarketPriceState::Capped => Color::srgba(0.45, 0.30, 0.05, 0.9),
    };
    for mut bg in &mut price_box {
        *bg = BackgroundColor(fill);
    }
    // C# `SellItemButton.Enabled`：价格合法才可提交；禁用态按 `GrayScale` 真灰度（批12）
    let allowed = state.allowed() && market.consign_item.is_some();
    for (e, inter, mut gray) in &mut sell_btn {
        let want_gray = !allowed;
        if gray.gray != want_gray {
            gray.gray = want_gray;
        }
        if !edge(e, inter, &mut prev_inter) {
            continue;
        }
        if !allowed {
            market.message = if market.consign_item.is_none() {
                "请先点物品格选择要出售的物品".to_string()
            } else {
                format!("价格无效（{} - {}）", TM_MIN_CONSIGN_PRICE, max)
            };
            continue;
        }
        let uid = market
            .consign_item
            .as_ref()
            .map(|i| i.unique_id)
            .unwrap_or(0);
        net.send_packet(&mir2_shared::packets::client::market::ConsignItem {
            unique_id: uid,
            price,
            panel_type: market.panel,
        });
        market.message = format!("提交寄售/拍卖 uid={} 价格={}", uid, price);
        market.consign_item = None;
        if let Some(t) = input.texts.get_mut(6) {
            t.clear();
        }
        input.active = None;
        // C# `SellItemButton.Click` 末行：`TMerchantDialog(MarketType)` 重开本页签（重新搜索）
        send_market_search(&net, "", ItemType::Nothing, true, 0, 0, market.panel);
    }
    // C# `ItemCell_Click`：再点一次取消；否则取背包选中物放入（并聚焦售价框）
    for (e, inter) in &cell {
        if !edge(e, inter, &mut prev_inter) {
            continue;
        }
        if market.consign_item.is_some() {
            market.consign_item = None;
            // 取消选择 → 解锁（C# `ItemCell_Click` 先解旧 `tempCell`）
            if let Some(slot) = market.consign_slot.take() {
                locked.unlock(InvLockReason::Consign, slot);
            }
            market.message = "已取消选择物品".to_string();
            continue;
        }
        let items = inv_q.single().map(|inv| inv.items.as_slice()).unwrap_or(&[]);
        let Some(sel) = inv_click.selected else {
            market.message = "请先在背包里点选一件物品".to_string();
            continue;
        };
        let Some(item) = items.get(sel).and_then(|s| s.as_ref()) else {
            continue;
        };
        market.consign_item = Some(item.clone());
        // C# `ItemCell_Click`：`tempCell = SelectedCell; tempCell.Locked = true`
        market.consign_slot = Some(sel);
        locked.lock(InvLockReason::Consign, sel);
        market.message = format!("已选择：{}", item.name);
        inv_click.selected = None;
        input.active = Some(6);
    }
    // C# `CollectSoldButton.Click`：领取已售金币（`Mode = Sold`）+ 刷新
    for (e, inter) in &collect_btn {
        if edge(e, inter, &mut prev_inter) {
            net.send_packet(&crate::network::MarketGetBackWire {
                mode: 1,
                auction_id: 0,
            });
            net.send_packet(&mir2_shared::packets::client::market::MarketRefresh);
            market.message = "领取已售金币".to_string();
        }
    }
    // C# `SellNowButton.Click`：立即售出（仅拍卖页签，需选中行）
    for (e, inter) in &sellnow_btn {
        if !edge(e, inter, &mut prev_inter) {
            continue;
        }
        if let Some(idx) = market.selected {
            let it = &market.listings[idx];
            net.send_packet(&crate::network::MarketSellNowWire {
                auction_id: it.auction_id as u64,
            });
            market.message = format!("立即售出商品 {}", it.auction_id);
        } else {
            market.message = "请先点击选中一个商品".to_string();
        }
    }
}

/// 寄售目标格渲染（C# `ItemCell`：显示已选物品图标/数量）
#[allow(clippy::too_many_arguments)]
fn market_row_system(
    mgr: Res<DialogManager>,
    market: Res<MarketState>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<UiImageCache>,
    mut rows: Query<(&MarketAuctionRow, &mut Visibility), Without<MarketRowBorder>>,
    mut icons: Query<(&MarketRowIcon, &mut ImageNode, &mut Node)>,
    mut labels: Query<(&MarketRowText, &mut Text, &mut TextColor)>,
    mut borders: Query<(&MarketRowBorder, &mut Visibility), Without<MarketAuctionRow>>,
    mut bottom_btns: Query<(&MarketBottomBtn, &mut UiGray), Without<MarketRowIcon>>,
) {
    if !mgr.is_open(DialogKind::Market) {
        return;
    }
    let user_mode = matches!(
        market.panel,
        MarketPanelType::Consign | MarketPanelType::Auction
    );
    let selected = market.selected;
    // 行显隐（C# `Rows[i].Clear()`：无数据 → Visible=false）
    for (row, mut vis) in &mut rows {
        let show = row_listing_index(&market, row.0).is_some();
        *vis = if show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    // 图标（C# `IconImage`：count>0 用 `Items[Image]`，否则 `Prguse[540]`；按 IconArea 居中）
    for (icon, mut node_img, mut node) in &mut icons {
        let Some(item) = row_listing_index(&market, icon.0).and_then(|i| market.listings.get(i))
        else {
            continue;
        };
        let handle = if item.count > 0 {
            ui_image(
                &mut libs,
                &mut images,
                &mut cache,
                LibraryName::Items,
                item.image as usize,
            )
        } else {
            ui_image(
                &mut libs,
                &mut images,
                &mut cache,
                LibraryName::Prguse,
                TM_ROW_PLACEHOLDER_FRAME,
            )
        };
        let Some(handle) = handle else {
            continue;
        };
        if node_img.image != handle {
            node_img.image = handle.clone();
        }
        let (iw, ih) = images
            .get(&handle)
            .map(|i| {
                let s = i.size();
                (s.x.max(1) as f32, s.y.max(1) as f32)
            })
            .unwrap_or((1.0, 1.0));
        let left = (TM_ROW_ICON_W - iw) / 2.0;
        let top = (TM_ROW_ICON_H - ih) / 2.0;
        if node.left != Val::Px(left) {
            node.left = Val::Px(left);
        }
        if node.top != Val::Px(top) {
            node.top = Val::Px(top);
        }
        if node.width != Val::Px(iw) {
            node.width = Val::Px(iw);
        }
        if node.height != Val::Px(ih) {
            node.height = Val::Px(ih);
        }
    }
    // 文本（C# `AuctionRow.Update` 的名称/价格/卖家/到期）
    for (label, mut text, mut color) in &mut labels {
        let text_new = match row_listing_index(&market, label.0)
            .and_then(|i| market.listings.get(i))
        {
            Some(item) => match label.1 {
                MarketRowTextKind::Name => Some((item.name.clone(), row_name_color(item.grade))),
                MarketRowTextKind::Price => Some((
                    row_price_text(item.price, item.item_type),
                    row_price_color(item.price),
                )),
                MarketRowTextKind::Seller => {
                    Some((item.seller.clone(), row_seller_color(&item.seller, user_mode)))
                }
                MarketRowTextKind::Expire => Some((
                    row_expire_text(item.consignment_date),
                    Color::WHITE,
                )),
            },
            None => None,
        };
        match text_new {
            Some((s, c)) => {
                if text.0 != s {
                    text.0 = s;
                }
                if color.0 != c {
                    color.0 = c;
                }
            }
            None => {
                if !text.0.is_empty() {
                    text.0.clear();
                }
            }
        }
    }
    // 选中框（C# `Rows[i].Border = Rows[i] == Selected`）
    for (border, mut vis) in &mut borders {
        let show = row_listing_index(&market, border.0)
            .map(|i| selected == Some(i))
            .unwrap_or(false);
        *vis = if show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    // C# `UpdateInterface`（:1005-1033）：选中 → Buy/Mail 可用、CollectSold 变灰；
    // `SellNow` 仅当选中行卖家为 `Bid Met`（拍卖已有人出价）
    let sel = selected.and_then(|i| market.listings.get(i));
    let has_sel = sel.is_some();
    let bid_met = sel.map(|i| i.seller == "Bid Met").unwrap_or(false);
    for (kind, mut gray) in &mut bottom_btns {
        // C# `GrayScale = !Enabled`（:1005-1033）：禁用态按 `grayscale.ps` 灰度绘制
        let want = !market_bottom_enabled(*kind, has_sel, bid_met);
        if gray.gray != want {
            gray.gray = want;
        }
    }
}

/// 寄售目标格渲染（C# `ItemCell`：显示已选物品图标/数量）
#[allow(clippy::too_many_arguments)]
fn market_price_filter_system(
    mgr: Res<DialogManager>,
    mut market: ResMut<MarketState>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<UiImageCache>,
    header_btn: Query<(Entity, &Interaction), With<MarketPriceFilterBtn>>,
    icon_btn: Query<(Entity, &Interaction), With<MarketPriceFilterIcon>>,
    mut icon_img: Query<(&MarketPriceFilterIcon, &mut ImageNode, &mut Visibility)>,
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
    if !mgr.is_open(DialogKind::Market) {
        return;
    }
    // C#：`TitlePriceLabel.Click` 与 `PriceFilterIcon.Click` 都调 `CyclePriceFilter()`
    let mut cycle = false;
    for (e, inter) in &header_btn {
        if edge(e, inter, &mut prev_inter) {
            cycle = true;
        }
    }
    for (e, inter) in &icon_btn {
        if edge(e, inter, &mut prev_inter) {
            cycle = true;
        }
    }
    if cycle {
        market.price_filter = market.price_filter.next();
        market.message = match market.price_filter {
            MarketPriceFilter::Normal => "价格排序：默认".to_string(),
            MarketPriceFilter::Low => "价格排序：从低到高".to_string(),
            MarketPriceFilter::High => "价格排序：从高到低".to_string(),
        };
    }
    // C# `UpdatePriceFilterIcon()`：Normal 隐藏，其余显示对应帧
    let want = market.price_filter.icon_frame().and_then(|idx| {
        ui_image(
            &mut libs,
            &mut images,
            &mut cache,
            LibraryName::Prguse2,
            idx,
        )
    });
    for (_icon, mut node, mut vis) in &mut icon_img {
        match (market.price_filter, want.as_ref()) {
            (MarketPriceFilter::Normal, _) | (_, None) => {
                *vis = Visibility::Hidden;
            }
            (_, Some(handle)) => {
                *vis = Visibility::Visible;
                if node.image != *handle {
                    node.image = handle.clone();
                }
            }
        }
    }
}

/// 写邮件（C# `MailButton.Click`：以选中行的卖家为收件人、按 `InterestedInPurchase` 预填正文）
fn market_mail_system(
    mgr: Res<DialogManager>,
    market: Res<MarketState>,
    mut compose: MessageWriter<crate::game::dialogs::mail::ComposeMail>,
    mail_btn: Query<(Entity, &Interaction), With<MarketMailBtn>>,
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
    if !mgr.is_open(DialogKind::Market) {
        return;
    }
    for (e, inter) in &mail_btn {
        if !edge(e, inter, &mut prev_inter) {
            continue;
        }
        let Some(item) = market.selected.and_then(|i| market.listings.get(i)) else {
            continue;
        };
        compose.write(crate::game::dialogs::mail::ComposeMail {
            to: item.seller.clone(),
            message: Some(market_mail_message(&item.name, item.price)),
        });
        tracing::info!("✉️ 市场写信给 {}", item.seller);
    }
}

/// 寄售目标格渲染（C# `ItemCell`：显示已选物品图标/数量）
fn market_consign_cell_system(
    market: Res<MarketState>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<UiImageCache>,
    mut cells: Query<(&MarketConsignCell, &mut UiItemCellData), Without<UiItemCellIcon>>,
) {
    for (_cell, mut data) in &mut cells {
        match market.consign_item.as_ref() {
            Some(item) => {
                data.icon = if item.image > 0 {
                    ui_image(
                        &mut libs,
                        &mut images,
                        &mut cache,
                        LibraryName::Items,
                        item.image as usize,
                    )
                } else {
                    None
                };
                data.count = (item.count > 1).then_some(item.count as u32);
            }
            None => {
                data.icon = None;
                data.count = None;
            }
        }
        data.dura_ratio = None;
    }
}

/// 执行确认动作（发送对应 C# 客户端包）
fn market_execute_confirm(
    net: &NetConnection,
    action: MarketConfirmAction,
    market: &mut MarketState,
) {
    match action {
        MarketConfirmAction::Buy {
            auction_id,
            bid_price,
        } => {
            net.send_packet(&mir2_shared::packets::client::market::MarketBuy {
                auction_id,
                bid_price,
            });
            market.message = format!("购买商品 {}（bid={}）", auction_id, bid_price);
        }
        MarketConfirmAction::GetBack { auction_id } => {
            net.send_packet(&crate::network::MarketGetBackWire {
                mode: 0,
                auction_id,
            });
            market.message = format!("取回/领取记录 {}", auction_id);
        }
    }
}

/// 确认框：显隐 + 文案 + Yes/No（C# `MirMessageBox` YesNo）
fn market_confirm_system(
    mgr: Res<DialogManager>,
    net: Res<NetConnection>,
    mut confirm: ResMut<MarketConfirm>,
    mut market: ResMut<MarketState>,
    mut widgets: Query<&mut Visibility, With<MarketConfirmWidget>>,
    mut texts: Query<&mut Text, With<MarketConfirmText>>,
    yes: Query<(Entity, &Interaction), (With<MarketConfirmYes>, Without<MarketConfirmNo>)>,
    no: Query<(Entity, &Interaction), (With<MarketConfirmNo>, Without<MarketConfirmYes>)>,
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
    // 市场窗关闭 → 确认框一并收起（C# 模态框随对话框关闭）
    if !mgr.is_open(DialogKind::Market) {
        confirm.visible = false;
        confirm.action = None;
    }
    for mut vis in &mut widgets {
        *vis = if confirm.visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for mut t in &mut texts {
        if t.0 != confirm.text {
            t.0 = confirm.text.clone();
        }
    }
    if !confirm.visible {
        return;
    }
    for (e, inter) in &yes {
        if !edge(e, inter, &mut prev_inter) {
            continue;
        }
        if let Some(action) = confirm.action {
            market_execute_confirm(&net, action, &mut market);
        }
        confirm.visible = false;
        confirm.action = None;
    }
    for (e, inter) in &no {
        if edge(e, inter, &mut prev_inter) {
            confirm.visible = false;
            confirm.action = None;
            market.message = "已取消".to_string();
        }
    }
}

/// 市场动作：Buy 键（C# `BuyButton.Click` :360-440）——
/// 寄售/拍卖页签（UserMode）走 `C.MarketGetBack{AuctionID}`；市场/商城走 `C.MarketBuy`
/// （拍卖行带 `BidPrice`；C# 用 MirAmountBox，Bevy 复用价格框 id 6，缺省 `当前价+1`）
fn market_action_system(
    mgr: Res<DialogManager>,
    mut market: ResMut<MarketState>,
    net: Res<NetConnection>,
    mut confirm: ResMut<MarketConfirm>,
    mut bid: ResMut<MarketBidPending>,
    mut amount: ResMut<crate::game::dialogs::amount_box::AmountBoxState>,
    buy_btn: Query<(Entity, &Interaction), With<MarketBuyBtn>>,
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
    if !mgr.is_open(DialogKind::Market) {
        return;
    }
    for (e, inter) in &buy_btn {
        if !edge(e, inter, &mut prev_inter) {
            continue;
        }
        let Some(idx) = market.selected else {
            market.message = "请先点击选中一个商品".to_string();
            continue;
        };
        let item = market.listings[idx].clone();
        match market_buy_outcome(&item, market.panel) {
            MarketBuyOutcome::Direct(action) => {
                market_execute_confirm(&net, action, &mut market);
            }
            MarketBuyOutcome::Confirm(text, action) => {
                confirm.visible = true;
                confirm.text = text;
                confirm.action = Some(action);
            }
            MarketBuyOutcome::BidAmount => {
                // C# `MirAmountBox(BidAmount, Item.Info.Image, uint.MaxValue, Price + 1, Price + 1)`
                amount.ask_with(
                    "出价金额",
                    Some((LibraryName::Items, item.image as usize)),
                    u32::MAX,
                    item.current_bid.saturating_add(1),
                    item.current_bid.saturating_add(1),
                );
                *bid = MarketBidPending {
                    auction_id: item.auction_id,
                    name: item.name.clone(),
                    min_bid: item.current_bid.saturating_add(1),
                };
            }
        }
    }
}

/// 待确认的拍卖出价（C# `bidAmount.OKButton.Click` → `MirMessageBox(ConfirmBidGoldForItem)`）
#[derive(Resource, Default)]
pub struct MarketBidPending {
    /// 0 = 无待确认出价
    pub auction_id: u64,
    pub name: String,
    pub min_bid: u32,
}

impl MarketBidPending {
    pub fn is_pending(&self) -> bool {
        self.auction_id != 0
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

/// 出价金额确定（`AmountBoxResult`）→ 弹确认框（C# `MirAmountBox` OK → `MirMessageBox` YesNo）。
fn market_bid_amount_system(
    mut results: MessageReader<crate::game::dialogs::amount_box::AmountBoxResult>,
    mut bid: ResMut<MarketBidPending>,
    mut confirm: ResMut<MarketConfirm>,
) {
    for r in results.read() {
        if !bid.is_pending() {
            continue;
        }
        match r.0 {
            Some(n) if n > 0 => {
                // C# `MinAmount = Price + 1`：低于下限按 C# 钳到下限
                let amount = n.max(bid.min_bid);
                confirm.visible = true;
                confirm.text = market_bid_text(&bid.name, amount);
                confirm.action = Some(MarketConfirmAction::Buy {
                    auction_id: bid.auction_id,
                    bid_price: amount,
                });
                bid.clear();
            }
            _ => bid.clear(),
        }
    }
}


/// 消费服务端市场事件（网络层只广播 ServerEvent；文案在此构造）
/// #2633 批次4 步9：寄售移除背包格直接写 `Inventory` 组件（HudState 已删）；实体未生成跳过（R1）。
fn market_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut market: ResMut<MarketState>,
    mut inv_q: Query<&mut Inventory, With<LocalPlayer>>,
    mut locked: ResMut<InvLockedSlots>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        match ev {
            ServerEvent::MarketPages { pages } => {
                market.pages = *pages;
            }
            ServerEvent::MarketListings { listings } => {
                // C# `GameScene.NPCMarketPage`：`Listings.AddRange(p.Listings)` 后
                // `Page = (Listings.Count - 1) / 10`（累积 + 跳到刚到的页）。
                // 回包不带页号：有未决请求 → 该页；否则视为新一轮搜索的第 0 页。
                let page = market.pending_page.take().unwrap_or(0);
                let items: Vec<MarketItem> = listings
                    .iter()
                    .map(|e| {
                        let name = if !e.item.name.is_empty() {
                            e.item.name.clone()
                        } else {
                            market
                                .item_names
                                .get(&e.item.item_index)
                                .cloned()
                                .unwrap_or_else(|| format!("#{}", e.item.item_index))
                        };
                        MarketItem {
                            auction_id: e.auction_id,
                            unique_id: e.unique_id,
                            name,
                            item_index: e.item.item_index,
                            image: e.item.image,
                            grade: e.item.grade,
                            count: e.item.count,
                            seller: e.seller.clone(),
                            price: e.price,
                            item_type: e.item_type,
                            current_bid: e.current_bid,
                            consignment_date: e.consignment_date,
                        }
                    })
                    .collect();
                accumulate_market_page(&mut market, page, items);
            }
            ServerEvent::MarketConsign { uid, success } => {
                // #2742：C# `GameScene.ConsignItem`（:5655-5667）在此解锁 `tempCell`
                if let Some(slot) = market.consign_slot.take() {
                    locked.unlock(InvLockReason::Consign, slot);
                }
                if *success {
                    // #720：寄售成功从背包移除（C# S.ConsignItem 语义）
                    if let Ok(mut inv) = inv_q.single_mut() {
                        if let Some(idx) = inv
                            .items
                            .iter()
                            .position(|s| s.as_ref().map(|it| it.unique_id) == Some(*uid))
                        {
                            inv.items[idx] = None;
                            tracing::info!("🏪 寄售成功，背包移除 uid={}", uid);
                        }
                    }
                    market.consign_ok = Some(*uid);
                    market.message = format!("寄售成功 uid={}", uid);
                } else {
                    market.message = "寄售失败".to_string();
                }
            }
            ServerEvent::MarketSuccess { message } => {
                market.message = message.clone();
            }
            ServerEvent::MarketFail { reason } => {
                market.message = format!("市场操作失败（原因 {}）", reason);
            }
            ServerEvent::UserInformation { item_names, .. } => {
                for (idx, name) in item_names {
                    market.item_names.insert(*idx, name.clone());
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    /// 商品行命中：C# `AuctionRow` 行矩形，拖动后跟随面板原点
    #[test]
    fn row_rect_origin_and_drag() {
        // C# 布局：面板 @(0,0)，首行 (127,82)、行高 33、354x32
        let (rx, ry, rw, rh) = market_row_rect(0, TM_POS.0, TM_POS.1);
        assert_eq!((rx, ry, rw, rh), (127.0, 82.0, 354.0, 32.0));
        assert_eq!(
            market_row_rect(9, TM_POS.0, TM_POS.1).1,
            82.0 + 9.0 * 33.0
        );
        // 拖动到 (330,100)：同一相对位置命中跟随（+delta 330,100）
        let (rx2, ry2, _, _) = market_row_rect(0, 330.0, 100.0);
        assert_eq!((rx2, ry2), (457.0, 182.0));
    }

    /// #2720：TrustMerchant 面板/页签/底部栏锚点对齐 C#（TrustMerchantDialog.cs:86-500）
    #[test]
    fn trust_merchant_layout_matches_csharp_anchors() {
        assert_eq!((TM_PANEL_W, TM_PANEL_H), (492.0, 478.0)); // Title[786] 原生尺寸
        assert_eq!(TM_POS, (0.0, 0.0)); // C# 未设 Location
        assert_eq!(TM_CLOSE, (465.0, 3.0));
        assert_eq!(TM_TABS[0], ("market", 9.0, 35.0, 789, 788));
        assert_eq!(TM_TABS[1], ("consign", 104.0, 35.0, 791, 790));
        assert_eq!(TM_TABS[2], ("auction", 199.0, 35.0, 817, 816));
        assert_eq!(TM_TABS[3], ("game_shop", 389.0, 35.0, 819, 818));
        assert_eq!(TM_SEARCH_POS, (11.0, 452.0));
        assert_eq!(TM_FIND_POS, (124.0, 448.0));
        assert_eq!(TM_REFRESH_POS, (320.0, 448.0));
        assert_eq!(TM_BUY_POS, (380.0, 448.0));
        assert_eq!(TM_BACK_POS, (251.0, 419.0));
        assert_eq!(TM_NEXT_POS, (320.0, 419.0));
        assert_eq!(TM_PAGE_POS, (260.0, 419.0));
        // 控件必须落在 492x478 面板内
        let inside = |(x, y): (f32, f32)| x >= 0.0 && y >= 0.0 && x <= TM_PANEL_W && y <= TM_PANEL_H;
        for (_, x, y, _, _) in TM_TABS {
            assert!(inside((x, y)));
        }
        assert!(inside(TM_CLOSE));
        assert!(inside(TM_SEARCH_POS));
        assert!(inside(TM_BUY_POS));
    }

    /// #2720：页签控件显隐映射（C# `TMerchantDialog(type)`：Mail 仅市场、
    /// CollectSold 仅寄售、SellNow 仅拍卖、寄售面板组只在寄售/拍卖）
    #[test]
    fn market_panel_part_visibility_matches_csharp() {
        use MarketForPanel::*;
        use MarketPanelType::*;
        for panel in [Market, Consign, Auction, GameShop] {
            assert_eq!(
                panel_part_visible(ConsignOrAuction, panel),
                matches!(panel, Consign | Auction),
                "寄售面板组 @ {panel:?}"
            );
            assert_eq!(
                panel_part_visible(ConsignOnly, panel),
                panel == Consign,
                "COLLECT @ {panel:?}"
            );
            assert_eq!(
                panel_part_visible(AuctionOnly, panel),
                panel == Auction,
                "SELLNOW @ {panel:?}"
            );
            assert_eq!(
                panel_part_visible(MarketOnly, panel),
                panel == Market,
                "MAIL @ {panel:?}"
            );
        }
    }

    /// #2720：确认框锚点（C# `MirMessageBox` YesNo：Prguse[360] 456x190 居中 + Yes/No Title[206..208]/[210..212]）
    #[test]
    fn market_confirm_box_matches_csharp() {
        assert_eq!(TM_CONFIRM_POS, (284.0, 289.0)); // (1024-456)/2, (768-190)/2
        assert_eq!((TM_CONFIRM_W, TM_CONFIRM_H), (456.0, 190.0));
        assert_eq!(TM_CONFIRM_TEXT_POS, (35.0, 35.0)); // Label 390x110
        assert_eq!(TM_CONFIRM_YES_POS, (260.0, 157.0));
        assert_eq!(TM_CONFIRM_NO_POS, (360.0, 157.0));
        assert_eq!((TM_CONFIRM_BTN_W, TM_CONFIRM_BTN_H), (76.0, 25.0));
    }

    /// #2720：买/取回确认分支（C# `BuyButton.Click` :360-440）
    #[test]
    fn market_buy_outcome_matches_csharp() {
        use MarketPanelType::*;
        let base = MarketItem {
            auction_id: 7,
            name: "屠龙".to_string(),
            price: 12_345,
            item_type: 0,
            seller: "For Sale".to_string(),
            ..Default::default()
        };
        // UserMode + 寄售 `For Sale` → 确认取回（文案 = ItemNotSoldGetBack）
        assert_eq!(
            market_buy_outcome(&base, Consign),
            MarketBuyOutcome::Confirm(
                "屠龙尚未售出，确定要取回它吗？".to_string(),
                MarketConfirmAction::GetBack { auction_id: 7 }
            )
        );
        // UserMode + 非 For Sale（如 `Sold`）→ 直接取回
        let sold = MarketItem {
            seller: "Sold".to_string(),
            ..base.clone()
        };
        assert_eq!(
            market_buy_outcome(&sold, Consign),
            MarketBuyOutcome::Direct(MarketConfirmAction::GetBack { auction_id: 7 })
        );
        // UserMode + 拍卖 `No Bid` → 确认取回
        let auc_nobid = MarketItem {
            item_type: 1,
            seller: "No Bid".to_string(),
            current_bid: 100,
            ..base.clone()
        };
        assert!(matches!(
            market_buy_outcome(&auc_nobid, Auction),
            MarketBuyOutcome::Confirm(_, MarketConfirmAction::GetBack { auction_id: 7 })
        ));
        // 非 UserMode 寄售 → 确认购买（`ConfirmBuyItemWithPrice`：千分位 + 金币）
        assert_eq!(
            market_buy_outcome(&base, Market),
            MarketBuyOutcome::Confirm(
                "确定要以12,345 金币购买屠龙吗？".to_string(),
                MarketConfirmAction::Buy {
                    auction_id: 7,
                    bid_price: 0
                }
            )
        );
        // 非 UserMode 商城 → 货币为「积分」
        match market_buy_outcome(&base, GameShop) {
            MarketBuyOutcome::Confirm(text, _) => {
                assert_eq!(text, "确定要以12,345 积分购买屠龙吗？")
            }
            other => panic!("{other:?}"),
        }
        // 非 UserMode 拍卖 → 先弹 `MirAmountBox`（出价金额），金额确定后再确认
        let auc = MarketItem {
            item_type: 1,
            current_bid: 150,
            ..base.clone()
        };
        assert_eq!(
            market_buy_outcome(&auc, Market),
            MarketBuyOutcome::BidAmount
        );
    }

    /// #2742：C# 拍卖出价走 `MirAmountBox(BidAmount, Item.Info.Image, uint.MaxValue, Price + 1,
    /// Price + 1)`（TrustMerchantDialog.cs:421-436）——金额框默认/下限 = 当前价 + 1，
    /// OK 后弹 `ConfirmBidGoldForItem` 确认；取消不弹确认。
    #[test]
    fn market_bid_uses_amount_box_like_csharp() {
        let mut amount = crate::game::dialogs::amount_box::AmountBoxState::default();
        amount.ask_with("出价金额", None, u32::MAX, 151, 151);
        assert!(amount.visible);
        assert_eq!(amount.value, "151", "C# 默认 `Price + 1`");
        assert_eq!((amount.min, amount.max), (151, u32::MAX));

        // 金额确定 → 弹确认框（低于下限钳到下限；C# `MinAmount`）
        let mut bid = MarketBidPending {
            auction_id: 7,
            name: "屠龙".to_string(),
            min_bid: 151,
        };
        let mut confirm = MarketConfirm::default();
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<crate::game::dialogs::amount_box::AmountBoxResult>();
        app.insert_resource(std::mem::take(&mut bid));
        app.insert_resource(std::mem::take(&mut confirm));
        app.add_systems(Update, market_bid_amount_system);
        app.update();
        app.world_mut()
            .write_message(crate::game::dialogs::amount_box::AmountBoxResult(Some(200)));
        app.update();
        let confirm = app.world().resource::<MarketConfirm>();
        assert!(confirm.visible);
        assert_eq!(confirm.text, "你确定要为屠龙出价200金币吗？");
        assert_eq!(
            confirm.action,
            Some(MarketConfirmAction::Buy {
                auction_id: 7,
                bid_price: 200
            })
        );
        assert!(!app.world().resource::<MarketBidPending>().is_pending());

        // 低于下限 → 钳到 `Price + 1`
        app.world_mut()
            .resource_mut::<MarketBidPending>()
            .auction_id = 7;
        app.world_mut().resource_mut::<MarketBidPending>().min_bid = 151;
        app.world_mut()
            .write_message(crate::game::dialogs::amount_box::AmountBoxResult(Some(100)));
        app.update();
        let confirm = app.world().resource::<MarketConfirm>();
        assert_eq!(
            confirm.action,
            Some(MarketConfirmAction::Buy {
                auction_id: 7,
                bid_price: 151
            })
        );

        // 取消（None）→ 不弹确认，pending 清空
        app.world_mut().resource_mut::<MarketConfirm>().visible = false;
        app.world_mut()
            .resource_mut::<MarketBidPending>()
            .auction_id = 7;
        app.world_mut()
            .write_message(crate::game::dialogs::amount_box::AmountBoxResult(None));
        app.update();
        assert!(!app.world().resource::<MarketConfirm>().visible);
        assert!(!app.world().resource::<MarketBidPending>().is_pending());
    }

    /// #2720：价格排序三态与图标（C# `CyclePriceFilter` / `UpdatePriceFilterIcon`）
    #[test]
    fn market_price_filter_cycles_and_icons() {
        use MarketPriceFilter::*;
        assert_eq!(Normal.next(), Low);
        assert_eq!(Low.next(), High);
        assert_eq!(High.next(), Normal); // 三态循环
        assert_eq!(Normal.icon_frame(), None);
        assert_eq!(Low.icon_frame(), Some(925)); // Prguse2[925]
        assert_eq!(High.icon_frame(), Some(926));
        assert_eq!(MarketPriceFilter::default(), Normal);
        // 图标锚点 = TitlePriceLabel(295,60,88x21) → (295+88-12, 60+(21-14)/2+2) = (371,65)
        assert_eq!(TM_PRICE_HEADER_POS, (295.0, 60.0));
        assert_eq!((TM_PRICE_HEADER_W, TM_PRICE_HEADER_H), (88.0, 21.0));
        assert_eq!(TM_PRICE_ICON_POS, (371.0, 65.0));
        assert_eq!((TM_PRICE_ICON_W, TM_PRICE_ICON_H), (12.0, 11.0));
    }

    /// #2742：C# `UpdateInterface`（TrustMerchantDialog.cs:1005-1033）底栏四键 `Enabled`
    /// （禁用态即 `GrayScale = true` 灰化）。
    #[test]
    fn market_bottom_buttons_gray_when_disabled() {
        use MarketBottomBtn::*;
        // 有选中：Buy/Mail 可用（不灰）、CollectSold 灰化
        assert!(market_bottom_enabled(Buy, true, false));
        assert!(!market_bottom_enabled(CollectSold, true, false));
        assert!(market_bottom_enabled(Mail, true, false));
        // 无选中：相反
        assert!(!market_bottom_enabled(Buy, false, false));
        assert!(market_bottom_enabled(CollectSold, false, false));
        assert!(!market_bottom_enabled(Mail, false, false));
        // SellNow 仅当选中行卖家为 Bid Met
        assert!(!market_bottom_enabled(SellNow, true, false));
        assert!(market_bottom_enabled(SellNow, true, true));
        assert!(!market_bottom_enabled(SellNow, false, true));
    }

    /// #2720：`GetOrderedListings()` 排序（Normal 原序 / Low 升序 / High 降序，稳定）
    #[test]
    fn market_display_order_sorts_by_price() {
        use MarketPriceFilter::*;
        let prices = [500u32, 100, 300, 100];
        assert_eq!(display_order(&prices, Normal), vec![0, 1, 2, 3]);
        assert_eq!(display_order(&prices, Low), vec![1, 3, 2, 0]); // 同价保持原序（稳定）
        assert_eq!(display_order(&prices, High), vec![0, 2, 1, 3]);
        assert!(display_order(&[], Low).is_empty());
        // `row_listing_index`：Normal 直取，Low 取排序后的第 n 个
        let mut market = MarketState::default();
        market.listings = prices
            .iter()
            .map(|p| MarketItem {
                price: *p,
                ..Default::default()
            })
            .collect();
        assert_eq!(row_listing_index(&market, 0), Some(0));
        market.price_filter = Low;
        assert_eq!(row_listing_index(&market, 0), Some(1));
        assert_eq!(row_listing_index(&market, 3), Some(0));
        assert_eq!(row_listing_index(&market, 4), None);
    }

    /// #2736：价格排序必须**跨页**（C# `Listings.AddRange` 后对全量 `Listings` 排序，
    /// `UpdateInterface` 取 `orderedListings[Page*10 + i]`）——第 0 页首行要能显示
    /// 来自第 1 页的最便宜商品，证明不是「只排当前页」。
    #[test]
    fn market_sort_accumulates_across_pages() {
        let mk = |price: u32| MarketItem {
            price,
            ..Default::default()
        };
        let mut market = MarketState::default();
        // 第 0 页（服务器顺序，10 条）
        accumulate_market_page(
            &mut market,
            0,
            vec![
                mk(500),
                mk(100),
                mk(300),
                mk(100),
                mk(900),
                mk(200),
                mk(700),
                mk(150),
                mk(600),
                mk(250),
            ],
        );
        assert_eq!((market.loaded_pages, market.page), (1, 0));
        // 第 1 页 `AddRange`（C# `NPCMarketPage`）
        accumulate_market_page(&mut market, 1, vec![mk(50), mk(800)]);
        assert_eq!((market.listings.len(), market.loaded_pages), (12, 2));
        assert_eq!(market.page, 1, "C# `Page = (Listings.Count - 1) / 10`");

        market.price_filter = MarketPriceFilter::Low;
        // 全量升序下标：50(10) 100(1) 100(3) 150(7) 200(5) 250(9) 300(2) 500(0) 600(8) 700(6) 800(11) 900(4)
        market.page = 0;
        let row0: Vec<Option<usize>> = (0..10).map(|s| row_listing_index(&market, s)).collect();
        assert_eq!(
            row0,
            vec![
                Some(10),
                Some(1),
                Some(3),
                Some(7),
                Some(5),
                Some(9),
                Some(2),
                Some(0),
                Some(8),
                Some(6)
            ]
        );
        assert_eq!(
            market.listings[row_listing_index(&market, 0).unwrap()].price,
            50,
            "第 0 页首行 = 全量最便宜（来自第 1 页）"
        );
        // 第 1 页 = 全量排序的后两条
        market.page = 1;
        assert_eq!(row_listing_index(&market, 0), Some(11));
        assert_eq!(row_listing_index(&market, 1), Some(4));
        assert_eq!(row_listing_index(&market, 2), None);
        // Normal 保持服务器累积顺序
        market.price_filter = MarketPriceFilter::Normal;
        assert_eq!(row_listing_index(&market, 1), Some(11));
    }

    /// #2736：页累积语义（C# `NPCMarket`/`NPCMarketPage`）——第 0 页替换并复位选中，
    /// 后续页续接，缺页忽略。
    #[test]
    fn market_page_accumulation_matches_csharp() {
        let mk = |price: u32| MarketItem {
            price,
            ..Default::default()
        };
        let mut market = MarketState::default();
        accumulate_market_page(&mut market, 0, vec![mk(10)]);
        assert_eq!((market.listings.len(), market.loaded_pages), (1, 1));

        market.selected = Some(0);
        accumulate_market_page(&mut market, 0, vec![mk(20), mk(30)]);
        assert_eq!((market.listings.len(), market.loaded_pages), (2, 1));
        assert_eq!(market.selected, None, "新一轮搜索结果应清空选中");
        assert_eq!(market.page, 0);

        accumulate_market_page(&mut market, 1, vec![mk(40)]);
        assert_eq!((market.listings.len(), market.loaded_pages), (3, 2));
        // 缺页（page > loaded_pages）忽略，不破坏已累积前缀
        accumulate_market_page(&mut market, 3, vec![mk(50)]);
        assert_eq!((market.listings.len(), market.loaded_pages), (3, 2));
        assert_eq!(market.page, 1);
    }

    /// #2736：翻页动作（C# `BackButton.Click` 本地、`NextButton.Click` 已累积则本地，
    /// 否则 `C.MarketPage`）。
    #[test]
    fn market_page_actions_match_csharp() {
        assert_eq!(back_page_action(0), None);
        assert_eq!(back_page_action(2), Some(1));
        // 已累积前缀内 → 本地翻页（无需请求）
        assert_eq!(next_page_action(0, 3, 3), Some((1, false)));
        assert_eq!(next_page_action(1, 3, 3), Some((2, false)));
        // 未累积 → 请求服务器
        assert_eq!(next_page_action(0, 1, 3), Some((1, true)));
        // 末页 / 只有一页 → 不动作
        assert_eq!(next_page_action(2, 3, 3), None);
        assert_eq!(next_page_action(0, 1, 1), None);
    }

    /// #2720：Mail 按钮锚点与正文（C# `MailButton` + `InterestedInPurchase`）
    #[test]
    fn market_mail_button_matches_csharp() {
        assert_eq!(TM_MAIL_POS, (350.0, 448.0));
        assert_eq!((TM_MAIL_W, TM_MAIL_H), (28.0, 25.0)); // Prguse[437..439]
        assert_eq!(
            market_mail_message("屠龙", 12345),
            "我有意购买屠龙，价格为12345。"
        );
        assert_eq!(market_mail_message("#853", 0), "我有意购买#853，价格为0。");
    }

    /// #2720：列表行锚点对齐 C# `AuctionRow`（TrustMerchantDialog.cs:1440-1522）
    #[test]
    fn trust_merchant_auction_row_matches_csharp() {
        assert_eq!((TM_ROW_X, TM_ROW_Y), (127.0, 82.0)); // Location = (127, 82 + i*33)
        assert_eq!((TM_ROW_W, TM_ROW_H), (354.0, 32.0)); // Size
        assert_eq!(TM_ROW_STEP, 33.0);
        assert_eq!((TM_ROW_ICON_W, TM_ROW_ICON_H), (34.0, 32.0)); // IconArea
        assert_eq!(TM_ROW_NAME_POS, (38.0, 8.0));
        assert_eq!(TM_ROW_PRICE_POS, (170.0, 8.0));
        assert_eq!(TM_ROW_SELLER_POS, (256.0, 0.0));
        assert_eq!(TM_ROW_EXPIRE_POS, (256.0, 14.0));
        assert_eq!(TM_ROW_PLACEHOLDER_FRAME, 540); // C# 空数量占位 Prguse[540]
        assert_eq!(TM_ROW_BORDER_COLOR, Color::srgb_u8(200, 100, 0)); // BorderColour
        // 行内元素都在行框内（图标区 + 4 标签；标签用 C# 声明尺寸）
        let inside = |(x, y): (f32, f32), w: f32, h: f32| {
            x >= 0.0 && y >= 0.0 && x + w <= TM_ROW_W && y + h <= TM_ROW_H
        };
        assert!(inside(TM_ROW_NAME_POS, 140.0, 20.0));
        assert!(inside(TM_ROW_PRICE_POS, 178.0, 20.0));
        // 卖家/到期列 C# 声明宽度超过行宽（AutoSize 会按文本收缩），只断言锚点不溢出右侧面板
        assert!(TM_ROW_X + TM_ROW_SELLER_POS.0 < TM_PANEL_W);
        assert!(TM_ROW_Y + TM_ROW_EXPIRE_POS.1 < TM_PANEL_H);
        // 行首 x 与「物品」表头对齐（C# 两者都是 127）；10 行不越出 492x478 面板
        assert_eq!(TM_ROW_X, TM_HEADERS[2].1);
        assert!(TM_ROW_Y + 9.0 * TM_ROW_STEP + TM_ROW_H <= TM_PANEL_H);
        // 行矩形命中（拖动后跟随面板原点）
        let (rx, ry, rw, rh) = market_row_rect(0, 0.0, 0.0);
        assert_eq!((rx, ry, rw, rh), (127.0, 82.0, 354.0, 32.0));
        assert_eq!(market_row_rect(9, 0.0, 0.0).1, 82.0 + 9.0 * 33.0);
        assert_eq!(market_row_rect(0, 330.0, 100.0).0, 457.0);
    }

    /// #2720：行文本/颜色对齐 C# `AuctionRow.Update`（:1565-1607）
    #[test]
    fn market_row_texts_match_csharp() {
        // 千分位 + 拍卖「出价」后缀
        assert_eq!(group_thousands(0), "0");
        assert_eq!(group_thousands(999), "999");
        assert_eq!(group_thousands(1000), "1,000");
        assert_eq!(group_thousands(1_234_567), "1,234,567");
        assert_eq!(row_price_text(1000, 0), "1,000");
        assert_eq!(row_price_text(1000, 1), "1,000 出价");
        // 价格阈值（>10M 红 / >1M 橙 / >100k 草绿 / >10k 天蓝 / 其余白）
        assert_eq!(row_price_color(10_000), Color::WHITE);
        assert_eq!(row_price_color(10_001), Color::srgb(0.0, 0.749, 1.0));
        assert_eq!(row_price_color(100_000), Color::srgb(0.0, 0.749, 1.0));
        assert_eq!(row_price_color(100_001), Color::srgb(0.486, 0.988, 0.0));
        assert_eq!(row_price_color(1_000_000), Color::srgb(0.486, 0.988, 0.0));
        assert_eq!(row_price_color(1_000_001), Color::srgb(1.0, 0.549, 0.0));
        assert_eq!(row_price_color(10_000_001), Color::srgb(1.0, 0.0, 0.0));
        // 名称品质（None=3/Common=4 黄→白，Rare=5 天蓝，Heroic=8 红）
        assert_eq!(row_name_color(3), Color::WHITE);
        assert_eq!(row_name_color(4), Color::WHITE);
        assert_eq!(row_name_color(5), Color::srgb(0.0, 0.749, 1.0));
        assert_eq!(row_name_color(6), Color::srgb(1.0, 0.549, 0.0));
        assert_eq!(row_name_color(7), Color::srgb(0.867, 0.627, 0.867));
        assert_eq!(row_name_color(8), Color::srgb(1.0, 0.0, 0.0));
        // 卖家列：非 UserMode 一律白；UserMode 状态串着色
        assert_eq!(row_seller_color("张三", false), Color::WHITE);
        assert_eq!(row_seller_color("Sold", true), Color::srgb(1.0, 0.843, 0.0));
        assert_eq!(row_seller_color("Expired", true), Color::srgb(1.0, 0.0, 0.0));
        assert_eq!(
            row_seller_color("Bid Met", true),
            Color::srgb(0.486, 0.988, 0.0)
        );
        assert_eq!(row_seller_color("No Bid", true), Color::WHITE);
        assert_eq!(row_seller_color("For Sale", true), Color::WHITE);
        // 到期文本：`dd/MM/yy HH:mm:ss`（17 字符、含 `/` 与 `:`）；无日期留空
        let text = row_expire_text(1_700_000_000);
        assert_eq!(text.len(), 17, "{text}");
        assert!(text.contains('/') && text.contains(':'), "{text}");
        assert_eq!(row_expire_text(0), "");
    }

    /// #2720：寄售/拍卖页签面板锚点对齐 C#
    /// （TrustMerchantDialog.cs:171-180 HelpLabel / :442-480 两键 / :544-579 ItemCell+售价框）
    #[test]
    fn trust_merchant_consign_panel_matches_csharp() {
        assert_eq!(TM_HELP_POS, (8.0, 237.0));
        assert_eq!((TM_HELP_W, TM_HELP_H), (115.0, 205.0));
        assert_eq!(TM_CONSIGN_CELL_POS, (47.0, 104.0));
        // C# `MirItemCell` 默认 Size(36,32)
        assert_eq!((TM_CONSIGN_CELL_W, TM_CONSIGN_CELL_H), (36.0, 32.0));
        assert_eq!(TM_PRICE_POS, (15.0, 165.0));
        assert_eq!((TM_PRICE_W, TM_PRICE_H), (100.0, 18.0));
        // `Title[700..702]` 实测 52x25；`Title[680..682]` 实测 72x25
        assert_eq!(TM_SELL_ITEM_POS, (39.0, 188.0));
        assert_eq!((TM_SELL_BTN_W, TM_SELL_BTN_H), (52.0, 25.0));
        assert_eq!(TM_COLLECT_SOLD_POS, (300.0, 448.0));
        assert_eq!((TM_COLLECT_BTN_W, TM_COLLECT_BTN_H), (72.0, 25.0));
        assert_eq!(TM_SELL_NOW_POS, (324.0, 448.0));
        // C# `Globals`：MinConsignment / MaxConsignment / MaxStartingBid（Shared/Globals.cs:44-48）
        assert_eq!(
            (
                TM_MIN_CONSIGN_PRICE,
                TM_MAX_CONSIGN_PRICE,
                TM_MAX_STARTING_BID
            ),
            (5000, 50_000_000, 50_000)
        );
        // Buy 两套精灵（市场 703..705 / 寄售·拍卖 706..708，同为 84x25）
        assert_eq!(TM_BUY_MARKET_FRAMES, (703, 704, 705));
        assert_eq!(TM_BUY_USER_FRAMES, (706, 707, 708));
        // 面板内
        let inside = |(x, y): (f32, f32), w: f32, h: f32| {
            x >= 0.0 && y >= 0.0 && x + w <= TM_PANEL_W && y + h <= TM_PANEL_H
        };
        assert!(inside(TM_HELP_POS, TM_HELP_W, TM_HELP_H));
        assert!(inside(
            TM_CONSIGN_CELL_POS,
            TM_CONSIGN_CELL_W,
            TM_CONSIGN_CELL_H
        ));
        assert!(inside(TM_PRICE_POS, TM_PRICE_W, TM_PRICE_H));
        assert!(inside(TM_SELL_ITEM_POS, TM_SELL_BTN_W, TM_SELL_BTN_H));
        assert!(inside(
            TM_COLLECT_SOLD_POS,
            TM_COLLECT_BTN_W,
            TM_COLLECT_BTN_H
        ));
        assert!(inside(TM_SELL_NOW_POS, TM_SELL_BTN_W, TM_SELL_BTN_H));
        // CollectSold 与 SellNow 互不重叠（300..372 / 324..376 分属两个页签，不同时显示）
        assert!(TM_COLLECT_SOLD_POS.0 < TM_SELL_NOW_POS.0);
    }

    /// #2720：表头文案随页签变化（C# `TMerchantDialog(type)` :1144-1300）
    #[test]
    fn market_header_texts_match_csharp() {
        use MarketHeader::*;
        use MarketPanelType::*;
        assert_eq!(header_text(Item, Market), "物品");
        assert_eq!(header_text(Item, Consign), "物品");
        assert_eq!(header_text(Price, Market), "价格 / 出价");
        assert_eq!(header_text(Price, Consign), "价格");
        assert_eq!(header_text(Price, Auction), "最高出价");
        assert_eq!(header_text(Price, GameShop), "价格");
        assert_eq!(header_text(Expiry, Market), "卖家 / 到期");
        assert_eq!(header_text(Expiry, Consign), "到期");
        assert_eq!(header_text(Expiry, Auction), "结束日期");
        assert_eq!(header_text(Expiry, GameShop), "");
        // SalePrice/Sell 只在寄售/拍卖页签（C# `Visible = false` + 文案）
        assert_eq!(header_text(SalePrice, Market), "");
        assert_eq!(header_text(SalePrice, Consign), "出售价格");
        assert_eq!(header_text(SalePrice, Auction), "起始出价");
        assert_eq!(header_text(Sell, Consign), "出售物品");
        // 表头锚点（C# Location/Size，居中绘制）
        assert_eq!(TM_HEADERS[0], (SalePrice, 15.0, 142.0, 100.0));
        assert_eq!(TM_HEADERS[1], (Sell, 10.0, 60.0, 110.0));
        assert_eq!(TM_HEADERS[2], (Item, 127.0, 60.0, 166.0));
        assert_eq!(TM_HEADERS[3], (Price, 295.0, 60.0, 88.0));
        assert_eq!(TM_HEADERS[4], (Expiry, 384.0, 60.0, 98.0));
    }

    /// #2720：售价三态（C# `TextBox_TextChanged` :1316-1366）
    /// 寄售 5000..50,000,000（<5000 Red、==50M Orange）；拍卖 0..50,000（==50k Orange）
    #[test]
    fn market_price_state_matches_csharp() {
        use MarketPanelType::*;
        assert_eq!(price_state(Consign, 0), MarketPriceState::Invalid);
        assert_eq!(price_state(Consign, 4999), MarketPriceState::Invalid);
        assert_eq!(price_state(Consign, 5000), MarketPriceState::Valid);
        assert_eq!(
            price_state(Consign, TM_MAX_CONSIGN_PRICE - 1),
            MarketPriceState::Valid
        );
        assert_eq!(
            price_state(Consign, TM_MAX_CONSIGN_PRICE),
            MarketPriceState::Capped
        );
        assert_eq!(price_state(Auction, 0), MarketPriceState::Valid);
        assert_eq!(price_state(Auction, 49_999), MarketPriceState::Valid);
        assert_eq!(price_state(Auction, 50_000), MarketPriceState::Capped);
        assert_eq!(price_state(Auction, 50_001), MarketPriceState::Capped);
        assert!(!price_state(Consign, 100).allowed());
        assert!(price_state(Auction, 100).allowed());
    }

    /// #2720：HelpLabel 文案内嵌 `Globals` 数值（C# :52-64 + Shared/Globals.cs:42-48）
    #[test]
    fn market_help_text_uses_globals() {
        let consign = help_text(MarketPanelType::Consign);
        assert!(consign.contains("寄售费用为每件5000金币"), "{consign}");
        assert!(consign.contains("最长可登记出售7天"), "{consign}");
        assert!(
            consign.contains("售价可设定范围：5000 - 50000000金币"),
            "{consign}"
        );
        let auction = help_text(MarketPanelType::Auction);
        assert!(
            auction.contains("拍卖费用为5000金币，单件起拍价最高为50000金币"),
            "{auction}"
        );
        assert!(auction.contains("最长可登记拍卖7天"), "{auction}");
    }


    use super::*;
    use crate::game::dialogs::inventory::InvItem;
    use crate::network::server_event::ServerEvent;

    fn mk_item(uid: u64) -> InvItem {
        InvItem {
            unique_id: uid,
            ..Default::default()
        }
    }

    /// 寄售成功移除背包格（#2633 批次4 步9：直接写 Inventory 组件，HudState 双写已删）。
    #[test]
    fn market_consign_removes_item_from_component() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<ServerEvent>();
        app.init_resource::<MarketState>();
        // #2742：market_server_events 现在按来源解锁背包格锁 → 需该资源
        app.init_resource::<InvLockedSlots>();
        app.add_systems(Update, market_server_events);
        app.world_mut().spawn((
            LocalPlayer,
            Inventory {
                items: vec![Some(mk_item(11)), Some(mk_item(22)), None],
                ..Default::default()
            },
        ));
        app.update(); // 初始化消息缓冲/系统状态

        // 寄售 uid=22（idx=1）成功 → Inventory 组件同格清空
        app.world_mut()
            .write_message(ServerEvent::MarketConsign { uid: 22, success: true });
        app.update();
        let inv = app
            .world_mut()
            .query_filtered::<&Inventory, With<LocalPlayer>>()
            .iter(app.world())
            .next()
            .cloned()
            .expect("LocalPlayer 应有 Inventory");
        assert!(inv.items[1].is_none(), "背包格 1 应被寄售移除");
        assert!(inv.items[0].is_some(), "背包格 0 应保持（只移除寄售格）");
    }

    /// 寄售失败不移除背包格，Inventory 组件保持不动。
    #[test]
    fn market_consign_fail_keeps_inventory() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<ServerEvent>();
        app.init_resource::<MarketState>();
        // #2742：market_server_events 现在按来源解锁背包格锁 → 需该资源
        app.init_resource::<InvLockedSlots>();
        app.add_systems(Update, market_server_events);
        app.world_mut().spawn((
            LocalPlayer,
            Inventory {
                items: vec![Some(mk_item(11)), None],
                ..Default::default()
            },
        ));
        app.update();

        app.world_mut()
            .write_message(ServerEvent::MarketConsign { uid: 11, success: false });
        app.update();
        let inv = app
            .world_mut()
            .query_filtered::<&Inventory, With<LocalPlayer>>()
            .iter(app.world())
            .next()
            .cloned()
            .expect("LocalPlayer 应有 Inventory");
        assert!(inv.items[0].is_some(), "寄售失败背包应保持");
    }
}

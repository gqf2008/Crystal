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
    self, MarketFilterDownBtn, MarketFilterSprites, MarketFilterUpBtn,
};
use crate::game::dialogs::inventory::{InvClickState, InvItem};
use crate::game::dialogs::text_input::TextInputState;
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::game::player_state::Inventory;
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{shared_cjk_font, ui_image, UiCjkFont, UiFont, UiImageCache};
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
    pub count: u16,
    pub seller: String,
    pub price: u32,
    /// 0=寄售 1=拍卖（C# MarketItemType）
    pub item_type: u8,
    /// 拍卖当前最高出价（寄售=0）
    pub current_bid: u32,
}

/// 市场状态
#[derive(Resource)]
pub struct MarketState {
    pub listings: Vec<MarketItem>,
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
}

impl Default for MarketState {
    fn default() -> Self {
        Self {
            listings: Vec::new(),
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
                market_consign_cell_system,
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
        // 商品列表 10 行 @(130,60+18i)（C# 列表区：左列 x≤120 为筛选树）
        for i in 0..10usize {
            spawn_label(
                p,
                &cjk,
                "",
                TM_LIST_X,
                TM_LIST_Y + i as f32 * 18.0,
                12.0,
                Color::WHITE,
                9,
            )
            .insert(MarketLine(i));
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
                .insert(MarketBuyBtn);
        }
        // 表头标签（C# 5 个 Title*Label，居中；文案随页签变化）
        for (kind, x, y, w) in TM_HEADERS {
            spawn_label_center(p, &cjk, "", x + w / 2.0, y, w, 12.0, Color::WHITE, 9).insert(kind);
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
            .insert((MarketSellItemBtn, MarketForPanel::ConsignOrAuction));
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
            .insert((MarketCollectSoldBtn, MarketForPanel::ConsignOnly));
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
                .insert((MarketSellNowBtn, MarketForPanel::AuctionOnly));
            }
        }
    });
    if let Some(sp) = filter_sprites {
        commands.insert_resource(sp);
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
    // 列表区随 C# 布局右移（左列 x≤120 为筛选树）：行矩形与绘制同源
    (ox + TM_LIST_X, oy + TM_LIST_Y + i as f32 * 18.0, 300.0, 16.0)
}

fn market_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut market: ResMut<MarketState>,
    net: Res<NetConnection>,
    mut input: ResMut<crate::game::dialogs::text_input::TextInputState>,
    close: Query<(Entity, &Interaction), With<MarketClose>>,
    refresh_btn: Query<(Entity, &Interaction), With<MarketRefreshBtn>>,
    search_btn: Query<(Entity, &Interaction), With<MarketSearchBtn>>,
    prev_btn: Query<(Entity, &Interaction), With<MarketPrevBtn>>,
    next_btn: Query<(Entity, &Interaction), With<MarketNextBtn>>,
    mouse: Res<ButtonInput<MouseButton>>,
    ui: (
        Query<&Window>,
        Query<&Node, With<MarketWidget>>,
    ),
    mut widgets: Query<&mut Visibility, With<MarketWidget>>,
    mut lines: Query<(&mut Text, &MarketLine)>,
    mut scroll: Query<&mut UiScrollList, With<MarketWidget>>,
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
        market.panel = MarketPanelType::Market;
        market.filter_index = 0;
        market.filter_sub_index = None;
        market.filter_skip = 0;
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
    for (e, inter) in &close {
        if edge(e, inter, &mut prev_inter) {
            mgr.close(DialogKind::Market);
        }
    }
    // 渲染（#89 滚轮翻页：scroll.offset 行号 ↔ market.page 同步）
    {
        let mut sl = scroll.single_mut();
        if let Ok(sl) = sl.as_mut() {
            sl.set_total(market.pages.max(1) * 10);
            let want = market.page * 10;
            if sl.offset != want {
                sl.offset = want; // 翻页按钮驱动 → 同步滚动条
            }
            let new_page = sl.offset / 10;
            if new_page != market.page {
                // 滚轮驱动 → 翻页并请求服务器
                market.page = new_page;
                net.send_packet(&crate::network::MarketPageWire {
                    page: new_page as u32,
                });
            }
        }
    }
    for (mut text, line) in &mut lines {
        text.0 = match line.0 {
            i if i < 10 => {
                let idx = market.page * 10 + i;
                match market.listings.get(idx) {
                    Some(it) => {
                        // C#：拍卖行显示当前最高出价（Price = CurrentBid）+ “出价”后缀
                        let price_txt = if it.item_type == 1 {
                            format!("{}出价", it.current_bid)
                        } else {
                            format!("{}金币", it.price)
                        };
                        format!(
                            "{:03}: {} x{} {} {}",
                            it.auction_id % 10000,
                            it.name,
                            it.count,
                            it.seller,
                            price_txt
                        )
                    }
                    None => String::new(),
                }
            }
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
                        let idx = market.page * 10 + i;
                        if idx < market.listings.len() {
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
    for (e, inter) in &refresh_btn {
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
    for (e, inter) in &search_btn {
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
    for (e, inter) in &prev_btn {
        if edge(e, inter, &mut prev_inter) {
            if market.page > 0 {
                market.page -= 1;
                net.send_packet(&crate::network::MarketPageWire { page: market.page as u32 });
            }
        }
    }
    for (e, inter) in &next_btn {
        if edge(e, inter, &mut prev_inter) && market.page + 1 < market.pages.max(1) {
            market.page += 1;
            net.send_packet(&crate::network::MarketPageWire { page: market.page as u32 });
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
            MarketForPanel::ConsignOrAuction => matches!(
                market.panel,
                MarketPanelType::Consign | MarketPanelType::Auction
            ),
            MarketForPanel::ConsignOnly => market.panel == MarketPanelType::Consign,
            MarketForPanel::AuctionOnly => market.panel == MarketPanelType::Auction,
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
    inv_q: Query<&Inventory, With<LocalPlayer>>,
    mut price_box: Query<&mut BackgroundColor, With<MarketPriceField>>,
    mut sell_btn: Query<(Entity, &Interaction, &mut ImageNode), With<MarketSellItemBtn>>,
    cell: Query<(Entity, &Interaction), With<MarketConsignCell>>,
    collect_btn: Query<(Entity, &Interaction), With<MarketCollectSoldBtn>>,
    sellnow_btn: Query<(Entity, &Interaction), With<MarketSellNowBtn>>,
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
    let consign_panel = matches!(
        market.panel,
        MarketPanelType::Consign | MarketPanelType::Auction
    );
    if !consign_panel {
        // 离开寄售/拍卖页签：C# `Hide()`/切页签会清掉 `SellItemSlot` 与售价框
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
    // C# `SellItemButton.Enabled`：价格合法才可提交（禁用态用暗化近似 GrayScale）
    let allowed = state.allowed() && market.consign_item.is_some();
    for (e, inter, mut node) in &mut sell_btn {
        node.color = if allowed {
            Color::WHITE
        } else {
            Color::srgb(0.55, 0.55, 0.55)
        };
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

/// 市场动作：Buy 键（C# `BuyButton.Click` :360-440）——
/// 寄售/拍卖页签（UserMode）走 `C.MarketGetBack{AuctionID}`；市场/商城走 `C.MarketBuy`
/// （拍卖行带 `BidPrice`；C# 用 MirAmountBox，Bevy 复用价格框 id 6，缺省 `当前价+1`）
fn market_action_system(
    mgr: Res<DialogManager>,
    mut market: ResMut<MarketState>,
    net: Res<NetConnection>,
    mut input: ResMut<crate::game::dialogs::text_input::TextInputState>,
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
        let it = &market.listings[idx];
        let id = it.auction_id;
        let is_auction = it.item_type == 1;
        let current_bid = it.current_bid;
        // C# UserMode（寄售/拍卖页签）：Buy 键 = 取回物品 / 领取金币
        if matches!(
            market.panel,
            MarketPanelType::Consign | MarketPanelType::Auction
        ) {
            net.send_packet(&crate::network::MarketGetBackWire {
                mode: 0,
                auction_id: id as u64,
            });
            market.message = format!("取回/领取记录 {}", id);
            continue;
        }
        // C#：寄售一口价 BidPrice=0；拍卖出价默认当前价+1（可复用价格框 id 6 自定义）
        let bid_price = if is_auction {
            let typed = input
                .texts
                .get(6)
                .cloned()
                .unwrap_or_default()
                .trim()
                .parse::<u32>()
                .unwrap_or(0);
            if typed > 0 {
                typed
            } else {
                current_bid.saturating_add(1)
            }
        } else {
            0
        };
        net.send_packet(&mir2_shared::packets::client::market::MarketBuy {
            auction_id: id,
            bid_price,
        });
        tracing::info!("🏪 购买商品 {}（bid={}）", id, bid_price);
    }
}


/// 消费服务端市场事件（网络层只广播 ServerEvent；文案在此构造）
/// #2633 批次4 步9：寄售移除背包格直接写 `Inventory` 组件（HudState 已删）；实体未生成跳过（R1）。
fn market_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut market: ResMut<MarketState>,
    mut inv_q: Query<&mut Inventory, With<LocalPlayer>>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        match ev {
            ServerEvent::MarketPages { pages } => {
                market.pages = *pages;
            }
            ServerEvent::MarketListings { listings } => {
                market.listings = listings
                    .iter()
                    .map(|(auction_id, unique_id, item_index, count, info_name, seller, price, item_type, current_bid)| {
                        let name = if !info_name.is_empty() {
                            info_name.clone()
                        } else {
                            market
                                .item_names
                                .get(item_index)
                                .cloned()
                                .unwrap_or_else(|| format!("#{}", item_index))
                        };
                        MarketItem {
                            auction_id: *auction_id,
                            unique_id: *unique_id,
                            name,
                            item_index: *item_index,
                            count: *count,
                            seller: seller.clone(),
                            price: *price,
                            item_type: *item_type,
                            current_bid: *current_bid,
                        }
                    })
                    .collect();
            }
            ServerEvent::MarketConsign { uid, success } => {
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
    /// 商品行命中：初始原点等价于原固定坐标，拖动后跟随面板
    #[test]
    fn row_rect_origin_and_drag() {
        // C# 布局：面板 @(0,0)，列表区首行 (130,60)、行高 18、宽 300
        let (rx, ry, rw, rh) = market_row_rect(0, TM_POS.0, TM_POS.1);
        assert_eq!((rx, ry, rw, rh), (130.0, 60.0, 300.0, 16.0));
        assert_eq!(market_row_rect(9, TM_POS.0, TM_POS.1).1, 60.0 + 9.0 * 18.0);
        // 拖动到 (330,100)：同一相对位置命中跟随（+delta 330,100）
        let (rx2, ry2, _, _) = market_row_rect(0, 330.0, 100.0);
        assert_eq!((rx2, ry2), (460.0, 160.0));
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

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

/// 商城商品（GameShopInfo 写入）
#[derive(Debug, Clone, Default)]
pub struct ShopItem {
    pub item_index: i32,
    pub name: String,
    pub gold_price: u32,
    pub credit_price: u32,
    pub category: String,
    pub stock: i32,
    /// C# `Item.Count`（购买确认文案 `{3}` 用）
    pub count: i32,
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
    /// 搜索关键词（C# GameshopDialog Search，本地按名称过滤）
    pub search: String,
    /// 分类列表（第 0 项 = 全部，C# Filters[22]；服务端 category 去重保序）
    pub categories: Vec<String>,
    /// 当前选中分类（空 = 全部）
    pub category: String,
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
}

impl Default for GameShopState {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            gold: 0,
            item_names: HashMap::new(),
            search: String::new(),
            categories: Vec::new(),
            category: String::new(),
            page: 0,
            qty: [1; 8],
            // C# `GameshopDialog` 构造即 `PaymentTypeGold.Checked = true`（`:195`
            // 与 `PaymentTypeCredit` 默认未勾选）→ 默认金币付款
            pay_type: 1,
            pending: None,
            confirm_text: String::new(),
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

/// 按名称+分类过滤商城商品（C# GameshopDialog Search + Filters：FriendlyName.Contains / category 相等，返回 items 下标）
fn filter_shop_items(items: &[ShopItem], search: &str, category: &str) -> Vec<usize> {
    let kw = search.trim().to_lowercase();
    items
        .iter()
        .enumerate()
        .filter(|(_, it)| {
            if !category.is_empty() && it.category != category {
                return false;
            }
            kw.is_empty() || it.name.to_lowercase().contains(&kw)
        })
        .map(|(i, _)| i)
        .collect()
}

/// 第 i 格当前展示的商品（C# `UpdateShop`：`filteredShop[i + Page*8]`；空格 None）
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
        app.add_systems(Update, shop_server_events.run_if(in_state(AppState::Game)));
        app.add_systems(OnEnter(AppState::Game), spawn_game_shop);
        app.add_systems(OnExit(AppState::Game), cleanup_game_shop);
        app.add_systems(
            Update,
            (game_shop_ui_system, game_shop_pay_system).run_if(in_state(AppState::Game)),
        );
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
                        font: FontSource::Handle(font.clone()),
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
        Query<(
            &mut Text,
            Option<&GameShopCellCount>,
            Option<&GameShopCellQty>,
        )>,
        Query<(&mut Text, &GameShopCat)>,
        Query<&mut Text, With<GameShopPageLabel>>,
    )>,
    buttons: ShopButtons,
    mut cat_scroll: Query<&mut UiScrollList, (With<GameShopWidget>, Without<GameShopCell>)>,
    mut requested: Local<bool>,
    mut prev_inter: Local<std::collections::HashMap<Entity, Interaction>>,
    panel_origin: Query<&Node, With<GameShopWidget>>,
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
            shop.category.clear();
            shop.page = 0;
            shop.qty = [1; 8];
            cat_list.offset = 0;
        }
    }
    let filtered = filter_shop_items(&shop.items, &shop.search, &shop.category);
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
                let n = if it.name.is_empty() {
                    format!("#{}", it.item_index)
                } else {
                    it.name.clone()
                };
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
    let filtered = filter_shop_items(&shop.items, &shop.search, &shop.category);
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
                        name: shop
                            .item_names
                            .get(&it.item_index)
                            .cloned()
                            .unwrap_or_default(),
                        gold_price: it.gold_price,
                        credit_price: it.credit_price,
                        category: it.category.clone(),
                        stock: it.stock,
                        count: it.count,
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
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(name: &str) -> ShopItem {
        ShopItem {
            item_index: 0,
            name: name.to_string(),
            gold_price: 1,
            credit_price: 0,
            category: String::new(),
            stock: 1,
            count: 1,
            can_buy_gold: true,
            can_buy_credit: true,
        }
    }

    fn item_cat(name: &str, category: &str) -> ShopItem {
        ShopItem {
            category: category.to_string(),
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
        assert_eq!(filter_shop_items(&items, "", "药品").len(), 2);
        assert_eq!(filter_shop_items(&items, "", "卷轴").len(), 1);
        assert_eq!(filter_shop_items(&items, "", "不存在").len(), 0);
        // 分类 + 名称 叠加过滤
        assert_eq!(filter_shop_items(&items, "金创", "药品").len(), 1);
        assert_eq!(filter_shop_items(&items, "金创", "卷轴").len(), 0);
    }

    #[test]
    fn shop_search_filters_by_name() {
        let items = vec![item("金创药"), item("太阳水"), item("回城卷")];
        assert_eq!(filter_shop_items(&items, "", "").len(), 3);
        assert_eq!(filter_shop_items(&items, "药", "").len(), 1);
        assert_eq!(filter_shop_items(&items, "水", "").len(), 1);
        assert_eq!(filter_shop_items(&items, "不存在", "").len(), 0);
        assert_eq!(filter_shop_items(&items, "  药  ", "").len(), 1);
        assert_eq!(filter_shop_items(&items, "JINCHUANG", "").len(), 0);
    }

    #[test]
    fn shop_search_returns_original_indices() {
        let items = vec![
            item("金创药"),
            item("太阳水"),
            item("回城卷"),
            item("金创药·大"),
        ];
        let idx = filter_shop_items(&items, "金创药", "");
        assert_eq!(idx, vec![0, 3]);
    }
}

// ============================================================================
// 商城对话框（M35）
// 参考：C# GameshopDialog（Title[411] 背景）+ ServerRust npc.rs GameshopBuy
// 网络：
//   C: GameshopBuy{ item_id=0 → 请求目录；>0 → 购买 }（wire: [item_id u32][quantity u32]）
//   S: GameShopInfo(250) 商品列表 / GameShopStock(251) 库存变化
// 购买成功物品通过邮件送达（服务端 send_mail_received_packet）
// ============================================================================

use std::collections::HashMap;

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
    load_lib_image, spawn_container, spawn_icon_button, spawn_image, spawn_label, spawn_panel,
};

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
}

/// 商城状态
#[derive(Resource)]
pub struct GameShopState {
    pub items: Vec<ShopItem>,
    pub selected: Option<usize>,
    pub gold: u32,
    pub message: String,
    pub item_names: HashMap<i32, String>,
    /// 搜索关键词（C# GameshopDialog Search，本地按名称过滤）
    pub search: String,
    /// 分类列表（第 0 项 = 全部，C# Filters[22]；服务端 category 去重保序）
    pub categories: Vec<String>,
    /// 当前选中分类（空 = 全部）
    pub category: String,
    /// 分类列表翻页（每页 10 行，C# Up/Down/PositionBar）
    pub category_page: usize,
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
            selected: None,
            gold: 0,
            message: String::new(),
            item_names: HashMap::new(),
            search: String::new(),
            categories: Vec::new(),
            category: String::new(),
            category_page: 0,
            // C# `GameshopDialog` 构造即 `PaymentTypeGold.Checked = true`（`:195`
            // 与 `PaymentTypeCredit` 默认未勾选）→ 默认金币付款
            pay_type: 1,
            pending: None,
            confirm_text: String::new(),
        }
    }
}

#[derive(Component)]
pub struct GameShopWidget;

#[derive(Component)]
pub struct GameShopClose;

#[derive(Component)]
pub struct GameShopBuy;

#[derive(Component)]
pub struct GameShopLine(usize);

#[derive(Component)]
pub struct GameShopCat(usize);

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
    let panel = spawn_panel(&mut commands, bg, px, py, 696.0, 476.0, 30);
    commands
        .entity(panel)
        .insert((DialogRoot(DialogKind::GameShop), GameShopWidget));

    commands.entity(panel).with_children(|p| {
        // 标题 Title[26]（C# (18,9)）
        if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Title, 26) {
            spawn_image(p, h, 18.0, 9.0, 103.0, 17.0, 8);
        }
        // 关闭（C# (671,4)）
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 360),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 361),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 362),
        ) {
            spawn_icon_button(p, n, h, pr, 671.0, 4.0, 20.0, 20.0, 10).insert(GameShopClose);
        }
        // 分类页签（C# Filters 区 @(11,102)，每页 10 行）
        for i in 0..10usize {
            spawn_label(
                p,
                &cjk,
                "",
                11.0,
                105.0 + i as f32 * 20.0,
                12.0,
                Color::srgb(0.9, 0.9, 0.9),
                9,
            )
            .insert(GameShopCat(i));
        }
        // 分类翻页（C# Up/Down @(120,103)/(120,421)；本实现行区 105..285）
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
            spawn_icon_button(p, n, h, pr, 120.0, 290.0, 16.0, 14.0, 10).insert(GameShopCatDown);
        }
        // 商品列表 10 行 @(135,90+20i)
        for i in 0..10usize {
            spawn_label(
                p,
                &cjk,
                "",
                135.0,
                90.0 + i as f32 * 20.0,
                12.0,
                Color::WHITE,
                9,
            )
            .insert(GameShopLine(i));
        }
        // 状态行（金币/消息）@(135,270)/(135,288)
        for i in 10..=11usize {
            spawn_label(
                p,
                &cjk,
                "",
                135.0,
                270.0 + (i - 10) as f32 * 18.0,
                12.0,
                Color::srgb(1.0, 0.9, 0.5),
                9,
            )
            .insert(GameShopLine(i));
        }
        // 购买按钮 @(160,305)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 206),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 207),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 208),
        ) {
            spawn_icon_button(p, n, h, pr, 160.0, 305.0, 76.0, 25.0, 10).insert(GameShopBuy);
        }
        // 搜索（C# Search @(540,69) 140x16；TextInput 31）
        spawn_label(p, &cjk, "搜索", 500.0, 67.0, 12.0, Color::WHITE, 9);
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

/// 显隐 + 渲染 + 关闭/购买 + 打开时请求目录
#[allow(clippy::too_many_arguments)]
fn game_shop_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut shop: ResMut<GameShopState>,
    mut input: ResMut<TextInputState>,
    net: Res<NetConnection>,
    close: Query<(Entity, &Interaction), With<GameShopClose>>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    mut widgets: Query<&mut Visibility, With<GameShopWidget>>,
    // #1354：B0001 修复（#1334 商城分类合入后启动 panic）——lines 与 cats 的 &mut Text 需 Without 隔离
    mut lines: Query<(&mut Text, &GameShopLine), Without<GameShopCat>>,
    mut cats: Query<(&mut Text, &GameShopCat), Without<GameShopLine>>,
    cat_up: Query<(Entity, &Interaction), With<GameShopCatUp>>,
    cat_down: Query<(Entity, &Interaction), With<GameShopCatDown>>,
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
    // 搜索同步（C# KeyUp 本地过滤；texts 由 text_input_system 每帧回填）
    if let Some(t) = input.texts.get(31) {
        shop.search = t.clone();
    }
    let filtered = filter_shop_items(&shop.items, &shop.search, &shop.category);
    for (e, inter) in &close {
        if edge(e, inter, &mut prev_inter) {
            mgr.close(DialogKind::GameShop);
        }
    }
    // 渲染（按过滤后的下标，C# Search 语义）
    for (mut text, line) in &mut lines {
        text.0 = match line.0 {
            i if i < 10 => match filtered.get(i).and_then(|&idx| shop.items.get(idx)) {
                Some(it) => format!(
                    "{}: {}  {}金币",
                    it.item_index,
                    if it.name.is_empty() {
                        format!("#{}", it.item_index)
                    } else {
                        it.name.clone()
                    },
                    it.gold_price
                ),
                None => String::new(),
            },
            10 => format!("我的金币: {}", shop.gold),
            11 => shop.message.clone(),
            _ => String::new(),
        };
    }
    // 分类渲染（C# Filters：第 0 项 = 全部；每页 10 行，▶ 标记当前选中）
    let cat_pages = shop.categories.len().div_ceil(10).max(1);
    if shop.category_page >= cat_pages {
        shop.category_page = cat_pages - 1;
    }
    for (mut text, row) in &mut cats {
        let idx = shop.category_page * 10 + row.0;
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
    // 分类翻页（C# UpButton/DownButton）
    for (e, inter) in &cat_up {
        if edge(e, inter, &mut prev_inter) && shop.category_page > 0 {
            shop.category_page -= 1;
        }
    }
    for (e, inter) in &cat_down {
        if edge(e, inter, &mut prev_inter) && shop.category_page + 1 < cat_pages {
            shop.category_page += 1;
        }
    }
    // 行点击选中
    if mouse.just_pressed(MouseButton::Left) {
        if let Ok(window) = windows.single() {
            if let Some(cursor) = window.cursor_position() {
                let (ox, oy) = panel_origin
                    .single()
                    .map(|n| crate::ui::theme::node_origin(n, (164.0, 146.0)))
                    .unwrap_or((164.0, 146.0));
                for i in 0..10usize {
                    let y = oy + 90.0 + i as f32 * 20.0;
                    if cursor.x >= ox + 135.0
                        && cursor.x <= ox + 540.0
                        && cursor.y >= y
                        && cursor.y <= y + 18.0
                    {
                        if let Some(&idx) = filtered.get(i) {
                            shop.selected = Some(idx);
                            let it = &shop.items[idx];
                            tracing::info!(
                                "🛒 选中商品: #{} {} {}金币",
                                it.item_index,
                                it.name,
                                it.gold_price
                            );
                        }
                        break;
                    }
                }
            }
        }
    }
    // 分类点击（x 108..200，行高 20；C# Filters.Click → SetCategories）
    if mouse.just_pressed(MouseButton::Left) {
        if let Ok(window) = windows.single() {
            if let Some(cursor) = window.cursor_position() {
                let (ox, oy) = panel_origin
                    .single()
                    .map(|n| crate::ui::theme::node_origin(n, (164.0, 146.0)))
                    .unwrap_or((164.0, 146.0));
                for i in 0..10usize {
                    let y = oy + 105.0 + i as f32 * 20.0;
                    if cursor.x >= ox + 11.0
                        && cursor.x <= ox + 120.0
                        && cursor.y >= y
                        && cursor.y <= y + 18.0
                    {
                        let idx = shop.category_page * 10 + i;
                        if let Some(c) = shop.categories.get(idx).cloned() {
                            shop.category = c;
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

/// 付款方式 + 余额标签 + 购买确认（C# `GameshopDialog.cs:185-210` 的复选框与
/// `MirGameShopCell.BuyProduct` :189-238 的整条购买流程；#2791 单元②）。
/// 独立系统：`game_shop_ui_system` 已 15 个参数（Bevy 上限 16）。
#[allow(clippy::too_many_arguments)]
fn game_shop_pay_system(
    mgr: Res<DialogManager>,
    mut shop: ResMut<GameShopState>,
    net: Res<NetConnection>,
    mut chat: ResMut<crate::game::chat::ChatState>,
    credit_q: Query<&crate::game::player_state::Credit, With<crate::actor::LocalPlayer>>,
    frames: Res<GameShopPayFrames>,
    buy_btn: Query<(Entity, &Interaction), With<GameShopBuy>>,
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
    // 购买按钮 → C# `MirGameShopCell.BuyProduct`（:189-238）
    for (e, inter) in &buy_btn {
        if !open || !edge(e, inter, &mut prev_inter) {
            continue;
        }
        let Some(idx) = shop.selected else {
            shop.message = "请先点击选中一个商品".to_string();
            continue;
        };
        let item = shop.items[idx].clone();
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
        let quantity: u32 = 1;
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
        shop.pending = Some(ShopPending { p_type, quantity });
        tracing::info!(
            "🛒 确认购买 #{} {} 付款方式={}",
            item.item_index,
            item.name,
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
                if let Some(idx) = shop.selected {
                    let it = shop.items[idx].clone();
                    net.send_packet(&crate::network::GameshopBuyWire {
                        g_index: it.item_index,
                        quantity: p.quantity as u8,
                        p_type: p.p_type,
                    });
                    tracing::info!(
                        "🛒 购买商城商品 #{} {}（付款 {}）",
                        it.item_index,
                        it.name,
                        p.p_type
                    );
                }
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
                shop.category_page = 0;
            }
            ServerEvent::ShopStock { item_id, stock } => {
                shop.message = format!("商品 #{} 库存剩余 {}", item_id, stock);
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
        let mut shop = DialogManager::default();
        shop.open(DialogKind::GameShop);
        world.insert_resource(shop);
        // 购买键（有 GameShopBuy 标记，不是付款复选框）
        let buy = world
            .spawn((
                Button,
                GameShopBuy,
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
        // 点购买键不应改付款方式（默认金币）；且未选中商品 → 仅写面板提示
        assert_eq!(world.resource::<GameShopState>().pay_type, 1);
        assert_eq!(
            world.resource::<GameShopState>().message,
            "请先点击选中一个商品"
        );
        let _ = (buy, gold, credit);

        // 点积分复选框 → 切换为积分
        world.entity_mut(credit).insert(Interaction::Pressed);
        world.entity_mut(buy).insert(Interaction::None);
        world
            .run_system_once(game_shop_pay_system)
            .expect("game_shop_pay_system 应成功");
        assert_eq!(world.resource::<GameShopState>().pay_type, 0);
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

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

use crate::actor::LocalPlayer;
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::game::player_state::Inventory;
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::UiFont;
use crate::ui::theme::{
    load_lib_image, spawn_container, spawn_icon_button, spawn_label, spawn_panel,
    spawn_scroll_bar_ui, UiScrollList,
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
#[derive(Resource, Default)]
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
}

#[derive(Component)]
pub struct MarketWidget;

#[derive(Component)]
pub struct MarketClose;

#[derive(Component)]
pub struct MarketRefreshBtn;

#[derive(Component)]
pub struct MarketSearchBtn;

#[derive(Component)]
pub struct MarketBuyBtn;

#[derive(Component)]
pub struct MarketConsignBtn;

#[derive(Component)]
pub struct MarketGetBackBtn;

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
    mut ui_font: ResMut<UiFont>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();

    // 面板 Prguse[170] @ (280,80)。加宽加高到 320x400：8 按钮 + 2 输入框 + 滚动条
    // 全在面板内（旧 sprite 布局底部按钮 rel y=265-385 悬空 207 高面板外）
    let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 170) else {
        return;
    };
    let panel = spawn_panel(&mut commands, bg, 280.0, 80.0, 320.0, 400.0, 30);
    commands.entity(panel).insert((
        DialogRoot(DialogKind::Market),
        MarketWidget,
        // #89 市场列表滚轮翻页：1 格 = 1 页（10 行）
        UiScrollList {
            rect_rel: (15.0, 40.0, 200.0, 180.0),
            row_h: 18.0,
            visible: 10,
            total: 0,
            offset: 0,
            step: 10,
            track_rel: (215.0, 40.0, 4.0, 180.0),
            thumb: None,
            z: 9,
        },
    ));

    commands.entity(panel).with_children(|p| {
        // 滚动条（面板子节点）
        spawn_scroll_bar_ui(p, (215.0, 40.0, 4.0, 180.0), 9);
        // 关闭 Prguse2[360/361/362] @(300,3)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 360),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 361),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 362),
        ) {
            spawn_icon_button(p, n, h, pr, 300.0, 3.0, 20.0, 20.0, 10).insert(MarketClose);
        }
        // 商品列表 10 行 @(15,40+18i)
        for i in 0..10usize {
            spawn_label(p, &font, "", 15.0, 40.0 + i as f32 * 18.0, 12.0, Color::WHITE, 9)
                .insert(MarketLine(i));
        }
        // 页签 + 消息行 @(15,225+18i)
        for i in 10..=11usize {
            spawn_label(p, &font, "", 15.0, 225.0 + (i - 10) as f32 * 18.0, 12.0, Color::srgb(1.0, 0.9, 0.5), 9)
                .insert(MarketLine(i));
        }
        // 按钮行 1：刷新/搜索/购买 @(20/110/200,265)；行 2：寄售/取回/立即售出 @(20/110/200,300)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 206),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 207),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 208),
        ) {
            spawn_icon_button(p, n.clone(), h.clone(), pr.clone(), 20.0, 265.0, 76.0, 25.0, 10)
                .insert(MarketRefreshBtn);
            spawn_icon_button(p, n.clone(), h.clone(), pr.clone(), 110.0, 265.0, 76.0, 25.0, 10)
                .insert(MarketSearchBtn);
            spawn_icon_button(p, n, h, pr, 20.0, 300.0, 76.0, 25.0, 10).insert(MarketConsignBtn);
        }
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 210),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 211),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 212),
        ) {
            spawn_icon_button(p, n.clone(), h.clone(), pr.clone(), 200.0, 265.0, 76.0, 25.0, 10)
                .insert(MarketBuyBtn);
            spawn_icon_button(p, n.clone(), h.clone(), pr.clone(), 110.0, 300.0, 76.0, 25.0, 10)
                .insert(MarketGetBackBtn);
            spawn_icon_button(p, n, h, pr, 200.0, 300.0, 76.0, 25.0, 10).insert(MarketSellNowBtn);
        }
        // 翻页 @(20/40,335)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 197),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 198),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 199),
        ) {
            spawn_icon_button(p, n, h, pr, 20.0, 335.0, 16.0, 14.0, 10).insert(MarketPrevBtn);
        }
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 207),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 208),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 209),
        ) {
            spawn_icon_button(p, n, h, pr, 40.0, 335.0, 16.0, 14.0, 10).insert(MarketNextBtn);
        }
        // 搜索/价格输入框（TextInput 5/6）@(20/180,365)
        spawn_market_input(p, &mut images, &font, 5, 20.0, 365.0, 120.0, 300.0, 445.0);
        spawn_market_input(p, &mut images, &font, 6, 180.0, 365.0, 120.0, 460.0, 445.0);
    });
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
) {
    spawn_container(parent, x, y, w, 20.0, 10)
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
        });
    let _ = images;
}

/// 显隐 + 渲染 + 按钮
#[allow(clippy::too_many_arguments)]
/// 商品行命中矩形（面板原点 ox/oy + 相对坐标；i 0..10）
fn market_row_rect(i: usize, ox: f32, oy: f32) -> (f32, f32, f32, f32) {
    // 宽度=右界−左界（620−295）：旧实现误把绝对右界当宽度，命中带右扩 15px
    (ox + 15.0, oy + 40.0 + i as f32 * 18.0, 325.0, 16.0)
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
    // 打开瞬间刷新市场
    if !*requested {
        *requested = true;
        net.send_packet(&mir2_shared::packets::client::market::MarketRefresh);
        tracing::info!("🏪 刷新市场");
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
            net.send_packet(&mir2_shared::packets::client::market::MarketRefresh);
            tracing::info!("🏪 刷新市场");
        }
    }
    // 搜索（C# TrustMerchantDialog FindButton → MarketSearch{Match}，名称子串；纯数字兼容编号）
    for (e, inter) in &search_btn {
        if edge(e, inter, &mut prev_inter) {
            let kw = input.texts.get(5).cloned().unwrap_or_default().trim().to_string();
            net.send_packet(&crate::network::MarketSearchWire { keyword: kw.clone() });
            tracing::info!("🏪 搜索市场: {}", kw);
            input.texts[5].clear();
            input.active = None;
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

/// 市场动作：购买 / 寄售 / 取回 / 立即售出（独立系统避免 Bevy 16 参数上限）
#[allow(clippy::too_many_arguments)]
fn market_action_system(
    mgr: Res<DialogManager>,
    mut market: ResMut<MarketState>,
    net: Res<NetConnection>,
    mut input: ResMut<crate::game::dialogs::text_input::TextInputState>,
    inv_q: Query<&Inventory, With<LocalPlayer>>,
    inv_click: Res<crate::game::dialogs::inventory::InvClickState>,
    buy_btn: Query<(Entity, &Interaction), With<MarketBuyBtn>>,
    consign_btn: Query<(Entity, &Interaction), With<MarketConsignBtn>>,
    getback_btn: Query<(Entity, &Interaction), With<MarketGetBackBtn>>,
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
    // 购买（选中行）
    for (e, inter) in &buy_btn {
        if edge(e, inter, &mut prev_inter) {
            if let Some(idx) = market.selected {
                let it = &market.listings[idx];
                let id = it.auction_id;
                // C#：寄售一口价 BidPrice=0；拍卖出价默认当前价+1（可复用价格输入框 id6 自定义）
                let bid_price = if it.item_type == 1 {
                    let typed = input
                        .texts
                        .get(6)
                        .cloned()
                        .unwrap_or_default()
                        .trim()
                        .parse::<u32>()
                        .unwrap_or(0);
                    if typed > 0 { typed } else { it.current_bid.saturating_add(1) }
                } else {
                    0
                };
                net.send_packet(&mir2_shared::packets::client::market::MarketBuy { auction_id: id, bid_price });
                tracing::info!("🏪 购买商品 {}（bid={}）", id, bid_price);
            } else {
                market.message = "请先点击选中一个商品".to_string();
            }
        }
    }
    // 寄售（选中背包物品 + 价格）
    let items = inv_q.single().map(|inv| inv.items.as_slice()).unwrap_or(&[]);
    for (e, inter) in &consign_btn {
        if edge(e, inter, &mut prev_inter) {
            let price = input
                .texts
                .get(6)
                .cloned()
                .unwrap_or_default()
                .trim()
                .parse::<u32>()
                .unwrap_or(0);
            let idx = inv_click
                .selected
                .filter(|i| items.get(*i).and_then(|s| s.as_ref()).is_some())
                .or_else(|| items.iter().position(|s| s.is_some()));
            if let Some(i) = idx {
                if let Some(item) = items.get(i).and_then(|s| s.as_ref()) {
                    if price == 0 {
                        market.message = "价格无效".to_string();
                        continue;
                    }
                    net.send_packet(&mir2_shared::packets::client::market::ConsignItem {
                        unique_id: item.unique_id,
                        price,
                        panel_type: mir2_shared::enums::MarketPanelType::Consign,
                    });
                    tracing::info!(
                        "🏪 寄售物品 [{}] uid={} 价格={}",
                        item.name,
                        item.unique_id,
                        price
                    );
                    input.texts[6].clear();
                    input.active = None;
                }
            } else {
                market.message = "背包没有可寄售的物品".to_string();
            }
        }
    }
    // 取回（选中行）
    for (e, inter) in &getback_btn {
        if edge(e, inter, &mut prev_inter) {
            if let Some(idx) = market.selected {
                let id = market.listings[idx].auction_id;
                // C# C.MarketGetBack：Mode=Any(0)（服务端按记录状态取回物品/领取金币）
                net.send_packet(&crate::network::MarketGetBackWire { mode: 0, auction_id: id as u64 });
                tracing::info!("🏪 取回商品 {}", id);
            } else {
                market.message = "请先点击选中一个商品".to_string();
            }
        }
    }
    // 立即售出（选中行）
    for (e, inter) in &sellnow_btn {
        if edge(e, inter, &mut prev_inter) {
            if let Some(idx) = market.selected {
                let it = &market.listings[idx];
                // C# C.MarketSellNow：仅 AuctionID
                net.send_packet(&crate::network::MarketSellNowWire {
                    auction_id: it.auction_id as u64,
                });
                tracing::info!("🏪 立即售出商品 {}", it.auction_id);
            } else {
                market.message = "请先点击选中一个商品".to_string();
            }
        }
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
        // 初始 (280,80)：首行 y=120（=80+40），x 起 295（=280+15）
        let (rx, ry, rw, rh) = market_row_rect(0, 280.0, 80.0);
        assert_eq!((rx, ry, rw, rh), (295.0, 120.0, 325.0, 16.0));
        assert_eq!(market_row_rect(9, 280.0, 80.0).1, 120.0 + 9.0 * 18.0);
        // 拖动到 (330,100)：同一相对位置命中跟随（+delta 50,20）
        let (rx2, ry2, _, _) = market_row_rect(0, 330.0, 100.0);
        assert_eq!((rx2, ry2), (345.0, 140.0));
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

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

use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::game::hud::HudState;
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiFont, UiImageCache,
};
use crate::ui::scroll_list::{spawn_scroll_bar, ScrollList};

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
            (market_ui_system, market_action_system, ui_button_system)
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
    mut cache: ResMut<UiImageCache>,
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<UiFont>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();

    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 170) {
        let e = spawn_ui_sprite(&mut commands, h, 280.0, 80.0, 6.0, 1.0);
        // #89 市场列表滚轮翻页：1 格 = 1 页（10 行）
        let (track, thumb) = spawn_scroll_bar(&mut commands, &mut images, (495.0, 120.0, 4.0, 180.0), 6.3);
        commands.entity(track).insert((DialogRoot(DialogKind::Market), MarketWidget, Visibility::Visible));
        commands.entity(thumb).insert((
            DialogRoot(DialogKind::Market),
            MarketWidget,
            Visibility::Visible,
        ));
        commands.entity(e).insert((
            DialogRoot(DialogKind::Market),
            MarketWidget,
            Visibility::Hidden,
            ScrollList {
                rect_rel: (15.0, 40.0, 200.0, 180.0),
                row_h: 18.0,
                visible: 10,
                total: 0,
                offset: 0,
                step: 10,
                track_rel: (215.0, 40.0, 4.0, 180.0),
                thumb: Some(thumb),
                z: 8.0,
            },
        ));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 360, 361, 362,
        280.0 + 300.0, 83.0, 7.0, 20.0, 20.0,
    ) {
        commands.entity(e).insert((
            MarketClose,
            DialogRoot(DialogKind::Market),
            MarketWidget,
        ));
    }
    // 商品列表 10 行
    for i in 0..10usize {
        let e = spawn_ui_text(
            &mut commands, &font, "",
            295.0, 120.0 + i as f32 * 18.0,
            12.0, Color::WHITE, 8.0,
        );
        commands.entity(e).insert((
            MarketLine(i),
            DialogRoot(DialogKind::Market),
            MarketWidget,
        ));
    }
    // 页签 + 消息行
    for i in 10..=11usize {
        let e = spawn_ui_text(
            &mut commands, &font, "",
            295.0, 305.0 + (i - 10) as f32 * 18.0,
            12.0, Color::srgb(1.0, 0.9, 0.5), 8.0,
        );
        commands.entity(e).insert((
            MarketLine(i),
            DialogRoot(DialogKind::Market),
            MarketWidget,
        ));
    }
    // 按钮行 1：刷新 / 搜索 / 购买
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 206, 207, 208,
        300.0, 345.0, 8.3, 76.0, 25.0,
    ) {
        commands.entity(e).insert((MarketRefreshBtn, DialogRoot(DialogKind::Market), MarketWidget));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 206, 207, 208,
        390.0, 345.0, 8.3, 76.0, 25.0,
    ) {
        commands.entity(e).insert((MarketSearchBtn, DialogRoot(DialogKind::Market), MarketWidget));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 210, 211, 212,
        480.0, 345.0, 8.3, 76.0, 25.0,
    ) {
        commands.entity(e).insert((MarketBuyBtn, DialogRoot(DialogKind::Market), MarketWidget));
    }
    // 按钮行 2：寄售 / 取回 / 立即售出
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 206, 207, 208,
        300.0, 380.0, 8.3, 76.0, 25.0,
    ) {
        commands.entity(e).insert((MarketConsignBtn, DialogRoot(DialogKind::Market), MarketWidget));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 210, 211, 212,
        390.0, 380.0, 8.3, 76.0, 25.0,
    ) {
        commands.entity(e).insert((MarketGetBackBtn, DialogRoot(DialogKind::Market), MarketWidget));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 210, 211, 212,
        480.0, 380.0, 8.3, 76.0, 25.0,
    ) {
        commands.entity(e).insert((MarketSellNowBtn, DialogRoot(DialogKind::Market), MarketWidget));
    }
    // 翻页
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 197, 198, 199,
        300.0, 415.0, 8.3, 16.0, 14.0,
    ) {
        commands.entity(e).insert((
            MarketPrevBtn,
            DialogRoot(DialogKind::Market),
            MarketWidget,
        ));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 207, 208, 209,
        320.0, 415.0, 8.3, 16.0, 14.0,
    ) {
        commands.entity(e).insert((
            MarketNextBtn,
            DialogRoot(DialogKind::Market),
            MarketWidget,
        ));
    }
    // 搜索/价格输入框
    let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    spawn_market_input(&mut commands, &white, &font, 5, 300.0, 445.0, 120.0);
    spawn_market_input(&mut commands, &white, &font, 6, 460.0, 445.0, 120.0);
}

/// 市场输入框（TextInputField(id) + 子 TextInputDisplay(id)）
fn spawn_market_input(
    commands: &mut Commands,
    white: &Handle<Image>,
    font: &Handle<Font>,
    id: usize,
    x: f32,
    y: f32,
    w: f32,
) {
    let box_e = commands
        .spawn((
            crate::ui::sprite_ui::UiEntity,
            DialogRoot(DialogKind::Market),
            MarketWidget,
            crate::game::dialogs::text_input::TextInputField(id),
            crate::game::dialogs::text_input::TextInputRect(x, y, w, 20.0),
            Sprite {
                image: white.clone(),
                color: Color::srgba(0.2, 0.2, 0.25, 0.9),
                custom_size: Some(Vec2::new(w, 20.0)),
                ..default()
            },
            bevy::sprite::Anchor::TOP_LEFT,
            Transform::from_xyz(x, -y, 8.1),
            Visibility::Hidden,
        ))
        .id();
    commands.entity(box_e).with_children(|p| {
        p.spawn((
            crate::game::dialogs::text_input::TextInputDisplay(id),
            Text2d::new(String::new()),
            bevy::sprite::Anchor::TOP_LEFT,
            TextFont {
                font: FontSource::Handle(font.clone()),
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(Color::srgb(1.0, 1.0, 1.0)),
            Transform::from_xyz(4.0, -2.0, 8.2),
        ));
    });
}

/// 显隐 + 渲染 + 按钮
#[allow(clippy::too_many_arguments)]
fn market_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut market: ResMut<MarketState>,
    net: Res<NetConnection>,
    mut input: ResMut<crate::game::dialogs::text_input::TextInputState>,
    close: Query<&UiButton, With<MarketClose>>,
    refresh_btn: Query<&UiButton, With<MarketRefreshBtn>>,
    search_btn: Query<&UiButton, With<MarketSearchBtn>>,
    prev_btn: Query<&UiButton, With<MarketPrevBtn>>,
    next_btn: Query<&UiButton, With<MarketNextBtn>>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    mut widgets: Query<&mut Visibility, With<MarketWidget>>,
    mut lines: Query<(&mut Text2d, &MarketLine)>,
    mut scroll: Query<&mut ScrollList, With<MarketWidget>>,
    mut requested: Local<bool>,
) {
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
    for btn in &close {
        if btn.clicked {
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
                    Some(it) => format!(
                        "{:03}: {} x{} {} {}金币",
                        it.auction_id % 10000,
                        it.name,
                        it.count,
                        it.seller,
                        it.price
                    ),
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
        if let Ok(window) = windows.single() {
            if let Some(cursor) = window.cursor_position() {
                for i in 0..10usize {
                    let y = 120.0 + i as f32 * 18.0;
                    if cursor.x >= 295.0 && cursor.x <= 620.0 && cursor.y >= y && cursor.y <= y + 16.0 {
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
    for btn in &refresh_btn {
        if btn.clicked {
            net.send_packet(&mir2_shared::packets::client::market::MarketRefresh);
            tracing::info!("🏪 刷新市场");
        }
    }
    // 搜索（item_index）
    for btn in &search_btn {
        if btn.clicked {
            let idx = input.texts.get(5).cloned().unwrap_or_default().trim().parse::<u32>().unwrap_or(0);
            net.send_packet(&crate::network::MarketSearchWire { item_index: idx });
            tracing::info!("🏪 搜索物品 #{}", idx);
            input.texts[5].clear();
            input.active = None;
        }
    }
    // 翻页
    for btn in &prev_btn {
        if btn.clicked {
            if market.page > 0 {
                market.page -= 1;
                net.send_packet(&crate::network::MarketPageWire { page: market.page as u32 });
            }
        }
    }
    for btn in &next_btn {
        if btn.clicked && market.page + 1 < market.pages.max(1) {
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
    hud: Res<crate::game::hud::HudState>,
    inv_click: Res<crate::game::dialogs::inventory::InvClickState>,
    buy_btn: Query<&UiButton, With<MarketBuyBtn>>,
    consign_btn: Query<&UiButton, With<MarketConsignBtn>>,
    getback_btn: Query<&UiButton, With<MarketGetBackBtn>>,
    sellnow_btn: Query<&UiButton, With<MarketSellNowBtn>>,
) {
    if !mgr.is_open(DialogKind::Market) {
        return;
    }
    // 购买（选中行）
    for btn in &buy_btn {
        if btn.clicked {
            if let Some(idx) = market.selected {
                let id = market.listings[idx].auction_id;
                net.send_packet(&mir2_shared::packets::client::market::MarketBuy { auction_id: id, bid_price: 0 });
                tracing::info!("🏪 购买商品 {}", id);
            } else {
                market.message = "请先点击选中一个商品".to_string();
            }
        }
    }
    // 寄售（选中背包物品 + 价格）
    for btn in &consign_btn {
        if btn.clicked {
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
                .filter(|i| hud.inventory.items.get(*i).and_then(|s| s.as_ref()).is_some())
                .or_else(|| hud.inventory.items.iter().position(|s| s.is_some()));
            if let Some(i) = idx {
                if let Some(item) = hud.inventory.items.get(i).and_then(|s| s.as_ref()) {
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
    for btn in &getback_btn {
        if btn.clicked {
            if let Some(idx) = market.selected {
                let id = market.listings[idx].auction_id;
                net.send_packet(&crate::network::MarketGetBackWire { listing_id: id as u32 });
                tracing::info!("🏪 取回商品 {}", id);
            } else {
                market.message = "请先点击选中一个商品".to_string();
            }
        }
    }
    // 立即售出（选中行）
    for btn in &sellnow_btn {
        if btn.clicked {
            if let Some(idx) = market.selected {
                let it = &market.listings[idx];
                net.send_packet(&crate::network::MarketSellNowWire {
                    unique_id: it.item_index as u32,
                    price: it.price,
                });
                tracing::info!("🏪 立即售出商品 {}", it.auction_id);
            } else {
                market.message = "请先点击选中一个商品".to_string();
            }
        }
    }
}


/// 消费服务端市场事件（网络层只广播 ServerEvent；文案在此构造）
fn market_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut market: ResMut<MarketState>,
    mut hud: ResMut<HudState>,
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
                    .map(|(auction_id, unique_id, item_index, count, info_name, seller, price)| {
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
                        }
                    })
                    .collect();
            }
            ServerEvent::MarketConsign { uid, success } => {
                if *success {
                    // #720：寄售成功从背包移除（C# S.ConsignItem 语义）
                    if let Some(idx) = hud
                        .inventory
                        .items
                        .iter()
                        .position(|s| s.as_ref().map(|it| it.unique_id) == Some(*uid))
                    {
                        hud.inventory.items[idx] = None;
                        tracing::info!("🏪 寄售成功，背包移除 uid={}", uid);
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

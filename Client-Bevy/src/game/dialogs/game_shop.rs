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
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiEntity, UiFont,
    UiImageCache,
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
}

/// 商城状态
#[derive(Resource, Default)]
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
                app.add_systems(
            Update,
            shop_server_events.run_if(in_state(AppState::Game)),
        );
app.add_systems(OnEnter(AppState::Game), spawn_game_shop);
        app.add_systems(OnExit(AppState::Game), cleanup_game_shop);
        app.add_systems(
            Update,
            (game_shop_ui_system, ui_button_system)
                .chain()
                .run_if(in_state(AppState::Game)),
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
    mut cache: ResMut<UiImageCache>,
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<UiFont>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();

    // 背景 Title[411]（C# GameshopDialog.Index=749 Title 库；placeholder 用 411）
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Title, 411) {
        let e = spawn_ui_sprite(&mut commands, h, 200.0, 80.0, 6.0, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::GameShop),
            GameShopWidget,
            Visibility::Hidden,
        ));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 360, 361, 362,
        200.0 + 330.0, 83.0, 7.0, 20.0, 20.0,
    ) {
        commands.entity(e).insert((
            GameShopClose,
            DialogRoot(DialogKind::GameShop),
            GameShopWidget,
        ));
    }
    // 商品列表 10 行
    for i in 0..10usize {
        let e = spawn_ui_text(
            &mut commands, &font, "",
            208.0, 130.0 + i as f32 * 20.0,
            12.0, Color::WHITE, 8.0,
        );
        commands.entity(e).insert((
            GameShopLine(i),
            DialogRoot(DialogKind::GameShop),
            GameShopWidget,
        ));
    }
    // 分类页签（C# GameshopDialog Filters + Up/Down/PositionBar；第 0 项 = 全部）
    for i in 0..10usize {
        let e = spawn_ui_text(
            &mut commands, &font, "",
            108.0, 130.0 + i as f32 * 20.0,
            12.0, Color::srgb(0.9, 0.9, 0.9), 8.0,
        );
        commands.entity(e).insert((
            GameShopCat(i),
            DialogRoot(DialogKind::GameShop),
            GameShopWidget,
        ));
    }
    // 分类翻页（Prguse2 197/198/199 上，207/208/209 下，16x14）
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 197, 198, 199,
        108.0, 335.0, 8.3, 16.0, 14.0,
    ) {
        commands.entity(e).insert((GameShopCatUp, DialogRoot(DialogKind::GameShop), GameShopWidget));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 207, 208, 209,
        128.0, 335.0, 8.3, 16.0, 14.0,
    ) {
        commands.entity(e).insert((GameShopCatDown, DialogRoot(DialogKind::GameShop), GameShopWidget));
    }
    // 状态行（金币/消息）
    for i in 10..=11usize {
        let e = spawn_ui_text(
            &mut commands, &font, "",
            208.0, 335.0 + (i - 10) as f32 * 18.0,
            12.0, Color::srgb(1.0, 0.9, 0.5), 8.0,
        );
        commands.entity(e).insert((
            GameShopLine(i),
            DialogRoot(DialogKind::GameShop),
            GameShopWidget,
        ));
    }
    // 购买按钮
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 206, 207, 208,
        360.0, 385.0, 8.3, 76.0, 25.0,
    ) {
        commands.entity(e).insert((
            GameShopBuy,
            DialogRoot(DialogKind::GameShop),
            GameShopWidget,
        ));
    }

    // 搜索框（C# GameshopDialog Search：本地按名称过滤，TextInput id 31）
    let white = images.add(crate::map_renderer::make_image(
        vec![255, 255, 255, 255],
        1,
        1,
    ));
    let label = spawn_ui_text(
        &mut commands, &font, "搜索",
        390.0, 106.0, 12.0, Color::WHITE, 8.1,
    );
    commands.entity(label).insert((DialogRoot(DialogKind::GameShop), GameShopWidget));
    let box_e = commands
        .spawn((
            UiEntity,
            DialogRoot(DialogKind::GameShop),
            GameShopWidget,
            TextInputField(31),
            TextInputRect(425.0, 105.0, 115.0, 18.0),
            Sprite {
                image: white.clone(),
                color: Color::srgba(0.2, 0.2, 0.25, 0.9),
                custom_size: Some(Vec2::new(115.0, 18.0)),
                ..default()
            },
            bevy::sprite::Anchor::TOP_LEFT,
            Transform::from_xyz(425.0, -105.0, 8.1),
            Visibility::Hidden,
        ))
        .id();
    commands.entity(box_e).with_children(|p| {
        p.spawn((
            TextInputDisplay(31),
            Text2d::new(String::new()),
            bevy::sprite::Anchor::TOP_LEFT,
            TextFont {
                font: FontSource::Handle(font.clone()),
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(Color::srgb(1.0, 1.0, 1.0)),
            Transform::from_xyz(4.0, -1.0, 8.2),
        ));
    });
}

/// 显隐 + 渲染 + 关闭/购买 + 打开时请求目录
#[allow(clippy::too_many_arguments)]
fn game_shop_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut shop: ResMut<GameShopState>,
    mut input: ResMut<TextInputState>,
    net: Res<NetConnection>,
    close: Query<&UiButton, With<GameShopClose>>,
    buy_btn: Query<&UiButton, With<GameShopBuy>>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    mut widgets: Query<&mut Visibility, With<GameShopWidget>>,
    mut lines: Query<(&mut Text2d, &GameShopLine)>,
    mut cats: Query<(&mut Text2d, &GameShopCat)>,
    cat_up: Query<&UiButton, With<GameShopCatUp>>,
    cat_down: Query<&UiButton, With<GameShopCatDown>>,
    mut requested: Local<bool>,
) {
    let open = mgr.is_open(DialogKind::GameShop);
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
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
            item_id: 0,
            quantity: 0,
        });
        tracing::info!("🛒 请求商城目录");
    }
    // 搜索同步（C# KeyUp 本地过滤；texts 由 text_input_system 每帧回填）
    if let Some(t) = input.texts.get(31) {
        shop.search = t.clone();
    }
    let filtered = filter_shop_items(&shop.items, &shop.search, &shop.category);
    for btn in &close {
        if btn.clicked {
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
                let label = if c.is_empty() { "全部".to_string() } else { c.clone() };
                if *c == shop.category { format!("▶ {}", label) } else { label }
            }
            None => String::new(),
        };
    }
    // 分类翻页（C# UpButton/DownButton）
    for btn in &cat_up {
        if btn.clicked && shop.category_page > 0 {
            shop.category_page -= 1;
        }
    }
    for btn in &cat_down {
        if btn.clicked && shop.category_page + 1 < cat_pages {
            shop.category_page += 1;
        }
    }
    // 行点击选中
    if mouse.just_pressed(MouseButton::Left) {
        if let Ok(window) = windows.single() {
            if let Some(cursor) = window.cursor_position() {
                for i in 0..10usize {
                    let y = 130.0 + i as f32 * 20.0;
                    if cursor.x >= 208.0 && cursor.x <= 540.0 && cursor.y >= y && cursor.y <= y + 18.0 {
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
                for i in 0..10usize {
                    let y = 130.0 + i as f32 * 20.0;
                    if cursor.x >= 108.0 && cursor.x <= 200.0 && cursor.y >= y && cursor.y <= y + 18.0 {
                        let idx = shop.category_page * 10 + i;
                        if let Some(c) = shop.categories.get(idx).cloned() {
                            shop.category = c;
                            tracing::info!("🛒 商城分类: {}", if shop.category.is_empty() { "全部" } else { &shop.category });
                        }
                        break;
                    }
                }
            }
        }
    }
    // 购买选中商品（C#：选中商品 → C.GameshopBuy{g_index, quantity=1}）
    for btn in &buy_btn {
        if btn.clicked {
            if let Some(idx) = shop.selected {
                let it = &shop.items[idx];
                net.send_packet(&crate::network::GameshopBuyWire {
                    item_id: it.item_index as u32,
                    quantity: 1,
                });
                tracing::info!("🛒 购买商城商品 #{} {}", it.item_index, it.name);
            } else {
                shop.message = "请先点击选中一个商品".to_string();
            }
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
                    .map(|(item_index, gold_price, credit_price, category, stock)| ShopItem {
                        item_index: *item_index,
                        name: shop
                            .item_names
                            .get(item_index)
                            .cloned()
                            .unwrap_or_default(),
                        gold_price: *gold_price,
                        credit_price: *credit_price,
                        category: category.clone(),
                        stock: *stock,
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
        }
    }

    fn item_cat(name: &str, category: &str) -> ShopItem {
        ShopItem { category: category.to_string(), ..item(name) }
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

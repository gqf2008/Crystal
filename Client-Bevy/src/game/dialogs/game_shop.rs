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

use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiFont, UiImageCache,
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
}

#[derive(Component)]
pub struct GameShopWidget;

#[derive(Component)]
pub struct GameShopClose;

#[derive(Component)]
pub struct GameShopBuy;

#[derive(Component)]
pub struct GameShopLine(usize);

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
}

/// 显隐 + 渲染 + 关闭/购买 + 打开时请求目录
#[allow(clippy::too_many_arguments)]
fn game_shop_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut shop: ResMut<GameShopState>,
    net: Res<NetConnection>,
    close: Query<&UiButton, With<GameShopClose>>,
    buy_btn: Query<&UiButton, With<GameShopBuy>>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    mut widgets: Query<&mut Visibility, With<GameShopWidget>>,
    mut lines: Query<(&mut Text2d, &GameShopLine)>,
    mut requested: Local<bool>,
) {
    let open = mgr.is_open(DialogKind::GameShop);
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        *requested = false;
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
    for btn in &close {
        if btn.clicked {
            mgr.close(DialogKind::GameShop);
        }
    }
    // 渲染
    for (mut text, line) in &mut lines {
        text.0 = match line.0 {
            i if i < 10 => match shop.items.get(i) {
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
    // 行点击选中
    if mouse.just_pressed(MouseButton::Left) {
        if let Ok(window) = windows.single() {
            if let Some(cursor) = window.cursor_position() {
                for i in 0..10usize {
                    let y = 130.0 + i as f32 * 20.0;
                    if cursor.x >= 208.0 && cursor.x <= 540.0 && cursor.y >= y && cursor.y <= y + 18.0 {
                        if i < shop.items.len() {
                            shop.selected = Some(i);
                            let it = &shop.items[i];
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

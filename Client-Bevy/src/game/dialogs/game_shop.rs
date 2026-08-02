// ============================================================================
// 商城对话框（M9 第 4 批）
// 布局参考：C# GameshopDialog / macroquad game_shop_dialog
//   - 背景 Title[411] 风格；商品列表 + 购买
// 网络：GameShopInfo/GameShopStock → 商品列表
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiFont, UiImageCache,
};

/// 商城商品
#[derive(Debug, Clone, Default)]
pub struct ShopItem {
    pub name: String,
    pub price: u32,
}

/// 商城状态（网络 GameShopInfo 等写入）
#[derive(Resource, Default)]
pub struct GameShopState {
    pub items: Vec<ShopItem>,
}

#[derive(Component)]
pub struct GameShopWidget;

#[derive(Component)]
pub struct GameShopClose;

#[derive(Component)]
pub struct GameShopLine(usize);

pub struct GameShopPlugin;

impl Plugin for GameShopPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameShopState>();
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
    for i in 0..8usize {
        let e = spawn_ui_text(
            &mut commands, &font, "",
            208.0, 140.0 + i as f32 * 22.0,
            12.0, Color::WHITE, 8.0,
        );
        commands.entity(e).insert((
            GameShopLine(i),
            DialogRoot(DialogKind::GameShop),
            GameShopWidget,
        ));
    }
}

fn game_shop_ui_system(
    mut mgr: ResMut<DialogManager>,
    shop: Res<GameShopState>,
    close: Query<&UiButton, With<GameShopClose>>,
    mut widgets: Query<&mut Visibility, With<GameShopWidget>>,
    mut lines: Query<(&mut Text2d, &GameShopLine)>,
) {
    let open = mgr.is_open(DialogKind::GameShop);
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        return;
    }
    for btn in &close {
        if btn.clicked {
            mgr.close(DialogKind::GameShop);
        }
    }
    for (mut text, line) in &mut lines {
        if let Some(item) = shop.items.get(line.0) {
            text.0 = format!("{}  {} 元宝", item.name, item.price);
        } else {
            text.0 = String::new();
        }
    }
}

// ============================================================================
// NPC 商店（M9 第 2 批收尾）
// 布局参考：C# NPCDialogs.cs NPCGoodsDialog / macroquad npc_goods_dialog.rs
//   - 背景 Prguse[1000]，位置 (0,224)，8 行商品列表
//   - 购买按钮 Title[312-314]；关闭 Prguse2[360-362]
// 网络：NPCGoods（商品列表，含 ItemInfo）→ 显示；BuyItem 购买
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::DialogRoot;
use crate::map_renderer::GameLibraries;
use crate::network::NetworkContext;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiFont,
    UiImageCache,
};

/// 商品条目
#[derive(Debug, Clone)]
pub struct GoodsEntry {
    pub item_index: i32,
    pub name: String,
    pub price: u32,
    pub count: u16,
}

/// NPC 商店状态
#[derive(Resource, Default)]
pub struct NpcGoodsState {
    pub visible: bool,
    pub title: String,
    pub goods: Vec<GoodsEntry>,
    pub selected: Option<usize>,
}

#[derive(Component)]
pub struct NpcGoodsWidget;

#[derive(Component)]
pub struct NpcGoodsClose;

#[derive(Component)]
pub struct NpcGoodsBuy;

#[derive(Component)]
pub struct NpcGoodsLine(usize);

pub struct NpcGoodsPlugin;

impl Plugin for NpcGoodsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NpcGoodsState>();
        app.add_systems(OnEnter(AppState::Game), spawn_npc_goods);
        app.add_systems(OnExit(AppState::Game), cleanup_npc_goods);
        app.add_systems(
            Update,
            (npc_goods_ui_system, ui_button_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_npc_goods(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_npc_goods(
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

    // 背景 Prguse[1000] 在 (0,224)
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 1000) {
        let e = spawn_ui_sprite(&mut commands, h, 0.0, 224.0, 6.0, 1.0);
        commands.entity(e).insert((
            DialogRoot(crate::game::dialogs::DialogKind::NpcGoods),
            NpcGoodsWidget,
            Visibility::Hidden,
        ));
    }

    // 关闭按钮（右上）
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 360, 361, 362,
        500.0, 227.0, 7.0, 20.0, 20.0,
    ) {
        commands.entity(e).insert((
            NpcGoodsClose,
            DialogRoot(crate::game::dialogs::DialogKind::NpcGoods),
            NpcGoodsWidget,
        ));
    }

    // 购买按钮
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 312, 313, 314,
        420.0, 224.0 + 250.0, 7.0, 76.0, 25.0,
    ) {
        commands.entity(e).insert((
            NpcGoodsBuy,
            DialogRoot(crate::game::dialogs::DialogKind::NpcGoods),
            NpcGoodsWidget,
        ));
    }

    // 8 行商品
    for i in 0..8usize {
        let e = spawn_ui_text(
            &mut commands, &font, "",
            12.0, 240.0 + i as f32 * 22.0,
            12.0, Color::WHITE, 8.0,
        );
        commands.entity(e).insert((
            NpcGoodsLine(i),
            DialogRoot(crate::game::dialogs::DialogKind::NpcGoods),
            NpcGoodsWidget,
        ));
    }
}

/// 显示/隐藏 + 商品列表渲染 + 选中/购买/关闭
fn npc_goods_ui_system(
    mut state: ResMut<NpcGoodsState>,
    net: Res<NetworkContext>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    close: Query<&UiButton, (With<NpcGoodsClose>, Without<NpcGoodsBuy>)>,
    buy: Query<&UiButton, (With<NpcGoodsBuy>, Without<NpcGoodsClose>)>,
    mut widgets: Query<&mut Visibility, With<NpcGoodsWidget>>,
    mut lines: Query<(&mut Text2d, &NpcGoodsLine)>,
) {
    for mut vis in widgets.iter_mut() {
        *vis = if state.visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !state.visible {
        return;
    }

    // 商品行（名称 + 价格）
    for (mut text, line) in &mut lines {
        if let Some(g) = state.goods.get(line.0) {
            text.0 = format!("{} x{}  {} 金", g.name, g.count, g.price);
        } else {
            text.0 = String::new();
        }
    }

    // 点击行选中
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else { return };
    if mouse.just_pressed(MouseButton::Left) {
        for i in 0..8usize {
            let y = 240.0 + i as f32 * 22.0;
            if cursor.x >= 12.0 && cursor.x <= 480.0 && cursor.y >= y && cursor.y <= y + 18.0 {
                if i < state.goods.len() {
                    state.selected = Some(i);
                    tracing::debug!("🏪 选中商品: {}", state.goods[i].name);
                }
                break;
            }
        }
    }

    // 关闭
    for btn in &close {
        if btn.clicked {
            state.visible = false;
            state.selected = None;
        }
    }
    // 购买（发送 BuyItem；后续接入）
    for _btn in &buy {
        if let Some(idx) = state.selected {
            if let Some(g) = state.goods.get(idx) {
                tracing::info!("🏪 购买 {} (item_index={})", g.name, g.item_index);
                // BuyItem 包字段待确认后接入
                let _ = &net;
            }
        }
    }
}

// ============================================================================
// 英雄装备对话框（#206）
// 参考：C# HeroDialog = CharacterDialog(MirGridType.HeroEquipment, Hero)
//   - 背景 Title[504]，位置 (ScreenWidth-264, 0)；角色页 Prguse[340] at (8,90)
//   - 14 个装备槽（C# EquipmentSlot 顺序，EQUIP_SLOTS 布局）
//   - 服务端 14 槽按 SERVER_SLOT_TO_POS 映射到显示位
// 交互：
//   - 点击装备格 → C.RemoveItem{Grid=HeroEquipment}（卸下回英雄背包）
//   - 英雄背包双击 → C.EquipItem{Grid=HeroInventory}（hero_inventory.rs）
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::character::{EQUIP_SLOTS, SERVER_SLOT_TO_POS, SLOT_H, SLOT_W};
use crate::game::dialogs::hero::HeroState;
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::UiFont;
use crate::ui::theme::{
    load_lib_image, spawn_icon_button, spawn_item_cell_ui, spawn_panel, UiItemCellData,
};

const DIALOG_X: f32 = 1024.0 - 264.0;
const DIALOG_Y: f32 = 0.0;
const PAGE_X: f32 = 8.0;
const PAGE_Y: f32 = 90.0;

#[derive(Component)]
pub struct HeroEquipWidget;

#[derive(Component)]
pub struct HeroEquipClose;

/// 装备显示位（EQUIP_SLOTS 下标）
#[derive(Component)]
pub struct HeroEquipSlot(pub usize);

pub struct HeroEquipmentPlugin;

impl Plugin for HeroEquipmentPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Game), spawn_hero_equipment);
        app.add_systems(OnExit(AppState::Game), cleanup_hero_equipment);
        app.add_systems(
            Update,
            hero_equip_ui_system.run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_hero_equipment(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_hero_equipment(
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

    // 背景 Title[504]（C# CharacterDialog.Index，264x380 @ (760,0)）
    let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Title, 504) else {
        return;
    };
    let panel = spawn_panel(&mut commands, bg, DIALOG_X, DIALOG_Y, 264.0, 380.0, 30);
    commands
        .entity(panel)
        .insert((DialogRoot(DialogKind::HeroEquipment), HeroEquipWidget));

    commands.entity(panel).with_children(|p| {
        // 角色页 Prguse[340]（C# CharacterPage at (8,90)，原生尺寸）
        if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 340) {
            let (iw, ih) = match libs.0.get_image(LibraryName::Prguse, 340) {
                Some(i) => (i.width.max(0) as f32, i.height.max(0) as f32),
                None => (190.0, 259.0),
            };
            crate::ui::theme::spawn_image(p, h, PAGE_X, PAGE_Y, iw, ih, 8);
        }
        // 关闭（C# CharacterDialog CloseButton at (241,3)）
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 360),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 361),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 362),
        ) {
            spawn_icon_button(p, n, h, pr, 241.0, 3.0, 20.0, 20.0, 10).insert(HeroEquipClose);
        }
        // 14 个装备槽（通用 UiItemCell；数据渲染交给 item_cell_ui_system，#90）
        for (pos, (rx, ry)) in EQUIP_SLOTS.iter().enumerate() {
            spawn_item_cell_ui(
                p,
                &mut images,
                &font,
                PAGE_X + rx,
                PAGE_Y + ry,
                SLOT_W,
                SLOT_H,
                9,
                pos,
            )
            .insert(HeroEquipSlot(pos));
        }
    });
}

/// 显隐 + 数据渲染 + 点击卸下
fn hero_equip_ui_system(
    mut mgr: ResMut<DialogManager>,
    hero: Res<HeroState>,
    net: Res<NetConnection>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    close: Query<(Entity, &Interaction), With<HeroEquipClose>>,
    mut widgets: Query<&mut Visibility, With<HeroEquipWidget>>,
    mut cells: Query<(&HeroEquipSlot, &mut UiItemCellData), Without<crate::ui::theme::UiItemCellIcon>>,
    mut prev_inter: Local<std::collections::HashMap<Entity, Interaction>>,
    panel_origin: Query<&Node, With<HeroEquipWidget>>,
) {
    fn edge(
        e: Entity,
        inter: &Interaction,
        prev: &mut std::collections::HashMap<Entity, Interaction>,
    ) -> bool {
        let was = prev.insert(e, *inter);
        *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
    }
    let open = mgr.is_open(DialogKind::HeroEquipment);
    for mut vis in &mut widgets {
        *vis = if open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !open {
        return;
    }
    for (e, inter) in &close {
        if edge(e, inter, &mut prev_inter) {
            mgr.close(DialogKind::HeroEquipment);
        }
    }
    // 数据：显示位 → 服务端槽位（SERVER_SLOT_TO_POS 反查）
    for (slot, mut data) in &mut cells {
        let server_idx = SERVER_SLOT_TO_POS.iter().position(|p| *p == slot.0);
        let item = server_idx
            .and_then(|i| hero.equipment.get(i))
            .and_then(|s| s.as_ref());
        match item {
            Some(item) => {
                data.icon = load_lib_image(
                    &mut libs,
                    &mut images,
                    crate::resources::libraries::LibraryName::Items,
                    item.image as usize,
                );
                data.count = None;
                data.dura_ratio = if item.max_dura > 0 {
                    Some((item.current_dura as f32 / item.max_dura as f32).clamp(0.0, 1.0))
                } else {
                    None
                };
            }
            None => {
                data.icon = None;
                data.count = None;
                data.dura_ratio = None;
            }
        }
    }
    // 点击装备格 → C.RemoveItem（卸下回英雄背包）
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let (ox, oy) = panel_origin
        .single()
        .map(|n| {
            (
                match n.left {
                    Val::Px(v) => v,
                    _ => DIALOG_X,
                },
                match n.top {
                    Val::Px(v) => v,
                    _ => DIALOG_Y,
                },
            )
        })
        .unwrap_or((DIALOG_X, DIALOG_Y));
    for (slot, _) in &cells {
        let (rx, ry) = EQUIP_SLOTS[slot.0];
        let sx = ox + PAGE_X + rx;
        let sy = oy + PAGE_Y + ry;
        if cursor.x >= sx && cursor.x <= sx + SLOT_W && cursor.y >= sy && cursor.y <= sy + SLOT_H {
            let server_idx = SERVER_SLOT_TO_POS.iter().position(|p| *p == slot.0);
            if let Some(item) = server_idx
                .and_then(|i| hero.equipment.get(i))
                .and_then(|s| s.as_ref())
            {
                net.send_packet(&mir2_shared::packets::client::item::RemoveItem {
                    grid: mir2_shared::enums::MirGridType::HeroEquipment,
                    unique_id: item.unique_id,
                    to: 0,
                });
                tracing::info!("🦸 英雄卸下装备 {} (uid={})", item.name, item.unique_id);
            }
            break;
        }
    }
}

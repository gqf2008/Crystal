// ============================================================================
// 英雄背包对话框（#203-B）
// 参考：C# HeroInventoryDialog（Client/MirScenes/Dialogs/HeroDialogs.cs）
//   - 背景 Prguse[1422]，可移动，居中
//   - 40 格（8x5）：Location = (14+x*37, 23+y*33)，GridType=HeroInventory
//   - 4 行锁条 Prguse[1423]（固定 40 格时隐藏）；HP/MP 锁条 1428/1429
//   - HP/MP 自动药按钮 Title[560-565] + 百分比标签 + HPItem/MPItem 物品格
// 交互（#203）：
//   - 点击英雄格选中 → 点主背包格 = C.TakeBackHeroItem（英雄→主）
//   - 主背包选中 → 点英雄格 = C.TransferHeroItem（主→英雄）
//   - 选中态与主背包共用 InvClickState（hero_selected / selected）
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::hero::{next_autopot, HeroState, STAT_HP, STAT_MP};
use crate::game::dialogs::inventory::{inv_slot_at, InvClickState};
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::controls::{spawn_item_cell, ItemCellData, ItemCellIcon};
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiFont, UiImageCache,
};

// 布局常量（相对对话框左上角；C# HeroInventoryDialog）
const CELL_W: f32 = 36.0;
const CELL_H: f32 = 32.0;
const GRID_COLS: usize = 8;
const GRID_ROWS: usize = 5;

/// 英雄背包格相对坐标（C# Location = (14+x*37, 23+y*33)）
fn hero_cell_pos(i: usize) -> (f32, f32) {
    let x = (i % GRID_COLS) as f32;
    let y = (i / GRID_COLS) as f32;
    (14.0 + x * 37.0, 23.0 + y * 33.0)
}

/// 光标坐标 → 英雄背包格（0..39）
fn hero_slot_at(cx: f32, cy: f32) -> Option<usize> {
    for i in 0..(GRID_COLS * GRID_ROWS) {
        let (sx, sy) = hero_cell_pos(i);
        if cx >= sx && cx <= sx + CELL_W && cy >= sy && cy <= sy + CELL_H {
            return Some(i);
        }
    }
    None
}

#[derive(Component)]
pub struct HeroInvWidget;

#[derive(Component)]
pub struct HeroInvClose;

#[derive(Component)]
pub struct HeroInvSlot(pub usize);

#[derive(Component)]
pub struct HeroInvLockBar(pub usize);

#[derive(Component)]
pub struct HeroInvHpLock;

#[derive(Component)]
pub struct HeroInvMpLock;

#[derive(Component)]
pub struct HeroInvHpBtn;

#[derive(Component)]
pub struct HeroInvMpBtn;

#[derive(Component)]
pub struct HeroInvHpLabel;

#[derive(Component)]
pub struct HeroInvMpLabel;

#[derive(Component)]
pub struct HeroInvHpItem;

#[derive(Component)]
pub struct HeroInvMpItem;

pub struct HeroInventoryPlugin;

impl Plugin for HeroInventoryPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Game), spawn_hero_inventory);
        app.add_systems(OnExit(AppState::Game), cleanup_hero_inventory);
        app.add_systems(
            Update,
            (
                hero_inv_visibility_system,
                hero_inv_data_system,
                hero_inv_click_system,
                ui_button_system,
            )
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_hero_inventory(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_hero_inventory(
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

    // 背景 Prguse[1422]，居中（C# Location = Center）
    let Some(bg) = ui_image(
        &mut libs,
        &mut images,
        &mut cache,
        LibraryName::Prguse,
        1422,
    ) else {
        return;
    };
    let (bw, bh) = libs
        .0
        .get_image(LibraryName::Prguse, 1422)
        .map(|i| (i.width.max(0) as f32, i.height.max(0) as f32))
        .unwrap_or((320.0, 250.0));
    let dx = (1024.0 - bw) / 2.0;
    let dy = (768.0 - bh) / 2.0;

    let e = spawn_ui_sprite(&mut commands, bg, dx, dy, 6.0, 1.0);
    commands.entity(e).insert((
        DialogRoot(DialogKind::HeroInventory),
        HeroInvWidget,
        Visibility::Hidden,
    ));

    // 关闭（C# Prguse2 360/361/362 at (299,2)）
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands,
        &mut libs,
        &mut images,
        &mut cache,
        LibraryName::Prguse2,
        360,
        361,
        362,
        dx + 299.0,
        dy + 2.0,
        7.0,
        20.0,
        20.0,
    ) {
        commands.entity(e).insert((
            HeroInvClose,
            DialogRoot(DialogKind::HeroInventory),
            HeroInvWidget,
        ));
    }

    // 40 格（通用 ItemCell；渲染交给 item_cell_system，#90）
    for i in 0..(GRID_COLS * GRID_ROWS) {
        let (cx, cy) = hero_cell_pos(i);
        let cell = spawn_item_cell(
            &mut commands,
            &mut images,
            &font,
            dx + cx,
            dy + cy,
            7.5,
            CELL_W,
            CELL_H,
            i,
        );
        commands.entity(cell).insert((
            HeroInvSlot(i),
            DialogRoot(DialogKind::HeroInventory),
            HeroInvWidget,
        ));
    }

    // 4 行锁条（C# Prguse[1423] at (14, 56+i*33)；固定 40 格时始终隐藏）
    for i in 0..4usize {
        if let Some(h) = ui_image(
            &mut libs,
            &mut images,
            &mut cache,
            LibraryName::Prguse,
            1423,
        ) {
            let le = spawn_ui_sprite(
                &mut commands,
                h,
                dx + 14.0,
                dy + 56.0 + i as f32 * 33.0,
                7.2,
                1.0,
            );
            commands.entity(le).insert((
                HeroInvLockBar(i),
                DialogRoot(DialogKind::HeroInventory),
                HeroInvWidget,
                Visibility::Hidden,
            ));
        }
    }
    // HP/MP 锁条（1428/1429 at (57,196)/(162,196)，!AutoPot 时显示）
    if let Some(h) = ui_image(
        &mut libs,
        &mut images,
        &mut cache,
        LibraryName::Prguse,
        1428,
    ) {
        let le = spawn_ui_sprite(&mut commands, h, dx + 57.0, dy + 196.0, 7.2, 1.0);
        commands.entity(le).insert((
            HeroInvHpLock,
            DialogRoot(DialogKind::HeroInventory),
            HeroInvWidget,
            Visibility::Hidden,
        ));
    }
    if let Some(h) = ui_image(
        &mut libs,
        &mut images,
        &mut cache,
        LibraryName::Prguse,
        1429,
    ) {
        let le = spawn_ui_sprite(&mut commands, h, dx + 162.0, dy + 196.0, 7.2, 1.0);
        commands.entity(le).insert((
            HeroInvMpLock,
            DialogRoot(DialogKind::HeroInventory),
            HeroInvWidget,
            Visibility::Hidden,
        ));
    }

    // HP/MP 自动药按钮（Title 560-565 at (58/206, h-60)，AutoPot 时显示）
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands,
        &mut libs,
        &mut images,
        &mut cache,
        LibraryName::Title,
        560,
        561,
        562,
        dx + 58.0,
        dy + bh - 60.0,
        8.3,
        60.0,
        25.0,
    ) {
        commands.entity(e).insert((
            HeroInvHpBtn,
            DialogRoot(DialogKind::HeroInventory),
            HeroInvWidget,
            Visibility::Hidden,
        ));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands,
        &mut libs,
        &mut images,
        &mut cache,
        LibraryName::Title,
        563,
        564,
        565,
        dx + 206.0,
        dy + bh - 60.0,
        8.3,
        60.0,
        25.0,
    ) {
        commands.entity(e).insert((
            HeroInvMpBtn,
            DialogRoot(DialogKind::HeroInventory),
            HeroInvWidget,
            Visibility::Hidden,
        ));
    }
    // 百分比标签（按钮下方）
    let hp_label = spawn_ui_text(
        &mut commands,
        &font,
        "",
        dx + 58.0,
        dy + bh - 33.0,
        12.0,
        Color::WHITE,
        8.4,
    );
    commands.entity(hp_label).insert((
        HeroInvHpLabel,
        DialogRoot(DialogKind::HeroInventory),
        HeroInvWidget,
        Visibility::Hidden,
    ));
    let mp_label = spawn_ui_text(
        &mut commands,
        &font,
        "",
        dx + 206.0,
        dy + bh - 33.0,
        12.0,
        Color::WHITE,
        8.4,
    );
    commands.entity(mp_label).insert((
        HeroInvMpLabel,
        DialogRoot(DialogKind::HeroInventory),
        HeroInvWidget,
        Visibility::Hidden,
    ));
    // HP/MP 物品格（C# HPItem at (122, h-55) / MPItem at (166, h-55)）
    let hp_item = spawn_item_cell(
        &mut commands,
        &mut images,
        &font,
        dx + 122.0,
        dy + bh - 55.0,
        7.5,
        34.0,
        30.0,
        40,
    );
    commands.entity(hp_item).insert((
        HeroInvHpItem,
        DialogRoot(DialogKind::HeroInventory),
        HeroInvWidget,
    ));
    let mp_item = spawn_item_cell(
        &mut commands,
        &mut images,
        &font,
        dx + 166.0,
        dy + bh - 55.0,
        7.5,
        34.0,
        30.0,
        41,
    );
    commands.entity(mp_item).insert((
        HeroInvMpItem,
        DialogRoot(DialogKind::HeroInventory),
        HeroInvWidget,
    ));
}

/// 显隐 + 关闭 + 标签/锁条/自动药组（依赖 ui_button_system 先跑，chain 保证）
#[allow(clippy::too_many_arguments)]
fn hero_inv_visibility_system(
    mut mgr: ResMut<DialogManager>,
    hero: Res<HeroState>,
    net: Res<NetConnection>,
    close: Query<&UiButton, With<HeroInvClose>>,
    hp_btn: Query<&UiButton, With<HeroInvHpBtn>>,
    mp_btn: Query<&UiButton, With<HeroInvMpBtn>>,
    // 单查询统一处理显隐（Option 组件区分角色，避免多个 &mut Visibility 查询冲突 B0001）
    mut widgets: Query<
        (
            &mut Visibility,
            Option<&mut Text2d>,
            Option<&HeroInvHpBtn>,
            Option<&HeroInvMpBtn>,
            Option<&HeroInvHpLabel>,
            Option<&HeroInvMpLabel>,
            Option<&HeroInvHpLock>,
            Option<&HeroInvMpLock>,
            Option<&HeroInvLockBar>,
        ),
        With<HeroInvWidget>,
    >,
) {
    let open = mgr.is_open(DialogKind::HeroInventory);
    for (mut vis, mut text, hp_btn_c, mp_btn_c, hp_label, mp_label, hp_lock, mp_lock, lockbar) in
        &mut widgets
    {
        *vis = if open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if !open {
            continue;
        }
        let hp_grp = hp_btn_c.is_some();
        let mp_grp = mp_btn_c.is_some();
        if hp_label.is_some() {
            if let Some(text) = text.as_mut() {
                text.0 = format!("{}%", hero.auto_pot_hp);
            }
        } else if mp_label.is_some() {
            if let Some(text) = text.as_mut() {
                text.0 = format!("{}%", hero.auto_pot_mp);
            }
        }
        if let Some(bar) = lockbar {
            let locked = hero.inventory.len() < 11 + 8 * bar.0;
            *vis = if locked {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        } else if hp_lock.is_some() || mp_lock.is_some() {
            *vis = if hero.auto_pot {
                Visibility::Hidden
            } else {
                Visibility::Visible
            };
        } else if hp_label.is_some() || mp_label.is_some() || hp_grp || mp_grp {
            *vis = if hero.auto_pot {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
    }
    if !open {
        return;
    }
    for btn in &close {
        if btn.clicked {
            mgr.close(DialogKind::HeroInventory);
        }
    }
    // HP/MP 阈值循环（C# HPButton → SetAutoPotValue）
    for btn in &hp_btn {
        if btn.clicked {
            let v = next_autopot(hero.auto_pot_hp);
            net.send_packet(&mir2_shared::packets::client::hero::SetAutoPotValue {
                stat: STAT_HP,
                value: v as u32,
            });
        }
    }
    for btn in &mp_btn {
        if btn.clicked {
            let v = next_autopot(hero.auto_pot_mp);
            net.send_packet(&mir2_shared::packets::client::hero::SetAutoPotValue {
                stat: STAT_MP,
                value: v as u32,
            });
        }
    }
}

/// 40 格数据 + HP/MP 物品图标 + 选中高亮
#[allow(clippy::too_many_arguments)]
fn hero_inv_data_system(
    hero: Res<HeroState>,
    click: Res<InvClickState>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<UiImageCache>,
    mut cells: Query<
        (&HeroInvSlot, &mut ItemCellData),
        (
            Without<ItemCellIcon>,
            Without<HeroInvHpItem>,
            Without<HeroInvMpItem>,
        ),
    >,
    mut cell_sprites: Query<
        (&mut Sprite, &HeroInvSlot),
        (
            Without<ItemCellIcon>,
            Without<HeroInvHpItem>,
            Without<HeroInvMpItem>,
        ),
    >,
    mut hp_item: Query<
        &mut ItemCellData,
        (
            With<HeroInvHpItem>,
            Without<HeroInvMpItem>,
            Without<HeroInvSlot>,
        ),
    >,
    mut mp_item: Query<
        &mut ItemCellData,
        (
            With<HeroInvMpItem>,
            Without<HeroInvHpItem>,
            Without<HeroInvSlot>,
        ),
    >,
) {
    for (slot, mut data) in &mut cells {
        let item = hero.inventory.get(slot.0).and_then(|s| s.as_ref());
        match item {
            Some(item) => {
                data.icon = ui_image(
                    &mut libs,
                    &mut images,
                    &mut cache,
                    crate::resources::libraries::LibraryName::Items,
                    item.image as usize,
                );
                data.count = if item.count > 1 {
                    Some(item.count as u32)
                } else {
                    None
                };
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
    // 选中高亮（黄色半透明，C# SelectedCell 语义）
    for (mut sprite, slot) in &mut cell_sprites {
        let target = if click.hero_selected == Some(slot.0) {
            Color::srgba(1.0, 0.9, 0.2, 0.35)
        } else {
            Color::srgba(0.0, 0.0, 0.0, 0.18)
        };
        if sprite.color != target {
            sprite.color = target;
        }
    }
    // HP/MP 物品格（配置的自动药物品图标）
    for mut data in &mut hp_item {
        data.icon = if hero.hp_item_index >= 0 {
            ui_image(
                &mut libs,
                &mut images,
                &mut cache,
                LibraryName::Items,
                hero.hp_item_index as usize,
            )
        } else {
            None
        };
        data.count = None;
        data.dura_ratio = None;
    }
    for mut data in &mut mp_item {
        data.icon = if hero.mp_item_index >= 0 {
            ui_image(
                &mut libs,
                &mut images,
                &mut cache,
                LibraryName::Items,
                hero.mp_item_index as usize,
            )
        } else {
            None
        };
        data.count = None;
        data.dura_ratio = None;
    }
}

/// 点击转移：主背包选中 → 英雄格 = TransferHeroItem；英雄格选中/取消
fn hero_inv_click_system(
    mgr: Res<DialogManager>,
    hero: Res<HeroState>,
    mut click: ResMut<InvClickState>,
    net: Res<NetConnection>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    time: Res<Time>,
    mut last_hero_click: Local<Option<(usize, f64)>>,
) {
    if !mgr.is_open(DialogKind::HeroInventory) {
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    // 主背包格点击（英雄选中态下由 inv_item_action_system 发 TakeBackHeroItem）
    if inv_slot_at(cursor.x, cursor.y).is_some() {
        return;
    }
    let Some(i) = hero_slot_at(cursor.x, cursor.y) else {
        return;
    };
    // #206：双击英雄背包格 → C.EquipItem（C# MirItemCell OnMouseDoubleClick）
    let now = time.elapsed_secs_f64();
    if let Some((last_i, last_t)) = *last_hero_click {
        if last_i == i && now - last_t < 0.4 {
            *last_hero_click = None;
            if let Some(item) = hero.inventory.get(i).and_then(|s| s.as_ref()) {
                if let Some(to) = item.equip_slot() {
                    net.send_packet(&mir2_shared::packets::client::item::EquipItem {
                        grid: mir2_shared::enums::MirGridType::HeroInventory,
                        unique_id: item.unique_id,
                        to,
                    });
                    tracing::info!("🦸 英雄装备 {} (uid={}) -> slot {}", item.name, item.unique_id, to);
                }
            }
            return;
        }
    }
    *last_hero_click = Some((i, now));
    if let Some(main_from) = click.selected {
        net.send_packet(&crate::network::TransferHeroItemWire {
            from: main_from as i32,
            to: i as i32,
        });
        click.selected = None;
        click.hero_selected = None;
        tracing::info!("🎒 转移物品 主背包{} -> 英雄{}", main_from, i);
    } else {
        // 选中/取消选中英雄格（空格不选中）
        if hero.inventory.get(i).and_then(|s| s.as_ref()).is_some() {
            click.hero_selected = if click.hero_selected == Some(i) {
                None
            } else {
                Some(i)
            };
        } else {
            click.hero_selected = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::hero_slot_at;

    #[test]
    fn hero_slot_hit_math() {
        // C# HeroInventoryDialog 布局：(14+x*37, 23+y*33)，格 36x32
        assert_eq!(hero_slot_at(14.0, 23.0), Some(0));
        assert_eq!(
            hero_slot_at(14.0 + 3.0 * 37.0 + 2.0, 23.0 + 4.0 * 33.0 + 2.0),
            Some(4 * 8 + 3)
        );
        assert_eq!(
            hero_slot_at(14.0 + 7.0 * 37.0 + 30.0, 23.0 + 4.0 * 33.0 + 28.0),
            Some(39)
        );
        assert_eq!(hero_slot_at(0.0, 0.0), None);
        assert_eq!(hero_slot_at(14.0 + 8.0 * 37.0, 23.0), None);
    }
}

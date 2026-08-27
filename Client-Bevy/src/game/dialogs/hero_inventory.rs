// ============================================================================
// 英雄背包对话框（#203-B）
// 参考：C# HeroInventoryDialog（Client/MirScenes/Dialogs/HeroDialogs.cs）
//   - 背景 Prguse[1422]，可移动，原点 (0,0)（构造器未设 Location → 默认）
//   - 40 格（8x5）：Location = (14+x*37, 23+y*33)，GridType=HeroInventory
//   - 4 行锁条 Prguse[1423]（固定 40 格时隐藏）；HP/MP 锁条 1428/1429
//   - HP/MP 自动药按钮 Title[560-565] + 百分比标签 + HPItem/MPItem 物品格
// 交互（#203）：
//   - 点击英雄格选中 → 点主背包格 = C.TakeBackHeroItem（英雄→主）
//   - 主背包选中 → 点英雄格 = C.TransferHeroItem（主→英雄）
//   - 选中态与主背包共用 InvClickState（hero_selected / selected）
// ============================================================================

use bevy::prelude::*;

use crate::actor::{LocalPlayer, MountState};
use crate::game::dialogs::hero::{HeroState, STAT_HP, STAT_MP, next_autopot};
use crate::game::dialogs::inventory::{
    InvClickState, InvDropConfirm, InvUiState, ItemUseFeedback, UseItemCtx, UseOutcome, inv_slot_at,
    item_use_sound_id, use_item_core,
};
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::game::player_state::{Inventory, Loadout};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{shared_cjk_font, UiCjkFont, UiFont};
use crate::ui::theme::{
    load_lib_image, spawn_icon_button, spawn_image, spawn_item_cell_ui, spawn_label, spawn_panel,
    UiItemCellData, UiItemCellIcon,
};

// 布局常量（相对对话框左上角；C# HeroInventoryDialog）
const CELL_W: f32 = 36.0;
const CELL_H: f32 = 32.0;
const GRID_COLS: usize = 8;
const GRID_ROWS: usize = 5;
/// 英雄背包窗口原点：C# HeroInventoryDialog 构造器未设 Location（HeroDialogs.cs:24-31）
/// → MirControl 默认 (0,0)。旧实现按屏幕居中，非 C# 行为。
pub const DIALOG_X: f32 = 0.0;
pub const DIALOG_Y: f32 = 0.0;

/// 英雄背包格相对坐标（C# Location = (14+x*37, 23+y*33)）
fn hero_cell_pos(i: usize) -> (f32, f32) {
    let x = (i % GRID_COLS) as f32;
    let y = (i / GRID_COLS) as f32;
    (14.0 + x * 37.0, 23.0 + y * 33.0)
}

/// 光标坐标 → 英雄背包格（0..39）。ox/oy = 面板当前原点（拖动/推位后跟随）
fn hero_slot_at(cx: f32, cy: f32, ox: f32, oy: f32) -> Option<usize> {
    for i in 0..(GRID_COLS * GRID_ROWS) {
        let (rx, ry) = hero_cell_pos(i);
        let (sx, sy) = (ox + rx, oy + ry);
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
            (hero_inv_visibility_system, hero_inv_data_system, hero_inv_click_system)
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

    // 背景 Prguse[1422]（C# HeroInventoryDialog Index=1422，324x266 @ (0,0)）
    let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 1422) else {
        return;
    };
    let (bw, bh) = (
        libs.0
            .get_image(LibraryName::Prguse, 1422)
            .map(|i| i.width.max(0) as f32)
            .unwrap_or(324.0),
        libs.0
            .get_image(LibraryName::Prguse, 1422)
            .map(|i| i.height.max(0) as f32)
            .unwrap_or(266.0),
    );
    let panel = spawn_panel(&mut commands, bg, DIALOG_X, DIALOG_Y, bw, bh, 30);
    commands
        .entity(panel)
        .insert((DialogRoot(DialogKind::HeroInventory), HeroInvWidget));

    commands.entity(panel).with_children(|p| {
        // 关闭（C# Prguse2 360/361/362 at (299,2)）
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 360),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 361),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 362),
        ) {
            spawn_icon_button(p, n, h, pr, 299.0, 2.0, 20.0, 20.0, 10).insert(HeroInvClose);
        }
        // 40 格（通用 UiItemCell；渲染交给 item_cell_ui_system，#90）
        for i in 0..(GRID_COLS * GRID_ROWS) {
            let (cx, cy) = hero_cell_pos(i);
            spawn_item_cell_ui(p, &mut images, &font, cx, cy, CELL_W, CELL_H, 9, i)
                .insert(HeroInvSlot(i));
        }
        // 4 行锁条（C# Prguse[1423] at (14, 56+i*33)；固定 40 格时始终隐藏）
        for i in 0..4usize {
            if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 1423) {
                spawn_image(p, h, 14.0, 56.0 + i as f32 * 33.0, 300.0, 33.0, 8)
                    .insert((HeroInvLockBar(i), Visibility::Hidden));
            }
        }
        // HP/MP 锁条（1428/1429 at (57,196)/(162,196)，!AutoPot 时显示）
        if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 1428) {
            spawn_image(p, h, 57.0, 196.0, 108.0, 62.0, 8)
                .insert((HeroInvHpLock, Visibility::Hidden));
        }
        if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 1429) {
            spawn_image(p, h, 162.0, 196.0, 108.0, 62.0, 8)
                .insert((HeroInvMpLock, Visibility::Hidden));
        }
        // HP/MP 自动药按钮（Title 560-565 at (58/206, h-60)，AutoPot 时显示）
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 560),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 561),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 562),
        ) {
            spawn_icon_button(p, n, h, pr, 58.0, bh - 60.0, 60.0, 25.0, 10)
                .insert((HeroInvHpBtn, Visibility::Hidden));
        }
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 563),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 564),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 565),
        ) {
            spawn_icon_button(p, n, h, pr, 206.0, bh - 60.0, 60.0, 25.0, 10)
                .insert((HeroInvMpBtn, Visibility::Hidden));
        }
        // 百分比标签（按钮下方）
        spawn_label(p, &cjk, "", 58.0, bh - 33.0, 12.0, Color::WHITE, 10)
            .insert((HeroInvHpLabel, Visibility::Hidden));
        spawn_label(p, &cjk, "", 206.0, bh - 33.0, 12.0, Color::WHITE, 10)
            .insert((HeroInvMpLabel, Visibility::Hidden));
        // HP/MP 物品格（C# HPItem at (122, h-55) / MPItem at (166, h-55)）
        spawn_item_cell_ui(p, &mut images, &font, 122.0, bh - 55.0, 34.0, 30.0, 9, 40)
            .insert(HeroInvHpItem);
        spawn_item_cell_ui(p, &mut images, &font, 166.0, bh - 55.0, 34.0, 30.0, 9, 41)
            .insert(HeroInvMpItem);
    });
}

/// 显隐 + 关闭 + 标签/锁条/自动药组（依赖 ui_button_system 先跑，chain 保证）
#[allow(clippy::too_many_arguments)]
fn hero_inv_visibility_system(
    mut mgr: ResMut<DialogManager>,
    hero: Res<HeroState>,
    net: Res<NetConnection>,
    close: Query<(Entity, &Interaction), With<HeroInvClose>>,
    hp_btn: Query<(Entity, &Interaction), With<HeroInvHpBtn>>,
    mp_btn: Query<(Entity, &Interaction), With<HeroInvMpBtn>>,
    // 单查询统一处理显隐（Option 组件区分角色，避免多个 &mut Visibility 查询冲突 B0001）
    mut widgets: Query<
        (
            &mut Visibility,
            Option<&mut Text>,
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
    for (e, inter) in &close {
        if edge(e, inter, &mut prev_inter) {
            mgr.close(DialogKind::HeroInventory);
        }
    }
    // HP/MP 阈值循环（C# HPButton → SetAutoPotValue）
    for (e, inter) in &hp_btn {
        if edge(e, inter, &mut prev_inter) {
            let v = next_autopot(hero.auto_pot_hp);
            net.send_packet(&mir2_shared::packets::client::hero::SetAutoPotValue {
                stat: STAT_HP,
                value: v as u32,
            });
        }
    }
    for (e, inter) in &mp_btn {
        if edge(e, inter, &mut prev_inter) {
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
    mut cells: Query<
        (&HeroInvSlot, &mut UiItemCellData),
        (
            Without<UiItemCellIcon>,
            Without<HeroInvHpItem>,
            Without<HeroInvMpItem>,
        ),
    >,
    mut cell_bg: Query<
        (&HeroInvSlot, &mut BackgroundColor),
        (
            Without<UiItemCellIcon>,
            Without<HeroInvHpItem>,
            Without<HeroInvMpItem>,
        ),
    >,
    mut hp_item: Query<
        &mut UiItemCellData,
        (
            With<HeroInvHpItem>,
            Without<HeroInvMpItem>,
            Without<HeroInvSlot>,
        ),
    >,
    mut mp_item: Query<
        &mut UiItemCellData,
        (
            With<HeroInvMpItem>,
            Without<HeroInvHpItem>,
            Without<HeroInvSlot>,
        ),
    >,
) {
    for (slot, mut data) in &mut cells {
        // #2602：网格格 → 英雄背包槽 2+idx（前 2 槽是英雄腰带，不进网格）
        let item = hero.inventory.get(2 + slot.0).and_then(|s| s.as_ref());
        match item {
            Some(item) => {
                data.icon = load_lib_image(
                    &mut libs,
                    &mut images,
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
    // 选中高亮（黄色半透明，C# SelectedCell 语义；hero_selected 存原始槽位 2+idx）
    for (slot, mut bg) in &mut cell_bg {
        let target = if click.hero_selected() == Some(2 + slot.0) {
            Color::srgba(1.0, 0.9, 0.2, 0.35)
        } else {
            Color::srgba(0.0, 0.0, 0.0, 0.18)
        };
        if bg.0 != target {
            bg.0 = target;
        }
    }
    // HP/MP 物品格（配置的自动药物品图标）
    for mut data in &mut hp_item {
        data.icon = if hero.hp_item_index >= 0 {
            load_lib_image(
                &mut libs,
                &mut images,
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
            load_lib_image(
                &mut libs,
                &mut images,
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
    // 元组折叠（系统参数上限 16）：背包页 UI 态 / 主背包命中原点 / 英雄背包面板原点
    inv_res: (
        Res<InvUiState>,
        Res<crate::game::dialogs::inventory::InventoryOrigin>,
        Query<&Node, With<HeroInvWidget>>,
    ),
    // #2633 批次4 步7：riding 读 `MountState`（HudState 已于步9 删除）；
    // 英雄性别/职业/等级走 HeroState（不属本地玩家组件）
    player_q: Query<(&Inventory, &Loadout, Option<&MountState>), With<LocalPlayer>>,
    mut feedback: ResMut<ItemUseFeedback>,
    mut confirm: ResMut<InvDropConfirm>,
    mut last_hero_click: Local<Option<(usize, f64)>>,
    belt_visible: Res<crate::game::dialogs::hero_belt::HeroBeltVisible>,
    belt_vertical: Res<crate::game::dialogs::hero_belt::HeroBeltVertical>,
    mut belt_armed: ResMut<crate::game::dialogs::hero_belt::HeroBeltUseArmed>,
) {
    // 网格格交互要求英雄背包开；腰带格独立（C# HeroBeltDialog 的 MirItemCell
    // 腰带可见即可点，审查 m3——网格格坐标区 (0..309, 0..190) 覆盖世界点击，
    // 背包关着时不得命中）
    let grid_open = mgr.is_open(DialogKind::HeroInventory);
    if !grid_open && !belt_visible.0 {
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
    if inv_slot_at(
        cursor.x,
        cursor.y,
        inv_res.0.page,
        player_q.single().map(|(inv, _, _)| inv.items.len()).unwrap_or(0),
        (inv_res.1.0, inv_res.1.1),
    )
    .is_some()
    {
        return;
    }
    // #2602 命中目标 → 英雄背包原始槽位：8x5 网格格 = 2+idx（C# ItemSlot = 2+idx，
    // HeroDialogs.cs:53，前 2 槽是腰带不进网格）；英雄腰带格 = 0/1（HeroBeltDialog，
    // 独立渲染/命中，横纵布局随其 Flip）
    let (hx, hy) = inv_res
        .2
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
    let slot = if let Some(i) = hero_slot_at(cursor.x, cursor.y, hx, hy).filter(|_| grid_open) {
        Some(2 + i)
    } else if belt_visible.0 {
        (0..crate::game::dialogs::hero_belt::BELT_SLOTS).find(|&j| {
            let (x, y) = if belt_vertical.0 {
                crate::game::dialogs::hero_belt::v_slot(j)
            } else {
                crate::game::dialogs::hero_belt::h_slot(j)
            };
            cursor.x >= x
                && cursor.x <= x + crate::game::dialogs::hero_belt::CELL_SIZE
                && cursor.y >= y
                && cursor.y <= y + crate::game::dialogs::hero_belt::CELL_SIZE
        })
    } else {
        None
    };
    let Some(slot) = slot else {
        return;
    };
    // #206：双击英雄格 → 使用/装备（C# MirItemCell OnMouseDoubleClick → UseItem；#1546 守卫链）
    let now = time.elapsed_secs_f64();
    if let Some((last_slot, last_t)) = *last_hero_click {
        if last_slot == slot && now - last_t < 0.4 {
            *last_hero_click = None;
            if let Some(item) = hero.inventory.get(slot).and_then(|s| s.as_ref()) {
                // C# UseItem HeroGridType：actor=Hero，CanUseItem 用英雄性别/职业/等级；
                // 钓鱼限制英雄格跳过（!HeroGridType && User.Fishing）；槽物品/坐骑检查用 User 装备
                let (gender, class, level) = hero
                    .current
                    .as_ref()
                    .map(|c| (c.gender as u8, c.class as u8, c.level))
                    .unwrap_or((0, 0, 1));
                let ctx = UseItemCtx {
                    grid: mir2_shared::enums::MirGridType::HeroInventory,
                    equipment: &hero.equipment,
                    gender,
                    class,
                    level,
                    check_fishing: false,
                    allow_consumable: true,
                };
                // 槽物品前置恒看主角色装备（C# CanUseItem User 侧）→ 主角色 Loadout slots（步6）
                let player_state = player_q.single().ok();
                let player_equipment = player_state
                    .map(|(_, l, _)| l.slots.as_slice())
                    .unwrap_or(&[]);
                // 骑乘判定读 `MountState`（实体缺失视同未骑乘，同原 hud.riding=false）
                let riding = player_state.map(|(_, _, m)| m.is_some()).unwrap_or(false);
                if use_item_core(item, &net, riding, false, &player_equipment, ctx, now, &mut feedback, &mut confirm)
                    == UseOutcome::Sent
                {
                    // #2611：腰带格（0/1）使用时武装补货（C# :574 Item.Count==1
                    // 才发——只有用最后一瓶时武装）
                    if slot < crate::game::dialogs::hero_belt::BELT_SLOTS && item.count == 1 {
                        belt_armed.0 = true;
                    }
                    if let Some(sid) = item_use_sound_id(item) {
                        feedback.sounds.push(sid);
                    }
                }
            }
            return;
        }
    }
    *last_hero_click = Some((slot, now));
    // #2631：选中态归 inventory 所有，经 take_selected 读并清（转移到英雄后不再保留主背包选中）
    if let Some(main_from) = click.take_selected() {
        net.send_packet(&crate::network::TransferHeroItemWire {
            from: main_from as i32,
            to: slot as i32,
        });
        click.clear_hero_selected();
        tracing::info!("🎒 转移物品 主背包{} -> 英雄{}", main_from, slot);
    } else {
        // 选中/取消选中英雄格（空格不选中；hero_selected 存原始背包槽位，
        // 主背包取回路径 TakeBackHeroItem from 直接消费它）
        if hero.inventory.get(slot).and_then(|s| s.as_ref()).is_some() {
            click.toggle_hero_selected(slot);
        } else {
            click.clear_hero_selected();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::hero_slot_at;

    #[test]
    fn hero_slot_hit_math() {
        // C# HeroInventoryDialog 布局：(14+x*37, 23+y*33)，格 36x32
        assert_eq!(hero_slot_at(14.0, 23.0, 0.0, 0.0), Some(0));
        assert_eq!(
            hero_slot_at(14.0 + 3.0 * 37.0 + 2.0, 23.0 + 4.0 * 33.0 + 2.0, 0.0, 0.0),
            Some(4 * 8 + 3)
        );
        assert_eq!(
            hero_slot_at(14.0 + 7.0 * 37.0 + 30.0, 23.0 + 4.0 * 33.0 + 28.0, 0.0, 0.0),
            Some(39)
        );
        assert_eq!(hero_slot_at(0.0, 0.0, 0.0, 0.0), None);
        assert_eq!(hero_slot_at(14.0 + 8.0 * 37.0, 23.0, 0.0, 0.0), None);
        // 拖动到 (400,200) 后命中跟随
        assert_eq!(hero_slot_at(400.0 + 14.0, 200.0 + 23.0, 400.0, 200.0), Some(0));
    }
}

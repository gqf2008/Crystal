// ============================================================================
// 钓鱼对话框（M39）
// 参考：C# FishingDialog（Prguse[1340]）+ ServerRust tick_fishing / FishingCast
// 网络：
//   C: FishingCast[fishing_type u8] / FishingChangeAutocast[enabled u8]
//   S: FishingUpdate(198)[progress i32][success u8]
// 收获结果通过系统聊天消息返回（C# ReceiveChat 语义）
// ============================================================================

use bevy::prelude::*;

use crate::actor::LocalPlayer;
use crate::game::dialogs::inventory::{InvClickState, InvItem};
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::game::player_state::{Inventory, Loadout};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{shared_cjk_font, UiCjkFont, UiFont};
use crate::ui::theme::{
    load_lib_image, spawn_animated_icon_button, spawn_container, spawn_icon_button, spawn_image,
    spawn_label, spawn_panel,
};

/// #2892 批B：面板精灵与 C# 原生尺寸（C# `FishingDialog.Index = 1340; Location = Center`）
pub const PANEL: (LibraryName, usize) = (LibraryName::Prguse, 1340);
pub const PANEL_SIZE: (f32, f32) = (200.0, 287.0);
/// C# `FishingStatusDialog`（`FishingDialog.cs:159-179`）：`Prguse[1341]` 244x128 @(390,300)、`Movable`
pub const STATUS_PANEL: (LibraryName, usize) = (LibraryName::Prguse, 1341);
pub const STATUS_SIZE: (f32, f32) = (244.0, 128.0);
pub const STATUS_X: f32 = 390.0;
pub const STATUS_Y: f32 = 300.0;

/// 钓鱼状态（FishingUpdate 写入）
#[derive(Resource, Default)]
pub struct FishingState {
    /// 0=未钓鱼 1=等待 2=上钩 3=收竿 5=自动钓鱼切换
    pub progress: i32,
    pub success: bool,
    pub autocast: bool,
    pub message: String,
}

#[derive(Component)]
pub struct FishingWidget;

#[derive(Component)]
pub struct FishingClose;

#[derive(Component)]
pub struct FishingCast;

#[derive(Component)]
pub struct FishingAutocast;

#[derive(Component)]
pub struct FishingLine(usize);

/// 主窗标题 `TitleLabel`（C# @(10,4) 180x20）
#[derive(Component)]
pub struct FishingTitle;
/// 状态窗根（C# `FishingStatusDialog`，与主窗分开）
#[derive(Component)]
pub struct FishingStatusRoot;
#[derive(Component)]
pub struct FishingStatusClose;
/// `ChanceLabel`（C# @(14,79)）
#[derive(Component)]
pub struct FishingChanceLabel;
/// 进度条（C# `ProgressBar` 为 `BeforeDraw` 自绘，本端用容器 + 填充近似）
#[derive(Component)]
pub struct FishingProgressBar;
#[derive(Component)]
pub struct FishingProgressFill;
/// 抛竿键禁用帧（C# `Title[149]`）
#[derive(Component)]
pub struct FishingCastDisabled;
/// 自动钓鱼勾选框（`Prguse[1343]/[1344]`）
#[derive(Component)]
pub struct FishingAutocastBox;
/// ESC 退出勾选框（`Prguse[1346]/[1347]`）
#[derive(Component)]
pub struct FishingEscTick;

/// 钓具槽（0=Hook 1=Float 2=Bait 3=Finder 4=Reel，C# FishingSlot）
#[derive(Component)]
struct FishingGearSlot(usize);

pub struct FishingPlugin;

impl Plugin for FishingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FishingState>();
        app.add_systems(
            Update,
            fishing_server_events.run_if(in_state(AppState::Game)),
        );
        app.add_systems(OnEnter(AppState::Game), spawn_fishing);
        app.add_systems(OnExit(AppState::Game), cleanup_fishing);
        app.add_systems(
            Update,
            (fishing_ui_system, fishing_gear_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_fishing(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

/// 钓具槽位置（C# FishingDialog Grid：Hook@(17,203) Float@(17,241) Bait@(57,241) Finder@(97,241) Reel@(137,241)，34x30）
const GEAR_POS: [(f32, f32); 5] = [
    (17.0, 203.0),
    (17.0, 241.0),
    (57.0, 241.0),
    (97.0, 241.0),
    (137.0, 241.0),
];

/// 找背包第一个空格（钓具卸下目标；无空格返回 None）
fn free_inventory_index(items: &[Option<InvItem>]) -> Option<usize> {
    items.iter().position(|s| s.is_none())
}

fn spawn_fishing(
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

    // 背景 Prguse[1340] 原生 200x287，C# Location = Center。
    let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 1340) else {
        return;
    };
    let (px, py) = crate::game::dialogs::center_origin(200.0, 287.0);
    let panel = spawn_panel(&mut commands, bg, px, py, 200.0, 287.0, 30);
    commands
        .entity(panel)
        .insert((DialogRoot(DialogKind::Fishing), FishingWidget));

    // ---- FishingStatusDialog（C# `FishingDialog.cs:159-320`）：`Prguse[1341]` 244x128
    //      @ ((1024-244)/2, 300) = (390,300)，`Movable = true`；与主窗**分开**（C# 两个独立窗）。
    if let Some(bg2) = load_lib_image(&mut libs, &mut images, STATUS_PANEL.0, STATUS_PANEL.1) {
        let status = spawn_panel(
            &mut commands,
            bg2,
            STATUS_X,
            STATUS_Y,
            STATUS_SIZE.0,
            STATUS_SIZE.1,
            31,
        );
        commands.entity(status).insert((
            // 独立 kind：C# `FishingDialog` 与 `FishingStatusDialog` 都 `Movable` 且各自拖动
            DialogRoot(DialogKind::FishingStatus),
            FishingWidget,
            FishingStatusRoot,
        ));
        commands.entity(status).with_children(|p| {
            // 进度条/机会条（C# `ProgressBar`@(14,62)216x12、`ChanceBar`@(14,64) 为 BeforeDraw 自绘；
            // 本端用容器 + 填充子实体近似）
            spawn_container(p, 14.0, 62.0, 216.0, 12.0, 8)
                .insert((
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.35)),
                    FishingProgressBar,
                ))
                .with_children(|c| {
                    c.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(0.0),
                            top: Val::Px(0.0),
                            width: Val::Px(0.0),
                            height: Val::Px(12.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.2, 0.9, 0.3)),
                        FishingProgressFill,
                    ));
                });
            // `ChanceLabel` @(14,79)（C# `:199-206`）
            spawn_label(p, &cjk, "", 14.0, 79.0, 12.0, Color::WHITE, 9).insert(FishingChanceLabel);
            // `ESCTick` `Prguse[1346]`（未勾）/`[1347]`（勾）@(135,41)（C# `:281-303`）
            if let Some(off) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 1346) {
                spawn_image(p, off, 135.0, 41.0, 12.0, 12.0, 8);
            }
            if let Some(on) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 1347) {
                spawn_image(p, on, 135.0, 41.0, 16.0, 12.0, 9)
                    .insert((FishingEscTick, Visibility::Hidden));
            }
            // 关闭 `Prguse2[360/361/362]` @(216,4) 24x21
            if let (Some(n), Some(h), Some(pr)) = (
                load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 360),
                load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 361),
                load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 362),
            ) {
                spawn_icon_button(p, n, h, pr, 216.0, 4.0, 24.0, 21.0, 10)
                    .insert(FishingStatusClose);
            }
            // 抛竿 `FishButton`：禁用帧 `Title[149]`；可抛时 10 帧动画 + 按下帧 `142` @(47,95)
            if let Some(disabled) = load_lib_image(&mut libs, &mut images, LibraryName::Title, 149)
            {
                spawn_icon_button(
                    p,
                    disabled.clone(),
                    disabled.clone(),
                    disabled,
                    47.0,
                    95.0,
                    60.0,
                    25.0,
                    10,
                )
                .insert(FishingCastDisabled);
            }
            let frames: Vec<Handle<Image>> = (0..10usize)
                .filter_map(|i| load_lib_image(&mut libs, &mut images, LibraryName::Title, 170 + i))
                .collect();
            if frames.len() < 10 {
                tracing::warn!("抛竿按钮动画帧缺失：{}/10（Title[170..179]）", frames.len());
            }
            if !frames.is_empty() {
                let pressed = load_lib_image(&mut libs, &mut images, LibraryName::Title, 142);
                spawn_animated_icon_button(
                    p, frames, None, pressed, 47.0, 95.0, 60.0, 25.0, 10, 0.13, true,
                )
                .insert(FishingCast);
            }
            // 自动钓鱼开关 `Title[180/181/182]` @(110,95) 48x25
            if let (Some(n), Some(h), Some(pr)) = (
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 180),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 181),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 182),
            ) {
                spawn_icon_button(p, n, h, pr, 110.0, 95.0, 48.0, 25.0, 10).insert(FishingAutocast);
            }
            // 自动钓鱼勾选框 `Prguse[1343]`（未）/`[1344]`（勾）@(172,95) 28x25
            if let Some(off) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 1343) {
                spawn_image(p, off, 172.0, 95.0, 28.0, 25.0, 8);
            }
            if let Some(on) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 1344) {
                spawn_image(p, on, 172.0, 95.0, 28.0, 25.0, 9)
                    .insert((FishingAutocastBox, Visibility::Hidden));
            }
        });
    }

    commands.entity(panel).with_children(|p| {
        // 关闭 Prguse2[360/361/362]：旧 sprite 在 rel(220,3) 悬空面板外（200 宽），
        // 移到面板右上角 (176,3)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 360),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 361),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 362),
        ) {
            spawn_icon_button(p, n, h, pr, 176.0, 3.0, 20.0, 20.0, 10).insert(FishingClose);
        }
        // 钓具槽（C# FishingDialog Grid：Hook/Float/Bait/Finder/Reel，34x30）
        for (i, (rx, ry)) in GEAR_POS.iter().enumerate() {
            spawn_container(p, *rx, *ry, 34.0, 30.0, 9)
                .insert(BackgroundColor(Color::srgba(0.2, 0.2, 0.25, 0.9)))
                .with_children(|c| {
                    spawn_label(c, &font, "—", 4.0, 8.0, 10.0, Color::WHITE, 10)
                        .insert(FishingGearSlot(i));
                });
        }
    });
}

/// 显隐 + 渲染 + 抛竿/自动钓鱼
#[allow(clippy::too_many_arguments)]
fn fishing_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut state: ResMut<FishingState>,
    net: Res<NetConnection>,
    close: Query<(Entity, &Interaction), With<FishingClose>>,
    status_close: Query<(Entity, &Interaction), With<FishingStatusClose>>,
    cast_btn: Query<(Entity, &Interaction), With<FishingCast>>,
    autocast_btn: Query<(Entity, &Interaction), With<FishingAutocast>>,
    mut widgets: Query<&mut Visibility, With<FishingWidget>>,
    // #2926：`ChanceLabel`（C# @(14,79)）+ 进度条填充（C# `ProgressBar` 为 BeforeDraw 自绘）
    mut chance: Query<&mut Text, With<FishingChanceLabel>>,
    mut fills: Query<&mut Node, With<FishingProgressFill>>,
    // `Without<FishingWidget>` 与上面的 widgets 查询显式互斥（两者都改 Visibility，B0001）
    mut autocast_box: Query<
        &mut Visibility,
        (
            With<FishingAutocastBox>,
            Without<FishingEscTick>,
            Without<FishingWidget>,
        ),
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
    let open = mgr.is_open(DialogKind::Fishing);
    // 状态窗跟随主窗显隐（C# 由钓鱼流程同时 Show/Hide 两窗）
    crate::game::dialogs::sync_dialog_state(&mut mgr, DialogKind::FishingStatus, open);
    for mut vis in widgets.iter_mut() {
        *vis = if open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !open {
        return;
    }
    for (e, inter) in close.iter().chain(status_close.iter()) {
        if edge(e, inter, &mut prev_inter) {
            mgr.close(DialogKind::Fishing);
            mgr.close(DialogKind::FishingStatus);
        }
    }
    let status = match state.progress {
        0 => "未钓鱼".to_string(),
        1 => "等待中…".to_string(),
        2 => "上钩了！".to_string(),
        3 => "收竿中…".to_string(),
        5 => "自动钓鱼已切换".to_string(),
        _ => format!("进度 {}", state.progress),
    };
    for mut text in &mut chance {
        let want = if state.message.is_empty() {
            format!("钓鱼状态: {status}")
        } else {
            format!("钓鱼状态: {status}  {}", state.message)
        };
        if text.0 != want {
            text.0 = want;
        }
    }
    // 进度条填充宽度（C# `ProgressBar` 自绘；本端按 `progress` 0..3 映射到 0..216px）
    let frac = state.progress.clamp(0, 3) as f32 / 3.0;
    for mut node in &mut fills {
        let want = Val::Px(216.0 * frac);
        if node.width != want {
            node.width = want;
        }
    }
    // 自动钓鱼勾选框（C# `AutoCastBox.Index = _autoCast ? 1344 : 1343`）
    for mut vis in &mut autocast_box {
        let want = if state.autocast {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
    }
    // 抛竿（C# FishingDialog → C.FishingCast）
    for (e, inter) in &cast_btn {
        if edge(e, inter, &mut prev_inter) {
            net.send_packet(&crate::network::FishingCastWire { fishing_type: 0 });
            state.message = "已抛竿，等待鱼上钩…".to_string();
            tracing::info!("🎣 抛竿");
        }
    }
    // 自动钓鱼开关
    for (e, inter) in &autocast_btn {
        if edge(e, inter, &mut prev_inter) {
            state.autocast = !state.autocast;
            net.send_packet(&crate::network::FishingChangeAutocastWire {
                enabled: state.autocast,
            });
            state.message = format!("自动钓鱼: {}", if state.autocast { "开" } else { "关" });
            tracing::info!("🎣 自动钓鱼: {}", state.autocast);
        }
    }
}

/// #1313：钓具槽显示 + 点击穿戴/卸下（C# FishingDialog Grid + EquipSlotItem/RemoveSlotItem）
#[allow(clippy::too_many_arguments)]
fn fishing_gear_system(
    mgr: Res<DialogManager>,
    inv_q: Query<(&Inventory, &Loadout), With<LocalPlayer>>,
    mut inv_click: ResMut<InvClickState>,
    net: Res<NetConnection>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    mut slots: Query<(&mut Text, &FishingGearSlot)>,
) {
    let open = mgr.is_open(DialogKind::Fishing);
    let player = inv_q.single().ok();
    let rod = player
        .and_then(|(_, l)| l.slots.get(0))
        .and_then(|e| e.as_ref());
    for (mut text, slot) in &mut slots {
        let name = rod
            .and_then(|r| r.slots.get(slot.0))
            .and_then(|s| s.as_ref())
            .map(|g| g.name.clone())
            .unwrap_or_else(|| "—".to_string());
        if text.0 != name {
            text.0 = name;
        }
    }
    if !open || !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let items = player.map(|(inv, _)| inv.items.as_slice()).unwrap_or(&[]);
    for (slot, (rx, ry)) in GEAR_POS.iter().enumerate() {
        let x = 280.0 + rx;
        let y = 80.0 + ry;
        if cursor.x < x || cursor.x > x + 34.0 || cursor.y < y || cursor.y > y + 30.0 {
            continue;
        }
        let Some(rod) = rod else { return };
        // 已占用 → 卸下回背包（C# RemoveSlotItem Grid=Fishing）
        if let Some(gear) = rod.slots.get(slot).and_then(|s| s.as_ref()) {
            let Some(to) = free_inventory_index(items) else {
                tracing::warn!("🎣 背包已满，无法卸下钓具");
                return;
            };
            net.send_packet(&crate::network::RemoveSlotItemWire {
                grid: mir2_shared::enums::MirGridType::Fishing as u8,
                grid_to: mir2_shared::enums::MirGridType::Inventory as u8,
                unique_id: gear.unique_id,
                to: to as i32,
                from_unique_id: rod.unique_id,
            });
            tracing::info!("🎣 卸下钓具 {} -> 背包{}", gear.name, to);
            return;
        }
        // 空槽 + 背包已选中物品 → 穿戴（C# EquipSlotItem GridTo=Fishing）
        // #2631：选中态归 inventory 所有，经接口访问。严格对齐旧码：仅当物品确实存在
        // 才发送并清除选中；陈旧选中（物品已被移除）保留选中态，不用 take_selected。
        if let Some(sel) = inv_click.selected() {
            if let Some(item) = items.get(sel).and_then(|s| s.as_ref()) {
                net.send_packet(&mir2_shared::packets::client::misc::EquipSlotItem {
                    grid: mir2_shared::enums::MirGridType::Inventory,
                    unique_id: item.unique_id,
                    to_slot: slot as i32,
                    grid_to: mir2_shared::enums::MirGridType::Fishing,
                });
                tracing::info!("🎣 穿戴钓具 {} -> 槽{}", item.name, slot);
                inv_click.clear_selected();
            }
        }
        return;
    }
}

/// 消费服务端钓鱼事件（网络层只广播 ServerEvent；文案在此构造）
fn fishing_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut fishing: ResMut<FishingState>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        if let ServerEvent::FishingUpdate { progress, success } = ev {
            fishing.progress = *progress;
            fishing.success = *success;
            fishing.message = match progress {
                1 => "等待中…".to_string(),
                2 => {
                    if *success {
                        "上钩了！".to_string()
                    } else {
                        "鱼跑了…".to_string()
                    }
                }
                _ => "钓鱼中".to_string(),
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fishing_origin_is_csharp_center() {
        assert_eq!(
            crate::game::dialogs::center_origin(200.0, 287.0),
            (412.0, 240.0)
        );
    }

    #[test]
    fn free_inventory_index_finds_first_empty() {
        let items = vec![
            Some(InvItem::default()),
            None,
            Some(InvItem::default()),
            None,
        ];
        assert_eq!(free_inventory_index(&items), Some(1));
        let full = vec![Some(InvItem::default()); 3];
        assert_eq!(free_inventory_index(&full), None);
        let empty: Vec<Option<InvItem>> = vec![];
        assert_eq!(free_inventory_index(&empty), None);
    }
}

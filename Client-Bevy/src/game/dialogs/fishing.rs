// ============================================================================
// 钓鱼对话框（M39）
// 参考：C# FishingDialog（Prguse[1340]）+ ServerRust tick_fishing / FishingCast
// 网络：
//   C: FishingCast[fishing_type u8] / FishingChangeAutocast[enabled u8]
//   S: FishingUpdate(198)[progress i32][success u8]
// 收获结果通过系统聊天消息返回（C# ReceiveChat 语义）
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::inventory::{InvClickState, InvItem};
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::game::hud::HudState;
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiFont, UiImageCache,
};

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

/// 钓具槽（0=Hook 1=Float 2=Bait 3=Finder 4=Reel，C# FishingSlot）
#[derive(Component)] struct FishingGearSlot(usize);

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
            (fishing_ui_system, fishing_gear_system, ui_button_system)
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
const GEAR_POS: [(f32, f32); 5] = [(17.0, 203.0), (17.0, 241.0), (57.0, 241.0), (97.0, 241.0), (137.0, 241.0)];

/// 找背包第一个空格（钓具卸下目标；无空格返回 None）
fn free_inventory_index(items: &[Option<InvItem>]) -> Option<usize> {
    items.iter().position(|s| s.is_none())
}

fn spawn_fishing(
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

    // 背景 Prguse[1340]（C# FishingDialog.Index=1340）
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 1340) {
        let e = spawn_ui_sprite(&mut commands, h, 280.0, 80.0, 6.0, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::Fishing),
            FishingWidget,
            Visibility::Hidden,
        ));
    }
    // 关闭（C# Prguse2 360/361/362）
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 360, 361, 362,
        280.0 + 220.0, 83.0, 7.0, 20.0, 20.0,
    ) {
        commands.entity(e).insert((
            FishingClose,
            DialogRoot(DialogKind::Fishing),
            FishingWidget,
        ));
    }
    // 状态行
    for i in 0..4usize {
        let e = spawn_ui_text(
            &mut commands, &font, "",
            298.0, 120.0 + i as f32 * 22.0,
            12.0, Color::WHITE, 8.0,
        );
        commands.entity(e).insert((
            FishingLine(i),
            DialogRoot(DialogKind::Fishing),
            FishingWidget,
        ));
    }
    // 抛竿按钮（#90 续：MirAnimatedButton，C# FishingDialog FishButton
    // Title[170..179] 10 帧 130ms 循环 + 按下帧 142）
    if let Some(e) = crate::ui::controls::spawn_animated_button(
        &mut commands,
        &mut libs,
        &mut images,
        &mut cache,
        LibraryName::Title,
        170,
        10,
        None,
        Some(142),
        300.0,
        220.0,
        8.3,
        76.0,
        25.0,
        0.13,
        true,
    ) {
        commands.entity(e).insert((
            FishingCast,
            DialogRoot(DialogKind::Fishing),
            FishingWidget,
        ));
    }
    // 自动钓鱼开关
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 210, 211, 212,
        390.0, 220.0, 8.3, 76.0, 25.0,
    ) {
        commands.entity(e).insert((
            FishingAutocast,
            DialogRoot(DialogKind::Fishing),
            FishingWidget,
        ));
    }
    // 钓具槽（C# FishingDialog Grid：Hook/Float/Bait/Finder/Reel，34x30）
    let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    for (i, (rx, ry)) in GEAR_POS.iter().enumerate() {
        let e = spawn_ui_sprite(
            &mut commands, white.clone(), 280.0 + rx, 80.0 + ry, 8.1, 1.0,
        );
        commands.entity(e).insert((
            Sprite {
                image: white.clone(),
                color: Color::srgba(0.2, 0.2, 0.25, 0.9),
                custom_size: Some(Vec2::new(34.0, 30.0)),
                ..default()
            },
            DialogRoot(DialogKind::Fishing),
            FishingWidget,
        ));
        let t = spawn_ui_text(
            &mut commands, &font, "—",
            280.0 + rx + 4.0, 80.0 + ry + 8.0,
            10.0, Color::WHITE, 8.2,
        );
        commands.entity(t).insert((FishingGearSlot(i), DialogRoot(DialogKind::Fishing), FishingWidget));
    }
}

/// 显隐 + 渲染 + 抛竿/自动钓鱼
#[allow(clippy::too_many_arguments)]
fn fishing_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut state: ResMut<FishingState>,
    net: Res<NetConnection>,
    close: Query<&UiButton, With<FishingClose>>,
    cast_btn: Query<&UiButton, With<FishingCast>>,
    autocast_btn: Query<&UiButton, With<FishingAutocast>>,
    mut widgets: Query<&mut Visibility, With<FishingWidget>>,
    mut lines: Query<(&mut Text2d, &FishingLine)>,
) {
    let open = mgr.is_open(DialogKind::Fishing);
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        return;
    }
    for btn in &close {
        if btn.clicked {
            mgr.close(DialogKind::Fishing);
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
    for (mut text, line) in &mut lines {
        text.0 = match line.0 {
            0 => format!("钓鱼状态: {}", status),
            1 => format!("自动钓鱼: {}", if state.autocast { "开" } else { "关" }),
            2 => state.message.clone(),
            3 => "需要装备鱼竿（武器栏）".to_string(),
            _ => String::new(),
        };
    }
    // 抛竿（C# FishingDialog → C.FishingCast）
    for btn in &cast_btn {
        if btn.clicked {
            net.send_packet(&crate::network::FishingCastWire { fishing_type: 0 });
            state.message = "已抛竿，等待鱼上钩…".to_string();
            tracing::info!("🎣 抛竿");
        }
    }
    // 自动钓鱼开关
    for btn in &autocast_btn {
        if btn.clicked {
            state.autocast = !state.autocast;
            net.send_packet(&crate::network::FishingChangeAutocastWire {
                enabled: state.autocast,
            });
            state.message = format!(
                "自动钓鱼: {}",
                if state.autocast { "开" } else { "关" }
            );
            tracing::info!("🎣 自动钓鱼: {}", state.autocast);
        }
    }
}


/// #1313：钓具槽显示 + 点击穿戴/卸下（C# FishingDialog Grid + EquipSlotItem/RemoveSlotItem）
#[allow(clippy::too_many_arguments)]
fn fishing_gear_system(
    mgr: Res<DialogManager>,
    hud: Res<HudState>,
    mut inv_click: ResMut<InvClickState>,
    net: Res<NetConnection>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    mut slots: Query<(&mut Text2d, &FishingGearSlot)>
) {
    let open = mgr.is_open(DialogKind::Fishing);
    let rod = hud.equipment.get(0).and_then(|e| e.as_ref());
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
    let Some(cursor) = window.cursor_position() else { return };
    for (slot, (rx, ry)) in GEAR_POS.iter().enumerate() {
        let x = 280.0 + rx;
        let y = 80.0 + ry;
        if cursor.x < x || cursor.x > x + 34.0 || cursor.y < y || cursor.y > y + 30.0 {
            continue;
        }
        let Some(rod) = rod else { return };
        // 已占用 → 卸下回背包（C# RemoveSlotItem Grid=Fishing）
        if let Some(gear) = rod.slots.get(slot).and_then(|s| s.as_ref()) {
            let Some(to) = free_inventory_index(&hud.inventory.items) else {
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
        if let Some(sel) = inv_click.selected {
            if let Some(item) = hud.inventory.items.get(sel).and_then(|s| s.as_ref()) {
                net.send_packet(&mir2_shared::packets::client::misc::EquipSlotItem {
                    grid: mir2_shared::enums::MirGridType::Inventory,
                    unique_id: item.unique_id,
                    to_slot: slot as i32,
                    grid_to: mir2_shared::enums::MirGridType::Fishing,
                });
                tracing::info!("🎣 穿戴钓具 {} -> 槽{}", item.name, slot);
                inv_click.selected = None;
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
    fn free_inventory_index_finds_first_empty() {
        let items = vec![Some(InvItem::default()), None, Some(InvItem::default()), None];
        assert_eq!(free_inventory_index(&items), Some(1));
        let full = vec![Some(InvItem::default()); 3];
        assert_eq!(free_inventory_index(&full), None);
        let empty: Vec<Option<InvItem>> = vec![];
        assert_eq!(free_inventory_index(&empty), None);
    }
}

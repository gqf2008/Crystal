// ============================================================================
// 仓库对话框（M18）
// 布局参考：C# NPCDialogs.cs StorageDialog（10 列 x 8 行，cell 36x32 间隔 1）
//   - 背景 Prguse[586]（本移植用半透明底板），位置右侧 (600, 60)，避免与 NPC/背包重叠
//   - 关闭按钮 Prguse2[360-362]
//   - 交互（原版 C# MirItemCell 拖放语义，选中+点击）：
//       选中背包物品 → 点仓库格 → C.StoreItem{From=背包格, To=仓库格}
//       选中仓库物品 → 点背包格 → C.TakeBackItem{From=仓库格, To=背包格}
//       点已选中格取消选中；点空格清空选择
// 网络：UserStorage（服务端仓库内容）→ 显示；操作后服务端发完整 UserStorage + UserInformation 刷新
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::inventory::{inv_slot_at, InvClickState, InvItem};
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::game::hud::HudState;
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_text, ui_button_system, ui_image, UiButton, UiFont, UiImageCache,
};
use crate::ui::controls::{spawn_item_cell, ItemCell, ItemCellData, ItemCellIcon};

/// 仓库数据（网络 UserStorage 写入）
#[derive(Resource, Default)]
pub struct StorageState {
    /// 80 格仓库（服务端 STORAGE_SIZE）
    pub items: Vec<Option<InvItem>>,
    pub visible: bool,
    /// 当前选中仓库格（原版 C# GameScene.SelectedCell）
    pub selected: Option<usize>,
}

const DIALOG_X: f32 = 600.0;
const DIALOG_Y: f32 = 60.0;
const COLS: usize = 10;
const ROWS: usize = 8;
const CELL_W: f32 = 36.0;
const CELL_H: f32 = 32.0;

#[derive(Component)]
pub struct StorageWidget;

#[derive(Component)]
pub struct StorageClose;

/// 仓库格子索引（0..79）
#[derive(Component, Clone, Copy)]
pub struct StorageSlot(pub usize);

pub struct StoragePlugin;

impl Plugin for StoragePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StorageState>();
        app.add_systems(OnEnter(AppState::Game), spawn_storage_dialog);
        app.add_systems(OnExit(AppState::Game), cleanup_storage);
        app.add_systems(
            Update,
            storage_server_events.run_if(in_state(AppState::Game)),
        );
        app.add_systems(
            Update,
            (storage_ui_system, storage_action_system, ui_button_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_storage(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_storage_dialog(
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

    // 关闭按钮（Prguse2 360/361/362），右上角
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands,
        &mut libs,
        &mut images,
        &mut cache,
        LibraryName::Prguse2,
        360,
        361,
        362,
        DIALOG_X + 363.0,
        DIALOG_Y + 3.0,
        7.0,
        20.0,
        20.0,
    ) {
        commands.entity(e).insert((
            StorageClose,
            DialogRoot(DialogKind::Storage),
            StorageWidget,
        ));
    }

    // 标题文字（原版 Title[0]：仓库）
    let title = spawn_ui_text(
        &mut commands,
        &font,
        "仓库",
        DIALOG_X + 18.0,
        DIALOG_Y + 8.0,
        12.0,
        Color::WHITE,
        8.0,
    );
    commands
        .entity(title)
        .insert((DialogRoot(DialogKind::Storage), StorageWidget));

    // 格子底板（80 格，10x8）+ 物品图标 + 堆叠数量（#90 通用 ItemCell）
    for i in 0..(COLS * ROWS) {
        let x = i % COLS;
        let y = i / COLS;
        let sx = DIALOG_X + 9.0 + x as f32 * (CELL_W + 1.0);
        let sy = DIALOG_Y + 60.0 + y as f32 * (CELL_H + 1.0);
        let slot = spawn_item_cell(&mut commands, &mut images, &font, sx, sy, 6.5, CELL_W, CELL_H, i);
        commands.entity(slot).insert((
            StorageSlot(i),
            DialogRoot(DialogKind::Storage),
            StorageWidget,
        ));
    }
}

/// 光标坐标 → 仓库格
fn storage_slot_at(cx: f32, cy: f32) -> Option<usize> {
    for i in 0..(COLS * ROWS) {
        let x = i % COLS;
        let y = i / COLS;
        let sx = DIALOG_X + 9.0 + x as f32 * (CELL_W + 1.0);
        let sy = DIALOG_Y + 60.0 + y as f32 * (CELL_H + 1.0);
        if cx >= sx && cx <= sx + CELL_W && cy >= sy && cy <= sy + CELL_H {
            return Some(i);
        }
    }
    None
}

/// 显示/隐藏 + 物品图标渲染 + 选中高亮 + 关闭
#[allow(clippy::type_complexity)]
fn storage_ui_system(
    state: Res<StorageState>,
    mut mgr: ResMut<DialogManager>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<UiImageCache>,
    mut all_vis: Query<(&mut Visibility, Option<&StorageSlot>), With<StorageWidget>>,
    mut cells: Query<(&mut ItemCellData, &ItemCell), With<StorageSlot>>,
    buttons: Query<(&UiButton, Option<&StorageClose>), With<StorageWidget>>,
    mut slots: Query<(&mut Sprite, &StorageSlot), Without<ItemCellIcon>>,
) {
    let open = state.visible && mgr.is_open(DialogKind::Storage);
    for (mut vis, _slot) in &mut all_vis {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }

    // 物品图标 + 数量（#90 通用 ItemCell：只写数据，渲染由 item_cell_system 处理）
    for (mut data, cell) in &mut cells {
        let item = state.items.get(cell.slot).and_then(|s| s.as_ref());
        data.icon = item.and_then(|it| {
            ui_image(
                &mut libs,
                &mut images,
                &mut cache,
                LibraryName::Items,
                it.image as usize,
            )
        });
        data.count = item.map(|it| it.count.max(1) as u32);
    }

    // 选中高亮（原版 C# SelectedCell 黄色语义）
    for (mut sprite, slot) in &mut slots {
        let selected = state.selected == Some(slot.0);
        let target = if selected {
            Color::srgba(1.0, 0.9, 0.2, 0.35)
        } else {
            Color::srgba(0.0, 0.0, 0.0, 0.18)
        };
        if sprite.color != target {
            sprite.color = target;
        }
    }

    // 关闭按钮
    for (btn, close) in &buttons {
        if btn.clicked && close.is_some() {
            mgr.close(DialogKind::Storage);
        }
    }
}

/// 仓库交互：选中+点击 存入/取出（原版 C# MirItemCell 拖放语义）
#[allow(clippy::too_many_arguments)]
fn storage_action_system(
    mut state: ResMut<StorageState>,
    mut inv_click: ResMut<InvClickState>,
    _hud: Res<HudState>,
    net: Res<NetConnection>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
) {
    if !state.visible || !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else { return };

    let storage_slot = storage_slot_at(cursor.x, cursor.y);
    let inv_slot = inv_slot_at(cursor.x, cursor.y);

    // 1) 选中了背包物品 → 点仓库格：存入（原版 C# SelectedCell Inventory → Storage 拖放）
    if let Some(from) = inv_click.selected {
        if let Some(to) = storage_slot {
            net.send_packet(&mir2_shared::packets::client::item::StoreItem {
                from: from as i32,
                to: to as i32,
            });
            tracing::info!("📦 存入仓库 {} -> {}", from, to);
            inv_click.selected = None;
            state.selected = None;
            return;
        }
    }

    // 2) 选中了仓库物品 → 点背包格：取出（原版 C# SelectedCell Storage → Inventory 拖放）
    if let Some(from) = state.selected {
        if let Some(to) = inv_slot {
            net.send_packet(&mir2_shared::packets::client::item::TakeBackItem {
                from: from as i32,
                to: to as i32,
            });
            tracing::info!("📦 取出仓库 {} -> {}", from, to);
            state.selected = None;
            inv_click.selected = None;
            return;
        }
    }

    // 3) 点仓库格：选中/取消选中（只有物品格可选中）
    if let Some(i) = storage_slot {
        match state.selected {
            Some(sel) if sel == i => state.selected = None,
            _ => {
                if state.items.get(i).and_then(|s| s.as_ref()).is_some() {
                    state.selected = Some(i);
                    inv_click.selected = None;
                }
            }
        }
    }

    // 4) 点背包物品格：交给背包系统（选中）；这里仅清掉仓库选择
    if inv_slot.is_some() {
        state.selected = None;
    }
}


/// 消费服务端仓库事件（网络层只广播 ServerEvent；仓库/背包打开逻辑归本模块）
fn storage_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut storage: ResMut<StorageState>,
    mut mgr: ResMut<DialogManager>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        if let ServerEvent::StorageOpened { items, visible } = ev {
            storage.items = items.clone();
            storage.visible = *visible;
            // 原版 C#：仓库打开时同时显示背包
            if !mgr.is_open(DialogKind::Storage) {
                mgr.open.push(DialogKind::Storage);
            }
            if !mgr.is_open(DialogKind::Inventory) {
                mgr.open.push(DialogKind::Inventory);
            }
        }
    }
}

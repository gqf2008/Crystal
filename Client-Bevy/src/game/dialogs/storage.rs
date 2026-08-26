// ============================================================================
// 仓库对话框（M18）
// 布局参考：C# NPCDialogs.cs StorageDialog（10 列 x 8 行，cell 36x32 间隔 1）
//   - 背景 Prguse[586]（实测 388x346），原点 (0,0)（C# StorageDialog
//     Location = new Point(0, 0)，NPCDialogs.cs:2807；格子/按钮偏移同 C#）
//   - 关闭按钮 Prguse2[360-362]
//   - 交互（原版 C# MirItemCell 拖放语义，选中+点击）：
//       选中背包物品 → 点仓库格 → C.StoreItem{From=背包格, To=仓库格}
//       选中仓库物品 → 点背包格 → C.TakeBackItem{From=仓库格, To=背包格}
//       点已选中格取消选中；点空格清空选择
// 网络：UserStorage（服务端仓库内容）→ 显示；操作后服务端发完整 UserStorage + UserInformation 刷新
// ============================================================================

use bevy::prelude::*;

use crate::actor::LocalPlayer;
use crate::game::dialogs::inventory::{
    InvClickState, InvDropConfirm, InvItem, InvUiState, ItemUseFeedback, UseItemCtx, UseOutcome,
    inv_slot_at, item_use_sound_id, use_item_core,
};
use crate::game::dialogs::text_input::{TextInputDisplay, TextInputField, TextInputRect};
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::game::player_state::{Inventory, Loadout, StatusFlags};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::UiFont;
use crate::ui::theme::{
    load_lib_image, spawn_container, spawn_icon_button, spawn_item_cell_ui_root, spawn_label,
    spawn_panel, UiItemCell, UiItemCellData, UiItemCellIcon,
};

/// 仓库数据（网络 UserStorage 写入）
#[derive(Resource, Default)]
pub struct StorageState {
    /// 80 格仓库（服务端 STORAGE_SIZE）
    pub items: Vec<Option<InvItem>>,
    pub visible: bool,
    /// 当前选中仓库格（原版 C# GameScene.SelectedCell）
    pub selected: Option<usize>,
    /// 仓库密码面板是否打开
    pub pwd_panel: bool,
    /// 仓库密码操作结果提示
    pub pwd_msg: String,
    /// 仓库解锁面板是否打开（#200：C# StorageDialog PromptStorageUnlock）
    pub unlock_panel: bool,
    /// 仓库解锁结果提示（#200）
    pub unlock_msg: String,
}

impl StorageState {
    /// 按服务端 ResizeStorage 调整格数（C# Array.Resize：截断/补空，上限 COLS*ROWS=80，#281）
    pub fn resize(&mut self, size: usize) {
        let size = size.min(COLS * ROWS);
        if size < self.items.len() {
            self.items.truncate(size);
        } else {
            self.items.resize(size, None);
        }
    }
}

/// 窗口原点 (0,0)：C# StorageDialog 显式 `Location = new Point(0, 0)`（NPCDialogs.cs:2812）。
/// 旧值 (600,60) 是移植期自定右置，与 C# 左上角原点不符。
const DIALOG_X: f32 = 0.0;
const DIALOG_Y: f32 = 0.0;
/// 仓库宽（Prguse[586] 实测 388x346）。C# Show 时背包推到 (仓宽+5, 仓Y)=(393,0) 并排
/// （NPCDialogs.cs:2967/2990）——避免仓库完全罩住背包
const STORAGE_W: f32 = 388.0;
const COLS: usize = 10;
const ROWS: usize = 8;
const CELL_W: f32 = 36.0;
const CELL_H: f32 = 32.0;
/// 面板/格子 GlobalZIndex：格子是根节点（非面板子实体），bevy 0.19 根节点按
/// GlobalZIndex 升序绘制——格子 z 必须高于面板 z，否则被面板背景盖住
/// （批38-40 评审 P0；密码/解锁覆盖层 45/46 之上不可盖）
pub const STORAGE_PANEL_Z: i32 = 30;
pub const STORAGE_CELL_Z: i32 = 31;

#[derive(Component)]
pub struct StorageWidget;

#[derive(Component)]
pub struct StorageClose;

/// 仓库密码按钮（C# StorageDialog ProtectButton）
#[derive(Component)]
pub struct StoragePwdBtn;

/// 仓库密码面板
#[derive(Component)]
pub struct StoragePwdPanel;
#[derive(Component)]
pub struct StoragePwdSet;
#[derive(Component)]
pub struct StoragePwdRemove;
#[derive(Component)]
pub struct StoragePwdClose;
#[derive(Component)]
pub struct StoragePwdMsg;

/// 仓库解锁面板（#200）
#[derive(Component)]
pub struct StorageUnlockPanel;
#[derive(Component)]
pub struct StorageUnlockOk;
#[derive(Component)]
pub struct StorageUnlockCancel;
#[derive(Component)]
pub struct StorageUnlockMsg;

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
            (
                storage_grid_sync_system,
                storage_ui_system,
                storage_action_system,
                storage_tooltip_system,
                storage_pwd_system,
                storage_unlock_system,
            )
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
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<UiFont>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();

    // 背景 Prguse[586]（C# StorageDialog.Index=586，实测 388x346 @ (0,0)）
    let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 586) else {
        return;
    };
    let panel = spawn_panel(&mut commands, bg, DIALOG_X, DIALOG_Y, STORAGE_W, 346.0, STORAGE_PANEL_Z);
    commands
        .entity(panel)
        .insert((DialogRoot(DialogKind::Storage), StorageWidget));

    commands.entity(panel).with_children(|p| {
        // 关闭按钮（Prguse2 360/361/362）@(363,3)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 360),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 361),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 362),
        ) {
            spawn_icon_button(p, n, h, pr, 363.0, 3.0, 20.0, 20.0, 10).insert(StorageClose);
        }
        // 标题文字
        spawn_label(p, &font, "仓库", 18.0, 8.0, 12.0, Color::WHITE, 9);
        // 仓库密码按钮 + 标签 @(18,330)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 206),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 207),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 208),
        ) {
            spawn_icon_button(p, n, h, pr, 18.0, 330.0, 76.0, 23.0, 10).insert(StoragePwdBtn);
        }
        spawn_label(p, &font, "仓库密码", 34.0, 334.0, 12.0, Color::WHITE, 11);
    });

    // 密码面板（根节点覆盖层 300x150 @ (18,360)，GlobalZIndex 45）
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(DIALOG_X + 18.0),
                top: Val::Px(DIALOG_Y + 360.0),
                width: Val::Px(300.0),
                height: Val::Px(150.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.1, 0.1, 0.15, 0.95)),
            StoragePwdPanel,
            DialogRoot(DialogKind::Storage),
            GlobalZIndex(45),
            Visibility::Hidden,
        ))
        .with_children(|p| {
            for (id, label, y) in [(0usize, "当前密码:", 370.0f32), (1, "新密码:", 400.0)] {
                spawn_label(p, &font, label, 28.0, y - 360.0, 12.0, Color::WHITE, 10);
                spawn_container(p, 100.0, y - 360.0, 200.0, 20.0, 10)
                    .insert((
                        BackgroundColor(Color::srgba(0.2, 0.2, 0.25, 0.9)),
                        crate::game::dialogs::text_input::TextInputField(id),
                        // 屏幕系命中框：容器是密码面板（根 @ x+18）的子实体，
                        // 相对 x=100 → 绝对 x+18+100=118（旧值漏加面板 18 偏移）
                        crate::game::dialogs::text_input::TextInputRect(DIALOG_X + 18.0 + 100.0, y, 200.0, 20.0),
                    ))
                    .with_children(|ic| {
                        ic.spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Px(4.0),
                                top: Val::Px(2.0),
                                ..default()
                            },
                            Text::new(String::new()),
                            TextFont {
                                font: FontSource::Handle(font.clone()),
                                font_size: FontSize::Px(12.0),
                                ..default()
                            },
                            TextColor(Color::WHITE),
                            ZIndex(11),
                            crate::game::dialogs::text_input::TextInputDisplay(id),
                        ));
                    });
            }
            spawn_label(p, &font, "", 28.0, 70.0, 12.0, Color::srgb(1.0, 0.9, 0.4), 11)
                .insert(StoragePwdMsg);
            // 设置 / 移除 / 关闭
            if let (Some(n), Some(h), Some(pr)) = (
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 206),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 207),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 208),
            ) {
                spawn_icon_button(p, n.clone(), h.clone(), pr.clone(), 28.0, 95.0, 70.0, 23.0, 10)
                    .insert(StoragePwdSet);
                spawn_label(p, &font, "设置", 43.0, 99.0, 12.0, Color::WHITE, 11);
                spawn_icon_button(p, n, h, pr, 108.0, 95.0, 70.0, 23.0, 10).insert(StoragePwdRemove);
                spawn_label(p, &font, "移除", 123.0, 99.0, 12.0, Color::WHITE, 11);
            }
            if let (Some(n), Some(h), Some(pr)) = (
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 210),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 211),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 212),
            ) {
                spawn_icon_button(p, n, h, pr, 188.0, 95.0, 70.0, 23.0, 10).insert(StoragePwdClose);
                spawn_label(p, &font, "关闭", 203.0, 99.0, 12.0, Color::WHITE, 11);
            }
        });

    // 解锁面板（根节点覆盖层 300x120 @ (18,180)，GlobalZIndex 46）
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(DIALOG_X + 18.0),
                top: Val::Px(DIALOG_Y + 180.0),
                width: Val::Px(300.0),
                height: Val::Px(120.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.1, 0.1, 0.15, 0.95)),
            StorageUnlockPanel,
            DialogRoot(DialogKind::Storage),
            GlobalZIndex(46),
            Visibility::Hidden,
        ))
        .with_children(|p| {
            spawn_label(p, &font, "请输入仓库密码", 28.0, 10.0, 12.0, Color::WHITE, 10);
            spawn_container(p, 100.0, 15.0, 200.0, 20.0, 10)
                .insert((
                    BackgroundColor(Color::srgba(0.2, 0.2, 0.25, 0.9)),
                    crate::game::dialogs::text_input::TextInputField(2),
                    // 同上：解锁面板根 @ x+18，容器相对 x=100 → 绝对 118
                    crate::game::dialogs::text_input::TextInputRect(DIALOG_X + 18.0 + 100.0, DIALOG_Y + 195.0, 200.0, 20.0),
                ))
                .with_children(|ic| {
                    ic.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(4.0),
                            top: Val::Px(2.0),
                            ..default()
                        },
                        Text::new(String::new()),
                        TextFont {
                            font: FontSource::Handle(font.clone()),
                            font_size: FontSize::Px(12.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        ZIndex(11),
                        crate::game::dialogs::text_input::TextInputDisplay(2),
                    ));
                });
            spawn_label(p, &font, "", 28.0, 45.0, 12.0, Color::srgb(1.0, 0.6, 0.4), 11)
                .insert(StorageUnlockMsg);
            if let (Some(n), Some(h), Some(pr)) = (
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 206),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 207),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 208),
            ) {
                spawn_icon_button(p, n.clone(), h.clone(), pr.clone(), 100.0, 75.0, 70.0, 23.0, 10)
                    .insert(StorageUnlockOk);
                spawn_label(p, &font, "确定", 115.0, 79.0, 12.0, Color::WHITE, 11);
            }
            if let (Some(n), Some(h), Some(pr)) = (
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 210),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 211),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 212),
            ) {
                spawn_icon_button(p, n, h, pr, 188.0, 75.0, 70.0, 23.0, 10).insert(StorageUnlockCancel);
                spawn_label(p, &font, "取消", 203.0, 79.0, 12.0, Color::WHITE, 11);
            }
        });

    // 格子底板不在此预生成：#281 由 storage_grid_sync_system 动态生成
}

/// 光标坐标 → 仓库格（按实际格数，#281）
fn storage_slot_at(cx: f32, cy: f32, size: usize) -> Option<usize> {
    for i in 0..size.min(COLS * ROWS) {
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
    mut all_vis: Query<(&mut Visibility, Option<&StorageSlot>), With<StorageWidget>>,
    mut cells: Query<(&mut UiItemCellData, &UiItemCell), With<StorageSlot>>,
    buttons: Query<(Entity, &Interaction, Option<&StorageClose>), With<StorageWidget>>,
    mut slots: Query<(&StorageSlot, &mut BackgroundColor), Without<UiItemCellIcon>>,
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
    let open = state.visible && mgr.is_open(DialogKind::Storage);
    for (mut vis, _slot) in &mut all_vis {
        *vis = if open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    // 物品图标 + 数量（#90 通用 UiItemCell：只写数据，渲染由 item_cell_ui_system 处理）
    for (mut data, cell) in &mut cells {
        let item = state.items.get(cell.slot).and_then(|s| s.as_ref());
        let icon = item.and_then(|it| {
            load_lib_image(
                &mut libs,
                &mut images,
                LibraryName::Items,
                it.image as usize,
            )
        });
        let count = item.map(|it| it.count.max(1) as u32);
        // 性能（#112）：无变化不写，避免每帧标记 Changed
        if data.icon.as_ref() != icon.as_ref() {
            data.icon = icon;
        }
        if data.count != count {
            data.count = count;
        }
    }

    // 选中高亮（原版 C# SelectedCell 黄色语义）
    for (slot, mut bg) in &mut slots {
        let selected = state.selected == Some(slot.0);
        let target = if selected {
            Color::srgba(1.0, 0.9, 0.2, 0.35)
        } else {
            Color::srgba(0.0, 0.0, 0.0, 0.18)
        };
        if bg.0 != target {
            bg.0 = target;
        }
    }

    // 关闭按钮
    for (e, inter, close) in &buttons {
        if edge(e, inter, &mut prev_inter) && close.is_some() {
            mgr.close(DialogKind::Storage);
        }
    }
}

/// 仓库交互：选中+点击 存入/取出（原版 C# MirItemCell 拖放语义）
#[allow(clippy::too_many_arguments)]
fn storage_action_system(
    mut state: ResMut<StorageState>,
    mut inv_click: ResMut<InvClickState>,
    // #2633 批次4 步7：gender/class/level/riding 改读组件（HudState 已于步9 删除）
    player_q: Query<
        (
            &Inventory,
            &StatusFlags,
            &Loadout,
            &crate::actor::ActorAppearance,
            &crate::game::player_state::Progression,
            Option<&crate::actor::MountState>,
        ),
        With<LocalPlayer>,
    >,
    inv_ui: Res<InvUiState>,
    net: Res<NetConnection>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    time: Res<Time>,
    inv_origin: Res<crate::game::dialogs::inventory::InventoryOrigin>,
    mut feedback: ResMut<ItemUseFeedback>,
    mut confirm: ResMut<InvDropConfirm>,
    mut last_storage_click: Local<Option<(usize, f64)>>,
) {
    if !state.visible || !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else {
        return;
    };

    let player = player_q.single().ok();
    let storage_slot = storage_slot_at(cursor.x, cursor.y, state.items.len());
    let inv_slot = inv_slot_at(
        cursor.x,
        cursor.y,
        inv_ui.page,
        player
            .map(|(inv, _, _, _, _, _)| inv.items.len())
            .unwrap_or(0),
        (inv_origin.0, inv_origin.1),
    );

    // #1546：仓库格双击 → 装备（C# MirItemCell.OnMouseDoubleClick → UseItem；消耗品要求 Grid==Inventory/HeroInventory 故仓库拦截）
    let now = time.elapsed_secs_f64();
    let mut dbl_storage = false;
    if let Some(i) = storage_slot {
        if let Some((last_i, last_t)) = *last_storage_click {
            if last_i == i && now - last_t < 0.4 {
                dbl_storage = true;
                *last_storage_click = None;
            } else {
                *last_storage_click = Some((i, now));
            }
        } else {
            *last_storage_click = Some((i, now));
        }
    }
    if dbl_storage {
        if let Some(item) = state
            .items
            .get(storage_slot.unwrap())
            .and_then(|s| s.as_ref())
        {
            let ctx = UseItemCtx {
                grid: mir2_shared::enums::MirGridType::Storage,
                equipment: player
                    .map(|(_, _, l, _, _, _)| l.slots.as_slice())
                    .unwrap_or(&[]),
                gender: player.map(|(_, _, _, a, _, _)| a.gender as u8).unwrap_or(0),
                class: player.map(|(_, _, _, a, _, _)| a.class as u8).unwrap_or(0),
                level: player.map(|(_, _, _, _, p, _)| p.level).unwrap_or(1),
                check_fishing: true,
                allow_consumable: false,
            };
            if use_item_core(
                item,
                &net,
                // 实体缺失视同未骑乘（原 hud.riding=false 默认）
                player
                    .map(|(_, _, _, _, _, m)| m.is_some())
                    .unwrap_or(false),
                player.map(|(_, f, _, _, _, _)| f.fishing).unwrap_or(false),
                player
                    .map(|(_, _, l, _, _, _)| l.slots.as_slice())
                    .unwrap_or(&[]),
                ctx,
                now,
                &mut feedback,
                &mut confirm,
            ) == UseOutcome::Sent
            {
                if let Some(sid) = item_use_sound_id(item) {
                    feedback.sounds.push(sid);
                }
            }
        }
        state.selected = None;
        return;
    }

    // 1) 选中了背包物品 → 点仓库格：存入（原版 C# SelectedCell Inventory → Storage 拖放）
    // #2631：选中态归 inventory 所有，经 selected() 读、clear_selected() 清（存入后不再保留）
    if let Some(from) = inv_click.selected() {
        if let Some(to) = storage_slot {
            inv_click.clear_selected();
            net.send_packet(&mir2_shared::packets::client::item::StoreItem {
                from: from as i32,
                to: to as i32,
            });
            tracing::info!("📦 存入仓库 {} -> {}", from, to);
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
            inv_click.clear_selected(); // #2631：经接口清（互斥）
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
                    inv_click.clear_selected(); // #2631：经接口清（与背包选中互斥）
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
/// #2633 批次4 步9：ItemStored/ItemTakenBack 移动背包格直接写 `Inventory` 组件（HudState 已删）。
fn storage_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut storage: ResMut<StorageState>,
    mut mgr: ResMut<DialogManager>,
    mut inv_origin: ResMut<crate::game::dialogs::inventory::InventoryOrigin>,
    // bevy_ui 迁移：背包面板根 Node.left = 屏幕 x，子节点随根整体平移
    mut inv_entities: Query<(&mut Node, &DialogRoot)>,
    mut inv_q: Query<&mut Inventory, With<LocalPlayer>>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        if let ServerEvent::StorageOpened { items, visible } = ev {
            storage.items = items.clone();
            storage.visible = *visible;
            // 原版 C#：仓库打开时同时显示背包，且背包推到 (仓宽+5, 仓Y)=(393,0)
            // 并排（NPCDialogs.cs:2967/2990 `InventoryDialog.Location = new Point(Size.Width+5, Location.Y)`）
            // —— 否则 388x346 的仓库完全罩住 316x236 的背包。
            let mut min_x = f32::MAX;
            for (node, root) in inv_entities.iter() {
                if root.0 == DialogKind::Inventory {
                    if let Val::Px(v) = node.left {
                        min_x = min_x.min(v);
                    }
                }
            }
            if min_x < f32::MAX {
                let dx = STORAGE_W + 5.0 - min_x;
                for (mut node, root) in &mut inv_entities {
                    if root.0 == DialogKind::Inventory {
                        let cur = match node.left {
                            Val::Px(v) => v,
                            _ => 0.0,
                        };
                        node.left = Val::Px(cur + dx);
                    }
                }
                *inv_origin =
                    crate::game::dialogs::inventory::InventoryOrigin(STORAGE_W + 5.0, 0.0);
            }
            if !mgr.is_open(DialogKind::Storage) {
                mgr.open.push(DialogKind::Storage);
            }
            if !mgr.is_open(DialogKind::Inventory) {
                mgr.open.push(DialogKind::Inventory);
            }
        }
        if let ServerEvent::StoragePasswordResult { result } = ev {
            // C# result：4=成功 2=当前密码错误 5=未设置密码
            storage.pwd_msg = match *result {
                4 => "仓库密码已保存".to_string(),
                2 => "当前密码错误".to_string(),
                5 => "未设置仓库密码".to_string(),
                _ => "仓库密码操作失败".to_string(),
            };
        }
        if let ServerEvent::StoragePrompt = ev {
            // #200：NPCStorage —— 有密码的仓库先弹解锁框（C# StorageDialog.Show → PromptStorageUnlock）
            storage.unlock_panel = true;
            storage.unlock_msg.clear();
        }
        if let ServerEvent::StorageResized { size } = ev {
            // #281：仓库扩容（C# S.ResizeStorage → Array.Resize + RefreshStorage2）
            storage.resize(*size);
            tracing::info!("📦 仓库扩容 -> {} 格", storage.items.len());
        }
        if let ServerEvent::StorageUnlockResult {
            result,
            has_password,
        } = ev
        {
            // C# result：0=成功 1=格式错 2=密码错 3=不可用 4=无密码直接解锁
            let _ = has_password;
            match *result {
                0 | 4 => {
                    storage.unlock_panel = false;
                    storage.unlock_msg.clear();
                }
                1 => storage.unlock_msg = "仓库密码格式不正确".to_string(),
                2 => storage.unlock_msg = "仓库密码错误".to_string(),
                3 => storage.unlock_msg = "无法使用仓库".to_string(),
                _ => storage.unlock_msg = "仓库解锁失败".to_string(),
            }
        }
        if let ServerEvent::ItemStored { from, to, success } = ev {
            // #512：C# S.StoreItem —— 背包 -> 仓库（success 时移动物品）
            if *success {
                let (fi, ti) = (*from as usize, *to as usize);
                if let Ok(mut inv) = inv_q.single_mut() {
                    if fi < inv.items.len() && ti < storage.items.len() {
                        if let Some(item) = inv.items[fi].take() {
                            if storage.items[ti].is_none() {
                                storage.items[ti] = Some(item);
                                tracing::info!("📦 存入仓库 {} -> {}（{}）", from, to, "成功");
                            } else {
                                inv.items[fi] = Some(item);
                                tracing::warn!("📦 存入仓库 {} -> {} 失败：目标格已占用", from, to);
                            }
                        }
                    }
                }
            }
        }
        if let ServerEvent::ItemTakenBack { from, to, success } = ev {
            // #512：C# S.TakeBackItem —— 仓库 -> 背包（success 时移动物品）
            if *success {
                let (fi, ti) = (*from as usize, *to as usize);
                if let Ok(mut inv) = inv_q.single_mut() {
                    if fi < storage.items.len() && ti < inv.items.len() {
                        if let Some(item) = storage.items[fi].take() {
                            if inv.items[ti].is_none() {
                                inv.items[ti] = Some(item);
                                tracing::info!("📦 取出仓库 {} -> {}（{}）", from, to, "成功");
                            } else {
                                storage.items[fi] = Some(item);
                                tracing::warn!("📦 取出仓库 {} -> {} 失败：目标格已占用", from, to);
                            }
                        }
                    }
                }
            }
        }
    }
}

/// 悬停提示（#93 通用 Tooltip）：光标在仓库物品格上显示 名称 x数量
fn storage_tooltip_system(
    state: Res<StorageState>,
    mut tooltip: ResMut<crate::ui::tooltip::TooltipState>,
    windows: Query<&Window>,
) {
    if !state.visible {
        tooltip.update(3, false, String::new(), Vec::new(), 0.0, 0.0);
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let mut hit: Option<crate::game::dialogs::inventory::InvItem> = None;
    if let Some(i) = storage_slot_at(cursor.x, cursor.y, state.items.len()) {
        hit = state.items.get(i).and_then(|s| s.as_ref()).cloned();
    }
    let Some(item) = hit else {
        tooltip.update(3, false, String::new(), Vec::new(), cursor.x, cursor.y);
        return;
    };
    // 与背包一致：完整属性行（#1244 item_tooltip_lines）
    let lines = crate::game::dialogs::inventory::item_tooltip_lines(&item);
    tooltip.update(3, true, item.name.clone(), lines, cursor.x, cursor.y);
}

/// 仓库密码面板：按钮开关 + 设置/移除/关闭 + 结果提示
fn storage_pwd_system(
    mut storage: ResMut<StorageState>,
    net: Res<NetConnection>,
    mut input: ResMut<crate::game::dialogs::text_input::TextInputState>,
    pwd_btn: Query<(Entity, &Interaction), With<StoragePwdBtn>>,
    set_btn: Query<(Entity, &Interaction), With<StoragePwdSet>>,
    remove_btn: Query<(Entity, &Interaction), With<StoragePwdRemove>>,
    close_btn: Query<(Entity, &Interaction), With<StoragePwdClose>>,
    mut panel: Query<&mut Visibility, (With<StoragePwdPanel>, Without<StoragePwdBtn>)>,
    mut msg: Query<&mut Text, With<StoragePwdMsg>>,
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
    let open = storage.pwd_panel;
    for mut vis in &mut panel {
        *vis = if open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for mut t in &mut msg {
        if t.0 != storage.pwd_msg {
            t.0 = storage.pwd_msg.clone();
        }
    }
    for (e, inter) in &pwd_btn {
        if edge(e, inter, &mut prev_inter) {
            storage.pwd_panel = !storage.pwd_panel;
            storage.pwd_msg.clear();
            if input.texts.len() < 2 {
                input.texts.resize(2, String::new());
            }
            input.active = None;
        }
    }
    for (e, inter) in &set_btn {
        if edge(e, inter, &mut prev_inter) && open {
            let current = input.texts.get(0).cloned().unwrap_or_default();
            let new = input.texts.get(1).cloned().unwrap_or_default();
            net.send_packet(&mir2_shared::packets::client::storage::SetStoragePassword {
                current_password: current,
                new_password: new,
            });
            tracing::info!("🔒 设置仓库密码");
        }
    }
    for (e, inter) in &remove_btn {
        if edge(e, inter, &mut prev_inter) && open {
            let current = input.texts.get(0).cloned().unwrap_or_default();
            net.send_packet(
                &mir2_shared::packets::client::storage::RemoveStoragePassword {
                    current_password: current,
                },
            );
            tracing::info!("🔓 移除仓库密码");
        }
    }
    for (e, inter) in &close_btn {
        if edge(e, inter, &mut prev_inter) && open {
            storage.pwd_panel = false;
            input.active = None;
        }
    }
}

/// 仓库解锁面板：输入密码 → C.UnlockStorage；取消关闭（#200，C# PromptStorageUnlock）
fn storage_unlock_system(
    mut storage: ResMut<StorageState>,
    net: Res<NetConnection>,
    mut input: ResMut<crate::game::dialogs::text_input::TextInputState>,
    ok_btn: Query<(Entity, &Interaction), With<StorageUnlockOk>>,
    cancel_btn: Query<(Entity, &Interaction), With<StorageUnlockCancel>>,
    mut panel: Query<&mut Visibility, With<StorageUnlockPanel>>,
    mut msg: Query<&mut Text, With<StorageUnlockMsg>>,
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
    let open = storage.unlock_panel;
    for mut vis in &mut panel {
        *vis = if open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for mut t in &mut msg {
        if t.0 != storage.unlock_msg {
            t.0 = storage.unlock_msg.clone();
        }
    }
    if open && input.texts.len() < 3 {
        input.texts.resize(3, String::new());
    }
    for (e, inter) in &ok_btn {
        if edge(e, inter, &mut prev_inter) && open {
            let password = input.texts.get(2).cloned().unwrap_or_default();
            net.send_packet(&mir2_shared::packets::client::storage::UnlockStorage { password });
            tracing::info!("🔓 发送仓库解锁请求");
            if let Some(t) = input.texts.get_mut(2) {
                t.clear();
            }
            input.active = None;
        }
    }
    for (e, inter) in &cancel_btn {
        if edge(e, inter, &mut prev_inter) && open {
            storage.unlock_panel = false;
            storage.unlock_msg.clear();
            input.active = None;
        }
    }
}

/// 仓库动态格子同步（#281）：按 StorageState.items.len() 生成/移除 StorageSlot 格子。
/// 对齐 C# StorageDialog Grid（10x8=80 上限）；缩容时移除多余格子。
fn storage_grid_sync_system(
    mut commands: Commands,
    state: Res<StorageState>,
    mut images: ResMut<Assets<Image>>,
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<UiFont>,
    slots: Query<(Entity, &StorageSlot)>,
) {
    let size = state.items.len().min(COLS * ROWS);
    if state.items.is_empty() && slots.is_empty() {
        return; // 进图 UserStorage 到达前：无格子可同步
    }
    // 缩容：移除超出 size 的格子
    for (e, s) in &slots {
        if s.0 >= size {
            commands.entity(e).despawn();
        }
    }
    let mut existing: Vec<usize> = slots
        .iter()
        .map(|(_, s)| s.0)
        .filter(|i| *i < size)
        .collect();
    existing.sort_unstable();
    if existing.len() == size {
        return;
    }
    // 扩容：补缺失格子
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();
    let mut next = 0usize;
    for i in 0..size {
        if existing.get(next).copied() == Some(i) {
            next += 1;
            continue;
        }
        let x = i % COLS;
        let y = i / COLS;
        let sx = DIALOG_X + 9.0 + x as f32 * (CELL_W + 1.0);
        let sy = DIALOG_Y + 60.0 + y as f32 * (CELL_H + 1.0);
        let cell = spawn_item_cell_ui_root(
            &mut commands,
            &mut images,
            &font,
            sx,
            sy,
            CELL_W,
            CELL_H,
            STORAGE_CELL_Z,
            i,
        );
        commands.entity(cell).insert((
            StorageSlot(i),
            DialogRoot(DialogKind::Storage),
            StorageWidget,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// C# StorageDialog `Location = new Point(0, 0)`（NPCDialogs.cs:2807）→ 左上角原点。
    /// 旧值 (600,60) 是移植期自定右置。
    #[test]
    fn storage_origin_matches_csharp() {
        assert_eq!(DIALOG_X, 0.0);
        assert_eq!(DIALOG_Y, 0.0);
        // 格网起点 (9,60)、步进 (37,33)（C# x*36+9+x, y%8*32+60+y%8，NPCDialogs.cs:2945）
        assert_eq!(DIALOG_X + 9.0 + 0.0 * (CELL_W + 1.0), 9.0);
        assert_eq!(DIALOG_Y + 60.0 + 0.0 * (CELL_H + 1.0), 60.0);
        assert_eq!(DIALOG_X + 9.0 + 9.0 * (CELL_W + 1.0), 342.0);
        assert_eq!(DIALOG_Y + 60.0 + 7.0 * (CELL_H + 1.0), 291.0);
    }

    /// 格子与面板同为根节点（GlobalZIndex 参与根排序）：格子必须高于面板
    /// （否则被面板背景盖住），且低于密码/解锁覆盖层 45/46（覆盖层应罩住格子）
    #[test]
    fn storage_cell_z_above_panel() {
        assert!(
            STORAGE_CELL_Z > STORAGE_PANEL_Z,
            "格子 z({STORAGE_CELL_Z}) 必须高于面板 z({STORAGE_PANEL_Z})"
        );
        assert!(STORAGE_CELL_Z < 45, "格子应低于密码/解锁覆盖层(45/46)");
    }

    fn mk_item(uid: u64) -> InvItem {
        InvItem {
            unique_id: uid,
            ..Default::default()
        }
    }

    fn storage_test_app() -> App {
        use crate::network::server_event::ServerEvent;
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<ServerEvent>();
        app.init_resource::<StorageState>();
        app.init_resource::<DialogManager>();
        app.insert_resource(crate::game::dialogs::inventory::InventoryOrigin(0.0, 0.0));
        app.add_systems(Update, storage_server_events);
        app
    }

    fn inv_items(app: &mut App) -> Vec<Option<u64>> {
        app.world_mut()
            .query_filtered::<&Inventory, With<LocalPlayer>>()
            .iter(app.world())
            .next()
            .map(|inv| {
                inv.items
                    .iter()
                    .map(|s| s.as_ref().map(|it| it.unique_id))
                    .collect()
            })
            .expect("LocalPlayer 应有 Inventory")
    }

    /// 存入仓库成功后背包物品直接从 `Inventory` 组件移除（#2633 批次4 步9 直写组件，R1 实体缺失跳过）。
    #[test]
    fn item_stored_removes_from_component() {
        use crate::network::server_event::ServerEvent;
        let mut app = storage_test_app();
        app.world_mut().spawn((
            LocalPlayer,
            Inventory {
                items: vec![Some(mk_item(31)), None],
                ..Default::default()
            },
        ));
        {
            let mut storage = app.world_mut().resource_mut::<StorageState>();
            storage.items = vec![None, None];
        }
        app.update(); // 初始化消息缓冲/系统状态

        // 背包格 0 (uid=31) 存入仓库格 0 → Inventory 组件背包格 0 清空
        app.world_mut().write_message(ServerEvent::ItemStored {
            from: 0,
            to: 0,
            success: true,
        });
        app.update();
        let storage = app.world().resource::<StorageState>();
        assert!(
            storage.items[0].as_ref().map(|it| it.unique_id) == Some(31),
            "仓库格 0 应收下 uid=31"
        );
        assert_eq!(
            inv_items(&mut app),
            vec![None, None],
            "Inventory 组件背包格 0 应被存入移除"
        );
    }

    /// 从仓库取回成功后背包物品直接写入 `Inventory` 组件（#2633 批次4 步9 直写组件，R1 实体缺失跳过）。
    #[test]
    fn item_taken_back_writes_component() {
        use crate::network::server_event::ServerEvent;
        let mut app = storage_test_app();
        app.world_mut().spawn((
            LocalPlayer,
            Inventory {
                items: vec![None, None],
                ..Default::default()
            },
        ));
        {
            let mut storage = app.world_mut().resource_mut::<StorageState>();
            storage.items = vec![Some(mk_item(47)), None];
        }
        app.update();

        // 仓库格 0 (uid=47) 取回背包格 1 → Inventory 组件背包格 1 收下
        app.world_mut().write_message(ServerEvent::ItemTakenBack {
            from: 0,
            to: 1,
            success: true,
        });
        app.update();
        let storage = app.world().resource::<StorageState>();
        assert!(storage.items[0].is_none(), "仓库格 0 应被取回清空");
        assert_eq!(
            inv_items(&mut app),
            vec![None, Some(47)],
            "Inventory 组件背包格 1 应收下 uid=47"
        );
    }
}

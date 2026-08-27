// ============================================================================
// NPC 出售/修理面板（M20）
// 布局参考：C# NPCDialogs.cs NPCDropDialog
//   - 背景 Prguse[392]，位置 (264,224)
//   - 确认按钮 Title[290-292] (114,62)；物品格 (38,72)
//   - 交互（原版 C# MirItemCell 拖放语义）：
//       点背包物品选中（SelectedCell）→ 点面板拖放区放入 TargetItem
//       → 点确认：Sell 卖整叠（C.SellItem{uid, count=整叠数量}）/ Repair 修理（C.RepairItem{uid}）
// 网络：NPCGoods(panel_type=Sell/Repair) → 打开面板（C# GameScene.NPCSell/NPCRepair）
// ============================================================================

use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::actor::LocalPlayer;
use crate::game::dialogs::inventory::{InvClickState, InvItem};
use crate::game::dialogs::{DialogKind, DialogRoot};
use crate::game::player_state::Inventory;
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::UiFont;
use crate::ui::theme::{
    load_lib_image, spawn_container, spawn_icon_button, spawn_image, spawn_label, spawn_panel,
};
use mir2_shared::enums::PanelType;

/// 出售/修理面板状态
#[derive(Resource, Default)]
pub struct SellPanelState {
    pub visible: bool,
    /// 当前模式（Sell / Repair / SpecialRepair）
    pub mode: Option<PanelType>,
    /// 面板中的目标物品（原版 C# NPCDropDialog.TargetItem）
    pub target: Option<InvItem>,
}

const DIALOG_X: f32 = 264.0;
const DIALOG_Y: f32 = 224.0;

#[derive(Component)]
pub struct SellPanelWidget;

#[derive(Component)]
pub struct SellPanelConfirm;

/// 拖放区（原版 C# ItemCell / NPCDropPanel_Click 的 (20,55,75,75) 区域）
#[derive(Component)]
pub struct SellPanelDrop;

/// 目标物品图标（拖放区子实体）
#[derive(Component)]
pub struct SellPanelIcon;

/// 提示文本（InfoLabel）
#[derive(Component)]
pub struct SellPanelInfo;

pub struct SellPanelPlugin;

impl Plugin for SellPanelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SellPanelState>();
        app.add_systems(
            Update,
            sell_panel_server_events.run_if(in_state(AppState::Game)),
        );
        app.add_systems(OnEnter(AppState::Game), spawn_sell_panel);
        app.add_systems(OnExit(AppState::Game), cleanup_sell_panel);
        app.add_systems(
            Update,
            (sell_panel_ui_system, sell_panel_action_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_sell_panel(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_sell_panel(
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

    // 背景 Prguse[392]（C# NPCDropDialog，176x146 @ (264,224)）
    let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 392) else {
        return;
    };
    let panel = spawn_panel(&mut commands, bg, DIALOG_X, DIALOG_Y, 176.0, 146.0, 30);
    commands
        .entity(panel)
        .insert((SellPanelWidget, DialogRoot(DialogKind::Npc)));

    commands.entity(panel).with_children(|p| {
        // 确认按钮 Title[290/291/292]（C# ConfirmButton (114,62)）
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 290),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 291),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 292),
        ) {
            spawn_icon_button(p, n, h, pr, 114.0, 62.0, 48.0, 25.0, 10)
                .insert(SellPanelConfirm);
        }
        // 提示文本（C# InfoLabel (30,10)）
        spawn_label(p, &font, "把物品放入面板后点确认", 30.0, 10.0, 12.0, Color::WHITE, 9)
            .insert(SellPanelInfo);
        // 拖放区（C# ItemCell (38,72) 区域 (20,55,75,75)）+ 目标图标
        spawn_container(p, 20.0, 55.0, 75.0, 75.0, 9)
            .insert((
                SellPanelDrop,
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.35)),
            ))
            .with_children(|c| {
                let white = images.add(crate::map_renderer::make_image(
                    vec![255, 255, 255, 255],
                    1,
                    1,
                ));
                spawn_image(c, white, 3.0, 3.0, 68.0, 68.0, 10).insert(SellPanelIcon);
            });
    });
}

/// 显示/隐藏 + 提示文本 + 目标物品图标
fn sell_panel_ui_system(
    state: Res<SellPanelState>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut widgets: Query<&mut Visibility, (With<SellPanelWidget>, Without<SellPanelIcon>)>,
    mut icons: Query<(&mut ImageNode, &mut Visibility), With<SellPanelIcon>>,
    mut info_texts: Query<(&mut Text, &SellPanelInfo)>,
) {
    for mut vis in &mut widgets {
        *vis = if state.visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for (mut text, _) in &mut info_texts {
        let new = match state.mode {
            Some(PanelType::Repair) | Some(PanelType::SpecialRepair) => "放入物品后点确认修理",
            _ => "放入物品后点确认出售",
        }
        .to_string();
        if text.0 != new {
            text.0 = new;
        }
    }

    // 目标物品图标
    for (mut node, mut vis) in &mut icons {
        match state.target.as_ref() {
            Some(item) => {
                let handle = load_lib_image(
                    &mut libs,
                    &mut images,
                    LibraryName::Items,
                    item.image as usize,
                );
                match handle {
                    Some(h) if node.image != h => node.image = h,
                    None => *vis = Visibility::Hidden,
                    _ => {}
                }
                if node.image.is_strong() {
                    *vis = Visibility::Visible;
                }
            }
            None => *vis = Visibility::Hidden,
        }
    }
}

/// 交互：点拖放区放入选中物品；点确认出售/修理
#[allow(clippy::too_many_arguments)]
fn sell_panel_action_system(
    mut state: ResMut<SellPanelState>,
    mut inv_click: ResMut<InvClickState>,
    inv_q: Query<&Inventory, With<LocalPlayer>>,
    net: Res<NetConnection>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    // 面板原点（拖后跟随；挂在 Npc kind 组随 NPC 对话框联合拖动/置顶）
    panel_origin: Query<&Node, With<SellPanelWidget>>,
    confirm_btns: Query<(Entity, &Interaction), With<SellPanelConfirm>>,
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
    if !state.visible {
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else {
        return;
    };

    // 点拖放区（C# NPCDropPanel_Click 区域 (20,55,75,75) 相对面板）：放入选中物品
    // （面板原点动态取——面板挂 Npc kind 可被联合拖动，固定 DIALOG_X/Y 会成死区）
    if mouse.just_pressed(MouseButton::Left) {
        let (ox, oy) = panel_origin
            .single()
            .map(|n| crate::ui::theme::node_origin(n, (DIALOG_X, DIALOG_Y)))
            .unwrap_or((DIALOG_X, DIALOG_Y));
        let dx = ox + 20.0;
        let dy = oy + 55.0;
        if cursor.x >= dx && cursor.x <= dx + 75.0 && cursor.y >= dy && cursor.y <= dy + 75.0 {
            // #2631：选中态归 inventory 所有，经接口访问。严格对齐旧码：仅当物品确实存在
            // 才放入并清除选中；陈旧选中（物品已被移除）保留选中态，不用 take_selected。
            if let Some(sel) = inv_click.selected() {
                if let Some(item) = inv_q
                    .single()
                    .ok()
                    .and_then(|inv| inv.items.get(sel).and_then(|s| s.as_ref()))
                {
                    state.target = Some(item.clone());
                    tracing::info!(
                        "🎯 放入面板: {} (uid={}) x{}",
                        item.name,
                        item.unique_id,
                        item.count
                    );
                    inv_click.clear_selected();
                }
            }
        }
    }

    // 点背包物品时若面板已打开且无选中 → 交给背包系统选中（原版 C# SelectedCell）
    // （这里只负责面板拖放区与确认）

    // 确认按钮
    for (e, inter) in &confirm_btns {
        if !edge(e, inter, &mut prev_inter) {
            continue;
        }
        let Some(item) = state.target.take() else {
            continue;
        };
        match state.mode {
            Some(PanelType::Sell) => {
                // 原版 C# Confirm：C.SellItem{UniqueID, Count=TargetItem.Count}（卖整叠）
                net.send_packet(&mir2_shared::packets::client::npc::SellItem {
                    unique_id: item.unique_id,
                    count: item.count.max(1),
                });
                tracing::info!(
                    "💰 面板出售 {} (uid={}) x{}",
                    item.name,
                    item.unique_id,
                    item.count
                );
            }
            Some(PanelType::Repair) | Some(PanelType::SpecialRepair) => {
                net.send_packet(&mir2_shared::packets::client::npc::RepairItem {
                    unique_id: item.unique_id,
                });
                tracing::info!("🔧 面板修理 {} (uid={})", item.name, item.unique_id);
            }
            _ => {}
        }
    }
}

/// 消费服务端出售/修理面板事件（网络层只广播 ServerEvent）
fn sell_panel_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut sell_panel: ResMut<SellPanelState>,
    mut mgr: ResMut<crate::game::dialogs::DialogManager>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        if let ServerEvent::NpcSellPanel { panel_type } = ev {
            sell_panel.mode = Some(*panel_type);
            sell_panel.target = None;
            sell_panel.visible = true;
            // C# NPCDropDialog.Show() 同时打开背包
            if !mgr.is_open(crate::game::dialogs::DialogKind::Inventory) {
                mgr.open.push(crate::game::dialogs::DialogKind::Inventory);
            }
        }
    }
}

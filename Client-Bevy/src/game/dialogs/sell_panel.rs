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

use crate::game::dialogs::inventory::{InvClickState, InvItem};
use crate::game::dialogs::{DialogKind, DialogRoot};
use crate::game::hud::HudState;
use crate::map_renderer::GameLibraries;
use crate::network::NetworkContext;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiEntity, UiFont,
    UiImageCache,
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
        app.add_systems(OnEnter(AppState::Game), spawn_sell_panel);
        app.add_systems(OnExit(AppState::Game), cleanup_sell_panel);
        app.add_systems(
            Update,
            (sell_panel_ui_system, sell_panel_action_system, ui_button_system)
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
    mut cache: ResMut<UiImageCache>,
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<UiFont>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();

    // 背景 Prguse[392]（C# NPCDropDialog）
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 392) {
        let e = spawn_ui_sprite(&mut commands, h, DIALOG_X, DIALOG_Y, 6.0, 1.0);
        commands
            .entity(e)
            .insert((SellPanelWidget, DialogRoot(DialogKind::Npc), Visibility::Hidden));
    }

    // 确认按钮 Title[290/291/292]（C# ConfirmButton）
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands,
        &mut libs,
        &mut images,
        &mut cache,
        LibraryName::Title,
        290,
        291,
        292,
        DIALOG_X + 114.0,
        DIALOG_Y + 62.0,
        7.0,
        32.0,
        23.0,
    ) {
        commands.entity(e).insert((
            SellPanelConfirm,
            SellPanelWidget,
            DialogRoot(DialogKind::Npc),
        ));
    }

    // 提示文本（C# InfoLabel (30,10)）
    let info = spawn_ui_text(
        &mut commands,
        &font,
        "把物品放入面板后点确认",
        DIALOG_X + 30.0,
        DIALOG_Y + 10.0,
        12.0,
        Color::WHITE,
        8.0,
    );
    commands.entity(info).insert((
        SellPanelInfo,
        SellPanelWidget,
        DialogRoot(DialogKind::Npc),
    ));

    // 拖放区（C# ItemCell (38,72)，36x32 格子 + 边框底色）
    let white = images.add(crate::map_renderer::make_image(
        vec![255, 255, 255, 255],
        1,
        1,
    ));
    let cell = commands
        .spawn((
            UiEntity,
            DialogRoot(DialogKind::Npc),
            SellPanelWidget,
            SellPanelDrop,
            Sprite {
                image: white.clone(),
                color: Color::srgba(0.0, 0.0, 0.0, 0.35),
                custom_size: Some(Vec2::new(75.0, 75.0)),
                ..default()
            },
            Anchor::TOP_LEFT,
            Transform::from_xyz(DIALOG_X + 20.0, -(DIALOG_Y + 55.0), 6.5),
            Visibility::Hidden,
        ))
        .id();
    commands.entity(cell).with_children(|p| {
        p.spawn((
            SellPanelIcon,
            Sprite {
                image: white.clone(),
                custom_size: Some(Vec2::new(68.0, 68.0)),
                ..default()
            },
            Anchor::TOP_LEFT,
            Transform::from_xyz(3.0, -3.0, 6.6),
            Visibility::Hidden,
        ));
    });
}

/// 显示/隐藏 + 提示文本 + 目标物品图标
fn sell_panel_ui_system(
    state: Res<SellPanelState>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<UiImageCache>,
    mut widgets: Query<
        (&mut Visibility, Option<&SellPanelInfo>),
        (With<SellPanelWidget>, Without<SellPanelIcon>),
    >,
    mut icons: Query<
        (&mut Sprite, &mut Visibility),
        (With<SellPanelIcon>, Without<SellPanelWidget>),
    >,
    mut info_texts: Query<(&mut Text2d, &SellPanelInfo)>,
) {
    for (mut vis, info) in &mut widgets {
        if info.is_some() {
            continue;
        }
        *vis = if state.visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for (mut text, _) in &mut info_texts {
        text.0 = match state.mode {
            Some(PanelType::Repair) | Some(PanelType::SpecialRepair) => "放入物品后点确认修理",
            _ => "放入物品后点确认出售",
        }
        .to_string();
    }

    // 目标物品图标
    for (mut sprite, mut vis) in &mut icons {
        match state.target.as_ref() {
            Some(item) => {
                let handle = ui_image(
                    &mut libs,
                    &mut images,
                    &mut cache,
                    LibraryName::Items,
                    item.image as usize,
                );
                match handle {
                    Some(h) if sprite.image != h => sprite.image = h,
                    None => *vis = Visibility::Hidden,
                    _ => {}
                }
                if sprite.image.is_strong() {
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
    hud: Res<HudState>,
    net: Res<NetworkContext>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    confirm_btns: Query<&UiButton, With<SellPanelConfirm>>,
) {
    if !state.visible {
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else { return };

    // 点拖放区（C# NPCDropPanel_Click 区域 (20,55,75,75) 相对面板）：放入选中物品
    if mouse.just_pressed(MouseButton::Left) {
        let dx = DIALOG_X + 20.0;
        let dy = DIALOG_Y + 55.0;
        if cursor.x >= dx && cursor.x <= dx + 75.0 && cursor.y >= dy && cursor.y <= dy + 75.0 {
            if let Some(sel) = inv_click.selected {
                if let Some(item) = hud.inventory.items.get(sel).and_then(|s| s.as_ref()) {
                    state.target = Some(item.clone());
                    inv_click.selected = None;
                    tracing::info!("🎯 放入面板: {} (uid={}) x{}", item.name, item.unique_id, item.count);
                }
            }
        }
    }

    // 点背包物品时若面板已打开且无选中 → 交给背包系统选中（原版 C# SelectedCell）
    // （这里只负责面板拖放区与确认）

    // 确认按钮
    for btn in &confirm_btns {
        if !btn.clicked {
            continue;
        }
        let Some(item) = state.target.take() else { continue };
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
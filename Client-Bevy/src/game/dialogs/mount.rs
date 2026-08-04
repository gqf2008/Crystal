// ============================================================================
// 坐骑对话框（M60）
// 参考：C# MountDialog（Client/MirScenes/Dialogs/MountDialog.cs）
//   - 面板 Prguse[160/167]（按孔数 4/5 切换）@ (10,30)
//   - 名称/忠诚度标签、骑乘按钮 Prguse[155/156/157] (262,70)、关闭/帮助
//   - 坐骑装备栏 5 格（Reins/Bells/Saddle/Ribbon/Mask @ (36/90/144/198/252, 323)）
//   - 骑乘按钮 → Chat "@ride"（C# Ride()；服务端 RIDE 命令切换 + 广播外观）
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::game::hud::HudState;
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiFont, UiImageCache,
};

const PANEL_X: f32 = 10.0;
const PANEL_Y: f32 = 30.0;

#[derive(Component)]
pub struct MountWidget;

#[derive(Component)]
pub struct MountClose;

#[derive(Component)]
pub struct MountRide;

#[derive(Component)]
pub struct MountPanel;

#[derive(Component)]
pub struct MountNameText;

#[derive(Component)]
pub struct MountLoyaltyText;

/// 坐骑装备格（index = Reins/Bells/Saddle/Ribbon/Mask）
#[derive(Component)]
pub struct MountGearCell(pub usize);

pub struct MountPlugin;

impl Plugin for MountPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Game), spawn_mount);
        app.add_systems(OnExit(AppState::Game), cleanup_mount);
        app.add_systems(
            Update,
            (mount_ui_system, ui_button_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_mount(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_mount(
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

    let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));

    // 面板（默认 5 孔 167）
    let panel = spawn_ui_sprite(&mut commands, white.clone(), PANEL_X, PANEL_Y, 6.0, 1.0);
    commands.entity(panel).insert((
        Sprite {
            image: white.clone(),
            custom_size: Some(Vec2::new(324.0, 377.0)),
            ..default()
        },
        MountPanel,
        DialogRoot(DialogKind::Mount),
        MountWidget,
        Visibility::Hidden,
    ));

    // 名称/忠诚度
    let e = spawn_ui_text(
        &mut commands, &font, "",
        PANEL_X + 30.0, PANEL_Y + 40.0, 15.0, Color::WHITE, 8.0,
    );
    commands.entity(e).insert((
        MountNameText,
        DialogRoot(DialogKind::Mount),
        MountWidget,
        Visibility::Hidden,
    ));
    let e = spawn_ui_text(
        &mut commands, &font, "",
        PANEL_X + 30.0, PANEL_Y + 60.0, 12.0, Color::WHITE, 8.0,
    );
    commands.entity(e).insert((
        MountLoyaltyText,
        DialogRoot(DialogKind::Mount),
        MountWidget,
        Visibility::Hidden,
    ));

    // 骑乘按钮 Prguse[155/156/157] (262,70) 36x32
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse, 155, 156, 157,
        PANEL_X + 262.0, PANEL_Y + 70.0, 7.0, 36.0, 32.0,
    ) {
        commands.entity(e).insert((
            MountRide,
            DialogRoot(DialogKind::Mount),
            MountWidget,
            Visibility::Hidden,
        ));
    }

    // 关闭 Prguse2[360/361/362] (297,3)、帮助 Prguse2[257/258/259] (274,3)
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 360, 361, 362,
        PANEL_X + 297.0, PANEL_Y + 3.0, 7.0, 24.0, 21.0,
    ) {
        commands.entity(e).insert((
            MountClose,
            DialogRoot(DialogKind::Mount),
            MountWidget,
            Visibility::Hidden,
        ));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 257, 258, 259,
        PANEL_X + 274.0, PANEL_Y + 3.0, 7.0, 24.0, 21.0,
    ) {
        commands.entity(e).insert((
            DialogRoot(DialogKind::Mount),
            MountWidget,
            Visibility::Hidden,
        ));
    }

    // 坐骑装备格 5 个（(36/90/144/198/252, 323)）
    for i in 0..5usize {
        let x = 36.0 + i as f32 * 54.0;
        let e = spawn_ui_sprite(&mut commands, white.clone(), PANEL_X + x, PANEL_Y + 323.0, 6.1, 1.0);
        commands.entity(e).insert((
            Sprite {
                image: white.clone(),
                custom_size: Some(Vec2::new(34.0, 30.0)),
                ..default()
            },
            MountGearCell(i),
            DialogRoot(DialogKind::Mount),
            MountWidget,
            Visibility::Hidden,
        ));
    }
}

#[allow(clippy::too_many_arguments)]
fn mount_ui_system(
    mut mgr: ResMut<DialogManager>,
    hud: Res<HudState>,
    net: ResMut<NetConnection>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<UiImageCache>,
    close: Query<&UiButton, With<MountClose>>,
    ride: Query<&UiButton, With<MountRide>>,
    mut widgets: Query<&mut Visibility, (With<MountWidget>, Without<MountGearCell>)>,
    mut panel: Query<(&mut Sprite, &MountPanel), Without<MountGearCell>>,
    mut names: Query<(&mut Text2d, Option<&MountNameText>, Option<&MountLoyaltyText>)>,
    mut gears: Query<(&mut Visibility, &mut Sprite, &MountGearCell)>,
    mut logged: Local<bool>,
) {
    use mir2_shared::packets::client::chat::Chat;

    let open = mgr.is_open(DialogKind::Mount);
    for mut vis in &mut widgets {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        *logged = false;
        return;
    }

    for btn in &close {
        if btn.clicked {
            mgr.close(DialogKind::Mount);
        }
    }
    for btn in &ride {
        if btn.clicked {
            net.send_packet(&Chat {
                message: "@ride".to_string(),
                linked_items: Vec::new(),
            });
            tracing::info!("🐴 请求骑乘/下马 (@ride)");
        }
    }

    // 坐骑物品 = 装备槽 10（Mount）
    let mount = hud.equipment.get(10).and_then(|s| s.as_ref());

    // 面板按坐骑孔数换图（4→160, 5→167）
    if let Ok((mut sprite, _)) = panel.single_mut() {
        let slot_count = mount.map(|m| m.slots.len()).unwrap_or(0);
        let idx = if slot_count == 4 { 160 } else { 167 };
        if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, idx) {
            if sprite.image != h {
                sprite.image = h;
                sprite.custom_size = None;
            }
        }
    }

    // 名称/忠诚度
    for (mut text, name, loyalty) in &mut names {
        if name.is_some() {
            text.0 = mount.map(|m| m.name.clone()).unwrap_or_default();
        } else if loyalty.is_some() {
            text.0 = mount
                .map(|m| format!("忠诚度 {}/{}", m.current_dura, m.max_dura))
                .unwrap_or_default();
        }
    }

    // 装备格：坐骑 gem 图标（slots[0..4]）
    for (mut vis, mut sprite, cell) in &mut gears {
        let gem = mount
            .and_then(|m| m.slots.get(cell.0))
            .and_then(|s| s.as_ref());
        let mut show = false;
        if let Some(g) = gem {
            if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Items, g.image as usize) {
                sprite.image = h;
                sprite.custom_size = None;
                show = true;
            }
        }
        *vis = if show { Visibility::Visible } else { Visibility::Hidden };
    }

    // E2E 日志
    if !*logged {
        match mount {
            Some(m) => tracing::info!(
                "🐴 坐骑: {} (耐久 {}/{}, {} 孔, 鞍={})",
                m.name,
                m.current_dura,
                m.max_dura,
                m.slots.len(),
                m.slots.get(2).and_then(|s| s.as_ref()).is_some()
            ),
            None => tracing::info!("🐴 坐骑: 未装备"),
        }
        *logged = true;
    }
}

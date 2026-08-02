// ============================================================================
#![allow(clippy::type_complexity)]
// 小地图（M9 第 1 批）
// 布局参考：C# MainDialogs.cs MiniMapDialog
//   - 位置 (ScreenWidth-126, 0)，背景 Prguse[2090]（大）/ Prguse[2091]（小）
//   - 地图显示区（小模式）：(3, 22, 120, 108)，深绿底 + 网格 + 玩家位置点
//   - 地图名标签 (2,2)、坐标标签 (46, Height-23)
// ============================================================================

use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::actor::LocalPlayer;
use crate::game::movement::world_to_tile;
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::{GameData, GameLibraries};
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiEntity, UiFont,
    UiImageCache,
};

const MINIMAP_X: f32 = 1024.0 - 126.0;
const MINIMAP_Y: f32 = 0.0;

/// 小地图显示区（小模式）
const MAP_RECT: (f32, f32, f32, f32) = (3.0, 22.0, 120.0, 108.0);

#[derive(Component)]
pub struct MiniMapWidget;

#[derive(Component)]
pub struct MiniMapBg;

#[derive(Component)]
pub struct MiniMapPlayerDot;

#[derive(Component)]
pub struct MiniMapNameText;

#[derive(Component)]
pub struct MiniMapPosText;

pub struct MiniMapPlugin;

impl Plugin for MiniMapPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Game), spawn_minimap);
        app.add_systems(OnExit(AppState::Game), cleanup_minimap);
        app.add_systems(
            Update,
            (minimap_toggle_system, minimap_ui_system, ui_button_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_minimap(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_minimap(
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

    // 背景 Prguse[2091]（小模式默认）
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 2091) {
        let e = spawn_ui_sprite(&mut commands, h, MINIMAP_X, MINIMAP_Y, 5.0, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::Minimap),
            MiniMapWidget,
            MiniMapBg,
            Visibility::Hidden,
        ));
    }

    // 地图区域底色（深绿矩形）
    let green = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    commands.spawn((
        UiEntity,
        DialogRoot(DialogKind::Minimap),
        MiniMapWidget,
        Sprite {
            image: green.clone(),
            color: Color::srgb(0.12, 0.16, 0.12),
            custom_size: Some(Vec2::new(MAP_RECT.2, MAP_RECT.3)),
            ..default()
        },
        Anchor::TOP_LEFT,
        Transform::from_xyz(MINIMAP_X + MAP_RECT.0, -(MINIMAP_Y + MAP_RECT.1), 5.1),
        Visibility::Hidden,
    ));

    // 玩家位置点（红点）
    let red = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    commands.spawn((
        UiEntity,
        DialogRoot(DialogKind::Minimap),
        MiniMapWidget,
        MiniMapPlayerDot,
        Sprite {
            image: red,
            color: Color::srgb(1.0, 0.1, 0.1),
            custom_size: Some(Vec2::new(4.0, 4.0)),
            ..default()
        },
        Anchor::TOP_LEFT,
        Transform::from_xyz(MINIMAP_X + MAP_RECT.0, -(MINIMAP_Y + MAP_RECT.1), 5.2),
        Visibility::Hidden,
    ));

    // 地图名（居中）
    let name = spawn_ui_text(
        &mut commands, &font, "",
        MINIMAP_X + 2.0 + 10.0, MINIMAP_Y + 2.0,
        12.0, Color::WHITE, 5.3,
    );
    commands.entity(name).insert((
        DialogRoot(DialogKind::Minimap),
        MiniMapWidget,
        MiniMapNameText,
    ));

    // 坐标
    let pos = spawn_ui_text(
        &mut commands, &font, "",
        MINIMAP_X + 46.0 + 8.0, MINIMAP_Y + 108.0 - 23.0,
        12.0, Color::WHITE, 5.3,
    );
    commands.entity(pos).insert((
        DialogRoot(DialogKind::Minimap),
        MiniMapWidget,
        MiniMapPosText,
    ));
}

/// 显示/隐藏 + 玩家位置点/地图名/坐标更新
/// M 键切换小地图（原版 KeybindOptions.Minimap）
fn minimap_toggle_system(
    mut mgr: ResMut<DialogManager>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    if keys.just_pressed(KeyCode::KeyM) {
        mgr.toggle(DialogKind::Minimap);
    }
}

fn minimap_ui_system(
    mgr: Res<DialogManager>,
    game_data: Res<GameData>,
    players: Query<&Transform, (With<LocalPlayer>, Without<MiniMapPlayerDot>)>,
    mut widgets: Query<&mut Visibility, (With<MiniMapWidget>, Without<MiniMapPlayerDot>)>,
    mut dot: Query<(&mut Visibility, &mut Transform), (With<MiniMapPlayerDot>, Without<MiniMapWidget>)>,
    mut name_texts: Query<&mut Text2d, (With<MiniMapNameText>, Without<MiniMapPosText>)>,
    mut pos_texts: Query<&mut Text2d, (With<MiniMapPosText>, Without<MiniMapNameText>)>,
) {
    let open = mgr.is_open(DialogKind::Minimap);
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }

    let (map_w, map_h) = match &game_data.map {
        Some(m) => (m.width as f32, m.height as f32),
        None => (1.0, 1.0),
    };

    if let Ok((mut dot_vis, mut dot_tf)) = dot.single_mut() {
        if !open {
            *dot_vis = Visibility::Hidden;
        } else {
            *dot_vis = Visibility::Visible;
            // 玩家格子坐标 → 小地图像素
            if let Ok(player_tf) = players.single() {
                let (tx, ty) = world_to_tile(player_tf.translation.x, player_tf.translation.y);
                let px = MINIMAP_X + MAP_RECT.0 + (tx as f32 / map_w) * MAP_RECT.2;
                let py = MINIMAP_Y + MAP_RECT.1 + (ty as f32 / map_h) * MAP_RECT.3;
                dot_tf.translation.x = px - 2.0;
                dot_tf.translation.y = -(py - 2.0);
                if let Ok(mut t) = pos_texts.single_mut() {
                    t.0 = format!("{},{}", tx, ty);
                }
            }
        }
    }

    if let Ok(mut t) = name_texts.single_mut() {
        t.0 = game_data.map.as_ref().map(|m| m.name.clone()).unwrap_or_default();
    }
}

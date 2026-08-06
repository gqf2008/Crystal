// ============================================================================
// 罗盘（M9 第 1 批）
// 参考：macroquad compass_dialog.rs / C# CompassDialog
//   - 底图 Prguse2[1470]，位置 (10,10)，点击可切换显示
// ============================================================================

use bevy::prelude::*;

use crate::map_renderer::GameLibraries;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{spawn_ui_sprite, ui_image, UiFont, UiImageCache};

#[derive(Component)]
pub struct CompassWidget;

/// #250 罗盘箭头（指向任务目标）
#[derive(Component)]
pub struct CompassArrow;

/// #250 罗盘目标状态（S.SetCompass 写入）
#[derive(Resource, Default)]
pub struct CompassState {
    pub target: Option<(i32, i32)>,
}

#[derive(Resource, Default)]
pub struct CompassVisible(pub bool);

pub struct CompassPlugin;

impl Plugin for CompassPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CompassVisible>();
        app.init_resource::<CompassState>();
        app.add_systems(OnEnter(AppState::Game), spawn_compass);
        app.add_systems(OnExit(AppState::Game), cleanup_compass);
        app.add_systems(
            Update,
            (
                compass_ui_system,
                compass_target_system,
                compass_arrow_system,
            )
                .chain()
                .after(crate::network::network_system)
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_compass(mut commands: Commands, roots: Query<Entity, With<CompassWidget>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_compass(
    mut commands: Commands,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<UiImageCache>,
    _fonts: ResMut<Assets<Font>>,
    _ui_font: ResMut<UiFont>,
) {
    libs.0.ensure_initialized();
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse2, 1470) {
        let e = spawn_ui_sprite(&mut commands, h, 10.0, 10.0, 5.0, 1.0);
        commands.entity(e).insert((CompassWidget, Visibility::Visible));
    }
    // #250：罗盘箭头（6x6 金色方块，初始指向右；按目标方向旋转）
    let tri = images.add(crate::map_renderer::make_image(
        (0..6 * 6 * 4)
            .map(|i| [255u8, 220, 50, 255][i % 4])
            .collect(),
        6,
        6,
    ));
    let arrow = commands
        .spawn((
            CompassArrow,
            Sprite::from_image(tri),
            bevy::sprite::Anchor::CENTER,
            Transform::from_xyz(40.0, -40.0, 5.1),
            Visibility::Hidden,
        ))
        .id();
    let _ = arrow;
}

/// 点击罗盘切换显示（原版：打开/关闭）
fn compass_ui_system(
    mut visible: ResMut<CompassVisible>,
    mut widgets: Query<&mut Visibility, With<CompassWidget>>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
) {
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else { return };
    if mouse.just_pressed(MouseButton::Left) {
        // 罗盘区域 (10,10) 大小约 60x60
        if cursor.x >= 10.0 && cursor.x <= 70.0 && cursor.y >= 10.0 && cursor.y <= 70.0 {
            visible.0 = !visible.0;
        }
    }
    for mut vis in widgets.iter_mut() {
        *vis = if visible.0 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

/// #250：S.SetCompass → 更新目标
fn compass_target_system(
    mut state: ResMut<CompassState>,
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
) {
    for ev in events.read() {
        if let crate::network::server_event::ServerEvent::CompassTarget { x, y } = ev {
            state.target = Some((*x, *y));
            tracing::info!("🧭 [COMPASS] 目标 ({},{})", x, y);
        }
    }
}

/// #250：箭头按玩家→目标方向旋转（UI y 向下，取 -atan2）
fn compass_arrow_system(
    state: Res<CompassState>,
    visible: Res<CompassVisible>,
    mut arrows: Query<(&mut Transform, &mut Visibility), With<CompassArrow>>,
    players: Query<
        &Transform,
        (
            With<crate::actor::LocalPlayer>,
            With<crate::actor::NetObjectId>,
            Without<CompassArrow>,
        ),
    >,
) {
    let Ok(mut arrow) = arrows.single_mut() else {
        return;
    };
    let Some((tx, ty)) = state.target else {
        *arrow.1 = Visibility::Hidden;
        return;
    };
    if !visible.0 {
        *arrow.1 = Visibility::Hidden;
        return;
    }
    *arrow.1 = Visibility::Visible;
    let angle = if let Ok(pf) = players.single() {
        let (px, py) = crate::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
        let dx = (tx - px) as f32;
        let dy = (ty - py) as f32;
        (dy).atan2(dx)
    } else {
        0.0
    };
    arrow.0.rotation = Quat::from_rotation_z(-angle);
}

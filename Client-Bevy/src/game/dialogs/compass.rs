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

#[derive(Resource, Default)]
pub struct CompassVisible(pub bool);

pub struct CompassPlugin;

impl Plugin for CompassPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CompassVisible>();
        app.add_systems(OnEnter(AppState::Game), spawn_compass);
        app.add_systems(OnExit(AppState::Game), cleanup_compass);
        app.add_systems(
            Update,
            (compass_ui_system,).run_if(in_state(AppState::Game)),
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

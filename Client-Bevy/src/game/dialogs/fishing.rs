// ============================================================================
// 钓鱼对话框（M9 第 4 批）
// 布局参考：macroquad fishing_dialog.rs（背景 Prguse[1340]）
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiFont, UiImageCache,
};

#[derive(Resource, Default)]
pub struct FishingState {
    pub lines: Vec<String>,
}

#[derive(Component)]
pub struct FishingWidget;

#[derive(Component)]
pub struct FishingClose;

#[derive(Component)]
pub struct FishingLine(usize);

pub struct FishingPlugin;

impl Plugin for FishingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FishingState>();
        app.add_systems(OnEnter(AppState::Game), spawn_fishing);
        app.add_systems(OnExit(AppState::Game), cleanup_fishing);
        app.add_systems(
            Update,
            (fishing_ui_system, ui_button_system)
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

    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 1340) {
        let e = spawn_ui_sprite(&mut commands, h, 280.0, 100.0, 6.0, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::Fishing),
            FishingWidget,
            Visibility::Hidden,
        ));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 360, 361, 362,
        280.0 + 320.0, 100.0 + 3.0, 7.0, 20.0, 20.0,
    ) {
        commands.entity(e).insert((
            FishingClose,
            DialogRoot(DialogKind::Fishing),
            FishingWidget,
        ));
    }
    for i in 0..8usize {
        let e = spawn_ui_text(
            &mut commands, &font, "",
            280.0 + 8.0, 100.0 + 60.0 + i as f32 * 20.0,
            12.0, Color::WHITE, 8.0,
        );
        commands.entity(e).insert((
            FishingLine(i),
            DialogRoot(DialogKind::Fishing),
            FishingWidget,
        ));
    }
}

fn fishing_ui_system(
    mut mgr: ResMut<DialogManager>,
    state: Res<FishingState>,
    close: Query<&UiButton, With<FishingClose>>,
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
    for (mut text, line) in &mut lines {
        text.0 = state.lines.get(line.0).cloned().unwrap_or_default();
    }
}

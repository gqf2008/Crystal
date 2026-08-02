// ============================================================================
// 举报对话框（M9 第 3 批）
// 布局参考：macroquad report_dialog.rs（背景 Prguse[1633]，位置 居中）
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
pub struct ReportState {
    pub lines: Vec<String>,
}

#[derive(Component)]
pub struct ReportWidget;

#[derive(Component)]
pub struct ReportClose;

#[derive(Component)]
pub struct ReportLine(usize);

pub struct ReportPlugin;

impl Plugin for ReportPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ReportState>();
        app.add_systems(OnEnter(AppState::Game), spawn_report);
        app.add_systems(OnExit(AppState::Game), cleanup_report);
        app.add_systems(
            Update,
            (report_ui_system, ui_button_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_report(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_report(
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

    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 1633) {
        let e = spawn_ui_sprite(&mut commands, h, 330.0, 260.0, 6.0, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::Report),
            ReportWidget,
            Visibility::Hidden,
        ));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 360, 361, 362,
        330.0 + 290.0, 260.0 + 3.0, 7.0, 20.0, 20.0,
    ) {
        commands.entity(e).insert((
            ReportClose,
            DialogRoot(DialogKind::Report),
            ReportWidget,
        ));
    }
    for i in 0..8usize {
        let e = spawn_ui_text(
            &mut commands, &font, "",
            330.0 + 8.0, 260.0 + 60.0 + i as f32 * 20.0,
            12.0, Color::WHITE, 8.0,
        );
        commands.entity(e).insert((
            ReportLine(i),
            DialogRoot(DialogKind::Report),
            ReportWidget,
        ));
    }
}

fn report_ui_system(
    mut mgr: ResMut<DialogManager>,
    state: Res<ReportState>,
    close: Query<&UiButton, With<ReportClose>>,
    mut widgets: Query<&mut Visibility, With<ReportWidget>>,
    mut lines: Query<(&mut Text2d, &ReportLine)>,
) {
    let open = mgr.is_open(DialogKind::Report);
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        return;
    }
    for btn in &close {
        if btn.clicked {
            mgr.close(DialogKind::Report);
        }
    }
    for (mut text, line) in &mut lines {
        text.0 = state.lines.get(line.0).cloned().unwrap_or_default();
    }
}

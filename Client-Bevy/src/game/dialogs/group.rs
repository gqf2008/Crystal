// ============================================================================
// 组队对话框（M9 第 2 批）
// 布局参考：macroquad group_dialog.rs
//   - 背景 Prguse[964]，位置 (250,100)，标题 Title[16] (18,9)
//   - 成员列表 y=40 每 20px；按钮 y=210
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiFont, UiImageCache,
};

/// 组队状态（网络 GroupMembersMap 等写入）
#[derive(Resource, Default)]
pub struct GroupState {
    pub members: Vec<String>,
}

#[derive(Component)]
pub struct GroupWidget;

#[derive(Component)]
pub struct GroupClose;

#[derive(Component)]
pub struct GroupMemberLine(usize);

pub struct GroupPlugin;

impl Plugin for GroupPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GroupState>();
        app.add_systems(OnEnter(AppState::Game), spawn_group);
        app.add_systems(OnExit(AppState::Game), cleanup_group);
        app.add_systems(
            Update,
            (group_ui_system, ui_button_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_group(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_group(
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

    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 964) {
        let e = spawn_ui_sprite(&mut commands, h, 250.0, 100.0, 6.0, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::Group),
            GroupWidget,
            Visibility::Hidden,
        ));
    }
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Title, 16) {
        let e = spawn_ui_sprite(&mut commands, h, 268.0, 109.0, 6.2, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::Group),
            GroupWidget,
            Visibility::Hidden,
        ));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 360, 361, 362,
        250.0 + 290.0, 103.0, 7.0, 20.0, 20.0,
    ) {
        commands.entity(e).insert((
            GroupClose,
            DialogRoot(DialogKind::Group),
            GroupWidget,
        ));
    }
    for i in 0..8usize {
        let e = spawn_ui_text(
            &mut commands, &font, "",
            258.0, 140.0 + i as f32 * 20.0,
            12.0, Color::WHITE, 8.0,
        );
        commands.entity(e).insert((
            GroupMemberLine(i),
            DialogRoot(DialogKind::Group),
            GroupWidget,
        ));
    }
}

fn group_ui_system(
    mut mgr: ResMut<DialogManager>,
    group: Res<GroupState>,
    close: Query<&UiButton, With<GroupClose>>,
    mut widgets: Query<&mut Visibility, With<GroupWidget>>,
    mut lines: Query<(&mut Text2d, &GroupMemberLine)>,
) {
    let open = mgr.is_open(DialogKind::Group);
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        return;
    }
    for btn in &close {
        if btn.clicked {
            mgr.close(DialogKind::Group);
        }
    }
    for (mut text, line) in &mut lines {
        text.0 = group.members.get(line.0).cloned().unwrap_or_default();
    }
}

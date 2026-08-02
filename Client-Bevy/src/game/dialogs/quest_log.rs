// ============================================================================
// 任务日志对话框（M9 第 2 批）
// 布局参考：macroquad quest_log_dialog.rs / C# QuestDialogs.cs
//   - 背景 Prguse[961]，位置 (200,60)
//   - 标题 Title[15] (18,9)；关闭 Prguse2[360-362] (289,3)
//   - 任务列表 10 行（QuestLogState，网络 QuestAccepted 等写入）
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiFont, UiImageCache,
};

/// 任务摘要
#[derive(Debug, Clone, Default)]
pub struct QuestSummary {
    pub index: i32,
    pub title: String,
    pub status: String,
}

/// 任务日志状态
#[derive(Resource, Default)]
pub struct QuestLogState {
    pub quests: Vec<QuestSummary>,
}

#[derive(Component)]
pub struct QuestLogWidget;

#[derive(Component)]
pub struct QuestLogClose;

#[derive(Component)]
pub struct QuestLine(usize);

pub struct QuestLogPlugin;

impl Plugin for QuestLogPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<QuestLogState>();
        app.add_systems(OnEnter(AppState::Game), spawn_quest_log);
        app.add_systems(OnExit(AppState::Game), cleanup_quest_log);
        app.add_systems(
            Update,
            (quest_log_ui_system, ui_button_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_quest_log(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_quest_log(
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

    // 背景 Prguse[961]
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 961) {
        let e = spawn_ui_sprite(&mut commands, h, 200.0, 60.0, 6.0, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::QuestLog),
            QuestLogWidget,
            Visibility::Hidden,
        ));
    }

    // 标题 Title[15]
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Title, 15) {
        let e = spawn_ui_sprite(&mut commands, h, 218.0, 69.0, 6.2, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::QuestLog),
            QuestLogWidget,
            Visibility::Hidden,
        ));
    }

    // 关闭按钮
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 360, 361, 362,
        489.0, 63.0, 7.0, 20.0, 20.0,
    ) {
        commands.entity(e).insert((
            QuestLogClose,
            DialogRoot(DialogKind::QuestLog),
            QuestLogWidget,
        ));
    }

    // 任务列表 10 行
    for i in 0..10usize {
        let e = spawn_ui_text(
            &mut commands, &font, "",
            210.0, 100.0 + i as f32 * 20.0,
            12.0, Color::WHITE, 8.0,
        );
        commands.entity(e).insert((
            QuestLine(i),
            DialogRoot(DialogKind::QuestLog),
            QuestLogWidget,
        ));
    }
}

fn quest_log_ui_system(
    mut mgr: ResMut<DialogManager>,
    quests: Res<QuestLogState>,
    close: Query<&UiButton, With<QuestLogClose>>,
    mut widgets: Query<&mut Visibility, With<QuestLogWidget>>,
    mut lines: Query<(&mut Text2d, &QuestLine)>,
) {
    let open = mgr.is_open(DialogKind::QuestLog);
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        return;
    }
    for btn in &close {
        if btn.clicked {
            mgr.close(DialogKind::QuestLog);
        }
    }
    for (mut text, line) in &mut lines {
        if let Some(q) = quests.quests.get(line.0) {
            text.0 = format!("{} ({})", q.title, q.status);
        } else {
            text.0 = String::new();
        }
    }
}

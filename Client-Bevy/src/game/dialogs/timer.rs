// ============================================================================
// 计时器对话框（M50）
// 纯客户端对话框（无网络依赖）
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiFont, UiImageCache,
};

/// 状态
#[derive(Resource, Default)]
pub struct TimerState {
    pub message: String,
}

#[derive(Component)]
pub struct TimerWidget;

#[derive(Component)]
pub struct TimerClose;

#[derive(Component)]
pub struct TimerLine(usize);

pub struct TimerPlugin;

impl Plugin for TimerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TimerState>();
        app.add_systems(OnEnter(AppState::Game), spawn_timer);
        app.add_systems(OnExit(AppState::Game), cleanup_timer);
        app.add_systems(
            Update,
            (timer_ui_system, ui_button_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_timer(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_timer(
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

    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 170) {
        let e = spawn_ui_sprite(&mut commands, h, 280.0, 80.0, 6.0, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::Timer),
            TimerWidget,
            Visibility::Hidden,
        ));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 360, 361, 362,
        280.0 + 300.0, 83.0, 7.0, 20.0, 20.0,
    ) {
        commands.entity(e).insert((
            TimerClose,
            DialogRoot(DialogKind::Timer),
            TimerWidget,
        ));
    }
    for i in 0..10usize {
        let e = spawn_ui_text(
            &mut commands, &font, "",
            298.0, 120.0 + i as f32 * 22.0,
            12.0, Color::WHITE, 8.0,
        );
        commands.entity(e).insert((
            TimerLine(i),
            DialogRoot(DialogKind::Timer),
            TimerWidget,
        ));
    }
}

/// 显隐 + 渲染 + 关闭
fn timer_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut timer: ResMut<TimerState>,
    close: Query<&UiButton, With<TimerClose>>,
    mut widgets: Query<&mut Visibility, With<TimerWidget>>,
    mut lines: Query<(&mut Text2d, &TimerLine)>,
) {
    let open = mgr.is_open(DialogKind::Timer);
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        return;
    }
    for btn in &close {
        if btn.clicked {
            mgr.close(DialogKind::Timer);
        }
    }
    const TIMER_LINES: [&str; 2] = [
        "—— 计时器 ——",
        "本机运行时间显示（占位）",
    ];
    for (mut text, line) in &mut lines {
        text.0 = match line.0 {
            i if i < 2 => TIMER_LINES[i].to_string(),
            i if i == 4 => timer.message.clone(),
            _ => String::new(),
        };
    }
    timer.message = format!("{} 对话框", "计时器");
}

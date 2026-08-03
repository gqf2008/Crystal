// ============================================================================
// 公告对话框（M50）
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
pub struct NoticeState {
    pub message: String,
}

#[derive(Component)]
pub struct NoticeWidget;

#[derive(Component)]
pub struct NoticeClose;

#[derive(Component)]
pub struct NoticeLine(usize);

pub struct NoticePlugin;

impl Plugin for NoticePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NoticeState>();
        app.add_systems(OnEnter(AppState::Game), spawn_notice);
        app.add_systems(OnExit(AppState::Game), cleanup_notice);
        app.add_systems(
            Update,
            (notice_ui_system, ui_button_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_notice(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_notice(
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
            DialogRoot(DialogKind::Notice),
            NoticeWidget,
            Visibility::Hidden,
        ));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 360, 361, 362,
        280.0 + 300.0, 83.0, 7.0, 20.0, 20.0,
    ) {
        commands.entity(e).insert((
            NoticeClose,
            DialogRoot(DialogKind::Notice),
            NoticeWidget,
        ));
    }
    for i in 0..10usize {
        let e = spawn_ui_text(
            &mut commands, &font, "",
            298.0, 120.0 + i as f32 * 22.0,
            12.0, Color::WHITE, 8.0,
        );
        commands.entity(e).insert((
            NoticeLine(i),
            DialogRoot(DialogKind::Notice),
            NoticeWidget,
        ));
    }
}

/// 显隐 + 渲染 + 关闭
fn notice_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut notice: ResMut<NoticeState>,
    close: Query<&UiButton, With<NoticeClose>>,
    mut widgets: Query<&mut Visibility, With<NoticeWidget>>,
    mut lines: Query<(&mut Text2d, &NoticeLine)>,
) {
    let open = mgr.is_open(DialogKind::Notice);
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        return;
    }
    for btn in &close {
        if btn.clicked {
            mgr.close(DialogKind::Notice);
        }
    }
    const NOTICE_LINES: [&str; 4] = [
        "—— 服务器公告 ——",
        "欢迎来到传奇2 Bevy 移植版",
        "本客户端为独立 Bevy worktree",
        "全部对话框已按原版 C# 对齐",
    ];
    for (mut text, line) in &mut lines {
        text.0 = match line.0 {
            i if i < 4 => NOTICE_LINES[i].to_string(),
            i if i == 4 => notice.message.clone(),
            _ => String::new(),
        };
    }
    notice.message = format!("{} 对话框", "公告");
}

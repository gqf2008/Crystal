// ============================================================================
// 邮件对话框（M9 第 3 批）
// 布局参考：macroquad mail_dialog.rs
//   - 背景 Prguse[956]，标题 Title[20]，位置 (280,80)
//   - 邮件列表 y=60 起每 22px
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiFont, UiImageCache,
};

/// 邮件条目
#[derive(Debug, Clone, Default)]
pub struct MailEntry {
    pub sender: String,
    pub subject: String,
    pub unread: bool,
}

/// 邮件状态（网络 ReceiveMail 等写入）
#[derive(Resource, Default)]
pub struct MailState {
    pub mails: Vec<MailEntry>,
}

#[derive(Component)]
pub struct MailWidget;

#[derive(Component)]
pub struct MailClose;

#[derive(Component)]
pub struct MailLine(usize);

pub struct MailPlugin;

impl Plugin for MailPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MailState>();
        app.add_systems(OnEnter(AppState::Game), spawn_mail);
        app.add_systems(OnExit(AppState::Game), cleanup_mail);
        app.add_systems(
            Update,
            (mail_ui_system, ui_button_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_mail(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_mail(
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

    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 956) {
        let e = spawn_ui_sprite(&mut commands, h, 280.0, 80.0, 6.0, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::Mail),
            MailWidget,
            Visibility::Hidden,
        ));
    }
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Title, 20) {
        let e = spawn_ui_sprite(&mut commands, h, 298.0, 89.0, 6.2, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::Mail),
            MailWidget,
            Visibility::Hidden,
        ));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 360, 361, 362,
        280.0 + 290.0, 83.0, 7.0, 20.0, 20.0,
    ) {
        commands.entity(e).insert((
            MailClose,
            DialogRoot(DialogKind::Mail),
            MailWidget,
        ));
    }
    for i in 0..8usize {
        let e = spawn_ui_text(
            &mut commands, &font, "",
            288.0, 140.0 + i as f32 * 22.0,
            12.0, Color::WHITE, 8.0,
        );
        commands.entity(e).insert((
            MailLine(i),
            DialogRoot(DialogKind::Mail),
            MailWidget,
        ));
    }
}

fn mail_ui_system(
    mut mgr: ResMut<DialogManager>,
    mail: Res<MailState>,
    close: Query<&UiButton, With<MailClose>>,
    mut widgets: Query<&mut Visibility, With<MailWidget>>,
    mut lines: Query<(&mut Text2d, &MailLine)>,
) {
    let open = mgr.is_open(DialogKind::Mail);
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        return;
    }
    for btn in &close {
        if btn.clicked {
            mgr.close(DialogKind::Mail);
        }
    }
    for (mut text, line) in &mut lines {
        if let Some(m) = mail.mails.get(line.0) {
            let mark = if m.unread { "● " } else { "" };
            text.0 = format!("{}{}: {}", mark, m.sender, m.subject);
        } else {
            text.0 = String::new();
        }
    }
}

// ============================================================================
// 邮件对话框（M22）
// 布局参考：macroquad mail_dialog.rs / C# MailDialog
//   - 背景 Prguse[956]，标题 Title[20]，位置 (280,80)
//   - 邮件列表 y=60 起每 22px；点击列表项 → C.ReadMail{mail_id} → 内容区显示正文/金币
// 网络：ReceiveMail（新邮件条目 / 邮件全文，服务端同 opcode 双格式）→ 列表 + 详情
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::network::NetworkContext;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiFont, UiImageCache,
};

/// 邮件列表条目
#[derive(Debug, Clone, Default)]
pub struct MailEntry {
    pub mail_id: u64,
    pub sender: String,
    pub subject: String,
    pub unread: bool,
    pub gold: u32,
}

/// 邮件详情（ReadMail 响应全文）
#[derive(Debug, Clone, Default)]
pub struct MailDetail {
    pub mail_id: u64,
    pub sender: String,
    pub subject: String,
    pub body: String,
    pub gold: u32,
    /// 附件名列表
    pub items: Vec<String>,
}

/// 邮件状态（网络 ReceiveMail 写入）
#[derive(Resource, Default)]
pub struct MailState {
    pub mails: Vec<MailEntry>,
    pub detail: Option<MailDetail>,
}

#[derive(Component)]
pub struct MailWidget;

#[derive(Component)]
pub struct MailClose;

#[derive(Component)]
pub struct MailLine(usize);

#[derive(Component)]
pub struct MailDetailText;

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
        let e = spawn_ui_sprite(&mut commands, h, 298.0, 88.0, 6.2, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::Mail),
            MailWidget,
            Visibility::Hidden,
        ));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 360, 361, 362,
        280.0 + 340.0, 83.0, 7.0, 20.0, 20.0,
    ) {
        commands.entity(e).insert((
            MailClose,
            DialogRoot(DialogKind::Mail),
            MailWidget,
        ));
    }
    // 邮件列表（8 行）
    for i in 0..8usize {
        let e = spawn_ui_text(
            &mut commands, &font, "",
            298.0, 140.0 + i as f32 * 22.0,
            12.0, Color::WHITE, 8.0,
        );
        commands.entity(e).insert((
            MailLine(i),
            DialogRoot(DialogKind::Mail),
            MailWidget,
        ));
    }
    // 内容区（正文/金币）
    let detail = spawn_ui_text(
        &mut commands, &font, "",
        298.0, 340.0, 12.0, Color::srgb(0.95, 0.95, 0.8), 8.0,
    );
    commands.entity(detail).insert((
        MailDetailText,
        DialogRoot(DialogKind::Mail),
        MailWidget,
    ));
}

/// 显示/隐藏 + 列表渲染 + 内容区 + 点击读取
#[allow(clippy::too_many_arguments)]
fn mail_ui_system(
    mut mgr: ResMut<DialogManager>,
    mail: Res<MailState>,
    net: Res<NetworkContext>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    close: Query<&UiButton, With<MailClose>>,
    mut widgets: Query<
        (&mut Visibility, Option<&MailLine>, Option<&MailDetailText>),
        With<MailWidget>,
    >,
    mut lines: Query<(&mut Text2d, &MailLine), Without<MailDetailText>>,
    mut detail_texts: Query<(&mut Text2d, &MailDetailText), Without<MailLine>>,
) {
    let open = mgr.is_open(DialogKind::Mail);
    for (mut vis, line, _det) in &mut widgets {
        if line.is_some() || _det.is_some() {
            continue;
        }
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
    // 列表
    for (mut text, line) in &mut lines {
        text.0 = match mail.mails.get(line.0) {
            Some(m) => {
                let mark = if m.unread { "（未读）" } else { "" };
                format!("{} - {}{}", m.sender, m.subject, mark)
            }
            None => String::new(),
        };
    }
    // 内容区
    for (mut text, _) in &mut detail_texts {
        text.0 = match mail.detail.as_ref() {
            Some(d) => {
                let mut s = format!("发件人: {}\n主题: {}\n\n{}", d.sender, d.subject, d.body);
                if d.gold > 0 {
                    s.push_str(&format!("\n金币: {}", d.gold));
                }
                if !d.items.is_empty() {
                    s.push_str(&format!("\n附件: {}", d.items.join(", ")));
                }
                s
            }
            None => "点击上方邮件查看内容".to_string(),
        };
    }
    // 点击列表项 → ReadMail
    if mouse.just_pressed(MouseButton::Left) {
        let Ok(window) = windows.single() else { return };
        let Some(cursor) = window.cursor_position() else { return };
        for i in 0..8usize {
            let y = 140.0 + i as f32 * 22.0;
            if cursor.x >= 298.0 && cursor.x <= 600.0 && cursor.y >= y && cursor.y <= y + 20.0 {
                if let Some(m) = mail.mails.get(i) {
                    net.send_packet(&mir2_shared::packets::client::mail::ReadMail {
                        mail_id: m.mail_id,
                    });
                    tracing::info!("📧 读取邮件: {} ({})", m.subject, m.mail_id);
                }
                break;
            }
        }
    }
}
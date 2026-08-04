// ============================================================================
// 邮件对话框（M22）
// 布局参考：macroquad mail_dialog.rs / C# MailDialog
//   - 背景 Prguse[956]，标题 Title[20]，位置 (280,80)
//   - 邮件列表 y=60 起每 22px；点击列表项 → C.ReadMail{mail_id} → 内容区显示正文/金币
// 网络：ReceiveMail（新邮件条目 / 邮件全文，服务端同 opcode 双格式）→ 列表 + 详情
// ============================================================================

use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::game::dialogs::text_input::{TextInputDisplay, TextInputField, TextInputRect};
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiEntity, UiFont,
    UiImageCache,
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
    /// 写邮件界面是否打开（输入框用通用 TextInputState id 0=收件人 1=主题 2=正文）
    pub compose: bool,
}

#[derive(Component)]
pub struct MailWidget;

#[derive(Component)]
pub struct MailClose;

#[derive(Component)]
pub struct MailLine(usize);

#[derive(Component)]
pub struct MailDetailText;

// 写邮件界面
#[derive(Component)]
pub struct MailWrite;

#[derive(Component)]
pub struct MailComposeWidget;

#[derive(Component)]
pub struct MailSendBtn;

#[derive(Component)]
pub struct MailCancelBtn;

pub struct MailPlugin;

impl Plugin for MailPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MailState>();
                app.add_systems(
            Update,
            mail_server_events.run_if(in_state(AppState::Game)),
        );
app.add_systems(OnEnter(AppState::Game), spawn_mail);
        app.add_systems(OnExit(AppState::Game), cleanup_mail);
        app.add_systems(
            Update,
            (mail_ui_system, mail_compose_system, ui_button_system)
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

/// 写邮件界面：写按钮 → 打开；发送/取消
fn mail_compose_system(
    mut mail: ResMut<MailState>,
    net: Res<NetConnection>,
    mut input: ResMut<crate::game::dialogs::text_input::TextInputState>,
    write_btn: Query<&UiButton, With<MailWrite>>,
    send_btn: Query<&UiButton, With<MailSendBtn>>,
    cancel_btn: Query<&UiButton, With<MailCancelBtn>>,
    mut compose_widgets: Query<
        &mut Visibility,
        (With<MailComposeWidget>, Without<MailWidget>),
    >,
) {
    let open = mail.compose;
    for mut vis in &mut compose_widgets {
        *vis = if open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for btn in &write_btn {
        if btn.clicked {
            mail.compose = true;
            mail.detail = None;
            if input.texts.len() < 3 {
                input.texts.resize(3, String::new());
            }
            tracing::info!("✉️ 打开写邮件");
        }
    }
    for btn in &send_btn {
        if btn.clicked && mail.compose {
            send_composed_mail(&net, &input);
            mail.compose = false;
            input.active = None;
        }
    }
    for btn in &cancel_btn {
        if btn.clicked && mail.compose {
            mail.compose = false;
            input.active = None;
        }
    }
}

/// 发送写好的邮件（原版 C# MailDialog 发送 → C.SendMail{Name, Message}；subject 由正文首行派生）
pub fn send_composed_mail(net: &NetConnection, input: &crate::game::dialogs::text_input::TextInputState) {
    let to = input.texts.get(0).cloned().unwrap_or_default();
    let subject = input.texts.get(1).cloned().unwrap_or_default();
    let body = input.texts.get(2).cloned().unwrap_or_default();
    if to.is_empty() {
        tracing::warn!("✉️ 收件人为空");
        return;
    }
    let message = if body.is_empty() {
        subject.clone()
    } else {
        format!("{}\n{}", subject, body)
    };
    net.send_packet(&mir2_shared::packets::client::mail::SendMail {
        name: to.clone(),
        message,
        gold: 0,
        items_idx: [0; 5],
        stamped: false,
    });
    tracing::info!("✉️ 发送邮件: {} - {}", to, subject);
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
    // 写邮件按钮
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 206, 207, 208,
        280.0 + 300.0, 280.0, 7.0, 60.0, 23.0,
    ) {
        commands.entity(e).insert((
            MailWrite,
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

    // ---- 写邮件界面（原版 C# MirInputBox 语义）----
    let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    let compose_bg = commands
        .spawn((
            UiEntity,
            DialogRoot(DialogKind::Mail),
            MailComposeWidget,
            Sprite {
                image: white.clone(),
                color: Color::srgba(0.0, 0.0, 0.0, 0.6),
                custom_size: Some(Vec2::new(360.0, 220.0)),
                ..default()
            },
            Anchor::TOP_LEFT,
            Transform::from_xyz(290.0, -90.0, 8.0),
            Visibility::Hidden,
        ))
        .id();
    // 标签 + 输入框（收件人/主题/正文）
    let fields: [(usize, &str, f32); 3] = [
        (0, "收件人:", 100.0),
        (1, "主题:", 140.0),
        (2, "正文:", 180.0),
    ];
    for (id, label, y) in fields {
        let _ = spawn_ui_text(
            &mut commands, &font, label,
            300.0, y, 12.0, Color::WHITE, 8.1,
        );
        let box_e = commands
            .spawn((
                UiEntity,
                DialogRoot(DialogKind::Mail),
                MailComposeWidget,
                TextInputField(id),
                TextInputRect(360.0, y, 270.0, 20.0),
                Sprite {
                    image: white.clone(),
                    color: Color::srgba(0.2, 0.2, 0.25, 0.9),
                    custom_size: Some(Vec2::new(270.0, 20.0)),
                    ..default()
                },
                Anchor::TOP_LEFT,
                Transform::from_xyz(360.0, -y, 8.1),
                Visibility::Hidden,
            ))
            .id();
        commands.entity(box_e).with_children(|p| {
            p.spawn((
                TextInputDisplay(id),
                Text2d::new(String::new()),
                Anchor::TOP_LEFT,
                TextFont {
                    font: FontSource::Handle(font.clone()),
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 1.0, 1.0)),
                Transform::from_xyz(4.0, -2.0, 8.2),
            ));
        });
    }
    let _ = compose_bg;
    // 发送 / 取消
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 206, 207, 208,
        300.0, 250.0, 8.3, 76.0, 25.0,
    ) {
        commands.entity(e).insert((
            MailSendBtn,
            DialogRoot(DialogKind::Mail),
            MailComposeWidget,
        ));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 210, 211, 212,
        390.0, 250.0, 8.3, 76.0, 25.0,
    ) {
        commands.entity(e).insert((
            MailCancelBtn,
            DialogRoot(DialogKind::Mail),
            MailComposeWidget,
        ));
    }
}

/// 显示/隐藏 + 列表渲染 + 内容区 + 点击读取
#[allow(clippy::too_many_arguments)]
fn mail_ui_system(
    mut mgr: ResMut<DialogManager>,
    mail: Res<MailState>,
    net: Res<NetConnection>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    close: Query<&UiButton, With<MailClose>>,
    mut widgets: Query<
        (&mut Visibility, Option<&MailLine>, Option<&MailDetailText>),
        (With<MailWidget>, Without<MailComposeWidget>),
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


/// 消费服务端邮件事件（网络层只广播 ServerEvent）
fn mail_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut mail: ResMut<MailState>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        if let ServerEvent::MailReceived { entry, detail } = ev {
            // 去重：同 mail_id 已存在则替换（全文包会更新未读标记）
            if let Some(existing) = mail.mails.iter_mut().find(|m| m.mail_id == entry.mail_id) {
                *existing = entry.clone();
            } else {
                mail.mails.insert(0, entry.clone());
            }
            if let Some(d) = detail {
                mail.detail = Some(d.clone());
            }
        }
    }
}

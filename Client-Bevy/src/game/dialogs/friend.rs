// ============================================================================
// 好友对话框（M25）
// 布局参考：C# FriendDialog.cs / macroquad friend_dialog.rs
//   - 背景 Title[199]，位置 (300,100)，标题 Title[6] (18,9)
//   - 好友列表 y=40 每 20px；打开时自动请求 C.RefreshFriends
// 网络：FriendUpdate（列表 / 单个添加，同 opcode 双格式）→ 列表渲染（在线/离线）
// ============================================================================

use bevy::prelude::*;

use crate::game::chat::{ChatChannel, ChatState};
use crate::game::dialogs::mail::MailState;
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiFont, UiImageCache,
};
use crate::ui::scroll_list::{spawn_scroll_bar, ScrollList};

/// 好友条目
#[derive(Debug, Clone, Default)]
pub struct FriendEntry {
    pub object_id: u32,
    pub name: String,
    pub memo: String,
    pub online: bool,
}

/// 待处理输入动作（添加/备注）
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FriendPending {
    Add,
    Memo(usize),
}

/// 好友状态（网络 FriendUpdate 写入）
#[derive(Resource, Default)]
pub struct FriendState {
    pub friends: Vec<FriendEntry>,
    /// 选中的好友行（删除/备注用）
    pub selected: Option<usize>,
    /// 待处理的内嵌输入动作
    pub pending: Option<FriendPending>,
}

#[derive(Component)]
pub struct FriendWidget;

#[derive(Component)]
pub struct FriendClose;

#[derive(Component)]
pub struct FriendAdd;

#[derive(Component)]
pub struct FriendRemove;

#[derive(Component)]
pub struct FriendMemo;

/// 私聊选中好友（C# FriendDialog WhisperButton）
#[derive(Component)]
pub struct FriendWhisper;

/// 发邮件给选中好友（C# FriendDialog EmailButton）
#[derive(Component)]
pub struct FriendEmail;

/// 内嵌输入框（添加/备注）
#[derive(Component)]
pub struct FriendInputBox;

#[derive(Component)]
pub struct FriendLine(usize);

pub struct FriendPlugin;

impl Plugin for FriendPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FriendState>();
                app.add_systems(
            Update,
            friend_server_events.run_if(in_state(AppState::Game)),
        );
app.add_systems(OnEnter(AppState::Game), spawn_friend);
        app.add_systems(OnExit(AppState::Game), cleanup_friend);
        app.add_systems(
            Update,
            (friend_ui_system, ui_button_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_friend(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_friend(
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

    // 背景 Title[199]
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Title, 199) {
        let e = spawn_ui_sprite(&mut commands, h, 300.0, 100.0, 6.0, 1.0);
        // #89 可滚动列表：10 行 × 20px，滚动条在列表右侧
        let (track, thumb) = spawn_scroll_bar(&mut commands, &mut images, (508.0, 140.0, 4.0, 200.0), 6.3);
        commands.entity(track).insert((DialogRoot(DialogKind::Friend), FriendWidget, Visibility::Visible));
        commands.entity(thumb).insert((
            DialogRoot(DialogKind::Friend),
            FriendWidget,
            Visibility::Visible,
        ));
        commands.entity(e).insert((
            DialogRoot(DialogKind::Friend),
            FriendWidget,
            Visibility::Hidden,
            ScrollList {
                rect_rel: (18.0, 40.0, 190.0, 200.0),
                row_h: 20.0,
                visible: 10,
                total: 0,
                offset: 0,
                step: 3,
                track_rel: (208.0, 40.0, 4.0, 200.0),
                thumb: Some(thumb),
                z: 8.0,
            },
        ));
    }
    // 标题 Title[6]
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Title, 6) {
        let e = spawn_ui_sprite(&mut commands, h, 318.0, 109.0, 6.2, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::Friend),
            FriendWidget,
            Visibility::Hidden,
        ));
    }
    // 关闭
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 360, 361, 362,
        300.0 + 206.0, 103.0, 7.0, 20.0, 20.0,
    ) {
        commands.entity(e).insert((
            FriendClose,
            DialogRoot(DialogKind::Friend),
            FriendWidget,
        ));
    }
    // 添加/删除/备注按钮（C# FriendDialog Add/Remove/Memo (60,241)/(88,241)/(116,241)）
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse, 554, 555, 556,
        300.0 + 60.0, 100.0 + 241.0, 7.2, 24.0, 22.0,
    ) {
        commands.entity(e).insert((FriendAdd, DialogRoot(DialogKind::Friend), FriendWidget));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse, 557, 558, 559,
        300.0 + 88.0, 100.0 + 241.0, 7.2, 24.0, 22.0,
    ) {
        commands.entity(e).insert((FriendRemove, DialogRoot(DialogKind::Friend), FriendWidget));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse, 560, 561, 562,
        300.0 + 116.0, 100.0 + 241.0, 7.2, 24.0, 22.0,
    ) {
        commands.entity(e).insert((FriendMemo, DialogRoot(DialogKind::Friend), FriendWidget));
    }
    // 邮件按钮（C# EmailButton Prguse 563-565 @(144,241)）
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse, 563, 564, 565,
        300.0 + 144.0, 100.0 + 241.0, 7.2, 24.0, 22.0,
    ) {
        commands.entity(e).insert((FriendEmail, DialogRoot(DialogKind::Friend), FriendWidget));
    }
    // 私聊按钮（C# WhisperButton Prguse 566-568 @(172,241)）
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse, 566, 567, 568,
        300.0 + 172.0, 100.0 + 241.0, 7.2, 24.0, 22.0,
    ) {
        commands.entity(e).insert((FriendWhisper, DialogRoot(DialogKind::Friend), FriendWidget));
    }
    // 内嵌输入框（添加/备注，TextInput id 30，#130）
    let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    let box_e = commands
        .spawn((
            crate::ui::sprite_ui::UiEntity,
            DialogRoot(DialogKind::Friend),
            FriendWidget,
            FriendInputBox,
            crate::game::dialogs::text_input::TextInputField(30),
            crate::game::dialogs::text_input::TextInputRect(318.0, 345.0, 180.0, 20.0),
            Sprite {
                image: white.clone(),
                color: Color::srgba(0.2, 0.2, 0.25, 0.9),
                custom_size: Some(Vec2::new(180.0, 20.0)),
                ..default()
            },
            bevy::sprite::Anchor::TOP_LEFT,
            Transform::from_xyz(318.0, -345.0, 8.1),
            Visibility::Hidden,
        ))
        .id();
    commands.entity(box_e).with_children(|p| {
        p.spawn((
            crate::game::dialogs::text_input::TextInputDisplay(30),
            Text2d::new(String::new()),
            bevy::sprite::Anchor::TOP_LEFT,
            TextFont {
                font: FontSource::Handle(font.clone()),
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(Color::WHITE),
            Transform::from_xyz(4.0, -2.0, 8.2),
        ));
    });

    // 好友列表（10 行）
    for i in 0..10usize {
        let e = spawn_ui_text(
            &mut commands, &font, "",
            318.0, 140.0 + i as f32 * 20.0,
            12.0, Color::WHITE, 8.0,
        );
        commands.entity(e).insert((
            FriendLine(i),
            DialogRoot(DialogKind::Friend),
            FriendWidget,
        ));
    }
}

/// 显隐 + 列表渲染 + 打开时自动请求刷新（原版 C# FriendDialog.Show → RefreshFriends）
#[allow(clippy::too_many_arguments)]
fn friend_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut friend: ResMut<FriendState>,
    mut mail: ResMut<MailState>,
    mut chat: ResMut<ChatState>,
    net: Res<NetConnection>,
    close: Query<&UiButton, With<FriendClose>>,
    btns: Query<(
        &UiButton,
        Has<FriendAdd>,
        Has<FriendRemove>,
        Has<FriendMemo>,
        Has<FriendEmail>,
        Has<FriendWhisper>,
    )>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    mut input: ResMut<crate::game::dialogs::text_input::TextInputState>,
    mut submits: MessageReader<crate::game::dialogs::text_input::TextInputSubmit>,
    mut input_box: Query<&mut Visibility, With<FriendInputBox>>,
    mut widgets: Query<(&mut Visibility, Option<&FriendLine>), (With<FriendWidget>, Without<FriendInputBox>)>,
    mut lines: Query<(&mut Text2d, &mut TextColor, &FriendLine)>,
    mut scroll: Query<&mut ScrollList, With<FriendWidget>>,
    mut requested: Local<bool>,
) {
    let open = mgr.is_open(DialogKind::Friend);
    for (mut vis, _line) in &mut widgets {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    // 内嵌输入框只在有待处理动作时显示
    let show_input = open && friend.pending.is_some();
    for mut vis in &mut input_box {
        *vis = if show_input { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        *requested = false;
        friend.pending = None;
        friend.selected = None;
        input.active = None;
        return;
    }
    // 打开瞬间请求一次好友列表
    if !*requested {
        *requested = true;
        net.send_packet(&mir2_shared::packets::client::friend::RefreshFriends);
        tracing::info!("👥 请求刷新好友列表");
    }
    for btn in &close {
        if btn.clicked {
            mgr.close(DialogKind::Friend);
        }
    }
    // 列表（在线标记，原版 C# 语义）；#89 支持滚轮滚动；#130 选中行高亮
    let mut sl = scroll.single_mut();
    if let Ok(sl) = sl.as_mut() {
        sl.set_total(friend.friends.len());
        let off = sl.offset;
        for (mut text, mut color, line) in &mut lines {
            let idx = off + line.0;
            let selected = friend.selected == Some(idx);
            text.0 = match friend.friends.get(idx) {
                Some(f) => {
                    let mark = if f.online { "（在线）" } else { "（离线）" };
                    let name = if f.memo.is_empty() {
                        f.name.clone()
                    } else {
                        format!("{} ({})", f.name, f.memo)
                    };
                    format!("{}{}", name, mark)
                }
                None => String::new(),
            };
            let c = if selected {
                Color::srgb(1.0, 0.9, 0.3)
            } else {
                Color::WHITE
            };
            if color.0 != c {
                color.0 = c;
            }
        }
    }

    // 点击行选中（#130）
    if mouse.just_pressed(MouseButton::Left) {
        if let Ok(window) = windows.single() {
            if let Some(cursor) = window.cursor_position() {
                let off = scroll.single().map(|s| s.offset).unwrap_or(0);
                for i in 0..10usize {
                    let y = 140.0 + i as f32 * 20.0;
                    if cursor.x >= 318.0 && cursor.x <= 500.0 && cursor.y >= y && cursor.y <= y + 18.0 {
                        let idx = off + i;
                        friend.selected = if friend.selected == Some(idx) { None } else { Some(idx) };
                        break;
                    }
                }
            }
        }
    }

    // 添加/删除/备注/邮件/私聊按钮（C# FriendDialog Add/Remove/Memo/Email/Whisper）
    for (btn, is_add, is_remove, is_memo, is_email, is_whisper) in &btns {
        if !btn.clicked {
            continue;
        }
        if is_add {
            friend.pending = Some(FriendPending::Add);
            input.texts[30].clear();
            input.active = Some(30);
            tracing::info!("👥 添加好友：请输入名字");
        } else if is_remove {
            if let Some(idx) = friend.selected {
                if let Some(f) = friend.friends.get(idx) {
                    net.send_packet(&mir2_shared::packets::client::friend::RemoveFriend {
                        character_index: f.object_id as i32,
                    });
                    tracing::info!("👥 删除好友: {}", f.name);
                    friend.selected = None;
                }
            }
        } else if is_memo {
            if let Some(idx) = friend.selected {
                friend.pending = Some(FriendPending::Memo(idx));
                input.texts[30].clear();
                input.active = Some(30);
                tracing::info!("👥 备注好友：请输入备注");
            }
        } else if is_email {
            // 邮件：ComposeMail(选中好友)（C# FriendDialog EmailButton）
            if let Some(f) = friend.selected.and_then(|i| friend.friends.get(i)).cloned() {
                mgr.open.push(DialogKind::Mail);
                mail.compose = true;
                mail.detail = None;
                mail.attach = vec![None; 5];
                mail.compose_gold = 0;
                if input.texts.len() < 4 {
                    input.texts.resize(4, String::new());
                }
                input.texts[0] = f.name.clone();
                input.active = None;
                tracing::info!("✉️ 给好友 {} 写邮件", f.name);
            }
        } else if is_whisper {
            // 私聊：在线预填 /w，离线系统提示（C# FriendDialog WhisperButton）
            if let Some(f) = friend.selected.and_then(|i| friend.friends.get(i)).cloned() {
                match friend_whisper_command(&f.name, f.online) {
                    Some(cmd) => {
                        chat.input_active = true;
                        chat.input_text = cmd;
                        tracing::info!("💬 私聊好友 {}", f.name);
                    }
                    None => {
                        chat.add_line("该玩家不在线".to_string(), Color::srgb(1.0, 0.3, 0.3), ChatChannel::System);
                        tracing::info!("💬 好友 {} 不在线", f.name);
                    }
                }
            }
        }
    }
    // 内嵌输入提交（Enter）
    for sub in submits.read() {
        if sub.0 != 30 {
            continue;
        }
        let name = input.texts.get(30).cloned().unwrap_or_default();
        let name = name.trim().to_string();
        if name.is_empty() {
            friend.pending = None;
            input.active = None;
            continue;
        }
        match friend.pending.take() {
            Some(FriendPending::Add) => {
                net.send_packet(&mir2_shared::packets::client::friend::AddFriend {
                    name: name.clone(),
                    blocked: false,
                });
                tracing::info!("👥 添加好友: {}", name);
            }
            Some(FriendPending::Memo(idx)) => {
                if let Some(f) = friend.friends.get(idx) {
                    net.send_packet(&mir2_shared::packets::client::friend::AddMemo {
                        character_index: f.object_id as i32,
                        memo: name.clone(),
                    });
                    tracing::info!("👥 备注好友 {}: {}", f.name, name);
                }
            }
            None => {}
        }
        input.texts[30].clear();
        input.active = None;
    }
}


/// 消费服务端好友事件（网络层只广播 ServerEvent）
fn friend_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut friend: ResMut<FriendState>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        if let ServerEvent::FriendUpdated { entries } = ev {
            for e in entries {
                if let Some(existing) = friend.friends.iter_mut().find(|f| f.object_id == e.object_id) {
                    *existing = e.clone();
                } else {
                    friend.friends.push(e.clone());
                }
            }
        }
    }
}
/// 好友私聊命令（C# WhisperButton：离线返回 None）
pub fn friend_whisper_command(name: &str, online: bool) -> Option<String> {
    if !online {
        return None;
    }
    Some(format!("/w {} ", name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whisper_command_online_offline() {
        assert_eq!(friend_whisper_command("Alice", true), Some("/w Alice ".to_string()));
        assert_eq!(friend_whisper_command("Alice", false), None);
    }
}


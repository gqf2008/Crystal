// ============================================================================
// 好友对话框（M25）
// 布局参考：C# FriendDialog.cs / macroquad friend_dialog.rs
//   - 背景 Title[199]，位置 (300,100)，标题 Title[6] (18,9)
//   - 好友列表 y=40 每 20px；打开时自动请求 C.RefreshFriends
// 网络：FriendUpdate（列表 / 单个添加，同 opcode 双格式）→ 列表渲染（在线/离线）
// ============================================================================

use bevy::prelude::*;

use crate::game::chat::{ChatChannel, ChatState};
use crate::game::dialogs::mail::ComposeMail;
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::UiFont;
use crate::ui::theme::{
    load_lib_image, spawn_container, spawn_icon_button, spawn_image, spawn_label, spawn_panel,
};

/// 好友条目
#[derive(Debug, Clone, Default)]
pub struct FriendEntry {
    pub object_id: u32,
    pub name: String,
    pub memo: String,
    /// 是否黑名单（C# ClientFriend.Blocked）
    pub blocked: bool,
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
    /// 当前页签（false=好友 true=黑名单，C# _blockedTab）
    pub blocked_tab: bool,
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

/// 好友页签（C# FriendLabel）
#[derive(Component)]
pub struct FriendTabFriend;

/// 黑名单页签（C# BlacklistLabel）
#[derive(Component)]
pub struct FriendTabBlock;

#[derive(Component)]
pub struct FriendLine(usize);

/// bevy_ui 行文本子节点（父 Button 挂 FriendLine，子文本挂 FriendLineText）
#[derive(Component)]
pub struct FriendLineText(usize);

/// friend_ui_system 的 Local 状态（合并以控制 Bevy 系统参数数 ≤16）
#[derive(Default)]
struct FriendLocal {
    prev_inter: std::collections::HashMap<Entity, Interaction>,
    requested: bool,
    offset: usize,
}

/// 好友动作按钮（添加/删除/备注/邮件/私聊；bevy_ui Interaction 驱动）
#[derive(Component, Clone, Copy)]
pub struct FriendAction {
    pub is_add: bool,
    pub is_remove: bool,
    pub is_memo: bool,
    pub is_email: bool,
    pub is_whisper: bool,
}

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
            friend_ui_system.run_if(in_state(AppState::Game)),
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
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<UiFont>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();
    let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));

    // 面板 Title[199]（264x272 @ 300,100）
    let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Title, 199) else {
        return;
    };
    let panel = spawn_panel(&mut commands, bg, 300.0, 100.0, 264.0, 272.0, 30);
    commands.entity(panel).insert((DialogRoot(DialogKind::Friend), FriendWidget));

    commands.entity(panel).with_children(|p| {
        // 标题 Title[6] @(18,9)
        if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Title, 6) {
            spawn_image(p, h, 18.0, 9.0, 57.0, 15.0, 9);
        }
        // 关闭 Prguse2[360/361/362] @(206,3)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 360),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 361),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 362),
        ) {
            spawn_icon_button(p, n, h, pr, 206.0, 3.0, 20.0, 20.0, 10).insert(FriendClose);
        }
        // 添加/删除/备注/邮件/私聊（Prguse 554-568 @(60/88/116/144/172, 241)）
        let acts: [(bool, bool, bool, bool, bool, usize, f32); 5] = [
            (true, false, false, false, false, 554, 60.0),
            (false, true, false, false, false, 557, 88.0),
            (false, false, true, false, false, 560, 116.0),
            (false, false, false, true, false, 563, 144.0),
            (false, false, false, false, true, 566, 172.0),
        ];
        for (is_add, is_remove, is_memo, is_email, is_whisper, idx, x) in acts {
            if let (Some(n), Some(h), Some(pr)) = (
                load_lib_image(&mut libs, &mut images, LibraryName::Prguse, idx),
                load_lib_image(&mut libs, &mut images, LibraryName::Prguse, idx + 1),
                load_lib_image(&mut libs, &mut images, LibraryName::Prguse, idx + 2),
            ) {
                spawn_icon_button(p, n, h, pr, x, 241.0, 24.0, 22.0, 10).insert((
                    FriendAction {
                        is_add,
                        is_remove,
                        is_memo,
                        is_email,
                        is_whisper,
                    },
                ));
            }
        }
        // 内嵌输入框（TextInput id 30）@(18,245) 180x20
        spawn_container(p, 18.0, 245.0, 180.0, 20.0, 10)
            .insert((
                FriendInputBox,
                BackgroundColor(Color::srgba(0.2, 0.2, 0.25, 0.9)),
                crate::game::dialogs::text_input::TextInputField(30),
                crate::game::dialogs::text_input::TextInputRect(318.0, 345.0, 180.0, 20.0),
            ))
            .with_children(|ic| {
                ic.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(4.0),
                        top: Val::Px(2.0),
                        ..default()
                    },
                    Text::new(String::new()),
                    TextFont {
                        font: FontSource::Handle(font.clone()),
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    ZIndex(11),
                    crate::game::dialogs::text_input::TextInputDisplay(30),
                ));
            });
        // 页签（好友/黑名单）@(18,18)/(70,18)
        spawn_label(p, &font, "好友", 18.0, 18.0, 12.0, Color::WHITE, 10)
            .insert((Button, FriendTabFriend));
        spawn_label(p, &font, "黑名单", 70.0, 18.0, 12.0, Color::WHITE, 10)
            .insert((Button, FriendTabBlock));
        // 好友列表（10 行，可点击 Button + 文本子节点）
        for i in 0..10usize {
            spawn_container(p, 18.0, 40.0 + i as f32 * 20.0, 190.0, 18.0, 9)
                .insert((Button, FriendLine(i)))
                .with_children(|rc| {
                    rc.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(0.0),
                            top: Val::Px(0.0),
                            ..default()
                        },
                        Text::new(String::new()),
                        TextFont {
                            font: FontSource::Handle(font.clone()),
                            font_size: FontSize::Px(12.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        ZIndex(10),
                        FriendLineText(i),
                    ));
                });
        }
    });
}

/// 显隐 + 列表渲染 + 打开时自动请求刷新（原版 C# FriendDialog.Show → RefreshFriends）
#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_arguments)]
fn friend_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut friend: ResMut<FriendState>,
    mut compose_mail: MessageWriter<ComposeMail>,
    mut chat: ResMut<ChatState>,
    net: Res<NetConnection>,
    mut wheels: MessageReader<bevy::input::mouse::MouseWheel>,
    close: Query<(Entity, &Interaction), With<FriendClose>>,
    actions: Query<(Entity, &Interaction, &FriendAction)>,
    tabs: Query<(Entity, &Interaction, Has<FriendTabBlock>)>,
    rows: Query<(Entity, &Interaction, &FriendLine), Without<FriendLineText>>,
    mut line_texts: Query<(&mut Text, &mut TextColor, &FriendLineText)>,
    mut input: ResMut<crate::game::dialogs::text_input::TextInputState>,
    mut submits: MessageReader<crate::game::dialogs::text_input::TextInputSubmit>,
    mut input_box: Query<&mut Visibility, With<FriendInputBox>>,
    mut widgets: Query<
        &mut Visibility,
        (With<FriendWidget>, Without<FriendInputBox>, Without<FriendLineText>),
    >,
    mut local: Local<FriendLocal>,
) {
    fn edge(
        e: Entity,
        inter: &Interaction,
        prev: &mut std::collections::HashMap<Entity, Interaction>,
    ) -> bool {
        let was = prev.insert(e, *inter);
        *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
    }

    let open = mgr.is_open(DialogKind::Friend);
    for mut vis in widgets.iter_mut() {
        *vis = if open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    let show_input = open && friend.pending.is_some();
    for mut vis in &mut input_box {
        *vis = if show_input {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !open {
        local.requested = false;
        friend.pending = None;
        friend.selected = None;
        input.active = None;
        local.offset = 0;
        return;
    }
    if !local.requested {
        local.requested = true;
        net.send_packet(&mir2_shared::packets::client::friend::RefreshFriends);
        tracing::info!("👥 请求刷新好友列表");
    }
    for (e, inter) in &close {
        if edge(e, inter, &mut local.prev_inter) {
            mgr.close(DialogKind::Friend);
        }
    }
    // 当前页签的显示列表
    let list = filter_friends(&friend.friends, friend.blocked_tab);
    let max_offset = list.len().saturating_sub(10);
    // 滚轮滚动
    let mut scroll_y = 0.0f32;
    for ev in wheels.read() {
        scroll_y += match ev.unit {
            bevy::input::mouse::MouseScrollUnit::Line => ev.y,
            bevy::input::mouse::MouseScrollUnit::Pixel => ev.y / 20.0,
        };
    }
    if scroll_y.abs() > 0.0 {
        local.offset = ((local.offset as i32) - (scroll_y * 3.0) as i32).clamp(0, max_offset as i32) as usize;
    }
    local.offset = (local.offset).min(max_offset);
    // 列表文本（含在线标记/备注/选中高亮）
    for (mut text, mut color, line) in &mut line_texts {
        let idx = local.offset + line.0;
        let selected = friend.selected == Some(idx);
        text.0 = match list.get(idx) {
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
        color.0 = if selected {
            Color::srgb(1.0, 0.9, 0.3)
        } else {
            Color::WHITE
        };
    }
    // 行点击选中（#130）
    for (e, inter, line) in &rows {
        if edge(e, inter, &mut local.prev_inter) {
            let idx = local.offset + line.0;
            friend.selected = if friend.selected == Some(idx) {
                None
            } else {
                Some(idx)
            };
        }
    }
    // 动作按钮 + 页签
    for (e, inter, act) in &actions {
        if !edge(e, inter, &mut local.prev_inter) {
            continue;
        }
        if act.is_add {
            friend.pending = Some(FriendPending::Add);
            input.texts[30].clear();
            input.active = Some(30);
        } else if act.is_remove {
            if let Some(idx) = friend.selected {
                if let Some(f) = list.get(idx) {
                    net.send_packet(&mir2_shared::packets::client::friend::RemoveFriend {
                        character_index: f.object_id as i32,
                    });
                    friend.selected = None;
                }
            }
        } else if act.is_memo {
            if let Some(idx) = friend.selected {
                friend.pending = Some(FriendPending::Memo(idx));
                input.texts[30].clear();
                input.active = Some(30);
            }
        } else if act.is_email {
            if let Some(f) = friend.selected.and_then(|i| list.get(i)).cloned() {
                compose_mail.write(ComposeMail { to: f.name.clone() });
            }
        } else if act.is_whisper {
            if let Some(f) = friend.selected.and_then(|i| list.get(i)).cloned() {
                match friend_whisper_command(&f.name, f.online) {
                    Some(cmd) => {
                        chat.input_active = true;
                        chat.input_text = cmd;
                    }
                    None => {
                        chat.add_line(
                            "该玩家不在线".to_string(),
                            Color::srgb(1.0, 0.3, 0.3),
                            ChatChannel::System,
                        );
                    }
                }
            }
        }
    }
    for (e, inter, is_block) in &tabs {
        if edge(e, inter, &mut local.prev_inter) {
            let target = is_block; // 黑名单页签 → true；好友页签 → false
            if friend.blocked_tab != target {
                friend.blocked_tab = target;
                friend.selected = None;
                local.offset = 0;
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
                    blocked: friend.blocked_tab,
                });
            }
            Some(FriendPending::Memo(idx)) => {
                if let Some(f) = list.get(idx) {
                    net.send_packet(&mir2_shared::packets::client::friend::AddMemo {
                        character_index: f.object_id as i32,
                        memo: name.clone(),
                    });
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
                if let Some(existing) = friend
                    .friends
                    .iter_mut()
                    .find(|f| f.object_id == e.object_id)
                {
                    *existing = e.clone();
                } else {
                    friend.friends.push(e.clone());
                }
            }
        }
    }
}
/// 按页签过滤好友列表（false=好友 true=黑名单，C# _blockedTab）
pub fn filter_friends(friends: &[FriendEntry], blocked_tab: bool) -> Vec<FriendEntry> {
    friends
        .iter()
        .filter(|f| f.blocked == blocked_tab)
        .cloned()
        .collect()
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
        assert_eq!(
            friend_whisper_command("Alice", true),
            Some("/w Alice ".to_string())
        );
        assert_eq!(friend_whisper_command("Alice", false), None);
    }

    #[test]
    fn filter_friends_by_tab() {
        let friends = vec![
            FriendEntry {
                object_id: 1,
                name: "a".into(),
                memo: String::new(),
                blocked: false,
                online: true,
            },
            FriendEntry {
                object_id: 2,
                name: "b".into(),
                memo: String::new(),
                blocked: true,
                online: false,
            },
        ];
        let ok = filter_friends(&friends, false);
        assert_eq!(ok.len(), 1);
        assert_eq!(ok[0].object_id, 1);
        let blk = filter_friends(&friends, true);
        assert_eq!(blk.len(), 1);
        assert_eq!(blk[0].object_id, 2);
    }
}

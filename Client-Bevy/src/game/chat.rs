// ============================================================================
// 聊天（M8）
// 交互参考：Client/MirScenes/Dialogs/MainDialogs.cs（ChatPanel / ChatTextBox）
// 显示：聊天面板（底部左侧），最新消息向上滚动；Enter 打开输入，再次 Enter 发送
// ============================================================================

use std::collections::VecDeque;

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;

use crate::game::hud::HudState;
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::scenes::AppState;
use crate::ui::pinyin_ime::{ImeFocus, PinyinIme};
use crate::ui::sprite_ui::{spawn_ui_text, UiButton, UiEntity, UiFont};

/// 聊天频道（主话框页签，对齐 C# MainDialogs ChatPanel）
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ChatChannel {
    /// 全部（仅页签用，行数据不落此值）
    All,
    System,
    Nearby,
    Guild,
    Group,
    Whisper,
}

/// 服务端 ChatType → 频道
pub fn chat_channel(t: mir2_shared::enums::ChatType) -> ChatChannel {
    use mir2_shared::enums::ChatType;
    match t {
        ChatType::Guild => ChatChannel::Guild,
        ChatType::Group => ChatChannel::Group,
        ChatType::WhisperIn | ChatType::WhisperOut => ChatChannel::Whisper,
        ChatType::System
        | ChatType::System2
        | ChatType::Announcement
        | ChatType::Hint
        | ChatType::LevelUp
        | ChatType::Mentor
        | ChatType::Trainer
        | ChatType::Relationship => ChatChannel::System,
        _ => ChatChannel::Nearby,
    }
}

/// 页签列表（顺序 = 显示顺序）
pub const CHAT_TABS: [ChatChannel; 6] = [
    ChatChannel::All,
    ChatChannel::System,
    ChatChannel::Nearby,
    ChatChannel::Guild,
    ChatChannel::Group,
    ChatChannel::Whisper,
];

/// 页签显示名
pub fn chat_tab_name(tab: ChatChannel) -> &'static str {
    match tab {
        ChatChannel::All => "全部",
        ChatChannel::System => "系统",
        ChatChannel::Nearby => "附近",
        ChatChannel::Guild => "行会",
        ChatChannel::Group => "队伍",
        ChatChannel::Whisper => "私聊",
    }
}

/// 聊天状态（网络 handler 写入，显示系统读取）
#[derive(Resource)]
pub struct ChatState {
    /// (文本, 颜色, 频道) 历史，最新在末尾
    pub lines: VecDeque<(String, Color, ChatChannel)>,
    /// 当前页签
    pub tab: ChatChannel,
    /// 输入框激活
    pub input_active: bool,
    /// 当前输入文本
    pub input_text: String,
    /// 显示行数
    pub visible_lines: usize,
}

impl Default for ChatState {
    fn default() -> Self {
        Self {
            lines: VecDeque::new(),
            tab: ChatChannel::All,
            input_active: false,
            input_text: String::new(),
            visible_lines: 8,
        }
    }
}

impl ChatState {
    pub fn add_line(&mut self, text: impl Into<String>, color: Color, channel: ChatChannel) {
        self.lines.push_back((text.into(), color, channel));
        while self.lines.len() > 200 {
            self.lines.pop_front();
        }
    }
}

/// 聊天面板根标记
#[derive(Component)]
struct ChatPanel;

/// 第 i 行文本
#[derive(Component)]
struct ChatLine(usize);

/// 输入行文本
#[derive(Component)]
struct ChatInputText;

/// 频道页签按钮
#[derive(Component)]
struct ChatTabBtn(ChatChannel);

/// 发送频道快捷按钮（C# ChatControlBar）
#[derive(Component)]
struct ChatBarBtn(&'static str, &'static str);

pub struct ChatPlugin;

impl Plugin for ChatPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Game), spawn_chat);
        app.add_systems(OnExit(AppState::Game), cleanup_chat);
        app.add_systems(
            Update,
            (chat_tab_system, chat_input_system, chat_display_system, chat_server_events)
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_chat(mut commands: Commands, roots: Query<Entity, With<ChatPanel>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_chat(
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

    let panel_x = 6.0;
    let panel_y = 768.0 - 150.0 - 190.0; // 主对话框上方

    // 面板底色（半透明黑，1x1 白图着色）
    let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    commands.spawn((
        UiEntity,
        ChatPanel,
        Sprite {
            image: white,
            custom_size: Some(Vec2::new(360.0, 172.0)),
            color: Color::srgba(0.0, 0.0, 0.0, 0.55),
            ..default()
        },
        Transform::from_xyz(panel_x + 180.0, -(panel_y + 75.0), 1.5),
        Visibility::default(),
    ));

    // 频道页签（主话框：全部/系统/附近/行会/队伍/私聊）
    let tab_w = 60.0;
    for (i, tab) in CHAT_TABS.iter().enumerate() {
        let tx = panel_x + 2.0 + i as f32 * tab_w;
        let e = spawn_ui_text(
            &mut commands, &font, chat_tab_name(*tab),
            tx, panel_y + 2.0,
            11.0, Color::srgb(0.8, 0.8, 0.8), 2.2,
        );
        commands.entity(e).insert((
            ChatTabBtn(*tab),
            UiButton {
                rect: (tx, panel_y + 2.0, tab_w - 2.0, 14.0),
                clicked: false,
            },
        ));
    }
    // 消息行（8 行）
    for i in 0..8usize {
        let e = spawn_ui_text(
            &mut commands, &font, "",
            panel_x + 4.0, panel_y + 20.0 + i as f32 * 16.0,
            12.0, Color::WHITE, 2.0,
        );
        commands.entity(e).insert(ChatLine(i));
    }
    // 输入行
    let e = spawn_ui_text(
        &mut commands, &font, "",
        panel_x + 4.0, panel_y + 150.0,
        12.0, Color::srgb(0.9, 0.9, 0.4), 2.0,
    );
    commands.entity(e).insert(ChatInputText);
    // 发送频道快捷按钮（C# ChatControlBar：附近/喊话/行会/队伍/私聊）
    // 点击把指令前缀填入输入框（服务端按前缀路由频道）
    let bar: [(&str, &str); 5] = [
        ("附近", ""),
        ("喊话", "/s "),
        ("行会", "/guild "),
        ("队伍", "/g "),
        ("私聊", "/w "),
    ];
    for (i, (label, prefix)) in bar.iter().enumerate() {
        // 频道快捷栏放在面板上方（C# ChatControlBar 为独立条）
        let bx = panel_x + 4.0 + i as f32 * 72.0;
        let by = panel_y - 16.0;
        let t = spawn_ui_text(
            &mut commands, &font, label,
            bx, by,
            11.0, Color::srgb(0.7, 0.9, 1.0), 2.2,
        );
        commands.entity(t).insert((
            ChatBarBtn(label, prefix),
            UiButton {
                rect: (bx, by, 70.0, 14.0),
                clicked: false,
            },
        ));
    }
}

/// 键盘输入：Enter 激活/发送；字符输入；Backspace 删除；内置拼音 IME 中文提交
fn chat_input_system(
    mut keys: MessageReader<KeyboardInput>,
    mut ime: ResMut<PinyinIme>,
    mut focus: ResMut<ImeFocus>,
    mut chat: ResMut<ChatState>,
    net: Res<NetConnection>,
    net_mode: Res<crate::network::NetMode>,
    hud: Res<HudState>,
) {
    let key_list: Vec<KeyboardInput> = keys.read().cloned().collect();

    // 回填 IME 聚焦框（聊天输入行屏幕矩形，候选条定位 + 判定字母是否进 IME）
    // 输入行位置见 spawn_chat：panel_x+4=10, panel_y+140=568
    // 只写 Some（None 由 clear_ime_focus 每帧统一重置，避免与 Game 态其他输入框互相覆盖）
    if chat.input_active {
        focus.rect = Some((10.0, 568.0, 350.0, 16.0));
    }

    // Enter：激活/发送（组合中被 IME 接管 → 跳过，不发送）
    for key in &key_list {
        if key.state != bevy::input::ButtonState::Pressed {
            continue;
        }
        if ime.consumes_key(key) {
            continue;
        }
        if key.logical_key == Key::Enter {
            if chat.input_active {
                let msg = chat.input_text.trim().to_string();
                chat.input_text.clear();
                chat.input_active = false;
                if !msg.is_empty() {
                    net.send_packet(&mir2_shared::packets::client::chat::Chat {
                        message: msg.clone(),
                        linked_items: Vec::new(),
                    });
                    // 本地回显（C# MainDialogs 发送时本地加入聊天面板；真实服务器不回发给自己）。
                    // 指令消息（/w /g /guild /s ! 等）服务器会回发对应频道回显 → 本地不再回显避免重复。
                    // mock 服务器会回显，只在真实 TCP 模式下本地回显避免重复
                    let is_cmd = msg.starts_with('/') || msg.starts_with('!');
                    if matches!(net_mode.0, crate::network::NetworkMode::Real) && !is_cmd {
                        chat.add_line(
                            format!("[{}]: {}", hud.name, msg),
                            Color::WHITE,
                            ChatChannel::Nearby,
                        );
                    }
                    tracing::info!("💬 发送聊天: {}", msg);
                }
            } else {
                chat.input_active = true;
            }
            continue;
        }
    }

    if !chat.input_active {
        return;
    }

    for key in &key_list {
        if key.state != bevy::input::ButtonState::Pressed {
            continue;
        }
        if ime.consumes_key(key) {
            continue;
        }
        if key.logical_key == Key::Backspace {
            chat.input_text.pop();
        } else if let Some(text) = &key.text {
            if !text.is_empty() {
                chat.input_text.push_str(text);
            }
        }
    }

    // 内置拼音 IME 提交的汉字
    if let Some(c) = ime.take_commit() {
        chat.input_text.push_str(&c);
    }
}

/// 显示：按页签过滤聊天行 + 输入行（单查询避免 B0001）
fn chat_display_system(
    chat: Res<ChatState>,
    mut texts: Query<(
        &mut Text2d,
        &mut TextColor,
        Option<&ChatLine>,
        Option<&ChatInputText>,
        Option<&ChatTabBtn>,
    )>,
) {
    // 按页签收集可见行（All 显示全部；否则只显示对应频道）
    let mut visible: Vec<(String, Color)> = chat
        .lines
        .iter()
        .filter(|(_, _, ch)| chat.tab == ChatChannel::All || *ch == chat.tab)
        .map(|(m, c, _)| (m.clone(), *c))
        .collect();
    let start = visible.len().saturating_sub(chat.visible_lines);
    for (mut text, mut color, line, input, tab_btn) in &mut texts {
        // 变化才更新，避免每帧重排文本（ICU4X 报错 + CPU，#31）
        if let Some(line) = line {
            let (msg, c) = match visible.get(start + line.0) {
                Some((m, c)) => (m.clone(), *c),
                None => (String::new(), Color::WHITE),
            };
            if text.0 != msg {
                text.0 = msg;
            }
            if color.0 != c {
                color.0 = c;
            }
        } else if input.is_some() {
            let new = if chat.input_active {
                format!("> {}", chat.input_text)
            } else {
                String::new()
            };
            if text.0 != new {
                text.0 = new;
            }
        } else if let Some(tab_btn) = tab_btn {
            // 选中页签高亮
            let selected = tab_btn.0 == chat.tab;
            let c = if selected {
                Color::srgb(1.0, 0.9, 0.3)
            } else {
                Color::srgb(0.8, 0.8, 0.8)
            };
            if color.0 != c {
                color.0 = c;
            }
        }
    }
}

/// 页签点击切换 + 发送频道快捷按钮
fn chat_tab_system(
    mut chat: ResMut<ChatState>,
    tabs: Query<(&UiButton, &ChatTabBtn)>,
    bar: Query<(&UiButton, &ChatBarBtn)>,
) {
    for (btn, tab) in &tabs {
        if btn.clicked && chat.tab != tab.0 {
            chat.tab = tab.0;
            tracing::info!("💬 聊天页签 -> {:?}", tab.0);
        }
    }
    for (btn, bar_btn) in &bar {
        if btn.clicked {
            chat.input_active = true;
            chat.input_text = bar_btn.1.to_string();
            tracing::info!("💬 频道快捷: {} -> prefix", bar_btn.0);
        }
    }
}


/// 聊天颜色映射（从网络层移入：展示逻辑归 UI 模块，网络只发 ServerEvent）
pub fn chat_color(t: mir2_shared::enums::ChatType) -> Color {
    use mir2_shared::enums::ChatType;
    match t {
        ChatType::Normal => Color::WHITE,
        ChatType::Shout | ChatType::Shout2 | ChatType::Shout3 => Color::srgb(1.0, 0.75, 0.3),
        ChatType::System | ChatType::System2 | ChatType::Announcement => {
            Color::srgb(1.0, 0.95, 0.4)
        }
        ChatType::Hint => Color::srgb(0.4, 1.0, 0.4),
        ChatType::Group => Color::srgb(0.5, 0.9, 1.0),
        ChatType::WhisperIn | ChatType::WhisperOut => Color::srgb(1.0, 0.5, 1.0),
        ChatType::Guild => Color::srgb(0.8, 0.6, 1.0),
        ChatType::LevelUp => Color::srgb(1.0, 0.9, 0.2),
        ChatType::Mentor | ChatType::Trainer | ChatType::Relationship => {
            Color::srgb(0.6, 1.0, 0.8)
        }
        _ => Color::WHITE,
    }
}

/// 消费服务端聊天事件更新 ChatState（网络层只广播 ServerEvent）
fn chat_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut chat: ResMut<ChatState>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        if let ServerEvent::Chat { text, chat_type } = ev {
            let color = chat_color(*chat_type);
            chat.add_line(text.clone(), color, chat_channel(*chat_type));
        }
    }
}

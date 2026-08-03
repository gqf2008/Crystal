// ============================================================================
// 聊天（M8）
// 交互参考：Client/MirScenes/Dialogs/MainDialogs.cs（ChatPanel / ChatTextBox）
// 显示：聊天面板（底部左侧），最新消息向上滚动；Enter 打开输入，再次 Enter 发送
// ============================================================================

use std::collections::VecDeque;

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;

use crate::map_renderer::GameLibraries;
use crate::network::NetworkContext;
use crate::scenes::AppState;
use crate::ui::pinyin_ime::{ImeFocus, PinyinIme};
use crate::ui::sprite_ui::{spawn_ui_text, UiEntity, UiFont};

/// 聊天状态（网络 handler 写入，显示系统读取）
#[derive(Resource)]
pub struct ChatState {
    /// (文本, 颜色) 历史，最新在末尾
    pub lines: VecDeque<(String, Color)>,
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
            input_active: false,
            input_text: String::new(),
            visible_lines: 8,
        }
    }
}

impl ChatState {
    pub fn add_line(&mut self, text: impl Into<String>, color: Color) {
        self.lines.push_back((text.into(), color));
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

pub struct ChatPlugin;

impl Plugin for ChatPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Game), spawn_chat);
        app.add_systems(OnExit(AppState::Game), cleanup_chat);
        app.add_systems(
            Update,
            (chat_input_system, chat_display_system).run_if(in_state(AppState::Game)),
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
            custom_size: Some(Vec2::new(360.0, 150.0)),
            color: Color::srgba(0.0, 0.0, 0.0, 0.55),
            ..default()
        },
        Transform::from_xyz(panel_x + 180.0, -(panel_y + 75.0), 1.5),
        Visibility::default(),
    ));

    // 消息行（8 行）
    for i in 0..8usize {
        let e = spawn_ui_text(
            &mut commands, &font, "",
            panel_x + 4.0, panel_y + 4.0 + i as f32 * 16.0,
            12.0, Color::WHITE, 2.0,
        );
        commands.entity(e).insert(ChatLine(i));
    }
    // 输入行
    let e = spawn_ui_text(
        &mut commands, &font, "",
        panel_x + 4.0, panel_y + 140.0,
        12.0, Color::srgb(0.9, 0.9, 0.4), 2.0,
    );
    commands.entity(e).insert(ChatInputText);
}

/// 键盘输入：Enter 激活/发送；字符输入；Backspace 删除；内置拼音 IME 中文提交
fn chat_input_system(
    mut keys: MessageReader<KeyboardInput>,
    mut ime: ResMut<PinyinIme>,
    mut focus: ResMut<ImeFocus>,
    mut chat: ResMut<ChatState>,
    net: Res<NetworkContext>,
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
                    // 本地回显（服务器也会广播回来，这里避免重复，交给服务器回显）
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

/// 显示：聊天行 + 输入行（单查询避免 B0001）
fn chat_display_system(
    chat: Res<ChatState>,
    mut texts: Query<(
        &mut Text2d,
        &mut TextColor,
        Option<&ChatLine>,
        Option<&ChatInputText>,
    )>,
) {
    let start = chat.lines.len().saturating_sub(chat.visible_lines);
    for (mut text, mut color, line, input) in &mut texts {
        if let Some(line) = line {
            if let Some((msg, c)) = chat.lines.get(start + line.0) {
                text.0 = msg.clone();
                color.0 = *c;
            } else {
                text.0 = String::new();
            }
        } else if input.is_some() {
            if chat.input_active {
                text.0 = format!("> {}", chat.input_text);
            } else {
                text.0 = String::new();
            }
        }
    }
}

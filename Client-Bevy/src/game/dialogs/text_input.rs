// ============================================================================
// 通用文本输入框（M26）
// 复用聊天输入模式（KeyboardInput MessageReader + PinyinIme 中文提交）：
//   - 点击输入框聚焦（原版 C# MirInputBox 语义）
//   - Enter 提交（返回 id，由使用方读取 TextInputState.texts[id]）
//   - Backspace 删除、字符输入、IME 中文提交
// 用法：spawn 实体挂 TextInputField(id) + TextInputRect(x,y,w,h)，子实体 TextInputDisplay(id) 显示文本
// ============================================================================

use bevy::prelude::*;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::sprite::Anchor;

use crate::ui::pinyin_ime::PinyinIme;
use crate::scenes::AppState;

/// 输入框状态（texts[id] = 输入框内容；active = 聚焦的输入框）
#[derive(Resource, Default)]
pub struct TextInputState {
    pub texts: Vec<String>,
    pub active: Option<usize>,
}

/// 输入框 id 标记
#[derive(Component, Clone, Copy)]
pub struct TextInputField(pub usize);

/// 输入框点击区域（屏幕坐标 + 尺寸）
#[derive(Component, Clone, Copy)]
pub struct TextInputRect(pub f32, pub f32, pub f32, pub f32);

/// 显示输入文本的子实体
#[derive(Component, Clone, Copy)]
pub struct TextInputDisplay(pub usize);

/// 提交消息（Enter 按下时发出，携带输入框 id）
#[derive(Message)]
pub struct TextInputSubmit(pub usize);

pub struct TextInputPlugin;

impl Plugin for TextInputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TextInputState>();
        app.add_message::<TextInputSubmit>();
        app.add_systems(
            Update,
            text_input_system.run_if(in_state(AppState::Game)),
        );
    }
}

/// 点击聚焦 + 键盘输入 + 显示同步 + Enter 提交
#[allow(clippy::too_many_arguments)]
fn text_input_system(
    mut state: ResMut<TextInputState>,
    mut keys: MessageReader<KeyboardInput>,
    mut ime: ResMut<PinyinIme>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    fields: Query<(Entity, &TextInputField, &TextInputRect)>,
    mut displays: Query<(&mut Text2d, &TextInputDisplay)>,
    mut submit: MessageWriter<TextInputSubmit>,
    mut last_active: Local<Option<usize>>,
) {
    // 输入框数量对齐
    let max_id = fields.iter().map(|(_, f, _)| f.0).max().unwrap_or(0);
    if state.texts.len() <= max_id {
        state.texts.resize(max_id + 1, String::new());
    }

    // 点击聚焦（原版 C# MirInputBox：点击输入框激活）
    if mouse.just_pressed(MouseButton::Left) {
        if let Ok(window) = windows.single() {
            if let Some(cursor) = window.cursor_position() {
                let mut clicked: Option<usize> = None;
                for (_e, f, r) in &fields {
                    if cursor.x >= r.0 && cursor.x <= r.0 + r.2 && cursor.y >= r.1 && cursor.y <= r.1 + r.3 {
                        clicked = Some(f.0);
                    }
                }
                // 点击输入框外 → 取消聚焦
                if clicked.is_none() && state.active.is_some() {
                    let outside = fields.iter().all(|(_, _, r)| {
                        !(cursor.x >= r.0 && cursor.x <= r.0 + r.2 && cursor.y >= r.1 && cursor.y <= r.1 + r.3)
                    });
                    if outside {
                        state.active = None;
                    }
                }
                if clicked.is_some() {
                    state.active = clicked;
                }
            }
        }
    }

    // Enter 提交 / 激活逻辑
    for key in keys.read() {
        if key.state != bevy::input::ButtonState::Pressed {
            continue;
        }
        if ime.consumes_key(key) {
            continue;
        }
        if key.logical_key == Key::Enter {
            if let Some(id) = state.active {
                submit.write(TextInputSubmit(id));
                // Enter 提交后保持聚焦（C# 输入框连续输入）
            }
        }
    }

    if let Some(active) = state.active {
        // 聚焦变化时刷新 IME 聚焦框
        if *last_active != Some(active) {
            *last_active = Some(active);
        }
        for key in keys.read() {
            if key.state != bevy::input::ButtonState::Pressed {
                continue;
            }
            if ime.consumes_key(key) {
                continue;
            }
            let text = &mut state.texts[active];
            if key.logical_key == Key::Backspace {
                text.pop();
            } else if let Some(t) = &key.text {
                if !t.is_empty() {
                    text.push_str(t);
                }
            }
        }
        // 内置拼音 IME 提交
        if let Some(c) = ime.take_commit() {
            state.texts[active].push_str(&c);
        }
    }

    // 显示同步
    for (mut text, disp) in &mut displays {
        text.0 = state.texts.get(disp.0).cloned().unwrap_or_default();
    }
}
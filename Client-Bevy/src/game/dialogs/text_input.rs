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

use crate::ui::pinyin_ime::{ImeFocus, PinyinIme};
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
    mut focus: ResMut<ImeFocus>,
) {
    // 输入框数量对齐
    let max_id = fields.iter().map(|(_, f, _)| f.0).max().unwrap_or(0);
    if state.texts.len() <= max_id {
        state.texts.resize(max_id + 1, String::new());
    }

    // 一帧键盘事件只读一次（MessageReader::read() 推进游标，二次 read 为空）
    let key_list: Vec<KeyboardInput> = keys.read().cloned().collect();

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

    // 回填 IME 聚焦框（只写 Some；None 由 clear_ime_focus 每帧统一重置，
    // 避免与 Game 态其他输入框如聊天框互相覆盖）
    if let Some(active) = state.active {
        for (_e, f, r) in &fields {
            if f.0 == active {
                focus.rect = Some((r.0, r.1, r.2, r.3));
                break;
            }
        }
    }

    // Enter 提交 / 激活逻辑
    for key in &key_list {
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
        for key in &key_list {
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

    // 显示同步（变化才更新，避免每帧重排文本，#31）
    for (mut text, disp) in &mut displays {
        let new = state.texts.get(disp.0).cloned().unwrap_or_default();
        if text.0 != new {
            text.0 = new;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::message::Messages;
    use bevy::input::keyboard::KeyCode;
    use bevy::input::{ButtonInput, ButtonState};
    use crate::scenes::AppState;
    use crate::ui::pinyin_ime::{PinyinIme, PinyinImePlugin};

    fn char_key(ch: &str) -> KeyboardInput {
        KeyboardInput {
            key_code: KeyCode::Space,
            logical_key: Key::Character(ch.into()),
            state: ButtonState::Pressed,
            text: Some(ch.into()),
            repeat: false,
            window: Entity::PLACEHOLDER,
        }
    }
    fn shift_key(state: ButtonState) -> KeyboardInput {
        KeyboardInput {
            key_code: KeyCode::ShiftLeft,
            logical_key: Key::Shift,
            state,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        }
    }
    fn send(app: &mut App, ev: KeyboardInput) {
        app.world_mut()
            .resource_mut::<Messages<KeyboardInput>>()
            .write(ev);
    }

    /// 组装一个处于 Game 态、含 1 个聚焦输入框(id=0)的最小 App
    fn app_with_focused_field() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(PinyinImePlugin);
        app.add_plugins(TextInputPlugin);
        app.add_message::<KeyboardInput>();
        app.init_resource::<ButtonInput<MouseButton>>();
        // 弱字体句柄：候选条实体不 spawn（纯逻辑测试）
        app.insert_resource(crate::ui::sprite_ui::UiFont(Handle::<Font>::default()));

        // 直接置 Game 态资源：in_state 只读 Res<State<AppState>>，无需
        // StatesPlugin/StateTransition（MinimalPlugins 不含 StatesPlugin）
        app.insert_resource(State::new(AppState::Game));

        // 输入框 id=0 + 点击矩形，并直接聚焦
        app.world_mut()
            .spawn((TextInputField(0), TextInputRect(340.0, 330.0, 200.0, 20.0)));
        {
            let mut st = app.world_mut().resource_mut::<TextInputState>();
            st.texts.resize(1, String::new());
            st.active = Some(0);
        }
        app.update(); // 稳定一帧：text_input 回填 ImeFocus
        app
    }

    /// 英文模式回归：字母原样进缓冲（证明字符循环能读到按键——修复前二次 read 为空）
    #[test]
    fn text_input_english_typing() {
        let mut app = app_with_focused_field();
        for c in ["a", "b", "c"] {
            send(&mut app, char_key(c));
            app.update();
        }
        assert_eq!(app.world().resource::<TextInputState>().texts[0], "abc");
    }

    /// 中文 IME 端到端：Shift 切中文 → 打 nihao → 数字 1 选候选 → 「你好」进缓冲。
    /// 依赖 text_input 每帧回填 ImeFocus.rect（修复前缺失 → 字母不进 IME）。
    #[test]
    fn text_input_chinese_ime_pipeline() {
        let mut app = app_with_focused_field();

        // Shift 单按切中文
        send(&mut app, shift_key(ButtonState::Pressed));
        app.update();
        send(&mut app, shift_key(ButtonState::Released));
        app.update();
        assert!(app.world().resource::<PinyinIme>().enabled());

        // 打 nihao（每帧 text_input 回填 focus.rect，字母被 IME 吞入拼音缓冲）
        for c in "nihao".chars() {
            send(&mut app, char_key(&c.to_string()));
            app.update();
        }

        // 数字 1 选首个候选 → 提交「你好」
        send(&mut app, char_key("1"));
        app.update();

        let st = app.world().resource::<TextInputState>();
        assert_eq!(st.texts[0], "你好");
        // 拼音缓冲已清空、英文字母未泄漏进缓冲
        assert!(!app.world().resource::<PinyinIme>().is_composing());
    }
}
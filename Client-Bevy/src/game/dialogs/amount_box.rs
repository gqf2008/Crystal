// ============================================================================
#![allow(clippy::type_complexity)]
// 数量输入框（M9 第 2 批，通用组件）
// 布局参考：macroquad amount_box.rs / C# MirAmountBox
//   - 背景 Prguse[238]；OK Title[200-202] (23,76)；Cancel Title[203-205] (110,76)
//   - 关闭 Prguse2[360-362] (180,3)；标题 (19,8)；数量输入区
// 使用：AmountBoxState.ask(title, max) → 用户输入 → AmountBoxResult 事件
// ============================================================================

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;

use crate::map_renderer::GameLibraries;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiFont, UiImageCache,
};

/// 数量输入结果事件（OK 时携带数量）
#[derive(Message, Debug)]
pub struct AmountBoxResult(pub Option<u32>);

#[derive(Resource, Default)]
pub struct AmountBoxState {
    pub visible: bool,
    pub title: String,
    pub max: u32,
    pub value: String,
}

#[derive(Component)]
pub struct AmountBoxWidget;

#[derive(Component)]
pub struct AmountOk;

#[derive(Component)]
pub struct AmountCancel;

#[derive(Component)]
pub struct AmountClose;

#[derive(Component)]
pub struct AmountTitleText;

#[derive(Component)]
pub struct AmountValueText;

pub struct AmountBoxPlugin;

impl Plugin for AmountBoxPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AmountBoxState>();
        app.add_message::<AmountBoxResult>();
        app.add_systems(OnEnter(AppState::Game), spawn_amount_box);
        app.add_systems(OnExit(AppState::Game), cleanup_amount_box);
        app.add_systems(
            Update,
            (amount_box_system, ui_button_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

impl AmountBoxState {
    /// 弹出数量输入框
    pub fn ask(&mut self, title: impl Into<String>, max: u32) {
        self.visible = true;
        self.title = title.into();
        self.max = max.max(1);
        self.value = String::new();
    }
}

fn cleanup_amount_box(mut commands: Commands, roots: Query<Entity, With<AmountBoxWidget>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_amount_box(
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

    // 居中弹窗
    let x = (1024.0 - 210.0) / 2.0;
    let y = (768.0 - 110.0) / 2.0;

    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 238) {
        let e = spawn_ui_sprite(&mut commands, h, x, y, 9.0, 1.0);
        commands.entity(e).insert((AmountBoxWidget, Visibility::Hidden));
    }

    // 标题
    let t = spawn_ui_text(&mut commands, &font, "", x + 19.0, y + 8.0, 12.0, Color::WHITE, 9.2);
    commands.entity(t).insert((AmountTitleText, AmountBoxWidget));

    // 数量值
    let v = spawn_ui_text(&mut commands, &font, "", x + 60.0, y + 40.0, 14.0, Color::WHITE, 9.2);
    commands.entity(v).insert((AmountValueText, AmountBoxWidget));

    // OK / Cancel / Close
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 200, 201, 202,
        x + 23.0, y + 76.0, 9.3, 76.0, 25.0,
    ) {
        commands.entity(e).insert((AmountOk, AmountBoxWidget));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 203, 204, 205,
        x + 110.0, y + 76.0, 9.3, 76.0, 25.0,
    ) {
        commands.entity(e).insert((AmountCancel, AmountBoxWidget));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 360, 361, 362,
        x + 180.0, y + 3.0, 9.3, 20.0, 20.0,
    ) {
        commands.entity(e).insert((AmountClose, AmountBoxWidget));
    }
}

/// 显示/隐藏 + 数字输入 + OK/Cancel/Close
#[allow(clippy::too_many_arguments)]
fn amount_box_system(
    mut state: ResMut<AmountBoxState>,
    mut result: MessageWriter<AmountBoxResult>,
    mut keys: MessageReader<KeyboardInput>,
    // #2596-3 接入 IME 契约：组合中的数字在选候选、退格在删拼音——数量框跳过，
    // 不再同帧双写（聊天输入激活时弹数量框，输入 "50" 会同时进两个缓冲）
    ime: Res<crate::ui::pinyin_ime::PinyinIme>,
    // 模态抢占（C# MirAmountBox 独立窗体夺走焦点）：打开瞬间收起聊天输入与
    // 通用输入框，数字/退格不再写进它们的缓冲
    mut chat: ResMut<crate::game::chat::ChatState>,
    mut text_input: ResMut<crate::game::dialogs::text_input::TextInputState>,
    mut was_open: Local<bool>,
    ok: Query<&UiButton, (With<AmountOk>, Without<AmountCancel>, Without<AmountClose>)>,
    cancel: Query<&UiButton, (With<AmountCancel>, Without<AmountOk>, Without<AmountClose>)>,
    close: Query<&UiButton, (With<AmountClose>, Without<AmountOk>, Without<AmountCancel>)>,
    mut widgets: Query<&mut Visibility, With<AmountBoxWidget>>,
    mut titles: Query<&mut Text2d, (With<AmountTitleText>, Without<AmountValueText>)>,
    mut values: Query<&mut Text2d, (With<AmountValueText>, Without<AmountTitleText>)>,
) {
    for mut vis in widgets.iter_mut() {
        *vis = if state.visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    // 打开瞬间收起其它输入框（模态抢占，#2596-3）
    if state.visible && !*was_open {
        chat.input_active = false;
        text_input.active = None;
    }
    *was_open = state.visible;
    if !state.visible {
        return;
    }

    // 数字键盘输入（一帧事件只读一次；未被 IME 用掉的事件按用掉次数配给，#2596-10）
    let key_list: Vec<KeyboardInput> = keys.read().cloned().collect();
    for key in ime.unconsumed(&key_list) {
        if key.state != bevy::input::ButtonState::Pressed {
            continue;
        }
        if key.logical_key == Key::Backspace {
            state.value.pop();
        } else if let Some(text) = &key.text {
            if text.chars().all(|c| c.is_ascii_digit()) && state.value.len() < 10 {
                state.value.push_str(text);
            }
        } else if let Key::Character(c) = &key.logical_key {
            // winit 注入/部分键盘事件 text=None，用 logical_key 兜底（原版 C# 任意可打印字符）
            if c.chars().all(|ch| ch.is_ascii_digit()) && state.value.len() < 10 {
                state.value.push_str(c);
            }
        }
    }

    for btn in &ok {
        if btn.clicked {
            let amount = state.value.parse::<u32>().ok().map(|v| v.min(state.max));
            state.visible = false;
            result.write(AmountBoxResult(amount));
        }
    }
    for btn in &cancel {
        if btn.clicked {
            state.visible = false;
            result.write(AmountBoxResult(None));
        }
    }
    for btn in &close {
        if btn.clicked {
            state.visible = false;
            result.write(AmountBoxResult(None));
        }
    }

    if let Ok(mut t) = titles.single_mut() {
        t.0 = state.title.clone();
    }
    if let Ok(mut v) = values.single_mut() {
        v.0 = state.value.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::chat::ChatState;
    use crate::game::dialogs::text_input::TextInputState;
    use crate::ui::pinyin_ime::{ImeFocus, PinyinIme, PinyinImePlugin};
    use bevy::ecs::message::Messages;
    use bevy::input::keyboard::KeyCode;
    use bevy::input::ButtonState;

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

    /// 最小 harness：数量框可见 + IME 聚焦回填 + amount_box_system 真实调度
    fn app_with_amount_box() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(PinyinImePlugin);
        app.add_message::<KeyboardInput>();
        app.add_message::<AmountBoxResult>();
        app.init_resource::<ButtonInput<MouseButton>>();
        app.init_resource::<ChatState>();
        app.init_resource::<TextInputState>();
        app.init_resource::<AmountBoxState>();
        app.insert_resource(crate::ui::sprite_ui::UiFont(Handle::<Font>::default()));
        app.insert_resource(State::new(AppState::Game));
        app.add_systems(Update, amount_box_system);
        app.add_systems(Update, |mut f: ResMut<ImeFocus>| {
            f.rect = Some((10.0, 10.0, 100.0, 16.0));
        });
        {
            let mut st = app.world_mut().resource_mut::<AmountBoxState>();
            st.visible = true;
        }
        app.update();
        app
    }

    /// #2596-3 回归：IME 组合中的数字在选候选——数量框跳过，不再同帧双写
    /// （旧实现聊天输入激活 + 弹数量框时，"50" 同时进聊天缓冲和数量框）。
    /// 普通数字（无组合）照常进数量框。
    #[test]
    fn ime_composing_digits_do_not_enter_amount_box() {
        let mut app = app_with_amount_box();
        // 切中文
        send(&mut app, shift_key(ButtonState::Pressed));
        app.update();
        send(&mut app, shift_key(ButtonState::Released));
        app.update();
        // 组合 "ni"
        for c in "ni".chars() {
            send(&mut app, char_key(&c.to_string()));
            app.update();
        }
        assert!(app.world().resource::<PinyinIme>().is_composing());

        // 组合中的数字 1 = 选首候选（被 IME 消费）→ 数量框不接收
        send(&mut app, char_key("1"));
        app.update();
        assert_eq!(
            app.world().resource::<AmountBoxState>().value,
            "",
            "组合中的数字在选候选，不应写进数量框"
        );

        // 组合已提交（无组合）→ 后续数字照常进数量框
        send(&mut app, char_key("5"));
        app.update();
        assert_eq!(app.world().resource::<AmountBoxState>().value, "5");
    }

    /// #2596-3 回归：数量框是模态输入——打开瞬间收起聊天输入与通用输入框，
    /// 数字/退格不再同时写进它们的缓冲。
    #[test]
    fn amount_box_open_closes_chat_and_text_input() {
        let mut app = app_with_amount_box();
        // 模拟聊天输入激活 → 数量框打开（harness 里 visible 已 true）
        app.world_mut().resource_mut::<ChatState>().input_active = true;
        app.world_mut().resource_mut::<TextInputState>().active = Some(0);
        // 数量框已打开过一帧（was_open=true）→ 关掉再开，触发抢占
        app.world_mut().resource_mut::<AmountBoxState>().visible = false;
        app.update();
        app.world_mut().resource_mut::<AmountBoxState>().visible = true;
        app.update();
        assert!(
            !app.world().resource::<ChatState>().input_active,
            "数量框打开应收起聊天输入"
        );
        assert!(
            app.world().resource::<TextInputState>().active.is_none(),
            "数量框打开应清空通用输入框聚焦"
        );
    }
}

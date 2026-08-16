// ============================================================================
// keyboard_nav - 对话框键盘交互（#92）
// 参考：C# KeyBindSettings Closeall（ESC 关闭全部窗口）+ MirControl 键盘导航
//   - ESC：关闭所有打开对话框（C# Closeall 语义）
//   - ↑/↓/PageUp/PageDown：滚动最上层打开对话框的列表（ScrollList）
//   - Tab/Shift+Tab：在最上层对话框按钮间切换焦点，Enter 触发点击
//     （click_remaining 保证对话框系统无论顺序都能读到点击）
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::{DialogManager, DialogRoot};
use crate::ui::scroll_list::ScrollList;
use crate::ui::sprite_ui::{UiButton, UiEntity};

/// 键盘导航状态
#[derive(Resource, Default)]
pub struct KeyboardNav {
    /// 当前聚焦按钮
    pub focused: Option<Entity>,
    /// Enter 触发后保持 clicked=true 的剩余帧数
    pub click_remaining: u8,
    /// 焦点高亮框实体
    pub highlight: Option<Entity>,
}

/// ESC 三级优先级（#2595，C# WinForms 焦点路由 + MirTextBox.cs:386-395）：
/// 1. 聊天输入开 → 本系统让路（chat_input_system 同帧关闭输入行；
///    注册处 .before(chat_input_system) 保证这里先看到 input_active=true）；
///    C# TextBox_KeyPress Escape → ActiveControl=null 且 e.Handled，不触发 Closeall
/// 2. 通用输入框聚焦 → 只取消聚焦（对话框不动）
/// 3. 无输入聚焦 → Closeall（C# KeyBindSettings Closeall）
///
/// 另（#2596-7）：Esc 被内置 IME 用来取消拼音组合的那帧，本系统整体让路——
/// ButtonInput 只知帧级真假，无从区分事件实例，故用 ime.escape_consumed()。
pub fn esc_close_dialogs_system(
    keys: Res<ButtonInput<KeyCode>>,
    ime: Res<crate::ui::pinyin_ime::PinyinIme>,
    chat: Res<crate::game::chat::ChatState>,
    mut mgr: ResMut<DialogManager>,
    mut input: ResMut<crate::game::dialogs::text_input::TextInputState>,
) {
    if !keys.just_pressed(KeyCode::Escape) {
        return;
    }
    // 取消组合的那次 Esc 已被 IME 用掉：只收候选条，不当帧连带关对话框
    if ime.escape_consumed() {
        return;
    }
    if chat.input_active {
        return;
    }
    if input.active.is_some() {
        input.active = None;
        return;
    }
    if !mgr.open.is_empty() {
        mgr.open.clear();
        tracing::info!("⌨️ ESC 关闭全部对话框");
    }
}

/// ↑/↓/PageUp/PageDown：滚动最上层打开对话框的 ScrollList
/// （#2595：文本输入聚焦时让路——箭头键进文本框，C# 焦点路由）
pub fn keyboard_scroll_lists_system(
    keys: Res<ButtonInput<KeyCode>>,
    gate: Res<crate::game::input_gate::TextInputGate>,
    mgr: Res<DialogManager>,
    mut lists: Query<(&mut ScrollList, &DialogRoot)>,
) {
    if gate.0 {
        return;
    }
    let Some(top) = mgr.open.last().copied() else {
        return;
    };
    let delta = if keys.just_pressed(KeyCode::ArrowUp) {
        -1
    } else if keys.just_pressed(KeyCode::ArrowDown) {
        1
    } else if keys.just_pressed(KeyCode::PageUp) {
        -3
    } else if keys.just_pressed(KeyCode::PageDown) {
        3
    } else {
        0
    };
    if delta == 0 {
        return;
    }
    for (mut list, root) in &mut lists {
        if root.0 != top {
            continue;
        }
        let max = list.max_offset() as i32;
        list.offset = (list.offset as i32 + delta).clamp(0, max) as usize;
        return;
    }
}

/// Tab/Shift+Tab 焦点切换 + Enter 触发点击 + 焦点高亮框
/// （#2595：文本输入聚焦时让路——C# 聚焦 TextBox 时 Tab 转发给游戏键位
/// （拾取，MainDialogs.cs:1160-1185），Enter 归输入框；对话框导航是移植附加，
/// 打字时整体让路，仅清理残留焦点/高亮）
pub fn tab_focus_system(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    gate: Res<crate::game::input_gate::TextInputGate>,
    mut nav: ResMut<KeyboardNav>,
    mgr: Res<DialogManager>,
    mut images: ResMut<Assets<Image>>,
    mut buttons: Query<(Entity, &mut UiButton, Option<&DialogRoot>)>,
    mut highlight_q: Query<(&mut Transform, &mut Sprite), Without<UiButton>>,
) {
    if gate.0 {
        nav.focused = None;
        nav.click_remaining = 0;
        if let Some(he) = nav.highlight {
            if let Ok((mut tf, _)) = highlight_q.get_mut(he) {
                tf.translation.x = -9999.0;
                tf.translation.y = -9999.0;
            }
        }
        return;
    }
    // 收集最上层打开对话框的按钮（按位置 y 排序便于上下导航）
    let top = mgr.open.last().copied();
    let mut cands: Vec<(Entity, (f32, f32, f32, f32))> = buttons
        .iter()
        .filter(|(_, _, root)| root.map(|r| Some(r.0) == top).unwrap_or(false))
        .map(|(e, b, _)| (e, b.rect))
        .collect();
    cands.sort_by(|a, b| a.1 .1.partial_cmp(&b.1 .1).unwrap_or(std::cmp::Ordering::Equal));

    if cands.is_empty() {
        nav.focused = None;
        return;
    }

    // Tab/Shift+Tab 移动焦点
    if keys.just_pressed(KeyCode::Tab) {
        let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
        let cur = nav.focused.and_then(|f| cands.iter().position(|(e, _)| *e == f));
        let next = match cur {
            Some(i) if shift => (i + cands.len() - 1) % cands.len(),
            Some(i) => (i + 1) % cands.len(),
            None => 0,
        };
        nav.focused = Some(cands[next].0);
        tracing::info!("⌨️ Tab 焦点切换");
    }

    // Enter 触发点击（保持几帧，避免对话框系统顺序导致漏读）
    if keys.just_pressed(KeyCode::Enter) && nav.focused.is_some() {
        nav.click_remaining = 3;
    }
    if nav.click_remaining > 0 {
        if let Some(f) = nav.focused {
            if let Ok((_, mut b, _)) = buttons.get_mut(f) {
                b.clicked = true;
            }
        }
        nav.click_remaining -= 1;
    }

    // 焦点高亮框
    let Some(f) = nav.focused else {
        if let Some(he) = nav.highlight {
            if let Ok((mut tf, _)) = highlight_q.get_mut(he) {
                tf.translation.x = -9999.0;
                tf.translation.y = -9999.0;
            }
        }
        return;
    };
    let Ok((_, b, _)) = buttons.get(f) else {
        return;
    };
    let (x, y, w, h) = b.rect;
    if nav.highlight.is_none() {
        let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
        let e = commands
            .spawn((
                UiEntity,
                Sprite {
                    image: white,
                    color: Color::srgba(1.0, 0.9, 0.2, 0.5),
                    custom_size: Some(Vec2::new(w + 2.0, h + 2.0)),
                    ..default()
                },
                bevy::sprite::Anchor::TOP_LEFT,
                Transform::from_xyz(x - 1.0, -(y - 1.0), 20.0),
                Visibility::Visible,
            ))
            .id();
        nav.highlight = Some(e);
    }
    if let Some(he) = nav.highlight {
        if let Ok((mut tf, mut sp)) = highlight_q.get_mut(he) {
            tf.translation.x = x - 1.0;
            tf.translation.y = -(y - 1.0);
            if let Some(cs) = sp.custom_size.as_mut() {
                *cs = Vec2::new(w + 2.0, h + 2.0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::chat::ChatState;
    use crate::game::dialogs::text_input::TextInputState;
    use crate::ui::pinyin_ime::{ImeFocus, PinyinDict, PinyinIme, PinyinImePlugin};
    use bevy::ecs::message::Messages;
    use bevy::input::keyboard::{Key, KeyboardInput};
    use bevy::input::ButtonInput;

    fn char_key(ch: &str) -> KeyboardInput {
        KeyboardInput {
            key_code: KeyCode::Space,
            logical_key: Key::Character(ch.into()),
            state: bevy::input::ButtonState::Pressed,
            text: Some(ch.into()),
            repeat: false,
            window: Entity::PLACEHOLDER,
        }
    }

    fn shift_key(state: bevy::input::ButtonState) -> KeyboardInput {
        KeyboardInput {
            key_code: KeyCode::ShiftLeft,
            logical_key: Key::Shift,
            state,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        }
    }

    fn esc_key() -> KeyboardInput {
        KeyboardInput {
            key_code: KeyCode::Escape,
            logical_key: Key::Escape,
            state: bevy::input::ButtonState::Pressed,
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

    /// esc_close_dialogs_system 依赖 PinyinIme（escape_consumed 让路）
    fn insert_ime(app: &mut App) {
        app.insert_resource(PinyinIme::new(PinyinDict::load()));
        app.init_resource::<ImeFocus>();
    }

    fn esc_app(chat_open: bool, text_active: Option<usize>) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<ButtonInput<KeyCode>>();
        insert_ime(&mut app);
        app.insert_resource(ChatState {
            input_active: chat_open,
            ..Default::default()
        });
        app.init_resource::<TextInputState>();
        app.init_resource::<DialogManager>();
        app.add_systems(Update, esc_close_dialogs_system);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        let mut mgr = app.world_mut().resource_mut::<DialogManager>();
        mgr.open.push(crate::game::dialogs::DialogKind::Inventory);
        if let Some(id) = text_active {
            app.world_mut().resource_mut::<TextInputState>().active = Some(id);
        }
        app
    }

    /// #2595 Esc 三级优先级（C# MirTextBox.cs:386-395）：
    /// 聊天输入开 → esc_close 让路（对话框不动，输入行由 chat_input_system 关）
    #[test]
    fn esc_yields_to_chat_input() {
        let mut app = esc_app(true, None);
        app.update();
        assert_eq!(
            app.world().resource::<DialogManager>().open.len(),
            1,
            "聊天输入开时 Esc 不应关对话框"
        );
        assert!(
            app.world().resource::<ChatState>().input_active,
            "esc_close 不动聊天输入（由 chat_input_system 同帧关）"
        );
    }

    /// 通用输入框聚焦 → 只取消聚焦，对话框不动
    #[test]
    fn esc_clears_generic_input_only() {
        let mut app = esc_app(false, Some(0));
        app.update();
        assert!(
            app.world().resource::<TextInputState>().active.is_none(),
            "Esc 应取消通用输入框聚焦"
        );
        assert_eq!(
            app.world().resource::<DialogManager>().open.len(),
            1,
            "输入框聚焦时 Esc 不应关对话框"
        );
    }

    /// 无任何输入聚焦 → Closeall（C# KeyBindSettings Closeall）
    #[test]
    fn esc_closes_all_dialogs() {
        let mut app = esc_app(false, None);
        app.update();
        assert!(
            app.world().resource::<DialogManager>().open.is_empty(),
            "无输入聚焦时 Esc 应关闭全部对话框"
        );
    }

    /// #2596-7 Esc 单键单效（IME 让路）：取消拼音组合的那次 Esc 只收候选条，
    /// 本系统不当帧连带关全部对话框；组合结束后的下一次 Esc 恢复 Closeall。
    /// ButtonInput 只知帧级真假，让路靠 ime.escape_consumed()。
    #[test]
    fn esc_yields_to_ime_composition_cancel() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(PinyinImePlugin); // PreUpdate 真实调度 IME 处理器
        app.add_message::<KeyboardInput>();
        // 弱字体句柄：候选条/chip 实体不 spawn（纯逻辑测试，pinyin_ime.rs 同款）
        app.insert_resource(crate::ui::sprite_ui::UiFont(Handle::<Font>::default()));
        app.init_resource::<ButtonInput<KeyCode>>();
        app.insert_resource(ChatState::default());
        app.init_resource::<TextInputState>();
        app.init_resource::<DialogManager>();
        app.add_systems(Update, esc_close_dialogs_system);
        // Update 回填聚焦（对齐生产契约：文本框每帧重写 ImeFocus）
        app.add_systems(Update, |mut f: ResMut<ImeFocus>| {
            f.rect = Some((10.0, 10.0, 100.0, 16.0));
        });

        // Shift 单按切中文 → 喂 'n' 组合中
        send(&mut app, shift_key(bevy::input::ButtonState::Pressed));
        app.update();
        send(&mut app, shift_key(bevy::input::ButtonState::Released));
        app.update();
        send(&mut app, char_key("n"));
        app.update();
        assert!(app.world().resource::<PinyinIme>().is_composing());

        // 同帧：Esc 键盘事件（IME 取消组合并消费）+ ButtonInput Esc 按下
        send(&mut app, esc_key());
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        let mut mgr = app.world_mut().resource_mut::<DialogManager>();
        mgr.open.push(crate::game::dialogs::DialogKind::Inventory);
        app.update();
        assert!(
            !app.world().resource::<PinyinIme>().is_composing(),
            "IME 应已取消组合"
        );
        assert_eq!(
            app.world().resource::<DialogManager>().open.len(),
            1,
            "取消组合的 Esc 不应连带关掉对话框"
        );

        // 组合已空 → 下一次 Esc 恢复 Closeall（ButtonInput 需先释放再按下，
        // just_pressed 只在释放→按下跳变置位）
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.release(KeyCode::Escape);
            keys.press(KeyCode::Escape);
        }
        app.update();
        assert!(
            app.world().resource::<DialogManager>().open.is_empty(),
            "组合结束后的 Esc 应恢复关闭全部对话框"
        );
    }
}

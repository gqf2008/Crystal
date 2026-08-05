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

/// ESC：关闭所有打开对话框（C# Closeall）+ 清空文本输入焦点
pub fn esc_close_dialogs_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut mgr: ResMut<DialogManager>,
    mut input: ResMut<crate::game::dialogs::text_input::TextInputState>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        if !mgr.open.is_empty() {
            mgr.open.clear();
            tracing::info!("⌨️ ESC 关闭全部对话框");
        }
        input.active = None;
    }
}

/// ↑/↓/PageUp/PageDown：滚动最上层打开对话框的 ScrollList
pub fn keyboard_scroll_lists_system(
    keys: Res<ButtonInput<KeyCode>>,
    mgr: Res<DialogManager>,
    mut lists: Query<(&mut ScrollList, &DialogRoot)>,
) {
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
pub fn tab_focus_system(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut nav: ResMut<KeyboardNav>,
    mgr: Res<DialogManager>,
    mut images: ResMut<Assets<Image>>,
    mut buttons: Query<(Entity, &mut UiButton, Option<&DialogRoot>)>,
    mut highlight_q: Query<(&mut Transform, &mut Sprite), Without<UiButton>>,
) {
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

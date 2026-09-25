// ============================================================================
// 屏幕通知（M9 第 4 批）
// 布局参考：macroquad chat_notice_dialog.rs
//   - 背景 Prguse[1361]/Layout[1360]，屏幕顶部通知
//   - ChatNotice 网络包触发，自动消失
// ============================================================================

use bevy::prelude::*;

use crate::map_renderer::GameLibraries;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::UiFont;
use crate::ui::sprite_ui::{shared_cjk_font, UiCjkFont};
use crate::ui::theme::{load_lib_image, spawn_panel};

/// #2892 批B：面板精灵与 C# 原生尺寸（C# `ChatNoticeDialog.Index = 1361; Library = Libraries.Prguse`，
/// `Location = (ScreenWidth/2 - W/2, ScreenHeight/6 - H/2)`）
pub const PANEL: (LibraryName, usize) = (LibraryName::Prguse, 1361);
pub const PANEL_SIZE: (f32, f32) = (660.0, 25.0);

fn chat_notice_origin(width: f32, height: f32) -> (f32, f32) {
    // C# ChatNoticeDialog：X = ScreenWidth/2 - W/2；
    // Y = ScreenHeight/6 - H/2（逐项整数除法）。
    (
        (crate::game::dialogs::UI_SCREEN_W / 2.0 - (width / 2.0).floor()).floor(),
        ((crate::game::dialogs::UI_SCREEN_H / 6.0).floor() - (height / 2.0).floor()).floor(),
    )
}

/// 屏幕通知状态（网络 ChatNotice 写入）
#[derive(Resource, Default)]
pub struct ChatNoticeState {
    pub visible: bool,
    pub text: String,
    /// 剩余显示时间（秒）
    pub remaining: f32,
}

#[derive(Component)]
pub struct ChatNoticeWidget;

#[derive(Component)]
pub struct ChatNoticeText;

pub struct ChatNoticePlugin;

impl Plugin for ChatNoticePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiCjkFont>();
        app.init_resource::<ChatNoticeState>();
        app.add_systems(OnEnter(AppState::Game), spawn_chat_notice);
        app.add_systems(OnExit(AppState::Game), cleanup_chat_notice);
        app.add_systems(Update, chat_notice_system.run_if(in_state(AppState::Game)));
    }
}

fn cleanup_chat_notice(mut commands: Commands, roots: Query<Entity, With<ChatNoticeWidget>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_chat_notice(
    mut commands: Commands,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<UiFont>,
    mut cjk_font: ResMut<UiCjkFont>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        crate::ui::sprite_ui::ensure_ui_font(&mut fonts, &mut ui_font);
    }
    let font = ui_font.0.clone();
    // 可能含中文（动态填充/服务端文案）：用自带 CJK 的主字体（Arial handle 画中文是豆腐）
    let cjk = shared_cjk_font(&mut fonts, &mut cjk_font);

    // 背景 Prguse[1361] 原生 660x25；位置按 C# ChatNoticeDialog 公式。
    let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 1361) else {
        return;
    };
    let (px, py) = chat_notice_origin(660.0, 25.0);
    let panel = spawn_panel(&mut commands, bg, px, py, 660.0, 25.0, 50);
    commands.entity(panel).insert(ChatNoticeWidget);
    commands.entity(panel).with_children(|p| {
        crate::ui::theme::spawn_label_center(
            p,
            &cjk,
            "",
            330.0,
            4.0,
            640.0,
            14.0,
            Color::srgb(1.0, 0.9, 0.4),
            9,
        )
        .insert(ChatNoticeText);
    });
}

/// 显示/计时消失
fn chat_notice_system(
    mut state: ResMut<ChatNoticeState>,
    time: Res<Time>,
    mut widgets: Query<&mut Visibility, (With<ChatNoticeWidget>, Without<ChatNoticeText>)>,
    mut texts: Query<&mut Text, With<ChatNoticeText>>,
) {
    if state.visible {
        state.remaining -= time.delta_secs();
        if state.remaining <= 0.0 {
            state.visible = false;
        }
    }
    for mut vis in widgets.iter_mut() {
        *vis = if state.visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if let Ok(mut t) = texts.single_mut() {
        // 剥离 {text/color} 标签 + 多行
        let stripped: Vec<String> = state
            .text
            .split('\n')
            .map(crate::ui::controls::strip_color_tags)
            .collect();
        let joined = stripped.join("\n");
        if t.0 != joined {
            t.0 = joined;
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn chat_notice_origin_matches_csharp() {
        assert_eq!(super::chat_notice_origin(660.0, 25.0), (182.0, 116.0));
    }
}

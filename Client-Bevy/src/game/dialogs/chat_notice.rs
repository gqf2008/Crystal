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
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_image, UiEntity, UiFont, UiImageCache,
};

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
        app.init_resource::<ChatNoticeState>();
        app.add_systems(OnEnter(AppState::Game), spawn_chat_notice);
        app.add_systems(OnExit(AppState::Game), cleanup_chat_notice);
        app.add_systems(
            Update,
            chat_notice_system.run_if(in_state(AppState::Game)),
        );
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
    mut cache: ResMut<UiImageCache>,
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<UiFont>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();

    // 背景（屏幕顶部中央）
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 1361) {
        let e = spawn_ui_sprite(&mut commands, h, 330.0, 80.0, 9.0, 1.0);
        commands.entity(e).insert((ChatNoticeWidget, Visibility::Hidden));
    }
    let t = spawn_ui_text(&mut commands, &font, "", 350.0, 90.0, 14.0, Color::srgb(1.0, 0.9, 0.4), 9.2);
    commands.entity(t).insert((ChatNoticeText, ChatNoticeWidget));
}

/// 显示/计时消失
fn chat_notice_system(
    mut state: ResMut<ChatNoticeState>,
    time: Res<Time>,
    mut widgets: Query<&mut Visibility, With<ChatNoticeWidget>>,
    mut texts: Query<&mut Text2d, With<ChatNoticeText>>,
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
        t.0 = state.text.clone();
    }
}

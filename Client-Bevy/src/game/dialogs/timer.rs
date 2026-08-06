// ============================================================================
// 计时器对话框（M50）
// 纯客户端对话框（无网络依赖）
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiFont, UiImageCache,
};

/// 状态
#[derive(Resource, Default)]
pub struct TimerState {
    pub message: String,
    /// #230：网络计时器是否激活（S.SetTimer 启动 / S.ExpireTimer 或倒计时归零关闭）
    pub active: bool,
    pub remaining: f32,
}

#[derive(Component)]
pub struct TimerWidget;

#[derive(Component)]
pub struct TimerClose;

#[derive(Component)]
pub struct TimerLine(usize);

pub struct TimerPlugin;

impl Plugin for TimerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TimerState>();
        app.add_systems(OnEnter(AppState::Game), spawn_timer);
        app.add_systems(OnExit(AppState::Game), cleanup_timer);
        app.add_systems(
            Update,
            (timer_network_events, timer_countdown, timer_ui_system, ui_button_system)
                .chain()
                .after(crate::network::network_system)
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_timer(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_timer(
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

    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 170) {
        let e = spawn_ui_sprite(&mut commands, h, 280.0, 80.0, 6.0, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::Timer),
            TimerWidget,
            Visibility::Hidden,
        ));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 360, 361, 362,
        280.0 + 300.0, 83.0, 7.0, 20.0, 20.0,
    ) {
        commands.entity(e).insert((
            TimerClose,
            DialogRoot(DialogKind::Timer),
            TimerWidget,
        ));
    }
    for i in 0..10usize {
        let e = spawn_ui_text(
            &mut commands, &font, "",
            298.0, 120.0 + i as f32 * 22.0,
            12.0, Color::WHITE, 8.0,
        );
        commands.entity(e).insert((
            TimerLine(i),
            DialogRoot(DialogKind::Timer),
            TimerWidget,
        ));
    }
}

/// 显隐 + 渲染 + 关闭
fn timer_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut timer: ResMut<TimerState>,
    close: Query<&UiButton, With<TimerClose>>,
    mut widgets: Query<&mut Visibility, With<TimerWidget>>,
    mut lines: Query<(&mut Text2d, &TimerLine)>,
) {
    let open = mgr.is_open(DialogKind::Timer);
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        return;
    }
    for btn in &close {
        if btn.clicked {
            mgr.close(DialogKind::Timer);
        }
    }
    const TIMER_LINES: [&str; 2] = [
        "—— 计时器 ——",
        "服务端计时（S.SetTimer）",
    ];
    for (mut text, line) in &mut lines {
        text.0 = match line.0 {
            i if i < 2 => TIMER_LINES[i].to_string(),
            i if i == 4 => timer.message.clone(),
            _ => String::new(),
        };
    }
    // 无网络计时器时显示占位文案（#230：有倒计时时由 timer_countdown 维护 message）
    if !timer.active {
        timer.message = format!("{} 对话框", "计时器");
    }
}

/// #230：S.SetTimer / S.ExpireTimer → 打开/关闭计时器对话框
fn timer_network_events(
    mut mgr: ResMut<DialogManager>,
    mut timer: ResMut<TimerState>,
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
) {
    for ev in events.read() {
        match ev {
            crate::network::server_event::ServerEvent::TimerSet { seconds, .. } => {
                timer.active = true;
                timer.remaining = (*seconds).max(1) as f32;
                mgr.open.push(DialogKind::Timer);
                tracing::info!("⏱️ [TIMER] 启动计时器 {} 秒", seconds);
            }
            crate::network::server_event::ServerEvent::TimerExpired { .. } => {
                timer.active = false;
                timer.remaining = 0.0;
                mgr.close(DialogKind::Timer);
                tracing::info!("⏱️ [TIMER] 计时器关闭");
            }
            _ => {}
        }
    }
}

/// #230：倒计时（归零自动关闭）
fn timer_countdown(time: Res<Time>, mut mgr: ResMut<DialogManager>, mut timer: ResMut<TimerState>) {
    if !timer.active {
        return;
    }
    timer.remaining -= time.delta_secs();
    timer.message = format!("剩余 {:.0} 秒", timer.remaining.max(0.0));
    if timer.remaining <= 0.0 {
        timer.active = false;
        timer.message = String::new();
        mgr.close(DialogKind::Timer);
        tracing::info!("⏱️ [TIMER] 倒计时归零");
    }
}

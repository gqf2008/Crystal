// ============================================================================
// 计时器对话框（M50）
// 纯客户端对话框（无网络依赖）
// bevy_ui 迁移（批 16）：面板 Prguse[170] @(280,80) 320x262，全节点化
// 注：C# TimerDialog 实际是 Prguse2 沙漏动画+数字位（Index=960/_libraryOffset=900，
//     起点 (ScreenWidth-120, ScreenHeight-230)）；本实现沿用既有 Bevy 简化面板
//     （Prguse[170] + 文本行），仅迁移渲染层，C# 逐帧对齐留待后续
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::UiFont;
use crate::ui::theme::{load_lib_image, spawn_icon_button, spawn_label, spawn_panel};

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
            (timer_network_events, timer_countdown, timer_ui_system)
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
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<UiFont>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();

    let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 170) else {
        return;
    };
    let panel = spawn_panel(&mut commands, bg, 280.0, 80.0, 320.0, 262.0, 30);
    commands
        .entity(panel)
        .insert((DialogRoot(DialogKind::Timer), TimerWidget));

    commands.entity(panel).with_children(|p| {
        // 关闭 Prguse2[360/361/362] @(300,3)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 360),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 361),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 362),
        ) {
            spawn_icon_button(p, n, h, pr, 300.0, 3.0, 20.0, 20.0, 10).insert(TimerClose);
        }
        // 10 行信息 @(18,40+22i)
        for i in 0..10usize {
            spawn_label(p, &font, "", 18.0, 40.0 + i as f32 * 22.0, 12.0, Color::WHITE, 9)
                .insert(TimerLine(i));
        }
    });
}

/// 显隐 + 渲染 + 关闭
fn timer_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut timer: ResMut<TimerState>,
    close: Query<(Entity, &Interaction), With<TimerClose>>,
    mut widgets: Query<&mut Visibility, With<TimerWidget>>,
    mut lines: Query<(&mut Text, &TimerLine)>,
    mut prev_inter: Local<std::collections::HashMap<Entity, Interaction>>,
) {
    fn edge(
        e: Entity,
        inter: &Interaction,
        prev: &mut std::collections::HashMap<Entity, Interaction>,
    ) -> bool {
        let was = prev.insert(e, *inter);
        *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
    }
    let open = mgr.is_open(DialogKind::Timer);
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        return;
    }
    for (e, inter) in &close {
        if edge(e, inter, &mut prev_inter) {
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

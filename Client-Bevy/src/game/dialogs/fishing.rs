// ============================================================================
// 钓鱼对话框（M39）
// 参考：C# FishingDialog（Prguse[1340]）+ ServerRust tick_fishing / FishingCast
// 网络：
//   C: FishingCast[fishing_type u8] / FishingChangeAutocast[enabled u8]
//   S: FishingUpdate(198)[progress i32][success u8]
// 收获结果通过系统聊天消息返回（C# ReceiveChat 语义）
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiFont, UiImageCache,
};

/// 钓鱼状态（FishingUpdate 写入）
#[derive(Resource, Default)]
pub struct FishingState {
    /// 0=未钓鱼 1=等待 2=上钩 3=收竿 5=自动钓鱼切换
    pub progress: i32,
    pub success: bool,
    pub autocast: bool,
    pub message: String,
}

#[derive(Component)]
pub struct FishingWidget;

#[derive(Component)]
pub struct FishingClose;

#[derive(Component)]
pub struct FishingCast;

#[derive(Component)]
pub struct FishingAutocast;

#[derive(Component)]
pub struct FishingLine(usize);

pub struct FishingPlugin;

impl Plugin for FishingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FishingState>();
                app.add_systems(
            Update,
            fishing_server_events.run_if(in_state(AppState::Game)),
        );
app.add_systems(OnEnter(AppState::Game), spawn_fishing);
        app.add_systems(OnExit(AppState::Game), cleanup_fishing);
        app.add_systems(
            Update,
            (fishing_ui_system, ui_button_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_fishing(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_fishing(
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

    // 背景 Prguse[1340]（C# FishingDialog.Index=1340）
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 1340) {
        let e = spawn_ui_sprite(&mut commands, h, 280.0, 80.0, 6.0, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::Fishing),
            FishingWidget,
            Visibility::Hidden,
        ));
    }
    // 关闭（C# Prguse2 360/361/362）
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 360, 361, 362,
        280.0 + 220.0, 83.0, 7.0, 20.0, 20.0,
    ) {
        commands.entity(e).insert((
            FishingClose,
            DialogRoot(DialogKind::Fishing),
            FishingWidget,
        ));
    }
    // 状态行
    for i in 0..4usize {
        let e = spawn_ui_text(
            &mut commands, &font, "",
            298.0, 120.0 + i as f32 * 22.0,
            12.0, Color::WHITE, 8.0,
        );
        commands.entity(e).insert((
            FishingLine(i),
            DialogRoot(DialogKind::Fishing),
            FishingWidget,
        ));
    }
    // 抛竿按钮
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 206, 207, 208,
        300.0, 220.0, 8.3, 76.0, 25.0,
    ) {
        commands.entity(e).insert((
            FishingCast,
            DialogRoot(DialogKind::Fishing),
            FishingWidget,
        ));
    }
    // 自动钓鱼开关
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 210, 211, 212,
        390.0, 220.0, 8.3, 76.0, 25.0,
    ) {
        commands.entity(e).insert((
            FishingAutocast,
            DialogRoot(DialogKind::Fishing),
            FishingWidget,
        ));
    }
}

/// 显隐 + 渲染 + 抛竿/自动钓鱼
#[allow(clippy::too_many_arguments)]
fn fishing_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut state: ResMut<FishingState>,
    net: Res<NetConnection>,
    close: Query<&UiButton, With<FishingClose>>,
    cast_btn: Query<&UiButton, With<FishingCast>>,
    autocast_btn: Query<&UiButton, With<FishingAutocast>>,
    mut widgets: Query<&mut Visibility, With<FishingWidget>>,
    mut lines: Query<(&mut Text2d, &FishingLine)>,
) {
    let open = mgr.is_open(DialogKind::Fishing);
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        return;
    }
    for btn in &close {
        if btn.clicked {
            mgr.close(DialogKind::Fishing);
        }
    }
    let status = match state.progress {
        0 => "未钓鱼".to_string(),
        1 => "等待中…".to_string(),
        2 => "上钩了！".to_string(),
        3 => "收竿中…".to_string(),
        5 => "自动钓鱼已切换".to_string(),
        _ => format!("进度 {}", state.progress),
    };
    for (mut text, line) in &mut lines {
        text.0 = match line.0 {
            0 => format!("钓鱼状态: {}", status),
            1 => format!("自动钓鱼: {}", if state.autocast { "开" } else { "关" }),
            2 => state.message.clone(),
            3 => "需要装备鱼竿（武器栏）".to_string(),
            _ => String::new(),
        };
    }
    // 抛竿（C# FishingDialog → C.FishingCast）
    for btn in &cast_btn {
        if btn.clicked {
            net.send_packet(&crate::network::FishingCastWire { fishing_type: 0 });
            state.message = "已抛竿，等待鱼上钩…".to_string();
            tracing::info!("🎣 抛竿");
        }
    }
    // 自动钓鱼开关
    for btn in &autocast_btn {
        if btn.clicked {
            state.autocast = !state.autocast;
            net.send_packet(&crate::network::FishingChangeAutocastWire {
                enabled: state.autocast,
            });
            state.message = format!(
                "自动钓鱼: {}",
                if state.autocast { "开" } else { "关" }
            );
            tracing::info!("🎣 自动钓鱼: {}", state.autocast);
        }
    }
}


/// 消费服务端钓鱼事件（网络层只广播 ServerEvent；文案在此构造）
fn fishing_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut fishing: ResMut<FishingState>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        if let ServerEvent::FishingUpdate { progress, success } = ev {
            fishing.progress = *progress;
            fishing.success = *success;
            fishing.message = match progress {
                1 => "等待中…".to_string(),
                2 => {
                    if *success {
                        "上钩了！".to_string()
                    } else {
                        "鱼跑了…".to_string()
                    }
                }
                _ => "钓鱼中".to_string(),
            };
        }
    }
}

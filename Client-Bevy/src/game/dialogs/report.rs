// ============================================================================
// 举报对话框（M45）
// 参考：C# ReportDialog + ServerRust ReportIssueRequest
// 网络（ServerRust gate 实际 wire）：
//   C: ReportIssue[type u32][description dotnet]（与 SharedRust [message dotnet] 不一致，手动构造）
// 结果通过系统聊天消息返回
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::network::NetworkContext;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiFont, UiImageCache,
};

/// 举报状态
#[derive(Resource, Default)]
pub struct ReportState {
    pub message: String,
}

#[derive(Component)]
pub struct ReportWidget;

#[derive(Component)]
pub struct ReportClose;

#[derive(Component)]
pub struct ReportSubmit;

#[derive(Component)]
pub struct ReportLine(usize);

pub struct ReportPlugin;

impl Plugin for ReportPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ReportState>();
        app.add_systems(OnEnter(AppState::Game), spawn_report);
        app.add_systems(OnExit(AppState::Game), cleanup_report);
        app.add_systems(
            Update,
            (report_ui_system, ui_button_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_report(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_report(
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
            DialogRoot(DialogKind::Report),
            ReportWidget,
            Visibility::Hidden,
        ));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 360, 361, 362,
        280.0 + 300.0, 83.0, 7.0, 20.0, 20.0,
    ) {
        commands.entity(e).insert((
            ReportClose,
            DialogRoot(DialogKind::Report),
            ReportWidget,
        ));
    }
    // 状态行 3 + 输入框（类型 TextInput 11 / 描述 12）
    for i in 0..3usize {
        let e = spawn_ui_text(
            &mut commands, &font, "",
            298.0, 120.0 + i as f32 * 22.0,
            12.0, Color::WHITE, 8.0,
        );
        commands.entity(e).insert((
            ReportLine(i),
            DialogRoot(DialogKind::Report),
            ReportWidget,
        ));
    }
    let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    spawn_report_input(&mut commands, &white, &font, 11, 298.0, 190.0, 80.0);
    spawn_report_input(&mut commands, &white, &font, 12, 298.0, 222.0, 200.0);
    // 提交按钮
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 206, 207, 208,
        360.0, 258.0, 8.3, 76.0, 25.0,
    ) {
        commands.entity(e).insert((
            ReportSubmit,
            DialogRoot(DialogKind::Report),
            ReportWidget,
        ));
    }
}

/// 举报输入框（TextInputField(id) + 子 TextInputDisplay(id)）
fn spawn_report_input(
    commands: &mut Commands,
    white: &Handle<Image>,
    font: &Handle<Font>,
    id: usize,
    x: f32,
    y: f32,
    w: f32,
) {
    let box_e = commands
        .spawn((
            crate::ui::sprite_ui::UiEntity,
            DialogRoot(DialogKind::Report),
            ReportWidget,
            crate::game::dialogs::text_input::TextInputField(id),
            crate::game::dialogs::text_input::TextInputRect(x, y, w, 20.0),
            Sprite {
                image: white.clone(),
                color: Color::srgba(0.2, 0.2, 0.25, 0.9),
                custom_size: Some(Vec2::new(w, 20.0)),
                ..default()
            },
            bevy::sprite::Anchor::TOP_LEFT,
            Transform::from_xyz(x, -y, 8.1),
            Visibility::Hidden,
        ))
        .id();
    commands.entity(box_e).with_children(|p| {
        p.spawn((
            crate::game::dialogs::text_input::TextInputDisplay(id),
            Text2d::new(String::new()),
            bevy::sprite::Anchor::TOP_LEFT,
            TextFont {
                font: FontSource::Handle(font.clone()),
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(Color::srgb(1.0, 1.0, 1.0)),
            Transform::from_xyz(4.0, -2.0, 8.2),
        ));
    });
}

/// 显隐 + 渲染 + 提交
#[allow(clippy::too_many_arguments)]
fn report_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut state: ResMut<ReportState>,
    net: Res<NetworkContext>,
    mut input: ResMut<crate::game::dialogs::text_input::TextInputState>,
    close: Query<&UiButton, With<ReportClose>>,
    submit_btn: Query<&UiButton, With<ReportSubmit>>,
    mut widgets: Query<&mut Visibility, With<ReportWidget>>,
    mut lines: Query<(&mut Text2d, &ReportLine)>,
) {
    let open = mgr.is_open(DialogKind::Report);
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        return;
    }
    for btn in &close {
        if btn.clicked {
            mgr.close(DialogKind::Report);
        }
    }
    for (mut text, line) in &mut lines {
        text.0 = match line.0 {
            0 => "举报（GM）".to_string(),
            1 => "类型（数字） + 描述".to_string(),
            2 => state.message.clone(),
            _ => String::new(),
        };
    }
    for btn in &submit_btn {
        if btn.clicked {
            let rtype = input
                .texts
                .get(11)
                .cloned()
                .unwrap_or_default()
                .trim()
                .parse::<u32>()
                .unwrap_or(0);
            let desc = input.texts.get(12).cloned().unwrap_or_default();
            let desc = desc.trim().to_string();
            if desc.is_empty() {
                state.message = "请填写描述".to_string();
                return;
            }
            net.send_packet(&crate::network::ReportIssueWire {
                issue_type: rtype,
                description: desc.clone(),
            });
            state.message = "举报已提交，感谢反馈".to_string();
            tracing::info!("📮 举报: type={} desc={}", rtype, desc);
            input.texts[11].clear();
            input.texts[12].clear();
            input.active = None;
        }
    }
}

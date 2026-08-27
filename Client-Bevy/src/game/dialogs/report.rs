// ============================================================================
// 举报对话框（M45）
// 参考：C# ReportDialog + ServerRust ReportIssueRequest
// 网络（ServerRust gate 实际 wire）：
//   C: ReportIssue[type u32][description dotnet]（与 SharedRust [message dotnet] 不一致，手动构造）
// 结果通过系统聊天消息返回
// bevy_ui 迁移（批 13）：面板 Prguse[170] @(280,80) 320x262，全节点化
//   - 关闭 Prguse2[360/361/362] @(300,3)
//   - 状态行 3 + 类型下拉（bevy_ui UiDropDown）+ 描述输入（TextInput 12）+ 提交 Title[206/207/208]
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{shared_cjk_font, UiCjkFont, UiFont};
use crate::ui::theme::{
    load_lib_image, spawn_container, spawn_dropdown_ui, spawn_icon_button, spawn_label,
    spawn_panel, UiDropDown,
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

#[derive(Component)]
pub struct ReportTypeDrop;

pub struct ReportPlugin;

impl Plugin for ReportPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ReportState>();
        app.add_systems(OnEnter(AppState::Game), spawn_report);
        app.add_systems(OnExit(AppState::Game), cleanup_report);
        app.add_systems(
            Update,
            report_ui_system.run_if(in_state(AppState::Game)),
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
    mut fonts: ResMut<Assets<Font>>,
    mut cjk_font: ResMut<UiCjkFont>,
    mut ui_font: ResMut<UiFont>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();
    let cjk = shared_cjk_font(&mut fonts, &mut cjk_font);

    // 面板 Prguse[170]（C# ReportDialog.Index=170，320x262 @ 280,80）
    let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 170) else {
        return;
    };
    let panel = spawn_panel(&mut commands, bg, 280.0, 80.0, 320.0, 262.0, 30);
    commands
        .entity(panel)
        .insert((DialogRoot(DialogKind::Report), ReportWidget));

    commands.entity(panel).with_children(|p| {
        // 关闭 Prguse2[360/361/362] @(300,3)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 360),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 361),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 362),
        ) {
            spawn_icon_button(p, n, h, pr, 300.0, 3.0, 20.0, 20.0, 10).insert(ReportClose);
        }
        // 状态行 3（0 标题 / 1 说明 / 2 反馈）@(18,40+22i)
        for i in 0..3usize {
            spawn_label(p, &cjk, "", 18.0, 40.0 + i as f32 * 22.0, 12.0, Color::WHITE, 9)
                .insert(ReportLine(i));
        }
        // 类型下拉（C# ReportType DropDown）@(18,110)
        spawn_dropdown_ui(
            p,
            &font,
            vec!["请选择类型".to_string(), "提交BUG".to_string(), "举报玩家".to_string()],
            None,
            (280.0, 80.0),
            18.0,
            110.0,
            80.0,
            20.0,
            3,
            9,
        )
        .insert(ReportTypeDrop);
        // 描述输入框（TextInput id 12）@(18,142)，命中矩形 = 屏幕坐标 (298,222,200,20)
        spawn_container(p, 18.0, 142.0, 200.0, 20.0, 10)
            .insert((
                BackgroundColor(Color::srgba(0.2, 0.2, 0.25, 0.9)),
                crate::game::dialogs::text_input::TextInputField(12),
                crate::game::dialogs::text_input::TextInputRect(298.0, 222.0, 200.0, 20.0),
            ))
            .with_children(|ic| {
                ic.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(4.0),
                        top: Val::Px(2.0),
                        ..default()
                    },
                    Text::new(String::new()),
                    TextFont {
                        font: FontSource::Handle(font.clone()),
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    ZIndex(11),
                    crate::game::dialogs::text_input::TextInputDisplay(12),
                ));
            });
        // 提交按钮 Title[206/207/208] @(80,178)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 206),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 207),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 208),
        ) {
            spawn_icon_button(p, n, h, pr, 80.0, 178.0, 76.0, 25.0, 11).insert(ReportSubmit);
        }
    });
}

/// 显隐 + 渲染 + 提交
#[allow(clippy::too_many_arguments)]
fn report_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut state: ResMut<ReportState>,
    net: Res<NetConnection>,
    mut input: ResMut<crate::game::dialogs::text_input::TextInputState>,
    close: Query<(Entity, &Interaction), With<ReportClose>>,
    submit_btn: Query<(Entity, &Interaction), With<ReportSubmit>>,
    type_dd: Query<&UiDropDown, With<ReportTypeDrop>>,
    mut widgets: Query<&mut Visibility, With<ReportWidget>>,
    mut lines: Query<(&mut Text, &ReportLine)>,
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
    let open = mgr.is_open(DialogKind::Report);
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        return;
    }
    for (e, inter) in &close {
        if edge(e, inter, &mut prev_inter) {
            mgr.close(DialogKind::Report);
        }
    }
    for (mut text, line) in &mut lines {
        text.0 = match line.0 {
            0 => "举报（GM）".to_string(),
            1 => "类型 + 描述".to_string(),
            2 => state.message.clone(),
            _ => String::new(),
        };
    }
    for (e, inter) in &submit_btn {
        if edge(e, inter, &mut prev_inter) {
            // #90 类型来自下拉（0=未选择）
            let rtype = type_dd.single().ok().and_then(|dd| dd.selected).unwrap_or(0) as u32;
            let desc = input.texts.get(12).cloned().unwrap_or_default();
            let desc = desc.trim().to_string();
            if rtype == 0 {
                state.message = "请选择举报类型".to_string();
                return;
            }
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
            input.texts[12].clear();
            input.active = None;
        }
    }
}

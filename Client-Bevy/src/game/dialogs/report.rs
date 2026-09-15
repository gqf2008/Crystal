// ============================================================================
// 举报对话框（M45）
// 参考：C# ReportDialog + ServerRust ReportIssueRequest
// 网络（ServerRust gate 实际 wire）：
//   C: ReportIssue[type u32][description dotnet]（与 SharedRust [message dotnet] 不一致，手动构造）
// 结果通过系统聊天消息返回
// bevy_ui：C# ReportDialog 360x244 @ Center；Prguse[1633] 缺失时深色兜底
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

/// C# `ReportDialog`（`Client/MirScenes/Dialogs/ReportDialog.cs:15-16`）：`Index = 1633; Library = Libraries.Prguse`
pub const PANEL: (LibraryName, usize) = (LibraryName::Prguse, 1633);
/// C# 无显式 `Size` → 用 art 尺寸；本端在数据包缺 `Prguse[1633]` 时按同尺寸深色兜底
pub const PANEL_SIZE: (f32, f32) = (360.0, 244.0);
/// 关闭键 `Prguse2[360..362]` @(336,3)（`ReportDialog.cs:21-30`，无 `Size` → art 24x21）
pub const CLOSE_REL: (f32, f32) = (336.0, 3.0);
/// 类型下拉 `ReportType` @(12,35) 170x14（`:33-41`）
pub const TYPE_DROP: (f32, f32, f32, f32) = (12.0, 35.0, 170.0, 14.0);
/// 描述框 `MessageArea` @(12,57) 330x150（`:46-54`，`MultiLine()`）
pub const MESSAGE_AREA: (f32, f32, f32, f32) = (12.0, 57.0, 330.0, 150.0);
/// 提交 `SendButton` `Title[607/608/609]` @(260,219)（`:56-65`，无 `Size` → art）
pub const SUBMIT_REL: (f32, f32) = (260.0, 219.0);

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
        app.add_systems(Update, report_ui_system.run_if(in_state(AppState::Game)));
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

    // C# ReportDialog: Prguse[1633]，Location = Center。当前数据包缺少 1633 时使用
    // 同尺寸深色兜底面板；控件仍按 C# 坐标保留，避免继续错用 Prguse[170]。
    let (px, py) = crate::game::dialogs::center_origin(PANEL_SIZE.0, PANEL_SIZE.1);
    let bg = load_lib_image(&mut libs, &mut images, PANEL.0, PANEL.1).unwrap_or_else(|| {
        images.add(crate::map_renderer::make_image(vec![22, 23, 30, 255], 1, 1))
    });
    let panel = spawn_panel(&mut commands, bg, px, py, PANEL_SIZE.0, PANEL_SIZE.1, 30);
    commands
        .entity(panel)
        .insert((DialogRoot(DialogKind::Report), ReportWidget));

    commands.entity(panel).with_children(|p| {
        // 标题/状态文本。
        spawn_label(p, &cjk, "", 12.0, 8.0, 12.0, Color::WHITE, 9).insert(ReportLine(0));
        spawn_label(
            p,
            &cjk,
            "",
            12.0,
            220.0,
            11.0,
            Color::srgb(1.0, 0.8, 0.4),
            9,
        )
        .insert(ReportLine(1));
        // 关闭 Prguse2[360/361/362] @(336,3)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 360),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 361),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 362),
        ) {
            // C# 无 `Size` → art 24x21（此前 20x20 是自造尺寸）
            spawn_icon_button(p, n, h, pr, CLOSE_REL.0, CLOSE_REL.1, 24.0, 21.0, 10)
                .insert(ReportClose);
        }
        // 类型下拉（C# ReportType @(12,35)，170x14）
        spawn_dropdown_ui(
            p,
            &font,
            vec![
                "请选择类型".to_string(),
                "提交BUG".to_string(),
                "举报玩家".to_string(),
            ],
            None,
            (px, py),
            12.0,
            35.0,
            170.0,
            14.0,
            3,
            9,
        )
        .insert(ReportTypeDrop);
        // 描述输入框（C# MessageArea @(12,57)，330x150）
        spawn_container(p, 12.0, 57.0, 330.0, 150.0, 10)
            .insert((
                BackgroundColor(Color::srgba(0.16, 0.17, 0.22, 0.96)),
                crate::game::dialogs::text_input::TextInputField(12),
                crate::game::dialogs::text_input::TextInputRect(px + 12.0, py + 57.0, 330.0, 150.0),
            ))
            .with_children(|ic| {
                ic.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(6.0),
                        top: Val::Px(5.0),
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
        // 提交按钮（C# SendButton Title[607/608/609] @(260,219)）
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 607),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 608),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 609),
        ) {
            spawn_icon_button(p, n, h, pr, 260.0, 219.0, 76.0, 25.0, 11).insert(ReportSubmit);
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
        *vis = if open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
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
            1 => state.message.clone(),
            _ => String::new(),
        };
    }
    for (e, inter) in &submit_btn {
        if edge(e, inter, &mut prev_inter) {
            // #90 类型来自下拉（0=未选择）
            let rtype = type_dd
                .single()
                .ok()
                .and_then(|dd| dd.selected)
                .unwrap_or(0) as u32;
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

#[cfg(test)]
mod tests {
    #[test]
    fn report_layout_matches_csharp() {
        assert_eq!(
            crate::game::dialogs::center_origin(360.0, 244.0),
            (332.0, 262.0)
        );
    }
}

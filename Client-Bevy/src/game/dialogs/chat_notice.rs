// ============================================================================
// 屏幕顶部公告横幅（C# `ChatNoticeDialog`，`Client/MirScenes/Dialogs/ChatNoticeDialog.cs`）
//   - 底条 `Prguse[1361]`（660x25，Opacity 0.7）+ 装饰边子层 `Prguse[1360]`（660x25 @(0,0)）
//   - 文案 `TextLabel1`：660x40 框 @(0,-6) 竖横居中、10 号字、纯黄 + 黑描边
//   - 显示 10s（`ViewTime = 10000`）后自隐
//
// 触发（C# `MainDialogs.cs:791-794`）：`ChatType.Announcement` 聊天消息 —— 除了进聊天面板，
// 还要 `ChatNoticeDialog.ShowNotice(RegexFunctions.CleanChatString(text))`。
// **2026-09-27 复核结论**：本端此前只有窗口壳（`ChatNoticeState` 没有任何写入方）⇒ 横幅
// 在实机上永远不会出现；本轮把触发接上，并按金标准补齐装饰层/透明度/字号/颜色。
// C# 的 `type = 1` 分支（`Prguse[1363]/[1362]` + 15 号字）**全树无调用方**，属死代码，不实现。
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::NotDraggable;
use crate::map_renderer::GameLibraries;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::UiFont;
use crate::ui::sprite_ui::{shared_cjk_font, UiCjkFont};
use crate::ui::theme::{load_lib_image, spawn_panel};

/// 面板精灵与 C# 原生尺寸（C# `ChatNoticeDialog.Index = 1361; Library = Libraries.Prguse`，
/// `Location = (ScreenWidth/2 - W/2, ScreenHeight/6 - H/2)`）——图头 660x25。
///
/// 还有一层**装饰边**：C# `Layout = new MirImageControl { Index = 1360, Location = (0,0) }`
/// 是面板的子控件 ⇒ 先画黑底条（1361）、再画这条边（1360）。图头同为 660x25。
pub const PANEL: (LibraryName, usize) = (LibraryName::Prguse, 1361);
pub const LAYOUT: (LibraryName, usize) = (LibraryName::Prguse, 1360);
pub const PANEL_SIZE: (f32, f32) = (660.0, 25.0);

/// C# `ChatNoticeDialog.ViewTime = 10000`（ms）——横幅显示时长
pub const VIEW_TIME_SECS: f32 = 10.0;
/// C# `ChatNoticeDialog.Opacity = 0.7F`（只作用在**黑底条**这一层；`Layout` 子控件自己没设
/// Opacity ⇒ 装饰边仍是 1.0。`MirImageControl.DrawControl` 每层各用自己的 Opacity）
pub const PANEL_OPACITY: f32 = 0.7;
/// C# `TextLabel1`：`Location = (0, -6)`、`Size = (660, 40)`、`VerticalCenter|HorizontalCenter`、
/// `Font(Settings.FontName, 10F)`、`ForeColour = Color.Yellow`、`OutLineColour = Color.Black`
pub const LABEL_BOX: (f32, f32, f32, f32) = (0.0, -6.0, 660.0, 40.0);
pub const LABEL_SIZE: f32 = 10.0;
/// 文字纵向位置：C# 在 40 高的框里竖中居中 ⇒ 中心 y = -6 + 20 = 14；
/// 本端标签是**顶部锚点**（见 `spawn_label_center`），字号 10 的行高约 12 ⇒ top = 14 - 6 ≈ 8。
pub const LABEL_TOP: f32 = 8.0;
/// C# `Color.Yellow` = (255,255,0)
pub const LABEL_COLOR: Color = Color::srgb(1.0, 1.0, 0.0);

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

impl ChatNoticeState {
    /// C# `ChatNoticeDialog.ShowNotice(text)`（`ChatNoticeDialog.cs:68-78`）：
    /// 写文本 + 显示 + `CurrentTime = CMain.Time + ViewTime`（10s。
    /// 注意 C# 的 `type = 1`（`Prguse[1363]/[1362]` + 15 号字）**没有任何调用方**
    /// ——全树只有 `MainDialogs.cs:794` 一处 `ShowNotice(text)`，故本端只实现 type 0，
    /// 不为死分支造 UI。）
    pub fn show(&mut self, text: impl Into<String>) {
        self.text = text.into();
        self.visible = true;
        self.remaining = VIEW_TIME_SECS;
    }
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
    let Some(bg) = load_lib_image(&mut libs, &mut images, PANEL.0, PANEL.1) else {
        return;
    };
    let bg_for_alpha = bg.clone();
    let (px, py) = chat_notice_origin(PANEL_SIZE.0, PANEL_SIZE.1);
    let panel = spawn_panel(&mut commands, bg, px, py, PANEL_SIZE.0, PANEL_SIZE.1, 50);
    commands
        .entity(panel)
        .insert((ChatNoticeWidget, NotDraggable));
    // C# `Opacity = 0.7F`（同 `dura_status` 的写法：C# `Library.Draw(..., Opacity)`
    // ↔ 本端 `ImageNode` 的颜色 alpha）
    commands
        .entity(panel)
        .insert(ImageNode::new(bg_for_alpha).with_color(Color::srgba(
            1.0,
            1.0,
            1.0,
            PANEL_OPACITY,
        )));
    let layout_img = load_lib_image(&mut libs, &mut images, LAYOUT.0, LAYOUT.1);
    commands.entity(panel).with_children(|p| {
        // C# `Layout`（`Prguse[1360]` @(0,0) 660x25）：画在黑底条之上，是横幅的装饰边
        if let Some(layout) = layout_img {
            crate::ui::theme::spawn_image(p, layout, 0.0, 0.0, PANEL_SIZE.0, PANEL_SIZE.1, 1);
        }
        // C# `TextLabel1`：660x40 框、竖横居中、10 号字、纯黄 + 黑描边
        crate::ui::theme::spawn_label_center(
            p,
            &cjk,
            "",
            330.0,
            LABEL_TOP,
            LABEL_BOX.2,
            LABEL_SIZE,
            LABEL_COLOR,
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
    use super::*;

    #[test]
    fn chat_notice_origin_matches_csharp() {
        assert_eq!(super::chat_notice_origin(660.0, 25.0), (182.0, 116.0));
    }

    /// 门禁（金标准 ⑧ 长尾窗）：逐项钉住 `ChatNoticeDialog.cs:13-56` 的常量。
    /// 阳性对照：把 `PANEL_OPACITY` 改回 1.0、或把 `LAYOUT` 指回 1361 ⇒ 本测试 FAILED。
    #[test]
    fn chat_notice_geometry_matches_csharp() {
        assert_eq!(PANEL, (LibraryName::Prguse, 1361), "C# Index = 1361");
        assert_eq!(LAYOUT, (LibraryName::Prguse, 1360), "C# Layout.Index = 1360");
        assert_eq!(PANEL_SIZE, (660.0, 25.0), "Prguse[1361] 图头 660x25");
        assert_eq!(VIEW_TIME_SECS, 10.0, "C# ViewTime = 10000ms");
        assert_eq!(PANEL_OPACITY, 0.7, "C# Opacity = 0.7F");
        assert_eq!(LABEL_BOX, (0.0, -6.0, 660.0, 40.0), "C# TextLabel1");
        assert_eq!(LABEL_SIZE, 10.0, "C# Font(..., 10F)");
        assert_eq!(
            LABEL_COLOR.to_srgba(),
            Color::srgb(1.0, 1.0, 0.0).to_srgba(),
            "C# Color.Yellow"
        );
        // C# 竖中居中 ⇒ 中心 y = -6 + 20 = 14；本端顶部锚点 ⇒ top + 行高/2 ≈ 14
        let center = LABEL_TOP + 6.0;
        assert!(
            (center - 14.0).abs() <= 1.0,
            "文字纵向中心应落在 C# 的 14（实得 {center}）"
        );
    }

    /// `ShowNotice` 语义：写文本 + 显示 + 10s 计时（C# `:68-78`）。
    #[test]
    fn show_sets_text_and_ten_second_timer() {
        let mut st = ChatNoticeState::default();
        assert!(!st.visible);
        st.show("全服公告");
        assert!(st.visible && st.text == "全服公告");
        assert_eq!(st.remaining, VIEW_TIME_SECS);
    }
}

// ============================================================================
// 帮助对话框（M50）
// 纯客户端对话框（无网络依赖）
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::keyboard_layout::{key_name, KeyboardState};
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiFont, UiImageCache,
};

/// 状态
#[derive(Resource, Default)]
pub struct HelpState {
    pub message: String,
    /// 当前页（C# HelpDialog CurrentPageNumber）
    pub page: usize,
}

#[derive(Component)]
pub struct HelpWidget;

#[derive(Component)]
pub struct HelpClose;

#[derive(Component)]
pub struct HelpLine(usize);

/// 上一页（C# PreviousButton Prguse2[240-242]）
#[derive(Component)]
pub struct HelpPrev;

/// 下一页（C# NextButton Prguse2[243-245]）
#[derive(Component)]
pub struct HelpNext;

/// 页标题（C# PageTitleLabel）
#[derive(Component)]
pub struct HelpTitleText;

/// 页码（C# PageLabel，格式 x / N）
#[derive(Component)]
pub struct HelpPageLabelText;

/// 生成帮助页（对齐 C# HelpDialog：快捷键 1/2 + 聊天快捷 + 操作说明）
/// 返回 (标题, 行列表)；快捷键页从 KeyboardState.bindings 动态读取（可重绑）
fn build_help_pages(state: &KeyboardState) -> Vec<(String, Vec<String>)> {
    let mut pages = Vec::new();
    // 快捷键页 1：移动/交互/界面（C# ShortcutPage1）
    let g1 = ["移动", "交互", "界面"];
    let p1: Vec<String> = state
        .bindings
        .iter()
        .filter(|b| g1.contains(&b.group))
        .map(|b| format!("{}：{}", b.action, key_name(b.key)))
        .collect();
    pages.push(("快捷键 1".to_string(), p1));
    // 快捷键页 2：系统/技能（C# ShortcutPage2）
    let p2: Vec<String> = state
        .bindings
        .iter()
        .filter(|b| b.group == "系统" || b.group == "技能")
        .map(|b| format!("{}：{}", b.action, key_name(b.key)))
        .collect();
    pages.push(("快捷键 2".to_string(), p2));
    // 聊天快捷（C# ShortcutPage3）
    pages.push((
        "聊天快捷".to_string(),
        vec![
            "/w 名字 内容：私聊".to_string(),
            "/g 内容：喊话".to_string(),
            "/guild 内容：行会".to_string(),
            "/group 内容：队伍".to_string(),
            "/friend 内容：好友".to_string(),
        ],
    ));
    // 操作说明
    pages.push((
        "操作说明".to_string(),
        vec![
            "F1-F8：施放绑定技能".to_string(),
            "左键：移动/选中/攻击".to_string(),
            "右键：使用/装备物品".to_string(),
            "中键：自动跑步".to_string(),
            "Tab：拾取地面物品".to_string(),
            "图文页（Help 库）暂缺".to_string(),
        ],
    ));
    pages
}

pub struct HelpPlugin;

impl Plugin for HelpPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HelpState>();
        app.add_systems(OnEnter(AppState::Game), spawn_help);
        app.add_systems(OnExit(AppState::Game), cleanup_help);
        app.add_systems(
            Update,
            (help_ui_system, ui_button_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_help(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_help(
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
            DialogRoot(DialogKind::Help),
            HelpWidget,
            Visibility::Hidden,
        ));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 360, 361, 362,
        280.0 + 300.0, 83.0, 7.0, 20.0, 20.0,
    ) {
        commands.entity(e).insert((
            HelpClose,
            DialogRoot(DialogKind::Help),
            HelpWidget,
        ));
    }
    // 页标题 / 页码（C# PageTitleLabel / PageLabel）
    let t = spawn_ui_text(
        &mut commands, &font, "",
        298.0, 96.0,
        13.0, Color::srgb(1.0, 0.9, 0.3), 8.0,
    );
    commands.entity(t).insert((HelpTitleText, DialogRoot(DialogKind::Help), HelpWidget));
    let t = spawn_ui_text(
        &mut commands, &font, "",
        480.0, 350.0,
        12.0, Color::WHITE, 8.0,
    );
    commands.entity(t).insert((HelpPageLabelText, DialogRoot(DialogKind::Help), HelpWidget));
    // 上一页/下一页（C# PreviousButton/NextButton Prguse2 240-242/243-245）
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 240, 241, 242,
        280.0 + 210.0, 80.0 + 485.0, 8.3, 16.0, 16.0,
    ) {
        commands.entity(e).insert((HelpPrev, DialogRoot(DialogKind::Help), HelpWidget));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 243, 244, 245,
        280.0 + 310.0, 80.0 + 485.0, 8.3, 16.0, 16.0,
    ) {
        commands.entity(e).insert((HelpNext, DialogRoot(DialogKind::Help), HelpWidget));
    }

    for i in 0..10usize {
        let e = spawn_ui_text(
            &mut commands, &font, "",
            298.0, 120.0 + i as f32 * 22.0,
            12.0, Color::WHITE, 8.0,
        );
        commands.entity(e).insert((
            HelpLine(i),
            DialogRoot(DialogKind::Help),
            HelpWidget,
        ));
    }
}

/// 显隐 + 渲染 + 关闭
/// 显隐 + 分页渲染 + 关闭
fn help_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut help: ResMut<HelpState>,
    keyboard: Res<KeyboardState>,
    close: Query<&UiButton, With<HelpClose>>,
    prev: Query<&UiButton, With<HelpPrev>>,
    next: Query<&UiButton, With<HelpNext>>,
    mut widgets: Query<&mut Visibility, With<HelpWidget>>,
    mut lines: Query<(&mut Text2d, &HelpLine)>,
    mut titles: Query<&mut Text2d, (With<HelpTitleText>, Without<HelpPageLabelText>)>,
    mut page_labels: Query<&mut Text2d, (With<HelpPageLabelText>, Without<HelpTitleText>)>,
) {
    let open = mgr.is_open(DialogKind::Help);
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        return;
    }
    for btn in &close {
        if btn.clicked {
            mgr.close(DialogKind::Help);
        }
    }
    let pages = build_help_pages(&keyboard);
    let total = pages.len();
    if help.page >= total {
        help.page = 0;
    }
    for btn in &prev {
        if btn.clicked {
            help.page = if help.page == 0 { total - 1 } else { help.page - 1 };
        }
    }
    for btn in &next {
        if btn.clicked {
            help.page = (help.page + 1) % total;
        }
    }
    let (title, page_lines) = &pages[help.page];
    for mut t in &mut titles {
        t.0 = format!("{}. {}", help.page + 1, title);
    }
    for mut t in &mut page_labels {
        t.0 = format!("{} / {}", help.page + 1, total);
    }
    for (mut text, line) in &mut lines {
        text.0 = page_lines.get(line.0).cloned().unwrap_or_default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_pages_build_from_keyboard() {
        let kb = KeyboardState::default();
        let pages = build_help_pages(&kb);
        assert!(pages.len() >= 4);
        assert!(!pages[0].1.is_empty(), "快捷键1 应有行");
        assert!(!pages[1].1.is_empty(), "快捷键2 应有行");
        // 快捷键行包含键名
        assert!(pages[0].1.iter().any(|l| l.contains("：" )));
    }
}

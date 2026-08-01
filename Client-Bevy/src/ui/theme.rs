// ============================================================================
// UI 共享工具（bevy_ui 原生 UI）
// ============================================================================

use bevy::prelude::*;

/// 编译期内嵌中文字体（Bevy 默认字体不支持中文）。
/// 用 include_bytes 保证任何启动目录/资产路径下都能加载。
pub fn load_cn_font(assets: &mut Assets<Font>) -> Handle<Font> {
    assets.add(Font::from_bytes(
        include_bytes!("../../assets/fonts/AlibabaPuHuiTi-3-55-Regular.ttf").to_vec(),
    ))
}

/// 常用配色（传奇 UI 风格）
pub mod colors {
    use bevy::prelude::*;

    pub const TITLE_GOLD: Color = Color::srgb(0.92, 0.80, 0.50);
    pub const PANEL_BG: Color = Color::srgb(0.12, 0.13, 0.18);
    pub const INPUT_BG: Color = Color::srgb(0.08, 0.09, 0.13);
    pub const BUTTON_BG: Color = Color::srgb(0.22, 0.18, 0.12);
    pub const BUTTON_HOVER: Color = Color::srgb(0.32, 0.26, 0.16);
    pub const BUTTON_PRESS: Color = Color::srgb(0.16, 0.13, 0.09);
    pub const TEXT: Color = Color::srgb(0.85, 0.83, 0.78);
    pub const GRAY: Color = Color::srgb(0.5, 0.5, 0.5);
}

/// 生成带文字的按钮
pub fn spawn_text_button(
    parent: &mut ChildSpawnerCommands,
    font: &FontSource,
    text: &str,
    font_size: f32,
    marker: impl Bundle,
) {
    parent
        .spawn((
            marker,
            Button,
            Node {
                width: Val::Px(200.0),
                height: Val::Px(38.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(colors::BUTTON_BG),
        ))
        .with_children(|b| {
            b.spawn((
                Text::new(text),
                TextFont {
                    font: font.clone(),
                    font_size: FontSize::Px(font_size),
                    ..default()
                },
                TextColor(colors::TEXT),
            ));
        });
}

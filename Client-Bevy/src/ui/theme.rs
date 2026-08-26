// ============================================================================
// UI 共享工具（bevy_ui 原生 UI）
// ============================================================================

use bevy::prelude::*;

use crate::map_renderer::GameLibraries;
use crate::resources::libraries::LibraryName;

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

/// 三帧图按钮（normal/hover/pressed），仿原版 Title.Lib 按钮帧
#[derive(Component)]
pub struct ImageButton {
    pub normal: Handle<Image>,
    pub hover: Handle<Image>,
    pub pressed: Handle<Image>,
}

/// 把某个 .Lib 图像加载成 Bevy Image 句柄
pub fn load_lib_image(
    libs: &mut GameLibraries,
    images: &mut Assets<Image>,
    name: LibraryName,
    index: usize,
) -> Option<Handle<Image>> {
    let info = libs.0.get_image(name, index)?;
    let rgba = info.rgba.clone()?;
    let w = info.width.max(0) as u32;
    let h = info.height.max(0) as u32;
    if w == 0 || h == 0 {
        return None;
    }
    Some(images.add(crate::map_renderer::make_image(rgba, w, h)))
}

/// 图按钮交互系统：根据 Interaction 切换三帧
pub fn image_button_system(
    mut q: Query<(&Interaction, &ImageButton, &mut ImageNode)>,
) {
    for (interaction, btn, mut node) in &mut q {
        let target = match interaction {
            Interaction::Pressed => &btn.pressed,
            Interaction::Hovered => &btn.hover,
            Interaction::None => &btn.normal,
        };
        if node.image != *target {
            node.image = target.clone();
        }
    }
}

// ============================================================================
// bevy_ui 迁移基座：绝对定位（Val::Px == UI 逻辑像素，左上角原点，y 向下），
// 与 Sprite UI 的 1024x768 逻辑坐标完全对齐（见 sprite_ui.rs spawn_ui_camera
// 的 IsDefaultUiCamera）。每个对话框 = 一个根面板 Node + 子节点（标签/图按钮）。
// ============================================================================

/// 绝对定位的基础 Node
fn abs_node(x: f32, y: f32, w: Option<f32>, h: Option<f32>) -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(x),
        top: Val::Px(y),
        width: w.map_or(Val::Auto, Val::Px),
        height: h.map_or(Val::Auto, Val::Px),
        ..default()
    }
}

/// 生成 .Lib 背景面板（bevy_ui Node + ImageNode）。返回根面板实体（DialogRoot 由调用方挂）。
pub fn spawn_panel(
    commands: &mut Commands,
    image: Handle<Image>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    z: i32,
) -> Entity {
    let mut node = abs_node(x, y, Some(w), Some(h));
    // 对话框内容默认裁剪到面板边界（对齐 C# 控件 ClipToParent；
    // 内容超过面板的列表/文本被裁掉，不再悬空到窗外）
    node.overflow = Overflow::clip();
    commands
        .spawn((
            node,
            ImageNode::new(image),
            GlobalZIndex(z),
            Visibility::Hidden,
        ))
        .id()
}

/// 子节点：绝对定位文本标签（相对父面板左上角）
pub fn spawn_label<'a>(
    parent: &'a mut ChildSpawnerCommands,
    font: &Handle<Font>,
    text: &str,
    x: f32,
    y: f32,
    size: f32,
    color: Color,
    z: i32,
) -> EntityCommands<'a> {
    parent.spawn((
        abs_node(x, y, None, None),
        Text::new(text),
        TextFont {
            font: FontSource::Handle(font.clone()),
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(color),
        ZIndex(z),
    ))
}

/// 子节点：绝对定位三帧图按钮（.Lib normal/hover/pressed）
pub fn spawn_icon_button<'a>(
    parent: &'a mut ChildSpawnerCommands,
    normal: Handle<Image>,
    hover: Handle<Image>,
    pressed: Handle<Image>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    z: i32,
) -> EntityCommands<'a> {
    parent.spawn((
        Button,
        abs_node(x, y, Some(w), Some(h)),
        ImageNode::new(normal.clone()),
        ImageButton { normal, hover, pressed },
        ZIndex(z),
    ))
}

/// 子节点：水平居中文本（cx=中心 x，width=排版宽度，Justify::Center）
pub fn spawn_label_center<'a>(
    parent: &'a mut ChildSpawnerCommands,
    font: &Handle<Font>,
    text: &str,
    cx: f32,
    y: f32,
    width: f32,
    size: f32,
    color: Color,
    z: i32,
) -> EntityCommands<'a> {
    parent.spawn((
        abs_node(cx - width / 2.0, y, Some(width), None),
        Text::new(text),
        TextFont {
            font: FontSource::Handle(font.clone()),
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(color),
        TextLayout::justify(Justify::Center),
        ZIndex(z),
    ))
}

/// 子节点：绝对定位图片（.Lib 图，动态换图用）
pub fn spawn_image<'a>(
    parent: &'a mut ChildSpawnerCommands,
    image: Handle<Image>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    z: i32,
) -> EntityCommands<'a> {
    parent.spawn((
        abs_node(x, y, Some(w), Some(h)),
        ImageNode::new(image),
        ZIndex(z),
    ))
}

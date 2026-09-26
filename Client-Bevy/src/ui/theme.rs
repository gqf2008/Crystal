// ============================================================================
// UI 共享工具（bevy_ui 原生 UI）
// ============================================================================

use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
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

/// 某帧的**原生尺寸**（= 原版 `MirImageControl` 不设 `Size` 时的控件尺寸）。
///
/// 为什么单列：原版大量控件只写 `Index`/`Library`/`Location`，尺寸**就是美术尺寸**；
/// 本端若在这些地方写死一个数字，很容易抄成另一张图的尺寸（实测三处标题图被拉成
/// `Title[15]` 的 103x17、技能页翻页钮被拉成 40x22）。要"跟美术一致"就得从图头取。
pub fn native_size(
    libs: &mut GameLibraries,
    name: LibraryName,
    index: usize,
) -> Option<(f32, f32)> {
    let info = libs.0.get_image(name, index)?;
    let w = info.width.max(0) as f32;
    let h = info.height.max(0) as f32;
    if w <= 0.0 || h <= 0.0 {
        return None;
    }
    Some((w, h))
}

/// 按**美术原生尺寸**铺一张图（`spawn_image` 的"原版语义"版本：不给尺寸，尺寸来自帧）。
/// 帧缺失/尺寸非法时返回 `None`，调用方不要退化成一个写死的尺寸——那正是要避免的漂移来源。
pub fn spawn_image_native<'a>(
    parent: &'a mut ChildSpawnerCommands,
    libs: &mut GameLibraries,
    images: &mut Assets<Image>,
    name: LibraryName,
    index: usize,
    x: f32,
    y: f32,
    z: i32,
) -> Option<Entity> {
    let (w, h) = native_size(libs, name, index)?;
    let handle = load_lib_image(libs, images, name, index)?;
    Some(spawn_image(parent, handle, x, y, w, h, z).id())
}

/// 图按钮交互系统：根据 Interaction 切换三帧
pub fn image_button_system(mut q: Query<(&Interaction, &ImageButton, &mut ImageNode)>) {
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

/// 面板根 Node 当前屏幕原点（left/top Px）。对话框拖动/推位后，内部固定坐标命中
/// 用「当前原点 + 相对坐标」跟随，避免停留在初始位置（bevy_ui 拖拽联动）。
pub fn node_origin(node: &Node, default: (f32, f32)) -> (f32, f32) {
    (
        match node.left {
            Val::Px(v) => v,
            _ => default.0,
        },
        match node.top {
            Val::Px(v) => v,
            _ => default.1,
        },
    )
}

/// UI 根面板的 `Visibility` → `Display` 桥接状态。
///
/// 根面板被业务系统设为 `Visibility::Hidden` 后，仍可能有显式
/// `Visibility::Visible` 的后代继续渲染；`Display::None` 才会真正隐藏整棵子树。
/// 这里记录关闭前的布局模式，根重新可见时恢复。
#[derive(Component, Default)]
pub(crate) struct UiRootDisplay {
    restore: Option<Display>,
}

/// 通用 UI 根显隐兜底：隐藏根切换为不渲染子树，重新可见时恢复原布局模式。
pub(crate) fn enforce_ui_root_display(
    mut roots: Query<(&Visibility, &mut Node, &mut UiRootDisplay)>,
) {
    for (vis, mut node, mut display) in &mut roots {
        if *vis == Visibility::Hidden {
            if display.restore.is_none() && node.display != Display::None {
                display.restore = Some(node.display);
            }
            node.display = Display::None;
        } else if let Some(restore) = display.restore.take() {
            node.display = restore;
        }
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
            UiRootDisplay::default(),
        ))
        .id()
}

/// 子节点：绝对定位文本标签（相对父面板左上角），**默认带 4 向黑色描边**。
///
/// #2817：C# `MirLabel` 构造器默认 `_outLine = true; _outLineColour = Color.Black`
/// （`Client/MirControls/MirLabel.cs:181-182`），按钮标题也吃这个默认
/// （`MirButton.cs:167-174` 里 `//OutLine = true,` 是被注释掉的冗余行）——
/// 全仓 `TextRenderer.DrawText` 只命中 `MirLabel.cs`，即 C# 文本默认全部带描边。
/// 显式关描边只有 4 处：物品格数量黄字（`MirItemCell.cs:2615`、`QuestDialogs.cs:1726`）、
/// 聊天文本（`MainDialogs.cs:962/1040`）；此外 `MirTextBox` 是原生 WinForms `TextBox`
/// （`MirTextBox.cs:143`）也不带描边。这些位置请用 [`spawn_label_plain`]。
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
    crate::ui::outlined_text::spawn_outlined_label(parent, font.clone(), text, x, y, size, color, z)
}

/// 子节点：绝对定位**定宽**文本标签（左上角锚点 + 指定行内对齐）。
///
/// C# `MirLabel` 的 `Location`(左上角) + `Size`(定宽) + `DrawFormat`(行内对齐) 三件套里，
/// [`spawn_label`] 只覆盖前两者且宽度自适应；需要行内对齐（尤其 `TextFormatFlags.Right`）
/// 的站点用本函数——对齐只有在**定宽**下才看得出效果。
pub fn spawn_outlined_label_block<'a>(
    parent: &'a mut ChildSpawnerCommands,
    font: &Handle<Font>,
    text: &str,
    x: f32,
    y: f32,
    width: f32,
    size: f32,
    color: Color,
    justify: Justify,
    z: i32,
) -> EntityCommands<'a> {
    crate::ui::outlined_text::spawn_outlined_label_block(
        parent, font, text, x, y, width, size, color, justify, z,
    )
}

/// 子节点：绝对定位**无描边**文本标签。
///
/// 只用于 C# 里确实没有描边的文本（见 [`spawn_label`] 注释的 4 处 `OutLine = false`
/// 与 `MirTextBox` 显示文本）。C# 证据写进调用点注释。
pub fn spawn_label_plain<'a>(
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
        ImageButton {
            normal,
            hover,
            pressed,
        },
        ZIndex(z),
    ))
}

/// 标准关闭钮标记（全 UI 交互验证：dialog_rect RPC 按它定位关闭钮中心）
#[derive(bevy::prelude::Component)]
pub struct CloseButton;

/// C# 标准关闭钮 `Prguse2[360..362]` 原生尺寸：各对话框均不设 `Size`
/// （如 `FriendDialog.cs:120-129`）→ 取 art 24x21。Bevy 侧曾统一自造 20x20，
/// `abs_node` 把 24x21 的图压进 20x20 节点。
pub const CLOSE_BTN_SIZE: (f32, f32) = (24.0, 21.0);

/// 子节点：C# 标准关闭钮 `Prguse2[360/361/362]`，节点尺寸跟随精灵原生尺寸
/// （[`CLOSE_BTN_SIZE`]），调用方只需给位置。帧缺失（资产未装）时返回 `None`。
pub fn spawn_close_button<'a>(
    parent: &'a mut ChildSpawnerCommands,
    libs: &mut GameLibraries,
    images: &mut Assets<Image>,
    x: f32,
    y: f32,
    z: i32,
) -> Option<EntityCommands<'a>> {
    let (Some(n), Some(h), Some(pr)) = (
        load_lib_image(libs, images, LibraryName::Prguse2, 360),
        load_lib_image(libs, images, LibraryName::Prguse2, 361),
        load_lib_image(libs, images, LibraryName::Prguse2, 362),
    ) else {
        return None;
    };
    let mut ec = spawn_icon_button(
        parent,
        n,
        h,
        pr,
        x,
        y,
        CLOSE_BTN_SIZE.0,
        CLOSE_BTN_SIZE.1,
        z,
    );
    ec.insert(CloseButton);
    Some(ec)
}

/// 子节点：水平居中文本（cx=中心 x，width=排版宽度，`Justify::Center`），**默认带描边**
/// （理由同 [`spawn_label`]）。
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
    crate::ui::outlined_text::spawn_outlined_label_center(
        parent, font, text, cx, y, width, size, color, z,
    )
}

/// 子节点：水平居中的**无描边**文本（例外清单同 [`spawn_label_plain`]）
pub fn spawn_label_center_plain<'a>(
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

/// 子节点：绝对定位空白容器（供页面/槽位/行等承载子元素）
pub fn spawn_container<'a>(
    parent: &'a mut ChildSpawnerCommands,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    z: i32,
) -> EntityCommands<'a> {
    parent.spawn((abs_node(x, y, Some(w), Some(h)), ZIndex(z)))
}

// ============================================================================
// bevy_ui 下拉框（C# MirDropDownBox 简化版）
// 闭合框 = Button（Node+BackgroundColor）+ 文本 + ▼；弹出面板 = 父面板子节点
// （绝对定位 Node）。Interaction 在 PreUpdate 刷新（UiSystems::Focus），本系统
// 在 Update 读取 → 边沿触发用 Local HashMap。
// ============================================================================

/// bevy_ui 下拉框状态（挂在闭合框实体上）
#[derive(Component)]
pub struct UiDropDown {
    pub items: Vec<String>,
    pub selected: Option<usize>,
    pub open: bool,
    /// 弹出面板实体（隐藏/显示）
    pub popup: Entity,
    /// 闭合框选中文字实体
    pub text: Entity,
    /// 闭合框矩形（屏幕坐标，点击外部关闭用）
    pub box_rect: (f32, f32, f32, f32),
    /// 选项行按钮实体（Interaction 命中）
    pub option_rows: Vec<Entity>,
    /// 选项行文字实体（最多 popup_rows 个）
    pub option_texts: Vec<Entity>,
    /// 弹出面板左上角（屏幕坐标）
    pub popup_pos: (f32, f32),
    /// 弹出面板宽度/行高/可视行数
    pub popup_w: f32,
    pub row_h: f32,
    pub popup_rows: usize,
    /// 滚动偏移
    pub scroll: usize,
    /// #2892 批D 单元①：弹出面板的**基准相对坐标**（父面板坐标系，C# `MirDropDownBox.Movable`）
    pub base_rel: (f32, f32),
    /// 拖动偏移（相对基准位置；C# `OnMouseMove` 改 `Location`）
    pub drag_offset: (f32, f32),
    /// 正在拖动时的抓取点（光标 − 弹出面板左上角，屏幕坐标）
    pub drag_grab: Option<(f32, f32)>,
}

/// 弹出面板标记
#[derive(Component)]
pub struct UiDropDownPopup;

/// 生成 bevy_ui 下拉框。origin = 父面板屏幕坐标左上角；x/y/w/h = 相对父面板的闭合框矩形。
/// 返回闭合框实体（调用方可插 marker/交互逻辑）。
pub fn spawn_dropdown_ui<'a>(
    parent: &'a mut ChildSpawnerCommands,
    font: &Handle<Font>,
    items: Vec<String>,
    selected: Option<usize>,
    origin: (f32, f32),
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    popup_rows: usize,
    z: i32,
) -> EntityCommands<'a> {
    let row_h = h;
    let popup_w = w + 4.0;
    let selected_text = items
        .get(selected.unwrap_or(usize::MAX))
        .cloned()
        .unwrap_or_default();

    // 闭合框：深色底 + 选中文字 + ▼（with_children 链式消费，避免长期持有 EntityCommands）
    let mut text_e = Entity::PLACEHOLDER;
    let box_e = parent
        .spawn((
            Button,
            abs_node(x, y, Some(w), Some(h)),
            BackgroundColor(Color::srgba(0.05, 0.05, 0.08, 0.95)),
            ZIndex(z),
        ))
        .with_children(|b| {
            text_e = b
                .spawn((
                    abs_node(6.0, (h - 12.0) / 2.0, None, None),
                    Text::new(selected_text),
                    TextFont {
                        font: FontSource::Handle(font.clone()),
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    ZIndex(1),
                ))
                .id();
            b.spawn((
                abs_node(w - 14.0, (h - 12.0) / 2.0, None, None),
                Text::new("▼"),
                TextFont {
                    font: FontSource::Handle(font.clone()),
                    font_size: FontSize::Px(10.0),
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.8, 0.8)),
                ZIndex(1),
            ));
        })
        .id();

    // 弹出面板：父面板子节点，绝对定位覆盖在内容之上，默认隐藏。
    // 选项行由下方 popup_cmds.with_children 统一 spawn 并收集 id。
    let popup = parent
        .spawn((
            abs_node(
                x - 2.0,
                y + h,
                Some(popup_w),
                Some(row_h * popup_rows as f32 + 2.0),
            ),
            BackgroundColor(Color::srgba(0.08, 0.08, 0.12, 0.98)),
            UiDropDownPopup,
            Visibility::Hidden,
            ZIndex(z + 3),
        ))
        .id();
    let mut option_rows = Vec::new();
    let mut option_texts = Vec::new();
    let mut popup_cmds = parent.commands_mut().entity(popup);
    popup_cmds.with_children(|op| {
        for i in 0..popup_rows {
            option_rows.push(
                op.spawn((
                    Button,
                    abs_node(4.0, 2.0 + i as f32 * row_h, Some(w), Some(h)),
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
                    ZIndex(1),
                ))
                .id(),
            );
            option_texts.push(
                op.spawn((
                    abs_node(6.0, 4.0 + i as f32 * row_h, None, None),
                    Text::new(String::new()),
                    TextFont {
                        font: FontSource::Handle(font.clone()),
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    ZIndex(2),
                ))
                .id(),
            );
        }
    });

    let mut cmds = parent.commands_mut();
    cmds.entity(box_e).insert(UiDropDown {
        items,
        selected,
        open: false,
        popup,
        text: text_e,
        box_rect: (origin.0 + x, origin.1 + y, w, h),
        option_rows,
        option_texts,
        popup_pos: (origin.0 + x - 2.0, origin.1 + y + h),
        popup_w,
        row_h,
        popup_rows,
        scroll: 0,
        base_rel: (x - 2.0, y + h),
        drag_offset: (0.0, 0.0),
        drag_grab: None,
    });
    cmds.entity(box_e)
}

/// bevy_ui 下拉框系统：展开/收起/选择/滚轮/点击外部关闭
/// 弹出面板矩形（屏幕坐标，含拖动偏移）——C# `MirDropDownBox.Movable` 拖动后命中也要跟着走
fn popup_rect(dd: &UiDropDown) -> (f32, f32, f32, f32) {
    (
        dd.popup_pos.0 + dd.drag_offset.0,
        dd.popup_pos.1 + dd.drag_offset.1,
        dd.popup_w,
        dd.row_h * dd.popup_rows as f32,
    )
}

pub fn dropdown_ui_system(
    mut dd_q: Query<(Entity, &Interaction, &mut UiDropDown)>,
    options: Query<&Interaction, Without<UiDropDown>>,
    mut texts: Query<&mut Text>,
    // #2892 批D 单元①：弹出面板还要能拖动（C# `MirDropDownBox.Movable = true`）→ 需要 Node
    mut popups: Query<(&mut Visibility, &mut Node), With<UiDropDownPopup>>,
    mut wheels: MessageReader<MouseWheel>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
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
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };

    // 1. 闭合框点击 → 切换展开（Interaction 边沿）
    for (e, inter, mut dd) in &mut dd_q {
        if edge(e, inter, &mut prev_inter) {
            dd.open = !dd.open;
            dd.scroll = 0;
            // C# `Show()` 时把弹出面板放回闭合框下方：重新打开时清掉上次的拖动偏移
            if dd.open {
                dd.drag_offset = (0.0, 0.0);
                dd.drag_grab = None;
            }
        }
    }

    // 1.5 弹出面板拖动（C# `MirControl.Movable`：按下**非选项行**区域才起拖，选项行仍是选择）
    {
        let mut any_row_pressed = false;
        for (_, _, dd) in dd_q.iter() {
            if !dd.open {
                continue;
            }
            if dd.option_rows.iter().any(|ent| {
                options
                    .get(*ent)
                    .map(|i| *i == Interaction::Pressed)
                    .unwrap_or(false)
            }) {
                any_row_pressed = true;
                break;
            }
        }
        if mouse.just_pressed(MouseButton::Left) && !any_row_pressed {
            for (_, _, mut dd) in dd_q.iter_mut() {
                if !dd.open {
                    continue;
                }
                let (px, py, pw, ph) = popup_rect(&dd);
                if cursor.x >= px && cursor.x <= px + pw && cursor.y >= py && cursor.y <= py + ph {
                    dd.drag_grab = Some((cursor.x - px, cursor.y - py));
                    break;
                }
            }
        }
        for (_, _, mut dd) in dd_q.iter_mut() {
            let Some(grab) = dd.drag_grab else {
                continue;
            };
            if mouse.pressed(MouseButton::Left) {
                let (bx, by) = dd.popup_pos;
                dd.drag_offset = (cursor.x - grab.0 - bx, cursor.y - grab.1 - by);
            } else {
                dd.drag_grab = None;
            }
        }
    }

    // 2. 滚轮：光标在弹出面板内 → 滚动选项
    let mut scroll_y = 0.0f32;
    for ev in wheels.read() {
        match ev.unit {
            MouseScrollUnit::Line => scroll_y += ev.y,
            MouseScrollUnit::Pixel => scroll_y += ev.y / 20.0,
        }
    }
    if scroll_y.abs() > 0.0 {
        for (_, _, mut dd) in dd_q.iter_mut() {
            if !dd.open {
                continue;
            }
            let (px, py, pw, ph) = (
                dd.popup_pos.0 + dd.drag_offset.0,
                dd.popup_pos.1 + dd.drag_offset.1,
                dd.popup_w,
                dd.row_h * dd.popup_rows as f32,
            );
            if cursor.x >= px && cursor.x <= px + pw && cursor.y >= py && cursor.y <= py + ph {
                let max = dd.items.len().saturating_sub(dd.popup_rows);
                dd.scroll =
                    (dd.scroll as i32 + scroll_y.round() as i32).clamp(0, max as i32) as usize;
                break;
            }
        }
    }

    // 3. 点击选项选中 / 点击外部关闭
    if mouse.just_pressed(MouseButton::Left) {
        for (_, _, mut dd) in dd_q.iter_mut() {
            if !dd.open {
                continue;
            }
            let (px, py, pw, ph) = (
                dd.popup_pos.0 + dd.drag_offset.0,
                dd.popup_pos.1 + dd.drag_offset.1,
                dd.popup_w,
                dd.row_h * dd.popup_rows as f32,
            );
            let in_popup =
                cursor.x >= px && cursor.x <= px + pw && cursor.y >= py && cursor.y <= py + ph;
            if in_popup {
                for (i, ent) in dd.option_rows.iter().enumerate() {
                    if let Ok(inter) = options.get(*ent) {
                        // 选中当前行
                        if *inter == Interaction::Pressed {
                            let idx = dd.scroll + i;
                            if idx < dd.items.len() {
                                dd.selected = Some(idx);
                            }
                            dd.open = false;
                            break;
                        }
                    }
                }
            } else {
                // 点不在面板内：若也不在闭合框内则关闭（闭合框内由第 1 步切换）
                let (bx, by, bw, bh) = dd.box_rect;
                let in_box =
                    cursor.x >= bx && cursor.x <= bx + bw && cursor.y >= by && cursor.y <= by + bh;
                if !in_box {
                    dd.open = false;
                }
            }
        }
    }

    // 4. 刷新：面板显隐、选项文字、选中文字
    for (_, _, dd) in dd_q.iter() {
        let sel_text = dd
            .items
            .get(dd.selected.unwrap_or(usize::MAX))
            .cloned()
            .unwrap_or_default();
        if let Ok(mut t) = texts.get_mut(dd.text) {
            if t.0 != sel_text {
                t.0 = sel_text;
            }
        }
        if let Ok((mut v, mut node)) = popups.get_mut(dd.popup) {
            *v = if dd.open {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
            // #2892 批D 单元①：弹出面板位置 = 基准相对坐标 + 拖动偏移（C# `Location`）
            let want_left = Val::Px(dd.base_rel.0 + dd.drag_offset.0);
            let want_top = Val::Px(dd.base_rel.1 + dd.drag_offset.1);
            if node.left != want_left {
                node.left = want_left;
            }
            if node.top != want_top {
                node.top = want_top;
            }
        }
        if dd.open {
            for (i, ent) in dd.option_texts.iter().enumerate() {
                let idx = dd.scroll + i;
                let s = dd.items.get(idx).cloned().unwrap_or_default();
                if let Ok(mut t) = texts.get_mut(*ent) {
                    if t.0 != s {
                        t.0 = s;
                    }
                }
            }
        }
    }
}

// ============================================================================
// bevy_ui 动画按钮（C# MirAnimatedButton：Index 起始帧轮播 + hover/pressed 状态帧）
// ============================================================================

/// 动画按钮状态（挂在闭合框实体上）
#[derive(Component)]
pub struct UiAnimatedButton {
    /// 轮播帧（Index 起始帧起 count 帧）
    pub frames: Vec<Handle<Image>>,
    /// 悬停帧（可选）
    pub hover: Option<Handle<Image>>,
    /// 按下帧（可选）
    pub pressed: Option<Handle<Image>>,
    /// 当前轮播帧下标
    pub frame: usize,
    /// 每帧间隔（秒）
    pub delay: f32,
    pub timer: f32,
    pub looping: bool,
    pub playing: bool,
}

/// 生成 bevy_ui 动画按钮（Button + ImageNode + UiAnimatedButton）
pub fn spawn_animated_icon_button<'a>(
    parent: &'a mut ChildSpawnerCommands,
    frames: Vec<Handle<Image>>,
    hover: Option<Handle<Image>>,
    pressed: Option<Handle<Image>>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    z: i32,
    delay: f32,
    looping: bool,
) -> EntityCommands<'a> {
    // （批次19-23 评审 F2）delay 钳位下限：轮播 while 里 `timer -= delay`，delay<=0
    // 时 timer 永不下降 → 单帧死循环（整帧卡死）。空帧会让 `frames[0]` 越界崩溃，
    // 这里语义化断言（调用点 fishing.rs 已有 !frames.is_empty() 守卫，此为防未来调用者）。
    let delay = delay.max(0.01);
    assert!(
        !frames.is_empty(),
        "spawn_animated_icon_button 需要至少 1 帧"
    );
    parent.spawn((
        Button,
        abs_node(x, y, Some(w), Some(h)),
        ImageNode::new(frames[0].clone()),
        UiAnimatedButton {
            frames,
            hover,
            pressed,
            frame: 0,
            delay,
            timer: 0.0,
            looping,
            playing: true,
        },
        ZIndex(z),
    ))
}

/// 动画按钮系统：时间步进轮播 + 状态帧（按下 > 悬停 > 轮播帧）
pub fn animated_button_ui_system(
    mut btns: Query<(&Interaction, &mut UiAnimatedButton, &mut ImageNode)>,
    time: Res<Time>,
) {
    for (inter, mut ab, mut node) in &mut btns {
        if ab.playing {
            ab.timer += time.delta_secs();
            while ab.timer >= ab.delay && !ab.frames.is_empty() {
                ab.timer -= ab.delay;
                if ab.looping {
                    ab.frame = (ab.frame + 1) % ab.frames.len();
                } else {
                    ab.frame = (ab.frame + 1).min(ab.frames.len() - 1);
                }
            }
        }
        let target = match inter {
            Interaction::Pressed => ab.pressed.as_ref().or_else(|| ab.frames.get(ab.frame)),
            Interaction::Hovered => ab.hover.as_ref().or_else(|| ab.frames.get(ab.frame)),
            Interaction::None => ab.frames.get(ab.frame),
        };
        if let Some(h) = target {
            if node.image != *h {
                node.image = h.clone();
            }
        }
    }
}

// ============================================================================
// bevy_ui 物品格（C# MirItemCell）
// 结构：格子实体（UiItemCell{slot} + UiItemCellData）→ 子实体
// UiItemCellIcon/UiItemCellCount/UiItemCellDura；渲染由 item_cell_ui_system
// 统一处理；对话框只需写 UiItemCellData。
// ============================================================================

/// 物品格数据（对话框每帧写入）
#[derive(Component, Default, Clone)]
pub struct UiItemCellData {
    /// 物品图标（Items 库图柄）
    pub icon: Option<Handle<Image>>,
    /// 堆叠数量（None 或 1 不显示数字）
    pub count: Option<u32>,
    /// 耐久比例 0.0-1.0（装备显示耐久条；None 不显示，C# MirItemCell DrawDurability）
    pub dura_ratio: Option<f32>,
}

/// 物品格（槽位）
#[derive(Component)]
pub struct UiItemCell {
    pub slot: usize,
}

/// 物品图标子实体
#[derive(Component)]
pub struct UiItemCellIcon(pub usize);

/// 堆叠数量子实体
#[derive(Component)]
pub struct UiItemCellCount(pub usize);

/// 耐久条子实体（装备显示，红色随耐久缩短）
#[derive(Component)]
pub struct UiItemCellDura(pub usize, pub f32);

/// 耐久条宽度（C# MirItemCell DrawDurability：满耐久=整格宽度，随比例缩短，最小 1px）
fn item_cell_dura_width(full: f32, ratio: f32) -> f32 {
    (full * ratio).max(1.0)
}

/// 生成 bevy_ui 通用物品格（底格 + 图标 + 数量 + 耐久条），返回格子实体
pub fn spawn_item_cell_ui<'a>(
    parent: &'a mut ChildSpawnerCommands,
    images: &mut Assets<Image>,
    font: &Handle<Font>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    z: i32,
    slot: usize,
) -> EntityCommands<'a> {
    let white = images.add(crate::map_renderer::make_image(
        vec![255, 255, 255, 255],
        1,
        1,
    ));
    let mut cmds = parent.spawn((
        abs_node(x, y, Some(w), Some(h)),
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.18)),
        UiItemCell { slot },
        UiItemCellData::default(),
        ZIndex(z),
    ));
    cmds.with_children(|p| {
        // 物品图标（白图占位，系统换物品图）
        p.spawn((
            abs_node(2.0, 2.0, Some(w - 4.0), Some(h - 4.0)),
            ImageNode::new(white.clone()),
            UiItemCellIcon(slot),
            Visibility::Hidden,
            ZIndex(z + 1),
        ));
        // 堆叠数量（右下角）
        p.spawn((
            abs_node(w - 16.0, h - 13.0, None, None),
            Text::new(String::new()),
            TextFont {
                font: FontSource::Handle(font.clone()),
                font_size: FontSize::Px(10.0),
                ..default()
            },
            TextColor(Color::srgb(1.0, 1.0, 0.6)),
            UiItemCellCount(slot),
            Visibility::Hidden,
            ZIndex(z + 2),
        ));
        // 耐久条（C# MirItemCell DrawDurability：红色随耐久缩短）
        p.spawn((
            abs_node(2.0, h - 4.0, Some(w - 4.0), Some(2.0)),
            BackgroundColor(Color::srgb(1.0, 0.2, 0.2)),
            UiItemCellDura(slot, w - 4.0),
            Visibility::Hidden,
            ZIndex(z + 3),
        ));
    });
    cmds
}

/// bevy_ui 物品格渲染系统：按 UiItemCellData 刷新图标/数量/耐久条
pub fn item_cell_ui_system(
    cells: Query<
        &UiItemCellData,
        (
            With<UiItemCell>,
            Without<UiItemCellIcon>,
            Without<UiItemCellCount>,
            Without<UiItemCellDura>,
        ),
    >,
    mut icons: Query<
        (&ChildOf, &mut ImageNode, &mut Visibility, &UiItemCellIcon),
        (Without<UiItemCellCount>, Without<UiItemCellDura>),
    >,
    mut counts: Query<
        (&ChildOf, &mut Text, &mut Visibility, &UiItemCellCount),
        (Without<UiItemCellIcon>, Without<UiItemCellDura>),
    >,
    mut duras: Query<
        (&ChildOf, &mut Node, &mut Visibility, &UiItemCellDura),
        (Without<UiItemCellIcon>, Without<UiItemCellCount>),
    >,
) {
    for (child_of, mut node, mut vis, _icon) in &mut icons {
        let data = cells.get(child_of.parent()).ok();
        if let Some(h) = data.and_then(|d| d.icon.clone()) {
            if node.image != h {
                node.image = h;
            }
        }
        let show = data
            .and_then(|d| d.icon.as_ref())
            .is_some_and(|h| h.is_strong());
        *vis = if show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for (child_of, mut text, mut vis, _count) in &mut counts {
        let data = cells.get(child_of.parent()).ok();
        let s = data
            .and_then(|d| d.count)
            .filter(|n| *n > 1)
            .map(|n| n.to_string())
            .unwrap_or_default();
        let show = !s.is_empty();
        if text.0 != s {
            text.0 = s;
        }
        *vis = if show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for (child_of, mut node, mut vis, dura) in &mut duras {
        let data = cells.get(child_of.parent()).ok();
        match data.and_then(|d| d.dura_ratio) {
            Some(ratio) if (0.0..=1.0).contains(&ratio) => {
                node.width = Val::Px(item_cell_dura_width(dura.1, ratio));
                *vis = Visibility::Visible;
            }
            _ => *vis = Visibility::Hidden,
        }
    }
}

#[cfg(test)]
mod tests {
    /// #2892 批D 单元①：下拉框弹出面板的命中矩形必须带上拖动偏移
    /// （C# `MirDropDownBox.Movable = true`：拖走后选项行/滚轮/点击外部关闭都要跟着走）。
    ///
    /// 阳性对照：把 `popup_rect` 改成只用 `popup_pos`（忽略偏移）→ 本测试 FAILED。
    #[test]
    fn dropdown_popup_rect_includes_drag_offset() {
        let mut dd = UiDropDown {
            items: vec!["a".to_string(), "b".to_string()],
            selected: None,
            open: true,
            popup: Entity::PLACEHOLDER,
            text: Entity::PLACEHOLDER,
            box_rect: (0.0, 0.0, 10.0, 10.0),
            option_rows: Vec::new(),
            option_texts: Vec::new(),
            popup_pos: (100.0, 50.0),
            popup_w: 80.0,
            row_h: 14.0,
            popup_rows: 4,
            scroll: 0,
            base_rel: (10.0, 5.0),
            drag_offset: (0.0, 0.0),
            drag_grab: None,
        };
        // 未拖动：与基准位置一致
        assert_eq!(popup_rect(&dd), (100.0, 50.0, 80.0, 56.0));
        // 拖动后：命中矩形整体平移（宽高不变）
        dd.drag_offset = (12.0, -8.0);
        assert_eq!(popup_rect(&dd), (112.0, 42.0, 80.0, 56.0));
        // 节点位置 = 基准相对坐标 + 偏移（`base_rel` 是父面板坐标系）
        assert_eq!(
            (
                dd.base_rel.0 + dd.drag_offset.0,
                dd.base_rel.1 + dd.drag_offset.1
            ),
            (22.0, -3.0)
        );
    }
    use super::*;
    use bevy::ecs::world::CommandQueue;

    /// 面板根 Node 当前原点：拖动/推位后命中跟随用
    #[test]
    fn node_origin_reads_px_with_default_fallback() {
        let node = Node {
            position_type: PositionType::Absolute,
            left: Val::Px(393.0),
            top: Val::Px(50.0),
            ..default()
        };
        assert_eq!(node_origin(&node, (0.0, 0.0)), (393.0, 50.0));
        // 非 Px 字段回退默认（如 Auto 布局节点）
        assert_eq!(node_origin(&Node::default(), (280.0, 80.0)), (280.0, 80.0));
    }

    /// `spawn_panel` 必须自动挂 `UiRootDisplay`，否则非 DialogRoot 面板
    /// （如 AssignKeyPanel）关闭时仍可能漏出显式 Visible 子控件。
    #[test]
    fn spawn_panel_marks_ui_root_display() {
        let mut world = World::new();
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);
        let panel = spawn_panel(&mut commands, Handle::default(), 0.0, 0.0, 10.0, 10.0, 1);
        queue.apply(&mut world);
        assert!(world.get::<UiRootDisplay>(panel).is_some());
    }

    /// 耐久条宽度：满耐久=整格，随比例缩短，最小 1px（C# MirItemCell DrawDurability）
    #[test]
    fn spawn_label_is_outlined_by_default() {
        // #2817：C# `MirLabel` 构造器默认 `_outLine = true`（`MirLabel.cs:181-182`），
        // 对话框文本默认必须带描边 → `spawn_label`/`spawn_label_center` 等价于 outlined 版；
        // 例外位置（物品格数量黄字 `OutLine = false`、`InputTextBox` 文本）走 `*_plain`
        let mut world = World::new();
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);
        let mut main = None;
        commands.spawn(Node::default()).with_children(|p| {
            main =
                Some(spawn_label(p, &Handle::default(), "T", 1.0, 2.0, 12.0, Color::WHITE, 5).id());
            spawn_label_plain(p, &Handle::default(), "1", 3.0, 4.0, 12.0, Color::WHITE, 5);
            spawn_label_center(
                p,
                &Handle::default(),
                "C",
                50.0,
                6.0,
                40.0,
                12.0,
                Color::WHITE,
                5,
            );
            spawn_label_center_plain(
                p,
                &Handle::default(),
                "P",
                50.0,
                8.0,
                40.0,
                12.0,
                Color::WHITE,
                5,
            );
        });
        queue.apply(&mut world);
        assert!(
            world
                .entity(main.unwrap())
                .contains::<crate::ui::outlined_text::OutlinedUiText>(),
            "spawn_label 默认应带描边"
        );
        let shadows = world
            .query_filtered::<Entity, With<crate::ui::outlined_text::OutlineUiShadow>>()
            .iter(&world)
            .count();
        let outlined = world
            .query_filtered::<Entity, With<crate::ui::outlined_text::OutlinedUiText>>()
            .iter(&world)
            .count();
        assert_eq!(shadows, 8, "两个带描边主体各 4 个黑副本");
        assert_eq!(outlined, 2, "只有 label / label_center 两个主体带描边");
    }

    /// 耐久条宽度：满耐久=整格，随比例缩短，最小 1px（C# MirItemCell DrawDurability）
    #[test]
    fn item_cell_dura_width_clamps() {
        assert_eq!(item_cell_dura_width(60.0, 1.0), 60.0);
        assert_eq!(item_cell_dura_width(60.0, 0.5), 30.0);
        assert_eq!(item_cell_dura_width(60.0, 0.0), 1.0);
        assert_eq!(item_cell_dura_width(60.0, 0.01), 1.0);
    }

    /// 批次19-23 评审 F2：delay<=0 会让轮播 while 永不推进（单帧死循环）→ 钳位下限
    #[test]
    fn animated_button_delay_clamped_to_minimum() {
        let mut world = World::new();
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);
        // 非法值：调用方漏传/误传
        commands.spawn_empty().with_children(|p| {
            spawn_animated_icon_button(
                p,
                vec![Handle::default()],
                None,
                None,
                0.0,
                0.0,
                10.0,
                10.0,
                0,
                0.0,
                true,
            );
        });
        // 合法正值原样保留（不过度钳位）
        commands.spawn_empty().with_children(|p| {
            spawn_animated_icon_button(
                p,
                vec![Handle::default()],
                None,
                None,
                0.0,
                0.0,
                10.0,
                10.0,
                0,
                0.13,
                true,
            );
        });
        queue.apply(&mut world);

        let mut delays = world
            .query_filtered::<&UiAnimatedButton, ()>()
            .iter(&world)
            .map(|ab| ab.delay)
            .collect::<Vec<_>>();
        delays.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(delays, vec![0.01, 0.13]);
    }

    /// 批次19-23 评审 F2：空帧会让 `frames[0]` 越界崩溃 → 语义化断言（fail fast）
    #[test]
    #[should_panic(expected = "需要至少 1 帧")]
    fn animated_button_rejects_empty_frames() {
        let mut world = World::new();
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);
        commands.spawn_empty().with_children(|p| {
            spawn_animated_icon_button(
                p,
                Vec::new(),
                None,
                None,
                0.0,
                0.0,
                10.0,
                10.0,
                0,
                0.13,
                true,
            );
        });
        queue.apply(&mut world);
    }

    /// #2961 项5 验收能力：滚轮命中可经 `CursorProbe` 驱动——无焦点/共享桌面上
    /// `Window::cursor_position()` 恒为 None，滚动**没法被自动化驱动**，本项就只能
    /// 靠"截图人工看"。修复前系统只读真实光标，故本测试的 (a) 段 FAILED。
    /// (b) 段是内建负控：探针为 None 且窗口无光标 → 不得滚动（证明开关是探针，
    /// 而不是把命中判定整个放宽了）。
    #[test]
    fn scroll_list_wheel_hits_via_cursor_probe() {
        use bevy::ecs::message::Messages;
        use bevy::ecs::system::RunSystemOnce;

        let build = |probe: Option<Vec2>, win_cursor: Option<Vec2>| {
            let mut world = World::new();
            world.init_resource::<Messages<MouseWheel>>();
            world.init_resource::<ButtonInput<MouseButton>>();
            world.init_resource::<crate::ui::scroll_list::ScrollDrag>();
            world.insert_resource(crate::control::CursorProbe { pos: probe });
            let e = world
                .spawn((
                    abs_node(0.0, 0.0, Some(200.0), Some(200.0)),
                    InheritedVisibility::VISIBLE,
                    UiScrollList {
                        rect_rel: (0.0, 0.0, 50.0, 50.0),
                        row_h: 10.0,
                        visible: 5,
                        total: 20,
                        offset: 0,
                        step: 1,
                        track_rel: (50.0, 0.0, 10.0, 50.0),
                        thumb: None,
                        z: 1,
                    },
                ))
                .id();
            // `win_cursor = None`：模拟无焦点/共享桌面（真实光标不可用）
            let mut window = Window::default();
            window.set_cursor_position(win_cursor);
            let win = world.spawn(window).id();
            world.write_message(MouseWheel {
                unit: MouseScrollUnit::Line,
                x: 0.0,
                y: 1.0,
                window: win,
                phase: bevy::input::touch::TouchPhase::Moved,
            });
            world.run_system_once(scroll_list_ui_system).unwrap();
            world.get::<UiScrollList>(e).unwrap().offset
        };

        // (a) 探针指向列表可视区 → 滚动生效
        assert_eq!(
            build(Some(Vec2::new(10.0, 10.0)), None),
            1,
            "探针注入的光标应能驱动滚轮"
        );
        // (b) 负控：无探针且窗口无光标 → 不该滚动
        assert_eq!(build(None, None), 0, "无任何光标来源时不得滚动");
        // (c) #2978 审查 P2：**优先级**必须钉住——探针优先于真实光标。
        // 若日后写成 `window.cursor_position().or(probe.pos)`，(a)(b) 仍会全绿，但在
        // 有真实光标（共享桌面/脚本跑一半有人动鼠标）的机器上注入会被真实光标盖掉，
        // 表现为"脚本偶尔说没滚动"的假阴性。窗口光标故意设在列表**外**。
        assert_eq!(
            build(Some(Vec2::new(10.0, 10.0)), Some(Vec2::new(180.0, 180.0))),
            1,
            "探针优先于真实光标：窗口光标在列表外时也必须按探针命中"
        );
    }

    /// #2968 审查阻塞项回归：**隐藏列表不得吞滚轮**——行会成员页与仓库页两个
    /// UiScrollList 屏幕区域重叠、z 相同时，靠可见性区分（C# 按页分发：隐藏页
    /// 收不到 MouseWheel）。阳性对照：去掉 `shown()` 门控 → z 更高的隐藏列表
    /// 会被选中，本测试 FAILED。
    #[test]
    fn scroll_list_hidden_list_never_takes_wheel() {
        use bevy::ecs::message::Messages;
        use bevy::ecs::system::RunSystemOnce;

        let mut world = World::new();
        world.init_resource::<Messages<MouseWheel>>();
        // 滚轮命中改读注入探针（#2961 项5）：本测试走**真实光标**路径，
        // 故探针保持 None（probe 为 None 时回落 window.cursor_position）
        world.init_resource::<crate::control::CursorProbe>();
        world.init_resource::<ButtonInput<MouseButton>>();
        world.init_resource::<crate::ui::scroll_list::ScrollDrag>();

        let list = |z: i32| UiScrollList {
            rect_rel: (0.0, 0.0, 50.0, 50.0),
            row_h: 10.0,
            visible: 5,
            total: 20,
            offset: 0,
            step: 1,
            track_rel: (50.0, 0.0, 10.0, 50.0),
            thumb: None,
            z,
        };
        // 隐藏列表 z 更高（若不跳过可见性，命中优先选它）
        let hidden = world
            .spawn((
                abs_node(0.0, 0.0, Some(200.0), Some(200.0)),
                InheritedVisibility::HIDDEN,
                list(9),
            ))
            .id();
        let shown = world
            .spawn((
                abs_node(0.0, 0.0, Some(200.0), Some(200.0)),
                InheritedVisibility::VISIBLE,
                list(1),
            ))
            .id();

        let mut window = Window::default();
        window.set_cursor_position(Some(Vec2::new(10.0, 10.0)));
        let win = world.spawn(window).id();
        world.write_message(MouseWheel {
            unit: MouseScrollUnit::Line,
            x: 0.0,
            y: 1.0,
            window: win,
            phase: bevy::input::touch::TouchPhase::Moved,
        });
        world
            .run_system_once(scroll_list_ui_system)
            .expect("滚动系统应可运行");
        assert_eq!(
            world.get::<UiScrollList>(hidden).map(|l| l.offset),
            Some(0),
            "隐藏列表不得接收滚轮（行会成员/仓库页重叠场景）"
        );
        assert_eq!(
            world.get::<UiScrollList>(shown).map(|l| l.offset),
            Some(1),
            "可见列表必须正常滚动"
        );
    }

    /// #2985 B2：滑块必须**真的跟着 offset 走**（高度按可见比、位置按行程比）。
    ///
    /// 曾经的 `list_thumb` 返回 `co.0`——而 `co` 是 `&ChildOf`，`co.0` 是**父实体**（列表自己），
    /// 于是 `thumb_write.get_mut(thumb)` 永远失败、`continue` 掉：滑块停在
    /// `spawn_scroll_bar_ui` 的初始值 16x40，滚到天涯也不动。
    ///
    /// 之前只有"滚轮改变 offset"的测试，`offset` 是对的，所以这个 bug 一路绿灯——
    /// 而用户看到的恰恰是"滚动条好像没实现"。故这里断言的是**滑块 Node 本身**。
    #[test]
    fn thumb_node_tracks_offset_and_visible_ratio() {
        use bevy::ecs::message::Messages;
        use bevy::ecs::system::RunSystemOnce;

        let mut world = World::new();
        world.init_resource::<Messages<MouseWheel>>();
        world.init_resource::<crate::control::CursorProbe>();
        world.init_resource::<ButtonInput<MouseButton>>();
        world.init_resource::<crate::ui::scroll_list::ScrollDrag>();

        let root = world
            .spawn(abs_node(0.0, 0.0, Some(400.0), Some(400.0)))
            .id();
        // 列表：轨道 (337,16,16,302)、可见 18 行、共 54 行 → 滑块高 302*18/54 ≈ 100.7
        let page = world
            .spawn((
                abs_node(0.0, 0.0, Some(352.0), Some(372.0)),
                ChildOf(root),
                InheritedVisibility::VISIBLE,
                UiScrollList {
                    rect_rel: (0.0, 0.0, 352.0, 372.0),
                    row_h: 15.0,
                    visible: 18,
                    total: 54,
                    offset: 0,
                    step: 1,
                    track_rel: (337.0, 16.0, 16.0, 302.0),
                    thumb: None,
                    z: 8,
                },
            ))
            .id();
        let thumb = world
            .spawn((
                abs_node(337.0, 16.0, Some(16.0), Some(40.0)),
                UiScrollThumb,
                ChildOf(page),
                InheritedVisibility::VISIBLE,
            ))
            .id();

        world
            .run_system_once(scroll_list_ui_system)
            .expect("滚动系统应可运行");
        // 与实现同序（先算比例再乘），否则 f32 舍入末位不同，断言会假红
        let expect_h = 302.0 * (18.0f32 / 54.0f32);
        let n = world.get::<Node>(thumb).expect("滑块应有 Node");
        assert_eq!(
            n.height,
            Val::Px(expect_h),
            "滑块高度必须按 可见/总 比例算（而不是停在初始 40）"
        );
        assert_eq!(n.top, Val::Px(16.0), "offset=0 时滑块贴轨道顶");
        assert_eq!(n.width, Val::Px(16.0));

        // 滚到底：行程 = 轨道高 - 滑块高 → 滑块贴轨道底
        world.get_mut::<UiScrollList>(page).unwrap().offset = 36; // max_offset = 54-18
        world
            .run_system_once(scroll_list_ui_system)
            .expect("滚动系统应可重复运行");
        let n = world.get::<Node>(thumb).unwrap();
        assert_eq!(
            n.top,
            Val::Px(16.0 + (302.0 - expect_h)),
            "offset 到底时滑块必须走到轨道末端"
        );
    }

    /// bug5 滚动条审计回归：列表挂在**页面容器**（面板子节点）时，滚轮命中必须沿
    /// ChildOf 链累加各层 Node.left/top 得到屏幕原点（C# 中控件 Location 相对父级、
    /// 命中判定用屏幕坐标）。否则页级列表（行会成员页/商城分类等）滚轮失效。
    ///
    /// 阳性对照：把 `origin()` 退回「只读本实体 left/top」→ 本测试 FAILED
    /// （页屏幕矩形 (110..160, 70..120)，旧原点 (10,20) 下光标 (120,80) 不命中）。
    #[test]
    fn scroll_list_wheel_hits_page_level_list_via_parent_chain() {
        use bevy::ecs::message::Messages;
        use bevy::ecs::system::RunSystemOnce;

        let mut world = World::new();
        world.init_resource::<Messages<MouseWheel>>();
        // 滚轮命中改读注入探针（#2961 项5）：本测试走**真实光标**路径，
        // 故探针保持 None（probe 为 None 时回落 window.cursor_position）
        world.init_resource::<crate::control::CursorProbe>();
        world.init_resource::<ButtonInput<MouseButton>>();
        world.init_resource::<crate::ui::scroll_list::ScrollDrag>();

        // 根面板 @(100,50)，页面容器 @(10,20)；列表可视区页内 (0,0,50,50)
        // → 屏幕矩形 (110,70)-(160,120)
        let root = world
            .spawn(abs_node(100.0, 50.0, Some(300.0), Some(300.0)))
            .id();
        let page = world
            .spawn((
                abs_node(10.0, 20.0, Some(200.0), Some(200.0)),
                ChildOf(root),
                // 裸测试世界无可见性传播系统：Node 派生组件默认 HIDDEN，须显式可见
                InheritedVisibility::VISIBLE,
                UiScrollList {
                    rect_rel: (0.0, 0.0, 50.0, 50.0),
                    row_h: 10.0,
                    visible: 5,
                    total: 20,
                    offset: 0,
                    step: 1,
                    track_rel: (50.0, 0.0, 10.0, 50.0),
                    thumb: None,
                    z: 1,
                },
            ))
            .id();

        // 窗口 + 光标放在页级列表屏幕矩形内 (120,80)
        let mut window = Window::default();
        window.set_cursor_position(Some(Vec2::new(120.0, 80.0)));
        let win = world.spawn(window).id();

        // 滚一格轮（Line 1 格 × step 1 = 1 行）
        world.write_message(MouseWheel {
            unit: MouseScrollUnit::Line,
            x: 0.0,
            y: 1.0,
            window: win,
            phase: bevy::input::touch::TouchPhase::Moved,
        });
        world
            .run_system_once(scroll_list_ui_system)
            .expect("滚动系统应可运行");
        assert_eq!(
            world.get::<UiScrollList>(page).map(|l| l.offset),
            Some(1),
            "光标命中页级列表屏幕矩形时滚轮必须滚动它（链式原点）"
        );

        // 光标移到页外 (105,55)（根面板内、页面矩形外）→ 不再命中，offset 不变
        world
            .get_mut::<Window>(win)
            .unwrap()
            .set_cursor_position(Some(Vec2::new(105.0, 55.0)));
        world.write_message(MouseWheel {
            unit: MouseScrollUnit::Line,
            x: 0.0,
            y: 1.0,
            window: win,
            phase: bevy::input::touch::TouchPhase::Moved,
        });
        world
            .run_system_once(scroll_list_ui_system)
            .expect("滚动系统应可重复运行");
        assert_eq!(
            world.get::<UiScrollList>(page).map(|l| l.offset),
            Some(1),
            "光标在页面矩形外时不得滚动页级列表"
        );
    }
}

// ============================================================================
// bevy_ui 可滚动列表（C# MirListBox + ScrollBar）
// 与 sprite scroll_list 同语义：UiScrollList 挂在面板根（Node.left/top = 屏幕坐标），
// rect_rel/track_rel 相对面板；滚轮滚动 + 滑块拖动 + 滑块跟随 offset。
// ============================================================================

/// 滚动命中/读值的**唯一**绝对原点算法：沿 `ChildOf` 链累加各层 `Node.left/top`
/// （根面板 / 页面容器通用）。
///
/// 滚轮命中、滑块命中（本模块）与 `control` RPC 的 `scroll` 读值**必须共用这一份**：
/// 三处各写一遍，任一处口径漂移都会让自动化脚本算出的命中点与实际判定错位，
/// 而那种错位只表现为"脚本说没滚动"这类难查的假阴性。
pub fn scroll_origin(
    e: Entity,
    parents: &Query<&ChildOf>,
    nodes: &Query<&Node, Without<UiScrollThumb>>,
) -> (f32, f32) {
    let mut x = 0.0;
    let mut y = 0.0;
    let mut cur = Some(e);
    while let Some(c) = cur {
        if let Ok(n) = nodes.get(c) {
            if let Val::Px(v) = n.left {
                x += v;
            }
            if let Val::Px(v) = n.top {
                y += v;
            }
        }
        cur = parents.get(c).ok().map(|co| co.parent());
    }
    (x, y)
}

/// bevy_ui 可滚动列表状态（挂在对话框容器实体上）
#[derive(Component, Debug, Clone)]
pub struct UiScrollList {
    /// 列表可视区（相对容器左上角）
    pub rect_rel: (f32, f32, f32, f32),
    /// 行高（px）
    pub row_h: f32,
    /// 可视行数
    pub visible: usize,
    /// 数据总行数（对话框每帧 set_total）
    pub total: usize,
    /// 当前滚动偏移（首行下标）
    pub offset: usize,
    /// 滚轮每格滚动行数
    pub step: usize,
    /// 滚动条轨道（相对容器左上角）
    pub track_rel: (f32, f32, f32, f32),
    /// 滚动条滑块实体（spawn_scroll_bar_ui 返回）
    pub thumb: Option<Entity>,
    /// z 排序（多个列表重叠时滚动最上层）
    pub z: i32,
}

impl UiScrollList {
    /// 最大可用偏移（数据不满一屏时为 0）
    pub fn max_offset(&self) -> usize {
        self.total.saturating_sub(self.visible)
    }

    /// 更新数据行数并夹紧偏移
    pub fn set_total(&mut self, total: usize) {
        self.total = total;
        self.offset = self.offset.min(self.max_offset());
    }
}

/// bevy_ui 滚动条滑块标记
#[derive(Component)]
pub struct UiScrollThumb;

/// 生成 bevy_ui 滚动条（轨道 + 滑块，面板子节点），返回滑块实体。
/// 轨道/滑块为 Node + BackgroundColor；位置由 scroll_list_ui_system 维护。
pub fn spawn_scroll_bar_ui(
    parent: &mut ChildSpawnerCommands,
    track_rel: (f32, f32, f32, f32),
    z: i32,
) -> (Entity, Entity) {
    // 轨道（半透明深色）
    let track = parent
        .spawn((
            abs_node(
                track_rel.0,
                track_rel.1,
                Some(track_rel.2),
                Some(track_rel.3),
            ),
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.35)),
            ZIndex(z),
        ))
        .id();
    // 滑块（浅色）
    let thumb = parent
        .spawn((
            abs_node(track_rel.0, track_rel.1, Some(track_rel.2), Some(40.0)),
            BackgroundColor(Color::srgba(0.85, 0.85, 0.9, 0.9)),
            UiScrollThumb,
            ZIndex(z + 1),
        ))
        .id();
    (track, thumb)
}

/// bevy_ui 滚轮滚动 + 滑块定位 + 滑块拖动
/// 滑块为列表容器子节点（UiScrollThumb），按父子关系查找，无需存实体。
/// thumb_read 只读用于查找/命中；thumb_write 用于每帧定位滑块。
/// 列表可挂在**页面容器**（面板子节点）上：屏幕原点沿 ChildOf 链累加
/// （面板拖动只改根 Node.left/top，链和自动跟随）。
#[allow(clippy::type_complexity)]
pub fn scroll_list_ui_system(
    mut wheels: MessageReader<MouseWheel>,
    windows: Query<&Window>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut drag: ResMut<crate::ui::scroll_list::ScrollDrag>,
    mut lists: Query<(Entity, &mut UiScrollList), Without<UiScrollThumb>>,
    thumb_read: Query<(Entity, &ChildOf, &UiScrollThumb)>,
    mut thumb_write: Query<(&ChildOf, &mut Node, &UiScrollThumb)>,
    parents: Query<&ChildOf>,
    node_read: Query<&Node, Without<UiScrollThumb>>,
    // 隐藏列表不参与滚轮/滑块拖动命中：列表挂在隐藏页（行会成员页/仓库页）
    // 或已关窗口上时不得吞输入；两页同坐标重叠时按此硬性区分。
    // 缺组件（极简测试 App 无可见性传播）按「可见」处理。
    lists_vis: Query<&InheritedVisibility, Without<UiScrollThumb>>,
    // #2961 项5 验收：滚轮命中改读注入探针——无焦点/共享桌面上
    // `Window::cursor_position()` 不可用，自动化驱动不了滚动，本项就只能靠"看图"。
    // 探针为 None 时回落真实光标，常态行为不变（click RPC 完成即撤探针）。
    probe: Res<crate::control::CursorProbe>,
) {
    // 滑块跟随 offset —— **必须在光标判定之前**跑：
    // 这一段只依赖列表状态，与光标无关，而下面的命中判定会在
    // `window.cursor_position()` 为 None（无焦点窗口 / 共享桌面 / 自动化）时提前 return。
    // 放在后面会让滑块永远停在 `spawn_scroll_bar_ui` 的初始值（16x40），
    // 不管总行数多少 —— 看上去就是"滚动条没实现"（2026-09-19 实机：28 行成员列表，
    // 滑块高度仍是 40 而不是 302*18/28≈194）。
    // 每帧把滑块移到 offset 对应位置（跟随容器拖动）
    for (e, list) in lists.iter() {
        let Some(thumb) = list_thumb(e, &thumb_read) else {
            continue;
        };
        let Ok((_, mut tn, _)) = thumb_write.get_mut(thumb) else {
            continue;
        };
        let (tx, ty, tw, th) = list.track_rel;
        let total = list.total.max(list.visible);
        let thumb_h = (th * (list.visible as f32 / total as f32)).clamp(14.0, th);
        let max_off = list.max_offset();
        let ratio = if max_off == 0 {
            0.0
        } else {
            list.offset as f32 / max_off as f32
        };
        let thumb_y = ty + ratio * (th - thumb_h);
        tn.left = Val::Px(tx);
        tn.top = Val::Px(thumb_y);
        tn.width = Val::Px(tw);
        tn.height = Val::Px(thumb_h);
    }

    // ---- 以下为命中/拖动/滚轮：都需要光标位置 ----
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor) = probe.pos.or_else(|| window.cursor_position()) else {
        return;
    };

    // 列表是否可见（含祖先传播）：隐藏列表跳过命中
    fn shown(e: Entity, vis: &Query<&InheritedVisibility, Without<UiScrollThumb>>) -> bool {
        vis.get(e).map(|v| v.get()).unwrap_or(true)
    }

    // 找某列表的子滑块（UiScrollThumb 且 parent == 列表实体）
    fn list_thumb(e: Entity, thumbs: &Query<(Entity, &ChildOf, &UiScrollThumb)>) -> Option<Entity> {
        thumbs
            .iter()
            .find(|(_, co, _)| co.parent() == e)
            .map(|(t, _, _)| t)
    }

    // 滑块拖动（C# MirScrollBar movable）
    if mouse.just_pressed(MouseButton::Left) && drag.dragging.is_none() {
        for (e, list) in lists.iter() {
            if !shown(e, &lists_vis) {
                continue;
            }
            let Some(thumb) = list_thumb(e, &thumb_read) else {
                continue;
            };
            let (ox, oy) = scroll_origin(e, &parents, &node_read);
            let total = list.total.max(list.visible);
            let (tx, ty, tw, th) = list.track_rel;
            let thumb_h = (th * (list.visible as f32 / total as f32)).clamp(14.0, th);
            let max_off = list.max_offset();
            let ratio = if max_off == 0 {
                0.0
            } else {
                list.offset as f32 / max_off as f32
            };
            let thumb_y = oy + ty + ratio * (th - thumb_h);
            if cursor.x >= ox + tx
                && cursor.x <= ox + tx + tw
                && cursor.y >= thumb_y
                && cursor.y <= thumb_y + thumb_h
            {
                drag.dragging = Some(thumb);
                drag.grab_offset = cursor.y - thumb_y;
                break;
            }
        }
    }
    if let Some(thumb_e) = drag.dragging {
        if !mouse.pressed(MouseButton::Left) {
            drag.dragging = None;
        } else {
            for (e, mut list) in lists.iter_mut() {
                if !shown(e, &lists_vis) || list_thumb(e, &thumb_read) != Some(thumb_e) {
                    continue;
                }
                let (_, oy) = scroll_origin(e, &parents, &node_read);
                let total = list.total.max(list.visible);
                let (_, ty, _, th) = list.track_rel;
                let thumb_h = (th * (list.visible as f32 / total as f32)).clamp(14.0, th);
                let track_top = oy + ty;
                let max_off = list.max_offset();
                if max_off == 0 {
                    list.offset = 0;
                    break;
                }
                let ty_clamped =
                    (cursor.y - drag.grab_offset).clamp(track_top, track_top + th - thumb_h);
                let ratio = ((ty_clamped - track_top) / (th - thumb_h)).clamp(0.0, 1.0);
                list.offset = (ratio * max_off as f32).round() as usize;
                break;
            }
        }
    }

    // 汇总本帧滚轮增量（行）。像素滚动按 ~20px/行折算。
    let mut scroll_y = 0.0f32;
    for ev in wheels.read() {
        match ev.unit {
            MouseScrollUnit::Line => scroll_y += ev.y,
            MouseScrollUnit::Pixel => scroll_y += ev.y / 20.0,
        }
    }

    if scroll_y.abs() > 0.0 {
        let mut best: Option<(i32, Entity)> = None;
        for (e, list) in lists.iter() {
            if !shown(e, &lists_vis) {
                continue;
            }
            let (ox, oy) = scroll_origin(e, &parents, &node_read);
            let (rx, ry, rw, rh) = list.rect_rel;
            if cursor.x >= ox + rx
                && cursor.x <= ox + rx + rw
                && cursor.y >= oy + ry
                && cursor.y <= oy + ry + rh
            {
                if best.map_or(true, |(bz, _)| list.z > bz) {
                    best = Some((list.z, e));
                }
            }
        }
        if let Some((_, e)) = best {
            if let Ok((_, mut list)) = lists.get_mut(e) {
                let rows = (scroll_y * list.step.max(1) as f32).round() as i32;
                let max = list.max_offset() as i32;
                list.offset = (list.offset as i32 + rows).clamp(0, max) as usize;
            }
        }
    }
}

/// 生成根级 bevy_ui 通用物品格（动态网格用；面板原点为 (0,0) 时坐标即绝对坐标）。
/// 返回格子实体。子节点（图标/数量/耐久条）与 spawn_item_cell_ui 同构。
///
/// 注意 z 语义：本函数挂的是 `GlobalZIndex`（根节点跨 UI 树全局层级），而面板
/// 背景也是根节点 GlobalZIndex——bevy 0.19 根节点按 `GlobalZIndex` 升序绘制
/// （ui_layout/stack.rs root_nodes 排序），故格子的 z **必须高于所属面板** z
/// 才不被面板背景盖住；若格子挂为面板子实体则应改用 `ZIndex`（兄弟序）。
#[allow(clippy::too_many_arguments)]
pub fn spawn_item_cell_ui_root(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    font: &Handle<Font>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    z: i32,
    slot: usize,
) -> Entity {
    let white = images.add(crate::map_renderer::make_image(
        vec![255, 255, 255, 255],
        1,
        1,
    ));
    let cell = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(x),
                top: Val::Px(y),
                width: Val::Px(w),
                height: Val::Px(h),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.18)),
            UiItemCell { slot },
            UiItemCellData::default(),
            GlobalZIndex(z),
        ))
        .id();
    commands.entity(cell).with_children(|p| {
        p.spawn((
            abs_node(2.0, 2.0, Some(w - 4.0), Some(h - 4.0)),
            ImageNode::new(white.clone()),
            UiItemCellIcon(slot),
            Visibility::Hidden,
            ZIndex(1),
        ));
        p.spawn((
            abs_node(w - 16.0, h - 13.0, None, None),
            Text::new(String::new()),
            TextFont {
                font: FontSource::Handle(font.clone()),
                font_size: FontSize::Px(10.0),
                ..default()
            },
            TextColor(Color::srgb(1.0, 1.0, 0.6)),
            UiItemCellCount(slot),
            Visibility::Hidden,
            ZIndex(2),
        ));
        p.spawn((
            abs_node(2.0, h - 4.0, Some(w - 4.0), Some(2.0)),
            BackgroundColor(Color::srgb(1.0, 0.2, 0.2)),
            UiItemCellDura(slot, w - 4.0),
            Visibility::Hidden,
            ZIndex(3),
        ));
    });
    cell
}

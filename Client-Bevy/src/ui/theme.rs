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
    });
    cmds.entity(box_e)
}

/// bevy_ui 下拉框系统：展开/收起/选择/滚轮/点击外部关闭
pub fn dropdown_ui_system(
    mut dd_q: Query<(Entity, &Interaction, &mut UiDropDown)>,
    options: Query<&Interaction, Without<UiDropDown>>,
    mut texts: Query<&mut Text>,
    mut popups: Query<&mut Visibility, With<UiDropDownPopup>>,
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
                dd.popup_pos.0,
                dd.popup_pos.1,
                dd.popup_w,
                dd.row_h * dd.popup_rows as f32,
            );
            if cursor.x >= px && cursor.x <= px + pw && cursor.y >= py && cursor.y <= py + ph {
                let max = dd.items.len().saturating_sub(dd.popup_rows);
                dd.scroll = (dd.scroll as i32 + scroll_y.round() as i32)
                    .clamp(0, max as i32) as usize;
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
                dd.popup_pos.0,
                dd.popup_pos.1,
                dd.popup_w,
                dd.row_h * dd.popup_rows as f32,
            );
            let in_popup = cursor.x >= px
                && cursor.x <= px + pw
                && cursor.y >= py
                && cursor.y <= py + ph;
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
                let in_box = cursor.x >= bx
                    && cursor.x <= bx + bw
                    && cursor.y >= by
                    && cursor.y <= by + bh;
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
        if let Ok(mut v) = popups.get_mut(dd.popup) {
            *v = if dd.open { Visibility::Visible } else { Visibility::Hidden };
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
    let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
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
        *vis = if show { Visibility::Visible } else { Visibility::Hidden };
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
        *vis = if show { Visibility::Visible } else { Visibility::Hidden };
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
    use super::*;
    use bevy::ecs::world::CommandQueue;

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
}

// ============================================================================
// bevy_ui 可滚动列表（C# MirListBox + ScrollBar）
// 与 sprite scroll_list 同语义：UiScrollList 挂在面板根（Node.left/top = 屏幕坐标），
// rect_rel/track_rel 相对面板；滚轮滚动 + 滑块拖动 + 滑块跟随 offset。
// ============================================================================

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
            abs_node(track_rel.0, track_rel.1, Some(track_rel.2), Some(track_rel.3)),
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
#[allow(clippy::type_complexity)]
pub fn scroll_list_ui_system(
    mut wheels: MessageReader<MouseWheel>,
    windows: Query<&Window>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut drag: ResMut<crate::ui::scroll_list::ScrollDrag>,
    mut lists: Query<(Entity, &mut UiScrollList, &Node), Without<UiScrollThumb>>,
    thumb_read: Query<(&ChildOf, &UiScrollThumb)>,
    mut thumb_write: Query<(&ChildOf, &mut Node, &UiScrollThumb)>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };

    // 容器屏幕原点（面板根 Node.left/top）
    fn origin(node: &Node) -> (f32, f32) {
        let x = match node.left {
            Val::Px(v) => v,
            _ => 0.0,
        };
        let y = match node.top {
            Val::Px(v) => v,
            _ => 0.0,
        };
        (x, y)
    }

    // 找某列表的子滑块（UiScrollThumb 且 parent == 列表实体）
    fn list_thumb(
        e: Entity,
        thumbs: &Query<(&ChildOf, &UiScrollThumb)>,
    ) -> Option<Entity> {
        thumbs
            .iter()
            .find(|(co, _)| co.parent() == e)
            .map(|(co, _)| co.0)
    }

    // 滑块拖动（C# MirScrollBar movable）
    if mouse.just_pressed(MouseButton::Left) && drag.dragging.is_none() {
        for (e, list, node) in lists.iter() {
            let Some(thumb) = list_thumb(e, &thumb_read) else {
                continue;
            };
            let (ox, oy) = origin(node);
            let total = list.total.max(list.visible);
            let (tx, ty, tw, th) = list.track_rel;
            let thumb_h = (th * (list.visible as f32 / total as f32)).clamp(14.0, th);
            let max_off = list.max_offset();
            let ratio = if max_off == 0 { 0.0 } else { list.offset as f32 / max_off as f32 };
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
            for (e, mut list, node) in lists.iter_mut() {
                if list_thumb(e, &thumb_read) != Some(thumb_e) {
                    continue;
                }
                let (ox, oy) = origin(node);
                let total = list.total.max(list.visible);
                let (_, ty, _, th) = list.track_rel;
                let thumb_h = (th * (list.visible as f32 / total as f32)).clamp(14.0, th);
                let track_top = oy + ty;
                let max_off = list.max_offset();
                if max_off == 0 {
                    list.offset = 0;
                    break;
                }
                let ty_clamped = (cursor.y - drag.grab_offset)
                    .clamp(track_top, track_top + th - thumb_h);
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
        for (e, list, node) in lists.iter() {
            let (ox, oy) = origin(node);
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
            if let Ok((_, mut list, _)) = lists.get_mut(e) {
                let rows = (scroll_y * list.step.max(1) as f32).round() as i32;
                let max = list.max_offset() as i32;
                list.offset = (list.offset as i32 + rows).clamp(0, max) as usize;
            }
        }
    }

    // 每帧把滑块移到 offset 对应位置（跟随容器拖动）
    for (e, list, _) in lists.iter() {
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
}

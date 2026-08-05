// ============================================================================
// controls - 通用基础控件（#90）
// 参考：C# Client/MirControls/*.cs
//   - CheckBox：双帧（勾选/未勾选）+ 标签，点击切换（MirCheckBox）
//   - DropDown：下拉框（MirDropDownBox）：闭合框显示选中项，点击展开最多 5 项，
//     滚轮滚动更多项，点击选项选中并关闭
// ============================================================================

use bevy::ecs::hierarchy::ChildOf;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;

use crate::map_renderer::GameLibraries;
use crate::resources::libraries::LibraryName;
use crate::ui::sprite_ui::{
    spawn_ui_button, spawn_ui_sprite, spawn_ui_text, ui_image, ButtonFrames, UiButton, UiEntity,
    UiImageCache,
};

/// 勾选框（MirCheckBox）：checked 状态 + 两套三态帧
#[derive(Component)]
pub struct CheckBox {
    pub checked: bool,
    /// 两套帧的图柄：off[3] / on[3]（normal/hover/pressed）
    pub off: [Handle<Image>; 3],
    pub on: [Handle<Image>; 3],
}

/// 生成勾选框（返回实体；帧图全部预载）
#[allow(clippy::too_many_arguments)]
pub fn spawn_checkbox(
    commands: &mut Commands,
    libs: &mut GameLibraries,
    images: &mut Assets<Image>,
    cache: &mut UiImageCache,
    name: LibraryName,
    off_idx: [usize; 3],
    on_idx: [usize; 3],
    x: f32,
    y: f32,
    z: f32,
    w: f32,
    h: f32,
    checked: bool,
) -> Option<Entity> {
    let off = [
        ui_image(libs, images, cache, name, off_idx[0])?,
        ui_image(libs, images, cache, name, off_idx[1])?,
        ui_image(libs, images, cache, name, off_idx[2])?,
    ];
    let on = [
        ui_image(libs, images, cache, name, on_idx[0])?,
        ui_image(libs, images, cache, name, on_idx[1])?,
        ui_image(libs, images, cache, name, on_idx[2])?,
    ];
    let frames = if checked { on.clone() } else { off.clone() };
    let e = spawn_ui_button(
        commands, libs, images, cache, name,
        // 帧索引只是占位（spawn_ui_button 会加载一次），随后用预载图柄覆盖
        off_idx[0], off_idx[1], off_idx[2],
        x, y, z, w, h,
    )?;
    commands.entity(e).insert((
        CheckBox { checked, off, on },
        ButtonFrames {
            normal: frames[0].clone(),
            hover: frames[1].clone(),
            pressed: frames[2].clone(),
        },
    ));
    Some(e)
}

/// 勾选框系统：点击切换 + 状态帧同步（在 ui_button_system 之后运行）
pub fn checkbox_system(
    mut boxes: Query<(&UiButton, &mut CheckBox, &mut ButtonFrames, &mut Sprite)>,
) {
    for (btn, mut cb, mut frames, mut sprite) in &mut boxes {
        if btn.clicked {
            cb.checked = !cb.checked;
        }
        let f = if cb.checked { &cb.on } else { &cb.off };
        if frames.normal != f[0] || frames.hover != f[1] || frames.pressed != f[2] {
            frames.normal = f[0].clone();
            frames.hover = f[1].clone();
            frames.pressed = f[2].clone();
            // 立即刷新当前帧（否则要等鼠标状态变化才换图）
            sprite.image = f[0].clone();
        }
    }
}

/// 动画按钮（MirAnimatedButton）：帧序列按间隔自动轮播 + 可选悬停/按下帧
/// 参考：C# Client/MirControls/MirAnimatedButton.cs（UpdateOffSet 循环帧）
#[derive(Component)]
pub struct AnimatedButton {
    /// 轮播帧（已预载图柄）
    pub frames: Vec<Handle<Image>>,
    /// 悬停帧（None 则显示当前轮播帧）
    pub hover: Option<Handle<Image>>,
    /// 按下帧（None 则显示当前轮播帧）
    pub pressed: Option<Handle<Image>>,
    /// 当前帧下标
    pub frame: usize,
    /// 帧间隔（秒）
    pub delay: f32,
    /// 累计时间
    pub timer: f32,
    /// 是否循环（false 播完停在最后一帧）
    pub looping: bool,
    /// 是否播放（C# Animated）
    pub playing: bool,
}

impl AnimatedButton {
    /// 帧步进纯逻辑：按 dt 推进帧下标（C# UpdateOffSet 语义）
    /// - 未播放 / 帧数 <=1 / 间隔 <=0：不动
    /// - 循环：播完回到 0
    /// - 单次：播完停在最后一帧并停止播放
    pub fn tick(&mut self, dt: f32) {
        if !self.playing || self.frames.len() <= 1 || self.delay <= 0.0 || dt <= 0.0 {
            return;
        }
        self.timer += dt;
        while self.timer >= self.delay {
            self.timer -= self.delay;
            if self.frame + 1 < self.frames.len() {
                self.frame += 1;
            } else if self.looping {
                self.frame = 0;
            } else {
                self.playing = false;
                break;
            }
        }
    }
}

/// 生成动画按钮：frames 为连续帧索引 [base_idx, base_idx+count)，hover/pressed 可选
/// （C# MirAnimatedButton：Index 为起始帧，OffSet 在 [0, AnimationCount) 内轮播）
#[allow(clippy::too_many_arguments)]
pub fn spawn_animated_button(
    commands: &mut Commands,
    libs: &mut GameLibraries,
    images: &mut Assets<Image>,
    cache: &mut UiImageCache,
    name: LibraryName,
    base_idx: usize,
    count: usize,
    hover_idx: Option<usize>,
    pressed_idx: Option<usize>,
    x: f32,
    y: f32,
    z: f32,
    w: f32,
    h: f32,
    delay: f32,
    looping: bool,
) -> Option<Entity> {
    let mut frames = Vec::with_capacity(count);
    for i in 0..count {
        frames.push(ui_image(libs, images, cache, name, base_idx + i)?);
    }
    let hover = match hover_idx {
        Some(i) => Some(ui_image(libs, images, cache, name, i)?),
        None => None,
    };
    let pressed = match pressed_idx {
        Some(i) => Some(ui_image(libs, images, cache, name, i)?),
        None => None,
    };
    let e = spawn_ui_sprite(commands, frames[0].clone(), x, y, z, 1.0);
    commands.entity(e).insert((
        UiButton {
            rect: (x, y, w, h),
            clicked: false,
        },
        AnimatedButton {
            frames,
            hover,
            pressed,
            frame: 0,
            delay,
            timer: 0.0,
            looping,
            playing: true,
        },
    ));
    Some(e)
}

/// 动画按钮系统：时间步进 + 状态帧（按下 > 悬停 > 轮播帧）
/// 依赖 ui_button_system 先运行（UiButton.clicked/rect；Bevy 会自动排序冲突系统）
pub fn animated_button_system(
    mut btns: Query<(&UiButton, &mut AnimatedButton, &mut Sprite)>,
    time: Res<Time>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
) {
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let down = mouse.pressed(MouseButton::Left);
    for (btn, mut ab, mut sprite) in &mut btns {
        let (x, y, w, h) = btn.rect;
        let over = cursor.x >= x && cursor.x <= x + w && cursor.y >= y && cursor.y <= y + h;
        ab.tick(time.delta_secs());
        let img = if down && over {
            ab.pressed.as_ref().or_else(|| ab.frames.get(ab.frame))
        } else if over {
            ab.hover.as_ref().or_else(|| ab.frames.get(ab.frame))
        } else {
            ab.frames.get(ab.frame)
        };
        if let Some(h) = img {
            if sprite.image != *h {
                sprite.image = h.clone();
            }
        }
    }
}
/// 下拉框（MirDropDownBox 简化版）：闭合框 + 弹出选项
#[derive(Component)]
pub struct DropDown {
    /// 选项
    pub items: Vec<String>,
    /// 当前选中下标
    pub selected: Option<usize>,
    /// 是否展开
    pub open: bool,
    /// 弹出面板实体（隐藏/显示）
    pub popup: Entity,
    /// 闭合框上显示选中项的文字实体
    pub text: Entity,
    /// 闭合框矩形（屏幕坐标，点击外部关闭用）
    pub box_rect: (f32, f32, f32, f32),
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
    /// z 序（弹出面板覆盖其他 UI）
    pub z: f32,
}

/// 弹出面板标记（用于点击外部关闭）
#[derive(Component)]
pub struct DropDownPopup;

/// 生成下拉框。返回下拉框实体；闭合框按钮用 UiButton（rect 命中）。
#[allow(clippy::too_many_arguments)]
pub fn spawn_dropdown(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    font: &Handle<Font>,
    items: Vec<String>,
    selected: Option<usize>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    popup_rows: usize,
    z: f32,
) -> Entity {
    let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    let popup_w = w + 4.0;
    let row_h = h;

    // 闭合框：深色底 + 文字 + 下拉箭头
    let box_e = commands
        .spawn((
            UiEntity,
            Sprite {
                image: white.clone(),
                color: Color::srgba(0.05, 0.05, 0.08, 0.95),
                custom_size: Some(Vec2::new(w, h)),
                ..default()
            },
            bevy::sprite::Anchor::TOP_LEFT,
            Transform::from_xyz(x, -y, z),
            Visibility::Visible,
            UiButton { rect: (x, y, w, h), clicked: false },
        ))
        .id();
    let text = spawn_ui_text(
        commands, font,
        &items.get(selected.unwrap_or(usize::MAX)).cloned().unwrap_or_default(),
        x + 6.0, y + (h - 12.0) / 2.0,
        12.0, Color::WHITE, z + 0.1,
    );
    let arrow = spawn_ui_text(commands, font, "▼", x + w - 14.0, y + (h - 12.0) / 2.0, 10.0, Color::srgb(0.8, 0.8, 0.8), z + 0.1);

    // 弹出面板：深色底 + 选项行
    let popup_y = y + h;
    let popup = commands
        .spawn((
            UiEntity,
            DropDownPopup,
            Sprite {
                image: white,
                color: Color::srgba(0.08, 0.08, 0.12, 0.98),
                custom_size: Some(Vec2::new(popup_w, row_h * popup_rows as f32 + 2.0)),
                ..default()
            },
            bevy::sprite::Anchor::TOP_LEFT,
            Transform::from_xyz(x - 2.0, -popup_y, z + 0.2),
            Visibility::Hidden,
        ))
        .id();
    let mut option_texts = Vec::new();
    for i in 0..popup_rows {
        let t = spawn_ui_text(
            commands, font, "",
            x + 4.0, popup_y + 2.0 + i as f32 * row_h,
            12.0, Color::WHITE, z + 0.3,
        );
        option_texts.push(t);
    }

    let dd = DropDown {
        items,
        selected,
        open: false,
        popup,
        text,
        box_rect: (x, y, w, h),
        option_texts,
        popup_pos: (x - 2.0, popup_y),
        popup_w,
        row_h,
        popup_rows,
        scroll: 0,
        z,
    };
    commands.entity(box_e).insert(dd);
    let _ = arrow;
    box_e
}

/// 下拉框系统：展开/收起/选择/滚轮/点击外部关闭
/// 依赖 ui_button_system 先运行（UiButton.clicked）。
#[allow(clippy::too_many_arguments)]
pub fn dropdown_system(
    mut dd_q: Query<(&UiButton, &mut DropDown)>,
    mut texts: Query<&mut Text2d>,
    mut popups: Query<&mut Visibility, With<DropDownPopup>>,
    mut wheels: MessageReader<MouseWheel>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
) {
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else { return };

    // 1. 闭合框点击 → 切换展开
    for (btn, mut dd) in &mut dd_q {
        if btn.clicked {
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
        for (_, mut dd) in dd_q.iter_mut() {
            if !dd.open {
                continue;
            }
            let (px, py, pw, ph) = (dd.popup_pos.0, dd.popup_pos.1, dd.popup_w, dd.row_h * dd.popup_rows as f32);
            if cursor.x >= px && cursor.x <= px + pw && cursor.y >= py && cursor.y <= py + ph {
                let max = dd.items.len().saturating_sub(dd.popup_rows);
                dd.scroll = (dd.scroll as i32 + scroll_y.round() as i32).clamp(0, max as i32) as usize;
                break;
            }
        }
    }

    // 3. 点击选项选中 / 点击外部关闭
    if mouse.just_pressed(MouseButton::Left) {
        for (_, mut dd) in dd_q.iter_mut() {
            if !dd.open {
                continue;
            }
            let (px, py, pw, ph) = (dd.popup_pos.0, dd.popup_pos.1, dd.popup_w, dd.row_h * dd.popup_rows as f32);
            let in_popup = cursor.x >= px && cursor.x <= px + pw && cursor.y >= py && cursor.y <= py + ph;
            if in_popup {
                for i in 0..dd.popup_rows {
                    let ry = py + i as f32 * dd.row_h;
                    if cursor.y >= ry && cursor.y <= ry + dd.row_h {
                        let idx = dd.scroll + i;
                        if idx < dd.items.len() {
                            dd.selected = Some(idx);
                        }
                        dd.open = false;
                        break;
                    }
                }
            } else {
                // 点不在面板内：若也不在闭合框内则关闭（闭合框内由第 1 步切换）
                let (bx, by, bw, bh) = dd.box_rect;
                let in_box = cursor.x >= bx && cursor.x <= bx + bw && cursor.y >= by && cursor.y <= by + bh;
                if !in_box {
                    dd.open = false;
                }
            }
        }
    }

    // 4. 刷新：面板显隐、选项文字、选中文字
    for (_, dd) in dd_q.iter_mut() {
        // 闭合框文字
        let sel_text = dd.items.get(dd.selected.unwrap_or(usize::MAX)).cloned().unwrap_or_default();
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


/// 滚动标签（MirScrollingLabel 简化版）：多行文本 + `{text/color}` 标签剥离
#[derive(Component)]
pub struct ScrollingLabel {
    /// 可视行数（C# VisibleLines）
    pub visible_lines: usize,
    /// 原始文本（可能含 {text/color} 标签，可能含 \n）
    pub text: String,
}

/// 剥离 `{text/color}` 颜色标签，返回纯文本（C# MirScrollingLabel.NewText 的文本部分）
pub fn strip_color_tags(line: &str) -> String {
    let chars: Vec<char> = line.chars().collect();
    let mut out = String::new();
    let mut i = 0usize;
    while i < chars.len() {
        if chars[i] == '{' {
            if let Some(end) = chars[i + 1..].iter().position(|&ch| ch == '}') {
                let inner: String = chars[i + 1..i + 1 + end].iter().collect();
                if let Some(slash) = inner.find('/') {
                    out.push_str(&inner[..slash]);
                } else {
                    out.push_str(&inner);
                }
                i += end + 2;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// 滚动标签系统：把 ScrollingLabel.text 按行剥离标签并写入 Text2d（最多 visible_lines 行）
pub fn scrolling_label_system(
    mut labels: Query<(&mut Text2d, &ScrollingLabel)>,
) {
    for (mut text, label) in &mut labels {
        let lines: Vec<String> = label
            .text
            .split('\n')
            .take(label.visible_lines.max(1))
            .map(strip_color_tags)
            .collect();
        let joined = lines.join("\n");
        if text.0 != joined {
            text.0 = joined;
        }
    }
}

// ============================================================================
// ItemCell - 通用物品格（MirItemCell 简化版）
// 结构：格子实体（ItemCell{slot} + ItemCellData）→ 子实体 ItemCellIcon/ItemCellCount
// 渲染由 item_cell_system 统一处理；对话框只需写 ItemCellData。
// ============================================================================

/// 物品格数据（对话框每帧写入）
#[derive(Component, Default, Clone)]
pub struct ItemCellData {
    /// 物品图标（Items 库图柄）
    pub icon: Option<Handle<Image>>,
    /// 堆叠数量（None 或 1 不显示数字）
    pub count: Option<u32>,
}

/// 物品格（槽位）
#[derive(Component)]
pub struct ItemCell {
    pub slot: usize,
}

/// 物品图标子实体
#[derive(Component)]
pub struct ItemCellIcon(pub usize);

/// 堆叠数量子实体
#[derive(Component)]
pub struct ItemCellCount(pub usize);

/// 生成通用物品格（底格 + 图标 + 数量），返回格子实体
pub fn spawn_item_cell(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    font: &Handle<Font>,
    x: f32,
    y: f32,
    z: f32,
    cell_w: f32,
    cell_h: f32,
    slot: usize,
) -> Entity {
    let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    let cell = commands
        .spawn((
            UiEntity,
            ItemCell { slot },
            ItemCellData::default(),
            Sprite {
                image: white.clone(),
                color: Color::srgba(0.0, 0.0, 0.0, 0.18),
                custom_size: Some(Vec2::new(cell_w, cell_h)),
                ..default()
            },
            bevy::sprite::Anchor::TOP_LEFT,
            Transform::from_xyz(x, -y, z),
            Visibility::Hidden,
        ))
        .id();
    commands.entity(cell).with_children(|p| {
        p.spawn((
            ItemCellIcon(slot),
            Sprite {
                image: white.clone(),
                custom_size: Some(Vec2::new(cell_w - 4.0, cell_h - 4.0)),
                ..default()
            },
            bevy::sprite::Anchor::TOP_LEFT,
            Transform::from_xyz(2.0, -2.0, z + 0.1),
            Visibility::Hidden,
        ));
        p.spawn((
            ItemCellCount(slot),
            Text2d::new(String::new()),
            bevy::sprite::Anchor::TOP_LEFT,
            TextFont {
                font: FontSource::Handle(font.clone()),
                font_size: FontSize::Px(10.0),
                ..default()
            },
            TextColor(Color::srgb(1.0, 1.0, 0.6)),
            Transform::from_xyz(cell_w - 16.0, -(cell_h - 13.0), z + 0.2),
            Visibility::Hidden,
        ));
    });
    cell
}

/// 物品格渲染系统：按 ItemCellData 刷新图标/数量
pub fn item_cell_system(
    cells: Query<&ItemCellData, (With<ItemCell>, Without<ItemCellIcon>, Without<ItemCellCount>)>,
    mut icons: Query<(&ChildOf, &mut Sprite, &mut Visibility, &ItemCellIcon), Without<ItemCellCount>>,
    mut counts: Query<(&ChildOf, &mut Text2d, &mut Visibility, &ItemCellCount), Without<ItemCellIcon>>,
) {
    for (child_of, mut sprite, mut vis, _icon) in &mut icons {
        let data = cells.get(child_of.parent()).ok();
        match data.and_then(|d| d.icon.clone()) {
            Some(h) if sprite.image != h => sprite.image = h,
            Some(_) => {}
            None => {}
        }
        let show = data.and_then(|d| d.icon.as_ref()).is_some_and(|h| h.is_strong());
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
}


#[cfg(test)]
mod tests {
    use super::{strip_color_tags, AnimatedButton};
    use bevy::prelude::*;

    fn ab(frames: usize, delay: f32, looping: bool) -> AnimatedButton {
        AnimatedButton {
            frames: vec![Handle::default(); frames],
            hover: None,
            pressed: None,
            frame: 0,
            delay,
            timer: 0.0,
            looping,
            playing: true,
        }
    }

    #[test]
    fn strip_tags_removes_color_segments() {
        assert_eq!(strip_color_tags("你好{世界/red}！"), "你好世界！");
        assert_eq!(strip_color_tags("纯文本"), "纯文本");
        assert_eq!(strip_color_tags("{a/b}{c/d}"), "ac");
    }

    #[test]
    fn animated_button_advances_by_delay() {
        let mut b = ab(10, 0.1, true);
        b.tick(0.25); // 2 个间隔
        assert_eq!(b.frame, 2);
        assert!(b.playing);
    }

    #[test]
    fn animated_button_loops_back_to_zero() {
        let mut b = ab(3, 0.1, true);
        b.tick(0.3); // 3 个间隔 → 回到 0
        assert_eq!(b.frame, 0);
        assert!(b.playing);
    }

    #[test]
    fn animated_button_one_shot_stops_at_last() {
        let mut b = ab(3, 0.1, false);
        b.tick(0.35);
        assert_eq!(b.frame, 2);
        assert!(!b.playing);
        b.tick(10.0);
        assert_eq!(b.frame, 2);
    }

    #[test]
    fn animated_button_paused_does_not_move() {
        let mut b = ab(10, 0.1, true);
        b.playing = false;
        b.tick(1.0);
        assert_eq!(b.frame, 0);
    }
}

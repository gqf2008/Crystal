// ============================================================================
// tooltip - 通用 Tooltip 体系（#93）
// 参考 C#：GameScene.DrawItemHint（物品提示框）+ MirControl.Hint（控件提示）
// 架构：
//   - TooltipState 资源：任意系统写入（source 归属 + 标题/多行/位置）
//   - TooltipHint(String) 组件：挂在 UiButton 上，tooltip_hint_system 自动检测悬停
//   - 常驻面板：背景 + 标题 + 最多 6 行，tooltip_panel_system 渲染（跟随光标、防出屏）
// 写入约定：每个写入方用独立 source id；无目标时只清除自己归属的提示，避免互相覆盖。
// ============================================================================

use bevy::prelude::*;

use crate::ui::sprite_ui::{spawn_ui_text, UiButton, UiEntity};

/// 通用提示状态
#[derive(Resource, Default)]
pub struct TooltipState {
    pub visible: bool,
    /// 当前提示归属方（0=无 1=按钮Hint 2=背包 3=仓库 4=其他）
    pub source: u16,
    pub title: String,
    pub lines: Vec<String>,
    pub x: f32,
    pub y: f32,
}

impl TooltipState {
    /// 写入方更新提示；无目标时调用以清除自己归属的提示。
    /// 性能（#112）：内容/位置无变化时早退，避免每帧标记 Changed 触发面板重绘。
    pub fn update(&mut self, source: u16, visible: bool, title: String, lines: Vec<String>, x: f32, y: f32) {
        if visible {
            if self.visible
                && self.source == source
                && self.title == title
                && self.lines == lines
                && self.x == x
                && self.y == y
            {
                return;
            }
            self.visible = true;
            self.source = source;
            self.title = title;
            self.lines = lines;
            self.x = x;
            self.y = y;
        } else if self.source == source {
            if !self.visible {
                return;
            }
            self.visible = false;
            self.source = 0;
            self.title.clear();
            self.lines.clear();
        }
    }
}

/// 静态文本提示（挂在 UiButton 上自动生效）
#[derive(Component)]
pub struct TooltipHint(pub String);

/// 面板背景
#[derive(Component)]
pub struct TooltipBg;

/// 面板标题
#[derive(Component)]
pub struct TooltipTitle;

/// 面板行
#[derive(Component)]
pub struct TooltipLine(pub usize);

/// 生成常驻提示面板（背景 + 标题 + 6 行），返回背景实体
pub fn spawn_tooltip_panel(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    font: &Handle<Font>,
) -> Entity {
    let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    let bg = commands
        .spawn((
            UiEntity,
            TooltipBg,
            Sprite {
                image: white,
                color: Color::srgba(0.08, 0.08, 0.12, 0.95),
                custom_size: Some(Vec2::new(10.0, 10.0)),
                ..default()
            },
            bevy::sprite::Anchor::TOP_LEFT,
            Transform::from_xyz(0.0, 0.0, 30.0),
            Visibility::Hidden,
        ))
        .id();
    let title = spawn_ui_text(
        commands, font, "", 0.0, 0.0, 13.0,
        Color::srgb(1.0, 0.9, 0.3), 30.1,
    );
    commands.entity(title).insert(TooltipTitle);
    for i in 0..6usize {
        let t = spawn_ui_text(
            commands, font, "", 0.0, 0.0, 12.0,
            Color::srgb(1.0, 1.0, 0.9), 30.2,
        );
        commands.entity(t).insert(TooltipLine(i));
    }
    bg
}

/// 按钮 Hint 检测（source=1）：悬停 UiButton+TooltipHint 显示
pub fn tooltip_hint_system(
    windows: Query<&Window>,
    buttons: Query<(&UiButton, &TooltipHint)>,
    mut state: ResMut<TooltipState>,
) {
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else { return };
    let mut hit = false;
    for (btn, hint) in &buttons {
        let (x, y, w, h) = btn.rect;
        if cursor.x >= x && cursor.x <= x + w && cursor.y >= y && cursor.y <= y + h {
            state.update(1, true, String::new(), vec![hint.0.clone()], cursor.x, cursor.y);
            hit = true;
            break;
        }
    }
    if !hit {
        state.update(1, false, String::new(), Vec::new(), 0.0, 0.0);
    }
}

/// 面板渲染：内容 + 跟随光标 + 防出屏
pub fn tooltip_panel_system(
    state: Res<TooltipState>,
    mut bg: Query<(&mut Transform, &mut Sprite, &mut Visibility), (With<TooltipBg>, Without<TooltipTitle>, Without<TooltipLine>)>,
    mut title: Query<(&mut Text2d, &mut Transform, &mut Visibility), (With<TooltipTitle>, Without<TooltipBg>, Without<TooltipLine>)>,
    mut lines: Query<(&mut Text2d, &mut Transform, &mut Visibility, &TooltipLine), (Without<TooltipBg>, Without<TooltipTitle>)>,
) {
    // 性能（#112）：TooltipState 未变化（update 已早退）时跳过面板重绘
    if !state.is_changed() {
        return;
    }
    let show = state.visible && (!state.title.is_empty() || !state.lines.is_empty());
    // 估算尺寸：CJK 约 1 字符 = 字号 px
    let mut max_chars = state.title.chars().count().max(1);
    for l in &state.lines {
        max_chars = max_chars.max(l.chars().count());
    }
    let w = (max_chars as f32 * 13.0 + 20.0).clamp(40.0, 500.0);
    let h = 24.0 + state.lines.len() as f32 * 16.0 + 8.0;
    let (mut px, mut py) = (state.x + 16.0, state.y + 16.0);
    if px + w > 1024.0 {
        px = (px - w - 32.0).max(0.0);
    }
    if py + h > 768.0 {
        py = (py - h - 32.0).max(0.0);
    }

    if let Ok((mut tf, mut sp, mut vis)) = bg.single_mut() {
        *vis = if show { Visibility::Visible } else { Visibility::Hidden };
        if show {
            tf.translation.x = px;
            tf.translation.y = -py;
            if let Some(cs) = sp.custom_size.as_mut() {
                *cs = Vec2::new(w, h);
            }
        }
    }
    if let Ok((mut t, mut tf, mut vis)) = title.single_mut() {
        *vis = if show && !state.title.is_empty() { Visibility::Visible } else { Visibility::Hidden };
        if show && !state.title.is_empty() {
            if t.0 != state.title { t.0 = state.title.clone(); }
            tf.translation.x = px + 8.0;
            tf.translation.y = -(py + 5.0);
        }
    }
    for (mut t, mut tf, mut vis, line) in &mut lines {
        let s = state.lines.get(line.0).cloned().unwrap_or_default();
        let visible = show && !s.is_empty();
        *vis = if visible { Visibility::Visible } else { Visibility::Hidden };
        if visible {
            if t.0 != s { t.0 = s; }
            tf.translation.x = px + 8.0;
            tf.translation.y = -(py + 24.0 + line.0 as f32 * 16.0);
        }
    }
}

/// 生成提示面板系统（加载字体后调用 spawn_tooltip_panel）
pub fn spawn_tooltip_panel_system(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<crate::ui::sprite_ui::UiFont>,
) {
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    spawn_tooltip_panel(&mut commands, &mut images, &ui_font.0);
}

/// 清理提示面板（OnExit(Game)）
pub fn despawn_tooltip_panel(mut commands: Commands, q: Query<Entity, With<TooltipBg>>) {
    for e in q.iter() {
        commands.entity(e).despawn();
    }
}

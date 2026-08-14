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
    /// 当前提示归属方（0=无 1=按钮Hint 2=背包 3=仓库 4=其他 5=角色/商品 11=HUD按钮Hint 12=头顶名字）
    pub source: u16,
    pub title: String,
    pub lines: Vec<String>,
    pub x: f32,
    pub y: f32,
    /// 文本是否带黑色描边。C# 依据：物品信息面板标签 OutLine=true（GameScene.cs
    /// ItemLabel 面板 97 处）、头顶名字 OutLine=true（PlayerObject.cs:5336），
    /// 而按钮 Hint 无描边（CMain.cs:518-539 HintTextLabel 未设 OutLine）
    /// → source=1（对话框按钮）/11（HUD 按钮）关闭。
    pub outlined: bool,
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
            self.outlined = !matches!(source, 1 | 11);
        } else if self.source == source {
            if !self.visible {
                return;
            }
            self.visible = false;
            self.source = 0;
            self.title.clear();
            self.lines.clear();
            self.outlined = false;
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

/// tooltip 文本描边副本标记（面板系统按 state.outlined 门控显隐；
/// 区别于其他描边文本的副本，见 outlined_text::OutlineShadow）
#[derive(Component)]
pub struct TooltipOutlineShadow;

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
    let mut line_entities = Vec::with_capacity(6);
    for i in 0..6usize {
        let t = spawn_ui_text(
            commands, font, "", 0.0, 0.0, 12.0,
            Color::srgb(1.0, 1.0, 0.9), 30.2,
        );
        commands.entity(t).insert(TooltipLine(i));
        line_entities.push(t);
    }
    // C# 物品信息面板标签 OutLine=true（GameScene.cs ItemLabel 面板）：描边副本常驻，
    // tooltip_panel_system 按 state.outlined（source=1 按钮 Hint 除外）门控显隐
    for (t, size) in
        std::iter::once((title, 13.0)).chain(line_entities.into_iter().map(|t| (t, 12.0)))
    {
        let shadows = crate::ui::outlined_text::outline_on(
            commands,
            t,
            "",
            font.clone(),
            size,
            bevy::sprite::Anchor::TOP_LEFT,
            false,
        );
        for s in shadows {
            commands.entity(s).insert(TooltipOutlineShadow);
        }
    }
    bg
}

/// 按钮 Hint 检测（source=1）：悬停 UiButton+TooltipHint 显示
pub fn tooltip_hint_system(
    windows: Query<&Window>,
    ui_cameras: Query<(&Camera, &GlobalTransform), With<crate::ui::sprite_ui::UiEntity>>,
    buttons: Query<(&UiButton, &TooltipHint)>,
    mut state: ResMut<TooltipState>,
) {
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else { return };
    // UI 相机 Fixed 1024x768：窗口缩放/DPI 下必须换算成 UI 逻辑坐标，
    // 否则命中与面板定位用物理像素，悬停位置全偏
    let Ok((cam, gtf)) = ui_cameras.single() else { return };
    let Ok(world) = cam.viewport_to_world_2d(gtf, cursor) else { return };
    let cursor = Vec2::new(world.x, -world.y);
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
    mut shadows: Query<&mut Visibility, (With<TooltipOutlineShadow>, Without<TooltipBg>, Without<TooltipTitle>, Without<TooltipLine>)>,
) {
    // 性能（#112）：TooltipState 未变化（update 已早退）时跳过面板重绘
    if !state.is_changed() {
        return;
    }
    let show = state.visible && (!state.title.is_empty() || !state.lines.is_empty());
    // C# 依据：按钮 Hint 无描边（CMain.cs HintTextLabel 未设 OutLine），
    // 物品面板/头顶名字有描边 → source=1 隐藏描边副本
    let outline_vis = if state.outlined && show {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for mut vis in &mut shadows {
        *vis = outline_vis;
    }
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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_early_out_on_same_content() {
        let mut s = TooltipState::default();
        s.update(2, true, "剑".to_string(), vec!["耐久: 10/10".to_string()], 10.0, 20.0);
        assert!(s.visible);
        assert_eq!(s.source, 2);
        // 相同内容再次写入：不应重复标记（visible/source/title/lines 不变）
        let before = (s.visible, s.source, s.title.clone(), s.lines.clone());
        s.update(2, true, "剑".to_string(), vec!["耐久: 10/10".to_string()], 10.0, 20.0);
        assert_eq!(
            (s.visible, s.source, s.title.clone(), s.lines.clone()),
            before
        );
    }

    #[test]
    fn update_clear_only_own_source() {
        let mut s = TooltipState::default();
        s.update(3, true, "仓库".to_string(), vec!["物品".to_string()], 0.0, 0.0);
        // 其他来源清除不影响当前
        s.update(2, false, String::new(), Vec::new(), 0.0, 0.0);
        assert!(s.visible);
        // 归属来源清除生效
        s.update(3, false, String::new(), Vec::new(), 0.0, 0.0);
        assert!(!s.visible);
    }

    /// C# 描边依据：按钮 Hint 无描边（CMain.cs:518-539 HintTextLabel 未设 OutLine），
    /// 物品面板/商品/头顶名字有描边（GameScene.cs ItemLabel 面板、PlayerObject.cs:5336）
    #[test]
    fn update_outlined_gated_by_source() {
        let mut s = TooltipState::default();
        s.update(
            1,
            true,
            String::new(),
            vec!["按钮提示".to_string()],
            0.0,
            0.0,
        );
        assert!(!s.outlined, "对话框按钮 Hint 无描边");
        s.update(
            11,
            true,
            "HUD按钮".to_string(),
            vec!["快捷键".to_string()],
            0.0,
            0.0,
        );
        assert!(
            !s.outlined,
            "HUD 按钮 Hint 无描边（C# MirButton Hint → HintTextLabel）"
        );
        for source in [2u16, 3, 4, 5, 12] {
            s.update(
                source,
                true,
                "物品".to_string(),
                vec!["属性".to_string()],
                0.0,
                0.0,
            );
            assert!(s.outlined, "source={source} 有描边");
        }
        s.update(
            1,
            true,
            String::new(),
            vec!["按钮提示".to_string()],
            0.0,
            0.0,
        );
        assert!(!s.outlined);
        // 清除后回到无描边
        s.update(1, false, String::new(), Vec::new(), 0.0, 0.0);
        assert!(!s.outlined);
    }

    /// 真实面板：物品 tooltip（source=2）→ 描边副本 Inherited；按钮 Hint（source=1）→ Hidden
    #[test]
    fn tooltip_panel_outline_shadows_gated_by_source() {
        use bevy::ecs::system::RunSystemOnce;
        use bevy::ecs::world::CommandQueue;

        use crate::ui::outlined_text::OutlineShadow;

        let mut world = World::new();
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);
        let mut images = Assets::<Image>::default();
        spawn_tooltip_panel(&mut commands, &mut images, &Handle::default());
        queue.apply(&mut world);

        // 面板含 title + 6 行 = 7 个描边文本 × 4 副本
        assert_eq!(
            world
                .query_filtered::<Entity, With<OutlineShadow>>()
                .iter(&world)
                .count(),
            28,
            "title + 6 行各 4 个黑色副本"
        );

        // 物品 tooltip：描边可见（C# ItemLabel 面板 OutLine=true）
        let mut state = TooltipState::default();
        state.update(
            2,
            true,
            "物品".to_string(),
            vec!["属性".to_string()],
            0.0,
            0.0,
        );
        world.insert_resource(state);
        world
            .run_system_once(tooltip_panel_system)
            .expect("面板渲染应成功");
        {
            let mut q = world.query_filtered::<&Visibility, With<TooltipOutlineShadow>>();
            for v in q.iter(&world) {
                assert_eq!(*v, Visibility::Inherited, "物品 tooltip 描边可见");
            }
        }

        // 按钮 Hint：描边隐藏（C# CMain.cs HintTextLabel 未设 OutLine）
        world.resource_mut::<TooltipState>().update(
            1,
            true,
            String::new(),
            vec!["按钮提示".to_string()],
            0.0,
            0.0,
        );
        world
            .run_system_once(tooltip_panel_system)
            .expect("面板渲染应成功");
        {
            let mut q = world.query_filtered::<&Visibility, With<TooltipOutlineShadow>>();
            for v in q.iter(&world) {
                assert_eq!(*v, Visibility::Hidden, "按钮 Hint 无描边");
            }
        }
    }
}

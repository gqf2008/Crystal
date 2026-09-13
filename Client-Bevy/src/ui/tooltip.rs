// ============================================================================
// tooltip - 通用 Tooltip 体系（#93）
// 参考 C#：GameScene.DrawItemHint（物品提示框）+ MirControl.Hint（控件提示）
// 架构：
//   - TooltipState 资源：任意系统写入（source 归属 + 标题/多行/位置）
//   - TooltipHint(String) 组件：挂在 UiButton 上，tooltip_hint_system 自动检测悬停
//   - 常驻面板：背景 + 标题 + 最多 6 行，tooltip_panel_system 渲染（跟随光标、防出屏）
// 写入约定：每个写入方用独立 source id；无目标时只清除自己归属的提示，避免互相覆盖。
// ============================================================================

use bevy::ecs::hierarchy::ChildOf;
use bevy::prelude::*;

use crate::ui::sprite_ui::UiButton;

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

/// #2771 通用按钮 Hint：挂在 **bevy UI `Button`**（`theme::spawn_icon_button` 那一类）上，
/// 与 `TooltipHint`（sprite-UI `UiButton` + `rect`）区分——后者靠 `UiButton.rect` 命中，
/// 前者靠「沿 `ChildOf` 链累加各级 `Node.left/top` 得到的绝对矩形」命中，因此**光标探针可驱动**
/// （无焦点环境可实机验证），且不受面板拖动/嵌套容器影响。
#[derive(Component)]
pub struct UiHint {
    pub text: String,
}

/// 通用按钮 Hint 的归属方（`TooltipState.source`，与其它写入方隔离）：#2771
pub const UI_HINT_SOURCE: u16 = 8;

/// 面板背景
#[derive(Component)]
pub struct TooltipBg;

/// 面板标题
#[derive(Component)]
pub struct TooltipTitle;

/// 面板行
#[derive(Component)]
pub struct TooltipLine(pub usize);

/// 提示面板层级（根节点 `GlobalZIndex`）：必须高于**所有**对话框（当前最大
/// `amount_box` = 60，见 `dialogs/amount_box.rs`）——C# 侧 `HintTextLabel`
/// 恒画在所有控件之后（`CMain.cs:534-540`），Bevy 旧实现把面板画在 sprite 层，
/// 而同一相机里 bevy_ui 节点整体画在 sprite 之后 → 面板内的按钮 Hint 被自己
/// 所属对话框盖住（光标在面板内时+16 偏移的提示框必然重叠，实机不可见）。
pub const TOOLTIP_Z: i32 = 90;

/// 生成常驻提示面板（背景 + 标题 + 6 行），返回背景实体。
///
/// #2775：改为 **bevy_ui 节点**（根节点 `GlobalZIndex(TOOLTIP_Z)` + 子文本），
/// 否则被对话框（bevy_ui）整体遮挡。描边用 `outlined_text::spawn_outlined_label`
/// （4 向 1px 黑副本的兄弟层级方案），内容变化由 `sync_outline_ui_system` 同步。
pub fn spawn_tooltip_panel(commands: &mut Commands, font: &Handle<Font>) -> Entity {
    let bg = commands
        .spawn((
            TooltipBg,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Px(10.0),
                height: Val::Px(10.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.08, 0.08, 0.12, 0.95)),
            GlobalZIndex(TOOLTIP_Z),
            Visibility::Hidden,
        ))
        .id();
    commands.entity(bg).with_children(|p| {
        // C# tooltip 文本全部有描边：物品信息面板标签 OutLine=true（GameScene.cs
        // CreateItemLabel），按钮 Hint 的 HintTextLabel 未显式设 OutLine 但
        // MirLabel 构造器默认 _outLine=true（MirLabel.cs:181-182）→ 同样有描边。
        crate::ui::outlined_text::spawn_outlined_label(
            p,
            font.clone(),
            "",
            8.0,
            5.0,
            13.0,
            Color::srgb(1.0, 0.9, 0.3),
            1,
        )
        .insert(TooltipTitle);
        for i in 0..6usize {
            crate::ui::outlined_text::spawn_outlined_label(
                p,
                font.clone(),
                "",
                8.0,
                24.0 + i as f32 * 16.0,
                12.0,
                Color::srgb(1.0, 1.0, 0.9),
                1,
            )
            .insert(TooltipLine(i));
        }
    });
    bg
}

/// #2771 通用按钮 Hint 检测（source=8）：悬停带 UiHint 的 bevy UI 按钮显示其文案。
///
/// 命中 = 光标（探针优先）落在「沿 ChildOf 链累加各级 Node.left/top 得到的绝对矩形」内；
/// 同帧多个命中取 ZIndex 最大者。沿父链累加而非查「同 kind 的对话框根」，因为同一
/// DialogKind 可能同时存在多个根面板（如 Group 的邀请确认框与主面板）。
pub fn ui_hint_system(
    windows: Query<&Window>,
    probe: Res<crate::control::CursorProbe>,
    ui_cameras: Query<(&Camera, &GlobalTransform), With<crate::ui::sprite_ui::UiEntity>>,
    nodes: Query<&Node>,
    parents: Query<&ChildOf>,
    hints: Query<(Entity, &UiHint, &Node, &ComputedNode, &InheritedVisibility, &ZIndex)>,
    mut state: ResMut<TooltipState>,
) {
    let clear = |state: &mut TooltipState| {
        state.update(UI_HINT_SOURCE, false, String::new(), Vec::new(), 0.0, 0.0);
    };
    let Some(raw) = crate::control::resolve_cursor(
        probe.pos,
        windows.single().ok().and_then(|w| w.cursor_position()),
    ) else {
        clear(&mut state);
        return;
    };
    let cursor = match ui_cameras.single() {
        Ok((cam, gtf)) => match cam.viewport_to_world_2d(gtf, raw) {
            Ok(w) => Vec2::new(w.x, -w.y),
            Err(_) => {
                clear(&mut state);
                return;
            }
        },
        Err(_) => {
            clear(&mut state);
            return;
        }
    };
    let mut topmost: Option<(&str, i32)> = None;
    for (entity, hint, node, computed, vis, z) in &hints {
        if !vis.get() {
            continue;
        }
        // #2775：空文案不参与命中——否则会写入 `lines = [""]`，`lines` 非空但无内容，
        // 面板显示成一个空框（实机复现：队伍成员行在无成员时留下空提示框）
        if hint.text.is_empty() {
            continue;
        }
        let (w, h) = match ui_hint_size(node, computed) {
            Some(s) => s,
            None => continue,
        };
        let (x, y) = match abs_ui_origin(&nodes, &parents, entity) {
            Some(p) => p,
            None => continue,
        };
        if ui_hint_hit((x, y, w, h), cursor) {
            let z = z.0;
            if topmost.map(|(_, top_z)| z > top_z).unwrap_or(true) {
                topmost = Some((hint.text.as_str(), z));
            }
        }
    }
    match topmost {
        Some((text, _)) => state.update(
            UI_HINT_SOURCE,
            true,
            String::new(),
            vec![text.to_string()],
            cursor.x,
            cursor.y,
        ),
        None => clear(&mut state),
    }
}

/// UiHint 按钮自身的绝对左上角（沿 ChildOf 链累加各级 Node 的 Px left/top）。
fn abs_ui_origin(
    nodes: &Query<&Node>,
    parents: &Query<&ChildOf>,
    entity: Entity,
) -> Option<(f32, f32)> {
    let (mut x, mut y) = (0.0f32, 0.0f32);
    let mut cur = entity;
    loop {
        let node = nodes.get(cur).ok()?;
        if let (Val::Px(l), Val::Px(t)) = (node.left, node.top) {
            x += l;
            y += t;
        }
        match parents.get(cur).ok() {
            Some(parent) => cur = parent.parent(),
            None => break,
        }
    }
    Some((x, y))
}

/// #2775：Hint 命中矩形尺寸——显式 `Px` 控件用声明尺寸（与 C# 控件尺寸同源），
/// 自动尺寸（`spawn_label` 这类文本按钮 `width/height = Auto`，如排行页签）回退到
/// 布局结果；`ComputedNode` 存的是**物理像素**，乘 `inverse_scale_factor` 转逻辑像素。
fn ui_hint_size(node: &Node, computed: &ComputedNode) -> Option<(f32, f32)> {
    match (node.width, node.height) {
        (Val::Px(w), Val::Px(h)) => Some((w, h)),
        _ => {
            if computed.is_empty() {
                return None;
            }
            let s = computed.size() * computed.inverse_scale_factor;
            Some((s.x, s.y))
        }
    }
}

/// 通用按钮 Hint 命中（绝对 UI 矩形 + 光标；边界含等号）
fn ui_hint_hit(rect: (f32, f32, f32, f32), cursor: Vec2) -> bool {
    let (x, y, w, h) = rect;
    cursor.x >= x && cursor.x <= x + w && cursor.y >= y && cursor.y <= y + h
}

/// 按钮 Hint 检测（source=1）：悬停 UiButton+TooltipHint 显示
pub fn tooltip_hint_system(
    windows: Query<&Window>,
    probe: Res<crate::control::CursorProbe>,
    ui_cameras: Query<(&Camera, &GlobalTransform), With<crate::ui::sprite_ui::UiEntity>>,
    buttons: Query<(&UiButton, &TooltipHint, &InheritedVisibility, &Transform)>,
    mut state: ResMut<TooltipState>,
) {
    let Ok(window) = windows.single() else { return };
    // #2771：无焦点环境（自动化）用控制接口的光标探针驱动
    let Some(cursor) = crate::control::resolve_cursor(probe.pos, window.cursor_position()) else {
        return;
    };
    // UI 相机 Fixed 1024x768：窗口缩放/DPI 下必须换算成 UI 逻辑坐标，
    // 否则命中与面板定位用物理像素，悬停位置全偏
    let Ok((cam, gtf)) = ui_cameras.single() else { return };
    let Ok(world) = cam.viewport_to_world_2d(gtf, cursor) else { return };
    let cursor = Vec2::new(world.x, -world.y);
    let mut topmost: Option<(&TooltipHint, f32)> = None;
    for (btn, hint, inherited, transform) in &buttons {
        if !inherited.get() {
            continue;
        }
        let (x, y, w, h) = btn.rect;
        if cursor.x >= x
            && cursor.x <= x + w
            && cursor.y >= y
            && cursor.y <= y + h
            && topmost
                .map(|(_, z)| transform.translation.z > z)
                .unwrap_or(true)
        {
            topmost = Some((hint, transform.translation.z));
        }
    }
    if let Some((hint, _)) = topmost {
        state.update(1, true, String::new(), vec![hint.0.clone()], cursor.x, cursor.y);
    } else {
        state.update(1, false, String::new(), Vec::new(), 0.0, 0.0);
    }
}

/// 面板渲染：内容 + 跟随光标 + 防出屏
///
/// #2775：面板现在是 **bevy_ui 节点**，而 bevy_ui 的 `FocusPolicy` 默认是 `Block`
/// （bevy_ui-0.19.1 `focus.rs:323`）——若提示框覆盖光标点，就会抢掉下方按钮的
/// `Interaction`。本函数保证绝不发生：跟随偏移 +16 时框的左/上边在光标右下，
/// 翻转时框的右/下边在光标左上（见 `tooltip_origin` 与其单测）。
pub fn tooltip_panel_system(
    state: Res<TooltipState>,
    mut bg: Query<(&mut Node, &mut Visibility), (With<TooltipBg>, Without<TooltipTitle>, Without<TooltipLine>)>,
    mut title: Query<(&mut Text, &mut Visibility), (With<TooltipTitle>, Without<TooltipBg>, Without<TooltipLine>)>,
    mut lines: Query<(&mut Text, &mut Visibility, &TooltipLine), (Without<TooltipBg>, Without<TooltipTitle>)>,
) {
        // 性能（#112）：TooltipState 未变化（update 已早退）时跳过面板重绘
    if !state.is_changed() {
        return;
    }
    // #2775：`lines = [""]` 这类「有元素但无内容」的写入同样不显示（各写入方兜底）
    let show =
        state.visible && (!state.title.is_empty() || state.lines.iter().any(|l| !l.is_empty()));
    // 描边副本（outlined_text 的兄弟层级副本）随父节点 bg 显隐自动跟随：
    // bevy_ui 里子实体恒画在父之后且 `Inherited` 继承父可见性，无需单独同步；
    // 正文内容变化由 `sync_outline_ui_system` 复制到 4 个副本（见插件注册顺序）。
    // 估算尺寸：CJK 约 1 字符 = 字号 px
    let mut max_chars = state.title.chars().count().max(1);
    for l in &state.lines {
        max_chars = max_chars.max(l.chars().count());
    }
    let w = (max_chars as f32 * 13.0 + 20.0).clamp(40.0, 500.0);
    let h = 24.0 + state.lines.len() as f32 * 16.0 + 8.0;
    let (px, py) = tooltip_origin(state.x, state.y, w, h);

    if let Ok((mut node, mut vis)) = bg.single_mut() {
        *vis = if show { Visibility::Visible } else { Visibility::Hidden };
        if show {
            node.left = Val::Px(px);
            node.top = Val::Px(py);
            node.width = Val::Px(w);
            node.height = Val::Px(h);
        }
    }
    if let Ok((mut t, mut vis)) = title.single_mut() {
        *vis = if show && !state.title.is_empty() { Visibility::Visible } else { Visibility::Hidden };
        if show && !state.title.is_empty() {
            if t.0 != state.title { t.0 = state.title.clone(); }
        } else if !t.0.is_empty() {
            // 隐藏时必须清空正文：描边副本由 `sync_outline_ui_system` 按正文内容同步，
            // 只隐藏正文会让 4 个黑副本留在屏幕上（实机复现：切换提示对象后残留暗字）
            t.0.clear();
        }
    }
    for (mut t, mut vis, line) in &mut lines {
        let s = state.lines.get(line.0).cloned().unwrap_or_default();
        let visible = show && !s.is_empty();
        *vis = if visible { Visibility::Visible } else { Visibility::Hidden };
        if visible {
            if t.0 != s { t.0 = s; }
        } else if !t.0.is_empty() {
            t.0.clear();
        }
    }
}

/// 提示框左上角（对齐 C# 光标跟随：+16；接近屏幕右下角时翻到光标左上侧防出屏）。
///
/// **关键不变量**：光标点恒在框外（框的边最多贴到光标，永不越过），
/// 故面板（bevy_ui，`FocusPolicy` 默认 `Block`）不会挡住它下方按钮的点击。
fn tooltip_origin(x: f32, y: f32, w: f32, h: f32) -> (f32, f32) {
    let (mut px, mut py) = (x + 16.0, y + 16.0);
    if px + w > 1024.0 {
        px = (px - w - 32.0).max(0.0);
    }
    if py + h > 768.0 {
        py = (py - h - 32.0).max(0.0);
    }
    (px, py)
}

/// 生成提示面板系统（加载字体后调用 spawn_tooltip_panel）
pub fn spawn_tooltip_panel_system(
    mut commands: Commands,
    mut fonts: ResMut<Assets<Font>>,
    mut cjk_font: ResMut<crate::ui::sprite_ui::UiCjkFont>,
) {
    // #2767：提示文本是**动态文本**（每次悬停换内容 → 每帧重排版）。parley 的 Hani 脚本回退
    // 只在实体首次排版时生效，用 Arial（`UiFont`）会在换文本后退化成 .notdef 豆腐（#2599）：
    // 悬停怪物「怪物5」实测渲染成「□□5」。与 NPC/公告等动态文本一致改用共享宋体主字体。
    let font = crate::ui::sprite_ui::shared_cjk_font(&mut fonts, &mut cjk_font);
    spawn_tooltip_panel(&mut commands, &font);
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

    /// #2775：文本按钮（Auto 尺寸，如排行页签）靠布局结果命中；显式 Px 控件仍用声明尺寸
    #[test]
    fn ui_hint_size_prefers_declared_px_then_layout() {
        let px = Node {
            width: Val::Px(24.0),
            height: Val::Px(22.0),
            ..default()
        };
        let laid_out = ComputedNode {
            size: Vec2::new(40.0, 28.0),
            inverse_scale_factor: 0.5,
            ..ComputedNode::default()
        };
        assert_eq!(ui_hint_size(&px, &laid_out), Some((24.0, 22.0)), "显式 Px 用声明值");
        let auto = Node {
            width: Val::Auto,
            height: Val::Auto,
            ..default()
        };
        assert_eq!(
            ui_hint_size(&auto, &laid_out),
            Some((20.0, 14.0)),
            "Auto 尺寸走布局结果（物理像素 × inverse_scale_factor）"
        );
        assert_eq!(
            ui_hint_size(&auto, &ComputedNode::default()),
            None,
            "尚未布局（0 尺寸）不得命中"
        );
    }

    /// #2771：通用按钮 Hint 的命中（绝对 UI 矩形 + 光标；边界含等号）
    #[test]
    fn ui_hint_hit_covers_rect_and_borders() {
        let rect = (100.0, 200.0, 24.0, 24.0);
        assert!(ui_hint_hit(rect, Vec2::new(112.0, 212.0)), "中心命中");
        assert!(ui_hint_hit(rect, Vec2::new(100.0, 200.0)), "左上角含边界");
        assert!(ui_hint_hit(rect, Vec2::new(124.0, 224.0)), "右下角含边界");
        assert!(!ui_hint_hit(rect, Vec2::new(99.0, 212.0)), "左外侧不命中");
        assert!(!ui_hint_hit(rect, Vec2::new(112.0, 225.0)), "下外侧不命中");
    }

    /// #2775：提示框永不含光标点（+16 跟随 / 贴边翻转都不越过）——面板是 bevy_ui 节点，
    /// `FocusPolicy` 默认 `Block`，一旦覆盖光标就会抢掉下方按钮的点击。
    #[test]
    fn tooltip_origin_never_covers_cursor() {
        let (w, h) = (200.0, 100.0);
        for (x, y) in [
            (0.0, 0.0),
            (100.0, 200.0),
            (900.0, 300.0),
            (1000.0, 760.0),
            (1024.0, 768.0),
            (500.0, 700.0),
        ] {
            let (px, py) = tooltip_origin(x, y, w, h);
            let inside = x >= px && x <= px + w && y >= py && y <= py + h;
            assert!(
                !inside,
                "光标 ({x},{y}) 落在提示框 ({px},{py},{w},{h}) 内会拦截点击"
            );
            assert!(px >= 0.0 && py >= 0.0, "提示框不得出屏左上");
        }
    }

    /// #2775：面板是 bevy_ui 根节点、`GlobalZIndex` 必须高于所有对话框（当前最大 60），
    /// 否则面板内按钮的 Hint 会被自己所属对话框整块盖住（旧 sprite 层级方案的回归点）。
    #[test]
    fn tooltip_z_sits_above_every_dialog() {
        assert!(
            TOOLTIP_Z > 60,
            "提示面板层级（{TOOLTIP_Z}）必须高于 amount_box(60) 等全部对话框"
        );
    }

    /// C# MirLabel 构造器默认 _outLine=true（MirLabel.cs:181-182）→ 按钮 Hint
    /// （CMain.cs:534-540 HintTextLabel 未显式设 OutLine）同样有描边。
    /// #2775：面板改 bevy_ui 后，描边副本是 `outlined_text` 的兄弟层级副本
    /// （title + 6 行各 4 个 = 28），随面板 bg 显隐跟随；正文与副本内容由
    /// `sync_outline_ui_system` 同步。
    #[test]
    fn tooltip_panel_is_ui_node_with_outlined_copies() {
        use bevy::ecs::system::RunSystemOnce;
        use bevy::ecs::world::CommandQueue;

        use crate::ui::outlined_text::{OutlineUiShadow, OutlineUiShadows};

        let mut world = World::new();
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);
        spawn_tooltip_panel(&mut commands, &Handle::default());
        queue.apply(&mut world);

        // 面板背景是根 UI 节点且带最高层级（不是旧的 sprite + Transform）
        let bg_z = world
            .query_filtered::<&GlobalZIndex, With<TooltipBg>>()
            .iter(&world)
            .next()
            .copied()
            .expect("面板背景应是根 UI 节点");
        assert_eq!(bg_z, GlobalZIndex(TOOLTIP_Z));

        // title + 6 行 = 7 个描边文本 × 4 副本
        assert_eq!(
            world
                .query_filtered::<Entity, With<OutlineUiShadow>>()
                .iter(&world)
                .count(),
            28,
            "title + 6 行各 4 个黑色副本"
        );
        assert_eq!(
            world
                .query_filtered::<Entity, With<OutlineUiShadows>>()
                .iter(&world)
                .count(),
            7,
            "7 个正文各自记录 4 个副本 id"
        );

        // 按钮 Hint（source=1）：面板显示 → 背景 Visible、行文本写入、位置跟随光标
        let mut state = TooltipState::default();
        state.update(
            1,
            true,
            String::new(),
            vec!["按钮提示".to_string()],
            100.0,
            200.0,
        );
        world.insert_resource(state);
        world
            .run_system_once(tooltip_panel_system)
            .expect("面板渲染应成功");
        {
            let (vis, node) = world
                .query_filtered::<(&Visibility, &Node), With<TooltipBg>>()
                .iter(&world)
                .next()
                .expect("面板背景存在");
            assert_eq!(*vis, Visibility::Visible, "有内容时面板可见");
            assert_eq!(node.left, Val::Px(116.0), "跟随光标 +16");
            assert_eq!(node.top, Val::Px(216.0), "跟随光标 +16");
        }
        let line0 = world
            .query_filtered::<(&Text, &TooltipLine), Without<TooltipTitle>>()
            .iter(&world)
            .find(|(_, l)| l.0 == 0)
            .map(|(t, _)| t.0.clone())
            .expect("第 0 行存在");
        assert_eq!(line0, "按钮提示");

        // #2775：`lines = [""]`（有元素无内容）不得显示空框
        world.resource_mut::<TooltipState>().update(
            8,
            true,
            String::new(),
            vec![String::new()],
            100.0,
            200.0,
        );
        world
            .run_system_once(tooltip_panel_system)
            .expect("面板渲染应成功");
        let vis = world
            .query_filtered::<&Visibility, With<TooltipBg>>()
            .iter(&world)
            .next()
            .copied()
            .expect("面板背景存在");
        assert_eq!(vis, Visibility::Hidden, "空内容不得显示空提示框");

        // 清除 → 面板隐藏
        world
            .resource_mut::<TooltipState>()
            .update(1, false, String::new(), Vec::new(), 0.0, 0.0);
        world
            .run_system_once(tooltip_panel_system)
            .expect("面板渲染应成功");
        let vis = world
            .query_filtered::<&Visibility, With<TooltipBg>>()
            .iter(&world)
            .next()
            .copied()
            .expect("面板背景存在");
        assert_eq!(vis, Visibility::Hidden, "面板隐藏");
    }
}

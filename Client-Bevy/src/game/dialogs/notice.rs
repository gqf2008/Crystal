// ============================================================================
// 公告对话框（M50 → #2602 批R 重写对齐 C# NoticeDialog）
// C# NoticeDialog（MirScenes/Dialogs/NoticeDialog.cs）：
//   - 背景 Prguse[961]（实测 316x466）@ 屏心偏上 ((SW-W)/2, (SH-H)/3)
//   - 标题 NameLabel Bold10 BurlyWood @(30,6)（宋体无 Bold 面，简化为常规）
//   - 正文 19 行/页（MaximumLines），TextLabel Size(420,20) WordBreak @(25,50+20i)
//   - 超 19 行才显示：Up Prguse2[470-472]@(293,33) / Down [473-475]@(293,418) /
//     可拖 PositionBar [205/206]@(293,46) y∈[46,399]，interval=400/(count-19)
//   - 滚轮翻页；Ok Title[193-195]@(120,436) / Close [360-362]@(289,3) 均关闭
//   - 富文本：{t/colour} 着色段（KnownColor）+ (t/link) 链接段（黄/悬停红，
//     http 开浏览器）——与 NPC 标记同「基础白字 + 独立叠加标签」架构（#2599）
// ============================================================================

use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{shared_cjk_font, UiCjkFont, UiFont};
use crate::ui::text_markup::{est_text_width, known_color, wrap_text};
use crate::ui::theme::{
    load_lib_image, spawn_close_button, spawn_icon_button, spawn_label, spawn_panel,
};

/// #2892 批B：面板精灵（C# `NoticeDialog.Index = 961; Library = Libraries.Prguse`）
pub const PANEL: (LibraryName, usize) = (LibraryName::Prguse, 961);

// —— 布局常量（NoticeDialog.cs；Prguse[961] 实测 316x466）——
/// 贴图用的**图头**尺寸（`Library.Draw` 原样 1:1 铺）
pub const BG_W: f32 = 316.0;
pub const BG_H: f32 = 466.0;
/// `GetTrueSize(Prguse[961])` 真宽 = **314**（图头 316 的最右 2 列全透明，高不裁）。
pub const BG_TRUE_W: f32 = 314.0;
/// C# `NoticeDialog.cs:33` `Location = ((SW - Size.Width)/2, (SH - Size.Height)/3)`，
/// `Size` = `GetTrueSize` ⇒ x = **(1024-314)/2 = 355**、y = (768-466)/3 = 100（逐项整除）。
/// 本端此前按图头算成 354（§3.2co 批④）。注意 y 用的是 **/3** 不是 /2。
pub const ORIGIN: (f32, f32) = (
    ((1024.0 - BG_TRUE_W) / 2.0).trunc(),
    ((768.0 - BG_H) / 3.0).trunc(),
);
pub const MAX_LINES: usize = 19;
pub const LINE_X: f32 = 25.0;
pub const LINE_Y0: f32 = 50.0;
pub const LINE_DY: f32 = 20.0;
pub const LINE_FONT_PX: f32 = 10.0;
/// 行宽（C# TextLabel Size(420,20) WordBreak）
pub const LINE_WRAP_W: f32 = 420.0;
pub const TITLE_REL: (f32, f32) = (30.0, 6.0);
/// BurlyWood = (222,184,135)
pub const TITLE_COLOR: Color = Color::srgb(0.87, 0.72, 0.53);
pub const CLOSE_REL: (f32, f32) = (289.0, 3.0);
pub const OK_REL: (f32, f32) = (120.0, 436.0);
pub const UP_REL: (f32, f32) = (293.0, 33.0);
pub const DOWN_REL: (f32, f32) = (293.0, 418.0);
pub const BAR_X: f32 = 293.0;
pub const BAR_Y_MIN: f32 = 46.0;
pub const BAR_Y_MAX: f32 = 399.0;
pub const BAR_W: f32 = 12.0;
pub const BAR_H: f32 = 18.0;
/// C# interval 基数：400 / (count - MaximumLines)（:143/:176）
pub const BAR_TRAVEL: f32 = 400.0;
/// Up/Down 按钮 art 原生尺寸（`Prguse2[470]/[473]` 实测 12x12）。
///
/// C# 给这两个钮写了 `Size = new Size(16,14)`（`NoticeDialog.cs:70-98`），但**绘制**走的是
/// `Libraries.Draw(Index, Location, …)` 的 art 原生尺寸，`Size` 只是命中框；本端此前把 art
/// 压进 20x20（§3.2bv 原版帧模板匹配：原版 12x12 落点不符率 0.0000，本端 20x20 拉伸出 0.52）。
pub const ARROW_W: f32 = 12.0;
pub const ARROW_H: f32 = 12.0;
/// Ok 按钮 art 原生尺寸（`Title[193]` 68x25；C# `NoticeDialog.cs:63-75` 未设 `Size` ⇒ 命中框 = art 尺寸）
pub const OK_W: f32 = 68.0;
pub const OK_H: f32 = 25.0;

/// 滑块间隔（调用方保证 count > MAX_LINES）。C# `400 / (count - 19)` 是
/// **int/int 截断除**（NoticeDialog.cs:143/:176）——f32 除在非整除 count
/// 时 y 定位偏 ~8px 且正反换算不自洽（审查 M5），必须整数除后转 f32
fn bar_interval(count: usize) -> f32 {
    (BAR_TRAVEL as i32 / (count as i32 - MAX_LINES as i32)) as f32
}

/// 首行下标 → 滑块 y（C# UpdatePositionBar :172-185）
fn position_bar_y(index: usize, count: usize) -> f32 {
    if count <= MAX_LINES {
        return BAR_Y_MIN;
    }
    (BAR_Y_MIN + index as f32 * bar_interval(count)).clamp(BAR_Y_MIN, BAR_Y_MAX)
}

/// 滑块 y → 首行下标（C# PositionBar_OnMoving :134-152）
fn index_from_bar_y(y: f32, count: usize) -> usize {
    if count <= MAX_LINES {
        return 0;
    }
    let max = count - MAX_LINES;
    (((y.clamp(BAR_Y_MIN, BAR_Y_MAX) - BAR_Y_MIN) / bar_interval(count)).floor() as usize).min(max)
}

/// 状态（#256：S.UpdateNotice 服务器公告，C# Notice: Title + Message）
#[derive(Resource, Default)]
pub struct NoticeState {
    pub title: String,
    pub message: String,
    /// message 按 420px 折行后的显示行（Update 时重算，C# Split("\r\n") + WordBreak）
    pub lines: Vec<String>,
    /// 当前首行下标（C# _index）
    pub index: usize,
    /// CJK 主字体（与 NPC 对话共享宋体资产，#2599 重排版豆腐教训）
    pub cjk_font: Handle<Font>,
}

#[derive(Component)]
pub struct NoticeWidget;

/// 滚动件（Up/Down/PositionBar）——超 19 行才显示（C# NewText :218-243）
#[derive(Component)]
pub struct NoticeScrollWidget;

/// 对话框背景（原点基准：所有相对坐标 + 拖动跟随都从它的 Transform 推算）
#[derive(Component)]
pub struct NoticeBg;

/// 标题标签
#[derive(Component)]
pub struct NoticeTitle;

/// PositionBar 滑块（与 Up/Down 的 NoticeScrollWidget 区分：本系统独占写它的 Transform）
#[derive(Component)]
pub struct NoticeBar;

/// 按钮种类（单查询分发，避免多 With<marker> 查询 B0001）
#[derive(Component)]
pub struct NoticeBtn(pub NoticeBtnKind);

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum NoticeBtnKind {
    Close,
    Ok,
    Up,
    Down,
}

#[derive(Component)]
pub struct NoticeLine(usize);

/// 行渲染缓存（窗口行未变不重建叠加标签）
#[derive(Component, Default)]
struct NoticeLineSrc {
    src: String,
    hover: Option<usize>,
    overlays: Vec<Entity>,
}

/// PositionBar 拖动状态（C# Movable + OnMoving）
#[derive(Default)]
struct BarDrag {
    dragging: bool,
    grab_offset: f32,
}

pub struct NoticePlugin;

impl Plugin for NoticePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NoticeState>();
        app.add_systems(OnEnter(AppState::Game), spawn_notice);
        app.add_systems(OnExit(AppState::Game), cleanup_notice);
        app.add_systems(
            Update,
            (notice_server_events, notice_ui_system)
                .chain()
                .after(crate::network::network_system)
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_notice(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_notice(
    mut commands: Commands,
    mut notice: ResMut<NoticeState>,
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
    if !notice.cjk_font.is_strong() {
        notice.cjk_font = shared_cjk_font(&mut fonts, &mut cjk_font);
    }
    let font = notice.cjk_font.clone();
    let (ox, oy) = ORIGIN;

    // 面板 Prguse[961]（316x466 @ 屏心偏上）
    let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 961) else {
        return;
    };
    let panel = spawn_panel(&mut commands, bg, ox, oy, BG_W, BG_H, 30);
    commands
        .entity(panel)
        .insert((DialogRoot(DialogKind::Notice), NoticeWidget));

    commands.entity(panel).with_children(|p| {
        // 标题（CJK 字体）
        spawn_label(
            p,
            &font,
            "",
            TITLE_REL.0,
            TITLE_REL.1,
            LINE_FONT_PX,
            TITLE_COLOR,
            9,
        )
        .insert(NoticeTitle);
        // Close Prguse2[360/361/362] @(289,3)（C# `NoticeDialog.cs:58`，无 Size → 原生 24x21）
        if let Some(mut btn) =
            spawn_close_button(p, &mut libs, &mut images, CLOSE_REL.0, CLOSE_REL.1, 10)
        {
            btn.insert(NoticeBtn(NoticeBtnKind::Close));
        }
        // Ok / Up / Down（图标按钮；宽高 = art 原生尺寸，见 `ARROW_W`/`OK_W` 注释）
        let buttons: [(
            NoticeBtnKind,
            LibraryName,
            usize,
            usize,
            usize,
            f32,
            f32,
            f32,
            f32,
        ); 3] = [
            (
                NoticeBtnKind::Ok,
                LibraryName::Title,
                193,
                194,
                195,
                OK_REL.0,
                OK_REL.1,
                OK_W,
                OK_H,
            ),
            (
                NoticeBtnKind::Up,
                LibraryName::Prguse2,
                470,
                471,
                472,
                UP_REL.0,
                UP_REL.1,
                ARROW_W,
                ARROW_H,
            ),
            (
                NoticeBtnKind::Down,
                LibraryName::Prguse2,
                473,
                474,
                475,
                DOWN_REL.0,
                DOWN_REL.1,
                ARROW_W,
                ARROW_H,
            ),
        ];
        for (kind, lib, n, h, pr, rx, ry, bw, bh) in buttons {
            if let (Some(nh), Some(hh), Some(ph)) = (
                load_lib_image(&mut libs, &mut images, lib, n),
                load_lib_image(&mut libs, &mut images, lib, h),
                load_lib_image(&mut libs, &mut images, lib, pr),
            ) {
                spawn_icon_button(p, nh, hh, ph, rx, ry, bw, bh, 10).insert(NoticeBtn(kind));
            }
        }
        // PositionBar `Prguse2[205/206]` @(293,46)（C# `NoticeDialog.cs:100-118`：
        // Movable、可拖范围 y∈[46,399]）；**只在超 19 行时显示**（`NewText` :218-243）。
        // 原版实机（26 行公告）实测落点 (648,146) = 面板 (354,100) + 面板内 (293,46)，
        // 12x18 art 不符率 0.0000（§3.2bv）。
        if let (Some(n), Some(h)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 205),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 206),
        ) {
            spawn_icon_button(p, n, h.clone(), h, BAR_X, BAR_Y_MIN, BAR_W, BAR_H, 10)
                .insert((NoticeBar, Visibility::Hidden));
        }
        // 19 行正文
        for i in 0..MAX_LINES {
            spawn_label(
                p,
                &font,
                "",
                LINE_X,
                LINE_Y0 + i as f32 * LINE_DY,
                LINE_FONT_PX,
                Color::WHITE,
                9,
            )
            .insert((
                NoticeLine(i),
                NoticeLineSrc::default(),
                FontHinting::Enabled,
            ));
        }
    });
}

/// 显隐 + 滚动 + 渲染
#[allow(clippy::type_complexity)]
fn notice_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut notice: ResMut<NoticeState>,
    mut wheels: MessageReader<MouseWheel>,
    buttons: Query<(Entity, &Interaction, &NoticeBtn)>,
    mut widgets: Query<
        &mut Visibility,
        (
            With<NoticeWidget>,
            Without<NoticeTitle>,
            Without<NoticeLine>,
            Without<NoticeBar>,
        ),
    >,
    // PositionBar（`Prguse2[205/206]`，C# `NoticeDialog.PositionBar`）：显隐 + 拖动跟手
    mut bar: Query<
        (&Interaction, &mut Node, &mut Visibility),
        (With<NoticeBar>, Without<NoticeLine>),
    >,
    // 面板节点（取实时原点：本窗 `Movable = true`，拖走以后条仍要按**面板内**坐标走）
    panel: Query<&Node, (With<NoticeWidget>, Without<NoticeBar>)>,
    cursor_src: crate::control::CursorSource,
    mut title: Query<&mut Text, (With<NoticeTitle>, Without<NoticeLine>)>,
    mut lines: Query<(&mut Text, &NoticeLine), Without<NoticeTitle>>,
    mut prev_inter: Local<std::collections::HashMap<Entity, Interaction>>,
    mut index: Local<usize>,
) {
    fn edge(
        e: Entity,
        inter: &Interaction,
        prev: &mut std::collections::HashMap<Entity, Interaction>,
    ) -> bool {
        let was = prev.insert(e, *inter);
        *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
    }
    let open = mgr.is_open(DialogKind::Notice);
    let count = notice.lines.len();
    let scrollable = count > MAX_LINES;
    for mut vis in widgets.iter_mut() {
        *vis = if open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !open {
        *index = 0;
        return;
    }
    // Close/Ok 关闭；Up/Down 翻一行（bevy_ui Interaction 边沿）
    for (e, inter, k) in &buttons {
        if !edge(e, inter, &mut prev_inter) {
            continue;
        }
        match k.0 {
            NoticeBtnKind::Close | NoticeBtnKind::Ok => {
                mgr.close(DialogKind::Notice);
            }
            NoticeBtnKind::Up => {
                if *index > 0 {
                    *index -= 1;
                }
            }
            NoticeBtnKind::Down => {
                if *index + MAX_LINES < count {
                    *index += 1;
                }
            }
        }
    }
    // 滚轮滚动
    let mut notches = 0.0f32;
    for ev in wheels.read() {
        notches += match ev.unit {
            MouseScrollUnit::Line => ev.y,
            MouseScrollUnit::Pixel => ev.y / 120.0,
        };
    }
    if notches != 0.0 && scrollable {
        let max = (count - MAX_LINES) as i32;
        *index = ((*index as i32) - notches.round() as i32).clamp(0, max) as usize;
    }
    *index = (*index).min(count.saturating_sub(MAX_LINES));
    // PositionBar：C# `NewText` :218-243 只在 `lines.Count > MaximumLines` 时显示；
    // 拖动按 `PositionBar_OnMoving` :134-152（`_index = floor((y-46)/interval)`，y 夹 [46,399]）。
    // 原版实机（26 行公告）面板内落点 (293,46)、art 12x18 不符率 0.0000（§3.2bv）。
    if let Ok((inter, mut node, mut vis)) = bar.single_mut() {
        if scrollable {
            *vis = Visibility::Visible;
            let panel_y = panel
                .single()
                .map(|n| crate::ui::theme::node_origin(n, ORIGIN).1)
                .unwrap_or(ORIGIN.1);
            if *inter == Interaction::Pressed {
                // 跟手读**真实光标**（注入探针在自动化下会被钉在按下点，见 `CursorSource::real`）
                if let Some(cur) = cursor_src.real() {
                    let raw = (cur.y - panel_y).clamp(BAR_Y_MIN, BAR_Y_MAX);
                    *index = index_from_bar_y(raw, count);
                    node.top = Val::Px(raw);
                }
            } else {
                node.top = Val::Px(position_bar_y(*index, count));
            }
        } else {
            *vis = Visibility::Hidden;
        }
    }
    // 标题
    if let Ok(mut t) = title.single_mut() {
        t.0 = notice.title.clone();
    }
    // 正文（简化：拼接普通文本，暂不做彩色/链接覆盖层；滚动条拖动也暂缓）
    for (mut text, line) in &mut lines {
        let src = notice
            .lines
            .get(*index + line.0)
            .cloned()
            .unwrap_or_default();
        text.0 = parse_notice_line(&src)
            .iter()
            .map(|seg| seg.text.clone())
            .collect();
    }
}

/// 把 `title`/`message` 灌进公告状态（C# `NoticeDialog.Update`：`Split("\r\n")` 后逐行折行、`_index = 0`）。
///
/// 服务端事件（`S.UpdateNotice`）与实机夹具（control RPC `notice_set`）共用这一支——
/// 两处各写一遍折行必然漂移（§3.2bv 的 A/B 要用**同一份文本**喂两端）。
pub fn set_notice(notice: &mut NoticeState, title: String, message: String) {
    notice.title = title;
    notice.message = message.clone();
    notice.lines = message
        .split("\r\n")
        .flat_map(|l| l.split('\n'))
        .flat_map(|l| wrap_text(l, LINE_FONT_PX, LINE_WRAP_W))
        .collect();
    notice.index = 0;
}

/// 消费服务端公告事件（S.UpdateNotice → 切行/折行 → 开窗）
fn notice_server_events(
    mut mgr: ResMut<DialogManager>,
    mut notice: ResMut<NoticeState>,
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
) {
    for ev in events.read() {
        if let crate::network::server_event::ServerEvent::NoticeUpdated { title, message } = ev {
            if message.trim().is_empty() {
                continue;
            }
            set_notice(&mut notice, title.clone(), message.clone());
            mgr.open.push(DialogKind::Notice);
            tracing::info!("📢 服务器公告: {}（{} 行）", title, notice.lines.len());
        }
    }
}

// ---------------------------------------------------------------------------
// 富文本标记（NoticeDialog.cs:12-13）：C={t/colour} 着色、L=(t/link) 链接
// ---------------------------------------------------------------------------

/// 一行解析后的渲染段
#[derive(Debug, PartialEq)]
pub struct NoticeSeg {
    pub text: String,
    pub color: Option<Color>,
    pub link: Option<String>,
}

/// 解析一行公告文本（标记外是普通段；空段/未闭合按原文显示）
pub fn parse_notice_line(line: &str) -> Vec<NoticeSeg> {
    let mut out: Vec<NoticeSeg> = Vec::new();
    let mut plain = String::new();
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        let c = chars[i];
        if c == '{' {
            if let Some(close) = find_char(&chars, i + 1, '}', 64) {
                let inner: String = chars[i + 1..close].iter().collect();
                if let Some((text, colour)) = inner.split_once('/') {
                    if !text.is_empty() && !colour.is_empty() {
                        push_plain(&mut out, &mut plain);
                        out.push(NoticeSeg {
                            text: text.to_string(),
                            color: known_color(colour),
                            link: None,
                        });
                        i = close + 1;
                        continue;
                    }
                }
            }
        }
        if c == '(' {
            if let Some(close) = find_char(&chars, i + 1, ')', 64) {
                let inner: String = chars[i + 1..close].iter().collect();
                if let Some((text, link)) = inner.split_once('/') {
                    if !text.is_empty() && !link.is_empty() {
                        push_plain(&mut out, &mut plain);
                        out.push(NoticeSeg {
                            text: text.to_string(),
                            color: None,
                            link: Some(link.to_string()),
                        });
                        i = close + 1;
                        continue;
                    }
                }
            }
        }
        plain.push(c);
        i += 1;
    }
    push_plain(&mut out, &mut plain);
    out
}

fn push_plain(out: &mut Vec<NoticeSeg>, plain: &mut String) {
    if !plain.is_empty() {
        out.push(NoticeSeg {
            text: std::mem::take(plain),
            color: None,
            link: None,
        });
    }
}

fn find_char(chars: &[char], from: usize, target: char, max: usize) -> Option<usize> {
    let end = (from + max).min(chars.len());
    (from..end).find(|&i| chars[i] == target)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// §3.2co 批④：`NoticeDialog` 原点按 C# `Size` = `GetTrueSize(Prguse[961])` = **314x466** 算
    /// （不是源码对面的图头 316x466）：`((1024-314)/2, (768-466)/3)` = **(355,100)**。
    /// 注意 y 用的是 **`/3`**（C# 原样），不是 `/2`；两项都是整数除法。
    #[test]
    fn notice_origin_uses_get_true_size() {
        assert_eq!((BG_W, BG_H), (316.0, 466.0), "图头（贴图 1:1 用）");
        assert_eq!(BG_TRUE_W, 314.0, "GetTrueSize(961).Width");
        assert_eq!(ORIGIN, (355.0, 100.0));
        assert_ne!(
            ((1024.0 - BG_W) / 2.0).trunc(),
            ORIGIN.0,
            "按图头会得 354（本轮修前的值）"
        );
        assert_eq!(
            ((768.0 - BG_H) / 3.0).trunc(),
            100.0,
            "y 用 /3（C# `(SH - Size.Height) / 3`）"
        );
    }

    fn seg(text: &str, color: Option<Color>, link: Option<&str>) -> NoticeSeg {
        NoticeSeg {
            text: text.to_string(),
            color,
            link: link.map(|s| s.to_string()),
        }
    }

    /// 着色 + 链接混排（C# 正则语义：首个 '/' 切、未知色白显）
    #[test]
    fn parse_color_and_link() {
        let segs = parse_notice_line("活动{今日开启/KHAKI}详情(点击/http://x.example)");
        assert_eq!(
            segs,
            vec![
                seg("活动", None, None),
                seg("今日开启", Some(Color::srgb(0.94, 0.9, 0.55)), None),
                seg("详情", None, None),
                seg("点击", None, Some("http://x.example")),
            ]
        );
    }

    /// 未闭合标记按原文渲染（不丢字）
    #[test]
    fn parse_unclosed_falls_back() {
        assert_eq!(parse_notice_line("a {bad"), vec![seg("a {bad", None, None)]);
        assert_eq!(parse_notice_line("a (bad"), vec![seg("a (bad", None, None)]);
    }

    /// 未知色名显示文本但不着色（不裸显花括号）
    #[test]
    fn parse_unknown_color_plain() {
        assert_eq!(
            parse_notice_line("{Text/NotAColor}"),
            vec![seg("Text", None, None)]
        );
    }

    /// 滑块数学（C# :134-185）：interval=400/(count-19)，y 夹 [46,399]
    #[test]
    fn bar_math_matches_csharp() {
        // count=39 → interval=20：index 0/1/max → 46/66/399（max=20 夹 399）
        assert_eq!(position_bar_y(0, 39), 46.0);
        assert_eq!(position_bar_y(1, 39), 66.0);
        assert_eq!(position_bar_y(20, 39), 399.0);
        assert_eq!(index_from_bar_y(46.0, 39), 0);
        assert_eq!(index_from_bar_y(65.9, 39), 0);
        assert_eq!(index_from_bar_y(66.0, 39), 1);
        // C# 原版怪癖：反向映射 floor((399-46)/20)=17——拖到底只能到 17，
        // 18-20 只能靠滚轮/Down 钮到达（OnMoving 整数 floor，如实复刻）
        assert_eq!(index_from_bar_y(500.0, 39), 17);
        // 拖到顶之上夹紧
        assert_eq!(index_from_bar_y(0.0, 39), 0);
        // ≤19 行无滚动
        assert_eq!(position_bar_y(3, 10), 46.0);
        assert_eq!(index_from_bar_y(300.0, 10), 0);
        // count=20：interval=400 → 任何 index ≥1 都到 399（C# 整数 interval 特例，如实复刻）
        assert_eq!(position_bar_y(1, 20), 399.0);
        // 非整除 count（审查 M5）：count=42 → C# int 除 interval=400/23=17（非 17.39），
        // 且 y↔index 往返自洽
        assert_eq!(bar_interval(42), 17.0);
        assert_eq!(position_bar_y(20, 42), 46.0 + 20.0 * 17.0);
        assert_eq!(index_from_bar_y(position_bar_y(20, 42), 42), 20);
        assert_eq!(index_from_bar_y(position_bar_y(7, 42), 42), 7);
    }

    /// 折行驱动分页：50 CJK → 42+8 两行，19 行/页 → 第 2 页 4 行
    #[test]
    fn wrap_drives_pages() {
        let lines = wrap_text(&"字".repeat(50), LINE_FONT_PX, LINE_WRAP_W);
        assert_eq!(lines.len(), 2);
    }

    /// §3.2bv：按钮/位置条的**绘制尺寸 = art 原生尺寸**（C# `Libraries.Draw` 不缩放；
    /// `NoticeDialog.cs` 只给 Up/Down 写了 `Size=(16,14)` 命中框）。
    ///
    /// 红检：把 Ok/Up/Down 改回 20x20（修前的写法）→ 本测试 FAILED。
    #[test]
    fn notice_buttons_use_native_art_size() {
        use crate::resources::libraries::{data_assets_present, resolve_data_path, Libraries};
        use bevy::ecs::system::RunSystemOnce;

        if !data_assets_present() {
            eprintln!("skip notice_buttons_use_native_art_size: 无 Data 资产");
            return;
        }
        let mut world = World::new();
        world.insert_resource(crate::map_renderer::GameLibraries(Libraries::new(
            resolve_data_path(),
        )));
        world.insert_resource(Assets::<Image>::default());
        world.insert_resource(Assets::<Font>::default());
        world.insert_resource(UiFont::default());
        world.insert_resource(UiCjkFont::default());
        world.insert_resource(NoticeState::default());
        world
            .run_system_once(spawn_notice)
            .expect("spawn_notice 应成功");

        let mut q = world.query::<(Option<&NoticeBtn>, Option<&NoticeBar>, &Node)>();
        let mut got: Vec<(String, f32, f32, f32, f32)> = Vec::new();
        for (btn, bar, node) in q.iter(&world) {
            let (l, t, w, h) = match (node.left, node.top, node.width, node.height) {
                (Val::Px(l), Val::Px(t), Val::Px(w), Val::Px(h)) => (l, t, w, h),
                _ => (-1.0, -1.0, -1.0, -1.0),
            };
            let name = match (btn.map(|b| b.0), bar.is_some()) {
                (Some(NoticeBtnKind::Ok), _) => "ok",
                (Some(NoticeBtnKind::Up), _) => "up",
                (Some(NoticeBtnKind::Down), _) => "down",
                (None, true) => "bar",
                _ => continue,
            };
            got.push((name.to_string(), l, t, w, h));
        }
        let find = |n: &str| got.iter().find(|g| g.0 == n).cloned();
        assert_eq!(
            find("ok"),
            Some(("ok".to_string(), OK_REL.0, OK_REL.1, OK_W, OK_H)),
            "Ok 按钮 = Title[193] art 原生 68x25 @(120,436)（C# 未设 Size）"
        );
        assert_eq!(
            find("up"),
            Some(("up".to_string(), UP_REL.0, UP_REL.1, ARROW_W, ARROW_H)),
            "Up = Prguse2[470] art 原生 12x12 @(293,33)"
        );
        assert_eq!(
            find("down"),
            Some(("down".to_string(), DOWN_REL.0, DOWN_REL.1, ARROW_W, ARROW_H)),
            "Down = Prguse2[473] art 原生 12x12 @(293,418)"
        );
        assert_eq!(
            find("bar"),
            Some(("bar".to_string(), BAR_X, BAR_Y_MIN, BAR_W, BAR_H)),
            "PositionBar = Prguse2[205] 12x18 @(293,46)"
        );
    }
}

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
use crate::ui::sprite_ui::{
    UiButton, UiCjkFont, UiEntity, UiFont, UiImageCache, shared_cjk_font, spawn_ui_sprite,
    spawn_ui_text, ui_button_system, ui_image,
};
use crate::ui::text_markup::{est_text_width, known_color, wrap_text};

// —— 布局常量（NoticeDialog.cs；Prguse[961] 实测 316x466）——
pub const BG_W: f32 = 316.0;
pub const BG_H: f32 = 466.0;
/// Location = ((SW-W)/2, (SH-H)/3)（C# 整除）= (354, 100)——用整数除再转 f32
pub const ORIGIN: (f32, f32) = (((1024.0 - BG_W) / 2.0).trunc(), ((768.0 - BG_H) / 3.0).trunc());
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
            (notice_server_events, notice_ui_system, ui_button_system)
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
    mut cache: ResMut<UiImageCache>,
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<UiFont>,
    mut cjk_font: ResMut<UiCjkFont>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    if !notice.cjk_font.is_strong() {
        notice.cjk_font = shared_cjk_font(&mut fonts, &mut cjk_font);
    }

    // 背景（NoticeDialog.cs:31-33）
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 961) {
        let e = spawn_ui_sprite(&mut commands, h, ORIGIN.0, ORIGIN.1, 6.0, 1.0);
        commands.entity(e).insert((
            NoticeBg,
            DialogRoot(DialogKind::Notice),
            NoticeWidget,
            Visibility::Hidden,
        ));
    }
    // 标题（:41-49）
    let title = spawn_ui_text(
        &mut commands,
        &notice.cjk_font,
        "",
        ORIGIN.0 + TITLE_REL.0,
        ORIGIN.1 + TITLE_REL.1,
        LINE_FONT_PX,
        TITLE_COLOR,
        8.0,
    );
    commands.entity(title).insert((
        NoticeTitle,
        DialogRoot(DialogKind::Notice),
        NoticeWidget,
        Visibility::Hidden,
    ));
    // Close（:51-61）/ Ok（:63-73）/ Up（:75-95）/ Down（:97-117）
    let buttons: [(NoticeBtnKind, LibraryName, usize, usize, usize, f32, f32); 4] = [
        (
            NoticeBtnKind::Close,
            LibraryName::Prguse2,
            360,
            361,
            362,
            CLOSE_REL.0,
            CLOSE_REL.1,
        ),
        (
            NoticeBtnKind::Ok,
            LibraryName::Title,
            193,
            194,
            195,
            OK_REL.0,
            OK_REL.1,
        ),
        (
            NoticeBtnKind::Up,
            LibraryName::Prguse2,
            470,
            471,
            472,
            UP_REL.0,
            UP_REL.1,
        ),
        (
            NoticeBtnKind::Down,
            LibraryName::Prguse2,
            473,
            474,
            475,
            DOWN_REL.0,
            DOWN_REL.1,
        ),
    ];
    for (kind, lib, n, h, p, rx, ry) in buttons {
        if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
            &mut commands,
            &mut libs,
            &mut images,
            &mut cache,
            lib,
            n,
            h,
            p,
            ORIGIN.0 + rx,
            ORIGIN.1 + ry,
            7.0,
            20.0,
            20.0,
        ) {
            let mut ec = commands.entity(e);
            ec.insert((
                NoticeBtn(kind),
                DialogRoot(DialogKind::Notice),
                NoticeWidget,
            ));
            if matches!(kind, NoticeBtnKind::Up | NoticeBtnKind::Down) {
                ec.insert(NoticeScrollWidget);
            }
        }
    }
    // PositionBar（:119-131）：挂 DialogRoot 随对话框清理/拖动（审查 M3——
    // 不挂会在重进 Game 时泄漏残留，且 bar_tf.single_mut 从第二个起失效）；
    // 定位由本系统每帧按背景 Transform 覆写，与 drag 系统平移无净冲突
    if let Some(h) = ui_image(
        &mut libs,
        &mut images,
        &mut cache,
        LibraryName::Prguse2,
        205,
    ) {
        let e = spawn_ui_sprite(
            &mut commands,
            h,
            ORIGIN.0 + BAR_X,
            ORIGIN.1 + BAR_Y_MIN,
            7.1,
            1.0,
        );
        commands.entity(e).insert((
            NoticeBar,
            NoticeScrollWidget,
            NoticeWidget,
            DialogRoot(DialogKind::Notice),
        ));
    }
    // 19 行正文（:264-273）
    for i in 0..MAX_LINES {
        let e = spawn_ui_text(
            &mut commands,
            &notice.cjk_font,
            "",
            ORIGIN.0 + LINE_X,
            ORIGIN.1 + LINE_Y0 + i as f32 * LINE_DY,
            LINE_FONT_PX,
            Color::WHITE,
            8.0,
        );
        commands.entity(e).insert((
            NoticeLine(i),
            NoticeLineSrc::default(),
            FontHinting::Enabled,
            DialogRoot(DialogKind::Notice),
            NoticeWidget,
            Visibility::Hidden,
        ));
    }
}

/// 显隐 + 滚动 + 渲染
#[allow(clippy::type_complexity)]
fn notice_ui_system(
    mut commands: Commands,
    mut mgr: ResMut<DialogManager>,
    mut notice: ResMut<NoticeState>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut wheels: MessageReader<MouseWheel>,
    windows: Query<&Window>,
    bg: Query<&Transform, With<NoticeBg>>,
    mut widgets: Query<(&mut Visibility, Option<&NoticeScrollWidget>), With<NoticeWidget>>,
    mut bar_tf: Query<&mut Transform, (With<NoticeBar>, Without<NoticeBg>)>,
    buttons: Query<(&mut UiButton, &NoticeBtn)>,
    mut lines: Query<
        (
            &mut Text2d,
            &mut TextColor,
            &mut TextFont,
            &Transform,
            &NoticeLine,
            &mut NoticeLineSrc,
        ),
        (Without<NoticeBg>, Without<NoticeBar>),
    >,
    mut title: Query<(&mut Text2d, &mut TextColor), (With<NoticeTitle>, Without<NoticeLine>)>,
    mut bar_drag: Local<BarDrag>,
) {
    let open = mgr.is_open(DialogKind::Notice);
    let count = notice.lines.len();
    // 滚动件显隐（C# NewText :218-243：超 MaximumLines 才显示）
    let scrollable = count > MAX_LINES;
    for (mut vis, scroll) in widgets.iter_mut() {
        *vis = if !open {
            Visibility::Hidden
        } else if scroll.is_some() {
            if scrollable {
                Visibility::Visible
            } else {
                Visibility::Hidden
            }
        } else {
            Visibility::Visible
        };
    }
    if !open {
        bar_drag.dragging = false;
        return;
    }

    // 对话框当前原点（拖动后偏离初始 ORIGIN——命中/定位都以它为基准）
    let (ox, oy) = bg
        .single()
        .map(|tf| (tf.translation.x, -tf.translation.y))
        .unwrap_or(ORIGIN);

    // Close/Ok 关闭；Up/Down 翻一行（C# :87-117）
    let mut clicked_kinds: Vec<NoticeBtnKind> = Vec::new();
    for (b, k) in buttons.iter() {
        if b.clicked {
            clicked_kinds.push(k.0);
        }
    }
    for k in clicked_kinds {
        match k {
            NoticeBtnKind::Close | NoticeBtnKind::Ok => {
                mgr.close(DialogKind::Notice);
            }
            NoticeBtnKind::Up => {
                if notice.index > 0 {
                    notice.index -= 1;
                }
            }
            NoticeBtnKind::Down => {
                if notice.index + MAX_LINES < count {
                    notice.index += 1;
                }
            }
        }
    }

    let Ok(window) = windows.single() else { return };
    let cursor = window.cursor_position();

    // 滚轮（C# LoginNoticeDialog_MouseWheel :154-170：对话框区域内滚动）
    let mut notches = 0.0f32;
    for ev in wheels.read() {
        notches += match ev.unit {
            MouseScrollUnit::Line => ev.y,
            MouseScrollUnit::Pixel => ev.y / 120.0,
        };
    }
    if notches != 0.0 && scrollable {
        if let Some(c) = cursor {
            if c.x >= ox && c.x <= ox + BG_W && c.y >= oy && c.y <= oy + BG_H {
                let max = (count - MAX_LINES) as i32;
                notice.index =
                    (notice.index as i32 - notches.round() as i32).clamp(0, max) as usize;
            }
        }
    }

    // PositionBar：命中按下 → 拖动换算 index → 每帧按 index 定位（C# :119-152/:172-185）
    let bar_y_rel = position_bar_y(notice.index, count);
    if let Some(c) = cursor {
        let bar_y_abs = oy + bar_y_rel;
        if mouse.just_pressed(MouseButton::Left)
            && !bar_drag.dragging
            && c.x >= ox + BAR_X
            && c.x <= ox + BAR_X + BAR_W
            && c.y >= bar_y_abs
            && c.y <= bar_y_abs + BAR_H
        {
            bar_drag.dragging = true;
            bar_drag.grab_offset = c.y - bar_y_abs;
        }
    }
    if bar_drag.dragging {
        if !mouse.pressed(MouseButton::Left) {
            bar_drag.dragging = false;
        } else if let Some(c) = cursor {
            let y = c.y - oy - bar_drag.grab_offset;
            notice.index = index_from_bar_y(y, count);
        }
    }
    if let Ok(mut tf) = bar_tf.single_mut() {
        tf.translation.x = ox + BAR_X;
        tf.translation.y = -(oy + bar_y_rel);
    }

    // 标题（C# NameLabel.Text = Notice.Title，空则不显示）
    if let Ok((mut text, mut color)) = title.single_mut() {
        text.0 = notice.title.clone();
        color.0 = TITLE_COLOR;
    }

    // 正文窗口渲染（基础白字 + 标记叠加段，锚定行实体当前 Transform）
    for (mut text, mut color, mut font, tf, line, mut src_cache) in lines.iter_mut() {
        let src = notice
            .lines
            .get(notice.index + line.0)
            .cloned()
            .unwrap_or_default();
        let (lx, ly) = (tf.translation.x, -tf.translation.y);
        let segs = parse_notice_line(&src);
        // 链接段悬停命中（黄→红，C# :333-336）：按段 x 区间
        let mut hover: Option<usize> = None;
        if let Some(c) = cursor {
            if c.y >= ly && c.y <= ly + LINE_DY && c.x >= lx {
                let mut px = 0.0f32;
                for (idx, seg) in segs.iter().enumerate() {
                    let w = est_text_width(&seg.text, LINE_FONT_PX);
                    if seg.link.is_some() && c.x <= lx + px + w {
                        hover = Some(idx);
                        break;
                    }
                    px += w;
                    if c.x < lx + px {
                        break; // 命中在普通段上
                    }
                }
            }
        }
        if src_cache.src == src && src_cache.hover == hover {
            continue;
        }
        src_cache.src = src.clone();
        src_cache.hover = hover;
        for e in src_cache.overlays.drain(..) {
            commands.entity(e).despawn();
        }
        if src.is_empty() {
            text.0 = String::new();
            continue;
        }
        // CJK 主字体（#2599：动态文本不依赖脚本回退）
        font.font = FontSource::Handle(notice.cjk_font.clone());
        let FontSize::Px(font_px) = font.font_size else {
            continue;
        };
        let mut stripped = String::new();
        let mut prefix_w = 0.0f32;
        let mut overlay_specs: Vec<(f32, String, Color)> = Vec::new();
        for (idx, seg) in segs.iter().enumerate() {
            let seg_col = if let Some(c) = seg.color {
                c
            } else if seg.link.is_some() {
                // 链接黄、悬停红（C# NewLink :327-336）
                if hover == Some(idx) {
                    Color::srgb(1.0, 0.0, 0.0)
                } else {
                    Color::srgb(1.0, 1.0, 0.0) // 链接黄（C# Color.Yellow）
                }
            } else {
                Color::WHITE
            };
            if seg.color.is_some() || seg.link.is_some() {
                overlay_specs.push((prefix_w, seg.text.clone(), seg_col));
            }
            prefix_w += est_text_width(&seg.text, font_px);
            stripped.push_str(&seg.text);
        }
        text.0 = stripped;
        color.0 = Color::WHITE;
        for (x_off, seg_text, seg_col) in overlay_specs {
            let e = commands
                .spawn((
                    UiEntity,
                    Text2d::new(seg_text),
                    Anchor::TOP_LEFT,
                    TextFont {
                        font: FontSource::Handle(notice.cjk_font.clone()),
                        font_size: FontSize::Px(font_px),
                        ..default()
                    },
                    TextColor(seg_col),
                    FontHinting::Enabled,
                    Transform::from_xyz(lx + x_off, tf.translation.y, tf.translation.z + 0.05),
                    Visibility::Visible,
                    DialogRoot(DialogKind::Notice),
                    NoticeWidget,
                ))
                .id();
            src_cache.overlays.push(e);
        }
    }

    // 链接点击开浏览器（C# NewLink :338-347，仅 http://）
    if mouse.just_pressed(MouseButton::Left) {
        let Some(c) = cursor else { return };
        for (_, _, _, tf, line, _) in lines.iter() {
            let Some(src) = notice.lines.get(notice.index + line.0) else {
                continue;
            };
            let (lx, ly) = (tf.translation.x, -tf.translation.y);
            if c.y < ly || c.y > ly + LINE_DY || c.x < lx {
                continue;
            }
            let mut px = 0.0f32;
            for seg in parse_notice_line(src) {
                let w = est_text_width(&seg.text, LINE_FONT_PX);
                if let Some(url) = &seg.link {
                    if c.x >= lx + px && c.x <= lx + px + w {
                        open_url(url);
                        return;
                    }
                }
                px += w;
                if c.x < lx + px {
                    break;
                }
            }
        }
    }
}

/// 打开外部链接（C# Process.Start(UseShellExecute)；非 Windows 记日志）。
/// http(s) 前缀大小写不敏感（C# StartsWith ignoreCase:true）。Windows 用
/// explorer.exe 直接传参——**不走 cmd /C start**（cmd 元字符 & | ^ % 会把
/// 服务器可控的 url 变成命令注入面，审查 M4）
fn open_url(url: &str) {
    let lower = url.to_ascii_lowercase();
    if !lower.starts_with("http://") && !lower.starts_with("https://") {
        tracing::info!("公告链接非 http(s)，忽略: {url}");
        return;
    }
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("explorer.exe").arg(url).spawn();
    }
    #[cfg(not(windows))]
    {
        tracing::info!("公告链接（本平台未接浏览器打开）: {url}");
    }
}

/// #256：S.UpdateNotice → 重算折行 + 打开公告对话框（C# Update :187-207）
fn notice_server_events(
    mut mgr: ResMut<DialogManager>,
    mut notice: ResMut<NoticeState>,
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
) {
    for ev in events.read() {
        if let crate::network::server_event::ServerEvent::NoticeUpdated { title, message } = ev {
            // C# Update :193-196：空白消息不开窗
            if message.trim().is_empty() {
                continue;
            }
            notice.title = title.clone();
            notice.message = message.clone();
            // 切行 + 420px 折行（审查 M1）：本移植服务端/mock 链路发 "\n"
            //（ServerRust session.rs join("\n")），C# 是 "\r\n" 直传——两种
            // 都切；折行展开计入行数（滑块 count 按显示行算，C# 按原始行数
            // + 控件内视觉折行——有意偏差：本移植滚动边界=可读行）
            notice.lines = message
                .split("\r\n")
                .flat_map(|l| l.split('\n'))
                .flat_map(|l| wrap_text(l, LINE_FONT_PX, LINE_WRAP_W))
                .collect();
            notice.index = 0;
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
        // {text/colour}（C# 正则非贪婪在首个 '/' 切）
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
        // (text/link)
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
}

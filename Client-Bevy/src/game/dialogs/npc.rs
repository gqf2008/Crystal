// ============================================================================
// NPC 对话框（M9 第 2 批 → 批 46 bevy_ui 迁移）
// 布局参考：macroquad npc_dialog.rs / C# NPCDialogs.cs
//   - 背景 Prguse[384/385]（实测 440x224），位置 (0,0)
//   - 文本区 (8,34)，行距 18；[@XXX] 行是选项，点击发送 CallNPC
//   - 关闭按钮 Prguse2[360-362] 在 (413,3)
// 网络：NPCResponse（行列表）→ 显示；CallNPC 推进
// 迁移说明：
//   - 面板根 = bevy_ui Node + ImageNode（spawn_panel），子节点绝对定位
//   - 行/叠加段 = bevy_ui Text（CJK 主字体，#2599 重排版豆腐教训）
//   - 服务器驱动显隐（NpcDialogState.visible），不走 DialogManager.open →
//     面板根挂 AlwaysVisible，避开 enforce_dialog_visibility 的"未 open 即隐藏"
//     兜底（否则 NPC 对话页在 PostUpdate 被强制隐藏，实机黑窗）
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::text_input::{
    TextInputDisplay, TextInputField, TextInputRect, TextInputState, TextInputSubmit,
};
use crate::game::dialogs::{AlwaysVisible, DialogKind, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{shared_cjk_font, UiCjkFont, UiFont};
use crate::ui::theme::{
    load_lib_image, spawn_animated_icon_button, spawn_container, spawn_icon_button, spawn_label,
    spawn_panel, spawn_scroll_bar_ui, UiScrollList,
};

/// 面板尺寸（Prguse[384] 实测 440x224）
pub const PANEL_W: f32 = 440.0;
pub const PANEL_H: f32 = 224.0;

/// NPC 对话框状态（网络写入）
#[derive(Resource, Default)]
pub struct NpcDialogState {
    pub visible: bool,
    pub npc_object_id: u32,
    pub lines: Vec<String>,
    /// CJK 主字体（宋体资产）：动态改写的行文本用它——parley 脚本回退在
    /// 重排版时失效（#2599 实机），主字体自带 CJK 则不依赖回退
    pub cjk_font: Handle<Font>,
}

#[derive(Component)]
pub struct NpcDialogWidget;

#[derive(Component)]
pub struct NpcClose;

#[derive(Component)]
pub struct NpcLine(usize);

/// 行渲染缓存（源文本 + 悬停态）——未变不重建，避免每帧重排（#112 同因）
#[derive(Component, Default)]
struct NpcLineSrc {
    src: String,
    hover: LineHover,
    /// 行内标记段的叠加标签实体（彩色段/链接段，C# NewColour 独立 MirLabel）
    overlays: Vec<Entity>,
}

/// 行悬停态（驱动重建与着色）：菜单行整行悬停（C# MirLabel 通栏热区），
/// 链接行按第 idx 个链接段的 x 区间命中（C# 每链接是独立 NewButton）
#[derive(Debug, Default, PartialEq, Clone, Copy)]
enum LineHover {
    #[default]
    None,
    Menu,
    Link(usize),
}

#[derive(Component)]
pub struct NpcQuest;

/// 行字号（逻辑 px）：spawn 行实体与点击/悬停的段区间度量共用同一来源，
/// 保证 x 命中判定与渲染定位一致
const NPC_LINE_FONT_PX: f32 = 13.0;

/// #272 NPC 输入状态（S.NPCRequestInput）
#[derive(Resource, Default)]
pub struct NpcInputState {
    pub npc_id: u32,
    pub page_name: String,
    pub active: bool,
}

/// #272 NPC 输入覆盖层根
#[derive(Component)]
pub struct NpcInputRoot;

/// #272 NPC 输入确定按钮
#[derive(Component)]
pub struct NpcInputOk;

pub struct NpcDialogPlugin;

impl Plugin for NpcDialogPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NpcDialogState>();
        app.init_resource::<UiCjkFont>();
        app.add_systems(OnEnter(AppState::Game), spawn_npc_dialog);
        app.add_systems(OnExit(AppState::Game), cleanup_npc_dialog);
        app.add_systems(
            Update,
            npc_dialog_server_events.run_if(in_state(AppState::Game)),
        );
        app.init_resource::<NpcInputState>();
        app.add_systems(
            Update,
            npc_input_overlay
                .after(crate::network::network_system)
                .run_if(in_state(AppState::Game)),
        );
        app.add_systems(Update, npc_ui_system.run_if(in_state(AppState::Game)));
    }
}

fn cleanup_npc_dialog(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_npc_dialog(
    mut commands: Commands,
    mut npc: ResMut<NpcDialogState>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut fonts: ResMut<Assets<Font>>,
    mut cjk_font: ResMut<UiCjkFont>,
) {
    libs.0.ensure_initialized();
    if !npc.cjk_font.is_strong() {
        // 共享宋体资产（#2602 批R：与公告等对话框复用同一 Handle）
        npc.cjk_font = shared_cjk_font(&mut fonts, &mut cjk_font);
    }
    let cjk = npc.cjk_font.clone();

    // 背景 Prguse[384] @ (0,0)；面板根（bevy_ui Node + ImageNode + Overflow::clip）
    let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 384) else {
        return;
    };
    let panel = spawn_panel(&mut commands, bg, 0.0, 0.0, PANEL_W, PANEL_H, 30);
    commands.entity(panel).insert((
        DialogRoot(DialogKind::Npc),
        NpcDialogWidget,
        // 服务器驱动显隐，不随 DialogManager.open 门控（见文件头迁移说明）
        AlwaysVisible,
        // #118 长对话页滚轮滚动（C# NPC 对话框支持 MouseWheel）
        UiScrollList {
            rect_rel: (8.0, 34.0, 400.0, 144.0),
            row_h: 18.0,
            visible: 8,
            total: 0,
            offset: 0,
            step: 3,
            track_rel: (420.0, 34.0, 4.0, 144.0),
            thumb: None,
            z: 8,
        },
    ));
    commands.entity(panel).with_children(|p| {
        // 滚动条（轨道 + 滑块，UiScrollThumb 子节点）
        spawn_scroll_bar_ui(p, (420.0, 34.0, 4.0, 144.0), 8);
        // 关闭按钮 Prguse2[360-362] @ (413,3)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 360),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 361),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 362),
        ) {
            spawn_icon_button(p, n, h, pr, 413.0, 3.0, 20.0, 20.0, 9).insert(NpcClose);
        }
        // 任务按钮（#90 续：MirAnimatedButton，C# NPCDialog QuestButton
        // Title[530..539] 10 帧 130ms 循环 + 悬停 284 / 按下 286，点击切换任务日志）
        let mut frames = Vec::new();
        for i in 530..540usize {
            if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Title, i) {
                frames.push(h);
            }
        }
        let hover = load_lib_image(&mut libs, &mut images, LibraryName::Title, 284);
        let pressed = load_lib_image(&mut libs, &mut images, LibraryName::Title, 286);
        if !frames.is_empty() {
            spawn_animated_icon_button(
                p,
                frames,
                hover,
                pressed,
                172.0,
                PANEL_H - 30.0,
                96.0,
                25.0,
                7,
                0.13,
                true,
            )
            .insert((NpcQuest, Visibility::Hidden));
        }
        // 8 行文本（bevy_ui Text，CJK 主字体）
        for i in 0..8usize {
            spawn_label(
                p,
                &cjk,
                "",
                8.0,
                34.0 + i as f32 * 18.0,
                NPC_LINE_FONT_PX,
                Color::WHITE,
                8,
            )
            .insert((NpcLine(i), NpcLineSrc::default(), FontHinting::Enabled));
        }
    });
}

/// 显示/关闭 + 文本渲染 + 选项点击
#[allow(clippy::type_complexity)]
fn npc_ui_system(
    mut commands: Commands,
    mut npc: ResMut<NpcDialogState>,
    mut npc_goods: ResMut<crate::game::dialogs::npc_goods::NpcGoodsState>,
    mut sell_panel: ResMut<crate::game::dialogs::sell_panel::SellPanelState>,
    mut storage: ResMut<crate::game::dialogs::storage::StorageState>,
    mut mgr: ResMut<crate::game::dialogs::DialogManager>,
    net: Res<NetConnection>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    close: Query<(Entity, &Interaction), With<NpcClose>>,
    mut quest_btns: Query<
        (Entity, &Interaction, &mut Visibility),
        (With<NpcQuest>, Without<NpcDialogWidget>),
    >,
    mut widgets: Query<&mut Visibility, With<NpcDialogWidget>>,
    mut lines: Query<
        (
            Entity,
            &mut Text,
            &mut TextColor,
            &mut TextFont,
            &Node,
            &NpcLine,
            &mut NpcLineSrc,
        ),
        With<NpcDialogWidget>,
    >,
    mut scroll: Query<(Entity, &mut UiScrollList), With<NpcDialogWidget>>,
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

    for mut vis in widgets.iter_mut() {
        *vis = if npc.visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    // 任务按钮（C# CheckQuestButtonDisplay：NPC 有可用任务才显示）
    let has_quest = npc
        .lines
        .iter()
        .any(|l| l.contains("可接受任务") || l.contains("可完成任务"));
    for (e, inter, mut vis) in &mut quest_btns {
        *vis = if npc.visible && has_quest {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if edge(e, inter, &mut prev_inter) && npc.visible && has_quest {
            mgr.toggle(DialogKind::QuestLog);
        }
    }
    if !npc.visible {
        // C# 语义：NPC 对话框关闭时联动隐藏商店/出售/仓库面板
        if npc_goods.visible {
            npc_goods.visible = false;
        }
        if sell_panel.visible {
            sell_panel.visible = false;
        }
        if storage.visible {
            storage.visible = false;
            mgr.close(crate::game::dialogs::DialogKind::Storage);
        }
        return;
    }

    // 关闭（bevy_ui Interaction 边沿）
    for (e, inter) in &close {
        if edge(e, inter, &mut prev_inter) {
            npc.visible = false;
        }
    }

    // 滚动偏移 + 总行数（#118）；面板根实体（叠加标签作为它的子节点）
    let panel = {
        let (panel, mut sl) = match scroll.single_mut() {
            Ok(v) => v,
            Err(_) => return,
        };
        sl.set_total(npc.lines.len());
        panel
    };
    let off = scroll.single().map(|s| s.1.offset).unwrap_or(0);
    let cjk = npc.cjk_font.clone();

    // 渲染行：按 #2599 标记解析分段（{t/Color} 着色段、<t/@key> 链接段）
    // 源文本/悬停未变不重建（span 子实体缓存）
    // 鼠标不在窗口内时 cursor_position()=None：文字照常渲染（仅无悬停高亮）——
    // 原实现在此 early-return，鼠标离开窗口后 NPC 文字永不渲染/更新（master 既有 bug）
    let Ok(window) = windows.single() else { return };
    let cursor = window.cursor_position();
    // 各行实体当前原点（bevy_ui 面板根 @(0,0)，行 Node.left/top 即屏幕坐标）——
    // 悬停/点击/叠加标签都以它为基准
    let mut line_pos: Vec<Option<(f32, f32)>> = vec![None; npc.lines.len().max(8)];
    for (_ent, mut text, mut color, mut font, node, line, mut src_cache) in &mut lines {
        let src = npc.lines.get(off + line.0).cloned().unwrap_or_default();
        let (lx, ly) = (
            match node.left {
                Val::Px(v) => v,
                _ => 0.0,
            },
            match node.top {
                Val::Px(v) => v,
                _ => 0.0,
            },
        );
        if line.0 < line_pos.len() {
            line_pos[line.0] = Some((lx, ly));
        }
        let clickable = is_clickable_npc_line(&src);
        let FontSize::Px(font_px) = font.font_size else {
            continue;
        };
        let segs = parse_npc_line(&src);
        let has_markup = segs.iter().any(|s| s.color.is_some() || s.link.is_some());
        // 悬停命中（C# NPCDialogs.cs:491-492）：菜单行整行热区；链接行按段
        // x 区间逐链接命中（区间与叠加标签定位同一 est_text_width 度量）
        let mut hover = LineHover::None;
        if clickable {
            if let Some(c) = cursor {
                if c.y >= ly && c.y <= ly + 16.0 {
                    if has_markup {
                        let mut px = 0.0f32;
                        for (idx, seg) in segs.iter().enumerate() {
                            let w = est_text_width(&seg.text, font_px);
                            if seg.link.is_some() && c.x >= lx + px && c.x <= lx + px + w {
                                hover = LineHover::Link(idx);
                                break;
                            }
                            px += w;
                        }
                    } else if c.x >= lx && c.x <= lx + 392.0 {
                        hover = LineHover::Menu;
                    }
                }
            }
        }
        if src_cache.src == src && src_cache.hover == hover {
            continue;
        }
        src_cache.src = src.clone();
        src_cache.hover = hover;
        // 所有重建路径统一换 CJK 主字体：parley 的 Hani 脚本回退只在实体
        // 首次排版生效，换页改 text.0 触发的重排版会退化为 .notdef 豆腐框
        // （#2599 实机验证），主字体自带宋体字形才不依赖回退。
        font.font = FontSource::Handle(cjk.clone());
        // 旧叠加标签整体重建（C# NewText 每页 Dispose 全部 _textButtons）
        for e in src_cache.overlays.drain(..) {
            commands.entity(e).despawn();
        }
        if src.is_empty() {
            text.0 = String::new();
            continue;
        }
        // [@XXX] 菜单行（无标记语法）：整行橙色（原行为）
        if !has_markup && clickable {
            text.0 = src.trim().trim_matches('"').to_string();
            color.0 = if hover == LineHover::Menu {
                Color::srgb(1.0, 0.95, 0.4)
            } else {
                Color::srgb(1.0, 0.85, 0.3)
            };
            continue;
        }
        // C# NPCDialog.NewText（NPCDialogs.cs:303-504，R/C 处理与
        // MirScrollingLabel.cs:62-108 同模式）：整行去标记文本一个基础白字
        // 标签 + 每个 {t/Color} 段一个独立彩色 MirLabel 叠加（NewColour）。
        // Bevy 侧不用 TextSpan 混排——实机验证 TextSpan 子段与重排版的 CJK
        // 都不走字体回退链、渲染为 .notdef 豆腐框（#2599），故按 C# 原结构
        // 为基础标签 + 独立叠加标签，全部用自带 CJK 的宋体资产（无需回退），
        // 叠加段以前缀估宽定位（宋体双宽度量，估宽即实际 advance）。
        let mut stripped = String::new();
        let mut prefix_w = 0.0f32;
        let mut overlay_specs: Vec<(f32, String, Color)> = Vec::new();
        for (idx, seg) in segs.iter().enumerate() {
            let seg_col = if let Some(c) = seg.color {
                c
            } else if seg.link.is_some() {
                // 仅悬停中的那个链接段高亮（C# 每链接独立按钮的悬停语义）
                if hover == LineHover::Link(idx) {
                    Color::srgb(1.0, 0.95, 0.4)
                } else {
                    Color::srgb(1.0, 0.85, 0.3)
                }
            } else {
                Color::WHITE
            };
            let is_plain = seg.color.is_none() && seg.link.is_none();
            if !is_plain {
                overlay_specs.push((prefix_w, seg.text.clone(), seg_col));
            }
            prefix_w += est_text_width(&seg.text, font_px);
            stripped.push_str(&seg.text);
        }
        text.0 = stripped;
        color.0 = Color::WHITE;
        for (x_off, seg_text, seg_col) in overlay_specs {
            // 叠加段 = 面板子节点（绝对定位，x=行原点 + 段前缀宽）
            let mut child = None;
            commands.entity(panel).with_children(|p| {
                child = Some(
                    spawn_label(p, &cjk, &seg_text, lx + x_off, ly, font_px, seg_col, 9)
                        .insert(NpcDialogWidget)
                        .id(),
                );
            });
            if let Some(e) = child {
                src_cache.overlays.push(e);
            }
        }
    }

    // 点击选项行（以 [@ 开头的行，#118 含滚动偏移）——点击本身要求光标在窗口内。
    // 链接行按段 x 区间分发到具体链接（C# 每链接独立 NewButton，551 个脚本行
    // 含 ≥2 链接，整行分发会错发第一个 key）；菜单行保持整行热区
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Some(cursor) = cursor else { return };
    for (i, l) in npc.lines.iter().enumerate() {
        if i >= off && i < off + 8 && is_clickable_npc_line(l) {
            let row = i - off;
            // 行实体当前屏幕原点（bevy_ui 行 Node.left/top）
            let Some((lx, ly)) = line_pos.get(row).copied().flatten() else {
                continue;
            };
            if cursor.y >= ly && cursor.y <= ly + 16.0 {
                let segs = parse_npc_line(l);
                let has_markup = segs.iter().any(|s| s.color.is_some() || s.link.is_some());
                let key = if has_markup {
                    // 段区间度量与渲染悬停/叠加定位同一 est_text_width
                    let mut px = 0.0f32;
                    segs.iter().find_map(|seg| {
                        let w = est_text_width(&seg.text, NPC_LINE_FONT_PX);
                        let hit = seg
                            .link
                            .as_ref()
                            .filter(|_| cursor.x >= lx + px && cursor.x <= lx + px + w);
                        px += w;
                        hit.map(|k| format!("[@{k}]"))
                    })
                } else if cursor.x >= lx && cursor.x <= lx + 392.0 {
                    Some(extract_npc_key(l))
                } else {
                    None
                };
                if let Some(key) = key {
                    // 菜单类型标记（购买按钮据此区分 BuyItem / BuyItemBack）
                    npc_goods.is_buyback = key.eq_ignore_ascii_case("[@BuyBack]");
                    net.send_packet(&mir2_shared::packets::client::npc::CallNPC {
                        object_id: npc.npc_object_id,
                        key: key.clone(),
                    });
                    tracing::info!("🧙 NPC 选项: {} → {}", l.trim(), key);
                    break;
                }
            }
        }
    }
}

/// 可点击的 NPC 菜单行：[@XXX] 或 <文字/@XXX>（原版 C# 链接格式）
fn is_clickable_npc_line(line: &str) -> bool {
    let t = line.trim();
    t.starts_with("[@") || t.contains("/@")
}

// ---------------------------------------------------------------------------
// #2599 NPC 对话文本标记（C# NPCDialogs.cs:18-26）
//   R = <text/@key>   行内链接：显示 text，点击发 CallNPC([@key])
//   C = {text/Color}  行内着色：显示 text（KnownColor 名，大小写不敏感）
//   B = <<text/@key>> 大按钮面板：本服 1088 个脚本 0 处使用，不移植（附 #2599）
//   L = (text/url)    外链按钮（点击开浏览器）：全服仅 2 处（MirGuide-0.txt），
//                     客户端移植暂不处理、按普通文本显示（附 #2599）
//   有意偏差：引号按脚本字符串定界符从纯文本段剔除（C# 实机会显示引号）——
//   本移植自 M9 起的既有约定，标记内文本的引号保留
// ---------------------------------------------------------------------------

/// 一行解析后的渲染段
#[derive(Debug, PartialEq)]
pub struct NpcSeg {
    pub text: String,
    /// {t/Color} 指定色（未知色名 → None 白）
    pub color: Option<Color>,
    /// <t/@key> 链接 key（不含 [@] 前后缀）
    pub link: Option<String>,
}

/// 解析一行 NPC 文本为渲染段（标记外的文本是普通段）
pub fn parse_npc_line(line: &str) -> Vec<NpcSeg> {
    let mut out: Vec<NpcSeg> = Vec::new();
    let mut plain = String::new();
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        let c = chars[i];
        // {text/Color}：花括号内按最后一个 '/' 切（C# 非贪婪正则在第一个 '/'
        // 切；现网 0 处多斜杠内容，两者行为等价，rsplit 对含 '/' 文本更稳）
        if c == '{' {
            if let Some(close) = find_char(&chars, i + 1, '}', 64) {
                let inner: String = chars[i + 1..close].iter().collect();
                if let Some((text, color_name)) = inner.rsplit_once('/') {
                    if !text.is_empty() && !color_name.is_empty() {
                        push_plain(&mut out, &mut plain);
                        out.push(NpcSeg {
                            text: text.to_string(),
                            color: known_color(color_name),
                            link: None,
                        });
                        i = close + 1;
                        continue;
                    }
                }
            }
        }
        // <text/@key>（含 <<text/@key>>：C# 里是独立大按钮面板 B 标记，
        // 本服 0 处使用，此处降级为行内链接等价处理，附 #2599）
        if c == '<' {
            let dbl = i + 1 < chars.len() && chars[i + 1] == '<';
            let content_start = if dbl { i + 2 } else { i + 1 };
            let terminator: Vec<char> = if dbl { vec!['>', '>'] } else { vec!['>'] };
            if let Some(content_end) = find_seq(&chars, content_start, &terminator) {
                let inner: String = chars[content_start..content_end].iter().collect();
                if let Some(slash) = inner.find("/@") {
                    let text = &inner[..slash];
                    let key = &inner[slash + 2..];
                    if !text.is_empty() && !key.is_empty() {
                        push_plain(&mut out, &mut plain);
                        out.push(NpcSeg {
                            text: text.to_string(),
                            color: None,
                            link: Some(key.to_string()),
                        });
                        i = content_end + terminator.len();
                        continue;
                    }
                }
            }
        }
        // 引号是脚本定界符，纯文本段剔除
        if c != '"' {
            plain.push(c);
        }
        i += 1;
    }
    push_plain(&mut out, &mut plain);
    out
}

fn push_plain(out: &mut Vec<NpcSeg>, plain: &mut String) {
    if !plain.is_empty() {
        out.push(NpcSeg {
            text: std::mem::take(plain),
            color: None,
            link: None,
        });
    }
}

/// 文本估宽 / KnownColor 映射移至 [`crate::ui::text_markup`]（#2602 批R 公告复用）
use crate::ui::text_markup::{est_text_width, known_color};

fn find_char(chars: &[char], from: usize, target: char, max: usize) -> Option<usize> {
    let end = (from + max).min(chars.len());
    (from..end).find(|&i| chars[i] == target)
}

fn find_seq(chars: &[char], from: usize, seq: &[char]) -> Option<usize> {
    if seq.is_empty() || from >= chars.len() {
        return None;
    }
    (from..=chars.len().saturating_sub(seq.len())).find(|&i| chars[i..].starts_with(seq))
}

/// 提取菜单键（统一为 "[@XXX]" 格式，服务端按该格式匹配）
pub fn extract_npc_key(line: &str) -> String {
    let t = line.trim();
    if t.starts_with("[@") {
        // 含结尾 ']'（[..end] 会丢 ']'，"[@main" 与服务端任何 key 都不匹配，
        // 实机点击菜单因此静默无效——e2e 无法注入点击，历史测试从未覆盖）
        let end = t.find(']').map(|i| i + 1).unwrap_or(t.len());
        t[..end].to_string()
    } else if let Some(slash) = t.find("/@") {
        // 跳过 "/@" 两字符（历史实现只 +1 漏掉 '@'，链接行会拼出 "[@@k1]"
        // 与服务端任何 key 都不匹配——本批测试暴露的 master 既有 bug）
        let rest = &t[slash + 2..];
        let end = rest.find('>').unwrap_or(rest.len());
        format!("[@{}]", &rest[..end])
    } else {
        t.to_string()
    }
}

/// 消费服务端 NPC 对话事件（网络层只广播 ServerEvent）
fn npc_dialog_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut npc: ResMut<NpcDialogState>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        if let ServerEvent::NpcDialog { lines, visible } = ev {
            npc.lines = lines.clone();
            npc.visible = *visible;
        }
    }
}

/// #272：NPC 输入覆盖层——S.NPCRequestInput → 弹输入框；确定/Enter → C.NPCConfirmInput
#[allow(clippy::too_many_arguments)]
fn npc_input_overlay(
    mut commands: Commands,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<UiFont>,
    net: Res<NetConnection>,
    mut state: ResMut<NpcInputState>,
    mut text_state: ResMut<TextInputState>,
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut submits: MessageReader<TextInputSubmit>,
    ok_btns: Query<(Entity, &Interaction), With<NpcInputOk>>,
    mut roots: Query<&mut Visibility, With<NpcInputRoot>>,
    mut prev_inter: Local<std::collections::HashMap<Entity, Interaction>>,
) {
    use crate::network::server_event::ServerEvent;

    for ev in events.read() {
        if let ServerEvent::NpcInputRequest { npc_id, page_name } = ev {
            state.npc_id = *npc_id;
            state.page_name = page_name.clone();
            state.active = true;
            text_state.texts.resize(1, String::new());
            text_state.texts[0].clear();
            text_state.active = Some(0);
            if roots.iter_mut().count() == 0 {
                spawn_npc_input_overlay(
                    &mut commands,
                    &mut libs,
                    &mut images,
                    &mut fonts,
                    &mut ui_font,
                    page_name,
                );
            }
            for mut vis in roots.iter_mut() {
                *vis = Visibility::Visible;
            }
            tracing::info!("⌨️ [NPC] 输入框打开 npc={} page={}", npc_id, page_name);
        }
    }

    let submitted = submits.read().any(|s| s.0 == 0);
    // bevy_ui Interaction 边沿（C# MirInputBox OKButton Click；Enter 同路径）
    let ok_clicked = ok_btns.iter().any(|(e, inter)| {
        let was = prev_inter.insert(e, *inter);
        *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
    });
    if state.active && (submitted || ok_clicked) {
        let value = text_state.texts.first().cloned().unwrap_or_default();
        net.send_packet(&mir2_shared::packets::client::npc::NPCConfirmInput {
            npc_id: state.npc_id,
            page_name: state.page_name.clone(),
            value,
        });
        tracing::info!(
            "⌨️ [NPC] 提交输入 -> npc={} page={}",
            state.npc_id,
            state.page_name
        );
        state.active = false;
        text_state.active = None;
        for mut vis in roots.iter_mut() {
            *vis = Visibility::Hidden;
        }
    }
}

/// 生成输入覆盖层（面板 + 提示 + 输入框 + 确定）
fn spawn_npc_input_overlay(
    commands: &mut Commands,
    libs: &mut GameLibraries,
    images: &mut Assets<Image>,
    fonts: &mut Assets<Font>,
    ui_font: &mut UiFont,
    page_name: &str,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(fonts);
    }
    let font = ui_font.0.clone();

    // 面板 Prguse[170]（实测 244x207）@ (280,80)，z 高于 NPC 主面板
    let Some(bg) = load_lib_image(libs, images, LibraryName::Prguse, 170) else {
        return;
    };
    let root = spawn_panel(commands, bg, 280.0, 80.0, 244.0, 207.0, 40);
    commands.entity(root).insert((NpcInputRoot, Visibility::Hidden));

    commands.entity(root).with_children(|p| {
        // 提示（C# CaptionLabel @(25,25) 语义）
        spawn_label(
            p,
            &font,
            &format!("请输入（{}）:", page_name),
            20.0,
            20.0,
            14.0,
            Color::WHITE,
            10,
        );
        // 输入框：容器（背景）+ TextInputDisplay 子 Text；TextInputRect 为屏幕坐标
        // （面板原点 (280,80) + 相对 (20,50) = (300,130)）
        spawn_container(p, 20.0, 50.0, 200.0, 22.0, 10)
            .insert((
                BackgroundColor(Color::srgba(0.2, 0.2, 0.25, 0.9)),
                TextInputField(0),
                TextInputRect(300.0, 130.0, 200.0, 22.0),
            ))
            .with_children(|ic| {
                ic.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(4.0),
                        top: Val::Px(2.0),
                        ..default()
                    },
                    Text::new(String::new()),
                    TextFont {
                        font: FontSource::Handle(font.clone()),
                        font_size: FontSize::Px(13.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    ZIndex(11),
                    TextInputDisplay(0),
                ));
            });
        // 确定（C# MirInputBox OKButton Title[200-202] @(60,123)；本实现沿用
        // 原 Prguse2[360-362] 三帧，落在面板内而非原越界的 (560,175)）
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(libs, images, LibraryName::Prguse2, 360),
            load_lib_image(libs, images, LibraryName::Prguse2, 361),
            load_lib_image(libs, images, LibraryName::Prguse2, 362),
        ) {
            spawn_icon_button(p, n, h, pr, 60.0, 123.0, 50.0, 22.0, 11).insert(NpcInputOk);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(text: &str, color: Option<Color>, link: Option<&str>) -> NpcSeg {
        NpcSeg {
            text: text.to_string(),
            color,
            link: link.map(|s| s.to_string()),
        }
    }

    /// #2599 真实脚本行（AncientNatural-D003.txt）：
    /// `"Dungeon of the Ancient <Ones/@omacavea>" {Level10~22./KHAKI}`
    /// 引号剔除 + 链接段 + 着色段
    #[test]
    fn parse_real_script_line() {
        let segs =
            parse_npc_line("\"Dungeon of the Ancient <Ones/@omacavea>\" {Level10~22./KHAKI}");
        assert_eq!(
            segs,
            vec![
                seg("Dungeon of the Ancient ", None, None),
                seg("Ones", None, Some("omacavea")),
                seg(" ", None, None),
                seg("Level10~22.", Some(Color::srgb(0.94, 0.9, 0.55)), None),
            ]
        );
    }

    /// <Close/@exit>：纯链接行
    #[test]
    fn parse_link_only_line() {
        let segs = parse_npc_line("<Close/@exit>");
        assert_eq!(segs, vec![seg("Close", None, Some("exit"))]);
    }

    /// 未知色名：显示文本但不着色（不裸显花括号）
    #[test]
    fn parse_unknown_color_renders_plain() {
        let segs = parse_npc_line("{Text/NotAColor}");
        assert_eq!(segs, vec![seg("Text", None, None)]);
    }

    /// 普通行 + 菜单行原样（[@main] 不是标记语法）
    #[test]
    fn parse_plain_and_menu_lines() {
        assert_eq!(parse_npc_line("hello"), vec![seg("hello", None, None)]);
        assert_eq!(parse_npc_line("[@main]"), vec![seg("[@main]", None, None)]);
    }

    /// 未闭合标记按普通文本渲染（不丢字）
    #[test]
    fn parse_unclosed_markup_falls_back() {
        assert_eq!(parse_npc_line("a {bad"), vec![seg("a {bad", None, None)]);
        assert_eq!(parse_npc_line("<bad"), vec![seg("<bad", None, None)]);
    }

    /// <<双括号>>按行内链接解析（C# B 大按钮面板不移植，本服 0 处，附 #2599）
    #[test]
    fn parse_double_bracket_link() {
        let segs = parse_npc_line("<<Buy/@shop>>");
        assert_eq!(segs, vec![seg("Buy", None, Some("shop"))]);
    }

    /// 宋体双宽度量：ASCII 恒 0.50em、CJK/全角恒 1.00em（upem 256 实测）——
    /// 叠加段定位/链接命中区间与实际排版 advance 一致，不得漂移
    #[test]
    fn est_width_dual_metrics() {
        assert_eq!(est_text_width("ab", 13.0), 13.0);
        assert_eq!(est_text_width("古", 13.0), 13.0);
        assert_eq!(est_text_width("a古b", 13.0), 26.0);
    }

    /// 多链接行整行 extract 只取第一个（行级语义）；逐链接分发见点击路径
    #[test]
    fn extract_key_first_link_on_whole_line() {
        assert_eq!(extract_npc_key("<A/@k1> | <B/@k2>"), "[@k1]");
        assert_eq!(extract_npc_key("[@main] 返回"), "[@main]");
    }

    /// is_clickable_npc_line 与链接行兼容（点击路径不变）
    #[test]
    fn clickable_line_with_markup() {
        assert!(is_clickable_npc_line(
            "\"Dungeon of the Ancient <Ones/@omacavea>\" {x/KHAKI}"
        ));
        assert!(is_clickable_npc_line("[@main]"));
        assert!(!is_clickable_npc_line("plain text"));
    }
}

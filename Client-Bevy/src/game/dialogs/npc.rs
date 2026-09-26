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

use crate::game::dialogs::{DialogKind, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::outlined_text::spawn_outlined_label;
use crate::ui::sprite_ui::{shared_cjk_font, UiCjkFont};
use crate::ui::theme::{
    load_lib_image, spawn_animated_icon_button, spawn_close_button, spawn_icon_button, spawn_panel,
    spawn_scroll_bar_ui, UiScrollList,
};

/// #2892 批B：面板精灵与 C# 原生尺寸/原点（C# `NPCDialog.Index = 995; Library = Libraries.Prguse`，
/// 无 `Location` → 默认 (0,0)）。
///
/// 更正（2026-09-26）：`Prguse[384]` 与 `Prguse[995]` **同为 440x224 但不是同一张图**
/// ——384 的框线更细、正文区多一条横向分隔线（把两张图导出来比才看得出；窗口级几何对表与
/// 写死尺寸审计都看不出来）。本端曾按"同一张图"用了 384，现按 C# 用 995。
pub const PANEL: (LibraryName, usize) = (LibraryName::Prguse, 384);

/// 面板尺寸（Prguse[384] 实测 440x224）
pub const PANEL_W: f32 = 440.0;
pub const PANEL_H: f32 = 224.0;

/// 面板背景：`Prguse[995]`（C# `NPCDialog` 构造器 `Index = 995; Library = Libraries.Prguse;`，
/// `NPCDialogs.cs:52`）。**曾用 `Prguse[384]`**——两张图同为 440x224，所以窗口级几何对表与
/// 尺寸审计都看不出来，只有把图导出来比才看得见（384 的框线更细、正文区里多一条横向分隔线）。
pub const NPC_PANEL: (LibraryName, usize) = (LibraryName::Prguse, 995);

/// 翻页箭头：`UpButton` = **`Prguse2`**[197/198/199] @(417,34)、`DownButton` = [207/208/209] @(417,175)，
/// 两颗都显式 `Size = new Size(16, 14)`（`NPCDialogs.cs:78-108`）。
/// 本端此前**没有画这两颗钮**，只用滚轮滚动——原版是"滚轮 + 箭头 + 可拖的 PositionBar"三件套。
///
/// ⚠️ **库名必须是 `Prguse2`**：2026-09-26 第一次补这两颗钮时写成了 `Prguse`，而 `Prguse[197]/[207]`
/// 是**空图（0x0）**——`load_lib_image` 直接返回 `None`，`if let (Some, Some, Some)` 静默跳过，
/// 于是"补了控件"在实机上等于**没补**，而只查位置/索引/尺寸的门禁完全看不出来。
/// 教训：跨库的精灵必须把**库名**也钉进常量与门禁里。
pub const NPC_ARROW_LIB: LibraryName = LibraryName::Prguse2;
pub const NPC_UP_POS: (f32, f32) = (417.0, 34.0);
pub const NPC_DOWN_POS: (f32, f32) = (417.0, 175.0);
pub const NPC_ARROW_SIZE: (f32, f32) = (16.0, 14.0);
pub const NPC_UP_FRAMES: (usize, usize, usize) = (197, 198, 199);
pub const NPC_DOWN_FRAMES: (usize, usize, usize) = (207, 208, 209);

/// `UpButton.Click` 语义（`NPCDialogs.cs:82-86`）：`if (_index <= 0) return; _index--;`
pub fn npc_scroll_up(offset: usize) -> usize {
    offset.saturating_sub(1)
}

/// `DownButton.Click` 语义（`NPCDialogs.cs:94-99`）：
/// `if (_index + MaximumLines >= CurrentLines.Count) return; _index++;`
pub fn npc_scroll_down(offset: usize, total: usize, visible: usize) -> usize {
    if offset + visible >= total {
        offset
    } else {
        offset + 1
    }
}
/// 关闭键 `Prguse2[360..362]` @(413,3)（`NPCDialogs.cs:139-140`，无 `Size` → 原生 24x21）
pub const CLOSE_POS: (f32, f32) = (413.0, 3.0);

/// 文本行几何：C# `TextLabel[i].Location = new Point(8, 34 + (i - _index) * 18)`、
/// `Size = (420, 20)`，可见 `MaximumLines = 8` 行（`NPCDialogs.cs:42/410-417`）。
pub const LINE_X: f32 = 8.0;
pub const LINE_Y0: f32 = 34.0;
pub const LINE_PITCH: f32 = 18.0;
pub const LINE_COUNT: usize = 8;

/// 滚轮命中区 = **整个对话框**（含标题栏与右侧空白）。
///
/// C# 在 `NPCDialog` 构造里就把 `MouseWheel += NPCDialog_MouseWheel` 挂在对话框**自身**
/// （`Client/MirScenes/Dialogs/NPCDialogs.cs:64`）；`:502/547/566/604` 挂在 `TextLabel[i]`
/// 与链接/颜色标签上的那几处是**重复挂载**（子控件不处理时事件冒泡到父控件）。
///
/// 此前本端取 (8,34,400,144)——只有 8 行文本框那一块，比原版小一圈：在原版能滚的
/// 标题栏/右侧空白上滚不动（owner 队列 `scroll-hitrect-npc`）。
pub const LIST_WHEEL_RECT: (f32, f32, f32, f32) = (0.0, 0.0, PANEL_W, PANEL_H);

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

/// 翻页箭头（C# `NPCDialog.UpButton` / `DownButton`）
#[derive(Component)]
pub struct NpcScrollUp;
#[derive(Component)]
pub struct NpcScrollDown;

#[derive(Component)]
/// 行号（0..8）。字段公开给 `npc_rows` 只读探针做行矩形换算——
/// 探针必须与点击分发读同一份行原点，否则夹具算出的点击点是"另一套几何"。
pub struct NpcLine(pub usize);

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
/// UI 已迁标准 `MirInputBox`（`input_box.rs`，`InputPurpose::NpcConfirm`）；
/// 本资源只记录「最近一次 NPC 输入请求」供自动化探针（auto/world.rs）断言。
#[derive(Resource, Default)]
pub struct NpcInputState {
    pub npc_id: u32,
    pub page_name: String,
    pub active: bool,
}

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
        // 描边文本（批46 P1：C# MirLabel 默认 OutLine=true）：sync 必须排在全部
        // Text 写方之后（同帧晚写副本陈旧——变更检测按 tick 严格比较）
        app.add_systems(
            Update,
            (
                npc_dialog_server_events,
                npc_input_state_system,
                npc_ui_system,
                npc_scroll_arrows_system,
                crate::ui::outlined_text::sync_outline_ui_system,
            )
                .chain()
                .after(crate::network::network_system)
                .run_if(in_state(AppState::Game)),
        );
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

    // 背景 Prguse[995] @ (0,0)；面板根（bevy_ui Node + ImageNode + Overflow::clip）
    let Some(bg) = load_lib_image(&mut libs, &mut images, NPC_PANEL.0, NPC_PANEL.1) else {
        return;
    };
    let panel = spawn_panel(&mut commands, bg, 0.0, 0.0, PANEL_W, PANEL_H, 30);
    commands.entity(panel).insert((
        DialogRoot(DialogKind::Npc),
        NpcDialogWidget,
        // #118 长对话页滚轮滚动（C# NPC 对话框支持 MouseWheel）
        UiScrollList {
            rect_rel: LIST_WHEEL_RECT,
            row_h: 18.0,
            visible: 8,
            total: 0,
            offset: 0,
            // 每格 1 行：C# `NPCDialog_MouseWheel`（NPCDialogs.cs:235-245）
            // `int count = e.Delta / MouseWheelScrollDelta;` → `_index -= count`
            step: 1,
            track_rel: (420.0, 34.0, 4.0, 144.0),
            thumb: None,
            z: 8,
        },
    ));
    commands.entity(panel).with_children(|p| {
        // 滚动条（轨道 + 滑块，UiScrollThumb 子节点）
        spawn_scroll_bar_ui(p, (420.0, 34.0, 4.0, 144.0), 8);
        // 翻页箭头（C# UpButton/DownButton；`NPCDialogs.cs:78-108`）
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, NPC_ARROW_LIB, NPC_UP_FRAMES.0),
            load_lib_image(&mut libs, &mut images, NPC_ARROW_LIB, NPC_UP_FRAMES.1),
            load_lib_image(&mut libs, &mut images, NPC_ARROW_LIB, NPC_UP_FRAMES.2),
        ) {
            spawn_icon_button(
                p,
                n,
                h,
                pr,
                NPC_UP_POS.0,
                NPC_UP_POS.1,
                NPC_ARROW_SIZE.0,
                NPC_ARROW_SIZE.1,
                9,
            )
            .insert((NpcScrollUp, NpcDialogWidget));
        } else {
            tracing::warn!(
                "🖱 NPC 窗：上翻箭头缺帧（{}[{}/{}/{}]）——控件不会出现，别静默跳过",
                "Prguse2", NPC_UP_FRAMES.0, NPC_UP_FRAMES.1, NPC_UP_FRAMES.2
            );
        }
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, NPC_ARROW_LIB, NPC_DOWN_FRAMES.0),
            load_lib_image(&mut libs, &mut images, NPC_ARROW_LIB, NPC_DOWN_FRAMES.1),
            load_lib_image(&mut libs, &mut images, NPC_ARROW_LIB, NPC_DOWN_FRAMES.2),
        ) {
            spawn_icon_button(
                p,
                n,
                h,
                pr,
                NPC_DOWN_POS.0,
                NPC_DOWN_POS.1,
                NPC_ARROW_SIZE.0,
                NPC_ARROW_SIZE.1,
                9,
            )
            .insert((NpcScrollDown, NpcDialogWidget));
        } else {
            tracing::warn!(
                "🖱 NPC 窗：下翻箭头缺帧（{}[{}/{}/{}]）——控件不会出现，别静默跳过",
                "Prguse2", NPC_DOWN_FRAMES.0, NPC_DOWN_FRAMES.1, NPC_DOWN_FRAMES.2
            );
        }
        // 关闭按钮 Prguse2[360-362] @ (413,3)
        if let Some(mut btn) =
            spawn_close_button(p, &mut libs, &mut images, CLOSE_POS.0, CLOSE_POS.1, 9)
        {
            btn.insert(NpcClose);
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
        for i in 0..LINE_COUNT {
            spawn_outlined_label(
                p,
                cjk.clone(),
                "",
                LINE_X,
                LINE_Y0 + i as f32 * LINE_PITCH,
                NPC_LINE_FONT_PX,
                Color::WHITE,
                8,
            )
            // 必须带 NpcDialogWidget：npc_ui_system 的行渲染查询以它为过滤，
            // 缺标记则行实体永不匹配、文字永不写入（2026-09-18 实机黑窗根因）
            .insert((
                NpcDialogWidget,
                NpcLine(i),
                NpcLineSrc::default(),
                FontHinting::Enabled,
            ));
        }
    });
}

/// 显示/关闭 + 文本渲染 + 选项点击
#[allow(clippy::type_complexity)]
fn npc_scroll_arrows_system(
    npc: Res<NpcDialogState>,
    up: Query<(Entity, &Interaction), (With<NpcScrollUp>, Without<NpcScrollDown>)>,
    down: Query<(Entity, &Interaction), (With<NpcScrollDown>, Without<NpcScrollUp>)>,
    mut scroll: Query<&mut UiScrollList, With<NpcDialogWidget>>,
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
    if !npc.visible {
        return;
    }
    let up_clicked = up.iter().any(|(e, i)| edge(e, i, &mut prev_inter));
    let down_clicked = down.iter().any(|(e, i)| edge(e, i, &mut prev_inter));
    if !(up_clicked || down_clicked) {
        return;
    }
    if let Ok(mut sl) = scroll.single_mut() {
        let (total, visible) = (sl.total, sl.visible);
        sl.offset = if up_clicked {
            npc_scroll_up(sl.offset)
        } else {
            npc_scroll_down(sl.offset, total, visible)
        };
    }
}

fn npc_ui_system(
    mut commands: Commands,
    mut npc: ResMut<NpcDialogState>,
    mut npc_goods: ResMut<crate::game::dialogs::npc_goods::NpcGoodsState>,
    mut sell_panel: ResMut<crate::game::dialogs::sell_panel::SellPanelState>,
    mut storage: ResMut<crate::game::dialogs::storage::StorageState>,
    mut mgr: ResMut<crate::game::dialogs::DialogManager>,
    net: Res<NetConnection>,
    mouse: Res<ButtonInput<MouseButton>>,
    // 命中判定走统一光标来源：**探针优先**（#2767）——NPC 窗是自绘文本、
    // 不是 bevy_ui 按钮，click/cursor RPC 注入的 PointerInput/HoverMap 到不了它，
    // 只有探针能进这条路。无探针时读真实光标，正常游玩行为不变。
    cursor_src: crate::control::CursorSource,
    close: Query<(Entity, &Interaction), With<NpcClose>>,
    // cascade 边沿状态：只在「可见→不可见」那一帧触发（实机交互 sweep 修正）
    mut npc_prev_visible: Local<bool>,
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

    // 状态驱动窗口同样进入 DialogManager，统一显隐兜底与世界输入锁。
    crate::game::dialogs::sync_dialog_state(&mut mgr, DialogKind::Npc, npc.visible);

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
        // C# 语义：`NPCDialog.Hide()` 级联只在「可见→不可见」那一帧发生
        // （C# `if (NPCDialog.Visible) NPCDialog.Hide();` 是边沿语义）——
        // 修复前是每帧强清：服务端事件/RPC 打开的仓库会被立刻再关掉
        // （交互 sweep `storage FAIL: 找不到标准关闭钮` 的根因）。
        if *npc_prev_visible {
            // 对话关闭 → 当前 NPC 归零（`npc_object_id` 的文档语义就是 "0 = 未开对话"）。
            // 不归零的话，下一次开窗（服务端推来的对话）前若有选项点击，会打到上一个 NPC 上。
            // **必须边沿归零**：每帧清零会把「开窗前调用方刚写入的 id」抹掉——
            // control npc_call/世界点击写 id → 等 NPCResponse 的若干帧里被清 0 →
            // 开窗后点选项发 CallNPC{object_id:0}，服务端 warn「unknown object_id 0」
            // 静默丢弃，表现为传送/仓库/买卖入口「点了没反应」（本地低延迟下响应常
            // 抢在下一帧前到达才没大面积暴露；RPC 驱动 200ms+ 响应几乎必中）。
            npc.npc_object_id = 0;
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
        }
        *npc_prev_visible = false;
        return;
    }
    *npc_prev_visible = true;

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
    let cursor = cursor_src.pos();
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
                    spawn_outlined_label(
                        p,
                        cjk.clone(),
                        &seg_text,
                        lx + x_off,
                        ly,
                        font_px,
                        seg_col,
                        9,
                    )
                    .insert((NpcDialogWidget, FontHinting::Enabled))
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
pub fn is_clickable_npc_line(line: &str) -> bool {
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

/// 行命中框高（`npc_ui_system` 的 `cursor.y <= ly + 16.0`）
pub const NPC_ROW_HIT_H: f32 = 16.0;
/// 菜单行（`[@XXX]`，无行内标记）的整行热区宽（C# MirLabel 通栏）
pub const NPC_MENU_HIT_W: f32 = 392.0;

/// 一个可点链接的**精确命中矩形**（`npc_rows` 只读探针用）
#[derive(Debug, Clone, PartialEq)]
pub struct NpcLinkTarget {
    pub row: usize,
    /// 链接段显示文本（如 `Access`）
    pub text: String,
    /// 点击后发出的 key（如 `[@Storage]`）
    pub key: String,
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
}

impl NpcLinkTarget {
    pub fn center(&self) -> (f32, f32) {
        ((self.x0 + self.x1) * 0.5, (self.y0 + self.y1) * 0.5)
    }
}

/// 把「渲染行原点 + 行文本」换算成每条链接的命中矩形。
///
/// 与 `npc_ui_system` 的点击分发**共用同一套度量**（行原点 = 渲染行的 `Node.left/top`、
/// 段宽 = `est_text_width`、行高 `NPC_ROW_HIT_H`），否则夹具点出来的坐标是"另一套几何"。
///
/// 存在的理由：行内链接形如 `<Access/@Storage> Storage`，**链接段只覆盖行首那段文字**，
/// 点在同一行后半截纯文本上不会分发（C# 里每个链接是独立 NewButton，同理）。
/// 夹具按"行中心/行右半"点会静默无反应——⑤ 开仓库 `<Access/@Storage>` 正是栽在这里，
/// 连查数轮都误以为"链接点了没反应"。
///
/// `rows` = `(行号, 行左 lx, 行上 ly)`，来自 `NpcLine` + `Node` 查询。
pub fn npc_link_targets(
    lines: &[String],
    off: usize,
    rows: &[(usize, f32, f32)],
) -> Vec<NpcLinkTarget> {
    let mut out = Vec::new();
    for (i, line) in lines.iter().enumerate().skip(off).take(8) {
        if !is_clickable_npc_line(line) {
            continue;
        }
        let row = i - off;
        let Some(&(_, lx, ly)) = rows.iter().find(|(r, _, _)| *r == row) else {
            continue;
        };
        let segs = parse_npc_line(line);
        let has_markup = segs.iter().any(|s| s.color.is_some() || s.link.is_some());
        if !has_markup {
            // 菜单行：整行热区（与点击分发的 else 分支同宽）
            out.push(NpcLinkTarget {
                row,
                text: line.trim().to_string(),
                key: extract_npc_key(line),
                x0: lx,
                y0: ly,
                x1: lx + NPC_MENU_HIT_W,
                y1: ly + NPC_ROW_HIT_H,
            });
            continue;
        }
        let mut px = 0.0f32;
        for seg in segs.iter() {
            let w = est_text_width(&seg.text, NPC_LINE_FONT_PX);
            if let Some(k) = seg.link.as_ref() {
                out.push(NpcLinkTarget {
                    row,
                    text: seg.text.clone(),
                    key: format!("[@{k}]"),
                    x0: lx + px,
                    y0: ly,
                    x1: lx + px + w,
                    y1: ly + NPC_ROW_HIT_H,
                });
            }
            px += w;
        }
    }
    out
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

/// #272：S.NPCRequestInput → 记录请求（UI 走标准 `MirInputBox`，见 `input_box.rs`）
fn npc_input_state_system(
    mut state: ResMut<NpcInputState>,
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
) {
    use crate::network::server_event::ServerEvent;

    for ev in events.read() {
        if let ServerEvent::NpcInputRequest { npc_id, page_name } = ev {
            state.npc_id = *npc_id;
            state.page_name = page_name.clone();
            state.active = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::{App, MinimalPlugins, Window};

    /// 回归（交互 sweep FAIL_NO_BTN 根因）：`NPCDialog.Hide()` 级联须只在
    /// 「可见→不可见」那一帧触发——修复前每帧强清，RPC/服务端打开的仓库/出售/商品
    /// 永远立关，且与 `dialog_rect` 交互验证不可达。
    #[test]
    fn npc_cascade_closes_linked_panels_only_on_fall_edge() {
        use crate::game::dialogs::{DialogKind, DialogManager};
        use crate::network::NetConnection;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<NpcDialogState>();
        app.init_resource::<crate::game::dialogs::npc_goods::NpcGoodsState>();
        app.init_resource::<crate::game::dialogs::sell_panel::SellPanelState>();
        app.init_resource::<crate::game::dialogs::storage::StorageState>();
        app.init_resource::<DialogManager>();
        app.insert_resource(NetConnection::default());
        app.insert_resource(bevy::input::ButtonInput::<bevy::input::mouse::MouseButton>::default());
        app.init_resource::<crate::control::CursorProbe>();
        app.world_mut().spawn(Window::default());
        app.add_systems(Update, npc_ui_system);

        // NPC 窗开着 → 联动窗不受清
        app.world_mut().resource_mut::<NpcDialogState>().visible = true;
        app.update();
        app.world_mut()
            .resource_mut::<crate::game::dialogs::storage::StorageState>()
            .visible = true;
        app.world_mut()
            .resource_mut::<DialogManager>()
            .open(DialogKind::Storage);
        app.update();
        assert!(
            app.world()
                .resource::<crate::game::dialogs::storage::StorageState>()
                .visible,
            "NPC 开窗状态下不得清联动窗"
        );

        // NPC 关 —— 这一帧级联清仓库
        app.world_mut().resource_mut::<NpcDialogState>().visible = false;
        app.update();
        assert!(
            !app.world()
                .resource::<crate::game::dialogs::storage::StorageState>()
                .visible,
            "NPC 关的那帧应级联清仓库"
        );

        // 再开仓库（模拟 RPC/服务端发起）必须留开——各帧不再强清
        app.world_mut()
            .resource_mut::<crate::game::dialogs::storage::StorageState>()
            .visible = true;
        app.world_mut()
            .resource_mut::<DialogManager>()
            .open(DialogKind::Storage);
        app.update();
        assert!(
            app.world()
                .resource::<crate::game::dialogs::storage::StorageState>()
                .visible,
            "级联只限边沿帧——重开的仓库必须留开"
        );
        assert!(
            app.world()
                .resource::<DialogManager>()
                .is_open(DialogKind::Storage),
            "重开应停留在管理栈"
        );
    }

    /// 行实体 spawn 必须带 NpcDialogWidget（2026-09-18 实机黑窗根因）：
    /// npc_ui_system 的行渲染查询以 With<NpcDialogWidget> 过滤，缺标记则
    /// 查询恒空、文字永不写入——窗口能开但文本区全黑。
    /// 红检：把 spawn 处的 NpcDialogWidget 标记去掉 → 本测试 FAILED。
    #[test]
    fn npc_line_entities_carry_dialog_widget_marker() {
        if !crate::resources::libraries::data_assets_present() {
            eprintln!("无 Data 资产，跳过");
            return;
        }
        let mut app = App::new();
        app.insert_resource(GameLibraries(crate::resources::libraries::Libraries::new(
            "Data",
        )));
        app.insert_resource(Assets::<Image>::default());
        app.insert_resource(Assets::<Font>::default());
        app.init_resource::<UiCjkFont>();
        app.init_resource::<NpcDialogState>();
        app.add_systems(Update, spawn_npc_dialog);
        app.update();

        let mut q = app
            .world_mut()
            .query_filtered::<&NpcLine, With<NpcDialogWidget>>();
        let n = q.iter(app.world()).count();
        assert_eq!(
            n, 8,
            "8 个行实体必须带 NpcDialogWidget（否则渲染查询永不命中）"
        );
    }

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

    /// ⑤ 回归门禁（仓库/BodyGuard 门窗）：行内链接的命中矩形**只覆盖链接段本身**，
    /// 不覆盖同行的纯文本尾巴。实测线上脚本行就是 `<Access/@Storage> Storage`——
    /// 链接段 `Access`（x∈[8,47]）之后还跟着纯文本 ` Storage`；
    /// 夹具按"行中心/行右半"点 (x=60) 落在尾巴上，客户端正确地不分发，
    /// 于是被误读成"点链接没反应"。本测试把这个语义钉死，避免再退回整行热区。
    ///
    /// 阳性对照（实做）：把 `npc_link_targets` 里链接段的 x1 改成整行宽
    /// （`lx + NPC_MENU_HIT_W`，即旧的整行热区语义）→ 本测试立即红。
    #[test]
    fn npc_link_target_covers_link_segment_only() {
        let lines = vec!["<Access/@Storage> Storage".to_string()];
        let targets = npc_link_targets(&lines, 0, &[(0, 8.0, 34.0)]);
        assert_eq!(targets.len(), 1, "该行恰好一条链接");
        let t = &targets[0];
        assert_eq!(t.key, "[@Storage]");
        assert_eq!((t.x0, t.y0), (8.0, 34.0));
        // `Access` = 6 个 ASCII 半宽 = 6 × (13/2) = 39
        assert_eq!(t.x1, 47.0, "链接段右边界 = 段宽（不含尾部纯文本）");
        assert_eq!(t.y1, 34.0 + NPC_ROW_HIT_H);
        // 行中心（x=60，落在 ` Storage` 上）不属于链接命中区间
        assert!(
            !(60.0 >= t.x0 && 60.0 <= t.x1),
            "尾部纯文本不得算作链接热区（否则夹具会以为自己点中了链接）"
        );
        // 链接段中心才是可点中心
        assert_eq!(t.center(), (27.5, 42.0));
    }

    /// 菜单行（`[@XXX]`，无行内标记）保持整行热区（C# MirLabel 通栏语义）
    #[test]
    fn npc_link_target_menu_line_is_whole_row() {
        let lines = vec!["[@main]".to_string()];
        let targets = npc_link_targets(&lines, 0, &[(0, 8.0, 34.0)]);
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].key, "[@main]");
        assert_eq!(targets[0].x1, 8.0 + NPC_MENU_HIT_W);
    }

    /// 滚动偏移外的行不产出命中矩形（点击分发同样只认 off..off+8）
    #[test]
    fn npc_link_target_respects_scroll_off() {
        let lines = vec![
            "line0".to_string(),
            "line1".to_string(),
            "<Access/@Storage> Storage".to_string(),
        ];
        assert!(npc_link_targets(&lines, 1, &[(2, 8.0, 70.0)]).is_empty());
        let t = npc_link_targets(&lines, 0, &[(2, 8.0, 70.0)]);
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].row, 2);
    }

    /// ⑤ 回归门禁（**上线阻塞级**）：NPC 对话里点行内选项 → 发出的 `CallNPC`
    /// 必须带**当前 NPC 的 object_id**，且 key 是链接的 key。
    ///
    /// 实测缺陷（2026-09-23）：`NpcDialogState.npc_object_id` 全仓没有任何写入点（恒 0），
    /// 于是每次点选项都发 `CallNPC{object_id: 0}`，服务端 `NPC call for unknown object_id 0`
    /// 静默丢弃——仓库 `<Access/@Storage>`、买卖入口、传送 Service 菜单、任务接受
    /// 全部"点了没反应"。客户端日志只有一行 `🧙 NPC 选项: ... → [@Storage]`，看着像成功。
    ///
    /// 阳性对照（实做）：把 `npc_call`/`interact`/世界点击三处 `npc_dialog.npc_object_id = id`
    /// 删掉（或把点击分发改回常量 0）→ 本测试立即红（解出的 object_id == 0）。
    #[test]
    fn npc_link_click_sends_callnpc_with_current_npc_id() {
        use crate::game::dialogs::{DialogKind, DialogManager};
        use crate::map_renderer::GameData;
        use crate::network::NetConnection;
        use mir2_shared::packets::base::deserialize_packet;
        use mir2_shared::packets::client::npc::CallNPC;

        const NPC_ID: u32 = 4242;
        let (tx, rx) = crossbeam_channel::unbounded::<Vec<u8>>();

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<NpcDialogState>();
        app.init_resource::<crate::game::dialogs::npc_goods::NpcGoodsState>();
        app.init_resource::<crate::game::dialogs::sell_panel::SellPanelState>();
        app.init_resource::<crate::game::dialogs::storage::StorageState>();
        app.init_resource::<DialogManager>();
        app.init_resource::<crate::control::CursorProbe>();
        app.insert_resource(NetConnection {
            to_server: Some(tx),
            ..Default::default()
        });
        app.insert_resource(GameData::default());
        app.world_mut().spawn(Window::default());

        {
            let mut npc = app.world_mut().resource_mut::<NpcDialogState>();
            npc.visible = true;
            npc.npc_object_id = NPC_ID;
            npc.lines = vec!["<Access/@Storage> Storage".to_string()];
        }
        // 面板根（scroll 查询用）：UiScrollList + NpcDialogWidget
        let panel = app
            .world_mut()
            .spawn((
                crate::ui::theme::UiScrollList {
                    rect_rel: (8.0, 34.0, 400.0, 144.0),
                    row_h: 18.0,
                    visible: 8,
                    total: 0,
                    offset: 0,
                    step: 1,
                    track_rel: (420.0, 34.0, 4.0, 144.0),
                    thumb: None,
                    z: 8,
                },
                NpcDialogWidget,
            ))
            .id();
        // 第 0 行渲染实体（原点 8,34 —— 与 spawn_npc_dialog 同式）
        let row = app
            .world_mut()
            .spawn((
                NpcLine(0),
                NpcLineSrc::default(),
                NpcDialogWidget,
                Text::default(),
                TextColor(Color::WHITE),
                TextFont::default(),
                Node {
                    left: Val::Px(8.0),
                    top: Val::Px(34.0),
                    ..Default::default()
                },
            ))
            .id();
        app.world_mut().entity_mut(panel).add_child(row);

        // 光标压在链接段 `Access` 上（x∈[8,47]）；鼠标左键刚按下
        let mut mouse = bevy::input::ButtonInput::<bevy::input::mouse::MouseButton>::default();
        mouse.press(bevy::input::mouse::MouseButton::Left);
        app.insert_resource(mouse);
        app.world_mut()
            .resource_mut::<crate::control::CursorProbe>()
            .pos = Some(Vec2::new(27.5, 42.0));
        app.add_systems(Update, npc_ui_system);
        app.update();

        let frame = rx.try_recv().expect("应发出一个 CallNPC 包");
        let mut cur = std::io::Cursor::new(frame.as_slice());
        let pkt: CallNPC = deserialize_packet(&mut cur).expect("应是合法 CallNPC 帧");
        assert_eq!(pkt.key, "[@Storage]", "链接 key 必须原样发出");
        assert_eq!(
            pkt.object_id, NPC_ID,
            "CallNPC 必须带当前 NPC 的 object_id（0 会被服务端丢弃：NPC call for unknown object_id 0）"
        );
        let _ = DialogKind::Npc;
    }

    /// ⑥ 回归门禁（上线阻塞级，实机 l5i 跨图夹具挖出）：RPC/点击写入 `npc_object_id`
    /// 后、`NPCResponse` 到达前的若干帧里，`npc_ui_system` 不得把 id 清 0——否则
    /// 开窗后点选项发 `CallNPC{object_id:0}`，服务端 warn「unknown object_id 0」
    /// 静默丢弃（传送/仓库/买卖入口「点了没反应」）。关闭边沿（可见→不可见）仍须
    /// 归零（"0 = 未开对话"语义）。
    ///
    /// 阳性对照：把 `npc_object_id = 0` 移出 `if *npc_prev_visible`（恢复每帧清零）
    /// → 第①步断言立即红。
    #[test]
    fn npc_object_id_zeroes_only_on_close_edge() {
        use crate::game::dialogs::DialogManager;
        use crate::map_renderer::GameData;
        use crate::network::NetConnection;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<NpcDialogState>();
        app.init_resource::<crate::game::dialogs::npc_goods::NpcGoodsState>();
        app.init_resource::<crate::game::dialogs::sell_panel::SellPanelState>();
        app.init_resource::<crate::game::dialogs::storage::StorageState>();
        app.init_resource::<DialogManager>();
        app.init_resource::<crate::control::CursorProbe>();
        app.insert_resource(NetConnection::default());
        app.insert_resource(GameData::default());
        app.insert_resource(bevy::input::ButtonInput::<bevy::input::mouse::MouseButton>::default());
        app.world_mut().spawn(Window::default());
        app.add_systems(Update, npc_ui_system);

        // ① RPC 开窗竞态：对话未开（visible=false）时调用方写入 id，
        //    连跑数帧（等响应）不得被清零
        app.world_mut()
            .resource_mut::<NpcDialogState>()
            .npc_object_id = 4242;
        for _ in 0..5 {
            app.update();
        }
        assert_eq!(
            app.world().resource::<NpcDialogState>().npc_object_id,
            4242,
            "开窗前调用方写入的 id 不得被每帧清零抹掉（NPCResponse 到达前的帧）"
        );

        // ② 可见→不可见边沿：必须归零（"0 = 未开对话"）
        app.world_mut().resource_mut::<NpcDialogState>().visible = true;
        app.update();
        app.world_mut().resource_mut::<NpcDialogState>().visible = false;
        app.update();
        assert_eq!(
            app.world().resource::<NpcDialogState>().npc_object_id,
            0,
            "可见→不可见边沿必须归零"
        );
    }

    /// 探针光标必须能驱动 NPC 行命中（`Click`/`cursor` RPC 注入的就是它）：
    /// 无探针时读真实窗口光标，有探针时**探针优先**——否则自动化里点链接永远无反应。
    /// 阳性对照：把 `cursor_src.pos()` 换回 `window.cursor_position()` → 本测试红
    /// （无真实光标的环境里注入坐标不生效）。
    #[test]
    fn cursor_probe_drives_npc_hit_test() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<crate::control::CursorProbe>();
        app.world_mut().spawn(Window::default());
        app.add_systems(Update, |src: crate::control::CursorSource| {
            // 只验证"探针优先"这一条通道
            assert_eq!(src.pos(), Some(Vec2::new(27.5, 42.0)));
        });
        app.world_mut()
            .resource_mut::<crate::control::CursorProbe>()
            .pos = Some(Vec2::new(27.5, 42.0));
        app.update();
    }

    /// 门禁（owner 队列 `scroll-hitrect-npc`）：NPC 对话框的滚轮命中区 = **整个对话框**，
    /// 因为 C# 把 `MouseWheel` 挂在对话框自身（`NPCDialogs.cs:64`），文本行/链接标签上那几处
    /// 只是重复挂载。判据是「命中区必须等于面板、且覆盖全部 8 行文本」。
    ///
    /// 阳性对照：把 `LIST_WHEEL_RECT` 改回旧值 `(8,34,400,144)` → 第 1 条断言立即红；
    /// 把 `LINE_COUNT` 加到 12（行区超出面板）→ 第 2 条断言红。
    #[test]
    fn npc_wheel_rect_is_whole_dialog_like_csharp() {
        assert_eq!(
            LIST_WHEEL_RECT,
            (0.0, 0.0, PANEL_W, PANEL_H),
            "C# `NPCDialog` 构造里把滚轮挂在对话框自身（NPCDialogs.cs:64）→ 命中区=整个面板"
        );
        let last_bottom = LINE_Y0 + (LINE_COUNT as f32 - 1.0) * LINE_PITCH + LINE_PITCH;
        assert!(
            last_bottom <= PANEL_H,
            "8 行文本（最后一行底 {last_bottom}）必须在面板内，否则命中区覆盖不到"
        );
        assert!(
            LIST_WHEEL_RECT.0 <= LINE_X && LIST_WHEEL_RECT.1 <= LINE_Y0,
            "命中区左上角必须不晚于文本行区左上角"
        );
        // 旧值必须与新版不同，且确实比面板小（这就是当初「原版能滚、本端滚不动」的量）
        let old = (8.0f32, 34.0f32, 400.0f32, 144.0f32);
        assert_ne!(
            LIST_WHEEL_RECT, old,
            "不得退回「只有 8 行文本框」的旧命中区"
        );
        assert!(
            old.2 < PANEL_W && old.3 < PANEL_H,
            "旧命中区比面板小一圈（右侧 {:.0}px、底部 {:.0}px 滚不动）",
            PANEL_W - old.2,
            PANEL_H - old.3
        );
    }

    /// 门禁（金标准逐窗复核 · MC 窗）：NPC 面板背景必须是 `Prguse[995]`。
    ///
    /// 依据：C# `NPCDialog` 构造器 `Index = 995; Library = Libraries.Prguse;`（`NPCDialogs.cs:52`）。
    /// 本端曾用 `Prguse[384]` —— 两张图**同为 440x224**，所以窗口级几何对表（`window_rect_table.py`）
    /// 与写死尺寸审计（`control_size_audit.py`）都看不出来，只有把两张图导出来比才看得见
    /// （384 的框线更细、正文区多一条横向分隔线）。
    ///
    /// 阳性对照：把常量改回 `(LibraryName::Prguse, 384)` → 本测试红。
    #[test]
    fn npc_panel_is_prguse_995() {
        assert_eq!(
            NPC_PANEL,
            (LibraryName::Prguse, 995),
            "C# NPCDialog 背景是 Prguse[995]（不是 384；两者同尺寸，只有比图才看得出）"
        );
    }

    /// 门禁（金窗逐窗复核 · NPC 窗）：翻页箭头的位置/三帧/尺寸逐字对齐 C#。
    ///
    /// 依据：`NPCDialogs.cs:78-108` —— `UpButton` `Index=197/HoverIndex=198/PressedIndex=199`、
    /// `Size=(16,14)`、`Location=(417,34)`；`DownButton` `207/208/209`、`Size=(16,14)`、`(417,175)`。
    /// 本端此前**没有这两颗钮**（只有滚轮），是"少画控件"而不是尺寸问题。
    ///
    /// 阳性对照：把 `NPC_UP_POS` 改成 `(420.0, 34.0)`（旧滚动条轨道 x）或把 `NPC_UP_FRAMES`
    /// 改成 `(0,1,2)` → 本测试红。
    #[test]
    fn npc_scroll_arrows_match_csharp() {
        // **库名是最容易漏的一项**：写成 `Prguse` 时 `Prguse[197]/[207]` 是空图（0x0）⇒
        // `load_lib_image` 返回 None、`if let (Some,Some,Some)` 静默跳过 ⇒ 实机上"补了等于没补"，
        // 而只查位置/索引/尺寸的门禁看不出来（2026-09-26 实测：实机 ui_nodes_at 在 (425,41) 只有面板根）。
        assert_eq!(
            NPC_ARROW_LIB,
            LibraryName::Prguse2,
            "C# `UpButton`/`DownButton` 的 Library 是 Prguse2（NPCDialogs.cs:80/83、101/104）"
        );
        assert_eq!(NPC_UP_POS, (417.0, 34.0));
        assert_eq!(NPC_DOWN_POS, (417.0, 175.0));
        assert_eq!(NPC_ARROW_SIZE, (16.0, 14.0));
        assert_eq!(NPC_UP_FRAMES, (197, 198, 199));
        assert_eq!(NPC_DOWN_FRAMES, (207, 208, 209));
    }

    /// 门禁：翻页箭头的点击语义逐字对齐 C#（`NPCDialogs.cs:82-99`）。
    ///
    /// 上：`if (_index <= 0) return;` → 0 时不动，其余 −1；
    /// 下：`if (_index + MaximumLines >= 行数) return;` → 到底不动，其余 +1。
    ///
    /// 阳性对照：把 `npc_scroll_up` 改成 `offset.saturating_sub(2)`、或把 `npc_scroll_down` 的
    /// 钳位条件写成 `offset >= total` → 本测试红。
    #[test]
    fn npc_arrow_scroll_semantics_match_csharp() {
        assert_eq!(npc_scroll_up(0), 0, "到顶不动（C# `_index <= 0 return`）");
        assert_eq!(npc_scroll_up(3), 2);
        // 10 行、可见 8 行：offset 2 已到底（2+8 >= 10）⇒ 下不动
        assert_eq!(npc_scroll_down(2, 10, 8), 2, "到底不动（C# `_index + MaximumLines >= 行数`）");
        assert_eq!(npc_scroll_down(1, 10, 8), 2);
        assert_eq!(npc_scroll_down(0, 8, 8), 0, "刚好一屏时不滚");
        assert_eq!(npc_scroll_down(0, 1, 8), 0, "只有一行时不滚");
    }
}

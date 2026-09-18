// ============================================================================
// 排名对话框（M9 第 3 批）
// 布局参考：C# `Client/MirScenes/Dialogs/RankingDialog.cs`
//   - 背景 Title[728]（原生 324x441）@ C# 居中公式 (350,163)
//   - #2892 批A 单元②：子控件全部按 C# 精灵与坐标（页签 Title[751..768] 图标、
//     关闭 Prguse2[360..362] 24x21@(300,3)、翻页 Prguse2[197..199]@(299,100) 与
//     [207..209]@(299,386)、滚动条 Prguse2[205/206]@(299,113)、
//     仅在线 Prguse[2086/2087]@(190,H-20)、MyRank 82x22@(229,36)、20 行 @(32,98+i*15)
//     四列 0/55/150/220）
//   - 仍待办：滚动条拖动（服务端只回前 20 名 → C# 亦不动，见 `SCROLL_POS` 注释）
// 网络：Ranking 请求 → 服务器回排名 → 显示
// ============================================================================

use bevy::prelude::*;

use crate::actor::{LocalPlayer, PlayerName};
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{shared_cjk_font, UiCjkFont};
use crate::ui::theme::{
    load_lib_image, spawn_icon_button, spawn_label, spawn_label_center, spawn_panel, CloseButton,
    ImageButton,
};

/// #2892：面板几何对齐 C# `RankingDialog.cs:37-46`——
/// `Index = 728; Library = Libraries.Title;`（原生 324x441），
/// `Location = new Point((Settings.ScreenWidth - Size.Width) / 2,
/// (Settings.ScreenHeight - Size.Height) / 2)`。
/// 1024x768 屏上即 (350,163)（C# 为整数除法，等价 `dialogs::center_origin`）。
///
/// 此前硬编码 (200,150)（macroquad 迁移样板遗留）→ 整窗偏移 (150,13)，玩家可见。
pub const PANEL_W: f32 = 324.0;
pub const PANEL_H: f32 = 441.0;
pub const PANEL_ORIGIN: (f32, f32) = (350.0, 163.0);

// ---------------------------------------------------------------------------
// #2892 批A 单元②：子控件几何全部按 C# `Client/MirScenes/Dialogs/RankingDialog.cs`
// （1024x768 固定布局，下列坐标均为**面板内**相对坐标）。
// ---------------------------------------------------------------------------

/// 行：C# `RankingRow` 20 个，`Location = (32, 98 + i*15)`、`Size = (270, 15)`（`:204-211`）
pub const ROW_COUNT: usize = 20;
pub const ROW_X: f32 = 32.0;
pub const ROW_Y0: f32 = 98.0;
pub const ROW_W: f32 = 270.0;
pub const ROW_H: f32 = 15.0;
/// 行内四列左边界：RankLabel(0,0)/NameLabel(55,0)/ClassLabel(150,0)/LevelLabel(220,0)（`:341-382`）
pub const ROW_LABEL_X: [f32; 4] = [0.0, 55.0, 150.0, 220.0];

/// 关闭键：`Prguse2[360..362]` 24x21 @(300,3)（`:47-58`）
pub const CLOSE_POS: (f32, f32) = (300.0, 3.0);
pub const CLOSE_SIZE: (f32, f32) = (24.0, 21.0);

/// 页签：构造顺序 All/Tao/War/Wiz/Sin/Arch，`Location = (10/40/60/80/100/120, 38)`（`:60-135`）
pub const TAB_POS: [(f32, f32); 6] = [
    (10.0, 38.0),
    (40.0, 38.0),
    (60.0, 38.0),
    (80.0, 38.0),
    (100.0, 38.0),
    (120.0, 38.0),
];
/// 页签三帧（normal/hover/pressed）：`Title[751..753]`(All) `[760..762]`(Tao) `[754..756]`(War)
/// `[763..765]`(Wiz) `[757..759]`(Sin) `[766..768]`(Arch)
pub const TAB_FRAMES: [(usize, usize, usize); 6] = [
    (751, 752, 753),
    (760, 761, 762),
    (754, 755, 756),
    (763, 764, 765),
    (757, 758, 759),
    (766, 767, 768),
];
/// 页签 → `SelectRank` 值（C# 构造顺序：All→0、Tao→3、War→1、Wiz→2、Sin→4、Arch→5）
pub const TAB_RANK: [u8; 6] = [0, 3, 1, 2, 4, 5];
/// 页签精灵原生尺寸（`Title[751]`=28x24，其余 24x20）
pub const TAB_SIZE: [(f32, f32); 6] = [
    (28.0, 24.0),
    (24.0, 20.0),
    (24.0, 20.0),
    (24.0, 20.0),
    (24.0, 20.0),
    (24.0, 20.0),
];

/// 翻页：`PrevButton Prguse2[197..199]` 12x12 @(299,100)、`NextButton [207..209]` @(299,386)（`:137-152`）
pub const PREV_POS: (f32, f32) = (299.0, 100.0);
pub const NEXT_POS: (f32, f32) = (299.0, 386.0);
pub const PAGE_SIZE: (f32, f32) = (12.0, 12.0);

/// 滚动条：`ScrollBar Prguse2[205/206]` 12x18 @ `(299, 100+13)`，拖动时 y 钳 [110,368]（`:153-181`）
pub const SCROLL_POS: (f32, f32) = (299.0, 113.0);
pub const SCROLL_SIZE: (f32, f32) = (12.0, 18.0);

/// 仅在线：`OnlineOnlyButton Prguse[2086]/[2087]` @ `(190, Size.Height-20)`（`:184`）
pub const ONLINE_POS: (f32, f32) = (190.0, PANEL_H - 20.0);

/// 我的排名：`MyRank` 82x22 @(229,36)，`Color.BurlyWood`、水平垂直居中（`:197-206`）
pub const MYRANK_POS: (f32, f32) = (229.0, 36.0);
pub const MYRANK_SIZE: (f32, f32) = (82.0, 22.0);

/// C# 行文字色（`RankingRow.Update` `:396-424`）：1=Gold、2=Silver、3=RosyBrown、
/// 自己=Green、其余=White。注意 C# 的 `if (==3) {} else if (自己) {} else if (>3) {}`
/// 链**会**把第 1/2 名的自己覆盖成 Green —— 原样保留该怪癖。
const COLOR_GOLD: Color = Color::srgb(1.0, 0.843, 0.0);
const COLOR_SILVER: Color = Color::srgb(0.753, 0.753, 0.753);
const COLOR_ROSY_BROWN: Color = Color::srgb(0.737, 0.561, 0.561);
const COLOR_RANK_GREEN: Color = Color::srgb(0.0, 0.502, 0.0);
const COLOR_BURLY_WOOD: Color = Color::srgb(0.871, 0.722, 0.529);

/// 行文字色（逐字复刻 C# `RankingRow.Update` 的 if/else-if 链，见 [`COLOR_GOLD`] 注释）
fn rank_row_color(rank: i32, name: &str, self_name: &str) -> Color {
    let mut c = Color::WHITE;
    if rank == 1 {
        c = COLOR_GOLD;
    }
    if rank == 2 {
        c = COLOR_SILVER;
    }
    if rank == 3 {
        c = COLOR_ROSY_BROWN;
    } else if name == self_name {
        c = COLOR_RANK_GREEN;
    } else if rank > 3 {
        c = Color::WHITE;
    }
    c
}

/// 排名条目（服务端 Rankings 包）
#[derive(Debug, Clone, Default)]
pub struct RankEntry {
    pub rank: i32,
    /// 玩家 object_id（离线角色为 0；排行榜行点击查看用）
    pub player_id: u32,
    pub player_name: String,
    pub class: u8,
    pub level: i32,
    pub experience: i64,
}

#[derive(Resource, Default)]
pub struct RankingState {
    pub visible: bool,
    pub entries: Vec<RankEntry>,
    /// 当前页签（0=All 1..5=职业，C# RankingDialog SelectRank）
    pub tab: u8,
    /// 仅在线（C# RankingDialog OnlineOnly）
    pub online_only: bool,
    /// 我的排名（C# MyRank；0=未上榜）
    pub my_rank: i32,
    /// 当前页首行在过滤后列表中的下标（翻页游标；跨系统共享，见 `ranking_row_click_system`）
    pub page_offset: usize,
}

#[derive(Component)]
pub struct RankingWidget;

#[derive(Component)]
pub struct RankingClose;

#[derive(Component)]
pub struct RankingLine(usize);

/// 行内文字格（C# `RankingRow` 的 RankLabel/NameLabel/ClassLabel/LevelLabel）
/// `field`：0=排名 1=名字 2=职业 3=等级
#[derive(Component)]
pub struct RankingCell {
    pub row: usize,
    pub field: u8,
}

/// 页签按钮（C# AllButton/WarButton/WizButton/TaoButton/SinButton/ArchButton）
#[derive(Component)]
pub struct RankingTab(pub u8);

/// 上一页（C# PrevButton）
#[derive(Component)]
pub struct RankingPrev;

/// 下一页（C# NextButton）
#[derive(Component)]
pub struct RankingNext;

/// 仅在线（C# OnlineOnlyButton）
#[derive(Component)]
pub struct RankingOnlineOnly;

/// 我的排名标签（C# MyRank）
#[derive(Component)]
pub struct RankingMyRank;

/// 职业名（C# 排行榜职业页签）
pub fn rank_class_name(class: u8) -> &'static str {
    match class {
        0 => "战士",
        1 => "法师",
        2 => "道士",
        3 => "刺客",
        4 => "弓箭手",
        _ => "未知",
    }
}

/// 按页签过滤（0=全部，1..5=对应职业；服务端暂返回全职业，本地过滤对齐 C# 页签语义）
pub fn filter_rank_tab(entries: &[RankEntry], tab: u8) -> Vec<RankEntry> {
    if tab == 0 {
        return entries.to_vec();
    }
    entries
        .iter()
        .filter(|e| e.class + 1 == tab)
        .cloned()
        .collect()
}

/// C# `GameScene.InspectTime = CMain.Time + 500`（`RankingDialog.cs:376-378`）——排行榜行点击 500ms 节流
const RANK_INSPECT_COOLDOWN_SECS: f64 = 0.5;

/// #1225：当前页第 `line` 行对应的条目（C# `RankingRow` 是全局序号，本端按 `offset + line` 的页窗口取值）。
fn rank_row_entry<'a>(
    filtered: &'a [RankEntry],
    offset: usize,
    line: usize,
) -> Option<&'a RankEntry> {
    filtered.get(offset + line)
}

/// #1225：行点击节流判定（C# `if (CMain.Time <= GameScene.InspectTime) return;`——相等也算冷却中）
fn rank_inspect_allowed(now_secs: f64, cooldown_until_secs: f64) -> bool {
    now_secs > cooldown_until_secs
}

/// Interaction 边沿检测：仅当从非 Pressed → Pressed 那帧触发一次（bevy_ui 无 just_pressed）
fn edge(
    e: Entity,
    inter: &Interaction,
    prev: &mut std::collections::HashMap<Entity, Interaction>,
) -> bool {
    let was = prev.insert(e, *inter);
    *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
}

pub struct RankingPlugin;

impl Plugin for RankingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RankingState>();
        app.add_systems(
            Update,
            ranking_server_events.run_if(in_state(AppState::Game)),
        );
        app.add_systems(OnEnter(AppState::Game), spawn_ranking);
        app.add_systems(OnExit(AppState::Game), cleanup_ranking);
        app.add_systems(
            Update,
            (ranking_ui_system, ranking_row_click_system).run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_ranking(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_ranking(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut fonts: ResMut<Assets<Font>>,
    mut cjk_font: ResMut<UiCjkFont>,
    mut libs: ResMut<GameLibraries>,
    ranking: Res<RankingState>,
) {
    // 面板内文案全是中文：#2775 与批17 同因（Arial 无 CJK 且 parley 的 Hani 回退只在
    // 实体首次排版生效）——本窗原先整屏豆腐，改用共享宋体主字体。
    let font = shared_cjk_font(&mut fonts, &mut cjk_font);

    // bevy_ui 面板 Title[728]（324x441）@ C# 居中 (350,163)
    let Some(bg) =
        crate::ui::theme::load_lib_image(&mut libs, &mut images, LibraryName::Title, 728)
    else {
        return;
    };
    let panel = crate::ui::theme::spawn_panel(
        &mut commands,
        bg,
        PANEL_ORIGIN.0,
        PANEL_ORIGIN.1,
        PANEL_W,
        PANEL_H,
        40,
    );
    commands
        .entity(panel)
        .insert((DialogRoot(DialogKind::Ranking), RankingWidget));

    commands.entity(panel).with_children(|p| {
        // 关闭 X：C# `CloseButton Prguse2[360..362]` 24x21 @(300,3)（`:47-58`）
        if let (Some(n), Some(h), Some(pr)) = (
            crate::ui::theme::load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 360),
            crate::ui::theme::load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 361),
            crate::ui::theme::load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 362),
        ) {
            crate::ui::theme::spawn_icon_button(
                p,
                n,
                h,
                pr,
                CLOSE_POS.0,
                CLOSE_POS.1,
                CLOSE_SIZE.0,
                CLOSE_SIZE.1,
                10,
            )
            .insert((RankingClose, CloseButton));
        }
        // 页签 6 个（构造顺序 All/Tao/War/Wiz/Sin/Arch，`SelectRank` 映射见 `TAB_RANK`）。
        // Hint 文案逐项取 C# `RankingDialog.cs:65/89/101/77/113/125`。
        const TAB_HINTS: [&str; 6] = [
            "总榜前 20",
            "道士前 20",
            "战士前 20",
            "法师前 20",
            "刺客前 20",
            "弓箭手前 20",
        ];
        for i in 0..6usize {
            let (n_i, h_i, p_i) = TAB_FRAMES[i];
            if let (Some(n), Some(h), Some(pr)) = (
                crate::ui::theme::load_lib_image(&mut libs, &mut images, LibraryName::Title, n_i),
                crate::ui::theme::load_lib_image(&mut libs, &mut images, LibraryName::Title, h_i),
                crate::ui::theme::load_lib_image(&mut libs, &mut images, LibraryName::Title, p_i),
            ) {
                crate::ui::theme::spawn_icon_button(
                    p,
                    n,
                    h,
                    pr,
                    TAB_POS[i].0,
                    TAB_POS[i].1,
                    TAB_SIZE[i].0,
                    TAB_SIZE[i].1,
                    10,
                )
                .insert((
                    RankingTab(TAB_RANK[i]),
                    crate::ui::tooltip::UiHint {
                        text: TAB_HINTS[i].to_string(),
                    },
                ));
            }
        }
        // 上一页 / 下一页：C# `Prguse2[197..199]`@(299,100)、`[207..209]`@(299,386)
        if let (Some(n), Some(h), Some(pr)) = (
            crate::ui::theme::load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 197),
            crate::ui::theme::load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 198),
            crate::ui::theme::load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 199),
        ) {
            crate::ui::theme::spawn_icon_button(
                p,
                n,
                h,
                pr,
                PREV_POS.0,
                PREV_POS.1,
                PAGE_SIZE.0,
                PAGE_SIZE.1,
                10,
            )
            .insert(RankingPrev);
        }
        if let (Some(n), Some(h), Some(pr)) = (
            crate::ui::theme::load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 207),
            crate::ui::theme::load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 208),
            crate::ui::theme::load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 209),
        ) {
            crate::ui::theme::spawn_icon_button(
                p,
                n,
                h,
                pr,
                NEXT_POS.0,
                NEXT_POS.1,
                PAGE_SIZE.0,
                PAGE_SIZE.1,
                10,
            )
            .insert(RankingNext);
        }
        // 滚动条手柄：C# `Prguse2[205/206]` 12x18 @(299,113)。
        // 拖动（C# `OnMoving` → `RowOffset`）本端暂不接线：服务端固定只回前 20 名
        // （`ServerRust/.../npc.rs` `take(20)`），故 C# `Move()` 的 `RankCount-20` 恒 0、
        // `GapPerRow = ScrollHeight / 0` —— 原版在该数据下同样不动（见 #2892 记录）。
        if let (Some(n), Some(h)) = (
            crate::ui::theme::load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 205),
            crate::ui::theme::load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 206),
        ) {
            crate::ui::theme::spawn_icon_button(
                p,
                n.clone(),
                h,
                n,
                SCROLL_POS.0,
                SCROLL_POS.1,
                SCROLL_SIZE.0,
                SCROLL_SIZE.1,
                10,
            );
        }
        // 仅在线勾选框：C# `Prguse[2086]`(未勾)/`[2087]`(勾) @(190, H-20)，右侧 LabelText
        if let (Some(u), Some(t)) = (
            crate::ui::theme::load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 2086),
            crate::ui::theme::load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 2087),
        ) {
            crate::ui::theme::spawn_icon_button(
                p,
                u.clone(),
                t.clone(),
                u,
                ONLINE_POS.0,
                ONLINE_POS.1,
                16.0,
                13.0,
                10,
            )
            .insert(RankingOnlineOnly);
        }
        crate::ui::theme::spawn_label(
            p,
            &font,
            // C# `OnlineOnlyButton.LabelText` = `ClientTextKeys.OnlineOnly`（Chinese.json「仅限在线」）
            "仅限在线",
            ONLINE_POS.0 + 20.0,
            ONLINE_POS.1 + 1.0,
            12.0,
            Color::srgb(0.8, 0.9, 1.0),
            9,
        );
        // 我的排名：C# `MyRank` 82x22 @(229,36) BurlyWood 居中（点击处理 `GoToMyRank()` 在 C# 是空函数）
        crate::ui::theme::spawn_label_center(
            p,
            &font,
            "",
            MYRANK_POS.0 + MYRANK_SIZE.0 / 2.0,
            MYRANK_POS.1,
            MYRANK_SIZE.0,
            12.0,
            COLOR_BURLY_WOOD,
            9,
        )
        .insert(RankingMyRank);
        // 20 行：C# `RankingRow @(32, 98+i*15) 270x15`，整行可点（`RankingRow.Click → Inspect()`），
        // 行内四列文字 0/55/150/220。
        for i in 0..ROW_COUNT {
            p.spawn((
                Button,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(ROW_X),
                    top: Val::Px(ROW_Y0 + i as f32 * ROW_H),
                    width: Val::Px(ROW_W),
                    height: Val::Px(ROW_H),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                ZIndex(9),
                RankingLine(i),
            ))
            .with_children(|row| {
                for (field, x) in ROW_LABEL_X.iter().enumerate() {
                    crate::ui::theme::spawn_label(row, &font, "", *x, 0.0, 12.0, Color::WHITE, 9)
                        .insert(RankingCell {
                            row: i,
                            field: field as u8,
                        });
                }
            });
        }
    });
}

fn ranking_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut ranking: ResMut<RankingState>,
    net: Res<NetConnection>,
    local_player: Query<&PlayerName, With<LocalPlayer>>,
    mut widgets: Query<&mut Visibility, With<RankingWidget>>,
    close: Query<(Entity, &Interaction), (With<RankingClose>, Without<RankingTab>)>,
    tabs: Query<(Entity, &Interaction, &RankingTab)>,
    prev: Query<
        (Entity, &Interaction),
        (With<RankingPrev>, Without<RankingTab>, Without<RankingNext>),
    >,
    next: Query<
        (Entity, &Interaction),
        (With<RankingNext>, Without<RankingTab>, Without<RankingPrev>),
    >,
    mut online: Query<
        (Entity, &Interaction, &ImageButton, &mut ImageNode),
        (With<RankingOnlineOnly>, Without<RankingTab>),
    >,
    mut my_rank_text: Query<&mut Text, (With<RankingMyRank>, Without<RankingCell>)>,
    mut cells: Query<(&mut Text, &mut TextColor, &RankingCell)>,
    mut prev_inter: Local<std::collections::HashMap<Entity, Interaction>>,
    mut requested: Local<bool>,
) {
    let open = ranking.visible || mgr.is_open(DialogKind::Ranking);
    for mut vis in widgets.iter_mut() {
        *vis = if open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !open {
        *requested = false;
        ranking.page_offset = 0;
        return;
    }

    // 关闭（点 X → 关闭排行榜）
    for (e, inter) in &close {
        if edge(e, inter, &mut prev_inter) {
            mgr.close(DialogKind::Ranking);
        }
    }
    // 打开瞬间请求排行榜（C# RankingDialog.Show → GetRanking）
    if !*requested {
        *requested = true;
        net.send_packet(&mir2_shared::packets::client::misc::GetRanking {
            rank_index: ranking.tab,
            online_only: ranking.online_only,
        });
        tracing::info!("🏅 请求排行榜");
    }
    let filtered = filter_rank_tab(&ranking.entries, ranking.tab);
    let max_offset = filtered.len().saturating_sub(ROW_COUNT);
    // 页签切换
    for (e, inter, t) in &tabs {
        if edge(e, inter, &mut prev_inter) && ranking.tab != t.0 {
            ranking.tab = t.0;
            ranking.page_offset = 0;
            net.send_packet(&mir2_shared::packets::client::misc::GetRanking {
                rank_index: t.0,
                online_only: ranking.online_only,
            });
            tracing::info!(
                "🏅 排行榜页签 {}",
                if t.0 == 0 {
                    "全部"
                } else {
                    rank_class_name(t.0 - 1)
                }
            );
        }
    }
    // 上一页 / 下一页（C# `RankingDialog.Move(±1)` `:239-252`：RowOffset 钳在 [0, RankCount-20]）
    for (e, inter) in &prev {
        if edge(e, inter, &mut prev_inter) {
            ranking.page_offset = ranking.page_offset.saturating_sub(1);
        }
    }
    for (e, inter) in &next {
        if edge(e, inter, &mut prev_inter) {
            ranking.page_offset = (ranking.page_offset + 1).min(max_offset);
        }
    }
    ranking.page_offset = ranking.page_offset.min(max_offset);
    // 仅在线（切换 + 帧同步）
    for (e, inter, ib, mut node) in &mut online {
        if edge(e, inter, &mut prev_inter) {
            ranking.online_only = !ranking.online_only;
            ranking.page_offset = 0;
            net.send_packet(&mir2_shared::packets::client::misc::GetRanking {
                rank_index: ranking.tab,
                online_only: ranking.online_only,
            });
            tracing::info!("🏅 排行榜仅在线 {}", ranking.online_only);
        }
        let want = if ranking.online_only {
            &ib.pressed
        } else {
            &ib.normal
        };
        if node.image != *want {
            node.image = want.clone();
        }
    }
    // 行文字（C# `RankingRow` 四列：排名/名字/职业/等级 + 名次着色）
    let self_name = local_player
        .single()
        .map(|n| n.0.clone())
        .unwrap_or_default();
    for (mut text, mut color, cell) in &mut cells {
        let (line, want) = match (filtered.get(ranking.page_offset + cell.row), cell.field) {
            (Some(e), f @ 0..=3) => {
                let s = match f {
                    0 => e.rank.to_string(),
                    1 => e.player_name.clone(),
                    2 => rank_class_name(e.class).to_string(),
                    _ => e.level.to_string(),
                };
                (s, rank_row_color(e.rank, &e.player_name, &self_name))
            }
            _ => (String::new(), Color::WHITE),
        };
        if text.0 != line {
            text.0 = line;
        }
        if color.0 != want {
            color.0 = want;
        }
    }
    // 我的排名（C# `MyRank`：`Ranked` = 「排名：{0}」/ `NotListed` = 「未列出」，`RankingDialog.cs:323-326`）
    for mut text in &mut my_rank_text {
        text.0 = if ranking.my_rank > 0 {
            format!("排名：{}", ranking.my_rank)
        } else {
            "未列出".to_string()
        };
    }
}

/// 行点击查看（C# `RankingDialog.cs:336` `RankingRow.Click → Inspect()`；`:374-380`：
/// `if (CMain.Time <= GameScene.InspectTime) return;` 500ms 节流 → `InspectDialog.InspectID = Index`
/// → `C.Inspect{ObjectID, Ranking=true}`）
///
/// 独立成系统：`ranking_ui_system` 参数已达 Bevy 16 上限（加查询即编译失败须拆独立系统）。
fn ranking_row_click_system(
    mut mgr: ResMut<DialogManager>,
    ranking: Res<RankingState>,
    net: Res<NetConnection>,
    rows: Query<(Entity, &Interaction, &RankingLine)>,
    mut prev_inter: Local<std::collections::HashMap<Entity, Interaction>>,
    time: Res<Time>,
    mut inspect_cooldown_until: Local<f64>,
) {
    if !(ranking.visible || mgr.is_open(DialogKind::Ranking)) {
        return;
    }
    let filtered = filter_rank_tab(&ranking.entries, ranking.tab);
    for (e, inter, line) in &rows {
        if !edge(e, inter, &mut prev_inter) {
            continue;
        }
        let Some(entry) = rank_row_entry(&filtered, ranking.page_offset, line.0) else {
            continue;
        };
        let now = time.elapsed_secs_f64();
        if !rank_inspect_allowed(now, *inspect_cooldown_until) {
            continue;
        }
        *inspect_cooldown_until = now + RANK_INSPECT_COOLDOWN_SECS;
        if !mgr.is_open(DialogKind::Inspect) {
            mgr.open(DialogKind::Inspect);
        }
        net.send_packet(&mir2_shared::packets::client::chat::Inspect {
            object_id: entry.player_id,
            ranking: true,
            name: entry.player_name.clone(),
        });
        tracing::info!(
            "🔍 查看排行榜玩家 {} (id={})",
            entry.player_name,
            entry.player_id
        );
    }
}

/// 消费服务端排行榜事件（网络层只广播 ServerEvent）
fn ranking_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut ranking: ResMut<RankingState>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        match ev {
            ServerEvent::Rankings { entries, my_rank } => {
                ranking.entries = entries.clone();
                ranking.my_rank = *my_rank;
            }
            ServerEvent::RankingsCleared => {
                ranking.entries.clear();
            }
            _ => {}
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    /// #2892：面板原点必须等于 C# `RankingDialog.cs:45` 的居中公式
    /// `((1024-W)/2, (768-H)/2)`（整数除法 → `dialogs::center_origin`）。
    #[test]
    fn panel_origin_matches_csharp_centering() {
        use crate::game::dialogs::center_origin;
        assert_eq!(
            center_origin(PANEL_W, PANEL_H),
            PANEL_ORIGIN,
            "原点应为 C# 居中公式结果（此前硬编码 (200,150) 偏 (150,13)）"
        );
        assert_eq!(
            PANEL_ORIGIN,
            (350.0, 163.0),
            "C# (1024-324)/2=350、(768-441)/2=163（整数除法）"
        );
    }

    fn entry(rank: i32, class: u8) -> RankEntry {
        RankEntry {
            rank,
            player_id: 0,
            player_name: format!("p{}", rank),
            class,
            level: 10,
            experience: 0,
        }
    }

    #[test]
    fn rank_tab_filter() {
        let entries = vec![
            entry(1, 0),
            entry(2, 1),
            entry(3, 2),
            entry(4, 3),
            entry(5, 4),
        ];
        assert_eq!(filter_rank_tab(&entries, 0).len(), 5);
        assert_eq!(filter_rank_tab(&entries, 1).len(), 1);
        assert_eq!(filter_rank_tab(&entries, 1)[0].rank, 1);
        assert_eq!(filter_rank_tab(&entries, 4)[0].rank, 4);
        assert_eq!(filter_rank_tab(&entries, 5)[0].rank, 5);
        assert!(filter_rank_tab(&entries, 6).is_empty());
    }

    /// #1225：行点击 → 当前页条目映射 + 500ms 节流（C# `RankingDialog.cs:374-380`）
    #[test]
    fn rank_row_click_window_and_throttle() {
        let entries: Vec<RankEntry> = (1..=25).map(|i| entry(i, 0)).collect();
        let filtered = filter_rank_tab(&entries, 0);
        // 第一页第 0 行 = 第 1 名
        assert_eq!(rank_row_entry(&filtered, 0, 0).map(|e| e.rank), Some(1));
        // 第二页（offset=10）第 0 行 = 第 11 名、第 4 行 = 第 15 名
        assert_eq!(rank_row_entry(&filtered, 10, 0).map(|e| e.rank), Some(11));
        assert_eq!(rank_row_entry(&filtered, 10, 4).map(|e| e.rank), Some(15));
        // 越界行（列表不足）→ None，不发包
        assert!(rank_row_entry(&filtered, 20, 5).is_none());
        // 节流：冷却期内不允许；`<=` 边界同样算冷却中（C# `CMain.Time <= InspectTime`）
        assert!(rank_inspect_allowed(1.0, 0.0));
        assert!(rank_inspect_allowed(1.51, 1.0));
        assert!(!rank_inspect_allowed(1.4, 1.5));
        assert!(!rank_inspect_allowed(1.5, 1.5));
    }

    /// #1225 行为级：真实 `App` + 真实 [`ranking_row_click_system`] —— 按下行 → 发
    /// `Inspect{object_id, ranking:true}` + 打开查看窗；500ms 冷却内再按不发包，冷却过后恢复。
    #[test]
    fn rank_row_click_emits_inspect_with_cooldown() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<DialogManager>();
        app.insert_resource(NetConnection::default());
        app.add_systems(Update, ranking_row_click_system);
        // 第二页（offset=10）第 1 行 → 第 12 名（player_id=120，仅 id 递增便于断言）
        let mut ranking = RankingState {
            visible: true,
            ..Default::default()
        };
        ranking.entries = (1..=12)
            .map(|i| RankEntry {
                player_id: i as u32 * 10,
                ..entry(i, 0)
            })
            .collect();
        ranking.page_offset = 10;
        app.insert_resource(ranking);
        let (tx, rx) = crossbeam_channel::unbounded::<Vec<u8>>();
        app.world_mut().resource_mut::<NetConnection>().to_server = Some(tx);
        let row = app
            .world_mut()
            .spawn((RankingLine(1), Interaction::None))
            .id();
        // 让 `Time` 走过 1s，脱离「启动瞬间 = 0」以免被 C# 同款 `<=` 边界挡住
        advance_time(&mut app, 1.0);

        // 帧 1：按下 → 发包 + 打开查看窗
        app.world_mut().entity_mut(row).insert(Interaction::Pressed);
        app.update();
        let raw = rx.try_recv().expect("按下排行榜行后应发出 Inspect 包");
        let mut cur = std::io::Cursor::new(raw);
        let inspect: mir2_shared::packets::client::chat::Inspect =
            mir2_shared::packets::base::deserialize_packet(&mut cur).expect("应为 Inspect 包");
        assert_eq!(
            inspect.object_id, 120,
            "应查看「当前页第 1 行」的条目（页窗口 offset + line）"
        );
        assert!(inspect.ranking, "排行榜查看须置 Ranking=true");
        assert_eq!(inspect.name, "p12");
        assert!(
            app.world()
                .resource::<DialogManager>()
                .is_open(DialogKind::Inspect),
            "行点击应打开查看窗"
        );

        // 帧 2：持续按下（无边沿）→ 不重复发包
        app.update();
        assert!(rx.try_recv().is_err(), "持续按下不应连续发包");

        // 帧 3-4：松开再按下 → 冷却中仍不发包
        app.world_mut().entity_mut(row).insert(Interaction::None);
        app.update();
        app.world_mut().entity_mut(row).insert(Interaction::Pressed);
        app.update();
        assert!(
            rx.try_recv().is_err(),
            "500ms 冷却内再按不发包（C# `CMain.Time <= GameScene.InspectTime`）"
        );

        // 冷却过后（>500ms）恢复响应
        advance_time(&mut app, 1.0);
        app.world_mut().entity_mut(row).insert(Interaction::None);
        app.update();
        app.world_mut().entity_mut(row).insert(Interaction::Pressed);
        app.update();
        assert!(rx.try_recv().is_ok(), "冷却过后应恢复响应");
    }

    /// 测试用：把虚拟时间推进 `secs`（`Time` 由 `MinimalPlugins` 的 `TimePlugin` 从它派生）。
    fn advance_time(app: &mut App, secs: f64) {
        app.world_mut()
            .resource_mut::<bevy::time::Time<bevy::time::Virtual>>()
            .advance_by(std::time::Duration::from_secs_f64(secs));
    }

    #[test]
    fn rank_class_names() {
        assert_eq!(rank_class_name(0), "战士");
        assert_eq!(rank_class_name(4), "弓箭手");
        assert_eq!(rank_class_name(99), "未知");
    }
}

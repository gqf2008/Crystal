// ============================================================================
// 大地图对话框（M53）
// 参考：C# BigMapDialog（Client/MirScenes/Dialogs/BigMapDialog.cs）
//   - 面板 Title[820]（760x500）居中；标题 (19,6)、关闭 (W-25,3)
//   - 视口 568x380 @ (14,52)：地形纹理（由地图瓦片采样生成）+ 玩家/ NPC 点
//   - NPC 列表行 x=590, y=50+i*21（右侧，18 行），点击选中 → 传送
//   - 滚动条 (W-21,48/417)、世界/我的位置/传送/搜索按钮（Title 821-829, Prguse2 1340-1342）
// 网络：服务端进图时推送 NewMapInfo（NPC 列表），TeleportToNPC 传送
// ============================================================================

use bevy::prelude::*;
use std::collections::HashMap;

use crate::game::dialogs::minimap::{CurrentMapIndex, MemberLocations};
use crate::game::dialogs::text_input::{
    TextInputDisplay, TextInputField, TextInputRect, TextInputState, TextInputSubmit,
};
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::game::movement::world_to_tile;
use crate::map_renderer::{GameData, GameLibraries};
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::outlined_text::spawn_outlined_label;
use crate::ui::sprite_ui::{shared_cjk_font, UiCjkFont, UiFont};
use crate::ui::theme::{
    load_lib_image, spawn_container, spawn_icon_button, spawn_image, spawn_label, spawn_panel,
    CloseButton,
};

/// #2892 批B：面板精灵（C# `BigMapDialog.Index = 820; Library = Libraries.Title; Location = Center`）
pub const PANEL: (LibraryName, usize) = (LibraryName::Title, 820);

/// 面板尺寸（Title[820] **图头** 760x500）——`Library.Draw` 原样 1:1 铺，节点尺寸按它给
pub const PANEL_W: f32 = 760.0;
pub const PANEL_H: f32 = 500.0;
/// 面板**真尺寸**（C# `MirImageControl.Size` = `Library.GetTrueSize(820)`）。
///
/// §3.2cl ②：`AutoSize`（构造默认 true）下 `Size` 取 `GetTrueSize` —— 裁掉 alpha=0 的边
/// （`Client/MirGraphics/MLibrary.cs:1050-1127`）。`Title[820]` 最右 1 列全透明 ⇒ 真宽 **759**，
/// 高不裁 ⇒ **500**。**布局/命中/裁剪一律用真尺寸**；贴图仍按图头 760 铺。
///
/// 2026-10-01 原版帧实测（`%TEMP%\golden_sandbox\shots\orig_bm0_open.png`，模板匹配 0.0000）：
/// 上滚钮 `Prguse2[197]` 在 **(870,182)** = 面板 132 + (`Size.Width`-21) + 48 ⇒ `Size.Width`=759 坐实。
pub const PANEL_TRUE_W: f32 = 759.0;
pub const PANEL_TRUE_H: f32 = 500.0;
/// 面板屏内原点（C# `MirControl.Center` = `((Settings.ScreenWidth - Size.Width)/2, …)` 整数除法）：
/// `((1024-759)/2, (768-500)/2)` = **(132,134)**。
/// 注意 `Title[820]` 真宽是奇数 759 ⇒ 若误用浮点除会得 132.5（§3.2cl 的 Help 同款坑）。
pub const PANEL_ORIGIN: (f32, f32) = (132.0, 134.0);
/// 玩家雷达点 `Prguse2[1350]`：图头 12x10、真尺寸 **10x10**。C# `BigMapDialog.cs:709-710`
/// `Location = ((int)x - s.Width/2, (int)y - s.Height/2)`，`s = UserRadarDot.Size` = `GetTrueSize(1350)`
/// ⇒ 居中偏移是 **-5,-5**（不是图头 12 的 -6）。
const DOT_TRUE_W: f32 = 10.0;
const DOT_TRUE_H: f32 = 10.0;
/// 搜索输入框（C# BigMapDialog.cs:204,207 SearchTextBox Location(59, Size.Height-27) Size(130,10)；
/// C# 无独立"搜索:"label，仅 SearchButton 带 Hint）。SEARCH_Y 为相对面板底部的偏移（H-27）。
pub const SEARCH_X: f32 = 59.0;
pub const SEARCH_Y_FROM_BOTTOM: f32 = 27.0;
pub const SEARCH_W: f32 = 130.0;
pub const SEARCH_H: f32 = 10.0;
/// 视口区域（C# BigMapViewPort 568x380 @ (14,52)）
const VIEW_X: f32 = 14.0;
const VIEW_Y: f32 = 52.0;
const VIEW_W: f32 = 568.0;
const VIEW_H: f32 = 380.0;

/// 视口画幅布局（C# `BigMapViewPort.OnBeforeDraw`，`BigMapDialog.cs:649-656`）：
/// `Size = Libraries.MiniMap.GetSize(BigMap)` ⇒ 画幅取 `min(568, W) x min(380, H)`，
/// 左上角 = `(14 + (568 - w)/2, 52 + (380 - h)/2)`（面板内相对坐标）。
///
/// 2026-09-28 金标准 A/B（`tools/acceptance/csharp_golden/README.md` §3.2g）实测证明
/// 原版大地图视口**就是 `Data/mmap.Lib` 里的 `MapInfo.BigMap` 那张图**（沙箱那份
/// `BigMap=101` 1052x700 → 缩放进 568x380，逐像素比对一致率 98.3%），不是另画地形。
///
/// 纯函数：门禁直接钉它（`bigmap_view_layout_matches_csharp`）。
#[must_use]
pub fn bigmap_view_layout(art: (f32, f32)) -> (f32, f32, f32, f32) {
    let w = VIEW_W.min(art.0);
    let h = VIEW_H.min(art.1);
    (
        VIEW_X + (VIEW_W - w) / 2.0,
        VIEW_Y + (VIEW_H - h) / 2.0,
        w,
        h,
    )
}
/// NPC 点池大小（超过部分不绘制）
const DOT_POOL: usize = 64;
/// 队友点池大小（C# Globals.MaxGroup）
const MEMBER_DOT_POOL: usize = 8;
/// 世界地图图标池大小
const WORLD_ICON_POOL: usize = 32;
/// NPC 行数（C# MaximumRows=18）
const MAX_ROWS: usize = 18;

/// 大地图 NPC 行
#[derive(Debug, Clone, Default)]
pub struct NpcRow {
    pub object_id: u32,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub icon: i32,
    pub can_teleport_to: bool,
}

/// 大地图状态（NewMapInfo 由网络层填充）
#[derive(Resource, Default)]
pub struct BigMapState {
    pub map_index: i32,
    pub title: String,
    pub npcs: Vec<NpcRow>,
    pub selected: Option<usize>,
    pub top_line: usize,
    /// 地形纹理是否已生成
    pub viewport_ready: bool,
    /// 视口纹理对应的（地图名, 大地图索引）——换图 / 换 `BigMap` 索引时重建。
    /// 旧实现只在**首次开窗**建一次，换图后大地图会一直显示上一张图。
    pub viewport_key: (String, u16),
    /// 地形纹理像素尺寸（生成后记录，供坐标换算）
    pub tex_size: (f32, f32),
    pub map_size: (f32, f32),
    /// #300 世界地图（C# S.WorldMapSetupInfo）
    pub world_enabled: bool,
    pub world_icons: Vec<mir2_shared::packets::server::map::WorldMapIcon>,
    pub teleport_cost: i32,
    /// 世界地图覆盖层是否打开（C# WorldMapImage.Visible）
    pub world_open: bool,
}

#[derive(Component)]
pub struct BigMapWidget;

#[derive(Component)]
pub struct BigMapWorld;

#[derive(Component)]
pub struct BigMapPosBar;

#[derive(Component)]
pub struct BigMapTerrain;

#[derive(Component)]
pub struct BigMapPlayerDot;

/// NPC 点池（index 对应 state.npcs 下标）
#[derive(Component)]
pub struct BigMapDot(pub usize);

/// 队友点（index 对应 MemberLocations.members 下标，C# BigMapDialog Players）
#[derive(Component)]
pub struct BigMapMemberDot(pub usize);

#[derive(Component)]
pub struct BigMapRow(pub usize);

#[derive(Component)]
pub struct BigMapTitleText;

/// 大地图标题：**未选目标地图时回落到当前地图名**。
///
/// C# `BigMapDialog.CurrentRecord` 的初值是**当前地图**的记录——进图时
/// `GameScene.cs:2219` 会 `BigMapDialog.SetTargetMap(info.MapIndex)`，而 `SetTargetMap`
/// 里 `CurrentRecord = GameScene.MapInfoList[MapIndex]`（`BigMapDialog.cs:304-320`），
/// `CurrentRecord` 的 setter 再把它写进 `TitleLabel.Text`（`BigMapDialog.cs:79-88`）；
/// 只有从世界地图点过地图图标后才换成目标地图（`BigMapDialog.cs:527`）。
///
/// 本端 `state.title` 只在点图标时写入（`update_big_map` 里 `state.title = icon.title`），
/// 所以常规打开时标题条是空的——A/B 帧实测（`ours_win_Bigmap.png` vs `orig_win_Bigmap.png`）：
/// 原版标题条有 9px 高的字，本端整条空白。这里补上 C# 的默认语义。
pub fn big_map_title(selected: &str, current_map: &str) -> String {
    if selected.is_empty() {
        current_map.to_string()
    } else {
        selected.to_string()
    }
}

#[derive(Component)]
pub struct BigMapCoordText;

#[derive(Component)]
pub struct BigMapWorldRoot;

#[derive(Component)]
pub struct BigMapWorldTitle;

/// 世界地图图标（index 对应 state.world_icons 下标）
#[derive(Component)]
pub struct BigMapWorldIcon(pub usize);

/// 大地图主按钮（单查询分发，避免多 With<marker> 查询超 SystemParam 上限）
#[derive(Component, Clone, Copy)]
pub enum BigMapBtnKind {
    Close,
    ScrollUp,
    ScrollDown,
    MyLocation,
    Teleport,
    Search,
}
#[derive(Component)]
pub struct BigMapBtn(pub BigMapBtnKind);

pub struct BigMapPlugin;

impl Plugin for BigMapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BigMapState>();
        app.add_systems(
            Update,
            big_map_server_events.run_if(in_state(AppState::Game)),
        );
        app.add_systems(OnEnter(AppState::Game), spawn_big_map);
        app.add_systems(OnExit(AppState::Game), cleanup_big_map);
        app.add_systems(
            Update,
            (
                big_map_ui_system,
                big_map_world_system,
                big_map_viewport_system,
                big_map_member_system,
                big_map_hint_system,
                // 描边副本同步须排在 Text 写方之后（批48 P1：C# MirLabel 默认描边）
                crate::ui::outlined_text::sync_outline_ui_system,
            )
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_big_map(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_big_map(
    mut commands: Commands,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut fonts: ResMut<Assets<Font>>,
    mut cjk_font: ResMut<UiCjkFont>,
    mut ui_font: ResMut<UiFont>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        crate::ui::sprite_ui::ensure_ui_font(&mut fonts, &mut ui_font);
    }
    // 标题/行文本可能含中文（#2599：动态文本 CJK 需主字体自带，不能依赖回退）
    let font = ui_font.0.clone();
    let cjk = crate::ui::sprite_ui::shared_cjk_font(&mut fonts, &mut cjk_font);

    let (pw, ph) = match libs.0.get_image(LibraryName::Title, 820) {
        Some(i) => (i.width.max(0) as f32, i.height.max(0) as f32),
        None => (PANEL_W, PANEL_H),
    };
    // C# 子控件一律按面板的 `Size` 定位 = `GetTrueSize(820)` = (759,500)，**不是图头** (760,500)
    let (lw, lh) = match libs.0.get_image(LibraryName::Title, 820) {
        Some(i) => {
            let (tw, th) = i.get_true_size();
            (tw.max(0) as f32, th.max(0) as f32)
        }
        None => (PANEL_TRUE_W, PANEL_TRUE_H),
    };
    let (px, py) = PANEL_ORIGIN;

    // 面板 Title[820]（760x500）@ 屏心
    let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Title, 820) else {
        return;
    };
    let panel = spawn_panel(&mut commands, bg, px, py, pw, ph, 30);
    commands
        .entity(panel)
        .insert((DialogRoot(DialogKind::BigMap), BigMapWidget));

    commands.entity(panel).with_children(|p| {
        // 标题（C# TitleLabel (19,6) 699x20）
        spawn_outlined_label(p, cjk.clone(), "", 19.0, 6.0, 14.0, Color::WHITE, 4)
            .insert(BigMapTitleText);
        // 关闭 (W-25,3)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 360),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 361),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 362),
        ) {
            spawn_icon_button(p, n, h, pr, lw - 25.0, 3.0, 24.0, 21.0, 8)
                .insert((BigMapBtn(BigMapBtnKind::Close), CloseButton));
        }
        // 大图（`Data/mmap.Lib[MapInfo.BigMap]`，首帧生成后填充）。
        // **不铺自造底色**：C# `BigMapViewPort` 没有背景（`MirControl.BackColour = Color.Empty`），
        // 画幅之外露的就是 `Title[820]` 面板美术本身（图比视口小时四周会露出面板）。
        let white = images.add(crate::map_renderer::make_image(
            vec![255, 255, 255, 255],
            1,
            1,
        ));
        spawn_container(p, VIEW_X, VIEW_Y, VIEW_W, VIEW_H, 1)
            .insert((ImageNode::new(white), BigMapTerrain));
        // 玩家雷达点 Prguse2[1350]（12x10）
        if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 1350) {
            spawn_image(p, h, VIEW_X, VIEW_Y, 12.0, 10.0, 3)
                .insert((BigMapPlayerDot, BigMapWidget));
        }
        // NPC 点池（绿色小方块 3x3）
        for i in 0..DOT_POOL {
            spawn_container(p, VIEW_X, VIEW_Y, 3.0, 3.0, 2).insert((
                BackgroundColor(Color::srgb(0.0, 1.0, 0.2)),
                BigMapDot(i),
                BigMapWidget,
            ));
        }
        // 队友点池（黄色小方块 3x3）
        for i in 0..MEMBER_DOT_POOL {
            spawn_container(p, VIEW_X, VIEW_Y, 3.0, 3.0, 2).insert((
                BackgroundColor(Color::srgb(1.0, 0.9, 0.2)),
                BigMapMemberDot(i),
            ));
        }
        // 上滚/下滚 (W-21,48)/(W-21,417)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 197),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 198),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 199),
        ) {
            // 尺寸取**美术原生**（`Prguse2[197]` 图头 12x12）：C# `ScrollUpButton` 不设 `Size`，
            // 而 `MirImageControl` 构造器把 `AutoSize` 置 true ⇒ 尺寸一律由帧决定
            spawn_icon_button(p, n, h, pr, lw - 21.0, 48.0, 12.0, 12.0, 8)
                .insert(BigMapBtn(BigMapBtnKind::ScrollUp));
        }
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 207),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 208),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 209),
        ) {
            spawn_icon_button(p, n, h, pr, lw - 21.0, 417.0, 12.0, 12.0, 8)
                .insert(BigMapBtn(BigMapBtnKind::ScrollDown));
        }
        // 位置条 Prguse2[205] (W-21, 61) 12x18（y 随滚动动态调整）
        if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 205) {
            spawn_image(p, h, lw - 21.0, 61.0, 12.0, 18.0, 7).insert(BigMapPosBar);
        }
        // 世界地图按钮 Title[827/828/829] (250, H-33)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 827),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 828),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 829),
        ) {
            spawn_icon_button(p, n, h, pr, 250.0, lh - 33.0, 80.0, 25.0, 8).insert(BigMapWorld);
        }
        // 我的位置 Title[824/825/826] (400, H-33)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 824),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 825),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 826),
        ) {
            spawn_icon_button(p, n, h, pr, 400.0, lh - 33.0, 80.0, 25.0, 8)
                .insert(BigMapBtn(BigMapBtnKind::MyLocation));
        }
        // 传送按钮 Title[821/822/823] (W-122, 432)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 821),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 822),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 823),
        ) {
            spawn_icon_button(p, n, h, pr, lw - 122.0, 432.0, 72.0, 25.0, 8)
                .insert(BigMapBtn(BigMapBtnKind::Teleport));
        }
        // 搜索按钮 Prguse2[1340/1341/1342] (23, H-36)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 1340),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 1341),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 1342),
        ) {
            spawn_icon_button(p, n, h, pr, 23.0, lh - 36.0, 32.0, 30.0, 8)
                .insert(BigMapBtn(BigMapBtnKind::Search));
        }
        // 搜索输入框（C# SearchTextBox (59, H-27) 130x10；TextInputField id=10）
        spawn_container(
            p,
            SEARCH_X,
            lh - SEARCH_Y_FROM_BOTTOM,
            SEARCH_W,
            SEARCH_H,
            8,
        )
        .insert((
            BackgroundColor(Color::srgba(0.2, 0.2, 0.25, 0.9)),
            TextInputField(10),
            TextInputRect(
                px + SEARCH_X,
                py + lh - SEARCH_Y_FROM_BOTTOM,
                SEARCH_W,
                SEARCH_H,
            ),
        ))
        .with_children(|ic| {
            ic.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(2.0),
                    top: Val::Px(0.0),
                    ..default()
                },
                Text::new(String::new()),
                TextFont {
                    font: FontSource::Handle(cjk.clone()),
                    font_size: FontSize::Px(10.0),
                    ..default()
                },
                TextColor(Color::WHITE),
                ZIndex(9),
                TextInputDisplay(10),
            ));
        });
        // 世界地图覆盖层（C# WorldMapImage：Prguse2[1360] 底 + 1365 云 + 1366 边框 @(10,0)）
        for (idx, z) in [(1360usize, 6), (1365, 7), (1366, 8)] {
            if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, idx) {
                spawn_image(p, h, 10.0, 0.0, 740.0, 500.0, z)
                    .insert((BigMapWorldRoot, Visibility::Hidden));
            }
        }
        // 悬停标题（C# WorldMapImage.TitleLabel：黑底白字，顶部居中）
        spawn_outlined_label(p, cjk.clone(), "", 10.0, 8.0, 12.0, Color::WHITE, 9).insert((
            BigMapWorldTitle,
            BigMapWorldRoot,
            Visibility::Hidden,
        ));
        // 世界地图图标池（MapLinkIcon 帧带 offset，C# UseOffSet=true）
        let wm_white = images.add(crate::map_renderer::make_image(
            vec![255, 255, 255, 255],
            1,
            1,
        ));
        for k in 0..WORLD_ICON_POOL {
            spawn_container(p, 10.0, 0.0, 16.0, 16.0, 9).insert((
                Button,
                ImageNode::new(wm_white.clone()),
                BigMapWorldIcon(k),
                BigMapWorldRoot,
                Visibility::Hidden,
            ));
        }
        // NPC 列表行（x=590, y=50+i*21，右侧）
        for i in 0..MAX_ROWS {
            spawn_outlined_label(
                p,
                cjk.clone(),
                "",
                590.0,
                50.0 + i as f32 * 21.0,
                12.0,
                Color::WHITE,
                4,
            )
            .insert(BigMapRow(i));
        }
        // 坐标标签 (519,435)
        spawn_outlined_label(p, cjk.clone(), "", 519.0, 435.0, 12.0, Color::WHITE, 4)
            .insert(BigMapCoordText);
    });
}

#[allow(clippy::too_many_arguments)]
fn big_map_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut state: ResMut<BigMapState>,
    net: ResMut<NetConnection>,
    btns: Query<(Entity, &Interaction, &BigMapBtn)>,
    mut input: ResMut<TextInputState>,
    mut submits: MessageReader<TextInputSubmit>,
    mut widgets: Query<
        (
            &mut Visibility,
            Option<&BigMapDot>,
            Option<&BigMapPlayerDot>,
        ),
        (
            With<BigMapWidget>,
            Without<BigMapWorldRoot>,
            Without<BigMapWorld>,
        ),
    >,
    mut rows: Query<(&mut Text, &BigMapRow)>,
    mut pos_bar: Query<&mut Node, With<BigMapPosBar>>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    mut prev_inter: Local<HashMap<Entity, Interaction>>,
    // B0001：只读 panel_origin(R Node) × 本系统 Node 写方需互斥（面板根不带其标记，不错杀）
    panel_origin: Query<&Node, (With<BigMapWidget>, With<DialogRoot>, Without<BigMapPosBar>)>,
) {
    fn edge(e: Entity, inter: &Interaction, prev: &mut HashMap<Entity, Interaction>) -> bool {
        let was = prev.insert(e, *inter);
        *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
    }
    let open = mgr.is_open(DialogKind::BigMap);
    let npc_count = state.npcs.len();
    if open && input.texts.len() < 11 {
        input.texts.resize(11, String::new());
    }
    for (mut vis, dot, pdot) in &mut widgets {
        let show = if pdot.is_some() {
            open
        } else if let Some(d) = dot {
            open && !state.world_open && d.0 < npc_count
        } else {
            open
        };
        *vis = if show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !open {
        return;
    }

    let max_scroll = state.npcs.len().saturating_sub(MAX_ROWS);
    let mut do_search = false;
    for (e, inter, b) in &btns {
        if !edge(e, inter, &mut prev_inter) {
            continue;
        }
        match b.0 {
            BigMapBtnKind::Close => {
                mgr.close(DialogKind::BigMap);
            }
            BigMapBtnKind::ScrollUp => {
                if state.top_line > 0 {
                    state.top_line -= 1;
                }
            }
            BigMapBtnKind::ScrollDown => {
                if state.top_line < max_scroll {
                    state.top_line += 1;
                }
            }
            BigMapBtnKind::MyLocation => {
                state.world_open = false;
                state.selected = None;
                state.top_line = 0;
                net.send_packet(&mir2_shared::packets::client::npc::RequestMapInfo {
                    map_index: state.map_index,
                });
                tracing::info!("🗺️ 回到我的位置 map={}", state.map_index);
            }
            BigMapBtnKind::Search => {
                do_search = true;
            }
            BigMapBtnKind::Teleport => {
                if let Some(idx) = state.selected {
                    if let Some(npc) = state.npcs.get(idx) {
                        if npc.can_teleport_to {
                            net.send_packet(&mir2_shared::packets::client::npc::TeleportToNPC {
                                object_id: npc.object_id,
                            });
                            tracing::info!("🗺️ 传送到 NPC: {} (id={})", npc.name, npc.object_id);
                        }
                    }
                }
            }
        }
    }
    // 搜索：按钮点击或输入框回车 → C.SearchMap（服务端按地图/NPC 名搜索并以系统消息返回）
    for s in submits.read() {
        if s.0 == 10 {
            do_search = true;
        }
    }
    if do_search {
        let keyword = input.texts.get(10).cloned().unwrap_or_default();
        let keyword = keyword.trim().to_string();
        if keyword.is_empty() {
            tracing::warn!("🗺️ 搜索关键词为空");
        } else {
            net.send_packet(&crate::network::SearchMapWire {
                keyword: keyword.clone(),
            });
            tracing::info!("🗺️ 搜索: {}", keyword);
        }
        input.active = None;
    }

    // 点击 NPC 行选中（C# BigMapNPCRow.Click）
    if let Ok(window) = windows.single() {
        if let Some(cursor) = window.cursor_position() {
            if mouse.just_pressed(MouseButton::Left) {
                let (ox, oy) = panel_origin
                    .single()
                    .map(|n| crate::ui::theme::node_origin(n, PANEL_ORIGIN))
                    .unwrap_or(PANEL_ORIGIN);
                for i in 0..MAX_ROWS {
                    let ry = oy + 50.0 + i as f32 * 21.0;
                    if cursor.x >= ox + 590.0
                        && cursor.x <= ox + 590.0 + 150.0
                        && cursor.y >= ry
                        && cursor.y <= ry + 18.0
                    {
                        let idx = state.top_line + i;
                        if idx < state.npcs.len() {
                            state.selected = Some(idx);
                            tracing::info!("🗺️ 选中 NPC: {}", state.npcs[idx].name);
                        }
                        break;
                    }
                }
            }
        }
    }

    // 位置条
    for mut node in &mut pos_bar {
        let pct = if max_scroll > 0 {
            state.top_line as f32 / max_scroll as f32
        } else {
            0.0
        };
        node.top = Val::Px(61.0 + pct * 342.0);
    }

    // 行文字
    for (mut text, row) in &mut rows {
        let idx = state.top_line + row.0;
        if let Some(npc) = state.npcs.get(idx) {
            let sel = state.selected == Some(idx);
            text.0 = format!(
                "{}{} ({},{})",
                if sel { "▶ " } else { "" },
                npc.name,
                npc.x,
                npc.y
            );
        } else {
            text.0 = String::new();
        }
    }
}

/// 世界地图覆盖层（#300）：World 按钮显隐/切换 + 图标同步/悬停/点击
#[allow(clippy::too_many_arguments)]
fn big_map_world_system(
    mgr: Res<DialogManager>,
    mut state: ResMut<BigMapState>,
    net: ResMut<NetConnection>,
    mut world_btn: Query<
        (Entity, &Interaction, &mut Visibility),
        (With<BigMapWorld>, Without<BigMapWorldRoot>),
    >,
    mut world_bg: Query<
        &mut Visibility,
        (
            With<BigMapWorldRoot>,
            Without<BigMapWorldIcon>,
            Without<BigMapWorldTitle>,
            Without<BigMapWorld>,
        ),
    >,
    mut world_title: Query<
        (&mut Text, &mut Visibility),
        (With<BigMapWorldTitle>, Without<BigMapWorld>),
    >,
    mut world_icons: Query<
        (
            Entity,
            &mut Visibility,
            &mut Node,
            &mut ImageNode,
            &Interaction,
            &BigMapWorldIcon,
        ),
        (
            With<BigMapWorldRoot>,
            Without<BigMapWorldTitle>,
            Without<BigMapWorld>,
        ),
    >,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut prev_open: Local<bool>,
    mut prev_inter: Local<HashMap<Entity, Interaction>>,
    windows: Query<&Window>,
    // B0001：只读 panel_origin(R Node) × world_icons(W Node) 需互斥（面板根不带图标标记）
    panel_origin: Query<
        &Node,
        (
            With<BigMapWidget>,
            With<DialogRoot>,
            Without<BigMapWorldIcon>,
        ),
    >,
) {
    fn edge(e: Entity, inter: &Interaction, prev: &mut HashMap<Entity, Interaction>) -> bool {
        let was = prev.insert(e, *inter);
        *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
    }
    let open = mgr.is_open(DialogKind::BigMap);
    // C# BigMapDialog.Show() → TargetMyLocation()：重新打开时回到当前地图列表
    if open && !*prev_open {
        state.world_open = false;
    }
    *prev_open = open;

    // 世界按钮仅在 setup.Enabled 时可见（C# WorldMapSetup）
    for (e, inter, mut vis) in &mut world_btn {
        *vis = if open && state.world_enabled {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if edge(e, inter, &mut prev_inter) && state.world_enabled {
            state.world_open = !state.world_open;
            tracing::info!(
                "🗺️ 世界地图 {}",
                if state.world_open { "打开" } else { "关闭" }
            );
        }
    }

    // 覆盖层显隐
    let world_show = open && state.world_enabled && state.world_open;
    for mut vis in &mut world_bg {
        *vis = if world_show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    // 图标同步 + 悬停标题 + 点击（C# WorldMapImage.MakeButtons：MapLinkIcon 帧 offset，UseOffSet=true）
    let (ox, oy) = panel_origin
        .single()
        .map(|n| crate::ui::theme::node_origin(n, PANEL_ORIGIN))
        .unwrap_or(PANEL_ORIGIN);
    let (wm_x, wm_y) = (ox + 10.0, oy);

    let mut hover_title = String::new();
    let mut clicked_icon: Option<usize> = None;
    if world_show {
        let cursor = windows.single().ok().and_then(|w| w.cursor_position());
        for (e, mut vis, mut node, mut image, inter, ic) in &mut world_icons {
            let k = ic.0;
            if k >= state.world_icons.len() {
                *vis = Visibility::Hidden;
                continue;
            }
            let icon = &state.world_icons[k];
            let idx = icon.image_index.max(0) as usize;
            let Some(info) = libs.0.get_image(LibraryName::MapLinkIcon, idx) else {
                *vis = Visibility::Hidden;
                continue;
            };
            let w = info.width.max(0) as f32;
            let h = info.height.max(0) as f32;
            let x = wm_x + info.offset_x as f32;
            let y = wm_y + info.offset_y as f32;
            let Some(hnd) = load_lib_image(&mut libs, &mut images, LibraryName::MapLinkIcon, idx)
            else {
                *vis = Visibility::Hidden;
                continue;
            };
            image.image = hnd;
            // 相对面板（wm_x-px=10, wm_y-py=0）
            node.left = Val::Px(10.0 + info.offset_x as f32);
            node.top = Val::Px(info.offset_y as f32);
            node.width = Val::Px(w);
            node.height = Val::Px(h);
            *vis = Visibility::Visible;
            if let Some(cursor) = cursor {
                if cursor.x >= x && cursor.x <= x + w && cursor.y >= y && cursor.y <= y + h {
                    hover_title = icon.title.clone();
                }
            }
            if edge(e, inter, &mut prev_inter) {
                clicked_icon = Some(k);
            }
        }
    } else {
        for (_, mut vis, _, _, _, _) in &mut world_icons {
            *vis = Visibility::Hidden;
        }
    }
    for (mut text, mut vis) in &mut world_title {
        text.0 = hover_title.clone();
        *vis = if world_show && !hover_title.is_empty() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    // 点击图标 → SetTargetMap（C# WorldMapImage button.Click）
    if let Some(k) = clicked_icon {
        if let Some(icon) = state.world_icons.get(k).cloned() {
            state.world_open = false;
            state.map_index = icon.map_index;
            state.title = icon.title.clone();
            state.npcs.clear();
            state.selected = None;
            state.top_line = 0;
            net.send_packet(&mir2_shared::packets::client::npc::RequestMapInfo {
                map_index: icon.map_index,
            });
            tracing::info!("🗺️ 世界地图切换到 {}: {}", icon.map_index, icon.title);
        }
    }
}

/// 大地图悬停提示的归属方（`TooltipState.source`，与其它写入方隔离）：#2767
pub const BIGMAP_TOOLTIP_SOURCE: u16 = 7;

/// #2767 大地图 Hint（C# `BigMapDialog.cs`）：
/// - `SearchButton.Hint = SearchForNPCs`（「搜索NPC」，:198，@(23, H-36) 32x30）；
/// - 队友光点 `Players[i].Hint = groupMemberLocation.Key`（队友名，:730；仅同图且非本人可见，与 `big_map_member_system` 同一可见性判定）。
/// 光标支持控制接口探针（无焦点环境可驱动验证）。
fn big_map_hint_system(
    mgr: Res<DialogManager>,
    state: Res<BigMapState>,
    locs: Res<MemberLocations>,
    current: Res<CurrentMapIndex>,
    windows: Query<&Window>,
    probe: Res<crate::control::CursorProbe>,
    ui_cameras: Query<(&Camera, &GlobalTransform), With<crate::ui::sprite_ui::UiEntity>>,
    dots: Query<(&Node, &Visibility, &BigMapMemberDot)>,
    mut tooltip: ResMut<crate::ui::tooltip::TooltipState>,
) {
    let clear = |tooltip: &mut crate::ui::tooltip::TooltipState| {
        tooltip.update(
            BIGMAP_TOOLTIP_SOURCE,
            false,
            String::new(),
            Vec::new(),
            0.0,
            0.0,
        );
    };
    if !mgr.is_open(DialogKind::BigMap) {
        clear(&mut tooltip);
        return;
    }
    let Some(raw) = crate::control::resolve_cursor(
        probe.pos,
        windows.single().ok().and_then(|w| w.cursor_position()),
    ) else {
        clear(&mut tooltip);
        return;
    };
    // UI 相机 Fixed{1024,768}：换算成 UI 逻辑坐标（与 tooltip_hint_system 一致）
    let cursor = match ui_cameras.single() {
        Ok((cam, gtf)) => match cam.viewport_to_world_2d(gtf, raw) {
            Ok(w) => Vec2::new(w.x, -w.y),
            Err(_) => {
                clear(&mut tooltip);
                return;
            }
        },
        Err(_) => {
            clear(&mut tooltip);
            return;
        }
    };
    let (px, py) = PANEL_ORIGIN;
    let local = (cursor.x - px, cursor.y - py);
    // 1) 队友光点：3x3 小方块，命中放宽到 ±4px（C# 控件 Size 同为小方块，人手可点）
    for (node, vis, dot) in &dots {
        if *vis != Visibility::Visible || dot.0 >= locs.members.len() {
            continue;
        }
        let (_, map_idx, _, _) = &locs.members[dot.0];
        if *map_idx as i32 != current.0 {
            continue;
        }
        let (x, y) = match (node.left, node.top) {
            (Val::Px(x), Val::Px(y)) => (x, y),
            _ => continue,
        };
        if big_map_dot_hit(local, x, y) {
            let name = locs.members[dot.0].0.clone();
            tooltip.update(
                BIGMAP_TOOLTIP_SOURCE,
                true,
                String::new(),
                vec![name],
                cursor.x,
                cursor.y,
            );
            return;
        }
    }
    // 2) 搜索按钮（C# SearchButton @(23, H-36) 32x30，Hint = 搜索NPC）
    if big_map_search_hit(local) {
        tooltip.update(
            BIGMAP_TOOLTIP_SOURCE,
            true,
            String::new(),
            vec!["搜索NPC".to_string()],
            cursor.x,
            cursor.y,
        );
        return;
    }
    clear(&mut tooltip);
}

/// 队友光点命中（点在面板局部坐标；光点 3x3，命中放宽到 ±4px）
fn big_map_dot_hit(local: (f32, f32), dot_x: f32, dot_y: f32) -> bool {
    (local.0 - dot_x).abs() <= 4.0 && (local.1 - dot_y).abs() <= 4.0
}

/// 搜索按钮命中（C# `SearchButton` @(23, H-36) 32x30）
fn big_map_search_hit(local: (f32, f32)) -> bool {
    let (x, y) = (23.0, PANEL_TRUE_H - 36.0);
    local.0 >= x && local.0 <= x + 32.0 && local.1 >= y && local.1 <= y + 30.0
}

/// 队友点定位（与玩家光点同公式：vx+(x/mw)*tw, vy+(y/mh)*th；x/y 为服务端瓦片坐标）
fn big_map_member_pos(
    x: i32,
    y: i32,
    mw: f32,
    mh: f32,
    tw: f32,
    th: f32,
    vx: f32,
    vy: f32,
) -> (f32, f32) {
    (vx + (x as f32 / mw) * tw, vy + (y as f32 / mh) * th)
}

/// 大地图队友光点（C# BigMapDialog Players[MaxGroup]；#1307）
fn big_map_member_system(
    mgr: Res<DialogManager>,
    state: Res<BigMapState>,
    locs: Res<MemberLocations>,
    current: Res<CurrentMapIndex>,
    mut dots: Query<(&mut Node, &mut Visibility, &BigMapMemberDot)>,
) {
    let open = mgr.is_open(DialogKind::BigMap);
    let (tw, th) = state.tex_size;
    let (mw, mh) = state.map_size;
    if tw <= 0.0 || mw <= 0.0 {
        for (_, mut vis, _) in &mut dots {
            *vis = Visibility::Hidden;
        }
        return;
    }
    let (px, py) = PANEL_ORIGIN;
    let vx = px + VIEW_X + (VIEW_W - tw) / 2.0;
    let vy = py + VIEW_Y + (VIEW_H - th) / 2.0;
    for (mut node, mut vis, dot) in &mut dots {
        if open && dot.0 < locs.members.len() {
            let (_, map_idx, mx, my) = &locs.members[dot.0];
            // #1309：只显示同图队友
            if *map_idx as i32 != current.0 {
                *vis = Visibility::Hidden;
                continue;
            }
            let (sx, sy) = big_map_member_pos(*mx, *my, mw, mh, tw, th, vx, vy);
            node.left = Val::Px(sx - px - 1.5);
            node.top = Val::Px(sy - py - 1.5);
            *vis = Visibility::Visible;
        } else {
            *vis = Visibility::Hidden;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn big_map_viewport_system(
    mgr: Res<DialogManager>,
    mut state: ResMut<BigMapState>,
    game_data: Res<GameData>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    // B0001 互斥：terrain/player_dot/npc_dots 三查询同写 Node——批48迁移遗漏
    // 互斥矩阵 → 调度器初始化即 panic（b0001_smoke 实证）。每个写方必须显式
    // `With<自身标记>`：仅 read fetch（&BigMapDot 等）不足以构成互斥对——实测
    // 两查询各自只挂对方 Without 而自身无显式 With 时判定仍死锁（tests 实验证）。
    mut terrain: Query<
        (&mut Node, &mut ImageNode, &mut Visibility),
        (
            With<BigMapTerrain>,
            Without<BigMapPlayerDot>,
            Without<BigMapDot>,
        ),
    >,
    mut player_dot: Query<
        (&mut Node, &mut Visibility),
        (With<BigMapPlayerDot>, Without<BigMapTerrain>),
    >,
    mut npc_dots: Query<
        (&mut Node, &mut BackgroundColor, &mut Visibility, &BigMapDot),
        (
            With<BigMapDot>,
            Without<BigMapPlayerDot>,
            Without<BigMapTerrain>,
        ),
    >,
    players: Query<&Transform, (With<crate::actor::LocalPlayer>, Without<BigMapWidget>)>,
    windows: Query<&Window>,
    mut texts: Query<(
        &mut Text,
        Option<&BigMapTitleText>,
        Option<&BigMapCoordText>,
    )>,
    // B0001：只读 panel_origin(R Node) × terrain/dots(W Node) 需互斥（面板根不带其标记）
    panel_origin: Query<
        &Node,
        (
            With<BigMapWidget>,
            With<DialogRoot>,
            Without<BigMapTerrain>,
            Without<BigMapPlayerDot>,
            Without<BigMapDot>,
        ),
    >,
) {
    let open = mgr.is_open(DialogKind::BigMap);
    if !open {
        return;
    }

    // 视口画幅生成：换图 / 换 `BigMap` 索引时重建
    let map_name_now = game_data.desired_map.clone().unwrap_or_default();
    let want_key = (map_name_now.clone(), game_data.big_map_index);
    if state.viewport_key != want_key {
        state.viewport_key = want_key;
        state.viewport_ready = false;
    }
    if !state.viewport_ready {
        let (mw, mh) = game_data
            .map
            .as_ref()
            .map(|m| (m.width.max(1) as f32, m.height.max(1) as f32))
            .unwrap_or((1.0, 1.0));
        // ① C# 路线：`Data/mmap.Lib` 里的 `MapInfo.BigMap` 大图（`BigMapDialog.cs:642-676`
        //    `Libraries.MiniMap.Draw(index, DisplayLocation, Size, …)`）——整张图缩放进
        //    `min(568,W) x min(380,H)` 的画幅，**不裁切、不平移**。
        let big_idx = game_data.big_map_index as usize;
        let mut built = false;
        if big_idx > 0 {
            if let Some(info) = libs.0.get_image(LibraryName::MiniMap, big_idx) {
                let (aw, ah) = (info.width.max(0) as f32, info.height.max(0) as f32);
                if aw > 0.0 && ah > 0.0 {
                    if let Some(rgba) = info.rgba.clone() {
                        let (lx, ly, w, h) = bigmap_view_layout((aw, ah));
                        let tex =
                            images.add(crate::map_renderer::make_image(rgba, aw as u32, ah as u32));
                        if let Ok((mut node, mut image, mut vis)) = terrain.single_mut() {
                            node.left = Val::Px(lx);
                            node.top = Val::Px(ly);
                            node.width = Val::Px(w);
                            node.height = Val::Px(h);
                            image.image = tex;
                            *vis = Visibility::Visible;
                        }
                        state.viewport_ready = true;
                        state.tex_size = (w, h);
                        state.map_size = (mw, mh);
                        built = true;
                        tracing::info!(
                            "🗺️ 大地图用 mmap.Lib[{}] {}x{} → 画幅 {}x{}",
                            big_idx,
                            aw,
                            ah,
                            w,
                            h
                        );
                    }
                }
            }
            if !built {
                tracing::warn!("🗺️ 大地图缺图：mmap.Lib[{big_idx}] 取不到（Data/mmap.Lib 缺失？）");
            }
        }
        if !built {
            // ② C# `OnBeforeDraw:644-645`：`index <= 0`（或取不到图）**什么都不画** ——
            //    不铺底色、不画地形、不画点。新开窗这条路已被 `Show()` 守卫挡住
            //    （`BigMap <= 0` 不开窗）；这条分支只在"窗开着时换到没大图的地图"时走到。
            state.tex_size = (0.0, 0.0);
            state.map_size = (mw, mh);
            state.viewport_ready = true;
            tracing::info!(
                "🗺️ 大地图：BigMap={} 无可画的大图 → 视口留空（C# OnBeforeDraw 直接 return）",
                game_data.big_map_index
            );
        }
    }

    let (tw, th) = state.tex_size;
    let (mw, mh) = state.map_size;
    // C# `OnBeforeDraw`：`index <= 0` 时整段提前 return —— 图没有，**对象点也不画**
    // （玩家雷达点/队友点/ NPC 点都不该残留在面板美术上）。
    if tw <= 0.0 || mw <= 0.0 {
        if let Ok((_, _, mut vis)) = terrain.single_mut() {
            *vis = Visibility::Hidden;
        }
        if let Ok((_, mut vis)) = player_dot.single_mut() {
            *vis = Visibility::Hidden;
        }
        for (_, _, mut vis, _) in &mut npc_dots {
            *vis = Visibility::Hidden;
        }
        return;
    }
    let (ox, oy) = panel_origin
        .single()
        .map(|n| crate::ui::theme::node_origin(n, PANEL_ORIGIN))
        .unwrap_or(PANEL_ORIGIN);
    let vx = ox + VIEW_X + (VIEW_W - tw) / 2.0;
    let vy = oy + VIEW_Y + (VIEW_H - th) / 2.0;

    // 玩家点
    if let Ok(player_tf) = players.single() {
        let (tx, ty) = world_to_tile(player_tf.translation.x, player_tf.translation.y);
        if let Ok((mut node, mut vis)) = player_dot.single_mut() {
            // C# `BigMapDialog.cs:709-710`：`Location = ((int)x - s.Width/2, (int)y - s.Height/2)`，
            // `s = UserRadarDot.Size` = `GetTrueSize(1350)` = (10,10) ⇒ 减 (5,5)。
            // §3.2cm：此前漏了这一步，雷达点整体偏右下 (5,5)。
            node.left = Val::Px(vx + (tx as f32 / mw) * tw - ox - DOT_TRUE_W / 2.0);
            node.top = Val::Px(vy + (ty as f32 / mh) * th - oy - DOT_TRUE_H / 2.0);
            *vis = Visibility::Visible;
        }
    }
    // NPC 点（选中黄、其余绿）
    for (mut node, mut color, mut vis, d) in &mut npc_dots {
        if let Some(npc) = state.npcs.get(d.0) {
            let sx = vx + (npc.x as f32 / mw) * tw;
            let sy = vy + (npc.y as f32 / mh) * th;
            node.left = Val::Px(sx - ox - 1.5);
            node.top = Val::Px(sy - oy - 1.5);
            let selected = state.selected == Some(d.0);
            color.0 = if selected {
                Color::srgb(1.0, 0.9, 0.1)
            } else {
                Color::srgb(0.0, 1.0, 0.2)
            };
            *vis = Visibility::Visible;
        } else {
            // C# `OnBeforeDraw` 只遍历**本图现存对象**（`MapControl.Objects`）——
            // 池子里多出来的点不该留在上一张地图的位置上。
            *vis = Visibility::Hidden;
        }
    }

    // 标题/坐标
    let current_map_title = game_data.map_title.clone();
    for (mut text, title, coord) in &mut texts {
        if title.is_some() {
            text.0 = big_map_title(&state.title, &current_map_title);
        } else if coord.is_some() {
            // #122 C# MakeCoordinateLabel：鼠标悬停视口显示鼠标坐标，否则显示玩家坐标
            let mut s = None;
            if let Ok(window) = windows.single() {
                if let Some(cursor) = window.cursor_position() {
                    if cursor.x >= vx
                        && cursor.x <= vx + tw
                        && cursor.y >= vy
                        && cursor.y <= vy + th
                    {
                        let tx = (((cursor.x - vx) / tw) * mw) as i32;
                        let ty = (((cursor.y - vy) / th) * mh) as i32;
                        s = Some(format!("[ {}, {} ]", tx, ty));
                    }
                }
            }
            if s.is_none() {
                if let Ok(player_tf) = players.single() {
                    let (tx, ty) = world_to_tile(player_tf.translation.x, player_tf.translation.y);
                    s = Some(format!("[ {}, {} ]", tx, ty));
                }
            }
            if let Some(s) = s {
                if text.0 != s {
                    text.0 = s;
                }
            }
        }
    }
}

/// 消费服务端大地图信息事件（网络层只广播 ServerEvent）
fn big_map_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut big_map: ResMut<BigMapState>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        if let ServerEvent::MapInfo {
            map_index,
            title,
            npcs,
        } = ev
        {
            big_map.map_index = *map_index;
            big_map.title = title.clone();
            big_map.npcs = npcs.clone();
            big_map.selected = None;
            big_map.top_line = 0;
        }
        // #300：世界地图配置（C# S.WorldMapSetupInfo，进图首次下发）
        if let ServerEvent::WorldMapSetup {
            enabled,
            icons,
            teleport_cost,
        } = ev
        {
            big_map.world_enabled = *enabled;
            big_map.world_icons = icons.clone();
            big_map.teleport_cost = *teleport_cost;
            tracing::info!(
                "🗺️ 世界地图配置: enabled={} icons={} cost={}",
                enabled,
                icons.len(),
                teleport_cost
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn member_pos_maps_tiles() {
        // x=50/200*400=100；y=100/400*800=200（与玩家光点同公式）
        let (x, y) = big_map_member_pos(50, 100, 200.0, 400.0, 400.0, 800.0, 10.0, 20.0);
        assert_eq!(x, 110.0);
        assert_eq!(y, 220.0);
    }

    #[test]
    fn member_pos_origin_and_edge() {
        assert_eq!(
            big_map_member_pos(0, 0, 200.0, 400.0, 400.0, 800.0, 0.0, 0.0),
            (0.0, 0.0)
        );
        assert_eq!(
            big_map_member_pos(200, 400, 200.0, 400.0, 400.0, 800.0, 0.0, 0.0),
            (400.0, 800.0)
        );
    }

    /// C# `CurrentRecord` 默认是当前地图 ⇒ 未选目标地图时标题回落当前地图名
    /// （`BigMapDialog.cs:79-88/304-320`）。回归点：此前 `state.title` 为空时标题条整条空白。
    #[test]
    fn title_falls_back_to_current_map() {
        assert_eq!(big_map_title("", "BichonProvince"), "BichonProvince");
        assert_eq!(big_map_title("", ""), "");
        assert_eq!(
            big_map_title("WoomaTemple", "BichonProvince"),
            "WoomaTemple"
        );
    }

    /// #2767：大地图两处 Hint 的命中——搜索按钮（C# @(23, H-36) 32x30）与队友点（3x3，放宽 ±4px）
    #[test]
    fn big_map_hint_hit_matches_csharp() {
        // （Hint 命中与视口画幅是两件事，视口见 `bigmap_view_layout_matches_csharp`）
        // 搜索按钮内部
        assert!(big_map_search_hit((30.0, PANEL_TRUE_H - 30.0)));
        // 按钮上/下/右侧（右侧即搜索输入框区域，C# 无 Hint）
        assert!(!big_map_search_hit((30.0, PANEL_TRUE_H - 40.0)));
        assert!(!big_map_search_hit((30.0, PANEL_TRUE_H - 4.0)));
        assert!(!big_map_search_hit((60.0, PANEL_TRUE_H - 30.0)));
        // 队友点：±4px 内命中，超过不命中
        assert!(big_map_dot_hit((100.0, 100.0), 102.0, 98.0));
        assert!(big_map_dot_hit((100.0, 100.0), 96.0, 104.0));
        assert!(!big_map_dot_hit((100.0, 100.0), 106.0, 98.0));
        assert!(!big_map_dot_hit((100.0, 100.0), 100.0, 92.0));
    }

    /// §3.2cm：`BigMapDialog` 的子控件全按面板的 `Size` = `GetTrueSize(Title[820])` 定位。
    ///
    /// 真尺寸 (759,500)：宽裁掉最右 1 列 alpha=0，高不裁。原版帧 `orig_bm0_open.png` 实测
    /// （win_locate 模板匹配不符率 0.0000）：
    /// - 上滚钮 `Prguse2[197]` @ **(870,182)** = `PANEL_ORIGIN.0 + (759-21)` , `PANEL_ORIGIN.1 + 48`
    /// - 下滚钮 `Prguse2[207]` @ **(870,551)** = 同上 x，y = 134 + 417
    /// - 我的位置钮 `Title[824]` @ **(532,601)** = 132 + 400, 134 + (500-33)
    ///
    /// 守两点：①真尺寸常量与图头不同（否则退回图头就静默偏 1px）；②真宽必须是**奇数**、
    /// 原点用整数除法（`(1024-759)/2 = 132` 而非浮点 132.5）。
    #[test]
    fn bigmap_child_anchors_use_get_true_size() {
        assert_eq!((PANEL_W, PANEL_H), (760.0, 500.0), "图头（贴图 1:1 用）");
        assert_eq!(
            (PANEL_TRUE_W, PANEL_TRUE_H),
            (759.0, 500.0),
            "C# GetTrueSize(820)"
        );
        assert_eq!(PANEL_ORIGIN, (132.0, 134.0), "Center 整数除法");
        assert_eq!(
            ((1024.0 - PANEL_TRUE_W) / 2.0).floor(),
            PANEL_ORIGIN.0,
            "真宽 759 是奇数：必须 floor，不能用 (1024-760)/2 蒙对"
        );
        // 图头模型（错）与真尺寸模型（对）在右锚控件上差 1px：
        assert_eq!(PANEL_ORIGIN.0 + (PANEL_TRUE_W - 21.0), 870.0, "上/下滚钮 x");
        assert_ne!(
            PANEL_ORIGIN.0 + (PANEL_W - 21.0),
            870.0,
            "图头模型 = 871（原版帧证否）"
        );
        assert_eq!(PANEL_ORIGIN.1 + 48.0, 182.0, "上滚钮 y");
        assert_eq!(
            PANEL_ORIGIN.1 + (PANEL_TRUE_H - 33.0),
            601.0,
            "我的位置钮 y"
        );
        assert_eq!(PANEL_ORIGIN.0 + 400.0, 532.0, "我的位置钮 x");
        // 雷达点居中偏移取真尺寸 10/2 = 5（图头 12 会得 6）
        assert_eq!(DOT_TRUE_W / 2.0, 5.0);
        assert_eq!(DOT_TRUE_H / 2.0, 5.0);
    }

    /// 2026-09-28 金标准 A/B（README §3.2g）：原版大地图视口 = `Data/mmap.Lib[MapInfo.BigMap]`
    /// 整图缩放进 `min(568,W) x min(380,H)` 的画幅（C# `BigMapDialog.cs:649-656`），
    /// 不是另画地形。两个实测样本：
    /// - 沙箱那份 `BigMap=101`（1052x700）→ 画幅 (14,52,568,380)，缩比 568/1052；
    /// - 本端 DB `BichonProvince` 的 `big_map=135`（528x350）→ 原尺寸居中 (34,67,528,350)。
    #[test]
    fn bigmap_view_layout_matches_csharp() {
        assert_eq!(
            bigmap_view_layout((1052.0, 700.0)),
            (14.0, 52.0, 568.0, 380.0)
        );
        assert_eq!(
            bigmap_view_layout((528.0, 350.0)),
            (34.0, 67.0, 528.0, 350.0)
        );
        // 正好等于视口：不补边、不裁切
        assert_eq!(
            bigmap_view_layout((568.0, 380.0)),
            (14.0, 52.0, 568.0, 380.0)
        );
        // 单边小于视口：仍按 C# 的 `(568 - w)/2` 居中（整数除法在 f32 下即真除）
        assert_eq!(
            bigmap_view_layout((500.0, 300.0)),
            (48.0, 92.0, 500.0, 300.0)
        );
    }
}

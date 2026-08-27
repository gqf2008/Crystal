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

use crate::game::dialogs::text_input::{
    TextInputDisplay, TextInputField, TextInputRect, TextInputState, TextInputSubmit,
};
use crate::game::dialogs::minimap::{CurrentMapIndex, MemberLocations};
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::game::movement::world_to_tile;
use crate::map_renderer::{GameData, GameLibraries};
use crate::network::NetConnection;
use crate::ui::outlined_text::spawn_outlined_label;
use crate::resources::libraries::LibraryName;
use crate::resources::map_reader::{resolve_map_path, MapReader};
use crate::scenes::AppState;
use crate::ui::sprite_ui::{UiCjkFont, UiFont};
use crate::ui::theme::{
    load_lib_image, spawn_container, spawn_icon_button, spawn_image, spawn_label, spawn_panel,
};

/// 面板尺寸（Title[820] 实测 760x500）
pub const PANEL_W: f32 = 760.0;
pub const PANEL_H: f32 = 500.0;
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
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    // 标题/行文本可能含中文（#2599：动态文本 CJK 需主字体自带，不能依赖回退）
    let font = ui_font.0.clone();
    let cjk = crate::ui::sprite_ui::shared_cjk_font(&mut fonts, &mut cjk_font);

    let (pw, ph) = match libs.0.get_image(LibraryName::Title, 820) {
        Some(i) => (i.width.max(0) as f32, i.height.max(0) as f32),
        None => (PANEL_W, PANEL_H),
    };
    let px = (1024.0 - pw) / 2.0;
    let py = (768.0 - ph) / 2.0;

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
            spawn_icon_button(p, n, h, pr, pw - 25.0, 3.0, 24.0, 21.0, 8)
                .insert(BigMapBtn(BigMapBtnKind::Close));
        }
        // 视口背景（深色底）
        spawn_container(p, VIEW_X, VIEW_Y, VIEW_W, VIEW_H, 0)
            .insert(BackgroundColor(Color::srgb(0.1, 0.13, 0.1)));
        // 地形纹理（首帧生成后填充）
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
            spawn_container(p, VIEW_X, VIEW_Y, 3.0, 3.0, 2)
                .insert((
                    BackgroundColor(Color::srgb(0.0, 1.0, 0.2)),
                    BigMapDot(i),
                    BigMapWidget,
                ));
        }
        // 队友点池（黄色小方块 3x3）
        for i in 0..MEMBER_DOT_POOL {
            spawn_container(p, VIEW_X, VIEW_Y, 3.0, 3.0, 2)
                .insert((
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
            spawn_icon_button(p, n, h, pr, pw - 21.0, 48.0, 16.0, 14.0, 8)
                .insert(BigMapBtn(BigMapBtnKind::ScrollUp));
        }
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 207),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 208),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 209),
        ) {
            spawn_icon_button(p, n, h, pr, pw - 21.0, 417.0, 16.0, 14.0, 8)
                .insert(BigMapBtn(BigMapBtnKind::ScrollDown));
        }
        // 位置条 Prguse2[205] (W-21, 61) 12x18（y 随滚动动态调整）
        if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 205) {
            spawn_image(p, h, pw - 21.0, 61.0, 12.0, 18.0, 7).insert(BigMapPosBar);
        }
        // 世界地图按钮 Title[827/828/829] (250, H-33)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 827),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 828),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 829),
        ) {
            spawn_icon_button(p, n, h, pr, 250.0, ph - 33.0, 80.0, 25.0, 8)
                .insert(BigMapWorld);
        }
        // 我的位置 Title[824/825/826] (400, H-33)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 824),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 825),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 826),
        ) {
            spawn_icon_button(p, n, h, pr, 400.0, ph - 33.0, 80.0, 25.0, 8)
                .insert(BigMapBtn(BigMapBtnKind::MyLocation));
        }
        // 传送按钮 Title[821/822/823] (W-122, 432)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 821),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 822),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 823),
        ) {
            spawn_icon_button(p, n, h, pr, pw - 122.0, 432.0, 72.0, 25.0, 8)
                .insert(BigMapBtn(BigMapBtnKind::Teleport));
        }
        // 搜索按钮 Prguse2[1340/1341/1342] (23, H-36)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 1340),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 1341),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 1342),
        ) {
            spawn_icon_button(p, n, h, pr, 23.0, ph - 36.0, 32.0, 30.0, 8)
                .insert(BigMapBtn(BigMapBtnKind::Search));
        }
        // 搜索输入框（C# SearchTextBox (59, H-27) 130x10；TextInputField id=10）
        spawn_container(p, SEARCH_X, ph - SEARCH_Y_FROM_BOTTOM, SEARCH_W, SEARCH_H, 8)
            .insert((
                BackgroundColor(Color::srgba(0.2, 0.2, 0.25, 0.9)),
                TextInputField(10),
                TextInputRect(
                    px + SEARCH_X,
                    py + ph - SEARCH_Y_FROM_BOTTOM,
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
                        font: FontSource::Handle(font.clone()),
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
        spawn_outlined_label(p, cjk.clone(), "", 10.0, 8.0, 12.0, Color::WHITE, 9)
            .insert((BigMapWorldTitle, BigMapWorldRoot, Visibility::Hidden));
        // 世界地图图标池（MapLinkIcon 帧带 offset，C# UseOffSet=true）
        let wm_white = images.add(crate::map_renderer::make_image(
            vec![255, 255, 255, 255],
            1,
            1,
        ));
        for k in 0..WORLD_ICON_POOL {
            spawn_container(p, 10.0, 0.0, 16.0, 16.0, 9)
                .insert((
                    Button,
                    ImageNode::new(wm_white.clone()),
                    BigMapWorldIcon(k),
                    BigMapWorldRoot,
                    Visibility::Hidden,
                ));
        }
        // NPC 列表行（x=590, y=50+i*21，右侧）
        for i in 0..MAX_ROWS {
            spawn_outlined_label(p, cjk.clone(), "", 590.0, 50.0 + i as f32 * 21.0, 12.0, Color::WHITE, 4)
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
        (&mut Visibility, Option<&BigMapDot>, Option<&BigMapPlayerDot>),
        (With<BigMapWidget>, Without<BigMapWorldRoot>, Without<BigMapWorld>),
    >,
    mut rows: Query<(&mut Text, &BigMapRow)>,
    mut pos_bar: Query<&mut Node, With<BigMapPosBar>>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    mut prev_inter: Local<HashMap<Entity, Interaction>>,
    // B0001：只读 panel_origin(R Node) × 本系统 Node 写方需互斥（面板根不带其标记，不错杀）
    panel_origin: Query<
        &Node,
        (
            With<BigMapWidget>,
            With<DialogRoot>,
            Without<BigMapPosBar>,
        ),
    >,
) {
    fn edge(
        e: Entity,
        inter: &Interaction,
        prev: &mut HashMap<Entity, Interaction>,
    ) -> bool {
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
                    .map(|n| {
                        crate::ui::theme::node_origin(
                            n,
                            ((1024.0 - PANEL_W) / 2.0, (768.0 - PANEL_H) / 2.0),
                        )
                    })
                    .unwrap_or(((1024.0 - PANEL_W) / 2.0, (768.0 - PANEL_H) / 2.0));
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
    mut world_title: Query<(&mut Text, &mut Visibility), (With<BigMapWorldTitle>, Without<BigMapWorld>)>,
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
    fn edge(
        e: Entity,
        inter: &Interaction,
        prev: &mut HashMap<Entity, Interaction>,
    ) -> bool {
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
    let (pw, ph) = match libs.0.get_image(LibraryName::Title, 820) {
        Some(i) => (i.width.max(0) as f32, i.height.max(0) as f32),
        None => (PANEL_W, PANEL_H),
    };
    let (ox, oy) = panel_origin
        .single()
        .map(|n| {
            crate::ui::theme::node_origin(
                n,
                ((1024.0 - pw) / 2.0, (768.0 - ph) / 2.0),
            )
        })
        .unwrap_or(((1024.0 - pw) / 2.0, (768.0 - ph) / 2.0));
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
            let Some(hnd) =
                load_lib_image(&mut libs, &mut images, LibraryName::MapLinkIcon, idx)
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

/// 队友点定位（与玩家光点同公式：vx+(x/mw)*tw, vy+(y/mh)*th；x/y 为服务端瓦片坐标）
fn big_map_member_pos(x: i32, y: i32, mw: f32, mh: f32, tw: f32, th: f32, vx: f32, vy: f32) -> (f32, f32) {
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
    let px = (1024.0 - PANEL_W) / 2.0;
    let py = (768.0 - PANEL_H) / 2.0;
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
        (&mut Node, &mut ImageNode),
        (
            With<BigMapTerrain>,
            Without<BigMapPlayerDot>,
            Without<BigMapDot>,
        ),
    >,
    mut player_dot: Query<
        &mut Node,
        (
            With<BigMapPlayerDot>,
            Without<BigMapTerrain>,
        ),
    >,
    mut npc_dots: Query<
        (&mut Node, &mut BackgroundColor, &BigMapDot),
        (
            With<BigMapDot>,
            Without<BigMapPlayerDot>,
            Without<BigMapTerrain>,
        ),
    >,
    players: Query<&Transform, (With<crate::actor::LocalPlayer>, Without<BigMapWidget>)>,
    windows: Query<&Window>,
    mut texts: Query<(&mut Text, Option<&BigMapTitleText>, Option<&BigMapCoordText>)>,
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

    // 地形生成（首次打开时）
    if !state.viewport_ready {
        if let Some(map) = &game_data.map {
            let map_name = game_data.desired_map.clone().unwrap_or_default();
            let map_path = resolve_map_path(&map_name);
            if let Ok(reader) = MapReader::new(&map_path) {
                let (tex, tw, th, mw, mh) =
                    build_terrain_texture(&mut libs, &mut images, &reader, &map);
                if let Ok((mut node, mut image)) = terrain.single_mut() {
                    node.left = Val::Px(VIEW_X + (VIEW_W - tw) / 2.0);
                    node.top = Val::Px(VIEW_Y + (VIEW_H - th) / 2.0);
                    node.width = Val::Px(tw);
                    node.height = Val::Px(th);
                    image.image = tex.clone();
                }
                state.viewport_ready = true;
                state.tex_size = (tw, th);
                state.map_size = (mw, mh);
                tracing::info!("🗺️ 大地图地形生成: {}x{} 纹理 {}x{}", mw, mh, tw, th);
            }
        }
    }

    let (tw, th) = state.tex_size;
    let (mw, mh) = state.map_size;
    if tw <= 0.0 || mw <= 0.0 {
        return;
    }
    let (ox, oy) = panel_origin
        .single()
        .map(|n| {
            crate::ui::theme::node_origin(
                n,
                ((1024.0 - PANEL_W) / 2.0, (768.0 - PANEL_H) / 2.0),
            )
        })
        .unwrap_or(((1024.0 - PANEL_W) / 2.0, (768.0 - PANEL_H) / 2.0));
    let vx = ox + VIEW_X + (VIEW_W - tw) / 2.0;
    let vy = oy + VIEW_Y + (VIEW_H - th) / 2.0;

    // 玩家点
    if let Ok(player_tf) = players.single() {
        let (tx, ty) = world_to_tile(player_tf.translation.x, player_tf.translation.y);
        if let Ok(mut node) = player_dot.single_mut() {
            node.left = Val::Px(vx + (tx as f32 / mw) * tw - ox);
            node.top = Val::Px(vy + (ty as f32 / mh) * th - oy);
        }
    }
    // NPC 点（选中黄、其余绿）
    for (mut node, mut color, d) in &mut npc_dots {
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
        }
    }

    // 标题/坐标
    for (mut text, title, coord) in &mut texts {
        if title.is_some() {
            text.0 = state.title.clone();
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
                    let (tx, ty) =
                        world_to_tile(player_tf.translation.x, player_tf.translation.y);
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


/// 由地图瓦片采样生成大地图地形纹理（每个采样点取该格背景瓦片平均色）
fn build_terrain_texture(
    libs: &mut GameLibraries,
    images: &mut Assets<Image>,
    reader: &MapReader,
    map: &crate::map_renderer::LoadedMap,
) -> (Handle<Image>, f32, f32, f32, f32) {
    let mw = map.width.max(1) as f32;
    let mh = map.height.max(1) as f32;
    // 显示尺寸适配视口
    let scale = (VIEW_W / mw).min(VIEW_H / mh);
    let dw = (mw * scale).max(1.0);
    let dh = (mh * scale).max(1.0);
    // 采样步长：纹理像素上限约 400x400
    let step = ((mw / 400.0).ceil() as usize).max(1);
    let tw = (map.width as usize).div_ceil(step);
    let th = (map.height as usize).div_ceil(step);

    let mut cache: std::collections::HashMap<(i16, i32), [u8; 4]> = std::collections::HashMap::new();
    let mut rgba = Vec::with_capacity(tw * th * 4);
    for ty in 0..th {
        for tx in 0..tw {
            let cx = tx * step;
            let cy = ty * step;
            let cell = reader
                .map_cells
                .get(cy)
                .and_then(|row| row.get(cx));
            let mut color = match cell {
                Some(c) => tile_avg_color(libs, &mut cache, c).unwrap_or([64, 110, 56, 255]),
                None => [64, 110, 56, 255],
            };
            // 不可行走（障碍）压暗
            if !map.is_walkable((cx as i32).min(map.width - 1), (cy as i32).min(map.height - 1)) {
                for ch in color.iter_mut().take(3) {
                    *ch = (*ch as u16 * 6 / 10) as u8;
                }
            }
            rgba.extend_from_slice(&color);
        }
    }
    let img = images.add(crate::map_renderer::make_image(rgba, tw as u32, th as u32));
    (img, dw, dh, mw, mh)
}

/// 瓦片平均色（带缓存）
fn tile_avg_color(
    libs: &mut GameLibraries,
    cache: &mut std::collections::HashMap<(i16, i32), [u8; 4]>,
    cell: &crate::resources::map_reader::CellInfo,
) -> Option<[u8; 4]> {
    let (lib, img) = cell.back_tile()?;
    if let Some(c) = cache.get(&(lib, img)) {
        return Some(*c);
    }
    let info = libs.0.get_map_image(lib, img)?;
    let rgba = info.rgba.as_ref()?;
    let w = info.width.max(0) as usize;
    let h = info.height.max(0) as usize;
    if w == 0 || h == 0 || rgba.len() < w * h * 4 {
        return None;
    }
    let mut r = 0u64;
    let mut g = 0u64;
    let mut b = 0u64;
    let n = (w * h) as u64;
    for i in 0..(w * h) {
        r += rgba[i * 4] as u64;
        g += rgba[i * 4 + 1] as u64;
        b += rgba[i * 4 + 2] as u64;
    }
    let c = [(r / n) as u8, (g / n) as u8, (b / n) as u8, 255];
    cache.insert((lib, img), c);
    Some(c)
}


/// 消费服务端大地图信息事件（网络层只广播 ServerEvent）
fn big_map_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut big_map: ResMut<BigMapState>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        if let ServerEvent::MapInfo { map_index, title, npcs } = ev {
            big_map.map_index = *map_index;
            big_map.title = title.clone();
            big_map.npcs = npcs.clone();
            big_map.selected = None;
            big_map.top_line = 0;
        }
        // #300：世界地图配置（C# S.WorldMapSetupInfo，进图首次下发）
        if let ServerEvent::WorldMapSetup { enabled, icons, teleport_cost } = ev {
            big_map.world_enabled = *enabled;
            big_map.world_icons = icons.clone();
            big_map.teleport_cost = *teleport_cost;
            tracing::info!("🗺️ 世界地图配置: enabled={} icons={} cost={}", enabled, icons.len(), teleport_cost);
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
        assert_eq!(big_map_member_pos(0, 0, 200.0, 400.0, 400.0, 800.0, 0.0, 0.0), (0.0, 0.0));
        assert_eq!(big_map_member_pos(200, 400, 200.0, 400.0, 400.0, 800.0, 0.0, 0.0), (400.0, 800.0));
    }
}

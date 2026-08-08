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

use crate::game::dialogs::text_input::{TextInputDisplay, TextInputField, TextInputRect, TextInputState, TextInputSubmit};
use crate::game::dialogs::minimap::MemberLocations;
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::game::movement::world_to_tile;
use crate::map_renderer::{GameData, GameLibraries};
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::resources::map_reader::{resolve_map_path, MapReader};
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiEntity, UiFont, UiImageCache,
};

/// 面板尺寸（Title[820] 实测 760x500）
const PANEL_W: f32 = 760.0;
const PANEL_H: f32 = 500.0;
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
pub struct BigMapClose;

#[derive(Component)]
pub struct BigMapScrollUp;

#[derive(Component)]
pub struct BigMapScrollDown;

#[derive(Component)]
pub struct BigMapWorld;

#[derive(Component)]
pub struct BigMapMyLocation;

#[derive(Component)]
pub struct BigMapTeleport;

#[derive(Component)]
pub struct BigMapSearch;

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
            (big_map_ui_system, big_map_world_system, big_map_viewport_system, big_map_member_system, ui_button_system)
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
    mut cache: ResMut<UiImageCache>,
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<UiFont>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();

    let (pw, ph) = match libs.0.get_image(LibraryName::Title, 820) {
        Some(i) => (i.width.max(0) as f32, i.height.max(0) as f32),
        None => (PANEL_W, PANEL_H),
    };
    let px = (1024.0 - pw) / 2.0;
    let py = (768.0 - ph) / 2.0;

    // 面板
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Title, 820) {
        let e = spawn_ui_sprite(&mut commands, h, px, py, 6.0, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::BigMap),
            BigMapWidget,
            Visibility::Hidden,
        ));
    }

    // 标题（C# TitleLabel (19,6) 699x20）
    let t = spawn_ui_text(
        &mut commands, &font, "",
        px + 19.0, py + 6.0, 14.0, Color::WHITE, 8.0,
    );
    commands.entity(t).insert((
        BigMapTitleText,
        DialogRoot(DialogKind::BigMap),
        BigMapWidget,
    ));

    // 关闭 (W-25,3)
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 360, 361, 362,
        px + pw - 25.0, py + 3.0, 7.0, 24.0, 21.0,
    ) {
        commands.entity(e).insert((
            BigMapClose,
            DialogRoot(DialogKind::BigMap),
            BigMapWidget,
        ));
    }

    // 视口背景（深色底）
    let dark = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    commands.spawn((
        DialogRoot(DialogKind::BigMap),
        BigMapWidget,
        Sprite {
            image: dark,
            color: Color::srgb(0.1, 0.13, 0.1),
            custom_size: Some(Vec2::new(VIEW_W, VIEW_H)),
            ..default()
        },
        Transform::from_xyz(px + VIEW_X, -(py + VIEW_Y), 6.1),
        Visibility::Hidden,
    ));

    // 地形纹理（首帧生成后填充）
    let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    let terrain = spawn_ui_sprite(&mut commands, white.clone(), px + VIEW_X, py + VIEW_Y, 6.2, 1.0);
    commands.entity(terrain).insert((
        Sprite {
            image: white,
            color: Color::WHITE,
            custom_size: Some(Vec2::new(VIEW_W, VIEW_H)),
            ..default()
        },
        BigMapTerrain,
        DialogRoot(DialogKind::BigMap),
        BigMapWidget,
    ));

    // 玩家雷达点 Prguse2[1350]
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse2, 1350) {
        let e = spawn_ui_sprite(&mut commands, h, px + VIEW_X, py + VIEW_Y, 6.4, 1.0);
        commands.entity(e).insert((
            BigMapPlayerDot,
            DialogRoot(DialogKind::BigMap),
            BigMapWidget,
        ));
    }

    // NPC 点池（绿色小方块）
    let dot_white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    for i in 0..DOT_POOL {
        let e = spawn_ui_sprite(&mut commands, dot_white.clone(), px + VIEW_X, py + VIEW_Y, 6.3, 1.0);
        commands.entity(e).insert((
            Sprite {
                image: dot_white.clone(),
                color: Color::srgb(0.0, 1.0, 0.2),
                custom_size: Some(Vec2::new(3.0, 3.0)),
                ..default()
            },
            BigMapDot(i),
            DialogRoot(DialogKind::BigMap),
            BigMapWidget,
        ));
    }

    // 队友点池（黄色小方块，C# BigMapDialog Players[MaxGroup]）
    for i in 0..MEMBER_DOT_POOL {
        let e = spawn_ui_sprite(&mut commands, dot_white.clone(), px + VIEW_X, py + VIEW_Y, 6.35, 1.0);
        commands.entity(e).insert((
            Sprite {
                image: dot_white.clone(),
                color: Color::srgb(1.0, 0.9, 0.2),
                custom_size: Some(Vec2::new(3.0, 3.0)),
                ..default()
            },
            BigMapMemberDot(i),
            DialogRoot(DialogKind::BigMap),
            BigMapWidget,
        ));
    }

    // 上滚/下滚 (W-21,48)/(W-21,417)
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 197, 198, 199,
        px + pw - 21.0, py + 48.0, 7.0, 16.0, 14.0,
    ) {
        commands.entity(e).insert((
            BigMapScrollUp,
            DialogRoot(DialogKind::BigMap),
            BigMapWidget,
        ));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 207, 208, 209,
        px + pw - 21.0, py + 417.0, 7.0, 16.0, 14.0,
    ) {
        commands.entity(e).insert((
            BigMapScrollDown,
            DialogRoot(DialogKind::BigMap),
            BigMapWidget,
        ));
    }
    // 位置条 (W-21, 61)
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse2, 205) {
        let e = spawn_ui_sprite(&mut commands, h, px + pw - 21.0, py + 61.0, 7.0, 1.0);
        commands.entity(e).insert((
            BigMapPosBar,
            DialogRoot(DialogKind::BigMap),
            BigMapWidget,
        ));
    }

    // 世界地图按钮 Title[827/828/829] (250, H-33)
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 827, 828, 829,
        px + 250.0, py + ph - 33.0, 7.0, 80.0, 25.0,
    ) {
        commands.entity(e).insert((
            BigMapWorld,
            DialogRoot(DialogKind::BigMap),
            BigMapWidget,
        ));
    }
    // 我的位置 Title[824/825/826] (400, H-33)
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 824, 825, 826,
        px + 400.0, py + ph - 33.0, 7.0, 80.0, 25.0,
    ) {
        commands.entity(e).insert((
            BigMapMyLocation,
            DialogRoot(DialogKind::BigMap),
            BigMapWidget,
        ));
    }
    // 传送按钮 Title[821/822/823] (W-122, 432)
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 821, 822, 823,
        px + pw - 122.0, py + 432.0, 7.0, 72.0, 25.0,
    ) {
        commands.entity(e).insert((
            BigMapTeleport,
            DialogRoot(DialogKind::BigMap),
            BigMapWidget,
        ));
    }
    // 搜索按钮 Prguse2[1340/1341/1342] (23, H-36)
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 1340, 1341, 1342,
        px + 23.0, py + ph - 36.0, 7.0, 32.0, 30.0,
    ) {
        commands.entity(e).insert((
            BigMapSearch,
            DialogRoot(DialogKind::BigMap),
            BigMapWidget,
        ));
    }
    // 搜索输入框（C# SearchTextBox；TextInputField id=10 供大地图专用）
    let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    let label = spawn_ui_text(&mut commands, &font, "搜索:", px + 23.0, py + ph - 70.0, 11.0, Color::WHITE, 8.1);
    commands.entity(label).insert((DialogRoot(DialogKind::BigMap), BigMapWidget));
    commands
        .spawn((
            UiEntity,
            DialogRoot(DialogKind::BigMap),
            BigMapWidget,
            TextInputField(10),
            TextInputRect(px + 60.0, py + ph - 70.0, 150.0, 20.0),
            Sprite {
                image: white,
                color: Color::srgba(0.2, 0.2, 0.25, 0.9),
                custom_size: Some(Vec2::new(150.0, 20.0)),
                ..default()
            },
            bevy::sprite::Anchor::TOP_LEFT,
            Transform::from_xyz(px + 60.0, -(py + ph - 70.0), 8.1),
            Visibility::Visible,
        ));
    let disp = spawn_ui_text(&mut commands, &font, "", px + 63.0, py + ph - 67.0, 11.0, Color::WHITE, 8.2);
    commands.entity(disp).insert((
        TextInputDisplay(10),
        DialogRoot(DialogKind::BigMap),
        BigMapWidget,
    ));

    // 世界地图覆盖层（C# WorldMapImage：Prguse2[1360] 底 + 1365 云 + 1366 边框，Location=(10,0)）
    let wm_x = px + 10.0;
    let wm_y = py;
    for (idx, z) in [(1360usize, 6.6f32), (1365, 6.7), (1366, 6.8)] {
        if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse2, idx) {
            let e = spawn_ui_sprite(&mut commands, h, wm_x, wm_y, z, 1.0);
            commands.entity(e).insert((
                BigMapWorldRoot,
                DialogRoot(DialogKind::BigMap),
                BigMapWidget,
                Visibility::Hidden,
            ));
        }
    }
    // 悬停标题（C# WorldMapImage.TitleLabel：黑底白字，顶部居中）
    let wt = spawn_ui_text(
        &mut commands, &font, "",
        wm_x, wm_y + 8.0, 12.0, Color::WHITE, 6.9,
    );
    commands.entity(wt).insert((
        BigMapWorldTitle,
        BigMapWorldRoot,
        DialogRoot(DialogKind::BigMap),
        BigMapWidget,
        Visibility::Hidden,
    ));
    // 世界地图图标池（MapLinkIcon 帧带 offset，C# UseOffSet=true）
    let wm_white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    for k in 0..WORLD_ICON_POOL {
        let e = spawn_ui_sprite(&mut commands, wm_white.clone(), wm_x, wm_y, 7.0, 1.0);
        commands.entity(e).insert((
            UiButton { rect: (wm_x, wm_y, 16.0, 16.0), clicked: false },
            BigMapWorldIcon(k),
            BigMapWorldRoot,
            DialogRoot(DialogKind::BigMap),
            BigMapWidget,
            Visibility::Hidden,
        ));
    }
    // NPC 列表行（x=590, y=50+i*21，右侧）
    for i in 0..MAX_ROWS {
        let e = spawn_ui_text(
            &mut commands, &font, "",
            px + 590.0, py + 50.0 + i as f32 * 21.0,
            12.0, Color::WHITE, 8.0,
        );
        commands.entity(e).insert((
            BigMapRow(i),
            DialogRoot(DialogKind::BigMap),
            BigMapWidget,
        ));
    }

    // 坐标标签 (519,435)
    let e = spawn_ui_text(
        &mut commands, &font, "",
        px + 519.0, py + 435.0, 12.0, Color::WHITE, 8.0,
    );
    commands.entity(e).insert((
        BigMapCoordText,
        DialogRoot(DialogKind::BigMap),
        BigMapWidget,
    ));
}


#[allow(clippy::too_many_arguments)]
fn big_map_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut state: ResMut<BigMapState>,
    net: ResMut<NetConnection>,
    close: Query<&UiButton, With<BigMapClose>>,
    scroll_up: Query<&UiButton, With<BigMapScrollUp>>,
    scroll_down: Query<&UiButton, With<BigMapScrollDown>>,
    myloc_btn: Query<&UiButton, With<BigMapMyLocation>>,
    teleport_btn: Query<&UiButton, With<BigMapTeleport>>,
    search_btn: Query<&UiButton, With<BigMapSearch>>,
    mut input: ResMut<TextInputState>,
    mut submits: MessageReader<TextInputSubmit>,
    mut widgets: Query<(
        &mut Visibility,
        Option<&BigMapDot>,
        Option<&BigMapPlayerDot>,
    ), (With<BigMapWidget>, Without<BigMapWorldRoot>, Without<BigMapWorld>)>,
    mut rows: Query<(&mut Text2d, &BigMapRow)>,
    mut pos_bar: Query<&mut Transform, With<BigMapPosBar>>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
) {
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
        *vis = if show { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        return;
    }

    for btn in &close {
        if btn.clicked {
            mgr.close(DialogKind::BigMap);
        }
    }
    let max_scroll = state.npcs.len().saturating_sub(MAX_ROWS);
    for btn in &scroll_up {
        if btn.clicked && state.top_line > 0 {
            state.top_line -= 1;
        }
    }
    for btn in &scroll_down {
        if btn.clicked && state.top_line < max_scroll {
            state.top_line += 1;
        }
    }
    for btn in &myloc_btn {
        if btn.clicked {
            state.world_open = false;
            state.selected = None;
            state.top_line = 0;
            net.send_packet(&mir2_shared::packets::client::npc::RequestMapInfo {
                map_index: state.map_index,
            });
            tracing::info!("🗺️ 回到我的位置 map={}", state.map_index);
        }
    }
    // 搜索：按钮点击或输入框回车 → C.SearchMap（服务端按地图/NPC 名搜索并以系统消息返回）
    let mut do_search = false;
    for btn in &search_btn {
        if btn.clicked {
            do_search = true;
        }
    }
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
            net.send_packet(&crate::network::SearchMapWire { keyword: keyword.clone() });
            tracing::info!("🗺️ 搜索: {}", keyword);
        }
        input.active = None;
    }
    for btn in &teleport_btn {
        if btn.clicked {
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

    // 点击 NPC 行选中（C# BigMapNPCRow.Click）
    if let Ok(window) = windows.single() {
        if let Some(cursor) = window.cursor_position() {
            if mouse.just_pressed(MouseButton::Left) {
                let px = (1024.0 - PANEL_W) / 2.0;
                let py = (768.0 - PANEL_H) / 2.0;
                for i in 0..MAX_ROWS {
                    let ry = py + 50.0 + i as f32 * 21.0;
                    if cursor.x >= px + 590.0
                        && cursor.x <= px + 590.0 + 150.0
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
    for mut tf in &mut pos_bar {
        let pct = if max_scroll > 0 {
            state.top_line as f32 / max_scroll as f32
        } else {
            0.0
        };
        tf.translation.y = -((768.0 - PANEL_H) / 2.0 + 61.0 + pct * 342.0);
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
    mut world_btn: Query<(&UiButton, &mut Visibility), (With<BigMapWorld>, Without<BigMapWorldRoot>)>,
    mut world_bg: Query<&mut Visibility, (With<BigMapWorldRoot>, Without<BigMapWorldIcon>, Without<BigMapWorldTitle>, Without<BigMapWorld>)>,
    mut world_title: Query<(&mut Text2d, &mut Visibility), (With<BigMapWorldTitle>, Without<BigMapWorld>)>,
    mut world_icons: Query<
        (&mut Visibility, &mut Sprite, &mut UiButton, &BigMapWorldIcon),
        (With<BigMapWorldRoot>, Without<BigMapWorldTitle>, Without<BigMapWorld>),
    >,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<UiImageCache>,
    mut prev_open: Local<bool>,
    windows: Query<&Window>,
) {
    let open = mgr.is_open(DialogKind::BigMap);
    // C# BigMapDialog.Show() → TargetMyLocation()：重新打开时回到当前地图列表
    if open && !*prev_open {
        state.world_open = false;
    }
    *prev_open = open;

    // 世界按钮仅在 setup.Enabled 时可见（C# WorldMapSetup）
    for (btn, mut vis) in &mut world_btn {
        *vis = if open && state.world_enabled {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if btn.clicked && state.world_enabled {
            state.world_open = !state.world_open;
            tracing::info!("🗺️ 世界地图 {}", if state.world_open { "打开" } else { "关闭" });
        }
    }

    // 覆盖层显隐
    let world_show = open && state.world_enabled && state.world_open;
    for mut vis in &mut world_bg {
        *vis = if world_show { Visibility::Visible } else { Visibility::Hidden };
    }

    // 图标同步 + 悬停标题 + 点击（C# WorldMapImage.MakeButtons：MapLinkIcon 帧 offset，UseOffSet=true）
    let (pw, ph) = match libs.0.get_image(LibraryName::Title, 820) {
        Some(i) => (i.width.max(0) as f32, i.height.max(0) as f32),
        None => (PANEL_W, PANEL_H),
    };
    let px = (1024.0 - pw) / 2.0;
    let py = (768.0 - ph) / 2.0;
    let (wm_x, wm_y) = (px + 10.0, py);

    let mut hover_title = String::new();
    let mut clicked_icon: Option<usize> = None;
    if world_show {
        let cursor = windows.single().ok().and_then(|w| w.cursor_position());
        for (mut vis, mut sprite, mut btn, ic) in &mut world_icons {
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
            let Some(hnd) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::MapLinkIcon, idx) else {
                *vis = Visibility::Hidden;
                continue;
            };
            sprite.image = hnd;
            sprite.custom_size = Some(Vec2::new(w, h));
            btn.rect = (x, y, w, h);
            *vis = Visibility::Visible;
            if let Some(cursor) = cursor {
                if cursor.x >= x && cursor.x <= x + w && cursor.y >= y && cursor.y <= y + h {
                    hover_title = icon.title.clone();
                }
            }
            if btn.clicked {
                clicked_icon = Some(k);
            }
        }
    } else {
        for (mut vis, _, _, _) in &mut world_icons {
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
    mut dots: Query<(&mut Transform, &mut Visibility, &BigMapMemberDot)>
) {
    let open = mgr.is_open(DialogKind::BigMap);
    let (tw, th) = state.tex_size;
    let (mw, mh) = state.map_size;
    if tw <= 0.0 || mw <= 0.0 {
        for (_, mut vis, _) in &mut dots { *vis = Visibility::Hidden; }
        return;
    }
    let px = (1024.0 - PANEL_W) / 2.0;
    let py = (768.0 - PANEL_H) / 2.0;
    let vx = px + VIEW_X + (VIEW_W - tw) / 2.0;
    let vy = py + VIEW_Y + (VIEW_H - th) / 2.0;
    for (mut tf, mut vis, dot) in &mut dots {
        if open && dot.0 < locs.members.len() {
            let (_, mx, my) = &locs.members[dot.0];
            let (sx, sy) = big_map_member_pos(*mx, *my, mw, mh, tw, th, vx, vy);
            tf.translation.x = sx - 1.5;
            tf.translation.y = -(sy - 1.5);
            *vis = Visibility::Visible;
        } else {
            *vis = Visibility::Hidden;
        }
    }
}

fn big_map_viewport_system(
    mgr: Res<DialogManager>,
    mut state: ResMut<BigMapState>,
    game_data: Res<GameData>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut sprites: Query<(
        &mut Sprite,
        &mut Transform,
        Option<&BigMapTerrain>,
        Option<&BigMapPlayerDot>,
        Option<&BigMapDot>,
    ), Without<crate::actor::LocalPlayer>>,
    players: Query<&Transform, (With<crate::actor::LocalPlayer>, Without<BigMapWidget>)>,
    windows: Query<&Window>,
    mut texts: Query<(&mut Text2d, Option<&BigMapTitleText>, Option<&BigMapCoordText>)>,
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
                let (tex, tw, th, mw, mh) = build_terrain_texture(&mut libs, &mut images, &reader, &map);
                for (mut sprite, _tf, terrain, _, _) in &mut sprites {
                    if terrain.is_some() {
                        sprite.image = tex.clone();
                        sprite.custom_size = Some(Vec2::new(tw, th));
                        sprite.rect = None;
                    }
                }
                state.viewport_ready = true;
                state.tex_size = (tw, th);
                state.map_size = (mw, mh);
                tracing::info!(
                    "🗺️ 大地图地形生成: {}x{} 纹理 {}x{}",
                    mw,
                    mh,
                    tw,
                    th
                );
            }
        }
    }

    let (tw, th) = state.tex_size;
    let (mw, mh) = state.map_size;
    if tw <= 0.0 || mw <= 0.0 {
        return;
    }
    let px = (1024.0 - PANEL_W) / 2.0;
    let py = (768.0 - PANEL_H) / 2.0;
    let vx = px + VIEW_X + (VIEW_W - tw) / 2.0;
    let vy = py + VIEW_Y + (VIEW_H - th) / 2.0;

    // 玩家点
    for (mut sprite, mut tf, _terrain, pdot, dot) in &mut sprites {
        if pdot.is_some() {
            if let Ok(player_tf) = players.single() {
                let (tx, ty) = world_to_tile(player_tf.translation.x, player_tf.translation.y);
                tf.translation.x = vx + (tx as f32 / mw) * tw;
                tf.translation.y = -(vy + (ty as f32 / mh) * th);
            }
        } else if let Some(d) = dot {
            if let Some(npc) = state.npcs.get(d.0) {
                let sx = vx + (npc.x as f32 / mw) * tw;
                let sy = vy + (npc.y as f32 / mh) * th;
                tf.translation.x = sx - 1.5;
                tf.translation.y = -(sy - 1.5);
                let selected = state.selected == Some(d.0);
                sprite.color = if selected {
                    Color::srgb(1.0, 0.9, 0.1)
                } else {
                    Color::srgb(0.0, 1.0, 0.2)
                };
            }
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
                    if cursor.x >= vx && cursor.x <= vx + tw && cursor.y >= vy && cursor.y <= vy + th {
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

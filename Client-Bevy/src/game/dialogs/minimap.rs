// ============================================================================
// 小地图（M9 第 1 批 + #162 大/小模式）
// 布局参考：C# MainDialogs.cs MiniMapDialog
//   - 位置 (ScreenWidth-126, 0)，背景 Prguse[2090]（大 128x154）/ Prguse[2091]（小 128x45）
//   - 地图显示区（仅大模式）：(3, 22, 120, 108)，深绿底 + 玩家位置点 + 对象光点
//   - 地图名标签 (2,2)、坐标标签 (46, Height-23)、ToggleButton (109,3)
//   - 底部按钮 MailButton(4,y)/BigMapButton(25,y)/LightSetting(102,y)，y = Size.Height-23
//   - C# 默认 _bigMode = true（Index=2090）；小模式不绘制地图，只留面板+按钮
// ============================================================================
#![allow(clippy::type_complexity)]

use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::actor::{LocalPlayer, Monster, Npc};
use crate::game::movement::world_to_tile;
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::{GameData, GameLibraries};
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::UiFont;
use crate::ui::theme::{
    load_lib_image, spawn_container, spawn_icon_button, spawn_image, spawn_label, spawn_panel,
};

const MINIMAP_X: f32 = 1024.0 - 126.0;
const MINIMAP_Y: f32 = 0.0;

/// 小地图显示区（对齐 C#：drawLocation=(3,22)，viewRect=120x108，仅大模式绘制）
const MAP_RECT: (f32, f32, f32, f32) = (3.0, 22.0, 120.0, 108.0);

/// 背景图索引（C# Index：2090 大 / 2091 小）
const BG_BIG: usize = 2090;
const BG_SMALL: usize = 2091;

/// 底部按钮/标签 y = Size.Height - 23（C# SetBigMode/SetSmallMode）
const BOTTOM_Y_BIG: f32 = 154.0 - 23.0; // 131
const BOTTOM_Y_SMALL: f32 = 45.0 - 23.0; // 22

#[derive(Component)]
pub struct MiniMapWidget;

/// 背景（保存大/小两套句柄，切换模式时换图）
#[derive(Component)]
pub struct MiniMapBg {
    pub big: Handle<Image>,
    pub small: Handle<Image>,
}

/// 小地图模式（大/小，C# MiniMapDialog _bigMode，默认大）
#[derive(Resource)]
pub struct MiniMapMode {
    pub big: bool,
}

impl Default for MiniMapMode {
    fn default() -> Self {
        Self { big: true }
    }
}

/// 大/小模式切换按钮（C# ToggleButton Prguse[2102/2103/2104] (109,3)）
#[derive(Component)]
pub struct MiniMapToggle;

/// 邮件按钮（C# MailButton Prguse[2099/2100/2101] (4, y)）
#[derive(Component)]
pub struct MiniMapMailButton;

/// 大地图按钮（C# BigMapButton Prguse[2096/2097/2098] (25, y)）
#[derive(Component)]
pub struct MiniMapBigMapButton;

/// 灯光状态指示（C# LightSetting：Prguse[2093] Normal/Day、[2095] Dawn、[2094] Evening、[2092] Night）
#[derive(Component)]
pub struct MiniMapLightSetting {
    pub normal: Handle<Image>,
    pub dawn: Handle<Image>,
    pub evening: Handle<Image>,
    pub night: Handle<Image>,
}

/// 地图区域底色
#[derive(Component)]
pub struct MiniMapMapArea;

#[derive(Component)]
pub struct MiniMapPlayerDot;

#[derive(Component)]
pub struct MiniMapNameText;

/// 对象光点（玩家白/NPC 绿/怪物红；2x2 点，C# RadarTexture 语义）
#[derive(Component)]
pub struct MiniMapActorDot(pub usize);

/// #254 小队成员光点（黄色）
#[derive(Component)]
pub struct MiniMapMemberDot(pub usize);

/// #254 成员位置状态（S.SendMemberLocation 按名字 upsert）
#[derive(Resource, Default)]
pub struct MemberLocations {
    /// (名字, 地图 index, x, y)；#1309 增地图用于跨图过滤
    pub members: Vec<(String, u16, i32, i32)>,
}

impl MemberLocations {
    pub fn upsert(&mut self, name: String, map_index: u16, x: i32, y: i32) {
        if let Some(entry) = self.members.iter_mut().find(|(n, _, _, _)| *n == name) {
            *entry = (name, map_index, x, y);
        } else {
            self.members.push((name, map_index, x, y));
        }
    }
}

/// 当前地图 index（#1309：由 ServerEvent::MapInfo 更新，供队友点跨图过滤）
#[derive(Resource, Default)]
pub struct CurrentMapIndex(pub i32);

#[derive(Component)]
pub struct MiniMapPosText;

pub struct MiniMapPlugin;

impl Plugin for MiniMapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MiniMapMode>();
        app.init_resource::<MemberLocations>();
        app.init_resource::<CurrentMapIndex>();
        app.add_systems(OnEnter(AppState::Game), spawn_minimap);
        app.add_systems(OnExit(AppState::Game), cleanup_minimap);
        app.add_systems(
            Update,
            // #148 小地图快捷键改由 dialog_hotkey_system 按键位设置处理（可重绑）
            (
                minimap_toggle_system,
                minimap_ui_system,
                minimap_member_events,
                current_map_index_events,
                minimap_member_dots_system,
            )
                .chain()
                .after(crate::network::network_system)
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_minimap(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_minimap(
    mut commands: Commands,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<UiFont>,
) {
if !crate::ui::sprite_ui::ui_enabled("map") {
    return;
}

    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();

    // 背景 Prguse[2090]（大模式默认 128x154 @ (898,0)；ui_system 随模式换图+改尺寸）
    let Some(big) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, BG_BIG) else {
        return;
    };
    let small = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, BG_SMALL);
    let panel = spawn_panel(&mut commands, big.clone(), MINIMAP_X, MINIMAP_Y, 128.0, 154.0, 20);
    commands.entity(panel).insert((
        DialogRoot(DialogKind::Minimap),
        MiniMapWidget,
        MiniMapBg { big: big.clone(), small: small.clone().unwrap_or_else(|| big.clone()) },
        Visibility::Hidden,
    ));

    commands.entity(panel).with_children(|p| {
        // 地图区域底色（深绿矩形，仅大模式显示）
        spawn_container(p, MAP_RECT.0, MAP_RECT.1, MAP_RECT.2, MAP_RECT.3, 1)
            .insert((
                MiniMapMapArea,
                BackgroundColor(Color::srgb(0.12, 0.16, 0.12)),
                Visibility::Hidden,
            ));
        // 玩家位置点（C# 玩家为白点 4x4）
        let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
        spawn_image(p, white.clone(), MAP_RECT.0, MAP_RECT.1, 4.0, 4.0, 2)
            .insert((MiniMapPlayerDot, Visibility::Hidden));
        // 对象光点（最多 24 个，#120 C# MiniMap RadarTexture 2x2）
        for i in 0..24usize {
            spawn_image(p, white.clone(), -999.0, -999.0, 2.0, 2.0, 2)
                .insert((MiniMapActorDot(i), Visibility::Hidden));
        }
        // #254：小队成员光点（最多 10 个，黄色 2x2）
        for i in 0..10usize {
            spawn_image(p, white.clone(), -999.0, -999.0, 2.0, 2.0, 2)
                .insert((MiniMapMemberDot(i), Visibility::Hidden));
        }
        // 地图名（C# MapNameLabel (2,2) 120x18）
        spawn_label(p, &font, "", 12.0, 2.0, 12.0, Color::WHITE, 3).insert(MiniMapNameText);
        // 坐标（C# LocationLabel (46, Height-23)）
        spawn_label(p, &font, "", 54.0, BOTTOM_Y_BIG, 12.0, Color::WHITE, 3)
            .insert(MiniMapPosText);
        // 大小切换按钮（C# ToggleButton Prguse[2102/2103/2104] (109,3)）
        spawn_minimap_button(p, &mut libs, &mut images, 2102, 2103, 2104, 109.0, 3.0, 4)
            .insert(MiniMapToggle);
        // 邮件按钮（C# MailButton Prguse[2099/2100/2101] (4, bottom_y)）
        spawn_minimap_button(p, &mut libs, &mut images, 2099, 2100, 2101, 4.0, BOTTOM_Y_BIG, 4)
            .insert(MiniMapMailButton);
        // 大地图按钮（C# BigMapButton Prguse[2096/2097/2098] (25, bottom_y)）
        spawn_minimap_button(p, &mut libs, &mut images, 2096, 2097, 2098, 25.0, BOTTOM_Y_BIG, 4)
            .insert(MiniMapBigMapButton);
        // 灯光状态指示（C# LightSetting：2093 Normal/Day、2095 Dawn、2094 Evening、2092 Night）
        if let (Some(normal), Some(dawn), Some(evening), Some(night)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 2093),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 2095),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 2094),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 2092),
        ) {
            spawn_image(p, normal.clone(), 102.0, BOTTOM_Y_BIG, 14.0, 14.0, 4)
                .insert(MiniMapLightSetting { normal, dawn, evening, night });
        }
    });
}

/// 按图像实际尺寸生成小地图三态按钮（面板子节点）
fn spawn_minimap_button<'a>(
    parent: &'a mut bevy::ecs::hierarchy::ChildSpawnerCommands,
    libs: &mut GameLibraries,
    images: &mut Assets<Image>,
    normal: usize,
    hover: usize,
    pressed: usize,
    x: f32,
    y: f32,
    z: i32,
) -> EntityCommands<'a> {
    let (w, h) = match libs.0.get_image(LibraryName::Prguse, normal) {
        Some(i) => (i.width.max(0) as f32, i.height.max(0) as f32),
        None => (14.0, 14.0),
    };
    let (n, hov, pr) = (
        load_lib_image(libs, images, LibraryName::Prguse, normal).unwrap(),
        load_lib_image(libs, images, LibraryName::Prguse, hover).unwrap(),
        load_lib_image(libs, images, LibraryName::Prguse, pressed).unwrap(),
    );
    spawn_icon_button(parent, n, hov, pr, x, y, w, h, z)
}


/// 按钮点击：大小切换 / 打开邮件 / 打开大地图（C# Click 处理）
fn minimap_toggle_system(
    mut mode: ResMut<MiniMapMode>,
    mut mgr: ResMut<DialogManager>,
    toggle: Query<(Entity, &Interaction), With<MiniMapToggle>>,
    mail: Query<(Entity, &Interaction), With<MiniMapMailButton>>,
    bigmap: Query<(Entity, &Interaction), With<MiniMapBigMapButton>>,
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
    if let Ok((e, inter)) = toggle.single() {
        if edge(e, inter, &mut prev_inter) {
            mode.big = !mode.big;
        }
    }
    if let Ok((e, inter)) = mail.single() {
        if edge(e, inter, &mut prev_inter) {
            mgr.toggle(DialogKind::Mail);
        }
    }
    if let Ok((e, inter)) = bigmap.single() {
        if edge(e, inter, &mut prev_inter) {
            mgr.toggle(DialogKind::BigMap);
        }
    }
}

fn minimap_ui_system(
    mgr: Res<DialogManager>,
    mode: Res<MiniMapMode>,
    dn: Res<crate::game::day_night::DayNight>,
    game_data: Res<GameData>,
    players: Query<&Transform, (With<LocalPlayer>, Without<MiniMapPlayerDot>)>,
    actors: Query<
        (&Transform, Option<&Monster>, Option<&Npc>),
        (
            Without<MiniMapPlayerDot>,
            Without<MiniMapActorDot>,
            Without<LocalPlayer>,
            Without<MiniMapPosText>,
            Without<MiniMapLightSetting>,
            Without<MiniMapMailButton>,
            Without<MiniMapBigMapButton>,
        ),
    >,
    mut widgets: Query<&mut Visibility, With<MiniMapWidget>>,
    mut bg: Query<(&mut Node, &mut ImageNode, &MiniMapBg)>,
    mut map_area: Query<&mut Visibility, (With<MiniMapMapArea>, Without<MiniMapPlayerDot>)>,
    mut dot: Query<
        (&mut Node, &mut Visibility),
        (With<MiniMapPlayerDot>, Without<MiniMapActorDot>),
    >,
    mut actor_dots: Query<
        (&mut Node, &mut ImageNode, &mut Visibility, &MiniMapActorDot),
        (Without<MiniMapPlayerDot>, Without<MiniMapBg>, Without<MiniMapMapArea>),
    >,
    mut name_texts: Query<&mut Text, (With<MiniMapNameText>, Without<MiniMapPosText>)>,
    mut pos_texts: Query<
        (&mut Text, &mut Node),
        (With<MiniMapPosText>, Without<MiniMapNameText>),
    >,
    mut light: Query<
        (&mut Node, &mut ImageNode, &MiniMapLightSetting),
        Without<MiniMapPosText>,
    >,
    mut mail_btn: Query<&mut Node, (With<MiniMapMailButton>, Without<MiniMapBigMapButton>, Without<MiniMapToggle>)>,
    mut bigmap_btn: Query<&mut Node, (With<MiniMapBigMapButton>, Without<MiniMapMailButton>, Without<MiniMapToggle>)>,
) {
    let open = mgr.is_open(DialogKind::Minimap);
    let big = mode.big;
    let bottom_y = if big { BOTTOM_Y_BIG } else { BOTTOM_Y_SMALL };

    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }

    // 背景换图 + 尺寸（2090 大 128x154 / 2091 小 128x45）
    for (mut node, mut img, bg) in &mut bg {
        let want = if big { bg.big.clone() } else { bg.small.clone() };
        if img.image != want {
            img.image = want;
        }
        node.width = Val::Px(128.0);
        node.height = Val::Px(if big { 154.0 } else { 45.0 });
    }

    let (map_w, map_h) = match &game_data.map {
        Some(m) => (m.width as f32, m.height as f32),
        None => (1.0, 1.0),
    };

    // 地图区域：仅大模式显示（C# Index != 2090 时不绘制地图）
    for mut vis in &mut map_area {
        *vis = if open && big { Visibility::Visible } else { Visibility::Hidden };
    }

    if let Ok((mut dot_node, mut dot_vis)) = dot.single_mut() {
        if !open || !big {
            *dot_vis = Visibility::Hidden;
        } else {
            *dot_vis = Visibility::Visible;
            // 玩家格子坐标 → 小地图像素（面板子节点，相对坐标；面板原点 (898,0)）
            if let Ok(player_tf) = players.single() {
                let (tx, ty) = world_to_tile(player_tf.translation.x, player_tf.translation.y);
                let px = MAP_RECT.0 + (tx as f32 / map_w) * MAP_RECT.2;
                let py = MAP_RECT.1 + (ty as f32 / map_h) * MAP_RECT.3;
                dot_node.left = Val::Px(px - 2.0);
                dot_node.top = Val::Px(py - 2.0);
                if let Ok((mut t, mut tf)) = pos_texts.single_mut() {
                    let s = format!("{},{}", tx, ty);
                    if t.0 != s {
                        t.0 = s; // 变化才更新，避免每帧重排文本（ICU4X/CPU，#31）
                    }
                    tf.left = Val::Px(54.0);
                    tf.top = Val::Px(bottom_y);
                }
            }
        }
    }

    // 对象光点（#120 C# RadarTexture）：玩家白/NPC 绿/怪物红；仅大模式
    if open && big {
        let mut points: Vec<(f32, f32, Color)> = actors
            .iter()
            .map(|(tf, mon, npc)| {
                let (tx, ty) = world_to_tile(tf.translation.x, tf.translation.y);
                let px = MAP_RECT.0 + (tx as f32 / map_w) * MAP_RECT.2;
                let py = MAP_RECT.1 + (ty as f32 / map_h) * MAP_RECT.3;
                let color = if npc.is_some() {
                    Color::srgb(0.0, 1.0, 0.2)
                } else if mon.is_some() {
                    Color::srgb(1.0, 0.1, 0.1)
                } else {
                    Color::WHITE // 其他玩家
                };
                (px, py, color)
            })
            .collect();
        for (mut node, mut img, mut vis, idx) in &mut actor_dots {
            match points.get(idx.0) {
                Some((px, py, color)) => {
                    node.left = Val::Px(px - 1.0);
                    node.top = Val::Px(py - 1.0);
                    if img.color != *color {
                        img.color = *color;
                    }
                    *vis = Visibility::Visible;
                }
                None => *vis = Visibility::Hidden,
            }
        }
    } else {
        for (_, _, mut vis, _) in &mut actor_dots {
            *vis = Visibility::Hidden;
        }
    }

    if let Ok(mut t) = name_texts.single_mut() {
        let name = game_data.map.as_ref().map(|m| m.name.clone()).unwrap_or_default();
        if t.0 != name {
            t.0 = name;
        }
    }

    // 灯光指示 y + 图标（C# GameScene.TimeOfDay：Normal/Day→2093 Dawn→2095 Evening→2094 Night→2092）
    for (mut node, mut img, lset) in &mut light {
        node.top = Val::Px(bottom_y);
        let want = match dn.light {
            mir2_shared::enums::LightSetting::Dawn => lset.dawn.clone(),
            mir2_shared::enums::LightSetting::Evening => lset.evening.clone(),
            mir2_shared::enums::LightSetting::Night => lset.night.clone(),
            _ => lset.normal.clone(),
        };
        if img.image != want {
            img.image = want;
        }
    }

    // 底部按钮位置（C# MailButton (4,y) / BigMapButton (25,y)）
    for mut node in &mut mail_btn {
        node.left = Val::Px(4.0);
        node.top = Val::Px(bottom_y);
    }
    for mut node in &mut bigmap_btn {
        node.left = Val::Px(25.0);
        node.top = Val::Px(bottom_y);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::libraries::{resolve_data_path, Libraries};

    /// 小地图布局对齐 C# MainDialogs.cs MiniMapDialog：
    /// - 背景 Prguse[2090] 128x154 / Prguse[2091] 128x45
    /// - 地图区 (3,22,120,108)，底部 y = Size.Height - 23（大 131 / 小 22）
    /// - ToggleButton(109,3)、MailButton(4,y)、BigMapButton(25,y)、LightSetting(102,y)
    #[test]
    fn minimap_layout_matches_csharp() {
        // 依赖真实 .Lib 数据：本地无数据（CI/新检出）时跳过，避免假红
        let data_path = resolve_data_path();
        if !data_path.join("Items.Lib").exists() {
            eprintln!("跳过：无本地游戏数据（{}），本测试依赖真实 .Lib 资源", data_path.display());
            return;
        }
        let mut libs = Libraries::new(data_path);
        libs.ensure_initialized();

        let big = libs.get_image(LibraryName::Prguse, BG_BIG).expect("Prguse[2090] 缺失");
        assert_eq!((big.width, big.height), (128, 154), "Prguse[2090] 应为 128x154（大模式）");
        let small = libs.get_image(LibraryName::Prguse, BG_SMALL).expect("Prguse[2091] 缺失");
        assert_eq!((small.width, small.height), (128, 45), "Prguse[2091] 应为 128x45（小模式）");

        assert_eq!(MAP_RECT, (3.0, 22.0, 120.0, 108.0), "地图区应对齐 C# viewRect+drawLocation");
        assert_eq!(BOTTOM_Y_BIG, 131.0, "大模式底部 y = 154-23");
        assert_eq!(BOTTOM_Y_SMALL, 22.0, "小模式底部 y = 45-23");

        // 三态按钮/指示图均存在且尺寸 > 0
        for idx in [2102usize, 2103, 2104, 2099, 2100, 2101, 2096, 2097, 2098, 2093] {
            let i = libs.get_image(LibraryName::Prguse, idx).unwrap_or_else(|| panic!("Prguse[{idx}] 缺失"));
            assert!(i.width > 0 && i.height > 0, "Prguse[{idx}] 尺寸应为正");
        }
        // 默认大模式（C# _bigMode = true）
        assert!(MiniMapMode::default().big, "C# 默认 _bigMode = true");
        println!("  ✓ 小地图 大/小模式布局与 C# 对齐（2090=128x154 / 2091=128x45）");
    }
}



/// #1309：ServerEvent::MapInfo → CurrentMapIndex（当前地图，供队友点跨图过滤）
fn current_map_index_events(
    mut current: ResMut<CurrentMapIndex>,
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
) {
    for ev in events.read() {
        if let crate::network::server_event::ServerEvent::MapInfo { map_index, .. } = ev {
            current.0 = *map_index;
        }
    }
}

/// #254：S.SendMemberLocation → MemberLocations（按名字 upsert）
fn minimap_member_events(
    mut locs: ResMut<MemberLocations>,
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
) {
    for ev in events.read() {
        if let crate::network::server_event::ServerEvent::MemberLocation { name, map_index, x, y } = ev {
            locs.upsert(name.clone(), *map_index, *x, *y);
        }
    }
}

/// #254：成员光点定位（与玩家光点同公式；无成员/小地图未开/大模式才显示）
fn minimap_member_dots_system(
    mgr: Res<DialogManager>,
    mode: Res<MiniMapMode>,
    game_data: Res<GameData>,
    locs: Res<MemberLocations>,
    current: Res<CurrentMapIndex>,
    mut dots: Query<
        (&mut Node, &mut Visibility, &MiniMapMemberDot),
        (
            Without<MiniMapWidget>,
            Without<MiniMapPlayerDot>,
            Without<MiniMapActorDot>,
        ),
    >,
) {
    let open = mgr.is_open(DialogKind::Minimap);
    let big = mode.big;
    let (map_w, map_h) = match &game_data.map {
        Some(m) => (m.width as f32, m.height as f32),
        None => (1.0, 1.0),
    };
    for (mut node, mut vis, idx) in &mut dots {
        let Some((_, map_idx, tx, ty)) = locs.members.get(idx.0) else {
            *vis = Visibility::Hidden;
            continue;
        };
        // #1309：只显示同图队友（C# GroupMembersMap 按图过滤）
        if *map_idx as i32 != current.0 {
            *vis = Visibility::Hidden;
            continue;
        }
        if !open || !big {
            *vis = Visibility::Hidden;
            continue;
        }
        *vis = Visibility::Visible;
        let px = MAP_RECT.0 + (*tx as f32 / map_w) * MAP_RECT.2;
        let py = MAP_RECT.1 + (*ty as f32 / map_h) * MAP_RECT.3;
        node.left = Val::Px(px - 1.0);
        node.top = Val::Px(py - 1.0);
    }
}

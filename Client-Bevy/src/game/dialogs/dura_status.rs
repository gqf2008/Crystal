// ============================================================================
// 耐久面板（M55）
// 参考：C# CharacterDuraPanel（Client/MirScenes/Dialogs/MainDialogs.cs）
//   - 切换按钮 DuraStatusDialog：Prguse[2111/2112/2113]（打开时 2110）
//     @ (MiniMap.X+86+20, MiniMap.Height) = (1004, 大154/小45)（随小地图大/小模式）
//   - 面板 Prguse[2105]（64x85）@ (963,200)，装备部位按耐久阈值切换
//     Prguse 索引：正常/警告/损坏 三态（2122-2160）
// 纯客户端：数据来自 `Loadout` 组件（#2633 批次4 步6；UserInformation 下发，含 current_dura/max_dura）
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::inventory::InvItem;
use crate::game::dialogs::minimap::MiniMapMode;
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::theme::{load_lib_image, spawn_image, spawn_panel, ImageButton};

/// 面板位置（C#：ScreenWidth-61=963, y=200；背景图 (3,3) 内布局）
const PANEL_X: f32 = 963.0;
const PANEL_Y: f32 = 200.0;

// C# DuraStatusDialog（MainDialogs.cs:3911）容器 @ (MiniMap.X+86, MiniMap.Size.Height)，
//   Character 切换钮相对 (20,0)（:3919）→ 绝对 (MiniMap.X+106, MiniMap.Height)，
//   且 SetBigMode/SetSmallMode（:2060,2070）随小地图大/小模式更新 Y。
//   MiniMap.X = 1024-126 = 898；高度 大=Prguse[2090]实测154 / 小=Prguse[2091]实测45。
/// 小地图左边 x = ScreenWidth-126（C# MiniMapDialog Location）
pub const MINIMAP_X: f32 = 1024.0 - 126.0; // 898
/// 小地图大/小模式高度（Prguse[2090]/[2091] 实测）
pub const MINIMAP_H_BIG: f32 = 154.0;
pub const MINIMAP_H_SMALL: f32 = 45.0;
/// 切换钮 X = MiniMap.X + 86（容器）+ 20（钮相对）（C#）
pub const BTN_X: f32 = MINIMAP_X + 86.0 + 20.0; // 1004

/// 切换钮 Y = 小地图当前高度（C# Location.Y = MiniMap.Size.Height，随大/小模式）
pub fn dura_btn_y(minimap_big: bool) -> f32 {
    if minimap_big {
        MINIMAP_H_BIG
    } else {
        MINIMAP_H_SMALL
    }
}

/// 装备部位（对应 ServerRust EquipmentSlot 0..13，#1136 补 Torch/Belt/Stone）
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DuraPieceKind {
    Weapon,
    Armour,
    Helmet,
    Necklace,
    BraceletL,
    BraceletR,
    RingL,
    RingR,
    Boots,
    Amulet,
    Mount,
    Torch,
    Belt,
    Stone,
}

/// 部位定义：服务端槽位 + 面板内相对坐标 + Prguse 三态索引 [正常, 警告, 危险]
struct PieceDef {
    kind: DuraPieceKind,
    server_slot: usize,
    rel: (f32, f32),
    idx: [usize; 3],
}

const PIECES: [PieceDef; 14] = [
    PieceDef { kind: DuraPieceKind::Weapon, server_slot: 0, rel: (4.0, 5.0), idx: [2125, 2126, 2127] },
    PieceDef { kind: DuraPieceKind::Armour, server_slot: 1, rel: (16.0, 11.0), idx: [2149, 2150, 2151] },
    PieceDef { kind: DuraPieceKind::Helmet, server_slot: 2, rel: (24.0, 3.0), idx: [2155, 2156, 2157] },
    PieceDef { kind: DuraPieceKind::Necklace, server_slot: 3, rel: (3.0, 67.0), idx: [2122, 2123, 2124] },
    PieceDef { kind: DuraPieceKind::BraceletL, server_slot: 4, rel: (3.0, 43.0), idx: [2143, 2144, 2145] },
    PieceDef { kind: DuraPieceKind::BraceletR, server_slot: 5, rel: (43.0, 43.0), idx: [2143, 2144, 2145] },
    PieceDef { kind: DuraPieceKind::RingL, server_slot: 6, rel: (3.0, 54.0), idx: [2131, 2132, 2133] },
    PieceDef { kind: DuraPieceKind::RingR, server_slot: 7, rel: (43.0, 54.0), idx: [2131, 2132, 2133] },
    PieceDef { kind: DuraPieceKind::Boots, server_slot: 8, rel: (17.0, 43.0), idx: [2152, 2153, 2154] },
    PieceDef { kind: DuraPieceKind::Amulet, server_slot: 9, rel: (16.0, 54.0), idx: [2134, 2135, 2136] },
    PieceDef { kind: DuraPieceKind::Mount, server_slot: 10, rel: (43.0, 68.0), idx: [2140, 2141, 2142] },
    // #1136：C# CharacterDuraPanel Torch/Belt/Stone（C# 位置：Torch(44,5) Belt(23,23) Stone(30,54)）
    PieceDef { kind: DuraPieceKind::Torch, server_slot: 11, rel: (44.0, 5.0), idx: [2146, 2147, 2148] },
    PieceDef { kind: DuraPieceKind::Belt, server_slot: 12, rel: (23.0, 23.0), idx: [2158, 2159, 2160] },
    PieceDef { kind: DuraPieceKind::Stone, server_slot: 13, rel: (30.0, 54.0), idx: [2137, 2137, 2137] },
];

#[derive(Component)]
pub struct DuraWidget;

/// 切换按钮（打开时换 2110）
#[derive(Component)]
pub struct DuraToggleBtn;

/// 耐久部位精灵（kind + 当前索引）
#[derive(Component)]
pub struct DuraPiece(pub DuraPieceKind);

pub struct DuraPlugin;

impl Plugin for DuraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Game), spawn_dura_status);
        app.add_systems(OnExit(AppState::Game), cleanup_dura_status);
        app.add_systems(
            Update,
            dura_status_ui_system.run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_dura_status(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_dura_status(
    mut commands: Commands,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
) {
    libs.0.ensure_initialized();

    // 切换按钮：独立根节点（C# DuraStatusDialog 恒可见，不随面板显隐）。
    // Prguse 2111 hover / 2112 pressed / 2113 normal；打开时 ui_system 换 2110。
    if let (Some(n), Some(h), Some(pr)) = (
        load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 2113),
        load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 2111),
        load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 2112),
    ) {
        commands.spawn((
            Button,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(BTN_X),
                top: Val::Px(dura_btn_y(true)),
                width: Val::Px(20.0),
                height: Val::Px(19.0),
                ..default()
            },
            ImageNode::new(n.clone()),
            ImageButton { normal: n, hover: h, pressed: pr },
            DuraToggleBtn,
            // 恒可见（C# DuraStatusDialog 不随面板显隐）：标记 AlwaysVisible，
            // 使通用对话框兜底（enforce_dialog_visibility）跳过本钮
            crate::game::dialogs::AlwaysVisible,
            DialogRoot(DialogKind::DuraStatus),
            GlobalZIndex(35),
            Visibility::Visible,
        ));
    }

    // 面板 Prguse[2105] @ (963,200)
    let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 2105) else {
        return;
    };
    let panel = spawn_panel(&mut commands, bg, PANEL_X, PANEL_Y, 64.0, 85.0, 30);
    commands
        .entity(panel)
        .insert((DialogRoot(DialogKind::DuraStatus), DuraWidget));

    // 部位图（C# Background @ (3,3) 内相对坐标；面板子节点，随面板显隐）
    commands.entity(panel).with_children(|p| {
        for def in &PIECES {
            if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, def.idx[0]) {
                // 部位图用帧原生尺寸（C# 不缩放）
                let (iw, ih) = match libs.0.get_image(LibraryName::Prguse, def.idx[0]) {
                    Some(i) => (i.width.max(0) as f32, i.height.max(0) as f32),
                    None => (18.0, 18.0),
                };
                spawn_image(p, h, 3.0 + def.rel.0, 3.0 + def.rel.1, iw, ih, 9)
                    .insert(DuraPiece(def.kind));
            }
        }
    });
}

/// 按 C# UpdateCharacterDura 规则计算耐久状态索引（返回 -1 表示空/损坏隐藏）
fn dura_index(item: &InvItem, kind: DuraPieceKind) -> i32 {
    let warning = item.max_dura / 2;
    let danger = item.max_dura / 5;
    let cur = item.current_dura;
    let (normal, warn, dang) = match kind {
        DuraPieceKind::Weapon => (2125, 2126, 2127),
        DuraPieceKind::Armour => (2149, 2150, 2151),
        DuraPieceKind::Helmet => (2155, 2156, 2157),
        DuraPieceKind::Necklace => (2122, 2123, 2124),
        DuraPieceKind::BraceletL | DuraPieceKind::BraceletR => (2143, 2144, 2145),
        DuraPieceKind::RingL | DuraPieceKind::RingR => (2131, 2132, 2133),
        DuraPieceKind::Boots => (2152, 2153, 2154),
        DuraPieceKind::Amulet => (2134, 2135, 2136),
        DuraPieceKind::Mount => (2140, 2141, 2142),
        DuraPieceKind::Torch => (2146, 2147, 2148),
        DuraPieceKind::Belt => (2158, 2159, 2160),
        // C# Stone 仅在耐久为 0 时显示 2137（破损帧），健康时隐藏
        DuraPieceKind::Stone => return if cur == 0 { 2137 } else { -1 },
    };
    if cur == 0 {
        return -1;
    }
    if cur > warning {
        normal
    } else if cur > danger {
        warn
    } else {
        dang
    }
}

fn dura_status_ui_system(
    mut mgr: ResMut<DialogManager>,
    loadout_q: Query<&crate::game::player_state::Loadout, With<crate::actor::LocalPlayer>>,
    mode: Res<MiniMapMode>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut widgets: Query<&mut Visibility, (With<DuraWidget>, Without<DuraPiece>)>,
    mut pieces: Query<(&mut Visibility, &mut ImageNode, &DuraPiece)>,
    mut toggle: Query<(Entity, &Interaction, &mut ImageButton, &mut Node), With<DuraToggleBtn>>,
    mut logged: Local<bool>,
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
    let open = mgr.is_open(DialogKind::DuraStatus);
    let equipment = loadout_q
        .single()
        .map(|l| l.slots.as_slice())
        .unwrap_or(&[]);
    if open && !*logged {
        for def in &PIECES {
            if let Some(item) = equipment.get(def.server_slot).and_then(|s| s.as_ref()) {
                tracing::info!(
                    "🔧 耐久部位: {:?} idx={} ({}/{})",
                    def.kind,
                    dura_index(item, def.kind),
                    item.current_dura,
                    item.max_dura
                );
            }
        }
        *logged = true;
    } else if !open {
        *logged = false;
    }

    // 面板显隐
    for mut vis in &mut widgets {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }

    // 切换按钮点击（C# Character.Click）
    for (e, inter, mut btn, mut node) in &mut toggle {
        if edge(e, inter, &mut prev_inter) {
            if open {
                mgr.close(DialogKind::DuraStatus);
            } else {
                mgr.open(DialogKind::DuraStatus);
            }
        }
        // 打开时换 2110 图标（image_button_system 按 normal 帧切换）
        if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, if open { 2110 } else { 2113 }) {
            if btn.normal != h {
                btn.normal = h.clone();
            }
        }
        // Y 跟随小地图大/小模式（C# SetBigMode/SetSmallMode 更新 DuraStatusPanel.Location）：
        // 同步命中区（Node.top，y 向下）与视觉位置，二者不可脱节
        let want_y = dura_btn_y(mode.big);
        let cur_y = match node.top {
            Val::Px(v) => v,
            _ => 0.0,
        };
        if (cur_y - want_y).abs() > 0.5 {
            node.top = Val::Px(want_y);
        }
    }

    // 部位：装备存在且耐久>0 显示对应三态，否则隐藏
    for (mut vis, mut node, piece) in &mut pieces {
        let mut show = false;
        if let Some(def) = PIECES.iter().find(|d| d.kind == piece.0) {
            if let Some(item) = equipment
                .get(def.server_slot)
                .and_then(|s| s.as_ref())
            {
                let idx = dura_index(item, piece.0);
                if idx > 0 {
                    if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, idx as usize) {
                        node.image = h;
                        show = true;
                    }
                }
            }
        }
        *vis = if open && show { Visibility::Visible } else { Visibility::Hidden };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(cur: u16, max: u16) -> InvItem {
        InvItem { current_dura: cur, max_dura: max, ..Default::default() }
    }

    #[test]
    fn torch_dura_frames() {
        // 正常 > max/2、警告 <= max/2、危险 <= max/5（C# ItemType.Torch：2146/2147/2148）
        assert_eq!(dura_index(&item(80, 100), DuraPieceKind::Torch), 2146);
        assert_eq!(dura_index(&item(50, 100), DuraPieceKind::Torch), 2147);
        assert_eq!(dura_index(&item(20, 100), DuraPieceKind::Torch), 2148);
        assert_eq!(dura_index(&item(0, 100), DuraPieceKind::Torch), -1);
    }

    #[test]
    fn belt_dura_frames() {
        assert_eq!(dura_index(&item(80, 100), DuraPieceKind::Belt), 2158);
        assert_eq!(dura_index(&item(50, 100), DuraPieceKind::Belt), 2159);
        assert_eq!(dura_index(&item(20, 100), DuraPieceKind::Belt), 2160);
        assert_eq!(dura_index(&item(0, 100), DuraPieceKind::Belt), -1);
    }

    #[test]
    fn stone_shows_broken_frame_only() {
        // C#：Stone 仅在耐久为 0 时显示 2137（破损帧），健康时隐藏
        assert_eq!(dura_index(&item(0, 100), DuraPieceKind::Stone), 2137);
        assert_eq!(dura_index(&item(50, 100), DuraPieceKind::Stone), -1);
    }

    /// 可见性行为护栏：C# DuraStatusDialog 切换钮**恒可见**（不随面板显隐），仅面板/部位随 open 门控。
    /// 真实 spawn + 跑 dura_status_ui_system（默认 DialogManager → DuraStatus 关闭），断言：
    /// 切换钮 Visible（修复前因挂 DuraWidget 被 widgets 查询隐藏 → 本测试变红 = 阳性对照）、面板 bg Hidden。
    #[test]
    fn toggle_always_visible_panel_gated() {
        use crate::resources::libraries::{resolve_data_path, Libraries};
        use bevy::ecs::system::RunSystemOnce;

        // 依赖真实 .Lib 数据：本地无数据（CI/新检出）时跳过，避免假红
        let data_path = resolve_data_path();
        if !data_path.join("Items.Lib").exists() {
            eprintln!("跳过：无本地游戏数据（{}），本测试依赖真实 .Lib 资源", data_path.display());
            return;
        }
        let mut world = World::new();
        world.insert_resource(GameLibraries(Libraries::new(data_path)));
        world.insert_resource(Assets::<Image>::default());
        world.insert_resource(crate::ui::sprite_ui::UiImageCache::default());
        world.insert_resource(DialogManager::default()); // DuraStatus 默认关闭
        world.insert_resource(MiniMapMode::default()); // 默认大模式
        world
            .run_system_once(spawn_dura_status)
            .expect("spawn_dura_status 应成功");
        world
            .run_system_once(dura_status_ui_system)
            .expect("dura_status_ui_system 应成功");

        // 切换钮：面板关闭也应 Visible（C# 恒可见）
        let mut tq = world.query_filtered::<&Visibility, With<DuraToggleBtn>>();
        let toggles: Vec<Visibility> = tq.iter(&world).copied().collect();
        assert_eq!(toggles.len(), 1, "应恰好 1 个切换钮");
        assert_eq!(
            toggles[0],
            Visibility::Visible,
            "切换钮应恒可见（C# DuraStatusDialog 不随面板显隐）"
        );

        // 面板 bg（DuraWidget 且非切换钮/非部位）：关闭时仍 Hidden（门控不变）
        let mut pq = world.query_filtered::<
            &Visibility,
            (With<DuraWidget>, Without<DuraToggleBtn>, Without<DuraPiece>),
        >();
        let panels: Vec<Visibility> = pq.iter(&world).copied().collect();
        assert_eq!(panels.len(), 1, "应恰好 1 个面板 bg");
        assert_eq!(panels[0], Visibility::Hidden, "面板关闭时 bg 应隐藏");
    }
}

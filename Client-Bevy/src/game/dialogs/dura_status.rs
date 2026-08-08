// ============================================================================
// 耐久面板（M55）
// 参考：C# CharacterDuraPanel（Client/MirScenes/Dialogs/MainDialogs.cs）
//   - 切换按钮 DuraStatusDialog：Prguse[2111/2112/2113]（打开时 2110）@ (984,124)
//   - 面板 Prguse[2105]（64x85）@ (963,200)，装备部位按耐久阈值切换
//     Prguse 索引：正常/警告/损坏 三态（2122-2160）
// 纯客户端：数据来自 HudState.equipment（UserInformation 下发，含 current_dura/max_dura）
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::inventory::InvItem;
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::game::hud::HudState;
use crate::map_renderer::GameLibraries;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_sprite, ui_button_system, ui_image, UiButton, UiImageCache,
};

/// 面板位置（C#：ScreenWidth-61=963, y=200；背景图 (3,3) 内布局）
const PANEL_X: f32 = 963.0;
const PANEL_Y: f32 = 200.0;
/// 切换按钮位置（C#：MiniMapDialog.X+86, MiniMapDialog.Height）
const BTN_X: f32 = 984.0;
const BTN_Y: f32 = 124.0;

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
            (dura_status_ui_system, ui_button_system)
                .chain()
                .run_if(in_state(AppState::Game)),
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
    mut cache: ResMut<UiImageCache>,
) {
    libs.0.ensure_initialized();

    // 切换按钮（Prguse 2111 hover / 2112 pressed / 2113 normal）
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse, 2113, 2111, 2112,
        BTN_X, BTN_Y, 6.0, 20.0, 19.0,
    ) {
        commands.entity(e).insert((
            DuraToggleBtn,
            DialogRoot(DialogKind::DuraStatus),
            DuraWidget,
            Visibility::Hidden,
        ));
    }

    // 面板 Prguse[2105] @ (963,200)
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 2105) {
        let e = spawn_ui_sprite(&mut commands, h, PANEL_X, PANEL_Y, 6.0, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::DuraStatus),
            DuraWidget,
            Visibility::Hidden,
        ));
    }

    // 部位精灵（C# Background @ (3,3) 内相对坐标）
    for def in &PIECES {
        if let Some(h) = ui_image(
            &mut libs,
            &mut images,
            &mut cache,
            LibraryName::Prguse,
            def.idx[0],
        ) {
            let e = spawn_ui_sprite(
                &mut commands,
                h,
                PANEL_X + 3.0 + def.rel.0,
                PANEL_Y + 3.0 + def.rel.1,
                6.1,
                1.0,
            );
            commands.entity(e).insert((
                DuraPiece(def.kind),
                DialogRoot(DialogKind::DuraStatus),
                DuraWidget,
                Visibility::Hidden,
            ));
        }
    }
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
    hud: Res<HudState>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<UiImageCache>,
    mut widgets: Query<&mut Visibility, (With<DuraWidget>, Without<DuraPiece>)>,
    mut pieces: Query<(&mut Visibility, &mut Sprite, &DuraPiece)>,
    mut toggle: Query<(&UiButton, &mut crate::ui::sprite_ui::ButtonFrames), With<DuraToggleBtn>>,
    mut logged: Local<bool>,
) {
    let open = mgr.is_open(DialogKind::DuraStatus);
    if open && !*logged {
        for def in &PIECES {
            if let Some(item) = hud.equipment.get(def.server_slot).and_then(|s| s.as_ref()) {
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

    // 面板 + 切换按钮显隐
    for mut vis in &mut widgets {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }

    // 切换按钮点击（C# Character.Click）
    for (btn, mut frames) in &mut toggle {
        if btn.clicked {
            if open {
                mgr.close(DialogKind::DuraStatus);
            } else {
                mgr.open(DialogKind::DuraStatus);
            }
        }
        // 打开时换 2110 图标
        if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, if open { 2110 } else { 2113 }) {
            if frames.normal != h {
                frames.normal = h.clone();
            }
        }
    }

    // 部位：装备存在且耐久>0 显示对应三态，否则隐藏
    for (mut vis, mut sprite, piece) in &mut pieces {
        let mut show = false;
        if let Some(def) = PIECES.iter().find(|d| d.kind == piece.0) {
            if let Some(item) = hud
                .equipment
                .get(def.server_slot)
                .and_then(|s| s.as_ref())
            {
                let idx = dura_index(item, piece.0);
                if idx > 0 {
                    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, idx as usize) {
                        sprite.image = h;
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
}

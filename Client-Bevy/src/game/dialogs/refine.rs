// ============================================================================
// 精炼对话框（M40 → 批10 C# 对齐）
// 参考：C# `RefineDialog`（Client/MirScenes/Dialogs/NPCDialogs.cs:2726-2764）
//   - 面板 `Index = 1002; Library = Prguse`（原生 164x207），`Location = (0, 225)`
//   - 标题精灵 `TitleLabel = Title[18]` @(28,8)
//   - 4x4 材料格：`MirItemCell Size = (34,32)`，
//     `Location = ((x*34)+12+x, (y*32)+37+y)`（GridType = Refine）
// 网络（Rust 自洽；与 C# 同名同序）：
//   C: DepositRefineItem[from 背包格][to 材料槽] / RetrieveRefineItem[from 材料槽][to 背包格]
//   S: 同名确认 [from][to][success] → 本地格子按确认更新（C# GameScene.cs:2753-2784）
// 说明：Rust 服务端 `to = 0` 是武器槽、材料从 1 起算，故格子 i ↔ 材料槽 i+1；
//   C# 的「待精炼武器」放在 NPCDialog 的 ItemCell（PanelType.Refine → Confirm →
//   C.RefineItem），Bevy 侧 NPC 精炼页入口留待下一单元，本单元只落地材料格。
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::inventory::InvClickState;
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::theme::{
    load_lib_image, spawn_icon_button, spawn_image, spawn_item_cell_ui, spawn_panel,
    UiItemCellData, UiItemCellIcon,
};

/// C# `RefineDialog` 面板原生尺寸（Prguse[1002]）
pub const REFINE_W: f32 = 164.0;
pub const REFINE_H: f32 = 207.0;
/// C# `Location = new Point(0, 225)`
const REFINE_POS: (f32, f32) = (0.0, 225.0);
/// C# `TitleLabel = Title[18]` @(28,8)（57x15）
const REFINE_TITLE_POS: (f32, f32) = (28.0, 8.0);
/// C# 4x4 材料格：`Size = (34,32)`、`Location = (x*34+12+x, y*32+37+y)`
const REFINE_GRID_ORIGIN: (f32, f32) = (12.0, 37.0);
pub const REFINE_CELL_W: f32 = 34.0;
pub const REFINE_CELL_H: f32 = 32.0;
const REFINE_GRID_COLS: usize = 4;
/// 材料格数量（C# `Grid = new MirItemCell[4*4]`；服务端 `REFINE_MATERIAL_SLOTS` 同为 16）
pub const REFINE_MATERIAL_SLOTS: usize = REFINE_GRID_COLS * REFINE_GRID_COLS;

/// 格子索引 → 服务端材料槽编号（Rust 端 `to = 0` 是武器槽，材料从 1 起算）
fn refine_material_slot(cell: usize) -> i32 {
    cell as i32 + 1
}

/// C# `Grid[i].Location`
pub fn refine_cell_pos(cell: usize) -> (f32, f32) {
    let x = (cell % REFINE_GRID_COLS) as f32;
    let y = (cell / REFINE_GRID_COLS) as f32;
    (
        REFINE_GRID_ORIGIN.0 + x * (REFINE_CELL_W + 1.0),
        REFINE_GRID_ORIGIN.1 + y * (REFINE_CELL_H + 1.0),
    )
}

/// 已存入的材料（本地镜像；服务端确认包更新）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefineMaterial {
    pub item_index: i32,
    pub image: u16,
    pub count: u16,
}

/// 精炼状态：材料格镜像 + 待确认存入（服务端确认前的占位）
#[derive(Resource)]
pub struct RefineState {
    pub message: String,
    pub materials: [Option<RefineMaterial>; REFINE_MATERIAL_SLOTS],
    pub pending: [Option<RefineMaterial>; REFINE_MATERIAL_SLOTS],
    /// 已请求存入武器（`to=0`）后待发起的精炼 uid（C# Confirm → `C.RefineItem`）
    pub pending_start: Option<u64>,
}

impl Default for RefineState {
    fn default() -> Self {
        Self {
            message: String::new(),
            materials: Default::default(),
            pending: Default::default(),
            pending_start: None,
        }
    }
}

/// 投放武器窗（`PanelType.Refine`）确认后请求「存入武器 + 开始精炼」：
/// C# `C.RefineItem{UniqueID}`；Rust 服务端语义为两步（先 `to=0` 存入，再按 uid 发起）。
#[derive(Message, Debug)]
pub struct RefineWeaponRequest {
    pub unique_id: u64,
    pub inv_slot: usize,
}

#[derive(Component)]
pub struct RefineWidget;

/// 材料格（0..16）
#[derive(Component)]
pub struct RefineCell(pub usize);

pub struct RefinePlugin;

impl Plugin for RefinePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RefineState>();
        app.add_message::<RefineWeaponRequest>();
        app.add_systems(OnEnter(AppState::Game), spawn_refine);
        app.add_systems(OnExit(AppState::Game), cleanup_refine);
        app.add_systems(
            Update,
            (
                refine_weapon_request_system,
                refine_server_events,
                refine_ui_system,
            )
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_refine(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_refine(
    mut commands: Commands,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<crate::ui::sprite_ui::UiFont>,
    mut cjk_font: ResMut<crate::ui::sprite_ui::UiCjkFont>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();
    let _cjk = crate::ui::sprite_ui::shared_cjk_font(&mut fonts, &mut cjk_font);

    // 面板 C# Prguse[1002] @ (0,225)
    let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 1002) else {
        return;
    };
    let panel = spawn_panel(
        &mut commands,
        bg,
        REFINE_POS.0,
        REFINE_POS.1,
        REFINE_W,
        REFINE_H,
        30,
    );
    commands
        .entity(panel)
        .insert((DialogRoot(DialogKind::Refine), RefineWidget));

    commands.entity(panel).with_children(|p| {
        // 标题精灵 C# Title[18] @(28,8)
        if let Some(title) = load_lib_image(&mut libs, &mut images, LibraryName::Title, 18) {
            spawn_image(
                p,
                title,
                REFINE_TITLE_POS.0,
                REFINE_TITLE_POS.1,
                57.0,
                15.0,
                9,
            );
        }
        // 注：C# `RefineDialog` 没有关闭键 —— 它随 NPC 对话窗收起（NPCDialogs.cs:1031
        // `NPCDialog.Hide()` → `RefineDialog.Hide()`），故此处不再放关闭按钮；
        // 收起联动由投放窗（`sell_panel`）在隐藏时关闭本对话框。
        // C# Grid：4x4 材料格（格子 i ↔ 服务端材料槽 i+1）
        for i in 0..REFINE_MATERIAL_SLOTS {
            let (cx, cy) = refine_cell_pos(i);
            spawn_item_cell_ui(
                p,
                &mut images,
                &font,
                cx,
                cy,
                REFINE_CELL_W,
                REFINE_CELL_H,
                9,
                i,
            )
            .insert((RefineCell(i), Button));
        }
    });
}

/// 投放武器窗确认 → 存入武器（`to=0`）并记录待发起的精炼 uid
fn refine_weapon_request_system(
    mut requests: MessageReader<RefineWeaponRequest>,
    mut state: ResMut<RefineState>,
    net: Res<NetConnection>,
) {
    for req in requests.read() {
        net.send_packet(&crate::network::RefineDepositWire {
            from: req.inv_slot as i32,
            to: 0,
        });
        state.pending_start = Some(req.unique_id);
        state.message = "已请求存入武器…".to_string();
        tracing::info!("🔨 存入精炼武器 uid={}", req.unique_id);
    }
}

/// 服务端确认包：更新本地材料格镜像（C# `GameScene.DepositRefineItem/RetrieveRefineItem`）
fn refine_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut state: ResMut<RefineState>,
    net: Res<NetConnection>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        match ev {
            ServerEvent::RefineDeposited { to, success, .. } => {
                // 武器槽（to=0）：成功后若有待发起 uid → 发 `RefineItem`（C# Confirm 语义）
                if *to == 0 {
                    if *success {
                        if let Some(uid) = state.pending_start.take() {
                            net.send_packet(&crate::network::RefineItemWire { unique_id: uid });
                            state.message = "精炼已开始".to_string();
                            tracing::info!("🔨 武器已存入，开始精炼 uid={}", uid);
                        }
                    } else {
                        state.pending_start = None;
                        state.message = "武器存入失败".to_string();
                    }
                    continue;
                }
                let cell = (*to - 1).max(0) as usize;
                if cell < REFINE_MATERIAL_SLOTS {
                    if *success {
                        let pending = state.pending[cell].take();
                        if let Some(item) = pending {
                            state.message = format!("已存入材料 #{}", item.item_index);
                            state.materials[cell] = Some(item);
                        }
                    } else {
                        state.pending[cell] = None;
                        state.message = "存入失败".to_string();
                    }
                }
            }
            ServerEvent::RefineRetrieved { from, success, .. } => {
                let cell = (*from - 1).max(0) as usize;
                if cell < REFINE_MATERIAL_SLOTS && *success {
                    state.materials[cell] = None;
                    state.pending[cell] = None;
                    state.message = "已取回材料".to_string();
                }
            }
            // C# `GameScene.RefineItem/RefineCancel` → `RefineDialog.RefineReset()`
            ServerEvent::RefineStarted { unique_id } => {
                state.materials = Default::default();
                state.pending = Default::default();
                state.pending_start = None;
                state.message = format!("精炼进行中（uid={}）", unique_id);
            }
            ServerEvent::RefineCancelled { .. } => {
                state.materials = Default::default();
                state.pending = Default::default();
                state.pending_start = None;
                state.message = "精炼已取消".to_string();
            }
            _ => {}
        }
    }
}

/// 显隐 + 材料格渲染 + 存入/取回
#[allow(clippy::too_many_arguments)]
fn refine_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut state: ResMut<RefineState>,
    net: Res<NetConnection>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut image_cache: ResMut<crate::ui::sprite_ui::UiImageCache>,
    inv_q: Query<&crate::game::player_state::Inventory, With<crate::actor::LocalPlayer>>,
    mut inv_click: ResMut<InvClickState>,
    mut cells: Query<(&RefineCell, &mut UiItemCellData), Without<UiItemCellIcon>>,
    cell_inter: Query<(Entity, &Interaction, &RefineCell)>,
    mut widgets: Query<&mut Visibility, With<RefineWidget>>,
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
    let open = mgr.is_open(DialogKind::Refine);
    for mut vis in widgets.iter_mut() {
        *vis = if open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !open {
        return;
    }
    let inv_items: Vec<Option<crate::game::dialogs::inventory::InvItem>> = inv_q
        .single()
        .map(|inv| inv.items.clone())
        .unwrap_or_default();
    // 材料格渲染：已存入（或待确认）显示物品图标 + 数量
    for (cell, mut data) in &mut cells {
        let item = state.materials[cell.0]
            .as_ref()
            .or(state.pending[cell.0].as_ref());
        match item {
            Some(m) => {
                data.icon = if m.image > 0 {
                    crate::ui::sprite_ui::ui_image(
                        &mut libs,
                        &mut images,
                        &mut image_cache,
                        LibraryName::Items,
                        m.image as usize,
                    )
                } else {
                    None
                };
                data.count = (m.count > 1).then_some(m.count as u32);
            }
            None => {
                data.icon = None;
                data.count = None;
            }
        }
        data.dura_ratio = None;
    }
    // 格子点击：空 → 存入背包选中物；已存入 → 取回到背包空格
    for (e, inter, cell) in &cell_inter {
        if !edge(e, inter, &mut prev_inter) {
            continue;
        }
        let slot = refine_material_slot(cell.0);
        if state.materials[cell.0].is_some() {
            match inv_items.iter().position(|s| s.is_none()) {
                Some(grid) => {
                    net.send_packet(&crate::network::RefineRetrieveWire {
                        from: slot,
                        to: grid as i32,
                    });
                    state.message = format!("已请求取回（背包格 {}）", grid);
                }
                None => state.message = "背包已满，无法取回".to_string(),
            }
            continue;
        }
        let Some(inv_slot) = inv_click.selected else {
            state.message = "先在背包选中材料，再点材料格".to_string();
            continue;
        };
        let Some(item) = inv_items.get(inv_slot).and_then(|s| s.as_ref()) else {
            continue;
        };
        net.send_packet(&crate::network::RefineDepositWire {
            from: inv_slot as i32,
            to: slot,
        });
        state.pending[cell.0] = Some(RefineMaterial {
            item_index: item.item_index,
            image: item.image,
            count: item.count,
        });
        state.message = format!("已请求存入 {}", item.name);
        inv_click.selected = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #2720：面板与材料格几何对齐 C# `RefineDialog`（NPCDialogs.cs:2732-2760）
    #[test]
    fn refine_layout_matches_csharp_anchors() {
        assert_eq!((REFINE_W, REFINE_H), (164.0, 207.0));
        assert_eq!(REFINE_POS, (0.0, 225.0));
        assert_eq!(REFINE_TITLE_POS, (28.0, 8.0));
        assert_eq!((REFINE_CELL_W, REFINE_CELL_H), (34.0, 32.0));
        assert_eq!(REFINE_MATERIAL_SLOTS, 16);
        assert_eq!(refine_cell_pos(0), (12.0, 37.0));
        assert_eq!(refine_cell_pos(3), (117.0, 37.0)); // 12 + 3*35
        assert_eq!(refine_cell_pos(4), (12.0, 70.0)); // 37 + 1*33
        assert_eq!(refine_cell_pos(15), (117.0, 136.0));
    }

    /// #2720：格子 ↔ 服务端材料槽映射（Rust `to=0` 为武器槽，材料 1..=16）
    #[test]
    fn refine_material_slot_mapping() {
        assert_eq!(refine_material_slot(0), 1);
        assert_eq!(refine_material_slot(15), 16);
    }
}

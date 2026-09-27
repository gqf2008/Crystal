// ============================================================================
// NPC 觉醒对话框（M54）
// 参考：C# NPCAwakeDialog（Client/MirScenes/Dialogs/NPCDialogs.cs）
//   - 面板 Title[710]（360x420）位于 (0,0)
//   - 升级按钮 Title[712/713/714] (115,391)；关闭 Prguse2[360/361/362] (284,4)
//   - 主物品格 (202,91)、材料标签 (67,317)/(192,317)、结果 (112,354)
//   - 觉醒类型选择（武器：攻/魔/道）
// 网络：AwakeningNeedMaterials → 材料需求；Awakening → 觉醒结果（服务端全链路已支持）
//
// #3264：C# 一共 7 个 `MirItemCell`（`GridType = AwakenItem`，`NPCDialogs.cs:1935-2008`）：
//   [0] 主物品 @(202,91)（可放）；[1]/[2] 只读**需求材料格** @(31,316)/(155,316)（`Enabled = false`，
//   由 `setNeedItems` 按服务端包填图 + `NeedItemLabel1/2` @(67,317)/(192,317) 写「需要 x×n」）；
//   [3..6] @(175,199)/(230,199)/(175,256)/(230,256) 是**玩家手动放置**的材料格
//   （C# `CheckNeedMaterials` 按名字比对 [1]/[2] 与 [3..6]）。
// 本端实现 [0] + [1] + [2]（含图标/边框/文案，实机抽点已核）。
// **[3..6] 不实现，并说明理由（不是漏项）**：本端 Rust 服务端结算觉醒材料时**按背包逐索引计数并消耗**
//   （`ServerRust/src/actors/world/awakening.rs` 的 `CountItemsByIndex` / `ConsumeItemsByIndex`），
//   且全仓**没有 `MirGridType::AwakenItem` 的服务端处理臂**（只有枚举定义）⇒ 画 4 个"能放东西"的格子
//   会是假交互。要与 C# 完全一致，需要先在服务端加「觉醒材料格状态 + `MoveItem{AwakenItem}` 臂 + 结算改造」，
//   属独立一轮（产品是否需要这个交互要先确认）。
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::inventory::InvItem;
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{shared_cjk_font, UiCjkFont, UiFont};
use crate::ui::theme::{
    load_lib_image, spawn_container, spawn_dropdown_ui, spawn_icon_button, spawn_image,
    spawn_label, spawn_panel, CloseButton, UiDropDown,
};

/// #2892 批B：面板精灵与 C# 原生尺寸（C# `NPCAwakeDialog.Index = 710; Library = Libraries.Title`）
pub const PANEL: (LibraryName, usize) = (LibraryName::Title, 710);
pub const PANEL_SIZE: (f32, f32) = (360.0, 420.0);
/// 主物品格尺寸 = C# `MirItemCell` 構造子里的默认 `Size = new Size(36, 32)`
/// （`Client/MirControls/MirItemCell.cs:184-186`）。本端此前画 36x28：不仅命中区比原版矮 4px，
/// `npc_awake_render_system` 还会把物品图**拉伸**到节点尺寸 ⇒ 图标纵向被压扁。
pub const MAIN_CELL_SIZE: (f32, f32) = (36.0, 32.0);
/// C# `ItemCells[1]`/`[2]`（"Required" 只读材料格）@(31,316)/(155,316)，
/// `BorderColour = Color.Lime`、`Enabled = false`（`NPCDialogs.cs:1947-1968`）。
pub const NEED_CELL_POS: [(f32, f32); 2] = [(31.0, 316.0), (155.0, 316.0)];
/// 对应的两行需求文案（C# `NeedItemLabel1/2` @(67,317)/(192,317)，`NPCDialogs.cs:1876-1895`）
pub const NEED_LABEL_POS: [(f32, f32); 2] = [(67.0, 317.0), (192.0, 317.0)];
/// C# `BorderColour = Color.Lime`（只读材料格边框）
pub const NEED_CELL_BORDER: Color = Color::srgb(0.0, 1.0, 0.0);

/// C# `ItemCells[3..6]` 四个**可放置格** @(175,199)/(230,199)/(175,256)/(230,256)
/// （`Client/MirScenes/Dialogs/NPCDialogs.cs:1989-2008`，`Enabled` 且 `GridType = MirGridType.AwakenItem`）。
///
/// 这 4 格的「内容」在原版里是对话框自己的客户端数组（`MirItemCell.cs:88`
/// `case MirGridType.AwakenItem: return NPCAwakeDialog.Items;`），而原版**服务端完全不处理**
/// `MirGridType.AwakenItem`（`rg -c AwakenItem Server --glob '*.cs'` 零命中）⇒ 本端也只做客户端态，
/// **不自造**服务端格状态 / `MoveItem{AwakenItem}` 臂。
pub const PLACE_CELL_POS: [(f32, f32); 4] = [
    (175.0, 199.0),
    (230.0, 199.0),
    (175.0, 256.0),
    (230.0, 256.0),
];

/// 觉醒物品当前可用的 `AwakeType` 上限（`Shape` 档位）：`NPCDialogs.cs` 只把 type 下拉给到这几档；
/// 这里只用于放置规则的两档 `Shape` 判定（`< 200` 普通材料 / `== 200` 现金材料）。
pub const AWAKEN_MATERIAL_SHAPE_CASH: i16 = 200;

/// **物品类型码用 Rust 枚举值**（InvItem.item_type 是服务端把 DB 的 C# ItemType 映射过来的
/// mir2_shared::enums::ItemType 值：Weapon=4 / Armour=5 / Helmet=7 / Awakening=38）。
///
/// **踩坑记录（2026-09-27，两次）**：先前先写成字面量 20/21/22/113（既非 C# 也非 Rust）⇒ 全被拒；
/// 再按 DB 里的 C# 值 1/2/4/35 改 ⇒ **仍被拒**（实机日志：『觉醒格 3 拒绝放入 AwakeningSoul0』）。
/// 真值只能从**实机 wire**看：本次实测 AwakeningSoul0 的 item_type 走的是 **Rust 值 38**
///（DB 里是 35，服务端映射 +3；与 ItemGrade 的 None=3 口径同源）。
/// ⇒ 结论：本端规则里引用 item_type 一律用 **mir2_shared::enums::ItemType** 的 Rust 值，
/// 别再拿 DB/C# 值去比。
pub const ITEM_TYPE_WEAPON_RS: u8 = mir2_shared::enums::ItemType::Weapon as u8;
pub const ITEM_TYPE_ARMOUR_RS: u8 = mir2_shared::enums::ItemType::Armour as u8;
pub const ITEM_TYPE_HELMET_RS: u8 = mir2_shared::enums::ItemType::Helmet as u8;
pub const ITEM_TYPE_AWAKENING_RS: u8 = mir2_shared::enums::ItemType::Awakening as u8;
/// ItemGrade::None（同样是 Rust 编号口径）
pub const ITEM_GRADE_NONE_RS: u8 = 3;

/// 放置规则（C# `MirItemCell.cs:1655-1785` `#region To Awakening` 的纯函数版，门禁钉它）：
///
/// * `slot == 0`：底材——只吃 武器/头盔/铠甲（`ItemType` 20/21/22）且 `grade` 非 `None`（本端 `3`）；
/// * `slot == 1 || slot == 2`：只读展示格（`Enabled = false`）⇒ 一律拒绝；
/// * `slot == 3 || slot == 4`：材料——`item_type == Awakening(113)` 且 `shape < 200`；
/// * `slot == 5 || slot == 6`：材料——`item_type == Awakening(113)` 且 `shape == 200`
///   （C# 原文注释 `//AllCashItem Korea Server Not Implementation.`）；
/// * 所有格都要求**目标格为空**（C# `ItemsIdx[_itemSlot] == 0`；本端用 `Option` 表达"空"，
///   有意偏离见 [NpcAwakeState::place_src] 的注释）。
///
/// 返回 `Ok(())` 或拒绝原因（仅用于日志——C# 的 `case -2` 提示框是**被注释掉的**，原版拒绝时静默）。
pub fn awake_place_accepts(
    slot: usize,
    item: &InvItem,
    cell_empty: bool,
) -> Result<(), &'static str> {
    if slot > 6 {
        return Err("槽位越界");
    }
    if !cell_empty {
        return Err("目标格已有物品");
    }
    match slot {
        0 => {
            // C# `ItemType.Weapon = 20 / Helmet = 21 / Armour = 22`、`ItemGrade.None = 3`（本端 +3 口径）
            const BASE_TYPES: [u8; 3] = [
                ITEM_TYPE_WEAPON_RS,
                ITEM_TYPE_ARMOUR_RS,
                ITEM_TYPE_HELMET_RS,
            ];
            if BASE_TYPES.contains(&item.item_type) && item.grade != ITEM_GRADE_NONE_RS {
                Ok(())
            } else {
                Err("底材只收 武器/头盔/铠甲 且有品质")
            }
        }
        1 | 2 => Err("只读展示格"),
        3 | 4 => {
            if item.item_type == ITEM_TYPE_AWAKENING_RS && item.shape < AWAKEN_MATERIAL_SHAPE_CASH {
                Ok(())
            } else {
                Err("材料格只收 Awakening 且 Shape < 200")
            }
        }
        _ => {
            if item.item_type == ITEM_TYPE_AWAKENING_RS && item.shape == AWAKEN_MATERIAL_SHAPE_CASH
            {
                Ok(())
            } else {
                Err("现金材料格只收 Awakening 且 Shape == 200")
            }
        }
    }
}

/// C# `setNeedItems`（`NPCDialogs.cs:2165-2193`）的需求文案：
/// `MaterialsCount[i] != 0` 才画格 + 写 `NeedItemQuantity` 文案（否则清空）。
pub fn need_item_text(name: &str, count: i32) -> String {
    if count == 0 {
        return String::new();
    }
    format!("需要 {} ×{}", name, count)
}

/// #1356：觉醒面板服务模式（C# PanelType：Awakening/Disassemble/Downgrade/Reset）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NpcAwakeService {
    #[default]
    Awaken,
    Disassemble,
    Downgrade,
    Reset,
}

impl NpcAwakeService {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Awaken => "觉醒",
            Self::Disassemble => "分解",
            Self::Downgrade => "降级",
            Self::Reset => "重置",
        }
    }
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Disassemble,
            2 => Self::Downgrade,
            3 => Self::Reset,
            _ => Self::Awaken,
        }
    }
}

/// 材料需求行
#[derive(Clone, Default)]
pub struct MaterialRow {
    pub item_id: i32,
    pub count: i32,
}

/// 觉醒状态
#[derive(Resource, Default)]
pub struct NpcAwakeState {
    /// 主物品 unique_id
    pub selected_uid: Option<u64>,
    /// 主物品（显示用）
    pub selected_item: Option<InvItem>,
    /// 觉醒类型（mir2_shared AwakeType；None=3）
    pub awake_type: Option<mir2_shared::enums::AwakeType>,
    /// 服务端返回的材料需求
    pub materials: Vec<MaterialRow>,
    /// 最近觉醒结果（1=成功 0=销毁 -1=失败 -2=满级 -3=金币不足 -4=材料不足）
    pub result: i32,
    pub result_text: String,
    /// #1356：当前服务模式（觉醒/分解/降级/重置）
    pub service: NpcAwakeService,
    /// C# `NPCAwakeDialog.Items`：格 3..6 这 4 个**可放置格**里的物品（纯客户端态）。
    pub place_items: [Option<InvItem>; 4],
    /// C# `NPCAwakeDialog.ItemsIdx`：每格物品的**来源背包格号**，`None` = 空。
    ///
    /// **有意偏离（已记录）**：C# 用 `0` 当"空"哨兵（`ItemsIdx[slot] == 0`），于是
    /// **背包 0 号格的物品放不进去**（放进去也会被当成空）。本端用 `Option<usize>` 避开这个冲突——
    /// 对玩家更合理，且不影响与原版的可见行为（除这个原版自身的 quirk）。
    pub place_src: [Option<usize>; 4],
}

#[derive(Component)]
pub struct NpcAwakeWidget;

#[derive(Component)]
pub struct NpcAwakeClose;

#[derive(Component)]
pub struct NpcAwakeUpgrade;

/// #1356：服务模式按钮（C# PanelType）
#[derive(Component)]
pub struct NpcAwakeServiceBtn(u8);

/// #1356：操作按钮文字（觉醒/分解/降级/重置）
#[derive(Component)]
pub struct NpcAwakeActionLabel;

#[derive(Component)]
pub struct NpcAwakeTypeDrop;

#[derive(Component)]
pub struct NpcAwakeMainIcon;

#[derive(Component)]
pub struct NpcAwakeMainName;

#[derive(Component)]
pub struct NpcAwakeMaterialText(pub usize);

/// #3264：只读材料格（C# `ItemCells[1]/[2]`，`Enabled = false`）与其图标层
#[derive(Component)]
pub struct NpcAwakeNeedCell(pub usize);
#[derive(Component)]
pub struct NpcAwakeNeedIcon(pub usize);

/// 格 3..6 的可放置格背景（点击命中用，C# `ItemCells[3..6]`）
#[derive(Component)]
pub struct NpcAwakePlaceCell(pub usize);
/// 格 3..6 的物品图标层
#[derive(Component)]
pub struct NpcAwakePlaceIcon(pub usize);

#[derive(Component)]
pub struct NpcAwakeResultText;

pub struct NpcAwakePlugin;

impl Plugin for NpcAwakePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiCjkFont>();
        app.init_resource::<NpcAwakeState>();
        app.add_systems(OnEnter(AppState::Game), spawn_npc_awake);
        app.add_systems(OnExit(AppState::Game), cleanup_npc_awake);
        app.add_systems(Update, awake_server_events.run_if(in_state(AppState::Game)));
        app.add_systems(
            Update,
            (
                npc_awake_ui_system,
                npc_awake_place_system,
                npc_awake_lock_sync,
                npc_awake_render_system,
            )
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

/// 格 3..6 的点击交互（C# `MirItemCell.cs:1655-1785` To Awakening / `:1164-1178` From AwakenItem）：
/// 先点背包选中（`InvClickState.selected`）再点目标格放入；点已有物品的格 = 取出
/// （发 `C.MoveItem{Grid=AwakenItem, From=To=来源背包格号}` 并立即清本地态，与原版同包同参）。
///
/// 单独成一个系统而不是塞进 `npc_awake_ui_system`：那个系统的参数**已经在 16 个上限**上
/// （既有 LESSON：函数式系统参数上限 16，超了连 `.chain()` 都不成立）。
#[allow(clippy::too_many_arguments)]
fn npc_awake_place_system(
    mgr: Res<crate::game::dialogs::DialogManager>,
    mut state: ResMut<NpcAwakeState>,
    net: Res<NetConnection>,
    inv_q: Query<&crate::game::player_state::Inventory, With<crate::actor::LocalPlayer>>,
    mut inv_click: ResMut<crate::game::dialogs::inventory::InvClickState>,
    mouse: Res<ButtonInput<MouseButton>>,
    ui: (Query<&Window>, Query<&Node, With<NpcAwakeWidget>>),
) {
    if !mgr.is_open(crate::game::dialogs::DialogKind::NpcAwake) {
        return;
    }
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Ok(window) = ui.0.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let (ox, oy) =
        ui.1.single()
            .map(|n| crate::ui::theme::node_origin(n, (0.0, 0.0)))
            .unwrap_or((0.0, 0.0));
    let hit = PLACE_CELL_POS.iter().position(|(cx, cy)| {
        cursor.x >= ox + cx
            && cursor.x <= ox + cx + MAIN_CELL_SIZE.0
            && cursor.y >= oy + cy
            && cursor.y <= oy + cy + MAIN_CELL_SIZE.1
    });
    let Some(i) = hit else { return };
    // 已有物品 → 取出
    if let Some(item) = state.place_items.get(i).and_then(|s| s.as_ref()).cloned() {
        if let Some(src) = state.place_src.get(i).and_then(|s| *s) {
            net.send_packet(&mir2_shared::packets::client::item::MoveItem {
                grid: mir2_shared::enums::MirGridType::AwakenItem,
                from: src as i32,
                to: src as i32,
            });
            tracing::info!(
                "⚒️ 取出觉醒格 {} 的物品 {}（C.MoveItem grid=AwakenItem from/to={}）",
                i + 3,
                item.name,
                src
            );
        }
        state.place_items[i] = None;
        state.place_src[i] = None;
        return;
    }
    // 空格 → 用背包选中格放入（规则见 `awake_place_accepts`）
    let Some(bag_slot) = inv_click.take_selected() else {
        return;
    };
    let taken = inv_q
        .single()
        .ok()
        .and_then(|inv| inv.items.get(bag_slot).and_then(|s| s.clone()));
    let Some(item) = taken else { return };
    match awake_place_accepts(i + 3, &item, true) {
        Ok(()) => {
            state.place_items[i] = Some(item.clone());
            state.place_src[i] = Some(bag_slot);
            tracing::info!(
                "⚒️ 放入觉醒格 {}：{} (uid={}, 来源背包格 {})",
                i + 3,
                item.name,
                item.unique_id,
                bag_slot
            );
        }
        Err(why) => {
            // C# `case -2:` 的 MessageBox 是被注释掉的（原版拒绝时静默）⇒ 本端只记日志，不自造提示
            tracing::info!("⚒️ 觉醒格 {} 拒绝放入 {}：{}", i + 3, item.name, why);
        }
    }
}

/// 同步「觉醒格来源格」的锁（C# `MirItemCell.cs:1677/1702`：放进觉醒格时 `SelectedCell.Locked = true`）。
///
/// 单独成一个系统而不是塞进 `npc_awake_ui_system`：那个系统的系统参数已经贴到上限，
/// 而且「锁」是**状态的投影**——每帧幂等重建最省心（与 `craft` 的 `sync_craft_locks` 同款）：
/// 窗开着就按 `place_src` 上锁、其余 `Awaken` 锁清掉；窗关了全清。
fn npc_awake_lock_sync(
    mgr: Res<crate::game::dialogs::DialogManager>,
    state: Res<NpcAwakeState>,
    mut locked: ResMut<crate::game::dialogs::inventory::InvLockedSlots>,
) {
    use crate::game::dialogs::inventory::InvLockReason;
    locked.unlock_all(InvLockReason::Awaken);
    if !mgr.is_open(crate::game::dialogs::DialogKind::NpcAwake) {
        return;
    }
    for slot in state.place_src.iter().flatten().copied() {
        locked.lock(InvLockReason::Awaken, slot);
    }
}

fn cleanup_npc_awake(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_npc_awake(
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
    let font = ui_font.0.clone();
    let cjk = shared_cjk_font(&mut fonts, &mut cjk_font);

    // 面板 Title[710]（360x420）C# Location (0,0)
    let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Title, 710) else {
        return;
    };
    let panel = spawn_panel(&mut commands, bg, 0.0, 0.0, PANEL_SIZE.0, PANEL_SIZE.1, 30);
    commands
        .entity(panel)
        .insert((DialogRoot(DialogKind::NpcAwake), NpcAwakeWidget));

    commands.entity(panel).with_children(|p| {
        // 关闭 Prguse2[360/361/362]（C# (284,4)）
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 360),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 361),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 362),
        ) {
            spawn_icon_button(p, n, h, pr, 284.0, 4.0, 24.0, 21.0, 10)
                .insert((NpcAwakeClose, CloseButton));
        }
        // 升级按钮 Title[712/713/714]（C# (115,391)）
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 712),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 713),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 714),
        ) {
            spawn_icon_button(p, n, h, pr, 115.0, 391.0, 80.0, 25.0, 10).insert((
                NpcAwakeUpgrade,
                // #93 通用 Tooltip：C# 升级按钮 Hint
                crate::ui::tooltip::TooltipHint("消耗材料执行觉醒".to_string()),
            ));
        }
        // 觉醒类型下拉（C# SelectAwakeType (35,141)）
        spawn_dropdown_ui(
            p,
            &cjk,
            vec!["攻".to_string(), "魔".to_string(), "道".to_string()],
            None,
            (0.0, 0.0),
            35.0,
            141.0,
            72.0,
            18.0,
            3,
            9,
        )
        .insert(NpcAwakeTypeDrop);
        // #1356：服务模式按钮（C# PanelType：觉醒/分解/降级/重置）@(30+72i,26)
        for (i, label) in ["觉醒", "分解", "降级", "重置"].iter().enumerate() {
            spawn_container(p, 30.0 + i as f32 * 72.0, 26.0, 64.0, 20.0, 9)
                .insert((
                    Button,
                    NpcAwakeServiceBtn(i as u8),
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
                ))
                .with_children(|b| {
                    b.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(2.0),
                            top: Val::Px(4.0),
                            ..default()
                        },
                        Text::new(*label),
                        TextFont {
                            font: FontSource::Handle(cjk.clone()),
                            font_size: FontSize::Px(12.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        ZIndex(1),
                    ));
                });
        }
        // #1356：操作按钮文字（升级按钮下方）
        spawn_label(p, &cjk, "觉醒", 118.0, 396.0, 12.0, Color::WHITE, 10)
            .insert(NpcAwakeActionLabel);
        // 主物品格（C# (202,91)）：图标 + 名字（白图占位，render 系统换物品图）
        let empty = images.add(crate::map_renderer::make_image(
            vec![255, 255, 255, 255],
            1,
            1,
        ));
        spawn_image(p, empty, 202.0, 91.0, MAIN_CELL_SIZE.0, MAIN_CELL_SIZE.1, 9)
            .insert(NpcAwakeMainIcon);
        spawn_label(p, &cjk, "", 202.0, 122.0, 11.0, Color::WHITE, 9).insert(NpcAwakeMainName);
        // 只读材料格 + 需求文案（C# `ItemCells[1]/[2]` @(31,316)/(155,316) 36x32、
        // `NeedItemLabel1/2` @(67,317)/(192,317)，`NPCDialogs.cs:1947-1968` / `:2165-2193`）
        for (i, (cx, cy)) in NEED_CELL_POS.iter().enumerate() {
            let cell_bg = images.add(crate::map_renderer::make_image(
                vec![255, 255, 255, 255],
                1,
                1,
            ));
            // 注意：**不能**先 `spawn_container` 再 `insert(Node{..default()})` —— 那会把容器
            // 自己的 `position_type/left/top/width/height` 覆盖成默认值（实机抽点表现为"格子不存在"，
            // 只命中到面板）。这里直接按需要的样式建节点。
            p.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(*cx),
                    top: Val::Px(*cy),
                    width: Val::Px(MAIN_CELL_SIZE.0),
                    height: Val::Px(MAIN_CELL_SIZE.1),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
                BorderColor::all(NEED_CELL_BORDER),
                ZIndex(9),
                NpcAwakeNeedCell(i),
            ))
            .with_children(|c| {
                c.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(1.0),
                        top: Val::Px(1.0),
                        width: Val::Px(MAIN_CELL_SIZE.0 - 2.0),
                        height: Val::Px(MAIN_CELL_SIZE.1 - 2.0),
                        ..default()
                    },
                    ImageNode::new(cell_bg),
                    ZIndex(1),
                    NpcAwakeNeedIcon(i),
                ));
            });
            let (lx, ly) = NEED_LABEL_POS[i];
            spawn_label(p, &cjk, "", lx, ly, 11.0, Color::WHITE, 9).insert(NpcAwakeMaterialText(i));
        }
        // 4 个**可放置格**（C# `ItemCells[3..6]` @(175,199)/(230,199)/(175,256)/(230,256)，
        // `NPCDialogs.cs:1989-2008`）。与只读格同款写法：直接按需要的样式建节点，
        // **不能**先 `spawn_container` 再 `insert(Node{..default()})`（会覆盖定位/尺寸）。
        for (i, (cx, cy)) in PLACE_CELL_POS.iter().enumerate() {
            let cell_bg = images.add(crate::map_renderer::make_image(
                vec![255, 255, 255, 255],
                1,
                1,
            ));
            p.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(*cx),
                    top: Val::Px(*cy),
                    width: Val::Px(MAIN_CELL_SIZE.0),
                    height: Val::Px(MAIN_CELL_SIZE.1),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
                BorderColor::all(NEED_CELL_BORDER),
                ZIndex(9),
                NpcAwakePlaceCell(i),
            ))
            .with_children(|c| {
                c.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(1.0),
                        top: Val::Px(1.0),
                        width: Val::Px(MAIN_CELL_SIZE.0 - 2.0),
                        height: Val::Px(MAIN_CELL_SIZE.1 - 2.0),
                        ..default()
                    },
                    ImageNode::new(cell_bg),
                    ZIndex(1),
                    Visibility::Hidden,
                    NpcAwakePlaceIcon(i),
                ));
            });
        }
        // 结果标签（C# GoldLabel (112,354)）
        spawn_label(
            p,
            &cjk,
            "",
            112.0,
            354.0,
            11.0,
            Color::srgb(1.0, 0.9, 0.1),
            9,
        )
        .insert(NpcAwakeResultText);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 门禁（金标准 ⑧ 长尾窗 / 2026-09-27）：主物品格必须是 C# `MirItemCell` 的默认尺寸。
    /// 阳性对照：把 `MAIN_CELL_SIZE` 改回 `(36.0, 28.0)`（修复前的值）⇒ 本测试 FAILED。
    #[test]
    fn main_item_cell_matches_csharp_mir_item_cell_default() {
        assert_eq!(
            PANEL,
            (LibraryName::Title, 710),
            "C# NPCAwakeDialog.Index = 710"
        );
        assert_eq!(PANEL_SIZE, (360.0, 420.0), "Title[710] 图头 360x420");
        assert_eq!(
            MAIN_CELL_SIZE,
            (36.0, 32.0),
            "C# MirItemCell 默认 Size = (36, 32)（MirItemCell.cs:184-186）"
        );
    }

    /// 门禁（#3264）：两个**只读材料格**必须落在 C# `ItemCells[1]/[2]` 的坐标上，
    /// 且文案规则与 C# `setNeedItems`（`NPCDialogs.cs:2165-2193`）一致：`count == 0` 不写文案。
    /// 阳性对照：把 `NEED_CELL_POS` 改回「只有主格」的旧值（例如 (0,0)）⇒ 坐标断言即红。
    #[test]
    fn need_material_cells_match_csharp() {
        assert_eq!(
            NEED_CELL_POS,
            [(31.0, 316.0), (155.0, 316.0)],
            "C# ItemCells[1]/[2] @(31,316)/(155,316)"
        );
        assert_eq!(
            NEED_LABEL_POS,
            [(67.0, 317.0), (192.0, 317.0)],
            "C# NeedItemLabel1/2 @(67,317)/(192,317)"
        );
        assert_eq!(
            NEED_CELL_BORDER,
            Color::srgb(0.0, 1.0, 0.0),
            "C# BorderColour = Color.Lime"
        );
        // `count == 0` → 空文案（C# `if (MaterialsCount[i] != 0) … else NeedItemLabel.Text = ""`）
        assert_eq!(need_item_text("勇气印记", 0), "");
        assert_eq!(need_item_text("勇气印记", 3), "需要 勇气印记 ×3");
    }

    /// 门禁（2026-09-27）：格 3..6 这 4 个**可放置格**必须落在 C# `ItemCells[3..6]` 的坐标上。
    /// 阳性对照：把 `PLACE_CELL_POS` 任一项改掉（例如 (0,0)）⇒ 坐标断言即红。
    #[test]
    fn place_cells_match_csharp() {
        assert_eq!(
            PLACE_CELL_POS,
            [
                (175.0, 199.0),
                (230.0, 199.0),
                (175.0, 256.0),
                (230.0, 256.0)
            ],
            "C# ItemCells[3..6] @(175,199)/(230,199)/(175,256)/(230,256)（NPCDialogs.cs:1989-2008）"
        );
    }

    /// 门禁（2026-09-27）：放置规则与 C# `MirItemCell.cs:1655-1785` 的 `#region To Awakening` 一致。
    ///
    /// 阳性对照（落地时实做）：把格 3/4 的 `shape < 200` 改成 `shape > 200` ⇒ 第 2、3 条断言立即红。
    #[test]
    fn awake_place_rules_match_csharp() {
        const AW: u8 = ITEM_TYPE_AWAKENING_RS;
        const WEAPON: u8 = ITEM_TYPE_WEAPON_RS;
        let mk = |item_type: u8, shape: i16, grade: u8| InvItem {
            item_type,
            shape,
            grade,
            name: "x".into(),
            ..Default::default()
        };
        // 格 3/4：材料形状 < 200
        assert!(awake_place_accepts(3, &mk(AW, 100, 3), true).is_ok());
        assert!(awake_place_accepts(4, &mk(AW, 100, 3), true).is_ok());
        assert!(
            awake_place_accepts(3, &mk(AW, 200, 3), true).is_err(),
            "shape==200 归格 5/6"
        );
        assert!(
            awake_place_accepts(3, &mk(WEAPON, 100, 3), true).is_err(),
            "非 Awakening 类型不收"
        );
        // 格 5/6：材料形状 == 200（C# 注释 //AllCashItem）
        assert!(awake_place_accepts(5, &mk(AW, 200, 3), true).is_ok());
        assert!(awake_place_accepts(6, &mk(AW, 200, 3), true).is_ok());
        assert!(awake_place_accepts(5, &mk(AW, 100, 3), true).is_err());
        // 目标格必须为空
        assert!(awake_place_accepts(3, &mk(AW, 100, 3), false).is_err());
        // 格 0：底材（武器/头盔/铠甲 + 有品质）；1/2 只读
        assert!(
            awake_place_accepts(0, &mk(WEAPON, 0, 4), true).is_ok(),
            "武器+品质"
        );
        assert!(
            awake_place_accepts(0, &mk(WEAPON, 0, 3), true).is_err(),
            "Grade=None 不行"
        );
        assert!(
            awake_place_accepts(0, &mk(AW, 100, 4), true).is_err(),
            "材料不是底材"
        );
        assert!(awake_place_accepts(1, &mk(AW, 100, 4), true).is_err());
        assert!(awake_place_accepts(2, &mk(AW, 100, 4), true).is_err());
    }
}

#[allow(clippy::too_many_arguments)]
fn npc_awake_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut state: ResMut<NpcAwakeState>,
    net: ResMut<NetConnection>,
    inv_q: Query<&crate::game::player_state::Inventory, With<crate::actor::LocalPlayer>>,
    close: Query<(Entity, &Interaction), With<NpcAwakeClose>>,
    upgrade: Query<(Entity, &Interaction), With<NpcAwakeUpgrade>>,
    mut type_dd: Query<(&mut UiDropDown, &NpcAwakeTypeDrop)>,
    // B0001 互斥：widgets 与 action/type_vis/mat_vis 同写 Visibility——
    // widgets 侧补三对 Without（实体标记互斥，spawn 处各只挂自己的标记）
    mut widgets: Query<
        &mut Visibility,
        (
            With<NpcAwakeWidget>,
            Without<NpcAwakeActionLabel>,
            Without<NpcAwakeTypeDrop>,
            Without<NpcAwakeMaterialText>,
        ),
    >,
    service_btns: Query<(Entity, &Interaction, &NpcAwakeServiceBtn)>,
    mut action: Query<(&mut Text, &mut Visibility), With<NpcAwakeActionLabel>>,
    mut type_vis: Query<&mut Visibility, (With<NpcAwakeTypeDrop>, Without<NpcAwakeActionLabel>)>,
    mut mat_vis: Query<
        &mut Visibility,
        (
            With<NpcAwakeMaterialText>,
            Without<NpcAwakeTypeDrop>,
            Without<NpcAwakeActionLabel>,
        ),
    >,
    mouse: Res<ButtonInput<MouseButton>>,
    ui: (Query<&Window>, Query<&Node, With<NpcAwakeWidget>>),
    mut last_uid: Local<Option<u64>>,
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
    use mir2_shared::packets::client::misc::{Awakening, AwakeningNeedMaterials};

    let open = mgr.is_open(DialogKind::NpcAwake);
    for mut vis in widgets.iter_mut() {
        *vis = if open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !open {
        // 关闭时也要隐藏下拉/材料/操作文字：它们不在 widgets 查询里，
        // 否则下拉框等会残留成屏幕上的孤按钮（觉醒类型下拉默认 Visible）
        for mut vis in &mut type_vis {
            *vis = Visibility::Hidden;
        }
        for mut vis in &mut mat_vis {
            *vis = Visibility::Hidden;
        }
        for (_, mut vis) in &mut action {
            *vis = Visibility::Hidden;
        }
        return;
    }
    // #1356：操作按钮文字 + 类型/材料显隐（非觉醒模式隐藏）
    for (mut text, mut vis) in &mut action {
        text.0 = state.service.label().to_string();
        *vis = Visibility::Visible;
    }
    for mut vis in &mut type_vis {
        *vis = if state.service == NpcAwakeService::Awaken {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for mut vis in &mut mat_vis {
        *vis = if state.service == NpcAwakeService::Awaken {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    // #1356：服务模式切换（C# PanelType）
    for (e, inter, svc) in &service_btns {
        if edge(e, inter, &mut prev_inter) {
            let new_svc = NpcAwakeService::from_u8(svc.0);
            if new_svc != state.service {
                state.service = new_svc;
                state.selected_uid = None;
                state.selected_item = None;
                state.awake_type = None;
                state.materials.clear();
                state.result_text = String::new();
            }
        }
    }

    for (e, inter) in &close {
        if edge(e, inter, &mut prev_inter) {
            mgr.close(DialogKind::NpcAwake);
        }
    }

    // 觉醒类型下拉（#90 通用 DropDown）：选中变化 → AwakeningNeedMaterials
    const TYPES: [mir2_shared::enums::AwakeType; 3] = [
        mir2_shared::enums::AwakeType::Dc,
        mir2_shared::enums::AwakeType::Mc,
        mir2_shared::enums::AwakeType::Sc,
    ];
    if let Ok((mut dd, _)) = type_dd.single_mut() {
        // 非觉醒模式：收起下拉（防止弹出面板残留）
        if state.service != NpcAwakeService::Awaken {
            dd.open = false;
        }
        // 换了主物品 → 清空类型选择
        if *last_uid != state.selected_uid {
            *last_uid = state.selected_uid;
            dd.selected = None;
            state.awake_type = None;
        }
        let new_type = dd.selected.and_then(|i| TYPES.get(i).copied());
        if new_type != state.awake_type {
            if let (Some(uid), Some(t)) = (state.selected_uid, new_type) {
                state.awake_type = Some(t);
                net.send_packet(&AwakeningNeedMaterials {
                    unique_id: uid,
                    awake_type: t,
                });
                tracing::info!("⚒️ 选择觉醒类型 {:?}，请求材料 uid={}", t, uid);
                state.result_text = String::new();
            } else {
                state.awake_type = None;
            }
        }
    }

    // 主物品格点击：循环选择背包武器（C# 从背包拖入）
    if let Ok(window) = ui.0.single() {
        if let Some(cursor) = window.cursor_position() {
            let (ox, oy) =
                ui.1.single()
                    .map(|n| crate::ui::theme::node_origin(n, (0.0, 0.0)))
                    .unwrap_or((0.0, 0.0));
            if mouse.just_pressed(MouseButton::Left)
                && cursor.x >= ox + 202.0
                && cursor.x <= ox + 238.0
                && cursor.y >= oy + 91.0
                && cursor.y <= oy + 119.0
            {
                // #1356：觉醒模式循环武器；分解/降级/重置循环全部物品
                let items = inv_q
                    .single()
                    .map(|inv| inv.items.as_slice())
                    .unwrap_or(&[]);
                let pool: Vec<InvItem> = if state.service == NpcAwakeService::Awaken {
                    items
                        .iter()
                        .flatten()
                        .filter(|it| it.item_type == 1)
                        .cloned()
                        .collect()
                } else {
                    items.iter().flatten().cloned().collect()
                };
                if !pool.is_empty() {
                    let cur = state
                        .selected_uid
                        .and_then(|u| pool.iter().position(|w| w.unique_id == u))
                        .map(|i| i + 1)
                        .unwrap_or(0);
                    let item = pool[cur % pool.len()].clone();
                    state.selected_uid = Some(item.unique_id);
                    state.selected_item = Some(item.clone());
                    state.awake_type = None;
                    state.materials.clear();
                    state.result_text = String::new();
                    tracing::info!("⚒️ 选择觉醒物品: {} (uid={})", item.name, item.unique_id);
                }
            }
        }
    }

    // 操作按钮：按服务发包（C# PanelType 语义）
    for (e, inter) in &upgrade {
        if edge(e, inter, &mut prev_inter) {
            match state.service {
                NpcAwakeService::Awaken => {
                    if let (Some(uid), Some(at)) = (state.selected_uid, state.awake_type) {
                        net.send_packet(&Awakening {
                            unique_id: uid,
                            awake_type: at,
                            position_idx: 0,
                        });
                        tracing::info!("⚒️ 执行觉醒 uid={} type={:?}", uid, at);
                    }
                }
                NpcAwakeService::Disassemble => {
                    if let Some(uid) = state.selected_uid {
                        net.send_packet(&mir2_shared::packets::client::misc::DisassembleItem {
                            unique_id: uid,
                        });
                        tracing::info!("🔧 分解物品 uid={}", uid);
                    }
                }
                NpcAwakeService::Downgrade => {
                    if let Some(uid) = state.selected_uid {
                        net.send_packet(&mir2_shared::packets::client::misc::DowngradeAwakening {
                            unique_id: uid,
                        });
                        tracing::info!("⬇️ 觉醒降级 uid={}", uid);
                    }
                }
                NpcAwakeService::Reset => {
                    if let Some(uid) = state.selected_uid {
                        net.send_packet(&mir2_shared::packets::client::misc::ResetAddedItem {
                            unique_id: uid,
                        });
                        tracing::info!("🔄 重置附加属性 uid={}", uid);
                    }
                }
            }
        }
    }
}

/// 渲染：主物品图标/名字 + 材料/结果标签
#[allow(clippy::too_many_arguments)]
fn npc_awake_render_system(
    mgr: Res<crate::game::dialogs::DialogManager>,
    state: Res<NpcAwakeState>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    net: Res<NetConnection>,
    item_info: Res<crate::game::item_names::ItemInfoCache>,
    mut requested: Local<std::collections::HashSet<i32>>,
    mut icon: Query<
        (&mut ImageNode, &NpcAwakeMainIcon),
        (Without<NpcAwakeMainName>, Without<NpcAwakeNeedIcon>),
    >,
    mut need_icons: Query<
        (&mut ImageNode, &NpcAwakeNeedIcon),
        (Without<NpcAwakeMainIcon>, Without<NpcAwakeMainName>),
    >,
    // 格 3..6 的图标层：只在格里有物品时显形（B0001 互斥：与上面两个 `&mut ImageNode`
    // 查询互斥——它们都写了 `Without<NpcAwakePlaceIcon>`）。
    mut place_icons: Query<
        (&mut ImageNode, &mut Visibility, &NpcAwakePlaceIcon),
        (Without<NpcAwakeMainIcon>, Without<NpcAwakeNeedIcon>),
    >,
    mut name: Query<
        &mut Text,
        (
            With<NpcAwakeMainName>,
            Without<NpcAwakeMainIcon>,
            Without<NpcAwakeMaterialText>,
            Without<NpcAwakeResultText>,
        ),
    >,
    mut mats: Query<
        (&mut Text, &NpcAwakeMaterialText),
        (
            With<NpcAwakeMaterialText>,
            Without<NpcAwakeResultText>,
            Without<NpcAwakeMainName>,
            Without<NpcAwakeMainIcon>,
        ),
    >,
    mut res: Query<
        &mut Text,
        (
            With<NpcAwakeResultText>,
            Without<NpcAwakeMaterialText>,
            Without<NpcAwakeMainName>,
            Without<NpcAwakeMainIcon>,
        ),
    >,
) {
    if !mgr.is_open(crate::game::dialogs::DialogKind::NpcAwake) {
        return;
    }
    for (mut node, _) in &mut icon {
        if let Some(item) = &state.selected_item {
            if let Some(h) = load_lib_image(
                &mut libs,
                &mut images,
                LibraryName::Items,
                item.image as usize,
            ) {
                node.image = h;
            }
        }
    }
    for mut text in &mut name {
        text.0 = state
            .selected_item
            .as_ref()
            .map(|i| i.name.clone())
            .unwrap_or_default();
    }
    // 只读材料格 + 需求文案（C# `setNeedItems`）：`MaterialsCount[i] != 0` 才画格/写文案。
    // 名字与图标帧都来自 `NewItemInfo` 缓存；表里没有就**发一次** `RequestItemInfo`（按索引去重，C#
    // `GameScene.RequestItemInfo` 同语义），名字先占位 `#id`。
    for (mut node, slot) in &mut need_icons {
        if let Some(m) = state.materials.get(slot.0).filter(|m| m.count != 0) {
            if let Some(frame) = item_info.images.get(&m.item_id).copied() {
                if let Some(h) =
                    load_lib_image(&mut libs, &mut images, LibraryName::Items, frame as usize)
                {
                    node.image = h;
                }
            } else if requested.insert(m.item_id) {
                net.send_packet(&mir2_shared::packets::client::info::RequestItemInfo {
                    item_index: m.item_id,
                });
                tracing::info!("🛠️ 觉醒材料缺物品信息，请求 ItemInfo: idx={}", m.item_id);
            }
        }
    }
    // 格 3..6：有物品就画图标（帧号来自物品自带 `image`），没有就隐藏整个图标层。
    for (mut node, mut vis, slot) in &mut place_icons {
        if let Some(item) = state.place_items.get(slot.0).and_then(|s| s.as_ref()) {
            if let Some(h) = load_lib_image(
                &mut libs,
                &mut images,
                LibraryName::Items,
                item.image as usize,
            ) {
                node.image = h;
                *vis = Visibility::Inherited;
            }
        } else {
            *vis = Visibility::Hidden;
        }
    }
    for (mut text, slot) in &mut mats {
        text.0 = match state.materials.get(slot.0).filter(|m| m.count != 0) {
            Some(m) => {
                let name = item_info
                    .names
                    .get(&m.item_id)
                    .cloned()
                    .unwrap_or_else(|| format!("#{}", m.item_id));
                need_item_text(&name, m.count)
            }
            None => String::new(),
        };
    }
    for mut text in &mut res {
        text.0 = state.result_text.clone();
    }
}

/// 消费服务端觉醒事件（网络层只广播 ServerEvent）
fn awake_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut awake: ResMut<NpcAwakeState>,
    mut mgr: ResMut<DialogManager>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        match ev {
            ServerEvent::AwakeningMaterials { materials } => {
                awake.materials = materials
                    .iter()
                    .map(|(item_id, count)| MaterialRow {
                        item_id: *item_id,
                        count: *count,
                    })
                    .collect();
            }
            ServerEvent::AwakeningResult {
                result,
                result_text,
            } => {
                awake.result = *result;
                awake.result_text = result_text.clone();
            }
            ServerEvent::NpcAwakePanel { service } => {
                // #1356：C# S.NPCAwakening/S.NPCDisassemble/S.NPCDowngrade/S.NPCReset → 打开面板
                awake.service = NpcAwakeService::from_u8(*service);
                awake.selected_uid = None;
                awake.selected_item = None;
                awake.awake_type = None;
                awake.materials.clear();
                awake.result_text = String::new();
                mgr.open(DialogKind::NpcAwake);
            }
            _ => {}
        }
    }
}

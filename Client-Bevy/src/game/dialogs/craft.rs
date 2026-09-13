// ============================================================================
// 合成对话框（M41；#2536 接入 NPC 合成面板入口）
// 参考：C# NPCDialogs.cs CraftDialog + GameScene.cs:4215
//   - S.NPCGoods(PanelType::Craft) 到达 → 商品对话框（合成产物列表）+ 本对话框同开
//   - 商品行点击产物 → 本对话框选中配方（C# 1090 ResetCells/RefreshCraftCells/Show）
//   - 商品对话框关闭 → 本对话框联动关闭（C# 1413 Hide → CraftDialog.Hide()）
// 网络（ServerRust gate 实际 wire）：
//   C: CraftItem[recipe_id u32][materials_count u32]
//   S: CraftItem[recipe_id u32][count u16][success u8] + 系统聊天消息
// 材料由服务端按配方校验/扣除（C# 玩家摆槽交互在 Rust wire 下不需要）
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::inventory::InventoryOrigin;
use crate::game::dialogs::npc_goods::NpcGoodsState;
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{shared_cjk_font, UiCjkFont, UiFont};
use crate::ui::gray::UiGray;
use crate::ui::theme::{
    load_lib_image, spawn_icon_button, spawn_item_cell_ui, spawn_label, spawn_panel, UiItemCellData,
    UiItemCellIcon,
};

/// #2536：当前选中的合成配方（产物；recipe_id 由服务端随合成商品 unique_id 下发）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedRecipe {
    pub recipe_id: u32,
    pub name: String,
}

/// 合成状态（CraftItem 响应写入）
#[derive(Resource, Default)]
pub struct CraftState {
    pub selected: Option<SelectedRecipe>,
    pub message: String,
    pub last_result: Option<(u32, u16, bool)>,
    /// #262 已学会配方（S.NewRecipeInfo）
    pub learned: Vec<i32>,
    /// #2720 配方详情（S.NewRecipeInfo 整份 ClientRecipeInfo，按 recipe_id 索引）：
    /// 产物/工具/材料/金币/成功率，合成材料槽与自动填充依赖。
    pub recipes: std::collections::HashMap<i32, mir2_shared::data::client_data::ClientRecipeInfo>,
    /// #2720 已放入合成槽的物品（C# `CraftDialog.Selected`）
    pub slots: [Option<CraftPlaced>; CRAFT_SLOT_COUNT],
}

/// 把「材料槽 → 来源背包格」的锁定关系同步进 [`InvLockedSlots`]（C# `CraftDialog.Selected`
/// 字典与 `cell.Locked` 一一对应；`ResetCells()` 时清空 → 本函数幂等收敛）。
pub fn sync_craft_locks(
    locked: &mut crate::game::dialogs::inventory::InvLockedSlots,
    slots: &[Option<CraftPlaced>; CRAFT_SLOT_COUNT],
) {
    // 只收敛 Craft 来源：装备/拆分/寄售等其它来源的锁不受影响
    locked.unlock_all(crate::game::dialogs::inventory::InvLockReason::Craft);
    for placed in slots.iter().flatten() {
        locked.lock(
            crate::game::dialogs::inventory::InvLockReason::Craft,
            placed.inv_slot,
        );
    }
}

/// 当前选中配方的详情（C# `CraftDialog.Recipe` 等价物）
pub fn selected_recipe_info<'a>(
    state: &'a CraftState,
) -> Option<&'a mir2_shared::data::client_data::ClientRecipeInfo> {
    state
        .selected
        .as_ref()
        .and_then(|r| state.recipes.get(&(r.recipe_id as i32)))
}

/// 配方行文案（C# CraftDialog RecipeLabel）
pub fn recipe_label(selected: &Option<SelectedRecipe>) -> String {
    match selected {
        Some(r) => format!("合成产物: {}", r.name),
        None => "未选择产物——点击左侧商品列表".to_string(),
    }
}

/// 合成对话框是否应随商品面板关闭（C# NPCDialogs.cs:1413 Hide → CraftDialog.Hide()；
/// 仅 Craft 面板联动——挂机脚本直开场景不受影响）
pub fn craft_should_close(npc_panel: mir2_shared::enums::PanelType, goods_visible: bool, craft_open: bool) -> bool {
    craft_open && npc_panel == mir2_shared::enums::PanelType::Craft && !goods_visible
}

/// C# `CraftDialog`（NPCDialogs.cs:2256）：面板 `Index = 1109; Library = Prguse`（原生 337x215）。
pub const CRAFT_W: f32 = 337.0;
pub const CRAFT_H: f32 = 215.0;
/// C# `CraftDialog.Show()`（NPCDialogs.cs:2448）：
/// `Location = (InventoryDialog.X - 12, InventoryDialog.Y + 236)`。
const CRAFT_REL_X: f32 = -12.0;
const CRAFT_REL_Y: f32 = 236.0;
/// C# 控件锚点（NPCDialogs.cs:2280-2391）。
pub const CRAFT_TITLE: (f32, f32) = (28.0, 8.0); // Title[18] 57x15
const CRAFT_RECIPE_LABEL: (f32, f32) = (22.0, 5.0); // RecipeLabel
const CRAFT_MESSAGE_LABEL: (f32, f32) = (10.0, 135.0); // PossibilityLabel
const CRAFT_GOLD_LABEL: (f32, f32) = (30.0, 190.0); // GoldLabel
pub const CRAFT_CLOSE_POS: (f32, f32) = (312.0, 3.0); // CloseButton（Prguse2[360..362] 24x21）
pub const CRAFT_AUTOFILL_POS: (f32, f32) = (165.0, 185.0); // AutoFillButton（Title[180..182] 48x25）
pub const CRAFT_CONFIRM_POS: (f32, f32) = (215.0, 185.0); // CraftButton（Title[336..338] 80x25）
/// 精灵首帧索引（Index/HoverIndex/PressedIndex 连续 3 帧）。
pub const CRAFT_AUTOFILL_INDEX: usize = 180;
pub const CRAFT_CONFIRM_INDEX: usize = 336;
/// C# `_toolCount` / `_ingredientCount`（NPCDialogs.cs:2261-2263）
const CRAFT_TOOL_COUNT: usize = 3;
const CRAFT_ING_COUNT: usize = 6;
const CRAFT_SLOT_COUNT: usize = CRAFT_TOOL_COUNT + CRAFT_ING_COUNT;
/// C# 格子几何：工具 `((x*44)+108, 44)`、材料 `((x-3)*40+52, 86)`，35x32
const CRAFT_TOOL_ORIGIN: (f32, f32) = (108.0, 44.0);
const CRAFT_TOOL_STEP: f32 = 44.0;
const CRAFT_ING_ORIGIN: (f32, f32) = (52.0, 86.0);
const CRAFT_ING_STEP: f32 = 40.0;
const CRAFT_CELL_W: f32 = 35.0;
const CRAFT_CELL_H: f32 = 32.0;

/// C# `Grid[idx].Location`（工具/材料两段）
fn craft_slot_pos(index: usize) -> (f32, f32) {
    if index < CRAFT_TOOL_COUNT {
        (
            CRAFT_TOOL_ORIGIN.0 + index as f32 * CRAFT_TOOL_STEP,
            CRAFT_TOOL_ORIGIN.1,
        )
    } else {
        let i = index - CRAFT_TOOL_COUNT;
        (
            CRAFT_ING_ORIGIN.0 + i as f32 * CRAFT_ING_STEP,
            CRAFT_ING_ORIGIN.1,
        )
    }
}

/// 已放入合成槽的背包物品（C# `CraftDialog.Selected`：格子 → 背包槽 + 锁定）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CraftPlaced {
    pub inv_slot: usize,
    pub item_index: i32,
    pub count: u16,
}

/// C# `Grid_Click` 校验：可放入当且仅当格子为空、物品索引与需求一致，
/// 工具要求 `CurrentDura >= min_dura`、材料要求 `count >= 需求数量`。
pub fn craft_slot_accepts(
    requirement: &mir2_shared::data::client_data::RecipeRequirement,
    filled: bool,
    item_index: i32,
    count: u16,
    current_dura: u16,
) -> bool {
    if filled || item_index != requirement.item_index {
        return false;
    }
    if requirement.min_dura > 0 {
        current_dura >= requirement.min_dura
    } else {
        count >= requirement.count
    }
}

/// C# `CraftDialog.CraftButton` 的 `Enabled`/`GrayScale`（NPCDialogs.cs:2379-2390 构造即
/// `GrayScale = true, Enabled = false`；`RefreshCraftCells`（:2686-2723）在选中配方后先置
/// 可用，再对每个工具/材料槽判 `need = Grid[i].Item == null || Item.Count < ShadowItem.Count`，
/// 任一槽未满足 → `Enabled = false; GrayScale = true`）。
///
/// Bevy 侧：未选配方 → 不可用；已选配方 → 该配方列出的工具/材料槽全部就位才可用
/// （超出 3 工具格 / 6 材料格的额外需求按 C# `continue` 语义忽略）。
pub fn craft_button_enabled(
    info: Option<&mir2_shared::data::client_data::ClientRecipeInfo>,
    slots: &[Option<CraftPlaced>; CRAFT_SLOT_COUNT],
) -> bool {
    let Some(info) = info else {
        return false;
    };
    let tools = info.tools.len().min(CRAFT_TOOL_COUNT);
    let ingredients = info.ingredients.len().min(CRAFT_ING_COUNT);
    slots[..tools].iter().all(|s| s.is_some())
        && slots[CRAFT_TOOL_COUNT..CRAFT_TOOL_COUNT + ingredients]
            .iter()
            .all(|s| s.is_some())
}

/// C# `AutoFill()`：按配方顺序（先工具后材料）在背包里挑未占用且满足条件的物品，
/// 返回与 `requirements` 等长的背包槽位（None = 没找到）。
pub fn craft_autofill_slots(
    requirements: &[mir2_shared::data::client_data::RecipeRequirement],
    inventory: &[(i32, u16, u16)],
    used: &[usize],
) -> Vec<Option<usize>> {
    let mut taken: Vec<usize> = used.to_vec();
    requirements
        .iter()
        .map(|req| {
            let found = inventory.iter().enumerate().position(|(i, (index, count, dura))| {
                if taken.contains(&i) || *index != req.item_index {
                    return false;
                }
                if req.min_dura > 0 {
                    *dura >= req.min_dura
                } else {
                    *count >= req.count
                }
            });
            if let Some(i) = found {
                taken.push(i);
            }
            found
        })
        .collect()
}

#[derive(Component)]
pub struct CraftWidget;

#[derive(Component)]
pub struct CraftClose;

#[derive(Component)]
pub struct CraftBtn;

/// 自动填充（C# `AutoFillButton`：按配方工具/材料从背包自动摆放）
#[derive(Component)]
pub struct CraftAutoFill;

/// 合成槽格子（0..3 = 工具，3..9 = 材料；C# `CraftDialog.Grid`）
#[derive(Component)]
pub struct CraftCell(pub usize);

#[derive(Component)]
pub struct CraftLine(usize);

pub struct CraftPlugin;

impl Plugin for CraftPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CraftState>();
        app.add_systems(
            Update,
            craft_server_events.run_if(in_state(AppState::Game)),
        );
        app.add_systems(OnEnter(AppState::Game), spawn_craft);
        app.add_systems(OnExit(AppState::Game), cleanup_craft);
        app.add_systems(
            Update,
            (craft_ui_system, craft_slots_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_craft(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_craft(
    mut commands: Commands,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut fonts: ResMut<Assets<Font>>,
    mut cjk_font: ResMut<UiCjkFont>,
    mut ui_font: ResMut<UiFont>,
    inv_origin: Res<InventoryOrigin>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();
    let cjk = shared_cjk_font(&mut fonts, &mut cjk_font);

    // 面板 C# Prguse[1109]（原生 337x215）；位置按 C# Show() 相对背包窗计算
    let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 1109) else {
        return;
    };
    let panel = spawn_panel(
        &mut commands,
        bg,
        inv_origin.0 + CRAFT_REL_X,
        inv_origin.1 + CRAFT_REL_Y,
        CRAFT_W,
        CRAFT_H,
        30,
    );
    commands
        .entity(panel)
        .insert((DialogRoot(DialogKind::Craft), CraftWidget));

    commands.entity(panel).with_children(|p| {
        // 标题精灵 C# TitleLabel Title[18] @(28,8)
        if let Some(title) = load_lib_image(&mut libs, &mut images, LibraryName::Title, 18) {
            crate::ui::theme::spawn_image(p, title, CRAFT_TITLE.0, CRAFT_TITLE.1, 57.0, 15.0, 9);
        }
        // 关闭 C# CloseButton Prguse2[360/361/362] @(312,3)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 360),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 361),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 362),
        ) {
            spawn_icon_button(p, n, h, pr, CRAFT_CLOSE_POS.0, CRAFT_CLOSE_POS.1, 24.0, 21.0, 10)
                .insert(CraftClose);
        }
        // C# RecipeLabel(22,5) / PossibilityLabel(10,135) / GoldLabel(30,190)；
        // CraftLine(3) 是 Bevy 扩展（已学会配方数），放在标题下方空位。
        spawn_label(p, &cjk, "", CRAFT_RECIPE_LABEL.0, CRAFT_RECIPE_LABEL.1, 12.0, Color::WHITE, 9)
            .insert(CraftLine(0));
        spawn_label(p, &cjk, "", CRAFT_MESSAGE_LABEL.0, CRAFT_MESSAGE_LABEL.1, 12.0, Color::WHITE, 9)
            .insert(CraftLine(1));
        spawn_label(p, &cjk, "", CRAFT_GOLD_LABEL.0, CRAFT_GOLD_LABEL.1, 12.0, Color::WHITE, 9)
            .insert(CraftLine(2));
        spawn_label(p, &cjk, "", CRAFT_RECIPE_LABEL.0, CRAFT_RECIPE_LABEL.1 + 16.0, 12.0, Color::WHITE, 9)
            .insert(CraftLine(3));
        // 自动填充 C# AutoFillButton Title[180..182] @(165,185)、合成 C# CraftButton Title[336..338] @(215,185)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, CRAFT_AUTOFILL_INDEX),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, CRAFT_AUTOFILL_INDEX + 1),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, CRAFT_AUTOFILL_INDEX + 2),
        ) {
            spawn_icon_button(p, n, h, pr, CRAFT_AUTOFILL_POS.0, CRAFT_AUTOFILL_POS.1, 48.0, 25.0, 10)
                .insert(CraftAutoFill);
        }
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, CRAFT_CONFIRM_INDEX),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, CRAFT_CONFIRM_INDEX + 1),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, CRAFT_CONFIRM_INDEX + 2),
        ) {
            spawn_icon_button(p, n, h, pr, CRAFT_CONFIRM_POS.0, CRAFT_CONFIRM_POS.1, 80.0, 25.0, 10)
                .insert((CraftBtn, UiGray::default()));
        }
        // C# Grid：3 工具格 + 6 材料格（影子格由 ui_system 按配方刷新）
        for i in 0..CRAFT_SLOT_COUNT {
            let (cx, cy) = craft_slot_pos(i);
            spawn_item_cell_ui(
                p,
                &mut images,
                &font,
                cx,
                cy,
                CRAFT_CELL_W,
                CRAFT_CELL_H,
                9,
                i,
            )
            .insert((CraftCell(i), Button));
        }
    });
}

/// 显隐 + 渲染 + 选择联动 + 合成
fn craft_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut state: ResMut<CraftState>,
    mut npc_goods: ResMut<NpcGoodsState>,
    mut locked: ResMut<crate::game::dialogs::inventory::InvLockedSlots>,
    inv_origin: Res<InventoryOrigin>,
    close: Query<(Entity, &Interaction), With<CraftClose>>,
    mut widgets: Query<&mut Visibility, With<CraftWidget>>,
    mut panel_node: Query<&mut Node, With<CraftWidget>>,
    mut lines: Query<(&mut Text, &CraftLine)>,
    mut prev_inter: Local<std::collections::HashMap<Entity, Interaction>>,
    mut was_open: Local<bool>,
) {
    fn edge(
        e: Entity,
        inter: &Interaction,
        prev: &mut std::collections::HashMap<Entity, Interaction>,
    ) -> bool {
        let was = prev.insert(e, *inter);
        *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
    }
    // #2536：商品行点击选中的配方（C# NPCDialogs.cs:1090 ResetCells/RefreshCraftCells/Show）
    if let Some((recipe_id, name)) = npc_goods.craft_pick.take() {
        state.selected = Some(SelectedRecipe { recipe_id, name });
        mgr.open(DialogKind::Craft);
    }
    // #2536：商品面板关闭 → 联动关闭（C# NPCDialogs.cs:1413）
    if craft_should_close(npc_goods.panel, npc_goods.visible, mgr.is_open(DialogKind::Craft)) {
        mgr.close(DialogKind::Craft);
    }
    let open = mgr.is_open(DialogKind::Craft);
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    // C# Show()：每次打开按背包窗当前位置定位（InventoryDialog.X-12, Y+236）
    if open && !*was_open {
        for mut node in &mut panel_node {
            node.left = Val::Px(inv_origin.0 + CRAFT_REL_X);
            node.top = Val::Px(inv_origin.1 + CRAFT_REL_Y);
        }
    }
    *was_open = open;
    if !open {
        return;
    }
    for (e, inter) in &close {
        if edge(e, inter, &mut prev_inter) {
            mgr.close(DialogKind::Craft);
            // C# `Hide()` → ResetCells()（含解锁来源背包格）
            state.slots = Default::default();
            sync_craft_locks(&mut locked, &state.slots);
        }
    }
    for (mut text, line) in &mut lines {
        text.0 = match line.0 {
            0 => recipe_label(&state.selected),
            1 => state.message.clone(),
            2 => match selected_recipe_info(&state) {
                // C# GoldLabel：金币需求 + 成功率
                Some(info) => format!(
                    "金币: {} 成功率: {}% 工具: {} 材料: {}",
                    info.gold,
                    info.chance,
                    info.tools.len(),
                    info.ingredients.len()
                ),
                None => "未选择产物——先在左侧商品列表选合成产物".to_string(),
            },
            3 => format!("已学会配方: {} 种", state.learned.len()),
            _ => String::new(),
        };
    }
}

/// 材料槽：影子格渲染 + 点击放入 + 自动填充 + 带槽位合成
/// （C# `CraftDialog` `Grid_Click` / `AutoFill` / `CraftItem`）
#[allow(clippy::too_many_arguments)]
fn craft_slots_system(
    mgr: Res<DialogManager>,
    mut state: ResMut<CraftState>,
    net: Res<NetConnection>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut image_cache: ResMut<crate::ui::sprite_ui::UiImageCache>,
    inv_q: Query<&crate::game::player_state::Inventory, With<crate::actor::LocalPlayer>>,
    mut inv_click: ResMut<crate::game::dialogs::inventory::InvClickState>,
    // #2736：C# `SelectedCell.Locked`——放入材料/自动填充后锁定来源背包格
    mut locked: ResMut<crate::game::dialogs::inventory::InvLockedSlots>,
    autofill_btn: Query<(Entity, &Interaction), With<CraftAutoFill>>,
    mut craft_btn: Query<(Entity, &Interaction, &mut UiGray), With<CraftBtn>>,
    mut cells: Query<(&CraftCell, &mut UiItemCellData), Without<UiItemCellIcon>>,
    cell_inter: Query<(Entity, &Interaction, &CraftCell)>,
    mut prev_inter: Local<std::collections::HashMap<Entity, Interaction>>,
    mut slot_recipe: Local<Option<i32>>,
) {
    fn edge(
        e: Entity,
        inter: &Interaction,
        prev: &mut std::collections::HashMap<Entity, Interaction>,
    ) -> bool {
        let was = prev.insert(e, *inter);
        *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
    }
    if !mgr.is_open(DialogKind::Craft) {
        return;
    }
    // 配方详情 + 背包快照；配方变化即清空材料槽（C# ResetCells）
    let info = selected_recipe_info(&state).cloned();
    let current_recipe = state.selected.as_ref().map(|r| r.recipe_id as i32);
    if *slot_recipe != current_recipe {
        *slot_recipe = current_recipe;
        state.slots = Default::default();
        sync_craft_locks(&mut locked, &state.slots); // C# `ResetCells()`：换配方即解锁
    }
    let inv_items: Vec<Option<crate::game::dialogs::inventory::InvItem>> =
        inv_q.single().map(|inv| inv.items.clone()).unwrap_or_default();
    let requirement = |i: usize| -> Option<mir2_shared::data::client_data::RecipeRequirement> {
        info.as_ref().and_then(|recipe| {
            if i < CRAFT_TOOL_COUNT {
                recipe.tools.get(i).cloned()
            } else {
                recipe.ingredients.get(i - CRAFT_TOOL_COUNT).cloned()
            }
        })
    };
    // 影子格渲染：已放入 → 该背包物品图标 + 数量；未放入 → 配方需求图标 + 数量
    for (cell, mut data) in &mut cells {
        let req = requirement(cell.0);
        let placed = state.slots[cell.0].clone();
        let (image_index, count) = match (&placed, req.as_ref()) {
            (Some(p), _) => {
                let image = inv_items
                    .get(p.inv_slot)
                    .and_then(|s| s.as_ref())
                    .map(|it| it.image as usize)
                    .or_else(|| req.as_ref().map(|r| r.image as usize));
                (image, p.count)
            }
            (None, Some(r)) => (Some(r.image as usize), r.count),
            (None, None) => (None, 0),
        };
        data.icon = match image_index {
            Some(idx) if idx > 0 => crate::ui::sprite_ui::ui_image(
                &mut libs,
                &mut images,
                &mut image_cache,
                LibraryName::Items,
                idx,
            ),
            _ => None,
        };
        data.count = (count > 1).then_some(count as u32);
        data.dura_ratio = None;
    }
    // C# `Grid_Click`：背包选中物 → 点击影子格放入（索引/数量/工具耐久校验）
    for (e, inter, cell) in &cell_inter {
        if !edge(e, inter, &mut prev_inter) {
            continue;
        }
        let Some(req) = requirement(cell.0) else {
            state.message = "请先选择合成产物".to_string();
            continue;
        };
        let filled = state.slots[cell.0].is_some();
        let Some(inv_slot) = inv_click.selected else {
            state.message = if filled {
                format!("已放入 {}", req.name)
            } else {
                format!("需要 {} —— 先在背包选中再点此格", req.name)
            };
            continue;
        };
        let Some(item) = inv_items.get(inv_slot).and_then(|s| s.as_ref()) else {
            continue;
        };
        if craft_slot_accepts(&req, filled, item.item_index, item.count, item.current_dura) {
            let name = item.name.clone();
            state.slots[cell.0] = Some(CraftPlaced {
                inv_slot,
                item_index: item.item_index,
                count: req.count,
            });
            // C# `Grid_Click`：放入后清空 SelectedCell 并 **锁定来源背包格**（:2433）
            inv_click.selected = None;
            state.message = format!("放入 {}", name);
            sync_craft_locks(&mut locked, &state.slots);
        } else if filled {
            state.message = "该槽已有物品".to_string();
        } else {
            state.message = format!("需要 {}（索引/数量/耐久不符）", req.name);
        }
    }
    // 自动填充（C# AutoFill：ResetCells(false) → 按配方顺序从背包挑匹配物品）
    for (e, inter) in &autofill_btn {
        if !edge(e, inter, &mut prev_inter) {
            continue;
        }
        match info.as_ref() {
            None => state.message = "请先选择合成产物".to_string(),
            Some(recipe) => {
                state.slots = Default::default();
                sync_craft_locks(&mut locked, &state.slots); // C# `AutoFill()` 先 `ResetCells(false)`
                let mut requirements = recipe.tools.clone();
                requirements.extend(recipe.ingredients.iter().cloned());
                let inventory: Vec<(i32, u16, u16)> = inv_items
                    .iter()
                    .map(|slot| {
                        slot.as_ref()
                            .map(|it| (it.item_index, it.count, it.current_dura))
                            .unwrap_or((i32::MIN, 0, 0))
                    })
                    .collect();
                let found = craft_autofill_slots(&requirements, &inventory, &[]);
                let mut placed = 0usize;
                for (i, slot) in found.iter().enumerate() {
                    if let Some(inv_slot) = slot {
                        state.slots[i] = Some(CraftPlaced {
                            inv_slot: *inv_slot,
                            item_index: requirements[i].item_index,
                            count: requirements[i].count,
                        });
                        placed += 1;
                    }
                }
                // C# `AutoFill`：逐格 `cell.Locked = true`（:2479/:2506）
                sync_craft_locks(&mut locked, &state.slots);
                state.message = format!("自动填充 {}/{} 槽", placed, requirements.len());
                tracing::info!("🔧 自动填充 {}/{} 槽", placed, requirements.len());
            }
        }
    }
    // C# `RefreshCraftCells`：任一工具/材料槽未满足 → `CraftButton.Enabled = false; GrayScale = true`
    // （构造默认即「不可用 + 灰度」），禁用时点击不触发（C# `MirControl` 的 `!Enabled` 早返回）。
    let craft_enabled = craft_button_enabled(info.as_ref(), &state.slots);
    for (e, inter, mut gray) in &mut craft_btn {
        let want_gray = !craft_enabled;
        if gray.gray != want_gray {
            gray.gray = want_gray;
        }
        if !craft_enabled || !edge(e, inter, &mut prev_inter) {
            continue;
        }
        // 合成（C# `CraftItem()`：材料槽全部就位才发包，带上选中的背包槽）
        let Some(r) = state.selected.clone() else {
            continue;
        };
        let slots: Vec<i32> = state
            .slots
            .iter()
            .filter_map(|s| s.as_ref().map(|p| p.inv_slot as i32))
            .collect();
        net.send_packet(&crate::network::CraftItemWire {
            unique_id: r.recipe_id as u64,
            count: 1,
            slots,
        });
        state.message = format!("合成 {} 中…", r.name);
        tracing::info!("🔧 合成配方 {}（{}）", r.recipe_id, r.name);
    }
}


/// 消费服务端合成事件（网络层只广播 ServerEvent；文案在此构造）
fn craft_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut craft: ResMut<CraftState>,
    mut locked: ResMut<crate::game::dialogs::inventory::InvLockedSlots>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        if let ServerEvent::RecipeLearned { recipe_id, info } = ev {
            // #262：学会配方；#2720：同时缓存整份配方（材料槽/自动填充数据源）
            if !craft.learned.contains(recipe_id) {
                craft.learned.push(*recipe_id);
            }
            craft.recipes.insert(*recipe_id, info.clone());
            craft.message = format!("学会配方 #{}", recipe_id);
        }
        if let ServerEvent::CraftResult { recipe_id, count, success } = ev {
            craft.last_result = Some((*recipe_id, *count, *success));
            craft.message = if *success {
                format!("合成成功！配方 {} ×{}", recipe_id, count)
            } else {
                format!("合成失败（配方 {}）", recipe_id)
            };
            // C# `S.CraftItem` → `CraftDialog.UpdateCraftCells()`：失效格解除锁定并清空
            craft.slots = Default::default();
            sync_craft_locks(&mut locked, &craft.slots);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mir2_shared::enums::PanelType;

    /// #2742：C# `RefreshCraftCells`（NPCDialogs.cs:2686-2723）——未选配方或任一工具/材料槽
    /// 未就位 → `CraftButton.Enabled = false; GrayScale = true`（构造默认也是禁用 + 灰度）。
    #[test]
    fn craft_button_enabled_matches_refresh_craft_cells() {
        use mir2_shared::data::client_data::{ClientRecipeInfo, RecipeRequirement};
        let req = |index: i32| RecipeRequirement {
            item_index: index,
            count: 1,
            image: index as u16,
            name: format!("#{index}"),
            min_dura: 0,
        };
        let recipe = ClientRecipeInfo {
            gold: 1,
            chance: 100,
            item: req(9),
            tools: vec![req(5)],
            ingredients: vec![req(1)],
        };
        let mut slots: [Option<CraftPlaced>; CRAFT_SLOT_COUNT] = Default::default();
        // 未选配方 → 不可用（C# 构造默认 `Enabled=false, GrayScale=true`）
        assert!(!craft_button_enabled(None, &slots));
        // 已选配方但槽位空 → 不可用
        assert!(!craft_button_enabled(Some(&recipe), &slots));
        // 只放工具（材料缺）→ 仍不可用
        slots[0] = Some(CraftPlaced {
            inv_slot: 1,
            item_index: 5,
            count: 1,
        });
        assert!(!craft_button_enabled(Some(&recipe), &slots));
        // 工具 + 材料都就位 → 可用
        slots[CRAFT_TOOL_COUNT] = Some(CraftPlaced {
            inv_slot: 2,
            item_index: 1,
            count: 1,
        });
        assert!(craft_button_enabled(Some(&recipe), &slots));
        // 只吃工具的配方：工具就位即可用
        let tool_only = ClientRecipeInfo {
            ingredients: vec![],
            ..recipe.clone()
        };
        assert!(craft_button_enabled(Some(&tool_only), &slots));
        // 超出 3 工具格 / 6 材料格的额外需求按 C# `continue` 忽略
        let many_tools = ClientRecipeInfo {
            tools: vec![req(5), req(5), req(5), req(5)],
            ingredients: vec![],
            ..recipe.clone()
        };
        // 只有第 1 个工具格就位 → 仍不可用（第 2/3 个工具格未就位）
        assert!(!craft_button_enabled(Some(&many_tools), &slots));
        // 第 2/3 个工具格就位 → 可用（第 4 个工具超出格子按 C# `continue` 不检查）
        slots[1] = Some(CraftPlaced {
            inv_slot: 3,
            item_index: 5,
            count: 1,
        });
        slots[2] = Some(CraftPlaced {
            inv_slot: 4,
            item_index: 5,
            count: 1,
        });
        assert!(craft_button_enabled(Some(&many_tools), &slots));
    }

    /// #2736：C# `CraftDialog.Selected`（材料槽 → 来源背包格）与 `cell.Locked` 一一对应，
    /// `ResetCells()` 清空槽位即全部解锁；同步幂等（重复调用不残留旧锁）
    #[test]
    fn craft_lock_sync_matches_placed_slots() {
        let mut slots: [Option<CraftPlaced>; CRAFT_SLOT_COUNT] = Default::default();
        let mut locked = crate::game::dialogs::inventory::InvLockedSlots::default();

        slots[0] = Some(CraftPlaced {
            inv_slot: 5,
            item_index: 1,
            count: 1,
        });
        slots[4] = Some(CraftPlaced {
            inv_slot: 12,
            item_index: 2,
            count: 3,
        });
        sync_craft_locks(&mut locked, &slots);
        assert!(locked.is_locked(5) && locked.is_locked(12));
        assert!(!locked.is_locked(0));

        // 幂等
        sync_craft_locks(&mut locked, &slots);
        assert!(locked.is_locked(5) && locked.is_locked(12));

        // 取出一个槽 → 该来源格解锁，其余保持
        slots[0] = None;
        sync_craft_locks(&mut locked, &slots);
        assert!(!locked.is_locked(5) && locked.is_locked(12));

        // C# `ResetCells()`：全清 → 全部解锁
        slots = Default::default();
        sync_craft_locks(&mut locked, &slots);
        assert!(!locked.is_locked(12));
    }

    fn sel() -> Option<SelectedRecipe> {
        Some(SelectedRecipe {
            recipe_id: 7,
            name: "精铁剑".to_string(),
        })
    }

    fn req(
        item_index: i32,
        count: u16,
        image: u16,
        min_dura: u16,
    ) -> mir2_shared::data::client_data::RecipeRequirement {
        mir2_shared::data::client_data::RecipeRequirement {
            item_index,
            count,
            image,
            name: format!("#{}", item_index),
            min_dura,
        }
    }

    /// #2720：格子放置校验（C# `Grid_Click`：工具看 `CurrentDura >= 1000`，材料看数量）
    #[test]
    fn craft_slot_accepts_matches_csharp_rules() {
        let tool = req(1001, 1, 33, 1000);
        assert!(!craft_slot_accepts(&tool, false, 1001, 1, 999)); // 耐久不足
        assert!(craft_slot_accepts(&tool, false, 1001, 1, 1000));
        assert!(!craft_slot_accepts(&tool, true, 1001, 1, 5000)); // 槽已占用
        assert!(!craft_slot_accepts(&tool, false, 1002, 1, 5000)); // 物品不符

        let ingredient = req(2001, 3, 55, 0);
        assert!(!craft_slot_accepts(&ingredient, false, 2001, 2, 0)); // 数量不足
        assert!(craft_slot_accepts(&ingredient, false, 2001, 3, 0));
    }

    /// #2720：AutoFill 按配方顺序（工具→材料）挑未占用且满足条件的背包物品
    #[test]
    fn craft_autofill_picks_matching_items() {
        let requirements = vec![req(1001, 1, 33, 1000), req(2001, 2, 55, 0), req(2002, 1, 56, 0)];
        let inventory = vec![(1001, 1, 1200), (2001, 5, 0), (2002, 1, 0)];
        assert_eq!(
            craft_autofill_slots(&requirements, &inventory, &[]),
            vec![Some(0), Some(1), Some(2)]
        );

        // 工具耐久不足 → 工具槽留空，材料照填
        let dull_tool = vec![(1001, 1, 100), (2001, 5, 0), (2002, 1, 0)];
        assert_eq!(
            craft_autofill_slots(&requirements, &dull_tool, &[]),
            vec![None, Some(1), Some(2)]
        );

        // 已占用的背包槽不重复使用
        assert_eq!(
            craft_autofill_slots(&requirements, &inventory, &[0]),
            vec![None, Some(1), Some(2)]
        );
    }

    /// #2720：格子坐标对齐 C# Grid（工具 (108+x*44,44)，材料 (52+(x-3)*40,86)）
    #[test]
    fn craft_slot_positions_match_csharp_grid() {
        assert_eq!(craft_slot_pos(0), (108.0, 44.0));
        assert_eq!(craft_slot_pos(2), (196.0, 44.0));
        assert_eq!(craft_slot_pos(3), (52.0, 86.0));
        assert_eq!(craft_slot_pos(8), (252.0, 86.0));
    }

    /// #2536：配方行文案（选中显示产物名，未选中给提示）
    #[test]
    fn recipe_label_shows_selection_or_hint() {
        assert_eq!(recipe_label(&sel()), "合成产物: 精铁剑");
        assert_eq!(recipe_label(&None), "未选择产物——点击左侧商品列表");
    }

    /// #2536：合成对话框仅随 Craft 面板关闭联动（挂机脚本直开不受影响）
    #[test]
    fn craft_closes_with_goods_panel_only_in_craft_mode() {
        assert!(craft_should_close(PanelType::Craft, false, true));
        assert!(!craft_should_close(PanelType::Craft, true, true));
        assert!(!craft_should_close(PanelType::Buy, false, true));
        assert!(!craft_should_close(PanelType::Craft, false, false));
    }

    /// #2720：Craft 面板与控件锚点对齐 C# `CraftDialog`（NPCDialogs.cs:2280-2391）。
    #[test]
    fn craft_layout_matches_csharp_anchors() {
        assert_eq!((CRAFT_W, CRAFT_H), (337.0, 215.0)); // Prguse[1109] 原生尺寸
        assert_eq!((CRAFT_REL_X, CRAFT_REL_Y), (-12.0, 236.0)); // Show() 相对背包
        assert_eq!(CRAFT_TITLE, (28.0, 8.0));
        assert_eq!(CRAFT_RECIPE_LABEL, (22.0, 5.0));
        assert_eq!(CRAFT_MESSAGE_LABEL, (10.0, 135.0));
        assert_eq!(CRAFT_GOLD_LABEL, (30.0, 190.0));
        assert_eq!(CRAFT_CLOSE_POS, (312.0, 3.0));
        assert_eq!(CRAFT_AUTOFILL_POS, (165.0, 185.0));
        assert_eq!(CRAFT_CONFIRM_POS, (215.0, 185.0));
        assert_eq!(CRAFT_AUTOFILL_INDEX, 180);
        assert_eq!(CRAFT_CONFIRM_INDEX, 336);
    }

    /// 控件必须落在面板内且互不重叠（C# 面板 337x215）。
    #[test]
    fn craft_buttons_fit_panel() {
        let inside = |(x, y): (f32, f32), w: f32, h: f32| {
            x >= 0.0 && y >= 0.0 && x + w <= CRAFT_W && y + h <= CRAFT_H
        };
        assert!(inside(CRAFT_CLOSE_POS, 24.0, 21.0));
        assert!(inside(CRAFT_AUTOFILL_POS, 48.0, 25.0));
        assert!(inside(CRAFT_CONFIRM_POS, 80.0, 25.0));
        assert!(CRAFT_AUTOFILL_POS.0 + 48.0 <= CRAFT_CONFIRM_POS.0);
    }

    /// #2720：选中配方 → 详情查找（材料槽/自动填充的数据源）
    #[test]
    fn selected_recipe_info_lookup() {
        use mir2_shared::data::client_data::{ClientRecipeInfo, RecipeRequirement};

        let req = |item_index: i32, count: u16, image: u16, min_dura: u16| RecipeRequirement {
            item_index,
            count,
            image,
            name: format!("#{}", item_index),
            min_dura,
        };

        let mut state = CraftState::default();
        assert!(selected_recipe_info(&state).is_none());

        state.recipes.insert(
            5,
            ClientRecipeInfo {
                gold: 250,
                chance: 80,
                item: req(9005, 1, 120, 0),
                tools: vec![req(1001, 1, 33, 1000)],
                ingredients: vec![req(2001, 3, 55, 0), req(2002, 1, 56, 0)],
            },
        );
        state.selected = Some(SelectedRecipe {
            recipe_id: 5,
            name: "测试产物".to_string(),
        });
        let info = selected_recipe_info(&state).expect("配方详情必须可查");
        assert_eq!(info.gold, 250);
        assert_eq!(info.tools.len(), 1);
        assert_eq!(info.ingredients.len(), 2);

        // 未下发的配方 id → 无详情（不会 panic）
        state.selected = Some(SelectedRecipe {
            recipe_id: 99,
            name: "未下发".to_string(),
        });
        assert!(selected_recipe_info(&state).is_none());
    }
}

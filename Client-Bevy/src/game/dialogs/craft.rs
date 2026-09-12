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
use crate::ui::theme::{load_lib_image, spawn_icon_button, spawn_label, spawn_panel};

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
const CRAFT_W: f32 = 337.0;
const CRAFT_H: f32 = 215.0;
/// C# `CraftDialog.Show()`（NPCDialogs.cs:2448）：
/// `Location = (InventoryDialog.X - 12, InventoryDialog.Y + 236)`。
const CRAFT_REL_X: f32 = -12.0;
const CRAFT_REL_Y: f32 = 236.0;
/// C# 控件锚点（NPCDialogs.cs:2280-2391）。
const CRAFT_TITLE: (f32, f32) = (28.0, 8.0); // Title[18] 57x15
const CRAFT_RECIPE_LABEL: (f32, f32) = (22.0, 5.0); // RecipeLabel
const CRAFT_MESSAGE_LABEL: (f32, f32) = (10.0, 135.0); // PossibilityLabel
const CRAFT_GOLD_LABEL: (f32, f32) = (30.0, 190.0); // GoldLabel
const CRAFT_CLOSE_POS: (f32, f32) = (312.0, 3.0); // CloseButton（Prguse2[360..362] 24x21）
const CRAFT_AUTOFILL_POS: (f32, f32) = (165.0, 185.0); // AutoFillButton（Title[180..182] 48x25）
const CRAFT_CONFIRM_POS: (f32, f32) = (215.0, 185.0); // CraftButton（Title[336..338] 80x25）
/// 精灵首帧索引（Index/HoverIndex/PressedIndex 连续 3 帧）。
const CRAFT_AUTOFILL_INDEX: usize = 180;
const CRAFT_CONFIRM_INDEX: usize = 336;

#[derive(Component)]
pub struct CraftWidget;

#[derive(Component)]
pub struct CraftClose;

#[derive(Component)]
pub struct CraftBtn;

/// 自动填充（C# `AutoFillButton`：按配方工具/材料从背包自动摆放）
#[derive(Component)]
pub struct CraftAutoFill;

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
            craft_ui_system.run_if(in_state(AppState::Game)),
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
                .insert(CraftBtn);
        }
    });
}

/// 显隐 + 渲染 + 选择联动 + 合成
fn craft_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut state: ResMut<CraftState>,
    mut npc_goods: ResMut<NpcGoodsState>,
    net: Res<NetConnection>,
    inv_origin: Res<InventoryOrigin>,
    close: Query<(Entity, &Interaction), With<CraftClose>>,
    craft_btn: Query<(Entity, &Interaction), With<CraftBtn>>,
    autofill_btn: Query<(Entity, &Interaction), With<CraftAutoFill>>,
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
                None => "材料槽待移植：当前服务端按配方自动扣材".to_string(),
            },
            3 => format!("已学会配方: {} 种", state.learned.len()),
            _ => String::new(),
        };
    }
    // 自动填充（C# AutoFill）：需要配方 Tools/Ingredients 全量数据
    // （S.NewRecipeInfo 目前只下发 recipe_id），协议扩展前仅提示。
    for (e, inter) in &autofill_btn {
        if edge(e, inter, &mut prev_inter) {
            state.message = match selected_recipe_info(&state) {
                Some(info) => format!(
                    "自动填充：需 {} 个工具 / {} 种材料（摆槽 UI 待移植）",
                    info.tools.len(),
                    info.ingredients.len()
                ),
                None => "请先选择合成产物".to_string(),
            };
            tracing::info!("🔧 自动填充：配方数据已就绪，摆槽 UI 待移植");
        }
    }
    // 合成
    for (e, inter) in &craft_btn {
        if edge(e, inter, &mut prev_inter) {
            if let Some(r) = state.selected.clone() {
                // #2573：C# C.CraftItem wire（UniqueID/Count/Slots；暂无材料槽选择 UI，
                // 槽位空 → 服务端按 DB 配方自动扣材）
                net.send_packet(&crate::network::CraftItemWire {
                    unique_id: r.recipe_id as u64,
                    count: 1,
                    slots: Vec::new(),
                });
                state.message = format!("合成 {} 中…", r.name);
                tracing::info!("🔧 合成配方 {}（{}）", r.recipe_id, r.name);
            } else {
                state.message = "请先在左侧商品列表点击合成产物".to_string();
            }
        }
    }
}


/// 消费服务端合成事件（网络层只广播 ServerEvent；文案在此构造）
fn craft_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut craft: ResMut<CraftState>,
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mir2_shared::enums::PanelType;

    fn sel() -> Option<SelectedRecipe> {
        Some(SelectedRecipe {
            recipe_id: 7,
            name: "精铁剑".to_string(),
        })
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
        use mir2_shared::data::client_data::ClientRecipeInfo;
        use mir2_shared::data::item::UserItem;

        let mut state = CraftState::default();
        assert!(selected_recipe_info(&state).is_none());

        state.recipes.insert(
            5,
            ClientRecipeInfo {
                gold: 250,
                chance: 80,
                item: UserItem::new(9005),
                tools: vec![UserItem::new(1001)],
                ingredients: vec![UserItem::new(2001), UserItem::new(2002)],
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

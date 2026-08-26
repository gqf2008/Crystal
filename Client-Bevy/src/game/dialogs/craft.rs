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

use crate::game::dialogs::npc_goods::NpcGoodsState;
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::UiFont;
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

#[derive(Component)]
pub struct CraftWidget;

#[derive(Component)]
pub struct CraftClose;

#[derive(Component)]
pub struct CraftBtn;

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
    mut ui_font: ResMut<UiFont>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();

    // 面板 Prguse[170] @ (280,80)。加宽到 320x207：关闭按钮(300,3)+合成按钮都在面板内
    // （批 20 同款：244 宽会裁掉关闭按钮）
    let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 170) else {
        return;
    };
    let panel = spawn_panel(&mut commands, bg, 280.0, 80.0, 320.0, 207.0, 30);
    commands
        .entity(panel)
        .insert((DialogRoot(DialogKind::Craft), CraftWidget));

    commands.entity(panel).with_children(|p| {
        // 关闭 Prguse2[360/361/362] @(300,3)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 360),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 361),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 362),
        ) {
            spawn_icon_button(p, n, h, pr, 300.0, 3.0, 20.0, 20.0, 10).insert(CraftClose);
        }
        // 选中配方 + 结果消息 + 提示 + 已学会数 @(18,40+22i)
        for i in 0..4usize {
            spawn_label(p, &font, "", 18.0, 40.0 + i as f32 * 22.0, 12.0, Color::WHITE, 9)
                .insert(CraftLine(i));
        }
        // 合成按钮 Title[206/207/208] @(80,160)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 206),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 207),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 208),
        ) {
            spawn_icon_button(p, n, h, pr, 80.0, 160.0, 76.0, 25.0, 10).insert(CraftBtn);
        }
    });
}

/// 显隐 + 渲染 + 选择联动 + 合成
fn craft_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut state: ResMut<CraftState>,
    mut npc_goods: ResMut<NpcGoodsState>,
    net: Res<NetConnection>,
    close: Query<(Entity, &Interaction), With<CraftClose>>,
    craft_btn: Query<(Entity, &Interaction), With<CraftBtn>>,
    mut widgets: Query<&mut Visibility, With<CraftWidget>>,
    mut lines: Query<(&mut Text, &CraftLine)>,
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
            2 => "点击左侧产物选中 → 点合成".to_string(),
            3 => format!("已学会配方: {} 种", state.learned.len()),
            _ => String::new(),
        };
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
        if let ServerEvent::RecipeLearned { recipe_id } = ev {
            // #262：学会配方
            if !craft.learned.contains(recipe_id) {
                craft.learned.push(*recipe_id);
            }
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
}

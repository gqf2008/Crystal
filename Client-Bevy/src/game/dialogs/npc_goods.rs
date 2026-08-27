// ============================================================================
// NPC 商店（M9 第 2 批收尾）
// 布局参考：C# NPCDialogs.cs NPCGoodsDialog / macroquad npc_goods_dialog.rs
//   - 背景 Prguse[1000]，位置 (0,224)，8 行商品列表
//   - 购买按钮 Title[312-314]；关闭 Prguse2[360-362]
// 网络：NPCGoods（商品列表，含 ItemInfo）→ 显示；BuyItem 购买
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::amount_box::{AmountBoxResult, AmountBoxState};
use crate::game::dialogs::DialogRoot;
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::UiFont;
use crate::ui::theme::{
    load_lib_image, spawn_icon_button, spawn_item_cell_ui, spawn_label, spawn_panel,
    spawn_scroll_bar_ui, UiItemCellData, UiScrollList,
};

/// 商品条目
#[derive(Debug, Clone)]
pub struct GoodsEntry {
    pub item_index: i32,
    /// 物品唯一 ID（回购列表原物品携带；普通商店商品为 0）
    pub unique_id: u64,
    pub name: String,
    pub price: u32,
    pub count: u16,
    /// Items 库图标帧（ItemInfo.image）
    pub image: u16,
    /// 物品类型（ItemType 枚举值，Tooltip 用）
    pub item_type: u8,
    /// 服务端物品描述（ItemInfo.tool_tip）
    pub tool_tip: Option<String>,
    /// 单组堆叠上限（ItemInfo.stack_size；C# BuyItem 用 StackSize>1 决定弹数量框）
    pub stack_size: u16,
}

/// 待确认的购买（数量框 OK 后发送，C# BuyItem amountBox.Amount）
#[derive(Debug, Clone, Copy)]
pub struct NpcBuyPending {
    pub item_index: u32,
    pub unique_id: u64,
    pub is_buyback: bool,
}

/// NPC 商店状态
#[derive(Resource)]
pub struct NpcGoodsState {
    pub visible: bool,
    pub title: String,
    pub goods: Vec<GoodsEntry>,
    pub selected: Option<usize>,
    /// 当前面板是否为回购列表（客户端按菜单项设置；购买按钮据此发 BuyItemBack）
    pub is_buyback: bool,
    /// 数量框待确认购买（C# amountBox.OKButton）
    pub pending_buy: Option<NpcBuyPending>,
    /// #珍珠商店：是否珍珠购买模式（C# NPCGoodsDialog.UsePearls；显示珍珠价，购买包不变）
    pub use_pearls: bool,
    /// #2536：当前面板类型（Craft → 合成产物列表；C# NPCGoodsDialog.PType）
    pub panel: mir2_shared::enums::PanelType,
    /// #2536：待合成对话框消费的选择（商品行点击 → (recipe_id, 产物名)；
    /// C# NPCDialogs.cs:1090 CraftDialog.ResetCells/RefreshCraftCells/Show）
    pub craft_pick: Option<(u32, String)>,
}

impl Default for NpcGoodsState {
    fn default() -> Self {
        Self {
            visible: false,
            title: String::new(),
            goods: Vec::new(),
            selected: None,
            is_buyback: false,
            pending_buy: None,
            use_pearls: false,
            panel: mir2_shared::enums::PanelType::Buy,
            craft_pick: None,
        }
    }
}

/// #2536：购买按钮显隐（C# NPCDialogs.cs:1142 Craft 面板 BuyButton.Visible=false）
fn buy_button_visible(state: &NpcGoodsState) -> bool {
    state.visible && state.panel != mir2_shared::enums::PanelType::Craft
}

/// 购买数量上限（C# BuyItem：max = min(StackSize, 库存)；非堆叠 = 1）
fn buy_max_quantity(stack_size: u16, stock: u16) -> u32 {
    if stack_size > 1 {
        (stock as u32).min(stack_size as u32).max(1)
    } else {
        1
    }
}

#[derive(Component)]
pub struct NpcGoodsWidget;

#[derive(Component)]
pub struct NpcGoodsClose;

#[derive(Component)]
pub struct NpcGoodsBuy;

#[derive(Component)]
pub struct NpcGoodsLine(usize);

/// 商品图标格（通用 ItemCell，带行号）
#[derive(Component)]
pub struct NpcGoodsCell(usize);

pub struct NpcGoodsPlugin;

impl Plugin for NpcGoodsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NpcGoodsState>();
                app.add_systems(
            Update,
            npc_goods_server_events.run_if(in_state(AppState::Game)),
        );
app.add_systems(OnEnter(AppState::Game), spawn_npc_goods);
        app.add_systems(OnExit(AppState::Game), cleanup_npc_goods);
        app.add_systems(
            Update,
            npc_goods_ui_system.run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_npc_goods(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_npc_goods(
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

    // 背景 Prguse[1000]（C# NPCGoodsDialog Index=1000，244x334 @ (0,224)）
    let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 1000) else {
        return;
    };
    let panel = spawn_panel(&mut commands, bg, 0.0, 224.0, 244.0, 334.0, 30);
    commands.entity(panel).insert((
        DialogRoot(crate::game::dialogs::DialogKind::NpcGoods),
        NpcGoodsWidget,
        // #124 长商品列表滚轮滚动（C# Up/Down；本实现用滑块条，位置在面板内右侧）
        UiScrollList {
            rect_rel: (10.0, 16.0, 230.0, 176.0),
            row_h: 22.0,
            visible: 8,
            total: 0,
            offset: 0,
            step: 3,
            track_rel: (220.0, 16.0, 4.0, 176.0),
            thumb: None,
            z: 9,
        },
    ));

    commands.entity(panel).with_children(|p| {
        // 滚动条（轨道+滑块，面板子节点，面板内右侧）
        spawn_scroll_bar_ui(p, (220.0, 16.0, 4.0, 176.0), 9);
        // 关闭按钮（C# (217,3)）
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 360),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 361),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 362),
        ) {
            spawn_icon_button(p, n, h, pr, 217.0, 3.0, 20.0, 20.0, 10).insert(NpcGoodsClose);
        }
        // 购买按钮（C# (77,304)）
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 312),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 313),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 314),
        ) {
            spawn_icon_button(p, n, h, pr, 77.0, 304.0, 76.0, 25.0, 10).insert(NpcGoodsBuy);
        }
        // 8 行商品（#110：左侧通用 UiItemCell 图标 + 右侧名称/价格文本，对齐 C# MirGoodsCell）
        for i in 0..8usize {
            let y = 16.0 + i as f32 * 22.0;
            spawn_item_cell_ui(p, &mut images, &font, 10.0, y, 32.0, 20.0, 9, i)
                .insert(NpcGoodsCell(i));
            spawn_label(p, &font, "", 48.0, y + 2.0, 12.0, Color::WHITE, 9)
                .insert(NpcGoodsLine(i));
        }
    });
}

/// 显示/隐藏 + 商品列表渲染 + 选中/购买/关闭
#[allow(clippy::type_complexity)]
fn npc_goods_ui_system(
    mut state: ResMut<NpcGoodsState>,
    mut amount: ResMut<AmountBoxState>,
    mut result: MessageReader<AmountBoxResult>,
    net: Res<NetConnection>,
    mouse: Res<ButtonInput<MouseButton>>,
    ui: (
        Query<&Window>,
        Query<&Node, With<NpcGoodsWidget>>,
    ),
    close: Query<(Entity, &Interaction), (With<NpcGoodsClose>, Without<NpcGoodsBuy>)>,
    mut buy: Query<(Entity, &Interaction, &mut Visibility), (With<NpcGoodsBuy>, Without<NpcGoodsClose>)>,
    mut widgets: Query<&mut Visibility, (With<NpcGoodsWidget>, Without<NpcGoodsBuy>)>,
    mut lines: Query<(&mut Text, &NpcGoodsLine)>,
    mut cells: Query<(&mut UiItemCellData, &NpcGoodsCell)>,
    mut scroll: Query<&mut UiScrollList, With<NpcGoodsWidget>>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut tooltip: ResMut<crate::ui::tooltip::TooltipState>,
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
    for mut vis in widgets.iter_mut() {
        *vis = if state.visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    // #2536：Craft 面板隐藏购买按钮（C# NPCDialogs.cs:1142）
    let buy_vis = buy_button_visible(&state);
    for (_, _, mut vis) in &mut buy {
        *vis = if buy_vis {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !state.visible {
        return;
    }
    // 数量框结果：OK → 按数量发送购买/回购（C# BuyItem Count=amountBox.Amount）
    for r in result.read() {
        let Some(pending) = state.pending_buy.take() else { continue };
        let Some(n) = r.0 else { continue };
        if n == 0 {
            continue;
        }
        if pending.is_buyback && pending.unique_id != 0 {
            net.send_packet(&mir2_shared::packets::client::npc::BuyItemBack {
                unique_id: pending.unique_id,
                count: n as u16,
            });
            tracing::info!("🔄 回购 uid={} x{}", pending.unique_id, n);
        } else {
            // #2376：发 UniqueID（C# 客户端 BuyItem.ItemIndex = SelectedItem.UniqueID）；
            // 常规商店商品服务端 unique_id=item_index，二手货为实例 id，服务端据此区分
            net.send_packet(&mir2_shared::packets::client::npc::BuyItem {
                item_index: pending.unique_id as u64,
                count: n as u16,
                panel_type: mir2_shared::enums::PanelType::Buy,
            });
            tracing::info!("🏪 购买 item={} x{}", pending.item_index, n);
        }
    }

    // 商品行（名称 + 价格，#124 支持滚轮滚动）
    {
        let mut sl = scroll.single_mut();
        if let Ok(sl) = sl.as_mut() {
            sl.set_total(state.goods.len());
        }
    }
    let off = scroll.single().map(|s| s.offset).unwrap_or(0);
    for (mut text, line) in &mut lines {
        if let Some(g) = state.goods.get(off + line.0) {
            // #2536：Craft 面板行是合成产物（不标价；价格由配方金币决定，服务端校验）
            text.0 = if state.panel == mir2_shared::enums::PanelType::Craft {
                format!("合成 {} x{}", g.name, g.count)
            } else if state.use_pearls {
                format!("{} x{}  {} 珍珠", g.name, g.count, g.price)
            } else {
                format!("{} x{}  {} 金", g.name, g.count, g.price)
            };
        } else {
            text.0 = String::new();
        }
    }

    // 商品图标（#110 通用 ItemCell 数据驱动渲染）
    for (mut data, cell) in &mut cells {
        let g = state.goods.get(off + cell.0);
        let icon = g.and_then(|g| {
            load_lib_image(
                &mut libs,
                &mut images,
                LibraryName::Items,
                g.image as usize,
            )
        });
        let count = g.map(|g| g.count.max(1) as u32);
        // 性能（#112）：无变化不写
        if data.icon.as_ref() != icon.as_ref() {
            data.icon = icon;
        }
        if data.count != count {
            data.count = count;
        }
    }

    // 悬停商品行 → 通用 Tooltip（#110）
    let Ok(window) = ui.0.single() else { return };
    let Some(cursor) = window.cursor_position() else { return };
    let mut hovered: Option<&GoodsEntry> = None;
    let (ox, oy) = ui
        .1
        .single()
        .map(|n| crate::ui::theme::node_origin(n, (0.0, 224.0)))
        .unwrap_or((0.0, 224.0));
    for i in 0..8usize {
        let y = oy + 16.0 + i as f32 * 22.0;
        if cursor.x >= ox + 12.0 && cursor.x <= ox + 480.0 && cursor.y >= y && cursor.y <= y + 18.0 {
            hovered = state.goods.get(off + i);
            break;
        }
    }
    if let Some(g) = hovered {
        let mut lines = vec![if state.use_pearls {
            format!("价格: {} 珍珠", g.price)
        } else {
            format!("价格: {} 金", g.price)
        }];
        lines.push(format!("类型: {}", crate::game::dialogs::inventory::item_type_name(g.item_type)));
        if let Some(t) = &g.tool_tip {
            if !t.is_empty() {
                lines.push(t.clone());
            }
        }
        tooltip.update(5, true, g.name.clone(), lines, cursor.x, cursor.y);
    } else {
        tooltip.update(5, false, String::new(), Vec::new(), cursor.x, cursor.y);
    }

    // 点击行选中
    let Ok(window) = ui.0.single() else { return };
    let Some(cursor) = window.cursor_position() else { return };
    if mouse.just_pressed(MouseButton::Left) {
        for i in 0..8usize {
            let y = oy + 16.0 + i as f32 * 22.0;
            if cursor.x >= ox + 12.0 && cursor.x <= ox + 480.0 && cursor.y >= y && cursor.y <= y + 18.0 {
                let idx = off + i;
                if idx < state.goods.len() {
                    state.selected = Some(idx);
                    // #2536：Craft 面板点击行 → 交给合成对话框选择配方
                    // （C# NPCDialogs.cs:1090 CraftDialog.ResetCells/RefreshCraftCells/Show）
                    if state.panel == mir2_shared::enums::PanelType::Craft {
                        // unique_id = recipe_id（服务端 send_craft_goods 下发）
                        let (recipe_id, name) = {
                            let g = &state.goods[idx];
                            (g.unique_id as u32, g.name.clone())
                        };
                        state.craft_pick = Some((recipe_id, name.clone()));
                        tracing::debug!("🔧 选中合成产物: {}", name);
                    } else {
                        tracing::debug!("🏪 选中商品: {}", state.goods[idx].name);
                    }
                }
                break;
            }
        }
    }

    // 关闭
    for (e, inter) in &close {
        if edge(e, inter, &mut prev_inter) {
            state.visible = false;
            state.selected = None;
            state.craft_pick = None;
        }
    }
    // 购买/回购（原版 C# NPCGoodsDialog 购买按钮 → C.BuyItem；回购面板 → C.BuyItemBack）
    for (e, inter, _) in &buy {
        if edge(e, inter, &mut prev_inter) {
            // #2536：Craft 面板无购买（C# NPCDialogs.cs:1104 DoubleClick return；按钮已隐藏）
            if state.panel == mir2_shared::enums::PanelType::Craft {
                continue;
            }
            if amount.visible {
                // 数量框打开期间忽略（C# Modal）
                continue;
            }
            if let Some(idx) = state.selected {
                if let Some(g) = state.goods.get(idx).cloned() {
                    if state.is_buyback && g.unique_id != 0 {
                        // 回购：可堆叠也弹数量框（C# 语义一致）
                        if g.stack_size > 1 {
                            state.pending_buy = Some(NpcBuyPending {
                                item_index: g.item_index as u32,
                                unique_id: g.unique_id,
                                is_buyback: true,
                            });
                            amount.ask(format!("回购 {} 数量", g.name), buy_max_quantity(g.stack_size, g.count));
                        } else {
                            net.send_packet(&mir2_shared::packets::client::npc::BuyItemBack {
                                unique_id: g.unique_id,
                                count: 1,
                            });
                            tracing::info!("🔄 回购 {} (uid={})", g.name, g.unique_id);
                        }
                    } else if g.stack_size > 1 {
                        // 堆叠商品 → 数量框（C# BuyItem：StackSize>1 弹 MirAmountBox）
                        state.pending_buy = Some(NpcBuyPending {
                            item_index: g.item_index as u32,
                            unique_id: g.unique_id,
                            is_buyback: false,
                        });
                        amount.ask(format!("购买 {} 数量", g.name), buy_max_quantity(g.stack_size, g.count));
                        tracing::info!("🏪 购买 {}: 弹数量框 max={}", g.name, buy_max_quantity(g.stack_size, g.count));
                    } else {
                        // #2376：发 UniqueID（见上）
                        net.send_packet(&mir2_shared::packets::client::npc::BuyItem {
                            item_index: g.unique_id as u64,
                            count: 1,
                            panel_type: mir2_shared::enums::PanelType::Buy,
                        });
                        tracing::info!("🏪 购买 {} (item_index={})", g.name, g.item_index);
                    }
                }
            }
        }
    }
}


/// 消费服务端 NPC 商品事件（网络层只广播 ServerEvent）
fn npc_goods_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut npc_goods: ResMut<NpcGoodsState>,
    mut mgr: ResMut<crate::game::dialogs::DialogManager>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        if let ServerEvent::NpcGoods { goods, panel, .. } = ev {
            npc_goods.goods = goods.clone();
            npc_goods.selected = None;
            npc_goods.visible = true;
            npc_goods.use_pearls = false;
            npc_goods.panel = *panel;
            // #2536：Craft 面板到达 → 同时打开合成对话框
            // （C# GameScene.cs:4215 NPCCraftGoodsDialog.Show() + CraftDialog.Show()）
            if *panel == mir2_shared::enums::PanelType::Craft {
                mgr.open(crate::game::dialogs::DialogKind::Craft);
            }
        }
        if let ServerEvent::PearlShop { goods, .. } = ev {
            // #珍珠商店：C# NPCPearlGoods → UsePearls=true
            npc_goods.goods = goods.clone();
            npc_goods.selected = None;
            npc_goods.visible = true;
            npc_goods.use_pearls = true;
            npc_goods.panel = mir2_shared::enums::PanelType::Buy;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buy_max_quantity_non_stackable_is_one() {
        assert_eq!(buy_max_quantity(1, 99), 1);
        assert_eq!(buy_max_quantity(0, 5), 1);
    }

    #[test]
    fn buy_max_quantity_stackable_caps_by_stock_and_stack() {
        assert_eq!(buy_max_quantity(10, 5), 5); // 库存 5 < 堆叠 10
        assert_eq!(buy_max_quantity(10, 99), 10); // 堆叠上限 10
    }

    /// #2536：Craft 面板隐藏购买按钮（C# NPCDialogs.cs:1142）
    #[test]
    fn craft_panel_hides_buy_button() {
        let mut s = NpcGoodsState::default();
        s.visible = true;
        assert!(buy_button_visible(&s));
        s.panel = mir2_shared::enums::PanelType::Craft;
        assert!(!buy_button_visible(&s));
        s.visible = false;
        assert!(!buy_button_visible(&s));
    }

    /// #2536：默认面板为 Buy（非合成面板不联动合成对话框关闭）
    #[test]
    fn default_panel_is_buy() {
        assert_eq!(
            NpcGoodsState::default().panel,
            mir2_shared::enums::PanelType::Buy
        );
    }
}


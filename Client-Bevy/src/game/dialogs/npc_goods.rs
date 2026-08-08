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
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiFont,
    UiImageCache,
};
use crate::ui::controls::{spawn_item_cell, ItemCellData, ItemCell};
use crate::ui::scroll_list::{spawn_scroll_bar, ScrollList};

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
#[derive(Resource, Default)]
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
            (npc_goods_ui_system, ui_button_system)
                .chain()
                .run_if(in_state(AppState::Game)),
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
    mut cache: ResMut<UiImageCache>,
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<UiFont>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();

    // 背景 Prguse[1000] 在 (0,224)
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 1000) {
        let e = spawn_ui_sprite(&mut commands, h, 0.0, 224.0, 6.0, 1.0);
        // #124 长商品列表滚轮滚动
        let (track, thumb) = spawn_scroll_bar(&mut commands, &mut images, (500.0, 240.0, 4.0, 176.0), 6.3);
        commands.entity(track).insert((
            DialogRoot(crate::game::dialogs::DialogKind::NpcGoods),
            NpcGoodsWidget,
            Visibility::Visible,
        ));
        commands.entity(thumb).insert((
            DialogRoot(crate::game::dialogs::DialogKind::NpcGoods),
            NpcGoodsWidget,
            Visibility::Visible,
        ));
        commands.entity(e).insert((
            DialogRoot(crate::game::dialogs::DialogKind::NpcGoods),
            NpcGoodsWidget,
            Visibility::Hidden,
            ScrollList {
                rect_rel: (12.0, 16.0, 470.0, 176.0),
                row_h: 22.0,
                visible: 8,
                total: 0,
                offset: 0,
                step: 3,
                track_rel: (500.0, 16.0, 4.0, 176.0),
                thumb: Some(thumb),
                z: 8.0,
            },
        ));
    }

    // 关闭按钮（右上）
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 360, 361, 362,
        500.0, 227.0, 7.0, 20.0, 20.0,
    ) {
        commands.entity(e).insert((
            NpcGoodsClose,
            DialogRoot(crate::game::dialogs::DialogKind::NpcGoods),
            NpcGoodsWidget,
        ));
    }

    // 购买按钮
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 312, 313, 314,
        420.0, 224.0 + 250.0, 7.0, 76.0, 25.0,
    ) {
        commands.entity(e).insert((
            NpcGoodsBuy,
            DialogRoot(crate::game::dialogs::DialogKind::NpcGoods),
            NpcGoodsWidget,
        ));
    }

    // 8 行商品（#110：左侧通用 ItemCell 图标 + 右侧名称/价格文本，对齐 C# MirGoodsCell）
    for i in 0..8usize {
        let y = 240.0 + i as f32 * 22.0;
        let cell = spawn_item_cell(&mut commands, &mut images, &font, 12.0, y, 7.8, 32.0, 20.0, i);
        commands.entity(cell).insert((
            NpcGoodsCell(i),
            DialogRoot(crate::game::dialogs::DialogKind::NpcGoods),
            NpcGoodsWidget,
        ));
        let e = spawn_ui_text(
            &mut commands, &font, "",
            50.0, y + 2.0,
            12.0, Color::WHITE, 8.0,
        );
        commands.entity(e).insert((
            NpcGoodsLine(i),
            DialogRoot(crate::game::dialogs::DialogKind::NpcGoods),
            NpcGoodsWidget,
        ));
    }
}

/// 显示/隐藏 + 商品列表渲染 + 选中/购买/关闭
fn npc_goods_ui_system(
    mut state: ResMut<NpcGoodsState>,
    mut amount: ResMut<AmountBoxState>,
    mut result: MessageReader<AmountBoxResult>,
    net: Res<NetConnection>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    close: Query<&UiButton, (With<NpcGoodsClose>, Without<NpcGoodsBuy>)>,
    buy: Query<&UiButton, (With<NpcGoodsBuy>, Without<NpcGoodsClose>)>,
    mut widgets: Query<&mut Visibility, With<NpcGoodsWidget>>,
    mut lines: Query<(&mut Text2d, &NpcGoodsLine)>,
    mut cells: Query<(&mut ItemCellData, &NpcGoodsCell)>,
    mut scroll: Query<&mut ScrollList, With<NpcGoodsWidget>>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<UiImageCache>,
    mut tooltip: ResMut<crate::ui::tooltip::TooltipState>,
) {
    for mut vis in widgets.iter_mut() {
        *vis = if state.visible {
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
            net.send_packet(&mir2_shared::packets::client::npc::BuyItem {
                item_index: pending.item_index as u64,
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
            text.0 = if state.use_pearls {
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
            ui_image(
                &mut libs,
                &mut images,
                &mut cache,
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
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else { return };
    let mut hovered: Option<&GoodsEntry> = None;
    for i in 0..8usize {
        let y = 240.0 + i as f32 * 22.0;
        if cursor.x >= 12.0 && cursor.x <= 480.0 && cursor.y >= y && cursor.y <= y + 18.0 {
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
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else { return };
    if mouse.just_pressed(MouseButton::Left) {
        for i in 0..8usize {
            let y = 240.0 + i as f32 * 22.0;
            if cursor.x >= 12.0 && cursor.x <= 480.0 && cursor.y >= y && cursor.y <= y + 18.0 {
                let idx = off + i;
                if idx < state.goods.len() {
                    state.selected = Some(idx);
                    tracing::debug!("🏪 选中商品: {}", state.goods[idx].name);
                }
                break;
            }
        }
    }

    // 关闭
    for btn in &close {
        if btn.clicked {
            state.visible = false;
            state.selected = None;
        }
    }
    // 购买/回购（原版 C# NPCGoodsDialog 购买按钮 → C.BuyItem；回购面板 → C.BuyItemBack）
    for btn in &buy {
        if btn.clicked {
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
                        net.send_packet(&mir2_shared::packets::client::npc::BuyItem {
                            item_index: g.item_index as u64,
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
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        if let ServerEvent::NpcGoods { goods, .. } = ev {
            npc_goods.goods = goods.clone();
            npc_goods.selected = None;
            npc_goods.visible = true;
            npc_goods.use_pearls = false;
        }
        if let ServerEvent::PearlShop { goods, .. } = ev {
            // #珍珠商店：C# NPCPearlGoods → UsePearls=true
            npc_goods.goods = goods.clone();
            npc_goods.selected = None;
            npc_goods.visible = true;
            npc_goods.use_pearls = true;
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
}


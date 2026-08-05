// ============================================================================
// NPC 商店（M9 第 2 批收尾）
// 布局参考：C# NPCDialogs.cs NPCGoodsDialog / macroquad npc_goods_dialog.rs
//   - 背景 Prguse[1000]，位置 (0,224)，8 行商品列表
//   - 购买按钮 Title[312-314]；关闭 Prguse2[360-362]
// 网络：NPCGoods（商品列表，含 ItemInfo）→ 显示；BuyItem 购买
// ============================================================================

use bevy::prelude::*;

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
        commands.entity(e).insert((
            DialogRoot(crate::game::dialogs::DialogKind::NpcGoods),
            NpcGoodsWidget,
            Visibility::Hidden,
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
    net: Res<NetConnection>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    close: Query<&UiButton, (With<NpcGoodsClose>, Without<NpcGoodsBuy>)>,
    buy: Query<&UiButton, (With<NpcGoodsBuy>, Without<NpcGoodsClose>)>,
    mut widgets: Query<&mut Visibility, With<NpcGoodsWidget>>,
    mut lines: Query<(&mut Text2d, &NpcGoodsLine)>,
    mut cells: Query<(&mut ItemCellData, &NpcGoodsCell)>,
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

    // 商品行（名称 + 价格）
    for (mut text, line) in &mut lines {
        if let Some(g) = state.goods.get(line.0) {
            text.0 = format!("{} x{}  {} 金", g.name, g.count, g.price);
        } else {
            text.0 = String::new();
        }
    }

    // 商品图标（#110 通用 ItemCell 数据驱动渲染）
    for (mut data, cell) in &mut cells {
        let g = state.goods.get(cell.0);
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
            hovered = state.goods.get(i);
            break;
        }
    }
    if let Some(g) = hovered {
        let mut lines = vec![format!("价格: {} 金", g.price)];
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
                if i < state.goods.len() {
                    state.selected = Some(i);
                    tracing::debug!("🏪 选中商品: {}", state.goods[i].name);
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
            if let Some(idx) = state.selected {
                if let Some(g) = state.goods.get(idx) {
                    if state.is_buyback && g.unique_id != 0 {
                        net.send_packet(&mir2_shared::packets::client::npc::BuyItemBack {
                            unique_id: g.unique_id,
                            count: 1,
                        });
                        tracing::info!("🔄 回购 {} (uid={})", g.name, g.unique_id);
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
        }
    }
}

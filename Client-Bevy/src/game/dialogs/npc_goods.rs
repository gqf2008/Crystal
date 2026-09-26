// ============================================================================
// NPC 商店（M9 第 2 批收尾）
// 布局参考：C# NPCDialogs.cs NPCGoodsDialog / macroquad npc_goods_dialog.rs
//   - 背景 Prguse[1000]，位置 (0,224)，8 行商品列表
//   - 购买按钮 Title[312-314]；关闭 Prguse2[360-362]
// 网络：NPCGoods（商品列表，含 ItemInfo）→ 显示；BuyItem 购买
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::amount_box::{AmountBoxResult, AmountBoxState};
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{shared_cjk_font, UiCjkFont, UiFont};
use crate::ui::theme::{
    load_lib_image, spawn_close_button, spawn_icon_button, spawn_item_cell_ui, spawn_label,
    spawn_panel, spawn_scroll_bar_ui, UiItemCellData, UiScrollList,
};

/// #2892 批B：面板精灵与 C# 原生尺寸/坐标（C# `NPCGoodsDialog.Index = 1000; Location = (0,224)`）
pub const PANEL: (LibraryName, usize) = (LibraryName::Prguse, 1000);
pub const PANEL_SIZE: (f32, f32) = (244.0, 334.0);
pub const PANEL_POS: (f32, f32) = (0.0, 224.0);
/// 关闭键 `Prguse2[360..362]` @(217,3)（`NPCDialogs.cs:1113-1114`，无 `Size` → 原生 24x21）
pub const CLOSE_POS: (f32, f32) = (217.0, 3.0);

/// 面板左上角的标题图：C# `BuyLabel = new MirImageControl { Index = 27; Library = Libraries.Title;
/// Location = new Point(20, 9) }`（`NPCDialogs.cs:1133-1139`）——不设 `Size` ⇒ 用美术原生 **37x14**。
/// 本端此前**没画这张图**（只画了面板/关闭/购买/列表），所以商品窗左上是空的。
///
/// 同一控件的**合成/加工**变体：`if (PType == PanelType.Craft) { BuyLabel.Index = 12; BuyButton.Visible = false; }`
/// （`:1141-1145`）——`Title[12]` 是那句 "CRAFT" 文案。本端商品窗只做买卖档，craft 档由独立
/// `craft.rs` 承担，故这里只落买卖档的 27。
pub const TITLE_IMG: (LibraryName, usize) = (LibraryName::Title, 27);
/// 合成/加工面板的同一张标签：`Title[12]`（C# `if (PType == PanelType.Craft) BuyLabel.Index = 12;`）
pub const TITLE_IMG_CRAFT: (LibraryName, usize) = (LibraryName::Title, 12);
pub const TITLE_POS: (f32, f32) = (20.0, 9.0);

/// 商品行几何 = C# `MirGoodsCell`（205x32）@ `Cells[i].Location = (10, 34 + i*33)`
/// （`Client/MirScenes/Dialogs/NPCDialogs.cs:1074-1084`、`Client/MirControls/MirGoodsCell.cs:20`）。
///
/// 此前本端按 macroquad 版压成 22px 行距、从 y=16 起 → 8 行只占面板上半，
/// 滚轮命中区跟着短了一截（owner 队列 `scroll-hitrect-npcgoods`）。
pub const ROW_X: f32 = 10.0;
pub const ROW_Y0: f32 = 34.0;
pub const ROW_PITCH: f32 = 33.0;
pub const ROW_H: f32 = 32.0;
pub const ROW_W: f32 = 205.0;
pub const ROW_COUNT: usize = 8;
/// 图标盒：C# `DrawItem` 把物品图**居中**画在 40x32 盒里（`MirGoodsCell.cs:139-141`），
/// 故 32x32 图标落在 `(ROW_X + 4, y)`；本端 cell 自带 2px 内缩，取 32x32 不拉伸。
pub const ICON_DX: f32 = 4.0;
/// 价格/名称列：C# `NameLabel@(44,0)` / `PriceLabel@(44,14)` 是**相对 cell** 的（`:28/46`）
/// → 面板内 x = `ROW_X + 44`。
pub const LABEL_DX: f32 = 44.0;

/// 滚轮命中区 = 8 个 `Cells` 的**并集**（C# 只给 `Cells[i]` 挂了 `MouseWheel`，
/// `NPCDialogs.cs:1101`；面板自身没挂）→ x 10..215、y 34..297。
/// 注意 C# 的 `NameLabel/PriceLabel/CountLabel` 都是 `NotControl`，鼠标事件不落在它们上，
/// 所以宽度取 cell 的 205 而不是文本宽度。
pub const LIST_WHEEL_RECT: (f32, f32, f32, f32) = (
    ROW_X,
    ROW_Y0,
    ROW_W,
    ROW_PITCH * (ROW_COUNT as f32 - 1.0) + ROW_H,
);
/// 滚动条：C# `UpButton@(219,35)`、`DownButton@(219,284)`、`PositionBar` 行程 49..282
/// （`NPCDialogs.cs:1148-1196/1328-1330`）→ 轨道 y 49..282（233 高）。
pub const SCROLL_TRACK: (f32, f32, f32, f32) = (219.0, 49.0, 4.0, 233.0);

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

/// 左上角标题图（买卖档 `Title[27]` / 合成档 `Title[12]`，C# `BuyLabel`）——面板切换时改图
#[derive(Component)]
pub struct NpcGoodsTitle;

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
        app.init_resource::<UiCjkFont>();
        app.init_resource::<NpcGoodsState>();
        app.add_systems(
            Update,
            npc_goods_server_events.run_if(in_state(AppState::Game)),
        );
        app.add_systems(OnEnter(AppState::Game), spawn_npc_goods);
        app.add_systems(OnExit(AppState::Game), cleanup_npc_goods);
        app.add_systems(
            Update,
            (
                npc_goods_dialog_sync_system,
                npc_goods_ui_system,
                npc_goods_title_system,
            )
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

    // 背景 Prguse[1000]（C# NPCGoodsDialog Index=1000，244x334 @ (0,224)）
    let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 1000) else {
        return;
    };
    let panel = spawn_panel(
        &mut commands,
        bg,
        PANEL_POS.0,
        PANEL_POS.1,
        PANEL_SIZE.0,
        PANEL_SIZE.1,
        30,
    );
    commands.entity(panel).insert((
        DialogRoot(crate::game::dialogs::DialogKind::NpcGoods),
        NpcGoodsWidget,
        // #124 长商品列表滚轮滚动（C# Up/Down；本实现用滑块条，位置在面板内右侧）
        UiScrollList {
            rect_rel: LIST_WHEEL_RECT,
            row_h: ROW_PITCH,
            visible: ROW_COUNT,
            total: 0,
            offset: 0,
            // 每格 1 行：C# `NPCGoodsPanel_MouseWheel`（NPCDialogs.cs:1310-1318）
            // `int count = e.Delta / MouseWheelScrollDelta;` → `StartIndex -= count`
            step: 1,
            track_rel: SCROLL_TRACK,
            thumb: None,
            z: 9,
        },
    ));

    commands.entity(panel).with_children(|p| {
        // 滚动条（轨道+滑块，面板子节点，面板内右侧）
        spawn_scroll_bar_ui(p, SCROLL_TRACK, 9);
        // 关闭按钮（C# (217,3)）
        if let Some(mut btn) =
            spawn_close_button(p, &mut libs, &mut images, CLOSE_POS.0, CLOSE_POS.1, 10)
        {
            btn.insert(NpcGoodsClose);
        }
        // 标题图 Title[27] @(20,9)，按**美术原生尺寸**铺（C# 不设 Size）——缺帧要留痕，
        // 别像 NPC 箭头那样"静默没生成"（那次是库名写错、图是 0x0）
        match crate::ui::theme::spawn_image_native(
            p,
            &mut libs,
            &mut images,
            TITLE_IMG.0,
            TITLE_IMG.1,
            TITLE_POS.0,
            TITLE_POS.1,
            9,
        ) {
            // 打标记：面板在买卖/合成之间切换时改这张图（C# 是构造期定 Index，本端单实例切换）
            Some(mut e) => {
                e.insert((NpcGoodsTitle, NpcGoodsWidget));
            }
            None => tracing::warn!("🛒 商品窗：标题图缺帧（Title[27]）——左上角会空着"),
        }
        // 购买按钮（C# (77,304)）
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 312),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 313),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 314),
        ) {
            // `Title[312]` 图头 80x25（C# `NPCDialogs.cs:1122-1132` `BuyButton` 无显式 Size
            // ⇒ AutoSize 取美术尺寸；原写死 76x25 是错的）
            spawn_icon_button(p, n, h, pr, 77.0, 304.0, 80.0, 25.0, 10).insert(NpcGoodsBuy);
        }
        // 8 行商品（#110：左侧通用 UiItemCell 图标 + 右侧名称/价格文本，对齐 C# MirGoodsCell）
        for i in 0..ROW_COUNT {
            let y = ROW_Y0 + i as f32 * ROW_PITCH;
            spawn_item_cell_ui(p, &mut images, &cjk, ROW_X + ICON_DX, y, ROW_H, ROW_H, 9, i)
                .insert(NpcGoodsCell(i));
            // 名称/价格行：C# `NameLabel@(44,0)`（cell 相对）→ 面板内 (ROW_X+44, y)
            spawn_label(
                p,
                &cjk,
                "",
                ROW_X + LABEL_DX,
                y + 2.0,
                12.0,
                Color::WHITE,
                9,
            )
            .insert(NpcGoodsLine(i));
        }
    });
}

/// 显示/隐藏 + 商品列表渲染 + 选中/购买/关闭
#[allow(clippy::type_complexity)]
/// 商品行命中矩形（面板原点 ox/oy + 相对坐标；i 0..8）= C# `Cells[i]` 的矩形。
///
/// **单一来源**：渲染位置（`spawn_npc_goods` 的行循环）、悬停/工具提示命中、滚轮命中区
/// （[`LIST_WHEEL_RECT`]）三处此前各写一份且互相漂移（渲染 y=16+i*22 / 悬停宽度写成 468 /
/// 滚轮只到 192）——现在全部由下面这组常量导出。
fn npc_goods_row_rect(i: usize, ox: f32, oy: f32) -> (f32, f32, f32, f32) {
    (ox + ROW_X, oy + ROW_Y0 + i as f32 * ROW_PITCH, ROW_W, ROW_H)
}

fn npc_goods_dialog_sync_system(state: Res<NpcGoodsState>, mut mgr: ResMut<DialogManager>) {
    crate::game::dialogs::sync_dialog_state(&mut mgr, DialogKind::NpcGoods, state.visible);
}

/// 标题图随面板类型切换（买卖档 `Title[27]` ↔ 合成档 `Title[12]`）。
///
/// 原版是在**构造期**用 `PType` 定 `BuyLabel.Index`（`NPCDialogs.cs:1141-1145`），本端是单实例
/// 在买卖/合成之间切换，所以按状态改图才是等价做法（改图而不是重建实体，避免闪烁）。
/// 同一段 C# 还会 `BuyButton.Visible = false`（合成档没有购买钮）——本端此前只做到"点击被忽略"
/// （`npc_goods_ui_system` 里 craft 直接 continue），按钮**还看得见**，这里一并对齐。
fn npc_goods_title_system(
    state: Res<NpcGoodsState>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<crate::ui::sprite_ui::UiImageCache>,
    mut title: Query<&mut ImageNode, With<NpcGoodsTitle>>,
    mut buy: Query<&mut Visibility, With<NpcGoodsBuy>>,
) {
    let craft = state.panel == mir2_shared::enums::PanelType::Craft;
    let want = if craft { TITLE_IMG_CRAFT } else { TITLE_IMG };
    if let Some(h) =
        crate::ui::sprite_ui::ui_image(&mut libs, &mut images, &mut cache, want.0, want.1)
    {
        for mut img in &mut title {
            if img.image != h {
                img.image = h.clone();
            }
        }
    }
    for mut vis in &mut buy {
        let v = if craft {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
        if *vis != v {
            *vis = v;
        }
    }
}

fn npc_goods_ui_system(
    mut state: ResMut<NpcGoodsState>,
    mut amount: ResMut<AmountBoxState>,
    mut result: MessageReader<AmountBoxResult>,
    net: Res<NetConnection>,
    mouse: Res<ButtonInput<MouseButton>>,
    ui: (Query<&Window>, Query<&Node, With<NpcGoodsWidget>>),
    close: Query<(Entity, &Interaction), (With<NpcGoodsClose>, Without<NpcGoodsBuy>)>,
    mut buy: Query<
        (Entity, &Interaction, &mut Visibility),
        (With<NpcGoodsBuy>, Without<NpcGoodsClose>),
    >,
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
        let Some(pending) = state.pending_buy.take() else {
            continue;
        };
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
            load_lib_image(&mut libs, &mut images, LibraryName::Items, g.image as usize)
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
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let mut hovered: Option<&GoodsEntry> = None;
    let (ox, oy) =
        ui.1.single()
            .map(|n| crate::ui::theme::node_origin(n, (0.0, 224.0)))
            .unwrap_or((0.0, 224.0));
    for i in 0..8usize {
        let (rx, ry, rw, rh) = npc_goods_row_rect(i, ox, oy);
        if cursor.x >= rx && cursor.x <= rx + rw && cursor.y >= ry && cursor.y <= ry + rh {
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
        lines.push(format!(
            "类型: {}",
            crate::game::dialogs::inventory::item_type_name(g.item_type)
        ));
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
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    if mouse.just_pressed(MouseButton::Left) {
        for i in 0..8usize {
            let (rx, ry, rw, rh) = npc_goods_row_rect(i, ox, oy);
            if cursor.x >= rx && cursor.x <= rx + rw && cursor.y >= ry && cursor.y <= ry + rh {
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
                            amount.ask(
                                format!("回购 {} 数量", g.name),
                                buy_max_quantity(g.stack_size, g.count),
                            );
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
                        amount.ask(
                            format!("购买 {} 数量", g.name),
                            buy_max_quantity(g.stack_size, g.count),
                        );
                        tracing::info!(
                            "🏪 购买 {}: 弹数量框 max={}",
                            g.name,
                            buy_max_quantity(g.stack_size, g.count)
                        );
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
    /// 商品行命中：初始原点等价于原固定坐标，拖动后跟随面板
    #[test]
    fn row_rect_origin_and_drag() {
        // 初始 (0,224)：C# `Cells[0] @ (10, 34)` → 首行 (10, 258)，205x32
        let (rx, ry, rw, rh) = npc_goods_row_rect(0, 0.0, 224.0);
        assert_eq!((rx, ry, rw, rh), (10.0, 258.0, 205.0, 32.0));
        assert_eq!(npc_goods_row_rect(7, 0.0, 224.0).1, 258.0 + 7.0 * 33.0);
        // 拖动到 (50,250)：跟随
        let (rx2, ry2, _, _) = npc_goods_row_rect(0, 50.0, 250.0);
        assert_eq!((rx2, ry2), (60.0, 284.0));
    }

    /// 门禁（owner 队列 `scroll-hitrect-npcgoods`）：滚轮命中区 = 8 个 `Cells` 的**精确并集**
    /// （C# 只给 `Cells[i]` 挂 `MouseWheel`，`NPCDialogs.cs:1101`），且必须与渲染/悬停同一组常量。
    ///
    /// 阳性对照：① 把 `LIST_WHEEL_RECT` 换成旧值 `(10,16,230,176)` → 第 1/2 条断言红；
    /// ② 只改行距 `ROW_PITCH`（如退回 22）而不动命中区 → 第 1 条断言红（并集必须跟着走）。
    #[test]
    fn goods_wheel_rect_is_exact_union_of_cells() {
        // 并集恒等：命中区 == 首行左上角 + 末行右下角
        let (fx, fy, fw, fh) = npc_goods_row_rect(0, 0.0, 0.0);
        let (lx, ly, lw, lh) = npc_goods_row_rect(ROW_COUNT - 1, 0.0, 0.0);
        assert_eq!(
            (
                LIST_WHEEL_RECT.0,
                LIST_WHEEL_RECT.1,
                LIST_WHEEL_RECT.2,
                LIST_WHEEL_RECT.3
            ),
            (fx, fy, lw, (ly + lh) - fy),
            "命中区必须是 8 个 Cells 的精确并集（首行左上 → 末行右下）"
        );
        // C# 数字：x 10..215、y 34..297
        assert_eq!(LIST_WHEEL_RECT, (10.0, 34.0, 205.0, 263.0));
        assert_eq!(fy + LIST_WHEEL_RECT.3, 297.0, "末行底 = 34 + 7*33 + 32");
        // 旧命中区（压到 y 192 为止）必须不含末行 → 这正是「底部 2-3 行滚不到」的量
        let old_bottom = 16.0 + 176.0;
        assert!(
            old_bottom < ly + lh,
            "旧命中区底部 {old_bottom} 必须在末行底 {} 之上（否则本项不成立）",
            ly + lh
        );
        assert_eq!(fx, ROW_X, "命中区左边界 = cell 左边界");
    }

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

    /// 门禁（金标准逐窗复核 ⑧）：商品窗左上角的标题图 —— 买卖档 `Title[27]`、合成档 `Title[12]`，
    /// 都在 `(20,9)`；且合成档的购买钮要**隐藏**（C# 同一段里 `BuyButton.Visible = false`）。
    ///
    /// 依据：`NPCDialogs.cs:1133-1145`（`BuyLabel = new MirImageControl { Index = 27;
    /// Library = Libraries.Title; Location = new Point(20, 9) }`，`if (PType == PanelType.Craft)
    /// { BuyLabel.Index = 12; BuyButton.Visible = false; }`）。图头实测 `Title[27]` = 37x14。
    /// 本端此前**完全没画这张图**（商品窗左上角是空的）——只有把 C# 的控件清单逐条对过来才看得出。
    ///
    /// 阳性对照：把 `TITLE_IMG` 改成 `(LibraryName::Title, 0)` → 第一条断言红。
    #[test]
    fn npc_goods_title_variants_match_csharp() {
        assert_eq!(
            TITLE_IMG,
            (LibraryName::Title, 27),
            "买卖档标题是 Title[27]"
        );
        assert_eq!(
            TITLE_IMG_CRAFT,
            (LibraryName::Title, 12),
            "合成档标题是 Title[12]"
        );
        assert_eq!(TITLE_POS, (20.0, 9.0), "C# Location = (20,9)");
    }
}

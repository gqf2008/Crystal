// ============================================================================
// 背包对话框（M9 第一批）
// 布局参考：Client/MirScenes/Dialogs/InventoryDialog.cs
//   - 窗口位置 (182, 217)，背景 Title[196]
//   - 标签页：道具(6,7) / 道具2(76,7) / 任务(146,7)，72x23
//   - 关闭按钮 (289,3) Prguse2[360/361/362]
//   - 金币 (40,212) 111x14；负重 (268,212)
//   - 格子：8 列 x 5 行，cell 36x32，起点 (9,37)，x 间隔 1
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::amount_box::{AmountBoxResult, AmountBoxState};
use crate::game::dialogs::character;
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::game::chat::{ChatChannel, ChatState};
use crate::game::sound::{play_sound_cached, SoundBank, SoundCache};
use crate::game::hud::HudState;
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use mir2_shared::enums::MirGridType;

use crate::ui::controls::{spawn_item_cell, ItemCellData};
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiFont,
    UiImageCache,
};

/// 背包物品条目（网络 UserInformation 写入）
#[derive(Debug, Clone, Default)]
pub struct InvItem {
    pub unique_id: u64,
    pub item_index: i32,
    pub name: String,
    pub image: u16,
    pub count: u16,
    /// ItemType 枚举值（SharedRust），用于区分使用/装备
    pub item_type: u8,
    /// ItemInfo.shape（部分物品按形状区分用途）
    pub shape: i16,
    /// 当前耐久（M55 耐久面板用）
    pub current_dura: u16,
    /// 最大耐久
    pub max_dura: u16,
    /// 镶嵌宝石槽（M56 SocketDialog 用；长度=孔数，元素=宝石）
    pub slots: Vec<Option<InvItem>>,
    /// 属性（Stat 枚举值 → 数值；C# ItemInfo.Stats，tooltip 用）
    pub stats: Vec<(u8, i32)>,
    /// 需求类型（C# RequiredType 枚举值：Level=3 等）
    pub required_type: u8,
    /// 需求数值（等级/属性值）
    pub required_amount: u8,
    /// 需求职业位掩码（C# RequiredClass：战士1/法师2/道士4/刺客8/弓16）
    pub required_class: u8,
    /// 需求性别位掩码（C# RequiredGender：Male=1 Female=2，#1544）
    pub required_gender: u8,
    /// 灵魂绑定（C# UserItem.SoulBoundId：-1 未绑定；Rust 哨兵 1=已绑定本人，#1544）
    pub soul_bound_id: i32,
    /// 重量（C# ItemInfo.Weight）
    pub weight: u16,
    /// 价格（C# ItemInfo.Price）
    pub price: u32,
}

impl InvItem {
    /// 是否可装备（Weapon/Armour/Helmet/Torch/Necklace/Bracelet/Ring/Amulet/Belt/Boots/Stone/Mount）
    pub fn is_equipment(&self) -> bool {
        use mir2_shared::enums::ItemType;
        matches!(
            ItemType::try_from(self.item_type),
            Ok(ItemType::Weapon)
                | Ok(ItemType::Armour)
                | Ok(ItemType::Helmet)
                | Ok(ItemType::Torch)
                | Ok(ItemType::Necklace)
                | Ok(ItemType::Bracelet)
                | Ok(ItemType::Ring)
                | Ok(ItemType::Amulet)
                | Ok(ItemType::Belt)
                | Ok(ItemType::Boots)
                | Ok(ItemType::Stone)
                | Ok(ItemType::Mount)
        )
    }

    /// 计算装备目标槽位（ServerRust EquipmentSlot 值 0..13，见 actors/inventory.rs）
    pub fn equip_slot(&self) -> Option<i32> {
        self.equip_slot_occupied(|_| false)
    }

    /// 状态感知装备槽（#1174，对齐 C# MirItemCell 双击 UseItem）：
    /// - 固定槽不变；手镯/戒指优先右槽（空位），占位回退左槽；都占用则不装备（None）
    pub fn equip_slot_occupied(&self, occupied: impl Fn(usize) -> bool) -> Option<i32> {
        use mir2_shared::enums::ItemType;
        let t = ItemType::try_from(self.item_type).ok()?;
        let s: i32 = match t {
            ItemType::Weapon => 0,   // Weapon
            ItemType::Armour => 1,   // Armour
            ItemType::Helmet => 2,   // Helmet
            ItemType::Necklace => 3, // Necklace
            // C# Bracelet：优先 BraceletR（空位或装护身符），否则 BraceletL；都占用不装备
            ItemType::Bracelet => {
                if !occupied(5) {
                    5
                } else if !occupied(4) {
                    4
                } else {
                    return None;
                }
            }
            // C# Ring：优先 RingR（空位），否则 RingL；都占用不装备
            ItemType::Ring => {
                if !occupied(7) {
                    7
                } else if !occupied(6) {
                    6
                } else {
                    return None;
                }
            }
            ItemType::Amulet => 9,   // Pendant
            ItemType::Boots => 8,    // Shoes
            ItemType::Mount => 10,   // Mount
            // #1136：C# 补槽（SharedRust ItemType：Torch=15 / Belt=12 / Stone=14）
            ItemType::Torch => 11,   // Torch
            ItemType::Belt => 12,    // Belt
            ItemType::Stone => 13,   // Stone
            _ => return None,
        };
        Some(s)
    }

    /// 是否可双击使用（药水/卷轴/书/食物/脚本/宠物/变身/装饰等）
    pub fn is_usable(&self) -> bool {
        use mir2_shared::enums::ItemType;
        matches!(
            ItemType::try_from(self.item_type),
            Ok(ItemType::Potion)
                | Ok(ItemType::Scroll)
                | Ok(ItemType::Book)
                | Ok(ItemType::Food)
                | Ok(ItemType::Script)
                | Ok(ItemType::Pets)
                | Ok(ItemType::Transform)
                | Ok(ItemType::Deco)
                | Ok(ItemType::MonsterSpawn)
                | Ok(ItemType::SealedHero)
                | Ok(ItemType::Ore)
                | Ok(ItemType::Meat)
                | Ok(ItemType::CraftingMaterial)
                | Ok(ItemType::Gem)
                | Ok(ItemType::Fish)
        )
    }

}

/// 背包最大格数（C# Grid 8x10=80，扩容上限；超出部分不渲染）
pub const MAX_INV_SLOTS: usize = 80;
/// 背包扩容上限（C# CharacterInfo.ResizeInventory 上限 86，AddButton 满格隐藏）
pub const MAX_INV_EXPAND: usize = 86;

/// 背包数据（网络 UserInformation.inventory 写入）
#[derive(Resource, Default)]
pub struct InventoryState {
    /// 动态格数背包（默认 40，ResizeInventory 扩容/缩容，#276）
    pub items: Vec<Option<InvItem>>,
    pub gold: u32,
    pub weight: u32,
    pub max_weight: u32,
    /// 任务物品格（C# QuestInventory 40 格；UserInformation.quest_inventory 写入）
    pub quest_inventory: Vec<Option<InvItem>>,
    /// 当前背包页（0=道具 1=道具2 2=任务；#276 双页扩容）
    pub page: usize,
}

impl InventoryState {
    /// 按服务端 ResizeInventory 调整格数（C# Array.Resize：截断/补空，上限 MAX_INV_SLOTS）
    pub fn resize(&mut self, size: usize) {

        let size = size.min(MAX_INV_SLOTS);
        if size < self.items.len() {
            self.items.truncate(size);
        } else {
            self.items.resize(size, None);
        }
    }

    /// #1544：RefreshStats 重量（C# User.RefreshStats 从物品重量重算；max_weight 由服务端 bag_weight 提供）
    pub fn refresh_weight(&mut self) {
        let w: u32 = self
            .items
            .iter()
            .flatten()
            .map(|it| it.weight as u32 * it.count as u32)
            .sum();
        self.weight = w;
    }
}

const DIALOG_X: f32 = 182.0;
const DIALOG_Y: f32 = 217.0;
const GRID_COLS: usize = 8;
const GRID_ROWS: usize = 5;
const QUEST_GRID_SIZE: usize = GRID_COLS * GRID_ROWS; // 任务格 8x5=40（C# QuestInventory）
const CELL_W: f32 = 36.0;
const CELL_H: f32 = 32.0;

#[derive(Component)]
pub struct InventoryPanel;

/// 背包对话框内所有 UI 元素（统一显隐）
#[derive(Component)]
pub struct DialogWidget;

#[derive(Component)]
pub struct InvTab(pub usize); // 0=道具 1=道具2 2=任务（#1342 QuestGrid）

#[derive(Component)]
pub struct InvGoldText;

#[derive(Component)]
pub struct InvWeightText;

pub struct InventoryDialogPlugin;

impl Plugin for InventoryDialogPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InvClickState>();
        app.init_resource::<InvDropConfirm>();
        app.init_resource::<InvPendingAmount>();
        app.init_resource::<ItemUseFeedback>();
        app.add_systems(OnEnter(AppState::Game), spawn_inventory_dialog);
        app.add_systems(OnEnter(AppState::Game), spawn_inv_confirm);
        app.add_systems(OnExit(AppState::Game), cleanup_dialogs);
        app.add_systems(
            Update,
            (
                inv_grid_sync_system,
                inventory_ui_system,
                inv_selection_system,
                inv_tooltip_system,
                inv_socket_open_system,
                inv_item_action_system,
                inv_confirm_system,
                inv_add_del_buttons_system,
                quest_inventory_events,
                ui_button_system,
                inv_sound_system,
            )
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

/// #1342：GainedQuestItem/DeleteQuestItem 增量更新任务格（C# QuestInventory）
fn quest_inventory_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut hud: ResMut<HudState>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        match ev {
            ServerEvent::QuestItemGained { item } => {
                if let Some(slot) = hud.inventory.quest_inventory.iter_mut().find(|s| s.is_none()) {
                    *slot = Some(item.clone());
                } else {
                    hud.inventory.quest_inventory.push(Some(item.clone()));
                }
                hud.inventory.quest_inventory.truncate(QUEST_GRID_SIZE);
            }
            ServerEvent::QuestItemDeleted { unique_id, count } => {
                for slot in hud.inventory.quest_inventory.iter_mut() {
                    if let Some(it) = slot {
                        if it.unique_id == *unique_id {
                            if it.count > *count {
                                it.count -= *count;
                            } else {
                                *slot = None;
                            }
                            break;
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

fn cleanup_dialogs(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

/// 生成背包对话框实体（初始隐藏，由 HUD 按钮/管理器显示）
fn spawn_inventory_dialog(
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

    // 背景 Title[196]
    if let Some(h) = ui_image(
        &mut libs,
        &mut images,
        &mut cache,
        crate::resources::libraries::LibraryName::Title,
        196,
    ) {
        let e = spawn_ui_sprite(&mut commands, h, DIALOG_X, DIALOG_Y, 6.0, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::Inventory),
            InventoryPanel,
            DialogWidget,
            Visibility::Hidden,
        ));
    }

    // 标签页按钮（Title 737/197 道具，738/168 道具2，739/198 任务）
    // #1342：任务页签（QuestGrid 8x5，C# QuestInventory）
    let tabs: [(usize, usize, usize, f32); 3] = [
        (0, 737, 197, 6.0),
        (1, 738, 168, 76.0),
        (2, 739, 198, 146.0),
    ];
    for (idx, normal, hover, x) in tabs {
        if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
            &mut commands,
            &mut libs,
            &mut images,
            &mut cache,
            crate::resources::libraries::LibraryName::Title,
            normal,
            hover,
            hover,
            DIALOG_X + x,
            DIALOG_Y + 7.0,
            7.0,
            72.0,
            23.0,
        ) {
            commands.entity(e).insert((
                InvTab(idx),
                DialogRoot(DialogKind::Inventory),
                DialogWidget,
            ));
        }
    }

    // 关闭按钮（Prguse2 360/361/362）
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands,
        &mut libs,
        &mut images,
        &mut cache,
        crate::resources::libraries::LibraryName::Prguse2,
        360,
        361,
        362,
        DIALOG_X + 289.0,
        DIALOG_Y + 3.0,
        7.0,
        20.0,
        20.0,
    ) {
        commands
            .entity(e)
            .insert((InvCloseBtn, DialogRoot(DialogKind::Inventory), DialogWidget));
    }

    // 金币/负重文本
    let gold = spawn_ui_text(
        &mut commands,
        &font,
        "0",
        DIALOG_X + 40.0,
        DIALOG_Y + 210.0,
        12.0,
        Color::WHITE,
        8.0,
    );
    commands
        .entity(gold)
        .insert((InvGoldText, DialogRoot(DialogKind::Inventory), DialogWidget));
    let weight = spawn_ui_text(
        &mut commands,
        &font,
        "0/0",
        DIALOG_X + 268.0,
        DIALOG_Y + 210.0,
        12.0,
        Color::WHITE,
        8.0,
    );
    commands.entity(weight).insert((
        InvWeightText,
        DialogRoot(DialogKind::Inventory),
        DialogWidget,
    ));

    // 扩展背包格购买按钮（C# InventoryDialog AddButton：Title 483/484/485 @(235,5)）
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 483, 484, 485,
        DIALOG_X + 235.0, DIALOG_Y + 5.0, 7.0, 23.0, 23.0,
    ) {
        commands.entity(e).insert((InvAddBtn, DialogRoot(DialogKind::Inventory), DialogWidget));
    }
    // 删除模式按钮（C# InventoryDialog DelItemButton：Prguse2 366/367/368 @(291,212)）
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 366, 367, 368,
        DIALOG_X + 291.0, DIALOG_Y + 212.0, 7.0, 20.0, 20.0,
    ) {
        commands.entity(e).insert((InvDelBtn, DialogRoot(DialogKind::Inventory), DialogWidget));
    }

    // 格子背景不在此预生成：#276 由 inv_grid_sync_system 按 InventoryState.items.len()
    // 动态生成/移除（进图 UserInformation 到达前 items 为空，避免先建后删抖动）
}

#[derive(Component)]
struct InvCloseBtn;

/// #1346：扩展背包格购买按钮（C# InventoryDialog AddButton）
#[derive(Component)]
struct InvAddBtn;
/// #1346：删除模式按钮（C# InventoryDialog DelItemButton）
#[derive(Component)]
struct InvDelBtn;

/// 光标坐标 → 背包格（按当前页与格数）；供仓库/交易/英雄对话框复用。
/// 对齐 C# InventoryDialog：page 0=道具（0..min(40,size)），1=道具2（40..size-1），
/// 位置 (i%8, (i/8)%5) 复用同一 8x5 区域（C# Grid Location = y%5）。
pub fn inv_slot_at(cx: f32, cy: f32, page: usize, size: usize) -> Option<usize> {
    let size = size.min(MAX_INV_SLOTS);
    let range: std::ops::Range<usize> = match page {
        0 => 0..size.min(GRID_COLS * GRID_ROWS),
        1 => (GRID_COLS * GRID_ROWS)..size,
        _ => return None,
    };
    for i in range {
        let x = i % GRID_COLS;
        let y = (i / GRID_COLS) % GRID_ROWS;
        let sx = DIALOG_X + 9.0 + x as f32 * (CELL_W + 1.0);
        let sy = DIALOG_Y + 37.0 + y as f32 * (CELL_H + 1.0);
        if cx >= sx && cx <= sx + CELL_W && cy >= sy && cy <= sy + CELL_H {
            return Some(i);
        }
    }
    None
}

/// 背包格子索引（0..MAX_INV_SLOTS-1）
#[derive(Component, Clone, Copy)]
pub struct InvSlot(pub usize);
/// 双击检测（记录最近一次左键点击的格子与时间）
#[derive(Resource, Default)]
pub struct InvClickState {
    pub last: Option<(usize, f64)>,
    /// 当前选中格子（原版 C# GameScene.SelectedCell）
    pub selected: Option<usize>,
    /// 英雄背包选中格（#203：与主背包双向转移共用选择态）
    pub hero_selected: Option<usize>,
    /// #1346：删除模式（C# DelItemButton ToggleDeleteMode）
    pub delete_mode: bool,
}

/// 丢弃确认框（原版 C# MirMessageBox YesNo：DropTip）
#[derive(Resource, Default)]
pub struct InvDropConfirm {
    pub visible: bool,
    pub text: String,
    pub unique_id: u64,
    pub count: u16,
    /// #1346：0=丢弃 1=删除 2=背包扩容
    pub mode: u8,
}

/// 数量框待处理操作（拆分/丢弃，原版 C# MirAmountBox OK 回调）
#[derive(Resource, Default)]
pub struct InvPendingAmount {
    pub split_uid: Option<u64>,
    pub drop_uid: Option<u64>,
    /// #1346：删除数量框待确认物品
    pub delete_uid: Option<u64>,
}

#[derive(Component)]
pub struct InvConfirmWidget;

#[derive(Component)]
pub struct InvConfirmYes;

#[derive(Component)]
pub struct InvConfirmNo;

/// 显示/隐藏 + 页切换 + 关闭 + 物品图标渲染 + 双击使用/装备
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn inventory_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut hud: ResMut<HudState>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<UiImageCache>,
    // 背景/标签/关闭/格子 统一显隐（格子带 InvSlot）
    mut all_vis: Query<
        (&mut Visibility, Option<&InvSlot>),
        (
            With<DialogWidget>,
            Without<InvGoldText>,
            Without<InvWeightText>,
        ),
    >,
    mut cells_data: Query<(&InvSlot, &mut ItemCellData)>,
    buttons: Query<
        (&UiButton, Option<&InvTab>),
        (
            With<DialogWidget>,
            Without<InvSlot>,
            Without<InvGoldText>,
            Without<InvWeightText>,
        ),
    >,
    mut money: Query<
        (
            &mut Text2d,
            &mut Visibility,
            Option<&InvGoldText>,
            Option<&InvWeightText>,
        ),
        (
            With<DialogWidget>,
            Or<(With<InvGoldText>, With<InvWeightText>)>,

            Without<InvSlot>,
        ),
    >,
) {
    let inv = &hud.inventory;
    let open = mgr.is_open(DialogKind::Inventory);
    let size = inv.items.len().min(MAX_INV_SLOTS);
    // 格子弹页显隐（#276）：道具=0..min(40,size)，道具2=40..size-1，任务页=0..40（QuestGrid）
    for (mut vis, slot) in &mut all_vis {
        let visible = if !open {
            false
        } else {
            match slot {
                Some(s) => match inv.page {
                    0 => s.0 < size.min(GRID_COLS * GRID_ROWS),
                    1 => s.0 >= GRID_COLS * GRID_ROWS && s.0 < size,
                    // #1342：任务页签显示 QuestGrid 40 格（C# QuestInventory 8x5）
                    2 => s.0 < QUEST_GRID_SIZE,
                    _ => false,
                },
                None => true, // 背景/标签/关闭按钮
            }
        };
        *vis = if visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !open {
        return;
    }

    // 物品数据 → 通用 ItemCell（图标/数量/耐久条由 item_cell_system 渲染，#90 续）
    for (slot, mut data) in &mut cells_data {
        let item = if inv.page == 2 {
            inv.quest_inventory.get(slot.0).and_then(|s| s.as_ref())
        } else {
            inv.items.get(slot.0).and_then(|s| s.as_ref())
        };
        match item {
            Some(item) => {
                let handle = ui_image(
                    &mut libs,
                    &mut images,
                    &mut cache,
                    crate::resources::libraries::LibraryName::Items,
                    item.image as usize,
                );
                data.icon = handle;
                data.count = if item.count > 1 { Some(item.count as u32) } else { None };
                data.dura_ratio = if item.is_equipment() && item.max_dura > 0 {
                    Some((item.current_dura as f32 / item.max_dura as f32).clamp(0.0, 1.0))
                } else {
                    None
                };
            }
            None => {
                data.icon = None;
                data.count = None;
                data.dura_ratio = None;
            }
        }
    }
    // 标签页切换 / 关闭按钮
    for (btn, tab) in &buttons {
        if btn.clicked {
            match tab {
                Some(t) => {
                    hud.inventory.page = t.0;
                    tracing::debug!("背包页 -> {}", t.0);
                }
                None => mgr.close(DialogKind::Inventory),
            }
        }
    }
    for (mut t, _vis, gold, weight) in &mut money {
        if gold.is_some() {
            t.0 = format!("{}", hud.inventory.gold);
        } else if weight.is_some() {
            t.0 = format!("{}/{}", hud.inventory.weight, hud.inventory.max_weight);
        }
    }
}

/// 悬停提示系统（#93/#106 通用 Tooltip）：物品格上显示 名称 + 类型/数量/耐久
fn inv_tooltip_system(
    inv: Res<crate::game::hud::HudState>,
    mut tooltip: ResMut<crate::ui::tooltip::TooltipState>,
    windows: Query<&Window>,
    slots: Query<&InvSlot>,
) {
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else { return };

    let page = inv.inventory.page;
    let size = inv.inventory.items.len().min(MAX_INV_SLOTS);
    let mut hit: Option<InvItem> = None;
    for slot in &slots {
        let i = slot.0;
        // 只命中当前页可见格（#276）
        let visible = match page {
            0 => i < size.min(GRID_COLS * GRID_ROWS),
            1 => i >= GRID_COLS * GRID_ROWS && i < size,
            2 => i < QUEST_GRID_SIZE,
            _ => false,
        };
        if !visible {
            continue;
        }
        let x = i % GRID_COLS;
        let y = (i / GRID_COLS) % GRID_ROWS;
        let sx = DIALOG_X + 9.0 + x as f32 * (CELL_W + 1.0);
        let sy = DIALOG_Y + 37.0 + y as f32 * (CELL_H + 1.0);
        if cursor.x >= sx && cursor.x <= sx + CELL_W && cursor.y >= sy && cursor.y <= sy + CELL_H {
            hit = if page == 2 {
                inv.inventory.quest_inventory.get(i).and_then(|s| s.as_ref()).cloned()
            } else {
                inv.inventory.items.get(i).and_then(|s| s.as_ref()).cloned()
            };
            break;
        }
    }
    let Some(item) = hit else {
        tooltip.update(2, false, String::new(), Vec::new(), cursor.x, cursor.y);
        return;
    };
    let lines = item_tooltip_lines(&item);
    tooltip.update(2, true, item.name.clone(), lines, cursor.x, cursor.y);
}

/// 物品 tooltip 行（对齐 C# MirItemCell：成对属性合并 + 单项 + 需求 + 重量/价格）
pub fn item_tooltip_lines(item: &InvItem) -> Vec<String> {
    use mir2_shared::enums::Stat;
    let mut lines = Vec::new();
    if item.count > 1 {
        lines.push(format!("数量: {}", item.count));
    }
    lines.push(format!("类型: {}", item_type_name(item.item_type)));
    if item.is_equipment() {
        lines.push(format!("耐久: {}/{}", item.current_dura, item.max_dura));
    }
    let get = |s: Stat| {
        item.stats
            .iter()
            .find(|(k, _)| *k == s as u8)
            .map(|(_, v)| *v)
            .unwrap_or(0)
    };
    // 成对属性：最小-最大（C# 显示 防御 0-5 等）
    for (min, max, label) in [
        (Stat::MinAC, Stat::MaxAC, "防御"),
        (Stat::MinMAC, Stat::MaxMAC, "魔御"),
        (Stat::MinDC, Stat::MaxDC, "攻击"),
        (Stat::MinMC, Stat::MaxMC, "魔法"),
        (Stat::MinSC, Stat::MaxSC, "道术"),
    ] {
        let mn = get(min);
        let mx = get(max);
        if mn != 0 || mx != 0 {
            lines.push(format!("{}: {}-{}", label, mn, mx));
        }
    }
    // 单项属性
    for (stat, label, suffix) in [
        (Stat::Accuracy, "准确", ""),
        (Stat::Agility, "敏捷", ""),
        (Stat::Luck, "幸运", ""),
        (Stat::HP, "生命", ""),
        (Stat::MP, "魔法值", ""),
        (Stat::AttackSpeed, "攻速", ""),
        (Stat::Reflect, "反伤", ""),
        (Stat::Strong, "强度", ""),
        (Stat::Holy, "神圣", ""),
        (Stat::Freezing, "冰冻", ""),
        (Stat::PoisonAttack, "中毒攻击", ""),
        (Stat::MagicResist, "魔法抗性", ""),
        (Stat::PoisonResist, "中毒抗性", ""),
        (Stat::HealthRecovery, "生命恢复", ""),
        (Stat::SpellRecovery, "魔法恢复", ""),
        (Stat::PoisonRecovery, "中毒恢复", ""),
        (Stat::CriticalRate, "暴击率", "%"),
        (Stat::CriticalDamage, "暴击伤害", ""),
    ] {
        let v = get(stat);
        if v != 0 {
            lines.push(format!("{}: +{}{}", label, v, suffix));
        }
    }
    // 需求（C# RequiredType：Level=3 或属性值；RequiredClass 位掩码）
    if item.required_type == 3 && item.required_amount > 0 {
        lines.push(format!("需要等级: {}", item.required_amount));
    } else if item.required_amount > 0 {
        for (k, label) in [
            (Stat::MaxAC as u8, "防御"),
            (Stat::MaxMAC as u8, "魔御"),
            (Stat::MaxDC as u8, "攻击"),
            (Stat::MaxMC as u8, "魔法"),
            (Stat::MaxSC as u8, "道术"),
        ] {
            if item.required_type == k {
                lines.push(format!("需要{}: {}", label, item.required_amount));
                break;
            }
        }
    }
    if item.required_class != 0 {
        let mut names = Vec::new();
        for (bit, n) in [(1u8, "战士"), (2, "法师"), (4, "道士"), (8, "刺客"), (16, "弓箭手")] {
            if item.required_class & bit != 0 {
                names.push(n);
            }
        }
        if !names.is_empty() {
            lines.push(format!("需要职业: {}", names.join("/")));
        }
    }
    if item.weight > 0 {
        lines.push(format!("重量: {}", item.weight));
    }
    if item.price > 0 {
        lines.push(format!("价格: {} 金", item.price));
    }
    lines
}

/// ItemType 枚举 → 中文名（对齐 C# ItemInfo.Type 常见分类）
pub fn item_type_name(t: u8) -> &'static str {
    use mir2_shared::enums::ItemType;
    match ItemType::try_from(t) {
        Ok(ItemType::Weapon) => "武器",
        Ok(ItemType::Armour) => "护甲",
        Ok(ItemType::Helmet) => "头盔",
        Ok(ItemType::Necklace) => "项链",
        Ok(ItemType::Bracelet) => "手镯",
        Ok(ItemType::Ring) => "戒指",
        Ok(ItemType::Amulet) => "护身符",
        Ok(ItemType::Belt) => "腰带",
        Ok(ItemType::Boots) => "靴子",
        Ok(ItemType::Stone) => "宝石",
        Ok(ItemType::Torch) => "火把",
        Ok(ItemType::Potion) => "药水",
        Ok(ItemType::Ore) => "矿石",
        Ok(ItemType::Meat) => "肉",
        Ok(ItemType::CraftingMaterial) => "材料",
        Ok(ItemType::Scroll) => "卷轴",
        Ok(ItemType::Gem) => "宝石",
        Ok(ItemType::Mount) => "坐骑",
        Ok(ItemType::Book) => "书籍",
        Ok(ItemType::Script) => "脚本",
        Ok(ItemType::Reins) => "缰绳",
        Ok(ItemType::Bells) => "铃铛",
        Ok(ItemType::Saddle) => "马鞍",
        Ok(ItemType::Ribbon) => "饰带",
        Ok(ItemType::Mask) => "面具",
        Ok(ItemType::Food) => "食物",
        Ok(ItemType::Hook) => "鱼钩",
        Ok(ItemType::Float) => "浮漂",
        Ok(ItemType::Bait) => "鱼饵",
        Ok(ItemType::Finder) => "探鱼器",
        Ok(ItemType::Reel) => "渔轮",
        Ok(ItemType::Fish) => "鱼",
        Ok(ItemType::Quest) => "任务物品",
        Ok(ItemType::Awakening) => "觉醒",
        Ok(ItemType::Pets) => "宠物",
        Ok(ItemType::Transform) => "变身",
        Ok(ItemType::Deco) => "装饰",
        Ok(ItemType::Socket) => "镶嵌",
        Ok(ItemType::MonsterSpawn) => "召唤",
        _ => "其他",
    }

}

/// 背包动态格子同步（#276）：按 InventoryState.items.len() 生成/移除 InvSlot 格子。
/// 对齐 C# InventoryDialog.Grid（8x10，位置 y%5 复用）；缩容时移除多余格子。
#[allow(clippy::too_many_arguments)]
fn inv_grid_sync_system(
    mut commands: Commands,
    hud: Res<HudState>,
    mut images: ResMut<Assets<Image>>,
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<UiFont>,
    slots: Query<(Entity, &InvSlot)>,
) {
    let size = hud.inventory.items.len().min(MAX_INV_SLOTS);
    if hud.inventory.items.is_empty() && slots.is_empty() {
        return; // 进图 UserInformation 到达前：无格子可同步
    }
    // 缩容：移除超出 size 的格子
    for (e, s) in &slots {
        if s.0 >= size {
            commands.entity(e).despawn();
        }
    }
    let mut existing: Vec<usize> = slots
        .iter()
        .map(|(_, s)| s.0)
        .filter(|i| *i < size)
        .collect();
    existing.sort_unstable();
    if existing.len() == size {
        return;
    }
    // 扩容：补缺失格子
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();
    let mut next = 0usize;
    for i in 0..size {
        if existing.get(next).copied() == Some(i) {
            next += 1;
            continue;
        }
        let x = i % GRID_COLS;
        let y = (i / GRID_COLS) % GRID_ROWS;
        let sx = DIALOG_X + 9.0 + x as f32 * (CELL_W + 1.0);
        let sy = DIALOG_Y + 37.0 + y as f32 * (CELL_H + 1.0);
        let cell = spawn_item_cell(
            &mut commands,
            &mut images,
            &font,
            sx,
            sy,
            6.5,
            CELL_W,
            CELL_H,
            i,
        );
        commands
            .entity(cell)
            .insert((DialogRoot(DialogKind::Inventory), DialogWidget, InvSlot(i)));
    }
}

/// 选中格子高亮（原版 C# SelectedCell 黄色边框语义：用黄色半透明覆盖表示）
fn inv_selection_system(
    click: Res<InvClickState>,
    mut slots: Query<(&mut Sprite, &InvSlot)>,
) {
    for (mut sprite, slot) in &mut slots {
        let selected = click.selected == Some(slot.0);
        let target = if selected {
            Color::srgba(1.0, 0.9, 0.2, 0.35)
        } else {
            Color::srgba(0.0, 0.0, 0.0, 0.18)
        };
        if sprite.color != target {
            sprite.color = target;
        }
    }
}



/// #1544：消费物品使用反馈队列（PlayItemSound 音效 + CanUseItem 拒绝提示）
fn inv_sound_system(
    mut commands: Commands,
    mut assets: ResMut<Assets<AudioSource>>,
    bank: Res<SoundBank>,
    mut cache: ResMut<SoundCache>,
    mut feedback: ResMut<ItemUseFeedback>,
    mut chat: ResMut<ChatState>,
) {
    let sounds = std::mem::take(&mut feedback.sounds);
    for id in sounds {
        play_sound_cached(&mut commands, &mut assets, &bank, &mut cache, id);
    }
    for msg in std::mem::take(&mut feedback.messages) {
        chat.add_line(msg, crate::game::chat::chat_color(mir2_shared::enums::ChatType::System), ChatChannel::System);
    }
}
/// #1544：物品使用反馈队列（音效 + CanUseItem 拒绝提示，合并减少系统参数）
#[derive(Resource, Default)]
pub struct ItemUseFeedback {
    pub sounds: Vec<u32>,
    pub messages: Vec<String>,
    /// #1544：物品使用节流（C# GameScene.UseItemTime = CMain.Time + 300）
    pub last_use: f64,
}
/// #1544：腰带快捷使用（C# BeltDialog.Grid[i].UseItem → UseItem 守卫）：
///   节流 + 钓鱼限制（骑乘允许药水，C# 仅禁非 Scroll/Potion/Torch）
/// 返回 true = 已发包。
pub(crate) fn try_use_belt_item(
    uid: u64,
    net: &NetConnection,
    hud: &HudState,
    now: f64,
    feedback: &mut ItemUseFeedback,
) -> bool {
    if now < feedback.last_use {
        return false;
    }
    if hud.fishing {
        return false;
    }
    feedback.last_use = now + 0.3;
    net.send_packet(&mir2_shared::packets::client::item::UseItem { unique_id: uid });
    true
}

/// #1544：物品使用音效（C# MirItemCell.PlayItemSound → SoundList）
pub(crate) fn item_use_sound_id(item: &InvItem) -> Option<u32> {
    use mir2_shared::enums::ItemType;
    let t = ItemType::try_from(item.item_type).ok()?;
    Some(match t {
        ItemType::Weapon => 10111,    // ClickWeapon
        ItemType::Armour => 10112,    // ClickArmour
        ItemType::Helmet => 10116,    // ClickHelmet
        ItemType::Necklace => 10115,  // ClickNecklace
        ItemType::Bracelet => 10114,  // ClickBracelet
        ItemType::Ring => 10113,      // ClickRing
        ItemType::Boots => 10117,     // ClickBoots
        ItemType::Potion => 10108,    // ClickDrug
        _ => 10118,                   // ClickItem
    })
}

/// #1544：CanUseItem 客户端检查（C# MirItemCell.CanUseItem：性别/职业/等级）
/// 返回 Err(提示语) 时不应发包；服务端仍会二次校验（#576）。
fn can_use_item_check(item: &InvItem, gender: u8, class: u8, level: u16) -> Result<(), &'static str> {
    // 性别：RequiredGender Male=1 Female=2；0/3(NONE=both) 视为不限制
    let gbit = 1u8 << gender; // MirGender Male=0→1, Female=1→2
    if item.required_gender != 0
        && item.required_gender != 3
        && (item.required_gender & gbit) == 0
    {
        return Err("性别不符");
    }
    // 职业：RequiredClass Warrior=1 Wizard=2 Taoist=4 Assassin=8 Archer=16
    let cbit = 1u8 << class; // MirClass Warrior=0 Wizard=1 Taoist=2 Assassin=3 Archer=4
    if item.required_class != 0
        && item.required_class != 31
        && (item.required_class & cbit) == 0
    {
        return Err("职业不符");
    }
    // 等级：RequiredType Level=3 / MaxLevel=9（SharedRust 枚举）
    match item.required_type {
        3 if (level as u8) < item.required_amount => return Err("等级不足"),
        9 if (level as u8) > item.required_amount => return Err("超过最高等级"),
        _ => {}
    }
    Ok(())
}

/// #1544：槽物品（坐骑/钓具）→ EquipSlotItem（C# MirItemCell.UseSlotItem）
/// 返回 (to_slot, GridTo)：
///   - Reins/Bells/Ribbon/Saddle/Mask → Mount 槽（0..4）
///   - Hook/Float/Bait/Finder/Reel → Fishing 槽（0..4）
fn slot_item_target(item: &InvItem) -> Option<(i32, MirGridType)> {
    use mir2_shared::enums::ItemType;
    let t = ItemType::try_from(item.item_type).ok()?;
    let (slot, grid) = match t {
        // C# MountSlot：Reins=0 Bells=1 Saddle=2 Ribbon=3 Mask=4
        ItemType::Reins => (0, MirGridType::Mount),
        ItemType::Bells => (1, MirGridType::Mount),
        ItemType::Saddle => (2, MirGridType::Mount),
        ItemType::Ribbon => (3, MirGridType::Mount),
        ItemType::Mask => (4, MirGridType::Mount),
        // C# FishingSlot：Hook=0 Float=1 Bait=2 Finder=3 Reel=4
        ItemType::Hook => (0, MirGridType::Fishing),
        ItemType::Float => (1, MirGridType::Fishing),
        ItemType::Bait => (2, MirGridType::Fishing),
        ItemType::Finder => (3, MirGridType::Fishing),
        ItemType::Reel => (4, MirGridType::Fishing),
        _ => return None,
    };
    Some((slot, grid))
}

/// #1544：槽物品前置检查（C# CanUseItem：坐骑配件需坐骑；钓具需鱼竿 shape 49/50）
fn slot_item_ready(hud: &HudState, grid_to: MirGridType) -> bool {
    match grid_to {
        MirGridType::Mount => hud.equipment.get(10).and_then(|s| s.as_ref()).is_some(),
        MirGridType::Fishing => matches!(
            hud.equipment.get(0).and_then(|s| s.as_ref()).map(|w| w.shape),
            Some(49) | Some(50),
        ),
        _ => false,
    }
}

/// #1544：使用/装备结果（Sent=已发包；Confirm=已弹确认框；Blocked=守卫拦截/无动作）
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum UseOutcome {
    Sent,
    Confirm,
    Blocked,
}


/// #1546：UseItem 上下文（C# MirItemCell.UseItem 对 User/Hero/Storage 的差异）
#[derive(Clone, Copy)]
pub(crate) struct UseItemCtx<'a> {
    /// 来源格（Inventory / HeroInventory / Storage）
    pub grid: MirGridType,
    /// 装备槽（User 或 Hero）
    pub equipment: &'a [Option<InvItem>],
    /// 角色/英雄性别
    pub gender: u8,
    /// 角色/英雄职业
    pub class: u8,
    /// 角色/英雄等级
    pub level: u16,
    /// 钓鱼限制（C# !HeroGridType && User.Fishing → 英雄格 false）
    pub check_fishing: bool,
    /// 允许消耗品（C# 消耗品要求 Grid==Inventory/HeroInventory → 仓库 false）
    pub allow_consumable: bool,
}

impl<'a> UseItemCtx<'a> {
    /// 主背包（hud）
    pub fn player(hud: &'a HudState) -> Self {
        Self {
            grid: MirGridType::Inventory,
            equipment: &hud.equipment,
            gender: hud.gender,
            class: hud.class,
            level: hud.level,
            check_fishing: true,
            allow_consumable: true,
        }
    }
}

/// 使用/装备物品（#1544 对齐 C# MirItemCell.UseItem 守卫）：

/// #1546：守卫链纯逻辑（不发包，便于单测）
/// 返回 Some(true)=可继续（已通过守卫）；Some(false)=槽物品无坐骑/鱼竿；None=被拦截
#[allow(clippy::too_many_arguments)]
fn use_item_guard(
    item: &InvItem,
    hud: &HudState,
    ctx: UseItemCtx,
    now: f64,
    feedback: &mut ItemUseFeedback,
) -> Option<bool> {
    // 1. 节流
    if now < feedback.last_use {
        return None;
    }
    // 2. 钓鱼（英雄格跳过：C# !HeroGridType && User.Fishing）
    if ctx.check_fishing && hud.fishing {
        feedback.messages.push("钓鱼中无法使用物品".to_string());
        return None;
    }
    // 3. 骑乘（仅 Scroll/Potion/Torch 可用）
    {
        use mir2_shared::enums::ItemType;
        let t = ItemType::try_from(item.item_type).ok();
        if hud.riding
            && !matches!(
                t,
                Some(ItemType::Scroll) | Some(ItemType::Potion) | Some(ItemType::Torch)
            )
        {
            feedback.messages.push("骑乘中无法使用该物品".to_string());
            return None;
        }
    }
    // 4. 灵魂绑定（Rust 哨兵：1=本人；-1/0=未绑定；>1=C# 迁移数据绑定他人）
    if item.soul_bound_id > 1 {
        feedback.messages.push("物品已绑定其他角色".to_string());
        return None;
    }
    // 5. CanUseItem
    if let Err(reason) = can_use_item_check(item, ctx.gender, ctx.class, ctx.level) {
        feedback.messages.push(reason.to_string());
        return None;
    }
    // 6. 槽物品前置（坐骑/鱼竿）
    if let Some((_, grid_to)) = slot_item_target(item) {
        return Some(slot_item_ready(hud, grid_to));
    }
    Some(true)
}
///   1. UseItemTime 节流（300ms）
///   2. 钓鱼限制（C# !HeroGridType && User.Fishing）
///   3. 骑乘限制（RidingMount 仅 Scroll/Potion/Torch）
///   4. SoulBoundId 绑定检查（-1 未绑定；1=Rust 哨兵=已绑定本人）
///   5. CanUseItem（性别/职业/等级）→ 聊天提示
///   6. 槽物品（坐骑/钓具）→ EquipSlotItem
///   7. Potion Shape 4 → 确认框（mode=3）
///   8. 装备/使用 → EquipItem / UseItem（消耗品按 ctx.allow_consumable）
/// 返回 UseOutcome：Sent=已发包 / Confirm=已弹确认框 / Blocked=拦截或无动作。
#[allow(clippy::too_many_arguments)]
pub(crate) fn use_item_core(
    item: &InvItem,
    net: &NetConnection,
    hud: &HudState,
    ctx: UseItemCtx,
    now: f64,
    feedback: &mut ItemUseFeedback,
    confirm: &mut InvDropConfirm,
) -> UseOutcome {
    // 守卫链（节流/钓鱼/骑乘/SoulBound/CanUseItem/槽物品前置）
    match use_item_guard(item, hud, ctx, now, feedback) {
        None => return UseOutcome::Blocked,
        Some(false) => {
            // 按物品目标（坐骑/钓具）提示，而非来源格
            let target_mount = slot_item_target(item)
                .map(|(_, g)| g == MirGridType::Mount)
                .unwrap_or(false);
            let msg = if target_mount {
                "请先装备坐骑"
            } else {
                "请先装备鱼竿"
            };
            feedback.messages.push(msg.to_string());
            return UseOutcome::Blocked;
        }
        Some(true) => {}
    }
    // 6. 槽物品（坐骑/钓具）→ EquipSlotItem（前置已由 use_item_guard 校验）
    if let Some((to_slot, grid_to)) = slot_item_target(item) {
        net.send_packet(&mir2_shared::packets::client::misc::EquipSlotItem {
            grid: ctx.grid,
            unique_id: item.unique_id,
            to_slot,
            grid_to,
        });
        tracing::info!(
            "🧩 槽物品使用 {} (uid={}) -> {:?}[{}]",
            item.name,
            item.unique_id,
            grid_to,
            to_slot
        );
        feedback.last_use = now + 0.3;
        return UseOutcome::Sent;
    }
    // 7. Potion Shape 4 → 确认框（C# AreYouWantUsePotion → MirMessageBox YesNo）
    if ctx.allow_consumable
        && item.item_type == mir2_shared::enums::ItemType::Potion as u8
        && item.shape == 4
    {
        confirm.text = "确定使用此药水吗？".to_string();
        confirm.unique_id = item.unique_id;
        confirm.count = 1;
        confirm.mode = 3; // PotionShape4 确认
        confirm.visible = true;
        tracing::info!("🧪 药水 Shape4 需确认 uid={}", item.unique_id);
        return UseOutcome::Confirm;
    }
    // 8. 装备/使用
    if item.is_equipment() {
        if let Some(to) = item.equip_slot_occupied(|s| ctx.equipment.get(s).and_then(|x| x.as_ref()).is_some()) {
            net.send_packet(&mir2_shared::packets::client::item::EquipItem {
                grid: ctx.grid,
                unique_id: item.unique_id,
                to,
            });
            tracing::info!(
                "⚔️ 使用/装备 {} (uid={}) -> 槽 {} (grid={:?})",
                item.name,
                item.unique_id,
                to,
                ctx.grid
            );
            feedback.last_use = now + 0.3;
            return UseOutcome::Sent;
        }
        return UseOutcome::Blocked;
    }
    // 消耗品：C# 要求 Grid==Inventory/HeroInventory（仓库不可用）
    if ctx.allow_consumable && item.is_usable() {
        net.send_packet(&mir2_shared::packets::client::item::UseItem {
            unique_id: item.unique_id,
        });
        tracing::info!("💊 使用 {} (uid={})", item.name, item.unique_id);
        feedback.last_use = now + 0.3;
        return UseOutcome::Sent;
    }
    tracing::debug!("背包物品 {} 不可用/不可装备 (grid={:?})", item.name, ctx.grid);
    UseOutcome::Blocked
}

/// 主背包快捷使用（#1544 包装，调用点保持兼容）
#[allow(clippy::too_many_arguments)]
fn use_or_equip(
    item: &InvItem,
    net: &NetConnection,
    hud: &HudState,
    now: f64,
    feedback: &mut ItemUseFeedback,
    confirm: &mut InvDropConfirm,
) -> UseOutcome {
    use_item_core(item, net, hud, UseItemCtx::player(hud), now, feedback, confirm)
}
/// #1346：扩展背包购买/删除模式按钮（C# InventoryDialog AddButton / DelItemButton）
#[allow(clippy::too_many_arguments)]
fn inv_add_del_buttons_system(
    mut hud: ResMut<HudState>,
    mut click: ResMut<InvClickState>,
    mut confirm: ResMut<InvDropConfirm>,
    add_btn: Query<&UiButton, With<InvAddBtn>>,
    del_btn: Query<&UiButton, With<InvDelBtn>>,
    mut add_vis: Query<&mut Visibility, With<InvAddBtn>>,
    mut del_sprite: Query<&mut Sprite, With<InvDelBtn>>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<UiImageCache>,
) {
    let len = hud.inventory.items.len();
    // C# AddButton.Visible = openLevel < 10（上限 86 格）
    let can_expand = len < MAX_INV_EXPAND;
    for mut vis in &mut add_vis {
        *vis = if can_expand { Visibility::Visible } else { Visibility::Hidden };
    }
    // 删除模式图标（C# DelItemButton.Index 366 ↔ 368）
    let del_idx = if click.delete_mode { 368 } else { 366 };
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse2, del_idx) {
        for mut s in &mut del_sprite {
            s.image = h.clone();
        }
    }
    for btn in &add_btn {
        if btn.clicked && can_expand {
            // C# cost = 1M + openLevel*1M（openLevel = (len-46)/4；Rust 基线 40）
            let level = len.saturating_sub(GRID_COLS * GRID_ROWS) / 4;
            let cost = 1_000_000u64 + (level as u64) * 1_000_000u64;
            confirm.text = format!("花费 {} 金币扩展背包格？", cost);
            confirm.mode = 2;
            confirm.visible = true;
        }
    }
    for btn in &del_btn {
        if btn.clicked {
            click.delete_mode = !click.delete_mode;
            if !click.delete_mode {
                click.selected = None;
            }
        }
    }
}

/// 生成丢弃确认框（原版 C# MirMessageBox：Prguse[360] 456x190，Yes/No Title[206-208]/[210-212]）
fn spawn_inv_confirm(
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
    // MirMessageBox 居中（原版 456x190 → (1024-456)/2=284, (768-190)/2=289）
    let (bx, by) = (284.0, 289.0);
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 360) {
        let e = spawn_ui_sprite(&mut commands, h, bx, by, 9.5, 1.0);
        commands.entity(e).insert((InvConfirmWidget, Visibility::Hidden));
    }
    let t = spawn_ui_text(
        &mut commands, &font, "", bx + 35.0, by + 35.0, 12.0, Color::WHITE, 9.6,
    );
    commands.entity(t).insert((InvConfirmWidget, Visibility::Hidden));
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 206, 207, 208,
        bx + 260.0, by + 157.0, 9.7, 76.0, 25.0,
    ) {
        commands.entity(e).insert((InvConfirmYes, InvConfirmWidget));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 210, 211, 212,
        bx + 360.0, by + 157.0, 9.7, 76.0, 25.0,
    ) {
        commands.entity(e).insert((InvConfirmNo, InvConfirmWidget));
    }
}

/// 丢弃确认框：Yes → DropItem；No → 关闭（原版 C# MirMessageBox YesNo）
fn inv_confirm_system(
    mut confirm: ResMut<InvDropConfirm>,
    mut click: ResMut<InvClickState>,
    net: Res<NetConnection>,
    mut widgets: Query<&mut Visibility, With<InvConfirmWidget>>,
    yes: Query<&UiButton, (With<InvConfirmYes>, Without<InvConfirmNo>)>,
    no: Query<&UiButton, (With<InvConfirmNo>, Without<InvConfirmYes>)>,
) {
    for mut vis in &mut widgets {
        *vis = if confirm.visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !confirm.visible {
        return;
    }
    for btn in &yes {
        if btn.clicked {
            match confirm.mode {
                1 => {
                    // #1346：删除模式（C# PromptDelete → C.DeleteItem），删除后退出删除模式
                    net.send_packet(&mir2_shared::packets::client::item::DeleteItem {
                        unique_id: confirm.unique_id,
                        count: confirm.count,
                        hero_inventory: false,
                    });
                    click.delete_mode = false;
                    tracing::info!("🗑️ 确认删除 uid={} count={}", confirm.unique_id, confirm.count);
                }
                3 => {
                    // #1544：Potion Shape 4 确认后使用（C# AreYouWantUsePotion → UseItem）
                    net.send_packet(&mir2_shared::packets::client::item::UseItem {
                        unique_id: confirm.unique_id,
                    });
                    tracing::info!("🧪 确认使用 Shape4 药水 uid={}", confirm.unique_id);
                }
                2 => {
                    // #1346：背包扩容（C# AddButton → C.Chat"@ADDINVENTORY"）
                    net.send_packet(&mir2_shared::packets::client::chat::Chat {
                        message: "@ADDINVENTORY".to_string(),
                        linked_items: Vec::new(),
                    });
                    tracing::info!("📦 请求背包扩容");
                }
                _ => {
                    net.send_packet(&mir2_shared::packets::client::item::DropItem {
                        unique_id: confirm.unique_id,
                        count: confirm.count as u32,
                        hero_inventory: false,
                    });
                    tracing::info!("🗑️ 确认丢弃 uid={} count={}", confirm.unique_id, confirm.count);
                }
            }
            confirm.visible = false;
            click.selected = None;
        }
    }
    for btn in &no {
        if btn.clicked {
            confirm.visible = false;
        }
    }
}

/// Ctrl+右键：打开镶嵌面板（C# MirItemCell.OpenItem）——独立系统避免主系统参数超限（Bevy 16 上限）
#[allow(clippy::too_many_arguments)]
fn inv_socket_open_system(
    hud: Res<HudState>,
    mut mgr: ResMut<DialogManager>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window>,
    mut socket: ResMut<crate::game::dialogs::socket::SocketState>,
) {
    if !mouse.just_pressed(MouseButton::Right) || !keys.pressed(KeyCode::ControlLeft) {
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else { return };
    let page = hud.inventory.page;
    let size = hud.inventory.items.len().min(MAX_INV_SLOTS);
    let range: std::ops::Range<usize> = match page {
        0 => 0..size.min(GRID_COLS * GRID_ROWS),
        1 => (GRID_COLS * GRID_ROWS)..size,
        _ => 0..0,
    };
    for i in range {
        let x = i % GRID_COLS;
        let y = (i / GRID_COLS) % GRID_ROWS;
        let sx = DIALOG_X + 9.0 + x as f32 * (CELL_W + 1.0);
        let sy = DIALOG_Y + 37.0 + y as f32 * (CELL_H + 1.0);
        if cursor.x >= sx && cursor.x <= sx + CELL_W && cursor.y >= sy && cursor.y <= sy + CELL_H {
            if let Some(item) = hud.inventory.items.get(i).and_then(|s| s.as_ref()) {
                if !item.slots.is_empty() {
                    socket.item = Some(item.clone());
                    mgr.open(DialogKind::Socket);
                    tracing::info!("💎 打开镶嵌面板: {} ({} 孔)", item.name, item.slots.len());
                }
            }
            return;
        }
    }
}

/// 物品高级交互：
///   - 右键 → 使用/装备（原版 C# MouseButtons.Right → UseItem）
///   - Shift+左键 → 拆分堆叠（MirAmountBox → SplitItem）
///   - 选中物品 + 点场景地面 → 丢弃（单件 YesNo 确认 / 多件数量框 → DropItem）
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn inv_item_action_system(
    hud: Res<HudState>,
    mut mgr: ResMut<DialogManager>,
    mut click: ResMut<InvClickState>,
    net: Res<NetConnection>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window>,
    time: Res<Time>,
    mut amount: ResMut<AmountBoxState>,
    mut feedback: ResMut<ItemUseFeedback>,
    mut confirm: ResMut<InvDropConfirm>,
    npc_goods: Res<crate::game::dialogs::npc_goods::NpcGoodsState>,
    mut pending: ResMut<InvPendingAmount>,
    mut result: MessageReader<AmountBoxResult>,
    all_buttons: Query<&UiButton>,
    // 弹窗模态门：上一帧有弹窗 → 本帧点击视为弹窗按钮，不处理格子（原版 C# Modal）
    mut last_modal: Local<bool>,
) {
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else { return };

    // #1342：任务物品格只读（C# MirGridType.QuestInventory 不可移动/使用）
    if hud.inventory.page == 2 {
        return;
    }

    // 数量框结果：拆分/丢弃
    for r in result.read() {
        let Some(n) = r.0 else {
            pending.split_uid = None;
            pending.drop_uid = None;
            continue;
        };
        if n == 0 {
            continue;
        }
        if let Some(uid) = pending.split_uid.take() {
            net.send_packet(&mir2_shared::packets::client::item::SplitItem {
                grid: MirGridType::Inventory,
                unique_id: uid,
                count: n,
            });
            tracing::info!("🔪 拆分物品 uid={} count={}", uid, n);
        } else if let Some(uid) = pending.drop_uid.take() {
            net.send_packet(&mir2_shared::packets::client::item::DropItem {
                unique_id: uid,
                count: n,
                hero_inventory: false,
            });
            tracing::info!("🗑️ 丢弃物品 uid={} count={}", uid, n);
        } else if let Some(uid) = pending.delete_uid.take() {
            // #1346：删除模式数量框确认（C# PromptDelete DoDelete）
            click.delete_mode = false;
            net.send_packet(&mir2_shared::packets::client::item::DeleteItem {
                unique_id: uid,
                count: n as u16,
                hero_inventory: false,
            });
            tracing::info!("🗑️ 删除物品 uid={} count={}", uid, n);
        }
    }

    // 光标下的背包格（按当前页与格数，#276）
    let page = hud.inventory.page;
    let size = hud.inventory.items.len().min(MAX_INV_SLOTS);
    let slot_at = |cx: f32, cy: f32| -> Option<usize> {
        let range: std::ops::Range<usize> = match page {
            0 => 0..size.min(GRID_COLS * GRID_ROWS),
            1 => (GRID_COLS * GRID_ROWS)..size,
            _ => 0..0,
        };
        for i in range {
            let x = i % GRID_COLS;
            let y = (i / GRID_COLS) % GRID_ROWS;
            let sx = DIALOG_X + 9.0 + x as f32 * (CELL_W + 1.0);
            let sy = DIALOG_Y + 37.0 + y as f32 * (CELL_H + 1.0);
            if cx >= sx && cx <= sx + CELL_W && cy >= sy && cy <= sy + CELL_H {
                return Some(i);
            }
        }
        None
    };

    // 弹窗模态门（原版 C# Modal：弹窗打开期间/刚关闭帧不响应格子点击）
    let modal_now = amount.visible || confirm.visible;
    let modal_was = *last_modal;
    *last_modal = modal_now;
    if modal_was || modal_now {
        return;
    }

    // #1346：删除模式左键点物品 → 数量框/确认 → C.DeleteItem（C# PromptDelete）
    if click.delete_mode && mouse.just_pressed(MouseButton::Left) {
        if let Some(i) = slot_at(cursor.x, cursor.y) {
            if let Some(item) = hud.inventory.items.get(i).and_then(|s| s.as_ref()) {
                if item.count > 1 {
                    pending.delete_uid = Some(item.unique_id);
                    amount.ask(format!("删除 {} 数量", item.name), item.count as u32);
                } else {
                    confirm.text = format!("确定删除 {} 吗？", item.name);
                    confirm.unique_id = item.unique_id;
                    confirm.count = 1;
                    confirm.mode = 1;
                    confirm.visible = true;
                }
                return;
            }
        }
    }

    // 双击/单击检测（原版 C# MirItemCell.OnMouseDoubleClick / OnMouseClick）
    let now = time.elapsed_secs_f64();
    let mut dbl: Option<usize> = None;
    let mut single: Option<usize> = None;
    if mouse.just_pressed(MouseButton::Left) {
        if let Some(i) = slot_at(cursor.x, cursor.y) {
            if let Some((last_i, last_t)) = click.last {
                if last_i == i && now - last_t < 0.4 {
                    dbl = Some(i);
                    click.last = None;
                } else {
                    click.last = Some((i, now));
                    single = Some(i);
                }
            } else {
                click.last = Some((i, now));
                single = Some(i);
            }
        }
    }
    // 单击：选中 → 移动（MoveItem，原版 C# MirItemCell.MoveItem）
    if dbl.is_none() {
        if let Some(i) = single {
            // #203：英雄背包选中格 → 点击主背包格 = 取回（C.TakeBackHeroItem）
            if let Some(hero_from) = click.hero_selected {
                net.send_packet(&crate::network::TakeBackHeroItemWire {
                    from: hero_from as i32,
                    to: i as i32,
                });
                click.hero_selected = None;
                tracing::info!("🎒 英雄取回物品 {} -> 背包 {}", hero_from, i);
            } else {
            match click.selected {
                Some(from) if from == i => click.selected = None,
                Some(from) => {
                    // #1604：拖到同类堆叠格 → C.MergeItem（C# MirItemCell.cs:815/906/980）；
                    // ServerRust move_item 目标格有物品会失败，merge_item 只由 MergeItem 触发
                    let same_stack = hud.inventory.items.get(i)
                        .and_then(|s| s.as_ref())
                        .zip(hud.inventory.items.get(from).and_then(|s| s.as_ref()))
                        .map(|(t, f)| t.item_index == f.item_index && t.unique_id != f.unique_id)
                        .unwrap_or(false);
                    if same_stack {
                        if let (Some(from_item), Some(to_item)) = (
                            hud.inventory.items.get(from).and_then(|s| s.as_ref()),
                            hud.inventory.items.get(i).and_then(|s| s.as_ref()),
                        ) {
                            net.send_packet(&mir2_shared::packets::client::item::MergeItem {
                                grid_from: MirGridType::Inventory,
                                grid_to: MirGridType::Inventory,
                                id_from: from_item.unique_id,
                                id_to: to_item.unique_id,
                            });
                            tracing::info!("🔗 合并物品 {} -> {}（uid {} -> {}）", from, i, from_item.unique_id, to_item.unique_id);
                        }
                    } else {
                        // 目标格子可空可满（服务端处理交换/移动）
                        net.send_packet(&mir2_shared::packets::client::item::MoveItem {
                            grid: MirGridType::Inventory,
                            from: from as i32,
                            to: i as i32,
                        });
                        tracing::info!("📦 移动物品 {} -> {}", from, i);
                    }
                    click.selected = None;
                }
                None => {
                    // 只有物品格可选中（空格不选中）
                    if hud.inventory.items.get(i).and_then(|s| s.as_ref()).is_some() {
                        click.selected = Some(i);
                    }
                }
            }
            }
        }
    }
    // 双击：使用/装备
    if let Some(i) = dbl {
        if let Some(item) = hud.inventory.items.get(i).and_then(|s| s.as_ref()) {
            if use_or_equip(item, &net, &hud, now, &mut feedback, &mut confirm) == UseOutcome::Sent {
                if let Some(sid) = item_use_sound_id(item) {
                    feedback.sounds.push(sid);
                }
            }
        }
    }
    // #1346：删除模式下右键取消（C# OnMouseClick right-click cancels bin toggle）
    if mouse.just_pressed(MouseButton::Right) && click.delete_mode {
        click.delete_mode = false;
        return;
    }


    // 右键：使用/装备
    if mouse.just_pressed(MouseButton::Right) {
        if let Some(i) = slot_at(cursor.x, cursor.y) {
            if let Some(item) = hud.inventory.items.get(i).and_then(|s| s.as_ref()) {
            if use_or_equip(item, &net, &hud, now, &mut feedback, &mut confirm) == UseOutcome::Sent {
                    if let Some(sid) = item_use_sound_id(item) {
                        feedback.sounds.push(sid);
                    }
                }
            }
        }
    }
    // Alt+左键：快速出售（原版 C# "Add support for ALT + click to sell quickly"）
    if mouse.just_pressed(MouseButton::Left) && keys.pressed(KeyCode::AltLeft) {
        if npc_goods.visible {
            if let Some(i) = slot_at(cursor.x, cursor.y) {
                if let Some(item) = hud.inventory.items.get(i).and_then(|s| s.as_ref()) {
                    net.send_packet(&mir2_shared::packets::client::npc::SellItem {
                        unique_id: item.unique_id,
                        count: 1,
                    });
                    tracing::info!("💰 出售 {} (uid={})", item.name, item.unique_id);
                }
            }
        }
        return;
    }

    // Shift+左键：拆分堆叠
    if mouse.just_pressed(MouseButton::Left) && keys.pressed(KeyCode::ShiftLeft) {
        if let Some(i) = slot_at(cursor.x, cursor.y) {
            if let Some(item) = hud.inventory.items.get(i).and_then(|s| s.as_ref()) {
                if item.count > 1 {
                    if !hud.inventory.items.iter().any(|s| s.is_none()) {
                        tracing::warn!("背包已满，无法拆分");
                        return;
                    }
                    amount.ask("拆分数量", (item.count - 1) as u32);
                    pending.split_uid = Some(item.unique_id);
                    tracing::info!(
                        "🔪 拆分 {} (uid={}) 最大 {}",
                        item.name,
                        item.unique_id,
                        item.count - 1
                    );
                    return;
                }
            }
        }
    }

    // 选中物品 + 左键点场景（非背包格/非装备格/非按钮/非背包面板）→ 丢弃流程
    if mouse.just_pressed(MouseButton::Left) {
        let Some(sel) = click.selected else { return };
        if slot_at(cursor.x, cursor.y).is_some() {
            return;
        }
        // 背包面板背景内不触发（原版：点对话框不丢物品）
        if cursor.x >= DIALOG_X
            && cursor.x <= DIALOG_X + 318.0
            && cursor.y >= DIALOG_Y
            && cursor.y <= DIALOG_Y + 256.0
        {
            return;
        }
        // 角色对话框装备格区域不触发
        if mgr.is_open(DialogKind::Character) {
            let in_eq = character::EQUIP_SLOTS.iter().any(|(ox, oy)| {
                let sx = character::DIALOG_X + ox;
                let sy = character::DIALOG_Y + oy;
                cursor.x >= sx
                    && cursor.x <= sx + character::SLOT_SIZE
                    && cursor.y >= sy
                    && cursor.y <= sy + character::SLOT_SIZE
            });
            if in_eq {
                return;
            }
        }
        // 任意 UI 按钮上不触发
        let over_btn = all_buttons.iter().any(|b| {
            let (x, y, w, h) = b.rect;
            cursor.x >= x && cursor.x <= x + w && cursor.y >= y && cursor.y <= y + h
        });
        if over_btn {
            return;
        }
        let Some(item) = hud.inventory.items.get(sel).and_then(|s| s.as_ref()) else {
            click.selected = None;
            return;
        };
        if item.count > 1 {
            amount.ask("丢弃数量", item.count as u32);
            pending.drop_uid = Some(item.unique_id);
        } else {
            confirm.text = format!("确定丢弃 {} 吗？", item.name);
            confirm.unique_id = item.unique_id;
            confirm.count = 1;
            confirm.visible = true;
        }
        tracing::info!("🗑️ 准备丢弃 {} (uid={})", item.name, item.unique_id);
        click.selected = None;
    }
}


/// #1592：自动喝 HP 药选药——优先 shape==0（C# ItemInfo Potion：0=HP 红药、1=MP 蓝药），
/// 无 HP 药时退化为任意药水（保持旧行为）。
pub fn pick_auto_hp_potion<'a>(items: impl Iterator<Item = &'a InvItem>) -> Option<&'a InvItem> {
    let mut fallback: Option<&InvItem> = None;
    for it in items {
        if mir2_shared::enums::ItemType::try_from(it.item_type) == Ok(mir2_shared::enums::ItemType::Potion) {
            if it.shape == 0 {
                return Some(it);
            }
            fallback.get_or_insert(it);
        }
    }
    fallback
}

#[cfg(test)]
mod tests {
    use super::*;
    use mir2_shared::enums::ItemType;

    fn item_with_type(t: ItemType) -> InvItem {
        InvItem {
            unique_id: 1,
            item_index: 1,
            name: "test".into(),
            image: 0,
            count: 1,
            item_type: t as u8,
            shape: 0,
            current_dura: 100,
            max_dura: 100,
            slots: Vec::new(),
            stats: Vec::new(),
            required_type: 0,
            required_amount: 0,
            required_class: 0,
            required_gender: 0,
            soul_bound_id: -1,
            weight: 0,
            price: 0,
        }
    }

    #[test]
    fn tooltip_lines_pairs_and_singles() {
        let mut it = item_with_type(ItemType::Weapon);
        it.stats = vec![
            (mir2_shared::enums::Stat::MinDC as u8, 3),
            (mir2_shared::enums::Stat::MaxDC as u8, 8),
            (mir2_shared::enums::Stat::Accuracy as u8, 2),
            (mir2_shared::enums::Stat::Luck as u8, 1),
        ];
        it.weight = 5;
        it.price = 120;
        it.required_type = 3; // Level
        it.required_amount = 30;
        let lines = item_tooltip_lines(&it);
        assert!(lines.iter().any(|l| l.contains("攻击: 3-8")));
        assert!(lines.iter().any(|l| l.contains("准确: +2")));
        assert!(lines.iter().any(|l| l.contains("幸运: +1")));
        assert!(lines.iter().any(|l| l.contains("需要等级: 30")));
        assert!(lines.iter().any(|l| l.contains("重量: 5")));
        assert!(lines.iter().any(|l| l.contains("价格: 120 金")));
    }

    #[test]
    fn tooltip_lines_requirements_class() {
        let mut it = item_with_type(ItemType::Armour);
        it.required_class = 1 | 2; // 战士/法师
        let lines = item_tooltip_lines(&it);
        assert!(lines.iter().any(|l| l.contains("需要职业: 战士/法师")));
    }

    #[test]
    fn equip_slot_bracelet_prefers_right_then_left() {
        let b = item_with_type(ItemType::Bracelet);
        // 右空 → BraceletR(5)
        assert_eq!(b.equip_slot_occupied(|_| false), Some(5));
        // 右占 → 回退 BraceletL(4)
        assert_eq!(b.equip_slot_occupied(|s| s == 5), Some(4));
        // 双占 → 不装备（C# 语义）
        assert_eq!(b.equip_slot_occupied(|s| s == 5 || s == 4), None);
    }

    #[test]
    fn equip_slot_ring_prefers_right_then_left() {
        let r = item_with_type(ItemType::Ring);
        assert_eq!(r.equip_slot_occupied(|_| false), Some(7));
        assert_eq!(r.equip_slot_occupied(|s| s == 7), Some(6));
        assert_eq!(r.equip_slot_occupied(|s| s == 7 || s == 6), None);
    }

    #[test]
    fn equip_slot_fixed_slots_unchanged() {
        assert_eq!(item_with_type(ItemType::Torch).equip_slot(), Some(11));
        assert_eq!(item_with_type(ItemType::Belt).equip_slot(), Some(12));
        assert_eq!(item_with_type(ItemType::Stone).equip_slot(), Some(13));
        assert_eq!(item_with_type(ItemType::Weapon).equip_slot(), Some(0));
    }


    #[test]
    fn can_use_item_gender_class_level() {
        let mut it = item_with_type(ItemType::Weapon);
        // 性别：Male=1（MirGender Male=0 → bit1）
        it.required_gender = 1;
        assert!(can_use_item_check(&it, 0, 0, 30).is_ok());
        assert!(can_use_item_check(&it, 1, 0, 30).is_err());
        // 职业：Warrior=1（MirClass Warrior=0 → bit1）
        it.required_gender = 0;
        it.required_class = 1;
        assert!(can_use_item_check(&it, 0, 0, 30).is_ok());
        assert!(can_use_item_check(&it, 0, 1, 30).is_err());
        // 等级：RequiredType Level=3（SharedRust）
        it.required_class = 0;
        it.required_type = 3;
        it.required_amount = 30;
        assert!(can_use_item_check(&it, 0, 0, 30).is_ok());
        assert!(can_use_item_check(&it, 0, 0, 29).is_err());
        // MaxLevel=9
        it.required_type = 9;
        it.required_amount = 40;
        assert!(can_use_item_check(&it, 0, 0, 40).is_ok());
        assert!(can_use_item_check(&it, 0, 0, 41).is_err());
        // 无需求
        it.required_type = 0;
        it.required_amount = 0;
        assert!(can_use_item_check(&it, 0, 0, 1).is_ok());
    }

    #[test]
    fn slot_item_target_mount_and_fishing() {
        let mut reins = item_with_type(ItemType::Reins);
        assert_eq!(slot_item_target(&reins), Some((0, MirGridType::Mount)));
        reins.item_type = ItemType::Mask as u8;
        assert_eq!(slot_item_target(&reins), Some((4, MirGridType::Mount)));
        let mut hook = item_with_type(ItemType::Hook);
        assert_eq!(slot_item_target(&hook), Some((0, MirGridType::Fishing)));
        hook.item_type = ItemType::Reel as u8;
        assert_eq!(slot_item_target(&hook), Some((4, MirGridType::Fishing)));
        let sword = item_with_type(ItemType::Weapon);
        assert_eq!(slot_item_target(&sword), None);
    }

    #[test]
    fn slot_item_ready_requires_mount_or_rod() {
        let mut hud = HudState::default();
        assert!(!slot_item_ready(&hud, MirGridType::Mount));
        assert!(!slot_item_ready(&hud, MirGridType::Fishing));
        let mut m = item_with_type(ItemType::Mount);
        hud.equipment[10] = Some(m);
        assert!(slot_item_ready(&hud, MirGridType::Mount));
        let mut rod = item_with_type(ItemType::Weapon);
        rod.shape = 49;
        hud.equipment[0] = Some(rod);
        assert!(slot_item_ready(&hud, MirGridType::Fishing));
        let mut sword = item_with_type(ItemType::Weapon);
        sword.shape = 0;
        hud.equipment[0] = Some(sword);
        assert!(!slot_item_ready(&hud, MirGridType::Fishing));
    }
    #[test]
    fn pick_auto_hp_potion_prefers_hp_shape() {
        // #1592：HP 药（shape 0）优先；只有 MP 药（shape 1）时退化；无药水 None
        let mut hp = InvItem::default();
        hp.item_type = mir2_shared::enums::ItemType::Potion as u8;
        hp.shape = 0;
        let mut mp = InvItem::default();
        mp.item_type = mir2_shared::enums::ItemType::Potion as u8;
        mp.shape = 1;
        let mut sword = InvItem::default();
        sword.item_type = mir2_shared::enums::ItemType::Weapon as u8;
        sword.shape = 0;

        // 背包：武器 → 蓝药 → 红药 → 选红药
        let items = [Some(sword.clone()), Some(mp.clone()), Some(hp.clone())];
        let picked = pick_auto_hp_potion(items.iter().flatten()).expect("应选到药水");
        assert_eq!(picked.shape, 0, "应优先 HP 药");

        // 只有蓝药 → 退化选蓝药
        let items = [Some(sword.clone()), Some(mp.clone())];
        let picked = pick_auto_hp_potion(items.iter().flatten()).expect("应退化到蓝药");
        assert_eq!(picked.shape, 1);

        // 无药水 → None
        let items = [Some(sword.clone())];
        assert!(pick_auto_hp_potion(items.iter().flatten()).is_none());
    }

    #[test]
    fn item_use_sound_maps_types() {
        assert_eq!(item_use_sound_id(&item_with_type(ItemType::Weapon)), Some(10111));
        assert_eq!(item_use_sound_id(&item_with_type(ItemType::Potion)), Some(10108));
        assert_eq!(item_use_sound_id(&item_with_type(ItemType::Food)), Some(10118));
    }

    #[test]
    fn use_item_cooldown_gates() {
        let mut fb = ItemUseFeedback::default();
        let now = 100.0;
        // 未节流 → 放行
        assert!(!(now < fb.last_use));
        fb.last_use = now + 0.3;
        assert!(now < fb.last_use);
    }

    #[test]
    fn refresh_weight_sums_items() {
        let mut inv = InventoryState::default();
        let mut a = item_with_type(ItemType::Potion);
        a.weight = 1;
        a.count = 2;
        let mut b = item_with_type(ItemType::Scroll);
        b.weight = 3;
        b.count = 1;
        inv.items = vec![Some(a), Some(b), None];
        inv.refresh_weight();
        assert_eq!(inv.weight, 5);
    }


    #[test]
    fn guard_hero_skips_fishing() {
        // #1546：英雄格 check_fishing=false → 钓鱼中也可使用（C# !HeroGridType && User.Fishing）
        let mut hud = HudState::default();
        hud.fishing = true;
        let mut fb = ItemUseFeedback::default();
        let potion = item_with_type(ItemType::Potion);
        let ctx_hero = UseItemCtx {
            grid: MirGridType::HeroInventory,
            equipment: &hud.equipment,
            gender: 0,
            class: 0,
            level: 1,
            check_fishing: false,
            allow_consumable: true,
        };
        assert!(use_item_guard(&potion, &hud, ctx_hero, 0.0, &mut fb).is_some());
        // 主背包 check_fishing=true → 钓鱼拦截
        let ctx_player = UseItemCtx::player(&hud);
        assert!(use_item_guard(&potion, &hud, ctx_player, 0.0, &mut fb).is_none());
    }

    #[test]
    fn guard_storage_blocks_consumable_but_allows_equip() {
        // #1546：仓库格 allow_consumable=false → 消耗品拦截；装备放行
        let mut hud = HudState::default();
        let mut fb = ItemUseFeedback::default();
        let ctx_storage = UseItemCtx {
            grid: MirGridType::Storage,
            equipment: &hud.equipment,
            gender: 0,
            class: 0,
            level: 1,
            check_fishing: true,
            allow_consumable: false,
        };
        // 守卫本身通过（消耗品拦截在 use_item_core 第 8 步）
        let potion = item_with_type(ItemType::Potion);
        assert!(use_item_guard(&potion, &hud, ctx_storage, 0.0, &mut fb).is_some());
        // 装备放行
        let sword = item_with_type(ItemType::Weapon);
        assert!(use_item_guard(&sword, &hud, ctx_storage, 0.0, &mut fb).is_some());
    }

    #[test]
    fn use_item_core_storage_blocks_consumable() {
        // 仓库双击药水 → Blocked（不发包）；仓库双击武器且槽空 → Sent
        let net = NetConnection::default();
        let mut hud = HudState::default();
        // 仓库双击药水 → Blocked（不发包）；仓库双击武器且槽空 → Sent
        let mut hud = HudState::default();
        let mut fb = ItemUseFeedback::default();
        let mut confirm = InvDropConfirm::default();
        let ctx_storage = UseItemCtx {
            grid: MirGridType::Storage,
            equipment: &hud.equipment,
            gender: 0,
            class: 0,
            level: 1,
            check_fishing: true,
            allow_consumable: false,
        };
        let potion = item_with_type(ItemType::Potion);
        assert_eq!(
            use_item_core(&potion, &net, &hud, ctx_storage, 0.0, &mut fb, &mut confirm),
            UseOutcome::Blocked
        );
        let sword = item_with_type(ItemType::Weapon);
        assert_eq!(
            use_item_core(&sword, &net, &hud, ctx_storage, 0.0, &mut fb, &mut confirm),
            UseOutcome::Sent
        );
    }

    #[test]
    fn use_item_core_hero_equips_with_hero_equipment() {
        let net = NetConnection::default();
        // #1546：英雄格装备用英雄装备槽判断（ctx.equipment）
        let hud = HudState::default();
        let mut fb = ItemUseFeedback::default();
        let mut confirm = InvDropConfirm::default();
        // Bracelet 智能槽：右(5)空 → Sent；右占回退左(4)；双占 → Blocked
        let mut bracelet = item_with_type(ItemType::Bracelet);
        bracelet.unique_id = 20;
        let mut hero_eq_empty = vec![None; 14];
        let ctx_empty = UseItemCtx {
            grid: MirGridType::HeroInventory,
            equipment: &hero_eq_empty,
            gender: 0,
            class: 0,
            level: 1,
            check_fishing: false,
            allow_consumable: true,
        };
        assert_eq!(
            use_item_core(&bracelet, &net, &hud, ctx_empty, 0.0, &mut fb, &mut confirm),
            UseOutcome::Sent
        );
        // 左右手镯都占用 → 不装备（C# BraceletR/L 都占用 → 不装备）
        let mut hero_eq_full = vec![None; 14];
        let mut br = item_with_type(ItemType::Bracelet);
        br.unique_id = 4;
        hero_eq_full[4] = Some(br);
        let mut br2 = item_with_type(ItemType::Bracelet);
        br2.unique_id = 5;
        hero_eq_full[5] = Some(br2);
        let ctx_full = UseItemCtx {
            grid: MirGridType::HeroInventory,
            equipment: &hero_eq_full,
            gender: 0,
            class: 0,
            level: 1,
            check_fishing: false,
            allow_consumable: true,
        };
        assert_eq!(
            use_item_core(&bracelet, &net, &hud, ctx_full, 0.0, &mut fb, &mut confirm),
            UseOutcome::Blocked
        );
    }
}


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

/// 背包数据（网络 UserInformation.inventory 写入）
#[derive(Resource, Default)]
pub struct InventoryState {
    /// 动态格数背包（默认 40，ResizeInventory 扩容/缩容，#276）
    pub items: Vec<Option<InvItem>>,
    pub gold: u32,
    pub weight: u32,
    pub max_weight: u32,
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
}

const DIALOG_X: f32 = 182.0;
const DIALOG_Y: f32 = 217.0;
const GRID_COLS: usize = 8;
const GRID_ROWS: usize = 5;
const CELL_W: f32 = 36.0;
const CELL_H: f32 = 32.0;

#[derive(Component)]
pub struct InventoryPanel;

/// 背包对话框内所有 UI 元素（统一显隐）
#[derive(Component)]
pub struct DialogWidget;

#[derive(Component)]
pub struct InvTab(pub usize); // 0=道具 1=道具2 2=任务

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
                inv_item_action_system,
                inv_confirm_system,
                ui_button_system,
            )
                .chain()
                .run_if(in_state(AppState::Game)),
        );
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

    // 格子背景不在此预生成：#276 由 inv_grid_sync_system 按 InventoryState.items.len()
    // 动态生成/移除（进图 UserInformation 到达前 items 为空，避免先建后删抖动）
}

#[derive(Component)]
struct InvCloseBtn;

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
}

/// 丢弃确认框（原版 C# MirMessageBox YesNo：DropTip）
#[derive(Resource, Default)]
pub struct InvDropConfirm {
    pub visible: bool,
    pub text: String,
    pub unique_id: u64,
    pub count: u16,
}

/// 数量框待处理操作（拆分/丢弃，原版 C# MirAmountBox OK 回调）
#[derive(Resource, Default)]
pub struct InvPendingAmount {
    pub split_uid: Option<u64>,
    pub drop_uid: Option<u64>,
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
    // 格子弹页显隐（#276）：道具=0..min(40,size)，道具2=40..size-1，任务页隐藏
    for (mut vis, slot) in &mut all_vis {
        let visible = if !open {
            false
        } else {
            match slot {
                Some(s) => match inv.page {
                    0 => s.0 < size.min(GRID_COLS * GRID_ROWS),
                    1 => s.0 >= GRID_COLS * GRID_ROWS && s.0 < size,
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
        let item = inv.items.get(slot.0).and_then(|s| s.as_ref());
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
            hit = inv.inventory.items.get(i).and_then(|s| s.as_ref()).cloned();
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



fn use_or_equip(item: &InvItem, net: &NetConnection, hud: &HudState) {
    if item.is_equipment() {
        if let Some(to) = item.equip_slot_occupied(|s| hud.equipment.get(s).and_then(|x| x.as_ref()).is_some()) {
            net.send_packet(&mir2_shared::packets::client::item::EquipItem {
                // 协议字段为 MirGridType；服务端按 unique_id 定位背包格
                grid: MirGridType::Inventory,
                unique_id: item.unique_id,
                to,
            });
            tracing::info!(
                "⚔️ 使用/装备 {} (uid={}) -> 槽 {}",
                item.name,
                item.unique_id,
                to
            );
        }
    } else if item.is_usable() {
        net.send_packet(&mir2_shared::packets::client::item::UseItem {
            unique_id: item.unique_id,
        });
        tracing::info!("💊 使用 {} (uid={})", item.name, item.unique_id);
    } else {
        tracing::debug!("背包物品 {} 不可用/不可装备", item.name);
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
            net.send_packet(&mir2_shared::packets::client::item::DropItem {
                unique_id: confirm.unique_id,
                count: confirm.count as u32,
                hero_inventory: false,
            });
            tracing::info!(
                "🗑️ 确认丢弃 uid={} count={}",
                confirm.unique_id,
                confirm.count
            );
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
    mut confirm: ResMut<InvDropConfirm>,
    npc_goods: Res<crate::game::dialogs::npc_goods::NpcGoodsState>,
    mut socket: ResMut<crate::game::dialogs::socket::SocketState>,
    mut pending: ResMut<InvPendingAmount>,
    mut result: MessageReader<AmountBoxResult>,
    all_buttons: Query<&UiButton>,
    // 弹窗模态门：上一帧有弹窗 → 本帧点击视为弹窗按钮，不处理格子（原版 C# Modal）
    mut last_modal: Local<bool>,
) {
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else { return };

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
                    // 目标格子可空可满（服务端处理交换/合并）
                    net.send_packet(&mir2_shared::packets::client::item::MoveItem {
                        grid: MirGridType::Inventory,
                        from: from as i32,
                        to: i as i32,
                    });
                    tracing::info!("📦 移动物品 {} -> {}", from, i);
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
            use_or_equip(item, &net, &hud);
        }
    }

    // Ctrl+右键：打开镶嵌面板（C# MirItemCell.OpenItem）
    if mouse.just_pressed(MouseButton::Right) && keys.pressed(KeyCode::ControlLeft) {
        if let Some(i) = slot_at(cursor.x, cursor.y) {
            if let Some(item) = hud.inventory.items.get(i).and_then(|s| s.as_ref()) {
                if !item.slots.is_empty() {
                    socket.item = Some(item.clone());
                    mgr.open(DialogKind::Socket);
                    tracing::info!("💎 打开镶嵌面板: {} ({} 孔)", item.name, item.slots.len());
                }
            }
        }
        return;
    }

    // 右键：使用/装备
    if mouse.just_pressed(MouseButton::Right) {
        if let Some(i) = slot_at(cursor.x, cursor.y) {
            if let Some(item) = hud.inventory.items.get(i).and_then(|s| s.as_ref()) {
                use_or_equip(item, &net, &hud);
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
}



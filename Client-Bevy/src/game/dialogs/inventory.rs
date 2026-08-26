// ============================================================================
// 背包对话框（M9 第一批）
// 布局参考：Client/MirScenes/Dialogs/InventoryDialog.cs
//   - 窗口原点 (0,0)（构造器未设 Location → MirControl 默认零值），背景 Title[196]
//   - 标签页：道具(6,7) / 道具2(76,7) / 任务(146,7)，72x23
//   - 关闭按钮 (289,3) Prguse2[360/361/362]
//   - 金币 (40,212) 111x14；负重 (268,212)
//   - 格子：8 列 x 5 行，cell 36x32，起点 (9,37)，x 间隔 1
// ============================================================================

use bevy::prelude::*;

use crate::actor::LocalPlayer;
use crate::game::chat::{ChatChannel, ChatState};
use crate::game::dialogs::amount_box::{AmountBoxResult, AmountBoxState};
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::game::player_state::{Gold, Inventory, Loadout, StatusFlags};
use crate::game::sets::GameSet;
use crate::game::sound::{SoundBank, SoundCache, play_sound_cached};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use mir2_shared::enums::MirGridType;

use crate::ui::sprite_ui::UiFont;
use crate::ui::theme::{
    load_lib_image, spawn_icon_button, spawn_image, spawn_item_cell_ui, spawn_label, spawn_panel,
    UiItemCellData,
};

/// 背包物品条目（网络 UserInformation 写入）
#[derive(Debug, Clone, Default)]
pub struct InvItem {
    pub unique_id: u64,
    pub item_index: i32,
    /// 物品品质（SharedRust ItemGrade +3：None=3/Common=4/Rare=5/Legendary=6/Mythical=7/Heroic=8）
    pub grade: u8,
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
            ItemType::Amulet => 9, // Pendant
            ItemType::Boots => 8,  // Shoes
            ItemType::Mount => 10, // Mount
            // #1136：C# 补槽（SharedRust ItemType：Torch=15 / Belt=12 / Stone=14）
            ItemType::Torch => 11, // Torch
            ItemType::Belt => 12,  // Belt
            ItemType::Stone => 13, // Stone
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

// #2633 批次4 收尾：`InventoryState`（原 `HudState.inventory` 字段类型）已随 HudState 一起删除——
// 背包数据归 `crate::game::player_state::Inventory` 组件（resize/refresh_weight 逻辑已移植过去）。

/// 背包翻页 UI 态（#2633 批次4：page 从背包数据剥离为单一 UI 资源，背包/英雄背包/仓库
/// 三处共读避免翻页不同步 R8；Inventory 玩家组件本就不含 page，设计 §6/§8）。
#[derive(Resource, Default)]
pub struct InvUiState {
    /// 当前背包页（0=道具 1=道具2 2=任务；#276 双页扩容）
    pub page: usize,
}

/// 背包窗口原点：C# InventoryDialog 构造器**未设 Location**（InventoryDialog.cs:25-31）
/// → MirControl 默认 (0,0)（左上角，_location 字段零值，MirControl.cs:300）。
/// 旧值 (182,217) 误把 WeightBar 的**局部**坐标当对话框原点（InventoryDialog.cs:37）。
pub const DIALOG_X: f32 = 0.0;
pub const DIALOG_Y: f32 = 0.0;
/// 金币文本对话框相对坐标（C# InventoryDialog.cs:137 GoldLabel (40,212) 111x14）
pub const GOLD_TEXT_X: f32 = 40.0;
pub const GOLD_TEXT_Y: f32 = 212.0;
/// 负重文本对话框相对坐标（C# InventoryDialog.cs:190 WeightLabel (268,212) 26x14）
pub const WEIGHT_TEXT_X: f32 = 268.0;
pub const WEIGHT_TEXT_Y: f32 = 212.0;
/// 扩容按钮命中区尺寸（C# InventoryDialog.cs:84 AddButton Size(72,23)；精灵 Title[483] 自然
/// 48x25 绘于 (235,5) 不受影响，此处仅对齐可点击矩形——原 23x23 小于可见按钮，右半点了无反应）
pub const ADD_BTN_W: f32 = 72.0;
pub const ADD_BTN_H: f32 = 23.0;
const GRID_COLS: usize = 8;
const GRID_ROWS: usize = 5;
const QUEST_GRID_SIZE: usize = GRID_COLS * GRID_ROWS; // 任务格 8x5=40（C# QuestInventory）
const CELL_W: f32 = 36.0;
const CELL_H: f32 = 32.0;

/// 背包背景 Title[196] 真实尺寸（C# InventoryDialog.Size 即背景图尺寸；
/// 探针实测 316x236，缺失回退同值）。交易开窗推位公式用（TradeDialogs.cs:154
/// `ScreenWidth - InventoryDialog.Size.Width`）。
pub fn inventory_real_size(libs: &mut GameLibraries) -> (f32, f32) {
    match libs.0.get_image(LibraryName::Title, 196) {
        Some(i) => (i.width.max(0) as f32, i.height.max(0) as f32),
        None => (316.0, 236.0),
    }
}

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

/// 负重条（C# WeightBar Prguse[24] 84x6，按填充度裁宽）
#[derive(Component)]
pub struct InvWeightBar;

/// 负重条填充（C# WeightBar_BeforeDraw :396-426）：percent = weight/max_weight
/// clamp [0,1]，宽度 = (84-3)*percent；色调近似三段（白≤50%/黄≤75%/红>75%，
/// 原 UI_32bit[471/470] 素材本机数据缺失——#2611 偏差）
fn inv_weight_bar_system(
    inv_q: Query<&Inventory, With<LocalPlayer>>,
    mut bars: Query<(&mut Node, &mut ImageNode), With<InvWeightBar>>,
) {
    let (max, weight) = inv_q
        .single()
        .map(|inv| (inv.max_weight, inv.weight))
        .unwrap_or((0, 0));
    let percent = if max == 0 {
        0.0
    } else {
        (weight as f32 / max as f32).clamp(0.0, 1.0)
    };
    let tint = if percent <= 0.50 {
        Color::srgb(1.0, 1.0, 1.0)
    } else if percent <= 0.75 {
        Color::srgb(1.0, 0.85, 0.3)
    } else {
        Color::srgb(1.0, 0.35, 0.25)
    };
    for (mut node, mut img) in &mut bars {
        img.color = tint;
        // 左端对齐裁宽（宽度按比例缩放；percent=0 不绘制——C# :402 早退）
        node.width = Val::Px((84.0 - 3.0) * percent);
    }
}

pub struct InventoryDialogPlugin;

impl Plugin for InventoryDialogPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InvClickState>();
        app.init_resource::<InvDropConfirm>();
        app.init_resource::<InvPendingAmount>();
        app.init_resource::<ItemUseFeedback>();
        app.init_resource::<InventoryOrigin>();
        app.init_resource::<InvUiState>();
        // #2631：背包自我右移让位（交易开窗解耦；背包实体/Origin 归本模块所有）
        app.add_message::<InventoryShiftRight>();
        app.add_systems(OnEnter(AppState::Game), spawn_inventory_dialog);
        app.add_systems(OnEnter(AppState::Game), spawn_inv_confirm);
        app.add_systems(OnExit(AppState::Game), cleanup_dialogs);
        // #2633 批次4：背包/装备 ServerEvent 写系统（玩家状态集，先于 Hud 读）。
        // 排序备注：同一帧 UserInformation 快照（全量覆盖 quest_inventory）必须先于
        // QuestItemGained 增量应用——否则先增量后快照会把本帧新任务物品抹掉。
        app.add_systems(
            Update,
            inventory_events
                .before(quest_inventory_events)
                .in_set(GameSet::PlayerState)
                .run_if(in_state(AppState::Game)),
        );
        app.add_systems(
            Update,
            (
                inventory_shift_right_system,
                inv_grid_sync_system,
                inventory_ui_system,
                inv_weight_bar_system,
                inv_selection_system,
                inv_tooltip_system,
                inv_socket_open_system,
                inv_item_action_system,
                inv_confirm_system,
                inv_add_del_buttons_system,
                quest_inventory_events,
                inv_sound_system,
            )
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

/// #1342：GainedQuestItem/DeleteQuestItem 增量更新任务格（C# QuestInventory）
/// #2633 批次4 步9：直接写 `Inventory` 组件（HudState 双写已删）。
/// R1 迟读递延（与 inventory_events/belt_restock_events 同构）：实体未生成时**先查实体
/// 后读事件**——reader 游标停在原地，实体生成帧连同登录窗口事件一起按到达序应用
/// （评审 finding 3 根因：此前 `continue` 于 single_mut 失败 = 边读边弃，窗口内
/// 任务物品整局丢失）。配合排序边 inventory_events.before(quest_inventory_events)，
/// UserInformation 快照（全量覆盖）必先于本帧增量。
/// 限制：Bevy 消息 2 帧寿命——实体若 2 帧内未生成，窗口事件过期丢失；与 inventory/belt
/// 既有限制一致（登录 UserInformation→ObjectPlayer 正常 0-1 帧）。
pub(crate) fn quest_inventory_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut inv_q: Query<&mut Inventory, With<LocalPlayer>>,
) {
    use crate::network::server_event::ServerEvent;
    let Ok(mut inv) = inv_q.single_mut() else {
        return; // 迟读递延：不读事件，游标留待实体生成帧
    };
    for ev in events.read() {
        match ev {
            ServerEvent::QuestItemGained { item } => {
                if let Some(slot) = inv
                    .quest_inventory
                    .iter_mut()
                    .find(|s| s.is_none())
                {
                    *slot = Some(item.clone());
                } else {
                    inv.quest_inventory.push(Some(item.clone()));
                }
                inv.quest_inventory.truncate(QUEST_GRID_SIZE);
            }
            ServerEvent::QuestItemDeleted { unique_id, count } => {
                for slot in inv.quest_inventory.iter_mut() {
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

/// 背包/装备写系统（#2633 批次4 步2：拆 hud_server_events 背包域，设计 §10）。
///
/// #2633 批次4 步9：HudState 删除后直接操作玩家实体 `Inventory`/`Loadout` 组件（唯一数据源）；
/// 实体未生成则本帧事件整体跳过（R1：UserInformation 等状态事件由 PendingPlayerEvents
/// 缓冲、实体生成后按序回放——其余事件由 UserInformation 全量兜底）。UserInformation 玩家属性部分归 player_vitals_events（两系统
/// 读同一事件、各写不同字段，MessageReader 游标独立合法）；ItemUsed 的腰带补货段归
/// potion_belt::belt_restock_events（须排在本系统扣减之前）。任务格增量由 quest_inventory_events 处理。
pub(crate) fn inventory_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut inv_q: Query<
        (
            &mut crate::game::player_state::Inventory,
            &mut crate::game::player_state::Loadout,
        ),
        With<LocalPlayer>,
    >,
) {
    use crate::network::server_event::ServerEvent;
    let Ok((mut inv, mut loadout)) = inv_q.single_mut() else {
        return;
    };
    for ev in events.read() {
        match ev {
            ServerEvent::InventoryMoved { from, to } => {
                if *from < inv.items.len() && *to < inv.items.len() {
                    inv.items.swap(*from, *to);
                }
            }
            ServerEvent::ItemEquipped { unique_id, to } => {
                // 从背包移除并放入装备槽；旧装备放回背包空格
                let from_idx = inv
                    .items
                    .iter()
                    .position(|s| s.as_ref().map(|it| it.unique_id) == Some(*unique_id));
                if let Some(from_idx) = from_idx {
                    let item = inv.items[from_idx].take();
                    if let Some(item) = item {
                        if *to < loadout.slots.len() {
                            let old = loadout.slots[*to].take();
                            loadout.slots[*to] = Some(item);
                            if let Some(old) = old {
                                if let Some(empty) =
                                    inv.items.iter_mut().find(|s| s.is_none())
                                {
                                    *empty = Some(old);
                                }
                            }
                        }
                    }
                }
            }
            ServerEvent::ItemRemoved { unique_id } => {
                // 卸下装备：清空装备槽并放回背包空格
                let mut item = None;
                for slot in loadout.slots.iter_mut() {
                    if slot.as_ref().map(|it| it.unique_id) == Some(*unique_id) {
                        item = slot.take();
                        break;
                    }
                }
                if let Some(item) = item {
                    if let Some(empty) = inv.items.iter_mut().find(|s| s.is_none()) {
                        *empty = Some(item);
                    }
                }
            }
            ServerEvent::UserInformation { .. } => {
                // 背包/装备部分（玩家属性部分归 player_vitals_events；金币唯一源是 Gold 组件）。
                // 与 reconcile 共用 apply_user_info_items 同一份映射。
                crate::game::player_state::apply_user_info_items(ev, &mut inv, &mut loadout);
            }
            ServerEvent::ItemUsed { unique_id } => {
                // 背包扣减段（腰带补货段归 belt_restock_events，须先于此运行）
                let idx = inv
                    .items
                    .iter()
                    .position(|s| s.as_ref().map(|it| it.unique_id) == Some(*unique_id));
                if let Some(idx) = idx {
                    let count = inv.items[idx]
                        .as_ref()
                        .map(|it| it.count)
                        .unwrap_or(0);
                    if count > 1 {
                        if let Some(it) = inv.items[idx].as_mut() {
                            it.count -= 1;
                        }
                    } else {
                        inv.items[idx] = None;
                    }
                    tracing::info!("💊 使用物品 uid={} 剩余 {}", unique_id, count.saturating_sub(1));
                    inv.refresh_weight();
                }
            }
            ServerEvent::ItemDuraChanged {
                unique_id,
                current_dura,
            } => {
                // #228：背包/装备栏按 unique_id 更新当前耐久
                let mut updated = false;
                for slot in inv.items.iter_mut().flatten() {
                    if slot.unique_id == *unique_id {
                        slot.current_dura = *current_dura;
                        updated = true;
                        break;
                    }
                }
                if !updated {
                    for slot in loadout.slots.iter_mut().flatten() {
                        if slot.unique_id == *unique_id {
                            slot.current_dura = *current_dura;
                            break;
                        }
                    }
                }
                tracing::info!("🔧 物品耐久变化 uid={} dura={}", unique_id, current_dura);
            }
            ServerEvent::ItemRepaired {
                unique_id,
                max_dura,
                current_dura,
            } => {
                // #240：修理结果 → 更新背包/装备栏 当前/最大耐久
                let mut updated = false;
                for slot in inv.items.iter_mut().flatten() {
                    if slot.unique_id == *unique_id {
                        slot.current_dura = *current_dura;
                        slot.max_dura = *max_dura;
                        updated = true;
                        break;
                    }
                }
                if !updated {
                    for slot in loadout.slots.iter_mut().flatten() {
                        if slot.unique_id == *unique_id {
                            slot.current_dura = *current_dura;
                            slot.max_dura = *max_dura;
                            break;
                        }
                    }
                }
                tracing::info!(
                    "🔧 物品修理 uid={} dura={}/{}",
                    unique_id,
                    current_dura,
                    max_dura
                );
            }
            ServerEvent::ItemSlotSizeChanged {
                unique_id,
                slot_size,
            } => {
                // #240：镶嵌槽位数量变化 → 调整 slots 长度
                for slot in inv.items.iter_mut().flatten() {
                    if slot.unique_id == *unique_id {
                        let n = (*slot_size).max(0) as usize;
                        slot.slots.resize(n, None);
                        break;
                    }
                }
                tracing::info!("📐 物品槽位变化 uid={} size={}", unique_id, slot_size);
            }
            ServerEvent::ItemUpgraded { item } => {
                // #258：物品升级 → 按 unique_id 替换背包/装备栏物品
                let mut updated = false;
                for slot in inv.items.iter_mut().flatten() {
                    if slot.unique_id == item.unique_id {
                        *slot = item.clone();
                        updated = true;
                        break;
                    }
                }
                if !updated {
                    for slot in loadout.slots.iter_mut().flatten() {
                        if slot.unique_id == item.unique_id {
                            *slot = item.clone();
                            break;
                        }
                    }
                }
                tracing::info!("⬆️ 物品升级替换: {}", item.name);
            }
            ServerEvent::ItemDeleted { unique_id } => {
                // #228：背包按 unique_id 删除（消耗/删除）
                let idx = inv
                    .items
                    .iter()
                    .position(|s| s.as_ref().map(|it| it.unique_id) == Some(*unique_id));
                if let Some(idx) = idx {
                    inv.items[idx] = None;
                    tracing::info!("🗑️ 删除物品 uid={}", unique_id);
                }
            }
            ServerEvent::ItemGained { item } => {
                // #228：获得物品 → 放入第一个空格
                if let Some(slot) = inv.items.iter_mut().find(|s| s.is_none()) {
                    *slot = Some(item.clone());
                    tracing::info!("🎁 获得物品入包: {} (uid={})", item.name, item.unique_id);
                } else {
                    tracing::warn!("🎒 背包已满，无法放入: {}", item.name);
                }
            }
            ServerEvent::InventoryResized { size } => {
                // #276：背包扩容（C# S.ResizeInventory → Array.Resize）
                inv.resize(*size);
                tracing::info!("🎒 背包扩容 -> {} 格", inv.items.len());
            }
            _ => {}
        }
    }
}

/// 生成背包对话框实体（初始隐藏，由 HUD 按钮/管理器显示）
fn spawn_inventory_dialog(
    mut commands: Commands,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<UiFont>,
    mut origin: ResMut<InventoryOrigin>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();
    // 场景重入重置原点（实体按常量重生成；资源若残留上局的推位/拖动偏移会脱节）
    *origin = InventoryOrigin(DIALOG_X, DIALOG_Y);

    // 背景 Title[196]（实测 316x236）@ (0,0)
    let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Title, 196) else {
        return;
    };
    let panel = spawn_panel(&mut commands, bg, DIALOG_X, DIALOG_Y, 316.0, 236.0, 30);
    commands.entity(panel).insert((
        DialogRoot(DialogKind::Inventory),
        InventoryPanel,
        DialogWidget,
    ));

    commands.entity(panel).with_children(|p| {
        // 标签页按钮（Title 737/197 道具，738/168 道具2，739/198 任务）
        // #1342：任务页签（QuestGrid 8x5，C# QuestInventory）
        let tabs: [(usize, usize, usize, f32); 3] = [
            (0, 737, 197, 6.0),
            (1, 738, 168, 76.0),
            (2, 739, 198, 146.0),
        ];
        for (idx, normal, hover, x) in tabs {
            if let (Some(n), Some(h), Some(pr)) = (
                load_lib_image(&mut libs, &mut images, LibraryName::Title, normal),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, hover),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, hover),
            ) {
                spawn_icon_button(p, n, h, pr, x, 7.0, 72.0, 23.0, 8).insert(InvTab(idx));
            }
        }
        // 关闭按钮（Prguse2 360/361/362）@(289,3)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 360),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 361),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 362),
        ) {
            spawn_icon_button(p, n, h, pr, 289.0, 3.0, 20.0, 20.0, 8).insert(InvCloseBtn);
        }
        // 金币/负重文本
        spawn_label(p, &font, "0", GOLD_TEXT_X, GOLD_TEXT_Y, 12.0, Color::WHITE, 8)
            .insert(InvGoldText);
        spawn_label(p, &font, "0/0", WEIGHT_TEXT_X, WEIGHT_TEXT_Y, 12.0, Color::WHITE, 8)
            .insert(InvWeightText);
        // 负重条（C# WeightBar：Prguse[24] 实测 84x6 @(182,217)，按填充度裁宽）
        if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 24) {
            spawn_image(p, h, 182.0, 217.0, 84.0, 6.0, 7).insert(InvWeightBar);
        }
        // 扩展背包格购买按钮（C# InventoryDialog AddButton：Title 483/484/485 @(235,5)）
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 483),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 484),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 485),
        ) {
            spawn_icon_button(p, n, h, pr, 235.0, 5.0, ADD_BTN_W, ADD_BTN_H, 8).insert(InvAddBtn);
        }
        // 删除模式按钮（C# InventoryDialog DelItemButton：Prguse2 366/367/368 @(291,212)）
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 366),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 367),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 368),
        ) {
            spawn_icon_button(p, n, h, pr, 291.0, 212.0, 20.0, 20.0, 8).insert(InvDelBtn);
        }
    });
    // 格子背景不在此预生成：#276 由 inv_grid_sync_system 按 Inventory 组件 items.len()
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

/// 背包对话框**当前**原点（屏幕坐标）。静态常量 DIALOG_X/Y 是初始位；仓库/交易
/// 开窗推位（C# NPCDialogs.cs:2967、TradeDialogs.cs:154）与拖动都会移动对话框，
/// 命中计算必须用本资源（spawn 时重置为初始位防场景重入残留）。
#[derive(Resource)]
pub struct InventoryOrigin(pub f32, pub f32);

impl Default for InventoryOrigin {
    fn default() -> Self {
        Self(DIALOG_X, DIALOG_Y)
    }
}

/// 交易打开时请求背包右移让位（#2631 跨对话框解耦 Message）。
/// C# TradeDialog.TradeAccept（TradeDialogs.cs:152-161）：
/// `InventoryDialog.Location = new Point(ScreenWidth - inv.W, 0)` —— 背包推到屏幕右侧。
/// 所有权：背包面板 Node.left / [`InventoryOrigin`] 仅由本模块改写；
/// 交易等外部对话框只发本 Message，由 [`inventory_shift_right_system`] 自我重排（幂等）。
#[derive(Message, Debug)]
pub struct InventoryShiftRight;

/// 光标坐标 → 背包格（按当前页与格数）；供仓库/交易/英雄对话框复用。
/// 对齐 C# InventoryDialog：page 0=道具（0..min(40,size)），1=道具2（40..size-1），
/// 位置 (i%8, (i/8)%5) 复用同一 8x5 区域（C# Grid Location = y%5）。
/// origin 取 [`InventoryOrigin`]——背包可能已被推位/拖动。
pub fn inv_slot_at(
    cx: f32,
    cy: f32,
    page: usize,
    size: usize,
    origin: (f32, f32),
) -> Option<usize> {
    let size = size.min(MAX_INV_SLOTS);
    let range: std::ops::Range<usize> = match page {
        0 => 0..size.min(GRID_COLS * GRID_ROWS),
        1 => (GRID_COLS * GRID_ROWS)..size,
        _ => return None,
    };
    for i in range {
        let x = i % GRID_COLS;
        let y = (i / GRID_COLS) % GRID_ROWS;
        let sx = origin.0 + 9.0 + x as f32 * (CELL_W + 1.0);
        let sy = origin.1 + 37.0 + y as f32 * (CELL_H + 1.0);
        if cx >= sx && cx <= sx + CELL_W && cy >= sy && cy <= sy + CELL_H {
            return Some(i);
        }
    }
    None
}

/// 背包格子索引（0..MAX_INV_SLOTS-1）
#[derive(Component, Clone, Copy)]
pub struct InvSlot(pub usize);

/// 双击检测 + 背包/英雄背包「当前选中」共享选择态（C# GameScene.SelectedCell 语义）。
///
/// **所有权（#2631）**：本状态归 inventory 模块所有；一切变更经下方公开方法进行，
/// 外部对话框不直接读写字段——
/// - 读当前背包选中格 → [`InvClickState::selected`]；
/// - 「读并清」转移语义（仓库存入/腰带穿戴/英雄转移后清选中）→ [`InvClickState::take_selected`]；
/// - 互斥清空（选中他处物品时取消背包选中）→ [`InvClickState::clear_selected`]；
/// - 英雄背包选中格 → [`InvClickState::hero_selected`] / [`InvClickState::toggle_hero_selected`]
///   / [`InvClickState::clear_hero_selected`]。
/// 取代旧「靠注释约定谁负责清选中」的隐式契约。
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

impl InvClickState {
    /// 读当前背包选中格（只读，不改动选择态）。
    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    /// 「读并清」：取出当前背包选中格并清空（转移/穿戴/出售后不再保留选中）。
    pub fn take_selected(&mut self) -> Option<usize> {
        self.selected.take()
    }

    /// 清空背包选中（选中他处物品/取消时互斥；幂等）。
    pub fn clear_selected(&mut self) {
        self.selected = None;
    }

    /// 读英雄背包选中格（只读）。
    pub fn hero_selected(&self) -> Option<usize> {
        self.hero_selected
    }

    /// 清空英雄背包选中格（转移到英雄/取消时；幂等）。
    pub fn clear_hero_selected(&mut self) {
        self.hero_selected = None;
    }

    /// 切换英雄背包选中格（再点同一格取消；空格由调用方保证不传入）。
    pub fn toggle_hero_selected(&mut self, slot: usize) {
        self.hero_selected = if self.hero_selected == Some(slot) {
            None
        } else {
            Some(slot)
        };
    }
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

/// 丢弃/删除/扩容确认文本（迁移补齐：原版该文本从未被渲染——spawn 空串且无系统写它）
#[derive(Component)]
pub struct InvConfirmText;

/// 显示/隐藏 + 页切换 + 关闭 + 物品图标渲染 + 双击使用/装备
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn inventory_ui_system(
    mut mgr: ResMut<DialogManager>,
    player_q: Query<(&Inventory, &Gold), With<LocalPlayer>>,
    mut inv_ui: ResMut<InvUiState>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    // 背景/标签/关闭/格子 统一显隐（格子带 InvSlot）
    mut all_vis: Query<
        (&mut Visibility, Option<&InvSlot>),
        (
            With<DialogWidget>,
            Without<InvGoldText>,
            Without<InvWeightText>,
        ),
    >,
    mut cells_data: Query<(&InvSlot, &mut UiItemCellData)>,
    buttons: Query<
        (Entity, &Interaction, Option<&InvTab>),
        (
            With<DialogWidget>,
            With<Button>,
            Without<InvSlot>,
            Without<InvGoldText>,
            Without<InvWeightText>,
            // 加格/删格按钮由 inv_add_del_buttons_system 处理，排除避免误关窗口
            Without<InvAddBtn>,
            Without<InvDelBtn>,
        ),
    >,
    mut money: Query<
        (
            &mut Text,
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
    let player = player_q.single().ok();
    let inv = player.map(|(inv, _)| inv);
    let open = mgr.is_open(DialogKind::Inventory);
    let size = inv.map(|inv| inv.items.len()).unwrap_or(0).min(MAX_INV_SLOTS);
    // 格子弹页显隐（#276）：道具=0..min(40,size)，道具2=40..size-1，任务页=0..40（QuestGrid）
    for (mut vis, slot) in &mut all_vis {
        let visible = if !open {
            false
        } else {
            match slot {
                Some(s) => match inv_ui.page {
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
        // 关闭时同时隐藏金币/负重文本（money 查询在 open 分支才更新，否则残留可见）
        for (_, mut vis, _, _) in &mut money {
            *vis = Visibility::Hidden;
        }
        return;
    }

    // 物品数据 → 通用 ItemCell（图标/数量/耐久条由 item_cell_ui_system 渲染）
    for (slot, mut data) in &mut cells_data {
        let item = if inv_ui.page == 2 {
            inv.and_then(|i| i.quest_inventory.get(slot.0).and_then(|s| s.as_ref()))
        } else {
            inv.and_then(|i| i.items.get(slot.0).and_then(|s| s.as_ref()))
        };
        match item {
            Some(item) => {
                let handle = load_lib_image(
                    &mut libs,
                    &mut images,
                    crate::resources::libraries::LibraryName::Items,
                    item.image as usize,
                );
                data.icon = handle;
                data.count = if item.count > 1 {
                    Some(item.count as u32)
                } else {
                    None
                };
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
    // 标签页切换 / 关闭按钮（bevy_ui Interaction 边沿）
    for (e, inter, tab) in &buttons {
        if !edge(e, inter, &mut prev_inter) {
            continue;
        }
        match tab {
            Some(t) => {
                inv_ui.page = t.0;
                tracing::debug!("背包页 -> {}", t.0);
            }
            None => mgr.close(DialogKind::Inventory),
        }
    }
    for (mut t, mut vis, gold, weight) in &mut money {
        *vis = Visibility::Visible;
        if gold.is_some() {
            t.0 = format!("{}", player.map(|(_, g)| g.0).unwrap_or(0));
        } else if weight.is_some() {
            let (w, mw) = inv.map(|i| (i.weight, i.max_weight)).unwrap_or((0, 0));
            t.0 = format!("{}/{}", w, mw);
        }
    }
}

/// 悬停提示系统（#93/#106 通用 Tooltip）：物品格上显示 名称 + 类型/数量/耐久
/// 命中用 InventoryOrigin（#2560：背包推位/拖动后 tooltip 跟随）
fn inv_tooltip_system(
    inv_q: Query<&Inventory, With<LocalPlayer>>,
    inv_ui: Res<InvUiState>,
    mut tooltip: ResMut<crate::ui::tooltip::TooltipState>,
    origin: Res<InventoryOrigin>,
    windows: Query<&Window>,
    slots: Query<&InvSlot>,
) {
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else {
        return;
    };

    let inv = inv_q.single().ok();
    let page = inv_ui.page;
    let size = inv.map(|i| i.items.len()).unwrap_or(0).min(MAX_INV_SLOTS);
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
        let sx = origin.0 + 9.0 + x as f32 * (CELL_W + 1.0);
        let sy = origin.1 + 37.0 + y as f32 * (CELL_H + 1.0);
        if cursor.x >= sx && cursor.x <= sx + CELL_W && cursor.y >= sy && cursor.y <= sy + CELL_H {
            hit = if page == 2 {
                inv.and_then(|i2| i2.quest_inventory.get(i).and_then(|s| s.as_ref()).cloned())
            } else {
                inv.and_then(|i2| i2.items.get(i).and_then(|s| s.as_ref()).cloned())
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
        for (bit, n) in [
            (1u8, "战士"),
            (2, "法师"),
            (4, "道士"),
            (8, "刺客"),
            (16, "弓箭手"),
        ] {
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

/// 背包动态格子同步（#276）：按 Inventory 组件 items.len() 生成/移除 InvSlot 格子。
/// 对齐 C# InventoryDialog.Grid（8x10，位置 y%5 复用）；缩容时移除多余格子。
/// 扩容补格按 InventoryOrigin 生成（#2560：背包不在 (0,0) 时新格与已平移格对齐）。
#[allow(clippy::too_many_arguments)]
fn inv_grid_sync_system(
    mut commands: Commands,
    inv_q: Query<&Inventory, With<LocalPlayer>>,
    mut images: ResMut<Assets<Image>>,
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<UiFont>,
    slots: Query<(Entity, &InvSlot)>,
    panel: Query<Entity, With<InventoryPanel>>,
) {
    let inv = inv_q.single().ok();
    let size = inv.map(|i| i.items.len()).unwrap_or(0).min(MAX_INV_SLOTS);
    if inv.map(|i| i.items.is_empty()).unwrap_or(true) && slots.is_empty() {
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
    // 扩容：补缺失格子（bevy_ui 子格，坐标相对面板根——面板可被交易推位/拖动）
    let Ok(panel) = panel.single() else {
        return;
    };
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
        let sx = 9.0 + x as f32 * (CELL_W + 1.0);
        let sy = 37.0 + y as f32 * (CELL_H + 1.0);
        let mut cell = Entity::PLACEHOLDER;
        commands.entity(panel).with_children(|p| {
            cell = spawn_item_cell_ui(p, &mut images, &font, sx, sy, CELL_W, CELL_H, 6, i).id();
        });
        commands.entity(cell).insert((
            DialogRoot(DialogKind::Inventory),
            DialogWidget,
            InvSlot(i),
        ));
    }
}

/// 消费 [`InventoryShiftRight`]：交易开窗时背包自我右移让位（#2631 跨对话框解耦）。
/// 逻辑等同 C# TradeDialog.TradeAccept；背包平移自身面板根 Node.left
/// 并覆写 [`InventoryOrigin`]（幂等——重复推位时 min 已在目标处，delta=0）。
/// 查询/过滤与旧 trade.rs `push_inventory_right` 完全一致（按 DialogKind::Inventory 过滤，
/// 与本插件其它系统组件访问不重叠，无 B0001）。
#[allow(clippy::type_complexity)]
fn inventory_shift_right_system(
    mut events: MessageReader<InventoryShiftRight>,
    mut libs: ResMut<GameLibraries>,
    mut inv_entities: Query<(&mut Node, &DialogRoot)>,
    mut inv_origin: ResMut<InventoryOrigin>,
) {
    let mut shift = false;
    for _ in events.read() {
        shift = true;
    }
    if !shift {
        return;
    }
    let (inv_w, _) = inventory_real_size(&mut libs);
    let target_x = 1024.0 - inv_w;
    // bevy_ui：背包面板根 Node.left = 屏幕 x；子节点（格/按钮/文本）随根整体平移
    let mut min_x = f32::MAX;
    for (node, root) in inv_entities.iter() {
        if root.0 != DialogKind::Inventory {
            continue;
        }
        if let Val::Px(v) = node.left {
            min_x = min_x.min(v);
        }
    }
    if min_x == f32::MAX {
        return; // 背包未生成
    }
    let dx = target_x - min_x;
    for (mut node, root) in inv_entities.iter_mut() {
        if root.0 != DialogKind::Inventory {
            continue;
        }
        let cur = match node.left {
            Val::Px(v) => v,
            _ => 0.0,
        };
        node.left = Val::Px(cur + dx);
    }
    // 同步 InventoryOrigin（镶嵌面板锚定 / Ctrl+右键入口等读背包当前原点的系统跟随推位）
    *inv_origin = InventoryOrigin(target_x, 0.0);
}

/// 选中格子高亮（原版 C# SelectedCell 黄色边框语义：用黄色半透明覆盖表示）
fn inv_selection_system(
    click: Res<InvClickState>,
    mut slots: Query<(&mut BackgroundColor, &InvSlot)>,
) {
    for (mut bg, slot) in &mut slots {
        let selected = click.selected == Some(slot.0);
        let target = if selected {
            Color::srgba(1.0, 0.9, 0.2, 0.35)
        } else {
            Color::srgba(0.0, 0.0, 0.0, 0.18)
        };
        if bg.0 != target {
            bg.0 = target;
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
        chat.add_line(
            msg,
            crate::game::chat::chat_color(mir2_shared::enums::ChatType::System),
            ChatChannel::System,
        );
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
    fishing: bool,
    now: f64,
    feedback: &mut ItemUseFeedback,
) -> bool {
    if now < feedback.last_use {
        return false;
    }
    if fishing {
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
        ItemType::Weapon => 10111,   // ClickWeapon
        ItemType::Armour => 10112,   // ClickArmour
        ItemType::Helmet => 10116,   // ClickHelmet
        ItemType::Necklace => 10115, // ClickNecklace
        ItemType::Bracelet => 10114, // ClickBracelet
        ItemType::Ring => 10113,     // ClickRing
        ItemType::Boots => 10117,    // ClickBoots
        ItemType::Potion => 10108,   // ClickDrug
        _ => 10118,                  // ClickItem
    })
}

/// #1544：CanUseItem 客户端检查（C# MirItemCell.CanUseItem：性别/职业/等级）
/// 返回 Err(提示语) 时不应发包；服务端仍会二次校验（#576）。
fn can_use_item_check(
    item: &InvItem,
    gender: u8,
    class: u8,
    level: u16,
) -> Result<(), &'static str> {
    // 性别：RequiredGender Male=1 Female=2；0/3(NONE=both) 视为不限制
    let gbit = 1u8 << gender; // MirGender Male=0→1, Female=1→2
    if item.required_gender != 0 && item.required_gender != 3 && (item.required_gender & gbit) == 0
    {
        return Err("性别不符");
    }
    // 职业：RequiredClass Warrior=1 Wizard=2 Taoist=4 Assassin=8 Archer=16
    let cbit = 1u8 << class; // MirClass Warrior=0 Wizard=1 Taoist=2 Assassin=3 Archer=4
    if item.required_class != 0 && item.required_class != 31 && (item.required_class & cbit) == 0 {
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
/// equipment 恒为**主角色**装备槽（C# 语义：坐骑/钓具槽物品看 User 装备，英雄格同；
/// #2633 批次4 步6：调用方传 `Loadout` 组件 slots，不再读 hud.equipment）。
fn slot_item_ready(equipment: &[Option<InvItem>], grid_to: MirGridType) -> bool {
    match grid_to {
        MirGridType::Mount => equipment.get(10).and_then(|s| s.as_ref()).is_some(),
        MirGridType::Fishing => matches!(
            equipment
                .get(0)
                .and_then(|s| s.as_ref())
                .map(|w| w.shape),
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
    /// 主背包（装备槽传 `Loadout` 组件 slots——步6；gender/class/level 步7 前仍由调用方
    /// 从 hud 取、步7 改 ActorAppearance/Progression，签名不再变）。
    pub fn player(equipment: &'a [Option<InvItem>], gender: u8, class: u8, level: u16) -> Self {
        Self {
            grid: MirGridType::Inventory,
            equipment,
            gender,
            class,
            level,
            check_fishing: true,
            allow_consumable: true,
        }
    }
}

/// 使用/装备物品（#1544 对齐 C# MirItemCell.UseItem 守卫）：

/// #1546：守卫链纯逻辑（不发包，便于单测）
/// 返回 Some(true)=可继续（已通过守卫）；Some(false)=槽物品无坐骑/鱼竿；None=被拦截
/// equipment 恒为主角色 `Loadout` slots（槽物品前置看 User 装备，步6）；
/// #2633 批次4 步7：riding 改由调用方读 `MountState` 传入（HudState 已于步9 删除）。
#[allow(clippy::too_many_arguments)]
fn use_item_guard(
    item: &InvItem,
    riding: bool,
    fishing: bool,
    equipment: &[Option<InvItem>],
    ctx: UseItemCtx,
    now: f64,
    feedback: &mut ItemUseFeedback,
) -> Option<bool> {
    // 1. 节流
    if now < feedback.last_use {
        return None;
    }
    // 2. 钓鱼（英雄格跳过：C# !HeroGridType && User.Fishing）
    if ctx.check_fishing && fishing {
        feedback.messages.push("钓鱼中无法使用物品".to_string());
        return None;
    }
    // 3. 骑乘（仅 Scroll/Potion/Torch 可用）
    {
        use mir2_shared::enums::ItemType;
        let t = ItemType::try_from(item.item_type).ok();
        if riding
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
        return Some(slot_item_ready(equipment, grid_to));
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
    riding: bool,
    fishing: bool,
    equipment: &[Option<InvItem>],
    ctx: UseItemCtx,
    now: f64,
    feedback: &mut ItemUseFeedback,
    confirm: &mut InvDropConfirm,
) -> UseOutcome {
    // 守卫链（节流/钓鱼/骑乘/SoulBound/CanUseItem/槽物品前置）
    match use_item_guard(item, riding, fishing, equipment, ctx, now, feedback) {
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
        if let Some(to) =
            item.equip_slot_occupied(|s| ctx.equipment.get(s).and_then(|x| x.as_ref()).is_some())
        {
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
    tracing::debug!(
        "背包物品 {} 不可用/不可装备 (grid={:?})",
        item.name,
        ctx.grid
    );
    UseOutcome::Blocked
}

/// 主背包快捷使用（#1544 包装，调用点保持兼容）
/// #2633 批次4 步7：gender/class/level/riding 由调用方读组件传入
///（`ActorAppearance`/`Progression`/`MountState`；HudState 已于步9 删除）
#[allow(clippy::too_many_arguments)]
fn use_or_equip(
    item: &InvItem,
    net: &NetConnection,
    gender: u8,
    class: u8,
    level: u16,
    riding: bool,
    fishing: bool,
    equipment: &[Option<InvItem>],
    now: f64,
    feedback: &mut ItemUseFeedback,
    confirm: &mut InvDropConfirm,
) -> UseOutcome {
    use_item_core(
        item,
        net,
        riding,
        fishing,
        equipment,
        UseItemCtx::player(equipment, gender, class, level),
        now,
        feedback,
        confirm,
    )
}
/// #1346：扩展背包购买/删除模式按钮（C# InventoryDialog AddButton / DelItemButton）
#[allow(clippy::too_many_arguments)]
fn inv_add_del_buttons_system(
    inv_q: Query<&Inventory, With<LocalPlayer>>,
    mut click: ResMut<InvClickState>,
    mut confirm: ResMut<InvDropConfirm>,
    mgr: Res<DialogManager>,
    add_btn: Query<(Entity, &Interaction), With<InvAddBtn>>,
    del_btn: Query<(Entity, &Interaction), With<InvDelBtn>>,
    mut add_vis: Query<&mut Visibility, With<InvAddBtn>>,
    mut del_img: Query<&mut ImageNode, With<InvDelBtn>>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
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
    let len = inv_q.single().map(|inv| inv.items.len()).unwrap_or(0);
    // C# AddButton.Visible = openLevel < 10（上限 86 格）；
    // 必须先判断背包对话框是否打开，否则关闭后按钮残留成屏幕上的孤按钮
    let can_expand = mgr.is_open(DialogKind::Inventory) && len < MAX_INV_EXPAND;
    for mut vis in &mut add_vis {
        *vis = if can_expand {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    // 删除模式图标（C# DelItemButton.Index 366 ↔ 368）
    let del_idx = if click.delete_mode { 368 } else { 366 };
    if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, del_idx) {
        for mut img in &mut del_img {
            img.image = h.clone();
        }
    }
    for (e, inter) in &add_btn {
        if edge(e, inter, &mut prev_inter) && can_expand {
            // C# cost = 1M + openLevel*1M（openLevel = (len-46)/4；Rust 基线 40）
            let level = len.saturating_sub(GRID_COLS * GRID_ROWS) / 4;
            let cost = 1_000_000u64 + (level as u64) * 1_000_000u64;
            confirm.text = format!("花费 {} 金币扩展背包格？", cost);
            confirm.mode = 2;
            confirm.visible = true;
        }
    }
    for (e, inter) in &del_btn {
        if edge(e, inter, &mut prev_inter) {
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
    let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 360) else {
        return;
    };
    let panel = spawn_panel(&mut commands, h, bx, by, 456.0, 190.0, 45);
    commands.entity(panel).insert((InvConfirmWidget, Visibility::Hidden));
    commands.entity(panel).with_children(|p| {
        spawn_label(p, &font, "", 35.0, 35.0, 12.0, Color::WHITE, 9)
            .insert((InvConfirmWidget, InvConfirmText));
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 206),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 207),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 208),
        ) {
            spawn_icon_button(p, n, h, pr, 260.0, 157.0, 76.0, 25.0, 10).insert(InvConfirmYes);
        }
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 210),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 211),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 212),
        ) {
            spawn_icon_button(p, n, h, pr, 360.0, 157.0, 76.0, 25.0, 10).insert(InvConfirmNo);
        }
    });
}

/// 丢弃确认框：Yes → DropItem；No → 关闭（原版 C# MirMessageBox YesNo）
fn inv_confirm_system(
    mut confirm: ResMut<InvDropConfirm>,
    mut click: ResMut<InvClickState>,
    net: Res<NetConnection>,
    mut widgets: Query<&mut Visibility, With<InvConfirmWidget>>,
    mut texts: Query<&mut Text, With<InvConfirmText>>,
    yes: Query<(Entity, &Interaction), (With<InvConfirmYes>, Without<InvConfirmNo>)>,
    no: Query<(Entity, &Interaction), (With<InvConfirmNo>, Without<InvConfirmYes>)>,
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
    for mut vis in &mut widgets {
        *vis = if confirm.visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    // 迁移补齐：原版确认文本从未渲染（spawn 空串且无系统写它）
    for mut t in &mut texts {
        let s = confirm.text.clone();
        if t.0 != s {
            t.0 = s;
        }
    }
    if !confirm.visible {
        return;
    }
    for (e, inter) in &yes {
        if !edge(e, inter, &mut prev_inter) {
            continue;
        }
        match confirm.mode {
            1 => {
                // #1346：删除模式（C# PromptDelete → C.DeleteItem），删除后退出删除模式
                net.send_packet(&mir2_shared::packets::client::item::DeleteItem {
                    unique_id: confirm.unique_id,
                    count: confirm.count as u16,
                    hero_inventory: false,
                });
                click.delete_mode = false;
                tracing::info!(
                    "🗑️ 确认删除 uid={} count={}",
                    confirm.unique_id,
                    confirm.count
                );
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
                tracing::info!(
                    "🗑️ 确认丢弃 uid={} count={}",
                    confirm.unique_id,
                    confirm.count
                );
            }
        }
        confirm.visible = false;
        click.selected = None;
    }
    for (e, inter) in &no {
        if edge(e, inter, &mut prev_inter) {
            confirm.visible = false;
        }
    }
}

/// Ctrl+右键：打开镶嵌面板（C# MirItemCell.OpenItem）——独立系统避免主系统参数超限（Bevy 16 上限）
#[allow(clippy::too_many_arguments)]
fn inv_socket_open_system(
    inv_q: Query<&Inventory, With<LocalPlayer>>,
    inv_ui: Res<InvUiState>,
    mut mgr: ResMut<DialogManager>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window>,
    origin: Res<InventoryOrigin>,
    mut socket: ResMut<crate::game::dialogs::socket::SocketState>,
) {
    if !mouse.just_pressed(MouseButton::Right) || !keys.pressed(KeyCode::ControlLeft) {
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Ok(inv) = inv_q.single() else { return };
    let page = inv_ui.page;
    let size = inv.items.len().min(MAX_INV_SLOTS);
    let range: std::ops::Range<usize> = match page {
        0 => 0..size.min(GRID_COLS * GRID_ROWS),
        1 => (GRID_COLS * GRID_ROWS)..size,
        _ => 0..0,
    };
    for i in range {
        let x = i % GRID_COLS;
        let y = (i / GRID_COLS) % GRID_ROWS;
        // 命中用 InventoryOrigin——背包可能已被仓库/交易推位或拖动
        let sx = origin.0 + 9.0 + x as f32 * (CELL_W + 1.0);
        let sy = origin.1 + 37.0 + y as f32 * (CELL_H + 1.0);
        if cursor.x >= sx && cursor.x <= sx + CELL_W && cursor.y >= sy && cursor.y <= sy + CELL_H {
            if let Some(item) = inv.items.get(i).and_then(|s| s.as_ref()) {
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

/// 点是否落在任一可见对话框面板根节点矩形内（丢弃门用）。
/// C# 语义：MirImageControl 构造器 `AutoSize = true`（MirImageControl.cs:170）
/// → `Size = Library.GetTrueSize(Index)`——对话框按背景图全幅吞掉点击，
/// 不落到地图 MouseDown（GameScene.cs:11361 的丢弃流程）。bevy_ui 迁移后
/// 各对话框 = 根面板 Node（left/top/width/height 即屏幕矩形），随推位/拖动恒准。
fn cursor_over_dialog<'a>(
    cursor: Vec2,
    mut dialogs: impl Iterator<Item = (&'a Node, &'a Visibility)>,
) -> bool {
    dialogs.any(|(node, vis)| {
        *vis == Visibility::Visible && {
            let x = match node.left {
                Val::Px(v) => v,
                _ => 0.0,
            };
            let y = match node.top {
                Val::Px(v) => v,
                _ => 0.0,
            };
            let w = match node.width {
                Val::Px(v) => v,
                _ => 0.0,
            };
            let h = match node.height {
                Val::Px(v) => v,
                _ => 0.0,
            };
            cursor.x >= x && cursor.x <= x + w && cursor.y >= y && cursor.y <= y + h
        }
    })
}

/// 物品高级交互：
///   - 右键 → 使用/装备（原版 C# MouseButtons.Right → UseItem）
///   - Shift+左键 → 拆分堆叠（MirAmountBox → SplitItem）
///   - 选中物品 + 点场景地面 → 丢弃（单件 YesNo 确认 / 多件数量框 → DropItem）
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn inv_item_action_system(
    // #2633 批次4 步7：gender/class/level/riding 读组件（HudState 已于步9 删除）
    player_q: Query<
        (
            &Inventory,
            &StatusFlags,
            &Loadout,
            &crate::actor::ActorAppearance,
            &crate::game::player_state::Progression,
            Option<&crate::actor::MountState>,
        ),
        With<LocalPlayer>,
    >,
    inv_ui: Res<InvUiState>,
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
    // 元组参数折叠（系统参数上限 16）：全部 UI 按钮 Interaction / 数量框结果 /
    // 背包命中原点 / 对话框根面板（丢弃门矩形）
    mut misc: (
        Query<&Interaction, With<Button>>,
        MessageReader<AmountBoxResult>,
        Res<InventoryOrigin>,
        Query<(&Node, &Visibility), With<DialogRoot>>,
    ),
    // 弹窗模态门：上一帧有弹窗 → 本帧点击视为弹窗按钮，不处理格子（原版 C# Modal）
    mut last_modal: Local<bool>,
) {
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else {
        return;
    };

    // #1342：任务物品格只读（C# MirGridType.QuestInventory 不可移动/使用）
    if inv_ui.page == 2 {
        return;
    }

    // 数量框结果：拆分/丢弃
    for r in misc.1.read() {
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

    // 光标下的背包格（按当前页与格数，#276；原点取 InventoryOrigin——推位/拖动后仍准确）
    let Ok((inv, flags, loadout, appearance, progression, mount)) = player_q.single() else { return };
    let my_gender = appearance.gender as u8;
    let my_class = appearance.class as u8;
    let my_level = progression.level;
    let riding = mount.is_some();
    let page = inv_ui.page;
    let size = inv.items.len().min(MAX_INV_SLOTS);
    let (ox, oy) = (misc.2.0, misc.2.1);
    let slot_at = |cx: f32, cy: f32| -> Option<usize> {
        let range: std::ops::Range<usize> = match page {
            0 => 0..size.min(GRID_COLS * GRID_ROWS),
            1 => (GRID_COLS * GRID_ROWS)..size,
            _ => 0..0,
        };
        for i in range {
            let x = i % GRID_COLS;
            let y = (i / GRID_COLS) % GRID_ROWS;
            let sx = ox + 9.0 + x as f32 * (CELL_W + 1.0);
            let sy = oy + 37.0 + y as f32 * (CELL_H + 1.0);
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
            if let Some(item) = inv.items.get(i).and_then(|s| s.as_ref()) {
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
                        let same_stack = inv
                            .items
                            .get(i)
                            .and_then(|s| s.as_ref())
                            .zip(inv.items.get(from).and_then(|s| s.as_ref()))
                            .map(|(t, f)| {
                                t.item_index == f.item_index && t.unique_id != f.unique_id
                            })
                            .unwrap_or(false);
                        if same_stack {
                            if let (Some(from_item), Some(to_item)) = (
                                inv.items.get(from).and_then(|s| s.as_ref()),
                                inv.items.get(i).and_then(|s| s.as_ref()),
                            ) {
                                net.send_packet(&mir2_shared::packets::client::item::MergeItem {
                                    grid_from: MirGridType::Inventory,
                                    grid_to: MirGridType::Inventory,
                                    id_from: from_item.unique_id,
                                    id_to: to_item.unique_id,
                                });
                                tracing::info!(
                                    "🔗 合并物品 {} -> {}（uid {} -> {}）",
                                    from,
                                    i,
                                    from_item.unique_id,
                                    to_item.unique_id
                                );
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
                        if inv.items.get(i).and_then(|s| s.as_ref()).is_some() {
                            click.selected = Some(i);
                        }
                    }
                }
            }
        }
    }
    // 双击：使用/装备
    if let Some(i) = dbl {
        if let Some(item) = inv.items.get(i).and_then(|s| s.as_ref()) {
            if use_or_equip(item, &net, my_gender, my_class, my_level, riding, flags.fishing, &loadout.slots, now, &mut feedback, &mut confirm)
                == UseOutcome::Sent
            {
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
            if let Some(item) = inv.items.get(i).and_then(|s| s.as_ref()) {
                if use_or_equip(item, &net, my_gender, my_class, my_level, riding, flags.fishing, &loadout.slots, now, &mut feedback, &mut confirm)
                    == UseOutcome::Sent
                {
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
                if let Some(item) = inv.items.get(i).and_then(|s| s.as_ref()) {
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
            if let Some(item) = inv.items.get(i).and_then(|s| s.as_ref()) {
                if item.count > 1 {
                    if !inv.items.iter().any(|s| s.is_none()) {
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

    // 选中物品 + 左键点场景（非背包格/非任何对话框/非按钮）→ 丢弃流程
    if mouse.just_pressed(MouseButton::Left) {
        let Some(sel) = click.selected else { return };
        if slot_at(cursor.x, cursor.y).is_some() {
            return;
        }
        // 点在任一可见对话框精灵 bbox 内不触发——C# 控件路由：对话框
        // （MirImageControl 构造器 AutoSize=true → Size=背景图 TrueSize）
        // 吞掉点击不落到地图 MouseDown（GameScene.cs:11361 才处理丢弃）。
        // 实体 bbox 覆盖一切对话框（背包面板/角色装备区等），推位/拖动后
        // 恒准——旧两处静态矩形（DIALOG+318x256 / character::DIALOG_X）
        // 在推位/拖动后失准（#2575）
        if cursor_over_dialog(cursor, misc.3.iter()) {
            return;
        }
        // 任意 UI 按钮上不触发（bevy_ui：Interaction 由 ui_focus_system 按命中计算）
        if misc.0.iter().any(|i| *i != Interaction::None) {
            return;
        }
        let Some(item) = inv.items.get(sel).and_then(|s| s.as_ref()) else {
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
        if mir2_shared::enums::ItemType::try_from(it.item_type)
            == Ok(mir2_shared::enums::ItemType::Potion)
        {
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

    /// 推位/拖动后 inv_slot_at 必须用 InventoryOrigin（PR #2553 审查：仓库开仓把背包
    /// 推到 (393,0)，静态原点 0,0 的命中全部落空——存取/使用/选择失效）。
    #[test]
    fn inv_slot_at_follows_inventory_origin() {
        // 初始位 (0,0)：首格左上 (9,37)
        assert_eq!(inv_slot_at(10.0, 38.0, 0, 80, (0.0, 0.0)), Some(0));
        assert_eq!(inv_slot_at(10.0, 38.0, 0, 80, (393.0, 0.0)), None);
        // 推位 (393,0) 后：首格命中移到 (393+9, 37)
        assert_eq!(inv_slot_at(402.0, 38.0, 0, 80, (393.0, 0.0)), Some(0));
        // 拖动到任意位 (100,50)
        assert_eq!(inv_slot_at(109.0, 88.0, 0, 80, (100.0, 50.0)), Some(0));
        // 默认原点 = 初始常量位
        assert_eq!(
            (InventoryOrigin::default().0, InventoryOrigin::default().1),
            (DIALOG_X, DIALOG_Y)
        );
    }

    /// #2560：扩容补格为面板子节点（bevy_ui 相对坐标）——背包推位/拖动后
    /// 新格随面板整体平移，与既有格恒对齐
    #[test]
    fn inv_grid_sync_spawns_cells_at_inventory_origin() {
        use bevy::ecs::system::RunSystemOnce;

        let mut world = World::new();
        // 2 格背包（既有 0，扩容补 1）：#2633 批次4 后 inv_grid_sync 读 Inventory 组件
        world.spawn((
            LocalPlayer,
            Inventory {
                items: vec![None, None],
                ..Default::default()
            },
        ));
        // 背包被推位到 (393,50)
        world.insert_resource(InventoryOrigin(393.0, 50.0));
        world.insert_resource(Assets::<Image>::default());
        world.insert_resource(Assets::<Font>::default());
        world.insert_resource(crate::ui::sprite_ui::UiFont::default());
        // 面板根（bevy_ui Node @ 推位后的绝对坐标）
        let panel = world
            .spawn((
                InventoryPanel,
                DialogWidget,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(393.0),
                    top: Val::Px(50.0),
                    ..default()
                },
            ))
            .id();
        // 既有格 0（面板子节点，相对 (9,37)）
        world.entity_mut(panel).with_children(|p| {
            p.spawn((
                InvSlot(0),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(9.0),
                    top: Val::Px(37.0),
                    ..default()
                },
            ));
        });

        world
            .run_system_once(inv_grid_sync_system)
            .expect("grid sync 应成功");

        // 扩容补出格 1：面板子节点，相对坐标 (9+37, 37)——不随推位变绝对坐标
        let mut q = world.query_filtered::<(&InvSlot, &Node, &ChildOf), ()>();
        let cell1 = q
            .iter(&world)
            .find(|(s, _, _)| s.0 == 1)
            .map(|(_, node, co)| {
                (
                    match node.left {
                        Val::Px(v) => v,
                        _ => -999.0,
                    },
                    match node.top {
                        Val::Px(v) => v,
                        _ => -999.0,
                    },
                    co.parent(),
                )
            })
            .expect("扩容应补出格 1");
        assert_eq!(
            cell1,
            (9.0 + 1.0 * (CELL_W + 1.0), 37.0, panel),
            "新格应为面板子节点且相对坐标对齐"
        );
    }

    /// R8：背包页签写入单一 InvUiState 资源（背包/英雄背包/仓库共读同一资源，翻页天然同步）。
    /// 点击任务页签（InvTab(2)）→ 共享 InvUiState.page 变 2，而非旧 hud.inventory.page。
    #[test]
    fn inventory_page_lives_in_shared_inv_ui_state() {
        use bevy::ecs::system::RunSystemOnce;

        let mut world = World::new();
        let mut mgr = DialogManager::default();
        mgr.open.push(DialogKind::Inventory);
        world.insert_resource(mgr);
        world.insert_resource(InvUiState::default());
        world.insert_resource(GameLibraries::default());
        world.insert_resource(Assets::<Image>::default());
        // 任务页签按钮（Pressed 边沿触发；带 Visibility 走 all_vis 分支，无 InvSlot → 恒可见）
        world.spawn((DialogWidget, Button, Interaction::Pressed, InvTab(2)));

        world
            .run_system_once(inventory_ui_system)
            .expect("inventory_ui_system 应运行");

        assert_eq!(
            world.resource::<InvUiState>().page,
            2,
            "页签点击应写入共享 InvUiState（R8 单一翻页源）"
        );
    }

    /// #2631：InventoryShiftRight → 背包自我右移让位（替代旧 trade.rs push_inventory_right）。
    /// bevy_ui 迁移：平移面板根 Node.left 并覆写 InventoryOrigin；子节点随根整体移动，
    /// 按钮命中区不再需要单独同步（bevy_ui Interaction 按布局命中）。
    #[test]
    fn inventory_shift_right_repositions_entities_and_origin() {
        use bevy::ecs::message::Messages;
        use bevy::ecs::system::RunSystemOnce;

        let mut world = World::new();
        world.insert_resource(Messages::<InventoryShiftRight>::default());
        // 未初始化的 GameLibraries → inventory_real_size 走缺省 (316,236)，无磁盘 IO
        world.insert_resource(GameLibraries::default());
        world.insert_resource(InventoryOrigin::default());
        // 背包两枚实体（初始位 (0,0) 基准：背景 (0,0)、首格 (9,37)）——bevy_ui Node
        world.spawn((
            DialogRoot(DialogKind::Inventory),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                ..default()
            },
        ));
        world.spawn((
            DialogRoot(DialogKind::Inventory),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(9.0),
                top: Val::Px(37.0),
                ..default()
            },
        ));
        // 非背包实体（交易窗）不应被平移
        world.spawn((
            DialogRoot(DialogKind::Trade),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(298.0),
                top: Val::Px(418.0),
                ..default()
            },
        ));
        world
            .resource_mut::<Messages<InventoryShiftRight>>()
            .write(InventoryShiftRight);

        world
            .run_system_once(inventory_shift_right_system)
            .expect("shift right 应成功");

        // target_x = 1024 - 316 = 708
        let mut q = world.query_filtered::<(&Node, &DialogRoot), ()>();
        let inv_min_x = q
            .iter(&world)
            .filter(|(_, r)| r.0 == DialogKind::Inventory)
            .map(|(n, _)| match n.left {
                Val::Px(v) => v,
                _ => f32::MAX,
            })
            .fold(f32::MAX, f32::min);
        assert_eq!(inv_min_x, 708.0, "背包左缘应右移到 target_x");
        let trade_x = q
            .iter(&world)
            .find(|(_, r)| r.0 == DialogKind::Trade)
            .map(|(n, _)| match n.left {
                Val::Px(v) => v,
                _ => -999.0,
            })
            .expect("交易实体存在");
        assert_eq!(trade_x, 298.0, "非背包实体不应移动");
        // InventoryOrigin 覆写
        let origin = world.resource::<InventoryOrigin>();
        assert_eq!((origin.0, origin.1), (708.0, 0.0));
    }

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
            grade: 0,
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

    /// #2575：丢弃门用对话框根面板矩形——推位/拖动后旧静态矩形（背包
    /// DIALOG+318x256 / character::DIALOG_X 装备格）失准；bevy_ui 面板根
    /// Node.left/top/width/height 即屏幕矩形，随推位/拖动恒准
    #[test]
    fn drop_gate_uses_dialog_entity_bbox() {
        let mut world = World::new();

        let mut q = world.query_filtered::<(&Node, &Visibility), With<DialogRoot>>();

        // 背包被推位到 (393,50)（交易/仓库推位），面板根 Title[196] 316x236
        let inv = world
            .spawn((
                DialogRoot(DialogKind::Inventory),
                Visibility::Visible,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(393.0),
                    top: Val::Px(50.0),
                    width: Val::Px(316.0),
                    height: Val::Px(236.0),
                    ..default()
                },
            ))
            .id();
        // 旧位置 (150,150)：推位后不在面板内 → 放行（丢弃流程应触发）
        assert!(!cursor_over_dialog(Vec2::new(150.0, 150.0), q.iter(&world)));
        // 推位后面板内 (500,150) 命中
        assert!(cursor_over_dialog(Vec2::new(500.0, 150.0), q.iter(&world)));

        // 关闭（Hidden）的对话框不吞
        world.entity_mut(inv).insert(Visibility::Hidden);
        assert!(!cursor_over_dialog(Vec2::new(500.0, 150.0), q.iter(&world)));
        world.entity_mut(inv).insert(Visibility::Visible);

        // 角色对话框拖动到 (300,180)：装备区随实体命中（旧静态
        // character::DIALOG_X 失准——#2575 装备区门）
        world.spawn((
            DialogRoot(DialogKind::Character),
            Visibility::Visible,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(300.0),
                top: Val::Px(180.0),
                width: Val::Px(280.0),
                height: Val::Px(340.0),
                ..default()
            },
        ));
        assert!(cursor_over_dialog(Vec2::new(340.0, 200.0), q.iter(&world)));
        // 旧角色对话框原点处不再命中
        assert!(!cursor_over_dialog(Vec2::new(60.0, 200.0), q.iter(&world)));

        // 无尺寸的对话框实体退化为点，不吞区域
        world.spawn((
            DialogRoot(DialogKind::Menu),
            Visibility::Visible,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(600.0),
                top: Val::Px(400.0),
                ..default()
            },
        ));
        assert!(!cursor_over_dialog(Vec2::new(650.0, 450.0), q.iter(&world)));

        // 边界含端点（>= / <=）
        assert!(cursor_over_dialog(Vec2::new(393.0, 50.0), q.iter(&world)));
        assert!(cursor_over_dialog(
            Vec2::new(393.0 + 316.0, 50.0 + 236.0),
            q.iter(&world)
        ));
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
        let mut equipment = vec![None; 14];
        assert!(!slot_item_ready(&equipment, MirGridType::Mount));
        assert!(!slot_item_ready(&equipment, MirGridType::Fishing));
        let m = item_with_type(ItemType::Mount);
        equipment[10] = Some(m);
        assert!(slot_item_ready(&equipment, MirGridType::Mount));
        let mut rod = item_with_type(ItemType::Weapon);
        rod.shape = 49;
        equipment[0] = Some(rod);
        assert!(slot_item_ready(&equipment, MirGridType::Fishing));
        let mut sword = item_with_type(ItemType::Weapon);
        sword.shape = 0;
        equipment[0] = Some(sword);
        assert!(!slot_item_ready(&equipment, MirGridType::Fishing));
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
        assert_eq!(
            item_use_sound_id(&item_with_type(ItemType::Weapon)),
            Some(10111)
        );
        assert_eq!(
            item_use_sound_id(&item_with_type(ItemType::Potion)),
            Some(10108)
        );
        assert_eq!(
            item_use_sound_id(&item_with_type(ItemType::Food)),
            Some(10118)
        );
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
        let mut inv = crate::game::player_state::Inventory::default();
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
        // #2633 步9：装备槽读 `Loadout` 组件（HudState 已删；默认 14 空槽）
        let loadout = Loadout::default();
        let mut fb = ItemUseFeedback::default();
        let potion = item_with_type(ItemType::Potion);
        let ctx_hero = UseItemCtx {
            grid: MirGridType::HeroInventory,
            equipment: &loadout.slots,
            gender: 0,
            class: 0,
            level: 1,
            check_fishing: false,
            allow_consumable: true,
        };
        assert!(use_item_guard(&potion, false, true, &loadout.slots, ctx_hero, 0.0, &mut fb).is_some());
        // 主背包 check_fishing=true → 钓鱼拦截
        let ctx_player = UseItemCtx::player(&loadout.slots, 0, 0, 1);
        assert!(use_item_guard(&potion, false, true, &loadout.slots, ctx_player, 0.0, &mut fb).is_none());
    }

    #[test]
    fn guard_storage_blocks_consumable_but_allows_equip() {
        // #1546：仓库格 allow_consumable=false → 消耗品拦截；装备放行
        let loadout = Loadout::default();
        let mut fb = ItemUseFeedback::default();
        let ctx_storage = UseItemCtx {
            grid: MirGridType::Storage,
            equipment: &loadout.slots,
            gender: 0,
            class: 0,
            level: 1,
            check_fishing: true,
            allow_consumable: false,
        };
        // 守卫本身通过（消耗品拦截在 use_item_core 第 8 步）
        let potion = item_with_type(ItemType::Potion);
        assert!(use_item_guard(&potion, false, false, &loadout.slots, ctx_storage, 0.0, &mut fb).is_some());
        // 装备放行
        let sword = item_with_type(ItemType::Weapon);
        assert!(use_item_guard(&sword, false, false, &loadout.slots, ctx_storage, 0.0, &mut fb).is_some());
    }

    #[test]
    fn use_item_core_storage_blocks_consumable() {
        // 仓库双击药水 → Blocked（不发包）；仓库双击武器且槽空 → Sent
        let net = NetConnection::default();
        let loadout = Loadout::default();
        let mut fb = ItemUseFeedback::default();
        let mut confirm = InvDropConfirm::default();
        let ctx_storage = UseItemCtx {
            grid: MirGridType::Storage,
            equipment: &loadout.slots,
            gender: 0,
            class: 0,
            level: 1,
            check_fishing: true,
            allow_consumable: false,
        };
        let potion = item_with_type(ItemType::Potion);
        assert_eq!(
            use_item_core(&potion, &net, false, false, &loadout.slots, ctx_storage, 0.0, &mut fb, &mut confirm),
            UseOutcome::Blocked
        );
        let sword = item_with_type(ItemType::Weapon);
        assert_eq!(
            use_item_core(&sword, &net, false, false, &loadout.slots, ctx_storage, 0.0, &mut fb, &mut confirm),
            UseOutcome::Sent
        );
    }

    #[test]
    fn use_item_core_hero_equips_with_hero_equipment() {
        let net = NetConnection::default();
        // #1546：英雄格装备用英雄装备槽判断（ctx.equipment）
        let loadout = Loadout::default();
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
            use_item_core(&bracelet, &net, false, false, &loadout.slots, ctx_empty, 0.0, &mut fb, &mut confirm),
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
            use_item_core(&bracelet, &net, false, false, &loadout.slots, ctx_full, 0.0, &mut fb, &mut confirm),
            UseOutcome::Blocked
        );
    }
}

// ============================================================================
// 本地玩家状态组件（#2633 批次4：HudState/CharacterState God Resource → 玩家实体组件）
// 设计：ecsplayer-design.md §6 组件 schema、§7 挂载、§10 系统拆分。
//
// 迁移策略（设计 §11）：批1-批7 双写过渡，读者逐批迁到组件；步8 删除 `CharacterState`
// 双源（读者已迁 Vitals/Progression/CombatStats/PlayerName），步9 删除 `HudState` 双写
// 与资源本身（读者已迁全套玩家组件）——终态各 ServerEvent 写系统只写 `LocalPlayer`
// 实体上的组件，唯一数据源。
//
// 聚合原则（设计 §6）：谁一起变（同一事件写）、谁一起被读（同一批 Query）就聚成一个
// 组件；并优先复用实体已有组件（PlayerName/NetObjectId/ActorAppearance/MountState，
// 不新建）。
// ============================================================================

use bevy::prelude::*;
use mir2_shared::enums::PetMode;

use crate::actor::{LocalPlayer, PlayerName};
use crate::game::dialogs::inventory::InvItem;
use crate::game::hud::HudState;
use crate::game::sets::GameSet;
use crate::network::server_event::ServerEvent;
use crate::scenes::AppState;

/// 生命/法力（HealthChanged{hp,mp} 恒同写；HUD 血蓝球、自动喝药、施法、角色面板同读）。
///
/// 默认值对齐 `HudState`（hp=1/max_hp=1000/mp=1/max_mp=600），使组件从生成起即为
/// `HudState` 的等值镜像，避免后续读者迁移后首帧读到全 0。
#[derive(Component, Clone, Copy, PartialEq)]
pub struct Vitals {
    pub hp: i32,
    pub max_hp: i32,
    pub mp: i32,
    pub max_mp: i32,
}

impl Default for Vitals {
    fn default() -> Self {
        Self {
            hp: 1,
            max_hp: 1000,
            mp: 1,
            max_mp: 600,
        }
    }
}

/// 等级/经验（LevelChanged 同写三者；HUD 经验条/等级、mentor、quest_log 同读）。
/// 默认值对齐 `HudState`（level=1/max_exp=100）。
#[derive(Component, Clone, Copy, PartialEq)]
pub struct Progression {
    pub level: u16,
    pub exp: i64,
    pub max_exp: i64,
}

impl Default for Progression {
    fn default() -> Self {
        Self {
            level: 1,
            exp: 0,
            max_exp: 100,
        }
    }
}

/// 金币（GoldGained/Lost 独立事件；HUD/背包/交易读）。单字段组件，最终消解
/// `hud.gold` 与 `hud.inventory.gold` 双源（设计 §9）。
#[derive(Component, Clone, Copy, PartialEq, Default)]
pub struct Gold(pub u32);

/// 声望/功勋（#248，CreditGained/Lost 独立事件；仅 auto/world 读）。
#[derive(Component, Clone, Copy, PartialEq, Default)]
pub struct Credit(pub u32);

/// 基础属性（#268，BaseStats 独立事件；auto/world 读）。
#[derive(Component, Clone, PartialEq, Default)]
pub struct BaseStats(pub Vec<i32>);

/// 角色面板战斗属性（UserInformation 一次同写；仅 character.rs 面板读）。
/// 对应 `CharacterState` 的 18 个面板属性 + stats（设计 §2/§6）。
#[derive(Component, Clone, Copy, PartialEq, Default)]
pub struct CombatStats {
    /// [min, max] AC/MAC/DC/MC/SC
    pub stats: [[i32; 2]; 5],
    pub critical_rate: i32,
    pub critical_damage: i32,
    pub attack_speed: i32,
    pub accuracy: i32,
    pub agility: i32,
    pub luck: i32,
    pub bag_weight: i32,
    pub wear_weight: i32,
    pub hand_weight: i32,
    pub magic_resist: i32,
    pub poison_resist: i32,
    pub health_recovery: i32,
    pub spell_recovery: i32,
    pub poison_recovery: i32,
    pub holy: i32,
    pub freezing: i32,
    pub poison_atk: i32,
}

/// 状态旗标（输入/移动/物品门控同读；各事件分别写但都是 bool，聚一起省 Query 数）。
/// 骑乘不复读——复用 `MountState`（存在即骑乘，由 object_state/spawn 维护）。
///
/// 注：sprint/sneaking 由 buff.rs 事件写（设计 §5），本批尚未迁移该写者，暂为 false。
#[derive(Component, Clone, Copy, PartialEq, Default)]
pub struct StatusFlags {
    pub dead: bool,
    pub fishing: bool,
    pub paralysis: bool,
    pub in_trap_rock: bool,
    pub sprint: bool,
    pub sneaking: bool,
    pub reincarnation_offered: bool,
}

/// 宠物模式（#1388，PetModeChanged 独立事件；HUD attack_mode_text 读）。
#[derive(Component, Clone, Copy, PartialEq)]
pub struct PetModeState(pub PetMode);

impl Default for PetModeState {
    fn default() -> Self {
        Self(PetMode::Both)
    }
}

/// 装备 14 槽（UserInformation/ItemEquipped/ItemRemoved/耐久/修理/升级写；
/// character/dura/mount/fishing/storage 读）。
///
/// 注：`InvItem` 未 derive `PartialEq`，故本组件暂不 derive `PartialEq`（后续读者
/// 迁移需要 `!=` 比较时再给 `InvItem` 补上）。
#[derive(Component, Clone)]
pub struct Loadout {
    pub slots: Vec<Option<InvItem>>,
}

impl Default for Loadout {
    fn default() -> Self {
        Self {
            // #1136：服务端补 Torch/Belt/Stone 共 14 槽（对齐 HudState.equipment 默认）
            slots: vec![None; 14],
        }
    }
}

/// 背包（UserInformation/背包 CRUD 写；~15 文件读）。
/// 已按设计 §6 剥离：`page`→背包 UI 资源、`gold`→`Gold` 组件；保留
/// `weight`/`max_weight`（服务端 bag_weight + 本地 refresh_weight）。
#[derive(Component, Clone, Default)]
pub struct Inventory {
    /// 动态格数背包（默认空，UserInformation 全量写入；ResizeInventory 扩容/缩容，#276）
    pub items: Vec<Option<InvItem>>,
    pub weight: u32,
    pub max_weight: u32,
    /// 任务物品格（C# QuestInventory；UserInformation.quest_inventory 写入）
    pub quest_inventory: Vec<Option<InvItem>>,
}

impl Inventory {
    /// 按服务端 ResizeInventory 调整格数（C# Array.Resize：截断/补空，上限 MAX_INV_SLOTS）。
    /// 逻辑同 `InventoryState::resize`；本批写路径用全量镜像（见 inventory_events），
    /// 该方法供后续「直接操作组件」的读者迁移批次使用。
    pub fn resize(&mut self, size: usize) {
        let size = size.min(crate::game::dialogs::inventory::MAX_INV_SLOTS);
        if size < self.items.len() {
            self.items.truncate(size);
        } else {
            self.items.resize(size, None);
        }
    }

    /// #1544：RefreshStats 重量（C# User.RefreshStats 从物品重量重算）。
    /// 逻辑同 `InventoryState::refresh_weight`；用途同 `resize`。
    pub fn refresh_weight(&mut self) {
        self.weight = self
            .items
            .iter()
            .flatten()
            .map(|it| it.weight as u32 * it.count as u32)
            .sum();
    }
}

/// 本地自动喝药行为（非服务端态：开关 + 冷却计时；auto_potion_system 读写）。
/// 本批仅挂载默认值，写者迁移（auto_potion_system）属后续批次（设计 §5/§11）。
#[derive(Component, Clone, Copy, PartialEq)]
pub struct AutoPotion {
    pub enabled: bool,
    pub cooldown: f32,
}

impl Default for AutoPotion {
    fn default() -> Self {
        // 对齐 HudState.auto_pot_hp 默认 true
        Self {
            enabled: true,
            cooldown: 0.0,
        }
    }
}

/// 本地玩家状态组件包：生成 `LocalPlayer` 实体时一次性挂载全部默认值（设计 §7 挂载 A）。
/// 复用组件（PlayerName/NetObjectId/ActorAppearance/MountState）由既有 spawn/object_state
/// 路径维护，不在此包内。
#[derive(Bundle, Default)]
pub struct LocalPlayerStateBundle {
    pub vitals: Vitals,
    pub progression: Progression,
    pub gold: Gold,
    pub credit: Credit,
    pub base_stats: BaseStats,
    pub combat_stats: CombatStats,
    pub status_flags: StatusFlags,
    pub pet_mode: PetModeState,
    pub loadout: Loadout,
    pub inventory: Inventory,
    pub auto_potion: AutoPotion,
}

/// 登录首帧 UserInformation 缓冲（#2633 §12 R1）。
///
/// R1 已证实：UserInformation（属性）先于本地 ObjectPlayer（生成实体）到达，且
/// spawn_local_player_with 走 Commands 延迟到帧尾才生成实体。故 UserInformation 到达时
/// `LocalPlayer` 实体尚不存在，组件写被跳过。此处暂存该快照（latest wins，后到覆盖先到），
/// 待实体生成后由 `apply_pending_user_info` 应用一次。
///
/// 只缓冲 UserInformation——HealthChanged 等高频事件随后会自我纠正，不值得缓冲。
#[derive(Resource, Default)]
pub struct PendingUserInfo(pub Option<ServerEvent>);

// ============================================================================
// ServerEvent 写系统（#2633 批次4 步2：拆 hud_server_events，设计 §10）
//
// 双写过渡（设计 §11 批1）：每个系统把值同时写进玩家组件与原 `HudState`
// （CharacterState 双写已在步8 删除；hud.* 保留至步9）。
// 组件写用 `Query<&mut X, With<LocalPlayer>>` + `single_mut()`；实体未生成时
// （UserInformation 可能先于 ObjectPlayer 到达，设计 §12 R1）跳过组件写、仅写
// HudState 兜底，UserInformation 另由 PendingUserInfo 缓冲待实体生成后应用。
// ============================================================================

/// 从 UserInformation 事件把玩家属性/面板属性写入 Vitals/Progression/Gold/CombatStats。
///
/// `player_vitals_events` 与 `apply_pending_user_info` 共用此一份字段映射，避免双份漂移
/// （#2633 R1）。非 UserInformation 事件为 no-op。只写组件，不写 HudState
/// （hud 侧由调用方按双写过渡单独处理，步9 删）。
pub(crate) fn apply_user_info_stats(
    ev: &ServerEvent,
    vitals: &mut Vitals,
    progression: &mut Progression,
    gold: &mut Gold,
    combat_stats: &mut CombatStats,
) {
    let ServerEvent::UserInformation {
        level,
        hp,
        mp,
        exp,
        max_exp,
        gold: g,
        max_hp,
        max_mp,
        ac,
        mac,
        dc,
        mc,
        sc,
        critical_rate,
        critical_damage,
        attack_speed,
        accuracy,
        agility,
        luck,
        bag_weight,
        wear_weight,
        hand_weight,
        magic_resist,
        poison_resist,
        health_recovery,
        spell_recovery,
        poison_recovery,
        holy,
        freezing,
        poison_atk,
        ..
    } = ev
    else {
        return;
    };
    vitals.hp = *hp;
    vitals.max_hp = *max_hp;
    vitals.mp = *mp;
    vitals.max_mp = *max_mp;
    progression.level = *level;
    progression.exp = *exp;
    progression.max_exp = (*max_exp).max(1);
    gold.0 = *g;
    combat_stats.stats = [*ac, *mac, *dc, *mc, *sc];
    combat_stats.critical_rate = *critical_rate;
    combat_stats.critical_damage = *critical_damage;
    combat_stats.attack_speed = *attack_speed;
    combat_stats.accuracy = *accuracy;
    combat_stats.agility = *agility;
    combat_stats.luck = *luck;
    combat_stats.bag_weight = *bag_weight;
    combat_stats.wear_weight = *wear_weight;
    combat_stats.hand_weight = *hand_weight;
    combat_stats.magic_resist = *magic_resist;
    combat_stats.poison_resist = *poison_resist;
    combat_stats.health_recovery = *health_recovery;
    combat_stats.spell_recovery = *spell_recovery;
    combat_stats.poison_recovery = *poison_recovery;
    combat_stats.holy = *holy;
    combat_stats.freezing = *freezing;
    combat_stats.poison_atk = *poison_atk;
}

/// 从 UserInformation 事件把背包/装备写入 Inventory/Loadout。
///
/// `inventory_events` 与 `apply_pending_user_info` 共用此一份字段映射，避免双份漂移
/// （#2633 R1）。非 UserInformation 事件为 no-op。
pub(crate) fn apply_user_info_items(ev: &ServerEvent, inventory: &mut Inventory, loadout: &mut Loadout) {
    let ServerEvent::UserInformation {
        inventory: items,
        equipment,
        quest_inventory,
        bag_weight,
        ..
    } = ev
    else {
        return;
    };
    inventory.items = items.clone();
    inventory.quest_inventory = quest_inventory.clone();
    // #1544：RefreshStats 重量（max_weight=服务端 bag_weight；weight 由物品重算）
    inventory.max_weight = (*bag_weight).max(0) as u32;
    inventory.refresh_weight();
    loadout.slots = equipment.clone();
}

pub struct PlayerStatePlugin;

impl Plugin for PlayerStatePlugin {
    fn build(&self, app: &mut App) {
        // 写方须排在读方前（设计 §12 R5）：先写玩家组件/HudState，后 sync_hud_data（Hud 集）读。
        app.configure_sets(Update, GameSet::PlayerState.before(GameSet::Hud));
        app.init_resource::<PendingUserInfo>();
        // R1 reconcile：须在事件写系统之前运行——缓冲快照先应用，本帧新到事件后写（新值胜）。
        app.add_systems(
            Update,
            apply_pending_user_info
                .before(player_vitals_events)
                .before(crate::game::dialogs::inventory::inventory_events)
                .in_set(GameSet::PlayerState)
                .run_if(in_state(AppState::Game)),
        );
        app.add_systems(
            Update,
            (player_vitals_events, player_status_events)
                .in_set(GameSet::PlayerState)
                .run_if(in_state(AppState::Game)),
        );
    }
}

/// 玩家生命/法、金币/声望、基础属性、经验/等级、宠物模式、面板属性 + UserInformation 玩家属性部分。
/// （设计 §10 `player_vitals_events`；背包/装备部分归 inventory_events，二者读同一事件不同组件。）
/// #2633 批次4 步8：CharacterState 双写已删除；hud.* 双写保留至步9。
#[allow(clippy::too_many_arguments)]
fn player_vitals_events(
    mut events: MessageReader<ServerEvent>,
    mut hud: ResMut<HudState>,
    mut pending: ResMut<PendingUserInfo>,
    mut vitals_q: Query<&mut Vitals, With<LocalPlayer>>,
    mut progression_q: Query<&mut Progression, With<LocalPlayer>>,
    mut gold_q: Query<&mut Gold, With<LocalPlayer>>,
    mut credit_q: Query<&mut Credit, With<LocalPlayer>>,
    mut base_stats_q: Query<&mut BaseStats, With<LocalPlayer>>,
    mut pet_mode_q: Query<&mut PetModeState, With<LocalPlayer>>,
    mut combat_stats_q: Query<&mut CombatStats, With<LocalPlayer>>,
    mut name_q: Query<&mut PlayerName, With<LocalPlayer>>,
) {
    for ev in events.read() {
        match ev {
            ServerEvent::PetModeChanged { mode } => {
                // #1388：HUD 宠物模式标签
                hud.pet_mode = *mode;
                if let Ok(mut p) = pet_mode_q.single_mut() {
                    p.0 = *mode;
                }
            }
            ServerEvent::HealthChanged { hp, mp } => {
                hud.hp = *hp;
                hud.mp = *mp;
                if let Ok(mut v) = vitals_q.single_mut() {
                    v.hp = *hp;
                    v.mp = *mp;
                }
            }
            ServerEvent::GoldGained { gold } => {
                hud.gold = hud.gold.saturating_add(*gold);
                if let Ok(mut g) = gold_q.single_mut() {
                    g.0 = g.0.saturating_add(*gold);
                }
            }
            ServerEvent::BaseStats { stats } => {
                // #268：基础属性（角色面板数据）
                hud.base_stats = stats.clone();
                if let Ok(mut b) = base_stats_q.single_mut() {
                    b.0 = stats.clone();
                }
                tracing::info!("📊 基础属性: {:?}", stats);
            }
            ServerEvent::PlayerNameUpdated { name } => {
                // #264：本地玩家改名。双写：hud.name（过渡，步9 删）+ 复用组件 `PlayerName`
                //（object_state 亦有同名维护，值同，重复写无害）。
                hud.name = name.clone();
                if let Ok(mut n) = name_q.single_mut() {
                    n.0 = name.clone();
                }
                tracing::info!("🏷️ 玩家改名 -> {}", name);
            }
            ServerEvent::CreditGained { credit } => {
                // #248：声望增加
                hud.credit = hud.credit.saturating_add(*credit);
                if let Ok(mut c) = credit_q.single_mut() {
                    c.0 = c.0.saturating_add(*credit);
                }
                tracing::info!("🏅 获得声望 +{}（当前 {}）", credit, hud.credit);
            }
            ServerEvent::CreditLost { amount } => {
                // #248：声望减少
                hud.credit = hud.credit.saturating_sub(*amount);
                if let Ok(mut c) = credit_q.single_mut() {
                    c.0 = c.0.saturating_sub(*amount);
                }
                tracing::info!("🏅 失去声望 -{}（当前 {}）", amount, hud.credit);
            }
            ServerEvent::GoldLost { amount } => {
                hud.gold = hud.gold.saturating_sub(*amount);
                if let Ok(mut g) = gold_q.single_mut() {
                    g.0 = g.0.saturating_sub(*amount);
                }
                tracing::info!("💸 失去金币 -{}（当前 {}）", amount, hud.gold);
            }
            ServerEvent::ExperienceGained { amount } => {
                hud.exp += *amount;
                if let Ok(mut p) = progression_q.single_mut() {
                    p.exp += *amount;
                }
                tracing::info!("✨ 获得经验 +{}（当前 {}/{}）", amount, hud.exp, hud.max_exp);
            }
            ServerEvent::LevelChanged {
                level,
                exp,
                max_exp,
            } => {
                hud.level = *level;
                hud.exp = *exp;
                hud.max_exp = (*max_exp).max(1);
                if let Ok(mut p) = progression_q.single_mut() {
                    p.level = *level;
                    p.exp = *exp;
                    p.max_exp = (*max_exp).max(1);
                }
                tracing::info!("⬆️ 升级 Lv.{} exp={}/{}", level, exp, max_exp);
            }
            ServerEvent::UserInformation {
                name,
                level,
                hp,
                mp,
                exp,
                max_exp,
                gold,
                gender,
                class,
                object_id,
                max_hp,
                max_mp,
                ..
            } => {
                // —— HudState 玩家属性部分（inventory/equipment 部分归 inventory_events；
                // CharacterState 双写已随资源删除，步8）——
                hud.name = name.clone();
                hud.level = *level;
                hud.hp = *hp;
                hud.mp = *mp;
                hud.max_hp = *max_hp;
                hud.max_mp = *max_mp;
                hud.exp = *exp;
                hud.max_exp = (*max_exp).max(1);
                hud.gold = *gold;
                hud.class = *class;
                hud.gender = *gender;
                hud.player_object_id = Some(*object_id);
                // —— 玩家组件：实体已生成就地写入（共享映射），未生成则缓冲快照待 reconcile（R1）——
                // 全部组件同挂 LocalPlayer 实体（LocalPlayerStateBundle），故单一查询成败即实体有无。
                // PlayerName 单独写：它由 spawn 路径插入（非 Bundle 成员），不参与"实体有无"判定，
                // 缺失时静默跳过（同其他写系统 R1 语义）。
                if let (Ok(mut v), Ok(mut p), Ok(mut g), Ok(mut cb)) = (
                    vitals_q.single_mut(),
                    progression_q.single_mut(),
                    gold_q.single_mut(),
                    combat_stats_q.single_mut(),
                ) {
                    apply_user_info_stats(ev, &mut v, &mut p, &mut g, &mut cb);
                } else {
                    // R1：UserInformation 先于 ObjectPlayer 到达、实体未生成——缓冲快照
                    // （latest wins），待 apply_pending_user_info 在实体生成后应用一次。
                    pending.0 = Some(ev.clone());
                }
                // #2633 批次4 步7：补写复用组件 `PlayerName`（读者已迁该组件；hud.name 双写保留）
                if let Ok(mut n) = name_q.single_mut() {
                    n.0 = name.clone();
                }
            }
            _ => {}
        }
    }
}

/// 玩家状态旗标（钓鱼/陷阱/麻痹/死亡/复活/轮回）+ 骑乘 HudState 镜像。
/// （设计 §10 `player_status_events`；sprint/sneaking 由 buff.rs 写、本批未迁移；
/// MountUpdated 的 MountState 由 object_state/spawn 维护，此处仅写 HudState.riding/mount_type。）
fn player_status_events(
    mut events: MessageReader<ServerEvent>,
    mut hud: ResMut<HudState>,
    mut flags_q: Query<&mut StatusFlags, With<LocalPlayer>>,
) {
    for ev in events.read() {
        match ev {
            ServerEvent::FishingUpdate { progress, .. } => {
                // #1544：钓鱼中不可使用物品
                hud.fishing = *progress != 0;
                if let Ok(mut f) = flags_q.single_mut() {
                    f.fishing = *progress != 0;
                }
            }
            ServerEvent::TrapRockChanged { in_trap } => {
                // #1550：陷阱中不可走/跑
                hud.in_trap_rock = *in_trap;
                if let Ok(mut f) = flags_q.single_mut() {
                    f.in_trap_rock = *in_trap;
                }
            }
            ServerEvent::LocalPoisonChanged { paralysis } => {
                // #1616：麻痹/冰冻毒锁定输入
                hud.paralysis = *paralysis;
                if let Ok(mut f) = flags_q.single_mut() {
                    f.paralysis = *paralysis;
                }
            }
            ServerEvent::MountUpdated {
                object_id,
                mount_type,
                is_mounted,
            } => {
                // #1544：本地玩家骑乘状态。MountState 组件由 object_state/spawn 路径维护，
                // 此处仅写 HudState 镜像（riding/mount_type），读者迁移（批6/§12 R7）后改用 MountState。
                if Some(*object_id) == hud.player_object_id {
                    hud.riding = *is_mounted;
                    // #1564：记录坐骑类型（骑乘音效 Tiger/Wolf 区分）
                    hud.mount_type = *mount_type;
                }
            }
            ServerEvent::PlayerDied => {
                hud.dead = true;
                hud.death_popup_dismissed = false;
                if let Ok(mut f) = flags_q.single_mut() {
                    f.dead = true;
                }
            }
            ServerEvent::ReincarnationRequested => {
                if hud.dead {
                    hud.reincarnation_offered = true;
                    if let Ok(mut f) = flags_q.single_mut() {
                        f.reincarnation_offered = true;
                    }
                }
            }
            ServerEvent::PlayerRevived => {
                hud.dead = false;
                hud.reincarnation_offered = false;
                hud.death_popup_dismissed = false;
                if let Ok(mut f) = flags_q.single_mut() {
                    f.dead = false;
                    f.reincarnation_offered = false;
                }
            }
            _ => {}
        }
    }
}

/// R1 reconcile（#2633 §12 R1）：LocalPlayer 实体生成后，把缓冲的 UserInformation 快照
/// 一次性应用到全部玩家组件（Vitals/Progression/Gold/CombatStats + Inventory/Loadout），
/// 然后清空 pending。
///
/// 入 GameSet::PlayerState 并排在事件写系统与 Hud 集之前：实体在 ObjectPlayer 帧尾生成后，
/// 下一帧本系统先应用缓冲、事件写系统再写本帧新值、Hud 集 sync_hud_data 最后读，无可见默认帧。
/// 实体尚未生成则保留 pending 留待下帧。不重注入事件（避免重复触发 log/其他 ServerEvent
/// 读者副作用）；组件写入与事件处理器共用 apply_user_info_stats/items 同一份映射。
#[allow(clippy::too_many_arguments)]
fn apply_pending_user_info(
    mut pending: ResMut<PendingUserInfo>,
    mut q: Query<
        (
            &mut Vitals,
            &mut Progression,
            &mut Gold,
            &mut CombatStats,
            &mut Inventory,
            &mut Loadout,
        ),
        With<LocalPlayer>,
    >,
    mut name_q: Query<&mut PlayerName, With<LocalPlayer>>,
) {
    let Some(ev) = pending.0.take() else {
        return;
    };
    // 全部组件同挂 LocalPlayer 实体（LocalPlayerStateBundle），单一元组查询成败即实体有无。
    if let Ok((mut v, mut p, mut g, mut cb, mut inv, mut lo)) = q.single_mut() {
        apply_user_info_stats(&ev, &mut v, &mut p, &mut g, &mut cb);
        apply_user_info_items(&ev, &mut inv, &mut lo);
        // #2633 批次4 步7：缓冲快照的 name 一并应用（PlayerName 由 spawn 插入，此时必在）
        if let ServerEvent::UserInformation { name, .. } = &ev {
            if let Ok(mut n) = name_q.single_mut() {
                n.0 = name.clone();
            }
        }
    } else {
        // 实体仍未生成（ObjectPlayer 尚未到达/未生效）：放回 pending，下一帧再试。
        pending.0 = Some(ev);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::dialogs::inventory::InvItem;
    use crate::game::dialogs::potion_belt::PotionBeltState;

    /// 注册 4 个写系统（与生产一致的 GameSet::PlayerState + belt 先于 inventory 排序 +
    /// in_state(Game) 门控），配齐所需资源。首个 update 在非 Game 态跑（schedule 初始化
    /// 期 B0001 检查），再切 Game 触发系统体。
    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
        app.init_state::<AppState>();
        app.add_message::<ServerEvent>();
        app.init_resource::<HudState>();
        app.init_resource::<PotionBeltState>();
        app.init_resource::<PendingUserInfo>();
        app.configure_sets(Update, GameSet::PlayerState.before(GameSet::Hud));
        app.add_systems(
            Update,
            (
                apply_pending_user_info
                    .before(player_vitals_events)
                    .before(crate::game::dialogs::inventory::inventory_events),
                player_vitals_events,
                player_status_events,
                crate::game::dialogs::potion_belt::belt_restock_events
                    .before(crate::game::dialogs::inventory::inventory_events),
                crate::game::dialogs::inventory::inventory_events,
            )
                .in_set(GameSet::PlayerState)
                .run_if(in_state(AppState::Game)),
        );
        app
    }

    fn enter_game(app: &mut App) {
        app.update(); // 非 Game 态一帧（B0001 初始化检查）
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Game);
        app.update(); // 切 Game
    }

    fn spawn_local(app: &mut App) -> Entity {
        app.world_mut()
            .spawn((LocalPlayer, LocalPlayerStateBundle::default()))
            .id()
    }

    fn get<T: Component>(app: &mut App) -> T
    where
        T: Clone,
    {
        app.world_mut()
            .query_filtered::<&T, With<LocalPlayer>>()
            .iter(app.world())
            .next()
            .expect("LocalPlayer 应有该组件")
            .clone()
    }

    fn item(uid: u64, item_index: i32, count: u16) -> InvItem {
        InvItem {
            unique_id: uid,
            item_index,
            count,
            ..Default::default()
        }
    }

    /// vitals/status 写路径 + HudState 双写等价（HealthChanged/LevelChanged/GoldGained/PlayerDied）
    #[test]
    fn vitals_status_write_components_and_hud() {
        let mut app = test_app();
        spawn_local(&mut app);
        enter_game(&mut app);

        app.world_mut()
            .write_message(ServerEvent::HealthChanged { hp: 500, mp: 200 });
        app.world_mut().write_message(ServerEvent::LevelChanged {
            level: 30,
            exp: 12345,
            max_exp: 99999,
        });
        app.world_mut()
            .write_message(ServerEvent::GoldGained { gold: 777 });
        app.world_mut().write_message(ServerEvent::PlayerDied);
        app.update();

        let hud = app.world().resource::<HudState>();
        assert_eq!((hud.hp, hud.mp), (500, 200));
        assert_eq!((hud.level, hud.exp, hud.max_exp), (30, 12345, 99999));
        assert_eq!(hud.gold, 777);
        assert!(hud.dead);

        let v: Vitals = get(&mut app);
        assert_eq!((v.hp, v.mp), (500, 200), "Vitals 组件应同步");
        let p: Progression = get(&mut app);
        assert_eq!((p.level, p.exp, p.max_exp), (30, 12345, 99999));
        let g: Gold = get(&mut app);
        assert_eq!(g.0, 777);
        let f: StatusFlags = get(&mut app);
        assert!(f.dead);
    }

    /// UserInformation：玩家属性/面板属性写组件 + HudState 双写（步8 起 CharacterState 已删）
    #[test]
    fn user_information_writes_vitals_progression_gold_combatstats() {
        let mut app = test_app();
        spawn_local(&mut app);
        enter_game(&mut app);

        app.world_mut().write_message(user_info(60, 800, 400));
        app.update();

        let hud = app.world().resource::<HudState>();
        assert_eq!((hud.hp, hud.max_hp, hud.mp, hud.max_mp), (800, 5000, 400, 2000));
        assert_eq!(hud.level, 60);
        assert_eq!(hud.gold, 4242);
        assert_eq!(hud.player_object_id, Some(31415));

        let v: Vitals = get(&mut app);
        assert_eq!((v.hp, v.max_hp, v.mp, v.max_mp), (800, 5000, 400, 2000));
        let p: Progression = get(&mut app);
        assert_eq!(p.level, 60);
        let g: Gold = get(&mut app);
        assert_eq!(g.0, 4242);
        let cb: CombatStats = get(&mut app);
        assert_eq!(cb.critical_rate, 17);
        assert_eq!(cb.bag_weight, 250);
    }

    /// 背包/装备写组件镜像 + HudState（ItemGained + UserInformation 背包部分）
    #[test]
    fn inventory_events_mirror_to_components() {
        let mut app = test_app();
        spawn_local(&mut app);
        enter_game(&mut app);

        // UserInformation 写入背包/装备
        app.world_mut().write_message(user_info(10, 100, 50));
        app.update();
        let hud = app.world().resource::<HudState>();
        assert_eq!(hud.inventory.items.len(), 4);
        assert_eq!(hud.equipment[0].as_ref().unwrap().unique_id, 900);
        let inv: crate::game::player_state::Inventory = get(&mut app);
        assert_eq!(inv.items.len(), 4, "Inventory 组件应镜像背包");
        assert_eq!(inv.items[0].as_ref().unwrap().unique_id, 1);
        let loadout: crate::game::player_state::Loadout = get(&mut app);
        assert_eq!(loadout.slots[0].as_ref().unwrap().unique_id, 900);

        // ItemGained 增量（进第一个空格）
        app.world_mut().write_message(ServerEvent::ItemGained {
            item: item(7, 70, 1),
        });
        app.update();
        let inv: crate::game::player_state::Inventory = get(&mut app);
        assert!(
            inv.items
                .iter()
                .flatten()
                .any(|it| it.unique_id == 7),
            "新物品应入包并镜像到组件"
        );
        let hud = app.world().resource::<HudState>();
        assert!(hud.inventory.items.iter().flatten().any(|it| it.unique_id == 7));
    }

    /// §9：gold 唯一源是 Gold 组件——UserInformation 不再写 hud.inventory.gold（双源已消）。
    #[test]
    fn gold_single_source_inventory_gold_not_written() {
        let mut app = test_app();
        spawn_local(&mut app);
        enter_game(&mut app);

        app.world_mut().write_message(user_info(10, 100, 50));
        app.update();

        let g: Gold = get(&mut app);
        assert_eq!(g.0, 4242, "Gold 组件为金币唯一源");
        let hud = app.world().resource::<HudState>();
        assert_eq!(hud.gold, 4242, "hud.gold 仍双写（步9 删）");
        assert_eq!(
            hud.inventory.gold, 0,
            "hud.inventory.gold 不再写入（§9 双源已消，背包金币文本读 Gold 组件）"
        );
    }

    /// 背包 CRUD（移动/删除）：组件与 hud 双写逐步一致（unique_id 序列比对）。
    #[test]
    fn inventory_move_delete_mirror_to_component() {
        fn ids(items: &[Option<InvItem>]) -> Vec<Option<u64>> {
            items.iter().map(|s| s.as_ref().map(|i| i.unique_id)).collect()
        }
        let mut app = test_app();
        spawn_local(&mut app);
        enter_game(&mut app);

        app.world_mut().write_message(user_info(10, 100, 50));
        app.update();

        // 移动 0 <-> 1（user_info 背包 [uid1, uid2, None, None]）
        app.world_mut()
            .write_message(ServerEvent::InventoryMoved { from: 0, to: 1 });
        app.update();
        let hud_ids = ids(&app.world().resource::<HudState>().inventory.items);
        let inv: Inventory = get(&mut app);
        assert_eq!(hud_ids, ids(&inv.items), "移动后 hud 与 Inventory 组件一致");

        // 删除当前 0 格物品（uid=2，移动后位于 0）
        let uid = inv.items[0].as_ref().unwrap().unique_id;
        app.world_mut()
            .write_message(ServerEvent::ItemDeleted { unique_id: uid });
        app.update();
        let hud_ids = ids(&app.world().resource::<HudState>().inventory.items);
        let inv: Inventory = get(&mut app);
        assert_eq!(hud_ids, ids(&inv.items), "删除后 hud 与 Inventory 组件一致");
        assert!(!inv.items.iter().flatten().any(|it| it.unique_id == uid));
    }

    /// ItemUsed：腰带补货须读「扣减前」背包（belt_restock 先于 inventory 扣减，§12 R6）
    #[test]
    fn belt_restock_reads_pre_deduct_inventory() {
        let mut app = test_app();
        spawn_local(&mut app);
        enter_game(&mut app);

        // 背包：stack A(uid=100,idx=500,count=1) + stack B(uid=200,idx=500,count=3)；腰带格 0 = uid 100
        {
            let mut hud = app.world_mut().resource_mut::<HudState>();
            hud.inventory.items = vec![Some(item(100, 500, 1)), Some(item(200, 500, 3))];
            let mut belt = app.world_mut().resource_mut::<PotionBeltState>();
            belt.slots[0] = Some(100);
        }
        app.world_mut()
            .write_message(ServerEvent::ItemUsed { unique_id: 100 });
        app.update();

        // 补货：找到同 item_index 的 stack B(uid=200)；若 belt 晚于扣减则读不到 used_item_index→不补
        let belt = app.world().resource::<PotionBeltState>();
        assert_eq!(belt.slots[0], Some(200), "腰带应补货为另一组同物品");
        // 扣减：stack A(count=1) 被移除
        let hud = app.world().resource::<HudState>();
        assert!(
            !hud.inventory.items.iter().flatten().any(|it| it.unique_id == 100),
            "已消耗物品应出包"
        );
        assert!(hud.inventory.items.iter().flatten().any(|it| it.unique_id == 200));
    }

    /// R1：实体未生成时组件写跳过、不 panic，HudState 仍写（读者零影响）
    #[test]
    fn write_skips_when_entity_missing() {
        let mut app = test_app();
        // 不 spawn LocalPlayer
        enter_game(&mut app);
        app.world_mut()
            .write_message(ServerEvent::HealthChanged { hp: 42, mp: 24 });
        app.update(); // 不应 panic
        let hud = app.world().resource::<HudState>();
        assert_eq!((hud.hp, hud.mp), (42, 24), "HudState 兜底写仍生效");
        // 无实体 → 查询不到组件（无法断言值，只要不 panic 即通过）
    }

    /// R1 修复：实体缺失时 UserInformation 被缓冲（不 panic、HudState 照写、组件查不到），
    /// 实体生成后 reconcile 一次性应用全部组件并清空 pending。
    #[test]
    fn pending_user_info_buffered_then_applied_on_spawn() {
        let mut app = test_app();
        // 不 spawn LocalPlayer
        enter_game(&mut app);

        // 实体缺失时写 UserInformation
        app.world_mut().write_message(user_info(60, 800, 400));
        app.update(); // 不应 panic

        // HudState/CharacterState 双写照常（读者零影响）
        let hud = app.world().resource::<HudState>();
        assert_eq!((hud.hp, hud.max_hp, hud.level, hud.gold), (800, 5000, 60, 4242));
        assert_eq!(hud.inventory.items.len(), 4);
        // 组件仍查不到（实体未生成）
        assert!(
            app.world_mut()
                .query_filtered::<&Vitals, With<LocalPlayer>>()
                .iter(app.world())
                .next()
                .is_none(),
            "实体未生成时不应有 Vitals 组件"
        );
        // 快照已缓冲
        assert!(app.world().resource::<PendingUserInfo>().0.is_some());

        // 生成 LocalPlayer（挂默认组件）→ reconcile 应应用缓冲快照
        spawn_local(&mut app);
        app.update();

        let v: Vitals = get(&mut app);
        assert_eq!((v.hp, v.max_hp, v.mp, v.max_mp), (800, 5000, 400, 2000));
        let p: Progression = get(&mut app);
        assert_eq!((p.level, p.exp, p.max_exp), (60, 500, 1000));
        let g: Gold = get(&mut app);
        assert_eq!(g.0, 4242);
        let cb: CombatStats = get(&mut app);
        assert_eq!(cb.critical_rate, 17);
        assert_eq!(cb.bag_weight, 250);
        assert_eq!(cb.stats, [[1, 2], [3, 4], [5, 6], [7, 8], [9, 10]]);
        let inv: crate::game::player_state::Inventory = get(&mut app);
        assert_eq!(inv.items.len(), 4);
        assert_eq!(inv.items[0].as_ref().unwrap().unique_id, 1);
        assert_eq!(inv.max_weight, 250);
        let loadout: crate::game::player_state::Loadout = get(&mut app);
        assert_eq!(loadout.slots[0].as_ref().unwrap().unique_id, 900);
        // pending 已清空
        assert!(app.world().resource::<PendingUserInfo>().0.is_none());
        // HudState 仍正确
        let hud = app.world().resource::<HudState>();
        assert_eq!((hud.hp, hud.level, hud.gold), (800, 60, 4242));
    }

    /// R1：实体缺失时连发两个 UserInformation → 后到覆盖先到（latest wins）
    #[test]
    fn pending_user_info_latest_wins() {
        let mut app = test_app();
        // 不 spawn LocalPlayer
        enter_game(&mut app);

        app.world_mut().write_message(user_info(60, 800, 400));
        app.world_mut().write_message(user_info(70, 999, 450));
        app.update();

        // 生成实体 → reconcile 应应用**后到**的快照（level=70/hp=999）
        spawn_local(&mut app);
        app.update();

        let p: Progression = get(&mut app);
        assert_eq!(p.level, 70, "后者覆盖前者（latest wins）");
        let v: Vitals = get(&mut app);
        assert_eq!(v.hp, 999);
        assert!(app.world().resource::<PendingUserInfo>().0.is_none());
    }

    /// 步7：PlayerNameUpdated / UserInformation 双写——hud.name 与 `PlayerName` 组件同步
    /// （读者基于组件，写者须保证镜像成立；PlayerName 由 spawn 路径插入，测试须显式挂载）。
    #[test]
    fn player_name_component_dual_written() {
        let mut app = test_app();
        app.world_mut().spawn((
            LocalPlayer,
            LocalPlayerStateBundle::default(),
            PlayerName(String::new()),
        ));
        enter_game(&mut app);

        app.world_mut()
            .write_message(ServerEvent::PlayerNameUpdated {
                name: "改名后".to_string(),
            });
        app.update();
        let hud = app.world().resource::<HudState>();
        assert_eq!(hud.name, "改名后", "hud.name 双写保留");
        let pn = app
            .world_mut()
            .query_filtered::<&PlayerName, With<LocalPlayer>>()
            .iter(app.world())
            .next()
            .expect("应有 PlayerName")
            .0
            .clone();
        assert_eq!(pn, "改名后", "PlayerNameUpdated 应同步 PlayerName 组件");

        app.world_mut().write_message(user_info(10, 100, 50));
        app.update();
        let hud = app.world().resource::<HudState>();
        assert_eq!(hud.name, "测试者");
        let pn = app
            .world_mut()
            .query_filtered::<&PlayerName, With<LocalPlayer>>()
            .iter(app.world())
            .next()
            .expect("应有 PlayerName")
            .0
            .clone();
        assert_eq!(pn, "测试者", "UserInformation 应同步 PlayerName 组件");
    }

    /// 步7：实体缺失时 PlayerName 写跳过不 panic（同 R1 语义），hud.name 照写
    #[test]
    fn player_name_write_skips_when_entity_missing() {
        let mut app = test_app();
        // 不 spawn LocalPlayer
        enter_game(&mut app);
        app.world_mut()
            .write_message(ServerEvent::PlayerNameUpdated {
                name: "无实体".to_string(),
            });
        app.update(); // 不应 panic
        let hud = app.world().resource::<HudState>();
        assert_eq!(hud.name, "无实体");
    }

    /// 构造 UserInformation 事件（背包 2 格、装备槽 0 有 uid=900）
    fn user_info(level: u16, hp: i32, mp: i32) -> ServerEvent {
        ServerEvent::UserInformation {
            name: "测试者".to_string(),
            level,
            hp,
            mp,
            exp: 500,
            max_exp: 1000,
            gold: 4242,
            class: 1,
            gender: 0,
            object_id: 31415,
            magics: Vec::new(),
            inventory: vec![Some(item(1, 10, 1)), Some(item(2, 20, 5)), None, None],
            equipment: {
                let mut e = vec![None; 14];
                e[0] = Some(item(900, 900, 1));
                e
            },
            quest_inventory: Vec::new(),
            item_names: Vec::new(),
            max_hp: 5000,
            max_mp: 2000,
            ac: [1, 2],
            mac: [3, 4],
            dc: [5, 6],
            mc: [7, 8],
            sc: [9, 10],
            critical_rate: 17,
            critical_damage: 18,
            attack_speed: 19,
            accuracy: 20,
            agility: 21,
            luck: 22,
            bag_weight: 250,
            wear_weight: 260,
            hand_weight: 270,
            magic_resist: 23,
            poison_resist: 24,
            health_recovery: 25,
            spell_recovery: 26,
            poison_recovery: 27,
            holy: 28,
            freezing: 29,
            poison_atk: 30,
        }
    }
}

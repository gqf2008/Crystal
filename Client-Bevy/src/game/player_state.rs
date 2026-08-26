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
    /// 逻辑源自原 `InventoryState::resize`（已删）；本批写路径用全量镜像（见 inventory_events），
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
    /// 逻辑源自原 `InventoryState::refresh_weight`（已删）；用途同 `resize`。
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

/// 登录首帧本地玩家事件缓冲（#2633 §12 R1 + 评审 M1）。
///
/// R1 已证实：UserInformation（属性）先于本地 ObjectPlayer 到达，且 spawn_local_player_with
/// 走 Commands 延迟到帧尾才生成实体。缓冲只装 UserInformation（latest wins）时，同一窗口内
/// HealthChanged 等事件被静默丢弃——回放快照后会用旧值覆盖更新的值（评审 M1）。
/// 故推广为**有序 Vec**：实体缺失期间全部可处理事件按到达顺序入队，实体生成后由
/// `apply_pending_events` 按序回放（旧→新，新值胜）。窗口仅登录数帧、且 is_* 过滤只入队
/// 已处理事件，无膨胀风险。
#[derive(Resource, Default)]
pub struct PendingPlayerEvents(pub Vec<ServerEvent>);

// ============================================================================
// ServerEvent 写系统（#2633 批次4 步2：拆 hud_server_events，设计 §10）
//
// 双写过渡（设计 §11 批1）：CS 双源已删（CharacterState 已于步8 删除、HudState 已于步9 删除）
// ——唯一数据源是玩家实体组件。
// 组件写用 `Query<&mut X, With<LocalPlayer>>` + `single_mut()`；实体未生成时
// （UserInformation 可能先于 ObjectPlayer 到达，设计 §12 R1）跳过组件写，
// 可处理事件另由 `PendingPlayerEvents` 缓冲待实体生成后按序回放（评审 M1）。
// ============================================================================

/// 从 UserInformation 事件把玩家属性/面板属性写入 Vitals/Progression/Gold/CombatStats。
///
/// `player_vitals_events` 与 `apply_pending_events` 共用此一份字段映射，避免双份漂移
/// （#2633 R1）。非 UserInformation 事件为 no-op。只写组件，不写 HudState
/// （HudState 已于步9 删除）。
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
/// `inventory_events` 与 `apply_pending_events` 共用此一份字段映射，避免双份漂移
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

/// 单一写映射：player_vitals_events / apply_pending_events 共用（#2633 R1，防双份漂移）。
/// 命中任一已处理事件返回 true（供实体缺失时判别是否入队）。
/// `name`：PlayerName 由 spawn 路径插入（非 Bundle 成员），不参与"实体有无"判定，
/// 缺失（None）时静默跳过——保持既有 R1 语义。
#[allow(clippy::too_many_arguments)]
fn apply_vitals_event(
    ev: &ServerEvent,
    vitals: &mut Vitals,
    progression: &mut Progression,
    gold: &mut Gold,
    credit: &mut Credit,
    base_stats: &mut BaseStats,
    pet_mode: &mut PetModeState,
    combat_stats: &mut CombatStats,
    name: Option<&mut PlayerName>,
) -> bool {
    match ev {
        ServerEvent::PetModeChanged { mode } => {
            // #1388：HUD 宠物模式标签
            pet_mode.0 = *mode;
            true
        }
        ServerEvent::HealthChanged { hp, mp } => {
            vitals.hp = *hp;
            vitals.mp = *mp;
            true
        }
        ServerEvent::GoldGained { gold: gained } => {
            gold.0 = gold.0.saturating_add(*gained);
            true
        }
        ServerEvent::BaseStats { stats } => {
            // #268：基础属性（角色面板数据）
            base_stats.0 = stats.clone();
            tracing::info!("📊 基础属性: {:?}", stats);
            true
        }
        ServerEvent::PlayerNameUpdated { name: new_name } => {
            // #264：本地玩家改名（复用组件 `PlayerName`；object_state 亦有同名维护，值同）
            if let Some(n) = name {
                n.0 = new_name.clone();
            }
            tracing::info!("🏷️ 玩家改名 -> {}", new_name);
            true
        }
        ServerEvent::CreditGained { credit: gained } => {
            // #248：声望增加
            credit.0 = credit.0.saturating_add(*gained);
            tracing::info!("🏅 获得声望 +{}（当前 {}）", gained, credit.0);
            true
        }
        ServerEvent::CreditLost { amount } => {
            // #248：声望减少
            credit.0 = credit.0.saturating_sub(*amount);
            tracing::info!("🏅 失去声望 -{}（当前 {}）", amount, credit.0);
            true
        }
        ServerEvent::GoldLost { amount } => {
            gold.0 = gold.0.saturating_sub(*amount);
            tracing::info!("💸 失去金币 -{}（当前 {}）", amount, gold.0);
            true
        }
        ServerEvent::ExperienceGained { amount } => {
            progression.exp += *amount;
            tracing::info!("✨ 获得经验 +{}（当前 {}/{}）", amount, progression.exp, progression.max_exp);
            true
        }
        ServerEvent::LevelChanged {
            level,
            exp,
            max_exp,
        } => {
            progression.level = *level;
            progression.exp = *exp;
            progression.max_exp = (*max_exp).max(1);
            tracing::info!("⬆️ 升级 Lv.{} exp={}/{}", level, exp, max_exp);
            true
        }
        ServerEvent::UserInformation { name: user_name, .. } => {
            // —— 玩家组件：实体已生成就地写入（共享映射），未生成则由调用方缓冲待回放（R1/M1）——
            // PlayerName 单独写：它由 spawn 路径插入（非 Bundle 成员），不参与"实体有无"判定，
            // 缺失时静默跳过（同其他写系统 R1 语义）。
            apply_user_info_stats(ev, vitals, progression, gold, combat_stats);
            // #2633 批次4 步7：补写复用组件 `PlayerName`
            if let Some(n) = name {
                n.0 = user_name.clone();
            }
            true
        }
        _ => false,
    }
}

/// 判别 `apply_vitals_event` 的命中集——两侧新增分支须同步（评审 M1；分支测试兜底）。
fn is_vitals_event(ev: &ServerEvent) -> bool {
    matches!(ev,
        ServerEvent::PetModeChanged { .. } | ServerEvent::HealthChanged { .. }
        | ServerEvent::GoldGained { .. } | ServerEvent::BaseStats { .. }
        | ServerEvent::PlayerNameUpdated { .. } | ServerEvent::CreditGained { .. }
        | ServerEvent::CreditLost { .. } | ServerEvent::GoldLost { .. }
        | ServerEvent::ExperienceGained { .. } | ServerEvent::LevelChanged { .. }
        | ServerEvent::UserInformation { .. })
}

pub struct PlayerStatePlugin;

impl Plugin for PlayerStatePlugin {
    fn build(&self, app: &mut App) {
        // 写方须排在读方前（设计 §12 R5）：先写玩家组件/HudState，后 sync_hud_data（Hud 集）读。
        app.configure_sets(Update, GameSet::PlayerState.before(GameSet::Hud));
        app.init_resource::<PendingPlayerEvents>();
        // R1/M1 reconcile：须在事件写系统之前运行——缓冲事件先回放，本帧新到事件后写（新值胜）。
        app.add_systems(
            Update,
            apply_pending_events
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
/// #2633 批次4 步8/9：CharacterState 双源已删、HudState 双写已删——唯一数据源是玩家实体组件。
#[allow(clippy::too_many_arguments)]
fn player_vitals_events(
    mut warned_multi: Local<bool>,
    mut events: MessageReader<ServerEvent>,
    mut pending: ResMut<PendingPlayerEvents>,
    mut q: Query<
        (
            &mut Vitals, &mut Progression, &mut Gold, &mut Credit,
            &mut BaseStats, &mut PetModeState, &mut CombatStats,
        ),
        With<LocalPlayer>,
    >,
    mut name_q: Query<&mut PlayerName, With<LocalPlayer>>,
) {
    // 全部组件同挂 LocalPlayer 实体（LocalPlayerStateBundle），单一元组查询成败即实体有无。
    let Ok((mut v, mut p, mut g, mut c, mut b, mut pm, mut cb)) = q.single_mut() else {
        // R1/M1：实体未生成——本窗口全部可处理事件按到达顺序缓冲，待 apply_pending_events 回放
        //（修复评审 M1：此前只缓冲 UserInformation、其余被丢弃）。正常窗口仅登录数帧。
        // 另：若实际是"多个 LocalPlayer 实体"（异常，理论已由登出清理修复消除），给一次告警便于定位。
        if !*warned_multi
            && matches!(q.single_mut(), Err(bevy_ecs::query::QuerySingleError::MultipleEntities(_)))
        {
            *warned_multi = true;
            tracing::warn!("⚠️ 多个 LocalPlayer 实体（登出未清理？CRITICAL-1）——本帧跳过组件写，事件按序缓冲");
        }
        for ev in events.read() {
            if is_vitals_event(ev) {
                pending.0.push(ev.clone());
            }
        }
        return;
    };
    let mut n = name_q.single_mut().ok();
    for ev in events.read() {
        apply_vitals_event(ev, &mut v, &mut p, &mut g, &mut c, &mut b, &mut pm, &mut cb, n.as_deref_mut());
    }
}

/// 单一写映射：player_status_events / apply_pending_events 共用（#2633 R1，防双份漂移）。
/// 命中任一已处理事件返回 true（供实体缺失时判别是否入队）。
/// 行为差异：实体缺失时 `death_ui.dismissed = false` 从"立即执行"变为"回放时执行"——
/// PlayerDied 不可能落在实体缺失窗口（必在游戏中），且回放至迟一帧，可接受。
fn apply_status_event(
    ev: &ServerEvent,
    flags: &mut StatusFlags,
    death_ui: &mut crate::game::hud::DeathDialogState,
) -> bool {
    match ev {
        ServerEvent::FishingUpdate { progress, .. } => {
            // #1544：钓鱼中不可使用物品
            flags.fishing = *progress != 0;
            true
        }
        ServerEvent::TrapRockChanged { in_trap } => {
            // #1550：陷阱中不可走/跑
            flags.in_trap_rock = *in_trap;
            true
        }
        ServerEvent::LocalPoisonChanged { paralysis } => {
            // #1616：麻痹/冰冻毒锁定输入
            flags.paralysis = *paralysis;
            true
        }
        ServerEvent::MountUpdated { .. } => {
            // #1544：本地玩家骑乘。MountState 组件由 object_state/spawn 路径维护
            //（object_state.rs MountUpdated 分支插入/移除），HudState 镜像随资源删除。
            // no-op：不缓冲（is_status_event 亦不含此变体）。
            false
        }
        ServerEvent::PlayerDied => {
            flags.dead = true;
            // 死亡弹窗重新弹出（C# ShowReviveMessage 只弹一次，#46）
            death_ui.dismissed = false;
            true
        }
        ServerEvent::ReincarnationRequested => {
            // 死门控（评审 MAJOR-2）：未死亡不发轮回请求
            if flags.dead {
                flags.reincarnation_offered = true;
            }
            true
        }
        ServerEvent::PlayerRevived => {
            flags.dead = false;
            flags.reincarnation_offered = false;
            death_ui.dismissed = false;
            true
        }
        _ => false,
    }
}

/// 判别 `apply_status_event` 的命中集（MountUpdated 为 no-op 不含在内；
/// 两侧新增分支须同步（评审 M1；分支测试兜底））。
fn is_status_event(ev: &ServerEvent) -> bool {
    matches!(ev,
        ServerEvent::FishingUpdate { .. } | ServerEvent::TrapRockChanged { .. }
        | ServerEvent::LocalPoisonChanged { .. } | ServerEvent::PlayerDied
        | ServerEvent::ReincarnationRequested | ServerEvent::PlayerRevived)
}

/// 玩家状态旗标（钓鱼/陷阱/麻痹/死亡/复活/轮回）。
/// （设计 §10 `player_status_events`；sprint/sneaking 由 buff.rs 写；
/// MountUpdated 的 MountState 由 object_state/spawn 维护——步9 起本系统不再为该事件写任何值。）
fn player_status_events(
    mut events: MessageReader<ServerEvent>,
    mut pending: ResMut<PendingPlayerEvents>,
    mut flags_q: Query<&mut StatusFlags, With<LocalPlayer>>,
    mut death_ui: ResMut<crate::game::hud::DeathDialogState>,
) {
    // R1/M1 同构：实体未生成（登录首帧）时全部可处理事件按到达顺序缓冲，待回放。
    let Ok(mut f) = flags_q.single_mut() else {
        for ev in events.read() {
            if is_status_event(ev) {
                pending.0.push(ev.clone());
            }
        }
        return;
    };
    for ev in events.read() {
        apply_status_event(ev, &mut f, &mut death_ui);
    }
}

/// R1/M1 reconcile（#2633 §12 R1）：LocalPlayer 实体生成后，把缓冲的本地玩家事件
/// **按到达顺序**回放——旧→新，新值胜（修复评审 M1：此前只缓冲 UserInformation 的
/// latest-wins 快照，窗口内 HealthChanged 等被丢弃，回放后旧快照覆盖新值）。
/// 组件写入与事件处理器共用 apply_vitals_event / apply_status_event /
/// apply_user_info_items（对非 UserInformation 为 no-op，可安全重复调用）同一份映射；
/// 回放会连带执行函数内日志（仅登录帧发生、至多重复一次，可接受）。
/// 实体未生成则不 take、留待下帧。不重注入事件（避免触发 ServerEvent 其他读者副作用）。
#[allow(clippy::too_many_arguments)]
fn apply_pending_events(
    mut pending: ResMut<PendingPlayerEvents>,
    mut q: Query<
        (
            &mut Vitals, &mut Progression, &mut Gold, &mut Credit, &mut BaseStats,
            &mut PetModeState, &mut CombatStats, &mut Inventory, &mut Loadout,
            &mut StatusFlags,
        ),
        With<LocalPlayer>,
    >,
    mut name_q: Query<&mut PlayerName, With<LocalPlayer>>,
    mut death_ui: ResMut<crate::game::hud::DeathDialogState>,
) {
    if pending.0.is_empty() {
        return;
    }
    let Ok((mut v, mut p, mut g, mut c, mut b, mut pm, mut cb, mut inv, mut lo, mut f)) = q.single_mut()
    else {
        return; // 实体仍未生成：保留 pending 下一帧再试
    };
    let mut n = name_q.single_mut().ok();
    let buffered = std::mem::take(&mut pending.0);
    for ev in &buffered {
        apply_vitals_event(ev, &mut v, &mut p, &mut g, &mut c, &mut b, &mut pm, &mut cb, n.as_deref_mut());
        apply_user_info_items(ev, &mut inv, &mut lo);
        apply_status_event(ev, &mut f, &mut death_ui);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::dialogs::inventory::InvItem;
    use crate::game::dialogs::potion_belt::PotionBeltState;
    use crate::game::hud::DeathDialogState;

    /// 注册 4 个写系统（与生产一致的 GameSet::PlayerState + belt 先于 inventory 排序 +
    /// in_state(Game) 门控），配齐所需资源。首个 update 在非 Game 态跑（schedule 初始化
    /// 期 B0001 检查），再切 Game 触发系统体。
    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
        app.init_state::<AppState>();
        app.add_message::<ServerEvent>();
        app.init_resource::<DeathDialogState>();
        app.init_resource::<PotionBeltState>();
        app.init_resource::<PendingPlayerEvents>();
        app.configure_sets(Update, GameSet::PlayerState.before(GameSet::Hud));
        // CRITICAL-1：登出回登录界面须清 LocalPlayer 实体（防"换角色重登出现双实体"，见 actor/mod.rs）
        app.add_systems(OnExit(AppState::Game), crate::actor::despawn_local_player);
        app.add_systems(
            Update,
            (
                apply_pending_events
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

    /// vitals/status 写路径（HealthChanged/LevelChanged/GoldGained/PlayerDied → 组件）
    #[test]
    fn vitals_status_write_components() {
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

        let v: Vitals = get(&mut app);
        assert_eq!((v.hp, v.mp), (500, 200), "Vitals 组件应更新");
        let p: Progression = get(&mut app);
        assert_eq!((p.level, p.exp, p.max_exp), (30, 12345, 99999));
        let g: Gold = get(&mut app);
        assert_eq!(g.0, 777);
        let f: StatusFlags = get(&mut app);
        assert!(f.dead);
        // 死亡弹窗应重置（PlayerDied → DeathDialogState.dismissed=false）
        assert!(!app.world().resource::<DeathDialogState>().dismissed);
    }

    /// UserInformation：玩家属性/面板属性写组件（步8 起 CharacterState 已删、步9 起 HudState 已删）
    #[test]
    fn user_information_writes_vitals_progression_gold_combatstats() {
        let mut app = test_app();
        spawn_local(&mut app);
        enter_game(&mut app);

        app.world_mut().write_message(user_info(60, 800, 400));
        app.update();

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

    /// 背包/装备写组件（ItemGained + UserInformation 背包部分）
    #[test]
    fn inventory_events_mirror_to_components() {
        let mut app = test_app();
        spawn_local(&mut app);
        enter_game(&mut app);

        // UserInformation 写入背包/装备
        app.world_mut().write_message(user_info(10, 100, 50));
        app.update();
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
    }

    /// §9：gold 唯一源是 Gold 组件——背包金币文本读 Gold 组件，无第二数据源可断言。
    #[test]
    fn gold_single_source() {
        let mut app = test_app();
        spawn_local(&mut app);
        enter_game(&mut app);

        app.world_mut().write_message(user_info(10, 100, 50));
        app.update();

        let g: Gold = get(&mut app);
        assert_eq!(g.0, 4242, "Gold 组件为金币唯一源");
    }

    /// 背包 CRUD（移动/删除）：组件逐步更新（unique_id 序列断言）。
    #[test]
    fn inventory_move_delete_updates_component() {
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
        let inv: Inventory = get(&mut app);
        assert_eq!(ids(&inv.items), [Some(2), Some(1), None, None], "移动后应交换");

        // 删除当前 0 格物品（uid=2，移动后位于 0）
        let uid = inv.items[0].as_ref().unwrap().unique_id;
        app.world_mut()
            .write_message(ServerEvent::ItemDeleted { unique_id: uid });
        app.update();
        let inv: Inventory = get(&mut app);
        assert!(!inv.items.iter().flatten().any(|it| it.unique_id == uid));
    }

    /// ItemUsed：腰带补货须读「扣减前」背包（belt_restock 先于 inventory 扣减，§12 R6）
    /// #2633 批次4 步9：补货系统改 `Query::single()` 读 Inventory 组件——本地玩家实体只允许
    /// 一只（重复 spawn 会使 single() 判多实体而跳过），这里不再预 spawn。
    #[test]
    fn belt_restock_reads_pre_deduct_inventory() {
        let mut app = test_app();
        enter_game(&mut app);

        // 背包：stack A(uid=100,idx=500,count=1) + stack B(uid=200,idx=500,count=3)；腰带格 0 = uid 100
        let e = spawn_local(&mut app);
        app.world_mut().entity_mut(e).insert(Inventory {
            items: vec![Some(item(100, 500, 1)), Some(item(200, 500, 3))],
            ..Default::default()
        });
        let mut belt = app.world_mut().resource_mut::<PotionBeltState>();
        belt.slots[0] = Some(100);
        app.world_mut()
            .write_message(ServerEvent::ItemUsed { unique_id: 100 });
        app.update();

        // 补货：找到同 item_index 的 stack B(uid=200)；若 belt 晚于扣减则读不到 used_item_index→不补
        let belt = app.world().resource::<PotionBeltState>();
        assert_eq!(belt.slots[0], Some(200), "腰带应补货为另一组同物品");
        // 扣减：stack A(count=1) 被移除
        let inv: Inventory = get(&mut app);
        assert!(
            !inv.items.iter().flatten().any(|it| it.unique_id == 100),
            "已消耗物品应出包"
        );
        assert!(inv.items.iter().flatten().any(|it| it.unique_id == 200));
    }

    /// R1/M1：实体未生成时组件写改为缓冲入队（不 panic），等待实体生成后按序回放
    #[test]
    fn write_skips_when_entity_missing() {
        let mut app = test_app();
        // 不 spawn LocalPlayer
        enter_game(&mut app);
        app.world_mut()
            .write_message(ServerEvent::HealthChanged { hp: 42, mp: 24 });
        app.update(); // 不应 panic
    }

    /// R1 修复：实体缺失时 UserInformation 被缓冲（不 panic），
    /// 实体生成后 reconcile 一次性应用全部组件并清空 pending。
    #[test]
    fn pending_user_info_buffered_then_applied_on_spawn() {
        let mut app = test_app();
        // 不 spawn LocalPlayer
        enter_game(&mut app);

        // 实体缺失时写 UserInformation
        app.world_mut().write_message(user_info(60, 800, 400));
        app.update(); // 不应 panic

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
        assert!(!app.world().resource::<PendingPlayerEvents>().0.is_empty());

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
        assert!(app.world().resource::<PendingPlayerEvents>().0.is_empty());
    }

    /// M1（评审）：实体缺失时全部可处理事件按到达顺序缓冲，实体生成后按序回放——
    /// HealthChanged/GoldGained/PlayerNameUpdated 不再被丢弃（原实现只缓冲 UserInformation
    /// 的 latest-wins 快照，回放后旧值覆盖新值）。
    #[test]
    fn pending_events_replayed_in_order() {
        let mut app = test_app();
        // 不 spawn LocalPlayer
        enter_game(&mut app);

        app.world_mut().write_message(user_info(60, 800, 400));
        app.world_mut()
            .write_message(ServerEvent::HealthChanged { hp: 500, mp: 100 });
        app.world_mut().write_message(ServerEvent::GoldGained { gold: 100 });
        app.world_mut()
            .write_message(ServerEvent::PlayerNameUpdated { name: "先改名后".to_string() });
        app.update(); // 不应 panic

        // 4 个可处理事件已按到达顺序入队
        assert_eq!(app.world().resource::<PendingPlayerEvents>().0.len(), 4);

        // 生成 LocalPlayer（挂默认组件 + PlayerName，后者由 spawn 路径插入）→ 按序回放
        let e = spawn_local(&mut app);
        app.world_mut().entity_mut(e).insert(PlayerName(String::new()));
        app.update();

        let v: Vitals = get(&mut app);
        assert_eq!(
            (v.hp, v.mp),
            (500, 100),
            "HealthChanged 后到覆盖 UserInfo（M1：此前被丢弃→旧值 800 胜）"
        );
        let g: Gold = get(&mut app);
        assert_eq!(g.0, 4342, "GoldGained 在 UserInformation gold=4242 基础上 +100");
        let p: Progression = get(&mut app);
        assert_eq!(p.level, 60);
        let pn = app
            .world_mut()
            .query_filtered::<&PlayerName, With<LocalPlayer>>()
            .iter(app.world())
            .next()
            .expect("应有 PlayerName")
            .0
            .clone();
        assert_eq!(pn, "先改名后", "PlayerNameUpdated 应随缓冲回放");
        assert!(app.world().resource::<PendingPlayerEvents>().0.is_empty());
    }

    /// 步7/9：PlayerNameUpdated / UserInformation 写 `PlayerName` 组件
    /// （PlayerName 由 spawn 路径插入，测试须显式挂载）。
    #[test]
    fn player_name_component_written() {
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

    /// M1（评审）：实体缺失时 PlayerNameUpdated 入队缓冲（不再静默丢弃），
    /// 实体生成后回放应用到 `PlayerName` 组件。
    #[test]
    fn player_name_buffered_when_entity_missing() {
        let mut app = test_app();
        // 不 spawn LocalPlayer
        enter_game(&mut app);
        app.world_mut()
            .write_message(ServerEvent::PlayerNameUpdated {
                name: "无实体".to_string(),
            });
        app.update(); // 不应 panic
        assert!(!app.world().resource::<PendingPlayerEvents>().0.is_empty());

        let e = spawn_local(&mut app);
        app.world_mut().entity_mut(e).insert(PlayerName(String::new()));
        app.update();
        let pn = app
            .world_mut()
            .query_filtered::<&PlayerName, With<LocalPlayer>>()
            .iter(app.world())
            .next()
            .expect("应有 PlayerName")
            .0
            .clone();
        assert_eq!(pn, "无实体", "PlayerNameUpdated 应随缓冲回放");
    }

    /// M1（评审）：实体缺失时只缓冲可处理事件——不属于 vitals/status 命中集的变体不入队。
    #[test]
    fn pending_buffers_only_handled_events() {
        let mut app = test_app();
        // 不 spawn LocalPlayer
        enter_game(&mut app);

        app.world_mut().write_message(user_info(60, 800, 400));
        app.world_mut().write_message(ServerEvent::LogOutFailed);
        app.world_mut().write_message(ServerEvent::ReturnToLogin);
        app.update(); // 不应 panic

        assert_eq!(
            app.world().resource::<PendingPlayerEvents>().0.len(),
            1,
            "仅 UserInformation 入队（非 vitals/status 事件不缓冲）"
        );
    }

    /// MAJOR-2（评审）：ReincarnationRequested 须死门控——未死亡不发轮回弹窗。
    #[test]
    fn reincarnation_requested_requires_dead_gate() {
        let mut app = test_app();
        spawn_local(&mut app);
        enter_game(&mut app);

        app.world_mut().write_message(ServerEvent::ReincarnationRequested);
        app.update();
        let f: StatusFlags = get(&mut app);
        assert!(!f.reincarnation_offered, "未死亡时不应弹轮回");

        app.world_mut().write_message(ServerEvent::PlayerDied);
        app.world_mut().write_message(ServerEvent::ReincarnationRequested);
        app.update();
        let f: StatusFlags = get(&mut app);
        assert!(f.dead, "PlayerDied 应置 dead");
        assert!(f.reincarnation_offered, "死亡后轮回请求应生效");
    }

    /// MAJOR-2（评审）：PlayerRevived 复位 dead/reincarnation_offered + 死亡弹窗 dismissed。
    #[test]
    fn player_revived_resets_flags_and_death_ui() {
        let mut app = test_app();
        spawn_local(&mut app);
        enter_game(&mut app);

        app.world_mut().write_message(ServerEvent::PlayerDied);
        app.world_mut().write_message(ServerEvent::ReincarnationRequested);
        app.update();
        let f: StatusFlags = get(&mut app);
        assert!(f.dead && f.reincarnation_offered);
        assert!(!app.world().resource::<DeathDialogState>().dismissed);

        app.world_mut().write_message(ServerEvent::PlayerRevived);
        app.update();
        let f: StatusFlags = get(&mut app);
        assert!(!f.dead, "PlayerRevived 应复位 dead");
        assert!(!f.reincarnation_offered, "PlayerRevived 应复位轮回请求");
        assert!(!app.world().resource::<DeathDialogState>().dismissed);
    }

    /// CRITICAL-1（评审）：登出/ReturnToLogin 回登录界面时 OnExit(Game) 清掉 LocalPlayer
    /// 实体——同进程换角色重登不再出现双实体 → 全库 Query::single() 静默 MultipleEntities。
    #[test]
    fn local_player_despawned_on_exit_game() {
        let mut app = test_app();
        spawn_local(&mut app);
        enter_game(&mut app);
        assert!(
            app.world_mut()
                .query_filtered::<&Vitals, With<LocalPlayer>>()
                .iter(app.world())
                .next()
                .is_some(),
            "进入游戏应存在 LocalPlayer"
        );

        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Login);
        app.update(); // 应触发 OnExit(Game) → despawn_local_player

        assert!(
            app.world_mut()
                .query_filtered::<&Vitals, With<LocalPlayer>>()
                .iter(app.world())
                .next()
                .is_none(),
            "退出游戏后 LocalPlayer 应被清除"
        );
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

use super::*;

// ============================================================
// 英雄系统 Handler
// ============================================================

/// 切换英雄
pub struct ChangeHeroRequest {
    pub session_id: u64,
    pub hero_index: u8,
}

impl Message<ChangeHeroRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: ChangeHeroRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        if msg.hero_index == 0 && state.hero_index == 0 {
            send_system_message(&self.gate_ref, msg.session_id, "你没有可用的英雄");
            return;
        }

        let _ = record.actor_ref.ask(SetHeroIndex { hero_index: msg.hero_index }).await;
        send_hero_update_packet(&self.gate_ref, msg.session_id, msg.hero_index);
        // #198：切换后生成/移除英雄对象
        if msg.hero_index != 0 {
            self.broadcast_hero_spawn(msg.session_id).await;
            // #203：下发完整英雄信息（背包/装备/自动药）
            self.send_hero_information_packet(msg.session_id).await;
        } else {
            self.broadcast_hero_remove(record.object_id).await;
        }
        debug!("Hero switched: {} -> index {}", state.name, msg.hero_index);
    }
}

impl WorldActor {
/// 下发 S.HeroInformation（C# HeroInformation : UserInformation + autopot，#203）
/// 数据：英雄身份取 player_heroes，背包/装备/自动药取 PlayerState.hero_inventory
pub(crate) async fn send_hero_information_packet(&self, session_id: u64) {
    let record = match self.players.get(&session_id) {
        Some(r) => r,
        None => return,
    };
    let state = match record.actor_ref.ask(GetPlayerState).await {
        Ok(Some(s)) => s,
        _ => return,
    };
    let hero = self
        .player_heroes
        .get(&session_id)
        .and_then(|hs| hs.iter().find(|h| h.index as u8 == state.hero_index))
        .cloned();
    let Some(hero) = hero else { return };

    let hero_oid = record.object_id.wrapping_add(HERO_OID_OFFSET);
    // 内联 ItemInfo（客户端显示名称/图标/类型，与 build_user_information_packet 一致）
    let mut enrich = |mut item: mir2_shared::data::item::UserItem| {
        super::enrich_item_info(&mut item, &self.item_infos);
        item
    };
    let inventory: Vec<Option<mir2_shared::data::item::UserItem>> = state
        .hero_inventory
        .backpack
        .iter()
        .map(|s| s.as_ref().map(|s| enrich(s.item.clone())))
        .collect();
    let equipment: Vec<Option<mir2_shared::data::item::UserItem>> = state
        .hero_inventory
        .equipment
        .iter()
        .cloned()
        .map(|s| s.map(|item| enrich(item)))
        .collect();
    let ai_hp = self.hero_ai_states.get(&session_id).map(|ai| ai.hp).unwrap_or(0);
    let ai_mp = self
        .hero_ai_states
        .get(&session_id)
        .map(|ai| ai.mp)
        .unwrap_or(0);

    let packet = mir2_shared::packets::server::hero::HeroInformation {
        object_id: hero_oid,
        name: hero.name.clone(),
        class: hero.class,
        gender: hero.gender,
        level: hero.level,
        hair: 0,
        hp: ai_hp,
        mp: ai_mp,
        experience: 0,
        max_experience: 100,
        inventory: Some(inventory),
        equipment: Some(equipment),
        // #218：英雄魔法（DB C# 编号 → 客户端 +3）
        magics: state
            .hero_magics
            .iter()
            .filter_map(|m| {
                self.magic_infos
                    .get(&(m.spell as u32))
                    .map(|info| super::build_client_magic(info, m))
            })
            .collect(),
        auto_pot: state.auto_pot_hp > 0 || state.auto_pot_mp > 0,
        auto_hp_percent: state.auto_pot_hp.min(100) as u8,
        auto_mp_percent: state.auto_pot_mp.min(100) as u8,
        hp_item_index: state.auto_pot_hp_item,
        mp_item_index: state.auto_pot_mp_item,
    };
    let mut body = Vec::new();
    if packet.write_body(&mut body).is_err() {
        warn!("Failed to serialize HeroInformation");
        return;
    }
    let _ = self.gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::HeroInformation as i16, &body),
    }).try_send();
    info!("🦸 HeroInformation sent: {} (oid={})", hero.name, hero_oid);
}

}
/// 从英雄背包取回物品（C# C.TakeBackHeroItem: From=英雄格 To=主背包格，#203）
pub struct TakeBackHeroItemRequest {
    pub session_id: u64,
    pub from: i32,
    pub to: i32,
}

impl Message<TakeBackHeroItemRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: TakeBackHeroItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };

        // 从英雄背包取回指定格子物品到主背包
        let _ = record.actor_ref.ask(crate::actors::player::TakeBackHeroItem {
            from: msg.from,
            to: msg.to,
        }).await;
        // 刷新双方数据：主背包（全量 UserInformation）+ 英雄（S.HeroInformation）
        self.refresh_hero_item_state(msg.session_id).await;
        debug!("Hero item taken back: session={} from={} to={}", msg.session_id, msg.from, msg.to);
    }
}

/// 转移物品到英雄背包（C# C.TransferHeroItem: From=主背包格 To=英雄格，#203）
pub struct TransferHeroItemRequest {
    pub session_id: u64,
    pub from: i32,
    pub to: i32,
}

impl Message<TransferHeroItemRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: TransferHeroItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };

        // 从主背包转移指定格子物品到英雄背包
        let _ = record.actor_ref.ask(crate::actors::player::TransferHeroItem {
            from: msg.from,
            to: msg.to,
        }).await;
        self.refresh_hero_item_state(msg.session_id).await;
        debug!("Hero item transferred: session={} from={} to={}", msg.session_id, msg.from, msg.to);
    }
}

impl WorldActor {
    /// 英雄物品转移后刷新：主背包全量 UserInformation + 英雄 S.HeroInformation
    async fn refresh_hero_item_state(&self, session_id: u64) {
        let record = match self.players.get(&session_id) {
            Some(r) => r,
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        let packet = super::build_user_information_packet(&state, &self.item_infos);
        let _ = self.gate_ref.tell(SendToClient {
            session_id,
            data: packet,
        }).try_send();
        self.send_hero_information_packet(session_id).await;
    }
}

// ============================================================
// 宠物系统 Handler
// ============================================================

/// 更新/设置宠物
pub struct UpdateIntelligentCreature {
    pub session_id: u64,
    pub creature_type: u8,
    pub pickup_mode: u8,
}

impl Message<UpdateIntelligentCreature> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: UpdateIntelligentCreature, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        let creature_type = CreatureType::from(msg.creature_type);
        let pickup = PickupMode::from(msg.pickup_mode);

        if creature_type == CreatureType::None {
            // 关闭宠物
            let mut log = state.creature_log;
            log.active_creature = None;
            let _ = record.actor_ref.ask(SetCreature { creature_log: log }).await;
            send_system_message(&self.gate_ref, msg.session_id, "宠物已关闭");
            return;
        }

        // 设置或更新宠物
        let mut log = state.creature_log;
        if let Some(ref mut c) = log.active_creature {
            // 更新已有宠物
            c.pickup_mode = pickup;
        } else {
            // 创建新宠物
            let mut creature = IntelligentCreature::new(creature_type);
            creature.pickup_mode = pickup;
            creature.enabled = true;
            log.active_creature = Some(creature);
        }
        let creature_ref = log.active_creature.clone();
        let _ = record.actor_ref.ask(SetCreature { creature_log: log }).await;

        send_creature_list_packet(&self.gate_ref, msg.session_id, creature_ref.as_ref());
        debug!("UpdateIntelligentCreature: {} type={:?} mode={:?}", state.name, creature_type, pickup);
    }
}

/// 宠物拾取地面物品
pub struct IntelligentCreaturePickup {
    pub session_id: u64,
    pub x: i32,
    pub y: i32,
}

impl Message<IntelligentCreaturePickup> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: IntelligentCreaturePickup, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // 检查是否有激活的宠物
        let pickup_mode = match &state.creature_log.active_creature {
            Some(c) if c.enabled && !c.is_starving() => c.pickup_mode,
            _ => {
                send_system_message(&self.gate_ref, msg.session_id, "没有可用的宠物");
                return;
            }
        };

        // 根据拾取模式过滤
        if pickup_mode == PickupMode::None {
            send_system_message(&self.gate_ref, msg.session_id, "宠物拾取模式未设置");
            return;
        }

        // 查找附近的地面物品（同地图）
        let distance = 3; // 宠物拾取范围
        let item_idx = self.ground_items.iter().position(|item| {
            item.map_index == state.map_index
                && (item.x - msg.x).abs() <= distance
                && (item.y - msg.y).abs() <= distance
        });

        if let Some(idx) = item_idx {
            let item = self.ground_items.remove(idx);
            let picked_oid = item.object_id;
            // 将物品添加到玩家背包
            let mut picked_up = false;
            if let Some(rec) = self.players.get(&msg.session_id) {
                if let Ok(true) = rec.actor_ref.ask(AddItemToInventory {
                    item: item.item.clone(),
                }).await {
                    picked_up = true;
                }
            }
            if picked_up {
                // 广播 ObjectRemove
                let remove_packet = Self::build_object_remove_packet(picked_oid);
                for (sid, rec) in &self.players {
                    if let Ok(Some(s)) = rec.actor_ref.ask(GetPlayerState).await {
                        if s.map_index == state.map_index {
                            let _ = self.gate_ref.tell(SendToClient {
                                session_id: *sid,
                                data: remove_packet.clone(),
                            }).await;
                        }
                    }
                }
                debug!("Creature pickup: {} picked up item at ({},{})",
                       state.name, msg.x, msg.y);
            } else {
                // 添加失败，放回去
                self.ground_items.push(item);
            }
        }
    }
}

/// 请求宠物更新列表
pub struct RequestIntelligentCreatureUpdates {
    pub session_id: u64,
    pub request_updates: bool,
}

impl Message<RequestIntelligentCreatureUpdates> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: RequestIntelligentCreatureUpdates, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // 发送当前宠物列表
        let creature_ref = state.creature_log.active_creature.clone();
        send_creature_list_packet(&self.gate_ref, msg.session_id, creature_ref.as_ref());
    }
}

// ============================================================
// 英雄战斗 AI
// ============================================================
//
// 对齐 C# Server/MirObjects/HeroObject.cs ProcessAI/ProcessTarget/Attack
// 及 5 个职业子类（Warrior/Wizard/Taoist/Assassin/Archer）。
//
// 关键差异：Rust 端 Hero 不是独立 WorldObject，没有持久位置/HP，因此 AI
// 将 Hero 位置模拟为主人附近（首次出战后挂在主人后方 1 格），目标从主人
// 视野内最近的怪物中选取（对应 C# FindTarget）。
//
// 输出队列模式（参考怪物 AI）：先在循环内收集移动/攻击意图，循环外统一应用，
// 避免对 self.monsters 的多重借用冲突。

/// 英雄 AI 视野半径（对齐 C# HeroObject.ViewRange = 8）
const HERO_VIEW_RANGE: i32 = 8;
/// 英雄跟随主人时的保持距离（C# ProcessRoam 跟随 Owner.Back）
const HERO_FOLLOW_DISTANCE: i32 = 5;
/// 英雄因远离主人被强制召回的距离阈值（C# Globals.DataRange）
const HERO_RECALL_DISTANCE: i32 = 35;
/// 英雄自动喝药 HP 阈值（百分比），HP 低于此值时后撤（对应 ProcessAutoPot + CounterAttack 行为）
const HERO_FLEE_HP_PERCENT: i32 = 30;
/// 英雄近战攻击范围（战士/刺客）
const HERO_MELEE_RANGE: i32 = 1;
/// 英雄远程攻击范围（法师/道士/弓箭手）
const HERO_RANGED_RANGE: i32 = 7;
/// 英雄自动喝药检查间隔（tick 数，C# AutoPotDelay=1000ms）
const HERO_AUTOPOT_INTERVAL_TICKS: u64 = 10;

/// 英雄自身增益类型（#1190：C# HumanObject 各职业 ProcessFriend 自增益）
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HeroBuffKind {
    Rage,
    ProtectionField,
    Haste,
    LightBody,
    MagicShield,
    MagicBooster,
    Concentration,
    SoulShield,
    BlessedArmour,
    UltimateEnhancer,
    PoisonShot,
}

/// 英雄增益实例（#1190）
#[derive(Clone, Copy, Debug)]
struct HeroBuff {
    kind: HeroBuffKind,
    /// 到期 tick（WorldActor.tick_count）
    expire_tick: u64,
    /// 技能等级（影响数值/时长）
    level: u8,
    /// 英雄自身等级（#1192：SoulShield/BlessedArmour 按目标等级加成）
    hero_level: u16,
}

/// 主人护盾类型（#1202：C# TaoistHero 给主人上 SoulShield/BlessedArmour/UltimateEnhancer）
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OwnerShieldKind {
    SoulShield,
    BlessedArmour,
    UltimateEnhancer,
}

/// 英雄 AI 运行时状态（每个出战英雄一个实例）
#[derive(Clone)]
pub struct HeroCombatAI {
    /// 英雄当前模拟位置（x）
    pub x: i32,
    /// 英雄当前模拟位置（y）
    pub y: i32,
    /// 朝向（0-7）
    pub direction: u8,
    /// 下次可攻击的 tick（冷却）
    pub next_attack_tick: u64,
    /// 下次可移动的 tick
    pub next_move_tick: u64,
    /// 下次可施法的 tick
    pub next_cast_tick: u64,
    /// 当前锁定的怪物 object_id（对应 C# HeroObject.Target）
    pub target_oid: Option<u32>,
    /// 当前 HP（由主人的 hero 缓存模拟，简化：使用主人 max_hp 的 60% 作为英雄 max_hp）
    pub hp: i32,
    /// 最大 HP
    pub max_hp: i32,
    /// 上次已下发主人的 HP（#1134：避免每 tick 重复发 HeroHealthChanged）
    pub last_sent_hp: i32,
    /// 药水累计待回复 HP（#1182 C# PotHealthAmount，NormalPotion 累加）
    pub pot_health: u32,
    /// 药水累计待回复 MP（#1186 C# PotManaAmount，NormalPotion 累加）
    pub pot_mana: u32,
    /// 下次自动喝药检查 tick（#1182 C# AutoPotTime）
    pub next_autopot_tick: u64,
    /// 当前 MP（#1186：C# Stats[MP]，施法耗蓝/回蓝）
    pub mp: i32,
    /// 最大 MP（#1186：C# Stats[MP]）
    pub max_mp: i32,
    /// 上次已下发主人的 MP（#1186：避免每 tick 重复发 HeroHealthChanged）
    pub last_sent_mp: i32,
    /// 自身增益列表（#1190：C# Buffs）
    pub buffs: Vec<HeroBuff>,
}

impl HeroCombatAI {
    /// 以主人状态初始化英雄 AI（主人后方 1 格出生）
    fn new_for_owner(owner_x: i32, owner_y: i32, hero_max_hp: i32, hero_max_mp: i32) -> Self {
        // #1180/#1186：英雄 HP/MP 用自身属性（C# Stats[HP]/Stats[MP]）
        let max_hp = hero_max_hp.max(1);
        let max_mp = hero_max_mp.max(1);
        Self {
            x: owner_x,
            y: owner_y.saturating_add(1),
            direction: 0,
            next_attack_tick: 0,
            next_move_tick: 0,
            next_cast_tick: 0,
            target_oid: None,
            hp: max_hp,
            max_hp,
            last_sent_hp: max_hp,
            pot_health: 0,
            pot_mana: 0,
            next_autopot_tick: 0,
            mp: max_mp,
            max_mp,
            last_sent_mp: max_mp,
            buffs: Vec::new(),
        }
    }
}

impl WorldActor {
    /// 英雄战斗 AI 主循环（每 3 ticks 运行一次，约 300ms）
    ///
    /// 流程对齐 C# HeroObject.Process：
    ///   1. ProcessSearch / FindTarget：在主人视野内找最近怪物
    ///   2. ProcessAI：按职业决定辅助技能（ProcessFriend）/攻击技能（ProcessAttack）
    ///   3. ProcessTarget：在攻击范围则攻击，否则移动靠近；远离主人时召回
    ///   4. ProcessRoam：无目标时跟随主人（保持 3-5 格）
    pub(crate) async fn tick_heroes(&mut self) {
        // 每 3 ticks 执行一次，降低 CPU 开销
        if self.tick_count % 3 != 0 {
            return;
        }

        use mir2_shared::enums::{DefenceType, MirClass, Spell};

        // ===== 阶段 0：收集所有出战英雄的快照 =====
        // (session_id, owner_state, class, hero_behaviour)
        struct HeroSnapshot {
            session_id: u64,
            owner_x: i32,
            owner_y: i32,
            owner_map: u16,
            class: MirClass,
            /// hero_behaviour: C# 0=Attack, 1=CounterAttack, 2=Follow, 3=Custom（#1198）
            behaviour: u8,
            /// 主人是否死亡
            owner_dead: bool,
            /// 主人当前 HP（道士治疗判定用）
            owner_hp: i32,
            /// 主人最大 HP
            owner_max_hp: i32,
            /// 主人用于战斗公式的属性快照
            owner_stats: crate::combat::attack::CombatStats,
            /// 主人等级（level_offset 用）
            owner_level: u16,
            /// 英雄自身战斗属性（#1180：C# BaseStats 移植，替代主人属性）
            hero_combat: crate::combat::attack::CombatStats,
            /// 英雄自身最大 HP（#1180）
            hero_max_hp: i32,
            /// 英雄自身最大 MP（#1186：C# Stats[MP]）
            hero_max_mp: i32,
            /// 英雄完整自身属性（#1184：DC/MC/SC 齐备，施法/治疗用自身属性）
            hero_stats: super::hero_stats::HeroStats,
            /// 英雄自身等级（#1182 C# PerTickRegen = 5 + Level/10）
            hero_level: u16,
            /// 已学技能 (C# spell, level)（#1186：施法耗蓝按实际等级）
            hero_magics: Vec<(i32, u8)>,
            /// 自动喝药 HP 阈值（0=关闭，C# AutoHPPercent）
            auto_hp_percent: u8,
            /// 自动喝药物品 index（C# HPItemIndex）
            hp_item_index: i32,
            /// 自动喝药 MP 阈值（0=关闭，C# AutoMPPercent）
            auto_mp_percent: u8,
            /// 自动喝蓝物品 index（C# MPItemIndex）
            mp_item_index: i32,
            /// 装备的毒护符 shape（#1192：C# GetPoison：Amulet shape 1=绿毒 / 2=红毒；0=无）
            hero_poison_shape: i32,
            /// 是否装备普通护符（#1192：C# GetAmulet：Amulet shape 0）
            hero_amulet: bool,
            /// 主人是否已有对应护盾 buff（SoulShield/BlessedArmour/UltimateEnhancer，#1202）
            owner_has_shields: [bool; 3],
            /// 主人是否中毒（#1210：道士 Purification 条件）
            owner_poisoned: bool,
        }

        let mut snapshots: Vec<HeroSnapshot> = Vec::new();
        for (session_id, record) in &self.players {
            if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                // hero_index > 0 表示有出战英雄
                // #1134：AI HP<=0 视为阵亡，不再参与战斗（REVIVEHERO 回满后恢复）
                let hero_dead = self.hero_ai_states.get(session_id).map(|ai| ai.hp <= 0).unwrap_or(false);
                if state.hero_index == 0 || state.is_dead || state.hero_despawned || hero_dead {
                    continue;
                }
                // #1198：hero_behaviour == 2 (Follow) 时英雄纯跟随，不参战
                // 但仍需移动跟随主人，所以保留快照（AI 内部判断 behaviour；C# 1=CounterAttack）
                // #1184：英雄自身属性只算一次（基础 + 装备加成）
                let hero = self.player_heroes.get(session_id)
                    .and_then(|hs| hs.iter().find(|h| h.index as u8 == state.hero_index))
                    .cloned();
                let hero_stats = hero.as_ref().map(|h| {
                    super::hero_stats::compute_hero_stats(
                        h.class,
                        h.level as i32,
                        &state.hero_inventory.equipment,
                        &self.item_infos,
                    )
                });
                let hero_level = hero.as_ref().map(|h| h.level).unwrap_or(state.level);
                snapshots.push(HeroSnapshot {
                    session_id: *session_id,
                    owner_x: state.x,
                    owner_y: state.y,
                    owner_map: state.map_index,
                    class: state.class,
                    behaviour: state.hero_behaviour,
                    owner_dead: state.is_dead,
                    owner_hp: state.hp,
                    owner_max_hp: state.max_hp,
                    owner_stats: state.to_combat_stats(),
                    owner_level: state.level,
                    // #1180/#1184：英雄自身属性（C# BaseStats + 英雄装备加成）
                    hero_combat: hero_stats
                        .map(|s| s.to_combat_stats())
                        .unwrap_or_else(|| state.to_combat_stats()),
                    hero_max_hp: hero_stats.map(|s| s.max_hp).unwrap_or(100),
                    hero_max_mp: hero_stats.map(|s| s.max_mp).unwrap_or(60),
                    hero_stats: hero_stats.unwrap_or_else(|| {
                        super::hero_stats::hero_base_stats(state.class, state.level as i32)
                    }),
                    hero_level,
                    hero_magics: state
                        .hero_magics
                        .iter()
                        .map(|m| (m.spell, m.level))
                        .collect(),
                    auto_hp_percent: state.auto_pot_hp.min(100) as u8,
                    hp_item_index: state.auto_pot_hp_item,
                    auto_mp_percent: state.auto_pot_mp.min(100) as u8,
                    mp_item_index: state.auto_pot_mp_item,
                    hero_poison_shape: hero_equip_poison_shape(
                        &state.hero_inventory.equipment,
                        &self.item_infos,
                    ),
                    hero_amulet: hero_has_amulet(
                        &state.hero_inventory.equipment,
                        &self.item_infos,
                    ),
                    // #1202：主人已有对应护盾 buff（UltimateEnhancer 用 DC/MC/SC Boost 近似）
                    owner_has_shields: [
                        state.buffs.iter().any(|b| {
                            matches!(b.buff_type, crate::combat::buff::BuffType::MacDefenseBoost { .. })
                        }),
                        state.buffs.iter().any(|b| {
                            matches!(b.buff_type, crate::combat::buff::BuffType::AcDefenseBoost { .. })
                        }),
                        state.buffs.iter().any(|b| {
                            matches!(
                                b.buff_type,
                                crate::combat::buff::BuffType::AttackBoost { .. }
                                    | crate::combat::buff::BuffType::McBoost { .. }
                                    | crate::combat::buff::BuffType::ScBoost { .. }
                            )
                        }),
                    ],
                    owner_poisoned: !state.poison_list.is_empty(),
                });
            }
        }

        if snapshots.is_empty() {
            return;
        }

        // ===== 阶段 1：预收集怪物快照（避免循环内借用 self.monsters） =====
        struct MonsterSnap {
            oid: u32,
            x: i32,
            y: i32,
            max_hp: i32,
            map_index: u16,
            /// #1192：目标是否已有绿毒/红毒（道士 Poisoning 条件）
            has_green: bool,
            has_red: bool,
            /// #1196：目标是否已有减速毒（道士 Curse 条件）
            has_slow: bool,
            /// #1212：目标是否亡灵（TurnUndead 条件）
            undead: bool,
            /// #1212：目标怪物等级（Repulsion/TurnUndead 条件）
            level: i32,
        }
        let monster_snaps: Vec<MonsterSnap> = self.monsters.values()
            .filter(|m| m.hp > 0)
            .map(|m| MonsterSnap {
                oid: m.object_id,
                x: m.x,
                y: m.y,
                max_hp: m.max_hp,
                map_index: m.map_index,
                has_green: m.poison_list.iter().any(|p| p.p_type.intersects(mir2_shared::enums::PoisonType::GREEN)),
                has_red: m.poison_list.iter().any(|p| p.p_type.intersects(mir2_shared::enums::PoisonType::RED)),
                has_slow: m.poison_list.iter().any(|p| p.p_type.intersects(mir2_shared::enums::PoisonType::SLOW)),
                undead: m.undead,
                level: self.monster_infos.get(&m.monster_index).map(|i| i.level).unwrap_or(0),
            })
            .collect();

        // ===== 阶段 2：意图收集（循环内只收集，不修改 self） =====
        // 移动意图：(hero_session_id, new_x, new_y, direction) —— 循环外更新 hero_ai_states
        let mut move_intents: Vec<(u64, i32, i32, u8)> = Vec::new();
        // 近战/远程物理攻击意图：(hero_session_id, target_oid, raw_damage, defence_type, is_ranged)
        let mut attack_intents: Vec<(u64, u32, i32, DefenceType, bool)> = Vec::new();
        // 弹道法术意图（法师/道士/弓箭手远程技能）：直接 push 到 pending_spell_completions
        // (session_id, spell, target_oid, target_x, target_y, damage, fire_at_tick, level)
        let mut spell_intents: Vec<(u64, u8, u32, i32, i32, i32, u64, u8)> = Vec::new();
        // 辅助意图（道士治疗主人 / 战士 buff）：暂时简化为发送 ObjectAttack 广播但不造伤害
        // (hero_session_id, target_session_or_zero, spell_id, is_heal)
        let mut support_intents: Vec<(u64, u64, u8, bool)> = Vec::new();
        // 自动喝药意图：(hero_session_id, item_index, is_mp) —— 阶段 2.4 统一消耗（C# ProcessAutoPot/TryAutoPot）
        let mut autopot_intents: Vec<(u64, i32, bool)> = Vec::new();
        // 毒意图：(hero_session_id, target_oid, poison_type, duration_s, value, tick_ms) —— 阶段 3e 应用（#1192/#1196）
        let mut poison_intents: Vec<(u64, u32, mir2_shared::enums::PoisonType, u32, i32, u64)> = Vec::new();
        // 法师 AoE 意图：(hero_session_id, spell, target_oid, tx, ty, raw, level) —— 阶段 3f 应用（#1200）
        let mut aoe_intents: Vec<(u64, u8, u32, i32, i32, i32, u8, i32)> = Vec::new();
        // 主人护盾意图：(hero_session_id, kind) —— 阶段 2.4b 应用（#1202）
        let mut owner_shield_intents: Vec<(u64, OwnerShieldKind)> = Vec::new();
        // 支持类法术动画意图：(hero_session_id, spell, target_oid) —— 阶段 3g 广播 ObjectMagic（#1208）
        let mut magic_anim_intents: Vec<(u64, u8, u32)> = Vec::new();
        // 净化意图：(hero_session_id) —— 阶段 2.4c 清除主人中毒（#1210）
        let mut purify_intents: Vec<u64> = Vec::new();
        // 击退意图：(target_oid, direction, distance) —— 阶段 3h 应用（#1212 Repulsion）
        let mut push_intents: Vec<(u32, u8, i32)> = Vec::new();
        // 超度意图：(hero_session_id, target_oid) —— 阶段 3h 击杀亡灵（#1212 TurnUndead）
        let mut turn_undead_intents: Vec<(u64, u32)> = Vec::new();

        for snap in &snapshots {
            // 确保该英雄有 AI 状态（首次出现则初始化）
            let ai = self
                .hero_ai_states
                .entry(snap.session_id)
                .or_insert_with(|| {
                    HeroCombatAI::new_for_owner(
                        snap.owner_x,
                        snap.owner_y,
                        snap.hero_max_hp,
                        snap.hero_max_mp,
                    )
                });
            // 暴露可变副本用于本 tick 决策（循环内不写回 self）
            let mut ai_local = ai.clone();
            // #1208：支持类法术 ObjectMagic 广播的目标 oid
            let owner_oid = self.players.get(&snap.session_id).map(|r| r.object_id).unwrap_or(0);
            let hero_oid = owner_oid.wrapping_add(HERO_OID_OFFSET);
            // #1190：清理过期增益
            ai_local.buffs.retain(|b| b.expire_tick > self.tick_count);
            // #1190：buff 对战斗属性/回蓝/冷却的影响（C# RefreshStats 对应项；hero_combat/hero_stats 为局部加 buff 副本）
            let mut hero_combat = snap.hero_combat;
            let mut hero_stats = snap.hero_stats;
            let shield_pct = hero_apply_buffs(&ai_local.buffs, snap.class, &mut hero_combat, &mut hero_stats);
            let haste_ticks = ai_local
                .buffs
                .iter()
                .find(|b| b.kind == HeroBuffKind::Haste)
                .map(|b| (b.level as i32 * 2 + 2) as u64)
                .unwrap_or(0);
            let concentrating = ai_local.buffs.iter().any(|b| b.kind == HeroBuffKind::Concentration);
            // #1134：英雄 HP 不再每 tick 强制满血——改为脱战缓慢回血（C# Stats 回血近似）
            // 上一 tick 无锁定目标视为脱战（战斗中不回血，损耗可见）
            if !snap.owner_dead && ai_local.hp > 0 && ai_local.hp < ai_local.max_hp
                && ai_local.target_oid.is_none()
            {
                let regen = (ai_local.max_hp / 100).max(1);
                ai_local.hp = (ai_local.hp + regen).min(ai_local.max_hp);
            }
            // #1182：药水持续回复（C# HumanObject.ProcessRegen：PerTickRegen = 5 + Level/10）
            // 每 AI tick 从 PotHealthAmount 扣除，战斗中也生效（与自然回血叠加）
            if ai_local.hp > 0 && ai_local.hp < ai_local.max_hp && ai_local.pot_health > 0 {
                let per_tick = (5 + snap.hero_level as i32 / 10).max(1) as u32;
                let need = (ai_local.max_hp - ai_local.hp) as u32;
                let regen = per_tick.min(ai_local.pot_health).min(need);
                ai_local.hp += regen as i32;
                ai_local.pot_health -= regen;
            }
            // #1186：自然回蓝（C# ProcessRegen：CanRegen 时 (int)(Stats[MP]*0.03)+1）
            if !snap.owner_dead
                && ai_local.mp > 0
                && ai_local.mp < ai_local.max_mp
                && ai_local.target_oid.is_none()
            {
                let mut regen = (ai_local.max_mp * 3 / 100 + 1).max(1);
                // #1190：Concentration 专注回蓝增强（近似 ×2）
                if concentrating {
                    regen *= 2;
                }
                ai_local.mp = (ai_local.mp + regen).min(ai_local.max_mp);
            }
            // #1186：药水持续回蓝（C# ProcessRegen：PerTickRegen 从 PotManaAmount 扣除）
            if ai_local.mp > 0 && ai_local.mp < ai_local.max_mp && ai_local.pot_mana > 0 {
                let per_tick = (5 + snap.hero_level as i32 / 10).max(1) as u32;
                let need = (ai_local.max_mp - ai_local.mp) as u32;
                let regen = per_tick.min(ai_local.pot_mana).min(need);
                ai_local.mp += regen as i32;
                ai_local.pot_mana -= regen;
            }
            // #1182/#1186：自动喝药检查（C# HeroObject.ProcessAutoPot，AutoPotDelay=1000ms）
            // HP：HP% < AutoHPPercent && HPItemIndex>0 && PotHealthAmount<=0
            // MP：MP% < AutoMPPercent && MPItemIndex>0 && PotManaAmount<=0
            if ai_local.hp > 0 && self.tick_count >= ai_local.next_autopot_tick {
                ai_local.next_autopot_tick = self.tick_count + HERO_AUTOPOT_INTERVAL_TICKS;
                let hp_pct = if ai_local.max_hp > 0 {
                    (ai_local.hp * 100 / ai_local.max_hp) as u8
                } else {
                    100
                };
                let mp_pct = if ai_local.max_mp > 0 {
                    (ai_local.mp * 100 / ai_local.max_mp) as u8
                } else {
                    100
                };
                if snap.auto_hp_percent > 0
                    && snap.hp_item_index > 0
                    && ai_local.pot_health == 0
                    && hp_pct < snap.auto_hp_percent
                {
                    autopot_intents.push((snap.session_id, snap.hp_item_index, false));
                }
                if snap.auto_mp_percent > 0
                    && snap.mp_item_index > 0
                    && ai_local.pot_mana == 0
                    && mp_pct < snap.auto_mp_percent
                {
                    autopot_intents.push((snap.session_id, snap.mp_item_index, true));
                }
            }

            // #1198：C# HeroBehaviour：2=Follow（原误把 1 当 Follow，实为 CounterAttack）
            let behaviour_follow = hero_behaviour_is_follow(snap.behaviour);

            // ===== 距主人过远：强制召回（C# OwnerRecall） =====
            let dist_to_owner = (ai_local.x - snap.owner_x).abs() + (ai_local.y - snap.owner_y).abs();
            if dist_to_owner > HERO_RECALL_DISTANCE {
                ai_local.x = snap.owner_x;
                ai_local.y = snap.owner_y.saturating_add(1);
                ai_local.direction = 0;
                ai_local.target_oid = None;
                move_intents.push((snap.session_id, ai_local.x, ai_local.y, ai_local.direction));
                *ai = ai_local;
                continue;
            }

            // ===== Follow 模式 或 主人死亡：只跟随主人 =====
            if behaviour_follow || snap.owner_dead {
                let target_dist = HERO_FOLLOW_DISTANCE;
                if dist_to_owner > target_dist && self.tick_count >= ai_local.next_move_tick {
                    // 向主人移动一步（复用 step_toward 逻辑）
                    let (nx, ny, dir) = step_towards(ai_local.x, ai_local.y, snap.owner_x, snap.owner_y);
                    ai_local.x = nx;
                    ai_local.y = ny;
                    ai_local.direction = dir;
                    ai_local.next_move_tick = self.tick_count + 2;
                    move_intents.push((snap.session_id, nx, ny, dir));
                }
                ai_local.target_oid = None;
                *ai = ai_local;
                continue;
            }

            // ===== Attack 模式：找目标（C# FindTarget） =====
            // 在主人视野内找最近的活怪
            let target = monster_snaps.iter()
                .filter(|m| m.map_index == snap.owner_map)
                // #1198：CounterAttack 只锁定正在攻击主人的怪（C# FindTarget：ob.Target != this/Owner 跳过）
                .filter(|m| {
                    if !hero_behaviour_is_counterattack(snap.behaviour) {
                        return true;
                    }
                    self.monsters
                        .get(&m.oid)
                        .map(|mo| mo.target_session == Some(snap.session_id))
                        .unwrap_or(false)
                })
                .map(|m| (m, (m.x - snap.owner_x).abs() + (m.y - snap.owner_y).abs()))
                .filter(|(_, d)| *d <= HERO_VIEW_RANGE)
                .min_by_key(|(_, d)| *d)
                .map(|(m, _)| m);

            if target.is_none() {
                // 无目标：跟随主人（ProcessRoam）
                ai_local.target_oid = None;
                if dist_to_owner > HERO_FOLLOW_DISTANCE && self.tick_count >= ai_local.next_move_tick {
                    let (nx, ny, dir) = step_towards(ai_local.x, ai_local.y, snap.owner_x, snap.owner_y);
                    ai_local.x = nx;
                    ai_local.y = ny;
                    ai_local.direction = dir;
                    ai_local.next_move_tick = self.tick_count + 2;
                    move_intents.push((snap.session_id, nx, ny, dir));
                }
                *ai = ai_local;
                continue;
            }

            let target = target.unwrap();
            ai_local.target_oid = Some(target.oid);
            let target_dist = (ai_local.x - target.x).abs() + (ai_local.y - target.y).abs();
            let can_attack = self.tick_count >= ai_local.next_attack_tick;
            let can_move = self.tick_count >= ai_local.next_move_tick;

            // #1190：ProcessFriend 自增益（C# 各子类：有目标且已学且无同 buff 且蓝足够 → 施放）
            // 施放当 tick 不攻击（C# ProcessFriend return 后跳过 ProcessAttack）
            let friend = hero_friend_buffs(snap.class)
                .iter()
                .find(|(spell, kind)| {
                    // #1210：道士 ProcessFriend 常驻预置块处理（跟随/待机也生效），此处跳过
                    snap.class != MirClass::Taoist
                        && hero_magic_level(&snap.hero_magics, *spell as u8) > 0
                        && !ai_local.buffs.iter().any(|b| b.kind == *kind)
                })
                .copied();
            if let Some((spell, kind)) = friend {
                let buff_lv = hero_magic_level(&snap.hero_magics, spell as u8);
                let cost = hero_spell_cost(&self.magic_infos, &snap.hero_magics, spell as u8);
                if ai_local.mp >= cost {
                    ai_local.mp -= cost;
                    ai_local.buffs.push(HeroBuff {
                        kind,
                        expire_tick: self.tick_count + hero_buff_duration(kind, buff_lv, &snap.hero_stats) * 10,
                        level: buff_lv,
                        hero_level: snap.hero_level,
                    });
                    support_intents.push((snap.session_id, snap.session_id, spell as u8, false));
                    // #1208：自增益广播 ObjectMagic（目标 = 英雄自身）
                    magic_anim_intents.push((snap.session_id, spell as u8, hero_oid));
                    ai_local.next_attack_tick = self.tick_count + 4;
                    *ai = ai_local;
                    continue;
                }
            }

            // #1210：C# TaoistHero.ProcessFriend 常驻（TargetList=[this,Owner]，跟随/待机也生效）
            // 顺序：净化 → 治疗（先自己后主人）→ 护盾（先自己后主人）；每 tick 只施放一个
            if snap.class == MirClass::Taoist && ai_local.hp > 0 && !snap.owner_dead {
                let hero_hp_pct = if ai_local.max_hp > 0 {
                    ai_local.hp * 100 / ai_local.max_hp
                } else { 100 };
                let owner_hp_pct = if snap.owner_max_hp > 0 {
                    snap.owner_hp * 100 / snap.owner_max_hp
                } else { 100 };
                let healing_lv = hero_magic_level(&snap.hero_magics, Spell::Healing as u8);
                // 1) 净化：主人中毒且已学（C# Random(4)<=Lv 成功）
                let pur_lv = hero_magic_level(&snap.hero_magics, Spell::Purification as u8);
                if hero_taoist_needs_purify(snap.owner_poisoned, pur_lv) {
                    let cost = hero_spell_cost(&self.magic_infos, &snap.hero_magics, Spell::Purification as u8);
                    if ai_local.mp >= cost {
                        ai_local.mp -= cost;
                        if hero_purification_roll(pur_lv) {
                            purify_intents.push(snap.session_id);
                        }
                        magic_anim_intents.push((snap.session_id, Spell::Purification as u8, owner_oid));
                        ai_local.next_attack_tick = self.tick_count + 4;
                        *ai = ai_local;
                        continue;
                    }
                }
                // 2) 治疗：已学 MassHealing → 群疗（自+主一次，C# GetDamage(SC)）；否则单疗（先自己后主人）
                let mass_lv = hero_magic_level(&snap.hero_magics, Spell::MassHealing as u8);
                if (mass_lv > 0 || healing_lv > 0) && (hero_hp_pct < 90 || owner_hp_pct < 90) {
                    let spell = if mass_lv > 0 {
                        Spell::MassHealing
                    } else {
                        Spell::Healing
                    };
                    let heal_cost = hero_spell_cost(&self.magic_infos, &snap.hero_magics, spell as u8);
                    if ai_local.mp >= heal_cost {
                        ai_local.mp -= heal_cost;
                        if mass_lv > 0 {
                            // #1214：C# MassHealing 3x3 友方群疗（自 + 主一次）
                            let amount = hero_mass_heal_amount(&self.magic_infos, &snap.hero_magics, &hero_stats, snap.class);
                            if hero_hp_pct < 90 {
                                ai_local.hp = (ai_local.hp + amount).min(ai_local.max_hp);
                            }
                            support_intents.push((snap.session_id, snap.session_id, Spell::MassHealing as u8, true));
                            magic_anim_intents.push((snap.session_id, Spell::MassHealing as u8, hero_oid));
                        } else if hero_hp_pct < 90 {
                            let amount = hero_heal_amount(&hero_stats, snap.hero_level);
                            ai_local.hp = (ai_local.hp + amount).min(ai_local.max_hp);
                            magic_anim_intents.push((snap.session_id, Spell::Healing as u8, hero_oid));
                        } else {
                            support_intents.push((snap.session_id, snap.session_id, Spell::Healing as u8, true));
                            magic_anim_intents.push((snap.session_id, Spell::Healing as u8, owner_oid));
                        }
                        ai_local.next_attack_tick = self.tick_count + 4;
                        *ai = ai_local;
                        continue;
                    }
                }
                // 3) 护盾：先自己后主人（SoulShield → BlessedArmour → UltimateEnhancer，护符门控）
                if snap.hero_amulet {
                    let self_shield = hero_friend_buffs(snap.class)
                        .iter()
                        .find(|(spell, kind)| {
                            hero_magic_level(&snap.hero_magics, *spell as u8) > 0
                                && !ai_local.buffs.iter().any(|b| b.kind == *kind)
                        })
                        .copied();
                    if let Some((spell, kind)) = self_shield {
                        let buff_lv = hero_magic_level(&snap.hero_magics, spell as u8);
                        let cost = hero_spell_cost(&self.magic_infos, &snap.hero_magics, spell as u8);
                        if ai_local.mp >= cost {
                            ai_local.mp -= cost;
                            ai_local.buffs.push(HeroBuff {
                                kind,
                                expire_tick: self.tick_count
                                    + hero_buff_duration(kind, buff_lv, &snap.hero_stats) * 10,
                                level: buff_lv,
                                hero_level: snap.hero_level,
                            });
                            magic_anim_intents.push((snap.session_id, spell as u8, hero_oid));
                            ai_local.next_attack_tick = self.tick_count + 4;
                            *ai = ai_local;
                            continue;
                        }
                    } else {
                        // 主人护盾（目标 = 主人）
                        let owner_kind = [
                            (OwnerShieldKind::SoulShield, 0usize),
                            (OwnerShieldKind::BlessedArmour, 1usize),
                            (OwnerShieldKind::UltimateEnhancer, 2usize),
                        ]
                        .iter()
                        .find(|(kind, idx)| {
                            !snap.owner_has_shields[*idx]
                                && hero_magic_level(&snap.hero_magics, hero_owner_shield_spell(*kind) as u8) > 0
                        })
                        .map(|(kind, _)| *kind);
                        if let Some(kind) = owner_kind {
                            let spell = hero_owner_shield_spell(kind);
                            let cost = hero_spell_cost(&self.magic_infos, &snap.hero_magics, spell as u8);
                            if ai_local.mp >= cost {
                                ai_local.mp -= cost;
                                owner_shield_intents.push((snap.session_id, kind));
                                magic_anim_intents.push((snap.session_id, spell as u8, owner_oid));
                                ai_local.next_attack_tick = self.tick_count + 4;
                                *ai = ai_local;
                                continue;
                            }
                        }
                    }
                }
            }

            // HP 低于阈值：后撤（对应 ArcherHero ProcessTarget 的远离逻辑 + 自动喝药）
            let hp_pct = if ai_local.max_hp > 0 {
                ai_local.hp * 100 / ai_local.max_hp
            } else { 100 };
            if hp_pct < HERO_FLEE_HP_PERCENT && can_move {
                let (nx, ny, dir) = step_away_from(target.x, target.y, ai_local.x, ai_local.y);
                if self.maps.get(&snap.owner_map).map(|m| m.is_walkable(nx, ny)).unwrap_or(true) {
                    ai_local.x = nx;
                    ai_local.y = ny;
                    ai_local.direction = dir;
                    ai_local.next_move_tick = self.tick_count + 2;
                    move_intents.push((snap.session_id, nx, ny, dir));
                }
                *ai = ai_local;
                continue;
            }

            // ===== 按职业决定攻击方式（C# 各职业子类 ProcessAttack） =====
            // 先确定攻击范围（近战 vs 远程）+ 防御类型
            let (attack_range, defence) = match snap.class {
                MirClass::Warrior | MirClass::Assassin => (HERO_MELEE_RANGE, DefenceType::Ac),
                MirClass::Wizard => (HERO_RANGED_RANGE, DefenceType::Mac),
                MirClass::Taoist => (HERO_RANGED_RANGE, DefenceType::Mac),
                MirClass::Archer => (HERO_RANGED_RANGE, DefenceType::Ac),
            };

            // #1188：战士 Thrusting / 刺客 HeavenlySword 可在距离 2 施放（C# 子类 InAttackRange 扩展）
            let ranged2_skill = match snap.class {
                MirClass::Warrior => Some(Spell::Thrusting),
                MirClass::Assassin => Some(Spell::HeavenlySword),
                _ => None,
            };
            if can_attack && target_dist == 2 {
                if let Some(spell) = ranged2_skill {
                    if hero_magic_level(&snap.hero_magics, spell as u8) > 0 {
                        ai_local.direction = direction_towards(ai_local.x, ai_local.y, target.x, target.y);
                        let raw = hero_attack_power(&hero_combat);
                        attack_intents.push((snap.session_id, target.oid, raw, DefenceType::Ac, true));
                        support_intents.push((snap.session_id, 0, spell as u8, false));
                        ai_local.next_attack_tick = self.tick_count + 6;
                        // #1190：Haste 缩短攻击冷却
                        if haste_ticks > 0 {
                            ai_local.next_attack_tick = ai_local
                                .next_attack_tick
                                .saturating_sub(haste_ticks)
                                .max(self.tick_count + 2);
                        }
                        *ai = ai_local;
                        continue;
                    }
                }
            }

            // ===== 在攻击范围内：攻击/施法 =====
            if target_dist <= attack_range && can_attack {
                ai_local.direction = direction_towards(ai_local.x, ai_local.y, target.x, target.y);

                match snap.class {
                    MirClass::Warrior => {
                        // #1188：C# WarriorHero.Attack 优先级取已学技能（Thrusting 已在距离 2 分支处理）
                        let raw = hero_attack_power(&hero_combat);
                        let learned = first_learned_spell(
                            &snap.hero_magics,
                            &[
                                Spell::Slaying,
                                Spell::HalfMoon,
                                Spell::CrossHalfMoon,
                                Spell::TwinDrakeBlade,
                                Spell::FlamingSword,
                            ],
                        );
                        let spell_id = learned.map(|(s, _)| s as u8).unwrap_or(Spell::None as u8);
                        attack_intents.push((snap.session_id, target.oid, raw, defence, false));
                        // 广播带 spell_id 的 ObjectAttack（循环外广播）
                        support_intents.push((snap.session_id, 0, spell_id, false));
                        ai_local.next_attack_tick = self.tick_count + 6; // ~600ms
                    }
                    MirClass::Assassin => {
                        // #1188：C# AssassinHero.Attack：DoubleSlash（已学）；HeavenlySword 已在距离 2 分支处理
                        let raw = hero_attack_power(&hero_combat);
                        let spell_id = if hero_magic_level(&snap.hero_magics, Spell::DoubleSlash as u8) > 0 {
                            Spell::DoubleSlash as u8
                        } else {
                            Spell::None as u8
                        };
                        attack_intents.push((snap.session_id, target.oid, raw, defence, false));
                        support_intents.push((snap.session_id, 0, spell_id, false));
                        ai_local.next_attack_tick = self.tick_count + 5;
                    }
                    MirClass::Wizard => {
                        // #1212：C# WizardHero：距离1 且目标等级<英雄等级 → Repulsion（击退）
                        let rep_lv = hero_magic_level(&snap.hero_magics, Spell::Repulsion as u8);
                        if target_dist == 1 && rep_lv > 0 && target.level < snap.hero_level as i32 {
                            let cost = hero_spell_cost(&self.magic_infos, &snap.hero_magics, Spell::Repulsion as u8);
                            if ai_local.mp >= cost {
                                ai_local.mp -= cost;
                                if hero_repulsion_succeeds(rep_lv, snap.hero_level as i32, target.level) {
                                    let dist = hero_repulsion_distance(rep_lv);
                                    let dir = direction_towards(ai_local.x, ai_local.y, target.x, target.y);
                                    push_intents.push((target.oid, dir, dist));
                                }
                                magic_anim_intents.push((snap.session_id, Spell::Repulsion as u8, target.oid));
                                ai_local.next_attack_tick = self.tick_count + 6;
                                *ai = ai_local;
                                continue;
                            }
                        }
                        // #1212：C# WizardHero：目标亡灵且已学 → TurnUndead（超度）
                        let turn_lv = hero_magic_level(&snap.hero_magics, Spell::TurnUndead as u8);
                        if target.undead && turn_lv > 0 {
                            let cost = hero_spell_cost(&self.magic_infos, &snap.hero_magics, Spell::TurnUndead as u8);
                            if ai_local.mp >= cost {
                                ai_local.mp -= cost;
                                if hero_turn_undead_kills(snap.hero_level as i32, target.level, turn_lv) {
                                    turn_undead_intents.push((snap.session_id, target.oid));
                                }
                                magic_anim_intents.push((snap.session_id, Spell::TurnUndead as u8, target.oid));
                                ai_local.next_attack_tick = self.tick_count + 8;
                                *ai = ai_local;
                                continue;
                            }
                        }
                        // #1204：C# WizardHero：自身被围（2 格内怪>1 且目标距离<3）→ FlameField/ThunderStorm（5x5 自身 AoE）
                        let monsters_xy: Vec<(u32, i32, i32)> =
                            monster_snaps.iter().map(|m| (m.oid, m.x, m.y)).collect();
                        let self_surrounded = hero_surrounded_count(&monsters_xy, ai_local.x, ai_local.y, 2) > 1
                            && target_dist < 3;
                        let storm = if self_surrounded {
                            first_learned_spell(&snap.hero_magics, &[Spell::FlameField, Spell::ThunderStorm])
                        } else {
                            None
                        };
                        if let Some((spell, level)) = storm {
                            // #1204：5x5 MAC AoE 于英雄自身位置（C# Map.cs ±2；ThunderStorm 非亡灵 /10 由 3f 处理）
                            let raw = hero_spell_damage(
                                &self.magic_infos,
                                &snap.hero_magics,
                                spell as u8,
                                &hero_stats,
                                snap.class,
                            );
                            let cost = hero_spell_cost(&self.magic_infos, &snap.hero_magics, spell as u8);
                            if ai_local.mp >= cost {
                                ai_local.mp -= cost;
                                aoe_intents.push((
                                    snap.session_id,
                                    spell as u8,
                                    target.oid,
                                    ai_local.x,
                                    ai_local.y,
                                    raw,
                                    level,
                                    2,
                                ));
                                ai_local.next_attack_tick = self.tick_count + 8;
                            } else {
                                // 蓝不足：1 格内近战兜底
                                let _ = hero_melee_fallback(
                                    snap.session_id, target.oid, target_dist,
                                    &hero_combat, &mut attack_intents, &mut support_intents,
                                );
                                ai_local.next_attack_tick = self.tick_count + 6;
                            }
                        } else {
                            // #1200：C# WizardHero：目标 1 格内有其他怪（TargetSurroundedCount>1）且已学 AoE → IceStorm/FireBang
                            let surrounded = hero_target_surrounded(
                                &monsters_xy,
                                target.oid,
                                target.x,
                                target.y,
                            );
                            let aoe = if surrounded {
                                first_learned_spell(&snap.hero_magics, &[Spell::IceStorm, Spell::FireBang])
                            } else {
                                None
                            };
                            if let Some((spell, level)) = aoe {
                                // #1200：3x3 MAC AoE（对齐 player MagicRequest FireBang/IceStorm 结算）
                                let raw = hero_spell_damage(
                                    &self.magic_infos,
                                    &snap.hero_magics,
                                    spell as u8,
                                    &hero_stats,
                                    snap.class,
                                );
                                let cost = hero_spell_cost(&self.magic_infos, &snap.hero_magics, spell as u8);
                                if ai_local.mp >= cost {
                                    ai_local.mp -= cost;
                                    aoe_intents.push((
                                        snap.session_id,
                                        spell as u8,
                                        target.oid,
                                        target.x,
                                        target.y,
                                        raw,
                                        level,
                                        1,
                                    ));
                                    ai_local.next_attack_tick = self.tick_count + 8;
                                } else {
                                    // 蓝不足：1 格内近战兜底
                                    let _ = hero_melee_fallback(
                                        snap.session_id, target.oid, target_dist,
                                        &hero_combat, &mut attack_intents, &mut support_intents,
                                    );
                                    ai_local.next_attack_tick = self.tick_count + 6;
                                }
                            } else {
                                // #1188：C# WizardHero 单体弹道优先级：FlameDisruptor → Vampirism → FrostCrunch → ThunderBolt → GreatFireBall → FireBall
                                let learned = first_learned_spell(
                                    &snap.hero_magics,
                                    &[
                                        Spell::FlameDisruptor,
                                        Spell::Vampirism,
                                        Spell::FrostCrunch,
                                        Spell::ThunderBolt,
                                        Spell::GreatFireBall,
                                        Spell::FireBall,
                                    ],
                                );
                                match learned {
                                    Some((spell, level)) => {
                                        // #1188：伤害 = C# GetDamage（英雄自身 MC + 魔法表 Power/Multiplier × 实际等级）
                                        let raw = hero_spell_damage(
                                            &self.magic_infos,
                                            &snap.hero_magics,
                                            spell as u8,
                                            &hero_stats,
                                            snap.class,
                                        );
                                        // #1186：耗蓝（C# CanUseMagic：MagicCost > MP → 无法施法）
                                        let cost = hero_spell_cost(&self.magic_infos, &snap.hero_magics, spell as u8);
                                        if ai_local.mp >= cost {
                                            ai_local.mp -= cost;
                                            spell_intents.push((
                                                snap.session_id,
                                                spell as u8,
                                                target.oid,
                                                target.x,
                                                target.y,
                                                raw,
                                                self.tick_count + 4, // 弹道延迟 ~400ms
                                                level,
                                            ));
                                            ai_local.next_attack_tick = self.tick_count + 8;
                                        } else {
                                            // 蓝不足：1 格内近战兜底（C# WizardHero 无蓝时 ProcessTarget 退避/近战）
                                            let _ = hero_melee_fallback(
                                                snap.session_id, target.oid, target_dist,
                                                &hero_combat, &mut attack_intents, &mut support_intents,
                                            );
                                            ai_local.next_attack_tick = self.tick_count + 6;
                                        }
                                    }
                                    None => {
                                        // 未学任何弹道技能：近战兜底
                                        let _ = hero_melee_fallback(
                                            snap.session_id, target.oid, target_dist,
                                            &hero_combat, &mut attack_intents, &mut support_intents,
                                        );
                                        ai_local.next_attack_tick = self.tick_count + 6;
                                    }
                                }
                            }
                        }
                    }
                    MirClass::Taoist => {
                        // #1210：净化/治疗/护盾已由常驻 ProcessFriend 预置块处理；攻击顺序 Poisoning→Curse→SoulFireBall→近战
                        let soulfire_lv = hero_magic_level(&snap.hero_magics, Spell::SoulFireBall as u8);
                        // #1192：Poisoning 可用 = 已学 + 装备毒护符 + 目标无对应毒（C# TaoistHero）
                        let poisoning_lv = hero_magic_level(&snap.hero_magics, Spell::Poisoning as u8);
                        let can_poison = snap.hero_poison_shape > 0
                            && poisoning_lv > 0
                            && if snap.hero_poison_shape == 1 {
                                !target.has_green
                            } else {
                                !target.has_red
                            };
                        // #1196：Curse 可用 = 已学 + 普通护符 + 目标无减速毒（C# TaoistHero 无 Curse buff 近似）
                        let curse_lv = hero_magic_level(&snap.hero_magics, Spell::Curse as u8);
                        let can_curse = curse_lv > 0 && snap.hero_amulet && !target.has_slow;
                        if can_poison {

                            // #1192：C# Poisoning：value = GetDamage(SC)；Duration=value*2+(Lv+1)*7、TickSpeed 2000
                            // 绿毒 Value = value/15 + Lv + 1（+Random PoisonAttack 近似省略）；红毒无伤害值（状态毒）
                            let cost = hero_spell_cost(&self.magic_infos, &snap.hero_magics, Spell::Poisoning as u8);
                            if ai_local.mp >= cost {
                                ai_local.mp -= cost;
                                let value = hero_attack_power_sc(&hero_stats).max(1);
                                let duration = (value * 2 + (poisoning_lv as i32 + 1) * 7).max(1) as u32;
                                let poison_value = if snap.hero_poison_shape == 1 {
                                    (value / 15 + poisoning_lv as i32 + 1).max(1)
                                } else {
                                    0 // C# Red 毒无 Value（状态毒）
                                };
                                let p_type = if snap.hero_poison_shape == 1 {
                                    mir2_shared::enums::PoisonType::GREEN
                                } else {
                                    mir2_shared::enums::PoisonType::RED
                                };
                                poison_intents.push((snap.session_id, target.oid, p_type, duration, poison_value, 2000));
                                support_intents.push((
                                    snap.session_id,
                                    snap.session_id,
                                    Spell::Poisoning as u8,
                                    false,
                                ));
                                // #1208：施毒广播 ObjectMagic（目标 = 怪物）
                                magic_anim_intents.push((snap.session_id, Spell::Poisoning as u8, target.oid));
                                ai_local.next_attack_tick = self.tick_count + 10;
                            } else {
                                let _ = hero_melee_fallback(
                                    snap.session_id, target.oid, target_dist,
                                    &hero_combat, &mut attack_intents, &mut support_intents,
                                );
                                ai_local.next_attack_tick = self.tick_count + 10;
                            }
                        } else if can_curse {
                            // #1196：C# TaoistHero Curse：护符 + 目标无 Curse（本服怪物无 buff，实现 Slow 毒部分）
                            let cost = hero_spell_cost(&self.magic_infos, &snap.hero_magics, Spell::Curse as u8);
                            if ai_local.mp >= cost {
                                ai_local.mp -= cost;
                                support_intents.push((
                                    snap.session_id,
                                    snap.session_id,
                                    Spell::Curse as u8,
                                    false,
                                ));
                                // #1208：诅咒广播 ObjectMagic（目标 = 怪物）
                                magic_anim_intents.push((snap.session_id, Spell::Curse as u8, target.oid));
                                // C# Map.cs：40% 概率附加 Slow 毒（Duration=1+(Lv+1)*2、TickSpeed 1000、Value=GetDamage(SC)）
                                if fastrand::i32(0..10) < 4 {
                                    let value = hero_spell_damage(
                                        &self.magic_infos,
                                        &snap.hero_magics,
                                        Spell::Curse as u8,
                                        &hero_stats,
                                        snap.class,
                                    )
                                    .max(1);
                                    let (cdur, cval) = hero_curse_slow(curse_lv, value);
                                    poison_intents.push((
                                        snap.session_id,
                                        target.oid,
                                        mir2_shared::enums::PoisonType::SLOW,
                                        cdur,
                                        cval,
                                        1000,
                                    ));
                                }
                                ai_local.next_attack_tick = self.tick_count + 10;
                            } else {
                                let _ = hero_melee_fallback(
                                    snap.session_id, target.oid, target_dist,
                                    &hero_combat, &mut attack_intents, &mut support_intents,
                                );
                                ai_local.next_attack_tick = self.tick_count + 10;
                            }
                        } else if soulfire_lv > 0 {
                            // #1184/#1188：道士符伤害用英雄自身 SC + 魔法表加成
                            let raw = hero_spell_damage(
                                &self.magic_infos,
                                &snap.hero_magics,
                                Spell::SoulFireBall as u8,
                                &hero_stats,
                                snap.class,
                            );
                            let cost = hero_spell_cost(&self.magic_infos, &snap.hero_magics, Spell::SoulFireBall as u8);
                            if ai_local.mp >= cost {
                                ai_local.mp -= cost;
                                spell_intents.push((
                                    snap.session_id,
                                    Spell::SoulFireBall as u8,
                                    target.oid,
                                    target.x,
                                    target.y,
                                    raw,
                                    self.tick_count + 4,
                                    soulfire_lv,
                                ));
                                ai_local.next_attack_tick = self.tick_count + 10;
                            } else {
                                let _ = hero_melee_fallback(
                                    snap.session_id, target.oid, target_dist,
                                    &hero_combat, &mut attack_intents, &mut support_intents,
                                );
                                ai_local.next_attack_tick = self.tick_count + 10;
                            }
                        } else {
                            let _ = hero_melee_fallback(
                                snap.session_id, target.oid, target_dist,
                                &hero_combat, &mut attack_intents, &mut support_intents,
                            );
                            ai_local.next_attack_tick = self.tick_count + 10;
                        }
                    }
                    MirClass::Archer => {
                        // #1194：C# ArcherHero：PoisonShot（已学+目标无绿毒+无 buff）→ StraightShot（MC/MAC）→ 近战
                        let poison_lv = hero_magic_level(&snap.hero_magics, Spell::PoisonShot as u8);
                        let straight_lv = hero_magic_level(&snap.hero_magics, Spell::StraightShot as u8);
                        let has_poison_buff = ai_local.buffs.iter().any(|b| b.kind == HeroBuffKind::PoisonShot);
                        if poison_lv > 0 && !target.has_green && !has_poison_buff {
                            // #1194：C# SpecialArrowShot：PoisonShot 魔法箭（MC 伤害/MAC 防御）
                            let raw = hero_spell_damage(
                                &self.magic_infos,
                                &snap.hero_magics,
                                Spell::PoisonShot as u8,
                                &hero_stats,
                                snap.class,
                            );
                            let cost = hero_spell_cost(&self.magic_infos, &snap.hero_magics, Spell::PoisonShot as u8);
                            if ai_local.mp >= cost {
                                ai_local.mp -= cost;
                                spell_intents.push((
                                    snap.session_id,
                                    Spell::PoisonShot as u8,
                                    target.oid,
                                    target.x,
                                    target.y,
                                    raw,
                                    self.tick_count + 4,
                                    poison_lv,
                                ));
                                // C# SpecialArrowShot：40% 概率附加 PoisonShot buff（5+5*Lv 秒）
                                if fastrand::i32(0..20) >= 8 {
                                    ai_local.buffs.push(HeroBuff {
                                        kind: HeroBuffKind::PoisonShot,
                                        expire_tick: self.tick_count
                                            + hero_buff_duration(HeroBuffKind::PoisonShot, poison_lv, &snap.hero_stats)
                                                * 10,
                                        level: poison_lv,
                                        hero_level: snap.hero_level,
                                    });
                                }
                                ai_local.next_attack_tick = self.tick_count + 8;
                            } else {
                                let _ = hero_melee_fallback(
                                    snap.session_id, target.oid, target_dist,
                                    &hero_combat, &mut attack_intents, &mut support_intents,
                                );
                                ai_local.next_attack_tick = self.tick_count + 6;
                            }
                        } else if straight_lv > 0 {
                            // #1194：C# StraightShot 用 MC 伤害（原 DC 为误对齐）
                            let raw = hero_spell_damage(
                                &self.magic_infos,
                                &snap.hero_magics,
                                Spell::StraightShot as u8,
                                &hero_stats,
                                snap.class,
                            );
                            let cost = hero_spell_cost(&self.magic_infos, &snap.hero_magics, Spell::StraightShot as u8);
                            if ai_local.mp >= cost {
                                ai_local.mp -= cost;
                                spell_intents.push((
                                    snap.session_id,
                                    Spell::StraightShot as u8,
                                    target.oid,
                                    target.x,
                                    target.y,
                                    raw,
                                    self.tick_count + 4,
                                    straight_lv,
                                ));
                                // #1194：PoisonShot buff 生效时本次射击附加绿毒（C# CompleteRangeAttack 取消 buff 并 ApplyPoison）
                                if has_poison_buff {
                                    let duration = (raw * 2 + (straight_lv as i32 + 1) * 7).max(1) as u32;
                                    let pv = (raw / 25 + straight_lv as i32 + 1).max(1);
                                    poison_intents.push((
                                        snap.session_id,
                                        target.oid,
                                        mir2_shared::enums::PoisonType::GREEN,
                                        duration,
                                        pv,
                                        2000,
                                    ));
                                    ai_local.buffs.retain(|b| b.kind != HeroBuffKind::PoisonShot);
                                }
                                ai_local.next_attack_tick = self.tick_count + 7;
                            } else {
                                let _ = hero_melee_fallback(
                                    snap.session_id, target.oid, target_dist,
                                    &hero_combat, &mut attack_intents, &mut support_intents,
                                );
                                ai_local.next_attack_tick = self.tick_count + 6;
                            }
                        } else {
                            let _ = hero_melee_fallback(
                                snap.session_id, target.oid, target_dist,
                                &hero_combat, &mut attack_intents, &mut support_intents,
                            );
                            ai_local.next_attack_tick = self.tick_count + 6;
                        }
                    }
                }
                // 战斗时英雄 HP 模拟损耗（敌人反击的近似，#1134 增强到可感知）
                let mut counter = (target.max_hp / 10).max(5);
                // #1190：MagicShield 减伤（C# DamageReductionPercent = (Lv+2)*10）
                if shield_pct > 0 {
                    counter = counter * (100 - shield_pct) / 100;
                }
                ai_local.hp = ai_local.hp.saturating_sub(counter / 3);
                // #1190：Haste 缩短攻击冷却（C# AttackSpeed = Lv*2+2）
                if haste_ticks > 0 {
                    ai_local.next_attack_tick = ai_local
                        .next_attack_tick
                        .saturating_sub(haste_ticks)
                        .max(self.tick_count + 2);
                }

            } else if target_dist > attack_range && can_move {
                // ===== 不在攻击范围：移动靠近目标（ProcessTarget.MoveTo） =====
                let (nx, ny, dir) = step_towards(ai_local.x, ai_local.y, target.x, target.y);
                if self.maps.get(&snap.owner_map).map(|m| m.is_walkable(nx, ny)).unwrap_or(true) {
                    ai_local.x = nx;
                    ai_local.y = ny;
                    ai_local.direction = dir;
                    ai_local.next_move_tick = self.tick_count + 2;
                    move_intents.push((snap.session_id, nx, ny, dir));
                }
            }

            *ai = ai_local;
        }

        // ===== 阶段 2.4：自动喝药（C# HeroObject.ProcessAutoPot → TryAutoPot → UseItem） =====
        // 每 HERO_AUTOPOT_INTERVAL_TICKS（约 1s）检查一次；仅在无待回复量且百分比低于阈值时喝 1 瓶。
        // shape 0 NormalPotion：PotHealthAmount/PotManaAmount += Stats（每 AI tick 回复 PerTickRegen）；
        // shape 1 SunPotion：立即回血回蓝（C# ChangeHP/ChangeMP）。
        for (session_id, item_index, is_mp) in &autopot_intents {
            let Some(record) = self.players.get(session_id).map(|r| r.clone()) else {
                continue;
            };
            // TryAutoPot：英雄背包里找第一个同 item_index 的药水
            let potion = record
                .actor_ref
                .ask(crate::actors::player::GetHeroPotionByItemIndex {
                    item_index: *item_index,
                })
                .await
                .unwrap_or(None);
            let Some(potion) = potion else { continue };
            let Some(db) = self.item_infos.get(&potion.item_index) else {
                continue;
            };
            // DB item_type 为 C# 原始值：13=Potion
            if db.item_type != 13 {
                continue;
            }
            let hp_pool = db
                .stats
                .get(&(mir2_shared::enums::Stat::HP as u8))
                .copied()
                .unwrap_or(0) as u32;
            let mp_pool = db
                .stats
                .get(&(mir2_shared::enums::Stat::MP as u8))
                .copied()
                .unwrap_or(0) as u32;
            // 目标维度必须有效（HP 意图看 HP、MP 意图看 MP）
            let valid = if *is_mp { mp_pool > 0 } else { hp_pool > 0 };
            if !valid {
                continue;
            }
            let consumed = record
                .actor_ref
                .ask(crate::actors::player::ConsumeHeroItem {
                    unique_id: potion.unique_id,
                })
                .await
                .unwrap_or(false);
            if !consumed {
                continue;
            }
            if let Some(ai) = self.hero_ai_states.get_mut(session_id) {
                if db.shape == 0 {
                    // NormalPotion：累计持续回复（C# PotHealthAmount/PotManaAmount += Stats，min ushort::MAX）
                    if hp_pool > 0 {
                        ai.pot_health = (ai.pot_health + hp_pool).min(u16::MAX as u32);
                    }
                    if mp_pool > 0 {
                        ai.pot_mana = (ai.pot_mana + mp_pool).min(u16::MAX as u32);
                    }
                } else if db.shape == 1 {
                    // SunPotion：立即回血回蓝（C# ChangeHP/ChangeMP）
                    if hp_pool > 0 && ai.hp > 0 {
                        ai.hp = (ai.hp + hp_pool as i32).min(ai.max_hp);
                    }
                    if mp_pool > 0 && ai.mp > 0 {
                        ai.mp = (ai.mp + mp_pool as i32).min(ai.max_mp);
                    }
                }
            }
            // 刷新英雄背包 UI（消耗后的数量）
            self.send_hero_information_packet(*session_id).await;
            debug!(
                "Hero auto-pot: session={} item_index={} uid={} is_mp={} shape={} hp_pool={} mp_pool={}",
                session_id, item_index, potion.unique_id, is_mp, db.shape, hp_pool, mp_pool
            );
        }

        // ===== 阶段 2.4b：道士英雄给主人上护盾（#1202：C# TaoistHero ProcessFriend 目标含 Owner） =====
        for (session_id, kind) in &owner_shield_intents {
            let Some(record) = self.players.get(session_id).map(|r| r.clone()) else {
                continue;
            };
            let Some(snap) = snapshots.iter().find(|s| s.session_id == *session_id) else {
                continue;
            };
            let spell = hero_owner_shield_spell(*kind);
            let buff_lv = hero_magic_level(&snap.hero_magics, spell as u8);
            let (buff_type, ticks) =
                hero_owner_shield_buff(*kind, snap.class, snap.owner_level, buff_lv, &snap.hero_stats);
            let _ = record
                .actor_ref
                .ask(crate::actors::player::ApplyBuff {
                    buff: crate::combat::buff::BuffInstance::new(buff_type, ticks, 1),
                })
                .await;
            debug!(
                "Hero Taoist cast {:?} on owner {} ({} ticks)",
                kind, session_id, ticks
            );
        }

        // ===== 阶段 2.4c：道士英雄净化主人（#1210：C# Purification → PurifyPoisons） =====
        for session_id in &purify_intents {
            if let Some(record) = self.players.get(session_id).map(|r| r.clone()) {
                let _ = record.actor_ref.ask(crate::actors::player::PurifyPoisons).await;
                debug!("Hero Taoist purified owner {}", session_id);
            }
        }

        // ===== 阶段 2.5：英雄 HP/MP 实时同步 + 阵亡处理（#1134/#1186） =====
        for snap in &snapshots {
            let (hp, mp) = match self.hero_ai_states.get(&snap.session_id) {
                Some(ai) => (ai.hp.max(0), ai.mp.max(0)),
                None => continue,
            };
            if hp <= 0 {
                // 阵亡：标记死亡 + 移除英雄对象 + 下发 HP=0（REVIVEHERO 复用现有复活）
                self.hero_die(snap.session_id).await;
                continue;
            }
            let last_hp = self
                .hero_ai_states
                .get(&snap.session_id)
                .map(|ai| ai.last_sent_hp)
                .unwrap_or(hp);
            let last_mp = self
                .hero_ai_states
                .get(&snap.session_id)
                .map(|ai| ai.last_sent_mp)
                .unwrap_or(mp);
            if hp == last_hp && mp == last_mp {
                continue;
            }
            // 下发 S.HeroHealthChanged（C# HeroObject.SendHealthChanged → Owner.Enqueue）
            let packet = mir2_shared::packets::server::combat::HeroHealthChanged {
                hp: hp as u32,
                mp: mp as u32,
            };
            let mut body = Vec::new();
            if packet.write_body(&mut body).is_ok() {
                let _ = self.gate_ref.tell(SendToClient {
                    session_id: snap.session_id,
                    data: build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::HeroHealthChanged as i16,
                        &body,
                    ),
                }).await;
            }
            if let Some(ai) = self.hero_ai_states.get_mut(&snap.session_id) {
                ai.last_sent_hp = hp;
                ai.last_sent_mp = mp;
            }
            // #1141：英雄头顶血条（C# S.ObjectHealth：percent + expire 秒，客户端挂 ActorHp）
            if let Some(record) = self.players.get(&snap.session_id) {
                let hero_oid = record.object_id.wrapping_add(HERO_OID_OFFSET);
                let max_hp = self.hero_ai_states.get(&snap.session_id).map(|ai| ai.max_hp).unwrap_or(hp);
                let percent = (hp * 100 / max_hp.max(1)).min(100) as u8;
                let ohealth = mir2_shared::packets::server::object::ObjectHealth {
                    object_id: hero_oid,
                    percent,
                    expire: 3,
                };
                let mut ohealth_body = Vec::new();
                if ohealth.write_body(&mut ohealth_body).is_ok() {
                    let data = build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::ObjectHealth as i16,
                        &ohealth_body,
                    );
                    for sid in self.players.keys() {
                        let _ = self.gate_ref.tell(SendToClient {
                            session_id: *sid,
                            data: data.clone(),
                        }).await;
                    }
                }
            }
            debug!("Hero HP changed: session={} hp={}", snap.session_id, hp);
        }

        // ===== 阶段 3：循环外应用意图（避免借用冲突） =====

        // 3a. 广播英雄移动（ObjectWalk）给所有在线玩家
        //     （英雄用虚拟 object_id = owner_oid + HERO_OID_OFFSET 以区分于主人）
        for (session_id, nx, ny, dir) in &move_intents {
            let owner_oid = match self.players.get(session_id).map(|r| r.object_id) {
                Some(oid) => oid,
                None => continue,
            };
            let hero_oid = owner_oid.wrapping_add(HERO_OID_OFFSET);
            let mut walk_body = Vec::new();
            walk_body.extend_from_slice(&hero_oid.to_le_bytes());
            walk_body.extend_from_slice(&(*nx as u32).to_le_bytes());
            walk_body.extend_from_slice(&(*ny as u32).to_le_bytes());
            walk_body.push(*dir);
            let walk_packet = build_packet_bytes(
                mir2_shared::enums::ServerPacketIds::ObjectWalk as i16,
                &walk_body,
            );
            // 简化：广播给所有在线玩家（单地图运行环境下足够）
            for sid in self.players.keys() {
                let _ = self.gate_ref.tell(SendToClient {
                    session_id: *sid,
                    data: walk_packet.clone(),
                }).await;
            }
        }

        // 3b. 应用物理攻击（近战/远程 AC）—— 走 combat_attack::resolve_attack
        for (session_id, target_oid, raw_damage, defence, _is_ranged) in &attack_intents {
            let attacker_stats = match snapshots.iter().find(|s| s.session_id == *session_id) {
                Some(s) => s.hero_combat,
                None => continue,
            };
            let level_offset = snapshots.iter()
                .find(|s| s.session_id == *session_id)
                .map(|s| s.hero_level.min(10) as u16)
                .unwrap_or(0);

            let defender_stats = match self.monsters.get(target_oid) {
                Some(m) => m.to_combat_stats(),
                None => continue,
            };
            let result = combat_attack::resolve_attack(
                &attacker_stats, &defender_stats, *raw_damage, *defence, level_offset,
            );
            if result.is_hit && result.damage > 0 {
                if let Some(monster) = self.monsters.get_mut(target_oid) {
                    monster.hp = monster.hp.saturating_sub(result.damage);
                    monster.provoked = true;
                    // 英雄攻击的怪物仇恨转移到主人（英雄本身不可被攻击）
                    monster.target_session = Some(*session_id);
                    // #1163：英雄伤害同样记 LastHitter（C# MapObject.LastHitter）——
                    // tick_heroes 在死亡处理之后运行，仅靠 target_session 会在下一 tick
                    // 怪物循环 hp<=0 时被清掉，导致主人/英雄拿不到击杀经验归属
                    monster.last_hitter_session = Some(*session_id);
                    debug!(
                        "Hero (owner {}) attacked monster '{}' (#{}) for {} dmg [hit={}, crit={}]",
                        session_id, monster.name, target_oid, result.damage, result.is_hit, result.is_critical
                    );
                }
            }
        }

        // 3c. 应用弹道法术（法师/道士/弓箭手）：直接 push 到 pending_spell_completions
        //     由 tick_spell_completions 在后续 tick 结算（复用现有弹道管线）
        for (session_id, spell, target_oid, tx, ty, damage, fire_at_tick, level) in &spell_intents {
            // #1206：广播 S.ObjectMagic（C# Magic → 客户端渲染弹道特效）
            broadcast_hero_magic(self, *session_id, *spell, *target_oid, *tx, *ty, *level).await;
            // #1184/#1196：英雄弹道用英雄自身（含增益）属性结算（LightBody 命中/MagicBooster MC 等生效）
            let hero_snap = snapshots.iter().find(|s| s.session_id == *session_id);
            let (buffed_combat, buffed_stats) = if let Some(s) = hero_snap {
                let mut combat = s.hero_combat;
                let mut stats = s.hero_stats;
                if let Some(ai) = self.hero_ai_states.get(&s.session_id) {
                    hero_apply_buffs(&ai.buffs, s.class, &mut combat, &mut stats);
                }
                (combat, stats)
            } else {
                (
                    crate::combat::attack::CombatStats::default(),
                    super::hero_stats::HeroStats::default(),
                )
            };
            self.pending_spell_completions.push(PendingSpellCompletion {
                fire_at_tick: *fire_at_tick,
                session_id: *session_id,
                spell: *spell,
                target_id: *target_oid,
                target_x: *tx,
                target_y: *ty,
                damage: *damage,
                // #1188/#1196：magic_stat 用英雄自身（含增益）魔法表伤害（Vampirism 等吸血/附加用）
                magic_stat: hero_snap
                    .map(|s| {
                        hero_spell_damage(
                            &self.magic_infos,
                            &s.hero_magics,
                            *spell,
                            &buffed_stats,
                            s.class,
                        )
                    })
                    .unwrap_or(10),
                hero_stats: Some(buffed_combat),
                hero_level: hero_snap.map(|s| s.hero_level),
                // #1188：施放等级用英雄已学实际等级（影响法术附加/经验/效果）
                spell_level: *level,
                bounce: 0,
            });
            // #220：英雄施法技能经验（spell 为 SharedRust 值，升级发 S.MagicLeveled）
            if let Some(record) = self.players.get(session_id) {
                if let Some((spell_enum, level, experience)) = record
                    .actor_ref
                    .ask(crate::actors::player::GainHeroSpellExp {
                        spell_shared: *spell,
                        amount: 1,
                    })
                    .await
                    .unwrap_or(None)
                {
                    let hero_oid = record.object_id.wrapping_add(HERO_OID_OFFSET);
                    let leveled = mir2_shared::packets::server::magic::MagicLeveled {
                        object_id: hero_oid,
                        spell: spell_enum,
                        level,
                        experience,
                    };
                    let mut body = Vec::new();
                    if leveled.write_body(&mut body).is_ok() {
                        let _ = self
                            .gate_ref
                            .tell(SendToClient {
                                session_id: *session_id,
                                data: build_packet_bytes(
                                    mir2_shared::enums::ServerPacketIds::MagicLeveled as i16,
                                    &body,
                                ),
                            })
                            .await;
                    }
                    debug!("Hero magic leveled: spell={:?} -> Lv.{}", spell_enum, level);
                }
            }
        }

        // 3d. 广播近战 ObjectAttack（带 spell_id）+ 道士治疗
        for (session_id, heal_target_session, spell_id, is_heal) in &support_intents {
            if *is_heal {
                // 道士治疗主人：直接 Heal（#1184：C# Healing = GetAttackPower(MinSC,MaxSC)*2 + Level）
                if let Some(record) = self.players.get(heal_target_session) {
                    let amount = snapshots.iter()
                        .find(|s| s.session_id == *session_id)
                        .map(|s| {
                            // #1214：MassHealing 用群疗量（C# GetDamage(SC)），单疗用原公式
                            if *spell_id == mir2_shared::enums::Spell::MassHealing as u8 {
                                hero_mass_heal_amount(&self.magic_infos, &s.hero_magics, &s.hero_stats, s.class)
                            } else {
                                hero_heal_amount(&s.hero_stats, s.hero_level)
                            }
                        })
                        .unwrap_or(5);
                    let _ = record.actor_ref.ask(crate::actors::player::Heal { amount }).await;
                    debug!("Hero healed owner {} for {} HP", heal_target_session, amount);
                }
            }
            // #1208：支持类法术改由 3g 广播 ObjectMagic；此处仅广播物理技能（Attack → S.ObjectAttack）
            if hero_support_spell_is_magic(*spell_id) {
                continue;
            }
            let ai = match self.hero_ai_states.get(session_id) {
                Some(a) => a.clone(),
                None => continue,
            };
            let owner_oid = match self.players.get(session_id).map(|r| r.object_id) {
                Some(oid) => oid,
                None => continue,
            };
            let hero_oid = owner_oid.wrapping_add(HERO_OID_OFFSET);
            let mut attack_body = Vec::new();
            attack_body.extend_from_slice(&hero_oid.to_le_bytes());
            attack_body.extend_from_slice(&(ai.x as u32).to_le_bytes());
            attack_body.extend_from_slice(&(ai.y as u32).to_le_bytes());
            attack_body.push(ai.direction);
            attack_body.push(*spell_id);
            attack_body.extend_from_slice(&0u16.to_le_bytes());
            attack_body.push(0u8);
            let attack_packet = build_packet_bytes(
                mir2_shared::enums::ServerPacketIds::ObjectAttack as i16,
                &attack_body,
            );
            for sid in self.players.keys() {
                let _ = self.gate_ref.tell(SendToClient {
                    session_id: *sid,
                    data: attack_packet.clone(),
                }).await;
            }
        }

        // 3e. 应用毒意图（#1192/#1196：C# ApplyPoison；道士 Poisoning/弓箭手毒箭 TickSpeed 2000、道士 Curse 1000）
        for (session_id, target_oid, p_type, duration, value, tick_ms) in &poison_intents {
            if let Some(monster) = self.monsters.get_mut(target_oid) {
                crate::combat::poison::apply_poison(
                    &mut monster.poison_list,
                    crate::combat::poison::Poison::new(*p_type, *duration, *value, *tick_ms),
                );
                monster.provoked = true;
                monster.last_hitter_session = Some(*session_id);
                if monster.target_session.is_none() {
                    monster.target_session = Some(*session_id);
                }
                debug!(
                    "Hero poisoned monster {} ({:?} {}s {}dmg/tick {}ms)",
                    target_oid, p_type, duration, value, tick_ms
                );
            }
        }

        // 3f. 应用法师英雄 AoE（#1200/#1204：C# FireBang/IceStorm 3x3、FlameField/ThunderStorm 5x5，MAC）
        for (session_id, spell, target_oid, tx, ty, raw, level, radius) in &aoe_intents {
            // #1206：广播 S.ObjectMagic（AoE 施放动画，目标位置）
            broadcast_hero_magic(self, *session_id, *spell, *target_oid, *tx, *ty, *level).await;
            // 用 buff 后英雄属性结算（命中/暴击）
            let hero_snap = snapshots.iter().find(|s| s.session_id == *session_id);
            let attacker_stats = if let Some(s) = hero_snap {
                let mut combat = s.hero_combat;
                let mut stats = s.hero_stats;
                if let Some(ai) = self.hero_ai_states.get(&s.session_id) {
                    hero_apply_buffs(&ai.buffs, s.class, &mut combat, &mut stats);
                }
                combat
            } else {
                continue;
            };
            let level_offset = hero_snap.map(|s| s.hero_level.min(10) as u16).unwrap_or(0);
            let hit_ids: Vec<u32> = self
                .monsters
                .iter()
                .filter(|(_, m)| {
                    m.hp > 0 && (m.x - *tx).abs() <= *radius && (m.y - *ty).abs() <= *radius
                })
                .map(|(id, _)| *id)
                .collect();
            for mid in &hit_ids {
                if let Some(monster) = self.monsters.get_mut(mid) {
                    // #1204：ThunderStorm 对非亡灵伤害 /10（C# Map.cs）
                    let dmg = if *spell == mir2_shared::enums::Spell::ThunderStorm as u8 && !monster.undead {
                        (*raw / 10).max(1)
                    } else {
                        *raw
                    };
                    let ds = monster.to_combat_stats();
                    let r = combat_attack::resolve_attack(
                        &attacker_stats,
                        &ds,
                        dmg,
                        mir2_shared::enums::DefenceType::Mac,
                        level_offset,
                    );
                    if r.is_hit && r.damage > 0 {
                        monster.hp = monster.hp.saturating_sub(r.damage);
                        monster.last_hitter_session = Some(*session_id);
                        monster.provoked = true;
                        if monster.target_session.is_none() {
                            monster.target_session = Some(*session_id);
                        }
                        for p in &r.applied_poisons {
                            crate::combat::poison::apply_poison(&mut monster.poison_list, *p);
                        }
                    }
                }
            }
            debug!(
                "Hero Wizard AoE spell={} at ({},{}) dmg={} hits={}",
                spell, tx, ty, raw, hit_ids.len()
            );
        }

        // 3g. 支持类法术（增益/治疗/毒/诅咒）广播 S.ObjectMagic（#1208：C# Magic → S.ObjectMagic）
        for (session_id, spell, target_oid) in &magic_anim_intents {
            let level = snapshots
                .iter()
                .find(|s| s.session_id == *session_id)
                .map(|s| hero_magic_level(&s.hero_magics, *spell))
                .unwrap_or(1);
            let (hx, hy) = self
                .hero_ai_states
                .get(session_id)
                .map(|a| (a.x, a.y))
                .unwrap_or((0, 0));
            broadcast_hero_magic(self, *session_id, *spell, *target_oid, hx, hy, level).await;
        }

        // 3h. 法师英雄 Repulsion 击退 + TurnUndead 超度亡灵（#1212）
        for (oid, dir, dist) in &push_intents {
            let _ = self.push_monster(*oid, *dir, *dist).await;
            debug!("Hero Repulsion pushed monster {} ({} tiles)", oid, dist);
        }
        for (session_id, oid) in &turn_undead_intents {
            if let Some(monster) = self.monsters.get_mut(oid) {
                // C# TurnUndead 成功：直接击杀亡灵（死亡处理由怪物 tick 按 hp<=0 + LastHitter 结算）
                monster.hp = 0;
                monster.last_hitter_session = Some(*session_id);
                monster.provoked = true;
                debug!("Hero TurnUndead killed undead monster {}", oid);
            }
        }
    }

    /// 英雄阵亡处理（#1134，对齐 C# HeroObject.Die 的最小实现）：
    /// 标记死亡（DB 持久化）+ 移除英雄对象 + 下发 S.HeroHealthChanged(0) + 系统消息。
    /// 复活复用现有 REVIVEHERO（npc.rs 已按 hero.dead / AI HP<=0 判定）。
    async fn hero_die(&mut self, session_id: u64) {
        let Some(record) = self.players.get(&session_id).cloned() else { return };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        // 标记当前出战英雄死亡（REVIVEHERO 判定 + DB 持久化）
        let mut died = false;
        if let Some(hs) = self.player_heroes.get_mut(&session_id) {
            if let Some(h) = hs.iter_mut().find(|h| h.index as u8 == state.hero_index) {
                h.dead = true;
                died = true;
            }
        }
        if died {
            let db_heroes: Vec<db::DbHero> = self.player_heroes.get(&session_id)
                .map(|hs| hs.iter().map(|h| db::DbHero {
                    index: h.index, name: h.name.clone(), level: h.level,
                    class: h.class as u8, gender: h.gender as u8,
                    dead: h.dead, sealed: h.sealed,
                }).collect())
                .unwrap_or_default();
            if let Err(e) = db::save_heroes(&self.db_pool, &state.name, &db_heroes).await {
                warn!("Failed to save heroes on hero death: {}", e);
            }
        }
        // 移除英雄对象（客户端消失）
        self.broadcast_hero_remove(record.object_id).await;
        // 下发 HP=0（客户端面板归零）
        let packet = mir2_shared::packets::server::combat::HeroHealthChanged { hp: 0, mp: 0 };
        let mut body = Vec::new();
        if packet.write_body(&mut body).is_ok() {
            let _ = self.gate_ref.tell(SendToClient {
                session_id,
                data: build_packet_bytes(
                    mir2_shared::enums::ServerPacketIds::HeroHealthChanged as i16,
                    &body,
                ),
            }).await;
        }
        send_system_message(&self.gate_ref, session_id, "英雄已阵亡，请找 NPC 复活（REVIVEHERO）");
        info!("Hero died: session={} hero_index={}", session_id, state.hero_index);
    }

    /// 英雄经验发放（#1142/#1163，对齐 C# HeroObject.GainExp/LevelUp）：
    /// - 累加经验并发 S.GainHeroExperience{Amount}
    /// - 满级经验升级：level+1、max_exp ×1.5、发 S.HeroLevelChanged、DB 持久化等级、广播对象刷新
    pub(crate) async fn grant_hero_experience(&mut self, session_id: u64, amount: u32) {
        if amount == 0 {
            return;
        }
        let Some(record) = self.players.get(&session_id).cloned() else { return };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        if state.hero_index == 0 || state.is_dead {
            return;
        }
        let Some(hero) = self.player_heroes.get_mut(&session_id)
            .and_then(|hs| hs.iter_mut().find(|h| h.index as u8 == state.hero_index))
        else {
            return;
        };
        if hero.dead {
            return;
        }
        hero.experience = hero.experience.saturating_add(amount);
        let mut leveled = false;
        while hero.experience >= hero.max_experience && hero.level < u16::MAX {
            hero.experience -= hero.max_experience;
            hero.level += 1;
            // #1180：C# HeroExpList 默认每级 100（Settings.HeroExpList[Level-1]）
            hero.max_experience = super::hero_stats::HERO_MAX_EXPERIENCE;
            leveled = true;
        }
        let hero_name = hero.name.clone();
        let hero_level = hero.level;
        let hero_exp = hero.experience;
        let hero_max = hero.max_experience;
        drop(hero); // 释放 player_heroes 借用后再用 self

        // S.GainHeroExperience（C# Hero.GainExp → Owner.Enqueue）
        let pkt = mir2_shared::packets::server::experience::GainHeroExperience { amount };
        let mut body = Vec::new();
        if pkt.write_body(&mut body).is_ok() {
            let _ = self.gate_ref.tell(SendToClient {
                session_id,
                data: build_packet_bytes(
                    mir2_shared::enums::ServerPacketIds::GainHeroExperience as i16,
                    &body,
                ),
            }).await;
        }
        if leveled {
            // S.HeroLevelChanged（C# Hero.LevelUp → Owner.Enqueue）
            let lvl_pkt = mir2_shared::packets::server::experience::HeroLevelChanged {
                level: hero_level,
                experience: hero_exp as i64,
                max_experience: hero_max as i64,
            };
            let mut lvl_body = Vec::new();
            if lvl_pkt.write_body(&mut lvl_body).is_ok() {
                let _ = self.gate_ref.tell(SendToClient {
                    session_id,
                    data: build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::HeroLevelChanged as i16,
                        &lvl_body,
                    ),
                }).await;
            }
            // DB 持久化等级
            let db_heroes: Vec<db::DbHero> = self.player_heroes.get(&session_id)
                .map(|hs| hs.iter().map(|h| db::DbHero {
                    index: h.index, name: h.name.clone(), level: h.level,
                    class: h.class as u8, gender: h.gender as u8,
                    dead: h.dead, sealed: h.sealed,
                }).collect())
                .unwrap_or_default();
            if let Err(e) = db::save_heroes(&self.db_pool, &state.name, &db_heroes).await {
                warn!("Failed to save heroes on hero level up: {}", e);
            }
            // 广播英雄对象刷新（等级显示，C# BroadcastInfo）
            self.broadcast_hero_spawn(session_id).await;
            info!("🦸 Hero {} leveled to Lv.{}", hero_name, hero_level);
        }
        debug!("Hero {} gained {} exp (total {}, Lv.{})", hero_name, amount, hero_exp, hero_level);
    }
}

/// 英雄虚拟 object_id 的偏移量（主人 oid + 此值，避免与真实怪物/玩家冲突）
const HERO_OID_OFFSET: u32 = 0x1000_0000;

/// 朝目标走一步的纯函数版（不依赖 MonsterState，供英雄 AI 复用移动逻辑）
fn step_towards(from_x: i32, from_y: i32, to_x: i32, to_y: i32) -> (i32, i32, u8) {
    const DIR_DX: [i32; 8] = [0, 1, 1, 1, 0, -1, -1, -1];
    const DIR_DY: [i32; 8] = [-1, -1, 0, 1, 1, 1, 0, -1];
    let mut best_dir = 4u8;
    let mut best_dist = ((to_x - from_x).pow(2) + (to_y - from_y).pow(2)) as u64;
    for dir in 0..8u8 {
        let nx = from_x + DIR_DX[dir as usize];
        let ny = from_y + DIR_DY[dir as usize];
        let dist = ((nx - to_x).pow(2) + (ny - to_y).pow(2)) as u64;
        if dist < best_dist {
            best_dist = dist;
            best_dir = dir;
        }
    }
    (from_x + DIR_DX[best_dir as usize], from_y + DIR_DY[best_dir as usize], best_dir)
}

/// 远离目标走一步（逃跑用）
fn step_away_from(target_x: i32, target_y: i32, from_x: i32, from_y: i32) -> (i32, i32, u8) {
    // 远离 = 朝向目标相反方向走一步
    let opp_x = from_x - (target_x - from_x);
    let opp_y = from_y - (target_y - from_y);
    step_towards(from_x, from_y, opp_x, opp_y)
}

/// 从 (from) 到 (to) 的 8 方向朝向
fn direction_towards(from_x: i32, from_y: i32, to_x: i32, to_y: i32) -> u8 {
    const DIR_DX: [i32; 8] = [0, 1, 1, 1, 0, -1, -1, -1];
    const DIR_DY: [i32; 8] = [-1, -1, 0, 1, 1, 1, 0, -1];
    let mut best_dir = 4u8;
    let mut best_dist = ((to_x - from_x).pow(2) + (to_y - from_y).pow(2)) as u64;
    for dir in 0..8u8 {
        let nx = from_x + DIR_DX[dir as usize];
        let ny = from_y + DIR_DY[dir as usize];
        let dist = ((nx - to_x).pow(2) + (ny - to_y).pow(2)) as u64;
        if dist < best_dist {
            best_dist = dist;
            best_dir = dir;
        }
    }
    best_dir
}

/// 英雄物理攻击力（#1184：C# GetAttackPower(Stats[MinDC], Stats[MaxDC]) 的稳定近似，取中值）
fn hero_attack_power(stats: &crate::combat::attack::CombatStats) -> i32 {
    if stats.max_atk > stats.min_atk {
        (stats.min_atk + stats.max_atk) / 2
    } else {
        stats.max_atk.max(1)
    }
}

/// 英雄已学技能等级（#1188：C# = SharedRust - 3；0 = 未学）
fn hero_magic_level(hero_magics: &[(i32, u8)], spell_shared: u8) -> u8 {
    let spell_cs = (spell_shared as i32).saturating_sub(3);
    hero_magics
        .iter()
        .find(|(s, _)| *s == spell_cs)
        .map(|(_, l)| *l)
        .unwrap_or(0)
}

/// 装备的毒护符 shape（#1192：C# GetPoison(1)：Amulet shape 1=绿毒 / 2=红毒；0=无）
fn hero_equip_poison_shape(
    equipment: &[Option<mir2_shared::data::item::UserItem>],
    item_infos: &std::collections::HashMap<i32, crate::db::ItemInfo>,
) -> i32 {
    let Some(Some(item)) = equipment.get(crate::actors::inventory::EquipmentSlot::Pendant as usize) else {
        return 0;
    };
    let Some(info) = item_infos.get(&item.item_index) else {
        return 0;
    };
    // DB item_type 为 C# 原始值：8=Amulet
    if info.item_type != 8 {
        return 0;
    }
    if info.shape == 1 || info.shape == 2 {
        info.shape
    } else {
        0
    }
}

/// 是否装备普通护符（#1192：C# GetAmulet(1)：Amulet shape 0 且数量足够）
fn hero_has_amulet(
    equipment: &[Option<mir2_shared::data::item::UserItem>],
    item_infos: &std::collections::HashMap<i32, crate::db::ItemInfo>,
) -> bool {
    let Some(Some(item)) = equipment.get(crate::actors::inventory::EquipmentSlot::Pendant as usize) else {
        return false;
    };
    let Some(info) = item_infos.get(&item.item_index) else {
        return false;
    };
    info.item_type == 8 && info.shape == 0 && item.count > 0
}

/// 道士英雄 SC 攻击力（#1192：C# GetAttackPower(MinSC, MaxSC) 的稳定近似）
fn hero_attack_power_sc(stats: &super::hero_stats::HeroStats) -> i32 {
    if stats.max_sc > stats.min_sc {
        (stats.min_sc + stats.max_sc) / 2
    } else {
        stats.max_sc.max(1)
    }
}

/// 道士 Curse 的 Slow 毒参数（#1196：C# Duration=1+(Lv+1)*2、TickSpeed 1000、Value=GetDamage(SC)）
fn hero_curse_slow(level: u8, value: i32) -> (u32, i32) {
    let duration = (1 + (level as i32 + 1) * 2).max(1) as u32;
    (duration, value.max(1))
}

/// C# HeroBehaviour：0=Attack, 1=CounterAttack, 2=Follow, 3=Custom（#1198）
fn hero_behaviour_is_follow(behaviour: u8) -> bool {
    behaviour == 2
}

/// C# HeroBehaviour：1=CounterAttack（#1198：只反击攻击主人的怪）
fn hero_behaviour_is_counterattack(behaviour: u8) -> bool {
    behaviour == 1
}

/// 指定位置 range 格内的怪数（#1204：C# WizardHero FindAllTargets(range, location).Count）
fn hero_surrounded_count(monsters: &[(u32, i32, i32)], x: i32, y: i32, range: i32) -> usize {
    monsters
        .iter()
        .filter(|(_, mx, my)| (mx - x).abs() <= range && (my - y).abs() <= range)
        .count()
}

/// 目标 1 格内是否还有其他怪（#1200：C# WizardHero TargetSurroundedCount > 1）
fn hero_target_surrounded(monsters: &[(u32, i32, i32)], target_oid: u32, target_x: i32, target_y: i32) -> bool {
    monsters.iter().any(|(oid, x, y)| {
        *oid != target_oid && (x - target_x).abs() <= 1 && (y - target_y).abs() <= 1
    })
}

/// 主人护盾对应 Spell（#1202）
fn hero_owner_shield_spell(kind: OwnerShieldKind) -> mir2_shared::enums::Spell {
    use mir2_shared::enums::Spell;
    match kind {
        OwnerShieldKind::SoulShield => Spell::SoulShield,
        OwnerShieldKind::BlessedArmour => Spell::BlessedArmour,
        OwnerShieldKind::UltimateEnhancer => Spell::UltimateEnhancer,
    }
}

/// 主人护盾 buff 规格（#1202：C# 值 = 目标等级/7+4；UltimateEnhancer 按目标职业 +DC/MC/SC，时长 = SC*4+(Lv+1)*50 秒）
fn hero_owner_shield_buff(
    kind: OwnerShieldKind,
    owner_class: mir2_shared::enums::MirClass,
    owner_level: u16,
    buff_lv: u8,
    stats: &super::hero_stats::HeroStats,
) -> (crate::combat::buff::BuffType, u32) {
    use crate::combat::buff::BuffType;
    use mir2_shared::enums::MirClass;
    let hero_kind = match kind {
        OwnerShieldKind::SoulShield => HeroBuffKind::SoulShield,
        OwnerShieldKind::BlessedArmour => HeroBuffKind::BlessedArmour,
        OwnerShieldKind::UltimateEnhancer => HeroBuffKind::UltimateEnhancer,
    };
    let ticks = (hero_buff_duration(hero_kind, buff_lv, stats) * 10) as u32;
    let buff = match kind {
        OwnerShieldKind::SoulShield => {
            BuffType::MacDefenseBoost {
                bonus: owner_level as i32 / 7 + 4,
            }
        }
        OwnerShieldKind::BlessedArmour => {
            BuffType::AcDefenseBoost {
                bonus: owner_level as i32 / 7 + 4,
            }
        }
        OwnerShieldKind::UltimateEnhancer => {
            let value = if stats.max_sc >= 5 {
                (stats.max_sc / 5).min(8)
            } else {
                1
            };
            match owner_class {
                MirClass::Warrior | MirClass::Assassin => BuffType::AttackBoost { bonus: value },
                MirClass::Wizard | MirClass::Archer => BuffType::McBoost { bonus: value },
                MirClass::Taoist => BuffType::ScBoost { bonus: value },
                _ => BuffType::AttackBoost { bonus: value },
            }
        }
    };
    (buff, ticks)
}

/// 支持类法术是否魔法（#1208：C# Magic → S.ObjectMagic；物理技能走 Attack → S.ObjectAttack）
fn hero_support_spell_is_magic(spell: u8) -> bool {
    use mir2_shared::enums::Spell;
    matches!(
        Spell::try_from(spell).unwrap_or(Spell::None),
        Spell::Healing
            | Spell::Poisoning
            | Spell::Curse
            | Spell::Rage
            | Spell::ProtectionField
            | Spell::Haste
            | Spell::LightBody
            | Spell::MagicShield
            | Spell::MagicBooster
            | Spell::Concentration
            | Spell::SoulShield
            | Spell::BlessedArmour
            | Spell::UltimateEnhancer
    )
}

/// 道士净化条件（#1210：主人中毒且已学 Purification）
fn hero_taoist_needs_purify(owner_poisoned: bool, purification_level: u8) -> bool {
    owner_poisoned && purification_level > 0
}

/// 道士净化成功率（#1210：C# Envir.Random.Next(4) <= Lv；Lv0=25%）
fn hero_purification_roll(level: u8) -> bool {
    fastrand::i32(0..4) <= level as i32
}

/// Repulsion 成功判定（#1212：C# Random(20) < 6 + Lv*3 + Level - ob.Level）
fn hero_repulsion_succeeds(spell_level: u8, hero_level: i32, target_level: i32) -> bool {
    hero_repulsion_succeeds_with(fastrand::i32(0..20), spell_level, hero_level, target_level)
}

fn hero_repulsion_succeeds_with(roll: i32, spell_level: u8, hero_level: i32, target_level: i32) -> bool {
    roll < 6 + spell_level as i32 * 3 + hero_level - target_level
}

/// Repulsion 击退距离（#1212：C# 1 + max(0, Lv-1) + Random(2)）
fn hero_repulsion_distance(spell_level: u8) -> i32 {
    hero_repulsion_distance_with(fastrand::i32(0..2), spell_level)
}

fn hero_repulsion_distance_with(roll: i32, spell_level: u8) -> i32 {
    1 + (spell_level as i32 - 1).max(0) + roll
}

/// TurnUndead 是否成功击杀（#1212：C# 两道概率判定，失败只引仇恨不杀）
fn hero_turn_undead_kills(hero_level: i32, target_level: i32, spell_level: u8) -> bool {
    hero_turn_undead_kills_with(
        fastrand::i32(0..2),
        fastrand::i32(0..100),
        hero_level,
        target_level,
        spell_level,
    )
}

fn hero_turn_undead_kills_with(
    roll_low: i32,
    roll_high: i32,
    hero_level: i32,
    target_level: i32,
    spell_level: u8,
) -> bool {
    // C#：Random(2) + Level - 1 <= 目标等级 → 只引仇恨
    if roll_low + hero_level - 1 <= target_level {
        return false;
    }
    // C#：Random(100) >= ((Lv+1)<<3) + dif → 只引仇恨
    let dif = hero_level - target_level + 15;
    if roll_high >= ((spell_level as i32 + 1) << 3) + dif {
        return false;
    }
    true
}

/// 道士群疗量（#1214：C# MassHealing value = magic.GetDamage(GetAttackPower(MinSC,MaxSC))）
fn hero_mass_heal_amount(
    magic_infos: &std::collections::HashMap<u32, crate::db::MagicInfo>,
    hero_magics: &[(i32, u8)],
    stats: &super::hero_stats::HeroStats,
    class: mir2_shared::enums::MirClass,
) -> i32 {
    hero_spell_damage(
        magic_infos,
        hero_magics,
        mir2_shared::enums::Spell::MassHealing as u8,
        stats,
        class,
    )
    .max(1)
}

/// 各职业 ProcessFriend 增益列表（#1190/#1192/#1210：C# 子类顺序；道士由常驻预置块使用）
fn hero_friend_buffs(
    class: mir2_shared::enums::MirClass,
) -> &'static [(mir2_shared::enums::Spell, HeroBuffKind)] {
    use mir2_shared::enums::{MirClass, Spell};
    match class {
        MirClass::Warrior => {
            &[
                (Spell::Rage, HeroBuffKind::Rage),
                (Spell::ProtectionField, HeroBuffKind::ProtectionField),
            ]
        }
        MirClass::Assassin => {
            &[
                (Spell::Haste, HeroBuffKind::Haste),
                (Spell::LightBody, HeroBuffKind::LightBody),
            ]
        }
        MirClass::Wizard => {
            &[
                (Spell::MagicShield, HeroBuffKind::MagicShield),
                (Spell::MagicBooster, HeroBuffKind::MagicBooster),
            ]
        }
        MirClass::Archer => &[(Spell::Concentration, HeroBuffKind::Concentration)],
        MirClass::Taoist => {
            &[
                (Spell::SoulShield, HeroBuffKind::SoulShield),
                (Spell::BlessedArmour, HeroBuffKind::BlessedArmour),
                (Spell::UltimateEnhancer, HeroBuffKind::UltimateEnhancer),
            ]
        }
        _ => &[],
    }
}

/// 增益时长（秒，#1190/#1192：C# HumanObject 各 Spell 实现）
/// SoulShield/BlessedArmour/UltimateEnhancer 时长依赖道士 SC（C# SC*4 + (Lv+1)*50）
fn hero_buff_duration(kind: HeroBuffKind, level: u8, stats: &super::hero_stats::HeroStats) -> u64 {
    let level = level as u64;
    match kind {
        HeroBuffKind::Rage => 18 + 6 * level,
        HeroBuffKind::ProtectionField => 45 + 15 * level,
        HeroBuffKind::Haste => 25 + 15 * level,
        HeroBuffKind::LightBody => (level + 1) * 30,
        // C# MagicShield 时长按 power 计（magic.GetPower(MC+15)），此处稳定近似
        HeroBuffKind::MagicShield => 30 + 10 * level,
        HeroBuffKind::MagicBooster => 60,
        HeroBuffKind::Concentration => 45 + 15 * level,
        // #1194：C# SpecialArrowShot：PoisonShot buff = 5 + 5*Lv 秒
        HeroBuffKind::PoisonShot => 5 + 5 * level,
        HeroBuffKind::SoulShield
        | HeroBuffKind::BlessedArmour
        | HeroBuffKind::UltimateEnhancer => {
            let sc = if stats.max_sc > stats.min_sc {
                (stats.min_sc + stats.max_sc) / 2
            } else {
                stats.max_sc.max(1)
            };
            (sc * 4 + (level as i32 + 1) * 50).max(1) as u64
        }
    }
}

/// 应用增益到英雄战斗属性（#1190/#1192：C# RefreshStats 对应项）；返回 MagicShield 减伤 %
fn hero_apply_buffs(
    buffs: &[HeroBuff],
    class: mir2_shared::enums::MirClass,
    combat: &mut crate::combat::attack::CombatStats,
    stats: &mut super::hero_stats::HeroStats,
) -> i32 {
    let mut shield_pct = 0;
    for b in buffs {
        match b.kind {
            HeroBuffKind::Rage => {
                // C#：MaxDC * (0.12 + 0.03*Lv) 加到 MinDC/MaxDC
                let add = (stats.max_dc as f32 * (0.12 + 0.03 * b.level as f32)).round() as i32;
                combat.min_atk += add;
                combat.max_atk += add;
            }
            HeroBuffKind::ProtectionField => {
                // C#：MaxAC * (0.2 + 0.03*Lv) 加到 MinAC/MaxAC
                let add = (stats.max_ac as f32 * (0.2 + 0.03 * b.level as f32)).round() as i32;
                combat.min_ac += add;
                combat.max_ac += add;
            }
            HeroBuffKind::LightBody => {
                // C#：Agility = (Lv+1)*2
                combat.agility += (b.level as i32 + 1) * 2;
            }
            HeroBuffKind::MagicBooster => {
                // C#：MinMC=MaxMC = 6 + Lv*6
                let add = 6 + b.level as i32 * 6;
                stats.min_mc += add;
                stats.max_mc += add;
            }
            HeroBuffKind::MagicShield => {
                // C#：DamageReductionPercent = (Lv+2)*10
                shield_pct = (b.level as i32 + 2) * 10;
            }
            HeroBuffKind::SoulShield => {
                // C#：MaxMAC = 目标Level/7 + 4
                combat.max_mac += b.hero_level as i32 / 7 + 4;
            }
            HeroBuffKind::BlessedArmour => {
                // C#：MaxAC = 目标Level/7 + 4
                combat.max_ac += b.hero_level as i32 / 7 + 4;
            }
            HeroBuffKind::UltimateEnhancer => {
                // C#：value = min(8, max(1, MaxSC/5))，按目标职业加 DC/MC/SC
                let value = if stats.max_sc >= 5 {
                    (stats.max_sc / 5).min(8)
                } else {
                    1
                };
                match class {
                    mir2_shared::enums::MirClass::Warrior
                    | mir2_shared::enums::MirClass::Assassin => combat.max_atk += value,
                    mir2_shared::enums::MirClass::Wizard
                    | mir2_shared::enums::MirClass::Archer => stats.max_mc += value,
                    mir2_shared::enums::MirClass::Taoist => stats.max_sc += value,
                    _ => {}
                }
            }
            // #1194：PoisonShot 是标记 buff（无属性，命中附加绿毒）
            HeroBuffKind::Haste | HeroBuffKind::Concentration | HeroBuffKind::PoisonShot => {}
        }
    }
    shield_pct
}

/// 按优先级取第一个已学技能（#1188：C# 各子类 ProcessAttack/Attack 顺序）
fn first_learned_spell(
    hero_magics: &[(i32, u8)],
    priority: &[mir2_shared::enums::Spell],
) -> Option<(mir2_shared::enums::Spell, u8)> {
    priority
        .iter()
        .map(|s| (*s, hero_magic_level(hero_magics, *s as u8)))
        .find(|(_, lvl)| *lvl > 0)
}

/// 英雄法术伤害（#1188：C# GetDamage = (DamageBase + GetPower()) * GetMultiplier()）
/// DamageBase = 英雄自身 MC/SC 中值；Power/Multiplier 来自魔法表 + 实际等级。
fn hero_spell_damage(
    magic_infos: &std::collections::HashMap<u32, crate::db::MagicInfo>,
    hero_magics: &[(i32, u8)],
    spell_shared: u8,
    stats: &super::hero_stats::HeroStats,
    class: mir2_shared::enums::MirClass,
) -> i32 {
    let spell_cs = (spell_shared as i32).saturating_sub(3);
    let level = hero_magic_level(hero_magics, spell_shared).max(1);
    let (min_v, max_v) = match class {
        mir2_shared::enums::MirClass::Wizard => (stats.min_mc, stats.max_mc),
        mir2_shared::enums::MirClass::Taoist => (stats.min_sc, stats.max_sc),
        // #1194：C# 弓箭手技能（StraightShot/PoisonShot）用 MC
        mir2_shared::enums::MirClass::Archer => (stats.min_mc, stats.max_mc),
        _ => (stats.min_dc, stats.max_dc),
    };
    let base = if max_v > min_v {
        (min_v + max_v) / 2
    } else {
        max_v.max(1)
    };
    match magic_infos.get(&(spell_cs as u32)) {
        Some(info) => crate::combat::magic::calc_magic_damage(info, level, base),
        None => base,
    }
}

/// 英雄近战兜底（#1188：未学技能/蓝不足时 1 格内近战；返回是否实际攻击）
fn hero_melee_fallback(
    session_id: u64,
    target_oid: u32,
    target_dist: i32,
    combat: &crate::combat::attack::CombatStats,
    attack_intents: &mut Vec<(u64, u32, i32, mir2_shared::enums::DefenceType, bool)>,
    support_intents: &mut Vec<(u64, u64, u8, bool)>,
) -> bool {
    if target_dist <= 1 {
        let mraw = hero_attack_power(combat);
        attack_intents.push((session_id, target_oid, mraw, mir2_shared::enums::DefenceType::Ac, false));
        support_intents.push((session_id, 0, mir2_shared::enums::Spell::None as u8, false));
        true
    } else {
        false
    }
}

/// 道士英雄治疗量（#1184：C# Healing = magic.GetDamage(GetAttackPower(MinSC,MaxSC)*2) + Level 的简化稳定近似）
fn hero_heal_amount(stats: &super::hero_stats::HeroStats, level: u16) -> i32 {
    let sc = if stats.max_sc > stats.min_sc {
        (stats.min_sc + stats.max_sc) / 2
    } else {
        stats.max_sc.max(1)
    };
    (sc * 2).max(1) + level as i32
}

/// 英雄法术 MP 费用（#1186：C# MagicCost = base_cost + level*level_cost；技能等级用英雄已学等级）
/// spell_shared 为 SharedRust 枚举值（C# = shared - 3）
fn hero_spell_cost(
    magic_infos: &std::collections::HashMap<u32, crate::db::MagicInfo>,
    hero_magics: &[(i32, u8)],
    spell_shared: u8,
) -> i32 {
    let spell_cs = (spell_shared as i32).saturating_sub(3);
    // 已学按实际等级计费（0 级 = 基础费）；未学按 1 级近似（实际不会施放）
    let level = hero_magics
        .iter()
        .find(|(s, _)| *s == spell_cs)
        .map(|(_, l)| *l)
        .unwrap_or(1);
    magic_infos
        .get(&(spell_cs as u32))
        .map(|info| crate::combat::magic::magic_cost(info, level))
        .unwrap_or(5)
}

/// 广播英雄弹道/施法的 ObjectAttack 包
async fn broadcast_hero_attack(
    world: &WorldActor,
    session_id: u64,
    spell: u8,
) {
    let ai = match world.hero_ai_states.get(&session_id) {
        Some(a) => a.clone(),
        None => return,
    };
    let owner_oid = match world.players.get(&session_id).map(|r| r.object_id) {
        Some(oid) => oid,
        None => return,
    };
    let hero_oid = owner_oid.wrapping_add(HERO_OID_OFFSET);
    let mut attack_body = Vec::new();
    attack_body.extend_from_slice(&hero_oid.to_le_bytes());
    attack_body.extend_from_slice(&(ai.x as u32).to_le_bytes());
    attack_body.extend_from_slice(&(ai.y as u32).to_le_bytes());
    attack_body.push(ai.direction);
    attack_body.push(spell);
    attack_body.extend_from_slice(&0u16.to_le_bytes());
    attack_body.push(0u8);
    let attack_packet = build_packet_bytes(
        mir2_shared::enums::ServerPacketIds::ObjectAttack as i16,
        &attack_body,
    );
    for sid in world.players.keys() {
        let _ = world.gate_ref.tell(SendToClient {
            session_id: *sid,
            data: attack_packet.clone(),
        }).await;
    }
}

/// 广播英雄施法动画（#1206：C# Magic → S.ObjectMagic，客户端据此渲染弹道/AoE 特效）
async fn broadcast_hero_magic(
    world: &WorldActor,
    session_id: u64,
    spell: u8,
    target_id: u32,
    target_x: i32,
    target_y: i32,
    level: u8,
) {
    let ai = match world.hero_ai_states.get(&session_id) {
        Some(a) => a.clone(),
        None => return,
    };
    let owner_oid = match world.players.get(&session_id).map(|r| r.object_id) {
        Some(oid) => oid,
        None => return,
    };
    let hero_oid = owner_oid.wrapping_add(HERO_OID_OFFSET);
    let object_magic = mir2_shared::packets::server::magic_combat::ObjectMagic {
        object_id: hero_oid,
        location_x: ai.x,
        location_y: ai.y,
        direction: mir2_shared::enums::MirDirection::try_from(ai.direction)
            .unwrap_or(mir2_shared::enums::MirDirection::Up),
        spell: mir2_shared::enums::Spell::try_from(spell)
            .unwrap_or(mir2_shared::enums::Spell::None),
        target_id,
        target_x,
        target_y,
        cast: true,
        level,
        self_broadcast: false,
        secondary_target_ids: Vec::new(),
    };
    let mut body = Vec::new();
    if object_magic.write_body(&mut body).is_err() {
        return;
    }
    let packet = build_packet_bytes(
        mir2_shared::enums::ServerPacketIds::ObjectMagic as i16,
        &body,
    );
    for sid in world.players.keys() {
        let _ = world.gate_ref.tell(SendToClient {
            session_id: *sid,
            data: packet.clone(),
        }).await;
    }
}

impl WorldActor {
    /// #198：广播英雄对象生成（ObjectPlayer，虚拟 oid = owner_oid + HERO_OID_OFFSET）
    pub(crate) async fn broadcast_hero_spawn(&self, session_id: u64) {
        let record = match self.players.get(&session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        let owner_oid = record.object_id;
        let hero_oid = owner_oid.wrapping_add(HERO_OID_OFFSET);
        let Some(hero) = self
            .player_heroes
            .get(&session_id)
            .and_then(|v| v.iter().find(|h| h.index as u8 == state.hero_index))
        else {
            return;
        };
        let (hx, hy) = self
            .hero_ai_states
            .get(&session_id)
            .map(|a| (a.x, a.y))
            .unwrap_or((state.x, state.y.saturating_add(1)));
        let weapon = state
            .inventory
            .get_equipment(EquipmentSlot::Weapon)
            .and_then(|it| self.item_infos.get(&it.item_index))
            .map(|i| i.shape as i16)
            .unwrap_or(-1);
        let weapon_effect = state
            .inventory
            .get_equipment(EquipmentSlot::Weapon)
            .and_then(|it| self.item_infos.get(&it.item_index))
            .map(|i| i.effect as i16)
            .unwrap_or(0);
        let armor = state
            .inventory
            .get_equipment(EquipmentSlot::Armour)
            .and_then(|it| self.item_infos.get(&it.item_index))
            .map(|i| i.shape as i16)
            .unwrap_or(0);
        let packet = build_object_player_packet(
            &hero.name,
            hero_oid,
            hx,
            hy,
            state.direction,
            hero.level,
            0,
            hero.class,
            hero.gender,
            state.hair,
            weapon,
            weapon_effect,
            armor,
            state.mount_type,
            state.is_mounted,
            0, // 英雄无等级特效
        );
        for sid in self.players.keys() {
            let _ = self.gate_ref.tell(SendToClient {
                session_id: *sid,
                data: packet.clone(),
            }).await;
        }
    }

    /// #198：广播英雄对象移除（ObjectRemove）
    pub(crate) async fn broadcast_hero_remove(&self, owner_oid: u32) {
        let hero_oid = owner_oid.wrapping_add(HERO_OID_OFFSET);
        let mut body = Vec::new();
        body.extend_from_slice(&hero_oid.to_le_bytes());
        let packet = build_packet_bytes(mir2_shared::enums::ServerPacketIds::ObjectRemove as i16, &body);
        for sid in self.players.keys() {
            let _ = self.gate_ref.tell(SendToClient {
                session_id: *sid,
                data: packet.clone(),
            }).await;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn magic_info(base_cost: i32, level_cost: i32) -> crate::db::MagicInfo {
        crate::db::MagicInfo {
            name: String::new(),
            spell: 0,
            base_cost,
            level_cost,
            icon: 0,
            level1: 0,
            level2: 0,
            level3: 0,
            need1: 0,
            need2: 0,
            need3: 0,
            delay_base: 0,
            delay_reduction: 0,
            power_base: 0,
            power_bonus: 0,
            mpower_base: 0,
            mpower_bonus: 0,
            range: 0,
            multiplier_base: 0.0,
            multiplier_bonus: 0.0,
        }
    }

    #[test]
    fn hero_spell_cost_uses_learned_level() {
        use mir2_shared::enums::Spell;
        let shared = Spell::FireBall as u8;
        let cs = shared.saturating_sub(3) as u32;
        let mut map = std::collections::HashMap::new();
        map.insert(cs, magic_info(5, 2));
        // 已学 3 级：base + 3*level_cost
        assert_eq!(hero_spell_cost(&map, &[(cs as i32, 3)], shared), 5 + 3 * 2);
        // 已学 0 级：base
        assert_eq!(hero_spell_cost(&map, &[(cs as i32, 0)], shared), 5);
        // 未学 → 默认 1 级
        assert_eq!(hero_spell_cost(&map, &[], shared), 5 + 2);
        // 无配置 → 兜底 5
        let empty = std::collections::HashMap::new();
        assert_eq!(hero_spell_cost(&empty, &[], shared), 5);
    }

    fn magic_info_damage(
        mpower_base: i32,
        mpower_bonus: i32,
        power_base: i32,
        power_bonus: i32,
        mult_base: f64,
        mult_bonus: f64,
    ) -> crate::db::MagicInfo {
        crate::db::MagicInfo {
            name: String::new(),
            spell: 0,
            base_cost: 0,
            level_cost: 0,
            icon: 0,
            level1: 0,
            level2: 0,
            level3: 0,
            need1: 0,
            need2: 0,
            need3: 0,
            delay_base: 0,
            delay_reduction: 0,
            power_base,
            power_bonus,
            mpower_base,
            mpower_bonus,
            range: 0,
            multiplier_base: mult_base,
            multiplier_bonus: mult_bonus,
        }
    }

    #[test]
    fn hero_magic_level_converts_shared_to_cs() {
        use mir2_shared::enums::Spell;
        let shared = Spell::FireBall as u8;
        let cs = shared.saturating_sub(3) as i32;
        // 已学 3 级
        assert_eq!(hero_magic_level(&[(cs, 3)], shared), 3);
        // 未学 → 0
        assert_eq!(hero_magic_level(&[(cs + 100, 3)], shared), 0);
        assert_eq!(hero_magic_level(&[], shared), 0);
    }

    #[test]
    fn first_learned_spell_respects_priority() {
        use mir2_shared::enums::Spell;
        let magics = vec![
            ((Spell::ThunderBolt as u8).saturating_sub(3) as i32, 2u8),
            ((Spell::FireBall as u8).saturating_sub(3) as i32, 1u8),
        ];
        // 优先级 ThunderBolt > GreatFireBall > FireBall：已学 ThunderBolt 应被选中
        let picked = first_learned_spell(
            &magics,
            &[Spell::ThunderBolt, Spell::GreatFireBall, Spell::FireBall],
        );
        assert_eq!(picked, Some((Spell::ThunderBolt, 2)));
        // 只学 FireBall 时选 FireBall
        let magics2 = vec![((Spell::FireBall as u8).saturating_sub(3) as i32, 1u8)];
        let picked2 = first_learned_spell(
            &magics2,
            &[Spell::ThunderBolt, Spell::GreatFireBall, Spell::FireBall],
        );
        assert_eq!(picked2, Some((Spell::FireBall, 1)));
        // 全未学 → None
        assert_eq!(
            first_learned_spell(&[], &[Spell::ThunderBolt, Spell::FireBall]),
            None
        );
    }

    #[test]
    fn hero_spell_damage_scales_with_level() {
        use mir2_shared::enums::{MirClass, Spell};
        let shared = Spell::FireBall as u8;
        let cs = shared.saturating_sub(3) as u32;
        let mut map = std::collections::HashMap::new();
        // C# GetDamage = (MC + GetPower()) * GetMultiplier()；GetPower = round(MPower/4*(Lv+1) + DefPower)
        map.insert(cs, magic_info_damage(40, 0, 5, 0, 1.0, 0.1));
        let stats = super::hero_stats::hero_base_stats(MirClass::Wizard, 30);
        let lv1 = hero_spell_damage(&map, &[(cs as i32, 1)], shared, &stats, MirClass::Wizard);
        let lv3 = hero_spell_damage(&map, &[(cs as i32, 3)], shared, &stats, MirClass::Wizard);
        // 等级越高伤害越高（multiplier 1.0+0.1*Lv，power 随 Lv+1 增长）
        assert!(lv3 > lv1);
        assert!(lv1 > 0);
    }

    #[test]
    fn hero_melee_fallback_only_at_range1() {
        use mir2_shared::enums::DefenceType;
        let combat = crate::combat::attack::CombatStats {
            min_atk: 5,
            max_atk: 9,
            ..Default::default()
        };
        let mut atk = Vec::new();
        let mut sup = Vec::new();
        let hit = hero_melee_fallback(1, 100, 1, &combat, &mut atk, &mut sup);
        assert!(hit);
        assert_eq!(atk.len(), 1);
        assert_eq!(atk[0].0, 1);
        assert_eq!(atk[0].1, 100);
        assert_eq!(atk[0].2, 7); // (5+9)/2
        assert_eq!(atk[0].3, DefenceType::Ac);
        assert!(!atk[0].4);
        assert_eq!(sup.len(), 1);
        // 距离 2 不攻击
        let hit2 = hero_melee_fallback(1, 100, 2, &combat, &mut atk, &mut sup);
        assert!(!hit2);
        assert_eq!(atk.len(), 1);
    }

    #[test]
    fn hero_friend_buffs_per_class() {
        use mir2_shared::enums::MirClass;
        assert_eq!(hero_friend_buffs(MirClass::Warrior).len(), 2);
        assert_eq!(hero_friend_buffs(MirClass::Assassin).len(), 2);
        assert_eq!(hero_friend_buffs(MirClass::Wizard).len(), 2);
        assert_eq!(hero_friend_buffs(MirClass::Archer).len(), 1);
        // #1192：道士 SoulShield/BlessedArmour/UltimateEnhancer
        assert_eq!(hero_friend_buffs(MirClass::Taoist).len(), 3);
        assert_eq!(hero_friend_buffs(MirClass::Warrior)[0].1, HeroBuffKind::Rage);
    }

    #[test]
    fn hero_buff_duration_matches_csharp() {
        use mir2_shared::enums::MirClass;
        let stats = super::hero_stats::hero_base_stats(MirClass::Taoist, 30);
        // Rage：18+6*Lv；Haste：25+15*Lv；LightBody：(Lv+1)*30；MagicBooster：60
        assert_eq!(hero_buff_duration(HeroBuffKind::Rage, 2, &stats), 30);
        assert_eq!(hero_buff_duration(HeroBuffKind::ProtectionField, 1, &stats), 60);
        assert_eq!(hero_buff_duration(HeroBuffKind::Haste, 2, &stats), 55);
        assert_eq!(hero_buff_duration(HeroBuffKind::LightBody, 1, &stats), 60);
        assert_eq!(hero_buff_duration(HeroBuffKind::MagicBooster, 3, &stats), 60);
        assert_eq!(hero_buff_duration(HeroBuffKind::Concentration, 1, &stats), 60);
        // #1192：道士护盾 Duration = SC*4 + (Lv+1)*50 秒（Taoist Lv30 SC 中值 = 4）
        assert_eq!(
            hero_buff_duration(HeroBuffKind::SoulShield, 1, &stats),
            4 * 4 + (1 + 1) * 50
        );
    }

    #[test]
    fn hero_apply_buffs_stats() {
        use mir2_shared::enums::MirClass;
        let base_stats = super::hero_stats::hero_base_stats(MirClass::Warrior, 30);
        let mut combat = base_stats.to_combat_stats();
        let mut stats = base_stats;
        let buffs = vec![
            HeroBuff { kind: HeroBuffKind::Rage, expire_tick: 0, level: 3, hero_level: 30 },
            HeroBuff { kind: HeroBuffKind::MagicShield, expire_tick: 0, level: 2, hero_level: 30 },
        ];
        let shield = hero_apply_buffs(&buffs, MirClass::Warrior, &mut combat, &mut stats);
        // Rage：MaxDC*(0.12+0.03*3)；MagicShield：(2+2)*10=40%
        let rage_add = (base_stats.max_dc as f32 * 0.21).round() as i32;
        assert_eq!(combat.min_atk, base_stats.min_dc + rage_add);
        assert_eq!(combat.max_atk, base_stats.max_dc + rage_add);
        assert_eq!(shield, 40);
        // MagicBooster：MC + 6+Lv*6（用法师基准，战士 MC 未启用会算出 i32::MAX）
        let wizard_base = super::hero_stats::hero_base_stats(MirClass::Wizard, 30);
        let mut stats2 = wizard_base;
        let mut combat2 = wizard_base.to_combat_stats();
        let buffs2 = vec![HeroBuff { kind: HeroBuffKind::MagicBooster, expire_tick: 0, level: 2, hero_level: 30 }];
        hero_apply_buffs(&buffs2, MirClass::Wizard, &mut combat2, &mut stats2);
        assert_eq!(stats2.max_mc, wizard_base.max_mc + 18);
        // LightBody：Agility + (Lv+1)*2
        let mut stats3 = base_stats;
        let mut combat3 = base_stats.to_combat_stats();
        let buffs3 = vec![HeroBuff { kind: HeroBuffKind::LightBody, expire_tick: 0, level: 1, hero_level: 30 }];
        hero_apply_buffs(&buffs3, MirClass::Assassin, &mut combat3, &mut stats3);
        assert_eq!(combat3.agility, base_stats.agility + 4);
    }

    #[test]
    fn hero_apply_taoist_shields() {
        use mir2_shared::enums::MirClass;
        let base = super::hero_stats::hero_base_stats(MirClass::Taoist, 30);
        // SoulShield：MaxMAC = Lv/7+4（30/7+4 = 8）
        let mut c1 = base.to_combat_stats();
        let mut s1 = base;
        hero_apply_buffs(
            &[HeroBuff { kind: HeroBuffKind::SoulShield, expire_tick: 0, level: 1, hero_level: 30 }],
            MirClass::Taoist, &mut c1, &mut s1,
        );
        assert_eq!(c1.max_mac, base.max_mac + 8);
        // BlessedArmour：MaxAC = Lv/7+4
        let mut c2 = base.to_combat_stats();
        let mut s2 = base;
        hero_apply_buffs(
            &[HeroBuff { kind: HeroBuffKind::BlessedArmour, expire_tick: 0, level: 1, hero_level: 28 }],
            MirClass::Taoist, &mut c2, &mut s2,
        );
        assert_eq!(c2.max_ac, base.max_ac + 8); // 28/7+4 = 8
        // UltimateEnhancer：道士 → MaxSC += min(8, max(1, MaxSC/5))
        let mut c3 = base.to_combat_stats();
        let mut s3 = base;
        hero_apply_buffs(
            &[HeroBuff { kind: HeroBuffKind::UltimateEnhancer, expire_tick: 0, level: 1, hero_level: 30 }],
            MirClass::Taoist, &mut c3, &mut s3,
        );
        assert_eq!(s3.max_sc, base.max_sc + 1); // MaxSC=7 → 7/5=1
    }

    #[test]
    fn hero_equip_poison_shape_and_amulet() {
        use crate::actors::inventory::EquipmentSlot;
        let mut item_infos = std::collections::HashMap::new();
        // 毒护符：Amulet(type=8) shape 1（绿毒）
        item_infos.insert(
            100,
            crate::db::ItemInfo {
                index: 100,
                name: String::from("PoisonAmulet"),
                item_type: 8,
                shape: 1,
                ..Default::default()
            },
        );
        // 普通护符：Amulet shape 0
        item_infos.insert(
            101,
            crate::db::ItemInfo {
                index: 101,
                name: String::from("Amulet"),
                item_type: 8,
                shape: 0,
                ..Default::default()
            },
        );
        // 毒护符装备在 Pendant 槽
        let mut eq: Vec<Option<mir2_shared::data::item::UserItem>> = vec![None; EquipmentSlot::COUNT];
        let mut poison = mir2_shared::data::item::UserItem::new(100);
        poison.count = 1;
        eq[EquipmentSlot::Pendant as usize] = Some(poison);
        assert_eq!(hero_equip_poison_shape(&eq, &item_infos), 1);
        assert!(!hero_has_amulet(&eq, &item_infos));
        // 普通护符
        let mut eq2: Vec<Option<mir2_shared::data::item::UserItem>> = vec![None; EquipmentSlot::COUNT];
        let mut amulet = mir2_shared::data::item::UserItem::new(101);
        amulet.count = 1;
        eq2[EquipmentSlot::Pendant as usize] = Some(amulet);
        assert_eq!(hero_equip_poison_shape(&eq2, &item_infos), 0);
        assert!(hero_has_amulet(&eq2, &item_infos));
        // 未装备 → 0/false
        let empty: Vec<Option<mir2_shared::data::item::UserItem>> = vec![None; EquipmentSlot::COUNT];
        assert_eq!(hero_equip_poison_shape(&empty, &item_infos), 0);
        assert!(!hero_has_amulet(&empty, &item_infos));
    }

    #[test]
    fn hero_poison_shot_duration_and_no_stat() {
        use mir2_shared::enums::MirClass;
        let stats = super::hero_stats::hero_base_stats(MirClass::Archer, 30);
        // C#：PoisonShot buff = 5 + 5*Lv 秒
        assert_eq!(hero_buff_duration(HeroBuffKind::PoisonShot, 2, &stats), 15);
        // PoisonShot 无属性加成
        let mut combat = stats.to_combat_stats();
        let mut s = stats;
        let shield = hero_apply_buffs(
            &[HeroBuff { kind: HeroBuffKind::PoisonShot, expire_tick: 0, level: 2, hero_level: 30 }],
            MirClass::Archer, &mut combat, &mut s,
        );
        assert_eq!(shield, 0);
        assert_eq!(combat.max_atk, stats.max_dc);
    }

    #[test]
    fn hero_spell_damage_archer_uses_mc() {
        use mir2_shared::enums::{MirClass, Spell};
        let shared = Spell::StraightShot as u8;
        let cs = shared.saturating_sub(3) as u32;
        let mut map = std::collections::HashMap::new();
        map.insert(cs, magic_info_damage(40, 0, 5, 0, 1.0, 0.1));
        let stats = super::hero_stats::hero_base_stats(MirClass::Archer, 30);
        let dmg = hero_spell_damage(&map, &[(cs as i32, 1)], shared, &stats, MirClass::Archer);
        // C# 弓箭手技能用 MC（非 DC）
        let mc = (stats.min_mc + stats.max_mc) / 2;
        assert_eq!(dmg, crate::combat::magic::calc_magic_damage(map.get(&cs).unwrap(), 1, mc));
        assert!(dmg > 0);
    }

    #[test]
    fn hero_curse_slow_params() {
        // C#：Duration = 1 + (Lv+1)*2；Value = GetDamage(SC)（至少 1）
        assert_eq!(hero_curse_slow(0, 30), (3, 30));
        assert_eq!(hero_curse_slow(2, 30), (7, 30));
        assert_eq!(hero_curse_slow(1, 0), (5, 1));
    }

    #[test]
    fn hero_behaviour_csharp_semantics() {
        // C#：0=Attack, 1=CounterAttack, 2=Follow, 3=Custom
        assert!(hero_behaviour_is_follow(2));
        assert!(!hero_behaviour_is_follow(0));
        assert!(!hero_behaviour_is_follow(1));
        assert!(!hero_behaviour_is_follow(3));
        assert!(hero_behaviour_is_counterattack(1));
        assert!(!hero_behaviour_is_counterattack(0));
        assert!(!hero_behaviour_is_counterattack(2));
    }

    #[test]
    fn hero_target_surrounded_detects_neighbors() {
        let mobs = vec![(1u32, 10i32, 10i32), (2, 11, 10), (3, 20, 20)];
        // 目标 (10,10)：旁边有 (11,10) → 被围
        assert!(hero_target_surrounded(&mobs, 1, 10, 10));
        // 目标 (20,20)：旁边无其他怪 → 未围
        assert!(!hero_target_surrounded(&mobs, 3, 20, 20));
        // 只有自己 → 未围
        assert!(!hero_target_surrounded(&[(5, 0, 0)], 5, 0, 0));
    }

    #[test]
    fn hero_owner_shield_specs() {
        use crate::combat::buff::BuffType;
        use mir2_shared::enums::MirClass;
        let stats = super::hero_stats::hero_base_stats(MirClass::Taoist, 30);
        // SoulShield：主人等级 30 → MacDefenseBoost 30/7+4 = 8；时长 = SC*4+(Lv+1)*50 秒 → ticks ×10
        let (bt, ticks) = hero_owner_shield_buff(
            OwnerShieldKind::SoulShield,
            MirClass::Taoist,
            30,
            1,
            &stats,
        );
        assert_eq!(bt, BuffType::MacDefenseBoost { bonus: 8 });
        assert_eq!(ticks, (hero_buff_duration(HeroBuffKind::SoulShield, 1, &stats) * 10) as u32);
        // BlessedArmour → AcDefenseBoost
        let (bt2, _) = hero_owner_shield_buff(
            OwnerShieldKind::BlessedArmour,
            MirClass::Taoist,
            30,
            1,
            &stats,
        );
        assert_eq!(bt2, BuffType::AcDefenseBoost { bonus: 8 });
        // UltimateEnhancer：道士主人 → ScBoost（MaxSC=4 → value=1）
        let (bt3, _) = hero_owner_shield_buff(
            OwnerShieldKind::UltimateEnhancer,
            MirClass::Taoist,
            30,
            1,
            &stats,
        );
        assert_eq!(bt3, BuffType::ScBoost { bonus: 1 });
        // 战士主人 → AttackBoost
        let (bt4, _) = hero_owner_shield_buff(
            OwnerShieldKind::UltimateEnhancer,
            MirClass::Warrior,
            30,
            1,
            &stats,
        );
        assert_eq!(bt4, BuffType::AttackBoost { bonus: 1 });
        // spell 映射
        assert_eq!(
            hero_owner_shield_spell(OwnerShieldKind::SoulShield),
            mir2_shared::enums::Spell::SoulShield
        );
    }
    #[test]
    fn hero_surrounded_count_ranges() {
        let mobs = vec![(1u32, 10i32, 10i32), (2, 11, 10), (3, 12, 10), (4, 20, 20)];
        // 英雄 (10,10)：2 格内 = (10,10),(11,10),(12,10) = 3
        assert_eq!(hero_surrounded_count(&mobs, 10, 10, 2), 3);
        // 1 格内 = (10,10),(11,10) = 2
        assert_eq!(hero_surrounded_count(&mobs, 10, 10, 1), 2);
        // (20,20) 2 格内 = 1
        assert_eq!(hero_surrounded_count(&mobs, 20, 20, 2), 1);
    }
    #[test]
    fn hero_support_spell_magic_classification() {
        use mir2_shared::enums::Spell;
        // 魔法类：治疗/增益/毒/诅咒
        assert!(hero_support_spell_is_magic(Spell::Healing as u8));
        assert!(hero_support_spell_is_magic(Spell::Rage as u8));
        assert!(hero_support_spell_is_magic(Spell::Poisoning as u8));
        assert!(hero_support_spell_is_magic(Spell::Curse as u8));
        assert!(hero_support_spell_is_magic(Spell::SoulShield as u8));
        // 物理类：Slaying/HalfMoon/None
        assert!(!hero_support_spell_is_magic(Spell::Slaying as u8));
        assert!(!hero_support_spell_is_magic(Spell::HalfMoon as u8));
        assert!(!hero_support_spell_is_magic(Spell::None as u8));
    }
    #[test]
    fn hero_taoist_needs_purify_gate() {
        // 主人中毒且已学 → 需要净化
        assert!(hero_taoist_needs_purify(true, 1));
        assert!(hero_taoist_needs_purify(true, 3));
        // 未中毒或未学 → 不需要
        assert!(!hero_taoist_needs_purify(false, 1));
        assert!(!hero_taoist_needs_purify(true, 0));
        assert!(!hero_taoist_needs_purify(false, 0));
        // 净化成功概率函数可调用且结果合理（Lv3 几乎必成功）
        let successes = (0..100).filter(|_| hero_purification_roll(3)).count();
        assert!(successes >= 90);
    }

    #[test]
    fn hero_repulsion_and_turnundead_rolls() {
        // Repulsion：roll < 6 + Lv*3 + Level - ob.Level（目标等级 < 英雄等级为前置条件）
        assert!(hero_repulsion_succeeds_with(0, 1, 30, 20));
        // 高 roll（≥ 阈值 19）→ 失败
        assert!(!hero_repulsion_succeeds_with(19, 1, 30, 20));
        // 击退距离：1 + max(0, Lv-1) + roll
        assert_eq!(hero_repulsion_distance_with(0, 1), 1);
        assert_eq!(hero_repulsion_distance_with(1, 3), 4); // 1 + 2 + 1
        // TurnUndead：roll_low + Level - 1 <= 目标等级 → 不杀
        assert!(!hero_turn_undead_kills_with(0, 0, 30, 35, 1));
        // roll_high 过大 → 不杀
        assert!(!hero_turn_undead_kills_with(1, 99, 30, 10, 1));
        // 双判定都过 → 杀（英雄 30 级 vs 亡灵 5 级）
        assert!(hero_turn_undead_kills_with(1, 0, 30, 5, 1));
    }

    #[test]
    fn hero_mass_heal_amount_uses_sc() {
        use mir2_shared::enums::{MirClass, Spell};
        let shared = Spell::MassHealing as u8;
        let cs = shared.saturating_sub(3) as u32;
        let mut map = std::collections::HashMap::new();
        map.insert(cs, magic_info_damage(40, 0, 5, 0, 1.0, 0.1));
        let stats = super::hero_stats::hero_base_stats(MirClass::Taoist, 30);
        let amount = hero_mass_heal_amount(&map, &[(cs as i32, 1)], &stats, MirClass::Taoist);
        // C#：value = GetDamage(GetAttackPower(SC))；Taoist Lv30 SC 中值 = 4
        let sc = (stats.min_sc + stats.max_sc) / 2;
        assert_eq!(amount, crate::combat::magic::calc_magic_damage(map.get(&cs).unwrap(), 1, sc));
        assert!(amount >= 1);
    }
}

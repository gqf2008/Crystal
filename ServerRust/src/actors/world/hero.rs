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
            /// hero_behaviour: 0=Attack, 1=Follow
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
                // hero_behaviour == 1 (Follow) 时英雄纯跟随，不参战
                // 但仍需移动跟随主人，所以保留快照（AI 内部判断 behaviour）
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
        }
        let monster_snaps: Vec<MonsterSnap> = self.monsters.values()
            .filter(|m| m.hp > 0)
            .map(|m| MonsterSnap {
                oid: m.object_id,
                x: m.x,
                y: m.y,
                max_hp: m.max_hp,
                map_index: m.map_index,
            })
            .collect();

        // ===== 阶段 2：意图收集（循环内只收集，不修改 self） =====
        // 移动意图：(hero_session_id, new_x, new_y, direction) —— 循环外更新 hero_ai_states
        let mut move_intents: Vec<(u64, i32, i32, u8)> = Vec::new();
        // 近战/远程物理攻击意图：(hero_session_id, target_oid, raw_damage, defence_type, is_ranged)
        let mut attack_intents: Vec<(u64, u32, i32, DefenceType, bool)> = Vec::new();
        // 弹道法术意图（法师/道士/弓箭手远程技能）：直接 push 到 pending_spell_completions
        // (session_id, spell, target_oid, target_x, target_y, damage, fire_at_tick)
        let mut spell_intents: Vec<(u64, u8, u32, i32, i32, i32, u64)> = Vec::new();
        // 辅助意图（道士治疗主人 / 战士 buff）：暂时简化为发送 ObjectAttack 广播但不造伤害
        // (hero_session_id, target_session_or_zero, spell_id, is_heal)
        let mut support_intents: Vec<(u64, u64, u8, bool)> = Vec::new();
        // 自动喝药意图：(hero_session_id, item_index, is_mp) —— 阶段 2.4 统一消耗（C# ProcessAutoPot/TryAutoPot）
        let mut autopot_intents: Vec<(u64, i32, bool)> = Vec::new();

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
                let regen = (ai_local.max_mp * 3 / 100 + 1).max(1);
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

            let behaviour_follow = snap.behaviour == 1; // Follow 模式：纯跟随

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

            // ===== 在攻击范围内：攻击/施法 =====
            if target_dist <= attack_range && can_attack {
                ai_local.direction = direction_towards(ai_local.x, ai_local.y, target.x, target.y);

                match snap.class {
                    MirClass::Warrior => {
                        // 战士近战：基础 DC 伤害，偶尔触发 Slaying（攻杀）
                        let raw = hero_attack_power(&snap.hero_combat);
                        let spell_id = if self.tick_count % 7 == 0 { Spell::Slaying as u8 } else { Spell::None as u8 };
                        attack_intents.push((snap.session_id, target.oid, raw, defence, false));
                        // 广播带 spell_id 的 ObjectAttack（循环外广播）
                        support_intents.push((snap.session_id, 0, spell_id, false));
                        ai_local.next_attack_tick = self.tick_count + 6; // ~600ms
                    }
                    MirClass::Assassin => {
                        // 刺客突进 + 近战：DoubleSlash 双击
                        let raw = hero_attack_power(&snap.hero_combat);
                        attack_intents.push((snap.session_id, target.oid, raw, defence, false));
                        // 偶尔突进（FlashDash）：模拟为额外一次伤害
                        if self.tick_count % 10 == 0 {
                            attack_intents.push((snap.session_id, target.oid, raw / 2, defence, false));
                            support_intents.push((snap.session_id, 0, Spell::FlashDash as u8, false));
                        } else {
                            support_intents.push((snap.session_id, 0, Spell::DoubleSlash as u8, false));
                        }
                        ai_local.next_attack_tick = self.tick_count + 5;
                    }
                    MirClass::Wizard => {
                        // 法师弹道：FireBall / ThunderBolt（亡灵 +50% 由 tick_spell_completions 处理）
                        let spell = Spell::FireBall;
                        // #1184：法师法术伤害用英雄自身 MC（C# WizardHero BeginMagic → 自身 Stats）
                        let raw = hero_spell_damage(&snap.hero_stats, snap.class);
                        // #1186：耗蓝（C# CanUseMagic：MagicCost > MP → 无法施法）
                        let cost =
                            hero_spell_cost(&self.magic_infos, &snap.hero_magics, spell as u8);
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
                            ));
                            ai_local.next_attack_tick = self.tick_count + 8;
                        } else {
                            // 蓝不足：1 格内近战兜底（C# WizardHero 无蓝时 ProcessTarget 退避/近战）
                            if target_dist <= 1 {
                                let mraw = hero_attack_power(&snap.hero_combat);
                                attack_intents.push((
                                    snap.session_id,
                                    target.oid,
                                    mraw,
                                    DefenceType::Ac,
                                    false,
                                ));
                                support_intents.push((
                                    snap.session_id,
                                    0,
                                    Spell::None as u8,
                                    false,
                                ));
                            }
                            ai_local.next_attack_tick = self.tick_count + 6;
                        }
                    }
                    MirClass::Taoist => {
                        // 道士：辅助为主（治疗主人/上盾）+ 毒 + SoulFireBall 弹道
                        // 优先辅助：主人 HP < 70% 时治疗
                        let owner_hp_pct = if snap.owner_max_hp > 0 {
                            snap.owner_hp * 100 / snap.owner_max_hp
                        } else { 100 };
                        // #1186：治疗也耗蓝（C# CanUseMagic）
                        let heal_cost = hero_spell_cost(
                            &self.magic_infos,
                            &snap.hero_magics,
                            Spell::Healing as u8,
                        );
                        if owner_hp_pct < 70 && ai_local.mp >= heal_cost {
                            ai_local.mp -= heal_cost;
                            support_intents.push((
                                snap.session_id,
                                snap.session_id,
                                Spell::Healing as u8,
                                true,
                            ));
                            ai_local.next_attack_tick = self.tick_count + 10;
                        } else {
                            // 否则施毒 + 弹道（#1184：道士符伤害用英雄自身 SC）
                            let spell = Spell::SoulFireBall;
                            let raw = hero_spell_damage(&snap.hero_stats, snap.class);
                            let cost =
                                hero_spell_cost(&self.magic_infos, &snap.hero_magics, spell as u8);
                            if ai_local.mp >= cost {
                                ai_local.mp -= cost;
                                spell_intents.push((
                                    snap.session_id,
                                    spell as u8,
                                    target.oid,
                                    target.x,
                                    target.y,
                                    raw,
                                    self.tick_count + 4,
                                ));
                            } else if target_dist <= 1 {
                                // 蓝不足：1 格内近战兜底
                                let mraw = hero_attack_power(&snap.hero_combat);
                                attack_intents.push((
                                    snap.session_id,
                                    target.oid,
                                    mraw,
                                    DefenceType::Ac,
                                    false,
                                ));
                                support_intents.push((
                                    snap.session_id,
                                    0,
                                    Spell::None as u8,
                                    false,
                                ));
                            }
                            ai_local.next_attack_tick = self.tick_count + 10;
                        }
                    }
                    MirClass::Archer => {
                        // 弓箭手远程：StraightShot（AC 防御的物理弹道）
                        let spell = Spell::StraightShot;
                        let raw = hero_attack_power(&snap.hero_combat);
                        // #1186：弓箭技能耗蓝（C# CanUseMagic）
                        let cost =
                            hero_spell_cost(&self.magic_infos, &snap.hero_magics, spell as u8);
                        if ai_local.mp >= cost {
                            ai_local.mp -= cost;
                            spell_intents.push((
                                snap.session_id,
                                spell as u8,
                                target.oid,
                                target.x,
                                target.y,
                                raw,
                                self.tick_count + 4,
                            ));
                            ai_local.next_attack_tick = self.tick_count + 7;
                        } else {
                            // 蓝不足：1 格内近战兜底
                            if target_dist <= 1 {
                                let mraw = hero_attack_power(&snap.hero_combat);
                                attack_intents.push((
                                    snap.session_id,
                                    target.oid,
                                    mraw,
                                    DefenceType::Ac,
                                    false,
                                ));
                                support_intents.push((
                                    snap.session_id,
                                    0,
                                    Spell::None as u8,
                                    false,
                                ));
                            }
                            ai_local.next_attack_tick = self.tick_count + 6;
                        }
                    }
                }
                // 战斗时英雄 HP 模拟损耗（敌人反击的近似，#1134 增强到可感知）
                let counter = (target.max_hp / 10).max(5);
                ai_local.hp = ai_local.hp.saturating_sub(counter / 3);
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
        for (session_id, spell, target_oid, tx, ty, damage, fire_at_tick) in &spell_intents {
            // 广播 ObjectAttack 作为弹道发射动画
            broadcast_hero_attack(self, *session_id, *spell).await;
            // #1184：英雄弹道用英雄自身属性结算（命中/暴击/等级）
            let hero_snap = snapshots.iter().find(|s| s.session_id == *session_id);
            self.pending_spell_completions.push(PendingSpellCompletion {
                fire_at_tick: *fire_at_tick,
                session_id: *session_id,
                spell: *spell,
                target_id: *target_oid,
                target_x: *tx,
                target_y: *ty,
                damage: *damage,
                magic_stat: hero_snap
                    .map(|s| hero_spell_damage(&s.hero_stats, s.class))
                    .unwrap_or(10),
                hero_stats: hero_snap.map(|s| s.hero_combat),
                hero_level: hero_snap.map(|s| s.hero_level),
                spell_level: 1,
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
                        .map(|s| hero_heal_amount(&s.hero_stats, s.hero_level))
                        .unwrap_or(5);
                    let _ = record.actor_ref.ask(crate::actors::player::Heal { amount }).await;
                    debug!("Hero healed owner {} for {} HP", heal_target_session, amount);
                }
            }
            // 广播 ObjectAttack（近战/施法动画）
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

/// 英雄法术伤害（#1184：C# HumanObject Magic 用英雄自身 Stats）
/// Wizard→MC / Taoist→SC / 其他→DC，取对应属性中值作为稳定近似。
fn hero_spell_damage(
    stats: &super::hero_stats::HeroStats,
    class: mir2_shared::enums::MirClass,
) -> i32 {
    let (min_v, max_v) = match class {
        mir2_shared::enums::MirClass::Wizard => (stats.min_mc, stats.max_mc),
        mir2_shared::enums::MirClass::Taoist => (stats.min_sc, stats.max_sc),
        _ => (stats.min_dc, stats.max_dc),
    };
    if max_v > min_v {
        (min_v + max_v) / 2
    } else {
        max_v.max(1)
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
}

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

    let packet = mir2_shared::packets::server::hero::HeroInformation {
        object_id: hero_oid,
        name: hero.name.clone(),
        class: hero.class,
        gender: hero.gender,
        level: hero.level,
        hair: 0,
        hp: ai_hp,
        mp: 0,
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
}

impl HeroCombatAI {
    /// 以主人状态初始化英雄 AI（主人后方 1 格出生）
    fn new_for_owner(owner_x: i32, owner_y: i32, owner_max_hp: i32) -> Self {
        let max_hp = (owner_max_hp as f32 * 0.6) as i32;
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
        }

        let mut snapshots: Vec<HeroSnapshot> = Vec::new();
        for (session_id, record) in &self.players {
            if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                // hero_index > 0 表示有出战英雄
                if state.hero_index == 0 || state.is_dead {
                    continue;
                }
                // hero_behaviour == 1 (Follow) 时英雄纯跟随，不参战
                // 但仍需移动跟随主人，所以保留快照（AI 内部判断 behaviour）
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

        for snap in &snapshots {
            // 确保该英雄有 AI 状态（首次出现则初始化）
            let ai = self.hero_ai_states
                .entry(snap.session_id)
                .or_insert_with(|| HeroCombatAI::new_for_owner(snap.owner_x, snap.owner_y, snap.owner_stats.max_atk.max(10) * 10));
            // 暴露可变副本用于本 tick 决策（循环内不写回 self）
            let mut ai_local = ai.clone();
            // 同步英雄 HP 到主人存活状态（简化：主人活着则英雄满血）
            if !snap.owner_dead {
                ai_local.hp = ai_local.max_hp;
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
                        let raw = hero_attack_power(&snap.owner_stats, true);
                        let spell_id = if self.tick_count % 7 == 0 { Spell::Slaying as u8 } else { Spell::None as u8 };
                        attack_intents.push((snap.session_id, target.oid, raw, defence, false));
                        // 广播带 spell_id 的 ObjectAttack（循环外广播）
                        support_intents.push((snap.session_id, 0, spell_id, false));
                        ai_local.next_attack_tick = self.tick_count + 6; // ~600ms
                    }
                    MirClass::Assassin => {
                        // 刺客突进 + 近战：DoubleSlash 双击
                        let raw = hero_attack_power(&snap.owner_stats, true);
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
                        let raw = hero_spell_damage(&snap.owner_stats, true);
                        spell_intents.push((
                            snap.session_id, spell as u8, target.oid,
                            target.x, target.y, raw, self.tick_count + 4, // 弹道延迟 ~400ms
                        ));
                        ai_local.next_attack_tick = self.tick_count + 8;
                    }
                    MirClass::Taoist => {
                        // 道士：辅助为主（治疗主人/上盾）+ 毒 + SoulFireBall 弹道
                        // 优先辅助：主人 HP < 70% 时治疗
                        let owner_hp_pct = if snap.owner_max_hp > 0 {
                            snap.owner_hp * 100 / snap.owner_max_hp
                        } else { 100 };
                        if owner_hp_pct < 70 {
                            support_intents.push((snap.session_id, snap.session_id, Spell::Healing as u8, true));
                        } else {
                            // 否则施毒 + 弹道
                            let spell = Spell::SoulFireBall;
                            let raw = hero_spell_damage(&snap.owner_stats, false);
                            spell_intents.push((
                                snap.session_id, spell as u8, target.oid,
                                target.x, target.y, raw, self.tick_count + 4,
                            ));
                        }
                        ai_local.next_attack_tick = self.tick_count + 10;
                    }
                    MirClass::Archer => {
                        // 弓箭手远程：StraightShot（AC 防御的物理弹道）
                        let spell = Spell::StraightShot;
                        let raw = hero_attack_power(&snap.owner_stats, false);
                        spell_intents.push((
                            snap.session_id, spell as u8, target.oid,
                            target.x, target.y, raw, self.tick_count + 4,
                        ));
                        ai_local.next_attack_tick = self.tick_count + 7;
                    }
                }
                // 战斗时英雄 HP 模拟损耗（敌人反击的近似）
                let counter = (target.max_hp / 20).max(2);
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
                Some(s) => s.owner_stats,
                None => continue,
            };
            let level_offset = snapshots.iter()
                .find(|s| s.session_id == *session_id)
                .map(|s| s.owner_level.min(10) as u16)
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
            self.pending_spell_completions.push(PendingSpellCompletion {
                fire_at_tick: *fire_at_tick,
                session_id: *session_id,
                spell: *spell,
                target_id: *target_oid,
                target_x: *tx,
                target_y: *ty,
                damage: *damage,
                magic_stat: snapshots.iter()
                    .find(|s| s.session_id == *session_id)
                    .map(|s| s.owner_stats.max_atk)
                    .unwrap_or(10),
                spell_level: 1,
            });
        }

        // 3d. 广播近战 ObjectAttack（带 spell_id）+ 道士治疗
        for (session_id, heal_target_session, spell_id, is_heal) in &support_intents {
            if *is_heal {
                // 道士治疗主人：直接 Heal
                if let Some(record) = self.players.get(heal_target_session) {
                    let amount = 30 + snapshots.iter()
                        .find(|s| s.session_id == *session_id)
                        .map(|s| s.owner_stats.max_atk / 4)
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

/// 英雄物理攻击力（基于主人属性，战士/刺客/弓箭手用，近战 true 走 DC，远程 false 走 DC×0.9 近似）
fn hero_attack_power(stats: &crate::combat::attack::CombatStats, melee: bool) -> i32 {
    let base = if stats.max_atk > stats.min_atk {
        (stats.min_atk + stats.max_atk) / 2
    } else {
        stats.max_atk.max(1)
    };
    if melee { base } else { (base as f32 * 0.9) as i32 }
}

/// 英雄法术伤害（法师 MC / 道士 SC）
///
/// 注意：CombatStats 把 DC/MC/SC 折叠到统一的 atk 字段，这里按职业取不同倍率近似：
/// 法师弹道（is_mc=true）走 max_atk；道士符（is_mc=false）走 max_atk 的 0.8 倍。
fn hero_spell_damage(stats: &crate::combat::attack::CombatStats, is_mc: bool) -> i32 {
    let base = stats.max_atk.max(6);
    if is_mc { base.max(8) } else { ((base as f32) * 0.8) as i32 }
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

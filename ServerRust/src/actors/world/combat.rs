use super::*;

/// #895：PvP/受击耐久损耗的非武器装备槽（C# DamageDura 排除 Weapon，且 Amulet 由
/// DamageItem 内部免疫；与 tick.rs 怪物命中路径的槽位一致）
const DAMAGE_DURA_ARMOR_SLOTS: [EquipmentSlot; 8] = [
    EquipmentSlot::Armour,
    EquipmentSlot::Helmet,
    EquipmentSlot::BraceletL,
    EquipmentSlot::BraceletR,
    EquipmentSlot::RingL,
    EquipmentSlot::RingR,
    EquipmentSlot::Shoes,
    EquipmentSlot::Necklace,
];

impl WorldActor {
    /// #895：PvP 受击装备耐久损耗（C# HumanObject.DamageDura：非武器槽 -1，
    /// 命中即扣，含致死；NoDuraLoss/Strong 减免由 DamageEquipment 处理）。
    /// 对齐 tick.rs 怪物命中路径；装备损坏时重算属性并广播外观。
    async fn damage_armor_on_pvp_hit(&self, session_id: u64) {
        let mut any_broke = false;
        if let Some(record) = self.players.get(&session_id) {
            for slot in DAMAGE_DURA_ARMOR_SLOTS {
                let broke = record.actor_ref
                    .ask(crate::actors::player::DamageEquipment { slot, amount: 1 })
                    .await
                    .unwrap_or(false);
                if broke {
                    any_broke = true;
                }
            }
        }
        if any_broke {
            if let Some(state) = self.recalculate_and_set_stat_bonuses(session_id).await {
                self.broadcast_equipment_visuals(session_id, &state).await;
            }
        }
    }
}

/// 近战攻击技能（SharedRust 枚举值，C# CompleteAttack 会 LevelMagic 的技能）
const ATTACK_SKILL_SPELLS: [u8; 7] = [
    SPELL_SLAYING,
    SPELL_THRUSTING,
    SPELL_HALFMOON,
    SPELL_CROSS_HALFMOON,
    SPELL_TWIN_DRAKE_BLADE,
    SPELL_DOUBLE_SLASH,
    SPELL_FLAMING_SWORD,
];

/// #1256：攻击技能查找——magics/hero_magics 存 C# 编号，入参为 SharedRust(+3)，
/// 需 -3 转换（此前 Slaying/HalfMoon/CrossHalfMoon 直接比较 SharedRust 值导致永不匹配）
fn find_attack_skill<'a>(
    magics: &'a [crate::actors::player::PlayerMagic],
    spell_shared: u8,
) -> Option<&'a crate::actors::player::PlayerMagic> {
    let cs = (spell_shared as i32).saturating_sub(3);
    magics.iter().find(|m| m.spell == cs)
}

impl WorldActor {
    /// #1256：C# CompleteAttack——近战命中给攻击技能经验（Random.Next(3)+1，与 MagicRequest 一致）
    async fn grant_attack_skill_exp(&self, session_id: u64, spell_shared: u8) {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        let info = self
            .magic_infos
            .get(&((spell_shared as u32).saturating_sub(3)))
            .cloned();
        if let Some(record) = self.players.get(&session_id) {
            let _ = record
                .actor_ref
                .ask(crate::actors::player::GainSpellExp {
                    spell: spell_shared,
                    amount: (1 + fastrand::i32(0..3)) as u16,
                    cast_time: now_ms,
                    info,
                })
                .await;
        }
    }

    /// #1256：ElectricShock 技能经验（C# ElectricShock：成功必给/失败 50%/无目标不给）
    async fn grant_electric_shock_exp(
        &self,
        session_id: u64,
        spell_shared: u8,
        now_ms: i64,
        spell_cs: u8,
    ) {
        let info = self.magic_infos.get(&(spell_cs as u32)).cloned();
        if let Some(record) = self.players.get(&session_id) {
            let _ = record
                .actor_ref
                .ask(crate::actors::player::GainSpellExp {
                    spell: spell_shared,
                    amount: (1 + fastrand::i32(0..3)) as u16,
                    cast_time: now_ms,
                    info,
                })
                .await;
        }
    }
}

/// 攻击请求（从 GateActor 转发）
pub struct WorldAttackRequest {
    pub session_id: u64,
    pub direction: u8,
    pub spell: u8,
}

/// #1578：C# Globals.LogDelay——攻击/施法后下线阻止时长（10s）
pub(crate) const LOGOUT_DELAY_MS: i64 = 10_000;

/// #1578：下线是否被阻止（C# MirConnection.LogOut：Envir.Time < Player.LogTime）
pub(crate) fn logout_blocked(now_ms: i64, block_until_ms: Option<i64>) -> bool {
    block_until_ms.is_some_and(|until| now_ms < until)
}

/// #1269：C# HumanObject.RefreshStats——AttackSpeed = 1400 - (Stat*60 + min(370, Level*14))，下限 550ms
fn player_attack_speed_ms(attack_speed_stat: i32, level: u16) -> i64 {
    let speed = 1400 - (attack_speed_stat * 60 + (level as i32 * 14).min(370));
    speed.max(550) as i64
}

/// #1519：C# GetRangeAttackPower——min 随距离缩小（Globals.MaxAttackRange=9，整数 floor）
fn range_attack_min_reduction(min: i32, range: i32) -> i32 {
    const MAX_RANGE: i32 = 9;
    let x = min * (MAX_RANGE - range) / MAX_RANGE;
    (min - x).max(0)
}

/// #1622：C# HumanObject.RangeAttack L2753——Chebyshev 距离 > Globals.MaxAttackRange(9) 即超范围
pub(crate) fn range_attack_out_of_range(caster_x: i32, caster_y: i32, target_x: i32, target_y: i32) -> bool {
    (target_x - caster_x).abs().max((target_y - caster_y).abs()) > 9
}

/// #1519：C# ApplyArcherState——MentalState 0/1/2 → 100 / 55+5*Lv / 80（返回伤害百分比）
fn archer_state_penalty(mental_state: u8, mental_lvl: u8) -> i32 {
    match mental_state {
        1 => 55 + mental_lvl as i32 * 5,
        2 => 80,
        _ => 100,
    }
}

/// #1519：C# chanceToHit = (100 + RangeAccuracyBonus(0) - (100/MaxAttackRange)*distance) * (focus?2:1)，<0 clamp 0
fn ranged_chance_to_hit(distance: i32, focus: bool) -> i32 {
    let base = 100 - (100 / 9) * distance; // RangeAccuracyBonus=0（Settings 默认）
    (base * if focus { 2 } else { 1 }).max(0)
}

/// #1515：C# TurnUndead 秒杀 threshold = ((Lv+1)<<3) + (Level - target.Level + 15)，clamp 0..100
fn turn_undead_threshold(player_level: u16, mon_level: i32, spell_level: u8) -> i32 {
    ((spell_level as i32 + 1) * 8 + player_level as i32 - mon_level + 15).clamp(0, 100)
}

/// #1269：C# CanAttack——麻痹/冰冻/眩晕中禁止攻击（Paralysis/LRParalysis/Frozen/Dazed）
fn attack_disabled_by_poison(poison_list: &[crate::combat::poison::Poison]) -> bool {
    use mir2_shared::enums::PoisonType;
    poison_list.iter().any(|p| {
        p.p_type.intersects(PoisonType::PARALYSIS)
            || p.p_type.intersects(PoisonType::LR_PARALYSIS)
            || p.p_type.intersects(PoisonType::FROZEN)
            || p.p_type.intersects(PoisonType::DAZED)
    })
}

/// #1287：C# CanCast——眩晕/迷惑/麻痹/冰冻中禁止施法（与 CanAttack 差异：
/// 施法查 Stun 不查 LRParalysis；攻击查 LRParalysis 不查 Stun）
fn cast_disabled_by_poison(poison_list: &[crate::combat::poison::Poison]) -> bool {
    use mir2_shared::enums::PoisonType;
    poison_list.iter().any(|p| {
        p.p_type.intersects(PoisonType::STUN)
            || p.p_type.intersects(PoisonType::DAZED)
            || p.p_type.intersects(PoisonType::PARALYSIS)
            || p.p_type.intersects(PoisonType::FROZEN)
    })
}

/// #1312：延迟弹道法术（C# CompleteMagic 结算时 `Attacked()>0 → LevelMagic`；
/// 命中才给经验，miss/0 伤害不给；FireBounce 每跳命中都给）
const DELAYED_HIT_EXP_SPELLS: [u8; 18] = [
    SPELL_FIREBALL,
    SPELL_GREAT_FIREBALL,
    SPELL_THUNDERBOLT,
    SPELL_FROST_CRUNCH,
    SPELL_VAMPIRISM,
    SPELL_FLAME_DISRUPTOR,
    SPELL_SOUL_FIREBALL,
    SPELL_METEOR_SHOWER,
    SPELL_FIRE_BOUNCE,
    SPELL_STRAIGHT_SHOT,
    SPELL_DOUBLE_SHOT,
    SPELL_BINDING_SHOT,
    SPELL_NAPALM_SHOT,
    SPELL_VAMPIRE_SHOT,
    SPELL_POISON_SHOT,
    SPELL_CRIPPLE_SHOT,
    SPELL_ELEMENTAL_SHOT,
    SPELL_CAT_TONGUE,
];

/// #1312：C# CompleteMagic——延迟弹道法术不在施法时给经验（移到命中结算）；
/// 基础攻击/ElectricShock 已处理也不加
fn should_grant_cast_exp(spell: u8, basic: bool, electric_handled: bool) -> bool {
    if basic || electric_handled || DELAYED_HIT_EXP_SPELLS.contains(&spell) {
        return false;
    }
    true
}

/// 采集请求（从 GateActor 转发）
pub struct HarvestRequest {
    pub session_id: u64,
    pub direction: u8,
}

impl Message<WorldAttackRequest> for WorldActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: WorldAttackRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => {
                warn!("Attack request for unknown session {}", msg.session_id);
                return;
            }
        };

        // 发送攻击请求到 PlayerActor，同时获取玩家属性用于伤害计算
        let attacker_state = record.actor_ref.ask(GetPlayerState).await.ok().flatten();
        if let Some(ref state) = attacker_state {
            if state.is_dead { return; }
            // #1269：C# CanAttack——麻痹/冰冻/眩晕中禁止攻击
            if attack_disabled_by_poison(&state.poison_list) {
                return;
            }
            // #1269：C# AttackTime = Envir.Time + AttackSpeed——攻击冷却服务端校验
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0);
            // #1506/#1508：AttackTime = 1400 - ((AttackSpeed*60) + min(370, Lv*14))；AttackSpeed 含 Haste/Fury buff 加成，Curse 再降 pct%
            let atk_spd_bonus = crate::combat::buff::get_stat_bonus(
                &state.buffs, &crate::combat::buff::BuffType::AttackSpeedBoost { percent: 0 },
            );
            let curse_pct = crate::combat::buff::get_stat_bonus(
                &state.buffs, &crate::combat::buff::BuffType::Curse { percent: 0 },
            );
            let total_atk_spd = (state.attack_speed + atk_spd_bonus) * (100 - curse_pct) / 100;
            let interval = player_attack_speed_ms(total_atk_spd, state.level);
            let last = self
                .player_last_attack_ms
                .get(&msg.session_id)
                .copied()
                .unwrap_or(0);
            if last > 0 && now_ms - last < interval {
                return;
            }
            self.player_last_attack_ms.insert(msg.session_id, now_ms);
            // #1578：C# HumanObject.Attack LogTime——攻击后 10s 内不可下线
            self.player_logout_block_ms.insert(msg.session_id, now_ms + LOGOUT_DELAY_MS);
        }

        // 攻击时自动下坐骑
        self.dismount_player(msg.session_id).await;

        // 攻击时打破隐身
        if self.invisible_sessions.remove(&msg.session_id) {
            if let Some(ref state) = attacker_state {
                let _ = record.actor_ref.ask(crate::actors::player::RemoveBuff {
                    buff_type: crate::combat::buff::BuffType::Invisibility,
                }).await;
                self.reveal_player_to_others(msg.session_id, state).await;
            }
        }

        if let (Some(ref state), Ok(Some(result))) = (attacker_state, record.actor_ref.ask(AttackRequest {
            session_id: msg.session_id,
            direction: msg.direction,
            spell: msg.spell,
        }).await) {
            // 广播 ObjectAttack 给其他玩家
            let others: Vec<_> = self.same_map_players(msg.session_id, state.map_index).await
                .into_iter()
                .map(|r| (r.actor_ref.clone(), r.session_id))
                .collect();

            let mut attack_body = Vec::new();
            attack_body.extend_from_slice(&result.object_id.to_le_bytes());
            attack_body.extend_from_slice(&(result.x as u32).to_le_bytes());
            attack_body.extend_from_slice(&(result.y as u32).to_le_bytes());
            attack_body.push(result.direction);
            attack_body.push(result.spell);
            attack_body.push(0u8); // level
            attack_body.push(0u8); // attack_type
            let packet = build_packet_bytes(mir2_shared::enums::ServerPacketIds::ObjectAttack as i16, &attack_body);
            // #1580：本地玩家自己的攻击动画（C# 客户端本地 ActionFeed；Bevy 依赖服务端回显）
            let _ = self.gate_ref.tell(SendToClient {
                session_id: msg.session_id,
                data: packet.clone(),
            }).await;

            // --- 检测是否命中怪物 ---
            // 计算攻击方向的前方位置
            let atk_dir = result.direction as usize % 8;
            let target_x = result.x + MON_DIR_DX[atk_dir];
            let target_y = result.y + MON_DIR_DY[atk_dir];

            // #77 诊断：攻击时打印玩家/目标格与附近怪物，核对客户端-服务端坐标同步
            debug!(
                "Attack {} at ({},{}) dir={} target=({},{})",
                state.name, state.x, state.y, result.direction, target_x, target_y
            );
            let nearby: Vec<String> = self
                .monsters
                .iter()
                .filter(|(_, m)| (m.x - state.x).abs() <= 5 && (m.y - state.y).abs() <= 5)
                .map(|(id, m)| format!("{}#{}@({},{}) hp={}", m.name, id, m.x, m.y, m.hp))
                .collect();
            if !nearby.is_empty() {
                debug!("Attack nearby: {}", nearby.join(", "));
            }

            // #471：主人当前召唤的宠物（协战目标分配用）
            let pet_ids: Vec<u32> = self.monsters.iter()
                .filter(|(_, m)| m.master_session == Some(msg.session_id))
                .map(|(id, _)| *id)
                .collect();
            let mut hit_monster = false;
            // HalfMoon/CrossHalfMoon 溅射目标（循环外应用，避免借用冲突）
            let mut halfmoon_splash: Vec<(u32, i32)> = Vec::new();
            // C# 弧/十字几何命中的格子（围绕玩家）
            let mut halfmoon_cells: Vec<(i32, i32)> = Vec::new();
            let mut primary_target_oid: u32 = 0; // 主目标 oid（溅射排除用）
            // #1256：近战命中给攻击技能经验（C# CompleteAttack LevelMagic）的触发标记
            let mut halfmoon_skill: Option<u8> = None;
            let mut mp_eater_triggered = false;
            let mut hemorrhage_triggered = false;
            for (oid, monster) in &mut self.monsters {
                // #1636：近战目标只在施法者同图（C# CurrentMap cell 语义）
                if monster.map_index != state.map_index {
                    continue;
                }
                let dist = (monster.x - target_x).abs() + (monster.y - target_y).abs();
                // #471：主人近战不攻击自己的召唤宠物（宠物是友方）
                if monster.master_session == Some(msg.session_id) {
                    continue;
                }
                // 近战只打正前方那一格（C# 语义）：dist==0 才命中。此前 <=1 会把
                // 攻击格旁边的守卫/怪物一并命中（#77 实测守卫被打死都不掉血挡住击杀）
                if dist == 0 {
                    // 命中怪物 - 使用完整战斗公式（命中/护甲/暴击/反伤/吸血/负面）
                    let attacker_stats = state.to_combat_stats();
                    let defender_stats = monster.to_combat_stats();
                    let mut raw_damage = combat_attack::get_attack_power(
                        attacker_stats.min_atk, attacker_stats.max_atk, attacker_stats.luck,
                    );
                    // C# Hemorrhage：武装状态（下次命中触发）时触发击伤害 = base × (0.2+0.05Lv)
                    let hemorrhage_armed = self.hemorrhage_armed.remove(&msg.session_id);
                    if hemorrhage_armed {
                        if let Some(magic) = state.magics.iter().find(|m| m.spell == (SPELL_HEMORRHAGE as i32 - 3)) {
                            let lv = magic.level as i32;
                            raw_damage = ((raw_damage as f32) * (0.2 + 0.05 * lv as f32)).max(1.0) as i32;
                        }
                    }
                    // #1451：LevelOffset = Level > attacker.Level ? 0 : min(10, attacker.Level - Level)
                    let level_offset = crate::combat::attack::level_offset(state.level, monster.level.max(0) as u16);
                    let attack_result = combat_attack::resolve_attack(
                        &attacker_stats, &defender_stats, raw_damage,
                        mir2_shared::enums::DefenceType::AcAgility, level_offset,
                    );
                    let damage = attack_result.damage;
                    monster.take_damage(damage);
                    monster.last_hitter_session = Some(msg.session_id);
                    self.pending_gather.push(msg.session_id);
                    monster.provoked = true;
                    // C# MonsterObject.Attacked：仅当无目标时锁定攻击者（Target == null 才设置）
                    if monster.target_session.is_none() {
                        monster.target_session = Some(msg.session_id);
                    }
                    // 施加战斗触发的 Poison（冰冻/毒攻），经 behavior.on_poison 过滤
                    for p in &attack_result.applied_poisons {
                        monster.try_apply_poison(*p);
                    }

                    // ===== 战士近战技能触发 =====
                    // #1517：Slaying（攻杀）——C# 无倍率配置 → GetDamage = base×1.0（主动命中无额外伤害）；
                    // 伤害来源是 RefreshSkills 被动 MaxDC +[5,6,7,8][Lv]（effective_max_attack 已计入）；
                    // 触发：跑动时 Random(12) <= Lv 武装，下一次攻击消费（C# HumanObject.cs:2956/3148）
                    let mut slaying_bonus = 0i32;
                    if let Some(lv) = find_attack_skill(&state.magics, SPELL_SLAYING).map(|m| m.level as i32) {
                        if fastrand::i32(0..12) <= lv {
                            debug!("Player {} Slaying triggered (level {})", result.object_id, lv);
                        }
                    }
                    // #312：FlamingSword —— C# Envir.cs MultiplierBase=1.4（无等级加成）：单次 1.4×
                    // Rust 主击已按 base 结算，此处追加 0.4× 近似合计 1.4×（防御只算一次）
                    let mut flaming_bonus = 0i32;
                    if let Some((expire, lv)) = self.flaming_sword.get(&msg.session_id).copied() {
                        self.flaming_sword.remove(&msg.session_id);
                        if self.tick_count < expire {
                            // C# Envir.cs FlamingSword：1.4+0.4Lv 单次（主击已计 base，追加 0.4+0.4Lv）
                            flaming_bonus = (damage as f32 * (0.4 + 0.4 * lv as f32)) as i32;
                            monster.take_damage(flaming_bonus);
                            monster.last_hitter_session = Some(msg.session_id);
                            debug!("Player {} FlamingSword bonus +{} on '{}' (#{})",
                                   result.object_id, flaming_bonus, monster.name, *oid);
                        }
                    }
                    // #318：TwinDrakeBlade/DoubleSlash —— 下一次近战攻击双段伤害（C# MultiplierBase=0.8/Bonus=0.1，一次性）
                    let mut second_hit = 0i32;
                    if let Some((expire, lv, kind)) = self.double_hit_melee.get(&msg.session_id).copied() {
                        self.double_hit_melee.remove(&msg.session_id);
                        if self.tick_count < expire {
                            second_hit = (damage as f32 * (0.8 + 0.1 * lv as f32)) as i32;
                            monster.take_damage(second_hit);
                            monster.last_hitter_session = Some(msg.session_id);
                            // TwinDrakeBlade 最终击：概率 Stun（C# HumanObject.cs:6803，Random(20)<=Lv+1）
                            if kind == 0 && fastrand::i32(0..20) <= lv as i32 + 1 {
                                crate::combat::poison::apply_poison(&mut monster.poison_list,
                                    crate::combat::poison::Poison::new(
                                        mir2_shared::enums::PoisonType::STUN, 2 + lv as u32, 0, 1000,
                                    ));
                                debug!("Player {} TwinDrakeBlade stunned '{}' ({}s)",
                                       result.object_id, monster.name, 2 + lv as u32);
                            }
                            let label = if kind == 0 { "TwinDrakeBlade" } else { "DoubleSlash" };
                            debug!("Player {} {} second hit +{} on '{}' (#{})",
                                   result.object_id, label, second_hit, monster.name, *oid);
                        }
                    }
                    // #448：FatalSword —— 被动：每次近战 10% 概率触发，下一击 +5*(Lv+1) 平伤
                    // （C# HumanObject.cs:3063 触发 / 6789 消费；defence=Agility 由 resolve 阶段近似）
                    let fatal_armed = self.fatal_sword_armed.remove(&msg.session_id);
                    if let Some(magic) = state.magics.iter().find(|m| m.spell == (SPELL_FATAL_SWORD as i32 - 3)) {
                        if !fatal_armed && fastrand::i32(0..10) == 0 {
                            self.fatal_sword_armed.insert(msg.session_id);
                            debug!("Player {} FatalSword armed", result.object_id);
                        }
                        if fatal_armed {
                            let fatal_bonus = 5 * (magic.level as i32 + 1); // C# GetPower = (MPowerBase 20/4)*(Lv+1)
                            monster.take_damage(fatal_bonus);
                            monster.last_hitter_session = Some(msg.session_id);
                            debug!("Player {} FatalSword bonus +{} on '{}' (#{})",
                                   result.object_id, fatal_bonus, monster.name, *oid);
                        }
                    }
                    // #345：MPEater —— 近战被动吸蓝（C# HumanObject.cs:3078）
                    if let Some(magic) = state.magics.iter().find(|m| m.spell == (SPELL_MPEATER as i32 - 3)) {
                        let lv = magic.level as i32;
                        let acc = state.accuracy;
                        let base_count = 1 + acc / 2;
                        let max_count = base_count + lv * 5;
                        let add = fastrand::i32(base_count..=(max_count.max(base_count)));
                        let count = self.mp_eater_count.entry(msg.session_id).or_insert(0);
                        *count += add;
                        debug!("Player {} MPEater count={} (add={})", result.object_id, *count, add);
                        if *count >= 100 {
                            mp_eater_triggered = true;
                            let add_mp = mp_eater_restore(lv, acc);
                            let _ = record.actor_ref.ask(crate::actors::player::AddMP { amount: add_mp }).await;
                            *count = 0;
                            debug!("Player {} MPEater restored {} MP", result.object_id, add_mp);
                        }
                    }
                    // #345：Hemorrhage —— 近战被动放血（C# HumanObject.cs:3110：count>=55 武装，下次命中触发）
                    if let Some(magic) = state.magics.iter().find(|m| m.spell == (SPELL_HEMORRHAGE as i32 - 3)) {
                        let lv = magic.level as i32;
                        let add = fastrand::i32(1..=(1 + lv * 2));
                        let count = self.hemorrhage_count.entry(msg.session_id).or_insert(0);
                        *count += add;
                        debug!("Player {} Hemorrhage count={} (add={})", result.object_id, *count, add);
                        if hemorrhage_armed {
                            hemorrhage_triggered = true;
                            // C#：武装命中 → 施放流血毒 + 复位
                            let duration = hemorrhage_duration(lv, state.luck).max(1) as u32;
                            let value = hemorrhage_value(state.effective_max_attack());
                            crate::combat::poison::apply_poison(
                                &mut monster.poison_list,
                                crate::combat::poison::Poison::new(
                                    mir2_shared::enums::PoisonType::BLEEDING, duration, value, 1000,
                                ),
                            );
                            *count = 0;
                            debug!("Player {} Hemorrhage bleeding on '{}' (dur={}s value={})",
                                   result.object_id, monster.name, duration, value);
                        } else if *count >= 55 {
                            // C#：武装（下次命中触发）
                            self.hemorrhage_armed.insert(msg.session_id);
                        }
                    }
                    // HalfMoon / CrossHalfMoon：C# 需 toggle 开启（HumanObject.cs:2929/3001）
                    // 倍率：HalfMoon 0.3+0.1Lv / CrossHalfMoon 0.4+0.1Lv（Envir.cs UpdateMagicInfo）
                    // #1256：magics 存 C# 编号，需 find_attack_skill 转换（此前直接比较 SharedRust 值永不匹配）
                    let halfmoon = find_attack_skill(&state.magics, SPELL_HALFMOON)
                        .filter(|m| m.toggled)
                        .map(|m| (SPELL_HALFMOON, m.level))
                        .or_else(|| {
                            find_attack_skill(&state.magics, SPELL_CROSS_HALFMOON)
                                .filter(|m| m.toggled)
                                .map(|m| (SPELL_CROSS_HALFMOON, m.level))
                        });
                    if let Some((skill_shared, lv)) = halfmoon {
                        let is_halfmoon = skill_shared == SPELL_HALFMOON;
                        let mult = if is_halfmoon {
                            0.3 + 0.1 * lv as f32
                        } else {
                            0.4 + 0.1 * lv as f32
                        };
                        let splash_dmg = ((damage as f32) * mult).max(1.0) as i32;
                        halfmoon_skill = Some(skill_shared);
                        // 标记触发；C# 几何：HalfMoon 从正前方逆时针起 4 格弧；CrossHalfMoon 周围 8 格（都跳过正前方）
                        halfmoon_splash.push((0, splash_dmg));
                        if halfmoon_cells.is_empty() {
                            let front = atk_dir;
                            if is_halfmoon {
                                for k in 0..4usize {
                                    let d = (front + 7 + k) % 8;
                                    if d == front {
                                        continue;
                                    }
                                    halfmoon_cells
                                        .push((state.x + MON_DIR_DX[d], state.y + MON_DIR_DY[d]));
                                }
                            } else {
                                for d in 0..8usize {
                                    if d == front {
                                        continue;
                                    }
                                    halfmoon_cells
                                        .push((state.x + MON_DIR_DX[d], state.y + MON_DIR_DY[d]));
                                }
                            }
                        }
                    }
                    let total_dmg = damage + slaying_bonus;
                    debug!("Player {} hit monster '{}' (#{}) for {} dmg (crit={}, slaying={}) (hp={}/{})",
                           result.object_id, monster.name, *oid, total_dmg, attack_result.is_critical, slaying_bonus, monster.hp, monster.max_hp);

                    // #1582：C# MonsterObject.Attacked——受击时转向攻击者（PointDirection）
                    monster.direction = crate::actors::world::ai::direction_towards(
                        monster.x, monster.y, result.x, result.y,
                    );

                    // 发送 ObjectStruck（受击动画）
                    let mut struck_body = Vec::new();
                    struck_body.extend_from_slice(&monster.object_id.to_le_bytes());
                    struck_body.extend_from_slice(&result.object_id.to_le_bytes());
                    struck_body.extend_from_slice(&(monster.x as u32).to_le_bytes());
                    struck_body.extend_from_slice(&(monster.y as u32).to_le_bytes());
                    struck_body.push(monster.direction);
                    let struck_packet = build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::ObjectStruck as i16, &struck_body);

                    // 发送 DamageIndicator（伤害数字）
                    let mut dmg_body = Vec::new();
                    dmg_body.extend_from_slice(&damage.to_le_bytes());
                    dmg_body.push(if attack_result.is_critical { 5u8 } else { 0u8 }); // damage_type: 0=Hit 5=Critical
                    dmg_body.extend_from_slice(&monster.object_id.to_le_bytes());
                    let dmg_packet = build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::DamageIndicator as i16, &dmg_body);

                    // 发送 ObjectHealth（百分比血条）
                    let percent = ((monster.hp.max(0) as f32 / monster.max_hp as f32) * 100.0) as u8;
                    let mut health_body = Vec::new();
                    health_body.extend_from_slice(&monster.object_id.to_le_bytes());
                    health_body.push(percent);
                    health_body.extend_from_slice(&3u16.to_le_bytes()); // expire（秒，C# ObjectHealth 语义，血条显示 3 秒）
                    let health_packet = build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::ObjectHealth as i16, &health_body);

                    // 广播给所有玩家
                    broadcast_to_map(&self.gate_ref, &self.players, monster.map_index, &struck_packet).await;
                    broadcast_to_map(&self.gate_ref, &self.players, monster.map_index, &dmg_packet).await;
                    broadcast_to_map(&self.gate_ref, &self.players, monster.map_index, &health_packet).await;

                    primary_target_oid = *oid;
                    hit_monster = true;
                    // #471：主人攻击的怪物作为所有宠物协战目标
                    for pid in &pet_ids {
                        self.pet_targets.insert(*pid, *oid);
                        debug!("Pet #{} target set -> monster #{}", pid, *oid);
                    }
                    break; // 一次只打一只
                }
            }

            // 应用 HalfMoon/CrossHalfMoon 溅射（循环外，避免借用冲突；C# 每格命中第一个目标）
            if !halfmoon_splash.is_empty() {
                let splash_dmg = halfmoon_splash[0].1;
                for (cx, cy) in &halfmoon_cells {
                    let mid = self.monsters.iter()
                        .find(|(id, m)| **id != primary_target_oid && m.hp > 0 && m.map_index == state.map_index && m.x == *cx && m.y == *cy)
                        .map(|(id, _)| *id);
                    if let Some(mid) = mid {
                        if let Some(sm) = self.monsters.get_mut(&mid) {
                            sm.take_damage(splash_dmg);
                            sm.last_hitter_session = Some(msg.session_id);
                            self.pending_gather.push(msg.session_id);
                            sm.provoked = true;
                            // C# MonsterObject.Attacked：仅当无目标时锁定攻击者
                            if sm.target_session.is_none() {
                                sm.target_session = Some(msg.session_id);
                            }
                        }
                    }
                }
            }

            // 武器耐久损耗（C# HumanObject.DamageWeapon：每次命中 Random(4)+1）
            if hit_monster {
                if let Some(record) = self.players.get(&msg.session_id) {
                    let broke = record.actor_ref.ask(crate::actors::player::DamageEquipment {
                        slot: EquipmentSlot::Weapon,
                        amount: (1 + fastrand::i32(0..4)) as u16,
                    }).await.unwrap_or(false);
                    if broke {
                        debug!("Player {} weapon broke!", result.object_id);
                        if let Some(state) = self.recalculate_and_set_stat_bonuses(msg.session_id).await {
                            self.broadcast_equipment_visuals(msg.session_id, &state).await;
                        }
                    }
                }
            }

            // #1256：C# CompleteAttack——近战命中给攻击技能经验（Random.Next(3)+1）
            if hit_monster {
                if ATTACK_SKILL_SPELLS.contains(&msg.spell) {
                    self.grant_attack_skill_exp(msg.session_id, msg.spell).await;
                } else if let Some(skill) = halfmoon_skill {
                    self.grant_attack_skill_exp(msg.session_id, skill).await;
                }
                if mp_eater_triggered {
                    self.grant_attack_skill_exp(msg.session_id, SPELL_MPEATER).await;
                }
                if hemorrhage_triggered {
                    self.grant_attack_skill_exp(msg.session_id, SPELL_HEMORRHAGE).await;
                }
            }

            // --- 玩家间伤害（仅在未命中怪物时） ---
            let mut pvp_halfmoon_skill: Option<u8> = None; // #1256：PvP 半月/十字触发技能
            let mut pvp_hit_skill: Option<u8> = None;      // #1256：PvP 本次命中技能（经验）
            if !hit_monster {
                for (other_actor, other_session) in others {
                    // 获取其他玩家位置做距离检测
                    if let Ok(Some(other_state)) = other_actor.ask(GetPlayerState).await {
                        // #1466：C# IsAttackTarget Dead——死亡玩家不可攻击
                        if other_state.is_dead { continue; }
                        // #1465：C# GMGameMaster——GM 保护模式不可攻击
                        if self.gm_protected.contains(&other_session) { continue; }
                        // 发送 ObjectAttack 动画（无论距离，C# Broadcast 与命中无关）
                        let _ = self.gate_ref.tell(SendToClient {
                            session_id: other_session,
                            data: packet.clone(),
                        }).await;

                        // #1623/#1636：C# HumanObject.Attack L2978——近战只命中正前方 1 格（同图）
                        if other_state.map_index != state.map_index { continue; }
                        let front_hit = other_state.x == target_x && other_state.y == target_y;
                        if front_hit {
                            // 攻击模式检查
                            if !can_attack_player(state, &other_state, &self.guild_wars) {
                                continue;
                            }

                            // 安全区保护：双方任一在安全区内则禁止伤害
                            let attacker_safe = self.maps.get(&state.map_index)
                                .map(|m| m.is_safe_zone(state.x, state.y))
                                .unwrap_or(false);
                            let target_safe = self.maps.get(&other_state.map_index)
                                .map(|m| m.is_safe_zone(other_state.x, other_state.y))
                                .unwrap_or(false);
                            if attacker_safe || target_safe {
                                continue;
                            }
                            // #1459：C# IsAttackTarget——CurrentMap.Info.NoFight 禁战地图不可攻击
                            if self.map_infos.get(&(state.map_index as i32)).map(|mi| mi.no_fight).unwrap_or(false)
                                || self.map_infos.get(&(other_state.map_index as i32)).map(|mi| mi.no_fight).unwrap_or(false)
                            {
                                continue;
                            }

                            // 使用完整战斗公式（玩家攻击玩家 PvP）
                            let attacker_stats = state.to_combat_stats();
                            let defender_stats = other_state.to_combat_stats();
                            let raw_damage = combat_attack::get_attack_power(
                                attacker_stats.min_atk, attacker_stats.max_atk, attacker_stats.luck,
                            );
                            // LevelOffset：防御方等级更高时为 0
                            let level_offset = if other_state.level > state.level {
                                0
                            } else {
                                (state.level - other_state.level).min(10) as u16
                            };
                            let attack_result = combat_attack::resolve_attack(
                                &attacker_stats, &defender_stats, raw_damage,
                                mir2_shared::enums::DefenceType::AcAgility, level_offset,
                            );
                            let damage = attack_result.damage;
                            // 施加战斗触发的 Poison 给目标玩家
                            if !attack_result.applied_poisons.is_empty() {
                                let _ = other_actor.ask(crate::actors::player::ApplyCombatPoisons {
                                    poisons: attack_result.applied_poisons,
                                }).await;
                            }
                            // C#：PvP 命中广播 ObjectStruck + DamageIndicator 给同图其他玩家
                    if damage > 0 {
                        self.broadcast_pvp_hit(
                            other_state.object_id, result.object_id,
                            other_state.x, other_state.y, other_state.direction, damage, other_state.map_index,
                            attack_result.is_critical,
                        ).await;
                    }
                    // PvP 近战技能（C# 对玩家同样走 melee 技能逻辑）
                    // TwinDrakeBlade/DoubleSlash 第二段
                    if let Some((expire, lv, kind)) = self.double_hit_melee.get(&msg.session_id).copied() {
                        self.double_hit_melee.remove(&msg.session_id);
                        if self.tick_count < expire {
                            let second = (damage as f32 * (0.8 + 0.1 * lv as f32)) as i32;
                            let _ = other_actor.ask(TakeDamage {
                                attacker_id: result.object_id,
                                attacker_session: msg.session_id,
                                damage: second,
                            }).await;
                            // #895：PvP 连击受击装备耐久损耗
                            self.damage_armor_on_pvp_hit(other_session).await;
                            // TwinDrakeBlade PvP 眩晕：需 PvpCanResistPoison 开启（C# 6803）
                            if kind == 0 && self.pvp_cfg.can_resist_poison && fastrand::i32(0..40) <= lv as i32 + 1 {
                                let _ = other_actor.ask(crate::actors::player::ApplyCombatPoisons {
                                    poisons: vec![crate::combat::poison::Poison::new(
                                        mir2_shared::enums::PoisonType::STUN, 2 + lv as u32, 0, 1000)],
                                }).await;
                                debug!("Player {} TwinDrakeBlade stunned player {}", result.object_id, other_session);
                            }
                            debug!("Player {} {} second hit +{} on player {}",
                                   result.object_id, if kind == 0 { "TwinDrakeBlade" } else { "DoubleSlash" },
                                   second, other_session);
                        }
                    }
                    // FlamingSword：1.4+0.4Lv 单次（追加 0.4+0.4Lv）
                    if let Some((expire, lv)) = self.flaming_sword.get(&msg.session_id).copied() {
                        self.flaming_sword.remove(&msg.session_id);
                        if self.tick_count < expire {
                            let bonus = (damage as f32 * (0.4 + 0.4 * lv as f32)) as i32;
                            let _ = other_actor.ask(TakeDamage {
                                attacker_id: result.object_id,
                                attacker_session: msg.session_id,
                                damage: bonus,
                            }).await;
                            // #895：PvP 追加伤害受击装备耐久损耗
                            self.damage_armor_on_pvp_hit(other_session).await;
                            debug!("Player {} FlamingSword bonus +{} on player {}", result.object_id, bonus, other_session);
                        }
                    }
                    // FatalSword 被动：10% 触发武装，下一击 +5*(Lv+1)
                    let fatal_armed = self.fatal_sword_armed.remove(&msg.session_id);
                    if fatal_armed {
                        if let Some(magic) = state.magics.iter().find(|m| m.spell == (SPELL_FATAL_SWORD as i32 - 3)) {
                            let bonus = 5 * (magic.level as i32 + 1);
                            let _ = other_actor.ask(TakeDamage {
                                attacker_id: result.object_id,
                                attacker_session: msg.session_id,
                                damage: bonus,
                            }).await;
                            // #895：PvP 追加伤害受击装备耐久损耗
                            self.damage_armor_on_pvp_hit(other_session).await;
                            debug!("Player {} FatalSword bonus +{} on player {}", result.object_id, bonus, other_session);
                        }
                    } else if state.magics.iter().any(|m| m.spell == (SPELL_FATAL_SWORD as i32 - 3))
                        && fastrand::i32(0..10) == 0 {
                        self.fatal_sword_armed.insert(msg.session_id);
                        debug!("Player {} FatalSword armed", result.object_id);
                    }
                    // HalfMoon/CrossHalfMoon PvP 溅射（C# 对玩家同样生效；toggle + 倍率 + 弧/十字几何）
                    // #1256：magics 存 C# 编号，需 find_attack_skill 转换
                    let halfmoon_pvp = find_attack_skill(&state.magics, SPELL_HALFMOON)
                        .filter(|m| m.toggled)
                        .map(|m| (SPELL_HALFMOON, m.level))
                        .or_else(|| find_attack_skill(&state.magics, SPELL_CROSS_HALFMOON)
                            .filter(|m| m.toggled)
                            .map(|m| (SPELL_CROSS_HALFMOON, m.level)));
                    if let Some((skill_shared, lv)) = halfmoon_pvp {
                        let is_halfmoon = skill_shared == SPELL_HALFMOON;
                        pvp_halfmoon_skill = Some(skill_shared);
                        let mult = if is_halfmoon {
                            0.3 + 0.1 * lv as f32
                        } else {
                            0.4 + 0.1 * lv as f32
                        };
                        let splash_dmg = ((damage as f32) * mult).max(1.0) as i32;
                        let front = atk_dir;
                        let mut cells: Vec<(i32, i32)> = Vec::new();
                        if is_halfmoon {
                            for k in 0..4usize {
                                let d = (front + 7 + k) % 8;
                                if d == front { continue; }
                                cells.push((state.x + MON_DIR_DX[d], state.y + MON_DIR_DY[d]));
                            }
                        } else {
                            for d in 0..8usize {
                                if d == front { continue; }
                                cells.push((state.x + MON_DIR_DX[d], state.y + MON_DIR_DY[d]));
                            }
                        }
                        let mut splash_hits: Vec<u64> = Vec::new();
                        for (sid, r) in &self.players {
                            if *sid == msg.session_id { continue; }
                            if let Ok(Some(s)) = r.actor_ref.ask(GetPlayerState).await {
                                if !s.is_dead && s.map_index == state.map_index && cells.contains(&(s.x, s.y)) {
                                    splash_hits.push(*sid);
                                }
                            }
                        }
                        for sid in &splash_hits {
                            if let Some(r) = self.players.get(sid) {
                                let _ = r.actor_ref.ask(TakeDamage {
                                    attacker_id: result.object_id,
                                    attacker_session: msg.session_id,
                                    damage: splash_dmg,
                                }).await;
                                // #895：PvP 溅射受击装备耐久损耗（C# Struck → DamageDura）
                                self.damage_armor_on_pvp_hit(*sid).await;
                            }
                        }
                        debug!("Player {} {} PvP splash dmg={} on {} players",
                               result.object_id, if skill_shared == SPELL_HALFMOON { "HalfMoon" } else { "CrossHalfMoon" },
                               splash_dmg, splash_hits.len());
                    }
                    // CounterAttack：受击方 7s 窗口激活时反击攻击者（C# HumanObject.cs 7212/7302）
                    if let Some((expire, lv)) = self.counter_attack.get(&other_session).copied() {
                        if self.tick_count <= expire {
                            self.counter_attack.remove(&other_session);
                            let counter_dmg = combat_attack::get_attack_power(
                                other_state.min_attack + other_state.bonus_min_attack,
                                other_state.max_attack + other_state.bonus_max_attack,
                                other_state.luck,
                            ).max(1);
                            let _ = record.actor_ref.ask(TakeDamage {
                                attacker_id: other_state.object_id,
                                attacker_session: other_session,
                                damage: counter_dmg,
                            }).await;
                            // #895：CounterAttack 反击受击装备耐久损耗（受害者 = 原攻击者）
                            self.damage_armor_on_pvp_hit(msg.session_id).await;
                            // 攻击者吃 Stun（Lv+1）秒
                            let _ = record.actor_ref.ask(crate::actors::player::ApplyCombatPoisons {
                                poisons: vec![crate::combat::poison::Poison::new(
                                    mir2_shared::enums::PoisonType::STUN, lv as u32 + 1, 0, 1000)],
                            }).await;
                            debug!("Player {} counter-attacked player {} ({} dmg)",
                                   other_session, msg.session_id, counter_dmg);
                        }
                    }
                    let pvp_died = other_actor.ask(TakeDamage {
                                attacker_id: result.object_id,
                                attacker_session: msg.session_id,
                                damage,
                            }).await.unwrap_or(false);
                    // #895：PvP 受击装备耐久损耗（C# Struck → DamageDura，命中即扣，含致死）
                    self.damage_armor_on_pvp_hit(other_session).await;
                    // #1256：记录 PvP 命中技能（每次攻击最多一次；C# CompleteAttack LevelMagic）
                    if pvp_hit_skill.is_none() {
                        pvp_hit_skill = if ATTACK_SKILL_SPELLS.contains(&msg.spell) {
                            Some(msg.spell)
                        } else {
                            pvp_halfmoon_skill
                        };
                    }
                    if pvp_died {
                                let died_packet = Self::build_object_died_packet(
                                    other_state.object_id, other_state.x, other_state.y, other_state.direction, 0u8);
                                for (sid, _) in &self.players {
                                    let _ = self.gate_ref.tell(SendToClient {
                                        session_id: *sid,
                                        data: died_packet.clone(),
                                    }).await;
                                }
                                self.handle_player_death_drop(other_session, other_state.x, other_state.y, other_state.map_index, true).await;

                                // 击杀玩家：增加 PK 值并广播名字颜色变化
                                let _ = record.actor_ref.ask(crate::actors::player::AddPkPoints { points: 100 }).await;
                                // #1751：C# Die——击杀/被击杀消息（MurderPlayer / MurderedByPlayer）
                                send_system_message(&self.gate_ref, msg.session_id,
                                    &format!("你谋杀了 {}", other_state.name));
                                send_system_message(&self.gate_ref, other_session,
                                    &format!("你被 {} 击杀了", record.name));
        // C# Die：击杀玩家 1/4 概率诅咒武器（Luck -1，Luck > -MaxLuck 时）
        if let Ok(Some(weapon)) = record.actor_ref.ask(crate::actors::player::GetEquipmentInfo {
            slot: crate::actors::inventory::EquipmentSlot::Weapon,
        }).await {
            if weapon.added_stats.get(mir2_shared::enums::Stat::Luck) > -10 && fastrand::i32(..4) == 0 { // C# Settings.MaxLuck = 10
                let _ = record.actor_ref.ask(crate::actors::player::AddWeaponLuck { delta: -1 }).await;
                send_system_message(&self.gate_ref, msg.session_id, "你的武器受到了诅咒！");
                debug!("Weapon cursed on player kill: {} -> {}", record.name, weapon.item_index);
            }
        }
                                // #921：逐观众广播名字颜色（C# BroadcastColourChange）
                                self.broadcast_viewer_colours(msg.session_id).await;
                                if let Some(r) = self.players.get_mut(&msg.session_id) {
                                    if let Ok(Some(attacker_state)) = record.actor_ref.ask(GetPlayerState).await {
                                        r.last_pk_points = attacker_state.pk_points;
                                    }
                                }
                            }
                            let hit_dist = (other_state.x - result.x).abs() + (other_state.y - result.y).abs();
                            debug!("Hit! {} damaged {} for {} (dist={}, crit={})",
                                   result.object_id, other_state.name, damage, hit_dist, attack_result.is_critical);
                        }
                    }
                }
                // #1256：C# CompleteAttack——PvP 命中给攻击技能经验（每次攻击最多一次）
                if let Some(skill) = pvp_hit_skill {
                    self.grant_attack_skill_exp(msg.session_id, skill).await;
                }
            } else {
                // 命中怪物时也要广播 ObjectAttack 给所有玩家
                for (_other_actor, other_session) in &self.players.iter().map(|(s, r)| (r.actor_ref.clone(), *s)).collect::<Vec<_>>() {
                    let _ = self.gate_ref.tell(SendToClient {
                        session_id: *other_session,
                        data: packet.clone(),
                    }).await;
                }
            }
        }
    }
}

// ============================================================
// 采集系统（Harvest：挖矿/采集）
// ============================================================

/// 矿脉状态（C# MineSpot：StonesLeft + LastRegenTick）
#[derive(Debug, Clone, Copy)]
pub(crate) struct MineSpotState {
    pub stones_left: u8,
    pub last_regen_tick: u64,
}

/// 矿脉最大储量（C# MineInfo.MaxStones 默认 80，Settings.cs Mine{i} 可配置）
const MINE_MAX_STONES: u8 = 80;
/// 矿脉再生间隔（C# MineInfo.SpotRegenRate 默认 5 分钟 = 3000 ticks @100ms）
const MINE_REGEN_TICKS: u64 = 3000;
/// 挖矿基础命中率（C# MineInfo.HitRate 默认 25，另加镐 Accuracy*10）
const MINE_HIT_RATE_BASE: i32 = 25;
/// Rubble 废墟持续时间（C# 5 分钟）
const RUBBLE_DURATION_MS: u64 = 300_000;

impl Message<HarvestRequest> for WorldActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: HarvestRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };

        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        if state.is_dead { return; }

        // #1657：C# Mining ActionTime=550ms（HumanObject）；服务端限流防无限挖矿
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        if let Some(last) = self.last_harvest_ms.get(&msg.session_id) {
            if now_ms - *last < 550 {
                return;
            }
        }
        self.last_harvest_ms.insert(msg.session_id, now_ms);

        let dir = msg.direction as usize % 8;
        let target_x = state.x + MON_DIR_DX[dir];
        let target_y = state.y + MON_DIR_DY[dir];

        debug!(
            "Harvest: {} session={} dir={} target=({}, {})",
            state.name, msg.session_id, dir, target_x, target_y
        );

        // 当前地图需为矿区（C# CurrentMap.Mine != null）
        let map_info = match self.map_infos.get(&(state.map_index as i32)).cloned() {
            Some(mi) => mi,
            None => return,
        };
        let mine_index = map_info.mine_index;
        if mine_index <= 0 {
            send_system_message(&self.gate_ref, msg.session_id, "这里没有什么可采集的");
            return;
        }

        // 目标格需在矿区范围内（C# MineSpot 判定）
        let in_mine_zone = map_info.mine_zones.iter().any(|z| {
            (target_x - z.x).abs() <= z.size && (target_y - z.y).abs() <= z.size
        });
        if !in_mine_zone {
            send_system_message(&self.gate_ref, msg.session_id, "这里不是矿区");
            return;
        }

        // 需装备可采矿的镐且耐久 > 0（C# Equipment[Weapon].Info.CanMine && CurrentDura > 0）
        let pickaxe_ok = state.inventory.get_equipment(EquipmentSlot::Weapon)
            .and_then(|item| self.item_infos.get(&item.item_index))
            .map(|info| (info.bool_flags & 0x10) != 0) // C# ItemInfo.CanMine
            .unwrap_or(false);
        if !pickaxe_ok {
            send_system_message(&self.gate_ref, msg.session_id, "你需要装备一把镐才能采矿");
            return;
        }
        let weapon_dura_ok = state.inventory.get_equipment(EquipmentSlot::Weapon)
            .map(|item| item.current_dura > 0)
            .unwrap_or(false);
        if !weapon_dura_ok {
            send_system_message(&self.gate_ref, msg.session_id, "你的镐耐久已耗尽");
            return;
        }

        // 广播 ObjectHarvest 给附近其他玩家
        let harvest_body = {
            let mut b = Vec::new();
            b.extend_from_slice(&state.object_id.to_le_bytes());
            b.extend_from_slice(&(target_x as i32).to_le_bytes());
            b.extend_from_slice(&(target_y as i32).to_le_bytes());
            b.push(msg.direction);
            build_packet_bytes(mir2_shared::enums::ServerPacketIds::ObjectHarvest as i16, &b)
        };
        for other in self.same_map_players(msg.session_id, state.map_index).await {
            let _ = self.gate_ref.tell(SendToClient {
                session_id: other.session_id,
                data: harvest_body.clone(),
            }).await;
        }

        // 矿脉储量：取/初始化（C# MineSpot.StonesLeft），枯竭则等待再生
        let spot_key = (state.map_index, target_x, target_y);
        {
            let spot = self.mine_spot_state.entry(spot_key).or_insert(MineSpotState {
                stones_left: fastrand::i32(0..MINE_MAX_STONES as i32) as u8,
                last_regen_tick: 0,
            });
            if spot.stones_left == 0 {
                if self.tick_count >= spot.last_regen_tick + MINE_REGEN_TICKS {
                    spot.stones_left = fastrand::i32(0..MINE_MAX_STONES as i32) as u8;
                    spot.last_regen_tick = self.tick_count;
                } else {
                    send_system_message(&self.gate_ref, msg.session_id, "这里的矿脉已枯竭，稍后再来");
                    return;
                }
            }
            spot.stones_left -= 1;
        }

        // 命中判定（C# Random(100) < HitRate + Accuracy*10；命中才出废墟/掉落/耗耐久）
        // C# Random(100) < (HitRate + Weapon.GetTotal(Accuracy)*10)；accuracy 含装备/技能加成
        let hit = fastrand::i32(0..100) < MINE_HIT_RATE_BASE + state.accuracy * 10;
        let mut result_msg = "没有挖到东西".to_string();
        if hit {
            // Rubble 废墟：玩家脚下创建/刷新（C# CurrentLocation 格，5 分钟）
            let rubble_oid = if let Some(existing) = self.spell_objects.values_mut().find(|so| {
                so.spell == mir2_shared::enums::Spell::Rubble
                    && so.map_index == state.map_index
                    && so.x == state.x && so.y == state.y
            }) {
                // 已有废墟：刷新过期时间（C# Rubble.ExpireTime = now + 5min）
                existing.expires_at_ms = RUBBLE_DURATION_MS;
                existing.object_id
            } else {
                let oid = self.alloc_object_id();
                self.spell_objects.insert(oid, spell::SpellObject::new(
                    oid,
                    mir2_shared::enums::Spell::Rubble,
                    0, 0,
                    state.map_index,
                    state.x, state.y,
                    RUBBLE_DURATION_MS,
                    0,
                    60_000,
                    0,
                    1,
                ));
                oid
            };
            if rubble_oid != 0 {
                // 广播 ObjectSpell(Rubble) 视觉（自己 + 同图玩家）
                let object_spell = mir2_shared::packets::server::magic_combat::ObjectSpell {
                    object_id: rubble_oid,
                    location_x: state.x,
                    location_y: state.y,
                    spell: mir2_shared::enums::Spell::Rubble,
                };
                let mut ob = Vec::new();
                if object_spell.write_body(&mut ob).is_ok() {
                    let pkt = build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::ObjectSpell as i16, &ob);
                    for (sid, r) in &self.players {
                        if let Ok(Some(os)) = r.actor_ref.ask(GetPlayerState).await {
                            if os.map_index == state.map_index {
                                let _ = self.gate_ref.tell(SendToClient {
                                    session_id: *sid, data: pkt.clone(),
                                }).await;
                            }
                        }
                    }
                }
            }

            // 掉落判定（保留现有按矿种的掉落表）
            let roll = fastrand::i32(0..100);
            let (drop_item_index, drop_count, drop_name) = match mine_index {
                1 if roll < 40 => (500, 1 + (roll % 3) as u16, "铁矿石"),
                1 if roll < 65 => (503, 1, "铜矿石"),
                1 if roll < 70 => (504, 1, "银矿石"),
                1 if roll < 71 => (505, 1, "黑铁矿石"),
                2 if roll < 40 => (501, 1, "金矿石"),
                2 if roll < 60 => (504, 1 + (roll % 2) as u16, "银矿石"),
                2 if roll < 70 => (506, 1, "铂金矿石"),
                2 if roll < 75 => (507, 1, "红宝石原石"),
                3 if roll < 20 => (508, 1, "软玉原石"),
                3 if roll < 35 => (509, 1, "紫水晶原石"),
                3 if roll < 40 => (510, 1, "钻石原石"),
                3 if roll < 43 => (511, 1, "蓝宝石原石"),
                _ => (0, 0, ""),
            };
            if drop_item_index > 0 {
                let item_name = self.item_infos.get(&drop_item_index)
                    .map(|i| i.name.clone())
                    .unwrap_or_else(|| drop_name.to_string());
                let item = mir2_shared::data::item::UserItem {
                    item_index: drop_item_index,
                    count: drop_count,
                    ..Default::default()
                };
                let _ = record.actor_ref.ask(crate::actors::player::AddItemToInventory { item }).await;
                result_msg = format!("采集成功！获得了 {} x{}", item_name, drop_count);
            } else {
                result_msg = "采集成功，但这次什么也没有挖到".to_string();
            }

            // 镐耐久消耗（C# DamageItem(weapon, 5+Random(15))）
            let _ = record.actor_ref.ask(crate::actors::player::DamageEquipment {
                slot: EquipmentSlot::Weapon,
                amount: (5 + fastrand::i32(0..15)) as u16,
            }).await;
        }

        // 延迟发送 ObjectHarvested 视觉包
        let object_id = state.object_id;
        let gate_ref = self.gate_ref.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(1500)).await;
            let mut b = Vec::new();
            b.extend_from_slice(&object_id.to_le_bytes());
            b.extend_from_slice(&(target_x as i32).to_le_bytes());
            b.extend_from_slice(&(target_y as i32).to_le_bytes());
            b.push(msg.direction);
            let packet = build_packet_bytes(
                mir2_shared::enums::ServerPacketIds::ObjectHarvested as i16, &b,
            );
            let _ = gate_ref.tell(SendToClient {
                session_id: msg.session_id,
                data: packet,
            }).await;
        });
        send_system_message(&self.gate_ref, msg.session_id, &result_msg);
        debug!("Harvest: {} mine_index={} spot({},{}) hit={} msg={}",
               state.name, mine_index, target_x, target_y, hit, result_msg);
    }
}

/// 查看玩家信息
pub struct InspectPlayerRequest {
    pub session_id: u64,
    pub target_id: u32,
    /// 排行榜查看（C# Inspect.Ranking）
    pub ranking: bool,
    /// 排行榜查看时离线玩家回查（Rust 无持久化角色 id，用名字查 DB）
    pub name: String,
}

impl Message<InspectPlayerRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: InspectPlayerRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let mut target_state: Option<crate::actors::player::PlayerState> = None;
        for r in self.players.values() {
            if let Ok(Some(s)) = r.actor_ref.ask(GetPlayerState).await {
                if s.object_id == msg.target_id {
                    target_state = Some(s);
                    break;
                }
            }
        }

        let Some(target) = target_state else {
            // 排行榜查看：在线找不到 → 按名字查 DB 返回基础信息（C# 离线同样可看）
            if msg.ranking && !msg.name.is_empty() {
                match sqlx::query(
                    "SELECT name, class, gender, level, guild_name FROM characters WHERE name = ?",
                )
                .bind(&msg.name)
                .fetch_optional(&self.db_pool)
                .await
                {
                    Ok(Some(row)) => {
                        use sqlx::Row;
                        let name: String = row.get("name");
                        let class: i32 = row.get("class");
                        let gender: i32 = row.get("gender");
                        let level: i32 = row.get("level");
                        let guild: Option<String> = row.get("guild_name");
                        send_basic_inspect_packet(
                            &self.gate_ref,
                            msg.session_id,
                            &name,
                            guild.as_deref().unwrap_or(""),
                            level as u16,
                            class as u8,
                            gender as u8,
                        );
                        return;
                    }
                    _ => {}
                }
            }
            send_system_message(&self.gate_ref, msg.session_id, "找不到目标玩家");
            return;
        };

        // 发送 PlayerInspect 包
        send_inspect_packet(&self.gate_ref, msg.session_id, &target);
    }
}
/// 观察玩家
pub struct ObservePlayerRequest {
    pub session_id: u64,
    pub target_id: u32,
}

impl Message<ObservePlayerRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: ObservePlayerRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let mut target_state: Option<crate::actors::player::PlayerState> = None;
        for r in self.players.values() {
            if let Ok(Some(s)) = r.actor_ref.ask(GetPlayerState).await {
                if s.object_id == msg.target_id {
                    target_state = Some(s);
                    break;
                }
            }
        }

        let Some(target) = target_state else {
            return;
        };

        // #1671：C# Envir.Observe——目标 AllowObserve 设置校验（Envir.cs:5183）
        if !target.allow_observe {
            debug!("Observe rejected: target {} has AllowObserve off", target.name);
            return;
        }

        // Send AllowObserve(true)
        let mut allow_body = Vec::new();
        allow_body.push(1u8);
        let _ = self.gate_ref.tell(SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::AllowObserve as i16, &allow_body),
        }).await;

        // Send PlayerInspect with target info
        send_inspect_packet(&self.gate_ref, msg.session_id, &target);
    }
}

/// 城镇复活请求
pub struct TownReviveRequest {
    pub session_id: u64,
}

impl Message<TownReviveRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: TownReviveRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };

        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        if !state.is_dead { return; }

        // 复活：重置 HP/MP 到最大值（C# PlayerObject.TownRevive）：
        // - PKPoints >= 200 → PK 城（Settings.PKTownMapName="3" / (848,677)），PK 城地图缺失则回绑定点
        // - 否则回绑定点；绑定点无效回退当前地图安全区出生点
        let (revive_map, spawn_x, spawn_y) = if state.pk_points >= 200 {
            let pk_town_map = self.map_infos.values()
                .find(|m| {
                    let f = m.file_name.to_lowercase();
                    f == "3" || f.starts_with("3.") || f.contains("mongchon") || f.contains("pranja")
                })
                .map(|m| m.index as u16);
            match pk_town_map {
                Some(m) => (m, PK_TOWN_X, PK_TOWN_Y),
                None => self.default_revive_spot(&state),
            }
        } else {
            self.default_revive_spot(&state)
        };

        let _ = record.actor_ref.ask(crate::actors::player::RevivePlayer {
            x: spawn_x,
            y: spawn_y,
            map_index: revive_map,
        }).await;

        // 发送 HealthChanged 通知
        let mut health_body = Vec::new();
        health_body.extend_from_slice(&(state.max_hp as u32).to_le_bytes());
        health_body.extend_from_slice(&(state.max_mp as u32).to_le_bytes());
        let _ = self.gate_ref.tell(crate::gate::actor::SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::HealthChanged as i16, &health_body),
        }).await;

        // 发送 Revived 包（C# S.Revived，空 body）：客户端靠它清除死亡状态恢复输入，
        // 只有 HealthChanged 不够——#55 实测客户端一直处于死亡状态
        let _ = self.gate_ref.tell(crate::gate::actor::SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::Revived as i16, &[]),
        }).await;
        // ObjectRevived 广播：其他玩家看到复活动画
        let mut obj_body = Vec::new();
        obj_body.extend_from_slice(&state.object_id.to_le_bytes());
        obj_body.push(1u8); // effect
        let revived_packet = build_packet_bytes(mir2_shared::enums::ServerPacketIds::ObjectRevived as i16, &obj_body);
        // #1686：复生广播只发同图玩家（C# CurrentMap）
        broadcast_to_map(&self.gate_ref, &self.players, state.map_index, &revived_packet).await;

        debug!("TownRevive: {} revived at map {} ({}, {})", state.name, revive_map, spawn_x, spawn_y);
    }
}

/// 远程攻击请求（同普通攻击，但带目标位置）
pub struct RangeAttackRequest {
    pub session_id: u64,
    pub direction: u8,
    pub target_id: u32,
    pub target_x: i32,
    pub target_y: i32,
}

impl Message<RangeAttackRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: RangeAttackRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        if state.is_dead { return; }
        // #1622：C# HumanObject.RangeAttack L2749-2752——非弓手职业不可远程攻击
        // （RealItem.Shape / ClassWeaponCount == 2 即 Archer 武器）
        if state.class != mir2_shared::enums::MirClass::Archer {
            return;
        }
        // #1269：C# CanAttack——麻痹/冰冻/眩晕中禁止远程攻击
        if attack_disabled_by_poison(&state.poison_list) {
            return;
        }
        // #1269：C# RangeAttack 同样受 AttackTime 冷却约束（与近战共享）
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        // #1506/#1508：AttackTime = 1400 - ((AttackSpeed*60) + min(370, Lv*14))；AttackSpeed 含 Haste/Fury buff 加成，Curse 再降 pct%
        let atk_spd_bonus = crate::combat::buff::get_stat_bonus(
            &state.buffs, &crate::combat::buff::BuffType::AttackSpeedBoost { percent: 0 },
        );
        let curse_pct = crate::combat::buff::get_stat_bonus(
            &state.buffs, &crate::combat::buff::BuffType::Curse { percent: 0 },
        );
        let total_atk_spd = (state.attack_speed + atk_spd_bonus) * (100 - curse_pct) / 100;
        let interval = player_attack_speed_ms(total_atk_spd, state.level);
        let last = self
            .player_last_attack_ms
            .get(&msg.session_id)
            .copied()
            .unwrap_or(0);
        if last > 0 && now_ms - last < interval {
            return;
        }
        self.player_last_attack_ms.insert(msg.session_id, now_ms);
        // #1578：C# HumanObject.RangeAttack LogTime——远程攻击后 10s 内不可下线
        self.player_logout_block_ms.insert(msg.session_id, now_ms + LOGOUT_DELAY_MS);

        // 记录玩家当前攻击目标（C# HumanObject.TargetID；宠物 FocusMasterTarget 用）
        if msg.target_id != 0 {
            self.player_targets.insert(msg.session_id, msg.target_id);
        }

        let object_id = state.object_id;
        let target_x = msg.target_x;
        let target_y = msg.target_y;
        // #1519：C# MaxDistance（Chebyshev）——Focus/距离缩放/命中率共用
        let attack_dist = (target_x - state.x).abs().max((target_y - state.y).abs());
        // #1622：C# HumanObject.RangeAttack L2753——目标超 Globals.MaxAttackRange(9) 拒绝
        if range_attack_out_of_range(state.x, state.y, target_x, target_y) {
            send_system_message(&self.gate_ref, msg.session_id, "目标超出攻击范围");
            return;
        }
        // C# Focus：Random(5) <= Lv → 命中率×2（HumanObject.cs:2804）
        let focus = state.magics.iter()
            .find(|m| m.spell == (mir2_shared::enums::Spell::Focus as i32 - 3))
            .map(|m| fastrand::i32(0..5) <= m.level as i32)
            .unwrap_or(false);
        // C# ApplyArcherState：MentalState 0/1/2 → 100 / 55+5*Lv / 80
        let mental_lvl = state.magics.iter()
            .find(|m| m.spell == (mir2_shared::enums::Spell::MentalState as i32 - 3))
            .map(|m| m.level)
            .unwrap_or(0);
        let archer_penalty = archer_state_penalty(
            self.mental_state.get(&msg.session_id).copied().unwrap_or(0),
            mental_lvl,
        );
        let ranged_chance = ranged_chance_to_hit(attack_dist, focus);

        // C# HumanObject.RangeAttack（HumanObject.cs:2745）：
        //   - Broadcast(S.ObjectRangeAttack{...}) 给其他玩家（拉弓动作，Broadcast 排除攻击者）
        //   - Enqueue(S.RangeAttack{TargetID, Target, Spell}) 给攻击者（弹道表现）
        let others: Vec<_> = self.same_map_players(msg.session_id, state.map_index).await
            .into_iter()
            .collect();
        let mut range_body = Vec::new();
        range_body.extend_from_slice(&object_id.to_le_bytes());
        range_body.extend_from_slice(&(state.x as u32).to_le_bytes());
        range_body.extend_from_slice(&(state.y as u32).to_le_bytes());
        range_body.push(msg.direction);
        range_body.extend_from_slice(&msg.target_id.to_le_bytes());
        range_body.extend_from_slice(&(target_x as u32).to_le_bytes());
        range_body.extend_from_slice(&(target_y as u32).to_le_bytes());
        range_body.push(0u8); // Type（C# 玩家恒 AttackRange1）
        range_body.push(0u8); // spell
        range_body.push(0u8); // spell_level
        let range_packet = build_packet_bytes(
            mir2_shared::enums::ServerPacketIds::ObjectRangeAttack as i16, &range_body);
        for other in &others {
            let _ = self.gate_ref.tell(SendToClient {
                session_id: other.session_id,
                data: range_packet.clone(),
            }).await;
        }
        // #1580：本地玩家自己的拉弓动画（C# 本地 ActionFeed；Bevy 依赖 ObjectRangeAttack 回显）
        let _ = self.gate_ref.tell(SendToClient {
            session_id: msg.session_id,
            data: range_packet.clone(),
        }).await;
        // S.RangeAttack 弹道（客户端 PendingEffect::Projectile：从玩家飞向目标）
        let mut proj_body = Vec::new();
        proj_body.extend_from_slice(&msg.target_id.to_le_bytes());
        proj_body.extend_from_slice(&(target_x as u32).to_le_bytes());
        proj_body.extend_from_slice(&(target_y as u32).to_le_bytes());
        proj_body.push(0u8); // spell
        let _ = self.gate_ref.tell(SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::RangeAttack as i16, &proj_body),
        }).await;

        // #1560：C# DelayedAction——命中/未命中都预约到箭矢飞行后结算（HumanObject.cs:2827-2836）
        // 目标怪物解析（客户端 C.RangeAttack.TargetID = 怪物 object_id）
        let attacker_stats = state.to_combat_stats();
        let targeted_monster_level = if msg.target_id != 0 {
            self.monsters.get(&msg.target_id).map(|m| m.level.max(0) as u16)
        } else {
            None
        };

        if let Some(monster_level) = targeted_monster_level {
            // 施法时掷命中（C# Envir.Random.Next(100) < chanceToHit）
            let hit = fastrand::i32(0..100) < ranged_chance;
            // 施法时算伤害（C# GetRangeAttackPower + ApplyArcherState；防御在飞行后按目标结算）
            let raw_damage = if hit {
                let eff_min = range_attack_min_reduction(attacker_stats.min_atk, attack_dist);
                let mut raw = combat_attack::get_attack_power(
                    eff_min, attacker_stats.max_atk, attacker_stats.luck,
                );
                raw = raw * archer_penalty / 100;
                raw
            } else {
                0
            };
            let level_offset = crate::combat::attack::level_offset(state.level, monster_level);
            let fire_at_tick = self.tick_count + range_flight_ticks(attack_dist);
            self.pending_range_completions.push(PendingRangeCompletion {
                fire_at_tick,
                session_id: msg.session_id,
                attacker_object_id: object_id,
                attacker_x: state.x,
                attacker_y: state.y,
                direction: msg.direction,
                target: RangeTarget::Monster(msg.target_id),
                target_x,
                target_y,
                attacker_stats,
                raw_damage,
                level_offset,
                hit,
            });
            debug!(
                "RangeAttack scheduled: {} -> monster {} hit={} dmg={} fire_at_tick={} (delay {} ticks)",
                state.name, msg.target_id, hit, raw_damage, fire_at_tick, range_flight_ticks(attack_dist)
            );
        } else {
            // #1566：PvP 目标（玩家）——施法时解析 + 预约延迟结算（与怪物目标一致，C# DelayedAction）
            // 优先按 target_id（客户端 C.RangeAttack.TargetID = 玩家 object_id），其次目标格 1 格内
            let mut target_player: Option<(u64, u16)> = None;
            for other in &others {
                if let Ok(Some(other_state)) = other.actor_ref.ask(GetPlayerState).await {
                    let id_match = msg.target_id != 0 && other_state.object_id == msg.target_id;
                    let cell_match = msg.target_id == 0
                        && other_state.map_index == state.map_index
                        && (other_state.x - target_x).abs() + (other_state.y - target_y).abs() <= 1;
                    if !id_match && !cell_match {
                        continue;
                    }
                    // 施法时基础校验（死亡/GM/安全区/禁战/攻击模式；飞行后重校验）
                    if other_state.is_dead { continue; }
                    if self.gm_protected.contains(&other.session_id) { continue; }
                    if !can_attack_player(&state, &other_state, &self.guild_wars) { continue; }
                    let attacker_safe = self.maps.get(&state.map_index)
                        .map(|m| m.is_safe_zone(state.x, state.y))
                        .unwrap_or(false);
                    let target_safe = self.maps.get(&other_state.map_index)
                        .map(|m| m.is_safe_zone(other_state.x, other_state.y))
                        .unwrap_or(false);
                    if attacker_safe || target_safe { continue; }
                    if self.map_infos.get(&(state.map_index as i32)).map(|mi| mi.no_fight).unwrap_or(false)
                        || self.map_infos.get(&(other_state.map_index as i32)).map(|mi| mi.no_fight).unwrap_or(false)
                    { continue; }
                    target_player = Some((other.session_id, other_state.level));
                    break;
                }
            }

            if let Some((defender_session, defender_level)) = target_player {
                // 施法时掷命中 + 算伤害（防御在飞行后按目标当前状态结算）
                let hit = fastrand::i32(0..100) < ranged_chance;
                let raw_damage = if hit {
                    let eff_min = range_attack_min_reduction(attacker_stats.min_atk, attack_dist);
                    let mut raw = combat_attack::get_attack_power(
                        eff_min, attacker_stats.max_atk, attacker_stats.luck,
                    );
                    raw = raw * archer_penalty / 100;
                    raw
                } else {
                    0
                };
                let level_offset = if defender_level > state.level {
                    0
                } else {
                    (state.level - defender_level).min(10) as u16
                };
                let fire_at_tick = self.tick_count + range_flight_ticks(attack_dist);
                self.pending_range_completions.push(PendingRangeCompletion {
                    fire_at_tick,
                    session_id: msg.session_id,
                    attacker_object_id: object_id,
                    attacker_x: state.x,
                    attacker_y: state.y,
                    direction: msg.direction,
                    target: RangeTarget::Player(defender_session),
                    target_x,
                    target_y,
                    attacker_stats,
                    raw_damage,
                    level_offset,
                    hit,
                });
                debug!(
                    "RangeAttack scheduled (PvP): {} -> player {} hit={} dmg={} fire_at_tick={} (delay {} ticks)",
                    state.name, defender_session, hit, raw_damage, fire_at_tick, range_flight_ticks(attack_dist)
                );
            }
        }
    }
}

impl WorldActor {
    /// #1566：远程攻击 PvP 结算（箭矢飞行后调用；C# DelayedType.Damage → Attacked）
    ///
    /// 飞行后按双方当前状态重校验（死亡/GM/安全区/禁战/攻击模式），
    /// 按目标当前防御结算伤害，处理 TakeDamage / 受击反馈 / 死亡（PK/武器诅咒/观众颜色/掉落）。
    /// 返回是否真正命中（false = 目标失效/落空，调用方不发 Miss 飘字）。
    pub(crate) async fn resolve_ranged_pvp_hit(
        &mut self,
        attacker_session: u64,
        attacker_object_id: u32,
        attacker_stats: crate::combat::attack::CombatStats,
        defender_session: u64,
        raw_damage: i32,
        level_offset: u16,
        direction: u8,
        target_x: i32,
        target_y: i32,
    ) -> bool {
        let (Some(attacker_record), Some(defender_record)) = (
            self.players.get(&attacker_session).cloned(),
            self.players.get(&defender_session).cloned(),
        ) else {
            return false;
        };
        let (Ok(Some(attacker_state)), Ok(Some(defender_state))) = (
            attacker_record.actor_ref.ask(GetPlayerState).await,
            defender_record.actor_ref.ask(GetPlayerState).await,
        ) else {
            return false;
        };
        if attacker_state.is_dead || defender_state.is_dead {
            return false;
        }
        // #1645：C# CompleteRangeAttack——目标跨图则箭矢落空（CurrentMap 隔离）
        if attacker_state.map_index != defender_state.map_index {
            return false;
        }
        if self.gm_protected.contains(&defender_session) {
            return false;
        }
        if !can_attack_player(&attacker_state, &defender_state, &self.guild_wars) {
            return false;
        }
        let attacker_safe = self.maps.get(&attacker_state.map_index)
            .map(|m| m.is_safe_zone(attacker_state.x, attacker_state.y))
            .unwrap_or(false);
        let target_safe = self.maps.get(&defender_state.map_index)
            .map(|m| m.is_safe_zone(defender_state.x, defender_state.y))
            .unwrap_or(false);
        if attacker_safe || target_safe {
            return false;
        }
        if self.map_infos.get(&(attacker_state.map_index as i32)).map(|mi| mi.no_fight).unwrap_or(false)
            || self.map_infos.get(&(defender_state.map_index as i32)).map(|mi| mi.no_fight).unwrap_or(false)
        {
            return false;
        }

        let defender_stats = defender_state.to_combat_stats();
        let attack_result = combat_attack::resolve_attack(
            &attacker_stats, &defender_stats, raw_damage,
            mir2_shared::enums::DefenceType::AcAgility, level_offset,
        );
        let damage = attack_result.damage;
        if !attack_result.applied_poisons.is_empty() {
            let _ = defender_record.actor_ref.ask(crate::actors::player::ApplyCombatPoisons {
                poisons: attack_result.applied_poisons,
            }).await;
        }
        // C#：PvP 命中广播 ObjectStruck + DamageIndicator 给同图其他玩家
        if damage > 0 {
            self.broadcast_pvp_hit(
                defender_state.object_id, attacker_object_id,
                defender_state.x, defender_state.y, defender_state.direction, damage, defender_state.map_index,
                attack_result.is_critical,
            ).await;
        }
        let pvp_died = defender_record.actor_ref.ask(TakeDamage {
                    attacker_id: attacker_object_id,
                    attacker_session,
                    damage,
                }).await.unwrap_or(false);
        // #895：PvP 受击装备耐久损耗（C# Struck → DamageDura，命中即扣，含致死）
        self.damage_armor_on_pvp_hit(defender_session).await;
        if pvp_died {
                    // 目标死亡处理
                    let died_packet = Self::build_object_died_packet(
                        defender_state.object_id, defender_state.x, defender_state.y, defender_state.direction, 0u8);
                    for (sid, _) in &self.players {
                        let _ = self.gate_ref.tell(SendToClient {
                            session_id: *sid,
                            data: died_packet.clone(),
                        }).await;
                    }
                    self.handle_player_death_drop(defender_session, defender_state.x, defender_state.y, defender_state.map_index, true).await;

                    // 增加 PK 值
                    let _ = attacker_record.actor_ref.ask(crate::actors::player::AddPkPoints { points: 100 }).await;
                    // #1751：C# Die——击杀/被击杀消息（MurderPlayer / MurderedByPlayer）
                    send_system_message(&self.gate_ref, attacker_session,
                        &format!("你谋杀了 {}", defender_state.name));
                    send_system_message(&self.gate_ref, defender_session,
                        &format!("你被 {} 击杀了", attacker_record.name));
        // C# Die：击杀玩家 1/4 概率诅咒武器（Luck -1，Luck > -MaxLuck 时）
        if let Ok(Some(weapon)) = attacker_record.actor_ref.ask(crate::actors::player::GetEquipmentInfo {
            slot: crate::actors::inventory::EquipmentSlot::Weapon,
        }).await {
            if weapon.added_stats.get(mir2_shared::enums::Stat::Luck) > -10 && fastrand::i32(..4) == 0 { // C# Settings.MaxLuck = 10
                let _ = attacker_record.actor_ref.ask(crate::actors::player::AddWeaponLuck { delta: -1 }).await;
                send_system_message(&self.gate_ref, attacker_session, "你的武器受到了诅咒！");
                debug!("Weapon cursed on player kill: {} -> {}", attacker_record.name, weapon.item_index);
            }
        }
                    // #921：逐观众广播名字颜色（C# BroadcastColourChange）
                    self.broadcast_viewer_colours(attacker_session).await;
                    if let Some(r) = self.players.get_mut(&attacker_session) {
                        if let Ok(Some(attacker_state2)) = attacker_record.actor_ref.ask(GetPlayerState).await {
                            r.last_pk_points = attacker_state2.pk_points;
                        }
                    }
                }
        debug!("RangeAttack PvP resolve: {} damaged {} for {}", attacker_state.name, defender_state.name, damage);
        true
    }
}

/// 弹道法术的延迟结算项（对齐 C# DelayedAction(DelayedType.Magic, fireTime, ...)）
///
/// 法师弹道类法术（FireBall/ThunderBolt/FrostCrunch/Vampirism）施法时
/// 不立即结算，而是按距离计算飞行时间后推入此队列，由主 tick 在到期时结算。
#[derive(Debug, Clone)]
pub struct PendingSpellCompletion {
    /// 到期 tick（WorldActor.tick_count）
    pub fire_at_tick: u64,
    pub session_id: u64,
    /// 法术原始值（u8，对应 Spell 枚举判别值）
    pub spell: u8,
    /// 目标 object_id（弹道类）
    pub target_id: u32,
    /// 目标快照位置（防移动 miss 校验用）
    pub target_x: i32,
    pub target_y: i32,
    /// 预计算的原始伤害（magic.GetDamage(MC) 结果）
    pub damage: i32,
    /// 施法者魔法属性（MC），用于 Vampirism 吸血计算
    pub magic_stat: i32,
    /// 英雄弹道专用：施法者战斗属性（英雄自身 CombatStats；None = 普通玩家，#1184）
    pub hero_stats: Option<crate::combat::attack::CombatStats>,
    /// 英雄弹道专用：施法者等级（英雄自身等级；None = 普通玩家，#1184）
    pub hero_level: Option<u16>,
    /// 法术等级
    pub spell_level: u8,
    /// FireBounce 剩余弹跳次数（0 = 非链式法术；C# bounce = magic.Level + 2）
    pub bounce: i32,
}

/// 远程攻击目标（#1566）：怪物 object_id 或玩家 session
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RangeTarget {
    Monster(u32),
    Player(u64),
}

/// 弓手远程攻击延迟结算项（#1560/#1566，对齐 C# HumanObject.RangeAttack 的
/// DelayedAction(DelayedType.Damage / DamageIndicator, Envir.Time + delay, ...)）：
/// 施法时广播 ObjectRangeAttack + S.RangeAttack 弹道，伤害在箭矢飞行 delay 后落地。
#[derive(Debug, Clone)]
pub struct PendingRangeCompletion {
    /// 到期 tick（WorldActor.tick_count，100ms/tick）
    pub fire_at_tick: u64,
    /// 攻击者 session
    pub session_id: u64,
    /// 攻击者 object_id（反馈包用）
    pub attacker_object_id: u32,
    /// 攻击者位置快照（#1582：受击时怪物转向攻击者）
    pub attacker_x: i32,
    pub attacker_y: i32,
    pub direction: u8,
    /// 目标（怪物 object_id / 玩家 session）
    pub target: RangeTarget,
    /// 目标快照位置（反馈包用）
    pub target_x: i32,
    pub target_y: i32,
    /// 攻击者战斗属性快照（施法时；C# 施法时算伤害、飞行后按目标防御结算）
    pub attacker_stats: crate::combat::attack::CombatStats,
    /// 预计算的原始 DC 伤害（GetRangeAttackPower + ApplyArcherState 后）
    pub raw_damage: i32,
    /// LevelOffset（C# Level > attacker.Level ? 0 : min(10, diff)）
    pub level_offset: u16,
    /// 是否命中（false = 箭矢落空 → DamageIndicator Miss）
    pub hit: bool,
}

/// #1560：箭矢飞行时间（C# HumanObject.cs:2827 delay = MaxDistance*50 + 550 ms；
/// tick=100ms，向上取整避免提前结算）
pub fn range_flight_ticks(attack_dist: i32) -> u64 {
    let delay_ms = attack_dist.max(0) * 50 + 550;
    (delay_ms as u64).div_ceil(100)
}

/// 技能释放请求
pub struct MagicRequest {
    pub session_id: u64,
    pub direction: u8,
    pub spell: u8,
    pub target_id: u32,
    pub target_x: i32,
    pub target_y: i32,
}

/// #306 HellFire：C# HumanObject.HellFire —— 前向直线 + Lv3 两条对角线，各 4 格
fn hellfire_cells(cx: i32, cy: i32, dir: u8, level: u8) -> Vec<(i32, i32)> {
    let dirs: Vec<usize> = if level >= 3 {
        vec![dir as usize % 8, (dir as usize + 7) % 8, (dir as usize + 1) % 8]
    } else {
        vec![dir as usize % 8]
    };
    let mut cells = Vec::new();
    for d in dirs {
        let mut x = cx;
        let mut y = cy;
        for _ in 0..4 {
            x += MON_DIR_DX[d];
            y += MON_DIR_DY[d];
            cells.push((x, y));
        }
    }
    cells
}

/// #306 IceThrust：C# HumanObject.IceThrust —— 前方 1 格主目标 + 相邻 8 格溅射
fn icethrust_cells(cx: i32, cy: i32, dir: u8) -> Vec<(i32, i32)> {
    let d = dir as usize % 8;
    let (tx, ty) = (cx + MON_DIR_DX[d], cy + MON_DIR_DY[d]);
    let mut cells = vec![(tx, ty)];
    for ox in -1..=1 {
        for oy in -1..=1 {
            if ox == 0 && oy == 0 {
                continue;
            }
            cells.push((tx + ox, ty + oy));
        }
    }
    cells
}

/// #306 Curse：C# Map.cs —— 7×7 区域
fn curse_cells(tx: i32, ty: i32) -> Vec<(i32, i32)> {
    let mut cells = Vec::new();
    for x in (tx - 3)..=(tx + 3) {
        for y in (ty - 3)..=(ty + 3) {
            cells.push((x, y));
        }
    }
    cells
}

/// #409 OneWithNature：5×5 区域（C# Map.cs:2101 location ±2）
fn curse_cells_5x5(tx: i32, ty: i32) -> Vec<(i32, i32)> {
    let mut cells = Vec::new();
    for x in (tx - 2)..=(tx + 2) {
        for y in (ty - 2)..=(ty + 2) {
            cells.push((x, y));
        }
    }
    cells
}

/// #328 Plague：C# Map.cs GetPointsInEffectiveSquare(location, 3) —— 3×3 区域
fn plague_cells(tx: i32, ty: i32) -> Vec<(i32, i32)> {
    let mut cells = Vec::new();
    for x in (tx - 1)..=(tx + 1) {
        for y in (ty - 1)..=(ty + 1) {
            cells.push((x, y));
        }
    }
    cells
}

/// #328 Plague：毒强度（C# Red → value/15+Lv+1；其余 value+(Lv+1)*2）
fn plague_temp_value(value: i32, level: u8, poison: mir2_shared::enums::PoisonType) -> i32 {
    if poison == mir2_shared::enums::PoisonType::RED {
        value / 15 + level as i32 + 1
    } else {
        value + (level as i32 + 1) * 2
    }
}

/// #328 Plague：毒持续时间（C# 2*(Lv+1)+value/10）
fn plague_duration(level: u8, value: i32) -> i32 {
    2 * (level as i32 + 1) + value / 10
}

/// #1447：C# UltimateEnhancer expiretime = GetAttackPower(SC)*4 + (Lv+1)*50（秒）
fn ultimate_enhancer_duration_ticks(sc_power: i32, level: u8) -> u32 {
    ((sc_power.max(1) * 4 + (level as i32 + 1) * 50) as u32) * 10
}

/// #345 MPEater：恢复 MP = 5*(Lv + Acc/4)（C# HumanObject.cs:3086）
fn mp_eater_restore(level: i32, accuracy: i32) -> i32 {
    5 * (level + accuracy / 4)
}

/// #345 Hemorrhage：流血持续时间 = Lv*2 + Luck/6（C# HumanObject.cs:3122）
fn hemorrhage_duration(level: i32, luck: i32) -> i32 {
    level * 2 + luck / 6
}

/// #345 Hemorrhage：流血强度 = MaxDC + 1（C# HumanObject.cs:3126）
fn hemorrhage_value(max_dc: i32) -> i32 {
    max_dc + 1
}

/// #377 弓手三连箭：状态持续时间 = 5 + 5*Lv（C# SpecialArrowShot buffTime）
pub(crate) fn special_shot_buff_time(level: u8) -> i32 {
    5 + 5 * level as i32
}

/// #395 幻觉：持续时间 = 随机 10-29 秒（C# HumanObject.cs:6342）
fn hallucination_duration() -> i32 {
    10 + fastrand::i32(0..20)
}

/// #395 幻觉：成功率（C#：roll 范围 Level+20+Lv*5，roll <= target.Level+10 失败；怪物按 Level=0）
fn hallucination_success(level: u8, caster_level: u16) -> bool {
    let roll = fastrand::i32(0..(caster_level as i32 + 20 + level as i32 * 5));
    roll > 10
}

impl WorldActor {
    /// #306：广播法术命中（ObjectStruck + DamageIndicator，对齐 C# Attacked() 表现）
    pub(crate) async fn broadcast_spell_hit(
        &self,
        hits: &[(u32, i32, i32, u8, i32)],
        attacker_id: u32,
    ) {
        for (oid, x, y, dir, damage) in hits {
            let mut struck_body = Vec::new();
            struck_body.extend_from_slice(&oid.to_le_bytes());
            struck_body.extend_from_slice(&attacker_id.to_le_bytes());
            struck_body.extend_from_slice(&(*x as u32).to_le_bytes());
            struck_body.extend_from_slice(&(*y as u32).to_le_bytes());
            struck_body.push(*dir);
            let struck_packet = build_packet_bytes(
                mir2_shared::enums::ServerPacketIds::ObjectStruck as i16, &struck_body);
            let mut dmg_body = Vec::new();
            dmg_body.extend_from_slice(&damage.to_le_bytes());
            dmg_body.push(0u8);
            dmg_body.extend_from_slice(&oid.to_le_bytes());
            let dmg_packet = build_packet_bytes(
                mir2_shared::enums::ServerPacketIds::DamageIndicator as i16, &dmg_body);
            let hit_map = self.monsters.get(oid).map(|m| m.map_index).unwrap_or(0);
            broadcast_to_map(&self.gate_ref, &self.players, hit_map, &struck_packet).await;
            broadcast_to_map(&self.gate_ref, &self.players, hit_map, &dmg_packet).await;
        }
    }
}

/// #1620：C# HumanObject.Magic InRange——目标格超施法范围（Chebyshev）
pub(crate) fn cast_out_of_range(
    caster_x: i32,
    caster_y: i32,
    target_x: i32,
    target_y: i32,
    range: i32,
) -> bool {
    range > 0
        && (target_x != 0 || target_y != 0)
        && (target_x - caster_x).abs().max((target_y - caster_y).abs()) > range
}

impl Message<MagicRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: MagicRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => { return; }
        };
        if state.is_dead { return; }
        // #1287：C# CanCast——眩晕/迷惑/麻痹/冰冻中禁止施法
        if cast_disabled_by_poison(&state.poison_list) {
            return;
        }
        // #1287：沉默禁施法（与 AttackRequest 的 Silence 检查一致）
        if state
            .buffs
            .iter()
            .any(|b| matches!(b.buff_type, crate::combat::buff::BuffType::Silence))
        {
            return;
        }

        // 记录玩家当前施法目标（C# HumanObject.TargetID；宠物 FocusMasterTarget 用）
        if msg.target_id != 0 {
            self.player_targets.insert(msg.session_id, msg.target_id);
        }

        // 施法时自动下坐骑
        self.dismount_player(msg.session_id).await;

        // 施法时打破隐身
        if self.invisible_sessions.remove(&msg.session_id) {
            let _ = record.actor_ref.ask(crate::actors::player::RemoveBuff {
                buff_type: crate::combat::buff::BuffType::Invisibility,
            }).await;
            self.reveal_player_to_others(msg.session_id, &state).await;
        }

        // Pre-allocate object ID for persistent spells (before spell_db borrow)
        let needs_spell_obj = matches!(msg.spell,
            SPELL_FIREWALL | SPELL_BLIZZARD | SPELL_METEOR_STRIKE | SPELL_POISON_CLOUD | SPELL_HEALING_CIRCLE | SPELL_EXPLOSIVE_TRAP
            | SPELL_DELAYED_EXPLOSION
        );
        let spell_oid = if needs_spell_obj { Some(self.alloc_object_id()) } else { None };

        // Validate spell exists in DB
        // DB magic_infos/player_magics 使用 C# 枚举编号，客户端发来的是 SharedRust(+3)
        let spell_cs = msg.spell.saturating_sub(3);
        let spell_db = self.magic_infos.get(&(spell_cs as u32));

        // 检查玩家是否已学习该技能（基础攻击魔法不需要学习）
        let basic_spells = [0, 1]; // None, 基础攻击（C# 编号）
        if !basic_spells.contains(&spell_cs) && !state.magics.iter().any(|m| m.spell == spell_cs as i32) {
            send_system_message(&self.gate_ref, msg.session_id, "你尚未学会这个技能");
            return;
        }
        let spell_range = spell_db.map(|m| m.range as i32).unwrap_or(2);
        // #1620：C# HumanObject.Magic——目标格超施法范围拒绝（location!=0 && Range!=0 && !InRange）
        if cast_out_of_range(state.x, state.y, msg.target_x, msg.target_y, spell_range) {
            send_system_message(&self.gate_ref, msg.session_id, "目标超出施法范围");
            return;
        }
        let power = spell_db.map(|m| m.power_base).unwrap_or(10); // for buff/heal scaling
        // Use spell level from PlayerMagic if learned
        let spell_level = state.magics.iter()
            .find(|m| m.spell == spell_cs as i32)
            .map(|m| m.level)
            .unwrap_or(0);

        // C#：施法广播 S.ObjectSpell 给同图其他玩家（ObjectID + 位置 + Spell）
        let spell_enum = mir2_shared::enums::Spell::try_from(msg.spell).unwrap_or(mir2_shared::enums::Spell::None);
        let obj_spell = mir2_shared::packets::server::magic_combat::ObjectSpell {
            object_id: state.object_id,
            location_x: state.x,
            location_y: state.y,
            spell: spell_enum,
        };
        let mut ob = Vec::new();
        if obj_spell.write_body(&mut ob).is_ok() {
            let pkt = build_packet_bytes(mir2_shared::enums::ServerPacketIds::ObjectSpell as i16, &ob);
            for (sid, r) in &self.players {
                if *sid == msg.session_id { continue; }
                if let Ok(Some(os)) = r.actor_ref.ask(GetPlayerState).await {
                    if os.map_index == state.map_index {
                        let _ = self.gate_ref.tell(SendToClient { session_id: *sid, data: pkt.clone() }).await;
                    }
                }
            }
        }

        // Global timestamp for CD + XP
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        // #1256：ElectricShock 分支内显式给技能经验（C#：成功必给/失败 50%/无目标不给）
        let mut electric_shock_exp_handled = false;

        // Cooldown check
        if let Some(spell_info) = spell_db {
            let delay_ms = crate::combat::magic::magic_delay(spell_info, spell_level);
            let last_cast = state.magics.iter()
                .find(|m| m.spell == spell_cs as i32)
                .map(|m| m.cast_time)
                .unwrap_or(0);
            if last_cast > 0 && (now_ms - last_cast) < delay_ms as i64 {
                let remaining = delay_ms as i64 - (now_ms - last_cast);
                send_system_message(&self.gate_ref, msg.session_id, &format!("技能冷却中，还需 {} 秒", remaining / 1000));
                return;
            }
        }

        let mp_cost = {
            let base = spell_db.map(|m| crate::combat::magic::magic_cost(m, spell_level)).unwrap_or(5);
            // C# HumanObject.cs:3381：TemporalFlux（Teleport/Blink/StormEscape 后 30s）施法耗蓝 +30%
            let penalty = state.buffs.iter().find_map(|b| match b.buff_type {
                crate::combat::buff::BuffType::TeleportManaPenalty { percent } => Some(percent),
                _ => None,
            }).unwrap_or(0);
            base + base * penalty / 100
        };

        // Decide which stat feeds this spell
        let magic_stat = match state.class {
            mir2_shared::enums::MirClass::Wizard => state.effective_max_mc(),
            mir2_shared::enums::MirClass::Taoist => state.effective_max_sc(),
            _ => state.effective_max_attack(), // Warriors/Assassins/Archers use Attack
        };

        // 检查并扣除 MP
        if state.mp < mp_cost {
            send_system_message(&self.gate_ref, msg.session_id, "魔法值不足");
            return;
        }
        let mp_ok = record.actor_ref.ask(DeductMP { amount: mp_cost }).await.unwrap_or(false);
        if !mp_ok {
            send_system_message(&self.gate_ref, msg.session_id, "魔法值不足");
            return;
        }
        // #1578：C# HumanObject 魔法 LogTime——成功施法后 10s 内不可下线
        self.player_logout_block_ms.insert(msg.session_id, now_ms + LOGOUT_DELAY_MS);
        // #312：冥想被动——施法后有概率返还 MP（C# HumanObject.cs:3827，概率≈(Lv+集中)/8）
        let meditation_lv = state.magics.iter()
            .find(|m| m.spell == (SPELL_MEDITATION as i32 - 3))
            .map(|m| m.level)
            .unwrap_or(0);
        if meditation_lv > 0 && fastrand::i32(0..8) < meditation_lv as i32 {
            let _ = record.actor_ref.ask(crate::actors::player::AddMP { amount: mp_cost as i32 }).await;
            send_system_message(&self.gate_ref, msg.session_id, &format!("冥想恢复 {} 魔法值", mp_cost));
            debug!("Magic: {} Meditation refunded {} MP", state.name, mp_cost);
        }

        let object_id = state.object_id;
        let target_x = msg.target_x;
        let target_y = msg.target_y;

        // 发送 MagicCast 给施法者（确认施法）
        let spell_enum = mir2_shared::enums::Spell::try_from(msg.spell)
            .unwrap_or(mir2_shared::enums::Spell::None);
        let magic_cast = mir2_shared::packets::server::magic_combat::MagicCast { spell: spell_enum };
        let mut cast_body = Vec::new();
        if magic_cast.write_body(&mut cast_body).is_ok() {
            let _ = self.gate_ref.tell(SendToClient {
                session_id: msg.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::MagicCast as i16, &cast_body),
            }).await;
        }

        // MeteorShower：主目标是怪物时，取周围 4 格内最多 3 个副目标（伤害减半，C# HumanObject.cs:5835）
        let meteor_secondary: Vec<(u32, i32, i32)> =
            if spell_enum == mir2_shared::enums::Spell::MeteorShower {
                let mut ids = Vec::new();
                if let Some(m) = self.monsters.get(&msg.target_id) {
                    if m.hp > 0 {
                        let mut nearby: Vec<(u32, i32, i32)> = self.monsters.iter()
                            .filter(|(id, mm)| {
                                **id != msg.target_id
                                    && mm.hp > 0
                                    && mm.map_index == m.map_index
                                    && (mm.x - m.x).abs() <= 4
                                    && (mm.y - m.y).abs() <= 4
                            })
                            .map(|(id, mm)| (*id, mm.x, mm.y))
                            .collect();
                        // 按距离升序取前 3（近似 C# FindAllNearby(4)）
                        nearby.sort_by_key(|(_, x, y)| (x - m.x).abs() + (y - m.y).abs());
                        ids = nearby.into_iter().take(3).collect();
                    }
                }
                ids
            } else {
                Vec::new()
            };

        // 广播 ObjectMagic 给其他玩家
        let others: Vec<_> = self.same_map_players(msg.session_id, state.map_index).await
            .into_iter()
            .collect();
        let object_magic = mir2_shared::packets::server::magic_combat::ObjectMagic {
            object_id,
            location_x: state.x,
            location_y: state.y,
            direction: mir2_shared::enums::MirDirection::try_from(msg.direction)
                .unwrap_or(mir2_shared::enums::MirDirection::Up),
            spell: spell_enum,
            target_id: msg.target_id,
            target_x,
            target_y,
            cast: true,
            level: spell_level,
            self_broadcast: false,
            secondary_target_ids: meteor_secondary.iter().map(|(id, _, _)| *id).collect(),
        };
        let mut om_body = Vec::new();
        if object_magic.write_body(&mut om_body).is_ok() {
            for other in &others {
                let _ = self.gate_ref.tell(SendToClient {
                    session_id: other.session_id,
                    data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::ObjectMagic as i16, &om_body),
                }).await;
            }
            // #1580：本地玩家自己的施法动画（self_broadcast=true；C# 本地 ActionFeed，Bevy 依赖回显）
            let mut self_om = object_magic.clone();
            self_om.self_broadcast = true;
            let mut self_body = Vec::new();
            if self_om.write_body(&mut self_body).is_ok() {
                let _ = self.gate_ref.tell(SendToClient {
                    session_id: msg.session_id,
                    data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::ObjectMagic as i16, &self_body),
                }).await;
            }
        }

        // 创建持久法术对象（火墙、暴风雪等）
        let spell_enum = mir2_shared::enums::Spell::try_from(msg.spell)
            .unwrap_or(mir2_shared::enums::Spell::None);
        let is_persistent = matches!(spell_enum,
            mir2_shared::enums::Spell::FireWall | mir2_shared::enums::Spell::Blizzard
            | mir2_shared::enums::Spell::MeteorStrike | mir2_shared::enums::Spell::PoisonCloud
            | mir2_shared::enums::Spell::HealingCircle | mir2_shared::enums::Spell::ExplosiveTrap
            | mir2_shared::enums::Spell::Portal | mir2_shared::enums::Spell::DelayedExplosion
        );
        let persistent_spell = if is_persistent {
            spell_oid.map(|oid| spell::create_persistent_spell(
                oid, object_id, msg.session_id, state.map_index,
                target_x, target_y, spell_level, magic_stat, spell_enum,
            ))
        } else {
            None
        };
        if let Some(mut spell_obj) = persistent_spell {
            // DelayedExplosion（C# HumanObject.DelayedExplosion）：施法后按距离延迟
            // `距离*50 + 500ms` 才触发；且要挂到目标身上（target_id 用于引爆命中）。
            if spell_obj.spell == mir2_shared::enums::Spell::DelayedExplosion {
                let (tx, ty) = if target_x == 0 && target_y == 0 {
                    self.monsters.get(&msg.target_id)
                        .map(|m| (m.x, m.y))
                        .unwrap_or((state.x, state.y))
                } else {
                    (target_x, target_y)
                };
                let dist = (state.x - tx).abs() + (state.y - ty).abs();
                spell_obj.expires_at_ms = (dist * 50 + 500).max(500) as u64;
                spell_obj.target_id = if msg.target_id != 0 { Some(msg.target_id) } else { None };
            }
            let spell_type = mir2_shared::enums::Spell::try_from(msg.spell)
                .unwrap_or(mir2_shared::enums::Spell::None);
            let object_spell = mir2_shared::packets::server::magic_combat::ObjectSpell {
                object_id: spell_obj.object_id,
                location_x: spell_obj.x,
                location_y: spell_obj.y,
                spell: spell_type,
            };
            let mut os_body = Vec::new();
            if object_spell.write_body(&mut os_body).is_ok() {
                let spell_packet = build_packet_bytes(
                    mir2_shared::enums::ServerPacketIds::ObjectSpell as i16, &os_body,
                );
                // Send to self + nearby players
                let session_ids: Vec<u64> = std::iter::once(msg.session_id)
                    .chain(self.same_map_players(msg.session_id, state.map_index).await.iter().map(|p| p.session_id))
                    .collect();
                for sid in &session_ids {
                    let _ = self.gate_ref.tell(SendToClient {
                        session_id: *sid,
                        data: spell_packet.clone(),
                    }).await;
                }
            }
            self.spell_objects.insert(spell_obj.object_id, spell_obj);
        }

        // 根据魔法类型执行不同效果
        match msg.spell {
            // --- 治愈类 ---
            // Healing：单目标友方（C# HumanObject.cs：health = GetDamage(SC*2) + Level）
            // MassHealing：目标点 3×3 内自己+同组（C# Map.cs：value = GetDamage(SC)）
            SPELL_HEALING | SPELL_MASS_HEALING => {
                let sc_power = crate::combat::attack::get_attack_power(
                    state.min_sc + state.bonus_min_sc,
                    state.max_sc + state.bonus_max_sc,
                    0,
                );
                if msg.spell == SPELL_HEALING {
                    // 友方目标：点击自己/同组玩家
                    let mut target_session = msg.session_id;
                    if msg.target_id != 0 {
                        for (sid, r) in &self.players {
                            if let Ok(Some(os)) = r.actor_ref.ask(GetPlayerState).await {
                                if os.object_id == msg.target_id {
                                    let friendly = *sid == msg.session_id
                                        || (os.group_id.is_some() && os.group_id == state.group_id);
                                    if friendly {
                                        target_session = *sid;
                                    }
                                    break;
                                }
                            }
                        }
                    }
                    let amount = sc_power * 2 + state.level as i32; // C# GetDamage(SC*2) + Level
                    if let Some(r) = self.players.get(&target_session) {
                        let _ = r.actor_ref.ask(crate::actors::player::Heal { amount }).await;
                    }
                    debug!("Magic: {} casts Healing on session {} (+{} HP)", state.name, target_session, amount);
                } else {
                    let cx = if target_x == 0 && target_y == 0 { state.x } else { target_x };
                    let cy = if target_x == 0 && target_y == 0 { state.y } else { target_y };
                    let amount = sc_power.max(1);
                    let mut healed = 0u32;
                    for (sid, r) in &self.players {
                        if let Ok(Some(s)) = r.actor_ref.ask(GetPlayerState).await {
                            let friendly = *sid == msg.session_id
                                || (s.group_id.is_some() && s.group_id == state.group_id);
                            // #1684：MassHealing 只治疗同图友方（C# CurrentMap）
                            if friendly && !s.is_dead
                                && s.map_index == state.map_index
                                && (s.x - cx).abs() <= 1 && (s.y - cy).abs() <= 1
                            {
                                let _ = r.actor_ref.ask(crate::actors::player::Heal { amount }).await;
                                healed += 1;
                            }
                        }
                    }
                    debug!("Magic: {} casts MassHealing (3x3, healed {} players, +{} HP)",
                           state.name, healed, amount);
                }
            }
            // HealingCircle：持续治疗场由 SpellObject 每跳治疗（C# 无即时自疗）
            SPELL_HEALING_CIRCLE => {
                debug!("Magic: {} casts HealingCircle (persistent field)", state.name);
            }
            // --- Buff 类 ---
            // MagicShield：C# 用 Stat.DamageReductionPercent（百分比减伤），非 DefenseBoost
            // 强度 = (level+2)*10%（Lv0=20/Lv1=30/Lv2=40），持续 = GetPower(MC+15) 秒
            SPELL_MAGIC_SHIELD => {
                let reduction_pct = ((spell_level as i32 + 2) * 10).min(80);
                // 持续时间近似：power 已含 MC 加成，转成 ticks（100ms/tick）
                let duration_ticks = ((power.max(15) as u32) * 10).min(6000); // 上限 10 分钟
                let _ = record.actor_ref.ask(crate::actors::player::ApplyDamageReduction {
                    percent: reduction_pct,
                    duration_ticks,
                }).await;
                debug!("Magic: {} casts MagicShield (damage -{}%)", state.name, reduction_pct);
            }
            // SoulShield / BlessedArmour：目标点 7×7 友方护盾（C# HumanObject.cs + Map.cs）
            // bonus = 目标等级/7+4；时长 = SC*4 + (Lv+1)*50 秒
            SPELL_SOUL_SHIELD | SPELL_BLESSED_ARMOUR => {
                let is_soul = msg.spell == SPELL_SOUL_SHIELD;
                let sc = state.effective_max_sc();
                let duration_ticks = ((sc * 4 + (spell_level as i32 + 1) * 50).max(1) as u32) * 10;
                let cx = if target_x == 0 && target_y == 0 { state.x } else { target_x };
                let cy = if target_x == 0 && target_y == 0 { state.y } else { target_y };
                let mut targets: Vec<u64> = vec![msg.session_id];
                if let Some(gid) = state.group_id {
                    for (sid, other) in &self.players {
                        if *sid == msg.session_id { continue; }
                        if let Ok(Some(s)) = other.actor_ref.ask(GetPlayerState).await {
                            // #1684：MassHiding 只隐身同图队友（C# CurrentMap）
                            if s.group_id == Some(gid)
                                && s.map_index == state.map_index
                                && (s.x - cx).abs() <= 3 && (s.y - cy).abs() <= 3 {
                                targets.push(*sid);
                            }
                        }
                    }
                }
                for sid in &targets {
                    let Some(other) = self.players.get(sid) else { continue; };
                    let level = if *sid == msg.session_id {
                        state.level
                    } else {
                        other.actor_ref.ask(GetPlayerState).await.ok().flatten().map(|s| s.level).unwrap_or(0)
                    };
                    let bonus = (level as i32 / 7 + 4).max(1);
                    let buff = crate::combat::buff::BuffInstance::new(
                        if is_soul {
                            crate::combat::buff::BuffType::MacDefenseBoost { bonus }
                        } else {
                            crate::combat::buff::BuffType::AcDefenseBoost { bonus }
                        },
                        duration_ticks,
                        5,
                    );
                    let _ = other.actor_ref.ask(crate::actors::player::ApplyBuff { buff }).await;
                }
                debug!("Magic: {} casts {} on {} targets (+{}, {}s)",
                       state.name, if is_soul { "SoulShield" } else { "BlessedArmour" },
                       targets.len(), state.level as i32 / 7 + 4, duration_ticks / 10);
            }
            // --- 道士 Debuff/控制类 ---
            // Poisoning：对目标怪物施毒（绿毒持续掉血/红毒降防御，C# Poisoning 消耗毒药物品）
            SPELL_POISONING => {
                // C# HumanObject.cs:6043：单目标（点击格首个怪物），value = GetDamage(SC)
                let value = crate::combat::attack::get_attack_power(
                    state.min_sc + state.bonus_min_sc,
                    state.max_sc + state.bonus_max_sc,
                    0,
                ).max(1);
                let mid = self.monsters.iter()
                    .find(|(_, m)| m.map_index == state.map_index && (m.x - target_x).abs() <= 1 && (m.y - target_y).abs() <= 1 && m.hp > 0)
                    .map(|(id, _)| *id);
                if let Some(mid) = mid {
                    if let Some(monster) = self.monsters.get_mut(&mid) {
                        // C# Shape1 绿毒：Duration = value*2 + (Lv+1)*7；Value = value/15 + Lv + 1 + Random(PoisonAttack)
                        let duration = (value * 2 + (spell_level as i32 + 1) * 7).max(1) as u32;
                        let poison_value = (value / 15 + spell_level as i32 + 1
                            + fastrand::i32(0..state.poison_attack.max(1))).max(1);
                        crate::combat::poison::apply_poison(&mut monster.poison_list,
                            crate::combat::poison::Poison::new(
                                mir2_shared::enums::PoisonType::GREEN, duration, poison_value, 2000,
                            ));
                        monster.provoked = true;
                        // C# MonsterObject.ApplyPoison：仅当无目标时锁定施毒者（Target == null 才设置）
                        if monster.target_session.is_none() {
                            monster.target_session = Some(msg.session_id);
                        }
                        debug!("Magic: {} casts Poisoning -> monster {} ({}s, {}dmg/tick)",
                               state.name, mid, duration, poison_value);
                    }
                } else {
                    debug!("Magic: {} casts Poisoning (no target near {},{})", state.name, target_x, target_y);
                }
            }
            // TrapHexagon：定身目标怪物（C# HumanObject.cs + Map.cs：跳过等级 > 施法+2 的怪物，
            // 时长 = (Lv*5+10) 秒）
            SPELL_TRAP_HEXAGON => {
                let hit_ids: Vec<u32> = self.monsters.iter()
                    .filter(|(_, m)| {
                        let dist = (m.x - target_x).abs() + (m.y - target_y).abs();
                        dist <= 1 && m.hp > 0 && m.map_index == state.map_index
                            && self.monster_infos.get(&m.monster_index).map(|i| i.level).unwrap_or(0) <= state.level as i32 + 2
                    })
                    .map(|(id, _)| *id)
                    .collect();
                let trapped_count = hit_ids.len();
                let duration = (spell_level as u32 * 5 + 10) as u32;
                for mid in hit_ids {
                    if let Some(monster) = self.monsters.get_mut(&mid) {
                        crate::combat::poison::apply_poison(&mut monster.poison_list,
                            crate::combat::poison::Poison::new(mir2_shared::enums::PoisonType::PARALYSIS, duration, 0, 1000));
                    }
                }
                debug!("Magic: {} casts TrapHexagon (trapped {} monsters, {}s)", state.name, trapped_count, duration);
            }
            // --- 道士 Buff/辅助类 ---
            // Hiding：自身隐身（怪物失去目标，C# BuffType.Hiding）
            SPELL_HIDING => {
                let buff = crate::combat::buff::BuffInstance::new(
                    crate::combat::buff::BuffType::Invisibility,
                    (30 + spell_level as u32 * 10) * 10, // 30-60s，100ms/tick
                    5,
                );
                let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff { buff }).await;
                self.invisible_sessions.insert(msg.session_id);
            if let Ok(Some(st)) = record.actor_ref.ask(GetPlayerState).await {
                self.broadcast_object_hidden(st.object_id, true, st.map_index).await;
            }
                debug!("Magic: {} casts Hiding (invisible)", state.name);
            }
            // MassHiding：组队隐身（目标点 3×3 友方 + C# 时长公式）
            SPELL_MASS_HIDING => {
                // C# 时长：value = GetAttackPower(MinSC,MaxSC)/2 + (Lv+1)*2 秒（HumanObject.cs:4500）
                let sc_power = crate::combat::attack::get_attack_power(
                    state.min_sc + state.bonus_min_sc,
                    state.max_sc + state.bonus_max_sc,
                    0,
                );
                let duration_ticks = ((sc_power / 2 + (spell_level as i32 + 1) * 2).max(1) as u32) * 10;
                // C# Map.cs MassHiding：目标点 3×3（±1）范围内友方（自己/同组）隐身
                let cx = if target_x == 0 && target_y == 0 { state.x } else { target_x };
                let cy = if target_x == 0 && target_y == 0 { state.y } else { target_y };
                let mut targets: Vec<u64> = vec![msg.session_id];
                if let Some(gid) = state.group_id {
                    for (sid, other) in &self.players {
                        if *sid == msg.session_id { continue; }
                        if let Ok(Some(s)) = other.actor_ref.ask(GetPlayerState).await {
                            if s.group_id == Some(gid)
                                && (s.x - cx).abs() <= 1 && (s.y - cy).abs() <= 1 {
                                targets.push(*sid);
                            }
                        }
                    }
                }
                for sid in &targets {
                    let buff = crate::combat::buff::BuffInstance::new(
                        crate::combat::buff::BuffType::Invisibility,
                        duration_ticks,
                        5,
                    );
                    let Some(other) = self.players.get(sid) else { continue; };
                    let _ = other.actor_ref.ask(crate::actors::player::ApplyBuff { buff }).await;
                    self.invisible_sessions.insert(*sid);
                    if let Ok(Some(st)) = other.actor_ref.ask(GetPlayerState).await {
                        self.broadcast_object_hidden(st.object_id, true, st.map_index).await;
                    }
                }
                debug!("Magic: {} casts MassHiding on {} targets ({}s)", state.name, targets.len(), duration_ticks / 10);
            }
            // Purification：解毒/清除 debuff（C# HumanObject.cs:4440 + CompleteMagic 6246）
            // 友方目标（自己/同组），成功率 Random(4) <= Lv（Lv0=25%）
            SPELL_PURIFICATION => {
                if fastrand::i32(0..4) > spell_level as i32 {
                    debug!("Magic: {} casts Purification (failed)", state.name);
                    return;
                }
                let mut target_session = msg.session_id;
                if msg.target_id != 0 {
                    for (sid, r) in &self.players {
                        if let Ok(Some(os)) = r.actor_ref.ask(GetPlayerState).await {
                            if os.object_id == msg.target_id {
                                let friendly = *sid == msg.session_id
                                    || (os.group_id.is_some() && os.group_id == state.group_id);
                                if friendly {
                                    target_session = *sid;
                                }
                                break;
                            }
                        }
                    }
                }
                if let Some(r) = self.players.get(&target_session) {
                    let _ = r.actor_ref.ask(crate::actors::player::PurifyPoisons).await;
                }
                debug!("Magic: {} casts Purification on session {} (success)", state.name, target_session);
            }
            // Entrapment：困魔咒（C# HumanObject.cs:4893 + CompleteMagic 6315）——
            // 拉拽目标怪物朝施法者反方向靠近（对角 min(|dx|,|dy|)，十字轴 |axis|-2），并麻痹 round((Lv+1)*0.8) 秒
            SPELL_ENTRAPMENT => {
                let mid = self.monsters.iter()
                    .filter(|(_, m)| m.hp > 0 && m.map_index == state.map_index && (m.x - target_x).abs() <= 1 && (m.y - target_y).abs() <= 1)
                    .map(|(id, _)| *id)
                    .next();
                let Some(mid) = mid else { return; };
                let (mx, my, mlevel) = match self.monsters.get(&mid) {
                    Some(m) => (m.x, m.y, self.monster_infos.get(&m.monster_index).map(|i| i.level).unwrap_or(0)),
                    None => return,
                };
                let dist = (state.x - mx).abs().max((state.y - my).abs());
                // C#：MaxDistance > 7 或目标等级 >= 施法等级 + 5 + Random(8) → 失败
                if dist > 7 || mlevel >= state.level as i32 + 5 + fastrand::i32(0..8) {
                    return;
                }
                // C#：Random(30) >= (Lv+1)*3 + (Level - targetLevel + 9) → 失败
                let levelgap = state.level as i32 - mlevel + 9;
                if fastrand::i32(0..30) >= ((spell_level as i32 + 1) * 3) + levelgap {
                    return;
                }
                // 麻痹时长（怪物）：round((Lv+1)*0.8)
                let duration = (((spell_level as i32 + 1) as f64) * 0.8).round() as u32;
                if duration > 0 {
                    if let Some(monster) = self.monsters.get_mut(&mid) {
                        crate::combat::poison::apply_poison(&mut monster.poison_list,
                            crate::combat::poison::Poison::new(mir2_shared::enums::PoisonType::PARALYSIS, duration, 0, 1000));
                    }
                }
                // 拉拽方向 = 施法者朝向的反方向（C# (Direction - 4) % 8）
                let pull_dir = ((msg.direction as usize + 4) % 8) as u8;
                let pulldistance = if pull_dir % 2 > 0 {
                    ((state.x - mx).abs().min((state.y - my).abs())).max(0)
                } else {
                    match pull_dir {
                        0 | 4 => ((state.y - my).abs() - 2).max(0), // Up/Down
                        _ => ((state.x - mx).abs() - 2).max(0),      // Left/Right
                    }
                };
                let moved = self.push_monster(mid, pull_dir, pulldistance.max(1)).await;
                debug!("Magic: {} casts Entrapment -> monster {} pulled {} tiles ({}s paralysis)",
                    state.name, mid, moved, duration);
            }
            // ShoulderDash：野蛮冲撞（C# HumanObject.cs：只冲刺 2 格 + 推开路径上等级更低的目标 1 格，无伤害）
            SPELL_SHOULDER_DASH => {
                let dir = msg.direction as usize % 8;
                let mut new_x = state.x;
                let mut new_y = state.y;
                let mut pushed = 0usize;
                for step in 0..2 {
                    let nx = new_x + MON_DIR_DX[dir];
                    let ny = new_y + MON_DIR_DY[dir];
                    let walkable = self.maps.get(&state.map_index)
                        .map(|m| m.is_walkable(nx, ny))
                        .unwrap_or(false);
                    if !walkable { break; }
                    // C#：路径上等级 < 施法等级的目标才推送
                    let hit: Option<(u32, i32)> = self.monsters.iter()
                        .find(|(_, m)| m.map_index == state.map_index && m.x == nx && m.y == ny && m.hp > 0)
                        .map(|(id, m)| (*id, self.monster_infos.get(&m.monster_index).map(|i| i.level).unwrap_or(0)));
                    if let Some((mid, mlevel)) = hit {
                        if mlevel < state.level as i32 {
                            let _ = self.push_monster(mid, dir as u8, 1).await;
                            pushed += 1;
                        }
                    }
                    new_x = nx;
                    new_y = ny;
                    let _ = step;
                }
                if new_x != state.x || new_y != state.y {
                    let _ = record.actor_ref.ask(crate::actors::player::SetPlayerPosition {
                        x: new_x, y: new_y, direction: msg.direction,
                        map_index: None, is_mounted: None,
                    }).await;
                    self.broadcast_position_change(msg.session_id, new_x, new_y, msg.direction).await;
                }
                debug!("Magic: {} casts ShoulderDash (dashed to {},{}, pushed {} monsters)",
                    state.name, new_x, new_y, pushed);
            }
            // Thrusting：刺杀（直线穿透 2 格，打前方 2 个格子）
            SPELL_THRUSTING => {
                let dir = msg.direction as usize % 8;
                let attacker_stats = state.to_combat_stats();
                let raw = crate::combat::attack::get_attack_power(
                    attacker_stats.min_atk, attacker_stats.max_atk, attacker_stats.luck,
                );
                // C# Envir.cs Thrusting：倍率 0.25+0.25Lv（GetDamage = base × Multiplier）
                let raw_damage = ((raw as f32) * (0.25 + 0.25 * spell_level as f32)).max(1.0) as i32;
                let mut cx = state.x;
                let mut cy = state.y;
                for _ in 0..2 {
                    cx += MON_DIR_DX[dir];
                    cy += MON_DIR_DY[dir];
                    let hit = self.monsters.iter()
                        .find(|(_, m)| m.map_index == state.map_index && m.x == cx && m.y == cy && m.hp > 0)
                        .map(|(id, _)| *id);
                    if let Some(mid) = hit {
                        if let Some(m) = self.monsters.get_mut(&mid) {
                            let ds = m.to_combat_stats();
                            // #1455：LevelOffset = Level > attacker.Level ? 0 : min(10, attacker.Level - Level)
                            let level_offset = crate::combat::attack::level_offset(state.level, m.level.max(0) as u16);
                            let r = combat_attack::resolve_attack(
                                &attacker_stats, &ds, raw_damage,
                                mir2_shared::enums::DefenceType::AcAgility, level_offset,
                            );
                            if r.is_hit && r.damage > 0 {
                                m.take_damage(r.damage);
                                m.last_hitter_session = Some(msg.session_id);
                                self.pending_gather.push(msg.session_id);
                                m.provoked = true;
                                // C# MonsterObject.Attacked：仅当无目标时锁定攻击者（魔法伤害同物理）
                                if m.target_session.is_none() {
                                    m.target_session = Some(msg.session_id);
                                }
                            }
                        }
                    }
                }
                debug!("Magic: {} casts Thrusting (line pierce 2)", state.name);
            }
            // --- 传送类 ---
            // Teleport：法师回城（C# MagicTeleport：传送到绑定点附近，半径 = 绑定地图尺寸/(Lv+1)）
            // Blink：定点传送，距离上限=Range，成功率=(level+1)/4
            // StormEscape：同 Blink（C# 同逻辑）
            SPELL_TELEPORT => {
                if let Some(mi) = self.map_infos.get(&(state.map_index as i32)) {
                    if mi.no_teleport {
                        send_system_message(&self.gate_ref, msg.session_id, "该地图无法使用传送魔法");
                        return;
                    }
                }
                // C# MagicTeleport：以绑定点为中心随机偏移（地图尺寸/(Lv+1)），最多 200 次尝试
                let bind_map = state.bind_map_index;
                let Some((map_w, map_h)) = self.bind_map_size(bind_map) else {
                    send_system_message(&self.gate_ref, msg.session_id, "未设置绑定点");
                    return;
                };
                let size_x = (map_w / (spell_level as i32 + 1)).max(1);
                let size_y = (map_h / (spell_level as i32 + 1)).max(1);
                let mut dest = None;
                if let Some(map) = self.maps.get(&(bind_map as u16)) {
                    for _ in 0..200 {
                        let rx = state.bind_x + fastrand::i32(-size_x..=size_x);
                        let ry = state.bind_y + fastrand::i32(-size_y..=size_y);
                        if map.is_valid(rx, ry) && map.is_walkable(rx, ry) {
                            dest = Some((rx, ry));
                            break;
                        }
                    }
                }
                if let Some((rx, ry)) = dest {
                    crate::actors::world::npc_script::teleport_player(
                        self, msg.session_id, bind_map as u16, rx, ry).await;
                    // C#：传送成功后 TemporalFlux（30s 施法耗蓝 +30%）
                    let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff {
                        buff: crate::combat::buff::BuffInstance::new(
                            crate::combat::buff::BuffType::TeleportManaPenalty { percent: 30 },
                            300, 1,
                        ),
                    }).await;
                    debug!("Magic: {} MagicTeleport to bind map {} ({},{})", state.name, bind_map, rx, ry);
                } else {
                    send_system_message(&self.gate_ref, msg.session_id, "传送失败，未找到合适位置");
                }
            }
            SPELL_BLINK | SPELL_STORM_ESCAPE => {
                if let Some(mi) = self.map_infos.get(&(state.map_index as i32)) {
                    if mi.no_teleport {
                        send_system_message(&self.gate_ref, msg.session_id, "该地图无法使用传送魔法");
                        return;
                    }
                    if mi.no_escape {
                        send_system_message(&self.gate_ref, msg.session_id, "该地图无法使用传送魔法");
                        return;
                    }
                }
                let (max_x, max_y) = self.maps.get(&state.map_index)
                    .map(|m| (m.width as i32, m.height as i32))
                    .unwrap_or((i32::MAX, i32::MAX));

                // Blink/StormEscape：距离校验 + 成功率（C# Random(4) >= Lv+1 失败）
                let dist = ((state.x - target_x).abs() + (state.y - target_y).abs()) as i32;
                let range = spell_db.map(|m| m.range as i32).unwrap_or(10);
                if dist > range {
                    send_system_message(&self.gate_ref, msg.session_id, "距离超出闪现范围");
                    return;
                }
                // 成功率 (level+1)/4：Random(4) >= level+1 则失败
                if fastrand::i32(0..4) >= spell_level as i32 + 1 {
                    debug!("Magic: {} Blink failed (random miss)", state.name);
                    return;
                }

                let tx = target_x.clamp(0, max_x - 1);
                let ty = target_y.clamp(0, max_y - 1);
                let _ = record.actor_ref.ask(crate::actors::player::SetPlayerPosition {
                    x: tx,
                    y: ty,
                    direction: msg.direction,
                    map_index: None,
                    is_mounted: None,
                }).await;
                self.broadcast_position_change(msg.session_id, tx, ty, msg.direction).await;
                // C#：闪现/风遁成功后 TemporalFlux（30s 施法耗蓝 +30%）
                let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff {
                    buff: crate::combat::buff::BuffInstance::new(
                        crate::combat::buff::BuffType::TeleportManaPenalty { percent: 30 },
                        300, 1,
                    ),
                }).await;
                debug!("Magic: {} blinks to ({}, {})", state.name, tx, ty);
            }
            // --- 弹道类法术（任务3）：FireBall/GreatFireBall/ThunderBolt/FrostCrunch/Vampirism ---
            // 对齐 C# HumanObject Fireball()/ThunderBolt()/Vampirism()：创建 DelayedAction，延迟后结算
            SPELL_FIREBALL | SPELL_GREAT_FIREBALL | SPELL_THUNDERBOLT
            | SPELL_FROST_CRUNCH | SPELL_VAMPIRISM | SPELL_FLAME_DISRUPTOR | SPELL_SOUL_FIREBALL
            | SPELL_METEOR_SHOWER => {
                let raw_damage = if let Some(info) = spell_db {
                    crate::combat::magic::calc_magic_damage(info, spell_level, magic_stat)
                } else {
                    fastrand::i32(5..=15)
                }.max(1);

                // 弹道延迟：FireBall 系 = 距离×50ms + 500ms；ThunderBolt/Vampirism = 固定 500ms
                let target_dist = ((state.x - target_x).abs() + (state.y - target_y).abs()) as u64;
                let delay_ms = match msg.spell {
                    SPELL_FIREBALL | SPELL_GREAT_FIREBALL | SPELL_FROST_CRUNCH | SPELL_METEOR_SHOWER => {
                        target_dist * 50 + 500
                    }
                    _ => 500, // ThunderBolt / Vampirism 固定 500ms
                };
                // tick_count 每 100ms +1，延迟按 100ms 取整（最少 1 tick）
                let fire_at_tick = self.tick_count + (delay_ms / 100).max(1);

                self.pending_spell_completions.push(PendingSpellCompletion {
                    fire_at_tick,
                    session_id: msg.session_id,
                    spell: msg.spell,
                    target_id: msg.target_id,
                    target_x,
                    target_y,
                    damage: raw_damage,
                    magic_stat,
                    hero_stats: None,
                    hero_level: None,
                    spell_level,
                    bounce: 0,
                });

                // MeteorShower：副目标（最多 3 个，周围 4 格）各吃 50% 伤害（C# HumanObject.cs:5852）
                if msg.spell == SPELL_METEOR_SHOWER {
                    for (sid, sx, sy) in &meteor_secondary {
                        self.pending_spell_completions.push(PendingSpellCompletion {
                            fire_at_tick,
                            session_id: msg.session_id,
                            spell: msg.spell,
                            target_id: *sid,
                            target_x: *sx,
                            target_y: *sy,
                            damage: (raw_damage / 2).max(1),
                            magic_stat,
                            hero_stats: None,
                            hero_level: None,
                            spell_level,
                            bounce: 0,
                        });
                    }
                }
                debug!("Magic: {} casts projectile spell={} dmg={} delay={}ms secondary={} (fires @tick {})",
                    state.name, msg.spell, raw_damage, delay_ms, meteor_secondary.len(), fire_at_tick);
            }
            // FireBounce：链式弹射（C# HumanObject.cs:5811；首跳延迟=距离×50+500ms，后续每跳=距离×50ms）
            SPELL_FIRE_BOUNCE => {
                let raw_damage = if let Some(info) = spell_db {
                    crate::combat::magic::calc_magic_damage(info, spell_level, magic_stat)
                } else { fastrand::i32(5..=15) }.max(1);
                let target_dist = ((state.x - target_x).abs() + (state.y - target_y).abs()) as u64;
                let delay_ms = target_dist * 50 + 500;
                let fire_at_tick = self.tick_count + (delay_ms / 100).max(1);
                self.pending_spell_completions.push(PendingSpellCompletion {
                    fire_at_tick,
                    session_id: msg.session_id,
                    spell: msg.spell,
                    target_id: msg.target_id,
                    target_x,
                    target_y,
                    damage: raw_damage,
                    magic_stat,
                    hero_stats: None,
                    hero_level: None,
                    spell_level,
                    bounce: spell_level as i32 + 2, // C# bounce = magic.Level + 2
                });
                debug!("Magic: {} casts FireBounce dmg={} bounce={} delay={}ms",
                    state.name, raw_damage, spell_level as i32 + 2, delay_ms);
            }
            // --- 即时 AoE 类法术（任务4）---
            // FireBang/IceStorm：3×3 AoE，MAC 伤害（C# Map.cs:952）
            SPELL_FIREBANG | SPELL_ICE_STORM => {
                let raw_damage = if let Some(info) = spell_db {
                    crate::combat::magic::calc_magic_damage(info, spell_level, magic_stat)
                } else { fastrand::i32(5..=15) }.max(1);
                let attacker_stats = state.to_combat_stats();
                // 3×3：target 周围 ±1 格
                let hit_ids: Vec<u32> = self.monsters.iter()
                    .filter(|(_, m)| {
                        let dx = (m.x - target_x).abs();
                        let dy = (m.y - target_y).abs();
                        dx <= 1 && dy <= 1 && m.hp > 0 && m.map_index == state.map_index
                    })
                    .map(|(id, _)| *id)
                    .collect();
                for mid in hit_ids {
                    if let Some(monster) = self.monsters.get_mut(&mid) {
                        let defender_stats = monster.to_combat_stats();
                        // #1455：LevelOffset = Level > attacker.Level ? 0 : min(10, attacker.Level - Level)
                        let level_offset = crate::combat::attack::level_offset(state.level, monster.level.max(0) as u16);
                        let r = combat_attack::resolve_attack(
                            &attacker_stats, &defender_stats, raw_damage,
                            mir2_shared::enums::DefenceType::Mac, level_offset,
                        );
                        if r.is_hit && r.damage > 0 {
                            monster.take_damage(r.damage);
                            monster.last_hitter_session = Some(msg.session_id);
                            self.pending_gather.push(msg.session_id);
                            monster.provoked = true;
                            // C# MonsterObject.Attacked：仅当无目标时锁定攻击者（魔法伤害同物理）
                            if monster.target_session.is_none() {
                                monster.target_session = Some(msg.session_id);
                            }
                            for p in &r.applied_poisons {
                                crate::combat::poison::apply_poison(&mut monster.poison_list, *p);
                            }
                        }
                    }
                }
                debug!("Magic: {} casts FireBang/IceStorm (3x3) dmg={}", state.name, raw_damage);
            }
            // Lightning：直线 6 格，每格首目标，MAC（C# Map.cs:1189）
            SPELL_LIGHTNING => {
                let raw_damage = if let Some(info) = spell_db {
                    crate::combat::magic::calc_magic_damage(info, spell_level, magic_stat)
                } else { fastrand::i32(5..=15) }.max(1);
                let attacker_stats = state.to_combat_stats();
                let dir = msg.direction as usize % 8;
                let mut cx = state.x;
                let mut cy = state.y;
                for _ in 0..6 {
                    cx += MON_DIR_DX[dir];
                    cy += MON_DIR_DY[dir];
                    // 找该格第一个怪物
                    let hit = self.monsters.iter()
                        .find(|(_, m)| m.map_index == state.map_index && m.x == cx && m.y == cy && m.hp > 0)
                        .map(|(id, _)| *id);
                    if let Some(mid) = hit {
                        if let Some(monster) = self.monsters.get_mut(&mid) {
                            let defender_stats = monster.to_combat_stats();
                            // #1455：LevelOffset = Level > attacker.Level ? 0 : min(10, attacker.Level - Level)
                            let level_offset = crate::combat::attack::level_offset(state.level, monster.level.max(0) as u16);
                            let r = combat_attack::resolve_attack(
                                &attacker_stats, &defender_stats, raw_damage,
                                mir2_shared::enums::DefenceType::Mac, level_offset,
                            );
                            if r.is_hit && r.damage > 0 {
                                monster.take_damage(r.damage);
                                monster.last_hitter_session = Some(msg.session_id);
                                self.pending_gather.push(msg.session_id);
                                monster.provoked = true;
                                // C# MonsterObject.Attacked：仅当无目标时锁定攻击者（魔法伤害同物理）
                                if monster.target_session.is_none() {
                                    monster.target_session = Some(msg.session_id);
                                }
                                for p in &r.applied_poisons {
                                    crate::combat::poison::apply_poison(&mut monster.poison_list, *p);
                                }
                            }
                        }
                        // C# 每格 break（只打第一个），但外层 i 继续 → 每格各打第一个
                    }
                }
                debug!("Magic: {} casts Lightning (line 6) dmg={}", state.name, raw_damage);
            }
            // ThunderStorm/FlameField：5×5 自身周围，MAC（C# Map.cs:1303）
            // ThunderStorm 对非亡灵伤害 ×1/10（下方按 monster.undead 调整）
            SPELL_THUNDERSTORM | SPELL_FLAME_FIELD => {
                let raw_damage = if let Some(info) = spell_db {
                    crate::combat::magic::calc_magic_damage(info, spell_level, magic_stat)
                } else { fastrand::i32(5..=15) }.max(1);
                let attacker_stats = state.to_combat_stats();
                let hit_ids: Vec<u32> = self.monsters.iter()
                    .filter(|(_, m)| {
                        let dx = (m.x - state.x).abs();
                        let dy = (m.y - state.y).abs();
                        dx <= 2 && dy <= 2 && m.hp > 0 && m.map_index == state.map_index
                    })
                    .map(|(id, _)| *id)
                    .collect();
                for mid in hit_ids {
                    if let Some(monster) = self.monsters.get_mut(&mid) {
                        let defender_stats = monster.to_combat_stats();
                        // ThunderStorm 对非亡灵伤害 ×1/10（C# Map.cs:1332），FlameField 全额
                        let is_thunderstorm = msg.spell == SPELL_THUNDERSTORM;
                        let adjusted_dmg = if is_thunderstorm && !monster.undead {
                            raw_damage / 10
                        } else {
                            raw_damage
                        };
                        // #1455：LevelOffset = Level > attacker.Level ? 0 : min(10, attacker.Level - Level)
                        let level_offset = crate::combat::attack::level_offset(state.level, monster.level.max(0) as u16);
                        let r = combat_attack::resolve_attack(
                            &attacker_stats, &defender_stats, adjusted_dmg,
                            mir2_shared::enums::DefenceType::Mac, level_offset,
                        );
                        if r.is_hit && r.damage > 0 {
                            monster.take_damage(r.damage);
                            monster.last_hitter_session = Some(msg.session_id);
                            self.pending_gather.push(msg.session_id);
                            monster.provoked = true;
                            // C# MonsterObject.Attacked：仅当无目标时锁定攻击者（魔法伤害同物理）
                            if monster.target_session.is_none() {
                                monster.target_session = Some(msg.session_id);
                            }
                            for p in &r.applied_poisons {
                                crate::combat::poison::apply_poison(&mut monster.poison_list, *p);
                            }
                        }
                    }
                }
                debug!("Magic: {} casts ThunderStorm/FlameField (5x5) dmg={}", state.name, raw_damage);
            }
            // #306：HellFire —— 三向直线 AoE（C# HumanObject.HellFire：Lv3 三向，各 4 格，MAC）
            SPELL_HELLFIRE => {
                let raw_damage = if let Some(info) = spell_db {
                    crate::combat::magic::calc_magic_damage(info, spell_level, magic_stat)
                } else { fastrand::i32(8..=20) }.max(1);
                let attacker_stats = state.to_combat_stats();
                let cells = hellfire_cells(state.x, state.y, msg.direction, spell_level);
                let hit_ids: Vec<u32> = self.monsters.iter()
                    .filter(|(_, m)| m.hp > 0 && m.map_index == state.map_index && cells.contains(&(m.x, m.y)))
                    .map(|(id, _)| *id)
                    .collect();
                let mut spell_hits: Vec<(u32, i32, i32, u8, i32)> = Vec::new();
                for mid in hit_ids {
                    if let Some(monster) = self.monsters.get_mut(&mid) {
                        let defender_stats = monster.to_combat_stats();
                        // #1455：LevelOffset = Level > attacker.Level ? 0 : min(10, attacker.Level - Level)
                        let level_offset = crate::combat::attack::level_offset(state.level, monster.level.max(0) as u16);
                        let r = combat_attack::resolve_attack(
                            &attacker_stats, &defender_stats, raw_damage,
                            mir2_shared::enums::DefenceType::Mac, level_offset,
                        );
                        if r.is_hit && r.damage > 0 {
                            monster.take_damage(r.damage);
                            monster.last_hitter_session = Some(msg.session_id);
                            self.pending_gather.push(msg.session_id);
                            monster.provoked = true;
                            // C# MonsterObject.Attacked：仅当无目标时锁定攻击者（魔法伤害同物理）
                            if monster.target_session.is_none() {
                                monster.target_session = Some(msg.session_id);
                            }
                            for p in &r.applied_poisons {
                                crate::combat::poison::apply_poison(&mut monster.poison_list, *p);
                            }
                            spell_hits.push((mid, monster.x, monster.y, monster.direction, r.damage));
                        }
                    }
                }
                self.broadcast_spell_hit(&spell_hits, object_id).await;
                debug!("Magic: {} casts HellFire ({} cells) dmg={} hits={}", state.name, cells.len(), raw_damage, spell_hits.len());
            }
            // #306：IceThrust —— 前方 1 格幸运暴击 + 60% 溅射（C# HumanObject.IceThrust）
            SPELL_ICETHRUST => {
                let mut raw_damage = if let Some(info) = spell_db {
                    crate::combat::magic::calc_magic_damage(info, spell_level, magic_stat)
                } else { fastrand::i32(8..=20) }.max(1);
                // C#：Random.Next(100) < (1 + Luck) → 伤害翻倍
                if fastrand::i32(0..100) < (1 + state.luck) {
                    raw_damage *= 2;
                }
                let attacker_stats = state.to_combat_stats();
                let cells = icethrust_cells(state.x, state.y, msg.direction);
                let mut spell_hits: Vec<(u32, i32, i32, u8, i32)> = Vec::new();
                for (i, (cx, cy)) in cells.iter().enumerate() {
                    let dmg = if i == 0 { raw_damage } else { (raw_damage as f32 * 0.6) as i32 };
                    let hit: Option<u32> = self.monsters.iter()
                        .find(|(_, m)| m.map_index == state.map_index && m.x == *cx && m.y == *cy && m.hp > 0)
                        .map(|(id, _)| *id);
                    if let Some(mid) = hit {
                        if let Some(monster) = self.monsters.get_mut(&mid) {
                            let defender_stats = monster.to_combat_stats();
                            // #1455：LevelOffset = Level > attacker.Level ? 0 : min(10, attacker.Level - Level)
                            let level_offset = crate::combat::attack::level_offset(state.level, monster.level.max(0) as u16);
                            let r = combat_attack::resolve_attack(
                                &attacker_stats, &defender_stats, dmg,
                                mir2_shared::enums::DefenceType::Mac, level_offset,
                            );
                            if r.is_hit && r.damage > 0 {
                                monster.take_damage(r.damage);
                                monster.last_hitter_session = Some(msg.session_id);
                                self.pending_gather.push(msg.session_id);
                                monster.provoked = true;
                                // C# MonsterObject.Attacked：仅当无目标时锁定攻击者（魔法伤害同物理）
                                if monster.target_session.is_none() {
                                    monster.target_session = Some(msg.session_id);
                                }
                                spell_hits.push((mid, monster.x, monster.y, monster.direction, r.damage));
                            }
                        }
                    }
                }
                self.broadcast_spell_hit(&spell_hits, object_id).await;
                debug!("Magic: {} casts IceThrust dmg={} hits={}", state.name, raw_damage, spell_hits.len());
            }
            // #306/#1508：Curse —— 7×7 区域每目标 40% 概率 Slow 毒 + 减伤（C# Map.cs:1837，value2=1+(Lv+1)*2）
            SPELL_CURSE => {
                // #1445：C# Curse 需普通护符并消耗 1（失败也消耗；HumanObject.cs:4860）
                if !record.actor_ref.ask(crate::actors::player::ConsumeAmuletForSummon { amount: 1 }).await.unwrap_or(false) {
                    debug!("Magic: {} casts Curse but has no amulet", state.name);
                    return;
                }
                let value2 = 1 + (spell_level as i32 + 1) * 2;
                // C# Curse：Random(10-(Lv+1)*2) > 2 失败（Lv0≈37.5% → Lv3=100%）
                let chance_n = (10 - (spell_level as i32 + 1) * 2).max(1);
                if fastrand::i32(0..chance_n) > 2 {
                    debug!("Magic: {} casts Curse (failed, n={})", state.name, chance_n);
                    return;
                }
                // C# damage = magic.GetDamage(SC)，Envir.cs MPowerBase=20 → +5(Lv+1)
                let sc_power = crate::combat::attack::get_attack_power(
                    state.min_sc + state.bonus_min_sc,
                    state.max_sc + state.bonus_max_sc,
                    0,
                );
                let damage = (sc_power + 5 * (spell_level as i32 + 1)).max(1);
                let cells = curse_cells(target_x, target_y);
                let duration = damage as u32;
                // —— 怪物目标（C# IsAttackTarget：跳过自己的宠物；每目标 Random.Next(10)>=4 跳过）——
                let hit_ids: Vec<u32> = self.monsters.iter()
                    .filter(|(_, m)| m.hp > 0 && m.map_index == state.map_index && cells.contains(&(m.x, m.y))
                        && m.master_session != Some(msg.session_id))
                    .map(|(id, _)| *id)
                    .collect();
                let monster_candidates = hit_ids.len();
                for mid in hit_ids {
                    if fastrand::i32(0..10) >= 4 { continue; }
                    if let Some(monster) = self.monsters.get_mut(&mid) {
                        // Slow 毒（C# Duration=damage 秒，Value=value2）
                        crate::combat::poison::apply_poison(
                            &mut monster.poison_list,
                            crate::combat::poison::Poison::new(
                                mir2_shared::enums::PoisonType::SLOW,
                                duration,
                                value2,
                                1000,
                            ),
                        );
                        monster.provoked = true;
                        monster.target_session = Some(msg.session_id);
                        // 减伤：value2%（C# 降低 MaxDC/MaxMC/MaxSC 输出百分比），持续 damage 秒
                        let until = self.tick_count + duration as u64 * 10;
                        self.cursed_monsters.insert(mid, (value2, until));
                    }
                }
                // —— 玩家目标（#1508：C# 7x7 含 Player；MaxDC/MC/SC + AttackSpeed RatePercent=-value2）——
                let now_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);
                let mut player_targets: Vec<(u64, crate::actors::player::PlayerState)> = Vec::new();
                for (sid, r) in &self.players {
                    if *sid == msg.session_id { continue; }
                    if let Ok(Some(os)) = r.actor_ref.ask(GetPlayerState).await {
                        if !os.is_dead && os.map_index == state.map_index
                            && cells.contains(&(os.x, os.y))
                        {
                            player_targets.push((*sid, os));
                        }
                    }
                }
                let player_candidates = player_targets.len();
                for (sid, os) in player_targets {
                    // C# IsAttackTarget(player)：按当前攻击模式判定
                    if !can_attack_player(&state, &os, &self.guild_wars) { continue; }
                    // 每目标 Random.Next(10)>=4 跳过（40% 命中）
                    if fastrand::i32(0..10) >= 4 { continue; }
                    if let Some(record2) = self.players.get(&sid) {
                        let mut ns = os.clone();
                        ns.poison_list.push(crate::combat::poison::Poison::new(
                            mir2_shared::enums::PoisonType::SLOW, duration, value2, 1000,
                        ));
                        crate::combat::buff::apply_buff(&mut ns.buffs, crate::combat::buff::BuffInstance::new(
                            crate::combat::buff::BuffType::Curse { percent: value2 },
                            duration * 10,
                            1,
                        ));
                        let _ = record2.actor_ref.ask(crate::actors::player::SetPlayerState { state: ns }).await;
                    }
                }
                debug!("Magic: {} casts Curse (7x7, monsters={} players={}, rate={}%)",
                       state.name, monster_candidates, player_candidates, value2);
            }
            // ===== 弓箭手（Archer）弹道物理系法术 =====
            // StraightShot：单目标弹道，延迟 = 距离×50ms + 500ms，AC 防御（弓箭手物理）
            // DoubleShot：对目标连发 2 次弹道（第二次延迟 +200ms）
            // BindingShot：弹道 + 命中后 Paralysis（在 complete_projectile_spell 结算）
            // NapalmShot：弹道 + 命中后 3×3 AOE（在 complete_projectile_spell 结算）
            // 伤害基于 DC（物理攻击），用 magic_stat（弓箭手类 = effective_max_attack）
            SPELL_STRAIGHT_SHOT | SPELL_DOUBLE_SHOT | SPELL_BINDING_SHOT | SPELL_NAPALM_SHOT | SPELL_CAT_TONGUE
            | SPELL_VAMPIRE_SHOT | SPELL_POISON_SHOT | SPELL_CRIPPLE_SHOT | SPELL_ELEMENTAL_SHOT => {
                // #1483：C# SpecialArrowShot——VampireShot/PoisonShot 未武装时 40% 概率武装
                if (msg.spell == SPELL_VAMPIRE_SHOT || msg.spell == SPELL_POISON_SHOT) && state.special_shot_armed == 0 {
                    if fastrand::i32(0..20) >= 8 {
                        let armed = if msg.spell == SPELL_VAMPIRE_SHOT { 1 } else { 2 };
                        let _ = record.actor_ref.ask(crate::actors::player::SetSpecialShotArmed { armed }).await;
                        debug!("Player {} armed {} special shot (40%)", state.name, if armed == 1 { "Vampire" } else { "Poison" });
                    }
                }
                // #1528：C# GetRangeAttackPower(MinMC, MaxMC, distance)——弓手技能用 MC（魔法箭），与英雄弓手一致
                let archer_dist = (target_x - state.x).abs().max((target_y - state.y).abs());
                let mc_min = state.effective_min_mc();
                let mc_max = state.effective_max_mc();
                let eff_min = range_attack_min_reduction(mc_min, archer_dist);
                let mut raw_damage = (crate::combat::attack::get_attack_power(
                    eff_min, mc_max, state.luck,
                ) + (power as i32) / 2).max(1);
                // ElementalShot（C# HumanObject.ElementalShot）：无元素时施法凝聚第一档并取消射击；
                // 有元素时伤害 = GetAttackPower(MinMC, MaxMC) + 元素球攻击加成（OrbsDmgList）
                if msg.spell == SPELL_ELEMENTAL_SHOT {
                    if !state.has_elemental {
                        self.obtain_element(msg.session_id, true).await;
                        debug!("Magic: {} casts ElementalShot without orbs -> gather orb", state.name);
                        return;
                    }
                    let mc_power = crate::combat::attack::get_attack_power(
                        state.min_mc + state.bonus_min_mc,
                        state.max_mc + state.bonus_max_mc,
                        0,
                    );
                    let orb_power = crate::actors::world::elements::elemental_orb_power(
                        state.elements_level, false);
                    raw_damage = (mc_power + (power as i32) / 2 + orb_power).max(1);
                    debug!("Magic: {} ElementalShot orb_power +{} (elements_level={})",
                           state.name, orb_power, state.elements_level);
                }

                // #1519/#1520：C# ApplyArcherState——MentalState 惩罚（trickshot/group attack）
                let mental_lvl = state.magics.iter()
                    .find(|m| m.spell == (mir2_shared::enums::Spell::MentalState as i32 - 3))
                    .map(|m| m.level)
                    .unwrap_or(0);
                let archer_penalty = archer_state_penalty(
                    self.mental_state.get(&msg.session_id).copied().unwrap_or(0),
                    mental_lvl,
                );
                raw_damage = raw_damage * archer_penalty / 100;

                // 弹道延迟：距离×50ms + 500ms（C# MaxDistance=Chebyshev）
                let target_dist = archer_dist as u64;
                let base_delay_ms = target_dist * 50 + 500;
                // tick_count 每 100ms +1，按 100ms 取整（最少 1 tick）
                let fire_at_tick = self.tick_count + (base_delay_ms / 100).max(1);

                self.pending_spell_completions.push(PendingSpellCompletion {
                    fire_at_tick,
                    session_id: msg.session_id,
                    spell: msg.spell,
                    target_id: msg.target_id,
                    target_x,
                    target_y,
                    damage: raw_damage,
                    magic_stat: mc_max,
                    hero_stats: None,
                    hero_level: None,
                    spell_level,
                    bounce: 0,
                });

                // DoubleShot：额外发一发，延迟 +200ms（2 ticks）
                if msg.spell == SPELL_DOUBLE_SHOT {
                    self.pending_spell_completions.push(PendingSpellCompletion {
                        fire_at_tick: fire_at_tick + 2,
                        session_id: msg.session_id,
                        spell: msg.spell,
                        target_id: msg.target_id,
                        target_x,
                        target_y,
                        damage: raw_damage,
                        magic_stat: mc_max,
                        hero_stats: None,
                        hero_level: None,
                        spell_level,
                        bounce: 0,
                    });
                }
                debug!("Magic: {} casts Archer projectile spell={} dmg={} delay={}ms (DoubleShot={})",
                    state.name, msg.spell, raw_damage, base_delay_ms, msg.spell == SPELL_DOUBLE_SHOT);
            }
            // Concentration：专注 buff（MP 回复），时长 45+15*Lv 秒（C# HumanObject.Concentration）
            SPELL_CONCENTRATION => {
                let bonus = 3 + spell_level as i32 * 2;
                let duration_ticks = ((45 + 15 * spell_level as i32) as u32) * 10;
                let buff = crate::combat::buff::BuffInstance::new(
                    crate::combat::buff::BuffType::MpRegenBoost { bonus },
                    duration_ticks,
                    5,
                );
                let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff { buff }).await;
                // 重置打断状态 + 广播 SetConcentration（C# UpdateConcentration(true,false)）
                let _ = record.actor_ref.ask(crate::actors::player::SetConcentrationInterrupt {
                    interrupted: false,
                    interrupt_time_ms: 0,
                }).await;
                self.concentration_visible.insert(msg.session_id, true);
                self.broadcast_set_concentration(state.object_id, true, false, state.map_index).await;
                debug!("Magic: {} casts Concentration (MP regen +{}, {}s)",
                       state.name, bonus, 45 + 15 * spell_level as i32);
            }
            // ElementalBarrier：元素护盾（C# HumanObject.cs:6417）——
            // 已有护盾不叠加；无元素时施法凝聚并取消；有元素时消耗元素获得防御加成
            SPELL_ELEMENTAL_BARRIER => {
                let reduction_pct = ((spell_level as i32 + 1) * 10).min(80);
                // C#：已有 ElementalBarrier buff 直接 return
                if state.buffs.iter().any(|b| matches!(
                    b.buff_type, crate::combat::buff::BuffType::DamageReduction { .. }))
                {
                    debug!("Magic: {} ElementalBarrier already active, skip", state.name);
                    return;
                }
                // C#：无元素时凝聚第一档并返回（不施放护盾）
                if !state.has_elemental {
                    self.obtain_element(msg.session_id, true).await;
                    debug!("Magic: {} casts ElementalBarrier without orbs -> gather orb", state.name);
                    return;
                }
                let mc_power = crate::combat::attack::get_attack_power(
                    state.min_mc + state.bonus_min_mc,
                    state.max_mc + state.bonus_max_mc,
                    0,
                ).max(1);
                let barrier_power = crate::actors::world::elements::elemental_orb_power(
                    state.elements_level, true);
                let duration_ticks = ((mc_power + barrier_power) as u32) * 10;
                // 消耗元素（C# ElementsLevel=0; ObtainElement(false)）
                self.consume_elemental(msg.session_id).await;
                let _ = record.actor_ref.ask(crate::actors::player::ApplyDamageReduction {
                    percent: reduction_pct,
                    duration_ticks,
                }).await;
                // C# CurrentMap.Broadcast(ObjectEffect ElementalBarrierUp)
                self.broadcast_object_effect(
                    state.object_id, mir2_shared::enums::SpellEffect::ElementalBarrierUp,
                    state.map_index,
                ).await;
                debug!("Magic: {} casts ElementalBarrier (damage -{}%, {}s, orb +{}s)",
                       state.name, reduction_pct, mc_power + barrier_power, barrier_power);
            }
            // Mirroring：分身术（C# HumanObject.cs Mirroring）——召唤 Clone 分身宠物（Settings.CloneName="Clone"）
            SPELL_MIRRORING => {
                const CLONE_NAME: &str = "Clone";
                // 已有存活分身 → 移除（C# monster.Die()）
                let existing: Option<u32> = self.monsters.iter()
                    .find(|(_, m)| m.master_session == Some(msg.session_id)
                        && m.name.eq_ignore_ascii_case(CLONE_NAME) && m.hp > 0)
                    .map(|(id, _)| *id);
                if let Some(oid) = existing {
                    if self.monsters.remove(&oid).is_some() {
                        let rm = Self::build_object_remove_packet(oid);
                        broadcast_to_map(&self.gate_ref, &self.players, state.map_index, &rm).await;
                    }
                    debug!("Magic: {} Mirroring removed existing clone #{}", state.name, oid);
                    return;
                }
                // 生成在前方 1 格（C# Front）
                let dir = msg.direction as usize % 8;
                let (sx, sy) = (state.x + MON_DIR_DX[dir], state.y + MON_DIR_DY[dir]);
                let mon_index = self.monster_name_index.get(CLONE_NAME.to_lowercase().as_str()).copied();
                match mon_index {
                    Some(idx) => {
                        if let Some(info) = self.monster_infos.get(&idx).cloned() {
                            let new_oid = self.alloc_object_id();
                            let hp = info.stats.get(&(mir2_shared::enums::Stat::HP as u8)).copied().unwrap_or(50);
                            let min_dmg = info.stats.get(&(mir2_shared::enums::Stat::MinDC as u8)).copied().unwrap_or(5);
                            let max_dmg = info.stats.get(&(mir2_shared::enums::Stat::MaxDC as u8)).copied().unwrap_or(10);
                            let spawn = MonsterSpawn {
                                name: info.name.clone(),
                                image: info.image as u16,
                                monster_index: idx,
                                x: sx, y: sy,
                                direction: msg.direction,
                                hp, min_dmg, max_dmg,
                                xp: info.experience,
                                map_index: state.map_index,
                                count: 1,
                                spread: 0,
                            };
                            let packet = build_object_monster_packet(&spawn, new_oid, &spawn.name);
                            broadcast_to_map(&self.gate_ref, &self.players, state.map_index, &packet).await;
                            let ai_profile = MonsterAiProfile::from_info(&info);
                            self.monsters.insert(new_oid, MonsterState {
                                object_id: new_oid,
                                name: spawn.name.clone(),
                                image: spawn.image,
                                monster_index: idx,
                                x: sx, y: sy, direction: msg.direction,
                                hp, max_hp: hp, min_dmg, max_dmg, xp: spawn.xp,
                                spawn_x: sx, spawn_y: sy, map_index: state.map_index,
                                spawn_spread: 0,
                                next_attack_tick: 0, next_move_tick: 0, next_summon_tick: 0,
                                ai_profile, ai_state: MonsterAiState::Idle,
 sitting: false,
 hidden: false,
 sit_down_tick: 0,
                                target_session: None,
                                last_hitter_session: None, provoked: false,
                                is_elite: false, is_boss: false,
                                min_ac: 0, max_ac: 0, min_mac: 0, max_mac: 0,
                                agility: 0, accuracy: 0,
                                armour_rate: 1.0, damage_rate: 1.0,
                                magic_resist: 0, critical_rate: 0, critical_damage: 0,
                                luck: 0, reflect: 0, damage_reduction_percent: 0, level: info.level, effect: info.effect,
                                poison_list: Vec::new(),
                                last_hit_damage: 0,
                                undead: info.undead,
                                master_session: Some(msg.session_id),
                                rarity: 0,
                                pet_experience: 0,
                                max_pet_level: 0,
                                recall_at_tick: 0,
                                behavior: crate::actors::world::ai::make_behavior(&spawn.name),
                            });
                            self.pet_levels.insert(new_oid, spell_level as i32);
                            debug!("Magic: {} casts Mirroring -> clone #{} at ({},{})",
                                   state.name, new_oid, sx, sy);
                        } else {
                            warn!("Mirroring '{}' found index {} but no MonsterInfo", CLONE_NAME, idx);
                        }
                    }
                    None => {
                        warn!("Mirroring '{}' not in monster_name_index (DB may lack this mob)", CLONE_NAME);
                    }
                }
            }
            // ===== 刺客法术（Assassin，buff 系 + 位移系 + 物理攻击系）=====
            // Haste：攻击速度提升（C# CompleteMagic 6149：AttackSpeed stat += Lv*2+2，时长 25+15Lv 秒）
            SPELL_HASTE => {
                // #1506：C# Stats[AttackSpeed] = Lv*2+2（2..8），AttackTime 公式直接消费 stat
                let pct = 2 + spell_level as i32 * 2;
                let buff = crate::combat::buff::BuffInstance::new(
                    crate::combat::buff::BuffType::AttackSpeedBoost { percent: pct },
                    (25 + spell_level as u32 * 15) * 10,
                    5,
                );
                let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff { buff }).await;
                debug!("Magic: {} casts Haste (attack speed +{}%, {}s)",
                       state.name, pct, 25 + spell_level as i32 * 15);
            }
            // LightBody：敏捷提升（C# CompleteMagic 6187：Agility += (Lv+1)*2，时长 (Lv+1)*30 秒）
            SPELL_LIGHT_BODY => {
                let agi_bonus = (spell_level as i32 + 1) * 2;
                let buff1 = crate::combat::buff::BuffInstance::new(
                    crate::combat::buff::BuffType::AgilityBoost { bonus: agi_bonus },
                    (spell_level as u32 + 1) * 300,
                    5,
                );
                let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff { buff: buff1 }).await;
                debug!("Magic: {} casts LightBody (agility +{}, {}s)",
                       state.name, agi_bonus, (spell_level as i32 + 1) * 30);
            }
            // Fury：攻速提升（C# CompleteMagic 6160：Stat.AttackSpeed=4，时长 60+10Lv 秒）
            SPELL_FURY => {
                let buff = crate::combat::buff::BuffInstance::new(
                    crate::combat::buff::BuffType::AttackSpeedBoost { percent: 4 },
                    (60 + spell_level as u32 * 10) * 10,
                    5,
                );
                let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff { buff }).await;
                debug!("Magic: {} casts Fury (attack speed +4, {}s)",
                       state.name, 60 + spell_level as i32 * 10);
            }
            // Rage：DC 提升（C# HumanObject.cs Rage：MaxDC/MinDC += round(MaxDC*(0.12+0.03Lv))，18+6Lv 秒）
            SPELL_RAGE => {
                let add_value = (state.max_attack as f32 * (0.12 + 0.03 * spell_level as f32)).round() as i32;
                let buff = crate::combat::buff::BuffInstance::new(
                    crate::combat::buff::BuffType::AttackBoost { bonus: add_value.max(1) },
                    (18 + spell_level as u32 * 6) * 10,
                    5,
                );
                let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff { buff }).await;
                debug!("Magic: {} casts Rage (DC +{}, {}s)",
                       state.name, add_value.max(1), 18 + spell_level as i32 * 6);
            }
            // SwiftFeet：移动速度大幅提升
            SPELL_SWIFT_FEET => {
                let spd_pct = 30 + spell_level as i32 * 10;
                let buff = crate::combat::buff::BuffInstance::new(
                    crate::combat::buff::BuffType::MoveSpeedBoost { percent: spd_pct }, 250 + spell_level as u32 * 50, 5);
                let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff { buff }).await;
                debug!("Magic: {} casts SwiftFeet (move speed +{}%)", state.name, spd_pct);
            }
            // MoonLight：隐身（刺客版，怪物失去目标）
            // C# 时长：(GetAttackPower(MinAC,MaxAC) + (Lv+1)*5) * 500ms
            SPELL_MOON_LIGHT => {
                let ac_power = crate::combat::attack::get_attack_power(
                    state.min_ac + state.bonus_min_ac,
                    state.max_ac + state.bonus_max_ac,
                    0,
                );
                let duration_ticks = ((ac_power + (spell_level as i32 + 1) * 5).max(1) as u32) * 5;
                let buff = crate::combat::buff::BuffInstance::new(
                    crate::combat::buff::BuffType::Invisibility,
                    duration_ticks, 5);
                let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff { buff }).await;
                self.invisible_sessions.insert(msg.session_id);
            if let Ok(Some(st)) = record.actor_ref.ask(GetPlayerState).await {
                self.broadcast_object_hidden(st.object_id, true, st.map_index).await;
            }
                debug!("Magic: {} casts MoonLight (invisible {}s)", state.name, duration_ticks / 10);
            }
            // DarkBody：刺客分身（C# HumanObject.cs:5323）——召唤 AssassinClone 宠物；已有存活分身则移除
            SPELL_DARK_BODY => {
                const CLONE_NAME: &str = "AssassinClone";
                // 已有存活分身 → 移除（C# monster.Die()）
                let existing: Option<u32> = self.monsters.iter()
                    .find(|(_, m)| m.master_session == Some(msg.session_id)
                        && m.name.eq_ignore_ascii_case(CLONE_NAME) && m.hp > 0)
                    .map(|(id, _)| *id);
                if let Some(oid) = existing {
                    if self.monsters.remove(&oid).is_some() {
                        let rm = Self::build_object_remove_packet(oid);
                        broadcast_to_map(&self.gate_ref, &self.players, state.map_index, &rm).await;
                    }
                    debug!("Magic: {} DarkBody removed existing clone #{}", state.name, oid);
                    return;
                }
                // 目标玩家 session（C# monster.Target = 点击目标）
                let target_session: Option<u64> = if msg.target_id != 0 {
                    let mut found = None;
                    for (sid, r) in &self.players {
                        if let Ok(Some(os)) = r.actor_ref.ask(GetPlayerState).await {
                            if os.object_id == msg.target_id {
                                found = Some(*sid);
                                break;
                            }
                        }
                    }
                    found
                } else {
                    None
                };
                let mon_index = self.monster_name_index.get(CLONE_NAME.to_lowercase().as_str()).copied();
                match mon_index {
                    Some(idx) => {
                        if let Some(info) = self.monster_infos.get(&idx).cloned() {
                            let new_oid = self.alloc_object_id();
                            let hp = info.stats.get(&(mir2_shared::enums::Stat::HP as u8)).copied().unwrap_or(50);
                            let min_dmg = info.stats.get(&(mir2_shared::enums::Stat::MinDC as u8)).copied().unwrap_or(5);
                            let max_dmg = info.stats.get(&(mir2_shared::enums::Stat::MaxDC as u8)).copied().unwrap_or(10);
                            let spawn = MonsterSpawn {
                                name: info.name.clone(),
                                image: info.image as u16,
                                monster_index: idx,
                                x: state.x,
                                y: state.y,
                                direction: msg.direction,
                                hp,
                                min_dmg,
                                max_dmg,
                                xp: info.experience,
                                map_index: state.map_index,
                                count: 1,
                                spread: 0,
                            };
                            let packet = build_object_monster_packet(&spawn, new_oid, &spawn.name);
                            broadcast_to_map(&self.gate_ref, &self.players, state.map_index, &packet).await;
                            let ai_profile = MonsterAiProfile::from_info(&info);
                            self.monsters.insert(new_oid, MonsterState {
                                object_id: new_oid,
                                name: spawn.name.clone(),
                                image: spawn.image,
                                monster_index: idx,
                                x: state.x, y: state.y, direction: msg.direction,
                                hp, max_hp: hp, min_dmg, max_dmg, xp: spawn.xp,
                                spawn_x: state.x, spawn_y: state.y, map_index: state.map_index,
                                spawn_spread: 0,
                                next_attack_tick: 0, next_move_tick: 0, next_summon_tick: 0,
                                ai_profile, ai_state: MonsterAiState::Idle,
 sitting: false,
 hidden: false,
 sit_down_tick: 0,
                                target_session, provoked: target_session.is_some(),
                                last_hitter_session: None,
                                is_elite: false, is_boss: false,
                                min_ac: 0, max_ac: 0, min_mac: 0, max_mac: 0,
                                agility: 0, accuracy: 0,
                                armour_rate: 1.0, damage_rate: 1.0,
                                magic_resist: 0, critical_rate: 0, critical_damage: 0,
                                luck: 0, reflect: 0, damage_reduction_percent: 0, level: info.level, effect: info.effect,
                                poison_list: Vec::new(),
                                last_hit_damage: 0,
                                undead: info.undead,
                                master_session: Some(msg.session_id),
                                rarity: 0,
                                pet_experience: 0,
                                max_pet_level: 0,
                                recall_at_tick: 0,
                                behavior: crate::actors::world::ai::make_behavior(&spawn.name),
                            });
                            self.pet_levels.insert(new_oid, spell_level as i32);
                            debug!("Magic: {} casts DarkBody -> clone #{} at ({},{})",
                                   state.name, new_oid, state.x, state.y);
                        } else {
                            warn!("DarkBody '{}' found index {} but no MonsterInfo", CLONE_NAME, idx);
                        }
                    }
                    None => {
                        warn!("DarkBody '{}' not in monster_name_index (DB may lack this mob)", CLONE_NAME);
                    }
                }
            }
            // HeavenlySword：直线 3 格 AoE（物理 AC 防御，类似 Thrusting 但更长）
            SPELL_HEAVENLY_SWORD => {
                let dir = msg.direction as usize % 8;
                let attacker_stats = state.to_combat_stats();
                let raw_damage = crate::combat::attack::get_attack_power(
                    attacker_stats.min_atk, attacker_stats.max_atk, attacker_stats.luck);
                let mut cx = state.x;
                let mut cy = state.y;
                let mut hit_ids: Vec<u32> = Vec::new();
                for _ in 0..3 {
                    cx += MON_DIR_DX[dir];
                    cy += MON_DIR_DY[dir];
                    if let Some((&mid, _)) = self.monsters.iter().find(|(_, m)| m.map_index == state.map_index && m.x == cx && m.y == cy && m.hp > 0) {
                        hit_ids.push(mid);
                    }
                }
                for mid in hit_ids {
                    if let Some(m) = self.monsters.get_mut(&mid) {
                        let ds = m.to_combat_stats();
                        // #1455：LevelOffset = Level > attacker.Level ? 0 : min(10, attacker.Level - Level)
                        let level_offset = crate::combat::attack::level_offset(state.level, m.level.max(0) as u16);
                        let r = combat_attack::resolve_attack(
                            &attacker_stats, &ds, raw_damage,
                            mir2_shared::enums::DefenceType::AcAgility, level_offset);
                        if r.is_hit && r.damage > 0 {
                            m.take_damage(r.damage);
                            m.last_hitter_session = Some(msg.session_id);
                            self.pending_gather.push(msg.session_id);
                            m.provoked = true;
                            // C# MonsterObject.Attacked：仅当无目标时锁定攻击者（魔法伤害同物理）
                            if m.target_session.is_none() {
                                m.target_session = Some(msg.session_id);
                            }
                            for p in &r.applied_poisons {
                                crate::combat::poison::apply_poison(&mut m.poison_list, *p);
                            }
                        }
                    }
                }
                debug!("Magic: {} casts HeavenlySword (line 3 AoE)", state.name);
            }
            // BladeAvalanche：冰刀斩（C# HumanObject.cs:4903）——3 列（前左/前/前右）×3 行前向 AoE
            // 前 2 行全额、第 3 行 60%；幸运暴击翻倍；MAC 防御
            SPELL_BLADE_AVALANCHE => {
                let mut raw = crate::combat::attack::get_attack_power(
                    state.min_attack + state.bonus_min_attack,
                    state.max_attack + state.bonus_max_attack,
                    state.luck,
                ).max(1);
                // C#：Random(0..100) <= 1+Luck → 翻倍
                if fastrand::i32(0..100) <= 1 + state.luck {
                    raw *= 2;
                }
                // C# Envir.cs BladeAvalanche：倍率 1+0.4Lv（幸运翻倍保留）
                let raw = ((raw as f32) * (1.0 + 0.4 * spell_level as f32)).max(1.0) as i32;
                let attacker_stats = state.to_combat_stats();
                let dir = msg.direction as usize % 8;
                let prev = (dir + 7) % 8;
                let next = (dir + 1) % 8;
                let mut hit_count = 0;
                for col_dir in [prev, dir, next] {
                    let start_x = state.x + MON_DIR_DX[col_dir];
                    let start_y = state.y + MON_DIR_DY[col_dir];
                    for j in 0..3i32 {
                        let hx = start_x + MON_DIR_DX[dir] * j;
                        let hy = start_y + MON_DIR_DY[dir] * j;
                        let cell_dmg = if j <= 1 { raw } else { ((raw as f64) * 0.6) as i32 };
                        let hit_ids: Vec<u32> = self.monsters.iter()
                            .filter(|(_, m)| m.map_index == state.map_index && m.x == hx && m.y == hy && m.hp > 0)
                            .map(|(id, _)| *id)
                            .collect();
                        for mid in hit_ids {
                            if let Some(monster) = self.monsters.get_mut(&mid) {
                                let ds = monster.to_combat_stats();
                                // #1455：LevelOffset = Level > attacker.Level ? 0 : min(10, attacker.Level - Level)
                                let level_offset = crate::combat::attack::level_offset(state.level, monster.level.max(0) as u16);
                                let r = combat_attack::resolve_attack(
                                    &attacker_stats, &ds, cell_dmg,
                                    mir2_shared::enums::DefenceType::Mac, level_offset,
                                );
                                if r.is_hit && r.damage > 0 {
                                    monster.take_damage(r.damage);
                                    monster.last_hitter_session = Some(msg.session_id);
                                    self.pending_gather.push(msg.session_id);
                                    monster.provoked = true;
                                    // C# MonsterObject.Attacked：仅当无目标时锁定攻击者（魔法伤害同物理）
                                    if monster.target_session.is_none() {
                                        monster.target_session = Some(msg.session_id);
                                    }
                                    hit_count += 1;
                                }
                            }
                        }
                    }
                }
                debug!("Magic: {} casts BladeAvalanche (3x3 front, hits {})", state.name, hit_count);
            }
            // CrescentSlash：前方扇形 AoE（前+左前+右前 3 格）
            SPELL_CRESCENT_SLASH => {
                let dir = msg.direction as usize % 8;
                let attacker_stats = state.to_combat_stats();
                let raw = crate::combat::attack::get_attack_power(
                    attacker_stats.min_atk, attacker_stats.max_atk, attacker_stats.luck);
                // C# Envir.cs CrescentSlash：倍率 1+0.4Lv
                let raw_damage = ((raw as f32) * (1.0 + 0.4 * spell_level as f32)).max(1.0) as i32;
                // 扇形：前方 dir + 左前 (dir+7)%8 + 右前 (dir+1)%8
                let fan_dirs = [dir, (dir + 7) % 8, (dir + 1) % 8];
                let mut hit_ids: Vec<u32> = Vec::new();
                for fd in fan_dirs {
                    let tx = state.x + MON_DIR_DX[fd];
                    let ty = state.y + MON_DIR_DY[fd];
                    if let Some((&mid, _)) = self.monsters.iter().find(|(_, m)| m.map_index == state.map_index && m.x == tx && m.y == ty && m.hp > 0) {
                        hit_ids.push(mid);
                    }
                }
                for mid in hit_ids {
                    if let Some(m) = self.monsters.get_mut(&mid) {
                        let ds = m.to_combat_stats();
                        // #1455：LevelOffset = Level > attacker.Level ? 0 : min(10, attacker.Level - Level)
                        let level_offset = crate::combat::attack::level_offset(state.level, m.level.max(0) as u16);
                        let r = combat_attack::resolve_attack(
                            &attacker_stats, &ds, raw_damage,
                            mir2_shared::enums::DefenceType::AcAgility, level_offset);
                        if r.is_hit && r.damage > 0 {
                            m.take_damage(r.damage);
                            m.last_hitter_session = Some(msg.session_id);
                            self.pending_gather.push(msg.session_id);
                            m.provoked = true;
                            // C# MonsterObject.Attacked：仅当无目标时锁定攻击者（魔法伤害同物理）
                            if m.target_session.is_none() {
                                m.target_session = Some(msg.session_id);
                            }
                        }
                    }
                }
                debug!("Magic: {} casts CrescentSlash (fan 3 AoE)", state.name);
            }
            // FlashDash：向前突进 4 格（纯位移，成功率 (level+1)/4）
            SPELL_FLASH_DASH => {
                if fastrand::i32(0..4) >= spell_level as i32 + 1 {
                    debug!("Magic: {} FlashDash failed (random)", state.name);
                    // 失败仍消耗 MP，不 return（继续走 XP 流程）
                } else {
                    let dir = msg.direction as usize % 8;
                    let (max_x, max_y) = self.maps.get(&state.map_index)
                        .map(|m| (m.width as i32, m.height as i32))
                        .unwrap_or((i32::MAX, i32::MAX));
                    let tx = (state.x + MON_DIR_DX[dir] * 4).clamp(0, max_x - 1);
                    let ty = (state.y + MON_DIR_DY[dir] * 4).clamp(0, max_y - 1);
                    let _ = record.actor_ref.ask(crate::actors::player::SetPlayerPosition {
                        x: tx, y: ty, direction: msg.direction,
                        map_index: None, is_mounted: None,
                    }).await;
                    self.broadcast_position_change(msg.session_id, tx, ty, msg.direction).await;
                    debug!("Magic: {} casts FlashDash to ({},{})", state.name, tx, ty);
                }
            }
            // BackStep：向后跳跃 3 格（direction 相反方向）
            SPELL_BACK_STEP => {
                let dir = msg.direction as usize % 8;
                let back_dir = (dir + 4) % 8; // 反方向
                let (max_x, max_y) = self.maps.get(&state.map_index)
                    .map(|m| (m.width as i32, m.height as i32))
                    .unwrap_or((i32::MAX, i32::MAX));
                let tx = (state.x + MON_DIR_DX[back_dir] * 3).clamp(0, max_x - 1);
                let ty = (state.y + MON_DIR_DY[back_dir] * 3).clamp(0, max_y - 1);
                let _ = record.actor_ref.ask(crate::actors::player::SetPlayerPosition {
                    x: tx, y: ty, direction: back_dir as u8,
                    map_index: None, is_mounted: None,
                }).await;
                self.broadcast_position_change(msg.session_id, tx, ty, back_dir as u8).await;
                debug!("Magic: {} casts BackStep to ({},{})", state.name, tx, ty);
            }
            // --- 召唤系法术（道士/法师/弓箭手）：在施法者前方 1 格 spawn 一只战斗召唤物 ---
            SPELL_SUMMON_SKELETON | SPELL_SUMMON_SHINSU | SPELL_SUMMON_HOLY_DEVA
            | SPELL_SUMMON_VAMPIRE | SPELL_SUMMON_TOAD | SPELL_SUMMON_SNAKES => {
                // C# HumanObject.SummonXxx：NoPets 地图禁止召唤（CurrentMap.Info.NoPets → ReceiveChat + return）
                if self.map_infos.get(&(state.map_index as i32))
                    .map(|m| m.no_pets)
                    .unwrap_or(false)
                {
                    send_system_message(&self.gate_ref, msg.session_id, "该地图禁止召唤宠物");
                    return;
                }
                // 召唤物名映射（对齐 C# HumanObject.SummonXxx，名需在 DB monster_infos 里）
                let summon_name: &str = match msg.spell {
                    SPELL_SUMMON_SKELETON => "Skeleton",
                    SPELL_SUMMON_SHINSU => "Shinsu",
                    SPELL_SUMMON_HOLY_DEVA => "HolyDeva",
                    SPELL_SUMMON_VAMPIRE => "Vampire",
                    SPELL_SUMMON_TOAD => "Toad",
                    SPELL_SUMMON_SNAKES => "Snakes",
                    _ => unreachable!(),
                };
                let (max_x, max_y) = self.maps.get(&state.map_index)
                    .map(|m| (m.width as i32, m.height as i32))
                    .unwrap_or((i32::MAX, i32::MAX));
                let dir = msg.direction as usize % 8;
                // 召唤物生成在施法者前方 1 格（对齐 C# target point）
                let sx = (state.x + MON_DIR_DX[dir]).clamp(0, max_x - 1);
                let sy = (state.y + MON_DIR_DY[dir]).clamp(0, max_y - 1);

                // C# SummonXxx：已有同名存活宠物 → 召回（传送到施法者前方 1 格）并返回，不重复生成
                let existing: Option<u32> = self.monsters.iter()
                    .find(|(_, m)| m.master_session == Some(msg.session_id)
                        && m.name.eq_ignore_ascii_case(summon_name) && m.hp > 0)
                    .map(|(id, _)| *id);
                if let Some(oid) = existing {
                    if let Some(m) = self.monsters.get_mut(&oid) {
                        m.x = sx;
                        m.y = sy;
                        m.direction = dir as u8;
                        let mut walk_body = Vec::new();
                        walk_body.extend_from_slice(&oid.to_le_bytes());
                        walk_body.extend_from_slice(&sx.to_le_bytes());
                        walk_body.extend_from_slice(&sy.to_le_bytes());
                        walk_body.push(dir as u8);
                        let walk_packet = build_packet_bytes(
                            mir2_shared::enums::ServerPacketIds::ObjectWalk as i16, &walk_body);
                        broadcast_to_map(&self.gate_ref, &self.players, m.map_index, &walk_packet).await;
                        debug!("Magic: {} recalls existing summon '{}' #{}", state.name, summon_name, oid);
                    }
                    return;
                }

                // C# SummonXxx：Pets.Count(x => x.Race == ObjectType.Monster) >= 2 拒绝（静默）
                let pet_count = self.monsters.values()
                    .filter(|m| m.master_session == Some(msg.session_id) && m.hp > 0)
                    .count();
                if pet_count >= 2 {
                    return;
                }

                // C# SummonSkeleton/SummonShinsu/SummonHolyDeva：需护身符并消耗（GetAmulet + ConsumeItem）
                if matches!(msg.spell, SPELL_SUMMON_SKELETON | SPELL_SUMMON_SHINSU | SPELL_SUMMON_HOLY_DEVA) {
                    let amulet_amount = match msg.spell {
                        SPELL_SUMMON_SKELETON => 1,
                        SPELL_SUMMON_SHINSU => 5,
                        _ => 2, // SummonHolyDeva
                    };
                    let ok = record.actor_ref.ask(crate::actors::player::ConsumeAmuletForSummon {
                        amount: amulet_amount,
                    }).await.unwrap_or(false);
                    if !ok {
                        // C#：无对应护身符 → 施法失败（静默 return）
                        return;
                    }
                }

                // C#：道士/法师召唤永久；弓手召唤 AliveTime：
                // Vampire=Lv*1500+15000ms，Toad=Lv*2000+25000ms，Snakes=Lv*1500+20000ms
                let recall_at_tick = match msg.spell {
                    SPELL_SUMMON_VAMPIRE => spell_level as u64 * 15 + 150,
                    SPELL_SUMMON_TOAD => spell_level as u64 * 20 + 250,
                    SPELL_SUMMON_SNAKES => spell_level as u64 * 15 + 200,
                    _ => 0,
                };

                // 按 monster_name_index 查 MonsterInfo（lowercase key，对齐 tick.rs boss_summons）
                let mon_index = self.monster_name_index.get(&summon_name.to_lowercase()).copied();
                match mon_index {
                    Some(idx) => {
                        // 先 clone MonsterInfo 避免 &self.monster_infos 与 &mut self.alloc_object_id 借用冲突
                        let info_opt = self.monster_infos.get(&idx).cloned();
                        if let Some(info) = info_opt {
                            let new_oid = self.alloc_object_id();
                            // C# MonsterObject.RefreshAll：PetLevel 属性加成
                            // HP += PetLevel*20；DC += PetLevel；AC/MAC += PetLevel*2（AC/MAC 在 insert 后补）
                            let pet_level = spell_level as i32;
                            let max_pet_level = if msg.spell == SPELL_SUMMON_SKELETON {
                                4 + pet_level
                            } else {
                                1 + pet_level * 2
                            };
                            let base_hp = info.stats.get(&(mir2_shared::enums::Stat::HP as u8)).copied().unwrap_or(50);
                            let base_min_dmg = info.stats.get(&(mir2_shared::enums::Stat::MinDC as u8)).copied().unwrap_or(5);
                            let base_max_dmg = info.stats.get(&(mir2_shared::enums::Stat::MaxDC as u8)).copied().unwrap_or(10);
                            let hp = base_hp + pet_level * 20;
                            let min_dmg = base_min_dmg + pet_level;
                            let max_dmg = base_max_dmg + pet_level;
                            // 广播 ObjectMonster 给所有玩家（spawn 通知）
                            let spawn = MonsterSpawn {
                                name: info.name.clone(),
                                image: info.image as u16,
                                monster_index: idx,
                                x: sx,
                                y: sy,
                                direction: dir as u8,
                                hp,
                                min_dmg,
                                max_dmg,
                                xp: info.experience,
                                map_index: state.map_index,
                                count: 1,
                                spread: 0,
                            };
                            // C# MonsterObject.Name：宠物名字 = 怪物名(主人名)
                            let display_name = format!("{}({})", spawn.name, state.name);
                            // C# RefreshNameColour：宠物名字颜色按 PetLevel（ARGB）
                            let name_colour: i32 = (if pet_level == 0 {
                                0xFFFFFFFFu32
                            } else {
                                match pet_level {
                                    1 => 0xFF00FFFFu32, // Aqua
                                    2 => 0xFF7FFFD4u32, // Aquamarine
                                    3 => 0xFF20B2AAu32, // LightSeaGreen
                                    4 => 0xFF6A5ACDu32, // SlateBlue
                                    5 => 0xFF4682B4u32, // SteelBlue
                                    6 => 0xFF0000FFu32, // Blue
                                    7 => 0xFF000080u32, // Navy
                                    _ => 0xFFFFFFFFu32, // White
                                }
                            }) as i32;
                            // C# Shinsu/HolyDeva/VampireSpider.GetInfo：Extra = Summoned（召唤物标记）
                            let extra = matches!(
                                msg.spell,
                                SPELL_SUMMON_SHINSU | SPELL_SUMMON_HOLY_DEVA | SPELL_SUMMON_VAMPIRE
                            );
                            let packet = build_object_monster_packet_extra(
                                &spawn, new_oid, &display_name, extra, name_colour);
                            broadcast_to_map(&self.gate_ref, &self.players, state.map_index, &packet).await;
                            let ai_profile = MonsterAiProfile::from_info(&info);
                            // 召唤物：target_session=主人、provoked=true 主动攻击
                            self.monsters.insert(new_oid, MonsterState {
                                object_id: new_oid,
                                name: spawn.name.clone(),
                                image: spawn.image,
                                monster_index: idx,
                                x: sx, y: sy, direction: dir as u8,
                                hp, max_hp: hp, min_dmg, max_dmg, xp: spawn.xp,
                                spawn_x: sx, spawn_y: sy, map_index: state.map_index,
                                spawn_spread: 0,
                                next_attack_tick: 0, next_move_tick: 0, next_summon_tick: 0,
                                ai_profile, ai_state: MonsterAiState::Idle,
 sitting: false,
 hidden: false,
 sit_down_tick: 0,
                                target_session: Some(msg.session_id),
                                last_hitter_session: None, provoked: true,
                                is_elite: false, is_boss: false,
                                min_ac: 0, max_ac: 0, min_mac: 0, max_mac: 0,
                                agility: 0, accuracy: 0,
                                armour_rate: 1.0, damage_rate: 1.0,
                                magic_resist: 0, critical_rate: 0, critical_damage: 0,
                                luck: 0, reflect: 0, damage_reduction_percent: 0, level: info.level, effect: info.effect,
                                poison_list: Vec::new(),
                                last_hit_damage: 0,
                                undead: info.undead,
                                master_session: Some(msg.session_id),
                                rarity: 0,
                                pet_experience: 0,
                                max_pet_level: max_pet_level as u8,
                                recall_at_tick: if recall_at_tick > 0 { self.tick_count + recall_at_tick } else { 0 },
                                behavior: crate::actors::world::ai::make_behavior(&spawn.name),
                            });
                            // 补 DB 基础 AC/MAC（原实现遗漏）并叠加 PetLevel 加成（C# RefreshAll）
                            if let Some(m) = self.monsters.get_mut(&new_oid) {
                                m.fill_combat_stats(&info);
                                m.min_ac += pet_level * 2;
                                m.max_ac += pet_level * 2;
                                m.min_mac += pet_level * 2;
                                m.max_mac += pet_level * 2;
                                // C# RefreshAll：Skeleton/Shinsu/Angel 按 MaxPetLevel 加速
                                // （自定义 AI 宠物的移动/攻击 tick 在 behavior 内硬编码，此处作用于默认 AI 路径）
                                if matches!(msg.spell, SPELL_SUMMON_SKELETON | SPELL_SUMMON_SHINSU | SPELL_SUMMON_HOLY_DEVA) {
                                    let move_save = (max_pet_level as u64).saturating_mul(13) / 10; // ≈ MaxPetLevel*130ms
                                    let atk_save = (max_pet_level as u64).saturating_mul(7) / 10;   // ≈ MaxPetLevel*70ms
                                    m.ai_profile.move_interval = m.ai_profile.move_interval.saturating_sub(move_save).max(1);
                                    m.ai_profile.attack_cooldown = m.ai_profile.attack_cooldown.saturating_sub(atk_save).max(1);
                                }
                            }
                            // 记录召唤物等级（C# MonsterObject.PetLevel = magic.Level）
                            self.pet_levels.insert(new_oid, spell_level as i32);
                            debug!("Magic: {} casts summon '{}' as #{} at ({},{}) (slave of {})",
                                state.name, summon_name, new_oid, sx, sy, msg.session_id);
                        } else {
                            warn!("Summon '{}' found index {} but no MonsterInfo (DB missing mob)",
                                summon_name, idx);
                            send_system_message(&self.gate_ref, msg.session_id,
                                "召唤失败：怪物资料缺失");
                        }
                    }
                    None => {
                        warn!("Summon '{}' not in monster_name_index (DB may lack this mob)", summon_name);
                        send_system_message(&self.gate_ref, msg.session_id, "召唤失败：未知怪物");
                    }
                }
            }
            // Stonetrap：召唤“石头”宠物到目标点（C# HumanObject.cs:5739 ArcherSummonStone / 6724 CompleteMagic）
            SPELL_STONETRAP => {
                const STONE_NAME: &str = "StoneTrap";
                let (max_x, max_y) = self.maps.get(&state.map_index)
                    .map(|m| (m.width as i32, m.height as i32))
                    .unwrap_or((i32::MAX, i32::MAX));
                let sx = target_x.clamp(0, max_x - 1);
                let sy = target_y.clamp(0, max_y - 1);

                // 已存在存活石头 → 拒绝（C# Only one active Stone alive）
                let has_alive_stone = self.monsters.values().any(|m| {
                    m.master_session == Some(msg.session_id)
                        && m.name.eq_ignore_ascii_case(STONE_NAME)
                        && m.hp > 0
                });
                if has_alive_stone {
                    send_system_message(&self.gate_ref, msg.session_id, "已有一只存活的石阵，无法重复召唤");
                    return;
                }
                // 宠物数量超限 → 拒绝（C# Pets.Count >= magic.Level + 1）
                let pet_count = self.monsters.values()
                    .filter(|m| m.master_session == Some(msg.session_id))
                    .count();
                if pet_count >= spell_level as usize + 1 {
                    send_system_message(&self.gate_ref, msg.session_id, "召唤物数量已达上限");
                    return;
                }

                // 按名查 MonsterInfo（lowercase key，对齐 tick.rs boss_summons）
                let mon_index = self.monster_name_index.get(&STONE_NAME.to_lowercase()).copied();
                match mon_index {
                    Some(idx) => {
                        let info_opt = self.monster_infos.get(&idx).cloned();
                        if let Some(info) = info_opt {
                            let new_oid = self.alloc_object_id();
                            let hp = info.stats.get(&(mir2_shared::enums::Stat::HP as u8)).copied().unwrap_or(50);
                            let min_dmg = info.stats.get(&(mir2_shared::enums::Stat::MinDC as u8)).copied().unwrap_or(5);
                            let max_dmg = info.stats.get(&(mir2_shared::enums::Stat::MaxDC as u8)).copied().unwrap_or(10);
                            let spawn = MonsterSpawn {
                                name: info.name.clone(),
                                image: info.image as u16,
                                monster_index: idx,
                                x: sx,
                                y: sy,
                                direction: msg.direction,
                                hp,
                                min_dmg,
                                max_dmg,
                                xp: info.experience,
                                map_index: state.map_index,
                                count: 1,
                                spread: 0,
                            };
                            // C# StoneTrap.Name：宠物名 = 怪物名(主人名)
                            let display_name = format!("{}({})", spawn.name, state.name);
                            let packet = build_object_monster_packet(&spawn, new_oid, &display_name);
                            broadcast_to_map(&self.gate_ref, &self.players, state.map_index, &packet).await;
                            let ai_profile = MonsterAiProfile::from_info(&info);
                            // 石阵存活时长：C# DieTime = now + (level*5+10) 秒
                            let duration_ticks = (spell_level as u64 * 5 + 10) * 10;
                            self.monsters.insert(new_oid, MonsterState {
                                object_id: new_oid,
                                name: spawn.name.clone(),
                                image: spawn.image,
                                monster_index: idx,
                                x: sx, y: sy, direction: msg.direction,
                                hp, max_hp: hp, min_dmg, max_dmg, xp: spawn.xp,
                                spawn_x: sx, spawn_y: sy, map_index: state.map_index,
                                spawn_spread: 0,
                                next_attack_tick: 0, next_move_tick: 0, next_summon_tick: 0,
                                ai_profile, ai_state: MonsterAiState::Idle,
 sitting: false,
 hidden: false,
 sit_down_tick: 0,
                                target_session: Some(msg.session_id),
                                last_hitter_session: None, provoked: true,
                                is_elite: false, is_boss: false,
                                min_ac: 0, max_ac: 0, min_mac: 0, max_mac: 0,
                                agility: 0, accuracy: 0,
                                armour_rate: 1.0, damage_rate: 1.0,
                                magic_resist: 0, critical_rate: 0, critical_damage: 0,
                                luck: 0, reflect: 0, damage_reduction_percent: 0, level: info.level, effect: info.effect,
                                poison_list: Vec::new(),
                                last_hit_damage: 0,
                                undead: info.undead,
                                master_session: Some(msg.session_id),
                                rarity: 0,
                                pet_experience: 0,
                                max_pet_level: 0,
                                recall_at_tick: self.tick_count + duration_ticks,
                                behavior: crate::actors::world::ai::make_behavior(&spawn.name),
                            });
                            debug!("Magic: {} casts Stonetrap '{}' as #{} at ({},{}) ({}s)",
                                state.name, STONE_NAME, new_oid, sx, sy, spell_level as u64 * 5 + 10);
                        } else {
                            warn!("Stonetrap '{}' found index {} but no MonsterInfo (DB missing mob)",
                                STONE_NAME, idx);
                            send_system_message(&self.gate_ref, msg.session_id, "召唤失败：怪物资料缺失");
                        }
                    }
                    None => {
                        warn!("Stonetrap '{}' not in monster_name_index (DB may lack this mob)", STONE_NAME);
                        send_system_message(&self.gate_ref, msg.session_id, "召唤失败：未知怪物");
                    }
                }
            }
            // ===== 特殊/辅助类法术（任务：补齐剩余主动法术）=====
            // --- 战士系 ---
            // LionRoar：5×5 区域怪物施加 LR 麻痹（C# Map.cs:1398，非嘲讽）
            // 条件：IsAttackTarget && 施法者 Level+3 >= 怪物 Level；Duration = Lv+2 秒
            SPELL_LION_ROAR => {
                let hit_ids: Vec<u32> = self.monsters.iter()
                    .filter(|(_, m)| {
                        (m.x - target_x).abs() <= 2
                            && (m.y - target_y).abs() <= 2
                            && m.hp > 0
                            && m.map_index == state.map_index
                            && m.master_session.is_none()
                    })
                    .map(|(id, _)| *id)
                    .collect();
                let mut paralyzed = 0u32;
                for mid in hit_ids {
                    let mon_level = self.monsters.get(&mid)
                        .and_then(|m| self.monster_infos.get(&m.monster_index))
                        .map(|i| i.level).unwrap_or(0);
                    // C#：player.Level + 3 < target.Level 跳过
                    if state.level as i32 + 3 < mon_level { continue; }
                    if let Some(monster) = self.monsters.get_mut(&mid) {
                        crate::combat::poison::apply_poison(
                            &mut monster.poison_list,
                            crate::combat::poison::Poison::new(
                                mir2_shared::enums::PoisonType::LR_PARALYSIS,
                                (spell_level as u32 + 2).max(1),
                                0,
                                1000,
                            ),
                        );
                        monster.provoked = true;
                        paralyzed += 1;
                    }
                }
                debug!("Magic: {} casts LionRoar (LRParalysis {} monsters)", state.name, paralyzed);
            }
            // BattleCry：5×5 区域概率嘲讽怪物（C# Map.cs:2250，非麻痹）
            // 共享一次 Random(100)，threshold 按 Lv：0→90 / 1→70 / 2→50 / 3→30（10/30/50/70% 成功）
            // 跳过 CoolEye==100 怪物；命中设 target=施法者
            SPELL_BATTLE_CRY => {
                let threshold = match spell_level {
                    0 => 90,
                    1 => 70,
                    2 => 50,
                    3 => 30,
                    _ => 100,
                };
                let random_value = fastrand::i32(0..100);
                let hit_ids: Vec<u32> = self.monsters.iter()
                    .filter(|(_, m)| {
                        (m.x - target_x).abs() <= 2
                            && (m.y - target_y).abs() <= 2
                            && m.hp > 0
                            && m.map_index == state.map_index
                            && m.master_session.is_none()
                    })
                    .map(|(id, _)| *id)
                    .collect();
                let mut taunted = 0u32;
                for mid in hit_ids {
                    // C#：randomValue > threshold 跳过（共享同一次随机）
                    if random_value > threshold { break; }
                    // C#：CoolEye == 100 跳过（对真视怪物无效）
                    let cool_eye = self.monsters.get(&mid)
                        .and_then(|m| self.monster_infos.get(&m.monster_index))
                        .map(|i| i.cool_eye == 100)
                        .unwrap_or(false);
                    if cool_eye { continue; }
                    if let Some(monster) = self.monsters.get_mut(&mid) {
                        monster.provoked = true;
                        monster.target_session = Some(msg.session_id);
                        taunted += 1;
                    }
                }
                debug!("Magic: {} casts BattleCry (taunted {} monsters, roll={} threshold={})",
                       state.name, taunted, random_value, threshold);
            }
            // ProtectionField：防护领域（C# HumanObject.cs ProtectionField）——
            // 仅自身 AC 提升：MaxAC/MinAC += round(MaxAC*(0.2+0.03Lv))，时长 45+15Lv 秒
            SPELL_PROTECTION_FIELD => {
                let add_value = (state.max_ac as f32 * (0.2 + 0.03 * spell_level as f32)).round() as i32;
                let duration_ticks = (45 + spell_level as u32 * 15) * 10;
                let buff = crate::combat::buff::BuffInstance::new(
                    crate::combat::buff::BuffType::AcDefenseBoost { bonus: add_value.max(1) },
                    duration_ticks,
                    5,
                );
                let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff { buff }).await;
                debug!("Magic: {} casts ProtectionField (AC +{}, {}s)",
                       state.name, add_value.max(1), 45 + spell_level as i32 * 15);
            }
            // CounterAttack：反击（C# HumanObject.cs:8550）——施放进入 7 秒窗口，受击时反击并消耗
            SPELL_COUNTER_ATTACK => {
                if self.counter_attack.contains_key(&msg.session_id) {
                    debug!("Magic: {} casts CounterAttack but already active", state.name);
                    return;
                }
                self.counter_attack.insert(msg.session_id, (self.tick_count + 70, spell_level));
                debug!("Magic: {} arms CounterAttack (7s window)", state.name);
            }
            // --- 法师系 ---
            // TurnUndead：秒杀低级亡灵（对齐 C# WizardObject.TurnUndead，HumanObject.cs:4216）
            // 双段判定：先概率嘲讽，未嘲讽再按 threshold 秒杀
            SPELL_TURN_UNDEAD => {
                // 目标格子的亡灵怪物
                let hit_ids: Vec<u32> = self.monsters.iter()
                    .filter(|(_, m)| {
                        let dist = (m.x - target_x).abs() + (m.y - target_y).abs();
                        dist <= spell_range.max(1) && m.hp > 0 && m.map_index == state.map_index && m.undead
                    })
                    .map(|(id, _)| *id)
                    .collect();
                let mut killed = 0u32;
                for mid in hit_ids {
                    // 查 MonsterInfo.level 用于等级差判定
                    let mon_level = self.monsters.get(&mid)
                        .and_then(|m| self.monster_infos.get(&m.monster_index))
                        .map(|i| i.level).unwrap_or(0);
                    // #1515：C# 第一步——Random(2) + Level - 1 <= target.Level 则嘲讽（不击杀）
                    if fastrand::i32(0..2) + state.level as i32 - 1 <= mon_level {
                        if let Some(monster) = self.monsters.get_mut(&mid) {
                            monster.provoked = true;
                            monster.target_session = Some(msg.session_id);
                        }
                        continue;
                    }
                    // C# 第二步——dif = Level - target.Level + 15；threshold = ((Lv+1)<<3) + dif
                    let threshold = turn_undead_threshold(state.level, mon_level, spell_level);
                    if fastrand::i32(0..100) >= threshold {
                        // Random(100) >= threshold → 嘲讽目标（不击杀）
                        if let Some(monster) = self.monsters.get_mut(&mid) {
                            monster.provoked = true;
                            monster.target_session = Some(msg.session_id);
                        }
                        continue;
                    }
                    // 否则秒杀
                    if let Some(monster) = self.monsters.get_mut(&mid) {
                        monster.hp = 0;
                        monster.provoked = true;
                        monster.target_session = Some(msg.session_id);
                        killed += 1;
                    }
                }
                debug!("Magic: {} casts TurnUndead (killed {} undead)", state.name, killed);
            }
            // #318：TwinDrakeBlade —— 施放后 10 秒内下一次近战攻击双段伤害（C# HumanObject.cs:8530）
            SPELL_TWIN_DRAKE_BLADE => {
                self.double_hit_melee.insert(msg.session_id, (self.tick_count + 100, spell_level, 0));
                debug!("Magic: {} casts TwinDrakeBlade (next melee double-hit, 10s)", state.name);
            }
            // #318：DoubleSlash —— 同上双段近战（刺客）
            SPELL_DOUBLE_SLASH => {
                self.double_hit_melee.insert(msg.session_id, (self.tick_count + 100, spell_level, 1));
                debug!("Magic: {} casts DoubleSlash (next melee double-hit, 10s)", state.name);
            }
            // #318：SlashingBurst —— 前方第 1 格 DC 伤害（AC 防御）+ 向前冲刺 2 格
            // （C# HumanObject.cs:5159 + Map.cs：count=1 只结算 1 格，DefenceType.AC）
            SPELL_SLASHING_BURST => {
                let dir = msg.direction as usize % 8;
                // C# Envir.cs SlashingBurst：倍率 3.25+0.25Lv（DC）
                let raw = crate::combat::attack::get_attack_power(
                    state.min_attack + state.bonus_min_attack,
                    state.max_attack + state.bonus_max_attack,
                    state.luck,
                );
                let raw_damage = ((raw as f32) * (3.25 + 0.25 * spell_level as f32)).max(1.0) as i32;
                let mut new_x = state.x;
                let mut new_y = state.y;
                let mut slashed_damage = 0i32;
                for step in 0..2 {
                    let nx = new_x + MON_DIR_DX[dir];
                    let ny = new_y + MON_DIR_DY[dir];
                    let walkable = self.maps.get(&state.map_index)
                        .map(|m| m.is_walkable(nx, ny))
                        .unwrap_or(false);
                    if !walkable { break; }
                    let hit: Option<u32> = self.monsters.iter()
                        .find(|(_, m)| m.map_index == state.map_index && m.x == nx && m.y == ny && m.hp > 0)
                        .map(|(id, _)| *id);
                    if let Some(mid) = hit {
                        if let Some(m) = self.monsters.get_mut(&mid) {
                            // C#：只结算前方第 1 格（Map.cs SlashingBurst count=1），AC 防御
                            if step == 0 {
                                let attacker_stats = state.to_combat_stats();
                                let defender_stats = m.to_combat_stats();
                                let level_offset = crate::combat::attack::level_offset(state.level, m.level.max(0) as u16);
                                let r = combat_attack::resolve_attack(
                                    &attacker_stats, &defender_stats, raw_damage,
                                    mir2_shared::enums::DefenceType::Ac, level_offset,
                                );
                                if r.is_hit && r.damage > 0 {
                                    m.take_damage(r.damage);
                                    m.last_hitter_session = Some(msg.session_id);
                                    self.pending_gather.push(msg.session_id);
                                    m.provoked = true;
                                    // C# MonsterObject.Attacked：仅当无目标时锁定攻击者（魔法伤害同物理）
                                    if m.target_session.is_none() {
                                        m.target_session = Some(msg.session_id);
                                    }
                                    slashed_damage += r.damage;
                                }
                            }
                        }
                    }
                    new_x = nx;
                    new_y = ny;
                    let _ = step;
                }
                if new_x != state.x || new_y != state.y {
                    let _ = record.actor_ref.ask(crate::actors::player::SetPlayerPosition {
                        x: new_x, y: new_y, direction: msg.direction,
                        map_index: None, is_mounted: None,
                    }).await;
                    self.broadcast_position_change(msg.session_id, new_x, new_y, msg.direction).await;
                }
                debug!("Magic: {} casts SlashingBurst (dashed to {},{}, dealt {} dmg)",
                       state.name, new_x, new_y, slashed_damage);
            }
            // #328：Plague —— 3×3 区域随机毒 + MaxSC×2 MAC 伤害（C# Map.cs:1972）
            SPELL_PLAGUE => {
                // #1446：C# Plague 需普通护符并消耗 1（HumanObject.cs:4827）
                if !record.actor_ref.ask(crate::actors::player::ConsumeAmuletForSummon { amount: 1 }).await.unwrap_or(false) {
                    debug!("Magic: {} casts Plague but has no amulet", state.name);
                    return;
                }
                // #1453：C# Plague——毒型由装备毒护符决定（GetPoison(1,1)=绿 / (1,2)=红）并消耗 1；无毒护符只伤害
                let poison_shape = state.equipped_poison_shape();
                let mut ptype = match poison_shape {
                    1 => mir2_shared::enums::PoisonType::GREEN,
                    2 => mir2_shared::enums::PoisonType::RED,
                    _ => mir2_shared::enums::PoisonType::NONE,
                };
                if poison_shape != 0 {
                    if !record.actor_ref.ask(crate::actors::player::ConsumePoisonAmuletForPlague { shape: poison_shape as u16 }).await.unwrap_or(false) {
                        ptype = mir2_shared::enums::PoisonType::NONE;
                    }
                }
                let value = if let Some(info) = spell_db {
                    crate::combat::magic::calc_magic_damage(info, spell_level, magic_stat)
                } else { fastrand::i32(5..=12) }.max(1);
                let damage = (magic_stat * 2).max(1);
                let attacker_stats = state.to_combat_stats();
                let cells = plague_cells(target_x, target_y);
                let hit_ids: Vec<u32> = self.monsters.iter()
                    .filter(|(_, m)| m.hp > 0 && m.map_index == state.map_index && cells.contains(&(m.x, m.y)))
                    .map(|(id, _)| *id)
                    .collect();
                let mut spell_hits: Vec<(u32, i32, i32, u8, i32)> = Vec::new();
                for mid in hit_ids {
                    if let Some(monster) = self.monsters.get_mut(&mid) {
                        let temp_value = plague_temp_value(value, spell_level, ptype);
                        if ptype != mir2_shared::enums::PoisonType::NONE {
                            let dur = plague_duration(spell_level, value).max(1) as u32;
                            crate::combat::poison::apply_poison(
                                &mut monster.poison_list,
                                crate::combat::poison::Poison::new(ptype, dur, temp_value, 1000),
                            );
                        }
                        let defender_stats = monster.to_combat_stats();
                        // #1455：LevelOffset = Level > attacker.Level ? 0 : min(10, attacker.Level - Level)
                        let level_offset = crate::combat::attack::level_offset(state.level, monster.level.max(0) as u16);
                        let r = combat_attack::resolve_attack(
                            &attacker_stats, &defender_stats, damage,
                            mir2_shared::enums::DefenceType::Mac, level_offset,
                        );
                        if r.is_hit && r.damage > 0 {
                            monster.take_damage(r.damage);
                            monster.last_hitter_session = Some(msg.session_id);
                            self.pending_gather.push(msg.session_id);
                            monster.provoked = true;
                            // C# MonsterObject.Attacked：仅当无目标时锁定攻击者（魔法伤害同物理）
                            if monster.target_session.is_none() {
                                monster.target_session = Some(msg.session_id);
                            }
                            spell_hits.push((mid, monster.x, monster.y, monster.direction, r.damage));
                        }
                    }
                }
                self.broadcast_spell_hit(&spell_hits, object_id).await;
                debug!("Magic: {} casts Plague (3x3, {} hit, dmg={})", state.name, spell_hits.len(), damage);
            }
            // #328：Trap —— 目标怪物 60 秒麻痹（C# Map.cs:2048 ShockTime）
            SPELL_TRAP => {
                // C# Map.cs Trap：目标等级 >= 施法等级+2 时跳过
                let hit: Option<(u32, i32)> = self.monsters.iter()
                    .find(|(_, m)| m.map_index == state.map_index && m.x == target_x && m.y == target_y && m.hp > 0)
                    .map(|(id, m)| (*id, self.monster_infos.get(&m.monster_index).map(|i| i.level).unwrap_or(0)));
                if let Some((mid, mlevel)) = hit {
                    if mlevel >= state.level as i32 + 2 {
                        debug!("Magic: {} casts Trap -> monster {} level {} too high", state.name, mid, mlevel);
                        return;
                    }
                    if let Some(monster) = self.monsters.get_mut(&mid) {
                        crate::combat::poison::apply_poison(
                            &mut monster.poison_list,
                            crate::combat::poison::Poison::new(
                                mir2_shared::enums::PoisonType::PARALYSIS, 60, 0, 1000,
                            ),
                        );
                        monster.provoked = true;
                        monster.target_session = Some(msg.session_id);
                        debug!("Magic: {} casts Trap -> monster {} paralyzed 60s", state.name, mid);
                    }
                } else {
                    debug!("Magic: {} casts Trap (no target at {},{})", state.name, target_x, target_y);
                }
            }
            // #345：MoonMist —— 隐身 + 自身周围 5×5 AC 范围伤害（C# HumanObject.cs:4565 + Map.cs:1347）
            SPELL_MOON_MIST => {
                // C#：已有 MoonLight buff 时不重复施放
                if self.invisible_sessions.contains(&msg.session_id) {
                    debug!("Magic: {} casts MoonMist but already invisible, skipped", state.name);
                    return;
                }
                // C# 时长：(GetAttackPower(MinAC, MaxAC) + (Lv+1)*5) * 500ms
                let ac_power = crate::combat::attack::get_attack_power(
                    state.min_ac + state.bonus_min_ac,
                    state.max_ac + state.bonus_max_ac,
                    0,
                );
                let duration_ticks = ((ac_power + (spell_level as i32 + 1) * 5).max(1) as u32) * 5;
                let buff = crate::combat::buff::BuffInstance::new(
                    crate::combat::buff::BuffType::Invisibility,
                    duration_ticks,
                    5,
                );
                let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff { buff }).await;
                self.invisible_sessions.insert(msg.session_id);
            if let Ok(Some(st)) = record.actor_ref.ask(GetPlayerState).await {
                self.broadcast_object_hidden(st.object_id, true, st.map_index).await;
            }
                let raw_damage = (magic_stat + (power as i32) / 2).max(1);
                let attacker_stats = state.to_combat_stats();
                // C# Map.cs:1347：location ±2 = 5×5
                let cells = curse_cells_5x5(state.x, state.y);
                let hit_ids: Vec<u32> = self.monsters.iter()
                    .filter(|(_, m)| m.hp > 0 && m.map_index == state.map_index && cells.contains(&(m.x, m.y)))
                    .map(|(id, _)| *id)
                    .collect();
                let mut spell_hits: Vec<(u32, i32, i32, u8, i32)> = Vec::new();
                for mid in hit_ids {
                    if let Some(monster) = self.monsters.get_mut(&mid) {
                        let ds = monster.to_combat_stats();
                        // #1455：LevelOffset = Level > attacker.Level ? 0 : min(10, attacker.Level - Level)
                        let level_offset = crate::combat::attack::level_offset(state.level, monster.level.max(0) as u16);
                        let r = combat_attack::resolve_attack(
                            &attacker_stats, &ds, raw_damage,
                            mir2_shared::enums::DefenceType::Ac, level_offset,
                        );
                        if r.is_hit && r.damage > 0 {
                            monster.take_damage(r.damage);
                            monster.last_hitter_session = Some(msg.session_id);
                            self.pending_gather.push(msg.session_id);
                            monster.provoked = true;
                            // C# MonsterObject.Attacked：仅当无目标时锁定攻击者（魔法伤害同物理）
                            if monster.target_session.is_none() {
                                monster.target_session = Some(msg.session_id);
                            }
                            spell_hits.push((mid, monster.x, monster.y, monster.direction, r.damage));
                        }
                    }
                }
                self.broadcast_spell_hit(&spell_hits, object_id).await;
                debug!("Magic: {} casts MoonMist (invisible {}s + 5x5 AC dmg={} hits={})",
                       state.name, duration_ticks / 10, raw_damage, spell_hits.len());
            }
            // #395：ImmortalSkin —— AC 提升 + DC 交换（C# HumanObject.cs:6171 CompleteMagic）
            // MaxDC = round(MaxDC * (0.05+0.01Lv)) * -1；MaxAC = round(MaxAC * (0.10+0.07Lv))
            SPELL_IMMORTAL_SKIN => {
                let ac_bonus = (state.max_ac as f32 * (0.10 + 0.07 * spell_level as f32)) as i32;
                let dc_penalty = ((state.max_attack + state.bonus_max_attack) as f32
                    * (0.05 + 0.01 * spell_level as f32)).round() as i32;
                let duration_ticks = ((60 + spell_level as i32) as u32) * 10;
                let buff = crate::combat::buff::BuffInstance::new(
                    crate::combat::buff::BuffType::AcDefenseBoost { bonus: ac_bonus.max(1) },
                    duration_ticks,
                    5,
                );
                let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff { buff }).await;
                if dc_penalty > 0 {
                    let dc_buff = crate::combat::buff::BuffInstance::new(
                        crate::combat::buff::BuffType::AttackBoost { bonus: -dc_penalty },
                        duration_ticks,
                        5,
                    );
                    let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff { buff: dc_buff }).await;
                }
                debug!("Magic: {} casts ImmortalSkin (AC +{}, DC -{}, {}s)",
                       state.name, ac_bonus.max(1), dc_penalty, 60 + spell_level as i32);
            }
            // #395：Hallucination —— 概率成功，怪物 10-29s 失去目标不攻击（C# HumanObject.cs:6342）
            SPELL_HALLUCINATION => {
                if hallucination_success(spell_level, state.level) {
                    let hit: Option<u32> = self.monsters.iter()
                        .find(|(_, m)| m.map_index == state.map_index && m.x == target_x && m.y == target_y && m.hp > 0)
                        .map(|(id, _)| *id);
                    if let Some(mid) = hit {
                        let dur = hallucination_duration();
                        let until = self.tick_count + dur as u64 * 10;
                        self.hallucinated.insert(mid, until);
                        if let Some(monster) = self.monsters.get_mut(&mid) {
                            monster.target_session = None;
                            monster.ai_state = crate::actors::world::MonsterAiState::Idle;
                        }
                        debug!("Magic: {} casts Hallucination -> monster {} confused {}s", state.name, mid, dur);
                    } else {
                        debug!("Magic: {} casts Hallucination (no target at {},{})", state.name, target_x, target_y);
                    }
                } else {
                    debug!("Magic: {} casts Hallucination (failed)", state.name);
                }
            }
            // #409/#1499：OneWithNature —— 5×5 AoE MAC 伤害 + 特殊箭武装（C# Map.cs：持有 PoisonShot buff 绿毒 / VampireShot buff 吸血）
            SPELL_ONE_WITH_NATURE => {
                let raw_damage = if let Some(info) = spell_db {
                    crate::combat::magic::calc_magic_damage(info, spell_level, magic_stat)
                } else { fastrand::i32(8..=20) }.max(1);
                let attacker_stats = state.to_combat_stats();
                let cells = curse_cells_5x5(target_x, target_y);
                let hit_ids: Vec<u32> = self.monsters.iter()
                    .filter(|(_, m)| m.hp > 0 && m.map_index == state.map_index && cells.contains(&(m.x, m.y)))
                    .map(|(id, _)| *id)
                    .collect();
                // 特殊箭武装（#1483）：1=VampireShot 吸血 / 2=PoisonShot 绿毒
                let has_vamp = state.special_shot_armed == 1;
                let has_poison = state.special_shot_armed == 2;
                let mut spell_hits: Vec<(u32, i32, i32, u8, i32)> = Vec::new();
                let mut vamp_total = 0i32;
                for mid in hit_ids {
                    if let Some(monster) = self.monsters.get_mut(&mid) {
                        let ds = monster.to_combat_stats();
                        // #1455：LevelOffset = Level > attacker.Level ? 0 : min(10, attacker.Level - Level)
                        let level_offset = crate::combat::attack::level_offset(state.level, monster.level.max(0) as u16);
                        let r = combat_attack::resolve_attack(
                            &attacker_stats, &ds, raw_damage,
                            mir2_shared::enums::DefenceType::Mac, level_offset,
                        );
                        if r.is_hit && r.damage > 0 {
                            monster.take_damage(r.damage);
                            monster.last_hitter_session = Some(msg.session_id);
                            self.pending_gather.push(msg.session_id);
                            monster.provoked = true;
                            // C# MonsterObject.Attacked：仅当无目标时锁定攻击者（魔法伤害同物理）
                            if monster.target_session.is_none() {
                                monster.target_session = Some(msg.session_id);
                            }
                            spell_hits.push((mid, monster.x, monster.y, monster.direction, r.damage));
                            // C# Vampire Effect：VampAmount += value*(Lv+1)*0.25（命中时累加）
                            if has_vamp {
                                vamp_total += (raw_damage as f32 * (spell_level as f32 + 1.0) * 0.25) as i32;
                            }
                        }
                        // C#：持有 PoisonShot buff 时必中绿毒（Duration = value*2 + (Lv+1)*7；
                        // Value = value/15 + Lv + 1 + Random(PoisonAttack)）
                        if has_poison {
                            let dur = (raw_damage * 2 + (spell_level as i32 + 1) * 7).max(1) as u32;
                            let val = (raw_damage / 15 + spell_level as i32 + 1
                                + fastrand::i32(0..state.poison_attack.max(1))).max(1);
                            crate::combat::poison::apply_poison(&mut monster.poison_list,
                                crate::combat::poison::Poison::new(
                                    mir2_shared::enums::PoisonType::GREEN, dur, val, 2000,
                                ));
                        }
                    }
                }
                // 吸血统一入队（tick 统一结算，与弹道 VampireShot 一致）
                if vamp_total > 0 {
                    self.vamp_heals.push((msg.session_id, vamp_total));
                }
                // C#：施放后消耗特殊箭 buff（AddBuff 1s 过期 → 武装归零）
                if has_vamp || has_poison {
                    let _ = record.actor_ref.ask(crate::actors::player::SetSpecialShotArmed { armed: 0 }).await;
                }
                self.broadcast_spell_hit(&spell_hits, object_id).await;
                debug!("Magic: {} casts OneWithNature (5x5, {} hit, dmg={}, vamp={})",
                       state.name, spell_hits.len(), raw_damage, vamp_total);
            }
            // #409：MentalState —— 模式 0/1/2 循环（C# HumanObject.cs:8571）
            SPELL_MENTAL_STATE => {
                let cur = self.mental_state.entry(msg.session_id).or_insert(0);
                *cur = (*cur + 1) % 3;
                let label = match *cur {
                    1 => "特技射击",
                    2 => "组队模式",
                    _ => "攻击模式",
                };
                send_system_message(&self.gate_ref, msg.session_id, &format!("精神状态切换到：{}", label));
                debug!("Magic: {} casts MentalState -> {}", state.name, label);
            }
            // #427：UltimateEnhancer —— 友方目标 DC/MC/SC 提升（C# HumanObject.cs:4784）
            // 按目标职业加成：战士/刺客→DC，法师/弓手→MC，道士→SC
            SPELL_ULTIMATE_ENHANCER => {
                // #1447：C# UltimateEnhancer 需普通护符并消耗 1（HumanObject.cs:4784）
                if !record.actor_ref.ask(crate::actors::player::ConsumeAmuletForSummon { amount: 1 }).await.unwrap_or(false) {
                    debug!("Magic: {} casts UltimateEnhancer but has no amulet", state.name);
                    return;
                }
                let sc = state.effective_max_sc();
                let value = if sc >= 5 { (sc / 5).min(8) } else { 1 };
                // #1447：C# expiretime = GetAttackPower(MinSC,MaxSC)*4 + (Lv+1)*50
                let sc_power = crate::combat::attack::get_attack_power(
                    state.min_sc + state.bonus_min_sc,
                    state.max_sc + state.bonus_max_sc,
                    0,
                ).max(1);
                let duration_ticks = ultimate_enhancer_duration_ticks(sc_power, spell_level);
                // 目标选择：msg.target_id 指向自己或同组玩家 → 对其施放；否则自己
                let mut target_session = msg.session_id;
                let mut target_class = state.class;
                if msg.target_id != 0 {
                    let mut found_any = false;
                    for (sid, r) in &self.players {
                        if let Ok(Some(os)) = r.actor_ref.ask(GetPlayerState).await {
                            if os.object_id == msg.target_id {
                                found_any = true;
                                let friendly = *sid == msg.session_id
                                    || (os.group_id.is_some() && os.group_id == state.group_id);
                                if friendly {
                                    target_session = *sid;
                                    target_class = os.class;
                                }
                                break;
                            }
                        }
                    }
                    // 自己的召唤物目标：怪物无 buff 系统，按 DC 提升近似作用于自身
                    if !found_any || target_session == msg.session_id {
                        if self.monsters.get(&msg.target_id)
                            .map(|m| m.master_session == Some(msg.session_id))
                            .unwrap_or(false)
                        {
                            target_class = state.class; // DC（怪物默认）
                        }
                    }
                }
                let (buff, label) = match target_class {
                    mir2_shared::enums::MirClass::Wizard | mir2_shared::enums::MirClass::Archer =>
                        (crate::combat::buff::BuffType::McBoost { bonus: value }, "MC"),
                    mir2_shared::enums::MirClass::Taoist =>
                        (crate::combat::buff::BuffType::ScBoost { bonus: value }, "SC"),
                    _ => (crate::combat::buff::BuffType::AttackBoost { bonus: value }, "DC"),
                };
                let inst = crate::combat::buff::BuffInstance::new(buff, duration_ticks, 5);
                if target_session == msg.session_id {
                    let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff { buff: inst }).await;
                } else if let Some(r) = self.players.get(&target_session) {
                    let _ = r.actor_ref.ask(crate::actors::player::ApplyBuff { buff: inst }).await;
                }
                debug!("Magic: {} casts UltimateEnhancer on session {} ({} +{}, {}s)",
                       state.name, target_session, label, value, duration_ticks / 10);
            }
            // #448：FatalSword —— C# 中为被动技能（近战 10% 触发），无主动施放分支；施放不消耗
            SPELL_FATAL_SWORD => {
                debug!("Magic: {} casts FatalSword (passive-only in C#, no active effect)", state.name);
            }
            // #448：PetEnhancer —— 召唤宠物 DC/AC 提升（C# HumanObject.cs:6363 CompleteMagic）
            // dcInc = 2 + 宠物等级*2；acInc = 4 + 宠物等级；时长 ≈ SC 秒（GetPower 默认 0）
            SPELL_PET_ENHANCER => {
                let sc = state.effective_max_sc();
                let duration_s = sc.max(1) as u32;
                let until = self.tick_count + (duration_s as u64) * 10;
                let pet: Option<u32> = self.monsters.iter()
                    .find(|(_, m)| m.master_session == Some(msg.session_id)
                        && m.map_index == state.map_index
                        && (m.x - target_x).abs() <= 2 && (m.y - target_y).abs() <= 2)
                    .map(|(id, _)| *id);
                if let Some(pid) = pet {
                    let pet_lv = self.pet_levels.get(&pid).copied().unwrap_or(0);
                    let dc_inc = 2 + pet_lv * 2;
                    let ac_inc = 4 + pet_lv;
                    self.pet_enhanced.insert(pid, (until, dc_inc, ac_inc));
                    debug!("Magic: {} casts PetEnhancer -> pet {} (DC+{} AC+{}, {}s)",
                           state.name, pid, dc_inc, ac_inc, duration_s);
                } else {
                    debug!("Magic: {} casts PetEnhancer (no pet near {},{})", state.name, target_x, target_y);
                }
            }
            // #312：FlamingSword —— 施放后 10 秒内下一次近战攻击附加火焰加成（C# HumanObject.cs:8538）
            SPELL_FLAMING_SWORD => {
                self.flaming_sword.insert(msg.session_id, (self.tick_count + 100, spell_level));
                debug!("Magic: {} casts FlamingSword (next melee +{:.2}x, 10s)",
                       state.name, 1.4 + 0.4 * spell_level as f32);
            }
            // #312：EnergyShield —— 减伤 buff（C# HumanObject.cs:4751，chance=10-(Luck/3+Lv+1)，吸收百分比转 HP）
            SPELL_ENERGY_SHIELD => {
                let chance = (10 - (state.luck / 3 + spell_level as i32 + 1)).max(2);
                let percent = ((1.0 / chance as f32) * 100.0).round() as i32;
                let duration_ticks = ((30 + 50 * spell_level as i32) as u32) * 10;
                let _ = record.actor_ref.ask(crate::actors::player::ApplyDamageReduction {
                    percent,
                    duration_ticks,
                }).await;
                debug!("Magic: {} casts EnergyShield (damage -{}%, {}s)",
                       state.name, percent, 30 + 50 * spell_level as i32);
            }
            // Repulsion/EnergyRepulsor：推开周围怪物（C# 两者共用 Repulsion 方法）
            // 命中 1-2 格内怪物，将其沿反方向推 1-2 格（受 can_push 限制）
            SPELL_REPULSION | SPELL_ENERGY_REPULSOR | SPELL_FIRE_BURST => {
                let push_range = (1 + spell_level as i32 / 2).min(2); // Lv0=1, Lv2+=2
                // 收集 (怪物id, 推动方向) —— 方向 = 怪物相对施法者
                let mut pushes: Vec<(u32, usize)> = Vec::new();
                for (id, m) in self.monsters.iter() {
                    // #1636：仅同图怪物（C# CurrentMap）
                    if m.map_index != state.map_index { continue; }
                    if m.hp <= 0 || m.master_session.is_some() { continue; }
                    let dx = m.x - state.x;
                    let dy = m.y - state.y;
                    let dist = dx.abs() + dy.abs();
                    if dist == 0 || dist > 2 { continue; }
                    // 推动方向：取 8 方向中最接近 (dx,dy) 的
                    let push_dir = best_dir(dx, dy);
                    pushes.push((*id, push_dir));
                }
                let (max_x, max_y) = self.maps.get(&state.map_index)
                    .map(|m| (m.width as i32, m.height as i32))
                    .unwrap_or((i32::MAX, i32::MAX));
                // 预取每只候选怪物的当前位置 + can_push（避免后续 &self.monsters 与 &mut 冲突）
                let mut candidates: Vec<(u32, usize, i32, i32)> = Vec::new(); // (id, dir, x, y)
                for (mid, pdir) in pushes {
                    let can_push = self.monsters.get(&mid)
                        .and_then(|m| self.monster_infos.get(&m.monster_index))
                        .map(|i| i.can_push).unwrap_or(true);
                    if !can_push { continue; }
                    if let Some(m) = self.monsters.get(&mid) {
                        candidates.push((mid, pdir, m.x, m.y));
                    }
                }
                // 被占用格子集合（用于阻挡判定），随移动动态更新
                let mut occupied: std::collections::HashSet<(i32, i32)> = self.monsters.values()
                    .filter(|m| m.hp > 0).map(|m| (m.x, m.y)).collect();
                let mut moved_packets: Vec<(u32, i32, i32, u8)> = Vec::new();
                for (mid, pdir, start_x, start_y) in candidates {
                    let mut nx = start_x;
                    let mut ny = start_y;
                    for _ in 0..push_range {
                        let tx = nx + MON_DIR_DX[pdir];
                        let ty = ny + MON_DIR_DY[pdir];
                        if tx < 0 || ty < 0 || tx >= max_x || ty >= max_y { break; }
                        let walkable = self.maps.get(&state.map_index)
                            .map(|m| m.is_walkable(tx, ty)).unwrap_or(true);
                        if !walkable { break; }
                        // 不能推到其他怪物身上（动态占用表）
                        if occupied.contains(&(tx, ty)) { break; }
                        nx = tx; ny = ty;
                    }
                    if nx != start_x || ny != start_y {
                        // 更新占用表：释放旧格、占用新格
                        occupied.remove(&(start_x, start_y));
                        occupied.insert((nx, ny));
                        if let Some(monster) = self.monsters.get_mut(&mid) {
                            monster.x = nx;
                            monster.y = ny;
                            monster.direction = ((pdir + 4) % 8) as u8; // 朝向施法者
                            // C# MonsterObject.Pushed：仅转身朝向施法者，不改目标、不激怒
                            moved_packets.push((mid, nx, ny, monster.direction));
                        }
                    }
                }
                // 广播被推动怪物的移动
                for (mid, mx, my, mdir) in moved_packets {
                    let mut walk_body = Vec::new();
                    walk_body.extend_from_slice(&mid.to_le_bytes());
                    walk_body.extend_from_slice(&mx.to_le_bytes());
                    walk_body.extend_from_slice(&my.to_le_bytes());
                    walk_body.push(mdir);
                    let walk_packet = build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::ObjectWalk as i16, &walk_body);
                    let hit_map = self.monsters.get(&mid).map(|m| m.map_index).unwrap_or(0);
                    broadcast_to_map(&self.gate_ref, &self.players, hit_map, &walk_packet).await;
                }
                debug!("Magic: {} casts Repulsion", state.name);
            }
            // ElectricShock：驯服怪物（对齐 C# HumanObject.cs ElectricShock）
            SPELL_ELECTRIC_SHOCK => {
                let target_mid: Option<u32> = self.monsters.iter()
                    .find(|(_, m)| {
                        let dist = (m.x - target_x).abs() + (m.y - target_y).abs();
                        dist <= 1 && m.hp > 0 && m.map_index == state.map_index
                    })
                    .map(|(id, _)| *id);
                if let Some(mid) = target_mid {
                    // 已驯服宠物：眩晕（ShockTime = (Lv*5+10)s）并清除目标（C# target.Master == this）
                    if self.monsters.get(&mid).map(|m| m.master_session == Some(msg.session_id)).unwrap_or(false) {
                        if let Some(monster) = self.monsters.get_mut(&mid) {
                            crate::combat::poison::apply_poison(&mut monster.poison_list,
                                crate::combat::poison::Poison::new(
                                    mir2_shared::enums::PoisonType::STUN, spell_level as u32 * 5 + 10, 0, 1000,
                                ));
                            monster.target_session = None;
                            debug!("Magic: {} ElectricShock stunned own pet {}", state.name, mid);
                        }
                        // #1256：C# ElectricShock——能走到这里说明驯服判定已成功，必给经验
                        self.grant_electric_shock_exp(msg.session_id, msg.spell, now_ms, spell_cs).await;
                        electric_shock_exp_handled = true;
                        return;
                    }
                    let can_tame = self.monsters.get(&mid)
                        .and_then(|m| self.monster_infos.get(&m.monster_index))
                        .map(|i| i.can_tame).unwrap_or(false);
                    if can_tame {
                        // C# 成功率：Random(4-Lv) == 0（Lv0=25% → Lv3=100%）
                        let n = (4 - spell_level as i32).max(1);
                        if fastrand::i32(0..n) == 0 {
                            // #1410：捕获驯服后的怪物名（随后广播 ObjectName；避免借用冲突）
                            let tamed_name = {
                                if let Some(monster) = self.monsters.get_mut(&mid) {
                                    monster.master_session = Some(msg.session_id);
                                    monster.target_session = None;
                                    monster.provoked = false;
                                    monster.recall_at_tick = 0; // C# 驯服宠物不消失
                                    Some(monster.name.clone())
                                } else {
                                    None
                                }
                            };
                            if let Some(name) = tamed_name {
                                // C# HumanObject.cs:4101：驯服成功 Broadcast(S.ObjectName) 刷新名字显示
                                self.broadcast_object_name(mid, &name).await;
                                debug!("Magic: {} casts ElectricShock (tamed monster {})", state.name, mid);
                                send_system_message(&self.gate_ref, msg.session_id, "驯服成功！");
                            }
                            // #1256：C# 驯服成功必给经验
                            self.grant_electric_shock_exp(msg.session_id, msg.spell, now_ms, spell_cs).await;
                            electric_shock_exp_handled = true;
                        } else {
                            // #1256：C# 失败 50% 概率给经验（Random(2)==0）
                            if fastrand::i32(0..2) == 0 {
                                self.grant_electric_shock_exp(msg.session_id, msg.spell, now_ms, spell_cs).await;
                            }
                            electric_shock_exp_handled = true;
                            // 失败时激怒怪物
                            if let Some(monster) = self.monsters.get_mut(&mid) {
                                monster.provoked = true;
                                monster.target_session = Some(msg.session_id);
                            }
                            debug!("Magic: {} ElectricShock failed on monster {}", state.name, mid);
                        }
                    } else {
                        // #1256：C# 不可驯服不给经验
                        electric_shock_exp_handled = true;
                        debug!("Magic: {} ElectricShock: monster {} not tamable", state.name, mid);
                    }
                } else {
                    // #1256：C# 无目标（target == null）不给经验
                    electric_shock_exp_handled = true;
                }
            }
            // MagicBooster：MC 提升（C# HumanObject.cs:4345 + CompleteMagic 6228：MinMC/MaxMC += 6+Lv*6，60s）
            SPELL_MAGIC_BOOSTER => {
                let bonus = 6 + spell_level as i32 * 6;
                let buff = crate::combat::buff::BuffInstance::new(
                    crate::combat::buff::BuffType::McBoost { bonus },
                    600, // 60s
                    5,
                );
                let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff { buff }).await;
                debug!("Magic: {} casts MagicBooster (MC +{})", state.name, bonus);
            }
            // --- 道士系 ---
            // Revelation：显血（C# HumanObject.cs:6284）——单目标（点击玩家/怪物），
            // Random(4)<=Lv 成功，value 秒内显示目标 HP（value = GetAttackPower(MinSC,MaxSC)+GetPower(0)）
            SPELL_REVELATION => {
                if fastrand::i32(0..4) > spell_level as i32 {
                    debug!("Magic: {} casts Revelation (failed)", state.name);
                    return;
                }
                let value = crate::combat::attack::get_attack_power(
                    state.min_sc + state.bonus_min_sc,
                    state.max_sc + state.bonus_max_sc,
                    0,
                ).max(1);
                let until = self.tick_count + (value as u64) * 10;
                // 目标：点击的玩家优先，其次点击格怪物
                let mut target_oid: Option<u32> = None;
                for (_sid, r) in &self.players {
                    if let Ok(Some(s)) = r.actor_ref.ask(GetPlayerState).await {
                        if s.object_id == msg.target_id {
                            target_oid = Some(s.object_id);
                            break;
                        }
                    }
                }
                if target_oid.is_none() {
                    target_oid = self.monsters.iter()
                        .find(|(_, m)| m.map_index == state.map_index && (m.x - target_x).abs() <= 1 && (m.y - target_y).abs() <= 1 && m.hp > 0)
                        .map(|(id, _)| *id);
                }
                if let Some(oid) = target_oid {
                    self.revealed_hp.insert(oid, until);
                    // 广播一次 ObjectHealth（客户端显示血条）
                    let (hp, max_hp) = if let Some(m) = self.monsters.get(&oid) {
                        (m.hp, m.max_hp)
                    } else {
                        let mut pos = (0i32, 1i32);
                        for (_sid, r) in &self.players {
                            if let Ok(Some(s)) = r.actor_ref.ask(GetPlayerState).await {
                                if s.object_id == oid {
                                    pos = (s.hp, s.max_hp);
                                    break;
                                }
                            }
                        }
                        pos
                    };
                    let percent = ((hp.max(0) as f32 / max_hp.max(1) as f32) * 100.0) as u8;
                    let mut body = Vec::new();
                    body.extend_from_slice(&oid.to_le_bytes());
                    body.push(percent);
                    body.extend_from_slice(&3u16.to_le_bytes());
                    let pkt = build_packet_bytes(mir2_shared::enums::ServerPacketIds::ObjectHealth as i16, &body);
                    broadcast_to_map(&self.gate_ref, &self.players, self.monsters.get(&oid).map(|m| m.map_index).unwrap_or(0), &pkt).await;
                    debug!("Magic: {} casts Revelation -> oid {} ({}s)", state.name, oid, value);
                } else {
                    debug!("Magic: {} casts Revelation (no target at {},{})", state.name, target_x, target_y);
                }
            }
            // Reincarnation：复活死亡玩家（对齐 C# TaoistObject.Reincarnation）
            // 实现：找附近 3 格死亡玩家 → OfferReincarnation（30s 有效期）+ RequestReincarnation（客户端确认）
            SPELL_REINCARNATION => {
                let revive_range = 3;
                // 从 player_death_queue 找附近死亡玩家
                let mut target_dead: Option<u64> = None;
                for sid in self.player_death_queue.keys() {
                    if *sid == msg.session_id { continue; }
                    if let Some(other) = self.players.get(sid) {
                        if let Ok(Some(s)) = other.actor_ref.ask(GetPlayerState).await {
                            if s.is_dead && s.map_index == state.map_index {
                                let dist = (s.x - state.x).abs() + (s.y - state.y).abs();
                                if dist <= revive_range {
                                    target_dead = Some(*sid);
                                    break;
                                }
                            }
                        }
                    }
                }
                if let Some(dead_sid) = target_dead {
                    if let Some(other) = self.players.get(&dead_sid) {
                        // #222：对齐 C# offer/accept 链路——设置轮回状态并请求确认
                        let expire_tick = self.tick_count + 300; // 30s 有效期
                        let _ = other
                            .actor_ref
                            .ask(crate::actors::player::OfferReincarnation {
                                host_session: msg.session_id,
                                expire_tick,
                            })
                            .await;
                        // 发送 S.RequestReincarnation（空包）给死亡玩家
                        let req =
                            mir2_shared::packets::server::miscellaneous::RequestReincarnation {};
                        let mut body = Vec::new();
                        if req.write_body(&mut body).is_ok() {
                            let _ = self
                                .gate_ref
                                .tell(SendToClient {
                                    session_id: dead_sid,
                                    data: build_packet_bytes(
                                        mir2_shared::enums::ServerPacketIds::RequestReincarnation
                                            as i16,
                                        &body,
                                    ),
                                })
                                .await;
                        }
                        debug!(
                            "Magic: {} casts Reincarnation (offered player {})",
                            state.name, dead_sid
                        );
                        send_system_message(
                            &self.gate_ref,
                            msg.session_id,
                            "轮回术已施展，等待对方确认…",
                        );
                    }
                } else {
                    send_system_message(&self.gate_ref, msg.session_id, "附近没有可复活的目标");
                    debug!("Magic: {} casts Reincarnation but no target", state.name);
                }
            }
            // --- 刺客系 ---
            // PoisonSword：C# HumanObject.cs:5289 —— 前左起 5 格弧即时涂绿毒（需毒药道具，Rust 不实现门槛）
            SPELL_POISON_SWORD => {
                // C# power = magic.GetDamage(GetAttackPower(MinDC,MaxDC))；PoisonSword 无倍率/MPower 配置 → = DC
                let power = crate::combat::attack::get_attack_power(
                    state.min_attack + state.bonus_min_attack,
                    state.max_attack + state.bonus_max_attack,
                    state.luck,
                ).max(1);
                let front = msg.direction as usize % 8;
                let mut poisoned = 0;
                for k in 0..5usize {
                    let d = (front + 7 + k) % 8; // PreviousDir 起顺时针 5 个方向
                    let hx = state.x + MON_DIR_DX[d];
                    let hy = state.y + MON_DIR_DY[d];
                    let mid = self.monsters.iter()
                        .find(|(_, m)| m.map_index == state.map_index && m.x == hx && m.y == hy && m.hp > 0)
                        .map(|(id, _)| *id);
                    if let Some(mid) = mid {
                        if let Some(monster) = self.monsters.get_mut(&mid) {
                            // C#：Duration = 3 + power/10 + Lv*3；Value = power/10 + Lv + 1 + Random(PoisonAttack)
                            let duration = (3 + power / 10 + spell_level as i32 * 3).max(1) as u32;
                            let value = (power / 10 + spell_level as i32 + 1
                                + fastrand::i32(0..state.poison_attack.max(1))).max(1);
                            crate::combat::poison::apply_poison(
                                &mut monster.poison_list,
                                crate::combat::poison::Poison::new(
                                    mir2_shared::enums::PoisonType::GREEN, duration, value, 1000,
                                ),
                            );
                            monster.provoked = true;
                            monster.target_session = Some(msg.session_id);
                            poisoned += 1;
                        }
                    }
                }
                debug!("Magic: {} casts PoisonSword (arc 5, poisoned {})", state.name, poisoned);
            }
            // --- 默认：其他伤害类（接入战斗公式 MAC）---
            _ => {
                let raw_damage = if let Some(info) = spell_db {
                    crate::combat::magic::calc_magic_damage(info, spell_level, magic_stat)
                } else {
                    fastrand::i32(5..=15)
                }.max(1);
                let attacker_stats = state.to_combat_stats();
                let hit_monster_ids: Vec<u32> = self.monsters.iter()
                    .filter(|(_, m)| {
                        let dist = (m.x - target_x).abs() + (m.y - target_y).abs();
                        dist <= spell_range && m.hp > 0 && m.map_index == state.map_index
                    })
                    .map(|(id, _)| *id)
                    .collect();

                for monster_id in hit_monster_ids {
                    if let Some(monster) = self.monsters.get_mut(&monster_id) {
                        let defender_stats = monster.to_combat_stats();
                        // #1455：LevelOffset = Level > attacker.Level ? 0 : min(10, attacker.Level - Level)
                        let level_offset = crate::combat::attack::level_offset(state.level, monster.level.max(0) as u16);
                        let r = combat_attack::resolve_attack(
                            &attacker_stats, &defender_stats, raw_damage,
                            mir2_shared::enums::DefenceType::Mac, level_offset,
                        );
                        if r.is_hit && r.damage > 0 {
                            monster.take_damage(r.damage);
                            monster.last_hitter_session = Some(msg.session_id);
                            self.pending_gather.push(msg.session_id);
                            monster.provoked = true;
                            // C# MonsterObject.Attacked：仅当无目标时锁定攻击者（魔法伤害同物理）
                            if monster.target_session.is_none() {
                                monster.target_session = Some(msg.session_id);
                            }
                            for p in &r.applied_poisons {
                                crate::combat::poison::apply_poison(&mut monster.poison_list, *p);
                            }
                        }
                        debug!("Magic: {} spell={} lv={} -> monster {} for {} dmg (crit={})",
                            state.name, msg.spell, spell_level, monster_id, r.damage, r.is_critical);
                    }
                }
            }
        }

        // Spell XP gain and cast_time update
        if should_grant_cast_exp(
            msg.spell,
            basic_spells.contains(&msg.spell),
            electric_shock_exp_handled,
        ) {
            // #1230：C# LevelMagic exp = Random.Next(3)+1；DB 信息用于等级门控/阈值/升级延迟
            let info = self.magic_infos.get(&(spell_cs as u32)).cloned();
            let _ = record.actor_ref.ask(crate::actors::player::GainSpellExp {
                spell: msg.spell,
                amount: (1 + fastrand::i32(0..3)) as u16,
                cast_time: now_ms,
                info,
            }).await;
        }
    }
}

/// 取最接近位移向量 (dx, dy) 的 8 方向索引（对齐 MON_DIR_DX/MON_DIR_DY）
/// 用于 Repulsion 等推开/弹射效果的推动方向计算
fn best_dir(dx: i32, dy: i32) -> usize {
    let mut best = 4usize; // 默认朝下（索引 4）
    let mut best_score = i64::MIN;
    for dir in 0..8usize {
        let sx = MON_DIR_DX[dir] as i64;
        let sy = MON_DIR_DY[dir] as i64;
        // 点积越大表示方向越一致
        let score = sx * dx as i64 + sy * dy as i64;
        if score > best_score {
            best_score = score;
            best = dir;
        }
    }
    best
}

#[cfg(test)]
mod spell_geometry_tests {
    use super::*;

    #[test]
    fn hellfire_lv0_single_line() {
        // 面朝下（dir=4），从 (10,10) 出发，4 格直线
        let cells = hellfire_cells(10, 10, 4, 0);
        assert_eq!(cells.len(), 4);
        assert_eq!(cells[0], (10, 11));
        assert_eq!(cells[3], (10, 14));
    }

    #[test]
    fn hellfire_lv3_three_lines() {
        // dir=4（下），Lv3 → 下 + 右下 + 左下，共 12 格
        let cells = hellfire_cells(10, 10, 4, 3);
        assert_eq!(cells.len(), 12);
        // 前 4 格为直线（下）
        assert_eq!(cells[0], (10, 11));
        assert_eq!(cells[4], (11, 11)); // 右下
        assert_eq!(cells[8], (9, 11)); // 左下
    }

    #[test]
    fn icethrust_target_and_splash() {
        let cells = icethrust_cells(10, 10, 2); // 右
        assert_eq!(cells.len(), 9);
        assert_eq!(cells[0], (11, 10)); // 主目标
        // 溅射含 (11,9) (11,11) (10,10) (12,10)
        assert!(cells.contains(&(11, 9)));
        assert!(cells.contains(&(11, 11)));
        assert!(cells.contains(&(10, 10)));
        assert!(cells.contains(&(12, 10)));
    }

    #[test]
    fn curse_area_7x7() {
        let cells = curse_cells(50, 60);
        assert_eq!(cells.len(), 49);
        assert!(cells.contains(&(47, 57)));
        assert!(cells.contains(&(53, 63)));
    }

    #[test]
    fn plague_area_3x3() {
        let cells = plague_cells(50, 60);
        assert_eq!(cells.len(), 9);
        assert!(cells.contains(&(49, 59)));
        assert!(cells.contains(&(51, 61)));
        assert!(cells.contains(&(50, 60)));
    }

    #[test]
    #[test]
    fn plague_values() {
        use mir2_shared::enums::PoisonType;
        // Red：value/15 + Lv + 1
        assert_eq!(plague_temp_value(30, 3, PoisonType::RED), 6);
        // 其他：value + (Lv+1)*2
        assert_eq!(plague_temp_value(30, 3, PoisonType::GREEN), 38);
        // 持续：2*(Lv+1)+value/10
        assert_eq!(plague_duration(3, 30), 11);
    }

    #[test]
    fn mp_eater_restore_value() {
        assert_eq!(mp_eater_restore(3, 0), 15);
        assert_eq!(mp_eater_restore(3, 4), 20);
    }

    #[test]
    fn hemorrhage_values() {
        assert_eq!(hemorrhage_duration(3, 6), 7);
        assert_eq!(hemorrhage_duration(0, 0), 0);
        assert_eq!(hemorrhage_value(50), 51);
    }

    #[test]
    fn special_shot_buff_time_value() {
        assert_eq!(special_shot_buff_time(0), 5);
        assert_eq!(special_shot_buff_time(3), 20);
    }

    #[test]
    fn hallucination_duration_range() {
        for _ in 0..100 {
            let d = hallucination_duration();
            assert!((10..=29).contains(&d), "duration out of range: {}", d);
        }
    }

    #[test]
    fn one_with_nature_area_5x5() {
        let cells = curse_cells_5x5(50, 60);
        assert_eq!(cells.len(), 25);
        assert!(cells.contains(&(48, 58)));
        assert!(cells.contains(&(52, 62)));
    }

    #[test]
    fn hallucination_success_high_level() {
        // 高等级：roll 范围很大，失败阈值 10 → 几乎必成功（10000 次抽样至少成功一次）
        let mut ok = false;
        for _ in 0..10000 {
            if hallucination_success(3, 30) {
                ok = true;
                break;
            }
        }
        assert!(ok);
    }
    #[test]
    fn ultimate_enhancer_duration_matches_csharp() {
        // #1447：C# expiretime = GetAttackPower(SC)*4 + (Lv+1)*50 秒（×10 ticks）
        assert_eq!(ultimate_enhancer_duration_ticks(10, 1), (10 * 4 + (1 + 1) * 50) * 10);
        assert_eq!(ultimate_enhancer_duration_ticks(10, 0), (10 * 4 + 50) * 10);
        // sc_power<=0 钳 1（get_attack_power max(1)）
        assert_eq!(ultimate_enhancer_duration_ticks(0, 1), (1 * 4 + (1 + 1) * 50) * 10);
    }

}

#[cfg(test)]
mod tests {
    use super::{
        archer_state_penalty, attack_disabled_by_poison, cast_disabled_by_poison, cast_out_of_range,
        find_attack_skill, range_attack_out_of_range,
        logout_blocked, player_attack_speed_ms, range_attack_min_reduction, range_flight_ticks,
        ranged_chance_to_hit, should_grant_cast_exp, turn_undead_threshold, ATTACK_SKILL_SPELLS,
        LOGOUT_DELAY_MS,
        DAMAGE_DURA_ARMOR_SLOTS, SPELL_CROSS_HALFMOON, SPELL_FIREBALL, SPELL_HALFMOON,
        SPELL_METEOR_SHOWER, SPELL_SLAYING,
    };
    use crate::actors::inventory::EquipmentSlot;
    use crate::actors::player::PlayerMagic;

    #[test]
    fn test_find_attack_skill_converts_shared_to_cs() {
        // #1256：magics 存 C# 编号，入参 SharedRust(+3)；Slaying C#=2 / SharedRust=5
        let mut magics = vec![PlayerMagic::new(2)]; // Slaying C#=2 / SharedRust=5
        assert!(find_attack_skill(&magics, SPELL_SLAYING).is_some());
        assert!(find_attack_skill(&magics, 4).is_none()); // Fencing 不是 Slaying
        let mut magics2 = vec![PlayerMagic::new(4)]; // HalfMoon C#=4 / SharedRust=7
        assert!(find_attack_skill(&magics2, SPELL_HALFMOON).is_some());
        assert!(find_attack_skill(&magics2, SPELL_CROSS_HALFMOON).is_none());
        // CrossHalfMoon C#=10 / SharedRust=13
        let mut magics3 = vec![PlayerMagic::new(10)];
        assert!(find_attack_skill(&magics3, SPELL_CROSS_HALFMOON).is_some());
        // 未学 → None
        let empty: Vec<PlayerMagic> = Vec::new();
        assert!(find_attack_skill(&empty, SPELL_SLAYING).is_none());
    }

    #[test]
    fn test_attack_skill_spells_list() {
        // #1256：C# CompleteAttack 会 LevelMagic 的近战技能（MPEater/Hemorrhage 被动单独触发）
        assert!(ATTACK_SKILL_SPELLS.contains(&SPELL_SLAYING));
        assert!(ATTACK_SKILL_SPELLS.contains(&SPELL_HALFMOON));
        assert!(ATTACK_SKILL_SPELLS.contains(&SPELL_CROSS_HALFMOON));
        assert!(!ATTACK_SKILL_SPELLS.contains(&super::SPELL_MPEATER));
        assert!(!ATTACK_SKILL_SPELLS.contains(&super::SPELL_HEMORRHAGE));
    }

    #[test]
    fn test_range_flight_ticks_matches_csharp_delay() {
        // #1560：C# HumanObject.cs:2827 delay = MaxDistance*50 + 550 ms；tick=100ms 向上取整
        assert_eq!(range_flight_ticks(0), 6);  // 550ms → ceil(5.5)=6
        assert_eq!(range_flight_ticks(1), 6);  // 600ms → 6
        assert_eq!(range_flight_ticks(3), 7);  // 700ms → 7
        assert_eq!(range_flight_ticks(9), 10); // 1000ms → 10
        assert_eq!(range_flight_ticks(-5), 6); // 负距离按 0 处理
    }

    #[test]
    fn test_cast_out_of_range_matches_csharp_inrange() {
        // #1620：C# HumanObject.Magic InRange（Chebyshev）
        // 超范围 → true
        assert!(cast_out_of_range(0, 0, 3, 0, 2));
        // 边界内：Chebyshev max(|dx|,|dy|) <= range 不超
        assert!(!cast_out_of_range(0, 0, 2, 2, 2));
        // 边界内
        assert!(!cast_out_of_range(0, 0, 2, 0, 2));
        assert!(!cast_out_of_range(0, 0, 1, 1, 2));
        // 自身施法（目标=自身位置）
        assert!(!cast_out_of_range(5, 5, 5, 5, 2));
        // range 0（自增益）不校验
        assert!(!cast_out_of_range(0, 0, 50, 50, 0));
        // 目标格 0（无目标 fallback）不校验
        assert!(!cast_out_of_range(0, 0, 0, 0, 2));
    }

    #[test]
    fn test_range_attack_out_of_range_matches_csharp_inrange() {
        // #1622：C# HumanObject.RangeAttack InRange(MaxAttackRange=9)（Chebyshev）
        // 边界内（max<=9）→ false
        assert!(!range_attack_out_of_range(0, 0, 9, 0));
        assert!(!range_attack_out_of_range(0, 0, 6, 6));
        assert!(!range_attack_out_of_range(0, 0, 7, 7));
        // 超范围 → true
        assert!(range_attack_out_of_range(0, 0, 10, 0));
        assert!(range_attack_out_of_range(0, 0, 0, 10));
        assert!(range_attack_out_of_range(0, 0, 7, 10));
        // 自身
        assert!(!range_attack_out_of_range(5, 5, 5, 5));
    }

    #[test]
    fn test_logout_blocked_matches_csharp_logtime() {
        // #1578：C# MirConnection.LogOut——Envir.Time < Player.LogTime（攻击后 10s）→ LogOutFailed
        assert_eq!(LOGOUT_DELAY_MS, 10_000, "C# Globals.LogDelay=10s");
        // 有阻止且未到期 → blocked
        assert!(logout_blocked(1000, Some(11000)));
        // 刚好到期 → 允许
        assert!(!logout_blocked(11000, Some(11000)));
        // 过期 → 允许
        assert!(!logout_blocked(12000, Some(11000)));
        // 无阻止记录 → 允许
        assert!(!logout_blocked(1000, None));
    }

    #[test]
    fn test_cast_disabled_by_poison() {
        use crate::combat::poison::Poison;
        use mir2_shared::enums::PoisonType;
        // #1287：C# CanCast——Stun/Dazed/Paralysis/Frozen 禁施法
        assert!(!cast_disabled_by_poison(&[]));
        assert!(cast_disabled_by_poison(&[Poison::new(PoisonType::STUN, 5, 0, 1000)]));
        assert!(cast_disabled_by_poison(&[Poison::new(PoisonType::DAZED, 5, 0, 1000)]));
        assert!(cast_disabled_by_poison(&[Poison::new(PoisonType::PARALYSIS, 5, 0, 1000)]));
        assert!(cast_disabled_by_poison(&[Poison::new(PoisonType::FROZEN, 5, 0, 1000)]));
        // C# CanCast 不查 LRParalysis（CanAttack 才查）；绿/红毒不禁施法
        assert!(!cast_disabled_by_poison(&[Poison::new(PoisonType::LR_PARALYSIS, 5, 0, 1000)]));
        assert!(!cast_disabled_by_poison(&[Poison::new(PoisonType::RED, 5, 3, 1000)]));
        assert!(!cast_disabled_by_poison(&[Poison::new(PoisonType::GREEN, 5, 3, 1000)]));
    }

    #[test]
    fn test_should_grant_cast_exp() {
        // #1312：C# CompleteMagic——延迟弹道法术一律不在施法时给经验（移到命中结算）
        // FireBall / MeteorShower：施法时不加
        assert!(!should_grant_cast_exp(SPELL_FIREBALL, false, false));
        assert!(!should_grant_cast_exp(SPELL_METEOR_SHOWER, false, false));
        // 基础攻击：不加
        assert!(!should_grant_cast_exp(SPELL_FIREBALL, true, false));
        // ElectricShock 已处理：不加（避免重复）
        assert!(!should_grant_cast_exp(SPELL_FIREBALL, false, true));
        // 非弹道法术（HalfMoon 近战技能）施法时加
        assert!(should_grant_cast_exp(SPELL_HALFMOON, false, false));
    }

    #[test]
    fn test_player_attack_speed_ms_formula() {
        // #1269：C# AttackSpeed = 1400 - (Stat*60 + min(370, Level*14))，下限 550ms
        // 0 攻速、1 级：1400 - (0 + 14) = 1386
        assert_eq!(player_attack_speed_ms(0, 1), 1386);
        // 高等级封顶 min(370, L*14)：30 级 420→370
        assert_eq!(player_attack_speed_ms(0, 30), 1400 - 370);
        // 攻速 10、30 级：1400 - (600+370) = 430 → 下限 550
        assert_eq!(player_attack_speed_ms(10, 30), 550);
        // 攻速 5、20 级：1400 - (300+280) = 820
        assert_eq!(player_attack_speed_ms(5, 20), 820);
        // 极端高攻速：仍为 550 下限
        assert_eq!(player_attack_speed_ms(100, 1), 550);
    }

    #[test]
    fn test_turn_undead_threshold() {
        // Lv0 施法者 vs 同级怪：8 + 15 = 23
        assert_eq!(turn_undead_threshold(30, 30, 0), 23);
        // 高等级玩家更容易秒杀：Lv3 32 + (50-30+15)=35 → 67
        assert_eq!(turn_undead_threshold(50, 30, 3), 67);
        // 远低于怪物等级 → clamp 0
        assert_eq!(turn_undead_threshold(10, 100, 0), 0);
        // 远超怪物等级 → clamp 100
        assert_eq!(turn_undead_threshold(90, 1, 3), 100);
    }

    #[test]
    fn test_archer_helpers() {
        // #1519：GetRangeAttackPower min 缩小——距离 0 全缩，距离 9 不变，中间取 floor
        assert_eq!(range_attack_min_reduction(10, 0), 0);
        assert_eq!(range_attack_min_reduction(10, 9), 10);
        assert_eq!(range_attack_min_reduction(10, 4), 5); // floor(10*5/9)=5
        // ApplyArcherState：MentalState 0/1/2 → 100 / 55+5*Lv / 80
        assert_eq!(archer_state_penalty(0, 0), 100);
        assert_eq!(archer_state_penalty(1, 2), 65);
        assert_eq!(archer_state_penalty(2, 0), 80);
        // chanceToHit：近距恒命中（>=100），远距按 100 - 11*dist，Focus ×2，<0 clamp 0
        assert_eq!(ranged_chance_to_hit(0, false), 100);
        assert_eq!(ranged_chance_to_hit(9, false), 1);
        assert_eq!(ranged_chance_to_hit(9, true), 2);
        assert_eq!(ranged_chance_to_hit(10, false), 0);
    }

    #[test]
    fn test_attack_disabled_by_poison() {
        use crate::combat::poison::Poison;
        use mir2_shared::enums::PoisonType;
        // 无中毒：可攻击
        assert!(!attack_disabled_by_poison(&[]));
        // 麻痹/冰冻/眩晕：禁攻
        assert!(attack_disabled_by_poison(&[Poison::new(PoisonType::PARALYSIS, 5, 0, 1000)]));
        assert!(attack_disabled_by_poison(&[Poison::new(PoisonType::LR_PARALYSIS, 5, 0, 1000)]));
        assert!(attack_disabled_by_poison(&[Poison::new(PoisonType::FROZEN, 5, 0, 1000)]));
        assert!(attack_disabled_by_poison(&[Poison::new(PoisonType::DAZED, 5, 0, 1000)]));
        // 红/绿毒不禁止攻击
        assert!(!attack_disabled_by_poison(&[Poison::new(PoisonType::RED, 5, 3, 1000)]));
        assert!(!attack_disabled_by_poison(&[Poison::new(PoisonType::GREEN, 5, 3, 1000)]));
    }

    #[test]
    fn test_damage_dura_armor_slots_excludes_weapon() {
        // #895：C# DamageDura 只扣非武器装备槽（本地 12 槽模型：排除 Weapon/Mount/Pendant）
        assert!(!DAMAGE_DURA_ARMOR_SLOTS.contains(&EquipmentSlot::Weapon));
        assert!(!DAMAGE_DURA_ARMOR_SLOTS.contains(&EquipmentSlot::Mount));
        assert!(!DAMAGE_DURA_ARMOR_SLOTS.contains(&EquipmentSlot::Pendant));
        assert!(DAMAGE_DURA_ARMOR_SLOTS.contains(&EquipmentSlot::Armour));
        assert!(DAMAGE_DURA_ARMOR_SLOTS.contains(&EquipmentSlot::Helmet));
        assert!(DAMAGE_DURA_ARMOR_SLOTS.contains(&EquipmentSlot::RingL));
        assert!(DAMAGE_DURA_ARMOR_SLOTS.contains(&EquipmentSlot::RingR));
        assert!(DAMAGE_DURA_ARMOR_SLOTS.contains(&EquipmentSlot::BraceletL));
        assert!(DAMAGE_DURA_ARMOR_SLOTS.contains(&EquipmentSlot::BraceletR));
        assert!(DAMAGE_DURA_ARMOR_SLOTS.contains(&EquipmentSlot::Shoes));
        assert!(DAMAGE_DURA_ARMOR_SLOTS.contains(&EquipmentSlot::Necklace));
    }
}



use super::*;

/// 怪物仇恨保留距离（C# Globals.DataRange = 16；超距/跨图/死亡 → 丢失目标）
const DATA_RANGE: i32 = 16;
/// 怪物巡逻间隔（C# MonsterObject.RoamDelay = 1000ms = 10 ticks）
const ROAM_DELAY_TICKS: u64 = 10;
/// 怪物索敌间隔（C# MonsterObject.SearchDelay = 3000ms = 30 ticks）
const SEARCH_DELAY_TICKS: u64 = 30;
/// 寻路缓存目标哨兵（monster_path_targets 首字段）：追击=玩家 session；回出生点=0；
/// 宠物跟随=u64::MAX；宠物协战=u64::MAX-1；怪物互伤=u64::MAX-2（#1697）
const PATH_TARGET_PET_FOLLOW: u64 = u64::MAX;
const PATH_TARGET_PET_ATTACK: u64 = u64::MAX - 1;
const PATH_TARGET_MONSTER_ATTACK: u64 = u64::MAX - 2;

/// #1699/#1765：构造 ObjectRangeAttack 包体（C# S.ObjectRangeAttack）：
/// object_id/x/y/direction/target_id/target_x/target_y/Type/spell/spell_level（28B，对齐 C#）
fn build_object_range_attack_body(
    attacker_oid: u32,
    x: i32,
    y: i32,
    dir: u8,
    target_id: u32,
    target_x: i32,
    target_y: i32,
    attack_type: u8,
    spell: u8,
) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&attacker_oid.to_le_bytes());
    body.extend_from_slice(&(x as u32).to_le_bytes());
    body.extend_from_slice(&(y as u32).to_le_bytes());
    body.push(dir);
    body.extend_from_slice(&target_id.to_le_bytes());
    body.extend_from_slice(&(target_x as u32).to_le_bytes());
    body.extend_from_slice(&(target_y as u32).to_le_bytes());
    body.push(attack_type);
    body.push(spell);
    body.push(0u8); // spell_level（C# Level 字节）
    body
}

/// #1703：普通 Ranged/Mage 怪物远程伤害延迟结算项（C# DelayedAction RangeDamage）
#[derive(Debug, Clone, Copy)]
pub(crate) struct RangedPendingHit {
    /// 到期 tick（100ms/tick）
    pub fire_tick: u64,
    pub attacker_oid: u32,
    pub target_session: u64,
    pub damage: i32,
    pub map_index: u16,
    /// 攻击时目标位置（弹道落地时用于安全区/伤害广播定位）
    pub px: i32,
    pub py: i32,
    /// #1730：Mage 怪（法术）→ MACAgility；Ranged/近战 → ACAgility（C# 法术吃魔防）
    pub is_magic: bool,
    pub target_in_safe: bool,
}

/// #1706：Boss Range 远程伤害延迟结算项（C# DelayedAction RangeDamage）
#[derive(Debug, Clone, Copy)]
pub(crate) struct BossRangedPendingHit {
    /// 到期 tick（100ms/tick）
    pub fire_tick: u64,
    pub attacker_oid: u32,
    pub target_session: u64,
    pub damage: i32,
    pub map_index: u16,
}
/// #1703/#1721：怪物命中玩家结算（C# Attacked / DelayedAction RangeDamage 落地）：
/// 完整结算（命中/护甲/反伤/减伤）+ ObjectStruck/DamageIndicator 广播 + 下坐骑 + 装备耐久 + 死亡掉落/经验惩罚。
/// 近战即时调用与远程延迟结算共用；返回反伤量（调用方施加给怪物）。
async fn apply_monster_hit_player(
    players: &HashMap<u64, PlayerRecord>,
    gate_ref: &ActorRef<GateActor>,
    death_exp_penalty_percent: u32,
    attacker_oid: u32,
    attacker_name: &str,
    attacker_stats: &crate::combat::attack::CombatStats,
    attacker_level: i32,
    target_session: u64,
    damage: i32,
    px: i32,
    py: i32,
    map_index: u16,
    target_in_safe: bool,
    defence_type: mir2_shared::enums::DefenceType,
    death_drops: &mut Vec<(u64, i32, i32, u16)>,
    dismount_sessions: &mut Vec<u64>,
    broken_armor: &mut Vec<(u64, EquipmentSlot)>,
) -> i32 {
    if !target_in_safe {
        if let Some(record) = players.get(&target_session) {
            // #1708：C# CompleteRangeAttack 校验——目标存活且同图才结算（远程飞行期间目标可能死亡/跨图）
            let Ok(Some(defender)) = record.actor_ref.ask(GetPlayerState).await else {
                return 0;
            };
            if defender.is_dead || defender.map_index != map_index {
                return 0;
            }
            // #1721：完整 C# Attacked 结算（命中/护甲/反伤/减伤，复用 resolve_attack）
            let (actual, reflected, is_miss, is_critical) = resolve_monster_vs_player(
                attacker_stats, attacker_level, &defender, damage, defence_type,
            );
            if is_miss {
                // 未命中/护甲全挡：同图其他玩家广播 Miss 飘字（C# GetArmour/Attacked BroadcastDamageIndicator(Miss)）
                broadcast_miss_feedback(
                    players, gate_ref, map_index, target_session, defender.object_id,
                ).await;
                return reflected;
            }
            let died = record.actor_ref.ask(TakeDamage {
                attacker_id: attacker_oid,
                attacker_session: target_session,
                damage: actual,
            }).await.unwrap_or(false);

            // #1598：C# HumanObject.Attacked（:7215/:7307）——向同图其他玩家
            // 广播 ObjectStruck + DamageIndicator（C# CurrentMap.Broadcast 排除受害者；受害者收 S.Struck）
            broadcast_hit_feedback(
                players, gate_ref, map_index, target_session,
                defender.object_id, defender.x, defender.y, defender.direction,
                attacker_oid, actual, if is_critical { 5 } else { 0 },
            ).await;

            // 被攻击时自动下坐骑
            if !died {
                dismount_sessions.push(target_session);
            }

            // 装备耐久损耗（C# HumanObject.DamageDura：受击时所有非武器槽位 -1；
            // #1230 致死也扣——C# DamageDura 在 ChangeHP 前调用）
            {
                let armor_slots = [
                    EquipmentSlot::Armour,
                    EquipmentSlot::Helmet,
                    EquipmentSlot::BraceletL,
                    EquipmentSlot::BraceletR,
                    EquipmentSlot::RingL,
                    EquipmentSlot::RingR,
                    EquipmentSlot::Shoes,
                    EquipmentSlot::Necklace,
                ];
                for slot in armor_slots {
                    let broke = record.actor_ref.ask(crate::actors::player::DamageEquipment {
                        slot,
                        amount: 1,
                    }).await.unwrap_or(false);
                    if broke {
                        debug!("Player session={} {:?} broke from monster damage!", target_session, slot);
                        // 延迟到怪物循环结束后广播（避免借用冲突）
                        broken_armor.push((target_session, slot));
                    }
                }
            }

            if died {
                let died_packet = WorldActor::build_object_died_packet(
                    defender.object_id, defender.x, defender.y, defender.direction, 0u8);
                // #1710：死亡动画只发同图玩家（C# CurrentMap.Broadcast；原实现漏过滤跨图）
                for (sid, _) in players {
                    if let Ok(Some(st)) = players.get(sid).expect("player exists").actor_ref.ask(GetPlayerState).await {
                        if st.map_index == map_index {
                            let _ = gate_ref.tell(SendToClient {
                                session_id: *sid,
                                data: died_packet.clone(),
                            }).await;
                        }
                    }
                }
                death_drops.push((target_session, defender.x, defender.y, defender.map_index));

                // 死亡经验惩罚（配置百分比；默认 0=关闭，对齐 C# 无通用死亡惩罚）
                if death_exp_penalty_percent > 0 {
                    let pct = (death_exp_penalty_percent.min(100)) as i64;
                    let penalty = ((defender.max_experience as i64 * pct) / 100).max(1) as i32;
                    let deducted = record.actor_ref.ask(crate::actors::player::DeductExperience {
                        amount: penalty,
                    }).await.unwrap_or(0);
                    if deducted > 0 {
                        send_system_message(
                            &gate_ref, target_session,
                            &format!("你损失了 {} 经验值", deducted)
                        );
                    }
                }
            }
            reflected
        } else {
            0
        }
    } else {
        debug!("Monster '{}' attack on {} blocked: target in safe zone", attacker_name, target_session);
        0
    }
}

/// #1721：怪物→玩家完整结算（C# HumanObject.Attacked 的 GetArmour/护甲/反伤/减伤，复用 resolve_attack）
/// 返回 (实际伤害, 反伤量, 是否未命中或护甲全挡)
fn resolve_monster_vs_player(
    attacker_stats: &crate::combat::attack::CombatStats,
    attacker_level: i32,
    defender: &crate::actors::player::PlayerState,
    raw_damage: i32,
    defence_type: mir2_shared::enums::DefenceType,
) -> (i32, i32, bool, bool) {
    let defender_stats = defender.to_combat_stats();
    let level_offset = crate::combat::attack::level_offset(attacker_level as u16, defender.level);
    let result = crate::combat::attack::resolve_attack(
        attacker_stats, &defender_stats, raw_damage, defence_type, level_offset,
    );
    let is_miss = !result.is_hit || (result.damage == 0 && result.reflected == 0);
    (result.damage, result.reflected, is_miss, result.is_critical)
}

/// #1768：怪物/宠物互伤结算（C# MonsterObject.Attacked(MonsterObject)：GetArmour + ArmourRate/DamageRate + armour>=damage→Miss）
/// 返回 (实际伤害, 是否 Miss)。不含 HumanObject 专属的暴击/反伤/吸血/EnergyShield 分支。
fn resolve_monster_vs_monster(
    attacker: &crate::combat::attack::CombatStats,
    defender: &crate::combat::attack::CombatStats,
    raw_damage: i32,
    defence_type: mir2_shared::enums::DefenceType,
) -> (i32, bool) {
    let (armour, hit) = crate::combat::attack::get_armour(defender, defence_type, attacker.accuracy);
    if !hit {
        return (0, true);
    }
    let armour = ((armour as f32 * defender.armour_rate) as i64)
        .clamp(i32::MIN as i64, i32::MAX as i64) as i32;
    let damage = ((raw_damage as f32 * defender.damage_rate) as i64)
        .clamp(i32::MIN as i64, i32::MAX as i64) as i32;
    if armour >= damage {
        return (0, true); // C# 护甲完全抵消 → BroadcastDamageIndicator(Miss)
    }
    ((damage - armour).max(0), false)
}

/// C# MonsterObject.ProcessPoison（MonsterObject.cs:1544-1546）：SLOW 毒时 MoveSpeed/AttackSpeed +100ms
/// （上限 3500ms = 35 tick）。tick=100ms，故 +1 tick。
#[inline]
fn slow_adjusted_ticks(base_ticks: u64, slowed: bool) -> u64 {
    if slowed { (base_ticks + 1).min(35) } else { base_ticks }
}

/// #1777/#1824：C# MonsterObject.CanMove/CanAttack（MonsterObject.cs:634-664）——控制毒禁移动/攻击：
/// 移动禁 PARALYSIS/LR_PARALYSIS/FROZEN/STUN；攻击另禁 DAZED。
/// C# 豁免：Light == 10/5 的光源怪对 STUN 免疫（被眩晕仍可移动/攻击）。
/// 返回 (禁移动, 禁攻击)。
fn monster_control_blocked(poison_list: &[crate::combat::poison::Poison], light: i32) -> (bool, bool) {
    use mir2_shared::enums::PoisonType;
    let stun_exempt = light == 10 || light == 5;
    let move_blocked = poison_list.iter().any(|p| {
        p.p_type.intersects(PoisonType::PARALYSIS)
            || p.p_type.intersects(PoisonType::LR_PARALYSIS)
            || p.p_type.intersects(PoisonType::FROZEN)
            || (p.p_type.intersects(PoisonType::STUN) && !stun_exempt)
    });
    let attack_blocked = move_blocked
        || poison_list.iter().any(|p| p.p_type.intersects(PoisonType::DAZED));
    (move_blocked, attack_blocked)
}

/// #1797：C# MonsterObject.CurrentPoison = 全部活动毒 PType 的 OR
fn combined_poison_flags(poison_list: &[crate::combat::poison::Poison]) -> mir2_shared::enums::PoisonType {
    use mir2_shared::enums::PoisonType;
    poison_list.iter().fold(PoisonType::NONE, |acc, p| acc | p.p_type)
}

/// #1721：向同图其他玩家广播 DamageIndicator Miss（C# GetArmour/Attacked BroadcastDamageIndicator(Miss)）
async fn broadcast_miss_feedback(
    players: &HashMap<u64, PlayerRecord>,
    gate_ref: &ActorRef<GateActor>,
    map_index: u16,
    exclude_session: u64,
    victim_oid: u32,
) {
    let mut dmg_body = Vec::new();
    dmg_body.extend_from_slice(&0i32.to_le_bytes()); // damage = 0
    dmg_body.push(4u8); // damage_type = Miss
    dmg_body.extend_from_slice(&victim_oid.to_le_bytes());
    let dmg_packet = build_packet_bytes(
        mir2_shared::enums::ServerPacketIds::DamageIndicator as i16, &dmg_body);
    for (sid, _) in players {
        if *sid == exclude_session {
            continue;
        }
        if let Ok(Some(st)) = players.get(sid).expect("player exists").actor_ref.ask(GetPlayerState).await {
            if st.map_index == map_index {
                let _ = gate_ref.tell(SendToClient { session_id: *sid, data: dmg_packet.clone() }).await;
            }
        }
    }
}

/// #1712：向同图其他玩家广播 ObjectStruck + DamageIndicator（C# CurrentMap.Broadcast 排除受害者；受害者收 S.Struck）
async fn broadcast_hit_feedback(
    players: &HashMap<u64, PlayerRecord>,
    gate_ref: &ActorRef<GateActor>,
    map_index: u16,
    exclude_session: u64,
    victim_oid: u32,
    victim_x: i32,
    victim_y: i32,
    victim_dir: u8,
    attacker_oid: u32,
    damage: i32,
    damage_type: u8,
) {
    let mut struck_body = Vec::new();
    struck_body.extend_from_slice(&victim_oid.to_le_bytes());
    struck_body.extend_from_slice(&attacker_oid.to_le_bytes());
    struck_body.extend_from_slice(&(victim_x as u32).to_le_bytes());
    struck_body.extend_from_slice(&(victim_y as u32).to_le_bytes());
    struck_body.push(victim_dir);
    let struck_packet = build_packet_bytes(
        mir2_shared::enums::ServerPacketIds::ObjectStruck as i16, &struck_body);
    let mut dmg_body = Vec::new();
    dmg_body.extend_from_slice(&damage.to_le_bytes());
    dmg_body.push(damage_type); // damage_type: 0=Hit 5=Critical
    dmg_body.extend_from_slice(&victim_oid.to_le_bytes());
    let dmg_packet = build_packet_bytes(
        mir2_shared::enums::ServerPacketIds::DamageIndicator as i16, &dmg_body);
    // 同图其他玩家（C# CurrentMap.Broadcast 排除受害者）
    for (sid, _) in players {
        if *sid == exclude_session {
            continue;
        }
        if let Ok(Some(st)) = players.get(sid).expect("player exists").actor_ref.ask(GetPlayerState).await {
            if st.map_index == map_index {
                let _ = gate_ref.tell(SendToClient { session_id: *sid, data: struck_packet.clone() }).await;
                let _ = gate_ref.tell(SendToClient { session_id: *sid, data: dmg_packet.clone() }).await;
            }
        }
    }
}

/// #1434：收集 master 的所有后代 slave oid（含多级；C# MonsterObject.SlaveList 死亡级联；不含 master 自身）
fn collect_slave_cascade(master: u32, slave_master: &std::collections::HashMap<u32, u32>) -> Vec<u32> {
    let mut out = Vec::new();
    let mut queue = vec![master];
    while let Some(cur) = queue.pop() {
        let children: Vec<u32> = slave_master.iter()
            .filter(|(_, m)| **m == cur)
            .map(|(s, _)| *s)
            .collect();
        for c in children {
            out.push(c);
            queue.push(c);
        }
    }
    out
}

/// 游戏主循环 Tick
pub struct Tick;

/// 延迟动作到期处理消息（独立于 Tick 消息，避免巨型 async 状态机内联进 Tick handler 导致栈溢出）
pub struct ProcessDelayedActions;

impl Message<ProcessDelayedActions> for WorldActor {
    type Reply = ();

    async fn handle(
        &mut self,
        _msg: ProcessDelayedActions,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.process_delayed_actions().await;
    }
}

/// C# DateTime.ToBinary Kind 位掩码（去掉 Kind 标志后比较 Ticks）
const DOTNET_BINARY_TICKS_MASK: i64 = 0x3FFF_FFFF_FFFF_FFFF;

/// 当前时间对应的 .NET Ticks（本地墙钟，C# Envir.Now.Ticks 语义）
pub(crate) fn dotnet_now_ticks() -> i64 {
    let now = chrono::Local::now().naive_local();
    let as_utc = now.and_utc();
    as_utc.timestamp() * 10_000_000
        + 621_355_968_000_000_000
        + as_utc.timestamp_subsec_nanos() as i64 / 100
}

/// #916：物品/租赁是否已到期（C# ExpireInfo.ExpiryDate <= Envir.Now）
pub(crate) fn item_expired(expiry_date_binary: i64, now_ticks: i64) -> bool {
    (expiry_date_binary & DOTNET_BINARY_TICKS_MASK) <= now_ticks
}

/// #914：C# HumanObject.ReduceExp——等级差经验衰减
/// （玩家等级 >= 怪物等级+10 时：amount - Round(Max(amount/15,1)*(Level-(targetLevel+10)))，最低 1；
///  C# Settings.ExpMobLevelDifference 默认开启）
fn reduce_exp(amount: i32, level: u16, target_level: i32) -> i32 {
    let target = target_level.max(0) as u16;
    if level < target + 10 {
        return amount;
    }
    let diff = (level - (target + 10)) as f64;
    let penalty = ((amount as f64 / 15.0).max(1.0) * diff).round() as i32;
    (amount - penalty).max(1)
}

/// C# PlayerObject.WinExp partyExpRate（nearCount 1..11，上限 11 人）
const PARTY_EXP_RATE: [f64; 11] = [1.0, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8, 1.9, 2.0, 2.1, 2.2];

/// #954：攻城箭塔单次伤害（C# ConquestArcher 攻击力简化常量）
const ARCHER_DAMAGE: i32 = 30;
/// #1513：箭塔攻击范围（C# ConquestArcher FindTarget 用 Info.ViewRange，取 10）
const ARCHER_RANGE: i32 = 10;

/// C# WinExp 组队单成员分配：expPoint * rate * memberLevel / sumLevel
fn party_exp_share(exp_after_reduce: i32, rate: f64, member_level: u16, sum_level: i32) -> i32 {
    if sum_level <= 0 {
        return exp_after_reduce;
    }
    (exp_after_reduce as f64 * rate * (member_level as f64) / (sum_level as f64)) as i32
}

/// C# MonsterObject.PetExp：玩家获得经验时，同图 InRange(16) 存活宠物获得同等经验
/// （Skeleton/Shinsu/Angel ×3）；PetExperience >= (PetLevel+1)*20000 升级一次，
/// RefreshAll：HP+20/级、AC/MAC+2/级、DC+1/级 + BroadcastHealthChange
async fn grant_pet_exp(world: &mut WorldActor, master: u64, amount: i64, map_index: u16, mx: i32, my: i32) {
    use crate::actors::world::ai::max_distance;
    let mut pet_targets: Vec<(u32, i64)> = Vec::new();
    for (oid, m) in &world.monsters {
        if m.master_session == Some(master) && m.hp > 0
            && m.map_index == map_index && max_distance(m.x, m.y, mx, my) <= 16
        {
            let gain = pet_exp_gain(&m.name, amount);
            pet_targets.push((*oid, gain));
        }
    }
    for (oid, gain) in pet_targets {
        let Some(m) = world.monsters.get_mut(&oid) else { continue };
        m.pet_experience = m.pet_experience.saturating_add(gain as u64);
        let pet_level = world.pet_levels.get(&oid).copied().unwrap_or(0);
        let max_lv = m.max_pet_level as i32;
        if pet_level >= max_lv {
            continue;
        }
        let threshold = ((pet_level + 1) as u64) * 20_000;
        if m.pet_experience < threshold {
            continue;
        }
        // C# PetExp：只升一级（每次 GainExp 至多一次）
        m.pet_experience -= threshold;
        let new_level = pet_level + 1;
        world.pet_levels.insert(oid, new_level);
        // C# RefreshAll：HP += PetLevel*20；AC/MAC += PetLevel*2；DC += PetLevel
        m.max_hp += 20;
        m.hp = m.max_hp;
        m.min_ac += 2;
        m.max_ac += 2;
        m.min_mac += 2;
        m.max_mac += 2;
        m.min_dmg += 1;
        m.max_dmg += 1;
        debug!("Pet '{}' #{} leveled to {} (exp={})", m.name, oid, new_level, m.pet_experience);
        // C# BroadcastHealthChange：ObjectHealth 百分比血条广播
        let percent = ((m.hp.max(0) as f32 / m.max_hp as f32) * 100.0) as u8;
        let mut health_body = Vec::new();
        health_body.extend_from_slice(&oid.to_le_bytes());
        health_body.push(percent);
        health_body.extend_from_slice(&3u16.to_le_bytes());
        let health_packet = build_packet_bytes(
            mir2_shared::enums::ServerPacketIds::ObjectHealth as i16, &health_body);
        // #1686：宠物血条广播只发同图玩家（C# CurrentMap）
        crate::actors::world::broadcast_to_map(&world.gate_ref, &world.players, map_index, &health_packet).await;
    }
}

/// C# MonsterObject.PetExp：Skeleton/Shinsu/Angel 宠物经验 ×3
fn pet_exp_gain(name: &str, amount: i64) -> i64 {
    if matches!(name, "Skeleton" | "Shinsu" | "Angel") {
        amount * 3
    } else {
        amount
    }
}

/// #1759：Boss Range 伤害防御类型（C# 各怪 DelayedAction RangeDamage / CompleteRangeAttack 的 DefenceType）
/// 精确名匹配（不区分大小写）；未收录默认 ACAgility（C# 默认敏捷物防，保持 #1721 既有行为，安全回退）。
fn boss_range_defence_type(name: &str) -> mir2_shared::enums::DefenceType {
    use mir2_shared::enums::DefenceType;
    let n = name.to_ascii_lowercase();
    if matches!(n.as_str(),
        // MACAgility：远程法术吃魔防+敏捷（C# DefenceType.MACAgility）
        "antcommander" | "avengingspirit" | "avengingwarrior" | "blacktortoise" | "bluesoul"
        | "bonelord" | "cannibaltentacles" | "catshaman" | "creeperplant" | "darkcaptain"
        | "darkdevil" | "darkdevourer" | "finialturtle" | "flamescythe" | "flamingmutant"
        | "frozenmagician" | "furbolgcommander" | "generalmeowmeow" | "hellkeeper"
        | "hoodedsummonerscrolls" | "kinghydrax" | "omacannibal" | "omamage" | "peacockspider"
        | "powerbead" | "restlessjar" | "rhinopriest" | "seedingsgeneral" | "septaoist"
        | "sepwizard" | "treeguardian" | "tucsongeneral" | "waterdragon" | "whitefoxman"
        | "witchdoctor"
    ) {
        DefenceType::MacAgility
    } else if matches!(n.as_str(),
        // MAC：远程魔法吃魔防（C# DefenceType.MAC）
        "burningzombie" | "darkomaking" | "elementguard" | "frosttiger" | "frozenzombie"
        | "holydeva" | "hoodedsummoner" | "icecrystalsoldier" | "iceguard" | "icephantom"
        | "jar2" | "kingguard" | "kirin" | "leftguard" | "manectricclaw" | "mudzombie"
        | "omaking" | "omawitchdoctor" | "redfoxman" | "redthunderzuma" | "rightguard"
        | "snowyeti" | "spittingtoad" | "swampwarrior" | "turtleking" | "yimoogi"
    ) {
        DefenceType::Mac
    } else if matches!(n.as_str(),
        // AC：远程物理吃物防（C# DefenceType.AC）
        "armadillo" | "hedgekektal" | "hornedcommander" | "hornedmage" | "hornedsorceror"
    ) {
        DefenceType::Ac
    } else {
        DefenceType::AcAgility
    }
}

/// #1761/#1763：怪物近战/AOE/直线伤害防御类型（C# 各怪 DelayedType.Damage / LineAttack / CompleteRangeAttack 的 DefenceType）
/// 精确名匹配（不区分大小写）；未收录默认 ACAgility（C# 默认敏捷物防，保持 #1721 既有行为，安全回退）。
/// 混合型（DemonWolf/SnowWolfKing/DarkOmaKing/OmaKing/TucsonGeneral/SandSnail/ScalyBeast 等近战与远程/直线不同）不映射，避免改错。
fn monster_melee_defence_type(name: &str) -> mir2_shared::enums::DefenceType {
    use mir2_shared::enums::DefenceType;
    let n = name.to_ascii_lowercase();
    if matches!(n.as_str(),
        // MACAgility：近战/直线吃魔防+敏捷（C# DefenceType.MACAgility）
        "burningzombie" | "cavemaggot" | "crystalspider" | "darkdevil" | "flamingwooma"
        | "flamespear" | "flyingstatue" | "frozenzombie" | "hoodedsummonerscrolls"
        | "hornedmage" | "icepillar" | "incarnatedzt" | "jar2" | "leftguard" | "plaguecrab"
        | "redthunderzuma" | "restlessjar" | "rightguard" | "seedingsgeneral" | "sepwizard"
        | "shamanzombie" | "stoningstatue" | "toxicghoul" | "treeguardian" | "treequeen"
        | "trollking" | "vampirespider" | "venomspider" | "yimoogi" | "zumataurus"
    ) {
        DefenceType::MacAgility
    } else if matches!(n.as_str(),
        // MAC：近战/AOE吃魔防（C# DefenceType.MAC）
        "charmedsnake" | "dragonstatue" | "earthgolem" | "elementguard" | "evilcentipede"
        | "evilmir" | "greatfoxspirit" | "iceguard" | "sephigharcher" | "thunderelement"
        | "tucsonegg"
    ) {
        DefenceType::Mac
    } else if matches!(n.as_str(),
        // AC：近战/AOE吃物防（C# DefenceType.AC）
        "cannibaltentacles" | "darkdevourer" | "darkwraith" | "elephantman" | "frozenaxeman"
        | "generalmeowmeow" | "guard" | "hellbomb" | "kirin" | "stonegolem" | "turtlegrass"
    ) {
        DefenceType::Ac
    } else {
        DefenceType::AcAgility
    }
}

/// C# Functions.InRange：切比雪夫距离（Abs(dx)<=range && Abs(dy)<=range）
fn in_range(ax: i32, ay: i32, bx: i32, by: i32, range: i32) -> bool {
    (ax - bx).abs().max((ay - by).abs()) <= range
}

/// #898：安全区回血量（C# SpellObject.Value=25，不超过 max_hp）
fn safe_zone_heal_hp(hp: i32, max_hp: i32) -> i32 {
    (hp + 25).min(max_hp)
}

/// #898：安全区回血 tick（C# Settings.SafeZoneHealing），独立于 Tick 消息避免栈溢出
pub struct ProcessSafeZoneHealing;

impl Message<ProcessSafeZoneHealing> for WorldActor {
    type Reply = ();

    async fn handle(
        &mut self,
        _msg: ProcessSafeZoneHealing,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.tick_safe_zone_healing().await;
    }
}

/// 元素系统 tick（专注恢复/过期广播 + 攒元素队列），独立于 Tick 消息避免栈溢出
pub struct ProcessElementalTick;

impl Message<ProcessElementalTick> for WorldActor {
    type Reply = ();

    async fn handle(
        &mut self,
        _msg: ProcessElementalTick,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.tick_elements().await;
        // 导师伤害加成刷新（C# ProcessBuffs Mentor buff + Attacked 判定）
        crate::actors::world::partners::tick_partner_bonuses(self).await;
        // 新手行会经验 buff 刷新（C# ProcessBuffs BuffType.Newbie）
        self.tick_newbie_bonus().await;
    }
}

/// 死亡回调处理消息（独立于 Tick 消息避免栈溢出）
pub struct ProcessDeathCallbacks;

impl Message<ProcessDeathCallbacks> for WorldActor {
    type Reply = ();

    async fn handle(
        &mut self,
        _msg: ProcessDeathCallbacks,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let pending = std::mem::take(&mut self.pending_death_callbacks);
        for (mut monster, player_positions) in pending {
            self.apply_death_callbacks(&mut monster, &player_positions).await;
        }
    }
}


/// 自动复活处理消息（独立于 Tick 消息，避免巨型 async 状态机内联进 Tick handler 导致 tokio 栈溢出，#881 回归）
pub struct ProcessRevives {
    pub sessions: Vec<u64>,
}

impl Message<ProcessRevives> for WorldActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: ProcessRevives,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.process_revives(msg.sessions).await;
    }
}

/// #986：C# FindTarget 子集——宠物在 Both/AttackOnly 下若无协战目标，
/// 找最近正在攻击主人（target_session == master）的怪物（C# IsAttackTarget：ob.Target == attacker.Master）。
fn pet_find_hostile_target(
    monster: &MonsterState,
    monster_snapshot: &[(u32, i32, i32, i32, i32, u16, i32, String, u16, u8)],
    monster_hostility: &std::collections::HashMap<u32, (Option<u64>, Option<u64>)>,
) -> Option<(u32, i32, i32)> {
    let master = monster.master_session?;
    monster_snapshot.iter()
        .filter(|s| s.0 != monster.object_id && s.5 == monster.map_index && s.3 > 0)
        .filter(|s| monster_hostility.get(&s.0).map_or(false, |(ms, ts)| {
            *ms != monster.master_session && *ts == Some(master)
        }))
        .min_by_key(|s| ((s.1 - monster.x).abs() + (s.2 - monster.y).abs(), s.0))
        .map(|s| (s.0, s.1, s.2))
}

impl WorldActor {

    /// 处理自动复活（C# Revive）：Revive + NoReconnect 传送 + ObjectRevived 广播。
    /// 独立消息处理：避免在 Tick handler 巨型状态机内联多个 ask/广播循环导致 tokio 栈溢出（#881）。
    pub(crate) async fn process_revives(&mut self, sessions: Vec<u64>) {
        for session_id in sessions {
            self.player_death_queue.remove(&session_id);
            let Some(record) = self.players.get(&session_id).cloned() else { continue };
            // 死亡地图 NoReconnect：由独立消息 ApplyNoReconnect 处理传送
            //（避免 handler 内同步加载大图导致 tokio 栈溢出，#881 回归）
            let (needs_noreconnect, object_id, revive_map) = match record.actor_ref.ask(GetPlayerState).await {
                Ok(Some(state)) => {
                    let nn = self.map_infos.get(&(state.map_index as i32))
                        .map(|mi| mi.no_reconnect && !mi.no_reconnect_map.is_empty())
                        .unwrap_or(false);
                    (nn, state.object_id, state.map_index)
                }
                _ => (false, 0, 0),
            };
            let _ = record.actor_ref.ask(crate::actors::player::Revive).await;
            if needs_noreconnect {
                if let Some(world_ref) = self.self_ref.clone() {
                    let _ = world_ref.tell(crate::actors::world::ApplyNoReconnect {
                        session_id,
                    }).try_send();
                }
            }
            // C# Revive：广播 ObjectRevived（其他玩家看到复活动画）
            if object_id != 0 {
                let mut obj_body = Vec::new();
                obj_body.extend_from_slice(&object_id.to_le_bytes());
                obj_body.push(1u8); // effect
                let revived_packet = build_packet_bytes(
                    mir2_shared::enums::ServerPacketIds::ObjectRevived as i16, &obj_body);
                // #1686：复生广播只发同图玩家（C# CurrentMap）
                broadcast_to_map(&self.gate_ref, &self.players, revive_map, &revived_packet).await;
            }
        }
    }
    /// C# Die()：主人死亡 → 宠物（master_session 匹配的召唤怪）消失
    pub(crate) async fn despawn_master_pets(&mut self, session_id: u64) {
        let pet_oids: Vec<u32> = self.monsters.iter()
            .filter(|(_, m)| m.master_session == Some(session_id) && m.hp > 0)
            .map(|(id, _)| *id)
            .collect();
        for oid in pet_oids {
            if let Some(monster) = self.monsters.remove(&oid) {
                let remove_packet = Self::build_object_remove_packet(oid);
                broadcast_to_map(&self.gate_ref, &self.players, monster.map_index, &remove_packet).await;
                debug!("Pet '{}' despawned on master death", monster.name);
            }
        }
    }

    /// #1290：C# ProcessRegen PotTime——药水池每 200ms 处理（PerTickRegen = 5 + Level/10）
    /// #1703：怪物远程伤害延迟结算（C# DelayedAction RangeDamage）
    pub(crate) async fn tick_ranged_pending(&mut self) {
        if self.ranged_pending.is_empty() {
            return;
        }
        let now = self.tick_count;
        let mut due: Vec<RangedPendingHit> = Vec::new();
        self.ranged_pending.retain(|h| {
            if h.fire_tick <= now {
                due.push(*h);
                false
            } else {
                true
            }
        });
        for hit in due {
            let mut death_drops: Vec<(u64, i32, i32, u16)> = Vec::new();
            let mut dismount_sessions: Vec<u64> = Vec::new();
            let mut broken_armor: Vec<(u64, EquipmentSlot)> = Vec::new();
            let attacker_stats = self.monsters.get(&hit.attacker_oid)
                .map(|m| m.to_combat_stats())
                .unwrap_or_default();
            let attacker_level = self.monsters.get(&hit.attacker_oid)
                .map(|m| m.level)
                .unwrap_or(0);
            let reflected = apply_monster_hit_player(
                &self.players, &self.gate_ref, self.death_exp_penalty_percent,
                hit.attacker_oid, "怪物", &attacker_stats, attacker_level,
                hit.target_session, hit.damage, hit.px, hit.py, hit.map_index,
                hit.target_in_safe,
                if hit.is_magic { mir2_shared::enums::DefenceType::MacAgility } else { mir2_shared::enums::DefenceType::AcAgility },
                &mut death_drops, &mut dismount_sessions, &mut broken_armor,
            ).await;
            if reflected > 0 {
                if let Some(m) = self.monsters.get_mut(&hit.attacker_oid) {
                    m.take_damage(reflected);
                    m.provoked = true;
                }
            }
            for (sid, x, y, map_index) in death_drops {
                self.handle_player_death_drop(sid, x, y, map_index, false).await;
            }
            for sid in dismount_sessions {
                self.dismount_player(sid).await;
            }
            for (target_session, slot) in &broken_armor {
                if let Some(state) = self.recalculate_and_set_stat_bonuses(*target_session).await {
                    if *slot == EquipmentSlot::Weapon || *slot == EquipmentSlot::Armour {
                        self.broadcast_equipment_visuals(*target_session, &state).await;
                    }
                }
            }
        }
    }

    /// #1706：Boss 远程伤害延迟结算（C# DelayedAction RangeDamage）
    pub(crate) async fn tick_boss_ranged_pending(&mut self) {
        if self.boss_ranged_pending.is_empty() {
            return;
        }
        let now = self.tick_count;
        let mut due: Vec<BossRangedPendingHit> = Vec::new();
        self.boss_ranged_pending.retain(|h| {
            if h.fire_tick <= now {
                due.push(*h);
                false
            } else {
                true
            }
        });
        for hit in due {
            if let Some(record) = self.players.get(&hit.target_session) {
                // #1708：C# CompleteRangeAttack 校验——目标存活且同图才结算（远程飞行期间目标可能死亡/跨图）
                let target_alive_same_map = record.actor_ref.ask(GetPlayerState).await
                    .map(|s| s.map(|st| !st.is_dead && st.map_index == hit.map_index).unwrap_or(false))
                    .unwrap_or(false);
                if !target_alive_same_map {
                    continue;
                }
                let attacker_stats = self.monsters.get(&hit.attacker_oid)
                    .map(|m| m.to_combat_stats())
                    .unwrap_or_default();
                let attacker_level = self.monsters.get(&hit.attacker_oid)
                    .map(|m| m.level)
                    .unwrap_or(0);
                let attacker_name = self.monsters.get(&hit.attacker_oid)
                    .map(|m| m.name.as_str())
                    .unwrap_or("");
                let mut is_critical = false;
                let actual = if let Ok(Some(defender)) = record.actor_ref.ask(GetPlayerState).await {
                    let (actual, reflected, is_miss, crit) = resolve_monster_vs_player(
                        &attacker_stats, attacker_level, &defender, hit.damage,
                        boss_range_defence_type(attacker_name),
                    );
                    is_critical = crit;
                    if reflected > 0 {
                        if let Some(m) = self.monsters.get_mut(&hit.attacker_oid) {
                            m.take_damage(reflected);
                            m.provoked = true;
                        }
                    }
                    if is_miss {
                        broadcast_miss_feedback(
                            &self.players, &self.gate_ref, hit.map_index, hit.target_session, defender.object_id,
                        ).await;
                        0
                    } else {
                        actual
                    }
                } else { hit.damage };
                let _ = record.actor_ref.ask(TakeDamage {
                    attacker_id: hit.attacker_oid,
                    attacker_session: hit.target_session,
                    damage: actual,
                }).await;
                // #1712：Boss 远程命中反馈——同图其他玩家看受击/飘字（用实际伤害）
                if let Ok(Some(victim)) = record.actor_ref.ask(GetPlayerState).await {
                    broadcast_hit_feedback(
                        &self.players, &self.gate_ref, hit.map_index, hit.target_session,
                        victim.object_id, victim.x, victim.y, victim.direction,
                        hit.attacker_oid, actual, if is_critical { 5 } else { 0 },
                    ).await;
                }
                // CounterAttack：受击方 7s 窗口激活时反击 Boss（C# HumanObject.cs 7212/7302）
                if let Some((expire, lv)) = self.counter_attack.get(&hit.target_session).copied() {
                    if self.tick_count <= expire {
                        self.counter_attack.remove(&hit.target_session);
                        let counter_dmg = if let Ok(Some(vs)) = record.actor_ref.ask(GetPlayerState).await {
                            crate::combat::attack::get_attack_power(
                                vs.min_attack + vs.bonus_min_attack,
                                vs.max_attack + vs.bonus_max_attack,
                                vs.luck,
                            ).max(1)
                        } else { 1 };
                        if let Some(m) = self.monsters.get_mut(&hit.attacker_oid) {
                            m.take_damage(counter_dmg);
                            m.provoked = true;
                            m.target_session = Some(hit.target_session);
                            crate::combat::poison::apply_poison(&mut m.poison_list,
                                crate::combat::poison::Poison::new(
                                    mir2_shared::enums::PoisonType::STUN, lv as u32 + 1, 0, 1000,
                                ));
                            debug!("Player {} counter-attacked boss {} ({} dmg)", hit.target_session, hit.attacker_oid, counter_dmg);
                        }
                    }
                }
            }
        }
    }

    pub(crate) async fn tick_potion_pools(&mut self) {
        if self.tick_count % 2 != 0 {
            return;
        }
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        for record in self.players.values() {
            if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                if state.is_dead
                    || state.pot_time_ms > now_ms
                    || (state.pot_hp_amount == 0 && state.pot_mp_amount == 0)
                {
                    continue;
                }
                let per_tick = (5 + state.level / 10).max(1) as u32;
                let _ = record
                    .actor_ref
                    .ask(crate::actors::player::TickPotionPool { per_tick, now_ms })
                    .await;
            }
        }
    }

    /// 玩家 Buff tick + 死亡复活（每 5 ticks）
    pub(crate) async fn tick_buffs_and_revive(&mut self) {
        if self.tick_count % 5 == 0 {
            let mut to_revive = Vec::new();
            let mut to_remove = Vec::new();
            let mut to_despawn_pets = Vec::new();
            let mut torch_broke: Vec<u64> = Vec::new();
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0);
            for (session_id, record) in &self.players {
                let _ = record.actor_ref.ask(crate::actors::player::TickBuff).await;
                if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                    // C# HumanObject.Process TorchTime：每 10s 火把耐久 -5，归零自动卸下+重算属性
                    if now_ms >= state.torch_burn_time {
                        let next = now_ms + 10_000;
                        let _ = record.actor_ref.ask(crate::actors::player::SetTorchBurnTime { burn_time: next }).await;
                        let has_torch = state.inventory.equipment
                            .get(crate::actors::inventory::EquipmentSlot::Torch as usize)
                            .map(|s| s.is_some())
                            .unwrap_or(false);
                        if has_torch {
                            let broke = record.actor_ref.ask(crate::actors::player::DamageEquipment {
                                slot: crate::actors::inventory::EquipmentSlot::Torch,
                                amount: 5,
                            }).await.unwrap_or(false);
                            if broke {
                                debug!("Player session={} torch burned out!", session_id);
                                torch_broke.push(*session_id);
                            }
                        }
                    }
                    if state.is_dead {
                        match self.player_death_queue.get(session_id) {
                            None => {
                                self.player_death_queue.insert(*session_id, self.tick_count);
                                // C# Die()：主人死亡 → 宠物消失（循环外处理避免借用冲突）
                                to_despawn_pets.push(*session_id);
                            }
                            Some(death_tick) => {
                                if self.tick_count >= death_tick + 60 {
                                    to_revive.push(*session_id);
                                }
                            }
                        }
                    } else if self.player_death_queue.contains_key(session_id) {
                        to_remove.push(*session_id);
                    }
                }
            }
            for session_id in to_remove {
                self.player_death_queue.remove(&session_id);
            }
            // 火把损坏：重算属性 + 广播外观（C# RefreshStats + EquipmentChanged）
            for session_id in torch_broke {
                if let Some(st) = self.recalculate_and_set_stat_bonuses(session_id).await {
                    self.broadcast_equipment_visuals(session_id, &st).await;
                }
            }
            if !to_revive.is_empty() {
                // 自动复活由独立消息 ProcessRevives 处理（避免 Tick handler 巨型状态机
                // 内联多个 ask/广播循环导致 tokio 栈溢出，#881 回归）
                if let Some(world_ref) = self.self_ref.clone() {
                    let _ = world_ref.tell(crate::actors::world::ProcessRevives {
                        sessions: to_revive,
                    }).try_send();
                }
            }
            for session_id in to_despawn_pets {
                self.despawn_master_pets(session_id).await;
            }

            // 怪物 Poison tick（与玩家同步，每 5 ticks 推进 1 秒）
            // DelayedExplosion 毒（C# MonsterObject.ProcessDelayedExplosion）：每 2s（20 ticks）
            // 推进阶段，广播 ObjectEffect(type=1/1/2)，阶段 2 在目标当前位置结算 3×3 AoE。
            let mut delayed_effects: Vec<(u32, u8, u16)> = Vec::new(); // (object_id, stage, map_index)
            let mut pending_explosions: Vec<(u64, u16, i32, i32, i32)> = Vec::new(); // (caster, map, x, y, value)
            for (_, monster) in &mut self.monsters {
                if monster.poison_list.is_empty() {
                    continue;
                }
                if self.tick_count % 20 == 0 {
                    let mut remove_delayed = false;
                    if let Some(p) = monster.poison_list.iter_mut()
                        .find(|p| p.p_type == mir2_shared::enums::PoisonType::DELAYED_EXPLOSION)
                    {
                        if monster.hp <= 0 {
                            // 目标已死：C# ProcessDelayedExplosion 在 Dead 时直接结束
                            remove_delayed = true;
                        } else {
                            if p.delayed_stage == 0 || self.tick_count >= p.delayed_next_tick {
                                p.delayed_stage = p.delayed_stage.saturating_add(1);
                            }
                            match p.delayed_stage {
                                1 => {
                                    if p.delayed_next_tick == 0 {
                                        p.delayed_next_tick = self.tick_count + 30;
                                    }
                                    delayed_effects.push((monster.object_id, 1, monster.map_index));
                                }
                                2 => {
                                    delayed_effects.push((monster.object_id, 2, monster.map_index));
                                    pending_explosions.push((
                                        p.owner_session, monster.map_index,
                                        monster.x, monster.y, p.value,
                                    ));
                                    remove_delayed = true;
                                }
                                _ => {}
                            }
                        }
                    }
                    if remove_delayed {
                        crate::combat::poison::remove_poison(
                            &mut monster.poison_list,
                            mir2_shared::enums::PoisonType::DELAYED_EXPLOSION,
                        );
                    }
                }
                let dmg = crate::combat::poison::tick_poisons(&mut monster.poison_list, 1);
                if dmg > 0 {
                    monster.take_damage(dmg);
                    // C# MonsterObject.Process：毒伤归属毒源（LastHitter = poison.Owner）
                    monster.last_hitter_session = monster.poison_list.iter().find(|p| p.owner_session != 0).map(|p| p.owner_session);
                }
            }
            // #1797：怪物中毒视觉同步（C# CurrentPoison 变更 → ObjectPoisoned：客户端染绿/解毒）
            let mut poison_updates: Vec<(u32, u16, mir2_shared::enums::PoisonType)> = Vec::new();
            for (oid, monster) in &self.monsters {
                let combined = combined_poison_flags(&monster.poison_list);
                let last = self.monster_poison_flags.get(oid).copied().unwrap_or(mir2_shared::enums::PoisonType::NONE);
                if combined != last {
                    self.monster_poison_flags.insert(*oid, combined);
                    poison_updates.push((*oid, monster.map_index, combined));
                }
            }
            for (oid, map_index, combined) in poison_updates {
                let packet = mir2_shared::packets::server::buff::ObjectPoisoned {
                    object_id: oid,
                    poison: combined,
                };
                let mut body = Vec::new();
                if packet.write_body(&mut body).is_ok() {
                    let pkt = build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::ObjectPoisoned as i16, &body);
                    broadcast_to_map(&self.gate_ref, &self.players, map_index, &pkt).await;
                }
            }
            // #1799：玩家中毒同步（C# HumanObject.SendPoisoned：Poisoned 给自己 + ObjectPoisoned 同图广播）
            let mut player_poison_updates: Vec<(u64, u32, u16, mir2_shared::enums::PoisonType)> = Vec::new();
            for (sid, record) in &self.players {
                let Ok(Some(st)) = record.actor_ref.ask(GetPlayerState).await else { continue };
                let combined = combined_poison_flags(&st.poison_list);
                let last = self.player_poison_flags.get(sid).copied().unwrap_or(mir2_shared::enums::PoisonType::NONE);
                if combined != last {
                    self.player_poison_flags.insert(*sid, combined);
                    player_poison_updates.push((*sid, st.object_id, st.map_index, combined));
                }
            }
            for (sid, oid, map_index, combined) in player_poison_updates {
                // Poisoned 给本人（客户端毒图标 + #1616 麻痹/冰冻输入锁）
                let p = mir2_shared::packets::server::buff::Poisoned { poison: combined };
                let mut body = Vec::new();
                if p.write_body(&mut body).is_ok() {
                    let pkt = build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::Poisoned as i16, &body);
                    let _ = self.gate_ref.tell(SendToClient { session_id: sid, data: pkt }).await;
                }
                // ObjectPoisoned 同图广播（他人看染绿/解毒）
                let op = mir2_shared::packets::server::buff::ObjectPoisoned {
                    object_id: oid,
                    poison: combined,
                };
                let mut obody = Vec::new();
                if op.write_body(&mut obody).is_ok() {
                    let pkt = build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::ObjectPoisoned as i16, &obody);
                    broadcast_to_map(&self.gate_ref, &self.players, map_index, &pkt).await;
                }
            }
            // 广播 DelayedExplosion 三级 ObjectEffect（同图玩家）
            for (oid, stage, map_index) in &delayed_effects {
                let effect = mir2_shared::packets::server::magic_combat::ObjectEffect {
                    object_id: *oid,
                    effect: mir2_shared::enums::SpellEffect::DelayedExplosion,
                    effect_type: *stage as u32,
                    delay_time: 0,
                    time: 0,
                };
                let mut body = Vec::new();
                if effect.write_body(&mut body).is_ok() {
                    let pkt = build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::ObjectEffect as i16, &body);
                    for (sid, r) in &self.players {
                        if let Ok(Some(os)) = r.actor_ref.ask(GetPlayerState).await {
                            if os.map_index == *map_index {
                                let _ = self.gate_ref.tell(SendToClient {
                                    session_id: *sid, data: pkt.clone(),
                                }).await;
                            }
                        }
                    }
                }
            }
            // DelayedExplosion 阶段 2：目标当前位置 3×3 AoE MAC 伤害（C# Map.cs case DelayedExplosion）
            for (caster_session, map_index, x, y, value) in pending_explosions {
                let (attacker_stats, caster_level) = match self.players.get(&caster_session) {
                    Some(r) => match r.actor_ref.ask(GetPlayerState).await {
                        Ok(Some(s)) if !s.is_dead => (s.to_combat_stats(), s.level),
                        _ => continue,
                    },
                    None => continue,
                };
                let hit_ids: Vec<u32> = self.monsters.iter()
                    .filter(|(_, m)| {
                        let dist = (m.x - x).abs() + (m.y - y).abs();
                        dist <= 1 && m.hp > 0 && m.map_index == map_index
                    })
                    .map(|(id, _)| *id)
                    .collect();
                for mid in hit_ids {
                    if let Some(monster) = self.monsters.get_mut(&mid) {
                        let defender_stats = monster.to_combat_stats();
                        // #1452：LevelOffset = Level > attacker.Level ? 0 : min(10, attacker.Level - Level)
                        let level_offset = crate::combat::attack::level_offset(caster_level, monster.level.max(0) as u16);
                        let r = crate::combat::attack::resolve_attack(
                            &attacker_stats, &defender_stats, value.max(1),
                            mir2_shared::enums::DefenceType::Mac, level_offset,
                        );
                        if r.is_hit && r.damage > 0 {
                            monster.take_damage(r.damage);
                            monster.last_hitter_session = Some(caster_session);
                            self.pending_gather.push(caster_session);
                            monster.provoked = true;
                            monster.target_session = Some(caster_session);
                        }
                    }
                }
                debug!("DelayedExplosion 3x3 AoE at ({},{}) map {} value {}", x, y, map_index, value);
            }
        }
    }

    /// 元素系统 tick（每 5 ticks，独立消息避免 Tick handler 栈溢出）：
    /// - 专注打断 3s 后自动恢复、buff 过期广播 SetConcentration(false,false)
    /// - 玩家伤害触发的元素攒取（C# GatherElement 每次命中）
    pub(crate) async fn tick_elements(&mut self) {
        if self.tick_count % 5 != 0 {
            return;
        }
        for (sid, record) in &self.players {
            if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                let active = state.buffs.iter().any(|b| matches!(
                    b.buff_type, crate::combat::buff::BuffType::MpRegenBoost { .. }));
                if state.concentration_interrupted
                    && self.tick_count as i64 * 100 >= state.concentration_interrupt_time
                {
                    let _ = record.actor_ref.ask(crate::actors::player::SetConcentrationInterrupt {
                        interrupted: false,
                        interrupt_time_ms: 0,
                    }).await;
                    self.broadcast_set_concentration(
                        state.object_id, true, false, state.map_index).await;
                }
                let prev = self.concentration_visible.get(sid).copied().unwrap_or(false);
                if active != prev {
                    self.broadcast_set_concentration(
                        state.object_id, active, false, state.map_index).await;
                    self.concentration_visible.insert(*sid, active);
                }
            }
        }
        // 玩家伤害触发的元素攒取（C# 每次命中 GatherElement）
        let gathers = std::mem::take(&mut self.pending_gather);
        for sid in gathers {
            self.gather_element(sid).await;
        }
    }

    /// 地图环境伤害（C# Map.cs MapLightning/MapLava：随机落雷/岩浆，3~15s 一波）
    /// + 禁止坐骑地图自动下坐骑（每 20 ticks）
    pub(crate) async fn tick_environment_damage(&mut self) {
        if self.tick_count % 20 == 0 {
            // C# Map.cs：Info.Lightning/Fire 且到时 → 对每个玩家生成一次落雷/岩浆
            // （25% 落在玩家脚下，75% 在 ±10 格内随机），值 = Random(0..damage)
            let mut strikes: Vec<(u16, i32, i32, bool)> = Vec::new(); // (map, x, y, is_lightning)
            for (map_index, mi) in self.map_infos.iter() {
                let map_index_u16 = *map_index as u16;
                if mi.lightning && mi.lightning_damage > 0 {
                    let next = self.map_lightning_next_tick.get(&map_index_u16).copied().unwrap_or(0);
                    if self.tick_count >= next {
                        self.map_lightning_next_tick.insert(
                            map_index_u16, self.tick_count + fastrand::i32(30..=150) as u64);
                        for (_, record) in &self.players {
                            if let Ok(Some(st)) = record.actor_ref.ask(GetPlayerState).await {
                                if st.is_dead || st.map_index != map_index_u16 { continue; }
                                let (sx, sy) = if fastrand::i32(0..4) == 0 {
                                    (st.x, st.y)
                                } else {
                                    (st.x - 10 + fastrand::i32(0..20), st.y - 10 + fastrand::i32(0..20))
                                };
                                let valid = self.maps.get(&map_index_u16)
                                    .map(|m| m.is_valid(sx, sy))
                                    .unwrap_or(false);
                                if valid {
                                    strikes.push((map_index_u16, sx, sy, true));
                                }
                            }
                        }
                    }
                }
                if mi.fire && mi.fire_damage > 0 {
                    let next = self.map_fire_next_tick.get(&map_index_u16).copied().unwrap_or(0);
                    if self.tick_count >= next {
                        self.map_fire_next_tick.insert(
                            map_index_u16, self.tick_count + fastrand::i32(30..=150) as u64);
                        for (_, record) in &self.players {
                            if let Ok(Some(st)) = record.actor_ref.ask(GetPlayerState).await {
                                if st.is_dead || st.map_index != map_index_u16 { continue; }
                                let (sx, sy) = if fastrand::i32(0..4) == 0 {
                                    (st.x, st.y)
                                } else {
                                    (st.x - 10 + fastrand::i32(0..20), st.y - 10 + fastrand::i32(0..20))
                                };
                                let valid = self.maps.get(&map_index_u16)
                                    .map(|m| m.is_valid(sx, sy))
                                    .unwrap_or(false);
                                if valid {
                                    strikes.push((map_index_u16, sx, sy, false));
                                }
                            }
                        }
                    }
                }
            }
            // 结算打击：广播 ObjectSpell 视觉 + 落点玩家受 MAC 伤害（C# SpellObject MapLightning/MapLava）
            for (map_index, sx, sy, is_lightning) in strikes {
                let damage = self.map_infos.get(&(map_index as i32))
                    .map(|mi| if is_lightning { mi.lightning_damage } else { mi.fire_damage })
                    .unwrap_or(0);
                if damage <= 0 { continue; }
                let value = fastrand::i32(0..damage);
                let spell = if is_lightning {
                    mir2_shared::enums::Spell::MapLightning
                } else {
                    mir2_shared::enums::Spell::MapLava
                };
                // ObjectSpell 广播（客户端视觉）
                let object_spell = mir2_shared::packets::server::magic_combat::ObjectSpell {
                    object_id: 0,
                    location_x: sx,
                    location_y: sy,
                    spell,
                };
                let mut ob = Vec::new();
                if object_spell.write_body(&mut ob).is_ok() {
                    let pkt = build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::ObjectSpell as i16, &ob);
                    for (sid, r) in &self.players {
                        if let Ok(Some(os)) = r.actor_ref.ask(GetPlayerState).await {
                            if os.map_index == map_index {
                                let _ = self.gate_ref.tell(SendToClient {
                                    session_id: *sid, data: pkt.clone(),
                                }).await;
                            }
                        }
                    }
                }
                // 落点玩家 MAC 伤害（C# player.Struck(Value, MAC)，无攻击者）
                let neutral = crate::combat::attack::CombatStats::default();
                for (sid, record) in &self.players {
                    if let Ok(Some(st)) = record.actor_ref.ask(GetPlayerState).await {
                        if st.is_dead || st.map_index != map_index || st.x != sx || st.y != sy {
                            continue;
                        }
                        let defender = st.to_combat_stats();
                        let r = crate::combat::attack::resolve_attack(
                            &neutral, &defender, value.max(0),
                            mir2_shared::enums::DefenceType::Mac, 0,
                        );
                        if r.is_hit && r.damage > 0 {
                            let died = record.actor_ref.ask(TakeDamage {
                                attacker_id: 0, // environment
                                attacker_session: 0,
                                damage: r.damage,
                            }).await.unwrap_or(false);
                            if died {
                                self.player_death_queue.insert(*sid, self.tick_count);
                                broadcast_system_message(&self.gate_ref, &self.players,
                                    &format!("{} 在{}中倒下了", st.name,
                                        if is_lightning { "雷暴" } else { "火海" }));
                            } else {
                                let msg = if is_lightning { "你受到了闪电伤害！" } else { "你受到了火焰伤害！" };
                                send_system_message(&self.gate_ref, *sid, msg);
                            }
                        }
                    }
                }
            }
        }

        // 自动下坐骑：进入禁止坐骑地图时
        if self.tick_count % 20 == 0 {
            let mut to_dismount: Vec<u64> = Vec::new();
            for (session_id, record) in &self.players {
                if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                    if state.is_mounted {
                        if let Some(mi) = self.map_infos.get(&(state.map_index as i32)) {
                            if mi.no_mount {
                                to_dismount.push(*session_id);
                            }
                        }
                    }
                }
            }
            for session_id in to_dismount {
                self.dismount_player(session_id).await;
                send_system_message(&self.gate_ref, session_id, "该地图禁止骑乘坐骑，已自动下坐骑");
            }
        }
    }

    /// 经验倍率过期、全局事件过期、随机世界事件、隐身过期（每 100 ticks）
    pub(crate) async fn tick_exp_events_and_invisibility(&mut self) {
        if self.tick_count % 100 == 0 {
            for (session_id, record) in &self.players {
                if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                    if state.exp_multiplier > 1.0 && self.tick_count >= state.exp_multiplier_end_tick {
                        let _ = record.actor_ref.ask(SetExpMultiplier {
                            multiplier: 1.0,
                            end_tick: 0,
                        }).await;
                        send_system_message(&self.gate_ref, *session_id, "双倍经验效果已结束");
                        debug!("Exp multiplier expired for session {}", session_id);
                    }
                    if state.drop_multiplier > 1.0 && self.tick_count >= state.drop_multiplier_end_tick {
                        let _ = record.actor_ref.ask(SetDropMultiplier {
                            multiplier: 1.0,
                            end_tick: 0,
                        }).await;
                        send_system_message(&self.gate_ref, *session_id, "掉落加成效果已结束");
                        debug!("Drop multiplier expired for session {}", session_id);
                    }
                }
            }
            // 全局事件过期广播
            if self.tick_count >= self.global_exp_event_end_tick && self.global_exp_event_end_tick > 0 {
                let event_name = self.global_event_name.take().unwrap_or_else(|| "活动".to_string());
                self.global_exp_multiplier = 1.0;
                self.global_drop_multiplier = 1.0;
                self.global_gold_multiplier = 1.0;
                self.global_exp_event_end_tick = 0;
                for (session_id, _) in &self.players {
                    send_system_message(&self.gate_ref, *session_id, &format!("全服{}已结束", event_name));
                }
                info!("Global event ended: {}", event_name);
            }
            // 随机世界事件触发（每 36000 ticks = 1 小时，20% 概率）
            if self.tick_count > 0 && self.tick_count % 36000 == 0 && self.global_exp_event_end_tick == 0 {
                let roll = fastrand::u32(1..=100);
                if roll <= 20 {
                    let event_roll = fastrand::u32(1..=100);
                    let (name, exp_mul, drop_mul, gold_mul, duration_min) = match event_roll {
                        1..=40 => ("双倍经验", 2.0, 1.0, 1.0, 10),
                        41..=70 => ("掉落狂欢", 1.0, 2.0, 1.0, 10),
                        71..=90 => ("金币雨", 1.0, 1.0, 2.0, 10),
                        _ => ("三重盛宴", 2.0, 2.0, 2.0, 5),
                    };
                    let duration_ticks = duration_min * 600;
                    self.global_exp_multiplier = exp_mul;
                    self.global_drop_multiplier = drop_mul;
                    self.global_gold_multiplier = gold_mul;
                    self.global_exp_event_end_tick = self.tick_count + duration_ticks;
                    self.global_event_name = Some(name.to_string());
                    broadcast_system_message(&self.gate_ref, &self.players,
                        &format!("【世界事件】{} 活动已启动！经验 x{} 掉落 x{} 金币 x{}，持续 {} 分钟！",
                            name, exp_mul, drop_mul, gold_mul, duration_min));
                    info!("Random world event started: {} (exp={} drop={} gold={} for {} min)",
                        name, exp_mul, drop_mul, gold_mul, duration_min);
                }
            }
            // 隐身过期检查：从 invisible_sessions 中移除已过期玩家并广播现身
            let invis_tag = std::mem::discriminant(&crate::combat::buff::BuffType::Invisibility);
            let mut to_reveal: Vec<(u64, crate::actors::player::PlayerState)> = Vec::new();
            for session_id in &self.invisible_sessions {
                if let Some(record) = self.players.get(session_id) {
                    if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                        let still_invisible = state.buffs.iter()
                            .any(|b| std::mem::discriminant(&b.buff_type) == invis_tag);
                        if !still_invisible {
                            to_reveal.push((*session_id, state));
                        }
                    }
                }
            }
            for (session_id, state) in to_reveal {
                self.invisible_sessions.remove(&session_id);
                self.reveal_player_to_others(session_id, &state).await;
                send_system_message(&self.gate_ref, session_id, "隐身效果已结束");
            }
        }
    }

    /// 休息经验加成累积（C# PlayerObject.Process：安全区每秒 _restedCounter++；每 RestedPeriod*60 秒给一次 GiveRestedBonus）
    /// 同时处理休息加成过期（每 10 ticks = 1 秒）
    pub(crate) async fn tick_rested(&mut self) {
        if self.tick_count % 10 != 0 {
            return;
        }
        let cfg = self.rested_cfg.clone();
        for (session_id, record) in &self.players {
            let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await else { continue };
            // 休息加成过期清理
            if state.rested_exp_percent > 0 {
                let now_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);
                if now_ms >= state.rested_exp_end_tick {
                    let _ = record.actor_ref.ask(crate::actors::player::SetRestedExp { percent: 0, end_tick: 0 }).await;
                    send_system_message(&self.gate_ref, *session_id, "休息经验加成已结束");
                    debug!("Rested expired for session {}", session_id);
                }
            }
            // 安全区每秒累积 + 到账（C#：counter 仅在安全区 +1；登录时已按离线分钟初始化）
            let in_safe = self.maps.get(&state.map_index)
                .map(|m| m.is_safe_zone(state.x, state.y))
                .unwrap_or(false);
            if in_safe || state.rested_counter > 0 {
                let mut counter = state.rested_counter;
                if in_safe {
                    counter = counter.saturating_add(1);
                }
                let count = counter / (cfg.period_secs.max(1) * 60);
                if count > 0 {
                    let _ = record.actor_ref.ask(crate::actors::player::GiveRestedBonus {
                        count,
                        buff_length_minutes: cfg.buff_length_minutes,
                        exp_bonus_percent: cfg.exp_bonus_percent,
                        max_bonus: cfg.max_bonus,
                    }).await;
                } else if counter != state.rested_counter {
                    let _ = record.actor_ref.ask(crate::actors::player::SetRestedCounter { counter }).await;
                }
            }
        }
    }


    /// #1354：向同地图玩家广播坐下/起身（C# FrostTiger.Sitting → CurrentMap.Broadcast(S.ObjectSitDown)）
    async fn broadcast_object_sit_down(&self, object_id: u32, x: i32, y: i32, direction: u8, sitting: bool) {
        let packet = mir2_shared::packets::server::miscellaneous::ObjectSitDown {
            object_id,
            location: (x, y),
            direction,
            sitting,
        };
        let mut body = Vec::new();
        if packet.write_body(&mut std::io::Cursor::new(&mut body)).is_err() {
            return;
        }
        let data = build_packet_bytes(mir2_shared::enums::ServerPacketIds::ObjectSitDown as i16, &body);
        let map_index = self.monsters.get(&object_id).map(|m| m.map_index).unwrap_or(0);
        for (sid, rec) in &self.players {
            if let Ok(Some(s)) = rec.actor_ref.ask(GetPlayerState).await {
                if s.map_index == map_index {
                    let _ = self.gate_ref.tell(SendToClient {
                        session_id: *sid,
                        data: data.clone(),
                    }).await;
                }
            }
        }
    }

    /// 广播对象显示/隐藏（C# S.ObjectShow / S.ObjectHide：Shinsu 形态切换等）
    async fn broadcast_object_show_hide(&self, object_id: u32, visible: bool) {
        let mut body = Vec::new();
        body.extend_from_slice(&object_id.to_le_bytes());
        let opcode = if visible {
            mir2_shared::enums::ServerPacketIds::ObjectShow as i16
        } else {
            mir2_shared::enums::ServerPacketIds::ObjectHide as i16
        };
        let packet = build_packet_bytes(opcode, &body);
        // #1686：显隐广播只发同图玩家（C# CurrentMap）
        let map_idx = self.monsters.get(&object_id).map(|m| m.map_index).unwrap_or(0);
        broadcast_to_map(&self.gate_ref, &self.players, map_idx, &packet).await;
    }

/// PK 值衰减 + 名字颜色广播（C# MapObject.Process：每 Settings.PKDelay=12 秒衰减 1 点）
    /// 死亡回调：调用 behavior.on_die 并应用其输出（C# Die 覆盖；
    /// 此前 on_die 从未被接线，HumanAssassin 死亡爆炸 / KingHydrax 死亡召唤等机制失效）。
    /// player_positions 为 AI 循环预收集的玩家快照 (session,x,y,oid,pk,hp,map)。
    async fn apply_death_callbacks(
        &mut self,
        monster: &mut MonsterState,
        player_positions: &[(u64, i32, i32, u32, i32, i32, u16, u16, i32)],
    ) {
        use crate::actors::world::ai::{self, AiCtx};
        let mut die_moves: Vec<(u32, i32, i32, u8)> = Vec::new();
        let mut die_attacks: Vec<ai::AttackAction> = Vec::new();
        let mut die_spell_fields: Vec<ai::SpellFieldSpawn> = Vec::new();
        let mut die_summons: Vec<ai::BossSummon> = Vec::new();
        let mut die_child_rocks: Vec<ai::ChildRockSpawn> = Vec::new();
        let mut die_heals: Vec<(u32, i32)> = Vec::new();
        let mut die_poisons: Vec<ai::PoisonPlayer> = Vec::new();
        let mut die_backsteps: Vec<(u32, u8, i32)> = Vec::new();
        let mut die_pushes: Vec<ai::PushPlayer> = Vec::new();
        let mut die_teleports: Vec<(u64, i32, i32, u8)> = Vec::new();
        let mut die_delayed: Vec<ai::DelayedAttack> = Vec::new();
        let mut die_taunts: Vec<(u32, u32)> = Vec::new();
        let mut die_monster_teleports: Vec<(u32, i32, i32)> = Vec::new();
        let mut die_player_buffs: Vec<(u64, crate::combat::buff::BuffInstance)> = Vec::new();
        let mut die_show_hide: Vec<(u32, bool)> = Vec::new();
        let mut die_sit_down: Vec<(u32, i32, i32, u8, bool)> = Vec::new();
        let mut die_effects: Vec<(u32, mir2_shared::enums::SpellEffect)> = Vec::new();
        let mut die_player_purges: Vec<u64> = Vec::new();
        let mut die_player_heals: Vec<(u64, i32)> = Vec::new();
        {
            // 死亡回调也提供玩家快照（C# Die 可 FindAllTargets；ToxicGhoul 死亡 AOE 毒等用）
            let die_player_snaps: Vec<ai::PlayerSnap> = player_positions.iter()
                .map(|(s, x, y, oid, pk, hp, map, lvl, min_dc)| ai::PlayerSnap {
                    session_id: *s, x: *x, y: *y, hp: *hp, map_index: *map, object_id: *oid, level: *lvl, pk_points: *pk, min_dc: *min_dc,
                }).collect();
            let die_monster_name_map: std::collections::HashMap<i32, String> =
                self.monster_infos.iter().map(|(k, v)| (*k, v.name.clone())).collect();
            let mut ctx = AiCtx {
                tick_count: self.tick_count,
                monster_oid: monster.object_id,
                monster_index: monster.monster_index,
                map_size: self.maps.get(&monster.map_index)
                    .map(|m| (m.width as i32, m.height as i32))
                    .unwrap_or((200, 200)),
                dragon_level: 0,
                players: &die_player_snaps,
                monsters: &[],
                monster_name_by_index: &die_monster_name_map,
                slave_count: 0,
                out_moves: &mut die_moves,
                out_attacks: &mut die_attacks,
                out_spell_fields: &mut die_spell_fields,
                out_summons: &mut die_summons,
                out_child_rocks: &mut die_child_rocks,
                out_heals: &mut die_heals,
                out_backsteps: &mut die_backsteps,
                out_poisons: &mut die_poisons,
                out_pushes: &mut die_pushes,
                out_player_teleports: &mut die_teleports,
                out_delayed_attacks: &mut die_delayed,
                out_monster_taunts: &mut die_taunts,
                out_monster_teleports: &mut die_monster_teleports,
                out_player_buffs: &mut die_player_buffs,
                out_show_hide: &mut die_show_hide,
                out_sit_down: &mut die_sit_down,
                out_effects: &mut die_effects,
                out_player_purges: &mut die_player_purges,
                out_player_heals: &mut die_player_heals,
                pet_level: self.pet_levels.get(&monster.object_id).copied().unwrap_or(0),
                master_pet_mode: None,
                master_target: None,
                has_master_monster_target: false,
            };
            // 临时取出 behavior 避免 &mut monster + &mut behavior 双重借用（与 AI 循环一致）
            let mut behavior = std::mem::replace(
                &mut monster.behavior,
                Box::new(crate::actors::world::ai::DefaultBehavior::new()),
            );
            behavior.on_die(monster, &mut ctx);
            monster.behavior = behavior;
        }
        // 应用死亡攻击（HumanAssassin 16 方向爆炸 → 半径 2 AOE）
        for atk in &die_attacks {
            let (attacker_oid, damage, cx, cy, radius) = match atk {
                ai::AttackAction::Aoe { attacker_oid, center_x, center_y, radius, damage, .. } => {
                    (*attacker_oid, *damage, *center_x, *center_y, *radius)
                }
                ai::AttackAction::Melee { attacker_oid, target_session, damage, .. } => {
                    let _ = target_session;
                    (*attacker_oid, *damage, monster.x, monster.y, 0)
                }
                ai::AttackAction::Range { attacker_oid, target_session, damage, .. } => {
                    let _ = target_session;
                    (*attacker_oid, *damage, monster.x, monster.y, 0)
                }
                // #1020：死亡回调直线攻击近似为半径=range 的 AOE
                ai::AttackAction::Line { attacker_oid, origin_x, origin_y, range, damage, .. } => {
                    (*attacker_oid, *damage, *origin_x, *origin_y, *range)
                }
            };
            // 广播 ObjectAttack（死亡爆炸动画）
            let mut attack_body = Vec::new();
            attack_body.extend_from_slice(&attacker_oid.to_le_bytes());
            attack_body.extend_from_slice(&(monster.x as u32).to_le_bytes());
            attack_body.extend_from_slice(&(monster.y as u32).to_le_bytes());
            attack_body.push(monster.direction);
            attack_body.push(0u8); // spell
            attack_body.push(0u8); // level
            attack_body.push(0u8); // attack_type
            let attack_packet = build_packet_bytes(
                mir2_shared::enums::ServerPacketIds::ObjectAttack as i16, &attack_body);
            // #1649：怪物生成/动画广播只发同图玩家（C# CurrentMap.Broadcast）
            broadcast_to_map(&self.gate_ref, &self.players, monster.map_index, &attack_packet).await;
            // 对范围内玩家造成伤害
            for (sid, px, py, _, _, _, pmap, _, _) in player_positions {
                if *pmap != monster.map_index { continue; }
                let dx = (px - cx).abs();
                let dy = (py - cy).abs();
                if dx.max(dy) > radius { continue; }
                if let Some(record) = self.players.get(sid) {
                    let _ = record.actor_ref.ask(crate::actors::player::TakeDamage {
                        attacker_id: attacker_oid,
                        attacker_session: *sid,
                        damage,
                    }).await;
                }
            }
        }
        // 应用死亡召唤（KingHydrax 死亡召唤 2 只 slave）
        for bs in &die_summons {
            let mon_index = self.monster_name_index.get(&bs.monster_name.to_lowercase()).copied();
            if let Some(idx) = mon_index {
                let info_opt = self.monster_infos.get(&idx).cloned();
                if let Some(info) = info_opt {
                    let new_oid = self.alloc_object_id();
                    let hp = info.stats.get(&(mir2_shared::enums::Stat::HP as u8)).copied().unwrap_or(50);
                    let min_dmg = info.stats.get(&(mir2_shared::enums::Stat::MinDC as u8)).copied().unwrap_or(5);
                    let max_dmg = info.stats.get(&(mir2_shared::enums::Stat::MaxDC as u8)).copied().unwrap_or(10);
                    let map_index = monster.map_index;
                    let spawn = MonsterSpawn {
                        name: info.name.clone(),
                        image: info.image as u16,
                        monster_index: idx,
                        x: bs.x, y: bs.y, direction: 0,
                        hp, min_dmg, max_dmg, xp: info.experience,
                        map_index,
                        count: 1,
                        spread: 0,
                    };
                    let packet = build_object_monster_packet(&spawn, new_oid, &spawn.name);
                    // #1649：怪物生成/动画广播只发同图玩家（C# CurrentMap.Broadcast）
                    broadcast_to_map(&self.gate_ref, &self.players, spawn.map_index, &packet).await;
                    let ai_profile = MonsterAiProfile::from_info(&info);
                    self.monsters.insert(new_oid, MonsterState {
                        object_id: new_oid,
                        name: spawn.name.clone(),
                        image: spawn.image,
                        monster_index: idx,
                        x: bs.x, y: bs.y, direction: 0,
                        hp, max_hp: hp, min_dmg, max_dmg, xp: spawn.xp,
                        spawn_x: bs.x, spawn_y: bs.y, map_index,
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
                        undead: false,
                        master_session: None,
                                rarity: 0,
                                pet_experience: 0,
                                max_pet_level: 0,
                        recall_at_tick: 0,
                        behavior: crate::actors::world::ai::make_behavior(&spawn.name),
                    });
                    if let Some(m) = self.monsters.get_mut(&new_oid) {
                        m.fill_combat_stats(&info);
                    }
                    debug!("Death callback summoned '{}' as #{} at ({},{}) slave={}",
                           spawn.name, new_oid, bs.x, bs.y, bs.is_slave);
                }
            }
        }
        // 应用死亡 poison / 推开 / 传送 / 延迟攻击
        for pp in &die_poisons {
            if let Some(record) = self.players.get(&pp.session_id) {
                let _ = record.actor_ref.ask(crate::actors::player::ApplyCombatPoisons {
                    poisons: vec![pp.poison],
                }).await;
            }
        }
        for pp in &die_pushes {
            let _ = self.push_player(pp.session_id, pp.dir, pp.distance).await;
        }
        for (sid, tx, ty, dir) in &die_teleports {
            if let Some(record) = self.players.get(sid) {
                if let Ok(Some(st)) = record.actor_ref.ask(GetPlayerState).await {
                    let walkable = self.maps.get(&st.map_index)
                        .map(|m| m.is_walkable(*tx, *ty))
                        .unwrap_or(false);
                    if walkable {
                        let _ = record.actor_ref.ask(crate::actors::player::SetPlayerPosition {
                            x: *tx, y: *ty, direction: *dir, map_index: None, is_mounted: None,
                        }).await;
                    }
                }
            }
        }
        for atk in &die_delayed {
            self.boss_pending_attacks.push((self.tick_count + atk.delay_ticks, *atk));
        }
        for (oid, tx, ty) in &die_monster_teleports {
            if let Some(m) = self.monsters.get_mut(oid) {
                let walkable = self.maps.get(&m.map_index)
                    .map(|mm| mm.is_walkable(*tx, *ty))
                    .unwrap_or(false);
                if walkable {
                    m.x = *tx;
                    m.y = *ty;
                    // 广播位置更新（ObjectWalk 近似 ObjectMonster）
                    let mut walk_body = Vec::new();
                    walk_body.extend_from_slice(&oid.to_le_bytes());
                    walk_body.extend_from_slice(&m.x.to_le_bytes());
                    walk_body.extend_from_slice(&m.y.to_le_bytes());
                    walk_body.push(m.direction);
                    let walk_packet = build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::ObjectWalk as i16, &walk_body);
                    // #1649：动画广播只发同图玩家（C# CurrentMap.Broadcast）
                    broadcast_to_map(&self.gate_ref, &self.players, m.map_index, &walk_packet).await;
                }
            }
        }
        for (sid, buff) in &die_player_buffs {
            if let Some(record) = self.players.get(sid) {
                let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff { buff: buff.clone() }).await;
            }
        }
    }


    pub(crate) async fn tick_pk_decay(&mut self) {
        if self.tick_count % 120 == 0 { // 12s × 10 ticks/s
            let mut colour_changes = Vec::new();
            for (session_id, record) in &self.players {
                let _ = record.actor_ref.ask(crate::actors::player::DecayPkPoints).await;
                if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                    // #921：自视角颜色（含 WarZone/行会战；进沙巴克/宣战即触发刷新，C# RefreshNameColour）
                    let new_colour = self.self_name_colour(&state);
                    let old_colour = record.last_colour;
                    if new_colour != old_colour {
                        colour_changes.push((*session_id, new_colour, state.pk_points));
                    }
                }
            }
            for (session_id, new_colour, pk_points) in colour_changes {
                if let Some(record) = self.players.get_mut(&session_id) {
                    record.last_pk_points = pk_points;
                    record.last_colour = new_colour;
                }
                // #921：逐观众广播名字颜色（C# BroadcastColourChange）
                self.broadcast_viewer_colours(session_id).await;
            }
        }
    }

    /// 钓鱼收获判定（每 tick；对齐 C# PlayerObject.UpdateFish + FishingCast(false) 收获）
    pub(crate) async fn tick_fishing(&mut self) {
        let mut caught = Vec::new(); // session_id
        let mut stopped = Vec::new(); // session_id
        // (session, item_index, 是否刷怪, x, y, map, autocast)
        let mut events: Vec<(u64, Option<i32>, bool, i32, i32, u16, bool)> = Vec::new();
        for (session_id, record) in &self.players {
            if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                if !state.is_fishing { continue; }

                let counter = self.fishing_tick_counters.entry(*session_id).or_insert(0);
                *counter += 1;

                // C# FishingProgressMax = Settings.FishingAttempts（30 ticks ≈ 3 秒）
                let attempts = self.fishing_cfg.attempts.max(1);
                if *counter < attempts {
                    continue;
                }

                let Some(session) = self.fishing_sessions.get(session_id).cloned() else {
                    stopped.push(*session_id);
                    continue;
                };

                // C# FishingCast(false)：FishingProgress > 99 → FishingChanceCounter++（跨抛竿保留）
                let counter = self.fishing_success_counters.entry(*session_id).or_insert(0);
                *counter = counter.saturating_add(1);

                // C# FishingCast(false)：getChance = FishingChance + Random(10,24) + (进度>50 ? flexibility/2 : 0)
                let get_chance = (session.chance + fastrand::i32(10..=24) + session.flexibility / 2).clamp(0, 100);

                if fastrand::i32(0..=100) <= get_chance {
                    // C# 成功收获 → FishingChanceCounter = 0
                    self.fishing_success_counters.insert(*session_id, 0);
                    // C#：Envir.FishingDrops.Where(Type == FishingAttribute) → AttemptDrop
                    let item_index = self.attempt_fishing_drop(session.cell_attribute, 0);
                    if let Some(idx) = item_index {
                        let item = crate::actors::inventory::make_item(idx, 1);
                        let added = record.actor_ref.ask(crate::actors::player::AddItemToInventory { item }).await.unwrap_or(false);
                        if added {
                            send_system_message(&self.gate_ref, *session_id, "钓到了物品！");
                        } else {
                            send_system_message(&self.gate_ref, *session_id, "钓到了物品，但背包已满！");
                        }
                    } else {
                        send_system_message(&self.gate_ref, *session_id, "鱼跑了...");
                    }

                    // C#：收获时按 MonsterSpawnChance 刷 FishingMonster（Next(100-chance)==0；100% 恒刷）
                    let spawn = self.fishing_cfg.monster_spawn_chance > 0
                        && !self.fishing_cfg.monster.is_empty()
                        && (self.fishing_cfg.monster_spawn_chance >= 100
                            || fastrand::i32(0..(100 - self.fishing_cfg.monster_spawn_chance as i32)) == 0);

                    // #1313：收获后卷线器耐久 -1（C# DamagedFishingItem(Reel,1)）；损坏且自动钓鱼 → 停止自动钓鱼
                    let reel_result = record.actor_ref.ask(crate::actors::player::FishingGearDamageMsg { slot: 4, amount: 1 }).await.unwrap_or(0);
                    let autocast = state.fishing_autocast && reel_result != 2;
                    events.push((*session_id, item_index, spawn, state.x, state.y, state.map_index, autocast));
                } else {
                    send_system_message(&self.gate_ref, *session_id, "鱼跑了...");
                    stopped.push(*session_id);
                }
            }
        }
        for (session_id, _item_index, spawn, x, y, map_index, autocast) in events {
            if spawn {
                let monster = self.fishing_cfg.monster.clone();
                let _ = self.spawn_monster_named(&monster, x, y, 1, map_index).await;
            }
            if autocast {
                caught.push(session_id);
            } else {
                stopped.push(session_id);
            }
        }
        for session_id in caught {
            self.fishing_tick_counters.insert(session_id, 0);
            // Send bite state then auto-recast waiting state
            let bite_packet = mir2_shared::packets::server::miscellaneous::FishingUpdate { fishing_progress: 2, fishing_success: true };
            let mut body = Vec::new();
            if let Ok(()) = mir2_shared::packets::Packet::write_body(&bite_packet, &mut body) {
                let _ = self.gate_ref.tell(SendToClient {
                    session_id,
                    data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::FishingUpdate as i16, &body),
                }).await;
            }
            // Then immediately send waiting state for autocast
            let wait_packet = mir2_shared::packets::server::miscellaneous::FishingUpdate { fishing_progress: 1, fishing_success: false };
            let mut body2 = Vec::new();
            if let Ok(()) = mir2_shared::packets::Packet::write_body(&wait_packet, &mut body2) {
                let _ = self.gate_ref.tell(SendToClient {
                    session_id,
                    data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::FishingUpdate as i16, &body2),
                }).await;
            }
        }
        for session_id in stopped {
            self.fishing_tick_counters.remove(&session_id);
            self.fishing_sessions.remove(&session_id);
            if let Some(record) = self.players.get(&session_id) {
                let _ = record.actor_ref.ask(crate::actors::player::SetFishing { is_fishing: false, autocast: false }).await;
            }
            // Send idle state
            let idle_packet = mir2_shared::packets::server::miscellaneous::FishingUpdate { fishing_progress: 0, fishing_success: false };
            let mut body = Vec::new();
            if let Ok(()) = mir2_shared::packets::Packet::write_body(&idle_packet, &mut body) {
                let _ = self.gate_ref.tell(SendToClient {
                    session_id,
                    data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::FishingUpdate as i16, &body),
                }).await;
            }
        }
    }

    /// 地面物品过期清理（每 50 ticks）
    pub(crate) async fn tick_ground_cleanup(&mut self) {
        if self.tick_count % 50 == 0 {
            // C# Settings.ItemTimeOut = 30s（配置化 item_timeout_ticks）；
            // 死亡掉落 PlayerDiedItemTimeOut = 120s（4×）
            let expired: Vec<_> = self.ground_items.iter()
                .filter(|gi| {
                    let lifetime = if gi.death_drop { self.item_timeout_ticks * 4 } else { self.item_timeout_ticks };
                    self.tick_count >= gi.drop_tick + lifetime
                })
                .map(|gi| (gi.object_id, gi.map_index))
                .collect();
            if !expired.is_empty() {
                self.ground_items.retain(|gi| {
                    let lifetime = if gi.death_drop { self.item_timeout_ticks * 4 } else { self.item_timeout_ticks };
                    self.tick_count < gi.drop_tick + lifetime
                });
                for (oid, map_idx) in &expired {
                    let remove_packet = Self::build_object_remove_packet(*oid);
                    for (sid, rec) in &self.players {
                        if let Ok(Some(s)) = rec.actor_ref.ask(GetPlayerState).await {
                            if s.map_index == *map_idx {
                                let _ = self.gate_ref.tell(SendToClient {
                                    session_id: *sid,
                                    data: remove_packet.clone(),
                                }).await;
                            }
                        }
                    }
                }
                debug!("Cleaned up {} expired ground items", expired.len());
            }
        }
    }

    /// 怪物重生处理（每 tick）
    pub(crate) async fn tick_respawn(&mut self) {
        let mut to_respawn = Vec::new();
        for (oid, (spawn, tick)) in &self.respawn_queue {
            if self.tick_count >= *tick {
                to_respawn.push((*oid, spawn.clone()));
            }
        }
        for (oid, spawn) in to_respawn {
            self.respawn_queue.remove(&oid);
            let new_oid = self.alloc_object_id();
            // C# RespawnInfo.Spread：重生时在出生点 ±spread 内随机可走格落点
            let (rx, ry) = random_spawn_pos(self.maps.get(&spawn.map_index), spawn.x, spawn.y, spawn.spread);
            let mut respawn_pos = spawn.clone();
            respawn_pos.x = rx;
            respawn_pos.y = ry;
            let monster_info_opt = self.monster_infos.get(&spawn.monster_index);
            let ai_profile = monster_info_opt
                .map(MonsterAiProfile::from_info)
                .unwrap_or_else(|| MonsterAiProfile {
                    ai_type: MonsterAiType::Aggressive,
                    aggro_range: 10,
                    attack_range: 1,
                    attack_cooldown: 5,
                    move_interval: 2,
                    flee_threshold: 0.0,
                });
            // 稀有度判定（C# MonsterRarityData.Roll：Uncommon/Rare/Elite）
            let rarity = crate::actors::world::roll_rarity(&self.rarity_cfg);
            let is_elite = rarity >= 3;
            let monster_level = monster_info_opt.map(|i| i.level).unwrap_or(0);
            let monster_effect = monster_info_opt.map(|i| i.effect).unwrap_or(0);
            let (hp_m, dmg_m, xp_m, def_m) = match rarity {
                3 => (self.rarity_cfg.elite_hp_multiplier,
                      self.rarity_cfg.elite_dmg_multiplier,
                      self.rarity_cfg.elite_xp_multiplier,
                      self.rarity_cfg.elite_defense_multiplier),
                2 => (self.rarity_cfg.rare_hp_multiplier,
                      self.rarity_cfg.rare_damage_multiplier,
                      self.rarity_cfg.rare_exp_multiplier,
                      self.rarity_cfg.rare_defense_multiplier),
                1 => (self.rarity_cfg.uncommon_hp_multiplier,
                      self.rarity_cfg.uncommon_damage_multiplier,
                      self.rarity_cfg.uncommon_exp_multiplier,
                      self.rarity_cfg.uncommon_defense_multiplier),
                _ => (1.0, 1.0, 1.0, 1.0),
            };
            let prefix = crate::actors::world::rarity_prefix(rarity);
            let _def_m = def_m; // 防御倍率接线（Rust 怪物基础 AC 为 0，实际无效果）
            let (name, hp, max_hp, min_dmg, max_dmg, xp) = if rarity > 0 {
                (
                    format!("{}{}", prefix, spawn.name),
                    (spawn.hp as f64 * hp_m).max(1.0) as i32,
                    (spawn.hp as f64 * hp_m).max(1.0) as i32,
                    (spawn.min_dmg as f64 * dmg_m) as i32,
                    (spawn.max_dmg as f64 * dmg_m) as i32,
                    (spawn.xp as f64 * xp_m).max(1.0) as i32,
                )
            } else {
                (spawn.name.clone(), spawn.hp, spawn.hp, spawn.min_dmg, spawn.max_dmg, spawn.xp)
            };
            // #1701：稀有度在下方掷点后才生成前缀名字——原实现在掷点前广播导致重生稀有怪名字无前缀；
            // 现在移到掷点/属性计算之后广播（C# MonsterObject.DropItem 后可见即带稀有名前缀/颜色）
            let packet = build_object_monster_packet(&respawn_pos, new_oid, &name);
            // #1649：怪物生成/动画广播只发同图玩家（C# CurrentMap.Broadcast）
            broadcast_to_map(&self.gate_ref, &self.players, respawn_pos.map_index, &packet).await;
            // #1701：稀有怪名字颜色（C# MonsterRarityData.NameColour → ObjectColourChanged）
            if rarity > 0 {
                let colour_packet = build_object_colour_changed_packet(new_oid, crate::actors::world::rarity_name_colour(rarity));
                broadcast_to_map(&self.gate_ref, &self.players, respawn_pos.map_index, &colour_packet).await;
            }
            self.monsters.insert(new_oid, MonsterState {
                object_id: new_oid,
                name: name.clone(),
                image: spawn.image,
                monster_index: spawn.monster_index,
                x: rx,
                y: ry,
                direction: spawn.direction,
                hp,
                max_hp,
                min_dmg,
                max_dmg,
                xp,
                spawn_x: spawn.x,
                spawn_y: spawn.y,
                spawn_spread: spawn.spread,
                map_index: spawn.map_index,
                next_attack_tick: 0,
                next_move_tick: 0,
                next_summon_tick: 0,
                ai_profile,
                ai_state: MonsterAiState::Idle,
                sitting: false,
                hidden: false,
                sit_down_tick: 0,
                target_session: None,
                last_hitter_session: None,
                provoked: false,
                rarity,
                is_elite,
                is_boss: false,
                min_ac: 0,
                max_ac: 0,
                min_mac: 0,
                max_mac: 0,
                agility: 0,
                accuracy: 0,
                armour_rate: 1.0,
                damage_rate: 1.0,
                magic_resist: 0,
                critical_rate: 0,
                critical_damage: 0,
                luck: 0,
                reflect: 0,
                level: monster_level,
                effect: monster_effect,
                damage_reduction_percent: 0,
                poison_list: Vec::new(),
                last_hit_damage: 0,
            undead: false,
                master_session: None,
                                pet_experience: 0,
                                max_pet_level: 0,
                recall_at_tick: 0,
                behavior: crate::actors::world::ai::make_behavior(&name),
            });
            if rarity > 0 {
                let map_name = self.map_infos.get(&(spawn.map_index as i32)).map(|m| m.title.clone()).unwrap_or_else(|| "未知地图".to_string());
                broadcast_system_message(&self.gate_ref, &self.players,
                    &format!("一只 {}{} 出现在 {}！勇士们，前往讨伐！", prefix.trim(), spawn.name, map_name));
                debug!("Rarity monster '{}' (tier {}) spawned as #{} at ({},{})", name, rarity, new_oid, spawn.x, spawn.y);
            } else {
                debug!("Monster '{}' respawned as #{}", spawn.name, new_oid);
            }
        }
    }

    /// 世界Boss超时消失（每 tick）
    pub(crate) async fn tick_boss_timeout(&mut self) {
        let mut boss_despawns = Vec::new();
        for (oid, despawn_tick) in &self.world_boss_queue {
            if should_despawn_boss(self.tick_count, *despawn_tick) {
                boss_despawns.push(*oid);
            }
        }
        for oid in boss_despawns {
            self.world_boss_queue.remove(&oid);
            if let Some(monster) = self.monsters.remove(&oid) {
                let packet = Self::build_object_remove_packet(oid);
                broadcast_to_map(&self.gate_ref, &self.players, monster.map_index, &packet).await;
                broadcast_system_message(&self.gate_ref, &self.players,
                    &format!("世界Boss {} 因无人挑战而消失了", monster.name));
                debug!("World boss '{}' (#{}) despawned (timeout)", monster.name, oid);
            }
        }
    }

    /// 任务超时检查（每 100 ticks）
    pub(crate) async fn tick_quest_timeout(&mut self) {
        if self.tick_count.is_multiple_of(100) {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            for (session_id, record) in &self.players {
                if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                    for quest in &state.quest_log.quests {
                        if quest.time_limit_seconds > 0
                            && matches!(quest.status, QuestStatus::InProgress | QuestStatus::Accepted)
                            && now.saturating_sub(quest.start_time) >= quest.time_limit_seconds as u64
                        {
                            let failed = record.actor_ref.ask(crate::actors::player::FailQuest {
                                quest_index: quest.quest_index,
                            }).await.unwrap_or(false);
                            if failed {
                                send_system_message(
                                    &self.gate_ref, *session_id,
                                    &format!("任务 '{}' 已超时失败", quest.title)
                                );
                                debug!("Quest expired: {} for session {}", quest.title, session_id);
                            }
                        }
                    }
                }
            }
        }
    }

    /// 宠物自动拾取（每 tick）
    pub(crate) async fn tick_pet_pickup(&mut self) {
        // #1586：归属校验（与玩家拾取 #1262 一致）——组号映射一次构建
        let mut player_groups: std::collections::HashMap<u64, Option<u64>> = std::collections::HashMap::new();
        for (sid, rec) in &self.players {
            if let Ok(Some(s)) = rec.actor_ref.ask(GetPlayerState).await {
                player_groups.insert(*sid, s.group_id);
            }
        }
        let mut pet_pickups: Vec<(usize, u64)> = Vec::new(); // (ground_item_index, session_id)
        for (session_id, record) in &self.players {
            if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                let creature = match state.creature_log.active_creature {
                    Some(ref c) if c.enabled && !c.is_starving() => c,
                    _ => continue,
                };
                let pickup_mode = creature.pickup_mode;
                if pickup_mode == crate::actors::creature::PickupMode::None {
                    continue;
                }
                // 找附近匹配的物品（最多拾取 1 个/ tick）
                for (gi_idx, gi) in self.ground_items.iter().enumerate() {
                    let dist = (state.x - gi.x).abs() + (state.y - gi.y).abs();
                    if dist > 3 { continue; }
                    if gi.map_index != state.map_index { continue; }
                    // #1586：归属校验（C# Owner=EXPOwner 60s / 同组可拾取 / 过期任意）
                    if !crate::actors::world::item::can_pick_drop(
                        self.tick_count,
                        gi.drop_tick,
                        crate::actors::world::item::DROP_OWNERSHIP_TICKS,
                        gi.dropper_session,
                        *session_id,
                        gi.dropper_session
                            .and_then(|d| player_groups.get(&d).copied().flatten()),
                        state.group_id,
                    ) {
                        continue;
                    }

                    let is_gold = gi.item.item_index == 0;
                    let should_pickup = match pickup_mode {
                        crate::actors::creature::PickupMode::GoldOnly => is_gold,
                        crate::actors::creature::PickupMode::GoldAndItem => true,
                        crate::actors::creature::PickupMode::All => true,
                        _ => false,
                    };
                    if should_pickup {
                        pet_pickups.push((gi_idx, *session_id));
                        break; // 每个玩家每 tick 最多拾取 1 个
                    }
                }
            }
        }

        // 应用拾取（从后往前删除，避免索引偏移）
        pet_pickups.sort_by(|a, b| b.0.cmp(&a.0));
        pet_pickups.dedup_by(|a, b| a.0 == b.0); // 同一物品只拾取一次

        for (gi_idx, session_id) in pet_pickups {
            if gi_idx >= self.ground_items.len() { continue; }
            let gi = self.ground_items.remove(gi_idx);

            // 广播移除
            let remove_packet = Self::build_object_remove_packet(gi.object_id);
            broadcast_to_map(&self.gate_ref, &self.players, gi.map_index, &remove_packet).await;

            if let Some(record) = self.players.get(&session_id) {
                if gi.item.item_index == 0 {
                    // 金币
                    let gold = gi.gold_amount as u64;
                    let _ = record.actor_ref.ask(crate::actors::player::AddGold { amount: gold }).await;
                    send_system_message(&self.gate_ref, session_id,
                        &format!("宠物帮你拾取了 {} 金币", gold));
                } else {
                    // 检查背包空间
                    let has_space = record.actor_ref.ask(crate::actors::player::HasItemSpace).await.unwrap_or(false);
                    if has_space {
                        let _ = record.actor_ref.ask(crate::actors::player::AddItemToInventory {
                            item: gi.item.clone(),
                        }).await;
                        send_system_message(
                            &self.gate_ref, session_id,
                            &format!("宠物帮你拾取了物品"));
                    } else {
                        // 背包已满，把物品掉回去
                        self.ground_items.push(gi);
                        send_system_message(&self.gate_ref, session_id,
                            "宠物发现物品但你的背包已满");
                    }
                }
            }
        }
    }

    /// NPC 商店自动补货（每小时）
    pub(crate) async fn tick_shop_restock(&mut self) {
        if self.tick_count.is_multiple_of(36000) {
            let mut restocked = 0usize;
            for goods_list in self.npc_goods.values_mut() {
                for good in goods_list.iter_mut() {
                    if !good.infinite_stock && good.stock < good.max_stock {
                        good.stock = good.max_stock;
                        restocked += 1;
                    }
                }
            }
            if restocked > 0 {
                info!("NPC shop restock: {} items restocked", restocked);
            }
        }
    }

    /// 精炼自动完成（每 100 ticks）
    pub(crate) async fn tick_refine_complete(&mut self) {
        if self.tick_count.is_multiple_of(100) {
            let current_time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            for (session_id, record) in &self.players {
                if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                    if let Some(ref item) = state.refine_log.active_refine {
                        if item.status == RefineStatus::Pending && current_time >= item.finish_time {
                            let mut log = state.refine_log.clone();
                            let success = log.finish();
                            let _ = record.actor_ref.ask(SetRefineLog { refine_log: log }).await;
                            if success {
                                send_system_message(&self.gate_ref, *session_id, "精炼完成！物品已提升");
                            } else {
                                send_system_message(&self.gate_ref, *session_id, "精炼失败，物品已损毁");
                            }
                            debug!("AutoRefine: {} result={}", state.name, success);
                        }
                    }
                }
            }
        }
    }

    /// #1283：C# ProcessRegen——每 10s 自然回血量 = (max*3% + 1) + (该值 * Recovery / Weight)
    fn natural_regen_amount(max_value: i32, recovery: i32, weight: u32) -> i32 {
        let base = (max_value * 3 / 100) + 1;
        base + base * recovery / (weight.max(1) as i32)
    }

    /// HP/MP 回复 + 宠物饥饿 tick（每 100 ticks）
    pub(crate) async fn tick_regen_and_hunger(&mut self) {
        if self.tick_count.is_multiple_of(100) {
            debug!(
                "World tick #{} (online: {}, monsters: {})",
                self.tick_count, self.players.len(), self.monsters.len()
            );

            let now_unix = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);

            // #935：required_group 地图组队人数不足 → 循环后统一送回绑定点（避免循环内 &mut self 借用冲突）
            let mut required_leave: Vec<(u64, u16, i32, i32, u8)> = Vec::new();

            // 每 10 秒（100 ticks @ 100ms）回复 HP/MP
            for record in self.players.values() {
                // 宠物饥饿值
                let _ = record.actor_ref.ask(TickCreatureHunger { dt_seconds: 10 }).await;

                if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                    // #1283：C# ProcessRegen——每 RegenDelay=10s 回 (max*3%+1) + Recovery 加成
                    // （此前 ×10 导致 10 倍过量）；受击（Attacked 重置 RegenTime）后 10s 内不回血
                    let now_ms = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis() as i64)
                        .unwrap_or(0);
                    let regen_blocked = now_ms - state.last_damage_ms < 10_000;
                    let hp_regen = if regen_blocked {
                        0
                    } else {
                        Self::natural_regen_amount(
                            state.max_hp,
                            state.health_recovery,
                            self.health_regen_weight,
                        )
                    };
                    let mp_regen = if regen_blocked {
                        0
                    } else {
                        Self::natural_regen_amount(
                            state.max_mp,
                            state.spell_recovery,
                            self.mana_regen_weight,
                        )
                    };
                    let new_hp = (state.hp + hp_regen).min(state.max_hp);
                    let new_mp = (state.mp + mp_regen).min(state.max_mp);

                    if new_hp != state.hp || new_mp != state.mp {
                        // 发送 HealthChanged
                        let mut health_body = Vec::new();
                        health_body.extend_from_slice(&(new_hp as u32).to_le_bytes());
                        health_body.extend_from_slice(&(new_mp as u32).to_le_bytes());
                        let _ = self.gate_ref.tell(SendToClient {
                            session_id: state.session_id,
                            data: build_packet_bytes(
                                mir2_shared::enums::ServerPacketIds::HealthChanged as i16,
                                &health_body,
                            ),
                        }).await;
                    }

                    // #887：仓库扩容过期降级（C# PlayerObject.Process：过期 → HasExpandedStorage=false
                    // + 系统消息 ExpandedStorageExpired + ResizeStorage{Size, false, ExpiryTime}）
                    if state.has_expanded_storage
                        && state.expanded_storage_expiry_date > 0
                        && now_unix > state.expanded_storage_expiry_date
                    {
                        let mut new_state = state.clone();
                        new_state.has_expanded_storage = false;
                        new_state.expanded_storage_expiry_date = 0;
                        let _ = record.actor_ref.ask(SetPlayerState { state: new_state }).await;
                        send_system_message(&self.gate_ref, state.session_id, "仓库扩容已到期，仓库恢复为 80 格。");
                        let mut resize_body = Vec::new();
                        let resize = mir2_shared::packets::server::ui_events::ResizeStorage {
                            size: state.inventory.storage.len() as i32,
                            has_expanded_storage: false,
                            expiry_time: 0,
                        };
                        if resize.write_body(&mut resize_body).is_ok() {
                            let _ = self.gate_ref.tell(SendToClient {
                                session_id: state.session_id,
                                data: build_packet_bytes(
                                    mir2_shared::enums::ServerPacketIds::ResizeStorage as i16,
                                    &resize_body,
                                ),
                            }).await;
                        }
                        if let Err(e) = db::update_account_storage_expansion(
                            &self.db_pool,
                            &record.account_username,
                            false,
                            0,
                        ).await {
                            warn!("Failed to persist storage expansion expiry for {}: {}", record.name, e);
                        }
                    }

                    // #935：C# CheckGroupValidityOnMap——required_group 地图组队人数不足强制送回绑定点
                    if !state.is_gm {
                        if let Some(mi) = self.map_infos.get(&(state.map_index as i32)) {
                            if mi.required_group {
                                let required = 2.max(mi.required_group_size);
                                let have = self.group_member_count(state.session_id).await;
                                if (have as i32) < required {
                                    required_leave.push((
                                        state.session_id,
                                        state.bind_map_index.max(0) as u16,
                                        state.bind_x,
                                        state.bind_y,
                                        state.direction,
                                    ));
                                    send_system_message(
                                        &self.gate_ref,
                                        state.session_id,
                                        &format!("组队人数不足（需要至少 {} 人），已被送回安全地图", required),
                                    );
                                }
                            }
                        }
                    }
                }
            }
            // #935：处理强制离开（加载绑定地图 + 传送）
            for (sid, bind_map, bx, by, dir) in required_leave {
                if let Some(file) = self.map_infos.get(&(bind_map as i32)).map(|m| m.file_name.clone()) {
                    if self.get_or_load_map(&file, bind_map).is_some() {
                        if let Some(map_data) = self.maps.get(&bind_map).cloned() {
                            if let Some(record) = self.players.get(&sid) {
                                let _ = record.actor_ref.ask(SetMapData { map: map_data }).await;
                            }
                        }
                    }
                }
                if let Some(record) = self.players.get(&sid) {
                    let _ = record.actor_ref.ask(SetPlayerPosition {
                        x: bx,
                        y: by,
                        direction: dir,
                        map_index: Some(bind_map),
                        is_mounted: None,
                    }).await;
                }
            }
        }
    }

    /// #898：安全区回血（C# Settings.SafeZoneHealing：开启后安全区内每 2 秒 +25 HP，
    /// 等效 C# Map 加载时放置的永久 Healing SpellObject Value=25 / TickSpeed=2000ms）
    pub(crate) async fn tick_safe_zone_healing(&mut self) {
        if !self.safe_zone_healing {
            return;
        }
        if self.tick_count.is_multiple_of(20) {
            for (session_id, record) in &self.players {
                if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                    if state.is_dead || state.hp >= state.max_hp {
                        continue;
                    }
                    let in_safe = self.maps.get(&state.map_index)
                        .map(|m| m.is_safe_zone(state.x, state.y))
                        .unwrap_or(false);
                    if !in_safe {
                        continue;
                    }
                    let new_hp = safe_zone_heal_hp(state.hp, state.max_hp);
                    if new_hp != state.hp {
                        let mp = state.mp;
                        let mut new_state = state.clone();
                        new_state.hp = new_hp;
                        let _ = record.actor_ref.ask(SetPlayerState { state: new_state }).await;
                        let mut body = Vec::new();
                        body.extend_from_slice(&(new_hp as u32).to_le_bytes());
                        body.extend_from_slice(&(mp as u32).to_le_bytes());
                        let _ = self.gate_ref.tell(SendToClient {
                            session_id: *session_id,
                            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::HealthChanged as i16, &body),
                        }).await;
                    }
                }
            }
        }
    }

    /// #916：物品过期/租赁到期清理（C# HumanObject.ProcessItems：每 60s 扫描背包/装备/仓库）
    pub(crate) async fn tick_item_expiry(&mut self) {
        if !self.tick_count.is_multiple_of(600) {
            return;
        }
        let now_ticks = dotnet_now_ticks();
        let mut clear_ring_sync: Vec<u64> = Vec::new();
        for (session_id, record) in &self.players {
            if let Ok(Some(mut state)) = record.actor_ref.ask(GetPlayerState).await {
                let mut expired_backpack: Vec<usize> = Vec::new();
                let mut expired_equip: Vec<usize> = Vec::new();
                let mut expired_storage: Vec<usize> = Vec::new();
                let mut deleted_packets: Vec<(u64, u32)> = Vec::new();
                let mut any_change = false;

                // 背包
                for (i, slot) in state.inventory.backpack.iter_mut().enumerate() {
                    if let Some(s) = slot {
                        let mut remove = false;
                        if let Some(exp) = &s.item.expire_info {
                            if item_expired(exp.expiry_date_binary, now_ticks) {
                                remove = true;
                            }
                        }
                        if let Some(rental) = &mut s.item.rental_information {
                            if rental.rental_locked && item_expired(rental.expiry_date_binary, now_ticks) {
                                // C#：租赁锁定到期 → 清掉 RentalInformation（解锁）
                                s.item.rental_information = None;
                                any_change = true;
                            }
                        }
                        if remove {
                            expired_backpack.push(i);
                            deleted_packets.push((s.item.unique_id, s.item.count as u32));
                            any_change = true;
                        }
                    }
                }
                // 装备
                for (i, slot) in state.inventory.equipment.iter_mut().enumerate() {
                    if let Some(item) = slot {
                        let mut remove = false;
                        if let Some(exp) = &item.expire_info {
                            if item_expired(exp.expiry_date_binary, now_ticks) {
                                remove = true;
                            }
                        }
                        if let Some(rental) = &mut item.rental_information {
                            if rental.rental_locked && item_expired(rental.expiry_date_binary, now_ticks) {
                                item.rental_information = None;
                                any_change = true;
                            }
                        }
                        if remove {
                            expired_equip.push(i);
                            deleted_packets.push((item.unique_id, item.count as u32));
                            any_change = true;
                        }
                    }
                }
                // 仓库
                for (i, slot) in state.inventory.storage.iter_mut().enumerate() {
                    if let Some(s) = slot {
                        let mut remove = false;
                        if let Some(exp) = &s.item.expire_info {
                            if item_expired(exp.expiry_date_binary, now_ticks) {
                                remove = true;
                            }
                        }
                        if let Some(rental) = &mut s.item.rental_information {
                            if rental.rental_locked && item_expired(rental.expiry_date_binary, now_ticks) {
                                s.item.rental_information = None;
                                any_change = true;
                            }
                        }
                        if remove {
                            expired_storage.push(i);
                            deleted_packets.push((s.item.unique_id, s.item.count as u32));
                            any_change = true;
                        }
                    }
                }

                if !any_change {
                    continue;
                }
                for i in expired_backpack {
                    state.inventory.backpack[i] = None;
                }
                let equip_removed = !expired_equip.is_empty();
                for i in expired_equip {
                    state.inventory.equipment[i] = None;
                }
                for i in expired_storage {
                    state.inventory.storage[i] = None;
                }
                let _ = record.actor_ref.ask(SetPlayerState { state }).await;
                for (uid, count) in &deleted_packets {
                    let pkt = mir2_shared::packets::server::experience::DeleteItem { unique_id: *uid, count: *count };
                    let mut body = Vec::new();
                    if pkt.write_body(&mut body).is_ok() {
                        let _ = self.gate_ref.tell(SendToClient {
                            session_id: *session_id,
                            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::DeleteItem as i16, &body),
                        }).await;
                    }
                }
                if !deleted_packets.is_empty() {
                    send_system_message(&self.gate_ref, *session_id, "部分物品已过期并被移除。");
                }
                if equip_removed {
                    if let Some(st) = self.recalculate_and_set_stat_bonuses(*session_id).await {
                        self.broadcast_equipment_visuals(*session_id, &st).await;
                    }
                    // #1540：ClearRing 特殊模式隐身同步（过期移除后解除）
                    clear_ring_sync.push(*session_id);
                }
            }
        }

        // #1540：ClearRing 特殊模式隐身同步（装备过期移除后解除）
        for sid in clear_ring_sync {
            self.sync_clear_ring_visibility(sid).await;
        }
    }

    /// 昼夜循环（每 600 ticks）
    pub(crate) async fn tick_day_night(&mut self) {
        if self.tick_count.is_multiple_of(600) {
            let hour = chrono::Local::now().hour();
            let new_light = Self::light_for_hour(hour);
            if new_light != self.current_light {
                self.current_light = new_light;
                for session_id in self.players.keys() {
                    self.send_time_of_day(*session_id, new_light);
                }
                let light_name = match new_light {
                    mir2_shared::enums::LightSetting::Dawn => "黎明",
                    mir2_shared::enums::LightSetting::Day => "白天",
                    mir2_shared::enums::LightSetting::Evening => "黄昏",
                    mir2_shared::enums::LightSetting::Night => "夜晚",
                    _ => "正常",
                };
                info!("Time of day changed to {} (hour={})", light_name, hour);
            }
        }
    }

    /// 定期自动保存（每 300 ticks）
    pub(crate) async fn tick_auto_save(&mut self) {
        if self.tick_count % 300 == 0 && !self.players.is_empty() {
            let player_count = self.players.len();
            let mut saved = 0;
            for record in self.players.values() {
                if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                    if let Err(e) = db::save_character(&self.db_pool, &state, &record.account_username).await {
                        warn!("Auto-save failed for player {}: {}", record.name, e);
                    } else {
                        saved += 1;
                    }
                    // 英雄列表同步保存（避免崩溃丢失新建/变更的英雄；C# 英雄随角色一起持久化）
                    if let Some(heroes) = self.player_heroes.get(&record.session_id) {
                        let db_heroes: Vec<db::DbHero> = heroes.iter().map(|h| db::DbHero {
                            index: h.index,
                            name: h.name.clone(),
                            level: h.level,
                            class: h.class as u8,
                            gender: h.gender as u8,
                            dead: h.dead,
                            sealed: h.sealed,
                        }).collect();
                        if let Err(e) = db::save_heroes(&self.db_pool, &record.name, &db_heroes).await {
                            warn!("Auto-save heroes failed for {}: {}", record.name, e);
                        }
                    }
                }
            }
            info!("Auto-saved {} players to database ({} online)", saved, player_count);
        }
    }

    /// 拍卖过期清理（每 36000 ticks = 1小时）
    pub(crate) async fn tick_auction_expiry(&mut self) {
        if self.tick_count % 36000 == 0 {
            let now = chrono::Local::now().timestamp();
            let seven_days = 7 * 24 * 60 * 60;
            let mut expired = Vec::new();
            for (idx, auction) in self.auctions.iter().enumerate() {
                if !auction.sold && (now - auction.consignment_date) > seven_days {
                    expired.push(idx);
                }
            }
            for idx in expired.into_iter().rev() {
                let auction = self.auctions.remove(idx);
                let _ = db::delete_auction(&self.db_pool, auction.auction_id as i64).await;

                // Return item to seller
                let mut seller_online = false;
                for (_, record) in &self.players {
                    if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                        if state.name == auction.seller_name {
                            let added = record.actor_ref.ask(AddItemToInventory { item: auction.item.clone() }).await.unwrap_or(false);
                            if added {
                                send_system_message(&self.gate_ref, record.session_id, "寄售物品已过期，已退回背包");
                            } else {
                                send_system_message(&self.gate_ref, record.session_id, "寄售物品已过期，背包已满，物品已通过邮件退回");
                                send_item_via_mail(&self.db_pool, &auction.seller_name, auction.item.clone(), "寄售物品退回", "寄售物品已过期，背包已满");
                            }
                            seller_online = true;
                            break;
                        }
                    }
                }
                if !seller_online {
                    // Seller offline — send item via mail
                    send_item_via_mail(&self.db_pool, &auction.seller_name, auction.item.clone(), "寄售物品退回", "寄售物品已过期");
                }
                debug!("Auction {} expired and removed", auction.auction_id);
            }
        }
    }

    /// 租赁过期处理（每 3600 ticks = 6分钟检查一次）
    pub(crate) async fn tick_rental_expiry(&mut self) {
        if self.tick_count % 3600 == 0 {
            let now = chrono::Local::now().timestamp();
            let mut expired_renters: Vec<String> = Vec::new();

            for (renter_name, rentals) in &mut self.player_rentals {
                let mut still_valid: Vec<RentedItem> = Vec::new();
                for rental in rentals.drain(..) {
                    if rental.expiry_timestamp > now {
                        still_valid.push(rental);
                        continue;
                    }
                    // Rental expired - try to remove from renter and return to owner
                    let mut returned = false;
                    for (_, record) in &self.players {
                        if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                            if state.name == *renter_name {
                                // Try to remove item from renter
                                let removed = record.actor_ref.ask(RemoveItemFromInventory {
                                    unique_id: rental.item.unique_id,
                                }).await.ok().flatten();
                                if removed.is_some() {
                                    // Return to owner if online
                                    for (_, owner_record) in &self.players {
                                        if let Ok(Some(owner_state)) = owner_record.actor_ref.ask(GetPlayerState).await {
                                            if owner_state.name == rental.owner_name {
                                                let added = owner_record.actor_ref.ask(AddItemToInventory {
                                                    item: rental.item.clone(),
                                                }).await.unwrap_or(false);
                                                if added {
                                                    send_system_message(&self.gate_ref, owner_record.session_id,
                                                        &format!("租赁物品 {} 已到期收回", rental.item.item_index));
                                                }
                                                break;
                                            }
                                        }
                                    }
                                    send_system_message(&self.gate_ref, record.session_id,
                                        &format!("租赁物品 {} 已到期，已归还给 {}", rental.item.item_index, rental.owner_name));
                                    returned = true;
                                } else {
                                    send_system_message(&self.gate_ref, record.session_id,
                                        &format!("租赁物品 {} 已到期，但物品不在背包中", rental.item.item_index));
                                }
                                break;
                            }
                        }
                    }
                    if !returned {
                        // Renter offline or item not in inventory — return to owner via online or mail
                        let mut owner_online = false;
                        for (_, owner_record) in &self.players {
                            if let Ok(Some(owner_state)) = owner_record.actor_ref.ask(GetPlayerState).await {
                                if owner_state.name == rental.owner_name {
                                    let added = owner_record.actor_ref.ask(AddItemToInventory {
                                        item: rental.item.clone(),
                                    }).await.unwrap_or(false);
                                    if added {
                                        send_system_message(&self.gate_ref, owner_record.session_id,
                                            &format!("租赁物品 {} 已到期收回", rental.item.item_index));
                                    } else {
                                        send_system_message(&self.gate_ref, owner_record.session_id,
                                            &format!("租赁物品 {} 已到期，背包已满，已通过邮件退回", rental.item.item_index));
                                        send_item_via_mail(&self.db_pool, &rental.owner_name, rental.item.clone(),
                                            "租赁物品退回", &format!("租赁物品 {} 已到期", rental.item.item_index));
                                    }
                                    owner_online = true;
                                    break;
                                }
                            }
                        }
                        if !owner_online {
                            send_item_via_mail(&self.db_pool, &rental.owner_name, rental.item.clone(),
                                "租赁物品退回", &format!("租赁物品 {} 已到期", rental.item.item_index));
                        }
                    }
                    debug!("Rental expired: {} -> {} item={}", rental.owner_name, renter_name, rental.item.item_index);
                }
                if still_valid.is_empty() {
                    expired_renters.push(renter_name.clone());
                } else {
                    *rentals = still_valid;
                }
            }
            for name in expired_renters {
                self.player_rentals.remove(&name);
            }
        }
    }

    pub(crate) async fn tick_dragon(&mut self) {
        use crate::actors::world::dragon::DragonState;

        // 先决定是否需要降级检查（无借用冲突）
        if let Some(ref mut dragon) = self.dragon_state {
            crate::actors::world::dragon::tick_dragon_delevel(
                dragon, self.tick_count, &self.gate_ref,
            ).await;
        }

        // C# EvilMir.ChangeHP：DragonLink 且受击（amount<0）→ DragonSystem.GainExp(Random(1,40))
        if let Some(dragon) = self.dragon_state.as_mut() {
            if let Some(oid) = dragon.evil_mir_oid {
                if let Some(m) = self.monsters.get(&oid) {
                    let prev = dragon.last_evil_mir_hp;
                    if prev > m.hp && m.hp > 0 {
                        let exp = fastrand::i32(1..40) as u64;
                        let levels = dragon.gain_exp(exp);
                        debug!("Dragon exp +{} from EvilMir hit (levels gained: {})", exp, levels);
                    }
                    dragon.last_evil_mir_hp = m.hp;
                }
            }
        }

        // Dragon 系统：根据 dragon_info 配置在龙地图上有玩家时生成/维持 EvilMir 作为世界Boss。
        // 简化：当 dragon_info 存在、玩家在龙地图上、且当前无活跃 EvilMir → 生成。
        let dragon_info = match self.dragon_info.clone() {
            Some(di) => di,
            None => return,
        };
        // 确保 dragon_state 存在（懒初始化，body_object_id 占位）
        if self.dragon_state.is_none() {
            self.dragon_state = Some(DragonState::new(0));
        }

        // 解析龙地图索引（按 map_file_name 查 map_infos）
        let dragon_map_index: Option<i32> = self.map_infos.values()
            .find(|m| m.file_name.eq_ignore_ascii_case(&dragon_info.map_file_name))
            .map(|m| m.index);

        // 收集龙地图上的玩家 session_id
        let mut dragon_map_sessions: Vec<u64> = Vec::new();
        if let Some(map_idx) = dragon_map_index {
            for (sid, record) in &self.players {
                if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                    if state.map_index as i32 == map_idx && !state.is_dead {
                        dragon_map_sessions.push(*sid);
                    }
                }
            }
        }

        // 节流：每 10 ticks 检查一次（~1秒），避免每个 tick 都查
        if self.tick_count % 10 != 0 {
            return;
        }

        // 读取 dragon 状态快照（短作用域，避免长时间持有 dragon_state 的可变借用）
        let (dragon_level, evil_mir_alive) = {
            let dragon_state = self.dragon_state.as_mut().unwrap();
            dragon_state.last_spawn_check = self.tick_count;
            // 当前 EvilMir 是否还活着？查 monsters
            let alive = dragon_state.evil_mir_oid
                .map(|oid| self.monsters.contains_key(&oid))
                .unwrap_or(false);
            if !alive {
                dragon_state.evil_mir_oid = None;
            }
            (dragon_state.level, alive)
        };

        // 有玩家在龙地图、且无活跃 EvilMir → 生成
        if dragon_map_sessions.is_empty() { return; }
        if evil_mir_alive { return; }

        // 解析 monster_info：优先用预解析的 monster_index，否则按名称查
        let monster_info = if let Some(i) = dragon_info.monster_index {
            self.monster_infos.get(&i).cloned()
        } else {
            self.monster_infos.values()
                .find(|m| m.name.eq_ignore_ascii_case(&dragon_info.monster_name))
                .cloned()
        };
        let monster_info = match monster_info {
            Some(mi) => mi,
            None => return,
        };
        let monster_index = monster_info.index;

        // 生成 EvilMir 作为世界Boss（使用 dragon_info 的 location）
        let spawn_oid = self.alloc_object_id();
        let level_mul = 1.0 + (dragon_level as f32 - 1.0) * 0.15; // 等级越高 Boss 越强
        let base_hp = monster_info.stats.get(&(mir2_shared::enums::Stat::HP as u8)).copied().unwrap_or(5000);
        let boss_hp = (base_hp as f32 * 10.0 * level_mul) as i32;
        let base_min_dmg = monster_info.stats.get(&(mir2_shared::enums::Stat::MinDC as u8)).copied().unwrap_or(20);
        let base_max_dmg = monster_info.stats.get(&(mir2_shared::enums::Stat::MaxDC as u8)).copied().unwrap_or(40);
        let boss_min_dmg = (base_min_dmg as f32 * 3.0 * level_mul) as i32;
        let boss_max_dmg = (base_max_dmg as f32 * 3.0 * level_mul) as i32;
        let boss_xp = (monster_info.experience as f32 * 5.0 * level_mul) as i32;
        let dragon_map_u16 = dragon_map_index.unwrap_or(0) as u16;

        let boss = MonsterState {
            object_id: spawn_oid,
            name: format!("[龙] {}", monster_info.name),
            image: monster_info.image as u16,
            monster_index,
            x: dragon_info.location_x,
            y: dragon_info.location_y,
            direction: 0,
            hp: boss_hp,
            max_hp: boss_hp,
            min_dmg: boss_min_dmg,
            max_dmg: boss_max_dmg,
            xp: boss_xp,
            spawn_x: dragon_info.location_x,
            spawn_y: dragon_info.location_y,
            spawn_spread: 0,
            map_index: dragon_map_u16,
            next_attack_tick: 0,
            next_move_tick: 0,
            next_summon_tick: 0,
            ai_profile: MonsterAiProfile::from_info(&monster_info),
            ai_state: MonsterAiState::Idle,
            sitting: false,
            hidden: false,
            sit_down_tick: 0,
            target_session: None,
            last_hitter_session: None,
            provoked: true,
            is_elite: false,
            is_boss: true,
            min_ac: 0, max_ac: 0,
            min_mac: 0, max_mac: 0,
            agility: 0, accuracy: 0,
            armour_rate: 1.0, damage_rate: 1.0,
            magic_resist: 0,
            critical_rate: 0, critical_damage: 0,
            luck: 0, reflect: 0, level: monster_info.level, effect: monster_info.effect,
            damage_reduction_percent: 0,
            poison_list: Vec::new(),
            last_hit_damage: 0,
            undead: false,
            master_session: None,
                                rarity: 0,
                                pet_experience: 0,
                                max_pet_level: 0,
            recall_at_tick: 0,
            behavior: ai::make_behavior(&monster_info.name),
        };
        self.monsters.insert(spawn_oid, boss);
        // 标记为世界Boss超时（1小时 = 36000 ticks 无挑战则消失）
        self.world_boss_queue.insert(spawn_oid, self.tick_count + 36000);
        self.dragon_state.as_mut().unwrap().evil_mir_oid = Some(spawn_oid);

        // 广播生成
        let packet = build_object_monster_packet(
            &MonsterSpawn {
                name: format!("[龙] {}", monster_info.name),
                image: monster_info.image as u16,
                monster_index,
                x: dragon_info.location_x,
                y: dragon_info.location_y,
                direction: 0,
                hp: boss_hp,
                min_dmg: boss_min_dmg,
                max_dmg: boss_max_dmg,
                xp: boss_xp,
                map_index: dragon_map_u16,
                count: 1,
                spread: 0,
            }, spawn_oid, &format!("[龙] {}", monster_info.name),
        );
        for session_id in self.players.keys() {
            let _ = self.gate_ref.tell(SendToClient {
                session_id: *session_id,
                data: packet.clone(),
            }).await;
        }
        let map_title = self.map_infos.get(&dragon_map_index.unwrap_or(0))
            .map(|m| m.title.clone())
            .unwrap_or_else(|| dragon_info.map_file_name.clone());
        broadcast_system_message(&self.gate_ref, &self.players,
            &format!("【龙系统】 EvilMir（{}级）降临 {}！龙之试炼开始！",
                dragon_level, map_title));
        info!("Dragon spawned EvilMir #{} (level={}, map={}) at ({},{})",
            spawn_oid, dragon_level,
            dragon_info.map_file_name, dragon_info.location_x, dragon_info.location_y);
    }

    /// 行会战到期检查（C# Envir.cs 2317-2327：TimeRemaining 递减，到期 EndWar）
    pub(crate) async fn tick_guild_wars(&mut self) {
        if self.tick_count % 30 != 0 {
            return; // 每 3 秒检查
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let mut expired: Vec<(String, String)> = Vec::new();
        for ((a, b), end) in &self.guild_war_ends {
            if now >= *end {
                expired.push((a.clone(), b.clone()));
            }
        }
        for (a, b) in expired {
            self.guild_war_ends.remove(&(a.clone(), b.clone()));
            if let Some(set) = self.guild_wars.get_mut(&a) {
                set.remove(&b);
            }
            if let Some(set) = self.guild_wars.get_mut(&b) {
                set.remove(&a);
            }
            // C# EndWar：通知双方行会
            for (sid, rec) in &self.players {
                if let Ok(Some(s)) = rec.actor_ref.ask(GetPlayerState).await {
                    if s.guild_name.as_deref() == Some(a.as_str())
                        || s.guild_name.as_deref() == Some(b.as_str())
                    {
                        send_system_message(&self.gate_ref, *sid,
                            &format!("行会 {} 与 {} 的战争已结束！", a, b));
                    }
                }
            }
            // #1340：C# EndWar → UpdatePlayersColours：停战即时刷新双方成员名字颜色
            self.refresh_guild_war_colours(&a, &b).await;
            debug!("Guild war expired: {} vs {}", a, b);
        }
    }

    pub(crate) async fn tick_conquest(&mut self) {
        // 1) 战争调度：开始/结束（每 tick 检查时间）
        //    先收集需要广播的消息，避免在借用 instance 时借用 gate_ref
        let mut messages: Vec<String> = Vec::new();

        for instance in &mut self.conquest_instances {
            let now = chrono::Local::now().naive_local();
            if instance.should_start_war(&now) {
                // 开战时重建 siege_structure_ids 关联（按 conquest_id 匹配）
                instance.siege_structure_ids = self.siege_structures.values()
                    .filter(|s| s.conquest_id == instance.id)
                    .map(|s| s.object_id)
                    .collect();
                // 重置结构 HP
                for oid in &instance.siege_structure_ids {
                    if let Some(s) = self.siege_structures.get_mut(oid) {
                        s.hp = s.max_hp;
                        s.damage_level = 0;
                    }
                }
                instance.start_war("攻击方");
                messages.push(format!("⚔️ 攻城战开始了！目标：区域 #{}", instance.id));
            }
            if instance.state == conquest::WarState::InProgress {
                let elapsed = chrono::Utc::now().timestamp() - instance.war_start_time;
                if elapsed >= instance.war_duration_secs {
                    // 战争结束：根据模式判定胜者
                    let winner = match instance.game {
                        conquest::ConquestGame::ControlPoints => {
                            // 控制点最多者为胜
                            let tally = instance.tally_control_points();
                            tally.into_iter()
                                .max_by_key(|(_, c)| *c)
                                .and_then(|(g, c)| if c > 0 { Some(g) } else { None })
                        }
                        _ => instance.end_war(),
                    };
                    // 结束战争状态（end_war 已在 _ 分支调用；控制点分支需要手动重置）
                    if instance.game == conquest::ConquestGame::ControlPoints {
                        instance.state = conquest::WarState::Ended;
                        if let Some(ref g) = winner {
                            instance.owner_guild = Some(g.clone());
                        }
                        instance.attacker_guild = None;
                    }
                    if let Some(ref g) = winner {
                        messages.push(format!("🏰 攻城战结束！{} 取得了区域 #{} 的控制权！", g, instance.id));
                    } else {
                        messages.push(format!("🏰 攻城战结束！区域 #{} 无人占领", instance.id));
                    }
                }
            }
        }
        for msg in &messages {
            broadcast_system_message(&self.gate_ref, &self.players, msg);
        }

        // 2) 攻城战斗：攻城器（Catapult）每 tick 对最近城墙/城门造成伤害。
        //    收集伤害事件（攻城器 object_id, 目标 object_id），统一应用避免借用冲突。
        //    只在战争进行中执行。
        let active_conquest_ids: Vec<i32> = self.conquest_instances.iter()
            .filter(|i| i.state == conquest::WarState::InProgress)
            .map(|i| i.id)
            .collect();
        if active_conquest_ids.is_empty() {
            return;
        }

        // 收集每个活跃区域的攻城器和它们的候选目标
        // (catapult_oid, target_oid)
        let mut siege_attacks: Vec<(u32, u32)> = Vec::new();
        {
            for cid in &active_conquest_ids {
                // 收集本区域的攻城器和城墙/城门 id
                let mut catapult_ids: Vec<u32> = Vec::new();
                let mut wall_ids: Vec<u32> = Vec::new();
                for (oid, s) in &self.siege_structures {
                    if s.conquest_id != *cid { continue; }
                    match s.structure_type {
                        conquest::SiegeStructureType::Catapult => catapult_ids.push(*oid),
                        conquest::SiegeStructureType::Wall | conquest::SiegeStructureType::CastleGate => {
                            if !s.is_destroyed() { wall_ids.push(*oid); }
                        }
                        _ => {}
                    }
                }
                if catapult_ids.is_empty() || wall_ids.is_empty() { continue; }
                for cat_oid in &catapult_ids {
                    let catapult = match self.siege_structures.get(cat_oid) {
                        Some(s) => s,
                        None => continue,
                    };
                    if catapult.is_destroyed() { continue; }
                    // 攻击间隔节流：用 damage_level + object_id 模拟冷却。
                    // 简化：每 CATAPULT_ATTACK_INTERVAL ticks 攻击一次。
                    if (self.tick_count + (*cat_oid as u64)) % conquest::CATAPULT_ATTACK_INTERVAL != 0 {
                        continue;
                    }
                    let target_oid = match conquest::find_nearest_target(
                        catapult.x, catapult.y, *cid, &self.siege_structures, &wall_ids,
                    ) {
                        Some(t) => t,
                        None => continue,
                    };
                    siege_attacks.push((*cat_oid, target_oid));
                }
            }
        }

        // 应用伤害 + 收集被摧毁事件
        let mut destroyed_events: Vec<(u32, String)> = Vec::new(); // (object_id, type_name)
        for (cat_oid, target_oid) in siege_attacks {
            if let Some(target) = self.siege_structures.get_mut(&target_oid) {
                let destroyed = target.take_damage(conquest::CATAPULT_DAMAGE_PER_HIT);
                if destroyed {
                    let type_name = match target.structure_type {
                        conquest::SiegeStructureType::Wall => "城墙",
                        conquest::SiegeStructureType::CastleGate => "城门",
                        _ => "防御设施",
                    };
                    destroyed_events.push((target_oid, type_name.to_string()));
                }
            }
            let _ = cat_oid; // 攻城器本身不损耗（简化）
        }
        for (oid, type_name) in &destroyed_events {
            broadcast_system_message(&self.gate_ref, &self.players,
                &format!("💥 区域内的{}（#{}）被攻城器摧毁！进攻方可以长驱直入！", type_name, oid));
            debug!("Conquest siege structure #{} ({}) destroyed", oid, type_name);
        }

        // 4) 箭塔自动攻击（C# ConquestArcher：战争期间攻击非守方玩家；每 30 ticks = 3s 一轮）
        if self.tick_count % 30 == 0 {
            // 预收集玩家 (session, map, x, y, guild)
            let player_snaps: Vec<(u64, u16, i32, i32, Option<String>)> = {
                let mut out = Vec::new();
                for (_sid, record) in &self.players {
                    if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                        if !state.is_dead {
                            out.push((state.session_id, state.map_index, state.x, state.y, state.guild_name.clone()));
                        }
                    }
                }
                out
            };
            let mut archer_attacks: Vec<(u32, u64, i32)> = Vec::new();
            for cid in &active_conquest_ids {
                let owner = self.conquest_instances.iter()
                    .find(|c| c.id == *cid)
                    .and_then(|c| c.owner_guild.clone());
                let conquest_map = self.conquest_instances.iter()
                    .find(|c| c.id == *cid)
                    .map(|c| c.map_index as u16)
                    .unwrap_or(u16::MAX);
                for (oid, s) in &self.siege_structures {
                    if s.conquest_id != *cid
                        || s.structure_type != conquest::SiegeStructureType::ArcherTower
                        || s.is_destroyed()
                    {
                        continue;
                    }
                    // #1513：有守卫坐标（s.x/s.y 非 0）时按 ARCHER_RANGE 过滤；未配置回退全地图
                    let has_coords = s.x != 0 || s.y != 0;
                    for (sid, map, px, py, guild) in &player_snaps {
                        if *map != conquest_map { continue; }
                        if has_coords && ((*px - s.x).abs() > ARCHER_RANGE || (*py - s.y).abs() > ARCHER_RANGE) {
                            continue;
                        }
                        // C# FindTarget：跳过守方（owner 行会）玩家
                        if let (Some(owner), Some(g)) = (&owner, guild) {
                            if g == owner { continue; }
                        }
                        archer_attacks.push((*oid, *sid, ARCHER_DAMAGE));
                        break;
                    }
                }
            }
            for (oid, sid, damage) in archer_attacks {
                if let Some(record) = self.players.get(&sid) {
                    let _ = record.actor_ref.ask(crate::actors::player::TakeDamage {
                        attacker_id: oid,
                        attacker_session: 0,
                        damage,
                    }).await;
                }
            }
        }

        // 3) 控制点占领判定（ControlPoints 模式）：每 60 ticks 检查玩家位置
        if self.tick_count % 60 != 0 { return; }

        // 预收集玩家 (session, x, y, guild_name, map_index)
        let player_snaps: Vec<(u64, i32, i32, Option<String>, i32)> = {
            let mut out = Vec::new();
            for (_sid, record) in &self.players {
                if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                    if !state.is_dead {
                        out.push((state.session_id, state.x, state.y, state.guild_name.clone(), state.map_index as i32));
                    }
                }
            }
            out
        };

        // 确保每个 instance 的 control_point_owners 与 control_points 等长
        for instance in &mut self.conquest_instances {
            if instance.control_point_owners.len() != instance.control_points.len() {
                instance.control_point_owners.resize(instance.control_points.len(), conquest::ControlPointState::default());
            }
        }

        // 对每个活跃的控制点模式区域执行占领判定
        for instance in &mut self.conquest_instances {
            if instance.state != conquest::WarState::InProgress { continue; }
            if instance.game != conquest::ConquestGame::ControlPoints { continue; }
            if instance.control_points.is_empty() { continue; }

            // 快照控制点坐标 + 地图索引，避免后续借用 instance
            let cps: Vec<(i32, i32, i32)> = instance.control_points.clone();
            let map_index = instance.map_index;

            // 收集每个 idx 的占领结果，循环结束后统一应用 add_score
            let mut newly_captured: Vec<String> = Vec::new();
            for (idx, (cx, cy, r)) in cps.iter().enumerate() {
                let cp_owner = &mut instance.control_point_owners[idx];
                // 找在本区域地图上、站在该控制点范围内的公会
                let mut guilds_here: Vec<String> = Vec::new();
                for (_sid, px, py, guild, pmidx) in &player_snaps {
                    if *pmidx != map_index { continue; }
                    if let Some(ref g) = guild {
                        let on_cp = (px - cx).abs() <= *r && (py - cy).abs() <= *r;
                        if on_cp && !guilds_here.contains(g) {
                            guilds_here.push(g.clone());
                        }
                    }
                }

                if guilds_here.is_empty() {
                    // 无人站点：进度缓慢回落
                    if cp_owner.progress > 0 { cp_owner.progress -= 1; }
                    cp_owner.contesting_guild = None;
                    continue;
                }

                // 仅一个公会站点 → 增加其占领进度
                if guilds_here.len() == 1 {
                    let g = &guilds_here[0];
                    cp_owner.contesting_guild = Some(g.clone());
                    if cp_owner.owner_guild.as_deref() == Some(g.as_str()) {
                        // 已是拥有者，维持
                    } else {
                        cp_owner.progress += 1;
                        if cp_owner.progress >= conquest::MAX_CONTROL_POINTS {
                            cp_owner.owner_guild = Some(g.clone());
                            cp_owner.progress = 0;
                            newly_captured.push(g.clone());
                        }
                    }
                } else {
                    // 多公会争夺：进度互相抵消，无人增长
                    cp_owner.contesting_guild = None;
                    if cp_owner.progress > 0 { cp_owner.progress -= 1; }
                }
            }
            // 统一应用积分（避免在借用 control_point_owners 时可变借用 scores）
            for g in newly_captured {
                instance.add_score(&g, 1);
            }
        }
    }

    pub(crate) async fn tick_robots(&mut self) {
        let now = chrono::Local::now().naive_local();
        let current_minute = now.minute();
        if self.robot_tasks.is_empty() || current_minute == self.robot_last_check_minute {
            return;
        }
        self.robot_last_check_minute = current_minute;
        let mut task_indices: Vec<usize> = vec![];
        for (i, task) in self.robot_tasks.iter().enumerate() {
            if task.should_fire(&now) {
                task_indices.push(i);
            }
        }
        for idx in &task_indices {
            let page = self.robot_tasks[*idx].page.clone();
            self.robot_tasks[*idx].mark_fired(&now);
            let msg = format!("[机器人] 定时事件触发: {}", page);
            broadcast_system_message(&self.gate_ref, &self.players, &msg);
        }
    }

    pub(crate) async fn tick_spells(&mut self) {
        use mir2_shared::enums::{Spell, PoisonType};
        use crate::actors::player::{GetPlayerState, Heal};
        use crate::combat::{attack, poison};

        let now = std::time::Instant::now();
        let mut expired_ids = Vec::new();
        // 收集需要结算的 spell tick：(caster_session, spell, x, y, tick_value, 命中怪物 ids)
        let mut spell_hits: Vec<(u64, Spell, i32, i32, i32, Vec<u32>)> = Vec::new();
        let mut heal_targets: Vec<u64> = Vec::new();
        let mut heal_amounts: Vec<i32> = Vec::new();

        // 第一阶段：遍历 spell_objects，更新 tick 时间，收集命中怪物 id
        for (obj_id, spell_obj) in &mut self.spell_objects {
            let elapsed = now.duration_since(spell_obj.created_at).as_millis() as u64;
            if spell_obj.is_expired(elapsed) && spell_obj.spell != Spell::DelayedExplosion {
                expired_ids.push(*obj_id);
                continue;
            }
            let since_last = now.duration_since(spell_obj.last_tick).as_millis() as u64;
            if since_last < spell_obj.tick_interval_ms {
                continue;
            }
            spell_obj.last_tick = now;

            match spell_obj.spell {
                Spell::FireWall | Spell::Blizzard | Spell::MeteorStrike | Spell::PoisonCloud => {
                    // 持久伤害法术：命中 spell 位置 ±1 的怪物（C# SpellObject.ProcessSpell 按单格）
                    let caster_freezing = if spell_obj.spell == Spell::Blizzard {
                        if let Some(record) = self.players.get(&spell_obj.caster_session) {
                            if let Ok(Some(cs)) = record.actor_ref.ask(GetPlayerState).await {
                                Some(cs.freezing)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    let hit_ids: Vec<u32> = self.monsters.iter()
                        .filter(|(_, m)| {
                            let dist = (m.x - spell_obj.x).abs() + (m.y - spell_obj.y).abs();
                            dist <= 1 && m.hp > 0 && m.map_index == spell_obj.map_index
                        })
                        .map(|(id, _)| *id)
                        .collect();
                    if !hit_ids.is_empty() {
                        spell_hits.push((
                            spell_obj.caster_session,
                            spell_obj.spell,
                            spell_obj.x,
                            spell_obj.y,
                            spell_obj.tick_value,
                            hit_ids.clone(),
                        ));
                        // C# SpellObject.ProcessSpell 附加效果
                        for mid in &hit_ids {
                            if let Some(monster) = self.monsters.get_mut(mid) {
                                if spell_obj.spell == Spell::PoisonCloud {
                                    // 绿毒 12s，Value = (MinSC+MaxSC)/2 + BonusDmg ≈ magic_stat
                                    crate::combat::poison::apply_poison(&mut monster.poison_list,
                                        crate::combat::poison::Poison::new(
                                            mir2_shared::enums::PoisonType::GREEN, 12,
                                            spell_obj.bonus.max(1), spell_obj.tick_interval_ms,
                                        ));
                                }
                                if spell_obj.spell == Spell::Blizzard && fastrand::i32(0..8) == 0 {
                                    // Slow：5 + Random(Freezing) 秒，TickSpeed 2000
                                    let freeze = caster_freezing.unwrap_or(0).max(1);
                                    crate::combat::poison::apply_poison(&mut monster.poison_list,
                                        crate::combat::poison::Poison::new(
                                            mir2_shared::enums::PoisonType::SLOW,
                                            (5 + fastrand::i32(0..freeze)) as u32, 0, 2000,
                                        ));
                                }
                            }
                        }
                    }
                }
                Spell::HealingCircle => {
                    if let Some(record) = self.players.get(&spell_obj.caster_session) {
                        if let Ok(Some(_cs)) = record.actor_ref.ask(GetPlayerState).await {
                            for (sid, other) in &self.players {
                                if let Ok(Some(s)) = other.actor_ref.ask(GetPlayerState).await {
                                    let dist = (s.x - spell_obj.x).abs() + (s.y - spell_obj.y).abs();
                                    // #1638：治疗圈只作用同图玩家（C# CurrentMap）
                                    if dist <= 2 && s.map_index == spell_obj.map_index && !heal_targets.contains(sid) {
                                        heal_targets.push(*sid);
                                        heal_amounts.push(spell_obj.tick_value.max(25));
                                    }
                                }
                            }
                        }
                    }
                }
                Spell::ExplosiveTrap => {
                    if !spell_obj.detonated {
                        // C# ExplosiveTrap：可攻击目标踩中该格才引爆（单目标 MAC）
                        let stepped: Option<u32> = self.monsters.iter()
                            .find(|(_, m)| m.x == spell_obj.x && m.y == spell_obj.y && m.hp > 0 && m.map_index == spell_obj.map_index)
                            .map(|(id, _)| *id);
                        if let Some(mid) = stepped {
                            debug!("SpellObject: ExplosiveTrap detonated at ({},{}) on monster {}", spell_obj.x, spell_obj.y, mid);
                            spell_obj.detonated = true;
                            spell_hits.push((
                                spell_obj.caster_session, spell_obj.spell,
                                spell_obj.x, spell_obj.y, spell_obj.tick_value, vec![mid],
                            ));
                            expired_ids.push(*obj_id);
                        }
                    }
                }
                Spell::DelayedExplosion => {
                    // C# HumanObject.cs:6462 DelayedType.Magic：延迟到期 → 对目标一次 MAC 伤害
                    // + 挂 DelayedExplosion 毒（目标身上三级 ObjectEffect 后 3×3 AoE）。
                    if !spell_obj.detonated && elapsed >= spell_obj.expires_at_ms {
                        debug!("SpellObject: DelayedExplosion impact on target {:?} at ({},{})",
                               spell_obj.target_id, spell_obj.x, spell_obj.y);
                        spell_obj.detonated = true;
                        let target_info = spell_obj.target_id
                            .and_then(|tid| self.monsters.get(&tid))
                            .map(|t| (t.object_id, t.x, t.y,
                                t.hp > 0 && t.map_index == spell_obj.map_index,
                                t.poison_list.iter().any(|p| p.p_type == PoisonType::DELAYED_EXPLOSION)));
                        if let Some((tid, tx, ty, alive, already_poisoned)) = target_info {
                            if alive {
                                // 1) 立即 MAC 伤害（spell_hits 第二阶段走 resolve_attack）
                                spell_hits.push((
                                    spell_obj.caster_session, spell_obj.spell,
                                    tx, ty, spell_obj.tick_value, vec![tid],
                                ));
                                // 2) 挂 DelayedExplosion 毒；已有同类型毒则不重复（C# ApplyPoison 直接 return）
                                if !already_poisoned {
                                    if let Some(t) = self.monsters.get_mut(&tid) {
                                        let mut p = poison::Poison::new(
                                            PoisonType::DELAYED_EXPLOSION,
                                            30, spell_obj.tick_value, 2000,
                                        );
                                        p.owner_session = spell_obj.caster_session;
                                        // delayed_stage/delayed_next_tick 保持 0：首次 %20 推进时
                                        // 进入阶段 1 并设置 +30 ticks（3s）的引爆窗口
                                        poison::apply_poison(&mut t.poison_list, p);
                                    }
                                }
                            }
                        }
                        expired_ids.push(*obj_id);
                    }
                }
                _ => {}
            }
        }

        // 第二阶段：对每个命中的怪物走战斗公式（MAC 防御 + 暴击 + 附加状态）
        // 按施法者分组缓存 CombatStats，减少 GetPlayerState 调用
        // #1452：缓存 (CombatStats, 等级) 用于 LevelOffset 等级差
        let mut caster_cache: std::collections::HashMap<u64, (crate::combat::attack::CombatStats, u16)> = std::collections::HashMap::new();
        for (caster_session, spell, _sx, _sy, tick_value, hit_ids) in spell_hits {
            // 获取施法者 CombatStats
            let (attacker_stats, caster_level) = if let Some((cs, lv)) = caster_cache.get(&caster_session) {
                (*cs, *lv)
            } else {
                let (stats, level) = match self.players.get(&caster_session) {
                    Some(r) => match r.actor_ref.ask(GetPlayerState).await {
                        Ok(Some(s)) if !s.is_dead => (s.to_combat_stats(), s.level),
                        _ => continue, // 施法者离线/死亡，跳过本次 tick
                    },
                    None => continue,
                };
                caster_cache.insert(caster_session, (stats, level));
                (stats, level)
            };

            for mid in hit_ids {
                if let Some(monster) = self.monsters.get_mut(&mid) {
                    let defender_stats = monster.to_combat_stats();
                    // #1452：C# LevelOffset = Level > attacker.Level ? 0 : min(10, attacker.Level - Level)
                    let level_offset = crate::combat::attack::level_offset(caster_level, monster.level.max(0) as u16);
                    let raw_damage = tick_value.max(1);
                    let r = attack::resolve_attack(
                        &attacker_stats, &defender_stats, raw_damage,
                        mir2_shared::enums::DefenceType::Mac, level_offset,
                    );
                    if r.is_hit && r.damage > 0 {
                        monster.take_damage(r.damage);
                        monster.last_hitter_session = Some(caster_session);
                        self.pending_gather.push(caster_session);
                        monster.provoked = true;
                        monster.target_session = Some(caster_session);

                        // 各法术附加状态（对齐 C# SpellObject.ProcessSpell）
                        match spell {
                            // Blizzard：1/8 概率 Slow（C# SpellObject.cs:175）
                            Spell::Blizzard => {
                                if fastrand::i32(0..8) == 0 {
                                    let dur = (5 + fastrand::i32(0..attacker_stats.freezing.max(1))) as u32;
                                    poison::apply_poison(&mut monster.poison_list,
                                        poison::Poison::new(PoisonType::SLOW, dur, 0, 2000));
                                }
                            }
                            // PoisonCloud：绿毒强度基于 tick_value（创建时按 magic_stat 算出，道士=SC）
                            Spell::PoisonCloud => {
                                let sc_approx = tick_value; // tick_value 基于 magic_stat（道士 SC）
                                let poison_value = (sc_approx / 4).min(10);
                                poison::apply_poison(&mut monster.poison_list,
                                    poison::Poison::new(PoisonType::GREEN, 12, poison_value, 1000));
                            }
                            // FireWall / MeteorStrike：纯伤害无附加
                            _ => {}
                        }
                        // 战斗触发的 Poison（攻击者 freezing/poison_attack）
                        for p in &r.applied_poisons {
                            poison::apply_poison(&mut monster.poison_list, *p);
                        }
                    }
                }
            }
        }

        for (sid, amount) in heal_targets.iter().zip(heal_amounts.iter()) {
            if let Some(record) = self.players.get(sid) {
                let _ = record.actor_ref.ask(Heal { amount: *amount }).await;
            }
        }
        for id in &expired_ids {
            self.spell_objects.remove(id);
        }
    }

    /// 弹道法术延迟结算（对齐 C# HumanObject.CompleteMagic）
    ///
    /// 每 tick 检查 pending_spell_completions 中到期的项，按 spell 分支结算：
    /// - FireBall/GreatFireBall/ThunderBolt：单目标 MAC 伤害（ThunderBolt 亡灵 +50%）
    /// - FrostCrunch：MAC 伤害 + 概率 Slow/Frozen
    /// - Vampirism：MAC 伤害 + 吸血
    pub(crate) async fn tick_spell_completions(&mut self) {
        use mir2_shared::enums::Spell;

        // Boss 延迟攻击结算（C# DelayedAction DelayedType.Damage）
        if !self.boss_pending_attacks.is_empty() {
            let now = self.tick_count;
            let mut due: Vec<ai::DelayedAttack> = Vec::new();
            self.boss_pending_attacks.retain(|(fire, atk)| {
                if *fire <= now {
                    due.push(*atk);
                    false
                } else {
                    true
                }
            });
            for atk in due {
                for (sid, r) in &self.players {
                    if let Ok(Some(st)) = r.actor_ref.ask(GetPlayerState).await {
                        if !st.is_dead
                            && st.map_index == atk.map_index
                            && (st.x - atk.center_x).abs() <= atk.radius
                            && (st.y - atk.center_y).abs() <= atk.radius
                        {
                            let _ = r.actor_ref.ask(TakeDamage {
                                attacker_id: atk.attacker_oid,
                                attacker_session: *sid,
                                damage: atk.damage,
                            }).await;
                        }
                    }
                }
                debug!("Boss delayed attack hit at ({},{}) radius {} dmg {}",
                       atk.center_x, atk.center_y, atk.radius, atk.damage);
            }
        }

        if self.pending_spell_completions.is_empty() {
            return;
        }

        // 取出到期的项
        let now = self.tick_count;
        let mut ready: Vec<PendingSpellCompletion> = Vec::new();
        self.pending_spell_completions.retain(|p| {
            if p.fire_at_tick <= now {
                ready.push(p.clone());
                false
            } else {
                true
            }
        });

        if ready.is_empty() {
            return;
        }

        // 按施法者分组，减少 GetPlayerState 调用
        for pending in ready {
            // 获取施法者状态
            let record = match self.players.get(&pending.session_id) {
                Some(r) => r.clone(),
                None => continue,
            };
            let caster_state = match record.actor_ref.ask(GetPlayerState).await {
                Ok(Some(s)) => s,
                _ => continue,
            };
            if caster_state.is_dead {
                continue;
            }
            // #1184：英雄弹道用英雄自身 CombatStats/等级结算（命中/暴击/等级差）
            let (attacker_stats, caster_level) = if let Some(hs) = pending.hero_stats {
                (
                    hs,
                    pending
                        .hero_level
                        .unwrap_or(caster_state.level),
                )
            } else {
                (
                    caster_state.to_combat_stats(),
                    caster_state.level,
                )
            };
            let spell_enum = Spell::try_from(pending.spell).unwrap_or(Spell::None);

            // 弹道类法术目标可能是怪物或玩家
            // 先查怪物（按 object_id），再查玩家
            // C# 用 InRange(target.CurrentLocation, targetLocation, 2) 防移动 miss

            match spell_enum {
                Spell::FireBall | Spell::GreatFireBall | Spell::ThunderBolt | Spell::FrostCrunch
                | Spell::Vampirism | Spell::FlameDisruptor | Spell::SoulFireBall
                | Spell::MeteorShower | Spell::FireBounce
                // 弓箭手弹道物理系（命中后按 AC 防御结算，BindingShot/NapalmShot 附加效果）
                | Spell::StraightShot | Spell::DoubleShot
                | Spell::BindingShot | Spell::NapalmShot
                | Spell::VampireShot | Spell::PoisonShot | Spell::CrippleShot | Spell::ElementalShot
                | Spell::CatTongue => {
                    Self::complete_projectile_spell(
                        self, pending, &caster_state, &attacker_stats, caster_level, spell_enum,
                    ).await;
                }
                _ => {
                    debug!("tick_spell_completions: unhandled spell {:?}", spell_enum);
                }
            }
        }

        // 处理 Vampirism 吸血回血（循环外统一发，避免借用冲突）
        let heals = std::mem::take(&mut self.vamp_heals);
        for (session_id, amount) in heals {
            if let Some(record) = self.players.get(&session_id) {
                let _ = record.actor_ref.ask(crate::actors::player::Heal { amount }).await;
            }
        }
    }

    /// #1560：弓手远程攻击延迟结算（C# DelayedType.Damage/DamageIndicator）
    ///
    /// 箭矢飞行（range_flight_ticks）到期后：
    /// - 命中：按目标当前防御结算伤害，广播 ObjectStruck + DamageIndicator + ObjectHealth，扣武器耐久；
    /// - 未命中：广播 DamageIndicator Miss（C# DamageType.Miss=4）；
    /// - 目标消失/已死 → 箭矢落空不结算。
    pub(crate) async fn tick_range_completions(&mut self) {
        if self.pending_range_completions.is_empty() {
            return;
        }
        let now = self.tick_count;
        let mut ready: Vec<PendingRangeCompletion> = Vec::new();
        self.pending_range_completions.retain(|p| {
            if p.fire_at_tick <= now {
                ready.push(p.clone());
                false
            } else {
                true
            }
        });

        for c in ready {
            match c.target {
                RangeTarget::Monster(monster_id) => {
                    // 目标怪物已消失/已死 → 箭矢落空（C# CompleteAttack 目标为空则无结算）
                    let mut struck_dir = 0u8;
                    let mut hit_critical = false;
                    let hit_damage = {
                        let Some(monster) = self.monsters.get_mut(&monster_id) else {
                            debug!("RangeAttack resolve: monster {} gone, arrow whiffs", monster_id);
                            continue;
                        };
                        if monster.hp <= 0 {
                            debug!("RangeAttack resolve: monster {} already dead, arrow whiffs", monster_id);
                            continue;
                        }
                        if !c.hit {
                            // 未命中：Miss 飘字（C# BroadcastDamageIndicator(DamageType.Miss)）
                            let mut dmg_body = Vec::new();
                            dmg_body.extend_from_slice(&0i32.to_le_bytes()); // damage = 0
                            dmg_body.push(4u8); // damage_type = Miss
                            dmg_body.extend_from_slice(&monster_id.to_le_bytes());
                            let dmg_packet = build_packet_bytes(
                                mir2_shared::enums::ServerPacketIds::DamageIndicator as i16, &dmg_body);
                            broadcast_to_map(&self.gate_ref, &self.players, self.monsters.get(&monster_id).map(|m| m.map_index).unwrap_or(0), &dmg_packet).await;
                            debug!("RangeAttack resolve: {} -> monster {} MISS", c.attacker_object_id, monster_id);
                            continue;
                        }

                        let defender_stats = monster.to_combat_stats();
                        let attack_result = combat_attack::resolve_attack(
                            &c.attacker_stats, &defender_stats, c.raw_damage,
                            mir2_shared::enums::DefenceType::AcAgility, c.level_offset,
                        );
                        let damage = attack_result.damage;
                        monster.take_damage(damage);
                        monster.last_hitter_session = Some(c.session_id);
                        monster.provoked = true;
                        if monster.target_session.is_none() {
                            monster.target_session = Some(c.session_id);
                        }
                        for p in &attack_result.applied_poisons {
                            crate::combat::poison::apply_poison(&mut monster.poison_list, *p);
                        }
                        // #1582：C# MonsterObject.Attacked——受击时转向攻击者（PointDirection）
                        monster.direction = crate::actors::world::ai::direction_towards(
                            monster.x, monster.y, c.attacker_x, c.attacker_y,
                        );
                        struck_dir = monster.direction;
                        hit_critical = attack_result.is_critical;
                        debug!("RangeAttack resolve: {} -> monster {} for {} damage (crit={})",
                               c.attacker_object_id, monster_id, damage, attack_result.is_critical);
                        damage
                    };
                    self.pending_gather.push(c.session_id);

                    // 武器耐久损耗（C# DamageWeapon：远程命中扣 Random(4)+1）
                    if let Some(record) = self.players.get(&c.session_id) {
                        let broke = record.actor_ref.ask(crate::actors::player::DamageEquipment {
                            slot: EquipmentSlot::Weapon,
                            amount: (1 + fastrand::i32(0..4)) as u16,
                        }).await.unwrap_or(false);
                        if broke {
                            debug!("Player {} weapon broke (ranged)!", c.attacker_object_id);
                            if let Some(state) = self.recalculate_and_set_stat_bonuses(c.session_id).await {
                                self.broadcast_equipment_visuals(c.session_id, &state).await;
                            }
                        }
                    }

                    // 广播受击动画 + 伤害飘字 + 血条给所有玩家（对齐近战路径）
                    let mut struck_body = Vec::new();
                    struck_body.extend_from_slice(&monster_id.to_le_bytes());
                    struck_body.extend_from_slice(&c.attacker_object_id.to_le_bytes());
                    struck_body.extend_from_slice(&(c.target_x as u32).to_le_bytes());
                    struck_body.extend_from_slice(&(c.target_y as u32).to_le_bytes());
                    struck_body.push(struck_dir);
                    let struck_packet = build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::ObjectStruck as i16, &struck_body);

                    let mut dmg_body = Vec::new();
                    dmg_body.extend_from_slice(&hit_damage.to_le_bytes());
                    dmg_body.push(if hit_critical { 5u8 } else { 0u8 }); // damage_type: 0=Hit 5=Critical
                    dmg_body.extend_from_slice(&monster_id.to_le_bytes());
                    let dmg_packet = build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::DamageIndicator as i16, &dmg_body);

                    let percent = self.monsters.get(&monster_id)
                        .map(|m| ((m.hp.max(0) as f32 / m.max_hp.max(1) as f32) * 100.0) as u8)
                        .unwrap_or(0);
                    let mut health_body = Vec::new();
                    health_body.extend_from_slice(&monster_id.to_le_bytes());
                    health_body.push(percent);
                    health_body.extend_from_slice(&3u16.to_le_bytes()); // expire（秒）
                    let health_packet = build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::ObjectHealth as i16, &health_body);

                    broadcast_to_map(&self.gate_ref, &self.players, self.monsters.get(&monster_id).map(|m| m.map_index).unwrap_or(0), &struck_packet).await;
                    broadcast_to_map(&self.gate_ref, &self.players, self.monsters.get(&monster_id).map(|m| m.map_index).unwrap_or(0), &dmg_packet).await;
                    broadcast_to_map(&self.gate_ref, &self.players, self.monsters.get(&monster_id).map(|m| m.map_index).unwrap_or(0), &health_packet).await;

                }
                RangeTarget::Player(defender_session) => {
                    // #1566：PvP 目标——命中走完整结算；未命中对目标玩家广播 Miss
                    if c.hit {
                        self.resolve_ranged_pvp_hit(
                            c.session_id,
                            c.attacker_object_id,
                            c.attacker_stats,
                            defender_session,
                            c.raw_damage,
                            c.level_offset,
                            c.direction,
                            c.target_x,
                            c.target_y,
                        ).await;
                    } else if let Some(record) = self.players.get(&defender_session) {
                        if let Ok(Some(st)) = record.actor_ref.ask(GetPlayerState).await {
                            let mut dmg_body = Vec::new();
                            dmg_body.extend_from_slice(&0i32.to_le_bytes()); // damage = 0
                            dmg_body.push(4u8); // damage_type = Miss
                            dmg_body.extend_from_slice(&st.object_id.to_le_bytes());
                            let dmg_packet = build_packet_bytes(
                                mir2_shared::enums::ServerPacketIds::DamageIndicator as i16, &dmg_body);
                            broadcast_to_map(&self.gate_ref, &self.players, st.map_index, &dmg_packet).await;
                            debug!("RangeAttack resolve: {} -> player {} MISS", c.attacker_object_id, st.object_id);
                        }
                    }
                }
            }
        }
    }

    /// #1312：C# CompleteMagic——弹道命中造成伤害才给技能经验（Random.Next(3)+1）
    async fn grant_spell_hit_exp(&self, session_id: u64, spell_shared: u8) {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        let spell_cs = spell_shared.saturating_sub(3);
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

    /// 弹道法术结算（单目标伤害 + 各法术附加效果）
    ///
    /// 防御类型：法师弹道（FireBall/ThunderBolt/...）用 MAC；弓箭手弹道
    /// （StraightShot/DoubleShot/BindingShot/NapalmShot）用 AC（物理）。
    async fn complete_projectile_spell(
        &mut self,
        pending: PendingSpellCompletion,
        caster_state: &crate::actors::player::PlayerState,
        attacker_stats: &crate::combat::attack::CombatStats,
        caster_level: u16,
        spell: mir2_shared::enums::Spell,
    ) {
        use mir2_shared::enums::{DefenceType, Spell, PoisonType};
        use crate::combat::{attack, poison};

        let target_id = pending.target_id;
        let raw_damage = pending.damage;

        // #1528：弓手技能为魔法箭（C# DelayedType.Magic → MAC 防御），玩家与英雄一致
        let is_archer = matches!(spell,
            Spell::StraightShot | Spell::DoubleShot | Spell::BindingShot | Spell::NapalmShot
            | Spell::VampireShot | Spell::PoisonShot | Spell::CrippleShot | Spell::ElementalShot
            | Spell::CatTongue);
        let defence = DefenceType::Mac;

        // 弓手被动 Focus（C# HumanObject CompleteRangeAttack：Random(5)<=Lv 时命中概率 ×2）
        let mut attacker_stats_owned = attacker_stats.clone();
        if is_archer {
            // #1184：英雄弹道查英雄技能（hero_magics 同 C# 编号）；普通玩家查玩家技能
            let focus_lv = if pending.hero_stats.is_some() {
                caster_state.hero_magics.iter().find(|m| m.spell == 121).map(|m| m.level).unwrap_or(0)
            } else {
                caster_state.magics.iter().find(|m| m.spell == 121).map(|m| m.level).unwrap_or(0)
            };
            if fastrand::i32(0..5) <= focus_lv as i32 {
                attacker_stats_owned.accuracy = attacker_stats_owned.accuracy.saturating_mul(2);
            }
        }

        // 查找目标怪物
        let monster_hit = {
            let monster = self.monsters.iter().find(|(_, m)| m.object_id == target_id);
            if let Some((_, m)) = monster {
                // 防移动 miss：目标当前位置 vs 弹道快照位置，InRange(2)
                let dist = (m.x - pending.target_x).abs() + (m.y - pending.target_y).abs();
                if dist > 2 {
                    debug!("Projectile spell {:?} missed target {} (moved {} tiles)", spell, target_id, dist);
                    None
                } else {
                    // ThunderBolt 亡灵 +50%（C# HumanObject.cs:4126），下方 final_damage 分支按 m.undead 加成
                    Some((m.x, m.y, m.map_index, m.to_combat_stats()))
                }
            } else {
                None
            }
        };

        if let Some((mx, my, hit_map, defender_stats)) = monster_hit {
            // ElementalShot 击退：命中即结算（C# CompleteMagic 中 Attacked 后无条件 DoKnockback）
            let mut elemental_knockback: Option<(u8, i32)> = None;
            if spell == Spell::ElementalShot {
                let mlevel = self.monsters.get(&target_id)
                    .and_then(|m| self.monster_infos.get(&m.monster_index))
                    .map(|i| i.level)
                    .unwrap_or(0);
                if fastrand::i32(0..20) < 6 + pending.spell_level as i32 * 3 + caster_state.level as i32 - mlevel {
                    let distance = 1 + (pending.spell_level as i32 - 1).max(0) + fastrand::i32(0..2);
                    elemental_knockback = Some((
                        crate::actors::world::ai::direction_towards(caster_state.x, caster_state.y, mx, my),
                        distance,
                    ));
                }
            }
            // 法术特化伤害
            let final_damage = match spell {
                // ThunderBolt 对亡灵 +50%（C# HumanObject.cs:4126）
                Spell::ThunderBolt => {
                    if let Some(m) = self.monsters.get(&target_id) {
                        if m.undead { (raw_damage as f32 * 1.5) as i32 } else { raw_damage }
                    } else { raw_damage }
                }
                // #395：FlameDisruptor 对非亡灵 +50%（C# HumanObject.cs:4252，与 ThunderBolt 相反）
                Spell::FlameDisruptor => {
                    if let Some(m) = self.monsters.get(&target_id) {
                        if !m.undead { (raw_damage as f32 * 1.5) as i32 } else { raw_damage }
                    } else { raw_damage }
                }
                _ => raw_damage,
            };

            // #1452：C# LevelOffset = Level > attacker.Level ? 0 : min(10, attacker.Level - Level)
            let mlevel = self.monsters.get(&target_id).map(|m| m.level).unwrap_or(0);
            let level_offset = crate::combat::attack::level_offset(caster_level, mlevel.max(0) as u16);
            let result = attack::resolve_attack(
                &attacker_stats_owned, &defender_stats, final_damage,
                defence, level_offset,
            );

            if result.is_hit && result.damage > 0 {
                // #1312：C# CompleteMagic `Attacked()>0 → LevelMagic`——命中造成伤害才给经验
                self.grant_spell_hit_exp(pending.session_id, pending.spell)
                    .await;
                // #1483：读取弓手特殊箭武装（施放时 40% 设置，命中时消耗）
                let special_armed = match self.players.get(&pending.session_id) {
                    Some(r) => match r.actor_ref.ask(GetPlayerState).await {
                        Ok(Some(s)) => s.special_shot_armed,
                        _ => 0,
                    },
                    None => 0,
                };
                if let Some(monster) = self.monsters.get_mut(&target_id) {
                    monster.take_damage(result.damage);
                    // #1724：魔法弹道命中广播——与近战/弓手一致（C# CompleteMagic → Attacked → ObjectStruck/DamageIndicator/ObjectHealth）
                    monster.direction = crate::actors::world::ai::direction_towards(
                        monster.x, monster.y, caster_state.x, caster_state.y,
                    );
                    let mut struck_body = Vec::new();
                    struck_body.extend_from_slice(&monster.object_id.to_le_bytes());
                    struck_body.extend_from_slice(&caster_state.object_id.to_le_bytes());
                    struck_body.extend_from_slice(&(monster.x as u32).to_le_bytes());
                    struck_body.extend_from_slice(&(monster.y as u32).to_le_bytes());
                    struck_body.push(monster.direction);
                    let struck_packet = build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::ObjectStruck as i16, &struck_body);
                    let mut dmg_body = Vec::new();
                    dmg_body.extend_from_slice(&result.damage.to_le_bytes());
                    dmg_body.push(if result.is_critical { 5u8 } else { 0u8 }); // damage_type: 0=Hit 5=Critical
                    dmg_body.extend_from_slice(&monster.object_id.to_le_bytes());
                    let dmg_packet = build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::DamageIndicator as i16, &dmg_body);
                    let percent = ((monster.hp.max(0) as f32 / monster.max_hp.max(1) as f32) * 100.0) as u8;
                    let mut health_body = Vec::new();
                    health_body.extend_from_slice(&monster.object_id.to_le_bytes());
                    health_body.push(percent);
                    health_body.extend_from_slice(&3u16.to_le_bytes()); // expire（秒）
                    let health_packet = build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::ObjectHealth as i16, &health_body);
                    // #1649：动画/飘字/血条广播只发同图玩家（C# CurrentMap.Broadcast）
                    let hit_map = monster.map_index;
                    broadcast_to_map(&self.gate_ref, &self.players, hit_map, &struck_packet).await;
                    broadcast_to_map(&self.gate_ref, &self.players, hit_map, &dmg_packet).await;
                    broadcast_to_map(&self.gate_ref, &self.players, hit_map, &health_packet).await;
                    monster.last_hitter_session = Some(pending.session_id);
                    self.pending_gather.push(pending.session_id);
                    monster.provoked = true;
                    monster.target_session = Some(pending.session_id);

                    // FrostCrunch：概率 Slow/Frozen（C# HumanObject.cs:5962）
                    if spell == Spell::FrostCrunch {
                        let magic_level = pending.spell_level;
                        // Slow：Random(100) <= magic.Level（玩家目标）或 Random(20) <= level（怪物）
                        if fastrand::i32(0..20) <= magic_level as i32 {
                            let duration = (5 + fastrand::i32(0..5)) as u32;
                            poison::apply_poison(&mut monster.poison_list,
                                poison::Poison::new(PoisonType::SLOW, duration, 0, 1000));
                        }
                        // Frozen：Random(40) <= level
                        if fastrand::i32(0..40) <= magic_level as i32 {
                            let duration = (5 + fastrand::i32(0..caster_state.freezing.max(1))) as u32;
                            poison::apply_poison(&mut monster.poison_list,
                                poison::Poison::new(PoisonType::FROZEN, duration, 0, 1000));
                        }
                    }

                    // BindingShot：命中后施加 Paralysis（定身 3s）
                    if spell == Spell::BindingShot {
                        poison::apply_poison(&mut monster.poison_list,
                            poison::Poison::new(PoisonType::PARALYSIS, 3, 0, 1000));
                    }

                    // #1484：C# SpecialArrowShot——仅 PoisonShot buff 武装时命中施绿毒（C# 公式）
                    if spell == Spell::PoisonShot && special_armed == 2 {
                        let lv = pending.spell_level as i32;
                        let dur = (result.damage * 2 + (lv + 1) * 7).max(1) as u32;
                        let val = (result.damage / 25 + lv + 1
                            + fastrand::i32(0..caster_state.poison_attack.max(1))).max(1);
                        poison::apply_poison(&mut monster.poison_list,
                            poison::Poison::new(PoisonType::GREEN, dur, val, 2000));
                        let _ = self.players.get(&pending.session_id)
                            .map(|r| r.actor_ref.tell(crate::actors::player::SetSpecialShotArmed { armed: 0 }).try_send());
                        debug!("PoisonShot buff consumed -> green poison {} ({}s)", val, dur / 10);
                    }
                    if spell == Spell::CrippleShot {
                        let dur = super::special_shot_buff_time(pending.spell_level).max(1) as u32;
                        poison::apply_poison(&mut monster.poison_list,
                            poison::Poison::new(PoisonType::SLOW, dur, 0, 1000));
                        debug!("CrippleShot slowed monster {} ({}s)", target_id, dur);
                    }
                    // CatTongue：20% 概率冰冻（C# CompleteMagic：Random(10)>=8，Duration=(Lv+1)*3s）
                    if spell == Spell::CatTongue && fastrand::i32(0..10) >= 8 {
                        poison::apply_poison(&mut monster.poison_list,
                            poison::Poison::new(PoisonType::FROZEN, (pending.spell_level as u32 + 1) * 3, 0, 1000));
                        debug!("CatTongue froze monster {} ({}s)", target_id, (pending.spell_level as u32 + 1) * 3);
                    }
                    if spell == Spell::VampireShot && special_armed == 1 {
                        // #1484：C# VampAmount = value*(Lv+1)*0.25（命中消耗 buff）
                        let vamp = (result.damage as f32 * (pending.spell_level as f32 + 1.0) * 0.25) as i32;
                        if vamp > 0 {
                            self.vamp_heals.push((pending.session_id, vamp));
                        }
                        let _ = self.players.get(&pending.session_id)
                            .map(|r| r.actor_ref.tell(crate::actors::player::SetSpecialShotArmed { armed: 0 }).try_send());
                        debug!("VampireShot buff consumed -> leeched {} HP", vamp);
                    }

                    // Vampirism：吸血 = 实伤 × (level+1) × 0.25（C# HumanObject.cs:6011）
                    if spell == Spell::Vampirism {
                        let vamp = (result.damage as f32 * (pending.spell_level as f32 + 1.0) * 0.25) as i32;
                        if vamp > 0 {
                            // 收集回血请求，循环外统一发（避免借用冲突）
                            self.vamp_heals.push((pending.session_id, vamp));
                        }
                    }

                    // 施加战斗触发的 Poison（冰冻攻击/毒物攻击，来自攻击者 Stats）
                    for p in &result.applied_poisons {
                        poison::apply_poison(&mut monster.poison_list, *p);
                    }

                    debug!("Projectile {:?} hit monster {} for {} dmg (crit={})",
                        spell, target_id, result.damage, result.is_critical);

                    // FireBounce：命中怪物后向周围 3 格随机目标继续弹射（C# HumanObject.cs:5944）
                    if spell == Spell::FireBounce && pending.bounce > 0 {
                        let candidates: Vec<(u32, i32, i32)> = self.monsters.iter()
                            .filter(|(id, m)| {
                                **id != target_id
                                    && m.hp > 0
                                    && m.map_index == hit_map
                                    && (m.x - mx).abs() <= 3
                                    && (m.y - my).abs() <= 3
                            })
                            .map(|(id, m)| (*id, m.x, m.y))
                            .collect();
                        if !candidates.is_empty() {
                            let idx = fastrand::usize(0..candidates.len());
                            let (next_id, nx, ny) = candidates[idx];
                            let next_dist = ((mx - nx).abs() + (my - ny).abs()) as u64;
                            let next_delay_ms = next_dist * 50; // 后续弹跳无 +500
                            let next_fire = self.tick_count + (next_delay_ms / 100).max(1);
                            self.pending_spell_completions.push(PendingSpellCompletion {
                                fire_at_tick: next_fire,
                                session_id: pending.session_id,
                                spell: pending.spell,
                                target_id: next_id,
                                target_x: nx,
                                target_y: ny,
                                damage: pending.damage,
                                magic_stat: pending.magic_stat,
                                hero_stats: pending.hero_stats,
                                hero_level: pending.hero_level,
                                spell_level: pending.spell_level,
                                bounce: pending.bounce - 1,
                            });
                            debug!("FireBounce bounces {} -> {} (bounce left {})",
                                target_id, next_id, pending.bounce - 1);
                        }
                    }
                }

            } else {
                debug!("Projectile {:?} missed/blocked target {}", spell, target_id);
            }

            // NapalmShot：命中后 3×3 AOE（爆炸溅射，排除已被直击的主目标）
            if spell == Spell::NapalmShot {
                let splash_ids: Vec<u32> = self.monsters.iter()
                    .filter(|(id, m)| {
                        **id != target_id
                            && m.map_index == hit_map
                            && (m.x - mx).abs() <= 1
                            && (m.y - my).abs() <= 1
                            && m.hp > 0
                    })
                    .map(|(id, _)| *id)
                    .collect();
                for sid in splash_ids {
                    if let Some(monster) = self.monsters.get_mut(&sid) {
                        let ds = monster.to_combat_stats();
                        let r = attack::resolve_attack(
                            &attacker_stats_owned, &ds, raw_damage, DefenceType::Ac, level_offset,
                        );
                        if r.is_hit && r.damage > 0 {
                            monster.take_damage(r.damage);
                            monster.last_hitter_session = Some(pending.session_id);
                            self.pending_gather.push(pending.session_id);
                            monster.provoked = true;
                            monster.target_session = Some(pending.session_id);
                            for p in &r.applied_poisons {
                                poison::apply_poison(&mut monster.poison_list, *p);
                            }
                        }
                    }
                }
                debug!("NapalmShot exploded at ({},{}) 3x3 splash", mx, my);
            }

            // ElementalShot：击退怪物（命中即结算，与是否造成伤害无关）
            if let Some((dir, distance)) = elemental_knockback {
                let _ = self.push_monster(target_id, dir, distance).await;
                debug!("ElementalShot knocked back monster {} {} tiles", target_id, distance);
            }
            // ElementalShot：命中后消耗元素（C# DelayedType.Magic 中 ElementsLevel=0 + ObtainElement(false)）
            if spell == Spell::ElementalShot {
                self.consume_elemental(pending.session_id).await;
            }
            return;
        }

        // 目标不是怪物，查玩家（PvP 弹道，如 SoulFireBall 打玩家）
        let mut pvp_knockback: Option<(u64, u8, i32)> = None;
        for (other_session, other_record) in &self.players {
            if let Ok(Some(other_state)) = other_record.actor_ref.ask(GetPlayerState).await {
                if other_state.object_id != target_id {
                    continue;
                }
                // #1645：C# CompleteMagic——目标校验（同图/未死/非GM/可攻击/安全区/禁战）
                if other_state.map_index != caster_state.map_index {
                    continue;
                }
                if other_state.is_dead {
                    continue;
                }
                if self.gm_protected.contains(&other_session) {
                    continue;
                }
                if !super::can_attack_player(caster_state, &other_state, &self.guild_wars) {
                    continue;
                }
                let attacker_safe = self.maps.get(&caster_state.map_index)
                    .map(|m| m.is_safe_zone(caster_state.x, caster_state.y))
                    .unwrap_or(false);
                let target_safe = self.maps.get(&other_state.map_index)
                    .map(|m| m.is_safe_zone(other_state.x, other_state.y))
                    .unwrap_or(false);
                if attacker_safe || target_safe {
                    continue;
                }
                if self.map_infos.get(&(caster_state.map_index as i32)).map(|mi| mi.no_fight).unwrap_or(false)
                    || self.map_infos.get(&(other_state.map_index as i32)).map(|mi| mi.no_fight).unwrap_or(false)
                {
                    continue;
                }
                let dist = (other_state.x - pending.target_x).abs() + (other_state.y - pending.target_y).abs();
                if dist > 2 {
                    continue;
                }
                let defender_stats = other_state.to_combat_stats();
                // #1452：PvP LevelOffset = Level > attacker.Level ? 0 : min(10, attacker.Level - Level)
                let level_offset = crate::combat::attack::level_offset(caster_level, other_state.level);
                let result = attack::resolve_attack(
                    &attacker_stats_owned, &defender_stats, raw_damage,
                    defence, level_offset,
                );
                if result.is_hit && result.damage > 0 {
                    // #1312：C# CompleteMagic——弹道命中玩家造成伤害才给经验
                    self.grant_spell_hit_exp(pending.session_id, pending.spell)
                        .await;
                    let actor_ref = other_record.actor_ref.clone();
                    let damage = result.damage;
                    let _ = actor_ref.ask(TakeDamage {
                        attacker_id: caster_state.object_id,
                        attacker_session: pending.session_id,
                        damage,
                    }).await;

                    // 弹道附加状态对玩家也生效（FrostCrunch 冰冻/BindingShot 定身）
                    let mut player_poisons = Vec::new();
                    if spell == Spell::FrostCrunch {
                        let ml = pending.spell_level;
                        if fastrand::i32(0..100) <= ml as i32 {
                            player_poisons.push(poison::Poison::new(PoisonType::SLOW, 4, 0, 1000));
                        }
                        if fastrand::i32(0..100) <= ml as i32 {
                            player_poisons.push(poison::Poison::new(PoisonType::FROZEN, 2, 0, 1000));
                        }
                    }
                    if spell == Spell::BindingShot {
                        player_poisons.push(poison::Poison::new(PoisonType::PARALYSIS, 3, 0, 1000));
                    }
                    if spell == Spell::PoisonShot {
                        let dur = super::special_shot_buff_time(pending.spell_level).max(1) as u32;
                        player_poisons.push(poison::Poison::new(PoisonType::GREEN, dur, (pending.damage / 10).max(1), 1000));
                    }
                    if spell == Spell::CrippleShot {
                        let dur = super::special_shot_buff_time(pending.spell_level).max(1) as u32;
                        player_poisons.push(poison::Poison::new(PoisonType::SLOW, dur, 0, 1000));
                    }
                    // PvP 冰冻需 PvpCanFreeze（C# Settings；默认 false 玩家不冰冻）
                    if spell == Spell::CatTongue && self.pvp_cfg.can_freeze && fastrand::i32(0..10) >= 8 {
                        player_poisons.push(poison::Poison::new(
                            PoisonType::FROZEN, (pending.spell_level as u32 + 1) * 3, 0, 1000));
                    }
                    if !player_poisons.is_empty() {
                        let _ = actor_ref.ask(crate::actors::player::ApplyCombatPoisons {
                            poisons: player_poisons,
                        }).await;
                    }

                    // ElementalShot：击退玩家（C# DoKnockback 同样作用于玩家，HumanObject.cs:5660）
                    if spell == Spell::ElementalShot {
                        if fastrand::i32(0..20) < 6 + pending.spell_level as i32 * 3 + caster_state.level as i32 - other_state.level as i32 {
                            let distance = 1 + (pending.spell_level as i32 - 1).max(0) + fastrand::i32(0..2);
                            pvp_knockback = Some((
                                *other_session,
                                crate::actors::world::ai::direction_towards(caster_state.x, caster_state.y, other_state.x, other_state.y),
                                distance,
                            ));
                        }
                    }

                    debug!("Projectile {:?} hit player {} for {} dmg", spell, target_id, damage);
                }
                break;
            }
        }

        // ElementalShot：击退玩家（循环外应用，避免借用冲突）
        if let Some((sid, dir, distance)) = pvp_knockback {
            let _ = self.push_player(sid, dir, distance).await;
            debug!("ElementalShot knocked back player {} {} tiles", sid, distance);
        }
        // ElementalShot：PvP 命中 / 目标移动 miss / 目标已消失，同样消耗元素
        //（C# HumanObject.cs:6423 目标无效分支 ElementsLevel=0 + ObtainElement(false)）
        if spell == Spell::ElementalShot {
            self.consume_elemental(pending.session_id).await;
        }
    }
}

// ============================================================
// Handler 实现
// ============================================================

impl Message<Tick> for WorldActor {
    type Reply = ();

    async fn handle(
        &mut self,
        _msg: Tick,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        // [DEBUG] 每 5 秒打一次 tick 确认 WorldActor 活着
        self.tick_count += 1;

        // NPC 脚本计时器到期清理（SETTIMER/EXPIRETIMER/CHECKTIMER，对齐 C# Envir.Timers）
        self.tick_npc_timers();

        // 组队位置广播（C# Group 周期 SendLocations；每 50 tick ≈ 5s，#1309）
        if self.tick_count % 50 == 0 {
            crate::actors::world::broadcast_group_locations(self).await;
            // #1325：寄售/拍卖到期结算（C# Envir.ProcessAuction）
            crate::actors::world::market::resolve_expired_auctions(self).await;
        }




        // --- 怪物 AI ---
        if !self.monsters.is_empty() && !self.players.is_empty() {
            // 收集所有玩家位置（避免在循环中借用 self）
            // 预收集玩家位置 + PK 值（用于 Guard AI 红名优先）
            let player_positions: Vec<(u64, i32, i32, u32, i32, i32, u16, u16, i32)> = {
                let mut results = Vec::new();
                let invis_tag = std::mem::discriminant(&crate::combat::buff::BuffType::Invisibility);
                for (session_id, record) in &self.players {
                    if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                        if !state.is_dead {
                            // 隐身玩家不会被怪物检测到
                            let is_invisible = state.buffs.iter()
                                .any(|b| std::mem::discriminant(&b.buff_type) == invis_tag);
                            if is_invisible { continue; }
                            let in_safe = self.maps.get(&state.map_index)
                                .map(|m| m.is_safe_zone(state.x, state.y))
                                .unwrap_or(false);
                            if !in_safe {
                                // (session, x, y, object_id, pk_points, hp, map_index, level)
                                results.push((*session_id, state.x, state.y, state.object_id, state.pk_points, state.hp, state.map_index, state.level, state.effective_min_attack()));
                            }
                        }
                    }
                }
                results
            };

            // 对每个怪物执行 AI
            let mut dead_monsters = Vec::new();
            let mut moved_monsters = Vec::new();
        let mut pet_recalls: Vec<(u32, i32, i32, u16)> = Vec::new();
            // 巡逻转身广播（C# ProcessRoam Turn → ObjectTurn）
            let mut monster_turns: Vec<(u32, u8, i32, i32)> = Vec::new();
            let mut moved_targets: HashSet<(i32, i32)> = HashSet::new();
            let mut death_drops: Vec<(u64, i32, i32, u16)> = Vec::new();
            let mut broken_armor: Vec<(u64, EquipmentSlot)> = Vec::new();
            let mut dismount_sessions: Vec<u64> = Vec::new();
            // 预收集怪物当前位置（用于碰撞检测）
            let monster_positions: HashSet<(i32, i32)> = self.monsters.values().map(|m| (m.x, m.y)).collect();
            // 预收集怪物快照（用于 Healer AI 寻找受伤盟友）
            let monster_snapshot: Vec<(u32, i32, i32, i32, i32, u16, i32, String, u16, u8)> = self.monsters.values()
                .map(|m| (m.object_id, m.x, m.y, m.hp, m.max_hp, m.map_index, m.monster_index, m.name.clone(), m.image, m.direction))
                .collect();
            // #986：怪物敌对关系预收集（oid → (master_session, target_session)），供宠物自主索敌
            let monster_hostility: std::collections::HashMap<u32, (Option<u64>, Option<u64>)> =
                self.monsters.iter()
                    .map(|(oid, m)| (*oid, (m.master_session, m.target_session)))
                    .collect();
            // Healer 治疗动作和 Summoner 召唤动作（在循环后应用）
            let mut heal_actions: Vec<(u32, i32)> = Vec::new();
            // #471 宠物协战动作（pet_oid, target_oid, damage, master_session，循环后应用）
            let mut pet_attacks: Vec<(u32, u32, i32, u64)> = Vec::new();
            // #1013 怪物互伤（C# StoneTrap 嘲讽后怪物攻击目标）：(attacker, target, damage)
            let mut monster_attacks: Vec<(u32, u32, i32)> = Vec::new();
            let mut summon_spawns: Vec<MonsterSpawn> = Vec::new();
            // Boss AI 输出队列（在循环后应用）
            let mut boss_moves: Vec<(u32, i32, i32, u8)> = Vec::new();
            let mut boss_attacks: Vec<ai::AttackAction> = Vec::new();
            let mut boss_spell_fields: Vec<ai::SpellFieldSpawn> = Vec::new();
            let mut boss_summons: Vec<ai::BossSummon> = Vec::new();
            let mut boss_child_rocks: Vec<ai::ChildRockSpawn> = Vec::new();
            let mut boss_heals: Vec<(u32, i32)> = Vec::new();
            let mut boss_poisons: Vec<ai::PoisonPlayer> = Vec::new();
            let mut boss_backsteps: Vec<(u32, u8, i32)> = Vec::new();
            let mut boss_pushes: Vec<ai::PushPlayer> = Vec::new();
            let mut boss_player_teleports: Vec<(u64, i32, i32, u8)> = Vec::new();
            let mut boss_delayed_attacks: Vec<ai::DelayedAttack> = Vec::new();
            let mut boss_taunts: Vec<(u32, u32)> = Vec::new();
            let mut boss_monster_teleports: Vec<(u32, i32, i32)> = Vec::new();
            let mut boss_player_buffs: Vec<(u64, crate::combat::buff::BuffInstance)> = Vec::new();
            let mut boss_show_hide: Vec<(u32, bool)> = Vec::new();
            let mut boss_sit_down: Vec<(u32, i32, i32, u8, bool)> = Vec::new();
            let mut boss_effects: Vec<(u32, mir2_shared::enums::SpellEffect)> = Vec::new();
            let mut boss_player_purges: Vec<u64> = Vec::new();
            let mut boss_player_heals: Vec<(u64, i32)> = Vec::new();
            // #1441：每个 master 当前存活 slave 数（C# SlaveList.Count；slave_master 预统计）
            let slave_counts: std::collections::HashMap<u32, usize> = {
                let mut m = std::collections::HashMap::new();
                for (soid, master) in &self.slave_master {
                    if self.monsters.contains_key(soid) {
                        *m.entry(*master).or_insert(0) += 1;
                    }
                }
                m
            };
            // 召唤物过期队列（到期 tick 已过 → 移除，不掉落）
            let mut expired_monsters: Vec<u32> = Vec::new();

            for (oid, monster) in &mut self.monsters {
                // ===== 召唤物时限检查（recall_at_tick > 0 表示为召唤物）=====
                if monster.recall_at_tick > 0 && self.tick_count >= monster.recall_at_tick {
                    expired_monsters.push(*oid);
                    continue;
                }
                // ===== Boss AI 分发 =====
                // 已注册 Boss 走 behavior.process_tick，普通怪走原有内联逻辑
                if ai::is_registered_boss(&monster.name) {
                    let monster_oid = monster.object_id;
                    let monster_index = monster.monster_index;
                    let _monster_map = monster.map_index;
                    let monster_name = monster.name.clone();
                    // 宠物元数据（C# Master.PMode / Master.Target / PetLevel；#471 协战）
                    let (master_pet_mode, master_target, has_master_monster_target) =
                        if let Some(master) = monster.master_session {
                            let mode = self.player_pet_modes.get(&master).copied();
                            let tgt = self.player_targets.get(&master).copied()
                                .and_then(|oid| player_positions.iter()
                                    .find(|(_, _, _, o, _, _, _, _, _)| *o == oid)
                                    .map(|(s, x, y, oid, pk, hp, map, lvl, min_dc)| ai::PlayerSnap {
                                        session_id: *s, x: *x, y: *y, hp: *hp, map_index: *map, object_id: *oid, level: *lvl, pk_points: *pk, min_dc: *min_dc,
                                    }));
                            (mode, tgt, self.pet_targets.contains_key(&monster.object_id))
                        } else {
                            (None, None, false)
                        };
                    let player_snaps: Vec<ai::PlayerSnap> = player_positions.iter()
                        .map(|(s, x, y, oid, pk, hp, map, lvl, min_dc)| ai::PlayerSnap {
                                        session_id: *s, x: *x, y: *y, hp: *hp, map_index: *map, object_id: *oid, level: *lvl, pk_points: *pk, min_dc: *min_dc,
                                    }).collect();
                    let monster_name_map: std::collections::HashMap<i32, String> =
                        self.monster_infos.iter().map(|(k, v)| (*k, v.name.clone())).collect();
                    // monster_snaps 从循环外预收集的 monster_snapshot 构建（避免 &mut self.monsters 借用冲突）
                    let monster_snaps: Vec<ai::MonsterSnap> = monster_snapshot.iter()
                        .map(|(oid, x, y, hp, max_hp, map, idx, _, _, _)| ai::MonsterSnap {
                            object_id: *oid, x: *x, y: *y, hp: *hp, max_hp: *max_hp,
                            map_index: *map, monster_index: *idx,
                        }).collect();
                    let mut ctx = ai::AiCtx {
                        tick_count: self.tick_count,
                        monster_oid, monster_index,
                        map_size: self.maps.get(&monster.map_index)
                            .map(|m| (m.width as i32, m.height as i32))
                            .unwrap_or((200, 200)),
                        dragon_level: self.dragon_state.as_ref().map(|d| d.level).unwrap_or(0),
                        players: &player_snaps,
                        monsters: &monster_snaps,
                        monster_name_by_index: &monster_name_map,
                        slave_count: slave_counts.get(&monster_oid).copied().unwrap_or(0),
                        out_moves: &mut boss_moves,
                        out_attacks: &mut boss_attacks,
                        out_spell_fields: &mut boss_spell_fields,
                        out_summons: &mut boss_summons,
                        out_child_rocks: &mut boss_child_rocks,
                        out_heals: &mut boss_heals,
                        out_backsteps: &mut boss_backsteps,
                        out_poisons: &mut boss_poisons,
                        out_pushes: &mut boss_pushes,
                        out_player_teleports: &mut boss_player_teleports,
                        out_delayed_attacks: &mut boss_delayed_attacks,
                        out_monster_taunts: &mut boss_taunts,
                        out_monster_teleports: &mut boss_monster_teleports,
                        out_player_buffs: &mut boss_player_buffs,
                        out_show_hide: &mut boss_show_hide,
                        out_sit_down: &mut boss_sit_down,
                        out_effects: &mut boss_effects,
                        out_player_purges: &mut boss_player_purges,
                        out_player_heals: &mut boss_player_heals,
                        pet_level: self.pet_levels.get(&monster_oid).copied().unwrap_or(0),
                        master_pet_mode,
                        master_target,
                        has_master_monster_target,
                    };
                    // 临时取出 behavior 避免 &mut monster + &mut behavior 双重借用
                    let mut behavior = std::mem::replace(
                        &mut monster.behavior,
                        Box::new(crate::actors::world::ai::DefaultBehavior::new()),
                    );
                    behavior.process_tick(monster, &mut ctx);
                    monster.behavior = behavior;

                    // 死亡检查：注册 Boss（含 StoneTrap 等）hp<=0 时进 dead_monsters
                    //（此前 Boss 分支直接 continue，hp<=0 永不消失/不触发 on_die）
                    if monster.hp <= 0 {
                        dead_monsters.push(*oid);
                        continue;
                    }

                    // #1013：应用怪物嘲讽（C# StoneTrap）→ monster_targets
                    for (target, taunter) in boss_taunts.drain(..) {
                        if target != monster.object_id {
                            self.monster_targets.insert(target, taunter);
                        }
                    }
                    // #471 宠物协战（自定义 AI 宠物）：攻击主人攻击的怪物，不主动攻击玩家
                    // C# CanAttack：MoveOnly/None 不允许攻击（PetMode）
                    if let Some(master) = monster.master_session {
                        // #1777：宠物受控制毒同样禁移动/攻击
                        let monster_light = self.monster_infos.get(&monster.monster_index).map(|i| i.light).unwrap_or(0);
                        let (pet_move_blocked, pet_attack_blocked) = monster_control_blocked(&monster.poison_list, monster_light);
                        let pet_may_attack = self.player_pet_modes
                            .get(&master)
                            .map(|m| matches!(m, mir2_shared::enums::PetMode::Both
                                | mir2_shared::enums::PetMode::AttackOnly
                                | mir2_shared::enums::PetMode::FocusMasterTarget))
                            .unwrap_or(true); // 无缓存默认允许（C# 默认 Both）
                        if !pet_may_attack {
                            monster.target_session = None;
                            continue;
                        }
                        // 协战目标（#471 主人攻击的怪物）
                        let mut pet_target: Option<(u32, i32, i32)> = None;
                        if let Some(tmid) = self.pet_targets.get(&monster.object_id).copied() {
                            let target_alive = monster_snapshot.iter()
                                .any(|s| s.0 == tmid && s.3 > 0 && s.5 == monster.map_index);
                            if !target_alive {
                                self.pet_targets.remove(&monster.object_id);
                            } else if let Some((_, tx, ty, _, _, _, _, _, _, _)) =
                                monster_snapshot.iter().find(|s| s.0 == tmid)
                            {
                                pet_target = Some((tmid, *tx, *ty));
                            }
                        }
                        // #986：无协战目标 → 找最近正在攻击主人的怪物（C# FindTarget 子集）
                        if pet_target.is_none() {
                            pet_target = pet_find_hostile_target(monster, &monster_snapshot, &monster_hostility);
                        }
                        if let Some((tmid, tx, ty)) = pet_target {
                            let dist = (tx - monster.x).abs() + (ty - monster.y).abs();
                            if dist <= 1 && self.tick_count >= monster.next_attack_tick && !pet_attack_blocked {
                                let dmg_range = (monster.max_dmg - monster.min_dmg).max(1);
                                let damage = ((self.tick_count.wrapping_add(monster.object_id as u64)
                                    .wrapping_mul(13)) as i32 % dmg_range) + monster.min_dmg;
                                // #1732：攻击前转向目标（C# MonsterObject.Attack）
                                monster.direction = crate::actors::world::ai::direction_towards(
                                    monster.x, monster.y, tx, ty,
                                );
                                pet_attacks.push((monster.object_id, tmid, damage, master));
                                monster.next_attack_tick = self.tick_count + slow_adjusted_ticks(
                                    monster.ai_profile.attack_cooldown,
                                    crate::combat::poison::is_slowed(&monster.poison_list),
                                );
                                monster.ai_state = MonsterAiState::Attack;
                            } else if self.tick_count >= monster.next_move_tick && !pet_move_blocked {
                                let mut path = self.monster_paths.entry(monster.object_id).or_default();
                                let recalc = path.is_empty()
                                    || self.monster_path_targets.get(&monster.object_id)
                                        .map(|(s, px, py)| *s != PATH_TARGET_PET_ATTACK || *px != tx || *py != ty)
                                        .unwrap_or(true)
                                    || !self.maps.get(&monster.map_index)
                                        .map(|m| m.is_walkable(path[0].0, path[0].1))
                                        .unwrap_or(false);
                                if recalc {
                                    *path = self.maps.get(&monster.map_index)
                                        .and_then(|m| crate::maps::pathfind::find_path(m, (monster.x, monster.y), (tx, ty)))
                                        .unwrap_or_default();
                                    self.monster_path_targets.insert(monster.object_id, (PATH_TARGET_PET_ATTACK, tx, ty));
                                }
                                if !path.is_empty() {
                                    let candidate = path[0];
                                    let walkable = self.maps.get(&monster.map_index)
                                        .map(|m| m.is_walkable(candidate.0, candidate.1))
                                        .unwrap_or(true);
                                    if walkable && !monster_positions.contains(&candidate) && moved_targets.insert(candidate) {
                                        let dir = (0..8)
                                            .find(|d| MON_DIR_DX[*d] == candidate.0 - monster.x && MON_DIR_DY[*d] == candidate.1 - monster.y)
                                            .unwrap_or(monster.direction as usize) as u8;
                                        moved_monsters.push((monster.object_id, candidate.0, candidate.1, dir));
                                        path.remove(0);
                                    }
                                }
                                monster.next_move_tick = self.tick_count + slow_adjusted_ticks(
                                    monster.ai_profile.move_interval,
                                    crate::combat::poison::is_slowed(&monster.poison_list),
                                );
                                monster.ai_state = MonsterAiState::Chase;
                            }
                        }
                    }
                    debug!("Boss '{}' AI tick processed", monster_name);
                    continue;
                }
                // ===== 静态环境物体（CanMove=false && CanAttack=false，对齐 C# Tree/Wall 等）=====
                // 不可移动、不攻击：跳过全部 AI 逻辑，仅保留死亡判定（循环末尾）。
                if ai::is_static_object(&monster.name) {
                    monster.ai_state = MonsterAiState::Idle;
                    if monster.hp <= 0 {
                        dead_monsters.push(*oid);
                    }
                    continue;
                }
                // ===== 被动环境物体（可移动但不主动攻击，对齐 C# Deer/Doe/Football）=====
                // 跳过攻击与追击；仍允许返回出生点漫游（由下方 else-if 分支处理）。
                let is_passive_obj = ai::is_passive_object(&monster.name);
                let profile = &monster.ai_profile;
                // C# Slow 毒：攻速/移速 +100ms（上限 3500ms）
                let slowed = crate::combat::poison::is_slowed(&monster.poison_list);
                // C# CanMove/CanAttack：控制毒禁移动/攻击（#1824：Light 10/5 光源怪豁免 STUN）
                let monster_light = self.monster_infos.get(&monster.monster_index).map(|i| i.light).unwrap_or(0);
                let (move_blocked, attack_blocked) = monster_control_blocked(&monster.poison_list, monster_light);

                // 找最近玩家（在视野范围内）
                // Guard AI：优先攻击红名玩家（PK 值 > 0）
                let mut nearest: Option<(u64, i32, i32, i32)> = None;
                if profile.ai_type == MonsterAiType::Guard {
                    // C# Guard（ai=6/58/113）：只攻击红名玩家（PK>0），绝不攻击清白玩家
                    // PlayerObject.IsAttackTarget(MonsterObject)：AI==6/58/113 → PKPoints >= 200
                    // #1157：去掉“无红名时回退任意玩家”，避免城镇守卫围杀 PK=0 玩家
                    let mut red_nearest: Option<(u64, i32, i32, i32)> = None;
                    for (session, px, py, _, pk, _, _, _, _) in &player_positions {
                        let dist = (monster.x - px).abs() + (monster.y - py).abs();
                        if dist <= profile.aggro_range && *pk > 0 {
                            if red_nearest.is_none_or(|n| dist < n.3) {
                                red_nearest = Some((*session, *px, *py, dist));
                            }
                        }
                    }
                    nearest = red_nearest;
                } else {
                    for (session, px, py, _, _, _, _, _, _) in &player_positions {
                        let dist = (monster.x - px).abs() + (monster.y - py).abs();
                        if dist <= profile.aggro_range {
                            if nearest.is_none_or(|n| dist < n.3) {
                                nearest = Some((*session, *px, *py, dist));
                            }
                        }
                    }
                }

                // 目标粘性 + 索敌（C# MonsterObject）：
                // - 已有目标：DataRange(16) 内保留（跨图/死亡/超距 → Target=null 仇恨丢失）
                // - 无目标：ProcessSearch（SearchDelay 3s 到点）→ 视野内最近玩家
                // - 有目标但到重搜时间：1/3 概率重新 FindTarget（可能切换目标）
                // C# ProcessTarget：目标死亡/丢失 → 立即重新索敌（不等 SearchDelay）
                let had_target = monster.target_session.is_some();
                let mut chase_target: Option<(u64, i32, i32, i32)> = None; // (session, px, py, dist)
                if let Some(ts) = monster.target_session {
                    if let Some((sid, px, py, _, _, hp, map, _, _)) =
                        player_positions.iter().find(|(s, _, _, _, _, _, _, _, _)| *s == ts)
                    {
                        let d = (monster.x - px).abs() + (monster.y - py).abs();
                        if *map == monster.map_index && *hp > 0 && d <= DATA_RANGE {
                            chase_target = Some((*sid, *px, *py, d));
                        } else {
                            monster.target_session = None; // 仇恨丢失
                        }
                    } else {
                        monster.target_session = None;
                    }
                }
                if chase_target.is_none() && had_target {
                    // 目标刚丢失（死亡/超距/跨图）：重置索敌计时，下一 tick 立即搜索
                    self.monster_search_ticks.insert(*oid, 0);
                }
                let search_due = self.monster_search_ticks.get(oid).copied().unwrap_or(0) <= self.tick_count;
                if chase_target.is_none() {
                    if let Some((sess, px, py, dist)) = nearest {
                        monster.target_session = Some(sess);
                        chase_target = Some((sess, px, py, dist));
                    }
                } else if search_due && fastrand::i32(0..3) == 0 {
                    // C# ProcessSearch：Target != null 时 1/3 概率重新搜索
                    if let Some((sess, px, py, dist)) = nearest {
                        monster.target_session = Some(sess);
                        chase_target = Some((sess, px, py, dist));
                    }
                }
                if search_due {
                    self.monster_search_ticks.insert(*oid, self.tick_count + SEARCH_DELAY_TICKS);
                }

                // 被动环境物体（Deer/Doe/Football 等）：不主动攻击/追击玩家，
                // 清空 nearest 使其跳过下方攻击+追击分支，仅保留返回出生点的漫游。
                if is_passive_obj {
                    nearest = None;
                    monster.target_session = None;
                }

                // 低血量逃跑判定（Coward）
                let hp_pct = monster.hp as f32 / monster.max_hp as f32;
                let is_fleeing = profile.ai_type == MonsterAiType::Coward && hp_pct < profile.flee_threshold;

                // 是否在攻击冷却中
                let can_attack = self.tick_count >= monster.next_attack_tick && !attack_blocked;
                // 是否可以移动（移动间隔）
                let can_move = self.tick_count >= monster.next_move_tick && !move_blocked;

                // Passive 怪物：未激怒时不主动攻击
                let should_chase = match profile.ai_type {
                    MonsterAiType::Passive => monster.provoked,
                    MonsterAiType::Guard => chase_target.is_some_and(|(_, _, _, d)| d <= profile.aggro_range) && dist_to_spawn(monster) <= profile.aggro_range * 2,
                    _ => chase_target.is_some(),
                };

                // #471：宠物——不主动攻击玩家；有协战目标则靠近/攻击
                // C# CanAttack：MoveOnly/None 不允许攻击（PetMode）
                let pet_may_attack = match monster.master_session {
                    Some(master) => self.player_pet_modes
                        .get(&master)
                        .map(|m| matches!(m, mir2_shared::enums::PetMode::Both
                            | mir2_shared::enums::PetMode::AttackOnly
                            | mir2_shared::enums::PetMode::FocusMasterTarget))
                        .unwrap_or(true), // 无缓存默认允许（C# 默认 Both）
                    None => false,
                };
                if monster.master_session.is_some() && pet_may_attack {
                    nearest = None;
                    chase_target = None;
                    monster.target_session = None;
                    // 协战目标（#471 主人攻击的怪物）
                    let mut pet_target: Option<(u32, i32, i32)> = None;
                    if let Some(tmid) = self.pet_targets.get(&monster.object_id).copied() {
                        let target_alive = monster_snapshot.iter().any(|s| s.0 == tmid && s.3 > 0 && s.5 == monster.map_index);
                        if !target_alive {
                            self.pet_targets.remove(&monster.object_id);
                        } else if let Some((_, tx, ty, _, _, _, _, _, _, _)) = monster_snapshot.iter().find(|s| s.0 == tmid) {
                            pet_target = Some((tmid, *tx, *ty));
                        }
                    }
                    // #986：无协战目标 → 找最近正在攻击主人的怪物（C# FindTarget 子集）
                    if pet_target.is_none() {
                        pet_target = pet_find_hostile_target(monster, &monster_snapshot, &monster_hostility);
                    }
                    if let Some((tmid, tx, ty)) = pet_target {
                        let dist = (tx - monster.x).abs() + (ty - monster.y).abs();
                        if dist <= 1 && can_attack {
                            let dmg_range = (monster.max_dmg - monster.min_dmg).max(1);
                            let damage = ((self.tick_count.wrapping_add(*oid as u64).wrapping_mul(13)) as i32 % dmg_range) + monster.min_dmg;
                            // #1732：攻击前转向目标（C# MonsterObject.Attack）
                            monster.direction = crate::actors::world::ai::direction_towards(
                                monster.x, monster.y, tx, ty,
                            );
                            pet_attacks.push((*oid, tmid, damage, monster.master_session.unwrap_or(0)));
                            monster.next_attack_tick = self.tick_count + slow_adjusted_ticks(profile.attack_cooldown, slowed);
                            monster.ai_state = MonsterAiState::Attack;
                        } else if can_move {
                            let mut path = self.monster_paths.entry(*oid).or_default();
                            let recalc = path.is_empty()
                                || self.monster_path_targets.get(&*oid)
                                    .map(|(s, px, py)| *s != PATH_TARGET_PET_ATTACK || *px != tx || *py != ty)
                                    .unwrap_or(true)
                                || !self.maps.get(&monster.map_index)
                                    .map(|m| m.is_walkable(path[0].0, path[0].1))
                                    .unwrap_or(false);
                            if recalc {
                                *path = self.maps.get(&monster.map_index)
                                    .and_then(|m| crate::maps::pathfind::find_path(m, (monster.x, monster.y), (tx, ty)))
                                    .unwrap_or_default();
                                self.monster_path_targets.insert(*oid, (PATH_TARGET_PET_ATTACK, tx, ty));
                            }
                            if !path.is_empty() {
                                let candidate = path[0];
                                let walkable = self.maps.get(&monster.map_index)
                                    .map(|m| m.is_walkable(candidate.0, candidate.1))
                                    .unwrap_or(true);
                                if walkable && !monster_positions.contains(&candidate) && moved_targets.insert(candidate) {
                                    let dir = (0..8)
                                        .find(|d| MON_DIR_DX[*d] == candidate.0 - monster.x && MON_DIR_DY[*d] == candidate.1 - monster.y)
                                        .unwrap_or(monster.direction as usize) as u8;
                                    moved_monsters.push((*oid, candidate.0, candidate.1, dir));
                                    path.remove(0);
                                }
                            }
                            monster.next_move_tick = self.tick_count + slow_adjusted_ticks(profile.move_interval, slowed);
                            monster.ai_state = MonsterAiState::Chase;
                        }
                    }
                }

                // #1013：怪物互伤目标（C# StoneTrap 嘲讽）——优先于玩家索敌
                let mut monster_target_active = false;
                if let Some(tmid) = self.monster_targets.get(oid).copied() {
                    let target_alive = monster_snapshot.iter()
                        .any(|s| s.0 == tmid && s.3 > 0 && s.5 == monster.map_index);
                    if !target_alive {
                        self.monster_targets.remove(oid);
                    } else if let Some((_, tx, ty, _, _, _, _, _, _, _)) =
                        monster_snapshot.iter().find(|s| s.0 == tmid)
                    {
                        monster_target_active = true;
                        monster.target_session = None;
                        let dist = (tx - monster.x).abs() + (ty - monster.y).abs();
                        if dist <= 1 && can_attack {
                            let dmg_range = (monster.max_dmg - monster.min_dmg).max(1);
                            let damage = ((self.tick_count.wrapping_add(*oid as u64).wrapping_mul(17)) as i32 % dmg_range) + monster.min_dmg;
                            // #1732：攻击前转向目标（C# MonsterObject.Attack）
                            monster.direction = crate::actors::world::ai::direction_towards(
                                monster.x, monster.y, *tx, *ty,
                            );
                            monster_attacks.push((*oid, tmid, damage));
                            monster.next_attack_tick = self.tick_count + slow_adjusted_ticks(profile.attack_cooldown, slowed);
                            monster.ai_state = MonsterAiState::Attack;
                        } else if can_move {
                            let mut path = self.monster_paths.entry(*oid).or_default();
                            let recalc = path.is_empty()
                                || self.monster_path_targets.get(oid)
                                    .map(|(s, px, py)| *s != PATH_TARGET_MONSTER_ATTACK || *px != *tx || *py != *ty)
                                    .unwrap_or(true)
                                || !self.maps.get(&monster.map_index)
                                    .map(|m| m.is_walkable(path[0].0, path[0].1))
                                    .unwrap_or(false);
                            if recalc {
                                *path = self.maps.get(&monster.map_index)
                                    .and_then(|m| crate::maps::pathfind::find_path(m, (monster.x, monster.y), (*tx, *ty)))
                                    .unwrap_or_default();
                                self.monster_path_targets.insert(*oid, (PATH_TARGET_MONSTER_ATTACK, *tx, *ty));
                            }
                            if !path.is_empty() {
                                let candidate = path[0];
                                let walkable = self.maps.get(&monster.map_index)
                                    .map(|m| m.is_walkable(candidate.0, candidate.1))
                                    .unwrap_or(true);
                                if walkable && !monster_positions.contains(&candidate) && moved_targets.insert(candidate) {
                                    let dir = (0..8)
                                        .find(|d| MON_DIR_DX[*d] == candidate.0 - monster.x && MON_DIR_DY[*d] == candidate.1 - monster.y)
                                        .unwrap_or(monster.direction as usize) as u8;
                                    moved_monsters.push((*oid, candidate.0, candidate.1, dir));
                                    path.remove(0);
                                }
                            }
                            monster.next_move_tick = self.tick_count + slow_adjusted_ticks(profile.move_interval, slowed);
                            monster.ai_state = MonsterAiState::Chase;
                        }
                    }
                }

                if monster_target_active {
                    // 已在上面处理怪物互伤攻击/移动，跳过玩家索敌
                } else if let Some((target_session, px, py, dist)) = chase_target {
                    // #395：幻觉——期内不攻击/不追击（C# HallucinationTime）
                    if self.hallucinated.get(&monster.object_id).is_some_and(|u| self.tick_count < *u) {
                        monster.target_session = None;
                        monster.ai_state = MonsterAiState::Idle;
                    } else if is_fleeing && can_move {
                        // 逃跑：远离目标
                        let (nx, ny, dir) = monster.step_away(px, py);
                        if self.maps.get(&monster.map_index).map(|m| m.is_walkable(nx, ny)).unwrap_or(true)
                            && !monster_positions.contains(&(nx, ny))
                            && moved_targets.insert((nx, ny))
                        {
                            moved_monsters.push((*oid, nx, ny, dir));
                        }
                        monster.next_move_tick = self.tick_count + slow_adjusted_ticks(profile.move_interval, slowed);
                        monster.ai_state = MonsterAiState::Flee;
                    } else if dist <= profile.attack_range && can_attack {
                        // Healer AI：优先治疗附近受伤的怪物
                        let mut did_heal = false;
                        if profile.ai_type == MonsterAiType::Healer {
                            let mut best_target: Option<(u32, i32)> = None; // (oid, deficit)
                            for (snap_oid, sx, sy, shp, smax, smap, _, _, _, _) in &monster_snapshot {
                                if *snap_oid == *oid { continue; }
                                if *smap != monster.map_index { continue; }
                                let dist_ally = (monster.x - sx).abs() + (monster.y - sy).abs();
                                if dist_ally <= profile.aggro_range && *shp < *smax {
                                    let deficit = *smax - *shp;
                                    if best_target.is_none_or(|(_, d)| deficit > d) {
                                        best_target = Some((*snap_oid, deficit));
                                    }
                                }
                            }
                            if let Some((target_oid, _)) = best_target {
                                let heal_amount = (monster.max_hp / 4).max(10);
                                heal_actions.push((target_oid, heal_amount));
                                monster.next_attack_tick = self.tick_count + slow_adjusted_ticks(profile.attack_cooldown, slowed);
                                monster.ai_state = MonsterAiState::Attack;
                                did_heal = true;
                                debug!("Monster '{}' (#{}) heals ally #{} for {} HP", monster.name, *oid, target_oid, heal_amount);
                                // 广播治疗法术效果
                                let mut heal_body = Vec::new();
                                heal_body.extend_from_slice(&monster.object_id.to_le_bytes());
                                heal_body.extend_from_slice(&(monster.x as u32).to_le_bytes());
                                heal_body.extend_from_slice(&(monster.y as u32).to_le_bytes());
                                heal_body.push(monster.direction);
                                heal_body.push(SPELL_HEALING);
                                heal_body.push(0u8); // level
                                heal_body.push(0u8); // attack_type
                                let heal_packet = build_packet_bytes(
                                    mir2_shared::enums::ServerPacketIds::ObjectAttack as i16, &heal_body);
                                // #1649：动画广播只发同图玩家（C# CurrentMap.Broadcast）
                                broadcast_to_map(&self.gate_ref, &self.players, monster.map_index, &heal_packet).await;
                            }
                        }
                        // Summoner AI：低血量时召唤援军
                        let mut did_summon = false;
                        if profile.ai_type == MonsterAiType::Summoner && !did_heal {
                            let hp_pct = monster.hp as f32 / monster.max_hp as f32;
                            if hp_pct < 0.5 && self.tick_count >= monster.next_summon_tick {
                                // 找附近可行走的位置
                                let offsets: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
                                let mut spawn_count = 0;
                                for (dx, dy) in offsets {
                                    if spawn_count >= 2 { break; }
                                    let sx = monster.x + dx;
                                    let sy = monster.y + dy;
                                    if self.maps.get(&monster.map_index).map(|m| m.is_walkable(sx, sy)).unwrap_or(false)
                                        && !monster_positions.contains(&(sx, sy))
                                    {
                                        summon_spawns.push(MonsterSpawn {
                                            name: format!("{}的召唤物", monster.name),
                                            image: monster.image,
                                            monster_index: monster.monster_index,
                                            x: sx,
                                            y: sy,
                                            direction: monster.direction,
                                            hp: (monster.max_hp / 2).max(1),
                                            min_dmg: (monster.min_dmg / 2).max(1),
                                            max_dmg: (monster.max_dmg / 2).max(1),
                                            xp: (monster.xp / 2).max(1),
                                            map_index: monster.map_index,
                                            count: 1,
                                            spread: 0,
                                        });
                                        spawn_count += 1;
                                    }
                                }
                                if spawn_count > 0 {
                                    monster.next_summon_tick = self.tick_count + 100; // 10秒冷却
                                    monster.next_attack_tick = self.tick_count + slow_adjusted_ticks(profile.attack_cooldown, slowed);
                                    monster.ai_state = MonsterAiState::Attack;
                                    did_summon = true;
                                    debug!("Monster '{}' (#{}) summons {} adds", monster.name, *oid, spawn_count);
                                }
                            }
                        }
                        if did_heal || did_summon {
                            // 已执行特殊动作，跳过普通攻击
                        } else {
                            // 攻击
                            let dmg_range = (monster.max_dmg - monster.min_dmg).max(1);
                            let mut damage = ((self.tick_count.wrapping_add(*oid as u64).wrapping_mul(7)) as i32 % dmg_range)
                                + monster.min_dmg;
                            // #448：宠物强化 DC 加成（PetEnhancer）
                            if let Some((until, dc_bonus, _ac)) = self.pet_enhanced.get(&monster.object_id).copied() {
                                if self.tick_count < until {
                                    damage += dc_bonus;
                                }
                            }
                            // #306：诅咒减伤（C# Curse 降低 MaxDC/MaxMC/MaxSC 输出百分比）
                            if let Some((pct, until)) = self.cursed_monsters.get(&monster.object_id) {
                                if self.tick_count < *until && *pct > 0 {
                                    damage = (damage * (100 - pct)) / 100;
                                }
                            }
                            debug!("Monster '{}' (#{}) attacks Player {} for {} dmg [AI={:?}]", monster.name, *oid, target_session, damage, profile.ai_type);
                            monster.next_attack_tick = self.tick_count + slow_adjusted_ticks(profile.attack_cooldown, slowed);
                            monster.ai_state = MonsterAiState::Attack;
                            // #1732：C# MonsterObject.Attack（:2162）——攻击前转向目标
                            monster.direction = crate::actors::world::ai::direction_towards(
                                monster.x, monster.y, px, py,
                            );

                        let is_ranged = matches!(profile.ai_type, MonsterAiType::Ranged | MonsterAiType::Mage);
                        let spell_id = match profile.ai_type {
                            MonsterAiType::Mage => SPELL_FIREBALL,
                            MonsterAiType::Ranged => 1u8,
                            _ => 0u8,
                        };

                        // #1699：远程/法术攻击广播 ObjectRangeAttack（客户端生成弹道 ProjectileFromTo）；
                        // 近战广播 ObjectAttack（C# MonsterObject.Attack）
                        let attack_packet = if is_ranged {
                            let target_oid = player_positions.iter()
                                .find(|(sid, _, _, _, _, _, _, _, _)| *sid == target_session)
                                .map(|(_, _, _, oid, _, _, _, _, _)| *oid)
                                .unwrap_or(0);
                            build_packet_bytes(
                                mir2_shared::enums::ServerPacketIds::ObjectRangeAttack as i16,
                                &build_object_range_attack_body(
                                    monster.object_id, monster.x, monster.y, monster.direction,
                                    target_oid, px, py, 0u8, spell_id,
                                ),
                            )
                        } else {
                            let mut attack_body = Vec::new();
                            attack_body.extend_from_slice(&monster.object_id.to_le_bytes());
                            attack_body.extend_from_slice(&(monster.x as u32).to_le_bytes());
                            attack_body.extend_from_slice(&(monster.y as u32).to_le_bytes());
                            attack_body.push(monster.direction);
                            attack_body.push(spell_id);
                            attack_body.push(0u8); // level
                            attack_body.push(0u8); // attack_type
                            build_packet_bytes(
                                mir2_shared::enums::ServerPacketIds::ObjectAttack as i16, &attack_body)
                        };
                        // #1649：动画广播只发同图玩家（C# CurrentMap.Broadcast）
                        broadcast_to_map(&self.gate_ref, &self.players, monster.map_index, &attack_packet).await;
                        // 安全区保护：目标在安全区内则不受怪物伤害
                        let target_in_safe = self.maps.get(&monster.map_index)
                            .map(|m| m.is_safe_zone(px, py))
                            .unwrap_or(false);

                        if is_ranged {
                            // #1703：远程伤害按弹道延迟结算（C# DelayedAction RangeDamage；复用玩家 range_flight_ticks）
                            let attack_dist = (monster.x - px).abs().max((monster.y - py).abs());
                            self.ranged_pending.push(RangedPendingHit {
                                fire_tick: self.tick_count + crate::actors::world::combat::range_flight_ticks(attack_dist),
                                attacker_oid: monster.object_id,
                                target_session,
                                damage,
                                map_index: monster.map_index,
                                px,
                                py,
                                is_magic: matches!(profile.ai_type, MonsterAiType::Mage),
                                target_in_safe,
                            });
                        } else {
                            let attacker_stats = monster.to_combat_stats();
                            let reflected = apply_monster_hit_player(
                                &self.players, &self.gate_ref, self.death_exp_penalty_percent,
                                monster.object_id, &monster.name, &attacker_stats, monster.level,
                                target_session, damage, px, py, monster.map_index,
                                target_in_safe, monster_melee_defence_type(&monster.name),
                                &mut death_drops, &mut dismount_sessions, &mut broken_armor,
                            ).await;
                            if reflected > 0 {
                                monster.take_damage(reflected);
                                monster.provoked = true;
                            }
                        }
                        } // close else (normal attack)
                    } else if should_chase && dist > profile.attack_range && can_move {
                        // #1691：追击——8 方向 A* 寻路（C# PathFinder.FindPath），
                        // 目标变更/路径失效才重算；贪心 step_toward 遇墙会卡住无法绕行
                        let mut path = self.monster_paths.entry(*oid).or_default();
                        let recalc = path.is_empty()
                            || self.monster_path_targets.get(oid)
                                .map(|(s, tx, ty)| *s != target_session || (tx - px).abs() > 3 || (ty - py).abs() > 3)
                                .unwrap_or(true)
                            || !self.maps.get(&monster.map_index)
                                .map(|m| m.is_walkable(path[0].0, path[0].1))
                                .unwrap_or(false);
                        if recalc {
                            *path = self.maps.get(&monster.map_index)
                                .and_then(|m| crate::maps::pathfind::find_path(m, (monster.x, monster.y), (px, py)))
                                .unwrap_or_default();
                            self.monster_path_targets.insert(*oid, (target_session, px, py));
                        }
                        // 沿路径走一步（首格被其他怪物占据则原地等待，路径保留）
                        if !path.is_empty() {
                            let candidate = path[0];
                            let walkable = self.maps.get(&monster.map_index)
                                .map(|m| m.is_walkable(candidate.0, candidate.1))
                                .unwrap_or(true);
                            if walkable && !monster_positions.contains(&candidate) && moved_targets.insert(candidate) {
                                let dir = (0..8)
                                    .find(|d| MON_DIR_DX[*d] == candidate.0 - monster.x && MON_DIR_DY[*d] == candidate.1 - monster.y)
                                    .unwrap_or(monster.direction as usize) as u8;
                                moved_monsters.push((*oid, candidate.0, candidate.1, dir));
                                path.remove(0);
                            }
                        }
                        monster.next_move_tick = self.tick_count + slow_adjusted_ticks(profile.move_interval, slowed);
                        monster.ai_state = MonsterAiState::Chase;
                    }
                } else if let Some(master) = monster.master_session {
                    // ===== 召唤物无目标 → 跟随主人 =====
                    // 简化版：有 master 且主人在线且距离>5 则 step_toward 主人位置
                    // C# CanMove/ProcessAI：仅 MoveOnly/Both/FocusMasterTarget 允许跟随/召回
                    //（AttackOnly/None 不移动，原地待命）
                    let pet_can_follow = self.player_pet_modes
                        .get(&master)
                        .map(|m| matches!(m, mir2_shared::enums::PetMode::MoveOnly
                            | mir2_shared::enums::PetMode::Both
                            | mir2_shared::enums::PetMode::FocusMasterTarget))
                        .unwrap_or(true); // 无缓存默认允许（C# 默认 Both）
                    if pet_can_follow && can_move {
                        let master_pos = player_positions.iter()
                            .find(|(sid, _, _, _, _, _, _, _, _)| *sid == master)
                            .map(|(_, x, y, _, _, _, _, pmap, _)| (*x, *y, *pmap));
                        if let Some((mx, my, master_map)) = master_pos {
                            // C# MonsterObject.ProcessAI：!InRange(Master, DataRange=16) 或跨图 → PetRecall（传送回主人）
                            let dist_master = (monster.x - mx).abs().max((monster.y - my).abs());
                            if monster.map_index != master_map || dist_master > 16 {
                                pet_recalls.push((*oid, mx, my, master_map));
                                monster.ai_state = MonsterAiState::Return;
                            } else if dist_master > 2 {
                                // #1693：宠物跟随主人（C# ProcessRoam → MoveTo(Master.Back) 用 PathFinder），
                                // A* 寻路到主人附近，保持 ~2 格；目标哨兵 session=u64::MAX 区分追击/回出生点
                                let mut path = self.monster_paths.entry(*oid).or_default();
                                let recalc = path.is_empty()
                                    || self.monster_path_targets.get(oid)
                                        .map(|(s, tx, ty)| *s != u64::MAX || *tx != mx || *ty != my)
                                        .unwrap_or(true)
                                    || !self.maps.get(&monster.map_index)
                                        .map(|m| m.is_walkable(path[0].0, path[0].1))
                                        .unwrap_or(false);
                                if recalc {
                                    *path = self.maps.get(&monster.map_index)
                                        .and_then(|m| crate::maps::pathfind::find_path(m, (monster.x, monster.y), (mx, my)))
                                        .unwrap_or_default();
                                    self.monster_path_targets.insert(*oid, (PATH_TARGET_PET_FOLLOW, mx, my));
                                }
                                if !path.is_empty() {
                                    let candidate = path[0];
                                    let walkable = self.maps.get(&monster.map_index)
                                        .map(|m| m.is_walkable(candidate.0, candidate.1))
                                        .unwrap_or(true);
                                    if walkable && !monster_positions.contains(&candidate) && moved_targets.insert(candidate) {
                                        let dir = (0..8)
                                            .find(|d| MON_DIR_DX[*d] == candidate.0 - monster.x && MON_DIR_DY[*d] == candidate.1 - monster.y)
                                            .unwrap_or(monster.direction as usize) as u8;
                                        moved_monsters.push((*oid, candidate.0, candidate.1, dir));
                                        path.remove(0);
                                    }
                                }
                                monster.next_move_tick = self.tick_count + slow_adjusted_ticks(profile.move_interval, slowed);
                                monster.ai_state = MonsterAiState::Return;
                            } else {
                                monster.ai_state = MonsterAiState::Idle;
                            }
                        } else {
                            // 主人离线：原地待命
                            monster.ai_state = MonsterAiState::Idle;
                        }
                    }
                } else if can_move && dist_to_spawn(monster) > 2 {
                    // #1691：无目标 → 回出生点（8 方向 A*，避免卡墙；目标哨兵 session=0）
                    let mut path = self.monster_paths.entry(*oid).or_default();
                    let recalc = path.is_empty()
                        || self.monster_path_targets.get(oid)
                            .map(|(s, tx, ty)| *s != 0 || *tx != monster.spawn_x || *ty != monster.spawn_y)
                            .unwrap_or(true)
                        || !self.maps.get(&monster.map_index)
                            .map(|m| m.is_walkable(path[0].0, path[0].1))
                            .unwrap_or(false);
                    if recalc {
                        *path = self.maps.get(&monster.map_index)
                            .and_then(|m| crate::maps::pathfind::find_path(m, (monster.x, monster.y), (monster.spawn_x, monster.spawn_y)))
                            .unwrap_or_default();
                        self.monster_path_targets.insert(*oid, (0, monster.spawn_x, monster.spawn_y));
                    }
                    if !path.is_empty() {
                        let candidate = path[0];
                        let walkable = self.maps.get(&monster.map_index)
                            .map(|m| m.is_walkable(candidate.0, candidate.1))
                            .unwrap_or(true);
                        if walkable && !monster_positions.contains(&candidate) && moved_targets.insert(candidate) {
                            let dir = (0..8)
                                .find(|d| MON_DIR_DX[*d] == candidate.0 - monster.x && MON_DIR_DY[*d] == candidate.1 - monster.y)
                                .unwrap_or(monster.direction as usize) as u8;
                            moved_monsters.push((*oid, candidate.0, candidate.1, dir));
                            path.remove(0);
                        }
                    }
                    monster.next_move_tick = self.tick_count + slow_adjusted_ticks(profile.move_interval, slowed);
                    monster.ai_state = MonsterAiState::Return;
                } else {
                    // C# ProcessRoam：无目标时按 RoamDelay(1s) 1/10 概率随机转身/走动
                    let roam_next = self.monster_roam_ticks.get(oid).copied().unwrap_or(0);
                    if can_move && self.tick_count >= roam_next {
                        self.monster_roam_ticks.insert(*oid, self.tick_count + ROAM_DELAY_TICKS);
                        if fastrand::i32(0..10) == 0 {
                            if fastrand::i32(0..3) == 0 {
                                // C# Turn：随机转身 + 广播 ObjectTurn
                                monster.direction = fastrand::i32(0..8) as u8;
                                monster_turns.push((*oid, monster.direction, monster.x, monster.y));
                            } else {
                                // C# Walk：沿当前方向走一步
                                let dir = monster.direction as usize % 8;
                                let (nx, ny) = (monster.x + MON_DIR_DX[dir], monster.y + MON_DIR_DY[dir]);
                                if self.maps.get(&monster.map_index).map(|m| m.is_walkable(nx, ny)).unwrap_or(false)
                                    && !monster_positions.contains(&(nx, ny))
                                    && moved_targets.insert((nx, ny))
                                {
                                    moved_monsters.push((*oid, nx, ny, monster.direction));
                                    monster.next_move_tick = self.tick_count + slow_adjusted_ticks(profile.move_interval, slowed);
                                }
                            }
                        }
                    }
                    monster.ai_state = MonsterAiState::Idle;
                }

                // C# MonsterObject.ProcessRegen：每 RegenDelay(10s) 回 2.2% max HP + 1（can_regen 时）
                if monster.behavior.can_regen() && monster.hp < monster.max_hp {
                    let next = self.monster_regen_ticks.get(oid).copied().unwrap_or(0);
                    if self.tick_count >= next {
                        self.monster_regen_ticks.insert(*oid, self.tick_count + 100); // 10s = 100 ticks
                        let regen = (monster.max_hp as f32 * 0.022) as i32 + 1;
                        monster.hp = (monster.hp + regen).min(monster.max_hp);
                    }
                }

                // 检查死亡
                if monster.hp <= 0 {
                    // C# EvilMir.Die：DragonLink 模式下死亡=睡眠 5 分钟（满血苏醒），而非真死
                    let is_dragon_evil_mir = self.dragon_state.as_ref()
                        .map(|d| d.evil_mir_oid == Some(*oid))
                        .unwrap_or(false);
                    if is_dragon_evil_mir {
                        let slept = monster.behavior.as_any_mut()
                            .and_then(|a| a.downcast_mut::<crate::actors::world::ai::bosses::evil_mir::EvilMirBehavior>())
                            .map(|b| {
                                b.sleep_on_death(self.tick_count);
                                monster.hp = monster.max_hp;
                                true
                            })
                            .unwrap_or(false);
                        if slept {
                            debug!("EvilMir #{} DragonLink: slept 5min instead of dying", *oid);
                        } else {
                            dead_monsters.push(*oid);
                        }
                    } else {
                        dead_monsters.push(*oid);
                    }
                }
            }

            // 应用 Healer 治疗（在循环外，避免借用冲突）
            for (target_oid, heal_amount) in &heal_actions {
                if let Some(target) = self.monsters.get_mut(target_oid) {
                    target.hp = (target.hp + *heal_amount).min(target.max_hp);
                }
            }

            // #471：宠物协战伤害（循环外应用，避免借用冲突）
            for (pid, tmid, damage, master) in &pet_attacks {
                // #1741：宠物位置/属性（在 get_mut 前读取，避免借用冲突）
                let (pet_x, pet_y) = self.monsters.get(pid).map(|m| (m.x, m.y)).unwrap_or((0, 0));
                let pet_stats = self.monsters.get(pid).map(|m| m.to_combat_stats()).unwrap_or_default();
                if let Some(tm) = self.monsters.get_mut(tmid) {
                    // #1768：宠物攻击按 C# MonsterObject.Attacked(MonsterObject) 结算——目标护甲减免 + 命中
                    let (actual, is_miss) = resolve_monster_vs_monster(
                        &pet_stats, &tm.to_combat_stats(), *damage,
                        mir2_shared::enums::DefenceType::AcAgility,
                    );
                    if is_miss {
                        // C# Attacked：Miss 广播，不造成伤害/不换目标（BroadcastDamageIndicator(Miss)）
                        let mut miss_body = Vec::new();
                        miss_body.extend_from_slice(&0i32.to_le_bytes());
                        miss_body.push(4u8); // damage_type = Miss
                        miss_body.extend_from_slice(&tm.object_id.to_le_bytes());
                        let miss_packet = build_packet_bytes(
                            mir2_shared::enums::ServerPacketIds::DamageIndicator as i16, &miss_body);
                        let miss_map = tm.map_index;
                        broadcast_to_map(&self.gate_ref, &self.players, miss_map, &miss_packet).await;
                        continue;
                    }
                    tm.take_damage(actual);
                    tm.provoked = true;
                    tm.target_session = Some(*master);
                    // #1741：C# EXPOwner = attacker.Master——宠物补刀击杀归属主人（经验/掉落）
                    tm.last_hitter_session = Some(*master);
                    // #1741：宠物协战命中反馈（与玩家/英雄/怪物一致：ObjectStruck/DamageIndicator/ObjectHealth）
                    let pet_x = if pet_x == 0 && pet_y == 0 { tm.x } else { pet_x };
                    let pet_y = if pet_x == 0 && pet_y == 0 { tm.y } else { pet_y };
                    tm.direction = crate::actors::world::ai::direction_towards(
                        tm.x, tm.y, pet_x, pet_y,
                    );
                    let mut struck_body = Vec::new();
                    struck_body.extend_from_slice(&tm.object_id.to_le_bytes());
                    struck_body.extend_from_slice(&pid.to_le_bytes());
                    struck_body.extend_from_slice(&(tm.x as u32).to_le_bytes());
                    struck_body.extend_from_slice(&(tm.y as u32).to_le_bytes());
                    struck_body.push(tm.direction);
                    let struck_packet = build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::ObjectStruck as i16, &struck_body);
                    let mut dmg_body = Vec::new();
                    dmg_body.extend_from_slice(&actual.to_le_bytes());
                    dmg_body.push(0u8); // damage_type = normal
                    dmg_body.extend_from_slice(&tm.object_id.to_le_bytes());
                    let dmg_packet = build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::DamageIndicator as i16, &dmg_body);
                    let percent = ((tm.hp.max(0) as f32 / tm.max_hp.max(1) as f32) * 100.0) as u8;
                    let mut health_body = Vec::new();
                    health_body.extend_from_slice(&tm.object_id.to_le_bytes());
                    health_body.push(percent);
                    health_body.extend_from_slice(&3u16.to_le_bytes()); // expire（秒）
                    let health_packet = build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::ObjectHealth as i16, &health_body);
                    let hit_map = tm.map_index;
                    broadcast_to_map(&self.gate_ref, &self.players, hit_map, &struck_packet).await;
                    broadcast_to_map(&self.gate_ref, &self.players, hit_map, &dmg_packet).await;
                    broadcast_to_map(&self.gate_ref, &self.players, hit_map, &health_packet).await;
                    debug!("Pet #{} assists hitting '{}' (#{}) for {} dmg (after armour)", pid, tm.name, tmid, actual);
                }
            }

            // #1013：怪物互伤伤害（C# StoneTrap 嘲讽后怪物攻击目标；循环外应用）
            for (aid, tmid, damage) in &monster_attacks {
                let attacker_stats = self.monsters.get(aid).map(|m| m.to_combat_stats()).unwrap_or_default();
                // #1770：攻击者位置/朝向（ObjectAttack 包体需要）
                let (ax, ay, adir) = self.monsters.get(aid)
                    .map(|m| (m.x, m.y, m.direction))
                    .unwrap_or((0, 0, 0));
                if let Some(tm) = self.monsters.get_mut(tmid) {
                    // #1768：怪物互伤按 C# MonsterObject.Attacked(MonsterObject) 结算——目标护甲减免 + 命中
                    let (actual, is_miss) = resolve_monster_vs_monster(
                        &attacker_stats, &tm.to_combat_stats(), *damage,
                        mir2_shared::enums::DefenceType::AcAgility,
                    );
                    // 广播 ObjectAttack（怪 A 攻击怪 B；#1770 补全 16B 包体）
                    let mut attack_body = Vec::new();
                    attack_body.extend_from_slice(&aid.to_le_bytes());
                    attack_body.extend_from_slice(&(ax as u32).to_le_bytes());
                    attack_body.extend_from_slice(&(ay as u32).to_le_bytes());
                    attack_body.push(adir);
                    attack_body.push(0u8); // spell
                    attack_body.push(0u8); // level
                    attack_body.push(0u8); // attack_type
                    let attack_packet = build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::ObjectAttack as i16, &attack_body);
                    // #1649：动画广播只发同图玩家（C# CurrentMap.Broadcast）
                    broadcast_to_map(&self.gate_ref, &self.players, tm.map_index, &attack_packet).await;
                    if is_miss {
                        // C# Attacked：护甲全挡/未命中 → BroadcastDamageIndicator(Miss)，不造成伤害
                        let mut miss_body = Vec::new();
                        miss_body.extend_from_slice(&0i32.to_le_bytes());
                        miss_body.push(4u8); // damage_type = Miss
                        miss_body.extend_from_slice(&tm.object_id.to_le_bytes());
                        let miss_packet = build_packet_bytes(
                            mir2_shared::enums::ServerPacketIds::DamageIndicator as i16, &miss_body);
                        let miss_map = tm.map_index;
                        broadcast_to_map(&self.gate_ref, &self.players, miss_map, &miss_packet).await;
                        continue;
                    }
                    tm.take_damage(actual);
                    tm.provoked = true;
                    // C# Attacked 命中广播：ObjectStruck + DamageIndicator(Hit) + ObjectHealth
                    let mut struck_body = Vec::new();
                    struck_body.extend_from_slice(&tm.object_id.to_le_bytes());
                    struck_body.extend_from_slice(&aid.to_le_bytes());
                    struck_body.extend_from_slice(&(tm.x as u32).to_le_bytes());
                    struck_body.extend_from_slice(&(tm.y as u32).to_le_bytes());
                    struck_body.push(tm.direction);
                    let struck_packet = build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::ObjectStruck as i16, &struck_body);
                    let mut dmg_body = Vec::new();
                    dmg_body.extend_from_slice(&actual.to_le_bytes());
                    dmg_body.push(0u8); // damage_type = normal
                    dmg_body.extend_from_slice(&tm.object_id.to_le_bytes());
                    let dmg_packet = build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::DamageIndicator as i16, &dmg_body);
                    let percent = ((tm.hp.max(0) as f32 / tm.max_hp.max(1) as f32) * 100.0) as u8;
                    let mut health_body = Vec::new();
                    health_body.extend_from_slice(&tm.object_id.to_le_bytes());
                    health_body.push(percent);
                    health_body.extend_from_slice(&3u16.to_le_bytes()); // expire（秒）
                    let health_packet = build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::ObjectHealth as i16, &health_body);
                    let hit_map = tm.map_index;
                    broadcast_to_map(&self.gate_ref, &self.players, hit_map, &struck_packet).await;
                    broadcast_to_map(&self.gate_ref, &self.players, hit_map, &dmg_packet).await;
                    broadcast_to_map(&self.gate_ref, &self.players, hit_map, &health_packet).await;
                    debug!("Monster #{} hits '{}' (#{}) for {} dmg (monster-vs-monster, after armour)", aid, tm.name, tmid, actual);
                }
            }

            // 应用 Summoner 召唤（在循环外创建新怪物）
            for spawn in &summon_spawns {
                let new_oid = self.alloc_object_id();
                let packet = build_object_monster_packet(spawn, new_oid, &spawn.name);
                // #1649：怪物生成/动画广播只发同图玩家（C# CurrentMap.Broadcast）
                broadcast_to_map(&self.gate_ref, &self.players, spawn.map_index, &packet).await;
                let monster_info_opt = self.monster_infos.get(&spawn.monster_index);
                let ai_profile = monster_info_opt
                    .map(MonsterAiProfile::from_info)
                    .unwrap_or_else(|| MonsterAiProfile {
                        ai_type: MonsterAiType::Aggressive,
                        aggro_range: 10,
                        attack_range: 1,
                        attack_cooldown: 5,
                        move_interval: 2,
                        flee_threshold: 0.0,
                    });
                let monster_level = monster_info_opt.map(|i| i.level).unwrap_or(0);
                let monster_effect = monster_info_opt.map(|i| i.effect).unwrap_or(0);
                self.monsters.insert(new_oid, MonsterState {
                    object_id: new_oid,
                    name: spawn.name.clone(),
                    image: spawn.image,
                    monster_index: spawn.monster_index,
                    x: spawn.x,
                    y: spawn.y,
                    direction: spawn.direction,
                    hp: spawn.hp,
                    max_hp: spawn.hp,
                    min_dmg: spawn.min_dmg,
                    max_dmg: spawn.max_dmg,
                    xp: spawn.xp,
                    spawn_x: spawn.x,
                    spawn_y: spawn.y,
                    spawn_spread: 0,
                    map_index: spawn.map_index,
                    next_attack_tick: 0,
                    next_move_tick: 0,
                    next_summon_tick: 0,
                    ai_profile,
                    ai_state: MonsterAiState::Idle,
                    sitting: false,
                    hidden: false,
                    sit_down_tick: 0,
                    target_session: None,
                    last_hitter_session: None,
                    provoked: false,
                    is_elite: false,
                    is_boss: false,
                    min_ac: 0,
                    max_ac: 0,
                    min_mac: 0,
                    max_mac: 0,
                    agility: 0,
                    accuracy: 0,
                    armour_rate: 1.0,
                    damage_rate: 1.0,
                    magic_resist: 0,
                    critical_rate: 0,
                    critical_damage: 0,
                    luck: 0,
                    reflect: 0,
                    level: monster_level,
                    effect: monster_effect,
                    damage_reduction_percent: 0,
                    poison_list: Vec::new(),
                    last_hit_damage: 0,
            undead: false,
                    master_session: None,
                                rarity: 0,
                                pet_experience: 0,
                                max_pet_level: 0,
                    recall_at_tick: 0,
                    behavior: crate::actors::world::ai::make_behavior(&spawn.name),
                });
                debug!("Summoned monster '{}' as #{} at ({},{})", spawn.name, new_oid, spawn.x, spawn.y);
            }

            // ===== 应用 Boss AI 输出队列 =====
            // Boss 显示/隐藏广播（C# ObjectShow/ObjectHide，如 Shinsu 形态切换）
            for (oid, visible) in boss_show_hide.drain(..) {
                self.broadcast_object_show_hide(oid, visible).await;
            }
            // #1354：Boss 坐下/起身广播（C# ObjectSitDown，如 FrostTiger 坐姿）
            for (oid, sx, sy, sdir, sitting) in boss_sit_down.drain(..) {
                self.broadcast_object_sit_down(oid, sx, sy, sdir, sitting).await;
            }
            // #1364：Boss 对象特效广播（C# ObjectEffect，如 DeathCrawlerBreath 吐息毒）
            for (oid, effect) in boss_effects.drain(..) {
                let map_idx = self.monsters.get(&oid).map(|m| m.map_index).unwrap_or(0);
                self.broadcast_object_effect(oid, effect, map_idx).await;
            }
            // #1391：净化玩家毒（C# PowerBead Effect==1 → PlayerActor.PurifyPoisons）
            for sid in boss_player_purges.drain(..) {
                if let Some(rec) = self.players.get(&sid) {
                    let _ = rec.actor_ref.ask(crate::actors::player::PurifyPoisons).await;
                }
            }
            // Boss 对玩家回血（C# MasterVampire 吸血主人 / Healer 治疗玩家）
            for (sid, amount) in boss_player_heals.drain(..) {
                if amount <= 0 { continue; }
                if let Some(r) = self.players.get(&sid) {
                    let _ = r.actor_ref.ask(crate::actors::player::Heal { amount }).await;
                }
            }
            // Boss 移动（合并到 moved_monsters 复用广播逻辑），校验 walkable 避免穿墙
            for (oid, nx, ny, dir) in boss_moves.drain(..) {
                // #1777：C# CanMove——Boss 受控制毒禁移动
                if self.monsters.get(&oid).map(|m| monster_control_blocked(&m.poison_list, self.monster_infos.get(&m.monster_index).map(|i| i.light).unwrap_or(0)).0).unwrap_or(false) {
                    continue;
                }
                let map_idx = self.monsters.get(&oid).map(|m| m.map_index).unwrap_or(0);
                let walkable = self.maps.get(&map_idx)
                    .map(|m| m.is_walkable(nx, ny))
                    .unwrap_or(true);
                if walkable {
                    moved_monsters.push((oid, nx, ny, dir));
                }
            }
            // Boss 后跳（#1801：C# SepHighArcher.BackStep——ObjectBackStep 广播 + 直接落位）
            for (oid, dir, max_dist) in boss_backsteps.drain(..) {
                let map_idx = self.monsters.get(&oid).map(|m| m.map_index).unwrap_or(0);
                let (sx, sy) = self.monsters.get(&oid).map(|m| (m.x, m.y)).unwrap_or((0, 0));
                let d = (dir as usize) % 8;
                let mut cx = sx;
                let mut cy = sy;
                let mut traveled = 0i32;
                for _ in 0..max_dist {
                    let nx = cx + MON_DIR_DX[d];
                    let ny = cy + MON_DIR_DY[d];
                    let ok = self.maps.get(&map_idx)
                        .map(|m| m.is_walkable(nx, ny))
                        .unwrap_or(false);
                    if !ok { break; }
                    cx = nx;
                    cy = ny;
                    traveled += 1;
                }
                if traveled == 0 { continue; }
                let p = mir2_shared::packets::server::movement::ObjectBackStep {
                    object_id: oid,
                    location_x: cx,
                    location_y: cy,
                    direction: mir2_shared::enums::MirDirection::try_from(dir).unwrap_or(mir2_shared::enums::MirDirection::Up),
                    distance: traveled,
                };
                let mut body = Vec::new();
                if p.write_body(&mut body).is_ok() {
                    let pkt = build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::ObjectBackStep as i16, &body);
                    broadcast_to_map(&self.gate_ref, &self.players, map_idx, &pkt).await;
                }
                if let Some(m) = self.monsters.get_mut(&oid) {
                    m.x = cx;
                    m.y = cy;
                }
            }
            // Boss 怪物自传送（C# MonsterObject.Teleport：ObjectTeleportOut → 更新位置 → ObjectTeleportIn）
            for (oid, tx, ty) in boss_monster_teleports.drain(..) {
                let (old_x, old_y, map_idx, dir) = self.monsters.get(&oid)
                    .map(|m| (m.x, m.y, m.map_index, m.direction))
                    .unwrap_or((0, 0, 0, 0));
                let walkable = self.maps.get(&map_idx)
                    .map(|m| m.is_walkable(tx, ty))
                    .unwrap_or(false);
                if !walkable {
                    continue;
                }
                let name = self.monsters.get(&oid).map(|m| m.name.as_str()).unwrap_or("");
                let ttype = Self::monster_teleport_type(name);
                // ObjectTeleportOut（旧位置）
                let mut out_body = Vec::new();
                out_body.extend_from_slice(&oid.to_le_bytes());
                out_body.push(ttype);
                out_body.extend_from_slice(&(old_x as u32).to_le_bytes());
                out_body.extend_from_slice(&(old_y as u32).to_le_bytes());
                let out_packet = build_packet_bytes(
                    mir2_shared::enums::ServerPacketIds::ObjectTeleportOut as i16, &out_body);
                broadcast_to_map(&self.gate_ref, &self.players, map_idx, &out_packet).await;
                // 直接更新位置（不走 ObjectWalk 移动动画）
                if let Some(m) = self.monsters.get_mut(&oid) {
                    m.x = tx;
                    m.y = ty;
                    m.direction = dir;
                }
                // ObjectTeleportIn（新位置）
                let mut in_body = Vec::new();
                in_body.extend_from_slice(&oid.to_le_bytes());
                in_body.push(ttype);
                in_body.extend_from_slice(&(tx as u32).to_le_bytes());
                in_body.extend_from_slice(&(ty as u32).to_le_bytes());
                let in_packet = build_packet_bytes(
                    mir2_shared::enums::ServerPacketIds::ObjectTeleportIn as i16, &in_body);
                broadcast_to_map(&self.gate_ref, &self.players, map_idx, &in_packet).await;
            }
            // Boss 给玩家加 buff（C# AddBuff：YinDevilNode/PowerBead 等）
            for (sid, buff) in boss_player_buffs.drain(..) {
                if let Some(record) = self.players.get(&sid) {
                    let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff { buff }).await;
                }
            }
            // Boss 攻击：广播 ObjectAttack + 对命中的玩家造成伤害
            for atk in &boss_attacks {
                // #1777：C# CanAttack——Boss 受控制毒禁攻击（仅拦截发起；已排程弹道照常落地，C# DelayedAction 同理）
                let atk_oid = match atk {
                    ai::AttackAction::Melee { attacker_oid, .. }
                    | ai::AttackAction::Range { attacker_oid, .. }
                    | ai::AttackAction::Aoe { attacker_oid, .. }
                    | ai::AttackAction::Line { attacker_oid, .. } => *attacker_oid,
                };
                if self.monsters.get(&atk_oid).map(|m| monster_control_blocked(&m.poison_list, self.monster_infos.get(&m.monster_index).map(|i| i.light).unwrap_or(0)).1).unwrap_or(false) {
                    continue;
                }
                // #1638：Boss 攻击只命中同图玩家（C# CurrentMap；Aoe/Line 动作不带 map，从攻击者查）
                let boss_map: u16 = match atk {
                    ai::AttackAction::Melee { attacker_oid, .. }
                    | ai::AttackAction::Range { attacker_oid, .. }
                    | ai::AttackAction::Aoe { attacker_oid, .. }
                    | ai::AttackAction::Line { attacker_oid, .. } => {
                        self.monsters.get(attacker_oid).map(|m| m.map_index).unwrap_or(0)
                    }
                };
                let (attacker_oid, targets, damage, spell_id, attack_type, atk_x, atk_y, atk_dir) = match atk {
                    ai::AttackAction::Melee { attacker_oid, target_session, damage, spell_id, attack_type } => {
                        (*attacker_oid, vec![*target_session], *damage, *spell_id, *attack_type, 0i32, 0i32, 0u8)
                    }
                    ai::AttackAction::Range { attacker_oid, target_session, damage, spell_id, .. } => {
                        (*attacker_oid, vec![*target_session], *damage, *spell_id, 0u8, 0i32, 0i32, 0u8)
                    }
                    ai::AttackAction::Aoe { attacker_oid, center_x, center_y, radius, damage, .. } => {
                        let tgts: Vec<u64> = player_positions.iter()
                            .filter(|(_, px, py, _, _, _, pmap, _, _)| {
                                let dx = (px - center_x).abs();
                                let dy = (py - center_y).abs();
                                dx.max(dy) <= *radius && *pmap == boss_map
                            })
                            .map(|(s, _, _, _, _, _, _, _, _)| *s)
                            .collect();
                        (*attacker_oid, tgts, *damage, 0u8, 0u8, *center_x, *center_y, 0u8)
                    }
                    // #1020：直线攻击（C# LineAttack：沿 direction 逐格命中）
                    ai::AttackAction::Line { attacker_oid, origin_x, origin_y, direction, range, damage, .. } => {
                        let dir = (*direction as usize) % 8;
                        let (ldx, ldy) = (MON_DIR_DX[dir], MON_DIR_DY[dir]);
                        let tgts: Vec<u64> = player_positions.iter()
                            .filter(|(_, px, py, _, _, _, pmap, _, _)| {
                                if *pmap != boss_map {
                                    return false;
                                }
                                for k in 1..=*range {
                                    if *px == origin_x + ldx * k && *py == origin_y + ldy * k {
                                        return true;
                                    }
                                }
                                false
                            })
                            .map(|(s, _, _, _, _, _, _, _, _)| *s)
                            .collect();
                        (*attacker_oid, tgts, *damage, 0u8, 0u8, *origin_x, *origin_y, *direction)
                    }
                };
                // 获取 Boss 位置用于广播
                let (boss_x, boss_y, boss_dir) = self.monsters.get(&attacker_oid)
                    .map(|m| (m.x, m.y, m.direction))
                    .unwrap_or((atk_x, atk_y, atk_dir));
                // #1699：远程攻击广播 ObjectRangeAttack（弹道）；近战/AOE/直线广播 ObjectAttack
                let attack_packet = if matches!(atk, ai::AttackAction::Range { .. }) {
                    let (target_oid, target_x, target_y) = targets.first()
                        .and_then(|sid| player_positions.iter()
                            .find(|(ps, _, _, _, _, _, _, _, _)| ps == sid)
                            .map(|(_, tx, ty, oid, _, _, _, _, _)| (*oid, *tx, *ty)))
                        .unwrap_or((0, 0, 0));
                    build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::ObjectRangeAttack as i16,
                        &build_object_range_attack_body(
                            attacker_oid, boss_x, boss_y, boss_dir,
                            target_oid, target_x, target_y, 0u8, spell_id,
                        ),
                    )
                } else {
                    let mut attack_body = Vec::new();
                    attack_body.extend_from_slice(&attacker_oid.to_le_bytes());
                    attack_body.extend_from_slice(&(boss_x as u32).to_le_bytes());
                    attack_body.extend_from_slice(&(boss_y as u32).to_le_bytes());
                    attack_body.push(boss_dir);
                    attack_body.push(spell_id);
                    attack_body.push(0u8); // level
                    attack_body.push(attack_type);
                    build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::ObjectAttack as i16, &attack_body)
                };
                // #1649：动画广播只发同图玩家（C# CurrentMap.Broadcast）
                broadcast_to_map(&self.gate_ref, &self.players, boss_map, &attack_packet).await;
                // 对命中玩家造成伤害
                if matches!(atk, ai::AttackAction::Range { .. }) {
                    // #1706：Boss 远程伤害按弹道延迟结算（C# DelayedAction RangeDamage；与 #1703 一致）
                    for sid in &targets {
                        let (tx, ty) = player_positions.iter()
                            .find(|(ps, _, _, _, _, _, _, _, _)| ps == sid)
                            .map(|(_, x, y, _, _, _, _, _, _)| (*x, *y))
                            .unwrap_or((boss_x, boss_y));
                        let dist = (boss_x - tx).abs().max((boss_y - ty).abs());
                        self.boss_ranged_pending.push(BossRangedPendingHit {
                            fire_tick: self.tick_count + crate::actors::world::combat::range_flight_ticks(dist),
                            attacker_oid,
                            target_session: *sid,
                            damage,
                            map_index: boss_map,
                        });
                    }
                } else {
                    let boss_stats = self.monsters.get(&attacker_oid)
                        .map(|m| m.to_combat_stats())
                        .unwrap_or_default();
                    let boss_level = self.monsters.get(&attacker_oid)
                        .map(|m| m.level)
                        .unwrap_or(0);
                    for sid in &targets {
                        if let Some(record) = self.players.get(sid) {
                            let boss_name = self.monsters.get(&attacker_oid)
                                .map(|m| m.name.as_str())
                                .unwrap_or("");
                            // #1721：Boss 攻击完整结算（C# Attacked：命中/护甲/反伤/减伤）
                            let mut is_critical = false;
                            let actual = if let Ok(Some(defender)) = record.actor_ref.ask(GetPlayerState).await {
                                let (actual, reflected, is_miss, crit) = resolve_monster_vs_player(
                                    &boss_stats, boss_level, &defender, damage,
                                    monster_melee_defence_type(boss_name),
                                );
                                is_critical = crit;
                                if reflected > 0 {
                                    if let Some(m) = self.monsters.get_mut(&attacker_oid) {
                                        m.take_damage(reflected);
                                        m.provoked = true;
                                    }
                                }
                                if is_miss {
                                    broadcast_miss_feedback(
                                        &self.players, &self.gate_ref, boss_map, *sid, defender.object_id,
                                    ).await;
                                    0
                                } else {
                                    actual
                                }
                            } else { damage };
                            let _ = record.actor_ref.ask(TakeDamage {
                                attacker_id: attacker_oid,
                                attacker_session: *sid,
                                damage: actual,
                            }).await;
                            // #1712：Boss 命中反馈——同图其他玩家看受击/飘字（C# Attacked → BroadcastDamageIndicator，用实际伤害）
                            if let Ok(Some(victim)) = record.actor_ref.ask(GetPlayerState).await {
                                broadcast_hit_feedback(
                                    &self.players, &self.gate_ref, boss_map, *sid,
                                    victim.object_id, victim.x, victim.y, victim.direction,
                                    attacker_oid, actual, if is_critical { 5 } else { 0 },
                                ).await;
                            }
                            // CounterAttack：受击方 7s 窗口激活时反击 Boss（C# HumanObject.cs 7212/7302）
                            if let Some((expire, lv)) = self.counter_attack.get(sid).copied() {
                                if self.tick_count <= expire {
                                    self.counter_attack.remove(sid);
                                    let counter_dmg = if let Ok(Some(vs)) = record.actor_ref.ask(GetPlayerState).await {
                                        crate::combat::attack::get_attack_power(
                                            vs.min_attack + vs.bonus_min_attack,
                                            vs.max_attack + vs.bonus_max_attack,
                                            vs.luck,
                                        ).max(1)
                                    } else { 1 };
                                    if let Some(m) = self.monsters.get_mut(&attacker_oid) {
                                        m.take_damage(counter_dmg);
                                        m.provoked = true;
                                        m.target_session = Some(*sid);
                                        crate::combat::poison::apply_poison(&mut m.poison_list,
                                            crate::combat::poison::Poison::new(
                                                mir2_shared::enums::PoisonType::STUN, lv as u32 + 1, 0, 1000,
                                            ));
                                        debug!("Player {} counter-attacked boss {} ({} dmg)", sid, attacker_oid, counter_dmg);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            // Boss 地面法术场：转为 SpellObject
            for sf in &boss_spell_fields {
                let oid = self.alloc_object_id();
                let spell_obj = spell::SpellObject::new(
                    oid, sf.spell, sf.caster_oid, sf.caster_session, 0,
                    sf.x, sf.y, sf.duration_ms, sf.value, sf.tick_ms, 1, sf.value,
                );
                self.spell_objects.insert(oid, spell_obj);
            }
            // Boss 召唤：按名称查 MonsterInfo 后生成（对齐 C# Envir.GetMonsterInfo(name)）
            for bs in &boss_summons {
                let mon_index = self.monster_name_index.get(&bs.monster_name.to_lowercase()).copied();
                if let Some(idx) = mon_index {
                    // 先 clone MonsterInfo 避免 &self.monster_infos 与 &mut self.alloc_object_id 借用冲突
                    let info_opt = self.monster_infos.get(&idx).cloned();
                    if let Some(info) = info_opt {
                        let new_oid = self.alloc_object_id();
                        let hp = info.stats.get(&(mir2_shared::enums::Stat::HP as u8)).copied().unwrap_or(50);
                        let min_dmg = info.stats.get(&(mir2_shared::enums::Stat::MinDC as u8)).copied().unwrap_or(5);
                        let max_dmg = info.stats.get(&(mir2_shared::enums::Stat::MaxDC as u8)).copied().unwrap_or(10);
                        let map_index = self.monsters.values().next().map(|m| m.map_index).unwrap_or(0);
                        // 广播新怪物生成
                        let spawn = MonsterSpawn {
                            name: info.name.clone(),
                            image: info.image as u16,
                            monster_index: idx,
                            x: bs.x,
                            y: bs.y,
                            direction: 0,
                            hp,
                            min_dmg,
                            max_dmg,
                            xp: info.experience,
                            map_index,
                            count: 1,
                            spread: 0,
                        };
                        let packet = build_object_monster_packet(&spawn, new_oid, &spawn.name);
                        // #1649：怪物生成/动画广播只发同图玩家（C# CurrentMap.Broadcast）
                        broadcast_to_map(&self.gate_ref, &self.players, spawn.map_index, &packet).await;
                        let ai_profile = MonsterAiProfile::from_info(&info);
                        self.monsters.insert(new_oid, MonsterState {
                            object_id: new_oid,
                            name: spawn.name.clone(),
                            image: spawn.image,
                            monster_index: idx,
                            x: bs.x, y: bs.y, direction: 0,
                            hp, max_hp: hp, min_dmg, max_dmg, xp: spawn.xp,
                            spawn_x: bs.x, spawn_y: bs.y, map_index,
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
            undead: false,
                            master_session: None,
                                rarity: 0,
                                pet_experience: 0,
                                max_pet_level: 0,
                            recall_at_tick: 0,
                            behavior: crate::actors::world::ai::make_behavior(&spawn.name),
                        });
                        // 填充战斗属性
                        if let Some(m) = self.monsters.get_mut(&new_oid) {
                            m.fill_combat_stats(&info);
                        }
                        // #1434：登记 slave 归属（C# SlaveList；master 死亡时级联清理）
                        if bs.is_slave {
                            if let Some(master) = bs.summoner_oid {
                                self.slave_master.insert(new_oid, master);
                            }
                        }
                        debug!("Boss summoned '{}' as #{} at ({},{}) slave={}", spawn.name, new_oid, bs.x, bs.y, bs.is_slave);
                    } else {
                        debug!("Boss summon '{}' found index {} but no MonsterInfo", bs.monster_name, idx);
                    }
                } else {
                    debug!("Boss summon '{}' not in monster_name_index (DB may lack this mob)", bs.monster_name);
                }
            }
            // #1437：TrapRock 子岩生成（C# TrapRock.Show：目标四角 ChildRock，立即可见、同目标、slave 级联）
            for cr in &boss_child_rocks {
                let mon_index = self.monster_name_index.get(&cr.monster_name.to_lowercase()).copied();
                if let Some(idx) = mon_index {
                    if let Some(info) = self.monster_infos.get(&idx).cloned() {
                        let new_oid = self.alloc_object_id();
                        let hp = info.stats.get(&(mir2_shared::enums::Stat::HP as u8)).copied().unwrap_or(50);
                        let min_dmg = info.stats.get(&(mir2_shared::enums::Stat::MinDC as u8)).copied().unwrap_or(5);
                        let max_dmg = info.stats.get(&(mir2_shared::enums::Stat::MaxDC as u8)).copied().unwrap_or(10);
                        let map_index = self.monsters.get(&cr.parent_oid).map(|m| m.map_index)
                            .or_else(|| self.monsters.values().next().map(|m| m.map_index))
                            .unwrap_or(0);
                        let spawn = MonsterSpawn {
                            name: info.name.clone(),
                            image: info.image as u16,
                            monster_index: idx,
                            x: cr.x,
                            y: cr.y,
                            direction: 0,
                            hp,
                            min_dmg,
                            max_dmg,
                            xp: info.experience,
                            map_index,
                            count: 1,
                            spread: 0,
                        };
                        let packet = build_object_monster_packet(&spawn, new_oid, &spawn.name);
                        // #1649：怪物生成/动画广播只发同图玩家（C# CurrentMap.Broadcast）
                        broadcast_to_map(&self.gate_ref, &self.players, spawn.map_index, &packet).await;
                        let ai_profile = MonsterAiProfile::from_info(&info);
                        // 子岩预设：shown=true、child=true、目标格锁定（C# ChildRock.Show）
                        let behavior = crate::actors::world::ai::bosses::trap_rock::TrapRockBehavior::child(
                            true,
                            (cr.target_x, cr.target_y),
                            cr.parent_oid,
                        );
                        self.monsters.insert(new_oid, MonsterState {
                            object_id: new_oid,
                            name: spawn.name.clone(),
                            image: spawn.image,
                            monster_index: idx,
                            x: cr.x, y: cr.y, direction: 0,
                            hp, max_hp: hp, min_dmg, max_dmg, xp: spawn.xp,
                            spawn_x: cr.x, spawn_y: cr.y, map_index,
                            spawn_spread: 0,
                            next_attack_tick: 0, next_move_tick: 0, next_summon_tick: 0,
                            ai_profile, ai_state: MonsterAiState::Idle,
                            sitting: false, hidden: false, sit_down_tick: 0,
                            target_session: Some(cr.target_session),
                            last_hitter_session: None, provoked: false,
                            is_elite: false, is_boss: false,
                            min_ac: 0, max_ac: 0, min_mac: 0, max_mac: 0,
                            agility: 0, accuracy: 0,
                            armour_rate: 1.0, damage_rate: 1.0,
                            magic_resist: 0, critical_rate: 0, critical_damage: 0,
                            luck: 0, reflect: 0, damage_reduction_percent: 0, level: info.level, effect: info.effect,
                            poison_list: Vec::new(),
                            last_hit_damage: 0, undead: false,
                            master_session: None, rarity: 0, pet_experience: 0, max_pet_level: 0,
                            recall_at_tick: 0,
                            behavior: Box::new(behavior),
                        });
                        // 登记 slave 归属（父岩死亡 → 子岩级联清理，C# SlaveList）
                        self.slave_master.insert(new_oid, cr.parent_oid);
                        debug!("TrapRock child '{}' #{} spawned at ({},{}) parent={}", spawn.name, new_oid, cr.x, cr.y, cr.parent_oid);
                    } else {
                        debug!("TrapRock child '{}' found index {} but no MonsterInfo", cr.monster_name, idx);
                    }
                } else {
                    debug!("TrapRock child '{}' not in monster_name_index", cr.monster_name);
                }
            }
            // Boss 对玩家的 poison
            for pp in &boss_poisons {
                if let Some(record) = self.players.get(&pp.session_id) {
                    let _ = record.actor_ref.ask(crate::actors::player::ApplyCombatPoisons {
                        poisons: vec![pp.poison],
                    }).await;
                }
            }
            // Boss 推开玩家（C# MapObject.Pushed：逐格校验 walkable，移动 + Pushed/ObjectPushed）
            for pp in &boss_pushes {
                let moved = self.push_player(pp.session_id, pp.dir, pp.distance).await;
                debug!("Boss pushed player {} {} tiles dir={}", pp.session_id, moved, pp.dir);
            }
            // Boss 延迟攻击：入队（C# DelayedAction DelayedType.Damage）
            for atk in &boss_delayed_attacks {
                self.boss_pending_attacks.push((self.tick_count + atk.delay_ticks, *atk));
            }
            // Boss 传送玩家（C# Target.Teleport：TurtleKing 拉拽等）
            for (sid, tx, ty, dir) in &boss_player_teleports {
                if let Some(record) = self.players.get(sid) {
                    if let Ok(Some(st)) = record.actor_ref.ask(GetPlayerState).await {
                        let walkable = self.maps.get(&st.map_index)
                            .map(|m| m.is_walkable(*tx, *ty))
                            .unwrap_or(false);
                        if walkable {
                            let _ = record.actor_ref.ask(crate::actors::player::SetPlayerPosition {
                                x: *tx, y: *ty, direction: *dir,
                                map_index: None, is_mounted: None,
                            }).await;
                            self.broadcast_player_teleport(*sid, st.x, st.y, *tx, *ty, *dir).await;
                            debug!("Boss teleported player {} to ({},{})", sid, tx, ty);
                        }
                    }
                }
            }
            // Boss 怪物互疗
            for (target_oid, amount) in &boss_heals {
                if let Some(m) = self.monsters.get_mut(target_oid) {
                    m.hp = (m.hp + *amount).min(m.max_hp);
                }
            }

            // 应用移动并广播
            for (oid, nx, ny, dir) in &moved_monsters {
                if let Some(m) = self.monsters.get_mut(oid) {
                    m.x = *nx;
                    m.y = *ny;
                    m.direction = *dir;

                    // 广播 ObjectWalk（object_id + x + y + direction，~12字节 vs ObjectMonster ~40字节）
                    debug!("Monster #{} moved to ({},{}) dir={} (broadcast)", oid, m.x, m.y, m.direction);
                    let mut walk_body = Vec::new();
                    walk_body.extend_from_slice(&oid.to_le_bytes());
                    walk_body.extend_from_slice(&m.x.to_le_bytes());
                    walk_body.extend_from_slice(&m.y.to_le_bytes());
                    walk_body.push(m.direction);
                    let walk_packet = build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::ObjectWalk as i16, &walk_body);
                    // #1649：动画广播只发同图玩家（C# CurrentMap.Broadcast）
                    broadcast_to_map(&self.gate_ref, &self.players, m.map_index, &walk_packet).await;
                }
            }


            // 宠物 PetRecall（C# MonsterObject.ProcessAI）：主人>16 格/跨图 → 传送到主人身边
            for (oid, tx, ty, mm) in &pet_recalls {
                if let Some(m) = self.monsters.get_mut(oid) {
                    // #1665：跨图召回 → 旧图 ObjectRemove + 新图 ObjectMonster（C# Teleport 重广播）
                    if m.map_index != *mm {
                        let old_map = m.map_index;
                        let pet_oid = m.object_id;
                        let spawn = MonsterSpawn {
                            name: m.name.clone(),
                            image: m.image,
                            monster_index: m.monster_index,
                            x: *tx,
                            y: *ty,
                            direction: m.direction,
                            hp: m.hp,
                            min_dmg: m.min_dmg,
                            max_dmg: m.max_dmg,
                            xp: m.xp,
                            map_index: *mm,
                            count: 1,
                            spread: 0,
                        };
                        let packet = build_object_monster_packet_extra(&spawn, pet_oid, &spawn.name, true, 0);
                        let mut rm = Vec::new();
                        rm.extend_from_slice(&pet_oid.to_le_bytes());
                        let remove_packet = build_packet_bytes(
                            mir2_shared::enums::ServerPacketIds::ObjectRemove as i16, &rm);
                        broadcast_to_map(&self.gate_ref, &self.players, old_map, &remove_packet).await;
                        broadcast_to_map(&self.gate_ref, &self.players, *mm, &packet).await;
                    }
                    // #1663：C# PetRecall 同步地图（MonsterObject.cs:1053 Teleport(Owner.CurrentMap)）
                    m.map_index = *mm;
                    m.x = *tx;
                    m.y = *ty;
                    let mut walk_body = Vec::new();
                    walk_body.extend_from_slice(&oid.to_le_bytes());
                    walk_body.extend_from_slice(&m.x.to_le_bytes());
                    walk_body.extend_from_slice(&m.y.to_le_bytes());
                    walk_body.push(m.direction);
                    let walk_packet = build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::ObjectWalk as i16, &walk_body);
                    // #1649：动画广播只发同图玩家（C# CurrentMap.Broadcast）
                    broadcast_to_map(&self.gate_ref, &self.players, m.map_index, &walk_packet).await;
                }
            }
            // 广播 ObjectTurn（C# ProcessRoam 转身；ObjectID + Location(i32,i32) + Direction(u8)）
            for (oid, dir, x, y) in &monster_turns {
                let mut turn_body = Vec::new();
                turn_body.extend_from_slice(&oid.to_le_bytes());
                turn_body.extend_from_slice(&x.to_le_bytes());
                turn_body.extend_from_slice(&y.to_le_bytes());
                turn_body.push(*dir);
                let turn_packet = build_packet_bytes(
                    mir2_shared::enums::ServerPacketIds::ObjectTurn as i16, &turn_body);
                // #1649：动画广播只发同图玩家（C# CurrentMap.Broadcast）
                broadcast_to_map(&self.gate_ref, &self.players, self.monsters.get(oid).map(|mm| mm.map_index).unwrap_or(0), &turn_packet).await;
            }

            // 处理破损装备广播（避免在怪物循环内借用 self）
            for (target_session, slot) in &broken_armor {
                if let Some(state) = self.recalculate_and_set_stat_bonuses(*target_session).await {
                    if *slot == EquipmentSlot::Weapon || *slot == EquipmentSlot::Armour {
                        self.broadcast_equipment_visuals(*target_session, &state).await;
                    }
                }
            }

            // 处理召唤物过期（无掉落，仅移除 + 广播 ObjectRemove）
            for oid in &expired_monsters {
                if let Some(monster) = self.monsters.remove(oid) {
                    self.monster_poison_flags.remove(oid);
                    let remove_packet = Self::build_object_remove_packet(*oid);
                    broadcast_to_map(&self.gate_ref, &self.players, monster.map_index, &remove_packet).await;
                    debug!("Summon '{}' (#{}) expired (recall_at_tick reached)", monster.name, oid);
                }
            }

            // 处理死亡怪物
            for oid in &dead_monsters {
                // #1369/#1399：死亡待复活/睡眠——保留尸体（不移除、不结算掉落/经验），首次发 ObjectDied
                let keep_for_revive = self.monsters.get(oid)
                    .map(|m| m.behavior.keep_corpse_for_revive())
                    .unwrap_or(false);
                if keep_for_revive {
                    if let Some(monster) = self.monsters.get_mut(oid) {
                        let first_died = monster.behavior.mark_death_announced();
                        if first_died {
                            let died_packet = Self::build_object_died_packet(
                                *oid, monster.x, monster.y, monster.direction,
                                    Self::monster_death_type(&monster.name, monster.master_session.is_some()),
                                );
                            broadcast_to_map(&self.gate_ref, &self.players, monster.map_index, &died_packet).await;
                            debug!("Monster #{} died, corpse kept for revive", oid);
                        }
                    }
                    continue;
                }
                if let Some(monster) = self.monsters.remove(oid) {
                    self.monster_poison_flags.remove(oid);
                    // #1434：C# MonsterObject.SlaveList——master 死亡，其召唤物（含多级）一起清理
                    let slave_oids = collect_slave_cascade(*oid, &self.slave_master);
                    for soid in slave_oids {
                        self.slave_master.remove(&soid);
                        if let Some(slave) = self.monsters.remove(&soid) {
                            self.monster_poison_flags.remove(&soid);
                            let slave_died = Self::build_object_died_packet(
                                soid, slave.x, slave.y, slave.direction,
                                Self::monster_death_type(&slave.name, slave.master_session.is_some()),
                            );
                            let slave_removed = Self::build_object_remove_packet(soid);
                            broadcast_to_map(&self.gate_ref, &self.players, slave.map_index, &slave_died).await;
                            broadcast_to_map(&self.gate_ref, &self.players, slave.map_index, &slave_removed).await;
                            debug!("Slave #{} died with master #{}", soid, oid);
                        }
                    }
                    debug!("Monster '{}' (#{}) died", monster.name, oid);

                    // ===== on_die 集成 =====
                    // 1. HellKnight 死亡 → 推进同地图 HellLord 阶段
                    let monster_name_lower = monster.name.to_lowercase();
                    if monster_name_lower.contains("hellknight") {
                        // C# KnightKilled：HellKnight 死亡 → HellLord stage+1 + 狂暴 2min
                        let helllord_oids: Vec<u32> = self.monsters.iter()
                            .filter(|(_, m)| m.name.to_lowercase().contains("helllord"))
                            .map(|(id, _)| *id)
                            .collect();
                        for hl_oid in helllord_oids {
                            let advanced = {
                                let lord = self.monsters.get_mut(&hl_oid);
                                match lord {
                                    Some(lord) => lord.behavior.as_any_mut()
                                        .and_then(|a| a.downcast_mut::<crate::actors::world::ai::bosses::hell_lord::HellLordBehavior>())
                                        .map(|hl| hl.advance_stage(self.tick_count)),
                                    None => None,
                                }
                            };
                            let _ = advanced;
                            debug!("HellKnight died, HellLord #{} stage advanced", hl_oid);
                        }
                    }
                    // 发送 ObjectDied（死亡动画）
                    let died_packet = Self::build_object_died_packet(
                        *oid, monster.x, monster.y, monster.direction,
                                    Self::monster_death_type(&monster.name, monster.master_session.is_some()),
                                );
                    // 发送 ObjectRemove（清理实体）
                    let remove_packet = Self::build_object_remove_packet(*oid);
                    broadcast_to_map(&self.gate_ref, &self.players, monster.map_index, &died_packet).await;
                    broadcast_to_map(&self.gate_ref, &self.players, monster.map_index, &remove_packet).await;

                    // 生成掉落物品
                    self.spawn_monster_drops(&monster).await;

                    // #1001/#1003：任务击杀进度（C# MonsterObject.Die → EXPOwner.CheckGroupQuestKill）
                    // 击杀者 = target_session（最后命中者，与掉落归属一致）；同组同图 16 格内未死成员共享
                    // #1016：击杀归属用 LastHitter（C# EXPOwner），回退 target_session
                    if let Some(killer) = monster.last_hitter_session.or(monster.target_session) {
                        // 击杀者 + 同组同图 16 格内未死成员（C# CheckGroupQuestKill）
                        let quest_sessions = self.quest_participants(
                            killer, monster.map_index, monster.x, monster.y).await;
                        for sid in quest_sessions {
                            if let Some(record) = self.players.get(&sid) {
                                let updates = record.actor_ref.ask(crate::actors::player::ProcessKillQuest {
                                    monster_index: monster.monster_index,
                                }).await.unwrap_or_default();
                                if !updates.is_empty() {
                                    for (quest_index, _, _) in &updates {
                                        if let Ok(Some(q)) = record.actor_ref.ask(crate::actors::player::GetQuest {
                                            quest_index: *quest_index,
                                        }).await {
                                            crate::actors::social_packets::send_quest_change_packet(
                                                &self.gate_ref, sid, &q);
                                        }
                                    }
                                    send_system_message(&self.gate_ref, sid, "任务进度更新：击杀目标");
                                }
                            }
                        }
                    }

                    // 世界Boss被击败广播
                    if monster.is_boss {
                        self.world_boss_queue.remove(oid);
                        broadcast_system_message(
                            &self.gate_ref, &self.players,
                            &format!("世界Boss {} 被英勇的勇士们击败了！", monster.name));
                        debug!("World boss '{}' defeated", monster.name);
                    }

                    // 发放经验（支持组队平分）
                    // C#：经验归属 LastHitter（最后造成伤害者），组内平分；
                    // 无 last_hitter 记录时回退 target_session（召唤物/宠物等路径）
                    let mut nearest_session: Option<u64> = None;
                    let mut nearest_group_id: Option<u64> = None;
                    let mut killer_level: u16 = 0;
                    let killer = monster.last_hitter_session.or(monster.target_session);
                    if let Some(sid) = killer.filter(|sid| self.players.contains_key(sid)) {
                        if let Some(record) = self.players.get(&sid) {
                            if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                                if !state.is_dead {
                                    nearest_session = Some(sid);
                                    nearest_group_id = state.group_id;
                                    killer_level = state.level;
                                }
                            }
                        }
                    }
                    if let Some(session_id) = nearest_session {
                        if let Some(group_id) = nearest_group_id {
                            // 组队经验：组内所有在线成员平分
                            let mut group_sessions = Vec::new();
                            for (sid, record) in &self.players {
                                if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                                    if state.group_id == Some(group_id) {
                                        // C# WinExp：Functions.InRange(..., Globals.DataRange=16) 切比雪夫
                                        if in_range(state.x, state.y, monster.x, monster.y, 16)
                                            && state.map_index == monster.map_index {
                                            group_sessions.push(*sid);
                                        }
                                    }
                                }
                            }
                            if !group_sessions.is_empty() {
                                // C# WinExp：先按击杀者等级做等级差衰减，再按 partyExpRate × 等级权重分配
                                let mon_level = self.monster_infos.get(&monster.monster_index).map(|m| m.level).unwrap_or(0);
                                let xp_after_reduce = reduce_exp(monster.xp, killer_level, mon_level);
                                let mut sum_level = 0i32;
                                let mut member_levels: Vec<(u64, u16, bool)> = Vec::new();
                                for sid in &group_sessions {
                                    if let Some(record) = self.players.get(sid) {
                                        if let Ok(Some(st)) = record.actor_ref.ask(GetPlayerState).await {
                                            sum_level += st.level as i32;
                                            member_levels.push((*sid, st.level, !st.is_dead));
                                        }
                                    }
                                }
                                let near_count = member_levels.len().clamp(1, PARTY_EXP_RATE.len());
                                let rate = PARTY_EXP_RATE[near_count - 1];
                                for (sid, lv, alive) in &member_levels {
                                    // C# WinExp：仅存活成员获得经验（死亡成员等级仍计入 sumLevel）
                                    if !*alive {
                                        continue;
                                    }
                                    let share = party_exp_share(xp_after_reduce, rate, *lv, sum_level);
                                    if let Some(record) = self.players.get(sid).cloned() {
                                        let gained = record.actor_ref.ask(crate::actors::player::AddExperience {
                                            amount: self.apply_global_exp_multiplier(share),
                                            experience_list: self.experience_list.clone(),
                                        }).await.unwrap_or(0);
                                        // C# GainExp：玩家获得经验时，同图 InRange(16) 存活宠物获得同等经验
                                        if gained > 0 {
                                            grant_pet_exp(self, *sid, gained, monster.map_index, monster.x, monster.y).await;
                                        }
                                    }
                                }
                                debug!("GroupXP: {} members split {} xp (rate={}) from '{}' (reduced from {})", member_levels.len(), xp_after_reduce, rate, monster.name, monster.xp);
                            }
                            // 组队任务击杀进度
                            for sid in &group_sessions {
                                if let Some(record) = self.players.get(sid) {
                                    let updates = record.actor_ref.ask(crate::actors::player::ProcessMonsterKill {
                                        monster_index: monster.monster_index,
                                    }).await.unwrap_or_default();
                                    if !updates.is_empty() {
                                        send_system_message(&self.gate_ref, *sid, &format!("任务进度更新：击杀了 {}", monster.name));
                                    }
                                    for (quest_index, _mid, complete) in updates {
                                        debug!("QuestKill: session={} quest={} monster={} complete={}", sid, quest_index, monster.monster_index, complete);
                                    }
                                }
                            }
                        } else if let Some(record) = self.players.get(&session_id).cloned() {
                            // C# WinExp：单人路径同样做等级差衰减
                            let mon_level = self.monster_infos.get(&monster.monster_index).map(|m| m.level).unwrap_or(0);
                            let xp_after = if let Ok(Some(st)) = record.actor_ref.ask(GetPlayerState).await {
                                reduce_exp(monster.xp, st.level, mon_level)
                            } else {
                                monster.xp
                            };
                            let gained = record.actor_ref.ask(crate::actors::player::AddExperience {
                                amount: self.apply_global_exp_multiplier(xp_after),
                                experience_list: self.experience_list.clone(),
                            }).await.unwrap_or(0);
                            // C# GainExp：玩家获得经验时，同图 InRange(16) 存活宠物获得同等经验
                            if gained > 0 {
                                grant_pet_exp(self, session_id, gained, monster.map_index, monster.x, monster.y).await;
                            }
                            // 单人任务击杀进度
                            let updates = record.actor_ref.ask(crate::actors::player::ProcessMonsterKill {
                                monster_index: monster.monster_index,
                            }).await.unwrap_or_default();
                            if !updates.is_empty() {
                                send_system_message(&self.gate_ref, session_id, &format!("任务进度更新：击杀了 {}", monster.name));
                            }
                            for (quest_index, _mid, complete) in updates {
                                debug!("QuestKill: session={} quest={} monster={} complete={}", session_id, quest_index, monster.monster_index, complete);
                            }
                        }

                        // #1163/#1142：C# PlayerObject.WinExp 末尾——HeroSpawned && !Hero.Dead 时英雄分得经验
                        //（Hero.ReduceExp(amount, targetLevel) * ExpRate → Hero.GainExp → S.GainHeroExperience）
                        if let Some(record) = self.players.get(&session_id).cloned() {
                            if let Ok(Some(st)) = record.actor_ref.ask(GetPlayerState).await {
                                if st.hero_index > 0 && !st.is_dead {
                                    let hero_level = self.player_heroes.get(&session_id)
                                        .and_then(|hs| hs.iter().find(|h| h.index as u8 == st.hero_index))
                                        .map(|h| h.level)
                                        .unwrap_or(1);
                                    let mon_level = self.monster_infos.get(&monster.monster_index)
                                        .map(|m| m.level).unwrap_or(0);
                                    let share = reduce_exp(monster.xp, hero_level, mon_level);
                                    let share = self.apply_global_exp_multiplier(share).max(0) as u32;
                                    if share > 0 {
                                        self.grant_hero_experience(session_id, share).await;
                                    }
                                }
                            }
                        }
                    }

                    // 加入重生队列（延迟从 map_respawns.delay（秒）读取；C# RespawnInfo.Delay）
                    // #1017：C# Map.cs——delay = max(1, Delay - RandomDelay + Random.Next(RandomDelay*2))
                    let respawn_delay_ticks: u64 = self.map_infos.get(&(monster.map_index as i32))
                        .and_then(|mi| mi.respawns.iter().find(|r| {
                            r.monster_index == monster.monster_index && r.x == monster.spawn_x && r.y == monster.spawn_y
                        }))
                        .map(|r| {
                            let base = r.delay.max(1) as i64;
                            let rd = r.random_delay.max(0) as i64;
                            let secs = if rd > 0 {
                                (base - rd + fastrand::i64(0..(rd * 2))).max(1)
                            } else {
                                base
                            };
                            (secs as u64) * 10
                        })
                        .unwrap_or(1800);
                    let respawn_tick = self.tick_count + respawn_delay_ticks;
                    let spawn = MonsterSpawn {
                        name: monster.name.clone(),
                        image: monster.image,
                        monster_index: monster.monster_index,
                        x: monster.spawn_x,
                        y: monster.spawn_y,
                        direction: monster.direction,
                        hp: monster.max_hp,
                        min_dmg: monster.min_dmg,
                        max_dmg: monster.max_dmg,
                        xp: monster.xp,
                        map_index: monster.map_index,
                        count: 1,
                        spread: monster.spawn_spread,
                    };
                    self.respawn_queue.insert(*oid, (spawn, respawn_tick));
                    // 死亡回调（C# Die 覆盖：HumanAssassin 爆炸 / KingHydrax 召唤等）——
                    // 入队由独立消息 ProcessDeathCallbacks 处理（避免 Tick handler 巨型状态机栈溢出）
                    self.pending_death_callbacks.push((monster, player_positions.clone()));
                }
            }

            // 处理玩家死亡掉落（在怪物循环外，避免借用冲突）
            for (sid, x, y, map_index) in death_drops {
                self.handle_player_death_drop(sid, x, y, map_index, false).await;
            }
            // 处理被怪物攻击后的自动下坐骑（在怪物循环外，避免借用冲突）
            for sid in dismount_sessions {
                self.dismount_player(sid).await;
            }
        }

        self.tick_buffs_and_revive().await;

        self.tick_potion_pools().await;

        self.tick_environment_damage().await;

        self.tick_exp_events_and_invisibility().await;

        self.tick_pk_decay().await;
        self.tick_rested().await;

        self.tick_fishing().await;

        self.tick_ground_cleanup().await;

        self.tick_respawn().await;

        self.tick_boss_timeout().await;

        self.tick_quest_timeout().await;

        self.tick_pet_pickup().await;

        self.tick_shop_restock().await;

        self.tick_refine_complete().await;
        self.tick_regen_and_hunger().await;

        // #898：安全区回血由独立消息处理（避免内联进 Tick handler 巨型状态机导致 tokio 栈溢出，#881 经验）
        if self.safe_zone_healing {
            if let Some(world_ref) = self.self_ref.clone() {
                let _ = world_ref.tell(ProcessSafeZoneHealing).try_send();
            }
        }

        self.tick_day_night().await;

        self.tick_auto_save().await;

        self.tick_item_expiry().await;

        self.tick_auction_expiry().await;

        self.tick_rental_expiry().await;

        self.tick_spells().await;

        self.tick_boss_ranged_pending().await;
        self.tick_ranged_pending().await;
        self.tick_spell_completions().await;

        // #1560：弓手远程攻击延迟结算（箭矢飞行后伤害落地 / Miss）
        self.tick_range_completions().await;

        self.tick_heroes().await;

        self.tick_robots().await;

        self.tick_dragon().await;

        self.tick_guild_wars().await;

        self.tick_conquest().await;
    }
}

#[cfg(test)]
mod tests {
    use super::{
        dotnet_now_ticks, in_range, item_expired, party_exp_share, pet_exp_gain, reduce_exp,
        collect_slave_cascade, safe_zone_heal_hp, boss_range_defence_type, monster_melee_defence_type,
        build_object_range_attack_body, resolve_monster_vs_monster, slow_adjusted_ticks,
        monster_control_blocked, combined_poison_flags,
        PARTY_EXP_RATE,
    };

    #[test]
    fn test_natural_regen_amount() {
        use super::WorldActor;
        // #1283：C# ProcessRegen——每 10s 回 (max*3%+1) + 该值*Recovery/Weight（非 ×10）
        // max=1000, recovery=0, weight=10：30+1 = 31
        assert_eq!(WorldActor::natural_regen_amount(1000, 0, 10), 31);
        // max=1000, recovery=10, weight=10：31 + 31*10/10 = 62
        assert_eq!(WorldActor::natural_regen_amount(1000, 10, 10), 62);
        // max=100, recovery=0：3+1 = 4
        assert_eq!(WorldActor::natural_regen_amount(100, 0, 10), 4);
        // weight=0 → max(1) 防除零
        assert_eq!(WorldActor::natural_regen_amount(100, 10, 0), 4 + 4 * 10 / 1);
    }

    #[test]
    fn test_safe_zone_heal_hp() {
        // #898：C# 安全区 Healing SpellObject Value=25 / TickSpeed=2000ms
        assert_eq!(safe_zone_heal_hp(100, 500), 125);
        assert_eq!(safe_zone_heal_hp(490, 500), 500); // 不超过 max_hp
        assert_eq!(safe_zone_heal_hp(500, 500), 500); // 满血不溢出
        assert_eq!(safe_zone_heal_hp(0, 500), 25);
    }

    #[test]
    fn test_reduce_exp() {
        // #914：C# ReduceExp——等级 < 怪物+10 不减；否则扣 Round(Max(amount/15,1)*diff)，最低 1
        assert_eq!(reduce_exp(1000, 30, 25), 1000); // 30 < 35：不减
        assert_eq!(reduce_exp(1000, 35, 25), 1000); // 35 == 35：不减
        // 40 级打 25 级怪：diff=5，penalty=Round(66.67*5)=333 → 667
        assert_eq!(reduce_exp(1000, 40, 25), 667);
        // 100 级打 1 级怪：diff=89，penalty 很大 → 最低 1
        assert_eq!(reduce_exp(100, 100, 1), 1);
        // amount 很小时 penalty 至少 1*diff
        assert_eq!(reduce_exp(5, 40, 25), 1); // 5 - 5 = 0 → 最低 1
    }

    #[test]
    fn test_party_exp_share() {
        // #914：C# WinExp 组队分配 expPoint * rate * memberLevel / sumLevel
        let xp = 1000;
        // 2 人：50/50，nearCount=2 → rate 1.3
        let sum = 50 + 50;
        assert_eq!(party_exp_share(xp, PARTY_EXP_RATE[1], 50, sum), 650);
        assert_eq!(party_exp_share(xp, PARTY_EXP_RATE[1], 50, sum), 650);
        // 等级权重：30 级 vs 70 级（sum=100，rate 1.0 单人兜底）
        assert_eq!(party_exp_share(1000, 1.0, 30, 100), 300);
        assert_eq!(party_exp_share(1000, 1.0, 70, 100), 700);
        // sum_level<=0 回退全额
        assert_eq!(party_exp_share(1000, 1.0, 50, 0), 1000);
    }

    #[test]
    fn test_item_expired() {
        // #916：C# ExpireInfo.ExpiryDate(DateTime.ToBinary) <= Envir.Now；Kind 位需掩码
        let now = dotnet_now_ticks();
        // 过去时间（含 Local Kind 高位 0x8000...）→ 已过期
        let past = (now - 60_000) | i64::MIN; // Local Kind 位 = 0x8000_0000_0000_0000
        assert!(item_expired(past, now));
        // 未来时间 → 未过期
        let future = (now + 60_000) | 0x4000_0000_0000_0000i64; // UTC Kind 位
        assert!(!item_expired(future, now));
        // 精确相等 → 过期（C# <=）
        assert!(item_expired(now, now));
        // 0（无时间）→ 视为已过期（C# DateTime.MinValue <= Now 恒真；调用方仅在字段存在时调用）
        assert!(item_expired(0, now));
    }

    #[test]
    fn test_dotnet_now_ticks_sane() {
        // #916：.NET Ticks 应大于 2023 年基线（638000000000000000）
        let now = dotnet_now_ticks();
        assert!(now > 638_000_000_000_000_000, "unexpected ticks: {}", now);
    }

    /// #1159：C# Functions.InRange = 切比雪夫（DataRange=16），斜向成员不应被曼哈顿误排除
    #[test]
    fn test_in_range_chebyshev() {
        // 斜向 10,10：曼哈顿=20 > 旧 12 会误排除；切比雪夫=10 <= 16 应包含
        assert!(in_range(0, 0, 10, 10, 16));
        // 正斜 16,0：切比雪夫=16 边界包含
        assert!(in_range(0, 0, 16, 0, 16));
        // 超界 17,0 排除
        assert!(!in_range(0, 0, 17, 0, 16));
        // 斜向 12,12：曼哈顿 24 超旧 12；切比雪夫 12 <= 16 包含
        assert!(in_range(0, 0, 12, 12, 16));
        assert!(!in_range(0, 0, 12, 12, 11));
    }

    /// #1159：C# WinExp 组队单成员分配 = exp * rate * level / sumLevel（等级权重）
    #[test]
    fn test_party_exp_share_level_weighted() {
        // 1000 xp、rate=1.0、2 人（30 级 + 70 级，sum=100）
        let share_low = party_exp_share(1000, 1.0, 30, 100);
        let share_high = party_exp_share(1000, 1.0, 70, 100);
        assert_eq!(share_low, 300);
        assert_eq!(share_high, 700);
        assert_eq!(share_low + share_high, 1000);
        // rate=1.3（2 人）：C# partyExpRate[1]=1.3
        assert_eq!(party_exp_share(1000, 1.3, 50, 100), 650);
        // sum=0 兜底返回原值
        assert_eq!(party_exp_share(1000, 1.0, 30, 0), 1000);
    }

    /// #1161：C# MonsterObject.PetExp——Skeleton/Shinsu/Angel 经验 ×3，其它 ×1
    #[test]
    fn test_pet_exp_gain_triple_for_special() {
        assert_eq!(pet_exp_gain("Skeleton", 100), 300);
        assert_eq!(pet_exp_gain("Shinsu", 100), 300);
        assert_eq!(pet_exp_gain("Angel", 100), 300);
        assert_eq!(pet_exp_gain("HolyDeva", 100), 100);
        assert_eq!(pet_exp_gain("Vampire", 100), 100);
    }
    /// #1434：C# MonsterObject.SlaveList——master 死亡级联清理（含多级）
    #[test]
    fn test_collect_slave_cascade_multilevel() {
        let mut sm = std::collections::HashMap::new();
        // master 1 → slaves 10, 20；20 → slave 30（多级）
        sm.insert(10, 1);
        sm.insert(20, 1);
        sm.insert(30, 20);
        // 无关怪物不参与
        sm.insert(99, 7);
        let mut got = collect_slave_cascade(1, &sm);
        got.sort();
        assert_eq!(got, vec![10, 20, 30]);
        // 单级：master 7 → slave 99
        assert_eq!(collect_slave_cascade(7, &sm), vec![99]);
        // 无 slave 的 master → 空
        assert!(collect_slave_cascade(42, &sm).is_empty());
    }

    /// #1759：Boss Range 伤害防御类型按 C# 配置映射（大小写不敏感）
    #[test]
    fn test_boss_range_defence_type() {
        use mir2_shared::enums::DefenceType;
        // MACAgility 组（远程法术）
        assert_eq!(boss_range_defence_type("AntCommander"), DefenceType::MacAgility);
        assert_eq!(boss_range_defence_type("WitchDoctor"), DefenceType::MacAgility);
        assert_eq!(boss_range_defence_type("peacockspider"), DefenceType::MacAgility);
        // MAC 组（远程魔法）
        assert_eq!(boss_range_defence_type("BurningZombie"), DefenceType::Mac);
        assert_eq!(boss_range_defence_type("Kirin"), DefenceType::Mac);
        assert_eq!(boss_range_defence_type("turtleking"), DefenceType::Mac);
        // AC 组（远程物理）
        assert_eq!(boss_range_defence_type("Armadillo"), DefenceType::Ac);
        assert_eq!(boss_range_defence_type("HedgeKekTal"), DefenceType::Ac);
        // ACAgility：显式 C# 组 + 未收录回退
        assert_eq!(boss_range_defence_type("AncientBringer"), DefenceType::AcAgility);
        assert_eq!(boss_range_defence_type("SomeUnknownMonster"), DefenceType::AcAgility);
        assert_eq!(boss_range_defence_type(""), DefenceType::AcAgility);
        // #1761：TucsonGeneral 远程主分支是 ProjectileAttack MACAgility
        assert_eq!(boss_range_defence_type("TucsonGeneral"), DefenceType::MacAgility);
    }

    /// #1761/#1763：怪物近战/AOE/直线伤害防御类型按 C# 配置映射（大小写不敏感）
    #[test]
    fn test_monster_melee_defence_type() {
        use mir2_shared::enums::DefenceType;
        // MACAgility 组（近战/直线法术）
        assert_eq!(monster_melee_defence_type("BurningZombie"), DefenceType::MacAgility);
        assert_eq!(monster_melee_defence_type("RestlessJar"), DefenceType::MacAgility);
        assert_eq!(monster_melee_defence_type("zumataurus"), DefenceType::MacAgility);
        assert_eq!(monster_melee_defence_type("CaveMaggot"), DefenceType::MacAgility);
        assert_eq!(monster_melee_defence_type("HornedMage"), DefenceType::MacAgility);
        assert_eq!(monster_melee_defence_type("IncarnatedZT"), DefenceType::MacAgility);
        assert_eq!(monster_melee_defence_type("ToxicGhoul"), DefenceType::MacAgility);
        // MAC 组（近战/AOE 魔法）
        assert_eq!(monster_melee_defence_type("CharmedSnake"), DefenceType::Mac);
        assert_eq!(monster_melee_defence_type("EvilCentipede"), DefenceType::Mac);
        assert_eq!(monster_melee_defence_type("EarthGolem"), DefenceType::Mac);
        assert_eq!(monster_melee_defence_type("IceGuard"), DefenceType::Mac);
        assert_eq!(monster_melee_defence_type("SepHighArcher"), DefenceType::Mac);
        // AC 组（近战物理）
        assert_eq!(monster_melee_defence_type("CannibalTentacles"), DefenceType::Ac);
        assert_eq!(monster_melee_defence_type("Kirin"), DefenceType::Ac);
        assert_eq!(monster_melee_defence_type("turtlegrass"), DefenceType::Ac);
        assert_eq!(monster_melee_defence_type("StoneGolem"), DefenceType::Ac);
        // 混合型/未收录回退 ACAgility
        assert_eq!(monster_melee_defence_type("OmaKing"), DefenceType::AcAgility);
        assert_eq!(monster_melee_defence_type("DemonWolf"), DefenceType::AcAgility);
        assert_eq!(monster_melee_defence_type("ScalyBeast"), DefenceType::AcAgility);
        assert_eq!(monster_melee_defence_type("SandSnail"), DefenceType::AcAgility);
        assert_eq!(monster_melee_defence_type("SomeUnknownMonster"), DefenceType::AcAgility);
        assert_eq!(monster_melee_defence_type(""), DefenceType::AcAgility);
    }

    /// #1765：ObjectRangeAttack 包体布局对齐 C#（28B：oid/x/y/dir/tid/tx/ty/Type/spell/level）
    #[test]
    fn test_object_range_attack_body_layout() {
        let body = build_object_range_attack_body(0x11223344, 11, 22, 3, 0x55667788, 33, 44, 2, 7);
        assert_eq!(body.len(), 28);
        assert_eq!(&body[0..4], &0x11223344u32.to_le_bytes());
        assert_eq!(&body[4..8], &11u32.to_le_bytes());
        assert_eq!(&body[8..12], &22u32.to_le_bytes());
        assert_eq!(body[12], 3); // direction
        assert_eq!(&body[13..17], &0x55667788u32.to_le_bytes()); // target_id
        assert_eq!(&body[17..21], &33u32.to_le_bytes());
        assert_eq!(&body[21..25], &44u32.to_le_bytes());
        assert_eq!(body[25], 2); // Type（C# AttackRange2）
        assert_eq!(body[26], 7); // spell
        assert_eq!(body[27], 0); // spell_level
    }

    /// #1768：怪物/宠物互伤按 C# MonsterObject.Attacked 结算（护甲减免/倍率/全挡 Miss/必中）
    #[test]
    fn test_resolve_monster_vs_monster() {
        use crate::combat::attack::CombatStats;
        use mir2_shared::enums::DefenceType;
        // 必中：目标 agility=0，accuracy=0 → rand_below(1)=0 > 0 为 false
        let attacker = CombatStats { accuracy: 0, ..Default::default() };
        // 护甲减免：min_ac=max_ac=40 → armour=40，伤害 100-40=60
        let defender = CombatStats { agility: 0, min_ac: 40, max_ac: 40, armour_rate: 1.0, damage_rate: 1.0, ..Default::default() };
        let (dmg, miss) = resolve_monster_vs_monster(&attacker, &defender, 100, DefenceType::AcAgility);
        assert!(!miss);
        assert_eq!(dmg, 60);
        // armour >= damage → Miss（护甲全挡）
        let tank = CombatStats { agility: 0, min_ac: 200, max_ac: 200, armour_rate: 1.0, damage_rate: 1.0, ..Default::default() };
        let (dmg, miss) = resolve_monster_vs_monster(&attacker, &tank, 100, DefenceType::AcAgility);
        assert!(miss);
        assert_eq!(dmg, 0);
        // DamageRate 0.5：100*0.5=50 - armour 10 = 40
        let soft = CombatStats { agility: 0, min_ac: 10, max_ac: 10, armour_rate: 1.0, damage_rate: 0.5, ..Default::default() };
        let (dmg, miss) = resolve_monster_vs_monster(&attacker, &soft, 100, DefenceType::AcAgility);
        assert!(!miss);
        assert_eq!(dmg, 40);
        // ArmourRate 2.0：armour 20，100-20=80
        let armoured = CombatStats { agility: 0, min_ac: 10, max_ac: 10, armour_rate: 2.0, damage_rate: 1.0, ..Default::default() };
        let (dmg, miss) = resolve_monster_vs_monster(&attacker, &armoured, 100, DefenceType::AcAgility);
        assert!(!miss);
        assert_eq!(dmg, 80);
    }

    /// #1775：Slow 毒减速（+1 tick=100ms，上限 35 tick=3500ms）
    #[test]
    fn test_slow_adjusted_ticks() {
        // 未减速：原值
        assert_eq!(slow_adjusted_ticks(25, false), 25);
        assert_eq!(slow_adjusted_ticks(4, false), 4);
        // 减速：+1 tick
        assert_eq!(slow_adjusted_ticks(25, true), 26);
        assert_eq!(slow_adjusted_ticks(4, true), 5);
        // 上限 35 tick（3500ms）
        assert_eq!(slow_adjusted_ticks(35, true), 35);
        assert_eq!(slow_adjusted_ticks(34, true), 35);
    }

    /// #1777：控制毒禁移动/攻击（C# CanMove/CanAttack）
    #[test]
    fn test_monster_control_blocked() {
        use crate::combat::poison::Poison;
        use mir2_shared::enums::PoisonType;
        // 空/普通毒：不阻塞
        assert_eq!(monster_control_blocked(&[], 0), (false, false));
        assert_eq!(monster_control_blocked(&[Poison::new(PoisonType::GREEN, 5, 10, 1000)], 0), (false, false));
        // 移动+攻击均禁：PARALYSIS / LR_PARALYSIS / FROZEN / STUN（普通怪 light=0）
        for t in [PoisonType::PARALYSIS, PoisonType::LR_PARALYSIS, PoisonType::FROZEN, PoisonType::STUN] {
            assert_eq!(monster_control_blocked(&[Poison::new(t, 5, 0, 1000)], 0), (true, true));
        }
        // #1824：Light 10/5 光源怪对 STUN 豁免（可移动/攻击），但 PARALYSIS/FROZEN 仍阻塞
        assert_eq!(monster_control_blocked(&[Poison::new(PoisonType::STUN, 5, 0, 1000)], 10), (false, false));
        assert_eq!(monster_control_blocked(&[Poison::new(PoisonType::STUN, 5, 0, 1000)], 5), (false, false));
        assert_eq!(monster_control_blocked(&[Poison::new(PoisonType::STUN, 5, 0, 1000)], 9), (true, true));
        assert_eq!(monster_control_blocked(&[Poison::new(PoisonType::PARALYSIS, 5, 0, 1000)], 10), (true, true));
        assert_eq!(monster_control_blocked(&[Poison::new(PoisonType::FROZEN, 5, 0, 1000)], 5), (true, true));
        // DAZED：只禁攻击不禁移动
        assert_eq!(monster_control_blocked(&[Poison::new(PoisonType::DAZED, 5, 0, 1000)], 0), (false, true));
        // SLOW 不阻塞
        assert_eq!(monster_control_blocked(&[Poison::new(PoisonType::SLOW, 5, 0, 1000)], 0), (false, false));
    }

    /// #1790：ObjectDied.Type 映射（C#：HumanAssassin=2 / Sep*·HumanWizard 有主=1 / 其余 0）
    #[test]
    fn test_monster_death_type_mapping() {
        use super::WorldActor;
        // HumanAssassin 恒 2（暗体特效死亡）
        assert_eq!(WorldActor::monster_death_type("HumanAssassin", false), 2);
        assert_eq!(WorldActor::monster_death_type("HumanAssassin", true), 2);
        // Sep*/HumanWizard 有 master → 1（特效+立即移除）
        assert_eq!(WorldActor::monster_death_type("SepHighWarrior", true), 1);
        assert_eq!(WorldActor::monster_death_type("SepWarrior", true), 1);
        assert_eq!(WorldActor::monster_death_type("sephigharcher", true), 1);
        assert_eq!(WorldActor::monster_death_type("HumanWizard", true), 1);
        // 无 master / 其他怪 → 0（尸体动画）
        assert_eq!(WorldActor::monster_death_type("SepHighWarrior", false), 0);
        assert_eq!(WorldActor::monster_death_type("HumanWizard", false), 0);
        assert_eq!(WorldActor::monster_death_type("HornedCommander", true), 0);
        assert_eq!(WorldActor::monster_death_type("", false), 0);
    }

    /// #1790：ObjectDied 包体 death_type 位置（4B 头 + oid/x/y/dir/type）
    #[test]
    fn test_object_died_body_layout() {
        use super::WorldActor;
        let body = WorldActor::build_object_died_packet(0x11223344, 11, 22, 3, 2);
        assert_eq!(body.len(), 18); // 4B 头 + 14B body
        assert_eq!(&body[4..8], &0x11223344u32.to_le_bytes());
        assert_eq!(&body[8..12], &11u32.to_le_bytes());
        assert_eq!(&body[12..16], &22u32.to_le_bytes());
        assert_eq!(body[16], 3); // direction
        assert_eq!(body[17], 2); // death_type
    }

    /// #1792：传送特效 Type 映射（C# effectnumber）
    #[test]
    fn test_monster_teleport_type_mapping() {
        use super::WorldActor;
        assert_eq!(WorldActor::monster_teleport_type("Yimoogi"), 1);
        assert_eq!(WorldActor::monster_teleport_type("RedFoxman"), 2);
        assert_eq!(WorldActor::monster_teleport_type("MutatedManworm"), 4);
        assert_eq!(WorldActor::monster_teleport_type("WitchDoctor"), 5);
        assert_eq!(WorldActor::monster_teleport_type("TurtleKing"), 6);
        assert_eq!(WorldActor::monster_teleport_type("Mandrill"), 7);
        assert_eq!(WorldActor::monster_teleport_type("DarkCaptain"), 8);
        assert_eq!(WorldActor::monster_teleport_type("Doe"), 9);
        assert_eq!(WorldActor::monster_teleport_type("HornedCommander"), 10);
        assert_eq!(WorldActor::monster_teleport_type("SnowWolfKing"), 11);
        // 未收录 → 0（默认特效）
        assert_eq!(WorldActor::monster_teleport_type("WhiteFoxman"), 0);
        assert_eq!(WorldActor::monster_teleport_type(""), 0);
    }

    /// #1797：组合毒标记 = 全部活动毒 PType 的 OR（C# CurrentPoison）
    #[test]
    fn test_combined_poison_flags() {
        use crate::combat::poison::Poison;
        use mir2_shared::enums::PoisonType;
        assert_eq!(combined_poison_flags(&[]), PoisonType::NONE);
        assert_eq!(
            combined_poison_flags(&[Poison::new(PoisonType::GREEN, 5, 10, 2000)]),
            PoisonType::GREEN
        );
        let both = combined_poison_flags(&[
            Poison::new(PoisonType::GREEN, 5, 10, 2000),
            Poison::new(PoisonType::RED, 5, 10, 2000),
        ]);
        assert!(both.intersects(PoisonType::GREEN) && both.intersects(PoisonType::RED));
    }

    /// #1805：怪物名归一化（去空格+小写）
    #[test]
    fn test_normalized_monster_name() {
        assert_eq!(crate::util::normalized_monster_name("Evil Mir"), "evilmir");
        assert_eq!(crate::util::normalized_monster_name("EvilMir"), "evilmir");
        assert_eq!(crate::util::normalized_monster_name("  Evil  Mir  "), "evilmir");
        assert_eq!(crate::util::normalized_monster_name("Sep High Warrior"), "sephighwarrior");
        assert_eq!(crate::util::normalized_monster_name(""), "");
    }
}
















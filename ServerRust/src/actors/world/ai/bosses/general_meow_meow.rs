//! GeneralMeowMeow（喵喵将军）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/GeneralMeowMeow.cs
//! 机制：可移动、AttackRange=12、
//!   - Energy Shield：HP 进入 70-80% / 40-50% / <=20% 三阶段时开启护盾（AC+100，减伤）
//!     护盾期间周期 MassThunderAttack（每个玩家脚下 GeneralMeowMeowThunder 法术场）
//!   - SlaveSpawnTime (60s)：召唤 3 只猫兵（min(3, 6-SlaveList.Count)）
//!   - 近战 (<=2)：8/9 普攻 DC / 1/9 Slam (DC*3)
//!   - 远程 (>2)：弹道 MAC 攻击

use crate::actors::world::MonsterState;
use mir2_shared::enums::Spell;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

/// 视野范围（C# ViewRange 用于寻敌）
const VIEW_RANGE: i32 = 20;
/// 近战判定距离（C# InRange(CurrentLocation, Target, 2)）
const MELEE_RANGE: i32 = 2;
/// Slave 召唤周期：60s = 600 ticks（C# SlaveSpawnTime = Settings.Second * 60）
const SLAVE_SPAWN_INTERVAL_TICKS: u64 = 600;
/// MassThunder 间隔下限：2s = 20 ticks（C# Random(2000)）
const THUNDER_MIN_TICKS: u64 = 20;
/// MassThunder 间隔上限：4s = 40 ticks（C# Random(4000)）
const THUNDER_MAX_TICKS: u64 = 40;
/// Energy Shield 持续时间：30s = 300 ticks（C# ShieldUpDuration = Settings.Second * 30）
const SHIELD_DURATION_TICKS: u64 = 300;
/// 召唤池（C# Settings.GeneralMeowMeowMob1..4）
const SLAVE_NAMES: [&str; 4] = [
    "StainHammerCat",
    "BlackHammerCat",
    "StrayCat",
    "CatShaman",
];

pub struct GeneralMeowMeowBehavior {
    /// 下次召唤 Slave 的 tick（对齐 C# SlaveSpawnTime）
    next_slave_tick: u64,
    /// 下次 MassThunder 的 tick（对齐 C# ThunderAttackTime）
    next_thunder_tick: u64,
    /// 当前护盾到期 tick（>0 表示护盾激活）
    shield_end_tick: u64,
}

impl GeneralMeowMeowBehavior {
    pub fn new() -> Self {
        Self {
            next_slave_tick: 0,
            next_thunder_tick: 0,
            shield_end_tick: 0,
        }
    }
}

impl MonsterBehavior for GeneralMeowMeowBehavior {
    fn on_attacked(&mut self, damage: i32) -> i32 {
        // C# Energy Shield：减伤 50%。简化为护盾期伤害减半。
        if self.shield_end_tick > 0 {
            (damage / 2).max(1)
        } else {
            damage
        }
    }

    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        // 懒初始化 SlaveSpawnTime（对齐 C# Spawned：SlaveSpawnTime = Envir.Time + 60s）
        if self.next_slave_tick == 0 {
            self.next_slave_tick = ctx.tick_count + SLAVE_SPAWN_INTERVAL_TICKS;
        }

        // 护盾到期检查
        if self.shield_end_tick > 0 && ctx.tick_count >= self.shield_end_tick {
            self.shield_end_tick = 0;
        }

        // ---- 定时器：召唤 Slave（C# ProcessAI：Envir.Time > SlaveSpawnTime）----
        if ctx.tick_count >= self.next_slave_tick {
            self.next_slave_tick = ctx.tick_count + SLAVE_SPAWN_INTERVAL_TICKS;
            // #1441：C# count = min(3, 6 - SlaveList.Count)
            for i in 0..slave_spawn_count(3, ctx.slave_count, 6) {
                let dir = (i as usize) % 8;
                let name = SLAVE_NAMES[fastrand::usize(0..SLAVE_NAMES.len())];
                ctx.out_summons.push(crate::actors::world::ai::BossSummon {
                    monster_name: name.to_string(),
                    x: monster.x + DIR_DX[dir] * (i + 1) as i32,
                    y: monster.y + DIR_DY[dir] * (i + 1) as i32,
                    is_slave: true,
                    summoner_oid: Some(monster.object_id),
                });
            }
        }

        // 无目标时不行动（C# ProcessTarget：Target==null 直接 return）
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);

        // ---- Energy Shield 阶段判定（C# Attack：stage1Bubble/stage2Bubble/stage3Bubble）----
        let hp_pct = if monster.max_hp > 0 {
            (monster.hp * 100) / monster.max_hp
        } else {
            0
        };
        let in_bubble_stage = (hp_pct >= 70 && hp_pct <= 80)
            || (hp_pct >= 40 && hp_pct <= 50)
            || hp_pct <= 20;

        if in_bubble_stage && self.shield_end_tick == 0 {
            // 激活护盾（C# AddBuff(GeneralMeowMeowShield, 30s)）
            self.shield_end_tick = ctx.tick_count + SHIELD_DURATION_TICKS;
        }

        // ---- 护盾期间周期 MassThunder（C# MassThunderAttack）----
        if self.shield_end_tick > 0 && ctx.tick_count >= self.next_thunder_tick {
            // C# ThunderAttackTime = Envir.Time + max(Random(2000), Random(4000))
            self.next_thunder_tick = ctx.tick_count + THUNDER_MIN_TICKS.max(THUNDER_MAX_TICKS);
            // 对攻击范围内每个玩家投放 GeneralMeowMeowThunder 法术场
            let targets: Vec<crate::actors::world::ai::PlayerSnap> =
                ctx.find_targets_in_range(target.x, target.y, VIEW_RANGE, monster.map_index)
                    .into_iter()
                    .copied()
                    .collect();
            for t in targets {
                let value = crate::combat::attack::get_attack_power(monster.min_mac, monster.max_mac, 0).max(1);
                ctx.out_spell_fields.push(crate::actors::world::ai::SpellFieldSpawn {
                    spell: Spell::GeneralMeowMeowThunder,
                    x: t.x,
                    y: t.y,
                    value,
                    duration_ms: 1000,
                    tick_ms: 500,
                    caster_oid: monster.object_id,
                    caster_session: 0,
                });
            }
        }

        // ---- 攻击 / 追击（C# Attack + ProcessTarget）----
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= MELEE_RANGE {
            if ctx.tick_count < monster.next_attack_tick {
                return;
            }
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;

            // C# GeneralMeowMeow.cs:93-110：8/9 普攻 / 1/9 Slam(DC*3)
            if fastrand::i32(0..9) != 0 {
                let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage,
                    spell_id: 0,
                    attack_type: 0,
                });
            } else {
                // Slam：DC*3（C# GetAttackPower(MinDC, MaxDC) * 3）
                let base = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage: base * 3,
                    spell_id: 0,
                    attack_type: 1,
                });
            }
        } else if dist <= 12 {
            // 远程弹道 MAC（C# GeneralMeowMeow.cs:112-120）
            if ctx.tick_count < monster.next_attack_tick {
                return;
            }
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let damage = crate::combat::attack::get_attack_power(monster.min_mac, monster.max_mac, monster.luck).max(1);
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                attacker_oid: monster.object_id,
                target_session: target.session_id,
                target_object_id: target.object_id,
                damage,
                spell_id: 0,
            });
        } else if ctx.tick_count >= monster.next_move_tick {
            // 追击（C# 标准 MoveTo）
            let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + monster.ai_profile.move_interval;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}

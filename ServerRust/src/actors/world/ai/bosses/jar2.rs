//! Jar2（坛子2）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/Jar2.cs（继承 Jar1）
//! 机制：静态（CanMove=false，AttackRange=6）；
//!      近战（dist<=1）且 1/3：近战 DC（MACAgility）；
//!      否则：远程 MC（MAC，攻速+500ms）+ 命中 1/5 冰冻（5s，tick 1000）

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;

const ATTACK_RANGE: i32 = 6;

pub struct Jar2Behavior {
    /// 是否已触发死亡召唤（C# Jar1.CompleteDeath → SpawnSlave）
    died: bool,
    /// 死亡时刻
    die_tick: u64,
}

impl Jar2Behavior {
    pub fn new() -> Self {
        Self { died: false, die_tick: 0 }
    }
}

impl MonsterBehavior for Jar2Behavior {
    fn can_move(&self) -> bool {
        false
    }

    fn can_regen(&self) -> bool {
        false
    }

    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        // C# Jar1.Die（Jar2 继承）：死亡 1s 后召唤一只随机同级怪（SpawnSlave）
        if monster.hp <= 0 {
            if !self.died {
                self.died = true;
                self.die_tick = ctx.tick_count;
            }
            if ctx.tick_count >= self.die_tick + 10 {
                let candidates: Vec<&(String, i32)> = ctx.monster_spawn_candidates.iter()
                    .filter(|(_, lv)| *lv <= monster.level && *lv >= monster.level - 10)
                    .collect();
                if let Some((name, _)) = candidates.get(fastrand::usize(0..candidates.len())).copied() {
                    ctx.out_summons.push(crate::actors::world::ai::BossSummon {
                        monster_name: name.clone(),
                        x: monster.x,
                        y: monster.y,
                        is_slave: false,
                        summoner_oid: Some(monster.object_id),
                    });
                }
                self.die_tick = u64::MAX;
            }
            return;
        }

        let target = match ctx.nearest_target(monster.x, monster.y, ATTACK_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= ATTACK_RANGE && ctx.tick_count >= monster.next_attack_tick {
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
            // C# 远程/魔法攻击用 MC（#2328）
            let mc_damage = crate::combat::attack::get_attack_power(monster.min_mc, monster.max_mc, monster.luck).max(1);
            // C# !ranged && Random.Next(3) == 0：近战 1/3
            if dist <= 1 && fastrand::i32(0..3) == 0 {
                monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage,
                    spell_id: 0,
                    attack_type: 0,
                });
            } else {
                monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown + 5;
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    target_object_id: target.object_id,
                    damage: mc_damage,
                    spell_id: 0,
                });
                // C# CompleteRangeAttack：1/5 冰冻（5s，tick 1000）
                if fastrand::i32(0..5) == 0 {
                    ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                        session_id: target.session_id,
                        poison: Poison::new(PoisonType::FROZEN, 5, crate::actors::world::ai::helpers::poison_sc_value(monster), 1000),
                    });
                }
            }
        }
    }
}

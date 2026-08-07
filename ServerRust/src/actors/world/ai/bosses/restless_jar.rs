//! RestlessJar（躁动之坛）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/RestlessJar.cs
//! 机制：CanMove=false、CanRegen=false、AttackRange=6
//!   近战（dist<=1）：
//!     - 2/3：旋转 AOE1（FindAllTargets(1, 自身)）
//!     - 1/3：HP>=50% → 龙卷单体 + 1/4 失明（10s，tick 1000）；否则 → 践踏 AOE1 + 推挤 1（MaxDC）
//!   远程：投射 MC（攻速*2）

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;

const ATTACK_RANGE: i32 = 6;
const AOE_RADIUS: i32 = 1;

pub struct RestlessJarBehavior;

impl RestlessJarBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for RestlessJarBehavior {
    fn can_move(&self) -> bool {
        false
    }

    fn can_regen(&self) -> bool {
        false
    }

    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, ATTACK_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= ATTACK_RANGE && ctx.tick_count >= monster.next_attack_tick {
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
            if dist <= 1 {
                monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
                // C# Random.Next(3)：0/1 → 旋转 AOE1；2 → 龙卷/践踏
                let roll = fastrand::i32(0..3);
                if roll < 2 {
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Aoe {
                        attacker_oid: monster.object_id,
                        center_x: monster.x,
                        center_y: monster.y,
                        radius: AOE_RADIUS,
                        damage,
                        spell_id: 0,
                    });
                } else {
                    let hp_pct = if monster.max_hp > 0 { monster.hp * 100 / monster.max_hp } else { 0 };
                    if hp_pct >= 50 {
                        // 龙卷：单体 + 1/4 失明（10s，tick 1000）
                        ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                            attacker_oid: monster.object_id,
                            target_session: target.session_id,
                            damage,
                            spell_id: 0,
                            attack_type: 1,
                        });
                        if fastrand::i32(0..4) == 0 {
                            ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                                session_id: target.session_id,
                                poison: Poison::new(PoisonType::BLINDNESS, 10, 0, 1000),
                            });
                        }
                    } else {
                        // 践踏：MaxDC 伤害 + AOE1 + 推挤 1
                        ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Aoe {
                            attacker_oid: monster.object_id,
                            center_x: monster.x,
                            center_y: monster.y,
                            radius: AOE_RADIUS,
                            damage: monster.max_dmg.max(1),
                            spell_id: 0,
                        });
                        let dir = direction_towards(monster.x, monster.y, target.x, target.y);
                        let nearby: Vec<u64> = ctx.find_targets_in_range(monster.x, monster.y, AOE_RADIUS, monster.map_index)
                            .iter().map(|p| p.session_id).collect();
                        for sid in nearby {
                            ctx.out_pushes.push(crate::actors::world::ai::PushPlayer {
                                session_id: sid,
                                dir,
                                distance: 1,
                            });
                        }
                    }
                }
            } else {
                // C# 远程：攻速*2
                monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown * 2;
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    target_object_id: target.object_id,
                    damage,
                    spell_id: 0,
                });
            }
        }
    }
}

//! SandSnail（沙蜗牛）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/SandSnail.cs
//! 机制：
//!   - 6/7：1/2 物理近战（DC）/ 1/2 魔法（MC，Type=2，毒标记→AOE 1 + 100% 绿毒 5s tick 2000）
//!   - 1/7：Halfmoon 弧形攻击（DC，4 格弧）

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;

const VIEW_RANGE: i32 = 12;
const AOE_RADIUS: i32 = 1;

pub struct SandSnailBehavior;

impl SandSnailBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for SandSnailBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= 1 && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
            // C# Envir.Random.Next(7) > 0：6/7 走普通攻击分支
            if fastrand::i32(0..7) > 0 {
                // C# Envir.Random.Next(2) > 0：1/2 物理近战 / 1/2 魔法毒分支
                if fastrand::i32(0..2) > 0 {
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        damage,
                        spell_id: 0,
                        attack_type: 0,
                    });
                } else {
                    // C# 毒分支：FindAllTargets(1) AOE + PoisonTarget(1, 5, Green, 2000)
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Aoe {
                        attacker_oid: monster.object_id,
                        center_x: monster.x,
                        center_y: monster.y,
                        radius: AOE_RADIUS,
                        damage,
                        spell_id: 0,
                    });
                    let nearby: Vec<u64> = ctx.find_targets_in_range(monster.x, monster.y, AOE_RADIUS, monster.map_index)
                        .iter().map(|p| p.session_id).collect();
                    for sid in nearby {
                        ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                            session_id: sid,
                            poison: Poison::new(PoisonType::GREEN, 5, damage, 2000),
                        });
                    }
                }
            } else {
                // C# 1/7 HalfmoonAttack(damage, 300)：PreviousDir 起 4 方向 × 距离 1
                let dir = direction_towards(monster.x, monster.y, target.x, target.y);
                monster.direction = dir;
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Arc {
                    attacker_oid: monster.object_id,
                    center_x: monster.x,
                    center_y: monster.y,
                    direction: dir,
                    count: 4,
                    damage,
                    spell_id: 0,
                    attack_type: 0,
                });
            }
            return;
        }

        if ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + monster.ai_profile.move_interval;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}

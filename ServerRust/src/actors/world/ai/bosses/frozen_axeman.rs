//! FrozenAxeman（冰霜斧手）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/FrozenAxeman.cs
//! 机制：
//!   - InAttackRange：2 格十字/对角
//!   - 近战 2/3：10s 冷却 Pull（Target.Pushed 朝怪→目标方向推 2-4 格 + 伤害）/ 否则普攻
//!   - 1/3 或远程：Type=1 双倍伤害（DC*2）

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 12;
const PULL_COOLDOWN: u64 = 100; // 10s

pub struct FrozenAxemanBehavior {
    next_pull_tick: u64,
}

impl FrozenAxemanBehavior {
    pub fn new() -> Self {
        Self { next_pull_tick: 0 }
    }
}

impl MonsterBehavior for FrozenAxemanBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dx = (target.x - monster.x).abs();
        let dy = (target.y - monster.y).abs();
        let dist = max_distance(monster.x, monster.y, target.x, target.y);
        // C# InAttackRange：2 格十字/对角
        let in_range = dx <= 2 && dy <= 2 && ((dx <= 1 && dy <= 1) || (dx == dy || dx % 2 == dy % 2));

        if in_range && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
            let dir = direction_towards(monster.x, monster.y, target.x, target.y);
            // C# !range && Random.Next(3) > 0：近战 2/3
            if dist <= 1 && fastrand::i32(0..3) > 0 {
                if ctx.tick_count >= self.next_pull_tick {
                    // C# Pull：推 2-4 格（怪→目标方向）+ 伤害
                    self.next_pull_tick = ctx.tick_count + PULL_COOLDOWN;
                    ctx.out_pushes.push(crate::actors::world::ai::PushPlayer {
                        session_id: target.session_id,
                        dir,
                        distance: 2 + fastrand::i32(0..3),
                    });
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        damage,
                        spell_id: 0,
                        attack_type: 2,
                    });
                } else {
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        damage,
                        spell_id: 0,
                        attack_type: 0,
                    });
                }
            } else {
                // C# Type=1 双倍伤害
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage: damage.saturating_mul(2),
                    spell_id: 0,
                    attack_type: 1,
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

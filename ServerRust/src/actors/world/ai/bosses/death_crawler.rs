//! DeathCrawler（死亡爬行者）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/DeathCrawler.cs
//! 机制：CompleteDeath：FindAllTargets(1) + 1/5 绿毒（5s，tick 2000）
//! （受击吐息毒依赖特效广播，暂不实现）

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;

const VIEW_RANGE: i32 = 12;
const AOE_RADIUS: i32 = 1;

pub struct DeathCrawlerBehavior;

impl DeathCrawlerBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for DeathCrawlerBehavior {
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
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                attacker_oid: monster.object_id,
                target_session: target.session_id,
                damage,
                spell_id: 0,
                attack_type: 0,
            });
            return;
        }

        if ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + monster.ai_profile.move_interval;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }

    /// C# CompleteDeath：1 格内 1/5 绿毒（5s，tick 2000）
    fn on_die(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
        let nearby: Vec<u64> = ctx.find_targets_in_range(monster.x, monster.y, AOE_RADIUS, monster.map_index)
            .iter().map(|p| p.session_id).collect();
        for sid in nearby {
            if fastrand::i32(0..5) == 0 {
                ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                    session_id: sid,
                    poison: Poison::new(PoisonType::GREEN, 5, damage, 2000),
                });
            }
        }
    }
}

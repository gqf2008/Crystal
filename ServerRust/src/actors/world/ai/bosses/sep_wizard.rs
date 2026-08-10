//! SepWizard（圣战法师）behavior（简化）
//!
//! C# 参考：Server/MirObjects/Monsters/SepWizard.cs
//! 机制：远程；10-30s 冷却排斥（1 格内低等级目标推 4）；1/3 火墙 AOE1；否则火球（MACAgility）

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 12;

pub struct SepWizardBehavior {
    next_repulsion_tick: u64,
}

impl SepWizardBehavior {
    pub fn new() -> Self {
        Self { next_repulsion_tick: 0 }
    }
}

impl MonsterBehavior for SepWizardBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= VIEW_RANGE && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
            // C# 排斥：10-30s 冷却，1 格内低等级目标推 4
            if ctx.tick_count >= self.next_repulsion_tick {
                let nearby: Vec<&crate::actors::world::ai::PlayerSnap> =
                    ctx.find_targets_in_range(monster.x, monster.y, 1, monster.map_index);
                let pushed: Vec<(u64, u8)> = nearby.iter()
                    .filter(|p| (p.level as i32) < monster.level)
                    .map(|p| {
                        let dir = direction_towards(monster.x, monster.y, p.x, p.y);
                        (p.session_id, dir)
                    }).collect();
                if !pushed.is_empty() {
                    self.next_repulsion_tick = ctx.tick_count + 100 + fastrand::i32(0..20) as u64 * 10; // 10-30s
                    for (sid, dir) in pushed {
                        ctx.out_pushes.push(crate::actors::world::ai::PushPlayer {
                            session_id: sid,
                            dir,
                            distance: 4,
                        });
                    }
                    return;
                }
            }
            // C# 1/3 FireBang / 2/3 GreatFireBall：均为投射（MACAgility），spell_id 区分动画
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                attacker_oid: monster.object_id,
                target_session: target.session_id,
                target_object_id: target.object_id,
                damage,
                spell_id: if fastrand::i32(0..3) == 0 { 1 } else { 0 },
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
}

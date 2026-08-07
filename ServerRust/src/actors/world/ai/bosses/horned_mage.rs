//! HornedMage（角魔法师）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/HornedMage.cs（继承 AxeSkeleton）
//! 机制：
//!   - 近距离（<=3 格）：MC AOE（FindAllTargets(3) 全体 MACAgility）
//!   - 远距离（>3 格）：4/5 DC 弹道 AC；1/5 传送目标（TeleportTarget 4 格随机）
//!   - 风筝走位（继承 AxeSkeleton）
//!
//! Attack（C# :15-63）：!ranged(<=3)→MC AOE；ranged→4/5 弹道 / 1/5 TeleportTarget。
//! CompleteAttack（C# :96-111）：FindAllTargets(3) 全体。

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const ATTACK_RANGE: i32 = 6;
const VIEW_RANGE: i32 = 15;
const CLOSE_RANGE: i32 = 3;

pub struct HornedMageBehavior;

impl HornedMageBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for HornedMageBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= ATTACK_RANGE && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            if dist <= CLOSE_RANGE {
                // 近距 MC AOE（FindAllTargets(3)）
                let hits: Vec<crate::actors::world::ai::PlayerSnap> =
                    ctx.find_targets_in_range(monster.x, monster.y, 3, monster.map_index)
                        .into_iter().copied().collect();
                for h in hits {
                    let damage = crate::combat::attack::get_attack_power(monster.min_mac, monster.max_mac, 0).max(1);
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                        attacker_oid: monster.object_id,
                        target_session: h.session_id,
                        damage,
                        spell_id: 0,
                        attack_type: 0,
                    });
                }
            } else {
                // 远距：4/5 弹道 / 1/5 传送目标（C# HornedMage.cs:61-66 TeleportTarget(4,4)）
                if fastrand::i32(0..5) > 0 {
                    let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        target_object_id: target.object_id,
                        damage,
                        spell_id: 0,
                    });
                } else {
                    // 把目标玩家传送到自身 ±4 随机点（尝试 4 次；推候选，tick 端校验 walkable）
                    for _ in 0..4 {
                        ctx.out_player_teleports.push((
                            target.session_id,
                            monster.x + fastrand::i32(-4..=4),
                            monster.y + fastrand::i32(-4..=4),
                            monster.direction,
                        ));
                    }
                }
            }
            return;
        }

        // 风筝走位
        if ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = if dist >= ATTACK_RANGE {
                step_toward(monster.x, monster.y, target.x, target.y)
            } else if dist < 3 {
                step_away(monster.x, monster.y, target.x, target.y)
            } else {
                return;
            };
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + monster.ai_profile.move_interval;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}

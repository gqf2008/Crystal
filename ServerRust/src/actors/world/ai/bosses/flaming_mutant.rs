//! FlamingMutant（燃烧突变体）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/FlamingMutant.cs
//! 机制：
//!   - 近战 Type0 DC 单体（ACAgility）→ 命中后 FindAllTargets(3) 推开（Pushed）
//!   - 远程 MC 弹道（MACAgility）→ 命中后目标 3 格内 AOE + 1/2 Paralysis（网）
//!
//! Attack（C# :19-56）：近战/远程分支。
//! CompleteAttack（C# :58-79）：推开 3 格内目标。
//! CompleteRangeAttack（C# :81-101）：AOE3 + 1/2 Paralysis。

use crate::actors::world::MonsterState;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 15;
const MELEE_RANGE: i32 = 1;
/// 推开范围（C# FindAllTargets(3, CurrentLocation)）
const PUSH_RADIUS: i32 = 3;
/// 远程网 AOE 范围（C# FindAllTargets(3, target)）
const WEB_RADIUS: i32 = 3;

pub struct FlamingMutantBehavior;

impl FlamingMutantBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for FlamingMutantBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= MELEE_RANGE {
            if ctx.tick_count >= monster.next_attack_tick {
                monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
                let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage,
                    spell_id: 0,
                    attack_type: 0,
                });
                // C# CompleteAttack: 拉向 Boss（Pushed 朝自身方向 dist-1 格，落点 Boss 邻格）
                let push_targets: Vec<crate::actors::world::ai::PlayerSnap> =
                    ctx.find_targets_in_range(monster.x, monster.y, PUSH_RADIUS, monster.map_index)
                        .into_iter().copied().collect();
                for pt in push_targets {
                    let dist = max_distance(monster.x, monster.y, pt.x, pt.y);
                    ctx.out_pushes.push(crate::actors::world::ai::PushPlayer {
                        session_id: pt.session_id,
                        dir: direction_towards(pt.x, pt.y, monster.x, monster.y),
                        distance: (dist - 1).max(1),
                    });
                }
            }
        } else if dist <= VIEW_RANGE {
            if ctx.tick_count >= monster.next_attack_tick {
                monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
                let damage = crate::combat::attack::get_attack_power(monster.min_mac, monster.max_mac, 0).max(1);
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    target_object_id: target.object_id,
                    damage,
                    spell_id: 0,
                });
                // C# CompleteRangeAttack: 目标 3 格内 AOE + 1/2 Paralysis
                let web_targets: Vec<crate::actors::world::ai::PlayerSnap> =
                    ctx.find_targets_in_range(target.x, target.y, WEB_RADIUS, monster.map_index)
                        .into_iter().copied().collect();
                for wt in web_targets {
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                        attacker_oid: monster.object_id,
                        target_session: wt.session_id,
                        damage,
                        spell_id: 0,
                        attack_type: 1,
                    });
                    // C# Random(2)==0 Paralysis
                    if fastrand::i32(0..2) == 0 {
                        ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                            session_id: wt.session_id,
                            poison: Poison::new(PoisonType::PARALYSIS, 1, 5, 1000),
                        });
                    }
                }
            }
        } else if ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + 2;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}

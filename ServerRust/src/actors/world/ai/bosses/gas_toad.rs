//! GasToad（毒气蟾蜍）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/GasToad.cs
//! 机制：
//!   - 近战三形态攻击（贴身 1 格）：
//!     · 5/7 概率 Type0 普攻（其中一半 Type2 重击+麻痹），1/7 概率 Type1 毒雾 AOE
//!   - Type1 毒雾：FindAllTargets(1) 全体 + 绿毒（poison=true 分支）
//!   - Type2 重击：单体 + 麻痹 5s（slam=true 分支）
//!
//! Attack（C# :14-63）：Random(7)>0 走 Type0/2，否则 Type1 毒雾。
//! CompleteAttack（C# :65-96）：poison→AOE+绿毒；slam→麻痹。

use crate::actors::world::MonsterState;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 12;
const MELEE_RANGE: i32 = 1;

pub struct GasToadBehavior;

impl GasToadBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for GasToadBehavior {
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
                let roll = fastrand::i32(0..7);
                if roll == 0 {
                    // Type1 毒雾：1 格全体 + 绿毒
                    let hits: Vec<crate::actors::world::ai::PlayerSnap> =
                        ctx.find_targets_in_range(monster.x, monster.y, 1, monster.map_index)
                            .into_iter().copied().collect();
                    for h in hits {
                        ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                            attacker_oid: monster.object_id,
                            target_session: h.session_id,
                            damage,
                            spell_id: 0,
                            attack_type: 1,
                        });
                        ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                            session_id: h.session_id,
                            poison: Poison::new(PoisonType::GREEN, 5, damage, 2000),
                        });
                    }
                } else if fastrand::i32(0..2) == 0 {
                    // Type2 重击 + 麻痹
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        damage,
                        spell_id: 0,
                        attack_type: 2,
                    });
                    ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                        session_id: target.session_id,
                        poison: Poison::new(PoisonType::PARALYSIS, 5, 0, 2000),
                    });
                } else {
                    // Type0 普通近战
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        damage,
                        spell_id: 0,
                        attack_type: 0,
                    });
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

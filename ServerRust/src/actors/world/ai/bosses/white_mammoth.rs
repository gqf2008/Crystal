//! WhiteMammoth（白色猛犸）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/Monsters/WhiteMammoth.cs
//! 机制：
//!   - 7/8 概率普通攻击：5/6 base.Attack（Type0）；1/6 Type1 DC*2 重击
//!   - 1/8 概率 Type2 MC 践踏：FindAllTargets(1) AOE + Dazed 2s
//!
//! Attack（C# :13-54）：Random(8)==0→stomp；else Random(6)==0→重击；else base。
//! CompleteAttack（C# :56-84）：stomp→AOE1 + Dazed。

use crate::actors::world::MonsterState;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 15;
const MELEE_RANGE: i32 = 1;

pub struct WhiteMammothBehavior;

impl WhiteMammothBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for WhiteMammothBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= MELEE_RANGE && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let roll8 = fastrand::i32(0..8);

            if roll8 > 0 {
                // 7/8 概率普通攻击
                if fastrand::i32(0..6) > 0 {
                    // 5/6 Type0 base.Attack（DC 单体）
                    let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        damage,
                        spell_id: 0,
                        attack_type: 0,
                    });
                } else {
                    // 1/6 Type1 DC*2 重击
                    let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg * 2, 0).max(1);
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        damage,
                        spell_id: 0,
                        attack_type: 1,
                    });
                }
            } else {
                // 1/8 Type2 MC 践踏：AOE 1 格 + Dazed
                let damage = crate::combat::attack::get_attack_power(monster.min_mac, monster.max_mac, 0).max(1);
                let hits: Vec<crate::actors::world::ai::PlayerSnap> =
                    ctx.find_targets_in_range(monster.x, monster.y, MELEE_RANGE, monster.map_index)
                        .into_iter().copied().collect();
                for h in hits {
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                        attacker_oid: monster.object_id,
                        target_session: h.session_id,
                        damage,
                        spell_id: 0,
                        attack_type: 2,
                    });
                    ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                        session_id: h.session_id,
                        poison: Poison::new(PoisonType::DAZED, 2, 5, 2000),
                    });
                }
            }
            return;
        }

        // 追击
        if dist > MELEE_RANGE && ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + 2;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}

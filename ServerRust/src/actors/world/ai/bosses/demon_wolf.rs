//! DemonWolf（恶魔狼）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/DemonWolf.cs
//! 机制：Pack = 5 格内同 AI 怪（用任意怪近似），倍率 = min(Pack,5)+1；
//!      Effect==1：3/4 近战 Type1（MaxDC*倍率）+ 1/4 出血毒；1/4 直线 LineAttack(3)（DC）+ 移动
//!      Effect!=1：近战（MaxDC*倍率，MACAgility）

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;

const VIEW_RANGE: i32 = 12;
const PACK_RANGE: i32 = 5;
const MAX_PACK_SIZE: usize = 5;

pub struct DemonWolfBehavior;

impl DemonWolfBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for DemonWolfBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dx = (target.x - monster.x).abs();
        let dy = (target.y - monster.y).abs();
        let dist = max_distance(monster.x, monster.y, target.x, target.y);
        let in_range = dx <= 2 && dy <= 2 && ((dx <= 1 && dy <= 1) || (dx == dy || dx % 2 == dy % 2));

        if in_range && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            // C# FindPackNearby(5)：5 格内同 AI 怪（用任意怪近似）
            let pack = ctx.monsters.iter()
                .filter(|m| m.map_index == monster.map_index && max_distance(m.x, m.y, monster.x, monster.y) <= PACK_RANGE)
                .count();
            let multiplier = (pack.min(MAX_PACK_SIZE) + 1) as i32;
            let base = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
            let dir = direction_towards(monster.x, monster.y, target.x, target.y);

            if monster.effect == 1 {
                // C# Effect==1：3/4 近战 Type1（倍率）+ 出血；1/4 直线 3 + 移动
                if dist <= 1 && fastrand::i32(0..4) > 0 {
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        damage: base.saturating_mul(multiplier),
                        spell_id: 0,
                        attack_type: 1,
                    });
                    // C# CompleteAttack：1/4 出血毒（5s，tick 1000）
                    if fastrand::i32(0..4) == 0 {
                        ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                            session_id: target.session_id,
                            poison: Poison::new(PoisonType::BLEEDING, 5, base, 1000),
                        });
                    }
                } else {
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Line {
                        attacker_oid: monster.object_id,
                        origin_x: monster.x,
                        origin_y: monster.y,
                        direction: dir,
                        range: 3,
                        damage: base,
                        spell_id: 0,
                    });
                    // C# LineAttack 后 MoveTo
                    let (nx, ny, d2) = step_toward(monster.x, monster.y, target.x, target.y);
                    ctx.out_moves.push((monster.object_id, nx, ny, d2));
                }
            } else {
                // C# Effect!=1：近战（倍率，MACAgility）
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage: base.saturating_mul(multiplier),
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

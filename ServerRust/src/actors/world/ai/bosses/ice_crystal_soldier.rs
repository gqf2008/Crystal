//! IceCrystalSoldier（冰晶士兵）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/IceCrystalSoldier.cs
//! 机制：
//!   - AOE 冷却 2-7s：冷却到 → 前方 1 格 FindAllTargets(1) AOE（DC*2，MAC）
//!   - 冷却中：1/4 近战 DC*1.5 / 否则普攻

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 12;
const AOE_RADIUS: i32 = 1;

pub struct IceCrystalSoldierBehavior {
    next_area_tick: u64,
}

impl IceCrystalSoldierBehavior {
    pub fn new() -> Self {
        Self { next_area_tick: 0 }
    }
}

impl MonsterBehavior for IceCrystalSoldierBehavior {
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
            let dir = direction_towards(monster.x, monster.y, target.x, target.y) as usize % 8;
            if ctx.tick_count >= self.next_area_tick {
                // C# AOE：FindAllTargets(1, 前方 1 格) DC*2
                self.next_area_tick = ctx.tick_count + 20 + fastrand::i32(0..5) as u64 * 10; // 2-7s
                let cx = monster.x + DIR_DX[dir];
                let cy = monster.y + DIR_DY[dir];
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Aoe {
                    attacker_oid: monster.object_id,
                    center_x: cx,
                    center_y: cy,
                    radius: AOE_RADIUS,
                    damage: damage.saturating_mul(2),
                    spell_id: 0,
                });
            } else if fastrand::i32(0..4) == 0 {
                // C# 1/4 近战 DC*1.5
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage: ((damage as f32 * 1.5) as i32).max(1),
                    spell_id: 0,
                    attack_type: 1,
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

//! SpittingSpider（吐丝蜘蛛）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/SpittingSpider.cs（继承 HarvestMonster）
//! 机制：
//!   - 特殊攻击范围：棋盘格对角（x==y 或 x%2==y%2），最远 2 格
//!   - LineAttack(2)：前方直线 2 格穿透伤害 + 绿毒 8s
//!
//! InAttackRange（C# :14-25）：(x<=1&&y<=1)||(x==y||x%2==y%2)，x/y<=2。
//! Attack（C# :27-46）：LineAttack(damage, 2, 300, ACAgility)。
//! CompleteAttack（C# :48-59）：命中后 PoisonTarget(8, 5, Green, 2000)。

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;

/// 视野范围
const VIEW_RANGE: i32 = 12;

/// C# InAttackRange 的棋盘格判定
fn in_spit_range(dx_abs: i32, dy_abs: i32) -> bool {
    if dx_abs > 2 || dy_abs > 2 {
        return false;
    }
    (dx_abs <= 1 && dy_abs <= 1) || (dx_abs == dy_abs || dx_abs % 2 == dy_abs % 2)
}

pub struct SpittingSpiderBehavior;

impl SpittingSpiderBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for SpittingSpiderBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);

        let dx = (target.x - monster.x).abs();
        let dy = (target.y - monster.y).abs();
        let in_range = in_spit_range(dx, dy);

        if in_range {
            if ctx.tick_count >= monster.next_attack_tick {
                monster.next_attack_tick = ctx.tick_count + 8;
                let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
                // LineAttack：方向上 2 格内的玩家（简化：Aoe 命中前方）
                let dir = direction_towards(monster.x, monster.y, target.x, target.y);
                let cx = monster.x + DIR_DX[dir as usize] * 2;
                let cy = monster.y + DIR_DY[dir as usize] * 2;
                let hits: Vec<crate::actors::world::ai::PlayerSnap> =
                    ctx.find_targets_in_range(cx, cy, 2, monster.map_index)
                        .into_iter().copied().collect();
                if hits.is_empty() {
                    // 至少打主目标
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        damage,
                        spell_id: 0,
                        attack_type: 0,
                    });
                    ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                        session_id: target.session_id,
                        poison: Poison::new(PoisonType::GREEN, 8, 5, 2000),
                    });
                }
                for h in hits {
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                        attacker_oid: monster.object_id,
                        target_session: h.session_id,
                        damage,
                        spell_id: 0,
                        attack_type: 0,
                    });
                    ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                        session_id: h.session_id,
                        poison: Poison::new(PoisonType::GREEN, 8, 5, 2000),
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

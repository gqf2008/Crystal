//! PlagueCrab（瘟疫螃蟹）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/PlagueCrab.cs
//! 机制：
//!   - InAttackRange 十字/对角线判定（4 格：x==0||y==0||x==y）
//!   - Attack：DC LineAttack(4) + MACAgility（瘟疫毒雾直线）
//!
//! Attack（C# :27-47）：DC LineAttack(4, 500, MACAgility)。
//! InAttackRange（C# :14-25）：x<=4&&y<=4 且 (x==0||y==0||x==y)。

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 15;
/// LineAttack 距离（C# LineAttack(damage, 4)）
const LINE_RANGE: i32 = 4;

pub struct PlagueCrabBehavior;

impl PlagueCrabBehavior {
    pub fn new() -> Self { Self }
}

impl MonsterBehavior for PlagueCrabBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);

        let dx_abs = (target.x - monster.x).abs();
        let dy_abs = (target.y - monster.y).abs();
        // 十字/对角线判定（C# InAttackRange）
        let in_line = dx_abs <= LINE_RANGE && dy_abs <= LINE_RANGE
            && (dx_abs == 0 || dy_abs == 0 || dx_abs == dy_abs);

        if in_line {
            if ctx.tick_count >= monster.next_attack_tick {
                monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
                let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
                // LineAttack(4)：朝目标方向直线 4 格命中全体（MACAgility）
                let dir = direction_towards(monster.x, monster.y, target.x, target.y);
                let ddx = DIR_DX[dir as usize];
                let ddy = DIR_DY[dir as usize];
                let hits: Vec<crate::actors::world::ai::PlayerSnap> = ctx
                    .find_targets_in_range(monster.x, monster.y, LINE_RANGE, monster.map_index)
                    .into_iter().copied()
                    .filter(|p| {
                        let rx = p.x - monster.x;
                        let ry = p.y - monster.y;
                        (rx == 0 && ddy == 0) || (ry == 0 && ddx == 0)
                            || (rx.signum() == ddx.signum() && ry.signum() == ddy.signum() && rx.abs() == ry.abs())
                    })
                    .collect();
                if hits.is_empty() {
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        damage,
                        spell_id: 0,
                        attack_type: 0,
                    });
                } else {
                    for h in hits {
                        ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                            attacker_oid: monster.object_id,
                            target_session: h.session_id,
                            damage,
                            spell_id: 0,
                            attack_type: 0,
                        });
                    }
                }
            }
            return;
        }

        // 追击
        if ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + 2;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}

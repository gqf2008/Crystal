//! Tornado（龙卷风）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/Tornado.cs
//! 机制：
//!   - 远程（>1 格）：吸拉 —— 把 5 格内全体玩家 Pushed 朝自身方向（dist-1 格）
//!   - 近战：标准普攻
//!   - AttackRange=5
//!
//! Attack（C# :20-55）：ranged→FindAllTargets(5) 逐个 Pushed(dir=朝自身, dist-1)。

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const ATTACK_RANGE: i32 = 5;
const VIEW_RANGE: i32 = 12;
const MELEE_RANGE: i32 = 1;

pub struct TornadoBehavior;

impl TornadoBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for TornadoBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= MELEE_RANGE {
            if ctx.tick_count >= monster.next_attack_tick {
                monster.next_attack_tick = ctx.tick_count + 6;
                let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage,
                    spell_id: 0,
                    attack_type: 0,
                });
            }
        } else if dist <= ATTACK_RANGE {
            // 吸拉：把范围内玩家朝自身拉近（C# Pushed 朝龙卷风方向 dist-1 格）
            if ctx.tick_count >= monster.next_attack_tick {
                monster.next_attack_tick = ctx.tick_count + 8;
                let pulls: Vec<crate::actors::world::ai::PlayerSnap> =
                    ctx.find_targets_in_range(monster.x, monster.y, ATTACK_RANGE, monster.map_index)
                        .into_iter().copied().collect();
                let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
                for p in pulls {
                    // 拉到自身相邻格（朝玩家方向 1 格处）
                    let dir = direction_towards(p.x, p.y, monster.x, monster.y);
                    let nx = monster.x - DIR_DX[dir as usize];
                    let ny = monster.y - DIR_DY[dir as usize];
                    ctx.out_moves.push((monster.object_id, nx, ny, dir)); // 用 move 槽近似位移（POC）
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                        attacker_oid: monster.object_id,
                        target_session: p.session_id,
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

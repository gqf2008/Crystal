//! SepHighAssassin（圣战高阶刺客）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/SepHighAssassin.cs
//! 机制：近战（AttackRange=3）；攻击：
//!   - 1/5 CrescentSlash：8 向新月斩（排除背向 3 方向），命中距离 1-2 目标全额伤害
//!   - 4/5 DoubleSlash：近战全额 + 0.8x 延迟伤害
//! 远程（出近战范围）：HeavenlySword 直线 LineAttack(3)

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 12;
const ATTACK_RANGE: i32 = 3; // C# AttackRange = 3

/// C# SepHighAssassin.CrescentSlash：8 向新月斩落点——排除背向 3 方向，命中距离 1-2
fn crescent_slash_cells(x: i32, y: i32, facing: usize) -> Vec<(i32, i32)> {
    let mut cells = Vec::new();
    let back = (facing + 4) % 8;
    for d in 0..8usize {
        if d == back || d == (back + 7) % 8 || d == (back + 1) % 8 {
            continue;
        }
        cells.push((x + DIR_DX[d], y + DIR_DY[d]));
        cells.push((x + DIR_DX[d] * 2, y + DIR_DY[d] * 2));
    }
    cells
}

pub struct SepHighAssassinBehavior;

impl SepHighAssassinBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for SepHighAssassinBehavior {
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
            // C# ProcessTarget：近战 4/5 Attack / 1/5 RangeAttack（HeavenlySword）
            if fastrand::i32(0..5) == 0 {
                let dir = direction_towards(monster.x, monster.y, target.x, target.y);
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Line {
                    attacker_oid: monster.object_id,
                    origin_x: monster.x,
                    origin_y: monster.y,
                    direction: dir,
                    range: ATTACK_RANGE,
                    damage,
                    spell_id: 0,
                });
                return;
            }
            if fastrand::i32(0..5) == 0 {
                // C# CrescentSlash：8 向新月斩（排除背向 3 方向），命中距离 1-2 全额伤害
                let facing = direction_towards(monster.x, monster.y, target.x, target.y) as usize % 8;
                let cells = crescent_slash_cells(monster.x, monster.y, facing);
                for p in ctx.players.iter().filter(|p| p.map_index == monster.map_index && p.hp > 0
                    && cells.contains(&(p.x, p.y))) {
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                        attacker_oid: monster.object_id,
                        target_session: p.session_id,
                        damage,
                        spell_id: 0,
                        attack_type: 2,
                    });
                }
            } else {
                // C# DoubleSlash：近战全额 + 0.8x 延迟伤害
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage,
                    spell_id: 0,
                    attack_type: 0,
                });
                let dmg = ((damage as f32 * 0.8) as i32).max(1);
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    target_object_id: target.object_id,
                    damage: dmg,
                    spell_id: 0,
                });
            }
            return;
        }

        // C# ProcessTarget：追击中（出近战范围）1/5 RangeAttack（HeavenlySword）
        if dist <= VIEW_RANGE && ctx.tick_count >= monster.next_attack_tick && fastrand::i32(0..5) == 0 {
            // C# RangeAttack：HeavenlySword 直线 LineAttack(3)
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
            let dir = direction_towards(monster.x, monster.y, target.x, target.y);
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Line {
                attacker_oid: monster.object_id,
                origin_x: monster.x,
                origin_y: monster.y,
                direction: dir,
                range: ATTACK_RANGE,
                damage,
                spell_id: 0,
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

#[cfg(test)]
mod tests {
    use super::*;

    /// #1784：新月斩落点——排除背向 3 方向，命中距离 1-2（10 格）
    #[test]
    fn test_crescent_slash_cells_shape() {
        // 朝向 Up（0），背向 Down（4）；排除 4/3/5，保留 0/1/2/6/7 各距离 1-2
        let cells = crescent_slash_cells(0, 0, 0);
        assert_eq!(cells.len(), 10);
        // 保留方向
        assert!(cells.contains(&(0, -1)));
        assert!(cells.contains(&(0, -2)));
        assert!(cells.contains(&(1, -1)));
        assert!(cells.contains(&(2, -2)));
        assert!(cells.contains(&(2, 0)));
        assert!(cells.contains(&(-1, -1)));
        assert!(cells.contains(&(-2, -2)));
        assert!(cells.contains(&(-2, 0)));
        // 排除背向（Down/DownLeft/DownRight）
        assert!(!cells.contains(&(0, 1)));
        assert!(!cells.contains(&(0, 2)));
        assert!(!cells.contains(&(-1, 1)));
        assert!(!cells.contains(&(1, 1)));
    }
}

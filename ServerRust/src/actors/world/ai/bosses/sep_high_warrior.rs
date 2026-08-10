//! SepHighWarrior（圣战高阶战士）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/SepHighWarrior.cs
//! 机制：近战（AttackRange=1）；攻击随机 5 选 1：
//!   - 1/5 TwinDrakeBlade：0.8x 近战 + 0.8x 投射 +（目标<=怪+8 且 5/20）眩晕 5s + ObjectEffect
//!   - 1/5 CrossHalfMoon：弧形 4 格弧（C# HalfmoonAttack）
//!   - 1/5 BladeAvalanche：前方 3 列 × 3 行刀山（j<=1 全额 / j>=2 0.6x，C# DefenceType.MAC）
//!   - 2/5 普攻（base.Attack）

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;

const VIEW_RANGE: i32 = 12;

/// C# SepHighWarrior.BladeAvalanche：前方 3 列（前/中/后方向）× 3 行刀山落点
/// 返回 (x, y, j)，j<=1 全额伤害 / j>=2 0.6x。
fn blade_avalanche_cells(x: i32, y: i32, dir: usize) -> Vec<(i32, i32, i32)> {
    let mut cells = Vec::new();
    for col in [-1i32, 0, 1] {
        let cdir = (dir as i32 + col).rem_euclid(8) as usize;
        let start = (x + DIR_DX[cdir], y + DIR_DY[cdir]);
        for j in 0..3i32 {
            cells.push((start.0 + DIR_DX[dir] * j, start.1 + DIR_DY[dir] * j, j));
        }
    }
    cells
}

pub struct SepHighWarriorBehavior;

impl SepHighWarriorBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for SepHighWarriorBehavior {
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
            match fastrand::i32(0..5) {
                0 => {
                    // C# TwinDrakeBlade：0.8x 近战 + 0.8x 投射 + 眩晕
                    let dmg = ((damage as f32 * 0.8) as i32).max(1);
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        damage: dmg,
                        spell_id: 0,
                        attack_type: 0,
                    });
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        target_object_id: target.object_id,
                        damage: dmg,
                        spell_id: 0,
                    });
                    // C#：目标<=怪+8 且 Random(20)<=5 → Stun 5s + 目标特效
                    if target.level as i32 <= monster.level + 8 && fastrand::i32(0..20) <= 5 {
                        ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                            session_id: target.session_id,
                            poison: Poison::new(PoisonType::STUN, 5, 0, 1000),
                        });
                        // C# TwinDrakeBlade（SepHighWarrior.cs:103）：眩晕时对目标广播特效
                        ctx.out_effects.push((target.object_id, mir2_shared::enums::SpellEffect::TwinDrakeBlade, 0, 0));
                    }
                }
                1 => {
                    // C# CrossHalfMoon（SepHighWarrior.cs:107）：HalfmoonAttack 4 格弧
                    let dir = direction_towards(monster.x, monster.y, target.x, target.y);
                    monster.direction = dir;
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Arc {
                        attacker_oid: monster.object_id,
                        center_x: monster.x,
                        center_y: monster.y,
                        direction: dir,
                        count: 4,
                        damage,
                        spell_id: 0,
                        attack_type: 1,
                    });
                }
                2 => {
                    // C# BladeAvalanche：前方 3 列 × 3 行刀山（j<=1 全额 / j>=2 0.6x）
                    let dir = direction_towards(monster.x, monster.y, target.x, target.y) as usize % 8;
                    for (cx, cy, j) in blade_avalanche_cells(monster.x, monster.y, dir) {
                        let dmg = if j <= 1 { damage } else { ((damage as f32 * 0.6) as i32).max(1) };
                        for p in ctx.players.iter().filter(|p| p.map_index == monster.map_index && p.hp > 0
                            && p.x == cx && p.y == cy) {
                            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                                attacker_oid: monster.object_id,
                                target_session: p.session_id,
                                damage: dmg,
                                spell_id: 0,
                                attack_type: 2,
                            });
                        }
                    }
                }
                _ => {
                    // C# base.Attack：普攻
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        damage,
                        spell_id: 0,
                        attack_type: 0,
                    });
                }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// #1780：刀山 3×3 落点对齐 C# BladeAvalanche（前方 3 列 × 3 行）
    #[test]
    fn test_blade_avalanche_cells_shape() {
        // 朝向 Up（dir=0）：列 = UpLeft/Up/UpRight，每列 3 行
        let cells = blade_avalanche_cells(0, 0, 0);
        assert_eq!(cells.len(), 9);
        // 中列第一格 = 正前方 (0,-1) j=0
        assert!(cells.contains(&(0, -1, 0)));
        // 左列 = UpLeft (-1,-1) 起
        assert!(cells.contains(&(-1, -1, 0)));
        assert!(cells.contains(&(-1, -2, 1)));
        assert!(cells.contains(&(-1, -3, 2)));
        // 右列 = UpRight (1,-1) 起
        assert!(cells.contains(&(1, -1, 0)));
        assert!(cells.contains(&(1, -2, 1)));
        assert!(cells.contains(&(1, -3, 2)));
        // 中列延伸
        assert!(cells.contains(&(0, -2, 1)));
        assert!(cells.contains(&(0, -3, 2)));
        // j 值 0/1/2 各 3 个
        let js: Vec<i32> = cells.iter().map(|c| c.2).collect();
        assert_eq!(js.iter().filter(|&&j| j == 0).count(), 3);
        assert_eq!(js.iter().filter(|&&j| j == 1).count(), 3);
        assert_eq!(js.iter().filter(|&&j| j == 2).count(), 3);
    }
}

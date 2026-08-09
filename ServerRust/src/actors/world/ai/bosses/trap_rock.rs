//! TrapRock（陷阱岩）behavior（#1437：补全子岩环绕）
//!
//! C# 参考：Server/MirObjects/Monsters/TrapRock.cs
//! 机制：静态；2s 后伏击目标（传送到目标四角之一 + 100% 麻痹 3s）+ 攻击 1/8 麻痹；
//!      目标移动/死亡 → 自毁；不可受击（on_attacked 返 0）
//! Show（C# :147-168）：父岩在其余三角生成 3 只 ChildRock（立即可见、同目标、近战）；
//!      父岩死亡 → 子岩级联清理（#1434 SlaveList）

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;

const VIEW_RANGE: i32 = 12;

/// 子岩环绕方向（C# TrapRock.Show：i=0,2,4,6 跳过 SpawnCorner）
fn trap_rock_child_corners(spawn_corner: u8) -> Vec<u8> {
    [0u8, 2, 4, 6]
        .iter()
        .copied()
        .filter(|c| *c != spawn_corner)
        .collect()
}

pub struct TrapRockBehavior {
    shown: bool,
    target_loc: (i32, i32),
    /// #1437：是否子岩（C# ChildRock）——立即可见、近战、不麻痹/不生成子岩
    child: bool,
    /// #1437：父岩 oid（子岩归属，父岩死亡级联清理）
    parent_oid: Option<u32>,
}

impl TrapRockBehavior {
    pub fn new() -> Self {
        Self { shown: false, target_loc: (0, 0), child: false, parent_oid: None }
    }

    /// #1437：构造已可见的子岩（tick.rs 生成 ChildRock 时用；C# ChildRock.Show 预设）
    pub(crate) fn child(shown: bool, target_loc: (i32, i32), parent_oid: u32) -> Self {
        Self { shown, target_loc, child: true, parent_oid: Some(parent_oid) }
    }
}

impl MonsterBehavior for TrapRockBehavior {
    fn can_move(&self) -> bool {
        false
    }

    fn can_regen(&self) -> bool {
        false
    }

    fn on_attacked(&mut self, _damage: i32) -> i32 {
        0 // C# Struck 返 0：不可伤
    }

    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => {
                if self.shown {
                    monster.hp = 0;
                }
                return;
            }
        };
        monster.target_session = Some(target.session_id);

        // #1437：子岩——目标移动/死亡 → 自毁；近战攻击（C# ChildRock.Attack：ObjectAttack，无 1/8 麻痹）
        if self.child {
            if target.x != self.target_loc.0 || target.y != self.target_loc.1 || target.hp <= 0 {
                monster.hp = 0;
                return;
            }
            if ctx.tick_count >= monster.next_attack_tick {
                monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
                let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
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

        if !self.shown {
            // C# Show：传送目标四角之一 + 100% 麻痹（3s）
            let corner = fastrand::i32(0..4) * 2;
            let tx = target.x + DIR_DX[corner as usize];
            let ty = target.y + DIR_DY[corner as usize];
            self.shown = true;
            self.target_loc = (target.x, target.y);
            ctx.out_monster_teleports.push((monster.object_id, tx, ty));
            ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                session_id: target.session_id,
                poison: Poison::new(PoisonType::PARALYSIS, 3, 0, 1000),
            });
            // #1437：C# Show——其余三角生成 ChildRock（立即可见、同目标、slave 归属父岩）
            for c in trap_rock_child_corners(corner as u8) {
                ctx.out_child_rocks.push(crate::actors::world::ai::ChildRockSpawn {
                    monster_name: monster.name.clone(),
                    x: target.x + DIR_DX[c as usize],
                    y: target.y + DIR_DY[c as usize],
                    target_session: target.session_id,
                    target_x: target.x,
                    target_y: target.y,
                    parent_oid: monster.object_id,
                });
            }
            return;
        }
        // C# 目标移动/死亡 → 自毁
        if target.x != self.target_loc.0 || target.y != self.target_loc.1 || target.hp <= 0 {
            monster.hp = 0;
            return;
        }
        // C# Attack：远程 + 1/8 麻痹
        if ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                attacker_oid: monster.object_id,
                target_session: target.session_id,
                target_object_id: target.object_id,
                damage,
                spell_id: 0,
            });
            if fastrand::i32(0..8) == 0 {
                ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                    session_id: target.session_id,
                    poison: Poison::new(PoisonType::PARALYSIS, 3, 0, 1000),
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::trap_rock_child_corners;

    #[test]
    fn child_corners_skip_spawn_corner() {
        // C# TrapRock.Show：i=0,2,4,6 跳过 SpawnCorner → 3 只
        for corner in [0u8, 2, 4, 6] {
            let corners = trap_rock_child_corners(corner);
            assert_eq!(corners.len(), 3);
            assert!(!corners.contains(&corner));
            for c in &corners {
                assert!([0u8, 2, 4, 6].contains(c));
            }
        }
    }
}

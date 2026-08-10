//! Kirin（麒麟）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/Kirin.cs
//! 机制：双形态（近战 + 冰系远程）
//!   - InAttackRange 特殊十字判定（2 格）
//!   - Attack：4/5 基础近战，1/5 强力近战 AC（Type=1）
//!   - RangeAttack：MC + IceThrust（前方 3x3 冰锥，1/5 Slow 毒）
//!   - 未进入射程时 1/5 远程攻击；同位时随机走开
//!
//! Attack（C# :81-111）：Random(5)==0→AC 强攻；else base.Attack。
//! RangeAttack/IceThrust（C# :113-190）：MC 3x3 + 1/5 Slow。

use crate::actors::world::MonsterState;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 15;
/// 近战十字判定范围（C# x>2||y>2 → false）
const MELEE_RANGE: i32 = 2;
/// 冰锥纵深（C# IceThrust col=3,row=3）
const THRUST_RANGE: i32 = 3;

pub struct KirinBehavior;

impl KirinBehavior {
    pub fn new() -> Self { Self }
}

impl MonsterBehavior for KirinBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);
        let in_melee = dist <= MELEE_RANGE;

        if ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;

            if in_melee {
                // 4/5 基础近战，1/5 AC 强攻（C# Attack Random(5)==0）
                let strong = fastrand::i32(0..5) == 0;
                let damage = if strong {
                    crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1)
                } else {
                    crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1)
                };
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage,
                    spell_id: if strong { 1 } else { 0 },
                    attack_type: if strong { 1 } else { 0 },
                });
            } else {
                // RangeAttack：1/5 概率（C# ProcessTarget Random(5)）
                if fastrand::i32(0..5) == 0 {
                    self.ice_thrust(monster, target, ctx);
                }
            }
            return;
        }

        // 走位
        if ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = if dist <= MELEE_RANGE && monster.x == target.x && monster.y == target.y {
                // 同位：随机走开（C# direction = Random(8)）
                let r = fastrand::usize(0..8);
                (monster.x + DIR_DX[r], monster.y + DIR_DY[r], r as u8)
            } else {
                step_toward(monster.x, monster.y, target.x, target.y)
            };
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + monster.ai_profile.move_interval;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}

impl KirinBehavior {
    /// 冰锥：MC 前方 3x3（3 列 × 3 深）区域，每命中 1/5 Slow 4s（C# Kirin.IceThrust :126）
    fn ice_thrust(&self, monster: &mut MonsterState, target: crate::actors::world::ai::PlayerSnap, ctx: &mut AiCtx) {
        let damage = crate::combat::attack::get_attack_power(monster.min_mac, monster.max_mac, 0).max(1);
        let dir = direction_towards(monster.x, monster.y, target.x, target.y);
        monster.direction = dir;
        // C# IceThrust：3 列（prevdir/dir/nextdir 起点）× 3 深 = 9 格
        let cells3 = ice_thrust_cells(monster.x, monster.y, dir, THRUST_RANGE as u8);
        let cells: Vec<(i32, i32)> = cells3.iter().map(|&(x, y, _)| (x, y)).collect();
        ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Cells {
            attacker_oid: monster.object_id,
            center_x: monster.x,
            center_y: monster.y,
            cells: cells.clone(),
            damage,
            spell_id: 0,
            attack_type: 2,
        });
        // C# 每命中 1/5 Slow（玩家 4s，tick 1000）
        let hit: Vec<u64> = ctx.find_targets_in_cells(&cells, monster.map_index)
            .iter().map(|p| p.session_id).collect();
        for sid in hit {
            if fastrand::i32(0..5) == 0 {
                ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                    session_id: sid,
                    poison: Poison::new(PoisonType::SLOW, 4, 0, 1000),
                });
            }
        }
    }
}

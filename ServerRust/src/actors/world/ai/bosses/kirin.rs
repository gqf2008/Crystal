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
            monster.next_move_tick = ctx.tick_count + 2;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}

impl KirinBehavior {
    /// 冰锥：MC 前方 3x3 区域，1/5 Slow（C# IceThrust）
    fn ice_thrust(&self, monster: &mut MonsterState, target: crate::actors::world::ai::PlayerSnap, ctx: &mut AiCtx) {
        let damage = crate::combat::attack::get_attack_power(monster.min_mac, monster.max_mac, 0).max(1);
        let dir = direction_towards(monster.x, monster.y, target.x, target.y);
        let dx = DIR_DX[dir as usize];
        let dy = DIR_DY[dir as usize];
        // 前方 3 格纵深 × 3 列（prevdir/dir/nextdir 起始）的玩家
        let hits: Vec<crate::actors::world::ai::PlayerSnap> = ctx
            .find_targets_in_range(monster.x, monster.y, THRUST_RANGE, monster.map_index)
            .into_iter().copied()
            .filter(|p| {
                let rx = p.x - monster.x;
                let ry = p.y - monster.y;
                // 朝目标方向的前方扇形/直线区域
                (rx * dx >= 0 && ry * dy >= 0) && (rx.abs() <= THRUST_RANGE && ry.abs() <= THRUST_RANGE)
            })
            .collect();
        if hits.is_empty() {
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                attacker_oid: monster.object_id,
                target_session: target.session_id,
                target_object_id: target.object_id,
                damage,
                spell_id: 0,
            });
            // 1/5 Slow（C# Random(5)==0）
            if fastrand::i32(0..5) == 0 {
                ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                    session_id: target.session_id,
                    poison: Poison::new(PoisonType::SLOW, 5, 0, 1000),
                });
            }
        } else {
            for h in hits {
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: h.session_id,
                    damage,
                    spell_id: 0,
                    attack_type: 0,
                });
                if fastrand::i32(0..5) == 0 {
                    ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                        session_id: h.session_id,
                        poison: Poison::new(PoisonType::SLOW, 5, 0, 1000),
                    });
                }
            }
        }
    }
}

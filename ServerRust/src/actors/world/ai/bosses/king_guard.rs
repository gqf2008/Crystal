//! KingGuard（王城护卫）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/KingGuard.cs
//! 机制：强力 Guard 变体，AttackRange=10
//!   - 近战（<=1）：2/3 普通单体 DC ACAgility；1/3 重击 DC*2 AC + AOE(3)
//!   - 远程（>1）：2/3 单体 MC MAC + 绿毒；1/3 重击 MC*2 MAC + AOE(AttackRange) Slow/Paralysis
//!
//! Attack（C# :27-90）：!ranged→4/5 普通 DC / 1/5 DC*2 AOE；ranged→2/3 MC / 1/3 MC*2 AOE。
//! CompleteRangeAttack（C# :117-153）：aoe→FindAllTargets(AttackRange)+Slow/Paralysis；else Green。

use crate::actors::world::MonsterState;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

/// C# AttackRange = 10
const ATTACK_RANGE: i32 = 10;
const VIEW_RANGE: i32 = 15;
const MELEE_RANGE: i32 = 1;
/// 近战 AOE 半径（C# FindAllTargets(3)）
const MELEE_AOE_RADIUS: i32 = 3;

pub struct KingGuardBehavior;

impl KingGuardBehavior {
    pub fn new() -> Self { Self }
}

impl MonsterBehavior for KingGuardBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= ATTACK_RANGE && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;

            if dist <= MELEE_RANGE {
                // 近战：4/5 普通 DC ACAgility；1/5 DC*2 AC AOE(3)
                if fastrand::i32(0..5) > 0 {
                    let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        damage,
                        spell_id: 0,
                        attack_type: 0,
                    });
                } else {
                    // 重击 DC*2 + AOE(3)
                    let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg * 2, 0).max(1);
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Aoe {
                        attacker_oid: monster.object_id,
                        center_x: monster.x,
                        center_y: monster.y,
                        radius: MELEE_AOE_RADIUS,
                        damage,
                        spell_id: 0,
                    });
                }
            } else {
                // 远程：2/3 MC MAC + 绿毒；1/3 MC*2 MAC + AOE Slow/Paralysis
                if fastrand::i32(0..3) > 0 {
                    let damage = crate::combat::attack::get_attack_power(monster.min_mac, monster.max_mac, 0).max(1);
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        target_object_id: target.object_id,
                        damage,
                        spell_id: 0,
                    });
                    // C# PoisonTarget(target,10,5,Green,1000)
                    // C# PoisonTarget(10,5,Green,1000)：1/10
                    if fastrand::i32(0..10) == 0 {
                        ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                            session_id: target.session_id,
                            poison: Poison::new(PoisonType::GREEN, 5, damage, 1000),
                        });
                    }
                } else {
                    // 重击 MC*2 + AOE(AttackRange) Slow + KingGuard 特效
                    let damage = crate::combat::attack::get_attack_power(monster.min_mac, monster.max_mac * 2, 0).max(1);
                    // C# CompleteRangeAttack FindAllTargets(AttackRange, CurrentLocation)：AOE 以怪物自身为中心
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Aoe {
                        attacker_oid: monster.object_id,
                        center_x: monster.x,
                        center_y: monster.y,
                        radius: ATTACK_RANGE,
                        damage,
                        spell_id: 0,
                    });
                    // C# KingGuard.cs:135-144：Random(3)>=0 恒真 → EffectType=0 + Slow；EffectType=1 + Paralysis 为死代码
                    let aoe_targets: Vec<crate::actors::world::ai::PlayerSnap> =
                        ctx.find_targets_in_range(monster.x, monster.y, ATTACK_RANGE, monster.map_index)
                            .into_iter().copied().collect();
                    for gt in aoe_targets {
                        ctx.out_effects.push((gt.object_id, mir2_shared::enums::SpellEffect::KingGuard, 0, 0));
                        // C# PoisonTarget(5,10,Slow,1000)：1/5
                        if fastrand::i32(0..5) == 0 {
                            ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                                session_id: gt.session_id,
                                poison: Poison::new(PoisonType::SLOW, 10, 0, 1000),
                            });
                        }
                    }
                }
            }
            return;
        }

        // 追击
        if ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + monster.ai_profile.move_interval;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}

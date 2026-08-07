//! RhinoPriest（犀牛祭司）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/RhinoPriest.cs（继承 AxeSkeleton）
//! 机制：
//!   - 继承 AxeSkeleton 远程弹道（AttackRange=6）+ 风筝走位
//!   - 近战：DC ACAgility
//!   - 远程：MC 弹道；2/3 普通(MACAgility)，1/3 蓝圈(MAC)
//!     - 普通：命中附加 RhinoPriestDebuff（DC/MC/SC 降伤害*duration）
//!     - 蓝圈：3/4 Slow 2s，1/4 Frozen 4s
//!   - 祭司特性：周期治疗/buff 附近友军（out_heals 近似 DC/MC 增益）
//!
//! Attack（C# :13-63）：!range→DC ACAgility；range→2/3 MACAgility(Debuff) / 1/3 MAC(Slow/Frozen)。
//! CompleteRangeAttack（C# :65-98）：poison→Slow/Frozen；else RhinoPriestDebuff。

use crate::actors::world::MonsterState;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const ATTACK_RANGE: i32 = 6;
const VIEW_RANGE: i32 = 15;
const MELEE_RANGE: i32 = 1;
/// 友军 buff 半径
const BUFF_RADIUS: i32 = 5;
/// 友军 buff 冷却（10s）
const BUFF_COOLDOWN_TICKS: u64 = 100;

pub struct RhinoPriestBehavior {
    next_buff_tick: u64,
}

impl RhinoPriestBehavior {
    pub fn new() -> Self {
        Self { next_buff_tick: 0 }
    }
}

impl MonsterBehavior for RhinoPriestBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        // 周期 buff 附近友军（祭司特性：out_heals 正值近似 DC/MC 增益）
        if ctx.tick_count >= self.next_buff_tick {
            let buffed: Vec<u32> = ctx.monsters.iter()
                .filter(|m| m.map_index == monster.map_index && m.hp > 0
                    && m.object_id != monster.object_id)
                .filter(|m| {
                    let dx = (m.x - monster.x).abs();
                    let dy = (m.y - monster.y).abs();
                    dx.max(dy) <= BUFF_RADIUS
                })
                .map(|m| m.object_id)
                .collect();
            for oid in buffed {
                ctx.out_heals.push((oid, monster.min_mac.max(1)));
            }
            self.next_buff_tick = ctx.tick_count + BUFF_COOLDOWN_TICKS;
        }

        if dist <= ATTACK_RANGE && ctx.tick_count >= monster.next_attack_tick {
            if dist <= MELEE_RANGE {
                // 近战 DC ACAgility
                monster.next_attack_tick = ctx.tick_count + 7;
                let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage,
                    spell_id: 0,
                    attack_type: 0,
                });
            } else {
                // 远程 MC：2/3 普通(MACAgility,Debuff) / 1/3 蓝圈(MAC,Slow/Frozen)
                monster.next_attack_tick = ctx.tick_count + 9;
                let damage = crate::combat::attack::get_attack_power(monster.min_mac, monster.max_mac, 0).max(1);
                let blue_circle = fastrand::i32(0..3) == 0;
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    target_object_id: target.object_id,
                    damage,
                    spell_id: if blue_circle { 1 } else { 0 },
                });
                if blue_circle {
                    // C# CompleteRangeAttack：3/4 Slow 分支（内层 1/2，5s）/ 1/4 Frozen 分支（内层 1/4，5s）
                    if fastrand::i32(0..4) > 0 {
                        if fastrand::i32(0..2) == 0 {
                            ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                                session_id: target.session_id,
                                poison: Poison::new(PoisonType::SLOW, 5, 0, 1000),
                            });
                        }
                    } else if fastrand::i32(0..4) == 0 {
                        ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                            session_id: target.session_id,
                            poison: Poison::new(PoisonType::FROZEN, 5, 0, 1000),
                        });
                    }
                }
                // 普通：RhinoPriestDebuff（DC/MC/SC 降低）用红毒近似持续减益
                else {
                    ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                        session_id: target.session_id,
                        poison: Poison::new(PoisonType::RED, 5, damage / 2, 1000),
                    });
                }
            }
            return;
        }

        // 风筝走位（继承 AxeSkeleton）
        if ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = if dist >= ATTACK_RANGE {
                step_toward(monster.x, monster.y, target.x, target.y)
            } else if dist < 3 {
                step_away(monster.x, monster.y, target.x, target.y)
            } else {
                return;
            };
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + 2;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}

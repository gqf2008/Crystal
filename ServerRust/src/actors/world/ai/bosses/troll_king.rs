//! TrollKing（巨魔王）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/TrollKing.cs
//! 机制：
//!   - AttackRange=7，近/远双模式 + 风筝走位（FearTime 控制攻击/逃跑）
//!   - 近战（<=3 格）：MC 范围 3 格 AOE（MACAgility），2/3 概率攻击、1/3 WalkAway
//!   - 远程（>3 格）：投石，目标点 3 格 AOE（ACAgility），命中后 Dazed 毒（投石）
//!   - dist>=AttackRange 追击，否则 WalkAway 拉开
//!
//! Attack（C# :21-79）：<=3 MC AOE(MACAgility)；>3 DC 范围(ACAgility)。
//! CompleteRangeAttack（C# :92-105）：Attacked + MACAgility 二次 + Dazed。

use crate::actors::world::MonsterState;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const ATTACK_RANGE: i32 = 7;
const VIEW_RANGE: i32 = 15;
/// 近战判定（C# InRange(,3)）
const MELEE_RANGE: i32 = 3;
const SPLASH_RADIUS: i32 = 3;

pub struct TrollKingBehavior;

impl TrollKingBehavior {
    pub fn new() -> Self { Self }
}

impl MonsterBehavior for TrollKingBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= ATTACK_RANGE && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + 8;

            if dist <= MELEE_RANGE {
                // 近战：2/3 概率范围攻击，1/3 WalkAway（C# Random(2)==0 || !InRange(,2)）
                if fastrand::i32(0..2) == 0 || dist > 2 {
                    // MC 范围 3 格 AOE（C# FindAllTargets(3) MACAgility）
                    let damage = crate::combat::attack::get_attack_power(monster.min_mac, monster.max_mac, 0).max(1);
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Aoe {
                        attacker_oid: monster.object_id,
                        center_x: monster.x,
                        center_y: monster.y,
                        radius: SPLASH_RADIUS,
                        damage,
                        spell_id: 0,
                    });
                } else {
                    // WalkAway（拉开）
                    let (nx, ny, dir) = step_away(monster.x, monster.y, target.x, target.y);
                    ctx.out_moves.push((monster.object_id, nx, ny, dir));
                    monster.next_move_tick = ctx.tick_count + 2;
                }
            } else {
                // 远程投石：目标点 3 格 AOE（ACAgility）+ 命中后 Dazed（C# DefenceType.ACAgility）
                let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Aoe {
                    attacker_oid: monster.object_id,
                    center_x: target.x,
                    center_y: target.y,
                    radius: SPLASH_RADIUS,
                    damage,
                    spell_id: 0,
                });
                // C# CompleteRangeAttack：命中后 Dazed 1s
                // C# PoisonTarget(1, random(MaxMC), Dazed, 1000)：恒生效、时长=random(MaxMC)（DC 近似）
                    ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                        session_id: target.session_id,
                        poison: Poison::new(PoisonType::DAZED, fastrand::i32(0..damage.max(1)) as u32, 0, 1000),
                    });
            }
            return;
        }

        // 走位：远了追近，近了（已在射程内）WalkAway 拉开（C# ProcessTarget :125-133）
        if ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = if dist >= ATTACK_RANGE {
                step_toward(monster.x, monster.y, target.x, target.y)
            } else {
                step_away(monster.x, monster.y, target.x, target.y)
            };
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + 2;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}

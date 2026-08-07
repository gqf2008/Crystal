//! ThunderElement（雷元素）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/ThunderElement.cs
//! 机制：
//!   - 闪电链：Attack 对自身 2 格范围全体 MAC 攻击（FindAllTargets(2)）
//!   - 攻击时 1/3 概率随机位移到目标 ±1 格（闪现）
//!   - 免疫所有非 Repulsion 伤害（Attacked 仅 Repulsion 生效）
//!   - 免疫毒（PoisonDamage 直接 return）
//!
//! Attack（C# :61-77）：DC + DelayedAction MAC。
//! CompleteAttack（C# :14-32）：FindAllTargets(2) 逐个 Attacked MAC。
//! ProcessTarget（C# :34-59）：1/3 概率 MoveTo Target±1；Attack。
//! Attacked（C# :79-90）：type != Repulsion → return 0（免疫）。

use crate::actors::world::MonsterState;
use crate::combat::poison::Poison;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 15;
const MELEE_RANGE: i32 = 2;

pub struct ThunderElementBehavior;

impl ThunderElementBehavior {
    pub fn new() -> Self { Self }
}

impl MonsterBehavior for ThunderElementBehavior {
    /// 免疫毒（C# PoisonDamage 直接 return）
    fn on_poison(&mut self, _poison: Poison) -> bool { false }

    /// C# Attacked：type != Repulsion → return 0（雷元素仅受击退伤害）
    /// 此处 behavior 层无法区分 DefenceType，统一按"免疫常规伤害"近似：
    /// 实际 Repulsion 推挤伤害由 Pushed 路径处理（上层），这里 on_attacked 返 0。
    fn on_attacked(&mut self, _damage: i32) -> i32 { 0 }

    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= MELEE_RANGE {
            if ctx.tick_count >= monster.next_attack_tick {
                monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
                // 闪电链：自身 2 格范围全体 MAC（C# CompleteAttack FindAllTargets(2)）
                let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Aoe {
                    attacker_oid: monster.object_id,
                    center_x: monster.x,
                    center_y: monster.y,
                    radius: MELEE_RANGE,
                    damage,
                    spell_id: 0,
                });
            }
            // 1/3 概率闪现到目标 ±1 格（C# ProcessTarget Random(3)==1）
            if fastrand::i32(0..3) == 1 && ctx.tick_count >= monster.next_move_tick {
                let nx = target.x + fastrand::i32(-1..=1);
                let ny = target.y + fastrand::i32(-1..=1);
                ctx.out_moves.push((monster.object_id, nx, ny, monster.direction));
                monster.next_move_tick = ctx.tick_count + 2;
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

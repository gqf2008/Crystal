//! FlameQueen（火焰女王）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/FlameQueen.cs
//! 机制：
//!   - AttackRange=3，可移动追击
//!   - HP<20% 周期 MassAttack：FindAllTargets(7) 全体 MC 远程弹道（延迟按距离）
//!   - 近战：若非贴身或 1/3 概率 → Type=1 近战；否则 Type=0 弹道
//!
//! #2859：**删除凭空生成的 `FireWall` 法术场**——原版 `FlameQueen.cs` 全文没有任何 `SpellObject`，
//! HP<20% 阶段就是一个「7 格内全体 MC 延迟弹道」的群伤（本端此前额外投放火墙，属于原版不存在的行为）。

use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::actors::world::MonsterState;

/// 视野范围
const VIEW_RANGE: i32 = 20;
/// 攻击范围（C# AttackRange = 3）
const ATTACK_RANGE: i32 = 3;
/// 近战判定
const MELEE_RANGE: i32 = 1;
/// MassAttack 周期：2-7s（C# 2000 + Random(5)*1000 ms）
const MASS_ATTACK_MIN_TICKS: u64 = 20;
/// MassAttack 半径（C# `FindAllTargets(7, CurrentLocation, false)`）
const MASS_ATTACK_RADIUS: i32 = 7;
/// MassAttack 分支的 `ActionTime = +800ms`（C# `FlameQueen.cs:56`）
const MASS_ACTION_TICKS: u64 = 8;

/// #2859：C# MassAttack 分支的有效冷却 = `max(ActionTime(800ms), AttackTime(AttackSpeed))`
pub(crate) fn mass_attack_cooldown_ticks(attack_cooldown: u64) -> u64 {
    attack_cooldown.max(MASS_ACTION_TICKS)
}

pub struct FlameQueenBehavior {
    /// 下次 MassAttack 的 tick（C# MassAttackTime）
    next_mass_tick: u64,
    spawned: bool,
}

impl Default for FlameQueenBehavior {
    fn default() -> Self {
        Self::new()
    }
}

impl FlameQueenBehavior {
    pub fn new() -> Self {
        Self {
            next_mass_tick: 0,
            spawned: false,
        }
    }
}

impl MonsterBehavior for FlameQueenBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        if !self.spawned {
            self.spawned = true;
        }

        // 无目标则返回
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);

        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        // C# `Attack()` 只在 `InAttackRange()`（AttackRange=3）内被调用；超出则 `ProcessTarget` 走近
        if dist > ATTACK_RANGE {
            if ctx.tick_count < monster.next_move_tick {
                return;
            }
            let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + monster.ai_profile.move_interval;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
            return;
        }
        // C# `CanAttack` 门控
        if ctx.tick_count < monster.next_attack_tick {
            return;
        }

        let hp_pct = if monster.max_hp > 0 {
            (monster.hp * 100) / monster.max_hp
        } else {
            0
        };

        // ---- HP<20% 阶段：周期 MassAttack（C# FlameQueen.cs:33-59）----
        if hp_pct < 20 && (self.next_mass_tick == 0 || ctx.tick_count >= self.next_mass_tick) {
            // C# MassAttackTime = Envir.Time + 2000 + Random(5)*1000
            self.next_mass_tick = ctx.tick_count + MASS_ATTACK_MIN_TICKS + fastrand::u64(0..5) * 10;
            // C# 本分支先 `ActionTime = +500` 再 `+800`、`AttackTime = +AttackSpeed`
            // ⇒ 有效冷却 = max(800ms, AttackSpeed)
            monster.next_attack_tick =
                ctx.tick_count + mass_attack_cooldown_ticks(monster.ai_profile.attack_cooldown);

            // C# `FindAllTargets(7, CurrentLocation, false)`：7 格内每个目标各挂一次
            // `DelayedAction(RangeDamage, 距离*50 + 750)`；本端用 `AttackAction::Range` 表达
            // （每目标一条弹道 ⇒ 各自广播一次 `ObjectRangeAttack`；本端飞行延迟常量为 +550ms）
            let targets: Vec<(u64, u32)> = ctx
                .find_targets_in_range(monster.x, monster.y, MASS_ATTACK_RADIUS, monster.map_index)
                .iter()
                .map(|t| (t.session_id, t.object_id))
                .collect();
            if targets.is_empty() {
                return;
            }
            let damage = crate::combat::attack::get_attack_power(monster.min_mc, monster.max_mc, 0);
            for (session_id, object_id) in targets {
                ctx.out_attacks
                    .push(crate::actors::world::ai::AttackAction::Range {
                        attacker_oid: monster.object_id,
                        target_session: session_id,
                        target_object_id: object_id,
                        damage,
                        spell_id: 0,
                    });
            }
            return;
        }

        // ---- 近战 / 弹道（C# :61-78）----
        monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
        let not_adjacent = dist > MELEE_RANGE;
        // C# `!InRange(1) || Random(3)==0` → Type=1 近战 Damage；否则（贴身 2/3）→ Type=0 RangeDamage
        let melee_anim = not_adjacent || fastrand::i32(0..3) == 0;
        // C# 此分支**没有** `damage == 0 → return` 短路，0 伤害也照常出手
        let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0);
        if melee_anim {
            ctx.out_attacks
                .push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage,
                    spell_id: 0,
                    attack_type: 1,
                });
        } else {
            ctx.out_attacks
                .push(crate::actors::world::ai::AttackAction::Range {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    target_object_id: target.object_id,
                    damage,
                    spell_id: 0,
                });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #2859：C# `FlameQueen.cs:41-57`——MassAttack 半径 7；本分支有效冷却 = `max(ActionTime 800ms, AttackSpeed)`
    #[test]
    fn mass_attack_params_match_csharp() {
        assert_eq!(MASS_ATTACK_RADIUS, 7);
        assert_eq!(MASS_ACTION_TICKS, 8);
        assert_eq!(mass_attack_cooldown_ticks(0), 8);
        assert_eq!(mass_attack_cooldown_ticks(3), 8);
        assert_eq!(mass_attack_cooldown_ticks(20), 20);
        // C# `MassAttackTime = now + 2000 + Random(5)*1000`
        assert_eq!(MASS_ATTACK_MIN_TICKS, 20);
    }
}

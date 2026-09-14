//! EarthGolem（地魔像）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/EarthGolem.cs（继承 ZumaMonster）
//! 机制：
//!   - 继承 ZumaMonster 石化休眠：FindNearby(4) 才唤醒（Stoned→Wake）
//!   - AttackRange=6；近战（<=1，2/3 概率）DC MAC；
//!     远程 MC：在目标点生成 3x3 地面冲击法术场（EarthGolemPile，1.2s + 0.5s 延迟）
//!   - 风筝走位（FearTime 2s）
//!
//! Attack（C# :49-119）：!ranged&&Random(3)>0→DC MAC；else→目标点 3x3 EarthGolemPile 法术场。
//! ProcessAI（C# :30-47）：FindNearby(4) 唤醒。

use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::actors::world::MonsterState;
use mir2_shared::enums::Spell;

const ATTACK_RANGE: i32 = 6;
const VIEW_RANGE: i32 = 15;
const MELEE_RANGE: i32 = 1;
/// 唤醒检测范围（C# FindNearby(4)）
const WAKE_RANGE: i32 = 4;
/// FearTime 持续（C# Envir.Time + 2000）
const FEAR_TICKS: u64 = 20;
/// C# `EarthGolem.cs:99`：`EarthGolemPile` 的 `start = 500`
const PILE_START_MS: u64 = 500;
/// C# `EarthGolem.cs:105`：`ExpireTime = 1200 + start`
const PILE_DURATION_MS: u64 = 1200;
/// C# `EarthGolem.cs:81-91`：`EarthGolemPile` 面积 = 目标点 ±1（3×3）
const PILE_RADIUS: i32 = 1;

pub struct EarthGolemBehavior {
    /// 是否石化休眠（继承 ZumaMonster Stoned）
    stoned: bool,
    fear_end_tick: u64,
}

impl Default for EarthGolemBehavior {
    fn default() -> Self {
        Self::new()
    }
}

impl EarthGolemBehavior {
    pub fn new() -> Self {
        Self {
            stoned: true,
            fear_end_tick: 0,
        }
    }
}

impl MonsterBehavior for EarthGolemBehavior {
    /// 石化期不可被攻击（继承 ZumaMonster IsAttackTarget = !Stoned）
    fn is_attackable(&self) -> bool {
        !self.stoned
    }

    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        // 石化唤醒检测（C# ProcessAI FindNearby(4)）
        if self.stoned {
            if ctx
                .nearest_target(monster.x, monster.y, WAKE_RANGE, monster.map_index)
                .is_some()
            {
                self.stoned = false; // C# Wake()
            } else {
                return; // 休眠中
            }
        }

        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= ATTACK_RANGE
            && ctx.tick_count < self.fear_end_tick
            && ctx.tick_count >= monster.next_attack_tick
        {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;

            if dist <= MELEE_RANGE && fastrand::i32(0..3) > 0 {
                // 近战 DC MAC（C# 2/3 概率）
                let damage =
                    crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0)
                        .max(1);
                ctx.out_attacks
                    .push(crate::actors::world::ai::AttackAction::Melee {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        damage,
                        spell_id: 0,
                        attack_type: 0,
                    });
            } else {
                // 远程：目标点 3×3 地面冲击法术场（C# EarthGolem.cs:79-117）
                // ——跳怪自身格 + `cell.Valid` 过滤、`start = 500`（总寿命 1200 + start）、`TickSpeed = 1000`、
                //    `Show` 仅锚点格（本端聚合为「1 对象 + cells」并在锚点广播）
                let cells = crate::actors::world::ai::helpers::area_cells(
                    target.x,
                    target.y,
                    PILE_RADIUS,
                    Some((monster.x, monster.y)),
                    |x, y| (ctx.is_walkable)(x, y),
                );
                if cells.is_empty() {
                    return;
                }
                let damage =
                    crate::combat::attack::get_attack_power(monster.min_mc, monster.max_mc, 0)
                        .max(1);
                ctx.out_spell_fields
                    .push(crate::actors::world::ai::SpellFieldSpawn {
                        spell: Spell::EarthGolemPile,
                        x: target.x,
                        y: target.y,
                        value: damage,
                        duration_ms: PILE_DURATION_MS,
                        tick_ms: 1000,
                        caster_oid: monster.object_id,
                        caster_session: 0,
                        cells,
                        show: true,
                        start_delay_ms: PILE_START_MS,
                    });
            }
            return;
        }

        // 刷新 FearTime
        self.fear_end_tick = ctx.tick_count + FEAR_TICKS;

        // 走位：过近拉开，远了追近
        if ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = if dist < ATTACK_RANGE {
                step_away(monster.x, monster.y, target.x, target.y)
            } else {
                step_toward(monster.x, monster.y, target.x, target.y)
            };
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + monster.ai_profile.move_interval;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #2859：C# `EarthGolem.cs:79-115`——`EarthGolemPile` 面积 = 目标点 ±1（3×3）、跳自身格、
    /// `start = 500`、`ExpireTime = 1200 + start`、`TickSpeed = 1000`
    #[test]
    fn pile_field_params_match_csharp() {
        assert_eq!(PILE_RADIUS, 1);
        assert_eq!(PILE_START_MS, 500);
        assert_eq!(PILE_DURATION_MS, 1200);
        let cells = crate::actors::world::ai::helpers::area_cells(
            0,
            0,
            PILE_RADIUS,
            Some((0, 0)),
            |_, _| true,
        );
        assert_eq!(cells.len(), 8, "3×3 跳自身格 = 8 格");
        let (expires_ms, last_tick_shift_ms) = crate::actors::world::spell::delayed_spell_timing(
            PILE_START_MS,
            PILE_DURATION_MS,
            1000,
        );
        assert_eq!((expires_ms, last_tick_shift_ms), (1700, -500));
    }
}

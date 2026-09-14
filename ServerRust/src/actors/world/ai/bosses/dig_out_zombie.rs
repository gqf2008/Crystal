//! DigOutZombie（钻地僵尸）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/DigOutZombie.cs
//! 机制：
//!   - 默认钻地隐身（Visible=false）：不可移动、不可攻击、不可被攻击、不阻挡
//!   - 玩家靠近 3 格内钻出（Visible=true），可移动/攻击
//!   - 每 2s 检测一次（VisibleTime）
//!
//! ProcessAI（C# :42-66）：Envir.Time>VisibleTime 时 FindNearby(3) 切换 Visible。
//! CanMove/CanAttack/Blocking（C# :14-34）：均要求 Visible。

use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::actors::world::MonsterState;

const APPEAR_RANGE: i32 = 3;
const VIEW_RANGE: i32 = 12;
const MELEE_RANGE: i32 = 1;
const CHECK_TICKS: u64 = 20;
/// #2861：C# `DigOutZombie.cs:70`——`Envir.Time > DigOutTime + 1000`（1s 后生成洞口）
pub(crate) const HOLE_DELAY_TICKS: u64 = 10;
/// #2861：C# `DigOutZombie.cs:76`——洞口 `ExpireTime = now + 5min`、`TickSpeed = 2000`
pub(crate) const HOLE_DURATION_MS: u64 = 5 * 60 * 1000;
pub(crate) const HOLE_TICK_MS: u64 = 2000;

/// #2861：C# `SpawnDigOutEffect`（`DigOutZombie.cs:68-87`）的触发条件——
/// `Visible && Envir.Time > DigOutTime + delay && !DoneDigOut`。
pub(crate) fn hole_ready(
    visible: bool,
    now_tick: u64,
    dig_out_tick: u64,
    delay_ticks: u64,
    done: bool,
) -> bool {
    visible && !done && now_tick >= dig_out_tick.saturating_add(delay_ticks)
}

pub struct DigOutZombieBehavior {
    visible: bool,
    next_check_tick: u64,
    spawned: bool,
    /// 钻出时刻（tick；1s 后生成洞口，C# DigOutTime + 1000）
    dig_out_tick: u64,
    /// #2861：钻出瞬间的坐标（C# `DigOutLocation`）——洞口必须落在**这里**，而不是 1s 后的当前位置
    dig_out_x: i32,
    dig_out_y: i32,
    /// 洞口是否已生成（C# DoneDigOut）
    hole_done: bool,
}

impl Default for DigOutZombieBehavior {
    fn default() -> Self {
        Self::new()
    }
}

impl DigOutZombieBehavior {
    pub fn new() -> Self {
        Self {
            visible: false,
            next_check_tick: 0,
            spawned: false,
            dig_out_tick: 0,
            dig_out_x: 0,
            dig_out_y: 0,
            hole_done: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #2861：C# `DigOutZombie.cs:68-87`——洞口触发条件 = `Visible && now > DigOutTime + 1000 && !DoneDigOut`
    #[test]
    fn hole_trigger_matches_csharp() {
        // 钻出于 tick 100、延迟 10 tick ⇒ tick 110 起可生成
        assert!(!hole_ready(true, 109, 100, HOLE_DELAY_TICKS, false));
        assert!(hole_ready(true, 110, 100, HOLE_DELAY_TICKS, false));
        // 不可见 / 已生成过 → 都不再生成
        assert!(!hole_ready(false, 200, 100, HOLE_DELAY_TICKS, false));
        assert!(!hole_ready(true, 200, 100, HOLE_DELAY_TICKS, true));
        assert_eq!(HOLE_DELAY_TICKS, 10);
        assert_eq!(HOLE_DURATION_MS, 5 * 60 * 1000);
        assert_eq!(HOLE_TICK_MS, 2000);
    }
}

impl MonsterBehavior for DigOutZombieBehavior {
    fn can_move(&self) -> bool {
        self.visible
    }
    fn is_attackable(&self) -> bool {
        self.visible
    }

    fn on_attacked(&mut self, damage: i32) -> i32 {
        if self.visible {
            damage
        } else {
            0
        }
    }

    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        if !self.spawned {
            self.next_check_tick = ctx.tick_count + CHECK_TICKS;
            self.spawned = true;
        }

        // 每 2s 检测钻出（C# ProcessAI）
        if ctx.tick_count >= self.next_check_tick {
            self.next_check_tick = ctx.tick_count + CHECK_TICKS;
            let has_near = ctx
                .nearest_target(monster.x, monster.y, APPEAR_RANGE, monster.map_index)
                .is_some();
            if !self.visible && has_near {
                self.visible = true;
                self.dig_out_tick = ctx.tick_count;
                // #2861：C# `DigOutZombie.cs:58`——记录钻出瞬间的坐标（洞口随后按此坐标生成）
                self.dig_out_x = monster.x;
                self.dig_out_y = monster.y;
                self.hole_done = false;
            }
        }

        if !self.visible {
            return;
        }

        // C# `SpawnDigOutEffect`（`:68-87`）：钻出 1s 后生成洞口 SpellObject（5 分钟，供 NeedHole 传送点使用）
        // ——落点用**钻出瞬间记录的坐标**、`Show=false`（C# 未设 Show 且 DigOut* 不在广播名单）、`Caster=null`
        if hole_ready(
            self.visible,
            ctx.tick_count,
            self.dig_out_tick,
            HOLE_DELAY_TICKS,
            self.hole_done,
        ) {
            self.hole_done = true;
            ctx.out_spell_fields
                .push(crate::actors::world::ai::SpellFieldSpawn {
                    spell: mir2_shared::enums::Spell::DigOutZombie,
                    x: self.dig_out_x,
                    y: self.dig_out_y,
                    value: 1,
                    duration_ms: HOLE_DURATION_MS,
                    tick_ms: HOLE_TICK_MS,
                    caster_oid: 0,
                    caster_session: 0,
                    cells: Vec::new(),
                    show: false,
                    start_delay_ms: 0,
                });
        }

        // 活跃期标准近战
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);
        if dist <= MELEE_RANGE {
            if ctx.tick_count >= monster.next_attack_tick {
                monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
                let damage = crate::combat::attack::get_attack_power(
                    monster.min_dmg,
                    monster.max_dmg,
                    monster.luck,
                )
                .max(1);
                ctx.out_attacks
                    .push(crate::actors::world::ai::AttackAction::Melee {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        damage,
                        spell_id: 0,
                        attack_type: 0,
                    });
            }
        } else if ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + monster.ai_profile.move_interval;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}

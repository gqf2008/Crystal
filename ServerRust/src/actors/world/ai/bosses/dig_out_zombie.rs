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

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const APPEAR_RANGE: i32 = 3;
const VIEW_RANGE: i32 = 12;
const MELEE_RANGE: i32 = 1;
const CHECK_TICKS: u64 = 20;

pub struct DigOutZombieBehavior {
    visible: bool,
    next_check_tick: u64,
    spawned: bool,
    /// 钻出时刻（tick；1s 后生成洞口，C# DigOutTime + 1000）
    dig_out_tick: u64,
    /// 洞口是否已生成（C# DoneDigOut）
    hole_done: bool,
}

impl DigOutZombieBehavior {
    pub fn new() -> Self {
        Self { visible: false, next_check_tick: 0, spawned: false, dig_out_tick: 0, hole_done: false }
    }
}

impl MonsterBehavior for DigOutZombieBehavior {
    fn can_move(&self) -> bool { self.visible }
    fn is_attackable(&self) -> bool { self.visible }

    fn on_attacked(&mut self, damage: i32) -> i32 {
        if self.visible { damage } else { 0 }
    }

    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        if !self.spawned {
            self.next_check_tick = ctx.tick_count + CHECK_TICKS;
            self.spawned = true;
        }

        // 每 2s 检测钻出（C# ProcessAI）
        if ctx.tick_count >= self.next_check_tick {
            self.next_check_tick = ctx.tick_count + CHECK_TICKS;
            let has_near = ctx.nearest_target(monster.x, monster.y, APPEAR_RANGE, monster.map_index).is_some();
            if !self.visible && has_near {
                self.visible = true;
                self.dig_out_tick = ctx.tick_count;
                self.hole_done = false;
            }
        }

        if !self.visible {
            return;
        }

        // C# SpawnDigOutEffect：钻出 1s 后生成洞口 SpellObject（5 分钟，供 NeedHole 传送点使用）
        if !self.hole_done && ctx.tick_count >= self.dig_out_tick + 10 {
            self.hole_done = true;
            ctx.out_spell_fields.push(crate::actors::world::ai::SpellFieldSpawn {
                spell: mir2_shared::enums::Spell::DigOutZombie,
                x: monster.x,
                y: monster.y,
                value: 1,
                duration_ms: 300_000,
                tick_ms: 2000,
                caster_oid: monster.object_id,
                caster_session: 0,
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
                let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
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
            monster.next_move_tick = ctx.tick_count + 2;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}

//! StoneGolem（石头傀儡）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/StoneGolem.cs
//! 机制：
//!   - AttackRange=4
//!   - 近战 Type0 DC 单体（AC 防御）
//!   - 远程 Type1：朝向方向 3 格处投放 StoneGolemQuake 法术场（5x5 AOE）
//!
//! Attack（C# :28-97）：近战/远程分支；远程→前方 3 格 5x5 Quake 法术场。

use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::actors::world::MonsterState;
use mir2_shared::enums::Spell;

const VIEW_RANGE: i32 = 15;
const ATTACK_RANGE: i32 = 4;
const MELEE_RANGE: i32 = 1;
/// 法术场投放点距自身的格数（C# PointMove(CurrentLocation, Direction, 3)）
const QUAKE_OFFSET: i32 = 3;
/// 5x5 法术场半径（C# y-2..=y+2, x-2..=x+2）
const QUAKE_RADIUS: i32 = 2;
/// C# `StoneGolem.cs:76`：`start = 500`（`ExpireTime = 800 + start`）
const QUAKE_START_MS: u64 = 500;
/// C# `StoneGolem.cs:82`：`ExpireTime = 800 + start`
const QUAKE_DURATION_MS: u64 = 800;

pub struct StoneGolemBehavior;

impl Default for StoneGolemBehavior {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #2859：C# `StoneGolem.cs:56-96`——`PointMove(Direction,3)` 处 5×5（**不跳自身格**）、
    /// `start = 500`、`ExpireTime = 800 + start`、`TickSpeed = 1000`
    #[test]
    fn quake_field_params_match_csharp() {
        assert_eq!(QUAKE_OFFSET, 3);
        assert_eq!(QUAKE_RADIUS, 2);
        assert_eq!(QUAKE_START_MS, 500);
        assert_eq!(QUAKE_DURATION_MS, 800);
        let cells =
            crate::actors::world::ai::helpers::area_cells(50, 50, QUAKE_RADIUS, None, |_, _| true);
        assert_eq!(cells.len(), 25);
        assert!(cells.contains(&(50, 50)));
        let (expires_ms, last_tick_shift_ms) = crate::actors::world::spell::delayed_spell_timing(
            QUAKE_START_MS,
            QUAKE_DURATION_MS,
            1000,
        );
        assert_eq!((expires_ms, last_tick_shift_ms), (1300, -500));
    }
}

impl StoneGolemBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for StoneGolemBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= ATTACK_RANGE && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let melee = dist <= MELEE_RANGE;

            if melee {
                // Type0 DC 单体
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
                // Type1 前方 3 格处 5x5 Quake 法术场
                // C# PointMove(CurrentLocation, Direction, 3) 精确 3 格（StoneGolem.cs:73）
                let dir =
                    (direction_towards(monster.x, monster.y, target.x, target.y) as usize) % 8;
                let center_x = monster.x + DIR_DX[dir] * QUAKE_OFFSET;
                let center_y = monster.y + DIR_DY[dir] * QUAKE_OFFSET;
                let value =
                    crate::combat::attack::get_attack_power(monster.min_mc, monster.max_mc, 0)
                        .max(1);
                // 5×5 法术场（C# StoneGolem.cs:56-96）：`cell.Valid` 过滤、**不跳自身格**、
                // `start = 500`（总寿命 800 + start）、`TickSpeed = 1000`、`Show` 仅锚点格
                // （C# 每格一个 SpellObject，本端聚合为「1 对象 + cells」并在锚点广播）
                let cells = crate::actors::world::ai::helpers::area_cells(
                    center_x,
                    center_y,
                    QUAKE_RADIUS,
                    None,
                    |x, y| (ctx.is_walkable)(x, y),
                );
                if !cells.is_empty() {
                    ctx.out_spell_fields
                        .push(crate::actors::world::ai::SpellFieldSpawn {
                            spell: Spell::StoneGolemQuake,
                            x: center_x,
                            y: center_y,
                            value,
                            duration_ms: QUAKE_DURATION_MS,
                            tick_ms: 1000,
                            caster_oid: monster.object_id,
                            caster_session: 0,
                            cells,
                            show: true,
                            start_delay_ms: QUAKE_START_MS,
                        });
                }
            }
            return;
        }

        // 追击
        if dist > ATTACK_RANGE && ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + monster.ai_profile.move_interval;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}

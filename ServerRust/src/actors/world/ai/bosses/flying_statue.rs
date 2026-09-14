//! FlyingStatue（飞石像）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/FlyingStatue.cs
//! 机制：近战（dist<=1）5/6 普攻 DC / 1/6 魔法 MC；
//!      远程 → SpawnIceTornado：随机目标 3x3 每格 SpellObject（Spell.FlyingStatueIceTornado，值=MC，1500+500ms，tick 3000）
//!      半血后风筝（<视野远离，>=视野接近）

use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::actors::world::MonsterState;
use mir2_shared::enums::Spell;

const VIEW_RANGE: i32 = 12;
/// C# `FlyingStatue.cs:103`：`start = 500`
const TORNADO_START_MS: u64 = 500;
/// C# `FlyingStatue.cs:109`：`ExpireTime = 1500 + start`
const TORNADO_DURATION_MS: u64 = 1500;
/// C# `FlyingStatue.cs:85-90`：冰龙卷面积 = 锚点 ±1（3×3）
const TORNADO_RADIUS: i32 = 1;

pub struct FlyingStatueBehavior;

impl Default for FlyingStatueBehavior {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #2859：C# `FlyingStatue.cs:85-118`——锚点 ±1（3×3）、跳自身格、`start = 500`、
    /// `ExpireTime = 1500 + start`、`TickSpeed = 3000`
    #[test]
    fn tornado_field_params_match_csharp() {
        assert_eq!(TORNADO_RADIUS, 1);
        assert_eq!(TORNADO_START_MS, 500);
        assert_eq!(TORNADO_DURATION_MS, 1500);
        let cells = crate::actors::world::ai::helpers::area_cells(
            10,
            10,
            TORNADO_RADIUS,
            Some((10, 10)),
            |_, _| true,
        );
        assert_eq!(cells.len(), 8);
        let (expires_ms, last_tick_shift_ms) = crate::actors::world::spell::delayed_spell_timing(
            TORNADO_START_MS,
            TORNADO_DURATION_MS,
            3000,
        );
        assert_eq!((expires_ms, last_tick_shift_ms), (2000, -2500));
    }
}

impl FlyingStatueBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for FlyingStatueBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);
        let damage =
            crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck)
                .max(1);
        // C# Type1 魔法近战 / SpawnIceTornado 用 MinMC/MaxMC
        let mc_damage =
            crate::combat::attack::get_attack_power(monster.min_mc, monster.max_mc, monster.luck)
                .max(1);

        if dist <= VIEW_RANGE && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            if dist <= 1 {
                // C# Random.Next(6) != 0：5/6 普攻 / 1/6 魔法
                if fastrand::i32(0..6) != 0 {
                    ctx.out_attacks
                        .push(crate::actors::world::ai::AttackAction::Melee {
                            attacker_oid: monster.object_id,
                            target_session: target.session_id,
                            damage,
                            spell_id: 0,
                            attack_type: 0,
                        });
                } else {
                    // C# Type1：MinMC/MaxMC 魔法近战
                    ctx.out_attacks
                        .push(crate::actors::world::ai::AttackAction::Melee {
                            attacker_oid: monster.object_id,
                            target_session: target.session_id,
                            damage: mc_damage,
                            spell_id: 0,
                            attack_type: 1,
                        });
                }
            } else {
                // C# `SpawnIceTornado`（`FlyingStatue.cs:75-122`）：从 `FindAllTargets(ViewRange)`
                // （**玩家+怪物**）随机取一个锚点，其脚下 3×3、跳怪自身格与非法格、`start = 500`、
                // `ExpireTime = 1500 + start`、`TickSpeed = 3000`、`Show` 仅锚点格。
                let mut candidates: Vec<(i32, i32)> = ctx
                    .find_targets_in_range(monster.x, monster.y, VIEW_RANGE, monster.map_index)
                    .iter()
                    .map(|p| (p.x, p.y))
                    .collect();
                candidates.extend(
                    ctx.monsters
                        .iter()
                        .filter(|m| {
                            m.object_id != monster.object_id
                                && m.map_index == monster.map_index
                                && m.hp > 0
                        })
                        .map(|m| (m.x, m.y)),
                );
                let (cx, cy) = if candidates.is_empty() {
                    (target.x, target.y)
                } else {
                    candidates[fastrand::usize(0..candidates.len())]
                };
                let cells = crate::actors::world::ai::helpers::area_cells(
                    cx,
                    cy,
                    TORNADO_RADIUS,
                    Some((monster.x, monster.y)),
                    |x, y| (ctx.is_walkable)(x, y),
                );
                if cells.is_empty() {
                    return;
                }
                ctx.out_spell_fields
                    .push(crate::actors::world::ai::SpellFieldSpawn {
                        spell: Spell::FlyingStatueIceTornado,
                        x: cx,
                        y: cy,
                        value: mc_damage,
                        duration_ms: TORNADO_DURATION_MS,
                        tick_ms: 3000,
                        caster_oid: monster.object_id,
                        caster_session: 0,
                        cells,
                        show: true,
                        start_delay_ms: TORNADO_START_MS,
                    });
            }
            return;
        }

        if ctx.tick_count >= monster.next_move_tick {
            // C# ProcessTarget：半血后 <视野远离，>=视野接近
            let hp_pct = if monster.max_hp > 0 {
                monster.hp * 100 / monster.max_hp
            } else {
                100
            };
            let (nx, ny, dir) = if hp_pct <= 50 && dist < VIEW_RANGE {
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

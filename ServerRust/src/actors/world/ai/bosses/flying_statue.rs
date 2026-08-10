//! FlyingStatue（飞石像）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/FlyingStatue.cs
//! 机制：近战（dist<=1）5/6 普攻 DC / 1/6 魔法 MC；
//!      远程 → SpawnIceTornado：随机目标 3x3 每格 SpellObject（Spell.FlyingStatueIceTornado，值=MC，1500+500ms，tick 3000）
//!      半血后风筝（<视野远离，>=视野接近）

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use mir2_shared::enums::Spell;

const VIEW_RANGE: i32 = 12;

pub struct FlyingStatueBehavior;

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
        let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
        // C# Type1 魔法近战 / SpawnIceTornado 用 MinMC/MaxMC
        let mc_damage = crate::combat::attack::get_attack_power(monster.min_mac, monster.max_mac, monster.luck).max(1);

        if dist <= VIEW_RANGE && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            if dist <= 1 {
                // C# Random.Next(6) != 0：5/6 普攻 / 1/6 魔法
                if fastrand::i32(0..6) != 0 {
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        damage,
                        spell_id: 0,
                        attack_type: 0,
                    });
                } else {
                    // C# Type1：MinMC/MaxMC 魔法近战
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        damage: mc_damage,
                        spell_id: 0,
                        attack_type: 1,
                    });
                }
            } else {
                // C# SpawnIceTornado：视野内随机目标（targets[Random.Next(count)]）3x3 法术场，值=MC
                let candidates: Vec<crate::actors::world::ai::PlayerSnap> =
                    ctx.find_targets_in_range(monster.x, monster.y, VIEW_RANGE, monster.map_index)
                        .into_iter().copied().collect();
                let (cx, cy) = if candidates.is_empty() {
                    (target.x, target.y)
                } else {
                    let p = candidates[fastrand::usize(0..candidates.len())];
                    (p.x, p.y)
                };
                for dy in -1..=1i32 {
                    for dx in -1..=1i32 {
                        let fx = cx + dx;
                        let fy = cy + dy;
                        // C# 跳过怪自身所在格（x==CurrentLocation.X && y==CurrentLocation.Y）
                        if fx == monster.x && fy == monster.y { continue; }
                        ctx.out_spell_fields.push(crate::actors::world::ai::SpellFieldSpawn {
                            spell: Spell::FlyingStatueIceTornado,
                            x: fx,
                            y: fy,
                            value: mc_damage,
                            duration_ms: 2000,
                            tick_ms: 3000,
                            caster_oid: monster.object_id,
                            caster_session: 0,
                        });
                    }
                }
            }
            return;
        }

        if ctx.tick_count >= monster.next_move_tick {
            // C# ProcessTarget：半血后 <视野远离，>=视野接近
            let hp_pct = if monster.max_hp > 0 { monster.hp * 100 / monster.max_hp } else { 100 };
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

//! FlameQueen（火焰女王）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/FlameQueen.cs
//! 机制：
//!   - AttackRange=3，可移动追击
//!   - HP<20% 周期 MassAttack：FindAllTargets(7) 全体 MC 远程弹道（延迟按距离）
//!   - 近战：若非贴身或 1/3 概率 → Type=1 近战；否则 Type=0 弹道
//!
//! 任务核心："火焰法术场"——在原版 MassAttack 弹道基础上，为 HP<20% 阶段
//! 追加在玩家脚下投放 FireWall 法术场（持续燃烧），强化"火焰女王"主题。

use mir2_shared::enums::Spell;
use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

/// 视野范围
const VIEW_RANGE: i32 = 20;
/// 攻击范围（C# AttackRange = 3）
const ATTACK_RANGE: i32 = 3;
/// 近战判定
const MELEE_RANGE: i32 = 1;
/// MassAttack 周期：2-7s（C# 2000 + Random(5)*1000 ms）
const MASS_ATTACK_MIN_TICKS: u64 = 20;
/// 法术场投放半径
const FIELD_RADIUS: i32 = 7;

pub struct FlameQueenBehavior {
    /// 下次 MassAttack 的 tick（C# MassAttackTime）
    next_mass_tick: u64,
    spawned: bool,
}

impl FlameQueenBehavior {
    pub fn new() -> Self {
        Self { next_mass_tick: 0, spawned: false }
    }
}

impl MonsterBehavior for FlameQueenBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        if !self.spawned {
            self.spawned = true;
        }

        let hp_pct = if monster.max_hp > 0 {
            (monster.hp * 100) / monster.max_hp
        } else {
            0
        };

        // ---- HP<20% 阶段：周期 MassAttack（全体弹道）+ 火焰法术场 ----
        if hp_pct < 20 {
            if self.next_mass_tick == 0 || ctx.tick_count >= self.next_mass_tick {
                // C# MassAttackTime = Envir.Time + 2000 + Random(5)*1000
                self.next_mass_tick = ctx.tick_count + MASS_ATTACK_MIN_TICKS
                    + fastrand::u64(0..5) * 10;

                // 对范围内每个玩家投放 FireWall 法术场（任务核心机制）
                let targets: Vec<crate::actors::world::ai::PlayerSnap> =
                    ctx.find_targets_in_range(monster.x, monster.y, FIELD_RADIUS, monster.map_index)
                        .into_iter().copied().collect();
                for t in targets {
                    let value = crate::combat::attack::get_attack_power(monster.min_mac, monster.max_mac, 0).max(1);
                    ctx.out_spell_fields.push(crate::actors::world::ai::SpellFieldSpawn {
                        spell: Spell::FireWall,
                        x: t.x,
                        y: t.y,
                        value,
                        duration_ms: 5000,
                        tick_ms: 500,
                        caster_oid: monster.object_id,
                        caster_session: 0,
                    });
                }
            }
        }

        // 无目标则返回
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);

        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= ATTACK_RANGE && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let not_adjacent = dist > MELEE_RANGE;
            let ranged = not_adjacent || fastrand::i32(0..3) == 0;
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
            if ranged {
                // C# Type=0：RangeDamage 弹道
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    target_object_id: target.object_id,
                    damage,
                    spell_id: 0,
                });
            } else {
                // C# Type=1：近战 Damage
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage,
                    spell_id: 0,
                    attack_type: 1,
                });
            }
        } else if dist > ATTACK_RANGE && ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + monster.ai_profile.move_interval;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}

//! DarkDevourer（暗黑吞噬者）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/DarkDevourer.cs
//! 机制：
//!   - 全视野攻击（InAttackRange = ViewRange），可移动追击
//!   - 近战（贴身）：DC + DefenceType.AC
//!   - 远程：DC 弹道 + DefenceType.MACAgility；Effect==1 时命中附加绿毒（吞噬回血语义）
//!
//! Attack（C# :18-52）：!ranged→DC AC；ranged→DC MACAgility（+500ms 冷却）。
//! CompleteRangeAttack（C# :54-68）：Effect==1 且命中 → Green 1s 吞噬毒。

use crate::actors::world::MonsterState;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 15;
const MELEE_RANGE: i32 = 1;

pub struct DarkDevourerBehavior;

impl DarkDevourerBehavior {
    pub fn new() -> Self { Self }
}

impl MonsterBehavior for DarkDevourerBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if ctx.tick_count >= monster.next_attack_tick {
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
            if dist <= MELEE_RANGE {
                // 近战 DC AC
                monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage,
                    spell_id: 0,
                    attack_type: 0,
                });
            } else {
                // 远程 DC MACAgility（+500ms 冷却）
                monster.next_attack_tick = ctx.tick_count + 10;
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    target_object_id: target.object_id,
                    damage,
                    spell_id: 0,
                });
                // Effect==1：命中后吞噬绿毒 1s（C# PoisonTarget(1,5,Green,1000)）
                ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                    session_id: target.session_id,
                    poison: Poison::new(PoisonType::GREEN, 5, damage, 1000),
                });
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

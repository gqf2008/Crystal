//! CharmedSnake（魅惑蛇）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/CharmedSnake.cs
//! 机制：召唤物——主人>15 格或离线 → 自毁；近战（MAC）+ 麻痹毒（概率 10-PetLevel、时长 4+PetLevel，值=SC，tick 1000）；
//!      死亡 3x3 爆炸（10*PetLevel 伤害，MACAgility）

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;

const VIEW_RANGE: i32 = 12;
const MASTER_RANGE: i32 = 15;

pub struct CharmedSnakeBehavior;

impl CharmedSnakeBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for CharmedSnakeBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        // C# Process：主人>15 格或离线 → 自毁
        if let Some(master) = monster.master_session {
            let master_ok = ctx.players.iter()
                .find(|p| p.session_id == master && p.map_index == monster.map_index)
                .map(|p| max_distance(p.x, p.y, monster.x, monster.y) <= MASTER_RANGE)
                .unwrap_or(false);
            if !master_ok {
                monster.hp = 0;
                return;
            }
        }
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= 1 && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                attacker_oid: monster.object_id,
                target_session: target.session_id,
                damage,
                spell_id: 0,
                attack_type: 0,
            });
            // C# PoisonTarget(10-PetLevel, 4+PetLevel, Paralysis, 1000)：概率/时长随 PetLevel、值=SC
            let pet_level = ctx.pet_level.max(1);
            let chance = (10 - pet_level).max(1);
            let duration = (4 + pet_level) as u32;
            if fastrand::i32(0..chance) == 0 {
                let sc_value = crate::combat::attack::get_attack_power(monster.min_sc, monster.max_sc, 0).max(1);
                ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                    session_id: target.session_id,
                    poison: Poison::new(PoisonType::PARALYSIS, duration, sc_value, 1000),
                });
            }
            return;
        }

        if ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + monster.ai_profile.move_interval;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }

    /// C# Die：3x3 爆炸（10*PetLevel 伤害，MACAgility）
    fn on_die(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Aoe {
            attacker_oid: monster.object_id,
            center_x: monster.x,
            center_y: monster.y,
            radius: 1,
            damage: (10 * ctx.pet_level).max(1),
            spell_id: 0,
        });
    }
}

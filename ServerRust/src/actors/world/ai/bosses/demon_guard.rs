//! DemonGuard（恶魔守卫）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/DemonGuard.cs（继承 ZumaMonster）
//! 机制：2/3 物理近战（DC，ACAgility）/ 1/3 魔法近战（MC，ACAgility）；
//!      复活：LifeCount=random(0..3) 次，每次复活 HP=MaxHP*(100-25*次数)/100（C# Revive 4-20s 延迟，此处简化为即时复活）

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 12;

pub struct DemonGuardBehavior {
    revival_count: u32,
    life_count: u32,
}

impl DemonGuardBehavior {
    pub fn new() -> Self {
        // C#：LifeCount = Envir.Random.Next(3)（0-2 次复活）
        Self { revival_count: 0, life_count: fastrand::i32(0..3) as u32 }
    }
}

impl MonsterBehavior for DemonGuardBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        // C# ProcessAI：死亡且 RevivalTime 到且次数未满 → Revive（此处简化为即时复活）
        if monster.hp <= 0 && self.revival_count < self.life_count {
            self.revival_count += 1;
            let newhp = (monster.max_hp as f32 * (100 - 25 * self.revival_count as i32) as f32 / 100.0) as i32;
            monster.hp = newhp.max(1);
            return;
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
            // C# Random.Next(3) > 0：2/3 物理 / 1/3 魔法
            let magic = fastrand::i32(0..3) == 0;
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                attacker_oid: monster.object_id,
                target_session: target.session_id,
                damage,
                spell_id: 0,
                attack_type: if magic { 1 } else { 0 },
            });
            return;
        }

        if ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + monster.ai_profile.move_interval;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}

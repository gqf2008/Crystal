//! RevivingZombie（复活僵尸）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/RevivingZombie.cs
//! 机制：
//!   - 死亡后定时复活（最多 LifeCount 次，LifeCount=Random(3)）
//!   - 每次复活 HP 衰减（25% 递减），经验也递减
//!   - Die 记录 DieTime + RevivalTime(4~24s)，ProcessAI 检测复活
//!
//! ProcessAI（C# :30-41）：Dead && Time>DieTime+RevivalTime && RevivalCount<LifeCount → Revive。
//! Die（C# :23-28）：RevivalTime=(4+Random(20))*1000。

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 12;
const MELEE_RANGE: i32 = 1;

pub struct RevivingZombieBehavior {
    /// 可复活次数（C# LifeCount = Random(3)，0..=2）
    life_count: u8,
    /// 已复活次数
    revival_count: u8,
    /// 死亡时刻 tick
    die_tick: u64,
    /// 复活延迟 ticks
    revival_delay: u64,
    /// 是否已记录死亡
    dying: bool,
}

impl RevivingZombieBehavior {
    pub fn new() -> Self {
        Self {
            life_count: fastrand::u8(0..3), // Random(3): 0,1,2
            revival_count: 0,
            die_tick: 0,
            revival_delay: 0,
            dying: false,
        }
    }
}

impl MonsterBehavior for RevivingZombieBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        // 死亡 → 记录复活时间
        if monster.hp <= 0 && !self.dying {
            self.dying = true;
            self.die_tick = ctx.tick_count;
            // C# (4 + Random(20)) 秒
            self.revival_delay = (40 + fastrand::u64(0..200)) as u64;
            return;
        }

        // 死亡期：检测复活
        if self.dying {
            if self.revival_count < self.life_count
                && ctx.tick_count >= self.die_tick + self.revival_delay
            {
                self.revival_count += 1;
                self.dying = false;
                // C# newhp = HP * (100 - 25*RevivalCount) / 100
                let pct = 100 - 25 * self.revival_count as i32;
                monster.hp = (monster.max_hp as i32 * pct / 100).max(1);
            } else {
                return;
            }
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
                monster.next_attack_tick = ctx.tick_count + 5;
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

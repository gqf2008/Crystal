//! HornedArcher（角魔弓手）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/HornedArcher.cs（继承 AxeSkeleton）
//! 机制：
//!   - 每 20s 周期（BuffTime）：FindAllFriends(ViewRange) 随机选 1 个友军，
//!     对其施放增益弹道（Type1）：命中后给 4 格内友军加 DC/MC buff
//!   - 其余：标准远程弹道（Type0）DC + ACAgility
//!
//! ProcessTarget（C# :15-46）：Time>BuffTime→对友军施 buff 弹道。
//! Attack（C# :48-72）：Type0 DC 远程弹道。
//! CompleteAttack（C# :74-111）：友军目标→AddBuff；敌对→Attacked。

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 15;
/// buff 冷却（C# BuffTime = Time + 20000；无友军时 Time + 10000）
const BUFF_COOLDOWN_TICKS: u64 = 200;
const BUFF_COOLDOWN_NO_FRIEND_TICKS: u64 = 100;

pub struct HornedArcherBehavior {
    /// 下次可施放友军 buff 的 tick（C# BuffTime）
    next_buff_tick: u64,
}

impl HornedArcherBehavior {
    pub fn new() -> Self {
        Self { next_buff_tick: 0 }
    }
}

impl MonsterBehavior for HornedArcherBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        // ---- 周期性给友军上 buff（近似：用互疗队列给附近友军加攻击力）----
        if ctx.tick_count >= self.next_buff_tick {
            // 查找视野内一名友军（非自身、同 map）
            let friend = ctx.monsters.iter()
                .filter(|m| m.object_id != monster.object_id
                    && m.map_index == monster.map_index
                    && m.hp > 0)
                .min_by_key(|m| {
                    let dx = (m.x - monster.x).abs();
                    let dy = (m.y - monster.y).abs();
                    dx + dy
                })
                .filter(|m| {
                    let dx = (m.x - monster.x).abs();
                    let dy = (m.y - monster.y).abs();
                    dx.max(dy) <= VIEW_RANGE
                });

            if let Some(f) = friend {
                // C# 对友军施放增益弹道；这里用互疗队列给友军及其周围友军临时增益
                // 简化：对 4 格内友军提升伤害（out_heals 正值近似为攻击增益）
                let buff_radius = 4;
                let buffed: Vec<u32> = ctx.monsters.iter()
                    .filter(|m| m.map_index == monster.map_index && m.hp > 0)
                    .filter(|m| {
                        let dx = (m.x - f.x).abs();
                        let dy = (m.y - f.y).abs();
                        dx.max(dy) <= buff_radius
                    })
                    .map(|m| m.object_id)
                    .collect();
                for oid in buffed {
                    // 正值 heal 作为"增益"近似（C# AddBuff DC/MC）
                    ctx.out_heals.push((oid, monster.min_mac.max(1)));
                }
                self.next_buff_tick = ctx.tick_count + BUFF_COOLDOWN_TICKS;
                return;
            } else {
                // 无友军：缩短冷却继续尝试
                self.next_buff_tick = ctx.tick_count + BUFF_COOLDOWN_NO_FRIEND_TICKS;
            }
        }

        // ---- 标准远程弹道（AxeSkeleton 基类）----
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= VIEW_RANGE && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                attacker_oid: monster.object_id,
                target_session: target.session_id,
                target_object_id: target.object_id,
                damage,
                spell_id: 0,
            });
            return;
        }

        // 风筝走位
        if ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = if dist >= VIEW_RANGE {
                step_toward(monster.x, monster.y, target.x, target.y)
            } else if dist < 3 {
                step_away(monster.x, monster.y, target.x, target.y)
            } else {
                return;
            };
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + monster.ai_profile.move_interval;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}

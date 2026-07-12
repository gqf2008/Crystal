//! Khazard（卡扎德）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/Khazard.cs
//! 机制：
//!   - 十字/对角范围判定（4 格内 x==0||y==0||x==y）
//!   - 远程（>1 格）时 PullAttack：沿朝向方向把目标拉近到自身
//!     （Target.Pushed 朝向自身方向），冷却 5s
//!   - 近战（贴身）base.Attack（DC 单体）
//!
//! Attack（C# :44-66）：Range&&CanPull→PullAttack；!Range→base。
//! PullAttack（C# :68-81）：沿 Direction 1-4 格找目标，Pushed 朝自身。

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 15;
/// 最大拉扯距离（C# i=1..=4）
const PULL_MAX_DIST: i32 = 4;
/// 拉扯冷却（C# PullTime = Time + 5000）
const PULL_COOLDOWN_TICKS: u64 = 50;

pub struct KhazardBehavior {
    /// 下次可拉扯的 tick（C# PullTime）
    next_pull_tick: u64,
}

impl KhazardBehavior {
    pub fn new() -> Self {
        Self { next_pull_tick: 0 }
    }
}

impl MonsterBehavior for KhazardBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dx = (target.x - monster.x).abs();
        let dy = (target.y - monster.y).abs();
        let dist = dx.max(dy);

        // 十字/对角判定（C# InAttackRange）
        let in_line = dx == 0 || dy == 0 || dx == dy;
        if !in_line || dist > PULL_MAX_DIST {
            // 追击
            if ctx.tick_count >= monster.next_move_tick {
                let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
                ctx.out_moves.push((monster.object_id, nx, ny, dir));
                monster.next_move_tick = ctx.tick_count + 2;
                monster.ai_state = crate::actors::world::MonsterAiState::Chase;
            }
            return;
        }

        let ranged = dist > 1;

        if ranged {
            // 远程：拉扯（冷却到期）
            if ctx.tick_count >= self.next_pull_tick && ctx.tick_count >= monster.next_attack_tick {
                self.next_pull_tick = ctx.tick_count + PULL_COOLDOWN_TICKS;
                monster.next_attack_tick = ctx.tick_count + 6;
                // 把目标拉到自身邻格（近似 C# Pushed 朝自身方向 i 格）
                let (nx, ny, _dir) = step_toward(target.x, target.y, monster.x, monster.y);
                ctx.out_moves.push((target.object_id, nx, ny, 0));
            }
        } else {
            // 近战：DC 单体
            if ctx.tick_count >= monster.next_attack_tick {
                monster.next_attack_tick = ctx.tick_count + 6;
                let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage,
                    spell_id: 0,
                    attack_type: 0,
                });
            }
        }
    }
}

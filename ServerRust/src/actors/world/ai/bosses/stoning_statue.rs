//! StoningStatue（石化雕像）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/StoningStatue.cs
//! 机制：
//!   - 棋盘格近战范围（同 SpittingSpider：x==y||x%2==y%2，最远 2 格）
//!   - 两种攻击形态切换（_areaTime 周期，初值+10s）：
//!     · 普通期：LineAttack(2) DC 直线
//!     · AOE 期（_areaTime 到期）：1.6s 后 FindAllTargets(2) MC 全体 + Dazed 毒
//!   - AOE 后重置 _areaTime = 5 + Random(10) 秒
//!
//! Attack（C# :31-73）：Time<_areaTime→LineAttack；否则 AOE+重置。
//! CompleteAttack（C# :76-101）：area→FindAllTargets(2) MC+Dazed。

use crate::actors::world::MonsterState;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 12;

fn in_statue_range(dx_abs: i32, dy_abs: i32) -> bool {
    if dx_abs > 2 || dy_abs > 2 {
        return false;
    }
    (dx_abs <= 1 && dy_abs <= 1) || (dx_abs == dy_abs || dx_abs % 2 == dy_abs % 2)
}

pub struct StoningStatueBehavior {
    /// 下次 AOE 时刻（ticks）。C# _areaTime 初值 long.MaxValue → 首次 +10s
    area_tick: u64,
    spawned: bool,
}

impl StoningStatueBehavior {
    pub fn new() -> Self {
        Self { area_tick: u64::MAX, spawned: false }
    }
}

impl MonsterBehavior for StoningStatueBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        if !self.spawned {
            // C# _areaTime = Envir.Time + 10000
            self.area_tick = ctx.tick_count + 100;
            self.spawned = true;
        }

        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dx = (target.x - monster.x).abs();
        let dy = (target.y - monster.y).abs();
        if !in_statue_range(dx, dy) {
            return;
        }
        if ctx.tick_count < monster.next_attack_tick {
            return;
        }

        if ctx.tick_count < self.area_tick {
            // 普通期：LineAttack DC
            monster.next_attack_tick = ctx.tick_count + 8;
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                attacker_oid: monster.object_id,
                target_session: target.session_id,
                damage,
                spell_id: 0,
                attack_type: 0,
            });
        } else {
            // AOE 期：FindAllTargets(2) MC + Dazed（C# 5+Random(10) 秒后重置）
            self.area_tick = ctx.tick_count + 50 + fastrand::u64(0..100);
            monster.next_attack_tick = ctx.tick_count + 16; // AttackSpeed*2
            let hits: Vec<crate::actors::world::ai::PlayerSnap> =
                ctx.find_targets_in_range(monster.x, monster.y, 2, monster.map_index)
                    .into_iter().copied().collect();
            for h in hits {
                let dmg = monster.max_mac.max(1);
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: h.session_id,
                    damage: dmg,
                    spell_id: 0,
                    attack_type: 1,
                });
                // C# PoisonTarget(2, Random(5,10), Dazed, 1000)
                let dur = fastrand::i32(5..10) as u32;
                ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                    session_id: h.session_id,
                    poison: Poison::new(PoisonType::DAZED, dur, 0, 1000),
                });
            }
        }
    }
}

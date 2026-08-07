//! BombSpider（炸弹蜘蛛）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/BombSpider.cs
//! 机制：
//!   - 自爆怪：贴近目标/超时/无目标 → Die → 范围 1 格 AOE + 绿毒
//!   - ExplosionTime = 出生后 5 分钟强制爆炸
//!   - 有目标时追击（MoveTo），贴身即爆
//!
//! ProcessTarget（C# :16-29）：Target==null||InAttackRange||超时 → Die。
//! CompleteDeath（C# :37-57）：FindAllTargets(1) 逐个 Attacked + 1/5 绿毒。

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;

const VIEW_RANGE: i32 = 15;
const MELEE_RANGE: i32 = 1;
/// 自爆倒计时（C# 5 分钟 = 3000 ticks）
const EXPLOSION_TICKS: u64 = 3000;

pub struct BombSpiderBehavior {
    spawned: bool,
    explosion_tick: u64,
}

impl BombSpiderBehavior {
    pub fn new() -> Self {
        Self { spawned: false, explosion_tick: 0 }
    }
}

impl MonsterBehavior for BombSpiderBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        if !self.spawned {
            self.explosion_tick = ctx.tick_count + EXPLOSION_TICKS;
            self.spawned = true;
        }

        let target = ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index);

        // 无目标或超时 → 自爆
        if target.is_none() || ctx.tick_count >= self.explosion_tick {
            self.explode(monster, ctx);
            return;
        }

        let target = *target.unwrap();
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        // 贴身 → 自爆
        if dist <= MELEE_RANGE {
            self.explode(monster, ctx);
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

impl BombSpiderBehavior {
    /// 自爆：1 格 AOE + 绿毒，然后自身死亡
    fn explode(&self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
        let hits: Vec<crate::actors::world::ai::PlayerSnap> =
            ctx.find_targets_in_range(monster.x, monster.y, 1, monster.map_index)
                .into_iter().copied().collect();
        for h in hits {
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                attacker_oid: monster.object_id,
                target_session: h.session_id,
                damage,
                spell_id: 0,
                attack_type: 0,
            });
            // 1/5 绿毒（C# Random(5)==0）
            if fastrand::i32(0..5) == 0 {
                ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                    session_id: h.session_id,
                    // C# 毒值 = SP 攻（DC 近似）
                    poison: Poison::new(PoisonType::GREEN, 5, damage, 2000),
                });
            }
        }
        monster.hp = 0;
    }
}

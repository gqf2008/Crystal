//! HarvestMonster（可采集怪物）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/HarvestMonster.cs
//! 机制：
//!   - 死亡后可被采集（Harvest）：RemainingSkinCount=2 次采集后掉落 Meat 类物品
//!   - Drop() 为空（不主动掉落，物品通过 Harvest 发放）
//!   - 采集品质 Quality 随魔法伤害降低
//!
//! 说明：Harvest 由玩家交互（PlayerObject.Harvest → monster.Harvest）触发，不在
//! AI tick 内处理。AI 层 HarvestMonster 继承 MonsterObject 默认行为（追击/反击），
//! 无独特攻击机制。此 behavior 仅为注册占位 + 记录可采集特性，AI 走普通近战追击。

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 8;
const MELEE_RANGE: i32 = 1;

pub struct HarvestMonsterBehavior;

impl HarvestMonsterBehavior {
    pub fn new() -> Self { Self }
}

impl MonsterBehavior for HarvestMonsterBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= MELEE_RANGE {
            if ctx.tick_count >= monster.next_attack_tick {
                monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
                let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage,
                    spell_id: 0,
                    attack_type: 0,
                });
            }
            return;
        }

        if ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + 2;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}

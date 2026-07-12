//! DarkDevil（暗黑恶魔）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/DarkDevil.cs
//! 机制：两种攻击模式（由 _areaTime 定时器切换）
//!   - 普通模式（_areaTime 内）：base.Attack 近战
//!   - AOE 模式（_areaTime 到期）：朝向 2 格前方 1 格范围 MACAgility AOE，伤害*3
//!   - 触发 AOE 后 _areaTime = Envir.Time + 2000 + Random(3)*1000（2~4s 冷却）
//!   - InAttackRange：AOE 模式射程 3，普通模式射程 1
//!
//! Attack（C# :23-46）：_areaTime 到期→ObjectRangeAttack + DelayedAction RangeDamage。
//! CompleteRangeAttack（C# :48-60）：DC*3，FindAllTargets(1, PointMove(self, dir, 2))。

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 15;
/// AOE 模式射程（C# Envir.Time > _areaTime ? 3 : 1）
const AREA_RANGE: i32 = 3;
/// AOE 命中半径（C# FindAllTargets(1)）
const AREA_RADIUS: i32 = 1;

pub struct DarkDevilBehavior {
    /// 下次可释放 AOE 的 tick（C# _areaTime）
    area_tick: u64,
}

impl DarkDevilBehavior {
    pub fn new() -> Self {
        Self { area_tick: 0 }
    }
}

impl MonsterBehavior for DarkDevilBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if ctx.tick_count < monster.next_attack_tick {
            return;
        }

        let in_area_mode = ctx.tick_count >= self.area_tick;
        let effective_range = if in_area_mode { AREA_RANGE } else { 1 };

        if dist > effective_range {
            // 追击
            if ctx.tick_count >= monster.next_move_tick {
                let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
                ctx.out_moves.push((monster.object_id, nx, ny, dir));
                monster.next_move_tick = ctx.tick_count + 2;
                monster.ai_state = crate::actors::world::MonsterAiState::Chase;
            }
            return;
        }

        monster.next_attack_tick = ctx.tick_count + 8;

        if in_area_mode {
            // AOE 模式：DC*3，朝向 2 格前方 1 格范围（C# PointMove(self,dir,2)）
            self.area_tick = ctx.tick_count + 20 + fastrand::i32(0..3) as u64 * 10; // 2~4s
            let dmg = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1) * 3;
            // 命中中心 = 朝目标方向走 2 格
            let dir = direction_towards(monster.x, monster.y, target.x, target.y);
            let cx = monster.x + crate::actors::world::ai::helpers::DIR_DX[dir as usize] * 2;
            let cy = monster.y + crate::actors::world::ai::helpers::DIR_DY[dir as usize] * 2;
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Aoe {
                attacker_oid: monster.object_id,
                center_x: cx,
                center_y: cy,
                radius: AREA_RADIUS,
                damage: dmg,
                spell_id: 0,
            });
        } else {
            // 普通近战（C# base.Attack）
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

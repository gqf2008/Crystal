//! HardenRhino（硬皮犀牛）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/HardenRhino.cs
//! 机制：近战（dist<=1）DC；目标>3 格且 1/3 → Dash 冲刺（无伤害，2 格步进多步移动），冷却 1500ms

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 12;
const DASH_RANGE: i32 = 3;
const DASH_COOLDOWN: u64 = 15; // 1500ms

pub struct HardenRhinoBehavior {
    next_dash_tick: u64,
}

impl HardenRhinoBehavior {
    pub fn new() -> Self {
        Self { next_dash_tick: 0 }
    }
}

impl MonsterBehavior for HardenRhinoBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        // C# ProcessSearch：目标>3 格且 1/3 → Dash 冲刺（无伤害）
        if dist > DASH_RANGE && ctx.tick_count >= self.next_dash_tick && fastrand::i32(0..3) == 0 {
            self.next_dash_tick = ctx.tick_count + DASH_COOLDOWN;
            // C# Dash（HardenRhino.cs:83-115）：沿目标方向 2 格步进，直到进入 2 格范围或遇阻挡；
            // 用多步 out_moves（每步 2 格，ctx.is_walkable 预校验、遇阻截断），tick 端逐条应用并广播 ObjectWalk（近似 ObjectRun）
            let mut cx = monster.x;
            let mut cy = monster.y;
            let mut cd = max_distance(cx, cy, target.x, target.y);
            while cd > 2 {
                let dir = (direction_towards(cx, cy, target.x, target.y) as usize) % 8;
                let nx = cx + DIR_DX[dir] * 2;
                let ny = cy + DIR_DY[dir] * 2;
                if !(ctx.is_walkable)(nx, ny) { break; }
                ctx.out_moves.push((monster.object_id, nx, ny, dir as u8));
                cx = nx;
                cy = ny;
                cd = max_distance(cx, cy, target.x, target.y);
            }
            return;
        }

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

//! CreeperPlant（爬行植物）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/CreeperPlant.cs（继承 CannibalPlant）
//! 机制：AttackRange=5；近战（dist<=1）DC / 远程 MC（MACAgility）
//! #1360：隐藏/现身（C# ProcessAI）：每 2s 检查 4 格内玩家——有玩家且隐藏 → 现身（ObjectShow）；
//!       可见且无玩家 → 隐藏（ObjectHide）+ 满血回复（SetHP(MaxHP)）；隐藏期间不攻击不移动。

use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::actors::world::MonsterState;

const ATTACK_RANGE: i32 = 5;
/// C# FindNearby(4)：4 格内玩家判定
const HIDE_CHECK_RANGE: i32 = 4;
/// C# VisibleTime 每 2s 检查一次（100ms/tick → 20 tick）
const HIDE_CHECK_INTERVAL: u64 = 20;

pub struct CreeperPlantBehavior {
    /// 下次隐身检查 tick（#1360）
    next_check_tick: u64,
}

impl CreeperPlantBehavior {
    pub fn new() -> Self {
        Self { next_check_tick: 0 }
    }
}

impl MonsterBehavior for CreeperPlantBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        // #1360：C# ProcessAI——每 2s 检查 4 格内是否有存活玩家
        if ctx.tick_count >= self.next_check_tick {
            self.next_check_tick = ctx.tick_count + HIDE_CHECK_INTERVAL;
            let players_nearby = !ctx
                .find_targets_in_range(monster.x, monster.y, HIDE_CHECK_RANGE, monster.map_index)
                .is_empty();
            if monster.hidden && players_nearby {
                // 有玩家且隐藏 → 现身（C# Visible=true + Broadcast(GetInfo) + ObjectShow）
                monster.hidden = false;
                ctx.out_show_hide.push((monster.object_id, true));
            } else if !monster.hidden && !players_nearby {
                // 可见且无玩家 → 隐藏 + 满血回复（C# Visible=false + ObjectHide + SetHP(MaxHP)）
                monster.hidden = true;
                ctx.out_show_hide.push((monster.object_id, false));
                monster.hp = monster.max_hp;
            }
        }
        // 隐藏期间不攻击不移动（C# 隐身植物无目标）
        if monster.hidden {
            return;
        }
        let target = match ctx.nearest_target(monster.x, monster.y, ATTACK_RANGE, monster.map_index)
        {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= ATTACK_RANGE && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let damage = crate::combat::attack::get_attack_power(
                monster.min_dmg,
                monster.max_dmg,
                monster.luck,
            )
            .max(1);
            if dist <= 1 {
                ctx.out_attacks
                    .push(crate::actors::world::ai::AttackAction::Melee {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        damage,
                        spell_id: 0,
                        attack_type: 0,
                    });
            } else {
                ctx.out_attacks
                    .push(crate::actors::world::ai::AttackAction::Range {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        target_object_id: target.object_id,
                        damage,
                        spell_id: 0,
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
}

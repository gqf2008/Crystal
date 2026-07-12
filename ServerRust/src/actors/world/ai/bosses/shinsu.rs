//! Shinsu（神兽）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/Shinsu.cs
//! 机制：道士召唤神兽（SummonShinsu），双形态切换
//!   - Mode 切换：有目标时 ModeTime = Envir.Time + 30000（30s 攻击形态）；
//!     超过 ModeTime 自动退出 Mode（隐藏休眠）；CanAttack 仅在 Mode=true 时生效
//!   - 攻击形态：DC LineAttack(2)（前方 2 格直线），InAttackRange 为特殊十字判定
//!   - 攻击形态方向：Mode?Shinsu1:Shinsu（双形态贴图）
//!
//! ProcessAI（C# :26-47）：Target!=null → ModeTime=+30s；进/出 Mode 广播 Show/Hide。
//! Attack（C# :63-82）：DC LineAttack(2)。

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 15;
/// Mode 持续（C# ModeTime = Envir.Time + 30000，30s）
const MODE_TICKS: u64 = 300;
/// LineAttack 距离（C# LineAttack(damage, 2)）
const LINE_RANGE: i32 = 2;

pub struct ShinsuBehavior {
    /// 当前是否处于攻击形态（C# Mode）
    mode: bool,
    /// Mode 到期 tick（C# ModeTime）
    mode_end_tick: u64,
    spawned: bool,
}

impl ShinsuBehavior {
    pub fn new() -> Self {
        Self { mode: false, mode_end_tick: 0, spawned: false }
    }
}

impl MonsterBehavior for ShinsuBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        if !self.spawned {
            self.spawned = true;
        }

        let target = ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index);

        // Mode 切换（C# ProcessAI）
        if let Some(t) = target {
            // 有目标：刷新 ModeTime 并进入攻击形态
            self.mode_end_tick = ctx.tick_count + MODE_TICKS;
            if !self.mode {
                self.mode = true; // C# Mode=true + ObjectShow
            }
            monster.target_session = Some(t.session_id);
        } else if ctx.tick_count >= self.mode_end_tick && self.mode {
            // 超时退出攻击形态（C# Mode=false + ObjectHide）
            self.mode = false;
            monster.target_session = None;
            return;
        }

        // 非攻击形态不攻击（C# CanAttack = base && Mode）
        if !self.mode {
            return;
        }

        let target = match target {
            Some(t) => *t,
            None => return,
        };
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        // 攻击形态：DC LineAttack(2)
        if dist <= LINE_RANGE {
            if ctx.tick_count >= monster.next_attack_tick {
                monster.next_attack_tick = ctx.tick_count + 6;
                let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
                // 直线 2 格：对朝目标方向前 2 格内的玩家施放（近似 LineAttack）
                let dir = direction_towards(monster.x, monster.y, target.x, target.y);
                let dx = DIR_DX[dir as usize];
                let dy = DIR_DY[dir as usize];
                // 收集直线格上的目标
                let hits: Vec<crate::actors::world::ai::PlayerSnap> = ctx
                    .find_targets_in_range(monster.x, monster.y, LINE_RANGE, monster.map_index)
                    .into_iter().copied()
                    .filter(|p| {
                        // 仅命中朝目标方向的直线格
                        let rx = p.x - monster.x;
                        let ry = p.y - monster.y;
                        (rx == 0 && dy == 0) || (ry == 0 && dx == 0) || (rx.signum() == dx.signum() && ry.signum() == dy.signum() && rx.abs() == ry.abs())
                    })
                    .collect();
                if hits.is_empty() {
                    // 退化为对主目标单体
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        damage,
                        spell_id: 0,
                        attack_type: 0,
                    });
                } else {
                    for h in hits {
                        ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                            attacker_oid: monster.object_id,
                            target_session: h.session_id,
                            damage,
                            spell_id: 0,
                            attack_type: 0,
                        });
                    }
                }
            }
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

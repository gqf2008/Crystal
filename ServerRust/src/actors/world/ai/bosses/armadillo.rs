//! Armadillo（犰狳）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/Armadillo.cs（继承 DigOutZombie）
//! 机制：
//!   - 钻地伏击（继承 DigOutZombie：靠近 3 格钻出）
//!   - 攻击随机 1/6 概率：0=Retreat（后跳 2 格 + 反向射程 AC 伤害）；1=三连击（半伤×3）；其余=普通近战
//!   - Retreat 命中后若目标未受伤则 _runAway=true（逃跑），被攻击 1/4 概率解除逃跑
//!
//! Attack（C# :58-113）：switch(Random(0,6))。
//! Retreat（C# :115-145）：ReverseDirection 退 2 格 + 延迟 RangeDamage。
//! ProcessTarget（C# :166-199）：_runAway 时朝远离方向 Walk。

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const APPEAR_RANGE: i32 = 3;
const VIEW_RANGE: i32 = 14;
const MELEE_RANGE: i32 = 1;
const CHECK_TICKS: u64 = 20;

pub struct ArmadilloBehavior {
    visible: bool,
    next_check_tick: u64,
    spawned: bool,
    /// 钻出时刻（tick；1s 后生成洞口，继承 C# DigOutZombie）
    dig_out_tick: u64,
    /// 洞口是否已生成（继承 C# DoneDigOut）
    hole_done: bool,
    /// 逃跑模式（C# _runAway）
    run_away: bool,
}

impl ArmadilloBehavior {
    pub fn new() -> Self {
        Self { visible: false, next_check_tick: 0, spawned: false, dig_out_tick: 0, hole_done: false, run_away: false }
    }
}

impl MonsterBehavior for ArmadilloBehavior {
    fn can_move(&self) -> bool { self.visible }
    fn is_attackable(&self) -> bool { self.visible }

    fn on_attacked(&mut self, damage: i32) -> i32 {
        // C# 被攻击 1/4 概率解除逃跑
        if self.run_away && fastrand::i32(0..4) == 0 {
            self.run_away = false;
        }
        if self.visible { damage } else { 0 }
    }

    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        if !self.spawned {
            self.next_check_tick = ctx.tick_count + CHECK_TICKS;
            self.spawned = true;
        }

        // 钻出检测
        if ctx.tick_count >= self.next_check_tick {
            self.next_check_tick = ctx.tick_count + CHECK_TICKS;
            if !self.visible {
                let has_near = ctx.nearest_target(monster.x, monster.y, APPEAR_RANGE, monster.map_index).is_some();
                if has_near {
                    self.visible = true;
                    self.dig_out_tick = ctx.tick_count;
                    self.hole_done = false;
                }
            }
        }
        if !self.visible {
            return;
        }

        // C# DigOutZombie.SpawnDigOutEffect（继承）：钻出 1s 后生成洞口 SpellObject（5 分钟）
        if !self.hole_done && ctx.tick_count >= self.dig_out_tick + 10 {
            self.hole_done = true;
            ctx.out_spell_fields.push(crate::actors::world::ai::SpellFieldSpawn {
                spell: mir2_shared::enums::Spell::DigOutArmadillo,
                x: monster.x,
                y: monster.y,
                value: 1,
                duration_ms: 300_000,
                tick_ms: 2000,
                caster_oid: monster.object_id,
                caster_session: 0,
            });
        }

        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        // 逃跑模式：朝远离方向移动
        if self.run_away {
            if ctx.tick_count >= monster.next_move_tick {
                let (nx, ny, dir) = step_away(monster.x, monster.y, target.x, target.y);
                ctx.out_moves.push((monster.object_id, nx, ny, dir));
                monster.next_move_tick = ctx.tick_count + 2;
            }
            return;
        }

        if dist <= MELEE_RANGE && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + 8;
            let roll = fastrand::i32(0..6);
            let dmg_full = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
            match roll {
                0 => {
                    // Retreat：后跳 + 反向射程伤害
                    if ctx.tick_count >= monster.next_move_tick {
                        let (nx, ny, dir) = step_away(monster.x, monster.y, target.x, target.y);
                        ctx.out_moves.push((monster.object_id, nx, ny, dir));
                        monster.next_move_tick = ctx.tick_count + 2;
                    }
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        target_object_id: target.object_id,
                        damage: dmg_full,
                        spell_id: 0,
                    });
                }
                1 => {
                    // 三连击（半伤×3）
                    let half = (dmg_full / 2).max(1);
                    for _ in 0..3 {
                        ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                            attacker_oid: monster.object_id,
                            target_session: target.session_id,
                            damage: half,
                            spell_id: 0,
                            attack_type: 1,
                        });
                    }
                }
                _ => {
                    // 普通近战
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        damage: dmg_full,
                        spell_id: 0,
                        attack_type: 0,
                    });
                }
            }
        } else if dist > MELEE_RANGE && ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + 2;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}

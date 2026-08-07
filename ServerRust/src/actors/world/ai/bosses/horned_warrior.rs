//! HornedWarrior（角魔战士）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/HornedWarrior.cs
//! 机制：
//!   - HP<50% 周期性进入盾牌态（Type=2 广播 + 10s 内 +500 AC）：
//!     盾牌期不攻击，背向目标逃跑
//!   - 盾牌期外近战：2/3 概率 Type0 DC 单体；1/3 概率 Type1 WideLineAttack(4格)
//!   - AttackRange=4 的对角/同奇偶网格判定
//!
//! Attack（C# :29-84）：Shield buff→return；2/3 DC；else WideLine。
//! ProcessTarget（C# :86-134）：!buff→MoveTo；buff→背向 Walk 逃跑。

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 15;
const ATTACK_RANGE: i32 = 4;
/// 盾牌态 AC 加成（C# [Stat.MaxAC]=500, [Stat.MinAC]=500）
const SHIELD_AC: i32 = 500;
/// 盾牌持续 ticks（C# Settings.Second * 10 = 10s）
const SHIELD_DURATION_TICKS: u64 = 100;
/// 盾牌冷却：15-20s（C# 15000 + Random(5000) ms）
const SHIELD_COOLDOWN_MIN_TICKS: u64 = 150;

pub struct HornedWarriorBehavior {
    /// 盾牌态结束 tick（>tick_count 即在盾牌期内）
    shield_until: u64,
    /// 下次可进入盾牌态的 tick（C# _ShieldTime）
    next_shield_available: u64,
}

impl HornedWarriorBehavior {
    pub fn new() -> Self {
        Self { shield_until: 0, next_shield_available: 0 }
    }

    fn in_shield(&self, tick: u64) -> bool {
        tick < self.shield_until
    }
}

impl MonsterBehavior for HornedWarriorBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        let hp_pct = if monster.max_hp > 0 { monster.hp * 100 / monster.max_hp } else { 100 };
        let shielded = self.in_shield(ctx.tick_count);

        // ---- 进入盾牌态（HP<50% 且冷却结束）----
        if !shielded && hp_pct < 50 && ctx.tick_count >= self.next_shield_available {
            self.shield_until = ctx.tick_count + SHIELD_DURATION_TICKS;
            self.next_shield_available = ctx.tick_count + SHIELD_COOLDOWN_MIN_TICKS
                + fastrand::u64(0..50);
            // 盾牌期临时加 AC（C# AddBuff MaxAC/MinAC=500）
            monster.max_ac += SHIELD_AC;
            monster.min_ac += SHIELD_AC;
            return;
        }

        // ---- 盾牌期结束：移除 AC 加成 ----
        if !shielded && monster.max_ac >= SHIELD_AC {
            // 仅在刚结束盾牌时回退一次（用 shield_until>0 标记曾经加过）
            if self.shield_until > 0 && ctx.tick_count >= self.shield_until {
                monster.max_ac -= SHIELD_AC;
                monster.min_ac -= SHIELD_AC;
                self.shield_until = 0;
            }
        }

        // ---- 盾牌期：背向逃跑 ----
        if shielded {
            if ctx.tick_count >= monster.next_move_tick {
                let (nx, ny, dir) = step_away(monster.x, monster.y, target.x, target.y);
                ctx.out_moves.push((monster.object_id, nx, ny, dir));
                monster.next_move_tick = ctx.tick_count + 2;
            }
            return;
        }

        // ---- 正常战斗 ----
        if dist <= ATTACK_RANGE && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + 6;
            // C# 2/3 DC 单体；1/3 WideLineAttack(4)
            if fastrand::i32(0..3) > 0 {
                // Type0 DC 单体
                let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage,
                    spell_id: 0,
                    attack_type: 0,
                });
            } else {
                // Type1 WideLine(4)：C# WideLineAttack(damage, 4, width=3) —— 3 条平行车道 × 4 格
                let dir = direction_towards(monster.x, monster.y, target.x, target.y) as usize % 8;
                let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
                // 起始 3 点：自身 + 左车道 + 右车道（C# Functions.Left/Right）
                let left = (dir + 7) % 8;
                let right = (dir + 1) % 8;
                let lanes = [
                    (monster.x, monster.y),
                    (monster.x + DIR_DX[left], monster.y + DIR_DY[left]),
                    (monster.x + DIR_DX[right], monster.y + DIR_DY[right]),
                ];
                for (lx, ly) in lanes {
                    for i in 1..=4i32 {
                        let tx = lx + DIR_DX[dir] * i;
                        let ty = ly + DIR_DY[dir] * i;
                        if let Some(p) = ctx.players.iter()
                            .find(|p| p.map_index == monster.map_index && p.x == tx && p.y == ty && p.hp > 0)
                        {
                            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                                attacker_oid: monster.object_id,
                                target_session: p.session_id,
                                damage,
                                spell_id: 0,
                                attack_type: 1,
                            });
                        }
                    }
                }
            }
            return;
        }

        // 追击
        if !shielded && ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + 2;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}

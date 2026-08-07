//! HumanAssassin（人形刺客 NPC）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/HumanAssassin.cs
//! 机制：可被召唤的人形刺客（自爆型）
//!   - AttackRange=1，双倍移动（Walk 走 2 格）
//!   - Attack 累计 AttackDamage，达 500 即 Die 触发自爆
//!   - Die：ExplosionDie 16 方向范围 AC 爆炸（暴击 DC*2）
//!   - 召唤时限 ExplosionTime=10s 到期强制 Die（仅宠物）
//!
//! Attack（C# :228-259）：AttackDamage>=500→Die；DC ACAgility 累加。
//! Die/ExplosionDie（C# :268-329）：16 方向 AC 爆炸。
//! ProcessAI（C# :138-149）：Master 且 Envir.Time>ExplosionTime → Die。

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 15;
const MELEE_RANGE: i32 = 1;
/// 自爆触发阈值（C# AttackDamage >= 500）
const EXPLOSION_THRESHOLD: i32 = 500;
/// 召唤时限（C# ExplosionTime = +10000，仅宠物）
const EXPLOSION_TIME_TICKS: u64 = 100;

pub struct HumanAssassinBehavior {
    /// 累计伤害（C# AttackDamage）
    attack_damage: i32,
    /// 自爆到期 tick（C# ExplosionTime，仅宠物）
    explosion_tick: u64,
    spawned: bool,
}

impl HumanAssassinBehavior {
    pub fn new() -> Self {
        Self { attack_damage: 0, explosion_tick: 0, spawned: false }
    }
}

impl MonsterBehavior for HumanAssassinBehavior {
    fn on_spawned(&mut self, _monster: &mut MonsterState) {
        self.spawned = true;
        // 注：ExplosionTime 在首次 process_tick 时按宠物身份 + 召唤时限初始化
    }

    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        // 首次 tick 初始化自爆时限（C# 构造 ExplosionTime = Envir.Time + 10000，仅宠物）
        if self.spawned && self.explosion_tick == 0 {
            self.explosion_tick = if monster.master_session.is_some() {
                ctx.tick_count + EXPLOSION_TIME_TICKS
            } else {
                u64::MAX // 非宠物永不到期
            };
        }
        // 宠物到期自爆（C# ProcessAI Master 且 > ExplosionTime）
        if ctx.tick_count >= self.explosion_tick {
            self.explode(monster, ctx);
            return;
        }

        // 累计伤害达阈值自爆（C# Attack AttackDamage>=500）
        if self.attack_damage >= EXPLOSION_THRESHOLD {
            self.explode(monster, ctx);
            return;
        }

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
                self.attack_damage += damage; // C# AttackDamage += damage
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

        // 双倍移动追击（C# Walk 走 2 格）：连续两步近似
        if ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + monster.ai_profile.move_interval;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }

    /// C# Die：ExplosionDie 16 方向 AC 爆炸
    fn on_die(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        self.explode(monster, ctx);
    }
}

impl HumanAssassinBehavior {
    /// 16 方向范围爆炸（C# ExplosionDie：i<16，PointMove(dir, i/8+1)）
    fn explode(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let crit = fastrand::i32(0..100) <= monster.accuracy;
        let base = if crit { monster.max_dmg * 2 } else { monster.min_dmg * 2 };
        let damage = (monster.min_dmg / 5 + 4 * 1) * base / 20 + monster.max_dmg;
        // 16 方向（2 圈）→ 以自身为中心半径 2 AOE 近似
        ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Aoe {
            attacker_oid: monster.object_id,
            center_x: monster.x,
            center_y: monster.y,
            radius: 2,
            damage: damage.max(1),
            spell_id: 0,
        });
        monster.hp = 0;
        if self.explosion_tick == 0 {
            self.explosion_tick = u64::MAX; // 防止重复触发
        }
    }
}

//! EarthGolem（地魔像）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/EarthGolem.cs（继承 ZumaMonster）
//! 机制：
//!   - 继承 ZumaMonster 石化休眠：FindNearby(4) 才唤醒（Stoned→Wake）
//!   - AttackRange=6；近战（<=1，2/3 概率）DC MAC；
//!     远程 MC：在目标点生成 3x3 地面冲击法术场（EarthGolemPile，1.2s + 0.5s 延迟）
//!   - 风筝走位（FearTime 2s）
//!
//! Attack（C# :49-119）：!ranged&&Random(3)>0→DC MAC；else→目标点 3x3 EarthGolemPile 法术场。
//! ProcessAI（C# :30-47）：FindNearby(4) 唤醒。

use crate::actors::world::MonsterState;
use mir2_shared::enums::Spell;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const ATTACK_RANGE: i32 = 6;
const VIEW_RANGE: i32 = 15;
const MELEE_RANGE: i32 = 1;
/// 唤醒检测范围（C# FindNearby(4)）
const WAKE_RANGE: i32 = 4;
/// FearTime 持续（C# Envir.Time + 2000）
const FEAR_TICKS: u64 = 20;

pub struct EarthGolemBehavior {
    /// 是否石化休眠（继承 ZumaMonster Stoned）
    stoned: bool,
    fear_end_tick: u64,
}

impl EarthGolemBehavior {
    pub fn new() -> Self {
        Self { stoned: true, fear_end_tick: 0 }
    }
}

impl MonsterBehavior for EarthGolemBehavior {
    /// 石化期不可被攻击（继承 ZumaMonster IsAttackTarget = !Stoned）
    fn is_attackable(&self) -> bool { !self.stoned }

    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        // 石化唤醒检测（C# ProcessAI FindNearby(4)）
        if self.stoned {
            if ctx.nearest_target(monster.x, monster.y, WAKE_RANGE, monster.map_index).is_some() {
                self.stoned = false; // C# Wake()
            } else {
                return; // 休眠中
            }
        }

        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= ATTACK_RANGE && ctx.tick_count < self.fear_end_tick
            && ctx.tick_count >= monster.next_attack_tick
        {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;

            if dist <= MELEE_RANGE && fastrand::i32(0..3) > 0 {
                // 近战 DC MAC（C# 2/3 概率）
                let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage,
                    spell_id: 0,
                    attack_type: 0,
                });
            } else {
                // 远程：目标点 3x3 地面冲击法术场（C# EarthGolemPile）
                let damage = crate::combat::attack::get_attack_power(monster.min_mac, monster.max_mac, 0).max(1);
                ctx.out_spell_fields.push(crate::actors::world::ai::SpellFieldSpawn {
                    spell: Spell::EarthGolemPile,
                    x: target.x,
                    y: target.y,
                    value: damage,
                    duration_ms: 1200,
                    tick_ms: 1000,
                    caster_oid: monster.object_id,
                    caster_session: 0,
                });
            }
            return;
        }

        // 刷新 FearTime
        self.fear_end_tick = ctx.tick_count + FEAR_TICKS;

        // 走位：过近拉开，远了追近
        if ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = if dist < ATTACK_RANGE {
                step_away(monster.x, monster.y, target.x, target.y)
            } else {
                step_toward(monster.x, monster.y, target.x, target.y)
            };
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + 2;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}

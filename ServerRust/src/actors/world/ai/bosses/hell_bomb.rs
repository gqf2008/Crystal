//! HellBomb（地狱炸弹）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/HellBomb.cs
//! 机制：CanMove=false、不可攻击、不回血；10s 后自动 Die →
//!       CompleteDeath：FindAllTargets(4) AOE + 按 image 毒（1=冰冻/2=眩晕/3=出血，5s，tick 2000）

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;

const EXPLODE_RADIUS: i32 = 4;
const FUSE_TICKS: u64 = 100; // 10s

pub struct HellBombBehavior {
    die_at_tick: u64,
}

impl HellBombBehavior {
    pub fn new() -> Self {
        Self { die_at_tick: 0 }
    }
}

impl MonsterBehavior for HellBombBehavior {
    // C#：只覆写 Struck（SpellObject 法术场伤害）返回 0；Attacked（普通攻击）仍可造成伤害，
    // 玩家可提前击破。Rust 引擎不区分 Attacked/Struck，此处保持可被普通攻击击杀（on_attacked 默认）。
    fn on_poison(&mut self, _poison: crate::combat::poison::Poison) -> bool {
        false // C# ApplyPoison 空实现：免疫毒
    }

    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        // C# ProcessTarget：Envir.Time > ExplosionTime → Die（10s 引信）
        if self.die_at_tick == 0 {
            self.die_at_tick = ctx.tick_count + FUSE_TICKS;
        }
        if ctx.tick_count >= self.die_at_tick {
            monster.hp = 0;
        }
    }

    /// C# CompleteDeath：AOE 4 + 按 image 毒
    fn on_die(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
        ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Aoe {
            attacker_oid: monster.object_id,
            center_x: monster.x,
            center_y: monster.y,
            radius: EXPLODE_RADIUS,
            damage,
            spell_id: 0,
        });
        // C# switch(Info.Image)：按名字近似（HellBomb1/2/3）
        let ptype = if monster.name.to_lowercase().contains("hellbomb1") {
            PoisonType::FROZEN
        } else if monster.name.to_lowercase().contains("hellbomb2") {
            PoisonType::DAZED
        } else {
            PoisonType::BLEEDING
        };
        let nearby: Vec<u64> = ctx.find_targets_in_range(monster.x, monster.y, EXPLODE_RADIUS, monster.map_index)
            .iter().map(|p| p.session_id).collect();
        for sid in nearby {
            // PoisonTarget(1, 5, type, 2000)：100%
            ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                session_id: sid,
                poison: Poison::new(ptype, 5, damage, 2000),
            });
        }
    }
}

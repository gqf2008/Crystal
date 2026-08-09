//! TucsonEgg（图森之蛋）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/TucsonEgg.cs
//! 机制：CanMove=false；攻击 AOE1 + 1/3 绿毒（5s，tick 2000）；
//!      Die：延迟伤害（AOE1 + 1/3 绿毒）+ Info.Effect==1 → SpawnSlave（孵化 TucsonGeneralEgg）

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;

const VIEW_RANGE: i32 = 12;
const AOE_RADIUS: i32 = 1;

pub struct TucsonEggBehavior;

impl TucsonEggBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for TucsonEggBehavior {
    fn can_move(&self) -> bool {
        false
    }

    /// C# Attacked：任何攻击固定 ChangeHP(-1) 且返回 1——蛋每次只受 1 点伤害
    fn on_attacked(&mut self, _damage: i32) -> i32 {
        1
    }

    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);

        if ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Aoe {
                attacker_oid: monster.object_id,
                center_x: monster.x,
                center_y: monster.y,
                radius: AOE_RADIUS,
                damage,
                spell_id: 0,
            });
            // C# CompleteAttack：1/3 绿毒（5s，tick 2000）
            let nearby: Vec<u64> = ctx.find_targets_in_range(monster.x, monster.y, AOE_RADIUS, monster.map_index)
                .iter().map(|p| p.session_id).collect();
            for sid in nearby {
                if fastrand::i32(0..3) == 0 {
                    ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                        session_id: sid,
                        poison: Poison::new(PoisonType::GREEN, 5, damage, 2000),
                    });
                }
            }
        }
    }

    /// C# Die：延迟 AOE1 + 1/3 绿毒；Effect==1 → 孵化 TucsonGeneralEgg
    fn on_die(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
        ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Aoe {
            attacker_oid: monster.object_id,
            center_x: monster.x,
            center_y: monster.y,
            radius: AOE_RADIUS,
            damage,
            spell_id: 0,
        });
        let nearby: Vec<u64> = ctx.find_targets_in_range(monster.x, monster.y, AOE_RADIUS, monster.map_index)
            .iter().map(|p| p.session_id).collect();
        for sid in nearby {
            if fastrand::i32(0..3) == 0 {
                ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                    session_id: sid,
                    poison: Poison::new(PoisonType::GREEN, 5, damage, 2000),
                });
            }
        }
        if monster.effect == 1 {
            ctx.out_summons.push(crate::actors::world::ai::BossSummon {
                monster_name: "TucsonGeneralEgg".to_string(),
                x: monster.x,
                y: monster.y,
                is_slave: false,
                summoner_oid: Some(monster.object_id),
            });
        }
    }
}

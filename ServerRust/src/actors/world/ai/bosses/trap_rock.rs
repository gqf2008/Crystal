//! TrapRock（陷阱岩）behavior（近似）
//!
//! C# 参考：Server/MirObjects/Monsters/TrapRock.cs
//! 机制：静态；2s 后伏击目标（传送到目标四角之一 + 100% 麻痹 3s）+ 攻击 1/8 麻痹；
//!      目标移动/死亡 → 自毁；不可受击（on_attacked 返 0）
//! 近似：子岩环绕（slave 状态机）暂不实现

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;

const VIEW_RANGE: i32 = 12;

pub struct TrapRockBehavior {
    shown: bool,
    target_loc: (i32, i32),
}

impl TrapRockBehavior {
    pub fn new() -> Self {
        Self { shown: false, target_loc: (0, 0) }
    }
}

impl MonsterBehavior for TrapRockBehavior {
    fn can_move(&self) -> bool {
        false
    }

    fn can_regen(&self) -> bool {
        false
    }

    fn on_attacked(&mut self, _damage: i32) -> i32 {
        0 // C# Struck 返 0：不可伤
    }

    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => {
                if self.shown {
                    monster.hp = 0;
                }
                return;
            }
        };
        monster.target_session = Some(target.session_id);
        if !self.shown {
            // C# Show：传送目标四角之一 + 100% 麻痹（3s）
            let corner = fastrand::i32(0..4) * 2;
            let tx = target.x + DIR_DX[corner as usize];
            let ty = target.y + DIR_DY[corner as usize];
            self.shown = true;
            self.target_loc = (target.x, target.y);
            ctx.out_monster_teleports.push((monster.object_id, tx, ty));
            ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                session_id: target.session_id,
                poison: Poison::new(PoisonType::PARALYSIS, 3, 0, 1000),
            });
            return;
        }
        // C# 目标移动/死亡 → 自毁
        if target.x != self.target_loc.0 || target.y != self.target_loc.1 || target.hp <= 0 {
            monster.hp = 0;
            return;
        }
        // C# Attack：远程 + 1/8 麻痹
        if ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                attacker_oid: monster.object_id,
                target_session: target.session_id,
                target_object_id: target.object_id,
                damage,
                spell_id: 0,
            });
            if fastrand::i32(0..8) == 0 {
                ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                    session_id: target.session_id,
                    poison: Poison::new(PoisonType::PARALYSIS, 3, 0, 1000),
                });
            }
        }
    }
}

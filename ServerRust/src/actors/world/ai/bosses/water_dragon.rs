//! WaterDragon（水龙）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/WaterDragon.cs（继承 EvilCentipede）
//! 机制（对齐 EvilCentipede 钻地/现身 + 自身水系吐息）：
//!   - 不能移动（CanMove=false），默认钻地隐身
//!   - 玩家靠近 3 格现身，离开 7 格再次隐身；隐身期免疫 + 满血
//!   - 现身期：贴身近战 DC；远程（>1 格）MC 弹道 + 5s 绿毒（水系吐息 PoisonTarget 7,5,Green）
//!
//! Attack（C# :35-73）：!ranged→DC ACAgility；ranged→MC MACAgility。
//! CompleteRangeAttack（C# :85-99）：finalDamage>0 → Green 7s。

use crate::actors::world::MonsterState;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

/// 现身检测间隔（C# EvilCentipede VisibleTime = Envir.Time + 2000）
const VISIBILITY_CHECK_TICKS: u64 = 20;
const APPEAR_RANGE: i32 = 3;
const DISAPPEAR_RANGE: i32 = 7;
const MELEE_RANGE: i32 = 1;

pub struct WaterDragonBehavior {
    visible: bool,
    next_visibility_tick: u64,
    spawned: bool,
}

impl WaterDragonBehavior {
    pub fn new() -> Self {
        Self { visible: false, next_visibility_tick: 0, spawned: false }
    }
}

impl MonsterBehavior for WaterDragonBehavior {
    fn can_move(&self) -> bool { false }
    fn can_regen(&self) -> bool { false }
    fn is_attackable(&self) -> bool { self.visible }
    fn on_attacked(&mut self, damage: i32) -> i32 {
        if self.visible { damage } else { 0 }
    }
    fn on_poison(&mut self, _poison: Poison) -> bool { false }

    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        if !self.spawned {
            self.next_visibility_tick = ctx.tick_count + VISIBILITY_CHECK_TICKS;
            self.spawned = true;
        }

        // 可见性切换（C# EvilCentipede.ProcessAI）
        if ctx.tick_count >= self.next_visibility_tick {
            self.next_visibility_tick = ctx.tick_count + VISIBILITY_CHECK_TICKS;
            let detect_range = if self.visible { DISAPPEAR_RANGE } else { APPEAR_RANGE };
            let has_near = ctx.nearest_target(monster.x, monster.y, detect_range, monster.map_index).is_some();
            if !self.visible && has_near {
                self.visible = true;
                monster.hp = monster.max_hp;
            } else if self.visible && !has_near {
                self.visible = false;
                monster.hp = monster.max_hp;
            }
        }

        if !self.visible {
            monster.hp = monster.max_hp;
            return;
        }

        // 现身期攻击
        if ctx.tick_count < monster.next_attack_tick {
            return;
        }
        let target = match ctx.nearest_target(monster.x, monster.y, DISAPPEAR_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= MELEE_RANGE {
            // 贴身近战 DC（C# DefenceType.ACAgility）
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                attacker_oid: monster.object_id,
                target_session: target.session_id,
                damage,
                spell_id: 0,
                attack_type: 0,
            });
        } else {
            // 水系吐息：MC 弹道 + 5s 绿毒（C# ranged AttackTime + AttackSpeed + 500）
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown + 5;
            let damage = crate::combat::attack::get_attack_power(monster.min_mac, monster.max_mac, 0).max(1);
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                attacker_oid: monster.object_id,
                target_session: target.session_id,
                target_object_id: target.object_id,
                damage,
                spell_id: 0,
            });
            // finalDamage>0 → Green 7s（C# PoisonTarget(target,7,5,Green,1000)）
            // C# PoisonTarget(7,5,Green,1000)：1/7
            if fastrand::i32(0..7) == 0 {
                ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                    session_id: target.session_id,
                    poison: Poison::new(PoisonType::GREEN, 5, poison_sc_value(monster), 1000),
                });
            }
        }
    }
}

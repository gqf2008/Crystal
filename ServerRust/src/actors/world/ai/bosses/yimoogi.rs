//! Yimoogi（蛇母/异魔蛇）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/Yimoogi.cs
//! 机制：可移动、出生4s后召唤同名分身（互引 SisterMob）、HP<10% 传送+召唤 WhiteSnake、
//! 三形态攻击（近战4/5 / 毒吐1/6红毒 / 远程弹兜底）

use crate::actors::world::MonsterState;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const ATTACK_RANGE: i32 = 7;
const POISON_RANGE: i32 = 4;

pub struct YimoogiBehavior {
    no_attack: bool,
    child_spawned: bool,
    is_child: bool,
    final_teleport: bool,
    spawn_ready_tick: u64,
}

impl YimoogiBehavior {
    pub fn new() -> Self {
        Self {
            no_attack: true,
            child_spawned: false,
            is_child: false,
            final_teleport: false,
            spawn_ready_tick: 0,
        }
    }
}

impl MonsterBehavior for YimoogiBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        // 初始化出生定时（C# Yimoogi.cs:29-34：4s 后才能召唤分身/攻击）
        if self.spawn_ready_tick == 0 {
            self.spawn_ready_tick = ctx.tick_count + 40; // 4s = 40 ticks
        }

        // HP<10% 一次性传送 + 召唤 WhiteSnake（C# Yimoogi.cs:115-135）
        if !self.is_child && !self.final_teleport && monster.hp <= monster.max_hp / 10 {
            // 传送：全图随机可行走格（C# TeleportRandom(40,0) → 随机 walkable cell）
            // behavior 无法查 walkability，推多个候选；tick 端 out_moves 逐个校验 walkable，最后有效者生效
            let (mw, mh) = ctx.map_size;
            for _ in 0..10 {
                ctx.out_moves.push((
                    monster.object_id,
                    fastrand::i32(0..mw.max(1)),
                    fastrand::i32(0..mh.max(1)),
                    monster.direction,
                ));
            }
            // 召唤 2 只 WhiteSnake（C# WhiteSerpent）
            for _ in 0..2 {
                ctx.out_summons.push(crate::actors::world::ai::BossSummon {
                    monster_name: "WhiteSerpent".to_string(),
                    x: monster.x + fastrand::i32(-1..=1),
                    y: monster.y + fastrand::i32(-1..=1),
                    is_slave: true,
                    summoner_oid: Some(monster.object_id),
                });
            }
            self.final_teleport = true;
            monster.target_session = None;
            return;
        }

        // 出生 4s 后召唤分身（仅本体，仅一次）（C# Yimoogi.cs:137-145）
        if !self.is_child && !self.child_spawned && ctx.tick_count >= self.spawn_ready_tick {
            // 召唤同名分身（IsChild=true）。Rust 用 monster_name 重新匹配 Yimoogi behavior，
            // 但 is_child 需要运行时标记——简化：召唤物名带后缀避免再次触发分身
            ctx.out_summons.push(crate::actors::world::ai::BossSummon {
                monster_name: format!("{} 分身", monster.name), // 分身不匹配 Yimoogi 注册
                x: monster.x + DIR_DX[monster.direction as usize % 8],
                y: monster.y + DIR_DY[monster.direction as usize % 8],
                is_slave: true,
                summoner_oid: Some(monster.object_id),
            });
            self.child_spawned = true;
            self.no_attack = false;
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            return;
        }

        if self.no_attack && ctx.tick_count < self.spawn_ready_tick {
            return;
        }
        self.no_attack = false;

        // 找目标 + 追击/攻击
        let target = match ctx.nearest_target(monster.x, monster.y, 20, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);

        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if ctx.tick_count >= monster.next_attack_tick && dist <= ATTACK_RANGE {
            // 攻击（C# Yimoogi.cs:67-113）
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);

            let in_melee = dist <= 2;
            if in_melee && fastrand::i32(0..5) > 0 {
                // 近战普攻 4/5
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id, damage, spell_id: 0, attack_type: 0,
                });
            } else if dist <= POISON_RANGE && fastrand::i32(0..6) == 0 {
                // 毒吐 1/6（红毒）
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id, damage, spell_id: 0, attack_type: 1,
                });
                ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                    session_id: target.session_id,
                    poison: Poison::new(PoisonType::RED, 6, poison_sc_value(monster), 2000),
                });
            } else {
                // 远程弹兜底
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    target_object_id: target.object_id,
                    damage, spell_id: 0,
                });
            }
        } else if dist > ATTACK_RANGE && ctx.tick_count >= monster.next_move_tick {
            // 追击
            if self.can_move() {
                let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
                ctx.out_moves.push((monster.object_id, nx, ny, dir));
                monster.next_move_tick = ctx.tick_count + monster.ai_profile.move_interval;
                monster.ai_state = crate::actors::world::MonsterAiState::Chase;
            }
        }
    }
}

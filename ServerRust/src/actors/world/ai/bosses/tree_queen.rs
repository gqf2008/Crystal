//! TreeQueen（树后）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/TreeQueen.cs
//! 机制：不能移动、双独立定时器驱动根刺法术场、近战时冷却×4 鼓励远程、
//! 近战两形态（FireBombardment 3格AOE / PushAttack 推开5格）

use crate::actors::world::MonsterState;
use crate::combat::poison::Poison;
use mir2_shared::enums::Spell;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;

pub struct TreeQueenBehavior {
    root_spawn_tick: u64,
    ground_root_spawn_tick: u64,
    not_near: bool,
    spawned: bool,
}

impl TreeQueenBehavior {
    pub fn new() -> Self {
        Self {
            root_spawn_tick: 0,
            ground_root_spawn_tick: 0,
            not_near: true,
            spawned: false,
        }
    }
}

impl MonsterBehavior for TreeQueenBehavior {
    fn can_move(&self) -> bool { false }
    fn can_regen(&self) -> bool { false }
    fn on_poison(&mut self, _poison: Poison) -> bool { false } // 免疫毒

    fn on_spawned(&mut self, _monster: &mut MonsterState) {
        // C# TreeQueen.cs:289-296：5s 后撒根，15s 后撒地根
        self.spawned = true;
    }

    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        if !self.spawned {
            self.root_spawn_tick = ctx.tick_count + 50;   // 5s
            self.ground_root_spawn_tick = ctx.tick_count + 150; // 15s
            self.spawned = true;
        }

        // 检测玩家是否近战（2 格内）。立即 copy 出来释放借用（避免后续 push 冲突）
        let near_player = ctx.nearest_target(monster.x, monster.y, 2, monster.map_index).copied();
        self.not_near = near_player.is_none();
        let near_mult = if self.not_near { 1 } else { 4 };

        // Root 定时器（C# TreeQueen.cs:298-310）
        if ctx.tick_count >= self.root_spawn_tick {
            if fastrand::i32(0..4) > 0 {
                // 3/4：单根（SpawnRoots）— 玩家附近随机点
                if let Some(t) = ctx.nearest_target(monster.x, monster.y, 30, monster.map_index).copied() {
                    let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0);
                    let dx = fastrand::i32(-5..=5);
                    let dy = fastrand::i32(-5..=5);
                    ctx.out_spell_fields.push(crate::actors::world::ai::SpellFieldSpawn {
                        spell: Spell::TreeQueenRoot,
                        x: t.x + dx,
                        y: t.y + dy,
                        value: damage.max(1),
                        duration_ms: 1500,
                        tick_ms: 2000,
                        caster_oid: monster.object_id,
                        caster_session: 0,
                    });
                }
            } else {
                // 1/4：群根（SpawnMassRoots）— 随机玩家脚下 7×7
                let targets: Vec<crate::actors::world::ai::PlayerSnap> = ctx.players.iter()
                    .filter(|p| p.map_index == monster.map_index && p.hp > 0)
                    .copied().collect();
                if let Some(t) = targets.get(fastrand::usize(0..targets.len())) {
                    let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0);
                    let offsets: [(i32, i32); 5] = [(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)];
                    for (ox, oy) in offsets {
                        ctx.out_spell_fields.push(crate::actors::world::ai::SpellFieldSpawn {
                            spell: Spell::TreeQueenMassRoots,
                            x: t.x + ox,
                            y: t.y + oy,
                            value: damage.max(1),
                            duration_ms: 1500,
                            tick_ms: 1000,
                            caster_oid: monster.object_id,
                            caster_session: 0,
                        });
                    }
                }
            }
            let next = fastrand::i32(1..=3) as u64 * 10; // 1-3s = 10-30 ticks
            self.root_spawn_tick = ctx.tick_count + next * near_mult as u64;
        }

        // GroundRoot 定时器（C# TreeQueen.cs:311-318）
        if ctx.tick_count >= self.ground_root_spawn_tick {
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0);
            // 自身周围 5×5 散点
            for _ in 0..3 {
                let dx = fastrand::i32(-5..=5);
                let dy = fastrand::i32(-5..=5);
                ctx.out_spell_fields.push(crate::actors::world::ai::SpellFieldSpawn {
                    spell: Spell::TreeQueenGroundRoots,
                    x: monster.x + dx,
                    y: monster.y + dy,
                    value: damage.max(1),
                    duration_ms: 900,
                    tick_ms: 1000,
                    caster_oid: monster.object_id,
                    caster_session: 0,
                });
            }
            let next = fastrand::i32(2..=3) as u64 * 10;
            self.ground_root_spawn_tick = ctx.tick_count + next * near_mult as u64;
        }

        // 近战攻击（C# TreeQueen.cs:54-100）：玩家 2 格内才攻击
        if let Some(t) = near_player {
            if ctx.tick_count >= monster.next_attack_tick {
                monster.next_attack_tick = ctx.tick_count + 5;
                let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
                let is_fire = fastrand::i32(0..2) == 0;
                if is_fire {
                    // FireBombardment：自身 3 格 AOE
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Aoe {
                    attacker_oid: monster.object_id,
                        center_x: monster.x, center_y: monster.y, radius: 3,
                        damage, spell_id: 0,
                    });
                } else {
                    // PushAttack：自身 1 格 + 推开（简化为单体伤害）
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                        target_session: t.session_id, damage, spell_id: 0, attack_type: 1,
                    });
                }
            }
        }
    }
}

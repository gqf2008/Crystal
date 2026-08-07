//! HellLord（地狱领主）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/HellLord.cs
//! 机制：不能移动、5阶段(stage 0..4，靠 Knight 被杀推进)、stage<4 完全无敌、
//! 自身不直接攻击（空 Attack），纯靠召唤 Knight + 撒 Bomb + Quake 法术场

use crate::actors::world::MonsterState;
use crate::combat::poison::Poison;
use mir2_shared::enums::Spell;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;

const RAGE_DURATION_TICKS: u64 = 1200; // 2 分钟狂暴

pub struct HellLordBehavior {
    stage: u8,        // 0..4
    begin: bool,
    raged: bool,
    rage_end_tick: u64,
}

impl HellLordBehavior {
    pub fn new() -> Self {
        Self {
            stage: 0,
            begin: true,
            raged: false,
            rage_end_tick: 0,
        }
    }

    /// Knight 被杀时推进阶段（由外部回调）
    pub fn advance_stage(&mut self, current_tick: u64) {
        self.raged = true;
        self.rage_end_tick = current_tick + RAGE_DURATION_TICKS;
        self.stage = (self.stage + 1).min(4);
    }
}

impl MonsterBehavior for HellLordBehavior {
    fn can_move(&self) -> bool { false }
    fn can_regen(&self) -> bool { false }
    fn on_poison(&mut self, _poison: Poison) -> bool { false } // 完全免疫毒

    fn as_any_mut(&mut self) -> Option<&mut dyn std::any::Any> {
        Some(self)
    }

    fn on_attacked(&mut self, damage: i32) -> i32 {
        // C# HellLord.cs:47-64：stage<4 时完全无敌
        if self.stage < 4 {
            0
        } else {
            damage
        }
    }

    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        // C# Process：玩家全部离开则复位
        let players_on_map = ctx.players.iter()
            .filter(|p| p.map_index == monster.map_index && p.hp > 0)
            .count();
        if players_on_map == 0 && self.stage > 0 {
            self.stage = 0;
            self.begin = true;
            return;
        }

        if ctx.tick_count < monster.next_attack_tick {
            return;
        }

        // ===== 阶段推进检测 =====
        // C# 语义：Knight 被玩家杀死 → KnightKilled() → stage += 1 + 狂暴 2min（由死亡回调 advance_stage 触发）；
        // 狂暴到期只补刷下一只 Knight（stage 不变）

        monster.next_attack_tick = ctx.tick_count + 6;

        // 狂暴到期 或 初次 → 召唤当前阶段 Knight（C# ProcessTarget）
        if (self.raged && ctx.tick_count >= self.rage_end_tick && self.stage < 4) || self.begin {
            self.begin = false;
            self.raged = false;
            let knight_names = ["HellKnight1", "HellKnight2", "HellKnight3", "HellKnight4"];
            if self.stage < 4 {
                if let Some(name) = knight_names.get(self.stage as usize) {
                    ctx.out_summons.push(crate::actors::world::ai::BossSummon {
                        monster_name: name.to_string(),
                        x: monster.x + fastrand::i32(-10..=10),
                        y: monster.y + fastrand::i32(-10..=10),
                        is_slave: true,
                    });
                }
            }
        }

        // 1/3 或狂暴时撒 Bomb（C# SpawnBomb）
        if self.raged || fastrand::i32(0..3) == 0 {
            let bombs = ["HellBomb1", "HellBomb2", "HellBomb3"];
            let bomb = bombs[fastrand::usize(0..3)];
            for p in ctx.players.iter().filter(|p| p.map_index == monster.map_index && p.hp > 0).take(1) {
                ctx.out_summons.push(crate::actors::world::ai::BossSummon {
                    monster_name: bomb.to_string(),
                    x: p.x + fastrand::i32(-5..=15),
                    y: p.y + fastrand::i32(-5..=15),
                    is_slave: false, // Bomb 不加 SlaveList（散落不管）
                });
            }
        }

        // 每次撒 Quake 法术场（C# SpawnQuakes）
        let quake_count = if self.raged { 10 } else { 5 };
        for p in ctx.players.iter().filter(|p| p.map_index == monster.map_index && p.hp > 0).take(1) {
            for _ in 0..quake_count {
                let dx = fastrand::i32(-15..=15);
                let dy = fastrand::i32(-15..=15);
                let spell = if fastrand::i32(0..2) == 0 { Spell::MapQuake1 } else { Spell::MapQuake2 };
                let damage = fastrand::i32(monster.min_dmg..=monster.max_dmg).max(1);
                ctx.out_spell_fields.push(crate::actors::world::ai::SpellFieldSpawn {
                    spell,
                    x: p.x + dx,
                    y: p.y + dy,
                    value: damage,
                    duration_ms: 2000,
                    tick_ms: 500,
                    caster_oid: monster.object_id,
                    caster_session: 0,
                });
            }
        }
    }
}

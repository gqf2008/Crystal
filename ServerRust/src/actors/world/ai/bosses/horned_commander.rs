//! HornedCommander（角魔统帅）behavior — 最复杂
//!
//! C# 参考：Server/MirObjects/Monsters/HornedCommander.cs
//! 机制：3阶段（HP<80%召唤8 Boulder / HP∈[10%,50%)周期RockSpike / HP<10%开20s盾+召唤Slave）
//! + 高级模式解锁6种攻击 + 免疫期 + 传送

use crate::actors::world::MonsterState;
use mir2_shared::enums::Spell;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

pub struct HornedCommanderBehavior {
    start_advanced: bool,
    immune: bool,
    called_boulders: bool,
    called_rock_spikes: bool,
    called_shield: bool,
    rock_spike_tick: u64,
    shield_end_tick: u64,
    /// C# _RockSpikeArea 7×7 锚点（以地图中心为基准，间距 5 格）
    rock_spike_anchors: Vec<(i32, i32)>,
    /// 下一个待生成的锚点索引
    rock_spike_index: usize,
}

impl HornedCommanderBehavior {
    pub fn new() -> Self {
        Self {
            start_advanced: false,
            immune: false,
            called_boulders: false,
            called_rock_spikes: false,
            called_shield: false,
            rock_spike_tick: 0,
            shield_end_tick: 0,
            rock_spike_anchors: Vec::new(),
            rock_spike_index: 0,
        }
    }
}

impl MonsterBehavior for HornedCommanderBehavior {
    fn is_attackable(&self) -> bool {
        !self.immune
    }

    fn on_attacked(&mut self, damage: i32) -> i32 {
        if self.immune {
            0
        } else {
            damage
        }
    }

    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        // 护盾到期检查（C# ProcessBuffEnd）
        if self.immune && ctx.tick_count >= self.shield_end_tick {
            self.immune = false;
            monster.next_attack_tick = ctx.tick_count + 3;
        }

        let hp_pct = (monster.hp as f32 / monster.max_hp as f32) * 100.0;

        // 进入高级模式（HP<100%，C# _StartAdvanced）
        if !self.start_advanced && hp_pct < 100.0 {
            self.start_advanced = true;
        }

        // Phase 0: HP<80% 召唤 8 个 Boulder（C# SpawnBoulder）
        if hp_pct < 80.0 && !self.called_boulders {
            for i in 0..8 {
                let dist = if i % 2 != 0 { 7 } else { 9 };
                let dir = i as usize % 8;
                ctx.out_summons.push(crate::actors::world::ai::BossSummon {
                    monster_name: "BoulderSpirit".to_string(),
                    x: monster.x + DIR_DX[dir] * dist,
                    y: monster.y + DIR_DY[dir] * dist,
                    is_slave: true,
                });
            }
            self.called_boulders = true;
        }

        if self.immune {
            return; // 护盾期跳过其余逻辑
        }

        // Phase 2: HP<10% 开盾 20s + 召唤 Slave（C# ProcessAI Phase 2）
        if hp_pct < 10.0 && !self.called_shield {
            self.called_shield = true;
            self.immune = true;
            self.shield_end_tick = ctx.tick_count + 200; // 20s = 200 ticks
            ctx.out_summons.push(crate::actors::world::ai::BossSummon {
                monster_name: "HornedSorceror".to_string(),
                x: monster.x + DIR_DX[monster.direction as usize % 8],
                y: monster.y + DIR_DY[monster.direction as usize % 8],
                is_slave: true,
            });
            return;
        }

        // Phase 1: HP∈[10%,50%) 周期刷 RockSpike（C# ProcessAI Phase 1）
        if hp_pct < 50.0 && hp_pct >= 10.0 {
            if !self.called_rock_spikes {
                self.called_rock_spikes = true;
                // C# SetupRockSpike：以地图中心为基准的 7×7 锚点网格，间距 5 格
                let (mw, mh) = ctx.map_size;
                let cx = mw / 2;
                let cy = mh / 2;
                self.rock_spike_anchors.clear();
                for ax in 0..7i32 {
                    for ay in 0..7i32 {
                        self.rock_spike_anchors.push((cx + (ax - 3) * 5, cy + (ay - 3) * 5));
                    }
                }
                self.rock_spike_index = 0;
            }
            if ctx.tick_count >= self.rock_spike_tick {
                // C# SpawnRockSpikes：每 5s 推进一个锚点，生成其周围 5×5 法术场
                if self.rock_spike_index < self.rock_spike_anchors.len() {
                    let (anchor_x, anchor_y) = self.rock_spike_anchors[self.rock_spike_index];
                    self.rock_spike_index += 1;
                    let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
                    for dy in -2..=2i32 {
                        for dx in -2..=2i32 {
                            ctx.out_spell_fields.push(crate::actors::world::ai::SpellFieldSpawn {
                                spell: Spell::HornedCommanderRockSpike,
                                x: anchor_x + dx,
                                y: anchor_y + dy,
                                value: damage,
                                duration_ms: 10 * 60 * 1000, // 10 分钟
                                tick_ms: 1000,
                                caster_oid: monster.object_id,
                                caster_session: 0,
                            });
                        }
                    }
                }
                self.rock_spike_tick = ctx.tick_count + 50; // 5s
            }
        }

        // 攻击逻辑（C# Attack：6 种形态随机）
        let target = match ctx.nearest_target(monster.x, monster.y, 20, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);

        let dist = max_distance(monster.x, monster.y, target.x, target.y);
        let attack_range = 2;

        if dist <= attack_range && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + 5;
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);

            if self.start_advanced {
                let roll = fastrand::i32(0..20);
                if roll == 0 {
                    // RockFall（1/20）：前方大范围 AOE
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Aoe {
                    attacker_oid: monster.object_id,
                        center_x: target.x, center_y: target.y, radius: 5,
                        damage: damage * 5, spell_id: 0,
                    });
                    self.immune = true;
                    self.shield_end_tick = ctx.tick_count + 30; // 蓄力期短暂免疫
                } else {
                    // 普攻
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                        target_session: target.session_id, damage, spell_id: 0, attack_type: 0,
                    });
                }
            } else {
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id, damage, spell_id: 0, attack_type: 0,
                });
            }
        } else if dist > attack_range && ctx.tick_count >= monster.next_move_tick {
            // 追击（C# 标准 MoveTo）
            let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + 2;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }

    fn on_die(&mut self, _monster: &mut MonsterState, _ctx: &mut AiCtx) {
        // C# Die：KillRockSpikes + KillSlaves（由调用方通过 is_slave 清理召唤物）
    }
}

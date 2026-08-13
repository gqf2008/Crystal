//! HornedCommander（角魔统帅）behavior — 最复杂
//!
//! C# 参考：Server/MirObjects/Monsters/HornedCommander.cs
//! 机制：3阶段（HP<80%召唤8 Boulder / HP∈[10%,50%)周期RockSpike / HP<10%开20s盾+召唤Slave）
//! + 高级模式解锁6种攻击 + 免疫期 + 传送

use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::actors::world::MonsterState;
use mir2_shared::enums::Spell;

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

/// C# HornedCommander.TeleportRandom(10, 10)：最多 10 次尝试返回 ±10 内可走点（None=失败留在原地）
fn teleport_random_point(
    x: i32,
    y: i32,
    is_walkable: impl Fn(i32, i32) -> bool,
) -> Option<(i32, i32)> {
    for _ in 0..10 {
        let tx = x + fastrand::i32(-10..=10);
        let ty = y + fastrand::i32(-10..=10);
        if is_walkable(tx, ty) {
            return Some((tx, ty));
        }
    }
    None
}

impl Default for HornedCommanderBehavior {
    fn default() -> Self {
        Self::new()
    }
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
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
        }

        let hp_pct = (monster.hp as f32 / monster.max_hp as f32) * 100.0;

        // 进入高级模式（HP<100%，C# _StartAdvanced）
        if !self.start_advanced && hp_pct < 100.0 {
            self.start_advanced = true;
        }
        // C# ProcessAI（:114-121）+ Reset（:126-137）：HP 回满 → 重置全部阶段标志
        //（C# 还会 KillRockSpikes/KillSlaves——法术场由调用方按召唤物归属清理）
        if self.start_advanced && hp_pct >= 100.0 {
            self.start_advanced = false;
            self.called_boulders = false;
            self.called_rock_spikes = false;
            self.called_shield = false;
            self.immune = false;
            self.rock_spike_index = 0;
            self.rock_spike_anchors.clear();
        }

        // Phase 0: HP<80% 召唤 8 个 Boulder（C# SpawnBoulder）
        if hp_pct < 80.0 && !self.called_boulders {
            // C# SpawnBoulder（:525-530）：距地图中心(26,32)≤20 → 先传送到中心再召唤
            //（C# _MapCentre 硬编码 HY01_morae_chon 中心）
            const MAP_CENTRE_X: i32 = 26;
            const MAP_CENTRE_Y: i32 = 32;
            let (bx, by) = if max_distance(monster.x, monster.y, MAP_CENTRE_X, MAP_CENTRE_Y) <= 20 {
                ctx.out_monster_teleports
                    .push((monster.object_id, MAP_CENTRE_X, MAP_CENTRE_Y));
                (MAP_CENTRE_X, MAP_CENTRE_Y)
            } else {
                (monster.x, monster.y)
            };
            for i in 0..8 {
                let dist = if i % 2 != 0 { 7 } else { 9 };
                let dir = i as usize % 8;
                ctx.out_summons.push(crate::actors::world::ai::BossSummon {
                    monster_name: "BoulderSpirit".to_string(),
                    x: bx + DIR_DX[dir] * dist,
                    y: by + DIR_DY[dir] * dist,
                    is_slave: true,
                    summoner_oid: Some(monster.object_id),
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
                summoner_oid: Some(monster.object_id),
            });
            return;
        }

        // Phase 1: HP∈[10%,50%) 周期刷 RockSpike（C# ProcessAI Phase 1）
        if (10.0..50.0).contains(&hp_pct) {
            if !self.called_rock_spikes {
                self.called_rock_spikes = true;
                // C# SetupRockSpike：以地图中心为基准的 7×7 锚点网格，间距 5 格
                let (mw, mh) = ctx.map_size;
                let cx = mw / 2;
                let cy = mh / 2;
                self.rock_spike_anchors.clear();
                for ax in 0..7i32 {
                    for ay in 0..7i32 {
                        self.rock_spike_anchors
                            .push((cx + (ax - 3) * 5, cy + (ay - 3) * 5));
                    }
                }
                self.rock_spike_index = 0;
            }
            if ctx.tick_count >= self.rock_spike_tick {
                // C# SpawnRockSpikes：每 5s 推进一个锚点，生成其周围 5×5 法术场
                if self.rock_spike_index < self.rock_spike_anchors.len() {
                    let (anchor_x, anchor_y) = self.rock_spike_anchors[self.rock_spike_index];
                    self.rock_spike_index += 1;
                    // C# SpawnRockSpikes 值=MC（HornedCommander.cs:391/398）
                    let damage =
                        crate::combat::attack::get_attack_power(monster.min_mc, monster.max_mc, 0)
                            .max(1);
                    for dy in -2..=2i32 {
                        for dx in -2..=2i32 {
                            ctx.out_spell_fields
                                .push(crate::actors::world::ai::SpellFieldSpawn {
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

        // 攻击逻辑（C# Attack：6 种形态；顺序概率 1/20 RockFall → 1/15 SpinHit → 1/10 HammerSmash → 1/10 Teleport → 50/50 普攻）
        let target = match ctx.nearest_target(monster.x, monster.y, 20, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);

        let dist = max_distance(monster.x, monster.y, target.x, target.y);
        let attack_range = 2;

        if dist <= attack_range && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let damage =
                crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
            let dir = direction_towards(monster.x, monster.y, target.x, target.y);
            monster.direction = dir;
            // 前方 2 格（C# Attack：front = PointMove(CurrentLocation, Direction, 2)）
            let (fx, fy) = (
                monster.x + DIR_DX[dir as usize] * 2,
                monster.y + DIR_DY[dir as usize] * 2,
            );

            if self.start_advanced {
                // 1/20 RockFall（C# :188-209）：蓄力期免疫，DC×loops(5-10)，前方 AOE5
                if fastrand::i32(0..20) == 0 {
                    let loops = fastrand::i32(5..10).max(1);
                    self.immune = true;
                    // C# ActionTime = Time + loops*500 + 500（10ms/tick）
                    self.shield_end_tick = ctx.tick_count + (loops as u64 * 5) + 5;
                    ctx.out_attacks
                        .push(crate::actors::world::ai::AttackAction::Aoe {
                            attacker_oid: monster.object_id,
                            center_x: fx,
                            center_y: fy,
                            radius: 5,
                            damage: damage.saturating_mul(loops),
                            spell_id: 0,
                        });
                    return;
                }
                // 1/15 SpinHit（C# :211-232）：蓄力期免疫，DC×loops(5-10)，自身 AOE3
                if fastrand::i32(0..15) == 0 {
                    let loops = fastrand::i32(5..10).max(1);
                    self.immune = true;
                    // C# spinDuration = loops*700 + 1500（10ms/tick）
                    self.shield_end_tick = ctx.tick_count + (loops as u64 * 7) + 15;
                    ctx.out_attacks
                        .push(crate::actors::world::ai::AttackAction::Aoe {
                            attacker_oid: monster.object_id,
                            center_x: monster.x,
                            center_y: monster.y,
                            radius: 3,
                            damage: damage.saturating_mul(loops),
                            spell_id: 0,
                        });
                    return;
                }
                // 1/10 HammerSmash（C# :234-245）：DC，前方 AOE3（CompleteRangeAttack aoeSize=3 at front）
                if fastrand::i32(0..10) == 0 {
                    ctx.out_attacks
                        .push(crate::actors::world::ai::AttackAction::Aoe {
                            attacker_oid: monster.object_id,
                            center_x: fx,
                            center_y: fy,
                            radius: 3,
                            damage,
                            spell_id: 0,
                        });
                    return;
                }
                // 1/10 Teleport（C# :247-256）：空 RangeDamage → CompleteRangeAttack → TeleportRandom(10,10) + Target=null
                if fastrand::i32(0..10) == 0 {
                    if let Some((tx, ty)) =
                        teleport_random_point(monster.x, monster.y, |x, y| (ctx.is_walkable)(x, y))
                    {
                        ctx.out_monster_teleports.push((monster.object_id, tx, ty));
                    }
                    monster.target_session = None;
                    return;
                }
            }
            // 普攻（C# :258-278）：50/50 Type0/1（同为 ACAgility，仅表现类型不同）
            ctx.out_attacks
                .push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage,
                    spell_id: 0,
                    attack_type: if fastrand::i32(0..2) == 0 { 0 } else { 1 },
                });
        } else if dist > attack_range && ctx.tick_count >= monster.next_move_tick {
            // 追击（C# 标准 MoveTo）
            let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + monster.ai_profile.move_interval;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }

    fn on_die(&mut self, _monster: &mut MonsterState, _ctx: &mut AiCtx) {
        // C# Die：KillRockSpikes + KillSlaves（由调用方通过 is_slave 清理召唤物）
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn teleport_random_point_within_range_and_walkable() {
        // C# TeleportRandom(10,10)：返回点必须在 ±10 内
        for _ in 0..500 {
            let pt = teleport_random_point(100, 100, |_, _| true).expect("always walkable");
            assert!(
                (pt.0 - 100).abs() <= 10 && (pt.1 - 100).abs() <= 10,
                "out of range: {:?}",
                pt
            );
        }
    }

    #[test]
    fn teleport_random_point_fails_when_no_walkable() {
        // 全部不可走 → None（留在原地）
        assert_eq!(teleport_random_point(0, 0, |_, _| false), None);
    }

    #[test]
    fn teleport_random_point_uses_walkable() {
        // 可走域 = 第一象限：返回点必须落在域内且 ±10 内；10 次尝试全失败（~2^-10）→ None 也合法
        let mut got = 0;
        for _ in 0..500 {
            if let Some(pt) = teleport_random_point(0, 0, |x, y| x >= 0 && y >= 0) {
                got += 1;
                assert!((pt.0).abs() <= 10 && (pt.1).abs() <= 10);
                assert!(pt.0 >= 0 && pt.1 >= 0, "walkable check ignored: {:?}", pt);
            }
        }
        assert!(got > 0, "500 次均未命中可走点");
    }
}

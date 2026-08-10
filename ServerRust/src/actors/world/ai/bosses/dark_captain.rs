//! DarkCaptain（黑暗队长）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/DarkCaptain.cs
//! 机制（3 个周期性特殊技能 + 近战）：
//!   - _ThunderTime（10-20s）：MC 雷击 → 目标 2 格 AOE（MACAgility）
//!   - _MassThunderTime（20-50s）：MC 大雷击 → 目标 5 格大 AOE
//!   - _OrbTime（30-40s）：召唤 PowerBead（在 ±4 格随机点尝试 4 次）
//!   - 近战 4/5：DC LineAttack(2)；1/5：DC Fullmoon 推开
//!   - 1/5 概率传送到更弱目标背后
//!
//! Attack（C# :20-119）：三定时器优先级 > 传送 > 近战。
//! CompleteRangeAttack（C# :121-136）：FindAllTargets(range) AOE。

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 15;
/// C# Settings.PowerBead 召唤物名（近似）
const ORB_MOB_NAME: &str = "PowerBead";

pub struct DarkCaptainBehavior {
    next_thunder_tick: u64,
    next_mass_thunder_tick: u64,
    next_orb_tick: u64,
    spawned: bool,
}

impl DarkCaptainBehavior {
    pub fn new() -> Self {
        Self { next_thunder_tick: 0, next_mass_thunder_tick: 0, next_orb_tick: 0, spawned: false }
    }
}

impl MonsterBehavior for DarkCaptainBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        if !self.spawned {
            // C# 构造：_ThunderTime=Time+10000；_MassThunderTime=Time+20000
            self.next_thunder_tick = ctx.tick_count + 100;
            self.next_mass_thunder_tick = ctx.tick_count + 200;
            self.next_orb_tick = ctx.tick_count + 300;
            self.spawned = true;
        }

        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= VIEW_RANGE && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;

            // ---- Thunder（2 格 AOE）----
            if ctx.tick_count >= self.next_thunder_tick {
                self.next_thunder_tick = ctx.tick_count + 100 + fastrand::u64(0..100);
                let damage = crate::combat::attack::get_attack_power(monster.min_mac, monster.max_mac, 0).max(1);
                let hits: Vec<crate::actors::world::ai::PlayerSnap> =
                    ctx.find_targets_in_range(monster.x, monster.y, 2, monster.map_index)
                        .into_iter().copied().collect();
                for h in hits {
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                        attacker_oid: monster.object_id,
                        target_session: h.session_id,
                        target_object_id: h.object_id,
                        damage,
                        spell_id: 0,
                    });
                }
                return;
            }

            // ---- MassThunder（5 格大 AOE）----
            if ctx.tick_count >= self.next_mass_thunder_tick {
                self.next_mass_thunder_tick = ctx.tick_count + 200 + fastrand::u64(0..300);
                let damage = crate::combat::attack::get_attack_power(monster.min_mac, monster.max_mac, 0).max(1);
                let hits: Vec<crate::actors::world::ai::PlayerSnap> =
                    ctx.find_targets_in_range(monster.x, monster.y, 5, monster.map_index)
                        .into_iter().copied().collect();
                for h in hits {
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                        attacker_oid: monster.object_id,
                        target_session: h.session_id,
                        target_object_id: h.object_id,
                        damage,
                        spell_id: 1,
                    });
                }
                return;
            }

            // ---- Orb 召唤（±4 格随机）----
            if ctx.tick_count >= self.next_orb_tick {
                self.next_orb_tick = ctx.tick_count + 300 + fastrand::u64(0..100);
                for _ in 0..4 {
                    let ox = monster.x + fastrand::i32(-4..=4);
                    let oy = monster.y + fastrand::i32(-4..=4);
                    ctx.out_summons.push(crate::actors::world::ai::BossSummon {
                        monster_name: ORB_MOB_NAME.to_string(),
                        x: ox,
                        y: oy,
                        is_slave: true,
                        summoner_oid: Some(monster.object_id),
                    });
                }
                return;
            }

            // ---- 近战分支 ----
            // 1/5 传送到更弱目标背后（C# TeleportBehindWeakerTarget）
            if fastrand::i32(0..5) == 0 {
                // 选视野内 MinDC 最低玩家（C# TeleportBehindWeakerTarget 按 MinDC）
                let weakest_opt: Option<(u64, i32, i32)> = {
                    let targets: Vec<_> = ctx
                        .find_targets_in_range(monster.x, monster.y, VIEW_RANGE, monster.map_index)
                        .into_iter()
                        .copied()
                        .collect();
                    weakest_player_by_dc(&targets).map(|p| (p.session_id, p.x, p.y))
                };
                if let Some((wsession, wx, wy)) = weakest_opt {
                    // 背后点：目标位置沿“远离队长”方向 1 格（PlayerSnap 无目标朝向，近似）
                    let dir = direction_towards(wx, wy, monster.x, monster.y);
                    let bx = wx + DIR_DX[dir as usize];
                    let by = wy + DIR_DY[dir as usize];
                    // 推 3 个候选（背后点 + 两个相邻偏移），tick 端校验 walkable
                    for (ox, oy) in [(bx, by), (bx + 1, by), (bx, by + 1)] {
                        ctx.out_moves.push((monster.object_id, ox, oy, dir));
                    }
                    monster.target_session = Some(wsession);
                }
                return;
            }

            if fastrand::i32(0..5) > 0 {
                // 4/5 DC LineAttack(2)
                let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage,
                    spell_id: 0,
                    attack_type: 0,
                });
            } else {
                // 1/5 DC Fullmoon 推开（AOE 1-2 格）
                let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
                let hits: Vec<crate::actors::world::ai::PlayerSnap> =
                    ctx.find_targets_in_range(monster.x, monster.y, 2, monster.map_index)
                        .into_iter().copied().collect();
                for h in hits {
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                        attacker_oid: monster.object_id,
                        target_session: h.session_id,
                        damage,
                        spell_id: 0,
                        attack_type: 1,
                    });
                    let (nx, ny, _d) = step_away(h.x, h.y, monster.x, monster.y);
                    ctx.out_moves.push((h.object_id, nx, ny, 0));
                }
            }
            return;
        }

        // 追击
        if ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + monster.ai_profile.move_interval;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}

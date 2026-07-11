//! DarkOmaKing（暗黑奥玛之王）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/DarkOmaKing.cs
//! 机制：可移动、AttackRange=6、双独立定时器驱动：
//!   - _OrbTime (20s)：召唤 2 个 PowerBead（在 8 距离内随机点）
//!   - _MassThunderTime (10s + 0-5s 抖动)：MassThunder AOE（自身 5 格 AOE，MAC 伤害）
//! 攻击：
//!   - 近战 (距离<=3)：3/4 普攻 DC / 1/4 FullmoonAttack 三连击(溅射) + 前方 3 格 DarkOmaKingNuke 法术场
//!   - 远程 (>3)：1/3 概率弹道远程攻击（MAC）
//! 死亡：清理 SlaveList（PowerBead）

use crate::actors::world::MonsterState;
use mir2_shared::enums::Spell;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

/// 攻击视野范围（C# AttackRange=6，但 ProcessTarget 用 ViewRange 寻敌）
const VIEW_RANGE: i32 = 20;
/// 近战判定距离（C# InRange(CurrentLocation, Target, 3)）
const MELEE_RANGE: i32 = 3;
/// Orb（PowerBead）召唤周期：20s = 200 ticks（C# _OrbTime）
const ORB_INTERVAL_TICKS: u64 = 200;
/// MassThunder 基础周期：10s = 100 ticks（C# _MassThunderTime）
const MASS_THUNDER_BASE_TICKS: u64 = 100;
/// MassThunder 随机抖动上限：5s = 50 ticks
const MASS_THUNDER_JITTER_TICKS: u64 = 50;
/// MassThunder AOE 半径（C# FindAllTargets(5, CurrentLocation)）
const MASS_THUNDER_RADIUS: i32 = 5;
/// PowerBead 召唤点距自身的最大距离（C# distance=8）
const ORB_SPAWN_DISTANCE: i32 = 8;
/// 每次召唤 PowerBead 数量（C# count=2）
const ORB_SPAWN_COUNT: usize = 2;

pub struct DarkOmaKingBehavior {
    /// 下次召唤 PowerBead 的 tick（对齐 C# _OrbTime）
    next_bead_tick: u64,
    /// 下次 MassThunder 的 tick（对齐 C# _MassThunderTime）
    next_thunder_tick: u64,
}

impl DarkOmaKingBehavior {
    pub fn new() -> Self {
        Self {
            // C# 构造函数：_MassThunderTime = Envir.Time + 10000; _OrbTime = Envir.Time + 20000
            // on_spawned 会用真实 tick_count 重置
            next_bead_tick: 0,
            next_thunder_tick: 0,
        }
    }
}

impl MonsterBehavior for DarkOmaKingBehavior {
    fn on_spawned(&mut self, _monster: &mut MonsterState) {
        // 占位：真实 tick_count 在首次 process_tick 时懒初始化（on_spawned 拿不到 ctx）
        self.next_bead_tick = 0;
        self.next_thunder_tick = 0;
    }

    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        // 懒初始化定时器（对齐 C# 构造函数：Thunder +10s, Orb +20s）
        if self.next_thunder_tick == 0 {
            self.next_thunder_tick = ctx.tick_count + MASS_THUNDER_BASE_TICKS;
        }
        if self.next_bead_tick == 0 {
            self.next_bead_tick = ctx.tick_count + ORB_INTERVAL_TICKS;
        }

        // 无目标时不行动（C# ProcessTarget：Target==null 直接 return）
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);

        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        // ---- 定时器驱动：PowerBead 召唤（C# DarkOmaKing.cs:44-68）----
        if ctx.tick_count >= self.next_bead_tick {
            self.next_bead_tick = ctx.tick_count + ORB_INTERVAL_TICKS;
            // C# count=2，每个 bead 在 ±distance=8 内随机点，避开自身和目标位置
            for _ in 0..ORB_SPAWN_COUNT {
                // 4 次尝试（C# attempts=4）
                for _ in 0..4 {
                    let dx = fastrand::i32(-ORB_SPAWN_DISTANCE..=ORB_SPAWN_DISTANCE);
                    let dy = fastrand::i32(-ORB_SPAWN_DISTANCE..=ORB_SPAWN_DISTANCE);
                    let sx = monster.x + dx;
                    let sy = monster.y + dy;
                    // 避开自身和目标（C# location == CurrentLocation || == Target 则 continue）
                    if (sx == monster.x && sy == monster.y)
                        || (sx == target.x && sy == target.y)
                    {
                        continue;
                    }
                    ctx.out_summons.push(crate::actors::world::ai::BossSummon {
                        monster_name: "PowerBead".to_string(),
                        x: sx,
                        y: sy,
                        is_slave: true, // 加入 slave_list，Boss 死亡时清理
                    });
                    break;
                }
            }
        }

        // ---- 定时器驱动：MassThunder AOE（C# DarkOmaKing.cs:70-85）----
        if ctx.tick_count >= self.next_thunder_tick {
            // C# 10s + Random(0,5000)
            let jitter = fastrand::u64(0..MASS_THUNDER_JITTER_TICKS);
            self.next_thunder_tick = ctx.tick_count + MASS_THUNDER_BASE_TICKS + jitter;

            // MAC 伤害 AOE（C# GetAttackPower(MinMC, MaxMC)）
            let damage = crate::combat::attack::get_attack_power(monster.min_mac, monster.max_mac, monster.luck).max(1);
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Aoe {
                attacker_oid: monster.object_id,
                center_x: monster.x,
                center_y: monster.y,
                radius: MASS_THUNDER_RADIUS,
                damage,
                spell_id: 0,
            });
            return; // C# MassThunder 分支后直接 return（ActionTime 已推迟）
        }

        // ---- 攻击 / 追击（C# Attack + ProcessTarget）----
        if dist <= MELEE_RANGE {
            if ctx.tick_count < monster.next_attack_tick {
                return;
            }
            monster.next_attack_tick = ctx.tick_count + 5;

            // C# DarkOmaKing.cs:87-133：ranged=false 时
            if fastrand::i32(0..4) > 0 {
                // 3/4：普攻 DC（Type=0）
                let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage,
                    spell_id: 0,
                    attack_type: 0,
                });
            } else {
                // 1/4：FullmoonAttack 三连击 + DarkOmaKingNuke 法术场（Type=1）
                let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
                // Fullmoon 溅射：自身周围 AOE 三次（C# FullmoonAttack × 3，延迟 500/1700/2500ms）
                for _ in 0..3 {
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Aoe {
                        attacker_oid: monster.object_id,
                        center_x: monster.x,
                        center_y: monster.y,
                        radius: 2, // FullmoonAttack 溅射范围 1-2 格
                        damage,
                        spell_id: 0,
                    });
                }
                // 前方 3 格投放 DarkOmaKingNuke 法术场（C# DarkOmaKing.cs:114-132）
                let dir = direction_towards(monster.x, monster.y, target.x, target.y) as usize;
                let nuke_x = monster.x + DIR_DX[dir % 8] * 3;
                let nuke_y = monster.y + DIR_DY[dir % 8] * 3;
                ctx.out_spell_fields.push(crate::actors::world::ai::SpellFieldSpawn {
                    spell: Spell::DarkOmaKingNuke,
                    x: nuke_x,
                    y: nuke_y,
                    value: monster.max_dmg, // C# Value = Stats[Stat.MaxDC]
                    duration_ms: 900,
                    tick_ms: 1000,
                    caster_oid: monster.object_id,
                    caster_session: 0,
                });
                // Nuke 模式冷却更长（C# ActionTime + 3400）
                monster.next_attack_tick = ctx.tick_count + 34;
            }
        } else if dist <= 6 {
            // 远程（C# DarkOmaKing.cs:134-148）：1/3 概率弹道 MAC 攻击
            if ctx.tick_count < monster.next_attack_tick {
                return;
            }
            monster.next_attack_tick = ctx.tick_count + 5;
            if fastrand::i32(0..3) == 0 {
                let damage = crate::combat::attack::get_attack_power(monster.min_mac, monster.max_mac, monster.luck).max(1);
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    target_object_id: target.object_id,
                    damage,
                    spell_id: 0,
                });
            }
        } else if ctx.tick_count >= monster.next_move_tick {
            // 追击（C# 标准 MoveTo）
            let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + 2;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }

    fn on_die(&mut self, _monster: &mut MonsterState, _ctx: &mut AiCtx) {
        // C# Die：Kill SlaveList（PowerBead）。由调用方通过 is_slave=true 标记统一清理。
    }
}

//! OmaKing（奥玛之王）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/OmaKing.cs
//! 机制：
//!   - AttackRange=7，可移动追击
//!   - 近战（2 格内，对角判定 InAttackRange）：2/3 概率 LineAttack + 推开/麻痹小怪；
//!     1/3 或远距离 → 远程 MC 弹道攻击（Type=1）
//!   - 任务核心：周期召唤奥玛系小怪（原版无显式召唤，但作为奥玛王控制力补充，
//!     对齐任务"召唤奥玛系小怪"）

use crate::actors::world::MonsterState;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

/// 视野范围（C# 标准 ViewRange）
const VIEW_RANGE: i32 = 20;
/// 远程攻击距离（C# AttackRange = 7）
const ATTACK_RANGE: i32 = 7;
/// 近战判定（C# InAttackRange：x<=1 && y<=1）
const MELEE_RANGE: i32 = 1;
/// 召唤周期（约 25s）
const SUMMON_INTERVAL_TICKS: u64 = 250;
/// 召唤池（C# 奥玛系：OmaFighter/OmaSlasher/OmaWitcher/OmaAxeman 等）
const SLAVE_NAMES: [&str; 4] = [
    "OmaFighter",
    "OmaSlasher",
    "OmaWitcher",
    "OmaAxeman",
];

pub struct OmaKingBehavior {
    next_summon_tick: u64,
    spawned: bool,
}

impl OmaKingBehavior {
    pub fn new() -> Self {
        Self { next_summon_tick: 0, spawned: false }
    }
}

impl MonsterBehavior for OmaKingBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        if !self.spawned {
            self.next_summon_tick = ctx.tick_count + SUMMON_INTERVAL_TICKS;
            self.spawned = true;
        }

        // ---- 周期召唤奥玛小怪 ----
        if ctx.tick_count >= self.next_summon_tick {
            self.next_summon_tick = ctx.tick_count + SUMMON_INTERVAL_TICKS;
            for i in 0..3 {
                let dir = (i as usize) % 8;
                let name = SLAVE_NAMES[fastrand::usize(0..SLAVE_NAMES.len())];
                ctx.out_summons.push(crate::actors::world::ai::BossSummon {
                    monster_name: name.to_string(),
                    x: monster.x + DIR_DX[dir] * 2,
                    y: monster.y + DIR_DY[dir] * 2,
                    is_slave: true,
                });
            }
        }

        // 无目标则返回
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);

        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= ATTACK_RANGE && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let is_melee = dist <= MELEE_RANGE;
            if is_melee && fastrand::i32(0..3) > 0 {
                // 2/3：LineAttack（DC）—— C# LineAttack(damage, 2, 300)：沿朝向每格命中第一个目标
                let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
                let dir = direction_towards(monster.x, monster.y, target.x, target.y) as usize % 8;
                for i in 1..=2i32 {
                    let tx = monster.x + DIR_DX[dir] * i;
                    let ty = monster.y + DIR_DY[dir] * i;
                    if let Some(p) = ctx.players.iter()
                        .find(|p| p.map_index == monster.map_index && p.x == tx && p.y == ty && p.hp > 0)
                    {
                        ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                            attacker_oid: monster.object_id,
                            target_session: p.session_id,
                            damage,
                            spell_id: 0,
                            attack_type: 0,
                        });
                    }
                }
                // 概率麻痹（C# Random(8)==0 → Paralysis 5s）
                if fastrand::i32(0..8) == 0 {
                    ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                        session_id: target.session_id,
                        poison: Poison::new(PoisonType::PARALYSIS, 5, 0, 1000),
                    });
                }
                // C# OmaKing.cs:86 Pushed(..., DirectionFromPoint, 3 + Random(3))
                ctx.out_pushes.push(crate::actors::world::ai::PushPlayer {
                    session_id: target.session_id,
                    dir: crate::actors::world::ai::direction_towards(monster.x, monster.y, target.x, target.y),
                    distance: 3 + fastrand::i32(0..3),
                });
            } else {
                // 1/3 或远距离：远程 MC 弹道（C# Type=1，DefenceType.MAC）
                let damage = crate::combat::attack::get_attack_power(monster.min_mac, monster.max_mac, 0).max(1);
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    target_object_id: target.object_id,
                    damage,
                    spell_id: 0,
                });
            }
        } else if dist > ATTACK_RANGE && ctx.tick_count >= monster.next_move_tick {
            // 追击（C# MoveTo）
            let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + 2;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}

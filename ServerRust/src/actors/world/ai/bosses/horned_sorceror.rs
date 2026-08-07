//! HornedSorceror（角法师）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/HornedSorceror.cs
//! 机制：HornedCommander 的 Slave，HP<90% 解锁特殊技
//!   - AttackRange=5；HP<90% 时：1/4 概率 Charged Stomp（免疫 + 2 格 AOE，SC*loops），
//!     1/4 概率 Dust Tornado（自身 5x5 法术场）
//!   - 近战：Thrust（DC 直线 2 格）/ Dust（MC 直线 3 格）/ 突进 Thrust
//!   - 远程：1/3 突进 Thrust，否则 MoveTo
//!   - Charged Stomp 期间 _Immune=true（IsAttackTarget 返 false）
//!
//! Attack（C# :42-133）：ChargedStomp(HP<90,1/4)→免疫+AOE；Tornado(1/4)→法术场；否则近/远分支。
//! ProcessTarget（C# :135-152）：InRange→Attack；否则 MoveTo Target.Front。

use crate::actors::world::MonsterState;
use mir2_shared::enums::Spell;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const ATTACK_RANGE: i32 = 5;
const VIEW_RANGE: i32 = 15;
const MELEE_RANGE: i32 = 1;
/// Charged Stomp 冷却（C# _ChargedStompTime + 20000 = 200 ticks）
const STOMP_COOLDOWN_TICKS: u64 = 200;
/// Dust Tornado 冷却（C# _TornadoTime + 15000 = 150 ticks）
const TORNADO_COOLDOWN_TICKS: u64 = 150;

pub struct HornedSorcerorBehavior {
    /// 下次可 Charged Stomp tick
    stomp_tick: u64,
    /// 下次可 Dust Tornado tick
    tornado_tick: u64,
    /// 当前是否免疫（Charged Stomp 充能期）
    immune: bool,
}

impl HornedSorcerorBehavior {
    pub fn new() -> Self {
        Self { stomp_tick: 0, tornado_tick: 0, immune: false }
    }
}

impl MonsterBehavior for HornedSorcerorBehavior {
    /// C# IsAttackTarget：!_Immune && base
    fn is_attackable(&self) -> bool { !self.immune }
    fn on_attacked(&mut self, damage: i32) -> i32 {
        if self.immune { 0 } else { damage }
    }

    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);
        let hp_pct = if monster.max_hp > 0 { monster.hp * 100 / monster.max_hp } else { 100 };

        if dist <= ATTACK_RANGE && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;

            // Charged Stomp：HP<90 且冷却到 且 1/4 概率
            if hp_pct < 90 && ctx.tick_count >= self.stomp_tick && fastrand::i32(0..4) == 0 {
                self.stomp_tick = ctx.tick_count + STOMP_COOLDOWN_TICKS;
                let loops = fastrand::i32(5..10).max(1) as i32;
                let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1) * loops;
                // 充能期免疫（C# _Immune=true，CompleteAttack 末尾复位）
                self.immune = false; // 此 tick 即结算，无需持续免疫窗口
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Aoe {
                    attacker_oid: monster.object_id,
                    center_x: monster.x,
                    center_y: monster.y,
                    radius: 2,
                    damage,
                    spell_id: 0,
                });
                return;
            }

            // Dust Tornado：HP<90 且冷却到 且 1/4 概率
            if hp_pct < 90 && ctx.tick_count >= self.tornado_tick && fastrand::i32(0..4) == 0 {
                self.tornado_tick = ctx.tick_count + TORNADO_COOLDOWN_TICKS;
                let damage = crate::combat::attack::get_attack_power(monster.min_mac, monster.max_mac, 0).max(1);
                // 自身 5x5 法术场（C# location ±2 网格）
                ctx.out_spell_fields.push(crate::actors::world::ai::SpellFieldSpawn {
                    spell: Spell::HornedSorcererDustTornado,
                    x: monster.x,
                    y: monster.y,
                    value: damage,
                    duration_ms: 15000,
                    tick_ms: 1000,
                    caster_oid: monster.object_id,
                    caster_session: 0,
                });
                return;
            }

            if dist <= MELEE_RANGE {
                // 近战：3/5 Thrust(DC 直线2) / 2/5 Dust(MC 直线3)（C# LineAttack(damage, 2/3, 300)）
                let is_thrust = fastrand::i32(0..5) > 2;
                let (damage, distance) = if is_thrust {
                    (crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1), 2)
                } else {
                    (crate::combat::attack::get_attack_power(monster.min_mac, monster.max_mac, 0).max(1), 3)
                };
                let dir = direction_towards(monster.x, monster.y, target.x, target.y) as usize % 8;
                // 沿朝向直线每格命中第一个存活玩家
                for i in 1..=distance {
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
            } else {
                // 远程：1/3 突进（直接近战伤害），否则 MoveTo（走追击分支）
                if fastrand::i32(0..3) == 0 {
                    let damage = monster.max_dmg.max(1);
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        damage,
                        spell_id: 0,
                        attack_type: 0,
                    });
                }
            }
            return;
        }

        // 追击（MoveTo Target.Front：贴近 1 格）
        if ctx.tick_count >= monster.next_move_tick && dist > MELEE_RANGE {
            let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + 2;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}

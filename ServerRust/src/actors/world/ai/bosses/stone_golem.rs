//! StoneGolem（石头傀儡）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/StoneGolem.cs
//! 机制：
//!   - AttackRange=4
//!   - 近战 Type0 DC 单体（AC 防御）
//!   - 远程 Type1：朝向方向 3 格处投放 StoneGolemQuake 法术场（5x5 AOE）
//!
//! Attack（C# :28-97）：近战/远程分支；远程→前方 3 格 5x5 Quake 法术场。

use mir2_shared::enums::Spell;
use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 15;
const ATTACK_RANGE: i32 = 4;
const MELEE_RANGE: i32 = 1;
/// 法术场投放点距自身的格数（C# PointMove(CurrentLocation, Direction, 3)）
const QUAKE_OFFSET: i32 = 3;
/// 5x5 法术场半径（C# y-2..=y+2, x-2..=x+2）
const QUAKE_RADIUS: i32 = 2;

pub struct StoneGolemBehavior;

impl StoneGolemBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for StoneGolemBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= ATTACK_RANGE && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let melee = dist <= MELEE_RANGE;

            if melee {
                // Type0 DC 单体
                let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage,
                    spell_id: 0,
                    attack_type: 0,
                });
            } else {
                // Type1 前方 3 格处 5x5 Quake 法术场
                let dir = direction_towards(monster.x, monster.y, target.x, target.y);
                // 朝向方向移动 3 格的中心点
                let (mut center_x, mut center_y, _) = step_toward(monster.x, monster.y, target.x, target.y);
                // 再沿同方向推 2 格达到 offset=3（已走 1 格）
                for _ in 0..(QUAKE_OFFSET - 1) {
                    let (nx, ny, _) = step_toward(center_x, center_y, target.x, target.y);
                    center_x = nx;
                    center_y = ny;
                }
                let _ = dir; // 方向已隐含在 center 坐标里
                let value = crate::combat::attack::get_attack_power(monster.min_mac, monster.max_mac, 0).max(1);
                // 5x5 法术场：C# 每格一个 SpellObject（全 25 格）
                for oy in -QUAKE_RADIUS..=QUAKE_RADIUS {
                    for ox in -QUAKE_RADIUS..=QUAKE_RADIUS {
                        ctx.out_spell_fields.push(crate::actors::world::ai::SpellFieldSpawn {
                            spell: Spell::StoneGolemQuake,
                            x: center_x + ox,
                            y: center_y + oy,
                            value,
                            duration_ms: 800,
                            tick_ms: 1000,
                            caster_oid: monster.object_id,
                            caster_session: 0,
                        });
                    }
                }
            }
            return;
        }

        // 追击
        if dist > ATTACK_RANGE && ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + 2;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}

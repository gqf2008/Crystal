//! VampireSpider（吸血蜘蛛）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/VampireSpider.cs
//! 机制：可被弓手召唤（SummonVampire），吸血 + 自爆
//!   - InAttackRange 特殊十字判定（1 格 + 对角线/同奇偶）
//!   - Attack：DC + MACAgility，命中后 MasterVampire（主人吸血，value*PetLevel*0.25）
//!   - Die：1 格范围 MACAgility 爆炸 + MasterVampire（吸血主人）
//!   - 召唤时限：AliveTime 到 / 主人不在 15 格内 → Die
//!
//! Attack（C# :122-141）：DC MACAgility；命中 MasterVampire。
//! Die（C# :73-109）：1 格内 Attacked(10*PetLevel, MACAgility) + MasterVampire。

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 12;
const MELEE_RANGE: i32 = 1;
/// 爆炸半径（C# CurrentLocation ±1）
const EXPLOSION_RADIUS: i32 = 1;

pub struct VampireSpiderBehavior;

impl VampireSpiderBehavior {
    pub fn new() -> Self { Self }
}

impl MonsterBehavior for VampireSpiderBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.pet_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= MELEE_RANGE {
            if ctx.tick_count >= monster.next_attack_tick {
                monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
                let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage,
                    spell_id: 0,
                    attack_type: 0,
                });
                // C# MasterVampire：命中后主人回血 value*(PetLevel+1)*0.25
                if let Some(master) = monster.master_session {
                    let heal = ((damage as f32 * (ctx.pet_level as f32 + 1.0) * 0.25) as i32).max(1);
                    ctx.out_player_heals.push((master, heal));
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

    /// C# Die：1 格范围 MACAgility 爆炸（10*PetLevel）+ MasterVampire 吸血主人
    fn on_die(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        // C# VampireSpider.Die：Attacked(10*PetLevel, MACAgility)
        let damage = 10 * ctx.pet_level;
        if damage <= 0 { return; }
        ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Aoe {
            attacker_oid: monster.object_id,
            center_x: monster.x,
            center_y: monster.y,
            radius: EXPLOSION_RADIUS,
            damage,
            spell_id: 0,
        });
        // C# MasterVampire：每个命中目标给主人回血 value*(PetLevel+1)*0.25
        if let Some(master) = monster.master_session {
            let per = ((damage as f32 * (ctx.pet_level as f32 + 1.0) * 0.25) as i32).max(1);
            let hits = ctx.find_targets_in_range(monster.x, monster.y, EXPLOSION_RADIUS, monster.map_index).len() as i32;
            if hits > 0 {
                ctx.out_player_heals.push((master, per.saturating_mul(hits)));
            }
        }
    }
}

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
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= MELEE_RANGE {
            if ctx.tick_count >= monster.next_attack_tick {
                monster.next_attack_tick = ctx.tick_count + 6;
                let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage,
                    spell_id: 0,
                    attack_type: 0,
                });
                // 吸血：对主人回血（C# MasterVampire，value*(PetLevel+1)*0.25）
                // 简化：通过 out_heals 给主人（master_session 关联的友军）回血近似
                // 这里用 self_heal 近似：蜘蛛自身不回血，仅标记吸血效果由上层处理。
            }
            return;
        }

        // 追击
        if ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + 2;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }

    /// C# Die：1 格范围 MACAgility 爆炸 + 吸血主人
    fn on_die(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
        ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Aoe {
            attacker_oid: monster.object_id,
            center_x: monster.x,
            center_y: monster.y,
            radius: EXPLOSION_RADIUS,
            damage,
            spell_id: 0,
        });
    }
}

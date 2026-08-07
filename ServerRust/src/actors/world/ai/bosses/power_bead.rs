//! PowerBead（能量珠）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/PowerBead.cs
//! 机制：静态（CanMove=false）
//!   - Effect==0：视野内随机目标远程伤害（MACAgility）
//!   - Effect==1：净化友军毒（引擎暂无净化原语，暂不实现）
//!   - Effect==2：给最近友军加 PowerBeadBuff（MaxAC/MaxMAC=DC 近似 AcDefenseBoost+MacDefenseBoost）

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::combat::buff::{BuffInstance, BuffType};

const VIEW_RANGE: i32 = 12;

pub struct PowerBeadBehavior;

impl PowerBeadBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for PowerBeadBehavior {
    fn can_move(&self) -> bool {
        false
    }

    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        if ctx.tick_count < monster.next_attack_tick {
            return;
        }
        monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
        let nearby = ctx.find_targets_in_range(monster.x, monster.y, VIEW_RANGE, monster.map_index);
        let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
        match monster.effect {
            0 => {
                // C# Effect==0：随机目标远程伤害
                if let Some(p) = nearby.get(fastrand::usize(0..nearby.len())).copied() {
                    monster.target_session = Some(p.session_id);
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                        attacker_oid: monster.object_id,
                        target_session: p.session_id,
                        target_object_id: p.object_id,
                        damage,
                        spell_id: 0,
                    });
                }
            }
            2 => {
                // C# Effect==2：给友军（Owner/最近玩家）加 PowerBeadBuff（MaxAC/MaxMAC=DC 近似）
                let owned: Vec<(u64, i32)> = nearby.iter()
                    .map(|p| (p.session_id, max_distance(monster.x, monster.y, p.x, p.y)))
                    .collect();
                if let Some(&(sid, _)) = owned.iter().min_by_key(|(_, d)| *d) {
                    ctx.out_player_buffs.push((
                        sid,
                        BuffInstance::new(BuffType::AcDefenseBoost { bonus: damage }, 50, 10),
                    ));
                    ctx.out_player_buffs.push((
                        sid,
                        BuffInstance::new(BuffType::MacDefenseBoost { bonus: damage }, 50, 10),
                    ));
                }
            }
            _ => {
                // Effect==1：净化（暂不实现）
            }
        }
    }
}

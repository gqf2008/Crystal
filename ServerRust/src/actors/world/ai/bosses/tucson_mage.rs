//! TucsonMage（图森法师）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/TucsonMage.cs
//! 机制：
//!   - 3 格内对角/同奇偶判定（x%3==y%3）
//!   - 近战 2/3：Type0 DC 单体（ACAgility）
//!   - 1/3 或远程：Type1 WideLineAttack（MC）——前方 + 左右各 60° 三方向各 2 格扇形 AOE
//!
//! Attack（C# :28-59）：2/3 DC；else WideLine。
//! WideLineAttack（C# :61-119）：前方+PreviousDir 起三个方向各 2 格。

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 15;
const ATTACK_RANGE: i32 = 3;
const WIDE_RANGE: i32 = 3;

pub struct TucsonMageBehavior;

impl TucsonMageBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for TucsonMageBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= ATTACK_RANGE && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let melee = dist <= 1;

            if melee && fastrand::i32(0..3) > 0 {
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
                // Type1 WideLineAttack（MC）：前方 + 三个偏转方向各 2 格 AOE
                let main_dir = direction_towards(monster.x, monster.y, target.x, target.y);
                let damage = crate::combat::attack::get_attack_power(monster.min_mac, monster.max_mac, 0).max(1);
                // C# WideLineAttack：前方 1 格直击 + PreviousDir 起 3 个方向各 2 格。
                // 从怪物视角的扇形方向 = main±1（#1832：原 ±2 打错方向）
                let fan_dirs = tucson_mage_fan_dirs(main_dir);
                let hits: Vec<crate::actors::world::ai::PlayerSnap> =
                    ctx.find_targets_in_range(monster.x, monster.y, WIDE_RANGE, monster.map_index)
                        .into_iter().copied().collect();
                for h in hits {
                    let hd = direction_towards(monster.x, monster.y, h.x, h.y);
                    if !fan_dirs.contains(&hd) {
                        continue;
                    }
                    // C#：正前方只打 1 格（PointMove(CurrentLocation, Direction, 1)），
                    // 左右扇区打 forward 起 1-2 格（从怪物视角距离 2-3）
                    if hd == main_dir && max_distance(monster.x, monster.y, h.x, h.y) > 1 {
                        continue;
                    }
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                        attacker_oid: monster.object_id,
                        target_session: h.session_id,
                        damage,
                        spell_id: 0,
                        attack_type: 1,
                    });
                }
            }
            return;
        }

        // 追击
        if dist > ATTACK_RANGE && ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + monster.ai_profile.move_interval;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}

/// #1832：C# TucsonMage.WideLineAttack 扇形方向 = PreviousDir 起三个 NextDir
/// （prev/cur/next）；从怪物视角近似 = main-1 / main / main+1。
fn tucson_mage_fan_dirs(main: u8) -> [u8; 3] {
    [
        main,
        (main + 1) % 8, // NextDir
        (main + 7) % 8, // PreviousDir
    ]
}

#[cfg(test)]
mod tests {
    use super::tucson_mage_fan_dirs;

    #[test]
    fn test_fan_dirs_are_adjacent() {
        for main in 0..8u8 {
            let dirs = tucson_mage_fan_dirs(main);
            assert!(dirs.contains(&main));
            assert!(dirs.contains(&((main + 1) % 8)));
            assert!(dirs.contains(&((main + 7) % 8)));
            // 不应包含 main±2（#1832 旧错误）
            assert!(!dirs.contains(&((main + 2) % 8)));
            assert!(!dirs.contains(&((main + 6) % 8)));
        }
        // 具体值：main=Right(2) → {Right, DownRight, UpRight}
        assert_eq!(tucson_mage_fan_dirs(2), [2, 3, 1]);
        // main=Up(0) → {Up, UpRight, UpLeft}
        assert_eq!(tucson_mage_fan_dirs(0), [0, 1, 7]);
    }
}

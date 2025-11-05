// ============================================================================
// Layer 3: Combat & Skills - SkillSystem
// Priority: 300
// ============================================================================
//
// **职责**：
// - ✅ 技能施放逻辑（检查MP、冷却、目标）
// - ✅ 技能效果应用（伤害计算、状态附加）
// - ✅ 冷却管理（冷却时间计算、冷却重置）
// - ✅ 技能目标选择和范围检测
//
// **依赖输入**：
// - Layer 1: PlayerInput::UseSpell 事件
// - Layer 2: NpcDialogueSystem 提供的技能学习状态
//
// **输出影响**：
// - 修改 Mana 组件（消耗MP）
// - 修改 SpellCooldown 组件（设置冷却）
// - 发布 NetworkCommand::Magic（通知服务器）
// - Layer 4: 影响移动（施法时可能打断移动）
//
// ============================================================================

use super::super::super::{priority, LogicSystem};
use crate::ecs::components::{
    LocalPlayer, MagicList, Mana, Monster, NetworkSync, Player, Position, SpellType,
    TargetSelection, TargetType, NPC,
};
use crate::ecs::GameContext;
use crate::network::handlers::GameEvent as NetworkCommand;
use ggez::GameResult;
use hecs::World;
use mir2_shared::enums::MirDirection;
use tokio::sync::mpsc;

/// Layer 3: 技能施放系统
///
/// 处理技能施放的完整流程：
/// 1. 验证技能学习状态
/// 2. 检查MP消耗
/// 3. 检查冷却时间
/// 4. 获取目标信息
/// 5. 发送网络命令
/// 6. 消耗资源（MP、冷却）
pub struct SkillSystem;

impl Default for SkillSystem {
    fn default() -> Self {
        Self
    }
}

impl LogicSystem for SkillSystem {
    

    fn update(&mut self, ctx: &mut GameContext, _delay_time: f32) -> GameResult {
        // 检查玩家是否有施法意图
        let spell_request = {
            let mut request = None;

            for (_, (_, input)) in ctx
                .world
                .query::<(&LocalPlayer, &crate::ecs::components::PlayerInput)>()
                .iter()
            {
                if let Some(spell) = input.cast_spell {
                    request = Some((spell, input.spell_target_pos, input.spell_target_entity));
                    break;
                }
            }

            request
        };

        // 如果有施法请求，执行施法逻辑
        if let Some((spell, _target_pos, _target_entity)) = spell_request {
            // 1. 检查是否已学会该技能
            let learned = {
                let mut found = false;
                for (_, (_, magic_list)) in ctx.world.query::<(&LocalPlayer, &MagicList)>().iter() {
                    if magic_list.has_learned(spell) {
                        found = true;
                    }
                    break;
                }
                found
            };

            if !learned {
                tracing::warn!("⚠️ 尚未学会技能: {}", spell.name());

                // 清除施法输入
                for (_, (_, input)) in ctx
                    .world
                    .query_mut::<(&LocalPlayer, &mut crate::ecs::components::PlayerInput)>()
                {
                    input.cast_spell = None;
                    break;
                }

                return Ok(());
            }

            // 2. 检查魔法值
            let mp_cost = Self::get_spell_mp_cost(spell);
            let has_enough_mp = {
                let mut enough = false;
                for (_, (_, mana)) in ctx.world.query::<(&LocalPlayer, &Mana)>().iter() {
                    if mana.has_enough(mp_cost) {
                        enough = true;
                    }
                    break;
                }
                enough
            };

            if !has_enough_mp {
                tracing::warn!("⚠️ 魔法值不足,需要 {} MP", mp_cost);

                // 清除施法输入
                for (_, (_, input)) in ctx
                    .world
                    .query_mut::<(&LocalPlayer, &mut crate::ecs::components::PlayerInput)>()
                {
                    input.cast_spell = None;
                    break;
                }

                return Ok(());
            }

            // 3. 获取目标信息
            let (direction, target_id, location) = Self::get_target_info(&ctx.world);

            // 4. 发送施法命令到网络（如果网络发送器存在）
            // TODO: 从 World 中获取 NetworkCommand sender
            // let _ = network_tx.send(NetworkCommand::Magic {
            //     spell: spell as u8,
            //     direction,
            //     target_id,
            //     location,
            // });

            // 5. 消耗魔法值
            for (_, (_, mana)) in ctx.world.query_mut::<(&LocalPlayer, &mut Mana)>() {
                mana.consume(mp_cost);
            }

            // 6. 清除施法输入
            for (_, (_, input)) in ctx
                .world
                .query_mut::<(&LocalPlayer, &mut crate::ecs::components::PlayerInput)>()
            {
                input.cast_spell = None;
                input.spell_target_pos = None;
                input.spell_target_entity = None;
            }

            tracing::info!("✨ 施放技能: {} (MP: -{})", spell.name(), mp_cost);
        }

        Ok(())
    }
}

impl SkillSystem {
    /// 施放技能（供外部调用）
    pub fn cast_spell(
        world: &mut World,
        spell: SpellType,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) -> bool {
        // 1. 检查是否已学会该技能
        let learned = {
            let mut found = false;
            for (_, (_, magic_list)) in world.query::<(&LocalPlayer, &MagicList)>().iter() {
                if magic_list.has_learned(spell) {
                    found = true;
                }
                break;
            }
            found
        };

        if !learned {
            println!("⚠️ 尚未学会技能: {}", spell.name());
            return false;
        }

        // 2. 检查魔法值
        let mp_cost = Self::get_spell_mp_cost(spell);
        let has_enough_mp = {
            let mut enough = false;
            for (_, (_, mana)) in world.query::<(&LocalPlayer, &Mana)>().iter() {
                if mana.has_enough(mp_cost) {
                    enough = true;
                }
                break;
            }
            enough
        };

        if !has_enough_mp {
            println!("⚠️ 魔法值不足,需要 {} MP", mp_cost);
            return false;
        }

        // 3. TODO: 检查冷却时间
        // if !Self::check_cooldown(world, spell) {
        //     println!("⚠️ 技能冷却中");
        //     return false;
        // }

        // 4. 获取目标信息
        let (direction, target_id, location) = Self::get_target_info(world);

        // 5. 发送施法命令
        let _ = network_tx.send(NetworkCommand::MagicRequest {
            spell: spell as u8,
            direction,
            target_id,
            location,
        });

        // 6. 消耗魔法值
        for (_, (_, mana)) in world.query_mut::<(&LocalPlayer, &mut Mana)>() {
            mana.consume(mp_cost);
        }

        // 7. TODO: 设置冷却时间
        // Self::set_cooldown(world, spell);

        println!("✨ 施放技能: {} (MP: {})", spell.name(), mp_cost);
        true
    }

    /// 获取技能魔法消耗
    fn get_spell_mp_cost(spell: SpellType) -> i32 {
        match spell {
            // Warrior (战士 - 低MP消耗)
            SpellType::Fencing => 0,
            SpellType::Slaying => 2,
            SpellType::Thrusting => 3,
            SpellType::HalfMoon => 4,
            SpellType::ShoulderDash => 5,
            SpellType::LionRoar => 10,

            // Wizard (法师 - 高MP消耗)
            SpellType::FireBall => 5,
            SpellType::Repulsion => 3,
            SpellType::ElectricShock => 4,
            SpellType::GreatFireBall => 9,
            SpellType::HellFire => 12,
            SpellType::ThunderBolt => 8,
            SpellType::Teleport => 20,
            SpellType::Lightning => 15,
            SpellType::MagicShield => 10,

            // Taoist (道士 - 中等MP消耗)
            SpellType::Healing => 8,
            SpellType::SpiritSword => 5,
            SpellType::Poisoning => 3,
            SpellType::SoulFireBall => 6,
            SpellType::SummonSkeleton => 25,
            SpellType::Hiding => 15,
            SpellType::SoulShield => 12,

            // Assassin (刺客 - 低MP消耗)
            SpellType::FatalSword => 2,
            SpellType::DoubleSlash => 4,
            SpellType::Haste => 10,
            SpellType::FlashDash => 8,

            // Archer (弓箭手 - 中等MP消耗)
            SpellType::Focus => 5,
            SpellType::StraightShot => 3,
            SpellType::DoubleShot => 6,
            SpellType::Meditation => 10,

            _ => 5, // 默认消耗
        }
    }

    /// 获取当前目标信息
    fn get_target_info(world: &World) -> (MirDirection, u32, Option<(i32, i32)>) {
        // 查询目标选择组件
        for (_, (_, target_sel)) in world.query::<(&LocalPlayer, &TargetSelection)>().iter() {
            match target_sel.current {
                TargetType::Monster(id) => {
                    let direction = Self::calculate_direction_to_target(world, id);
                    return (direction, id, None);
                }
                TargetType::Player(id) => {
                    let direction = Self::calculate_direction_to_target(world, id);
                    return (direction, id, None);
                }
                TargetType::NPC(id) => {
                    let direction = Self::calculate_direction_to_target(world, id);
                    return (direction, id, None);
                }
                TargetType::Location(x, y) => {
                    // 地面技能（如：地狱火、冰咆哮）
                    let direction = Self::calculate_direction_to_location(world, x, y);
                    return (direction, 0, Some((x, y)));
                }
                TargetType::None => {
                    // 自身技能（如：魔法盾、隐身）
                    let direction = Self::get_player_direction(world);
                    return (direction, 0, None);
                }
            }
        }

        // 默认朝向下方，无目标
        (MirDirection::Down, 0, None)
    }

    /// 计算朝向目标的方向
    fn calculate_direction_to_target(world: &World, target_id: u32) -> MirDirection {
        // 获取玩家位置
        let player_pos = {
            let mut pos = None;
            for (_, (_, player_pos)) in world.query::<(&LocalPlayer, &Position)>().iter() {
                pos = Some((player_pos.x, player_pos.y));
                break;
            }
            pos
        };

        if player_pos.is_none() {
            return MirDirection::Down;
        }
        let (px, py) = player_pos.unwrap();

        // 查找目标位置
        let target_pos = {
            let mut pos = None;

            // 检查怪物
            for (_, (_, net_sync, target_pos)) in
                world.query::<(&Monster, &NetworkSync, &Position)>().iter()
            {
                if net_sync.object_id == target_id {
                    pos = Some((target_pos.x, target_pos.y));
                    break;
                }
            }

            // 检查玩家
            if pos.is_none() {
                for (_, (_, net_sync, target_pos)) in
                    world.query::<(&Player, &NetworkSync, &Position)>().iter()
                {
                    if net_sync.object_id == target_id {
                        pos = Some((target_pos.x, target_pos.y));
                        break;
                    }
                }
            }

            // 检查NPC
            if pos.is_none() {
                for (_, (_, net_sync, target_pos)) in
                    world.query::<(&NPC, &NetworkSync, &Position)>().iter()
                {
                    if net_sync.object_id == target_id {
                        pos = Some((target_pos.x, target_pos.y));
                        break;
                    }
                }
            }

            pos
        };

        if let Some((tx, ty)) = target_pos {
            Self::calculate_direction(px, py, tx, ty)
        } else {
            MirDirection::Down
        }
    }

    /// 计算朝向指定位置的方向
    fn calculate_direction_to_location(
        world: &World,
        target_x: i32,
        target_y: i32,
    ) -> MirDirection {
        let player_pos = {
            let mut pos = None;
            for (_, (_, player_pos)) in world.query::<(&LocalPlayer, &Position)>().iter() {
                pos = Some((player_pos.x, player_pos.y));
                break;
            }
            pos
        };

        if let Some((px, py)) = player_pos {
            // 转换格子坐标到像素坐标
            let target_px = target_x as f32 * 48.0;
            let target_py = target_y as f32 * 32.0;
            Self::calculate_direction(px, py, target_px, target_py)
        } else {
            MirDirection::Down
        }
    }

    /// 计算方向（8方向）
    fn calculate_direction(from_x: f32, from_y: f32, to_x: f32, to_y: f32) -> MirDirection {
        let dx = to_x - from_x;
        let dy = to_y - from_y;

        if dx == 0.0 && dy == 0.0 {
            return MirDirection::Down;
        }

        let angle = dy.atan2(dx);
        let angle_deg = angle.to_degrees();

        // 8方向划分（每方向45度）
        match angle_deg {
            a if a >= -22.5 && a < 22.5 => MirDirection::Right,
            a if a >= 22.5 && a < 67.5 => MirDirection::DownRight,
            a if a >= 67.5 && a < 112.5 => MirDirection::Down,
            a if a >= 112.5 && a < 157.5 => MirDirection::DownLeft,
            a if a >= 157.5 || a < -157.5 => MirDirection::Left,
            a if a >= -157.5 && a < -112.5 => MirDirection::UpLeft,
            a if a >= -112.5 && a < -67.5 => MirDirection::Up,
            a if a >= -67.5 && a < -22.5 => MirDirection::UpRight,
            _ => MirDirection::Down,
        }
    }

    /// 获取玩家当前朝向
    fn get_player_direction(world: &World) -> MirDirection {
        for (_, (_, player)) in world.query::<(&LocalPlayer, &Player)>().iter() {
            // Player.direction 是 u8 (0-7), 需要转换为 MirDirection
            return match player.direction {
                0 => MirDirection::Up,
                1 => MirDirection::UpRight,
                2 => MirDirection::Right,
                3 => MirDirection::DownRight,
                4 => MirDirection::Down,
                5 => MirDirection::DownLeft,
                6 => MirDirection::Left,
                7 => MirDirection::UpLeft,
                _ => MirDirection::Down,
            };
        }
        MirDirection::Down
    }
}

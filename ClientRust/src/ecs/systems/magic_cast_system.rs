// ============================================================================
// 技能施放系统 - 处理技能施放逻辑
// ============================================================================

use hecs::{World, Entity};
use crate::ecs::components::{
    SpellType, MagicList, Mana, LocalPlayer, TargetSelection, TargetType,
    Position, Monster, Player, NPC, NetworkSync, Camera
};
use crate::network::NetworkCommand;
use tokio::sync::mpsc;
use mir2_shared::enums::MirDirection;

/// 技能施放系统
pub struct MagicCastSystem;

impl MagicCastSystem {
    pub fn new() -> Self {
        Self
    }
    
    /// 施放技能
    pub fn cast_spell(
        world: &mut World,
        spell: SpellType,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) -> bool {
        // 检查是否已学会该技能
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
        
        // 检查魔法值
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
        
        // 获取目标信息
        let (direction, target_id, location) = Self::get_target_info(world);
        
        // 发送施法命令
        let _ = network_tx.send(NetworkCommand::Magic {
            spell: spell as u8,
            direction,
            target_id,
            location,
        });
        
        // 消耗魔法值
        for (_, (_, mana)) in world.query_mut::<(&LocalPlayer, &mut Mana)>() {
            mana.consume(mp_cost);
        }
        
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
                    // 计算朝向怪物的方向
                    let direction = Self::calculate_direction_to_target(world, id);
                    return (direction, id, None);
                },
                TargetType::Player(id) => {
                    let direction = Self::calculate_direction_to_target(world, id);
                    return (direction, id, None);
                },
                TargetType::NPC(id) => {
                    // NPC目标 (例如治疗NPC)
                    let direction = Self::calculate_direction_to_target(world, id);
                    return (direction, id, None);
                },
                TargetType::Location(x, y) => {
                    // 地面技能
                    let direction = Self::calculate_direction_to_location(world, x, y);
                    return (direction, 0, Some((x, y)));
                },
                TargetType::None => {
                    // 无目标,使用玩家当前朝向
                    let direction = Self::get_player_direction(world);
                    return (direction, 0, None);
                }
            }
        }
        
        // 默认朝下
        (MirDirection::Down, 0, None)
    }
    
    /// 计算朝向目标的方向
    fn calculate_direction_to_target(world: &World, target_id: u32) -> MirDirection {
        use crate::ecs::components::NetworkSync;
        
        // 获取玩家位置
        let player_pos = {
            let mut pos = Position::new(0.0, 0.0);
            for (_, (_, p)) in world.query::<(&LocalPlayer, &Position)>().iter() {
                pos = *p;
                break;
            }
            pos
        };
        
        // 查找目标位置 (通过 NetworkSync.object_id 匹配)
        let target_pos = {
            let mut pos = player_pos; // 默认使用玩家位置
            
            // 先尝试查找怪物
            for (_, (_, p, net_sync)) in world.query::<(&Monster, &Position, &NetworkSync)>().iter() {
                if net_sync.object_id == target_id {
                    pos = *p;
                    break;
                }
            }
            
            // TODO: 如果需要,也可以查找其他玩家和NPC
            // for (_, (_, p, net_sync)) in world.query::<(&PlayerMarker, &Position, &NetworkSync)>().iter() {
            //     if net_sync.object_id == target_id { pos = *p; break; }
            // }
            
            pos
        };
        
        Self::calculate_direction(player_pos, target_pos)
    }
    
    /// 计算朝向位置的方向
    fn calculate_direction_to_location(world: &World, x: i32, y: i32) -> MirDirection {
        let player_pos = {
            let mut pos = Position::new(0.0, 0.0);
            for (_, (_, p)) in world.query::<(&LocalPlayer, &Position)>().iter() {
                pos = *p;
                break;
            }
            pos
        };
        
        let target_pos = Position::from_grid(x, y);
        Self::calculate_direction(player_pos, target_pos)
    }
    
    /// 计算两点之间的方向
    fn calculate_direction(from: Position, to: Position) -> MirDirection {
        let dx = to.x - from.x;
        let dy = to.y - from.y;
        
        // 使用8方向判断
        if dx.abs() > dy.abs() * 2.0 {
            // 主要是水平方向
            if dx > 0.0 { MirDirection::Right } else { MirDirection::Left }
        } else if dy.abs() > dx.abs() * 2.0 {
            // 主要是垂直方向
            if dy > 0.0 { MirDirection::Down } else { MirDirection::Up }
        } else {
            // 对角线方向
            match (dx > 0.0, dy > 0.0) {
                (true, true) => MirDirection::DownRight,
                (true, false) => MirDirection::UpRight,
                (false, true) => MirDirection::DownLeft,
                (false, false) => MirDirection::UpLeft,
            }
        }
    }
    
    /// 获取玩家当前朝向
    fn get_player_direction(world: &World) -> MirDirection {
        // 从 Player 组件获取方向
        for (_, (_, player)) in world.query::<(&LocalPlayer, &Player)>().iter() {
            // Player.direction 是 u8, 需要转换为 MirDirection
            return MirDirection::try_from(player.direction as u8)
                .unwrap_or(MirDirection::Down);
        }
        
        // 默认朝下
        MirDirection::Down
    }
    
    /// 选择目标 (点击选择)
    pub fn select_target_at_position(
        world: &mut World,
        screen_x: f32,
        screen_y: f32,
    ) -> bool {
        // 1. 获取相机位置,转换屏幕坐标到世界坐标
        let world_pos = {
            let mut wx = screen_x;
            let mut wy = screen_y;
            
            // 相机位置存储在 Position 组件中
            for (_, (_, pos)) in world.query::<(&Camera, &Position)>().iter() {
                wx = screen_x + pos.x;
                wy = screen_y + pos.y;
                break;
            }
            (wx, wy)
        };
        
        // 2. 查找该位置附近的怪物 (48x32格子范围)
        let click_tolerance = 48.0; // 点击容差
        let mut nearest_monster: Option<(u32, f32)> = None; // (object_id, distance)
        
        for (_, (monster, pos, net_sync)) in world.query::<(&Monster, &Position, &NetworkSync)>().iter() {
            let dx = pos.x - world_pos.0;
            let dy = pos.y - world_pos.1;
            let distance = (dx * dx + dy * dy).sqrt();
            
            if distance < click_tolerance {
                if let Some((_, min_dist)) = nearest_monster {
                    if distance < min_dist {
                        nearest_monster = Some((net_sync.object_id, distance));
                    }
                } else {
                    nearest_monster = Some((net_sync.object_id, distance));
                }
            }
        }
        
        // 3. 更新 TargetSelection
        if let Some((object_id, _)) = nearest_monster {
            for (_, (_, target_sel)) in world.query_mut::<(&LocalPlayer, &mut TargetSelection)>() {
                target_sel.select_monster(object_id);
                println!("🎯 选中怪物: ID={}", object_id);
                return true;
            }
        }
        
        println!("❌ 未找到目标");
        false
    }
    
    /// 切换目标 (Tab键)
    pub fn cycle_target(world: &mut World) -> bool {
        use crate::ecs::components::{Monster, NetworkSync};
        
        // 1. 获取玩家位置
        let player_pos = {
            let mut pos = Position::new(0.0, 0.0);
            for (_, (_, p)) in world.query::<(&LocalPlayer, &Position)>().iter() {
                pos = *p;
                break;
            }
            pos
        };
        
        // 2. 获取当前目标ID
        let current_target_id = {
            let mut id = None;
            for (_, (_, target_sel)) in world.query::<(&LocalPlayer, &TargetSelection)>().iter() {
                if let Some(monster_id) = target_sel.get_monster_id() {
                    id = Some(monster_id);
                }
                break;
            }
            id
        };
        
        // 3. 收集附近的怪物并按距离排序
        let mut monsters: Vec<(u32, f32)> = Vec::new(); // (object_id, distance)
        
        for (_, (_, pos, net_sync)) in world.query::<(&Monster, &Position, &NetworkSync)>().iter() {
            let dx = pos.x - player_pos.x;
            let dy = pos.y - player_pos.y;
            let distance = (dx * dx + dy * dy).sqrt();
            
            // 只选择屏幕范围内的怪物 (20格以内)
            if distance < 960.0 { // 20 * 48
                monsters.push((net_sync.object_id, distance));
            }
        }
        
        if monsters.is_empty() {
            println!("⚠️ 附近没有怪物");
            return false;
        }
        
        // 按距离排序
        monsters.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        
        // 4. 选择下一个目标
        let next_target_id = if let Some(current_id) = current_target_id {
            // 找到当前目标的位置,选择下一个
            let current_idx = monsters.iter().position(|(id, _)| *id == current_id);
            if let Some(idx) = current_idx {
                let next_idx = (idx + 1) % monsters.len();
                monsters[next_idx].0
            } else {
                // 当前目标不在列表中,选择最近的
                monsters[0].0
            }
        } else {
            // 没有当前目标,选择最近的
            monsters[0].0
        };
        
        // 5. 更新目标选择
        for (_, (_, target_sel)) in world.query_mut::<(&LocalPlayer, &mut TargetSelection)>() {
            target_sel.select_monster(next_target_id);
            println!("🔄 切换目标: ID={}", next_target_id);
            return true;
        }
        
        false
    }
    
    /// 清除当前目标
    pub fn clear_target(world: &mut World) {
        for (_, (_, target_sel)) in world.query_mut::<(&LocalPlayer, &mut TargetSelection)>() {
            target_sel.clear();
            println!("❌ 清除目标");
        }
    }
    
    /// 检查技能是否需要目标
    pub fn spell_requires_target(spell: SpellType) -> bool {
        match spell {
            // 需要目标的技能
            SpellType::Healing |
            SpellType::Poisoning |
            SpellType::SoulFireBall |
            SpellType::FireBall |
            SpellType::GreatFireBall |
            SpellType::ThunderBolt |
            SpellType::Lightning => true,
            
            // 不需要目标的技能 (自身BUFF或范围技能)
            SpellType::MagicShield |
            SpellType::Teleport |
            SpellType::Hiding |
            SpellType::SoulShield |
            SpellType::Haste |
            SpellType::Meditation => false,
            
            _ => false,
        }
    }
    
    /// 检查技能是否是地面技能
    pub fn spell_is_ground_target(spell: SpellType) -> bool {
        matches!(
            spell,
            SpellType::FireWall |
            SpellType::Teleport |
            SpellType::TrapHexagon |
            SpellType::Trap |
            SpellType::ExplosiveTrap
        )
    }
}

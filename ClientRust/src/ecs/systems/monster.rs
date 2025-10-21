// ============================================================================
// Monster System - 怪物AI和行为系统
// ============================================================================
//
// 功能：
// - 怪物AI更新（追击、攻击、巡逻等）
// - 怪物移动逻辑
// - 怪物动画更新
// - 怪物与玩家交互
//
// 对应 C# 版本：
// - MonsterObject.Process()
// - MonsterObject.ProcessAI()
// - MonsterObject.ProcessMovement()
//
// ============================================================================

use hecs::World;
use crate::ecs::components::{
    Position, MonsterComp, AnimationComp, Health, AIState, Velocity,
    Player, MirAction,
};

/// 怪物系统 - 处理所有怪物的AI和行为
pub struct MonsterSystem;

impl MonsterSystem {
    /// 更新所有怪物
    pub fn update(world: &mut World, delta_time: f32) {
        // 1. 更新怪物AI
        Self::update_ai(world);
        
        // 2. 更新怪物移动
        Self::update_movement(world, delta_time);
        
        // 3. 更新怪物动画（已由 AnimationSystem 统一处理）
    }
    
    /// 更新怪物AI逻辑
    fn update_ai(world: &mut World) {
        // 首先查找玩家位置
        let player_pos = Self::find_player_position(world);
        
        // 遍历所有怪物
        for (_entity, (monster, pos, ai_state, health)) in 
            world.query::<(&MonsterComp, &mut Position, &mut AIState, &Health)>().iter() 
        {
            // 跳过死亡怪物
            if health.current <= 0 {
                continue;
            }
            
            // 根据AI类型执行不同逻辑
            match monster.ai_type {
                0 => {
                    // AI = 0: 无AI，静止不动
                    ai_state.current_action = crate::ecs::components::AIAction::Idle;
                }
                1 => {
                    // AI = 1: 近战攻击型（最常见）
                    Self::ai_melee_attack(pos, ai_state, player_pos);
                }
                2 => {
                    // AI = 2: 远程攻击型
                    Self::ai_ranged_attack(pos, ai_state, player_pos);
                }
                3 => {
                    // AI = 3: 巡逻型
                    Self::ai_patrol(pos, ai_state, monster);
                }
                _ => {
                    // 其他AI类型暂未实现
                }
            }
        }
    }
    
    /// 近战攻击AI
    fn ai_melee_attack(
        pos: &mut Position,
        ai_state: &mut AIState,
        player_pos: Option<(f32, f32)>
    ) {
        if let Some((px, py)) = player_pos {
            let distance = Self::distance((pos.x, pos.y), (px, py));
            
            if distance < 1.5 {
                // 在攻击范围内 - 攻击
                ai_state.current_action = crate::ecs::components::AIAction::Attack;
                ai_state.target_pos = Some((px, py));
            } else if distance < 10.0 {
                // 在视野范围内 - 追击
                ai_state.current_action = crate::ecs::components::AIAction::Chase;
                ai_state.target_pos = Some((px, py));
            } else {
                // 超出视野 - 闲置
                ai_state.current_action = crate::ecs::components::AIAction::Idle;
                ai_state.target_pos = None;
            }
        } else {
            // 找不到玩家 - 闲置
            ai_state.current_action = crate::ecs::components::AIAction::Idle;
            ai_state.target_pos = None;
        }
    }
    
    /// 远程攻击AI
    fn ai_ranged_attack(
        pos: &mut Position,
        ai_state: &mut AIState,
        player_pos: Option<(f32, f32)>
    ) {
        if let Some((px, py)) = player_pos {
            let distance = Self::distance((pos.x, pos.y), (px, py));
            
            if distance >= 3.0 && distance < 8.0 {
                // 在最佳攻击范围内 - 攻击
                ai_state.current_action = crate::ecs::components::AIAction::Attack;
                ai_state.target_pos = Some((px, py));
            } else if distance < 3.0 {
                // 太近了 - 后退
                ai_state.current_action = crate::ecs::components::AIAction::Retreat;
                ai_state.target_pos = Some((px, py));
            } else if distance < 12.0 {
                // 太远了 - 追击
                ai_state.current_action = crate::ecs::components::AIAction::Chase;
                ai_state.target_pos = Some((px, py));
            } else {
                // 超出视野 - 闲置
                ai_state.current_action = crate::ecs::components::AIAction::Idle;
                ai_state.target_pos = None;
            }
        } else {
            ai_state.current_action = crate::ecs::components::AIAction::Idle;
            ai_state.target_pos = None;
        }
    }
    
    /// 巡逻AI
    fn ai_patrol(
        pos: &mut Position,
        ai_state: &mut AIState,
        monster: &MonsterComp,
    ) {
        // 如果没有巡逻点，随机选择一个
        if ai_state.patrol_points.is_empty() {
            // 在出生点周围生成4个巡逻点
            let spawn_x = monster.spawn_x;
            let spawn_y = monster.spawn_y;
            
            ai_state.patrol_points = vec![
                (spawn_x + 5.0, spawn_y),
                (spawn_x, spawn_y + 5.0),
                (spawn_x - 5.0, spawn_y),
                (spawn_x, spawn_y - 5.0),
            ];
            ai_state.current_patrol_index = 0;
        }
        
        // 获取当前目标巡逻点
        if let Some(&target) = ai_state.patrol_points.get(ai_state.current_patrol_index) {
            let distance = Self::distance((pos.x, pos.y), target);
            
            if distance < 0.5 {
                // 到达巡逻点，切换到下一个
                ai_state.current_patrol_index = (ai_state.current_patrol_index + 1) % ai_state.patrol_points.len();
                ai_state.current_action = crate::ecs::components::AIAction::Idle;
            } else {
                // 继续前往巡逻点
                ai_state.current_action = crate::ecs::components::AIAction::Patrol;
                ai_state.target_pos = Some(target);
            }
        }
    }
    
    /// 更新怪物移动
    fn update_movement(world: &mut World, delta_time: f32) {
        for (_entity, (pos, ai_state, anim, velocity)) in 
            world.query::<(&mut Position, &AIState, &mut AnimationComp, Option<&mut Velocity>)>().iter() 
        {
            match ai_state.current_action {
                crate::ecs::components::AIAction::Chase | 
                crate::ecs::components::AIAction::Patrol => {
                    if let Some(target) = ai_state.target_pos {
                        // 计算移动方向
                        let dx = target.0 - pos.x;
                        let dy = target.1 - pos.y;
                        let distance = (dx * dx + dy * dy).sqrt();
                        
                        if distance > 0.1 {
                            // 归一化方向向量
                            let move_speed = 2.0; // 格子/秒
                            let vx = (dx / distance) * move_speed;
                            let vy = (dy / distance) * move_speed;
                            
                            // 更新位置
                            pos.x += vx * delta_time;
                            pos.y += vy * delta_time;
                            
                            // 更新动画为行走
                            if anim.action != MirAction::Walking {
                                anim.action = MirAction::Walking;
                                anim.frame_index = 0;
                            }
                            
                            // 更新速度组件（如果有）
                            if let Some(vel) = velocity {
                                vel.dx = vx;
                                vel.dy = vy;
                            }
                            
                            // 更新朝向
                            Self::update_direction_from_movement(anim, vx, vy);
                        }
                    }
                }
                crate::ecs::components::AIAction::Retreat => {
                    if let Some(target) = ai_state.target_pos {
                        // 远离目标
                        let dx = pos.x - target.0;
                        let dy = pos.y - target.1;
                        let distance = (dx * dx + dy * dy).sqrt();
                        
                        if distance > 0.1 {
                            let move_speed = 1.5;
                            let vx = (dx / distance) * move_speed;
                            let vy = (dy / distance) * move_speed;
                            
                            pos.x += vx * delta_time;
                            pos.y += vy * delta_time;
                            
                            if anim.action != MirAction::Walking {
                                anim.action = MirAction::Walking;
                                anim.frame_index = 0;
                            }
                            
                            Self::update_direction_from_movement(anim, vx, vy);
                        }
                    }
                }
                crate::ecs::components::AIAction::Attack => {
                    // 攻击动画
                    if anim.action != MirAction::Attack1 {
                        anim.action = MirAction::Attack1;
                        anim.frame_index = 0;
                    }
                }
                crate::ecs::components::AIAction::Idle => {
                    // 站立动画
                    if anim.action != MirAction::Standing {
                        anim.action = MirAction::Standing;
                        anim.frame_index = 0;
                    }
                }
            }
        }
    }
    
    /// 根据移动方向更新朝向
    fn update_direction_from_movement(anim: &mut AnimationComp, vx: f32, vy: f32) {
        // 计算8方向
        let angle = vy.atan2(vx).to_degrees();
        let direction = ((angle + 22.5) / 45.0).floor() as i32;
        
        // 转换为0-7的方向值
        let dir = ((direction + 8) % 8) as u8;
        
        // 映射到传奇的8方向
        // 0=右, 1=右下, 2=下, 3=左下, 4=左, 5=左上, 6=上, 7=右上
        anim.direction = match dir {
            0 => 0, // 右
            1 => 1, // 右下
            2 => 2, // 下
            3 => 3, // 左下
            4 => 4, // 左
            5 => 5, // 左上
            6 => 6, // 上
            7 => 7, // 右上
            _ => 0,
        };
    }
    
    /// 查找玩家位置
    fn find_player_position(world: &World) -> Option<(f32, f32)> {
        for (_entity, (_, pos)) in world.query::<(&Player, &Position)>().iter() {
            return Some((pos.x, pos.y));
        }
        None
    }
    
    /// 计算两点之间的距离
    fn distance(a: (f32, f32), b: (f32, f32)) -> f32 {
        let dx = b.0 - a.0;
        let dy = b.1 - a.1;
        (dx * dx + dy * dy).sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_distance_calculation() {
        let dist = MonsterSystem::distance((0.0, 0.0), (3.0, 4.0));
        assert!((dist - 5.0).abs() < 0.01);
    }
    
    #[test]
    fn test_ai_melee_in_range() {
        let mut pos = Position { x: 10.0, y: 10.0 };
        let mut ai_state = AIState::default();
        let player_pos = Some((11.0, 10.0)); // 距离 = 1.0
        
        MonsterSystem::ai_melee_attack(&mut pos, &mut ai_state, player_pos);
        
        assert!(matches!(ai_state.current_action, crate::ecs::components::AIAction::Attack));
    }
    
    #[test]
    fn test_ai_melee_chase_range() {
        let mut pos = Position { x: 10.0, y: 10.0 };
        let mut ai_state = AIState::default();
        let player_pos = Some((15.0, 10.0)); // 距离 = 5.0
        
        MonsterSystem::ai_melee_attack(&mut pos, &mut ai_state, player_pos);
        
        assert!(matches!(ai_state.current_action, crate::ecs::components::AIAction::Chase));
    }
}

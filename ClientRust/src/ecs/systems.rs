// ============================================================================
// Systems - ECS 系统 (游戏逻辑处理)
// ============================================================================

use hecs::World;
use std::time::Duration;

use super::components::*;
use crate::graphics::libraries::get_map_library;

// ============================================================================
// MovementSystem - 移动系统
// ============================================================================

pub struct MovementSystem;

impl MovementSystem {
    /// 更新所有实体的移动
    pub fn update(world: &mut World, delta: Duration) {
        let delta_secs = delta.as_secs_f32();

        for (_, (pos, vel)) in world.query_mut::<(&mut Position, &Velocity)>() {
            // 更新偏移量 (用于平滑移动插值)
            pos.offset_x += (vel.dx * delta_secs) as i32;
            pos.offset_y += (vel.dy * delta_secs) as i32;

            // 当偏移量达到一个格子时,更新格子坐标
            const CELL_WIDTH: i32 = 48;
            const CELL_HEIGHT: i32 = 32;

            if pos.offset_x.abs() >= CELL_WIDTH {
                pos.x += pos.offset_x.signum();
                pos.offset_x -= pos.offset_x.signum() * CELL_WIDTH;
            }

            if pos.offset_y.abs() >= CELL_HEIGHT {
                pos.y += pos.offset_y.signum();
                pos.offset_y -= pos.offset_y.signum() * CELL_HEIGHT;
            }
        }
    }

    /// 移动实体到指定位置 (立即移动,不使用插值)
    pub fn teleport(world: &mut World, entity: hecs::Entity, x: i32, y: i32) {
        if let Ok(mut pos) = world.get::<&mut Position>(entity) {
            pos.x = x;
            pos.y = y;
            pos.offset_x = 0;
            pos.offset_y = 0;
        }
    }
}

// ============================================================================
// AnimationSystem - 动画系统
// ============================================================================

pub struct AnimationSystem;

impl AnimationSystem {
    /// 更新所有动画
    pub fn update(world: &mut World, delta: Duration) {
        let delta_ms = delta.as_millis() as u32;

        for (entity, (anim, sprite)) in world.query_mut::<(&mut AnimationComp, &mut SpriteComp)>() {
            let finished = anim.update(delta_ms);
            
            // 更新精灵帧
            sprite.frame = anim.frame_index as i32;

            // 如果动画播放完成且不循环,可以在这里处理
            if finished && !anim.loop_animation {
                // 例如: 技能特效播放完毕后移除实体
                // 这里只是标记,实际移除由 cleanup 系统处理
            }
        }
    }
}

// ============================================================================
// LifetimeSystem - 生命周期系统
// ============================================================================

pub struct LifetimeSystem;

impl LifetimeSystem {
    /// 更新所有生命周期
    pub fn update(world: &mut World, delta: Duration) {
        let delta_ms = delta.as_millis() as u32;

        for (_, lifetime) in world.query_mut::<&mut Lifetime>() {
            lifetime.update(delta_ms);
        }
    }
}

// ============================================================================
// AISystem - AI 系统
// ============================================================================

pub struct AISystem;

impl AISystem {
    /// 更新怪物 AI
    pub fn update(world: &mut World, _delta: Duration) {
        // 简单的 AI 示例
        for (entity, (ai, pos, monster)) in world.query_mut::<(&mut AIState, &Position, &MonsterComp)>() {
            match ai.mode {
                AIMode::Idle => {
                    // 待机状态: 随机巡逻
                    // TODO: 实现巡逻逻辑
                }
                AIMode::Patrol => {
                    // 巡逻状态
                    // TODO: 实现巡逻路径
                }
                AIMode::Chase => {
                    // 追击状态
                    if let Some(target) = ai.target_entity {
                        // TODO: 移动向目标
                    }
                }
                AIMode::Attack => {
                    // 攻击状态
                    if let Some(target) = ai.target_entity {
                        // TODO: 执行攻击
                    }
                }
                AIMode::Retreat => {
                    // 撤退状态
                    // TODO: 远离玩家
                }
            }
        }
    }
}

// ============================================================================
// CombatSystem - 战斗系统
// ============================================================================

pub struct CombatSystem;

impl CombatSystem {
    /// 处理伤害
    pub fn apply_damage(world: &mut World, entity: hecs::Entity, damage: i32) -> bool {
        if let Ok(mut health) = world.get::<&mut Health>(entity) {
            health.take_damage(damage);
            return health.is_alive();
        }
        false
    }

    /// 处理治疗
    pub fn apply_heal(world: &mut World, entity: hecs::Entity, amount: i32) {
        if let Ok(mut health) = world.get::<&mut Health>(entity) {
            health.heal(amount);
        }
    }

    /// 检查攻击命中
    pub fn check_hit(attacker_acc: u8, defender_agi: u8) -> bool {
        use rand::Rng;
        let mut rng = rand::rng();
        let hit_chance = (attacker_acc as f32 / (attacker_acc + defender_agi) as f32) * 100.0;
        rng.random_range(0..100) < hit_chance as i32
    }
}

// ============================================================================
// RenderSystem - 渲染系统 (与 GGEZ 配合)
// ============================================================================

pub struct RenderSystem;

impl RenderSystem {
    /// 收集所有可见实体并排序
    pub fn collect_visible_entities(
        world: &World,
        camera_x: i32,
        camera_y: i32,
        viewport_width: i32,
        viewport_height: i32,
    ) -> Vec<(hecs::Entity, Position, SpriteComp, RenderOrder)> {
        let mut entities = Vec::new();

        // 计算可见区域
        let min_x = camera_x - viewport_width / 2;
        let max_x = camera_x + viewport_width / 2;
        let min_y = camera_y - viewport_height / 2;
        let max_y = camera_y + viewport_height / 2;

        // 收集可见实体
        for (entity, (pos, sprite, order)) in world.query::<(&Position, &SpriteComp, &RenderOrder)>().iter() {
            if pos.x >= min_x && pos.x <= max_x && pos.y >= min_y && pos.y <= max_y {
                entities.push((entity, *pos, sprite.clone(), *order));
            }
        }

        // 按渲染顺序排序
        entities.sort_by(|a, b| {
            a.3.layer
                .cmp(&b.3.layer)
                .then_with(|| a.3.z_order.cmp(&b.3.z_order))
        });

        entities
    }
}

// ============================================================================
// NetworkSyncSystem - 网络同步系统
// ============================================================================

pub struct NetworkSyncSystem;

impl NetworkSyncSystem {
    /// 同步远程玩家位置
    pub fn sync_remote_player(
        world: &mut World,
        player_id: u32,
        x: i32,
        y: i32,
        direction: MirDirection,
        action: MirAction,
    ) {
        // 查找远程玩家实体
        let entity = world
            .query::<&RemotePlayer>()
            .iter()
            .find(|(_, remote)| remote.id == player_id)
            .map(|(e, _)| e);

        if let Some(entity) = entity {
            // 更新位置
            if let Ok(mut pos) = world.get::<&mut Position>(entity) {
                pos.x = x;
                pos.y = y;
            }

            // 更新方向
            if let Ok(mut dir) = world.get::<&mut DirectionComp>(entity) {
                dir.current = direction;
                dir.target = direction;
            }

            // 更新动画
            if let Ok(mut anim) = world.get::<&mut AnimationComp>(entity) {
                anim.action = action;
                anim.frame_index = 0;
            }
        }
    }

    /// 同步怪物状态
    pub fn sync_monster(
        world: &mut World,
        monster_id: u32,
        x: i32,
        y: i32,
        hp: i32,
        action: MirAction,
    ) {
        let entity = world
            .query::<&MonsterComp>()
            .iter()
            .find(|(_, monster)| monster.id == monster_id)
            .map(|(e, _)| e);

        if let Some(entity) = entity {
            // 更新位置
            if let Ok(mut pos) = world.get::<&mut Position>(entity) {
                pos.x = x;
                pos.y = y;
            }

            // 更新血量
            if let Ok(mut health) = world.get::<&mut Health>(entity) {
                health.current = hp;
            }

            // 更新动画
            if let Ok(mut anim) = world.get::<&mut AnimationComp>(entity) {
                anim.action = action;
            }
        }
    }
}

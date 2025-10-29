// ============================================================================
// Layer 5: State Update - ParticleSystem
// Priority: 510
// ============================================================================
//
// **职责**：
// - 粒子效果更新
// - 生命期管理
// - 位置和速度计算
//
// C# 参考: ParticleEngine.cs Process() 方法
// ============================================================================

use hecs::World;
use ggez::GameResult;
use crate::ecs::systems::{System, priority};
use crate::ecs::components::{Particle, ParticleEmitter, Position};

/// 粒子系统 - 管理粒子效果生命周期
pub struct ParticleSystem;

impl System for ParticleSystem {
    fn priority(&self) -> u32 {
        priority::PARTICLE
    }

    fn update(&mut self, world: &mut World, delay_time: f32) -> GameResult {
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f32();

        // 1. 更新粒子位置和速度
        for (_id, (particle, emitter)) in world.query_mut::<(&mut Particle, Option<&ParticleEmitter>)>() {
            // 更新位置: Position += Velocity
            let vx = particle.velocity.x;
            let vy = particle.velocity.y;
            particle.position.x += vx * delay_time;
            particle.position.y += vy * delay_time;

            // 更新图像帧
            if let Some(ref mut image) = particle.image_info {
                if current_time >= image.next_frame_time {
                    image.current_frame += 1;
                    
                    if image.current_frame >= image.frame_count {
                        image.current_frame = 0; // 循环播放
                    }
                    
                    // 计算下一帧时间 (50ms间隔)
                    image.next_frame_time = current_time + image.frame_interval;
                }
            }

            // 应用外力 (如风力、重力)
            if let Some(emitter) = emitter {
                particle.velocity.x += emitter.force_velocity.x * delay_time;
                particle.velocity.y += emitter.force_velocity.y * delay_time;
            }
        }

        // 2. 移除过期粒子
        let mut to_remove = Vec::new();
        for (id, particle) in world.query_mut::<&Particle>() {
            if current_time >= particle.alive_until {
                to_remove.push(id);
            }
        }

        for entity_id in to_remove {
            let _ = world.despawn(entity_id);
        }

        // 3. 发射器生成新粒子
        for (_id, emitter) in world.query_mut::<&mut ParticleEmitter>() {
            if emitter.generate_particles && current_time >= emitter.next_particle_time {
                emitter.next_particle_time = current_time + emitter.spawn_interval;
                
                // 注意: 实际粒子生成逻辑应该在具体的粒子类型系统中实现
                // 这里只更新计时器
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::components::{ParticleColor, BlendMode};

    #[test]
    fn test_particle_lifetime() {
        let mut world = World::new();
        let mut system = ParticleSystem;

        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f32();

        // 创建一个过期粒子
        world.spawn((
            Particle {
                position: Position { x: 100.0, y: 100.0 },
                velocity: Position { x: 1.0, y: 1.0 },
                color: ParticleColor { r: 255, g: 255, b: 255, a: 255 },
                size: 1.0,
                alive_until: current_time - 1.0, // 已过期
                blend_mode: BlendMode::Normal,
                blend_rate: 1.0,
                image_info: None,
            },
        ));

        system.update(&mut world, 0.016).unwrap();

        // 验证过期粒子已被移除
        assert_eq!(world.len(), 0);
    }

    #[test]
    fn test_particle_position_update() {
        let mut world = World::new();
        let mut system = ParticleSystem;

        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f32();

        let entity = world.spawn((
            Particle {
                position: Position { x: 100.0, y: 100.0 },
                velocity: Position { x: 10.0, y: 20.0 },
                color: ParticleColor { r: 255, g: 255, b: 255, a: 255 },
                size: 1.0,
                alive_until: current_time + 100.0, // 100秒后过期,足够测试
                blend_mode: BlendMode::Normal,
                blend_rate: 1.0,
                image_info: None,
            },
        ));

        system.update(&mut world, 1.0).unwrap();

        let particle = world.get::<&Particle>(entity).unwrap();
        assert!((particle.position.x - 110.0).abs() < 0.01);
        assert!((particle.position.y - 120.0).abs() < 0.01);
    }
}


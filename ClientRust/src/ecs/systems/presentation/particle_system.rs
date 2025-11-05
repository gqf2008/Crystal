// 处理特效
use hecs::World;
use ggez::GameResult;
use crate::ecs::GameContext;
use crate::ecs::systems::{LogicSystem, priority};
use crate::ecs::components::{Particle, ParticleEmitter, Position};

/// 粒子系统 - 管理粒子效果生命周期
#[derive(ecs_macros::LogicSystem)]
pub struct ParticleSystem;

impl LogicSystem for ParticleSystem {

    fn update(&mut self, ctx: &mut GameContext, delay_time: f32) -> GameResult {
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f32();

        // 1. 更新粒子位置和速度
        for (_id, (particle, emitter)) in ctx.world.query_mut::<(&mut Particle, Option<&ParticleEmitter>)>() {
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
        for (id, particle) in ctx.world.query_mut::<&Particle>() {
            if current_time >= particle.alive_until {
                to_remove.push(id);
            }
        }

        for entity_id in to_remove {
            let _ = ctx.world.despawn(entity_id);
        }

        // 3. 发射器生成新粒子
        for (_id, emitter) in ctx.world.query_mut::<&mut ParticleEmitter>() {
            if emitter.generate_particles && current_time >= emitter.next_particle_time {
                emitter.next_particle_time = current_time + emitter.spawn_interval;
                
                // 注意: 实际粒子生成逻辑应该在具体的粒子类型系统中实现
                // 这里只更新计时器
            }
        }

        Ok(())
    }
}

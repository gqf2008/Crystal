// 处理特效
use crate::components::{Particle, ParticleColor, ParticleEmitter, ParticleType};
use crate::game::GameContext;
use crate::game::GameResult;
use crate::systems::LogicSystem;
use macroquad::prelude::get_time;

/// 粒子系统 - 管理粒子效果生命周期
#[derive(ecs_macros::LogicSystem)]
pub struct ParticleSystem {
    /// 简易种子计数器（避免同时生成的粒子参数完全相同）
    seed_counter: u64,
}

impl Default for ParticleSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl ParticleSystem {
    pub fn new() -> Self {
        Self { seed_counter: 0 }
    }

    fn next_seed(&mut self) -> f32 {
        self.seed_counter = self.seed_counter.wrapping_add(1);
        ((self.seed_counter as f32 * 0.618_034) % 1.0).abs().fract()
    }
}

impl LogicSystem for ParticleSystem {
    fn update(&mut self, ctx: &mut GameContext, delay_time: f32) -> GameResult {
        // 使用 macroquad 的时间源（秒），避免 SystemTime 在极端情况下的 before-epoch unwrap。
        let current_time = get_time() as f32;

        // 0. 消费表现层事件（先收集粒子，再 spawn 避免 E0502）
        let mut particles_to_spawn: Vec<Particle> = Vec::new();
        for event in ctx.events().presentation_events() {
            match event {
                crate::event_bus::PresentationEvent::SpawnParticle {
                    particle_type,
                    position,
                    velocity,
                    duration,
                } => {
                    let ptype = match particle_type {
                        crate::event_bus::ParticleType::Fire => ParticleType::RedFogEmber,
                        crate::event_bus::ParticleType::Smoke => ParticleType::Fog,
                        crate::event_bus::ParticleType::Blood => ParticleType::RedFog,
                        crate::event_bus::ParticleType::Magic => ParticleType::BlueFog,
                        crate::event_bus::ParticleType::Heal => ParticleType::WhiteEmber,
                        crate::event_bus::ParticleType::Poison => ParticleType::YellowFog,
                    };
                    if let Some(mut p) = self.spawn_particle_at(position.0, position.1, ptype) {
                        if let Some((vx, vy)) = velocity {
                            p.velocity.x = *vx;
                            p.velocity.y = *vy;
                        }
                        p.alive_until = current_time + *duration;
                        particles_to_spawn.push(p);
                    }
                }
                crate::event_bus::PresentationEvent::ProjectileEffect {
                    projectile_type,
                    from,
                    to,
                    speed,
                } => {
                    let dx = to.0 - from.0;
                    let dy = to.1 - from.1;
                    let dist = (dx * dx + dy * dy).sqrt().max(0.001);
                    let lifetime = dist / *speed;
                    let (r, g, b) = match projectile_type {
                        crate::event_bus::ProjectileType::Fireball => (255, 100, 30),
                        crate::event_bus::ProjectileType::Lightning => (255, 255, 80),
                        crate::event_bus::ProjectileType::IceBolt => (100, 200, 255),
                        crate::event_bus::ProjectileType::Arrow => (180, 140, 100),
                    };
                    let mut p = Particle::new(
                        from.0,
                        from.1,
                        dx / dist * *speed,
                        dy / dist * *speed,
                        lifetime,
                    );
                    p.color = ParticleColor { r, g, b, a: 220 };
                    p.size = 4.0;
                    particles_to_spawn.push(p);
                }
                _ => {}
            }
        }
        for p in particles_to_spawn {
            ctx.world.spawn((p,));
        }

        // 1. 更新粒子位置和速度
        for (particle, emitter) in ctx
            .world
            .query_mut::<(&mut Particle, Option<&ParticleEmitter>)>()
        {
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
        for eref in ctx.world.iter() {
            if let Some(particle) = eref.get::<&Particle>() {
                if current_time >= particle.alive_until {
                    to_remove.push(eref.entity());
                }
            }
        }

        for entity_id in to_remove {
            let _ = ctx.world.despawn(entity_id);
        }

        // 3. 发射器生成新粒子（先收集参数，再 spawn 避免可变借用冲突）
        let mut to_spawn: Vec<(f32, f32, ParticleType)> = Vec::new();
        for emitter in ctx.world.query_mut::<&mut ParticleEmitter>() {
            if emitter.generate_particles && current_time >= emitter.next_particle_time {
                emitter.next_particle_time = current_time + emitter.spawn_interval;
                to_spawn.push((
                    emitter.emitter_location.x,
                    emitter.emitter_location.y,
                    emitter.particle_type,
                ));
            }
        }
        for (x, y, ptype) in to_spawn {
            if let Some(p) = self.spawn_particle_at(x, y, ptype) {
                ctx.world.spawn((p,));
            }
        }

        Ok(())
    }
}

impl ParticleSystem {
    /// 根据粒子类型生成单个粒子
    fn spawn_particle_at(&mut self, x: f32, y: f32, ptype: ParticleType) -> Option<Particle> {
        match ptype {
            ParticleType::Rain => Some(self.make_rain(x, y)),
            ParticleType::Snow => Some(self.make_snow(x, y)),
            ParticleType::Fog => Some(self.make_fog(x, y)),
            ParticleType::Sand => Some(self.make_sand(x, y)),
            ParticleType::Blizzard => Some(self.make_blizzard(x, y)),
            ParticleType::BlueFog => Some(self.make_colored_fog(x, y, 100, 150, 255)),
            ParticleType::YellowFog => Some(self.make_colored_fog(x, y, 255, 255, 100)),
            ParticleType::RedFog => Some(self.make_colored_fog(x, y, 255, 80, 80)),
            ParticleType::WhiteEmber => Some(self.make_ember(x, y, 255, 255, 255)),
            ParticleType::YellowEmber => Some(self.make_ember(x, y, 255, 220, 50)),
            ParticleType::RedFogEmber => Some(self.make_ember(x, y, 255, 80, 50)),
            ParticleType::BlizzardFrost => Some(self.make_frost(x, y)),
            ParticleType::Bird => Some(self.make_bird(x, y)),
            ParticleType::FogCloud => Some(self.make_fog_cloud(x, y)),
            ParticleType::FloatingFlower => Some(self.make_flower(x, y)),
            ParticleType::FlowersRain => Some(self.make_flowers_rain(x, y)),
            ParticleType::Leaves => Some(self.make_leaf(x, y)),
            ParticleType::FireyLeaves => Some(self.make_firey_leaf(x, y)),
            ParticleType::PurpleLeaves => Some(self.make_purple_leaf(x, y)),
            ParticleType::Test => Some(self.make_fog(x, y)),
            ParticleType::None => None,
        }
    }

    fn rand(&mut self, min: f32, max: f32) -> f32 {
        min + (max - min) * self.next_seed()
    }

    fn make_rain(&mut self, x: f32, y: f32) -> Particle {
        let vx = self.rand(-20.0, 20.0);
        let vy = self.rand(300.0, 500.0);
        let lifetime = self.rand(0.5, 1.5);
        let mut p = Particle::new(x + self.rand(-100.0, 100.0), y, vx, vy, lifetime);
        p.color = ParticleColor {
            r: 150,
            g: 180,
            b: 255,
            a: 120,
        };
        p.size = self.rand(1.0, 2.0);
        p
    }

    fn make_snow(&mut self, x: f32, y: f32) -> Particle {
        let vx = self.rand(-30.0, 30.0);
        let vy = self.rand(50.0, 120.0);
        let lifetime = self.rand(2.0, 4.0);
        let mut p = Particle::new(x + self.rand(-150.0, 150.0), y, vx, vy, lifetime);
        p.color = ParticleColor {
            r: 240,
            g: 245,
            b: 255,
            a: 200,
        };
        p.size = self.rand(2.0, 4.0);
        p
    }

    fn make_fog(&mut self, x: f32, y: f32) -> Particle {
        let vx = self.rand(-10.0, 10.0);
        let vy = self.rand(-5.0, 5.0);
        let lifetime = self.rand(3.0, 6.0);
        let mut p = Particle::new(
            x + self.rand(-80.0, 80.0),
            y + self.rand(-20.0, 20.0),
            vx,
            vy,
            lifetime,
        );
        p.color = ParticleColor {
            r: 180,
            g: 180,
            b: 180,
            a: 80,
        };
        p.size = self.rand(15.0, 30.0);
        p
    }

    fn make_colored_fog(&mut self, x: f32, y: f32, r: u8, g: u8, b: u8) -> Particle {
        let vx = self.rand(-10.0, 10.0);
        let vy = self.rand(-5.0, 5.0);
        let lifetime = self.rand(3.0, 6.0);
        let mut p = Particle::new(
            x + self.rand(-80.0, 80.0),
            y + self.rand(-20.0, 20.0),
            vx,
            vy,
            lifetime,
        );
        p.color = ParticleColor { r, g, b, a: 80 };
        p.size = self.rand(15.0, 30.0);
        p
    }

    fn make_ember(&mut self, x: f32, y: f32, r: u8, g: u8, b: u8) -> Particle {
        let vx = self.rand(-40.0, 40.0);
        let vy = self.rand(-80.0, -20.0);
        let lifetime = self.rand(0.5, 2.0);
        let mut p = Particle::new(
            x + self.rand(-30.0, 30.0),
            y + self.rand(-30.0, 30.0),
            vx,
            vy,
            lifetime,
        );
        p.color = ParticleColor { r, g, b, a: 220 };
        p.size = self.rand(1.0, 3.0);
        p
    }

    fn make_sand(&mut self, x: f32, y: f32) -> Particle {
        let vx = self.rand(80.0, 200.0);
        let vy = self.rand(-20.0, 20.0);
        let lifetime = self.rand(1.0, 3.0);
        let mut p = Particle::new(x, y + self.rand(-50.0, 50.0), vx, vy, lifetime);
        p.color = ParticleColor {
            r: 210,
            g: 180,
            b: 120,
            a: 150,
        };
        p.size = self.rand(1.0, 3.0);
        p
    }

    fn make_frost(&mut self, x: f32, y: f32) -> Particle {
        let vx = self.rand(-15.0, 15.0);
        let vy = self.rand(80.0, 200.0);
        let lifetime = self.rand(1.0, 2.5);
        let mut p = Particle::new(x + self.rand(-100.0, 100.0), y, vx, vy, lifetime);
        p.color = ParticleColor {
            r: 200,
            g: 230,
            b: 255,
            a: 180,
        };
        p.size = self.rand(2.0, 5.0);
        p
    }

    fn make_bird(&mut self, x: f32, y: f32) -> Particle {
        let vx = self.rand(50.0, 150.0);
        let vy = self.rand(-10.0, 10.0);
        let lifetime = self.rand(2.0, 5.0);
        let mut p = Particle::new(x, y, vx, vy, lifetime);
        p.color = ParticleColor {
            r: 60,
            g: 60,
            b: 60,
            a: 255,
        };
        p.size = self.rand(5.0, 10.0);
        p
    }

    fn make_flower(&mut self, x: f32, y: f32) -> Particle {
        let vx = self.rand(-20.0, 20.0);
        let vy = self.rand(-40.0, -10.0);
        let lifetime = self.rand(3.0, 6.0);
        let mut p = Particle::new(x + self.rand(-50.0, 50.0), y, vx, vy, lifetime);
        let colors = [
            ParticleColor {
                r: 255,
                g: 180,
                b: 200,
                a: 200,
            },
            ParticleColor {
                r: 255,
                g: 220,
                b: 150,
                a: 200,
            },
            ParticleColor {
                r: 200,
                g: 150,
                b: 255,
                a: 200,
            },
        ];
        p.color = colors[(self.seed_counter as usize) % 3];
        p.size = self.rand(3.0, 6.0);
        p
    }

    fn make_leaf(&mut self, x: f32, y: f32) -> Particle {
        let vx = self.rand(-30.0, 50.0);
        let vy = self.rand(30.0, 80.0);
        let lifetime = self.rand(3.0, 6.0);
        let mut p = Particle::new(x + self.rand(-50.0, 50.0), y, vx, vy, lifetime);
        p.color = ParticleColor {
            r: 80,
            g: 180,
            b: 50,
            a: 200,
        };
        p.size = self.rand(3.0, 6.0);
        p
    }

    fn make_firey_leaf(&mut self, x: f32, y: f32) -> Particle {
        let vx = self.rand(-30.0, 50.0);
        let vy = self.rand(-50.0, -10.0);
        let lifetime = self.rand(1.0, 3.0);
        let mut p = Particle::new(x + self.rand(-30.0, 30.0), y, vx, vy, lifetime);
        p.color = ParticleColor {
            r: 255,
            g: 150,
            b: 30,
            a: 220,
        };
        p.size = self.rand(3.0, 6.0);
        p
    }

    fn make_purple_leaf(&mut self, x: f32, y: f32) -> Particle {
        let vx = self.rand(-30.0, 50.0);
        let vy = self.rand(30.0, 80.0);
        let lifetime = self.rand(3.0, 6.0);
        let mut p = Particle::new(x + self.rand(-50.0, 50.0), y, vx, vy, lifetime);
        p.color = ParticleColor {
            r: 150,
            g: 50,
            b: 200,
            a: 200,
        };
        p.size = self.rand(3.0, 6.0);
        p
    }

    /// 暴风雪：比普通雨更快、更密集、带水平漂移
    fn make_blizzard(&mut self, x: f32, y: f32) -> Particle {
        let vx = self.rand(100.0, 300.0); // 强水平风
        let vy = self.rand(150.0, 350.0); // 比普通雨稍慢
        let lifetime = self.rand(0.8, 2.0);
        let mut p = Particle::new(x + self.rand(-200.0, 200.0), y, vx, vy, lifetime);
        p.color = ParticleColor {
            r: 200,
            g: 220,
            b: 255,
            a: 180,
        };
        p.size = self.rand(1.5, 3.5);
        p
    }

    /// 花瓣雨：带颜色变化、飘落效果（缓慢下降+水平摇摆）
    fn make_flowers_rain(&mut self, x: f32, y: f32) -> Particle {
        let vx = self.rand(-40.0, 40.0); // 明显摇摆
        let vy = self.rand(80.0, 180.0); // 缓慢飘落
        let lifetime = self.rand(2.0, 4.0);
        let mut p = Particle::new(x + self.rand(-150.0, 150.0), y, vx, vy, lifetime);
        let colors = [
            ParticleColor {
                r: 255,
                g: 150,
                b: 180,
                a: 200,
            },
            ParticleColor {
                r: 255,
                g: 200,
                b: 150,
                a: 200,
            },
            ParticleColor {
                r: 255,
                g: 180,
                b: 220,
                a: 200,
            },
            ParticleColor {
                r: 255,
                g: 220,
                b: 180,
                a: 200,
            },
        ];
        p.color = colors[(self.seed_counter as usize) % 4];
        p.size = self.rand(2.0, 5.0); // 比普通雨大
        p
    }

    /// 云雾：大颗粒、缓慢移动、半透明
    fn make_fog_cloud(&mut self, x: f32, y: f32) -> Particle {
        let vx = self.rand(-5.0, 5.0); // 极慢水平移动
        let vy = self.rand(-3.0, 3.0); // 轻微上下浮动
        let lifetime = self.rand(5.0, 10.0); // 更长生命周期
        let mut p = Particle::new(
            x + self.rand(-100.0, 100.0),
            y + self.rand(-40.0, 40.0),
            vx,
            vy,
            lifetime,
        );
        p.color = ParticleColor {
            r: 200,
            g: 200,
            b: 210,
            a: 60,
        }; // 更透明
        p.size = self.rand(25.0, 50.0); // 更大颗粒
        p
    }
}

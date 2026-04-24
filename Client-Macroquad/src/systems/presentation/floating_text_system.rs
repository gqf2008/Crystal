use crate::components::{FloatingText, Position};
use crate::game::{GameContext, GameResult};
use crate::systems::LogicSystem;
use macroquad::prelude::Color;

/// 待生成的漂浮文本参数
struct FloatingTextSpawn {
    x: f32,
    y: f32,
    text: String,
    color: Color,
    duration: f64,
    rise_speed: f32,
}

/// 漂浮文本系统：更新上浮位置并清理过期实体
#[derive(ecs_macros::LogicSystem)]
pub struct FloatingTextSystem;

impl Default for FloatingTextSystem {
    fn default() -> Self {
        Self
    }
}

impl LogicSystem for FloatingTextSystem {
    fn update(&mut self, ctx: &mut GameContext, dt: f32) -> GameResult {
        let now = macroquad::prelude::get_time();

        // 先收集表现层事件，避免 E0502
        let mut to_spawn: Vec<FloatingTextSpawn> = Vec::new();
        for event in ctx.events().presentation_events() {
            match event {
                crate::event_bus::PresentationEvent::FloatingText { text, position, color, font_size: _, duration } => {
                    to_spawn.push(FloatingTextSpawn {
                        x: position.0,
                        y: position.1,
                        text: text.clone(),
                        color: *color,
                        duration: *duration as f64,
                        rise_speed: 30.0,
                    });
                }
                crate::event_bus::PresentationEvent::FloatingHeal { amount, position } => {
                    to_spawn.push(FloatingTextSpawn {
                        x: position.0,
                        y: position.1,
                        text: format!("+{}", amount),
                        color: Color::from_rgba(80, 255, 80, 255),
                        duration: 1.5,
                        rise_speed: 30.0,
                    });
                }
                crate::event_bus::PresentationEvent::FloatingExperience { amount, position } => {
                    to_spawn.push(FloatingTextSpawn {
                        x: position.0,
                        y: position.1,
                        text: format!("+{} EXP", amount),
                        color: Color::from_rgba(255, 220, 100, 255),
                        duration: 1.5,
                        rise_speed: 25.0,
                    });
                }
                _ => {}
            }
        }
        for spawn in to_spawn {
            let _ = ctx.world.spawn((
                Position { x: spawn.x, y: spawn.y },
                FloatingText {
                    text: spawn.text,
                    start_time: now,
                    duration: spawn.duration,
                    rise_speed: spawn.rise_speed,
                    color: Some(spawn.color),
                },
            ));
        }

        let mut to_remove: Vec<hecs::Entity> = Vec::new();

        for eref in ctx.world.iter() {
            let (Some(mut pos), Some(ft)) = (eref.get::<&mut Position>(), eref.get::<&FloatingText>()) else {
                continue;
            };
            // 过期清理
            if now >= ft.start_time + ft.duration {
                to_remove.push(eref.entity());
                continue;
            }

            // 上浮（只改 y，避免影响碰撞/寻路）
            pos.y -= ft.rise_speed * dt;
        }

        for e in to_remove {
            let _ = ctx.world.despawn(e);
        }

        Ok(())
    }
}

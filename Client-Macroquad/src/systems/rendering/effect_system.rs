use crate::components::{LibrarySprite, Position, RenderPass, SpriteBlendMode};
use crate::game::GameResult;
use crate::systems::RenderSystem;
use macroquad::miniquad::{BlendFactor, BlendState, BlendValue, Equation};
use macroquad::prelude::*;

#[derive(ecs_macros::RenderSystem)]
pub struct EffectRenderSystem {
    add_blend_material: Material,
}

impl EffectRenderSystem {
    pub fn new() -> Self {
        let add_blend_material = load_material(
            ShaderSource::Glsl {
                vertex: include_str!("../../../shaders/default.vert"),
                fragment: include_str!("../../../shaders/default.frag"),
            },
            MaterialParams {
                pipeline_params: PipelineParams {
                    color_blend: Some(BlendState::new(
                        Equation::Add,
                        BlendFactor::Value(BlendValue::SourceAlpha),
                        BlendFactor::One,
                    )),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

        Self { add_blend_material }
    }
}

impl RenderSystem for EffectRenderSystem {
    fn draw(&mut self, _world: &hecs::World) -> GameResult {
        // ghost pass 只画本地玩家，不画特效
        if _world
            .query::<&RenderPass>()
            .iter()
            .next()
            .map(|(_, pass)| pass.local_only)
            .unwrap_or(false)
        {
            return Ok(());
        }

        let alpha = _world
            .query::<&RenderPass>()
            .iter()
            .next()
            .map(|(_, pass)| pass.alpha)
            .unwrap_or(1.0)
            .clamp(0.0, 1.0);

        let draw_layer = |lib_sprite: &LibrarySprite, pos: &Position, tint: Color, offset: Vec2| -> bool {
            let Some(info) = lib_sprite.library.get_texture(lib_sprite.texture_index()) else {
                return false;
            };
            let Some(tex) = info.image else {
                return false;
            };
            let draw_x = pos.x + info.offset_x as f32 + offset.x;
            let draw_y = pos.y + info.offset_y as f32 + offset.y;
            draw_texture_ex(
                &tex,
                draw_x,
                draw_y,
                tint,
                DrawTextureParams { ..Default::default() },
            );
            true
        };

        for (_entity, (spr, pos)) in _world.query::<(&LibrarySprite, &Position)>().iter() {
            let tint = Color::new(1.0, 1.0, 1.0, alpha);

            // 1) ADD 混合：当作“特效层”渲染（放在 SpriteRenderSystem 之后）
            if matches!(spr.blend_mode, SpriteBlendMode::Additive) {
                gl_use_material(&self.add_blend_material);
                let _ = draw_layer(spr, pos, tint, Vec2::ZERO);
                gl_use_default_material();
                continue;
            }
        }

        Ok(())
    }
}
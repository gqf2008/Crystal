use crate::components::{HoverHighlight, LibrarySprite, Position, RenderPass, SpriteBlendMode};
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

        let hovered_npc_object_id = _world
            .query::<&HoverHighlight>()
            .iter()
            .next()
            .and_then(|(_, hh)| hh.npc_object_id);

        let cam_zoom = _world
            .query::<&crate::components::Camera>()
            .iter()
            .next()
            .map(|(_, c)| c.zoom)
            .unwrap_or(1.0)
            .max(0.0001);

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

            // NPC 悬停轮廓：用“多次偏移描边”的方式模拟原版轮廓高亮
            if let Some(hover_oid) = hovered_npc_object_id {
                if matches!(spr.blend_mode, SpriteBlendMode::Alpha) {
                    if let Ok(sync) = _world.get::<&crate::components::NetworkSync>(_entity) {
                        if sync.object_type == crate::components::NetworkObjectType::NPC
                            && sync.object_id == hover_oid
                        {
                            // 轮廓粗细按屏幕像素近似：world_offset = px / zoom
                            // 之前 1.5px 偏细，在部分 NPC 资源上不明显；这里加粗一圈。
                            let o1 = 3.5 / cam_zoom;
                            let o2 = 6.0 / cam_zoom;
                            let outline_tint = Color::new(
                                0.1,
                                1.0,
                                0.1,
                                (0.95 * alpha).clamp(0.0, 1.0),
                            );
                            let offsets1 = [
                                vec2(-o1, 0.0),
                                vec2(o1, 0.0),
                                vec2(0.0, -o1),
                                vec2(0.0, o1),
                                vec2(-o1, -o1),
                                vec2(o1, -o1),
                                vec2(-o1, o1),
                                vec2(o1, o1),
                            ];
                            let offsets2 = [
                                vec2(-o2, 0.0),
                                vec2(o2, 0.0),
                                vec2(0.0, -o2),
                                vec2(0.0, o2),
                                vec2(-o2, -o2),
                                vec2(o2, -o2),
                                vec2(-o2, o2),
                                vec2(o2, o2),
                            ];

                            for off in offsets1 {
                                let _ = draw_layer(spr, pos, outline_tint, off);
                            }
                            for off in offsets2 {
                                let _ = draw_layer(spr, pos, outline_tint, off);
                            }
                        }
                    }
                }
            }

            match spr.blend_mode {
                SpriteBlendMode::Additive => {
                    gl_use_material(&self.add_blend_material);
                    let _ = draw_layer(spr, pos, tint, Vec2::ZERO);
                    gl_use_default_material();
                }
                SpriteBlendMode::Alpha => {
                    let _ = draw_layer(spr, pos, tint, Vec2::ZERO);
                }
            }
        }

        Ok(())
    }
}
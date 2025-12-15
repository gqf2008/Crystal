mod character;
mod weapon;

use crate::components::{Camera, FrontOcclusion, Position, RenderPass, RenderStage};
use crate::systems::RenderSystem;
// use ggez::{graphics::GraphicsContext, GameResult};
use macroquad::miniquad::{BlendFactor, BlendState, BlendValue, Equation};
use macroquad::prelude::*;

#[derive(ecs_macros::RenderSystem)]
pub struct SpriteRenderSystem {
    add_blend_material: Material,
}

impl SpriteRenderSystem {
    pub fn new() -> Self {
        // 创建 ADD 混合材质 (dst + src * alpha)
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

impl SpriteRenderSystem {
    /// 获取相机变换参数
    #[allow(dead_code)]
    fn get_camera_transform(world: &hecs::World) -> Option<(f32, f32, f32)> {
        let mut query = world.query::<(&Camera, &Position)>();
        if let Some((_, (camera, cam_pos))) = query.iter().next() {
            Some((cam_pos.x, cam_pos.y, camera.zoom))
        } else {
            None
        }
    }

    /// 世界坐标 → 屏幕坐标
    #[allow(dead_code)]
    fn world_to_screen(
        world_x: f32,
        world_y: f32,
        cam_x: f32,
        cam_y: f32,
        zoom: f32,
        screen_width: f32,
        screen_height: f32,
    ) -> (f32, f32) {
        let relative_x = (world_x - cam_x) * zoom;
        let relative_y = (world_y - cam_y) * zoom;
        (
            screen_width / 2.0 + relative_x,
            screen_height / 2.0 + relative_y,
        )
    }
}

impl RenderSystem for SpriteRenderSystem {
    fn draw(
        &mut self,
        world: &hecs::World,
    ) -> crate::game::GameResult {
        let pass = world
            .query::<&RenderPass>()
            .iter()
            .next()
            .map(|(_, pass)| *pass)
            .unwrap_or_default();

        match pass.stage {
            RenderStage::Normal => {
                // 正常世界渲染
                self.draw_character(world, &self.add_blend_material, pass.alpha, pass.local_only)?;
            }
            RenderStage::PostFront => {
                // PostFront：如果本地玩家被前景遮挡，则画一层 ghost（半透明本地玩家）
                let occluded = world
                    .query::<&FrontOcclusion>()
                    .iter()
                    .next()
                    .map(|(_, o)| o.local_player_occluded)
                    .unwrap_or(false);

                if occluded {
                    const PLAYER_GHOST_ALPHA: f32 = 0.45;
                    self.draw_character(world, &self.add_blend_material, PLAYER_GHOST_ALPHA, true)?;
                }
            }
            RenderStage::Ui => {
                // UI stage 不画世界精灵
            }
        }
        Ok(())
    }
}

mod character;
mod weapon;

use crate::components::{Camera,Position};
use crate::systems::RenderSystem;
// use ggez::{graphics::GraphicsContext, GameResult};

#[derive(ecs_macros::RenderSystem)]
pub struct SpriteRenderSystem;

impl SpriteRenderSystem {
    pub fn new() -> Self {
        Self
    }
}

impl SpriteRenderSystem {
    /// 获取相机变换参数
    fn get_camera_transform(world: &hecs::World) -> Option<(f32, f32, f32)> {
        let mut query = world.query::<(&Camera, &Position)>();
        if let Some((_, (camera, cam_pos))) = query.iter().next() {
            Some((cam_pos.x, cam_pos.y, camera.zoom))
        } else {
            None
        }
    }

    /// 世界坐标 → 屏幕坐标
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
        _world: &hecs::World,
    ) -> crate::game::GameResult {
        // TODO: 重写为 macroquad API
        // self.draw_character(world)?;
        Ok(())
    }
}

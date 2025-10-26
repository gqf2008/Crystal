// ============================================================================
// Item Rendering Module - 物品渲染模块
// ============================================================================

use super::RenderSystem;
use crate::ecs::components::{ItemDrop, Position, Camera};
use ggez::{Context, GameResult, graphics::Canvas};
use hecs::World;

impl RenderSystem {
    /// 绘制地面物品
    /// 
    /// 参数：
    /// - ctx: ggez 上下文
    /// - canvas: 画布
    /// - world: ECS 世界
    /// - camera_pos: 相机位置
    /// - camera: 相机组件
    pub fn draw_items(
        ctx: &mut Context,
        canvas: &mut Canvas,
        world: &World,
        camera_pos: &Position,
        camera: &Camera,
    ) -> GameResult<()> {
        use crate::graphics::libraries::get_library;
        use crate::ecs::systems::CameraSystem;
        use crate::graphics::libraries::LibraryName;
        
        // 遍历所有地面物品实体
        for (_entity, (item_drop, pos)) in 
            world.query::<(&ItemDrop, &Position)>().iter() 
        {
            // 获取物品图标库 (FloorItems)
            let lib = match get_library(LibraryName::FloorItems) {
                Some(lib) => lib,
                None => continue, // 库不存在，跳过
            };
            
            // 将世界坐标转换为屏幕坐标
            let (screen_x, screen_y) = CameraSystem::world_to_screen(
                camera_pos,
                camera,
                pos.x,
                pos.y
            );
            
            // 简单裁剪：只渲染屏幕内的物品
            if screen_x < -50.0 || screen_x > camera.screen_width + 50.0 ||
               screen_y < -50.0 || screen_y > camera.screen_height + 50.0 {
                continue;
            }
            
            // 绘制物品图标
            let mut lib_locked = lib.lock().unwrap();
            if let Ok(_) = lib_locked.draw(
                ctx,
                canvas,
                item_drop.item_index as usize,
                screen_x,
                screen_y
            ) {
                // 绘制成功
            }
        }
        
        Ok(())
    }
}

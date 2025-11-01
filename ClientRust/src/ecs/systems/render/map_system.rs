use crate::ecs::components::{Camera, MapTile, Position, RenderConfig, TileLayer};
use crate::ecs::systems::DrawSystem;
use crate::ecs::{CELL_HEIGHT, CELL_WIDTH};
use crate::graphics::get_map_library;
use ggez::graphics::DrawParam;
use ggez::GameResult;

pub struct MapRenderSystem;

impl DrawSystem for MapRenderSystem {
    fn draw(
        &mut self,
        ctx: &mut ggez::Context,
        canvas: &mut ggez::graphics::Canvas,
        world: &hecs::World,
    ) -> GameResult {
        // 获取相机和配置
        let (camera, camera_pos, config) = {
            let mut camera_opt = None;
            let mut camera_pos_opt = None;
            
            for (_, (cam, pos)) in world.query::<(&Camera, &Position)>().iter() {
                camera_opt = Some(cam.clone());
                camera_pos_opt = Some(pos.clone());
                break;
            }
            
            let mut config_opt = None;
            for (_, cfg) in world.query::<&RenderConfig>().iter() {
                config_opt = Some(cfg.clone());
                break;
            }
            
            match (camera_opt, camera_pos_opt, config_opt) {
                (Some(cam), Some(pos), Some(cfg)) => (cam, pos, cfg),
                _ => return Ok(()), // 没有相机或配置，跳过渲染
            }
        };
        
        // 计算视口范围（世界坐标）
        let half_width = (camera.screen_width / 2.0) / camera.zoom;
        let half_height = (camera.screen_height / 2.0) / camera.zoom;
        let view_left = camera_pos.x - half_width;
        let view_right = camera_pos.x + half_width;
        let view_top = camera_pos.y - half_height;
        let view_bottom = camera_pos.y + half_height;
        
        // 按层渲染：Back -> Middle -> Front
        let layers = [
            (TileLayer::Back, config.show_back),
            (TileLayer::Middle, config.show_middle),
            (TileLayer::Front, config.show_front),
        ];
        
        for (layer, should_show) in layers.iter() {
            if !should_show {
                continue;
            }
            
            // 收集该层的所有瓦片（包括动画瓦片）
            let mut tiles_to_draw: Vec<(i32, i32, i16, usize, bool)> = Vec::new();
            
            // Front 层需要更大的底部视口（建筑物是长条形，UV坐标在左下角）
            let bottom_extra = if matches!(layer, TileLayer::Front) {
                800.0  // Front 层底部额外扩展 800 像素
            } else {
                200.0  // Back/Middle 层保持 200 像素
            };
            
            // 静态瓦片
            for (_, tile) in world.query::<&MapTile>().iter().filter(|(_, t)| matches!(t.layer, layer)) {
                // 计算瓦片的世界坐标
                let world_x = (tile.grid_x * CELL_WIDTH) as f32;
                let world_y = (tile.grid_y * CELL_HEIGHT) as f32;
                
                // 视口裁剪（Front 层底部扩大）
                if world_x > view_right + 200.0
                    || world_x < view_left - 200.0
                    || world_y > view_bottom + bottom_extra
                    || world_y < view_top - 200.0
                {
                    continue;
                }
                
                tiles_to_draw.push((
                    tile.grid_x,
                    tile.grid_y,
                    tile.library_index,
                    tile.image_index as usize,
                    false,  // 不是动画瓦片
                ));
            }
            
            // 动画瓦片（使用 AnimatedTile 计算当前帧）
            use crate::ecs::components::AnimatedTile;
            for (_, (tile, anim)) in world.query::<(&MapTile, &AnimatedTile)>().iter().filter(|(_, (t, _))| matches!(t.layer, layer)) {
                let world_x = (tile.grid_x * CELL_WIDTH) as f32;
                let world_y = (tile.grid_y * CELL_HEIGHT) as f32;
                
                // 使用与静态瓦片相同的视口裁剪规则
                if world_x > view_right + 200.0
                    || world_x < view_left - 200.0
                    || world_y > view_bottom + bottom_extra
                    || world_y < view_top - 200.0
                {
                    continue;
                }
                
                // 计算当前帧（简化版本，实际应该由 AnimationSystem 更新）
                // 这里临时使用基础图像索引
                tiles_to_draw.push((
                    tile.grid_x,
                    tile.grid_y,
                    tile.library_index,
                    tile.image_index as usize,  // 应该根据动画帧计算
                    true,  // 是动画瓦片
                ));
            }
            
            // 渲染该层的瓦片
            for (grid_x, grid_y, lib_index, img_index, is_anim) in tiles_to_draw {
                if let Some(lib) = get_map_library(lib_index) {
                    if let Ok(mut lib_guard) = lib.lock() {
                        // 先获取尺寸
                        let (tile_w, tile_h) = lib_guard
                            .get_size(img_index)
                            .unwrap_or((CELL_WIDTH as i16, CELL_HEIGHT as i16));
                        
                        // 获取纹理信息（包括偏移量）
                        if let Ok(info) = lib_guard.get_or_create_texture(ctx, img_index) {
                            if let Some(image) = &info.image {
                                // 计算世界坐标（格子坐标）
                                let world_x = (grid_x * CELL_WIDTH) as f32;
                                let world_y = (grid_y * CELL_HEIGHT) as f32;
                                
                                // 关键：图像 UV 坐标在左下角！
                                // 需要将图像左下角对齐到网格坐标
                                // 
                                // 静态层（无动画）：不使用 info.x/y 偏移
                                // 动态层（有动画）：使用 info.x/y 偏移（但目前我们还没有动画）
                                let adjusted_x = world_x;  // 静态层不使用 info.x
                                let adjusted_y = world_y - tile_h as f32;  // 静态层不使用 info.y
                                
                                // 世界坐标 -> 屏幕坐标
                                let screen_x = (adjusted_x - camera_pos.x) * camera.zoom
                                    + camera.screen_width / 2.0;
                                let screen_y = (adjusted_y - camera_pos.y) * camera.zoom
                                    + camera.screen_height / 2.0;
                                
                                // 绘制
                                canvas.draw(
                                    image,
                                    DrawParam::default()
                                        .dest([screen_x, screen_y])
                                        .scale([camera.zoom, camera.zoom]),
                                );
                            }
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
}

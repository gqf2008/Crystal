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
            
            // 收集该层的所有瓦片
            let mut tiles_to_draw: Vec<(i32, i32, usize, usize)> = Vec::new();
            
            for (_, tile) in world.query::<&MapTile>().iter() {
                if !matches!(tile.layer, layer) {
                    continue;
                }
                
                // 计算瓦片的世界坐标
                let world_x = (tile.grid_x * CELL_WIDTH) as f32;
                let world_y = (tile.grid_y * CELL_HEIGHT) as f32;
                
                // 简单的视口裁剪（预估瓦片可能很大，所以给较大的边界）
                if world_x > view_right + 200.0
                    || world_x < view_left - 200.0
                    || world_y > view_bottom + 200.0
                    || world_y < view_top - 200.0
                {
                    continue;
                }
                
                tiles_to_draw.push((
                    tile.grid_x,
                    tile.grid_y,
                    tile.library_index,
                    tile.image_index,
                ));
            }
            
            // 渲染该层的瓦片
            for (grid_x, grid_y, lib_index, img_index) in tiles_to_draw {
                if let Some(lib) = get_map_library(lib_index) {
                    if let Ok(mut lib_guard) = lib.lock() {
                        // 获取纹理信息
                        if let Ok(info) = lib_guard.get_or_create_texture(ctx, img_index) {
                            if let Some(image) = &info.image {
                                // 获取瓦片尺寸
                                let (tile_w, tile_h) = lib_guard
                                    .get_size(img_index)
                                    .unwrap_or((CELL_WIDTH as i16, CELL_HEIGHT as i16));
                                
                                // 计算世界坐标
                                let world_x = (grid_x * CELL_WIDTH) as f32;
                                let world_y = (grid_y * CELL_HEIGHT) as f32;
                                
                                // 调整 Y 坐标（底部对齐）
                                let adjusted_y = if (tile_w as i32 != CELL_WIDTH
                                    || tile_h as i32 != CELL_HEIGHT)
                                    && (tile_w as i32 != CELL_WIDTH * 2
                                        || tile_h as i32 != CELL_HEIGHT * 2)
                                {
                                    world_y + CELL_HEIGHT as f32 - tile_h as f32
                                } else {
                                    world_y
                                };
                                
                                // 世界坐标 -> 屏幕坐标
                                let screen_x = (world_x - camera_pos.x) * camera.zoom
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

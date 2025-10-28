// ============================================================================
// 瓦片渲染模块
// ============================================================================

use ggez::{Context, GameResult};
use ggez::graphics::{self, Canvas, DrawParam, Color, BlendMode};
use hecs::World;
use crate::ecs::components::{Position, Camera, RenderConfig, MapTile, TileLayer, VisibleArea};
use crate::ecs::{CELL_WIDTH, CELL_HEIGHT};
use crate::ecs::systems::CameraSystem;
use crate::graphics::libraries::get_map_library;
use super::RenderSystem;
use std::time::Instant;

impl RenderSystem {
    /// 绘制瓦片系统(带可见性裁剪和LOD优化)
    pub fn draw_tiles(
        ctx: &mut Context,
        canvas: &mut Canvas,
        world: &World,
        pos: &Position,
        camera: &Camera,
        config: &RenderConfig,
        visible_area_entity: hecs::Entity,
    ) -> GameResult<()> {
        // 🐛 调试：首次绘制时输出信息
        static mut FIRST_DRAW: bool = true;
        static mut DRAW_COUNTER: u32 = 0;
        unsafe {
            DRAW_COUNTER += 1;
            if FIRST_DRAW || DRAW_COUNTER % 300 == 0 {
                tracing::info!(
                    "🗺️ draw_tiles: camera_pos=({:.1}, {:.1}), zoom={:.2}, screen={}×{}",
                    pos.x, pos.y, camera.zoom, camera.screen_width, camera.screen_height
                );
                FIRST_DRAW = false;
            }
        }
        
        // 计算可见区域
        let projection_scale = 1.0 / camera.zoom;
        let half_width = camera.screen_width / 2.0 * projection_scale;
        let half_height = camera.screen_height / 2.0 * projection_scale;

        let left = pos.x - half_width;
        let right = pos.x + half_width;
        let top = pos.y - half_height;
        let bottom = pos.y + half_height;

        // 🔧 修复边缘黑块：增加buffer以确保边缘瓦片正确渲染
        let base_buffer = if camera.zoom < 0.4 { 2 } else { 3 };
        let buffer = ((base_buffer as f32 * projection_scale).ceil() as i32).max(2).min(8);

        // 转换为地图格子坐标
        let start_x = ((left / CELL_WIDTH as f32).floor() as i32 - buffer).max(0);
        let end_x = (right / CELL_WIDTH as f32).ceil() as i32 + buffer;
        let start_y = ((top / CELL_HEIGHT as f32).floor() as i32 - buffer).max(0);
        let end_y = (bottom / CELL_HEIGHT as f32).ceil() as i32 + buffer;

        // 🎨 Front层特殊处理：向下扩展更多格子（建筑物可能很高）
        let front_extra_cells = ((15.0 * projection_scale).ceil() as i32).max(8).min(30);
        let front_end_y = end_y + front_extra_cells;

        // 🔍 检测可见区域或缩放是否变化
        if let Ok(mut visible_area) = world.get::<&mut VisibleArea>(visible_area_entity) {
            let min_cell_threshold = 1;
            let x_changed = (visible_area.start_x - start_x).abs() >= min_cell_threshold
                || (visible_area.end_x - end_x).abs() >= min_cell_threshold;
            let y_changed = (visible_area.start_y - start_y).abs() >= min_cell_threshold
                || (visible_area.end_y - end_y).abs() >= min_cell_threshold;
            let front_y_changed =
                (visible_area.front_end_y - front_end_y).abs() >= min_cell_threshold;
            let zoom_changed = (visible_area.zoom - camera.zoom).abs() > 0.05;

            let area_changed = x_changed || y_changed || front_y_changed || zoom_changed;

            // 如果区域变化，重新查询可见瓦片
            if area_changed {
                visible_area.visible_entities.clear();

                // 🔥 收集可见实体（带 z_order 和 Y 坐标用于排序）
                let mut visible_with_sort_key: Vec<(hecs::Entity, i32, i32)> = Vec::new();

                // 🎯 LOD优化
                let lod_skip = config.enable_lod && camera.zoom < 0.5;

                // 查询所有瓦片并过滤
                for (entity, tile) in world.query::<&MapTile>().iter() {
                    if lod_skip && tile.layer != TileLayer::Back {
                        if (tile.grid_x + tile.grid_y) % 2 == 0 {
                            continue;
                        }
                    }

                    let in_visible_range = match tile.layer {
                        TileLayer::Front => {
                            tile.grid_x >= start_x
                                && tile.grid_x <= end_x
                                && tile.grid_y >= start_y
                                && tile.grid_y <= front_end_y
                        }
                        _ => {
                            tile.grid_x >= start_x
                                && tile.grid_x <= end_x
                                && tile.grid_y >= start_y
                                && tile.grid_y <= end_y
                        }
                    };

                    if in_visible_range {
                        visible_with_sort_key.push((entity, tile.z_order, tile.grid_y));
                    }
                }

                // 🎯 按 Z轴排序（z_order 优先，相同则按 Y 坐标）
                visible_with_sort_key.sort_by(|a, b| {
                    match a.1.cmp(&b.1) {
                        std::cmp::Ordering::Equal => a.2.cmp(&b.2),
                        other => other,
                    }
                });

                visible_area.visible_entities = visible_with_sort_key.into_iter().map(|(e, _, _)| e).collect();

                // 更新缓存
                visible_area.start_x = start_x;
                visible_area.end_x = end_x;
                visible_area.start_y = start_y;
                visible_area.end_y = end_y;
                visible_area.front_end_y = front_end_y;
                visible_area.zoom = camera.zoom;
                visible_area.camera_x = pos.x;
                visible_area.camera_y = pos.y;
                visible_area.last_update = Instant::now();
            }

            // 🎯 绘制缓存的可见瓦片
            // 🚀 批量渲染优化：按混合模式分组，减少状态切换
            let mut normal_tiles = Vec::new();
            let mut blend_tiles = Vec::new();
            
            for &entity in &visible_area.visible_entities {
                if let Ok(tile) = world.get::<&MapTile>(entity) {
                    // 根据配置跳过某些层
                    match tile.layer {
                        TileLayer::Back if !config.show_back => continue,
                        TileLayer::Middle if !config.show_middle => continue,
                        TileLayer::Front if !config.show_front => continue,
                        _ => {}
                    }

                    // 🆕 获取遮挡透明度（仅 Front 层）
                    let alpha = if tile.layer == TileLayer::Front {
                        if let Ok(occlusion) = world.get::<&crate::ecs::components::TileOcclusion>(entity) {
                            occlusion.current_alpha
                        } else {
                            1.0
                        }
                    } else {
                        1.0
                    };

                    // 按混合模式分组
                    if tile.use_blend {
                        blend_tiles.push((tile, alpha));
                    } else {
                        normal_tiles.push((tile, alpha));
                    }
                }
            }

            // 先渲染普通瓦片
            if !normal_tiles.is_empty() {
                canvas.set_blend_mode(BlendMode::ALPHA);
                for (tile, alpha) in normal_tiles {
                    Self::draw_tile_fast(ctx, canvas, &tile, pos, camera, config, alpha)?;
                }
            }

            // 再渲染混合瓦片
            if !blend_tiles.is_empty() {
                canvas.set_blend_mode(Self::create_blend_mode());
                for (tile, alpha) in blend_tiles {
                    Self::draw_tile_fast(ctx, canvas, &tile, pos, camera, config, alpha)?;
                }
                canvas.set_blend_mode(BlendMode::ALPHA);
            }
        }

        Ok(())
    }

    /// 绘制单个瓦片（快速版本）
    pub fn draw_tile_fast(
        ctx: &mut Context,
        canvas: &mut Canvas,
        tile: &MapTile,
        pos: &Position,
        camera: &Camera,
        config: &RenderConfig,
        alpha: f32,  // 🆕 透明度参数（用于遮挡效果）
    ) -> GameResult<()> {
        if let Some(mlib) = get_map_library(tile.library_index) {
            if let Ok(mut mlib) = mlib.lock() {
                let (tile_w, tile_h) = mlib
                    .get_size(tile.image_index as usize)
                    .unwrap_or((CELL_WIDTH as i16, CELL_HEIGHT as i16));
                
                match mlib.get_or_create_texture(ctx, tile.image_index as usize) {
                    Ok(info) => {
                        if let Some(ref texture) = info.image {
                            let mut world_x = (tile.grid_x * CELL_WIDTH) as f32;
                            let world_y = (tile.grid_y * CELL_HEIGHT) as f32;

                            let mut adjusted_y = if (tile_w as i32 != CELL_WIDTH
                                || tile_h as i32 != CELL_HEIGHT)
                                && (tile_w as i32 != CELL_WIDTH * 2
                                    || tile_h as i32 != CELL_HEIGHT * 2)
                            {
                                world_y + CELL_HEIGHT as f32 - tile_h as f32
                            } else {
                                world_y
                            };

                            // 🔥 Front层混合模式偏移
                            if tile.use_blend && tile.layer == TileLayer::Front {
                                world_x = world_x - 1.0 * CELL_WIDTH as f32;
                                adjusted_y = adjusted_y - 4.0 * CELL_HEIGHT as f32;
                            }

                            let (screen_x, screen_y) =
                                CameraSystem::world_to_screen(pos, camera, world_x, adjusted_y);

                            // 🚀 屏幕剔除
                            let tile_screen_w = tile_w as f32 * camera.zoom;
                            let tile_screen_h = tile_h as f32 * camera.zoom;
                            
                            if tile.layer != TileLayer::Front {
                                if screen_x + tile_screen_w < 0.0 
                                    || screen_x > camera.screen_width
                                    || screen_y + tile_screen_h < 0.0
                                    || screen_y > camera.screen_height {
                                    return Ok(());
                                }
                            }

                            // 🔥 Front层使用ADD混合模式
                            let old_blend_mode = if tile.use_blend && tile.layer == TileLayer::Front {
                                let current = canvas.blend_mode();
                                canvas.set_blend_mode(graphics::BlendMode::ADD);
                                Some(current)
                            } else {
                                None
                            };

                            // 绘制瓦片（正常亮度，不使用遮挡透明度）
                            let color = Color::from_rgba(
                                (255.0 * tile.brightness) as u8,
                                (255.0 * tile.brightness) as u8,
                                (255.0 * tile.brightness) as u8,
                                255,  // 始终完全不透明
                            );

                            canvas.draw(
                                texture,
                                DrawParam::default()
                                    .dest([screen_x, screen_y])
                                    .scale([camera.zoom, camera.zoom])
                                    .color(color),
                            );

                            // 恢复混合模式
                            if let Some(old_mode) = old_blend_mode {
                                canvas.set_blend_mode(old_mode);
                            }

                            // 绘制边框 (调试用)
                            if config.show_borders {
                                let border_color = match tile.layer {
                                    TileLayer::Back => Color::from_rgb(255, 0, 0),
                                    TileLayer::Middle => Color::from_rgb(0, 255, 0),
                                    TileLayer::Front => Color::from_rgb(0, 150, 255),
                                };

                                let border = graphics::Mesh::new_rectangle(
                                    ctx,
                                    graphics::DrawMode::stroke(1.0),
                                    graphics::Rect::new(
                                        screen_x,
                                        screen_y,
                                        tile_screen_w,
                                        tile_screen_h,
                                    ),
                                    border_color,
                                )?;
                                canvas.draw(&border, DrawParam::default());
                            }
                        }
                    }
                    Err(_) => {}
                }
            }
        }

        Ok(())
    }

    /// 🎯 绘制Front层（带角色遮挡透明效果）
    pub fn draw_front_layer_with_occlusion(
        ctx: &mut Context,
        canvas: &mut Canvas,
        world: &World,
        pos: &Position,
        camera: &Camera,
        config: &RenderConfig,
        visible_area_entity: hecs::Entity,
        player_positions: &[(f32, f32)],
    ) -> GameResult<()> {
        use crate::ecs::Coordinates;
        
        if let Ok(visible_area) = world.get::<&VisibleArea>(visible_area_entity) {
            for &entity in &visible_area.visible_entities {
                if let Ok(tile) = world.get::<&MapTile>(entity) {
                    if !matches!(tile.layer, TileLayer::Front) {
                        continue;
                    }
                    if !config.show_front {
                        continue;
                    }
                    
                    // 检查瓦片是否与任何角色重叠
                    let mut has_overlap = false;
                    for &(player_x, player_y) in player_positions {
                        let (player_grid_x, player_grid_y) = Coordinates::world_to_grid(player_x, player_y);
                        let dx = (tile.grid_x - player_grid_x).abs();
                        let dy = player_grid_y - tile.grid_y;
                        
                        if dx <= 3 && dy >= 0 && dy <= 5 {
                            has_overlap = true;
                            break;
                        }
                    }
                    
                    if has_overlap {
                        Self::draw_tile_fast(ctx, canvas, &tile, pos, camera, config, 0.4)?;
                    } else {
                        Self::draw_tile_fast(ctx, canvas, &tile, pos, camera, config, 1.0)?;
                    }
                }
            }
        }
        
        Ok(())
    }

    /// 绘制单个瓦片（支持自定义透明度）
    fn draw_tile_with_alpha(
        ctx: &mut Context,
        canvas: &mut Canvas,
        tile: &MapTile,
        pos: &Position,
        camera: &Camera,
        _config: &RenderConfig,
        alpha: f32,
    ) -> GameResult<()> {
        if let Some(mlib) = get_map_library(tile.library_index) {
            if let Ok(mut mlib) = mlib.lock() {
                let (tile_w, tile_h) = mlib
                    .get_size(tile.image_index as usize)
                    .unwrap_or((CELL_WIDTH as i16, CELL_HEIGHT as i16));
                
                let (offset_x, offset_y) = mlib
                    .get_offset(tile.image_index as usize)
                    .unwrap_or((0, 0));
                
                match mlib.get_or_create_texture(ctx, tile.image_index as usize) {
                    Ok(info) => {
                        if let Some(ref texture) = info.image {
                            let mut world_x = (tile.grid_x * CELL_WIDTH) as f32;
                            let world_y = (tile.grid_y * CELL_HEIGHT) as f32;
                            
                            let mut adjusted_y = if (tile_w as i32 != CELL_WIDTH
                                || tile_h as i32 != CELL_HEIGHT)
                                && (tile_w as i32 != CELL_WIDTH * 2
                                    || tile_h as i32 != CELL_HEIGHT * 2)
                            {
                                world_y + CELL_HEIGHT as f32 - tile_h as f32
                            } else {
                                world_y
                            };
                            
                            if tile.use_blend {
                                adjusted_y -= 2.0;
                            }
                            
                            world_x += offset_x as f32;
                            let final_y = adjusted_y + offset_y as f32;
                            
                            let (screen_x, screen_y) =
                                CameraSystem::world_to_screen(pos, camera, world_x, final_y);
                            
                            // 🔥 Front层使用ADD混合模式
                            let old_blend_mode = if tile.use_blend {
                                let current = canvas.blend_mode();
                                canvas.set_blend_mode(graphics::BlendMode::ADD);
                                Some(current)
                            } else {
                                None
                            };
                            
                            let color = Color::from_rgba(255, 255, 255, (alpha * 255.0) as u8);
                            
                            canvas.draw(
                                texture,
                                graphics::DrawParam::default()
                                    .dest([screen_x, screen_y])
                                    .scale([camera.zoom, camera.zoom])
                                    .color(color),
                            );
                            
                            if let Some(old_mode) = old_blend_mode {
                                canvas.set_blend_mode(old_mode);
                            }
                        }
                    }
                    Err(_) => {}
                }
            }
        }
        
        Ok(())
    }
}

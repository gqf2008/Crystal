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
            // 元组: (grid_x, grid_y, lib_index, img_index, is_anim, use_blend)
            let mut tiles_to_draw: Vec<(i32, i32, i16, usize, bool, bool)> = Vec::new();
            
            // Front 层需要更大的底部视口（建筑物是长条形，UV坐标在左下角）
            let bottom_extra = if matches!(layer, TileLayer::Front) {
                800.0  // Front 层底部额外扩展 800 像素
            } else {
                200.0  // Back/Middle 层保持 200 像素
            };
            
            // 先收集所有有动画的瓦片实体（避免重复绘制）
            use crate::ecs::components::AnimatedTile;
            let mut animated_entities = std::collections::HashSet::new();
            let current_layer = *layer;  // 解引用以便在 filter 中使用
            for (entity, (tile, _)) in world.query::<(&MapTile, &AnimatedTile)>().iter().filter(|(_, (t, _))| t.layer == current_layer) {
                animated_entities.insert(entity);
            }
            
            // 静态瓦片（排除有动画的）
            if config.show_static_tiles {  // ✅ 静态瓦片开关
                for (entity, tile) in world.query::<&MapTile>().iter().filter(|(_, t)| t.layer == current_layer) {
                    // 如果这个实体有动画，跳过（稍后在动画瓦片部分绘制）
                    if animated_entities.contains(&entity) {
                        continue;
                    }
                
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
                        tile.use_blend,
                    ));
                }
            }  // ✅ 静态瓦片 if 块结束
            
            // 动画瓦片（使用 AnimatedTile 计算当前帧）
            // ✅ 动画瓦片开关（独立于 show_animations 播放控制）
            if config.show_animated_tiles {
                // 根据 config.show_animations 决定是否播放动画（暂停时仍显示第一帧）
                if config.show_animations {
                    for (_, (tile, anim)) in world.query::<(&MapTile, &AnimatedTile)>().iter().filter(|(_, (t, _))| t.layer == current_layer) {
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
                            tile.use_blend,
                        ));
                    }
                }
            }
            
            // 渲染该层的瓦片
            for (grid_x, grid_y, lib_index, img_index, is_anim, use_blend) in tiles_to_draw {
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
                                
                                // MapEditor 渲染逻辑 (Main.cs:785-1030)
                                // 
                                // Back层 (line 971-993):
                                //   永远: drawY = Y * CellHeight
                                //
                                // Middle层 (line 919-970):
                                //   - 标准尺寸: drawY = Y * CellHeight
                                //   - 非标准尺寸: drawY = (Y+1) * CellHeight - Height
                                //
                                // Front层 (line 785-876):
                                //   - 标准尺寸: drawY = Y * CellHeight
                                //   - 非标准尺寸 + 无动画: drawY = (Y+1) * CellHeight - Height
                                //   - 非标准尺寸 + 动画 + 混合 + 新地图库(100-199): drawY = (Y+1) * CellHeight - 3*CellHeight
                                //   - 非标准尺寸 + 动画(其他情况): drawY = (Y+1) * CellHeight - Height
                                
                                let is_standard_size = (tile_w == CELL_WIDTH as i16 && tile_h == CELL_HEIGHT as i16)
                                    || (tile_w == CELL_WIDTH as i16 * 2 && tile_h == CELL_HEIGHT as i16 * 2);
                                
                                let adjusted_x = world_x;
                                let adjusted_y = if matches!(layer, TileLayer::Back) {
                                    // Back层: 永远直接使用Y坐标
                                    world_y
                                } else if is_standard_size {
                                    // Middle层和Front层的标准尺寸: 直接使用Y坐标
                                    world_y
                                } else {
                                    // 非标准尺寸的偏移计算
                                    // GameScene.cs:11967-11972 - blend 特殊处理仅用于 Front 层
                                    if matches!(layer, TileLayer::Front) && use_blend {
                                        // Front层 + 混合模式: 检查库索引
                                        if lib_index == 14 || lib_index == 27 || (lib_index > 99 && lib_index < 199) {
                                            // 特殊库: 使用 -3*CellHeight 偏移（灯光效果）
                                            // C#: drawY - (3 * CellHeight) = (y+1)*48 - 144 = (y-2)*48
                                            // Rust: world_y = y*48, 所以 y*48 - 2*48 = (y-2)*48
                                            world_y - (2 * CELL_HEIGHT) as f32
                                        } else {
                                            // 普通混合: 使用标准非标准偏移
                                            // C#: drawY - s.Height = (y+1)*48 - Height
                                            // Rust: y*48 + 48 - Height
                                            world_y + CELL_HEIGHT as f32 - tile_h as f32
                                        }
                                    } else {
                                        // 非 blend 或非 Front 层: 标准非标准尺寸偏移
                                        // C#: drawY - s.Height (MapEditor 和 Middle 层都用这个)
                                        world_y + CELL_HEIGHT as f32 - tile_h as f32
                                    }
                                };
                                
                                // 判断是否应用图像内部偏移（基于C# GameScene.cs:11967-11980）
                                let should_apply_offset = if matches!(layer, TileLayer::Front) {
                                    if use_blend {
                                        // Blend tiles: special libs (14/27/100-199) OR specific indices (2723-2732)
                                        lib_index == 14 || lib_index == 27 || 
                                        (lib_index > 99 && lib_index < 199) ||
                                        (img_index >= 2723 && img_index <= 2732)
                                    } else if lib_index == 28 {
                                        // Lib 28: apply offset if non-empty
                                        info.x != 0 || info.y != 0
                                    } else {
                                        false
                                    }
                                } else {
                                    // Back/Middle layers: never apply offset
                                    false
                                };

                                let (adjusted_x_final, adjusted_y_final) = if should_apply_offset {
                                    (adjusted_x + info.x as f32, adjusted_y + info.y as f32)
                                } else {
                                    (adjusted_x, adjusted_y)
                                };
                                
                                // 世界坐标 -> 屏幕坐标
                                let screen_x = (adjusted_x_final - camera_pos.x) * camera.zoom
                                    + camera.screen_width / 2.0;
                                let screen_y = (adjusted_y_final - camera_pos.y) * camera.zoom
                                    + camera.screen_height / 2.0;
                                
                                // 🔥 如果需要混合模式（火焰等动画），使用 ADD 混合
                                let old_blend_mode = if use_blend {
                                    let current = canvas.blend_mode();
                                    canvas.set_blend_mode(ggez::graphics::BlendMode::ADD);
                                    Some(current)
                                } else {
                                    None
                                };
                                
                                // 绘制（使用白色作为颜色，确保 alpha 混合正确）
                                canvas.draw(
                                    image,
                                    DrawParam::default()
                                        .dest([screen_x, screen_y])
                                        .scale([camera.zoom, camera.zoom])
                                        .color(ggez::graphics::Color::WHITE),
                                );
                                
                                // 恢复原来的混合模式
                                if let Some(old_mode) = old_blend_mode {
                                    canvas.set_blend_mode(old_mode);
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
}

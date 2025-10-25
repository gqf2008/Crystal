// ============================================================================
// Render System - 渲染系统
// ============================================================================
//
// ✅ 完整实现 - 已从 map_viewer_ecs.rs 迁移:
//   - draw_tiles: 瓦片渲染 + 视口裁剪 + LOD优化 (完整)
//   - draw_tile_fast: 单个瓦片快速渲染 (完整)
//   - draw_player: 角色渲染 + 装备系统 (完整)
//   - draw_path: 寻路路径可视化(调试) (完整)
//   - draw_tiles_instanced: 批量渲染优化 (完整)
//   - draw_grid: 网格绘制 (完整)
//   - draw_obstacles: 障碍物可视化 (完整)

use ggez::{Context, GameResult};
use ggez::graphics::{Canvas, DrawParam, Color, BlendMode, BlendComponent, BlendFactor, BlendOperation};
use hecs::World;

// 从共享模块导入类型
use crate::ecs::components::{
    Position, Camera, RenderConfig, MapTile, Player,
};

/// 渲染系统
pub struct RenderSystem;

impl RenderSystem {
    /// 创建 ADD 混合模式 (火焰/特效)
    pub fn create_blend_mode() -> BlendMode {
        BlendMode {
            color: BlendComponent {
                src_factor: BlendFactor::One,
                dst_factor: BlendFactor::One,
                operation: BlendOperation::Add,
            },
            alpha: BlendComponent {
                src_factor: BlendFactor::One,
                dst_factor: BlendFactor::One,
                operation: BlendOperation::Add,
            },
        }
    }

    /// 绘制所有瓦片 (带视口裁剪优化 + 屏幕剔除)
    pub fn draw_tiles(
        ctx: &mut Context,
        canvas: &mut Canvas,
        world: &World,
        pos: &Position,
        camera: &Camera,
        config: &RenderConfig,
        visible_area_entity: hecs::Entity,
    ) -> GameResult<()> {
        use crate::ecs::{TileLayer, MapTile, VisibleArea};
        use std::time::Instant;
        
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
        // 即使缩放很小，也保持至少2格的buffer以避免黑块
        let base_buffer = if camera.zoom < 0.4 { 2 } else { 3 };
        let buffer = ((base_buffer as f32 * projection_scale).ceil() as i32).max(2).min(8);

        // 转换为地图格子坐标
        let start_x = ((left / CELL_WIDTH as f32).floor() as i32 - buffer).max(0);
        let end_x = (right / CELL_WIDTH as f32).ceil() as i32 + buffer;
        let start_y = ((top / CELL_HEIGHT as f32).floor() as i32 - buffer).max(0);
        let end_y = (bottom / CELL_HEIGHT as f32).ceil() as i32 + buffer;

        // 🎨 Front层特殊处理：向下扩展更多格子（建筑物可能很高）
        // 🏢 重要：Front层必须保持足够扩展，否则屏幕底部建筑物顶部会被裁剪
        let front_extra_cells = ((15.0 * projection_scale).ceil() as i32).max(8).min(30);
        let front_end_y = end_y + front_extra_cells;

        // 🔍 检测可见区域或缩放是否变化
        if let Ok(mut visible_area) = world.get::<&mut VisibleArea>(visible_area_entity) {
            let min_cell_threshold = 1; // 🔧 修复：移动1格就重建，避免边缘黑块
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
                // 🐛 调试：输出可见区域变化信息
                unsafe {
                    if DRAW_COUNTER % 300 == 0 {
                        tracing::info!(
                            "🔍 可见区域: grid_range=({},{})-({},{}), front_end_y={}, tiles将重建",
                            start_x, start_y, end_x, end_y, front_end_y
                        );
                    }
                }
                
                visible_area.visible_entities.clear();

                // 🔥 收集可见实体（带 z_order 和 Y 坐标用于排序）
                let mut visible_with_sort_key: Vec<(hecs::Entity, i32, i32)> = Vec::new();

                // 🎯 LOD优化：暂时禁用（因为棋盘剔除会导致移动时闪烁）
                // 如果需要 LOD，应该使用固定的世界坐标而非格子坐标进行判断
                let lod_skip = if config.enable_lod && camera.zoom < 0.5 {
                    true
                } else {
                    false
                };

                // 查询所有瓦片并过滤
                for (entity, tile) in world.query::<&MapTile>().iter() {
                    // LOD 已禁用，不再跳过任何瓦片
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
                        // 🎯 使用 z_order 作为主排序键，Y坐标作为次排序键
                        visible_with_sort_key.push((entity, tile.z_order, tile.grid_y));
                    }
                }

                // 🎯 按 Z轴排序（z_order 优先，相同则按 Y 坐标）
                visible_with_sort_key.sort_by(|a, b| {
                    match a.1.cmp(&b.1) {
                        std::cmp::Ordering::Equal => a.2.cmp(&b.2),  // z_order 相同则按 Y
                        other => other,  // 否则按 z_order
                    }
                });

                // 只保存实体ID
                visible_area.visible_entities = visible_with_sort_key.into_iter().map(|(e, _, _)| e).collect();

                // 🐛 调试：输出瓦片统计
                unsafe {
                    if DRAW_COUNTER % 300 == 0 {
                        tracing::info!(
                            "📊 可见瓦片数量: total={}",
                            visible_area.visible_entities.len()
                        );
                    }
                }

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

            // 🎯 绘制缓存的可见瓦片（实时读取MapTile以支持动画）
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

                    // 按混合模式分组
                    if tile.use_blend {
                        blend_tiles.push(tile);
                    } else {
                        normal_tiles.push(tile);
                    }
                }
            }

            // 先渲染普通瓦片（一次设置混合模式）
            if !normal_tiles.is_empty() {
                canvas.set_blend_mode(BlendMode::ALPHA);
                for tile in normal_tiles {
                    Self::draw_tile_fast(ctx, canvas, &tile, pos, camera, config)?;
                }
            }

            // 再渲染混合瓦片（一次设置混合模式）
            if !blend_tiles.is_empty() {
                canvas.set_blend_mode(Self::create_blend_mode());
                for tile in blend_tiles {
                    Self::draw_tile_fast(ctx, canvas, &tile, pos, camera, config)?;
                }
                canvas.set_blend_mode(BlendMode::ALPHA);
            }
        }

        Ok(())
    }

    /// 🎯 绘制Front层（带角色遮挡透明效果）
    /// 当Front层瓦片与角色位置重叠时，使用半透明绘制
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
        use crate::ecs::{TileLayer, MapTile, VisibleArea};
        use crate::ecs::Coordinates;
        
        // 获取可见区域的瓦片
        if let Ok(visible_area) = world.get::<&VisibleArea>(visible_area_entity) {
            for &entity in &visible_area.visible_entities {
                if let Ok(tile) = world.get::<&MapTile>(entity) {
                    // 只处理Front层
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
                        
                        // 检查瓦片是否在角色上方附近
                        let dx = (tile.grid_x - player_grid_x).abs();
                        let dy = player_grid_y - tile.grid_y; // 正值表示瓦片在角色上方
                        
                        // 如果瓦片在角色上方且X方向接近，认为有遮挡
                        if dx <= 3 && dy >= 0 && dy <= 5 {
                            has_overlap = true;
                            break;
                        }
                    }
                    
                    // 根据是否重叠设置不同的透明度
                    if has_overlap {
                        // 🎯 半透明绘制（alpha=0.4，让玩家能看到角色）
                        Self::draw_tile_with_alpha(ctx, canvas, &tile, pos, camera, config, 0.4)?;
                    } else {
                        // 正常绘制
                        Self::draw_tile_fast(ctx, canvas, &tile, pos, camera, config)?;
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
        use crate::ecs::{CELL_WIDTH, CELL_HEIGHT};
        use crate::ecs::systems::CameraSystem;
        use crate::graphics::libraries::get_map_library;
        use ggez::graphics::Color;
        
        if let Some(mlib) = get_map_library(tile.library_index) {
            if let Ok(mut mlib) = mlib.lock() {
                // 先获取所有需要的信息
                let (tile_w, tile_h) = mlib
                    .get_size(tile.image_index as usize)
                    .unwrap_or((CELL_WIDTH as i16, CELL_HEIGHT as i16));
                
                let (offset_x, offset_y) = mlib
                    .get_offset(tile.image_index as usize)
                    .unwrap_or((0, 0));
                
                // 然后获取纹理
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
                            
                            // Front层混合模式偏移
                            if tile.use_blend {
                                adjusted_y -= 2.0;
                            }
                            
                            // 应用偏移量
                            world_x += offset_x as f32;
                            let final_y = adjusted_y + offset_y as f32;
                            
                            let (screen_x, screen_y) =
                                CameraSystem::world_to_screen(pos, camera, world_x, final_y);
                            
                            // 🎯 使用自定义透明度
                            let color = Color::from_rgba(255, 255, 255, (alpha * 255.0) as u8);
                            
                            canvas.draw(
                                texture,
                                ggez::graphics::DrawParam::default()
                                    .dest([screen_x, screen_y])
                                    .scale([camera.zoom, camera.zoom])
                                    .color(color),
                            );
                        }
                    }
                    Err(_) => {}
                }
            }
        }
        
        Ok(())
    }

    /// 绘制单个瓦片（快速版：不切换混合模式）
    pub fn draw_tile_fast(
        ctx: &mut Context,
        canvas: &mut Canvas,
        tile: &MapTile,
        pos: &Position,
        camera: &Camera,
        config: &RenderConfig,
    ) -> GameResult<()> {
        use crate::ecs::{CELL_WIDTH, CELL_HEIGHT, TileLayer};
        use crate::ecs::systems::CameraSystem;
        use crate::graphics::libraries::get_map_library;
        use ggez::graphics;
        
        if let Some(mlib) = get_map_library(tile.library_index) {
            if let Ok(mut mlib) = mlib.lock() {
                // 先获取尺寸
                let (tile_w, tile_h) = mlib
                    .get_size(tile.image_index as usize)
                    .unwrap_or((CELL_WIDTH as i16, CELL_HEIGHT as i16));
                
                // 再获取纹理
                match mlib.get_or_create_texture(ctx, tile.image_index as usize) {
                    Ok(info) => {
                        if let Some(ref texture) = info.image {
                        // 计算世界坐标
                        let mut world_x = (tile.grid_x * CELL_WIDTH) as f32;
                        let world_y = (tile.grid_y * CELL_HEIGHT) as f32;

                        // 调整Y坐标 (大型物体需要向上偏移)
                        let mut adjusted_y = if (tile_w as i32 != CELL_WIDTH
                            || tile_h as i32 != CELL_HEIGHT)
                            && (tile_w as i32 != CELL_WIDTH * 2
                                || tile_h as i32 != CELL_HEIGHT * 2)
                        {
                            world_y + CELL_HEIGHT as f32 - tile_h as f32
                        } else {
                            world_y
                        };

                        // 🔥 Front层混合模式偏移（火焰、光效等特效）
                        if tile.use_blend && tile.layer == TileLayer::Front {
                            world_x = world_x - 1.0 * CELL_WIDTH as f32;
                            adjusted_y = adjusted_y - 4.0 * CELL_HEIGHT as f32;
                        }

                        // 世界坐标转屏幕坐标
                        let (screen_x, screen_y) =
                            CameraSystem::world_to_screen(pos, camera, world_x, adjusted_y);

                        // 🚀 屏幕剔除：如果完全在屏幕外，跳过绘制
                        // 🏢 但 Front 层不做剔除，因为高建筑物纹理是长条状的
                        let tile_screen_w = tile_w as f32 * camera.zoom;
                        let tile_screen_h = tile_h as f32 * camera.zoom;
                        
                        // 只对 Back 和 Middle 层做屏幕外剔除
                        if tile.layer != TileLayer::Front {
                            if screen_x + tile_screen_w < 0.0 
                                || screen_x > camera.screen_width
                                || screen_y + tile_screen_h < 0.0
                                || screen_y > camera.screen_height {
                                return Ok(());
                            }
                        }

                        // 绘制瓦片（不切换混合模式，外部已设置）
                        let color = Color::from_rgba(
                            (255.0 * tile.brightness) as u8,
                            (255.0 * tile.brightness) as u8,
                            (255.0 * tile.brightness) as u8,
                            255,
                        );

                        canvas.draw(
                            texture,
                            DrawParam::default()
                                .dest([screen_x, screen_y])
                                .scale([camera.zoom, camera.zoom])
                                .color(color),
                        );

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

    /// 绘制角色
    pub fn draw_player(
        ctx: &mut Context,
        canvas: &mut Canvas,
        player: &Player,
        player_pos: &Position,
        camera_pos: &Position,
        camera: &Camera,
    ) -> GameResult<()> {
        use crate::graphics::libraries::{get_library, LibraryName};
        use crate::ecs::systems::CameraSystem;
        
        // 🎨 使用 CArmours(0) 库绘制角色（默认装备）
        // CArmours 库帧布局（参考 player_object.rs）:
        //   - Standing: 0-31   (8方向 * 4帧)
        //   - Walking:  32-79  (8方向 * 6帧)
        //   - Running:  80-127 (8方向 * 6帧)
        //   - Attack1:  128-175 (8方向 * 6帧)
        //
        // 公式: DrawFrame = action_frame_start + direction * frames_per_direction + frame_index
        //       FinalFrame = DrawFrame + ArmourOffSet (Male=0, Female=808)
        
        // 计算 DrawFrame
        let action_frame_start = player.action.frame_start();
        let frames_per_direction = player.action.frame_count();
        let direction_offset = (player.direction as i32) * frames_per_direction;
        let draw_frame = action_frame_start + direction_offset + player.frame_index;
        
        // 暂不考虑性别偏移（默认男性，偏移=0）
        let armour_offset = 0;
        let final_frame = draw_frame + armour_offset;
        
        // 🐛 DEBUG: 首次绘制打印帧信息
        static mut FIRST_DRAW: bool = true;
        unsafe {
            if FIRST_DRAW {
                let dir_name = match player.direction {
                    0 => "Up(上)",
                    1 => "UpRight(右上)",
                    2 => "Right(右)",
                    3 => "DownRight(右下)",
                    4 => "Down(下)",
                    5 => "DownLeft(左下)",
                    6 => "Left(左)",
                    7 => "UpLeft(左上)",
                    _ => "Unknown",
                };
                
                println!("\n🎨 === 角色帧计算调试 ===");
                println!("动作: {:?}", player.action);
                println!("方向: {} - {}", player.direction, dir_name);
                println!("当前帧索引: {}/{}", player.frame_index, frames_per_direction);
                println!("动作起始帧: {}", action_frame_start);
                println!("方向偏移: {} (方向{} * 每方向{}帧)", direction_offset, player.direction, frames_per_direction);
                println!("DrawFrame: {} + {} + {} = {}", action_frame_start, direction_offset, player.frame_index, draw_frame);
                println!("性别偏移: {}", armour_offset);
                println!("FinalFrame: {} + {} = {}", draw_frame, armour_offset, final_frame);
                println!("使用库: CArmours(0)");
                println!("========================\n");
                FIRST_DRAW = false;
            }
        }
        
        // ✅ 获取角色纹理 - 使用正确的角色库（不是地图库！）
        if let Some(mlib) = get_library(LibraryName::CArmours(0)) {
            if let Ok(mut mlib) = mlib.lock() {
                // 获取尺寸和偏移量
                let (char_w, char_h) = mlib
                    .get_size(final_frame as usize)
                    .unwrap_or((48, 64));
                
                let (_offset_x, _offset_y) = mlib
                    .get_offset(final_frame as usize)
                    .unwrap_or((0, 0));
                
                // 获取纹理
                match mlib.get_or_create_texture(ctx, final_frame as usize) {
                    Ok(info) => {
                        if let Some(ref texture) = info.image {
                            // 🎯 纹理位置计算:
                            // player_pos 现在是格子中心(红点)
                            // 纹理底边应该对齐格子底边，X轴居中
                            use crate::ecs::{CELL_HEIGHT};
                            let green_bottom_y = player_pos.y + (CELL_HEIGHT as f32 / 2.0);
                            let world_x = player_pos.x - (char_w as f32 / 2.0);
                            let world_y = green_bottom_y - char_h as f32;
                            
                            let (screen_x, screen_y) = CameraSystem::world_to_screen(
                                camera_pos, 
                                camera, 
                                world_x,
                                world_y
                            );
                            
                            // 🎯 角色与Front层使用ADD混合
                            // 当角色在树木、建筑等Front层物体下方时，使用ADD混合实现半透明遮挡效果
                            canvas.set_blend_mode(Self::create_blend_mode());
                            canvas.draw(
                                texture,
                                DrawParam::default()
                                    .dest([screen_x, screen_y])
                                    .scale([camera.zoom, camera.zoom])
                                    .color(Color::WHITE),
                            );
                            // 恢复默认混合模式
                            canvas.set_blend_mode(BlendMode::ALPHA);
                        }
                    }
                    Err(_) => {}
                }
            }
        }
        
        Ok(())
    }

    /// 绘制角色（带Front层重叠检测）
    pub fn draw_player_with_world(
        ctx: &mut Context,
        canvas: &mut Canvas,
        world: &World,
        player: &Player,
        player_pos: &Position,
        camera_pos: &Position,
        camera: &Camera,
    ) -> GameResult<()> {
        use crate::graphics::libraries::{get_library, LibraryName};
        use crate::ecs::systems::CameraSystem;
        use crate::ecs::components::LocalPlayer;
        
        // 🔒 只有收到服务器位置确认后才绘制人物纹理
        // 检查玩家实体是否有 NetworkSync 组件
        let mut has_server_position = false;
        for (_entity, (_local, _network_sync)) in world.query::<(&LocalPlayer, &crate::ecs::components::NetworkSync)>().iter() {
            // 玩家实体有 NetworkSync 组件,说明已经收到服务器位置
            has_server_position = true;
            break;
        }
        
        if !has_server_position {
            // 还没收到服务器位置,不绘制纹理
            return Ok(());
        }
        use crate::ecs::components::{MapTile, TileLayer, PlayerAppearance};
        use mir2_shared::enums::{MirClass, MirGender};
        
        // � 尝试获取 PlayerAppearance 组件（如果存在）
        let appearance = world.query::<&PlayerAppearance>()
            .iter()
            .next()
            .map(|(_, app)| app.clone());
        
        // 如果没有外观组件，使用默认值
        let (class, gender, armour_index) = if let Some(app) = &appearance {
            (app.class, app.gender, app.armour)
        } else {
            // 默认：战士，男性，盔甲索引0
            (MirClass::Warrior, MirGender::Male, 0)
        };
        
        // 🎨 根据职业和性别选择库
        // 重要：原版C#客户端所有职业都使用 CArmours 库（男女通用）
        // 性别差异通过 ArmourOffSet 帧偏移实现:
        //   - 男性: offset = 0
        //   - 女性: offset = 808 (普通动作) 或 352 (altAnim: 跑步/射箭)
        let library_index = armour_index.max(0);
        let library_name = LibraryName::CArmours(library_index as usize);
        
        // 计算 DrawFrame
        let action_frame_start = player.action.frame_start();
        let frames_per_direction = player.action.frame_count();
        let direction_offset = (player.direction as i32) * frames_per_direction;
        let draw_frame = action_frame_start + direction_offset + player.frame_index;
        
        // 🚺 性别帧偏移（原版逻辑）
        // TODO: 支持 altAnim (跑步/射箭等特殊动作使用 ARArmours 库和不同偏移)
        let armour_offset = match gender {
            MirGender::Male => 0,
            MirGender::Female => 808,  // 女性普通动作偏移
        };
        let final_frame = draw_frame + armour_offset;
        
        tracing::debug!("🎭 角色渲染: class={:?}, gender={:?}, armour={}, action={:?}, draw_frame={}, offset={}, final={}", 
            class, gender, armour_index, player.action, draw_frame, armour_offset, final_frame);
        
        // 🎯 遮挡检测：检测角色是否被Front层瓦片遮挡
        // 遮挡条件：
        // 1. Front层瓦片的世界Y坐标 <= 角色脚底Y坐标（瓦片在前面绘制）
        // 2. Front层瓦片在屏幕空间与角色有重叠
        use crate::ecs::Coordinates;
        use crate::ecs::{CELL_WIDTH, CELL_HEIGHT};
        
        let mut _has_front_overlap = false;
        
        // 角色脚底的世界坐标和格子坐标
        let player_world_x = player_pos.x;
        let player_world_y = player_pos.y;
        let (player_grid_x, player_grid_y) = Coordinates::world_to_grid(player_world_x, player_world_y);
        
        // 预先获取角色的尺寸信息（用于碰撞检测）
        // 🎯 使用稍大的检测范围，避免边缘临界状态导致闪烁
        let char_height = 80.0; // 角色大约高度（加大检测范围）
        let char_width = 64.0;  // 角色大约宽度（加大检测范围）
        
        for (_, tile) in world.query::<&MapTile>().iter() {
            // 只检查Front层
            if !matches!(tile.layer, TileLayer::Front) {
                continue;
            }
            
            // 瓦片的世界坐标（左上角）
            let tile_world_x = (tile.grid_x * CELL_WIDTH) as f32;
            let tile_world_y = (tile.grid_y * CELL_HEIGHT) as f32;
            
            // 条件1: Front层瓦片的Y坐标 <= 角色的Y坐标（格子空间）
            // 即瓦片在角色前面或同一行
            if tile.grid_y > player_grid_y {
                continue; // 瓦片在角色后面，不会遮挡
            }
            
            // 条件2: 在世界空间检查X方向重叠
            // Front层瓦片通常比较大（树木、建筑），需要获取实际尺寸
            // 简化处理：假设Front层瓦片至少覆盖 2x2 格子 (96x64)
            let tile_width = CELL_WIDTH as f32 * 2.0;  // 假设宽度
            let tile_height = CELL_HEIGHT as f32 * 3.0; // 假设高度（建筑物可能更高）
            
            // 角色的包围盒（以脚底为基准）
            let char_left = player_world_x - char_width / 2.0;
            let char_right = player_world_x + char_width / 2.0;
            let char_top = player_world_y - char_height;
            let char_bottom = player_world_y;
            
            // 瓦片的包围盒
            let tile_left = tile_world_x;
            let tile_right = tile_world_x + tile_width;
            let tile_top = tile_world_y;
            let tile_bottom = tile_world_y + tile_height;
            
            // AABB碰撞检测
            let x_overlap = char_right > tile_left && char_left < tile_right;
            let y_overlap = char_bottom > tile_top && char_top < tile_bottom;
            
            if x_overlap && y_overlap {
                _has_front_overlap = true;
                break;
            }
        }
        // 纹理尺寸和偏移量 (用于后续 AABB 计算)
        let mut char_w = 48;
        let mut char_h = 64;
        
        // ✅ 获取角色纹理 - 根据职业和性别使用对应的库
        if let Some(mlib) = get_library(library_name) {
            if let Ok(mut mlib) = mlib.lock() {
                // 获取尺寸和偏移量
                (char_w, char_h) = mlib
                    .get_size(final_frame as usize)
                    .unwrap_or((48, 64));
                
                let (_offset_x, _offset_y) = mlib
                    .get_offset(final_frame as usize)
                    .unwrap_or((0, 0));
                
                // 获取纹理
                match mlib.get_or_create_texture(ctx, final_frame as usize) {
                    Ok(info) => {
                        if let Some(ref texture) = info.image {
                            // 🎯 纹理位置计算:
                            // player_pos 现在是格子中心(红点)
                            // 纹理底边应该对齐格子底边
                            // 
                            // 格子底边Y = player_pos.y + CELL_HEIGHT/2
                            // 纹理底边Y = world_y + char_h
                            // 所以: world_y = player_pos.y + CELL_HEIGHT/2 - char_h
                            // 
                            // ⚠️ X方向: 原工程中角色在格子右侧，所以需要向右偏移
                            // 格子中心 + 半格宽度 = 格子右边缘
                            let green_bottom_y = player_pos.y + (CELL_HEIGHT as f32 / 2.0);
                            let world_x = player_pos.x + (CELL_WIDTH as f32 / 2.0) - (char_w as f32 / 2.0);
                            let world_y = green_bottom_y - char_h as f32;
                            
                            let (screen_x, screen_y) = CameraSystem::world_to_screen(
                                camera_pos, 
                                camera, 
                                world_x,
                                world_y
                            );
                            
                            // 🎯 角色始终保持完全可见（不改变透明度）
                            // 遮挡效果由渲染顺序控制：Front层绘制在角色之后
                            let color = Color::WHITE;
                            
                            canvas.draw(
                                texture,
                                DrawParam::default()
                                    .dest([screen_x, screen_y])
                                    .scale([camera.zoom, camera.zoom])
                                    .color(color),
                            );
                        }
                    }
                    Err(_) => {}
                }
            }
        }
        
        // 🐛 调试绘制:显示碰撞检测和渲染相关的边界
        use ggez::graphics;
        
        // 1. 绘制人物所在格子边界(绿色) - 用于移动碰撞检测
        //    服务器检查: ValidPoint(格子是否可行走) + cell.Objects(格子内对象阻挡)
        //    红点(player_pos)应该在绿框的中心位置
        let grid_world_x = player_pos.x - (CELL_WIDTH as f32 / 2.0);  // 格子左边 = 中心 - 半格宽
        let grid_world_y = player_pos.y - (CELL_HEIGHT as f32 / 2.0);  // 格子顶边 = 中心 - 半格高
        let (grid_screen_x, grid_screen_y) = CameraSystem::world_to_screen(
            camera_pos,
            camera,
            grid_world_x,
            grid_world_y,
        );
        
        let grid_rect = graphics::Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::stroke(2.0),
            graphics::Rect::new(
                grid_screen_x,
                grid_screen_y,
                CELL_WIDTH as f32 * camera.zoom,
                CELL_HEIGHT as f32 * camera.zoom,
            ),
            Color::from_rgb(0, 255, 0), // 绿色边框
        )?;
        canvas.draw(&grid_rect, DrawParam::default());
        
        // 2. 绘制人物包围盒(黄色) - 应该完全包裹人物纹理
        //    AABB用于与Front层瓦片做遮挡检测
        //    黄框底边应该与绿框底边对齐，X轴居中对齐
        //    绿框底边Y = player_pos.y + CELL_HEIGHT/2
        //    黄框底边Y = char_top + char_h = 绿框底边Y
        //    所以: char_top = player_pos.y + CELL_HEIGHT/2 - char_h
        
        let green_bottom_y = player_pos.y + (CELL_HEIGHT as f32 / 2.0);  // 绿框底边
        let char_left = player_pos.x - (char_w as f32 / 2.0);  // 黄框X居中对齐
        let char_top = green_bottom_y - char_h as f32;  // 黄框底边对齐绿框底边
        let (char_screen_x, char_screen_y) = CameraSystem::world_to_screen(
            camera_pos,
            camera,
            char_left,
            char_top,
        );
        
        let char_rect = graphics::Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::stroke(2.0),
            graphics::Rect::new(
                char_screen_x,
                char_screen_y,
                char_w as f32 * camera.zoom,
                char_h as f32 * camera.zoom,
            ),
            Color::from_rgb(255, 255, 0), // 黄色边框
        )?;
        canvas.draw(&char_rect, DrawParam::default());
        
        // 3. 绘制人物脚底中心点(红色圆点) - Position组件的实际位置
        //    这是角色的锚点,用于计算纹理渲染位置和格子坐标
        let (foot_screen_x, foot_screen_y) = CameraSystem::world_to_screen(
            camera_pos,
            camera,
            player_pos.x,
            player_pos.y,
        );
        
        let foot_circle = graphics::Mesh::new_circle(
            ctx,
            graphics::DrawMode::fill(),
            [foot_screen_x, foot_screen_y],
            4.0 * camera.zoom,
            0.1,
            Color::from_rgb(255, 0, 0), // 红色圆点
        )?;
        canvas.draw(&foot_circle, DrawParam::default());
        
        // 4. 绘制坐标文字
        let coord_text = graphics::Text::new(format!("({}, {})", player_grid_x, player_grid_y));
        canvas.draw(
            &coord_text,
            DrawParam::default()
                .dest([grid_screen_x + 5.0, grid_screen_y + 5.0])
                .color(Color::from_rgb(255, 255, 255))
                .scale([0.8, 0.8]),
        );
        
        Ok(())
    }

    /// 绘制寻路路径 (调试用)
    pub fn draw_path(
        ctx: &mut Context,
        canvas: &mut Canvas,
        world: &World,
        camera_pos: &Position,
        camera: &Camera,
    ) -> GameResult<()> {
        use crate::ecs::{Player, Coordinates};
        use crate::ecs::systems::CameraSystem;
        use ggez::graphics;
        
        // 查询玩家的路径信息
        for (_entity, (player, player_pos)) in world.query::<(&Player, &Position)>().iter() {
            if player.path.is_empty() {
                continue;
            }

            // 绘制从当前位置到第一个路径点的线段
            if let Some(&(first_x, first_y)) = player.path.get(player.path_index) {
                // 第一个路径点的世界坐标
                let (first_world_x, first_world_y) = Coordinates::grid_to_world_center(first_x, first_y);
                
                // 转换到屏幕坐标
                let (player_screen_x, player_screen_y) = CameraSystem::world_to_screen(
                    camera_pos, camera, player_pos.x, player_pos.y
                );
                let (first_screen_x, first_screen_y) = CameraSystem::world_to_screen(
                    camera_pos, camera, first_world_x, first_world_y
                );
                
                // 🎯 绘制连接线前,检查坐标是否合理
                // 即使超出屏幕也绘制,但避免极端值导致的渲染问题
                if player_screen_x.is_finite() && player_screen_y.is_finite() 
                    && first_screen_x.is_finite() && first_screen_y.is_finite() {
                    // 绘制连接线 (黄色)
                    let line = graphics::Mesh::new_line(
                        ctx,
                        &[[player_screen_x, player_screen_y], [first_screen_x, first_screen_y]],
                        2.0,
                        Color::from_rgb(255, 255, 0), // 黄色
                    )?;
                    canvas.draw(&line, DrawParam::default());
                }
            }

            // 绘制路径点之间的连接线
            for i in player.path_index..(player.path.len() - 1) {
                let (x1, y1) = player.path[i];
                let (x2, y2) = player.path[i + 1];
                
                // 转换到世界坐标
                let (world_x1, world_y1) = Coordinates::grid_to_world_center(x1, y1);
                let (world_x2, world_y2) = Coordinates::grid_to_world_center(x2, y2);
                
                // 转换到屏幕坐标
                let (screen_x1, screen_y1) = CameraSystem::world_to_screen(
                    camera_pos, camera, world_x1, world_y1
                );
                let (screen_x2, screen_y2) = CameraSystem::world_to_screen(
                    camera_pos, camera, world_x2, world_y2
                );
                
                // 🎯 检查坐标是否合理 (允许超出屏幕,但必须是有限值)
                if screen_x1.is_finite() && screen_y1.is_finite() 
                    && screen_x2.is_finite() && screen_y2.is_finite() {
                    
                    // 🎯 即使超出屏幕也绘制 (让GPU自己裁剪)
                    if let Ok(line) = graphics::Mesh::new_line(
                        ctx,
                        &[[screen_x1, screen_y1], [screen_x2, screen_y2]],
                        2.0,
                        Color::from_rgb(0, 255, 255), // 青色
                    ) {
                        canvas.draw(&line, DrawParam::default());
                    }
                }
            }

            // 绘制路径点标记 (小圆点)
            for (idx, &(x, y)) in player.path.iter().enumerate() {
                if idx < player.path_index {
                    continue; // 跳过已经走过的点
                }
                
                // 转换到世界坐标
                let (world_x, world_y) = Coordinates::grid_to_world_center(x, y);
                
                // 转换到屏幕坐标
                let (screen_x, screen_y) = CameraSystem::world_to_screen(
                    camera_pos, camera, world_x, world_y
                );
                
                // 🎯 只绘制有效坐标的路径点
                if screen_x.is_finite() && screen_y.is_finite() {
                    // 绘制圆点
                    // 当前目标点用更大的红色圆圈
                    let (radius, color) = if idx == player.path_index {
                        (6.0, Color::from_rgb(255, 0, 0)) // 红色,大圆
                    } else {
                        (3.0, Color::from_rgb(255, 255, 0)) // 黄色,小圆
                    };
                    
                    // 使用圆形绘制路径点
                    if let Ok(circle) = graphics::Mesh::new_circle(
                        ctx,
                        graphics::DrawMode::fill(),
                        [screen_x, screen_y],
                        radius,
                        0.1,
                        color,
                    ) {
                        canvas.draw(&circle, DrawParam::default());
                    }
                }
            }
        }
        
        Ok(())
    }

    /// 使用 InstanceArray 批量绘制相同纹理的瓦片(性能优化)
    /// 
    /// 相比逐个 canvas.draw()，InstanceArray 可以：
    /// - 减少 draw 调用次数（N → 1）
    /// - 减少 CPU → GPU 通信开销（约 70%）
    /// - 提升 15-30% 的 FPS（取决于瓦片数量）
    pub fn draw_tiles_instanced(
        ctx: &mut Context,
        canvas: &mut Canvas,
        world: &World,
        entities: &[hecs::Entity],
        pos: &Position,
        camera: &Camera,
        config: &RenderConfig,
    ) -> GameResult<()> {
        use crate::ecs::{CELL_WIDTH, CELL_HEIGHT, TileLayer, MapTile};
        use crate::ecs::systems::CameraSystem;
        use crate::graphics::libraries::get_map_library;
        use ggez::graphics::{self, InstanceArray};
        
        if entities.is_empty() {
            return Ok(());
        }

        // 获取第一个瓦片的纹理信息
        let first_tile = match world.get::<&MapTile>(entities[0]) {
            Ok(tile) => tile,
            Err(_) => return Ok(()),
        };
        let mlib = match get_map_library(first_tile.library_index) {
            Some(lib) => lib,
            None => return Ok(()),
        };

        let mut mlib_locked = match mlib.lock() {
            Ok(guard) => guard,
            Err(_) => return Ok(()),
        };

        // 获取纹理
        let texture_info = match mlib_locked.get_or_create_texture(ctx, first_tile.image_index as usize) {
            Ok(info) => info,
            Err(_) => return Ok(()),
        };

        let texture = match &texture_info.image {
            Some(tex) => tex.clone(),
            None => return Ok(()),
        };

        // 获取纹理尺寸
        let (tile_w, tile_h) = mlib_locked
            .get_size(first_tile.image_index as usize)
            .unwrap_or((CELL_WIDTH as i16, CELL_HEIGHT as i16));

        // 释放锁
        drop(mlib_locked);

        // 创建 InstanceArray
        let mut instances = InstanceArray::new(&ctx.gfx, texture);
        // 注意：GGEZ 0.10.0-rc0 的 InstanceArray 没有 set_ordered 方法
        // 但我们已经在查询时手动排序了，所以不需要

        // 为每个瓦片添加实例
        for &entity in entities {
            let tile = match world.get::<&MapTile>(entity) {
                Ok(t) => t,
                Err(_) => continue,
            };
            // 计算世界坐标
            let mut world_x = (tile.grid_x * CELL_WIDTH) as f32;
            let world_y = (tile.grid_y * CELL_HEIGHT) as f32;

            // 调整Y坐标 (大型物体需要向上偏移)
            let mut adjusted_y = if (tile_w as i32 != CELL_WIDTH
                || tile_h as i32 != CELL_HEIGHT)
                && (tile_w as i32 != CELL_WIDTH * 2
                    || tile_h as i32 != CELL_HEIGHT * 2)
            {
                world_y + CELL_HEIGHT as f32 - tile_h as f32
            } else {
                world_y
            };

            // 🔥 Front层混合模式偏移（火焰、光效等特效）
            if tile.use_blend && tile.layer == TileLayer::Front {
                world_x = world_x - 1.0 * CELL_WIDTH as f32;
                adjusted_y = adjusted_y - 4.0 * CELL_HEIGHT as f32;
            }

            // 世界坐标转屏幕坐标
            let (screen_x, screen_y) =
                CameraSystem::world_to_screen(pos, camera, world_x, adjusted_y);

            // 🚀 屏幕剔除：如果完全在屏幕外，跳过
            let tile_screen_w = tile_w as f32 * camera.zoom;
            let tile_screen_h = tile_h as f32 * camera.zoom;
            
            if tile.layer != TileLayer::Front {
                if screen_x + tile_screen_w < 0.0 
                    || screen_x > camera.screen_width
                    || screen_y + tile_screen_h < 0.0
                    || screen_y > camera.screen_height {
                    continue;
                }
            }

            // 添加到实例数组
            let color = Color::from_rgba(
                (255.0 * tile.brightness) as u8,
                (255.0 * tile.brightness) as u8,
                (255.0 * tile.brightness) as u8,
                255,
            );

            instances.push(
                DrawParam::default()
                    .dest([screen_x, screen_y])
                    .scale([camera.zoom, camera.zoom])
                    .color(color)
                    .z(tile.z_order),  // 🎯 Z 顺序（InstanceArray 会使用）
            );

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

        // 🚀 一次性绘制所有实例（这是性能提升的关键！）
        canvas.draw(&instances, DrawParam::default());

        Ok(())
    }

    /// 绘制网格
    pub fn draw_grid(
        ctx: &mut Context,
        canvas: &mut Canvas,
        world: &World,
        pos: &Position,
        camera: &Camera,
    ) -> GameResult<()> {
        use crate::ecs::{CELL_WIDTH, CELL_HEIGHT, MapData};
        use crate::ecs::systems::CameraSystem;
        use ggez::graphics;
        
        // 获取地图尺寸
        let (map_width, map_height) = world
            .query::<&MapData>()
            .iter()
            .next()
            .map(|(_, data)| (data.width, data.height))
            .unwrap_or((100, 100));

        let left = pos.x + (0.0 - camera.screen_width / 2.0) / camera.zoom;
        let right = pos.x + (camera.screen_width - camera.screen_width / 2.0) / camera.zoom;
        let top = pos.y + (0.0 - camera.screen_height / 2.0) / camera.zoom;
        let bottom = pos.y + (camera.screen_height - camera.screen_height / 2.0) / camera.zoom;

        let start_x = ((left / CELL_WIDTH as f32).floor() as i32).max(0);
        let end_x = ((right / CELL_WIDTH as f32).ceil() as i32).min(map_width);
        let start_y = ((top / CELL_HEIGHT as f32).floor() as i32).max(0);
        let end_y = ((bottom / CELL_HEIGHT as f32).ceil() as i32).min(map_height);

        let grid_color = Color::from_rgba(0, 255, 0, 120);

        // 垂直线
        for x in start_x..=end_x {
            let world_x = (x * CELL_WIDTH) as f32;
            let (screen_x, _) = CameraSystem::world_to_screen(pos, camera, world_x, 0.0);

            if screen_x >= 0.0 && screen_x <= camera.screen_width {
                let line = graphics::Mesh::new_line(
                    ctx,
                    &[[screen_x, 0.0], [screen_x, camera.screen_height]],
                    1.0,
                    grid_color,
                )?;
                canvas.draw(&line, DrawParam::default());
            }
        }

        // 水平线
        for y in start_y..=end_y {
            let world_y = (y * CELL_HEIGHT) as f32;
            let (_, screen_y) = CameraSystem::world_to_screen(pos, camera, 0.0, world_y);

            if screen_y >= 0.0 && screen_y <= camera.screen_height {
                let line = graphics::Mesh::new_line(
                    ctx,
                    &[[0.0, screen_y], [camera.screen_width, screen_y]],
                    1.0,
                    grid_color,
                )?;
                canvas.draw(&line, DrawParam::default());
            }
        }

        Ok(())
    }

    /// 绘制障碍物
    pub fn draw_obstacles(
        ctx: &mut Context,
        canvas: &mut Canvas,
        world: &World,
        pos: &Position,
        camera: &Camera,
    ) -> GameResult<()> {
        use crate::ecs::{CELL_WIDTH, CELL_HEIGHT, MapData};
        use crate::ecs::systems::CameraSystem;
        use ggez::graphics::{self, Text, TextFragment};
        
        let map_data = world
            .query::<&MapData>()
            .iter()
            .next()
            .map(|(_, data)| data.clone());

        if let Some(map_data) = map_data {
            let left = pos.x + (0.0 - camera.screen_width / 2.0) / camera.zoom;
            let right = pos.x + (camera.screen_width - camera.screen_width / 2.0) / camera.zoom;
            let top = pos.y + (0.0 - camera.screen_height / 2.0) / camera.zoom;
            let bottom = pos.y + (camera.screen_height - camera.screen_height / 2.0) / camera.zoom;

            let start_x = ((left / CELL_WIDTH as f32).floor() as i32).max(0);
            let end_x = ((right / CELL_WIDTH as f32).ceil() as i32).min(map_data.width);
            let start_y = ((top / CELL_HEIGHT as f32).floor() as i32).max(0);
            let end_y = ((bottom / CELL_HEIGHT as f32).ceil() as i32).min(map_data.height);

            // 🔴 障碍物颜色（半透明红色，更明显）
            let obstacle_color = Color::from_rgba(255, 0, 0, 150);
            let text_color = Color::from_rgb(255, 255, 0);

            for y in start_y..end_y {
                for x in start_x..end_x {
                    if x >= 0 && x < map_data.width && y >= 0 && y < map_data.height {
                        let cell = &map_data.cells[x as usize][y as usize];
                        
                        // 🎯 正确的障碍物判断：使用 back_image 的高位标记
                        let has_obstacle = (cell.back_image & 0x20000000) != 0;
                        
                        if has_obstacle {
                            let world_x = (x * CELL_WIDTH) as f32;
                            let world_y = (y * CELL_HEIGHT) as f32;
                            let (screen_x, screen_y) =
                                CameraSystem::world_to_screen(pos, camera, world_x, world_y);

                            // 绘制障碍物方块
                            let rect = graphics::Mesh::new_rectangle(
                                ctx,
                                graphics::DrawMode::fill(),
                                graphics::Rect::new(
                                    screen_x,
                                    screen_y,
                                    CELL_WIDTH as f32 * camera.zoom,
                                    CELL_HEIGHT as f32 * camera.zoom,
                                ),
                                obstacle_color,
                            )?;
                            canvas.draw(&rect, DrawParam::default());

                            // 🔤 绘制障碍物标记文字（更大字体）
                            if camera.zoom > 0.5 {  // 只在放大时显示文字
                                let text = Text::new(
                                    TextFragment::new("X")
                                        .scale(24.0)  // 加大字体
                                        .color(text_color)
                                );
                                
                                let text_x = screen_x + (CELL_WIDTH as f32 * camera.zoom - 12.0) / 2.0;
                                let text_y = screen_y + (CELL_HEIGHT as f32 * camera.zoom - 24.0) / 2.0;
                                
                                canvas.draw(&text, DrawParam::default().dest([text_x, text_y]));
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
    
    /// 绘制怪物
    /// 
    /// 参数：
    /// - ctx: ggez 上下文
    /// - canvas: 画布
    /// - world: ECS 世界
    /// - camera_pos: 相机位置
    /// - camera: 相机组件
    pub fn draw_monsters(
        _ctx: &mut Context,
        canvas: &mut Canvas,
        world: &World,
        camera_pos: &Position,
        camera: &Camera,
    ) -> GameResult<()> {
        use crate::ecs::components::{MonsterData, Animation};
        use crate::graphics::libraries::{get_library_from_array, LibraryArray};
        use crate::ecs::systems::CameraSystem;
        use mir2_shared::MirAction;
        
        // 遍历所有怪物实体
        for (_entity, (monster, pos, anim)) in 
            world.query::<(&MonsterData, &Position, &Animation)>().iter() 
        {
            // 获取怪物图库
            // 怪物库使用 LibraryArray::Monsters
            // 索引范围: 0-999
            let lib_index = (monster.monster_index / 1000) as usize;
            
            let lib = match get_library_from_array(LibraryArray::Monsters, lib_index) {
                Some(lib) => lib,
                None => continue, // 库不存在，跳过
            };
            
            // 计算帧索引
            // 怪物动画布局（与玩家类似）:
            //   - Standing: 每方向 4 帧
            //   - Walking:  每方向 6 帧
            //   - Attack1:  每方向 6 帧
            //   - Die:      每方向 10 帧
            //   - Dead:     每方向 1 帧
            
            let action_frame_start = match anim.action {
                MirAction::Standing => 0,
                MirAction::Walking => 32,    // 8方向 * 4帧 = 32
                MirAction::Attack1 => 80,    // 32 + 8方向 * 6帧 = 80
                MirAction::Struck => 128,    // 80 + 8方向 * 6帧 = 128
                MirAction::Die => 144,       // 128 + 8方向 * 2帧 = 144
                MirAction::Dead => 224,      // 144 + 8方向 * 10帧 = 224
                _ => 0,
            };
            
            let frames_per_direction = match anim.action {
                MirAction::Standing => 4,
                MirAction::Walking => 6,
                MirAction::Attack1 => 6,
                MirAction::Struck => 2,
                MirAction::Die => 10,
                MirAction::Dead => 1,
                _ => 4,
            };
            
            let direction_offset = (anim.direction as i32) * frames_per_direction;
            let draw_frame = action_frame_start + direction_offset + anim.frame_index as i32;
            
            // 怪物索引偏移
            let monster_offset = (monster.monster_index % 1000) as i32;
            let final_frame = draw_frame + monster_offset;
            
            // 转换为屏幕坐标
            let (screen_x, screen_y) = CameraSystem::world_to_screen(
                camera_pos,
                camera,
                pos.x,
                pos.y,
            );
            
            // 绘制怪物
            let mut lib_locked = lib.lock().unwrap();
            if let Ok(image_info) = lib_locked.get_image_info(final_frame as usize) {
                // 计算绘制位置（考虑偏移）
                let draw_x = screen_x + image_info.x as f32 * camera.zoom;
                let draw_y = screen_y + image_info.y as f32 * camera.zoom;
                
                // 绘制精灵
                if let Some(image) = &image_info.image {
                    canvas.draw(
                        image,
                        DrawParam::default()
                            .dest([draw_x, draw_y])
                            .scale([camera.zoom, camera.zoom]),
                    );
                }
            }
        }
        
        Ok(())
    }
    
    /// 绘制怪物血条和名称
    /// 
    /// 参数：
    /// - ctx: ggez 上下文
    /// - canvas: 画布
    /// - world: ECS 世界
    /// - camera_pos: 相机位置
    /// - camera: 相机组件
    pub fn draw_monster_info(
        ctx: &mut Context,
        canvas: &mut Canvas,
        world: &World,
        camera_pos: &Position,
        camera: &Camera,
    ) -> GameResult<()> {
        use crate::ecs::components::{MonsterData, Health};
        use crate::ecs::systems::CameraSystem;
        use ggez::graphics::{Text, Mesh, DrawMode, Rect};
        
        // 遍历所有怪物实体
        for (_entity, (monster, pos, health)) in 
            world.query::<(&MonsterData, &Position, &Health)>().iter() 
        {
            // 跳过死亡怪物
            if health.current <= 0 {
                continue;
            }
            
            // 转换为屏幕坐标
            let (screen_x, screen_y) = CameraSystem::world_to_screen(
                camera_pos,
                camera,
                pos.x,
                pos.y,
            );
            
            // 名称位置（怪物上方）
            let name_y = screen_y - 60.0 * camera.zoom;
            
            // 绘制名称
            let name_text = Text::new(&monster.name);
            let name_width = name_text.measure(ctx)?.x;
            let name_x = screen_x - name_width / 2.0;
            
            canvas.draw(
                &name_text,
                DrawParam::default()
                    .dest([name_x, name_y])
                    .color(Color::from_rgb(255, 255, 255)),
            );
            
            // 血条位置（名称下方）
            let hp_bar_width = 50.0 * camera.zoom;
            let hp_bar_height = 4.0 * camera.zoom;
            let hp_bar_y = name_y + 16.0;
            let hp_bar_x = screen_x - hp_bar_width / 2.0;
            
            // 血条背景（黑色）
            let bg_rect = Mesh::new_rectangle(
                ctx,
                DrawMode::fill(),
                Rect::new(hp_bar_x, hp_bar_y, hp_bar_width, hp_bar_height),
                Color::from_rgb(0, 0, 0),
            )?;
            canvas.draw(&bg_rect, DrawParam::default());
            
            // 血条前景（红色，根据血量百分比）
            let hp_percent = health.current as f32 / health.max as f32;
            let hp_color = if hp_percent > 0.5 {
                Color::from_rgb(0, 255, 0) // 绿色
            } else if hp_percent > 0.25 {
                Color::from_rgb(255, 255, 0) // 黄色
            } else {
                Color::from_rgb(255, 0, 0) // 红色
            };
            
            let fg_rect = Mesh::new_rectangle(
                ctx,
                DrawMode::fill(),
                Rect::new(
                    hp_bar_x + 1.0,
                    hp_bar_y + 1.0,
                    (hp_bar_width - 2.0) * hp_percent,
                    hp_bar_height - 2.0,
                ),
                hp_color,
            )?;
            canvas.draw(&fg_rect, DrawParam::default());
        }
        
        Ok(())
    }

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
        use crate::ecs::components::ItemDrop;
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




// ============================================================================
// NPC Rendering Module - NPC渲染模块
// ============================================================================

use super::RenderSystem;
use crate::ecs::components::{NPCData, Position, Camera, Animation, Direction};
use crate::ecs::RenderConfig;
use ggez::{Context, GameResult, graphics::{self, Canvas, DrawParam}};
use hecs::World;

impl RenderSystem {
    /// 绘制所有NPC
    pub fn draw_npcs(
        ctx: &mut Context,
        canvas: &mut Canvas,
        world: &World,
        camera_pos: &Position,
        camera: &Camera,
        config: &RenderConfig,
    ) -> GameResult<()> {
        use crate::graphics::libraries::{get_library_from_array, LibraryArray};
        use crate::ecs::systems::CameraSystem;
        
        // 遍历所有NPC实体 - 包含Direction组件以获取正确的方向
        for (_entity, (npc, pos, anim, dir)) in 
            world.query::<(&NPCData, &Position, &Animation, &Direction)>().iter() 
        {
            // 使用Direction组件的current值而不是Animation.direction
            let direction = dir.current as u8;
            
            // 获取NPC图库
            // NPC库使用 LibraryArray::NPCs
            // ⚠️ 修复：不应该除以1000，直接使用npc_index作为库索引
            let lib_index = npc.npc_index as usize;
            
            let lib = match get_library_from_array(LibraryArray::NPCs, lib_index) {
                Some(lib) => lib,
                None => continue, // 库不存在，跳过
            };
            
            // 🎯 使用FrameSet计算帧索引,支持不同动作
            use crate::objects::frames::{DEFAULT_NPC_FRAMES, get_frame};
            
            let (action_frame_start, frames_per_direction) = if let Some(frame) = get_frame(&DEFAULT_NPC_FRAMES, anim.action) {
                (frame.start, frame.count)
            } else {
                // 默认使用Standing动作
                (0, 4)
            };
            
            // 使用Direction组件的值而不是Animation.direction
            let direction_offset = (direction as i32) * frames_per_direction;
            let draw_frame = action_frame_start + direction_offset + anim.frame_index as i32;
            
            // ⚠️ 修复：不需要npc_offset，直接使用draw_frame
            let final_frame = draw_frame;
            
            // 转换为屏幕坐标
            let (screen_x, screen_y) = CameraSystem::world_to_screen(
                camera_pos,
                camera,
                pos.x,
                pos.y,
            );
            
            // 🔧 使用 get_or_create_texture 确保图像被加载到GPU (与怪物渲染一致)
            let mut lib_locked = lib.lock().unwrap();
            match lib_locked.get_or_create_texture(ctx, final_frame as usize) {
                Ok(image_info) => {
                    // 计算绘制位置（考虑偏移）
                    let draw_x = screen_x + image_info.x as f32 * camera.zoom;
                    let draw_y = screen_y + image_info.y as f32 * camera.zoom;
                    
                    // 🎨 应用NPC颜色染色
                    let color = Self::argb_to_color(npc.colour);
                    
                    // 绘制主图像
                    if let Some(image) = &image_info.image {
                        canvas.draw(
                            image,
                            DrawParam::default()
                                .dest([draw_x, draw_y])
                                .scale([camera.zoom, camera.zoom])
                                .color(color),
                        );
                        
                        // 🔲 调试边框
                        if config.show_npc_borders {
                            let image_width = image.width() as f32 * camera.zoom;
                            let image_height = image.height() as f32 * camera.zoom;
                            let rect = graphics::Rect::new(draw_x, draw_y, image_width, image_height);
                            let mesh = graphics::Mesh::new_rectangle(
                                ctx,
                                graphics::DrawMode::stroke(1.0),
                                rect,
                                graphics::Color::from_rgb(0, 255, 255), // 青色
                            ).ok();
                            if let Some(mesh) = mesh {
                                canvas.draw(&mesh, DrawParam::default());
                            }
                        }
                    }
                    
                    // 🌟 绘制特效层
                    if let Some(frame_data) = get_frame(&DEFAULT_NPC_FRAMES, anim.action) {
                        if frame_data.effect_count > 0 {
                            let effect_frame = frame_data.effect_start + direction_offset + anim.frame_index as i32;
                            // ⚠️ 修复：不需要npc_offset
                            let final_effect_frame = effect_frame;
                            
                            if let Ok(effect_info) = lib_locked.get_or_create_texture(ctx, final_effect_frame as usize) {
                                if let Some(effect_image) = &effect_info.image {
                                    let effect_x = screen_x + effect_info.x as f32 * camera.zoom;
                                    let effect_y = screen_y + effect_info.y as f32 * camera.zoom;
                                    
                                    canvas.draw(
                                        effect_image,
                                        DrawParam::default()
                                            .dest([effect_x, effect_y])
                                            .scale([camera.zoom, camera.zoom])
                                            .color(color),
                                    );
                                }
                            }
                        }
                    }
                    
                    // 🏷️ 绘制NPC名字
                    drop(lib_locked); // 释放库锁
                    Self::draw_npc_name(ctx, canvas, &npc.name, screen_x, screen_y - 40.0 * camera.zoom, camera);
                }
                Err(_) => {} // 忽略错误,继续下一个NPC
            }
        }
        
        Ok(())
    }
    
    /// 🎯 绘制单个NPC (用于Y-sorting)
    pub fn draw_single_npc(
        ctx: &mut Context,
        canvas: &mut Canvas,
        world: &World,
        entity: hecs::Entity,
        pos: &Position,
        camera_pos: &Position,
        camera: &Camera,
        config: &RenderConfig,
    ) -> GameResult<()> {
        use crate::graphics::libraries::{get_library_from_array, LibraryArray};
        use crate::ecs::systems::CameraSystem;
        
        // 获取NPC数据和动画
        let npc = match world.get::<&NPCData>(entity) {
            Ok(n) => n,
            Err(_) => return Ok(()),
        };
        
        let anim = match world.get::<&Animation>(entity) {
            Ok(a) => a,
            Err(_) => return Ok(()),
        };
        
        // 获取Direction组件
        let dir = match world.get::<&Direction>(entity) {
            Ok(d) => d,
            Err(_) => return Ok(()), // 如果没有Direction组件,跳过
        };
        let direction = dir.current as u8;
        
        // 获取NPC图库
        // ⚠️ 修复：不应该除以1000，直接使用npc_index作为库索引
        let lib_index = npc.npc_index as usize;
        let lib = match get_library_from_array(LibraryArray::NPCs, lib_index) {
            Some(lib) => lib,
            None => return Ok(()),
        };
        
        // 🎯 使用FrameSet计算帧索引,支持不同动作
        use crate::objects::frames::{DEFAULT_NPC_FRAMES, get_frame};
        
        let (action_frame_start, frames_per_direction) = if let Some(frame) = get_frame(&DEFAULT_NPC_FRAMES, anim.action) {
            (frame.start, frame.count)
        } else {
            (0, 4) // 默认
        };
        
        // 使用Direction组件的值
        let direction_offset = (direction as i32) * frames_per_direction;
        let draw_frame = action_frame_start + direction_offset + anim.frame_index as i32;
        // ⚠️ 修复：不需要npc_offset，直接使用draw_frame
        let final_frame = draw_frame;
        
        // 转换为屏幕坐标
        let (screen_x, screen_y) = CameraSystem::world_to_screen(
            camera_pos,
            camera,
            pos.x,
            pos.y,
        );
        
        // 🔧 使用 get_or_create_texture 确保图像被加载
        let mut lib_locked = lib.lock().unwrap();
        
        // 🔧 修复NPC闪烁：如果帧索引超出范围，降级到第0帧
        let image_result = lib_locked.get_or_create_texture(ctx, final_frame as usize);
        let image_info = match image_result {
            Ok(info) => info,
            Err(e) => {
                // 帧索引超出范围，尝试使用第0帧作为降级方案
                tracing::debug!("⚠️ [NPC闪烁修复] NPC {} 帧{}加载失败，降级到第0帧。error={:?}", 
                    npc.name, final_frame, e);
                match lib_locked.get_or_create_texture(ctx, 0) {
                    Ok(fallback) => fallback,
                    Err(e2) => {
                        // 连第0帧都失败了，跳过此NPC
                        tracing::error!("❌ NPC {} 第0帧也加载失败! lib_index={}, error={:?}", 
                            npc.name, lib_index, e2);
                        return Ok(());
                    }
                }
            }
        };
        
        // 绘制主图像
        let draw_x = screen_x + image_info.x as f32 * camera.zoom;
        let draw_y = screen_y + image_info.y as f32 * camera.zoom;
        
        // 🎨 应用NPC颜色染色
        let color = Self::argb_to_color(npc.colour);
        
        if let Some(image) = &image_info.image {
            canvas.draw(
                image,
                DrawParam::default()
                    .dest([draw_x, draw_y])
                    .scale([camera.zoom, camera.zoom])
                    .color(color),
            );
            
            // 🔲 调试边框
            if config.show_npc_borders {
                let image_width = image.width() as f32 * camera.zoom;
                let image_height = image.height() as f32 * camera.zoom;
                let rect = graphics::Rect::new(draw_x, draw_y, image_width, image_height);
                let mesh = graphics::Mesh::new_rectangle(
                    ctx,
                    graphics::DrawMode::stroke(1.0),
                    rect,
                    graphics::Color::from_rgb(0, 255, 255), // 青色
                ).ok();
                if let Some(mesh) = mesh {
                    canvas.draw(&mesh, DrawParam::default());
                }
            }
        } else {
            // 图像为空（理论上不应该发生）
            tracing::warn!("⚠️ NPC {} 图像为空! lib_index={}", npc.name, lib_index);
        }
        
        // 🌟 绘制特效层 (武器、装饰等)
        if let Some(frame_data) = get_frame(&DEFAULT_NPC_FRAMES, anim.action) {
            if frame_data.effect_count > 0 {
                let effect_frame = frame_data.effect_start + direction_offset + anim.frame_index as i32;
                // ⚠️ 修复：不需要npc_offset
                let final_effect_frame = effect_frame;
                
                if let Ok(effect_info) = lib_locked.get_or_create_texture(ctx, final_effect_frame as usize) {
                    if let Some(effect_image) = &effect_info.image {
                        let effect_x = screen_x + effect_info.x as f32 * camera.zoom;
                        let effect_y = screen_y + effect_info.y as f32 * camera.zoom;
                        
                        canvas.draw(
                            effect_image,
                            DrawParam::default()
                                .dest([effect_x, effect_y])
                                .scale([camera.zoom, camera.zoom])
                                .color(color), // 特效层也应用相同颜色
                        );
                    }
                }
            }
        }
        
        // 🏷️ 绘制NPC名字(头顶上方)
        drop(lib_locked); // 释放库锁,避免死锁
        Self::draw_npc_name(ctx, canvas, &npc.name, screen_x, screen_y - 40.0 * camera.zoom, camera);
        
        // 📋 绘制任务图标(如果有)
        if let Ok(quest_marker) = world.get::<&crate::ecs::components::QuestMarker>(entity) {
            Self::draw_quest_icon(ctx, canvas, quest_marker.icon, screen_x, screen_y - 60.0 * camera.zoom, camera);
        }
        
        Ok(())
    }
}

// ============================================================================
// Monster Rendering Module - 怪物渲染模块
// ============================================================================

use super::RenderSystem;
use crate::ecs::{RenderConfig, components::{Animation, Camera, Direction, Health, MonsterData, Position}};
use ggez::{Context, GameResult, graphics::{self, Canvas, DrawParam, Color, Text, Mesh, DrawMode, Rect}};
use hecs::World;

impl RenderSystem {

    /// 绘制怪物
    /// 
    /// 参数：
    /// - ctx: ggez 上下文
    /// - canvas: 画布
    /// - world: ECS 世界
    /// - camera_pos: 相机位置
    /// - camera: 相机组件
    /// 🎯 绘制单个怪物 (用于Y-sorting)
    pub fn draw_single_monster(
        ctx: &mut Context,
        canvas: &mut Canvas,
        world: &World,
        entity: hecs::Entity,
        pos: &Position,
        camera_pos: &Position,
        camera: &Camera,
        config: &RenderConfig,
    ) -> GameResult<()> {
        use crate::ecs::components::{MonsterData, Animation, Direction};
        use crate::graphics::libraries::{get_library_from_array, LibraryArray};
        use crate::ecs::systems::CameraSystem;
        
        // 获取怪物数据和动画
        let monster = match world.get::<&MonsterData>(entity) {
            Ok(m) => m,
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
        
        // 获取怪物图库
        let lib_index = monster.monster_index as usize;
        let lib = match get_library_from_array(LibraryArray::Monsters, lib_index) {
            Some(lib) => lib,
            None => return Ok(()),
        };
        
        // 获取动作帧配置
        use crate::objects::frames::{DEFAULT_MONSTER_FRAMES, get_frame};
        let frame = match get_frame(&DEFAULT_MONSTER_FRAMES, anim.action) {
            Some(f) => f,
            None => return Ok(()), // 未定义的动作,不绘制
        };
        
        // 计算帧索引: Frame.Start + (Frame.OffSet * Direction) + FrameIndex
        // 这与C#原版完全一致: DrawFrame = Frame.Start + (Frame.OffSet * (byte)Direction) + FrameIndex;
        // 使用Direction组件的值
        let direction_offset = frame.offset() * (direction as i32);
        let draw_frame = frame.start + direction_offset + anim.frame_index as i32;
        let final_frame = draw_frame;
        
        // 转换为屏幕坐标
        let (screen_x, screen_y) = CameraSystem::world_to_screen(
            camera_pos,
            camera,
            pos.x,
            pos.y,
        );
        
        // 绘制怪物
        let mut lib_locked = lib.lock().unwrap();
        match lib_locked.get_or_create_texture(ctx, final_frame as usize) {
            Ok(image_info) => {
                let draw_x = screen_x + image_info.x as f32 * camera.zoom;
                let draw_y = screen_y + image_info.y as f32 * camera.zoom;
                
                if let Some(image) = &image_info.image {
                    canvas.draw(
                        image,
                        DrawParam::default()
                            .dest([draw_x, draw_y])
                            .scale([camera.zoom, camera.zoom]),
                    );
                    
                    // 🔲 调试边框
                    if config.show_monster_borders {
                        let image_width = image.width() as f32 * camera.zoom;
                        let image_height = image.height() as f32 * camera.zoom;
                        let rect = graphics::Rect::new(draw_x, draw_y, image_width, image_height);
                        let mesh = graphics::Mesh::new_rectangle(
                            ctx,
                            graphics::DrawMode::stroke(1.0),
                            rect,
                            graphics::Color::from_rgb(255, 0, 255), // 紫色
                        ).ok();
                        if let Some(mesh) = mesh {
                            canvas.draw(&mesh, DrawParam::default());
                        }
                    }
                }
            }
            Err(_) => {}
        }
        
        Ok(())
    }

    /// 绘制所有怪物
    pub fn draw_monsters(
        ctx: &mut Context,
        canvas: &mut Canvas,
        world: &World,
        camera_pos: &Position,
        camera: &Camera,
        config: &RenderConfig,
    ) -> GameResult<()> {
        use crate::graphics::libraries::{get_library_from_array, LibraryArray};
        use crate::ecs::systems::CameraSystem;
        
        // 遍历所有怪物实体 - 包含Direction组件以获取正确的方向
        for (_entity, (monster, pos, anim, dir)) in 
            world.query::<(&MonsterData, &Position, &Animation, &Direction)>().iter() 
        {
            // 使用Direction组件的current值而不是Animation.direction
            let direction = dir.current as u8;
            
            // 🐛 调试信息
            println!("👹 绘制怪物: name={}, monster_index={}, pos=({:.1}, {:.1}), action={:?}, dir={}", 
                monster.name, monster.monster_index, pos.x, pos.y, anim.action, direction);
            
            // 获取怪物图库
            // 传奇怪物库组织: 每个怪物有独立的库文件
            // - 000.Lib = 怪物0 (所有帧)
            // - 001.Lib = 怪物1 (所有帧)
            // - 002.Lib = 怪物2 (所有帧)
            // 所以 lib_index = monster_index
            let lib_index = monster.monster_index as usize;
            
            println!("  🔍 尝试加载库: LibraryArray::Monsters[{}]", lib_index);
            
            let lib = match get_library_from_array(LibraryArray::Monsters, lib_index) {
                Some(lib) => lib,
                None => {
                    println!("  ❌ 怪物图库 {} 不存在", lib_index);
                    tracing::warn!("⚠️ 怪物图库 {} 不存在", lib_index);
                    continue;
                }
            };
            
            println!("  ✅ 怪物图库 {} 加载成功", lib_index);
            
            // 获取动作帧配置
            use crate::objects::frames::{DEFAULT_MONSTER_FRAMES, get_frame};
            let frame = match get_frame(&DEFAULT_MONSTER_FRAMES, anim.action) {
                Some(f) => f,
                None => {
                    println!("  ⚠️ 未定义的怪物动作: {:?}", anim.action);
                    continue;
                }
            };
            
            // 计算帧索引: Frame.Start + (Frame.OffSet * Direction) + FrameIndex
            // 这与C#原版完全一致: DrawFrame = Frame.Start + (Frame.OffSet * (byte)Direction) + FrameIndex;
            // 使用Direction组件的值而不是Animation.direction
            let direction_offset = frame.offset() * (direction as i32);
            let draw_frame = frame.start + direction_offset + anim.frame_index as i32;
            let final_frame = draw_frame;
            
            println!("  📊 帧计算: lib_index={}, frame_start={}, offset={}, dir={}, dir_offset={}, frame_idx={}, final={}", 
                lib_index, frame.start, frame.offset(), direction, direction_offset, anim.frame_index, final_frame);
            
            // 转换为屏幕坐标
            let (screen_x, screen_y) = CameraSystem::world_to_screen(
                camera_pos,
                camera,
                pos.x,
                pos.y,
            );
            
            // 绘制怪物
            let mut lib_locked = lib.lock().unwrap();
            let lib_count = lib_locked.count();
            println!("  📚 库中图像数量: {}, 请求帧: {}", lib_count, final_frame);
            
            // 🔧 使用 get_or_create_texture 确保图像被加载到GPU
            match lib_locked.get_or_create_texture(ctx, final_frame as usize) {
                Ok(image_info) => {
                    // 计算绘制位置（考虑偏移）
                    let draw_x = screen_x + image_info.x as f32 * camera.zoom;
                    let draw_y = screen_y + image_info.y as f32 * camera.zoom;
                    
                    println!("  🎨 绘制位置: screen=({:.1}, {:.1}), offset=({}, {}), final=({:.1}, {:.1})", 
                        screen_x, screen_y, image_info.x, image_info.y, draw_x, draw_y);
                    
                    // 绘制精灵
                    if let Some(image) = &image_info.image {
                        canvas.draw(
                            image,
                            DrawParam::default()
                                .dest([draw_x, draw_y])
                                .scale([camera.zoom, camera.zoom]),
                        );
                        println!("  ✅ 怪物精灵绘制成功");
                        
                        // 🔲 调试边框
                        if config.show_monster_borders {
                            let image_width = image.width() as f32 * camera.zoom;
                            let image_height = image.height() as f32 * camera.zoom;
                            let rect = graphics::Rect::new(draw_x, draw_y, image_width, image_height);
                            let mesh = graphics::Mesh::new_rectangle(
                                ctx,
                                graphics::DrawMode::stroke(1.0),
                                rect,
                                graphics::Color::from_rgb(255, 0, 255), // 紫色
                            ).ok();
                            if let Some(mesh) = mesh {
                                canvas.draw(&mesh, DrawParam::default());
                            }
                        }
                    } else {
                        println!("  ⚠️ 怪物图像为空 (image_info存在但image为None)!");
                        tracing::warn!("  ⚠️ 怪物图像为空!");
                    }
                }
                Err(e) => {
                    println!("  ❌ 无法获取/创建怪物纹理: frame={}, 库大小={}, 错误: {}", final_frame, lib_count, e);
                    tracing::warn!("  ⚠️ 无法获取怪物图像信息: frame={}", final_frame);
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
        use crate::ecs::systems::CameraSystem;
        
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
}

// ============================================================================
// Character Render System - 角色渲染系统
// ============================================================================
//
// **优先级**: 610 (在地图渲染后，调试系统前)
//
// **职责**:
// - 渲染所有玩家角色（本地玩家和其他玩家）
// - 处理角色动画帧（站立、行走、跑步）
// - 处理多层渲染（身体、武器、翅膀等）
//
// **C# 参考**:
// - PlayerObject.DrawBody() - 绘制角色身体
// - PlayerObject.DrawWeapon() - 绘制武器
// - CHumEffect[class][gender] - 角色库索引
//
// **渲染流程**:
// 1. 查询所有 (Player, Position, PlayerAppearance)
// 2. 按 Y 坐标排序（深度排序）
// 3. 计算动画帧索引
// 4. 从 CHumEffect 库加载精灵图
// 5. 应用相机变换绘制到屏幕
//
// ============================================================================

use hecs::World;
use ggez::GameResult;
use ggez::graphics::{Canvas, Color, DrawParam, GraphicsContext};

use crate::ecs::GameContext;
use crate::ecs::components::{
    Camera, Position, Player, PlayerAppearance, TimeTracker,
};
use crate::ecs::systems::System;
use crate::graphics::libraries::{get_library_from_array, LibraryArray};
use mir2_shared::enums::{MirClass, MirGender};

/// 角色渲染系统
pub struct CharacterRenderSystem;

impl CharacterRenderSystem {
    /// 获取相机变换参数
    fn get_camera_transform(world: &World) -> Option<(f32, f32, f32)> {
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

    /// 获取角色库索引
    /// 
    /// C# 逻辑: CHumEffect[class][gender]
    /// - Male Warrior = 0, Female Warrior = 1
    /// - Male Wizard = 2, Female Wizard = 3
    /// - Male Taoist = 4, Female Taoist = 5
    /// - Male Assassin = 6, Female Assassin = 7
    /// - Male Archer = 8, Female Archer = 9
    fn get_character_library_index(class: MirClass, gender: MirGender) -> usize {
        let class_base = match class {
            MirClass::Warrior => 0,
            MirClass::Wizard => 2,
            MirClass::Taoist => 4,
            MirClass::Assassin => 6,
            MirClass::Archer => 8,
        };
        
        let gender_offset = match gender {
            MirGender::Male => 0,
            MirGender::Female => 1,
        };
        
        class_base + gender_offset
    }

    /// 计算角色动画帧索引
    /// 
    /// C# 逻辑参考: PlayerObject.cs DrawBody()
    /// ```csharp
    /// int index = BaseIndex + (Direction * FrameCount) + CurrentFrame
    /// ```
    fn calculate_frame_index(
        player: &Player,
        time_tracker: &TimeTracker,
    ) -> i32 {
        use crate::ecs::components::PlayerAction;
        
        let action_start = player.action.frame_start();
        let frame_count = player.action.frame_count();
        let frame_interval = player.action.frame_interval();
        
        // 基于全局动画计数器计算当前帧
        let animation_tick = (time_tracker.animation_count as i32) / frame_interval;
        let current_frame = animation_tick % frame_count;
        
        // 计算最终索引：基础索引 + 方向偏移 + 帧偏移
        action_start + (player.direction as i32 * frame_count) + current_frame
    }

    /// 渲染单个角色
    fn render_character(
        ctx: &mut GraphicsContext,
        canvas: &mut Canvas,
        player: &Player,
        pos: &Position,
        appearance: &PlayerAppearance,
        time_tracker: &TimeTracker,
        cam_x: f32,
        cam_y: f32,
        zoom: f32,
    ) -> GameResult {
        let screen_width = ctx.drawable_size().0;
        let screen_height = ctx.drawable_size().1;

        // 计算屏幕坐标
        let (screen_x, screen_y) = Self::world_to_screen(
            pos.x, pos.y, cam_x, cam_y, zoom, screen_width, screen_height
        );

        // 计算动画帧索引
        let frame_index = Self::calculate_frame_index(player, time_tracker);

        // 🎨 C# PlayerObject.DrawBody() 逻辑:
        // 1. 先绘制身体 (BodyLibrary = CArmours[Armour])
        // 2. 再绘制头发 (HairLibrary = CHair[Hair])
        // 3. DrawFrame = Frame.Start + (Frame.OffSet * Direction) + FrameIndex
        // 4. ArmourOffSet 用于不同性别的动画偏移
        
        let color = Color::WHITE;
        
        // 1️⃣ 绘制身体 (CArmours库 - 盔甲/服装)
        let armour_index = appearance.armour.max(0) as usize; // 0 = 默认裸体
        tracing::info!("🎯 尝试获取身体库: armour_index={}", armour_index);
        if let Some(body_lib) = get_library_from_array(LibraryArray::CArmours, armour_index) {
            tracing::info!("✅ 获取到身体库锁");
            let mut body_lib = body_lib.lock().unwrap();
            
            // 应用性别偏移 (C# ArmourOffSet)
            let armour_offset = if appearance.gender == MirGender::Male {
                0
            } else {
                808 // 女性角色动画偏移
            };
            
            let body_frame = (frame_index + armour_offset) as usize;
            tracing::info!("🎨 计算帧索引: frame_index={}, offset={}, body_frame={}", frame_index, armour_offset, body_frame);
            
            tracing::info!("📦 尝试获取或创建纹理: body_frame={}", body_frame);
            match body_lib.get_or_create_texture(ctx, body_frame) {
                Ok(info) => {
                    tracing::info!("✅ 成功获取纹理信息");
                    let draw_x = screen_x - (info.x as f32) * zoom;
                    let draw_y = screen_y - (info.y as f32) * zoom;
                    
                    if let Some(ref image) = info.image {
                        canvas.draw(
                            image,
                            DrawParam::new()
                                .dest([draw_x, draw_y])
                                .scale([zoom, zoom])
                                .color(color),
                        );
                    }
                }
                Err(e) => {
                    tracing::error!("❌ 获取纹理失败: {:?}", e);
                    return Ok(()); // 跳过渲染，避免崩溃
                }
            }
        }
        
        // 2️⃣ 绘制头发 (CHair库)
        let hair_index = appearance.hair.max(0) as usize;
        if hair_index > 0 {
            if let Some(hair_lib) = get_library_from_array(LibraryArray::CHair, hair_index) {
                let mut hair_lib = hair_lib.lock().unwrap();
                
                // 头发使用相同的帧索引和偏移
                let hair_offset = if appearance.gender == MirGender::Male {
                    0
                } else {
                    808
                };
                
                let hair_frame = (frame_index + hair_offset) as usize;

                if let Ok(info) = hair_lib.get_or_create_texture(ctx, hair_frame) {
                    let draw_x = screen_x - (info.x as f32) * zoom;
                    let draw_y = screen_y - (info.y as f32) * zoom;
                    
                    if let Some(ref image) = info.image {
                        canvas.draw(
                            image,
                            DrawParam::new()
                                .dest([draw_x, draw_y])
                                .scale([zoom, zoom])
                                .color(color),
                        );
                    }
                }
            }
        }
        
        // 如果没有身体或头发纹理，绘制占位符
        // (这里简化处理，实际游戏中裸体也会有默认纹理)
        
        // 🔧 调试：绘制角色脚底位置标记（红点）
        // canvas.draw(
        //     &ggez::graphics::Quad,
        //     DrawParam::new()
        //         .dest([screen_x - 2.0, screen_y - 2.0])
        //         .scale([4.0, 4.0])
        //         .color(Color::from_rgba(255, 0, 0, 255)),
        // );

        Ok(())
    }


}


// 实现 DrawSystem trait
impl crate::ecs::systems::DrawSystem for CharacterRenderSystem {
    fn draw(&mut self, ctx: &mut GraphicsContext, canvas: &mut Canvas, world: &hecs::World) -> GameResult {
        tracing::info!("👤 CharacterRenderSystem::draw() 开始");
        // 获取相机变换
        let Some((cam_x, cam_y, zoom)) = Self::get_camera_transform(world) else {
            tracing::info!("⏭️  CharacterRenderSystem: 没有相机，跳过渲染");
            return Ok(());
        };
        tracing::info!("✅ CharacterRenderSystem: 有相机，继续渲染");

        // 获取时间追踪器
        let time_tracker = {
            let mut query = world.query::<&TimeTracker>();
            if let Some((_, tracker)) = query.iter().next() {
                tracker.clone()
            } else {
                return Ok(());
            }
        };

        // 收集并排序角色
        let mut characters_to_render = Vec::new();
        
        for (_entity, (player, pos, appearance)) in 
            world.query::<(&Player, &Position, &PlayerAppearance)>().iter() 
        {
            characters_to_render.push((player.clone(), pos.clone(), appearance.clone()));
        }

        // 按 Y 坐标排序（实现深度排序）
        characters_to_render.sort_by(|a, b| {
            a.1.y.partial_cmp(&b.1.y).unwrap_or(std::cmp::Ordering::Equal)
        });

        // 渲染所有角色
        for (player, pos, appearance) in characters_to_render {
            Self::render_character(
                ctx, canvas, &player, &pos, &appearance,
                &time_tracker, cam_x, cam_y, zoom
            )?;
        }

        Ok(())
    }
}



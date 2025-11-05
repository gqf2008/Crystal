// ============================================================================
// 地图渲染系统 - 混合系统(Hybrid System)
// ============================================================================
// 职责：
// 1. update(): 更新地图瓦片动画帧(水波、岩浆、火焰等)
// 2. draw(): 渲染地图三层(Back/Middle/Front)
//
// 功能：
// - 瓦片动画管理(AnimatedTile)
// - 三层渲染架构(Back/Middle/Front)
// - 视口裁剪优化
// - 混合模式支持(Normal/ADD)
// ============================================================================

use crate::ecs::components::{AnimatedTile, Camera, MapTile, Position, RenderConfig, TileLayer};
use crate::ecs::systems::{LogicSystem, RenderSystem};
use crate::ecs::{CELL_HEIGHT, CELL_WIDTH, GameContext, GameWorld};
use crate::graphics::get_map_library;
use ggez::graphics::{DrawParam, GraphicsContext};
use ggez::GameResult;

/// 地图渲染系统 - 混合系统
///
/// update(): 更新动画瓦片的帧索引
/// draw(): 渲染地图到屏幕
#[derive(ecs_macros::RenderSystem)]
pub struct MapRenderSystem {
    /// 全局动画计数器(模拟 C# AnimationCount)
    animation_counter: u32,
    /// 累积时间(秒)
    accumulated_time: f32,
    /// 每次递增计数器需要的时间(秒)
    counter_interval: f32,
}

impl MapRenderSystem {
    pub fn new() -> Self {
        Self {
            animation_counter: 0,
            accumulated_time: 0.0,
            counter_interval: 1.0 / 60.0, // 60 FPS 基准
        }
    }

    /// 计算动画帧偏移
    ///
    /// C# 逻辑:
    /// ```csharp
    /// index += (AnimationCount % (animation + (animation * animationTick))) / (1 + animationTick);
    /// ```
    fn calculate_frame_offset(&self, frame_count: u8, frame_interval: u8) -> i32 {
        if frame_count == 0 {
            return 0;
        }

        let total_ticks = (frame_count as u32 + frame_count as u32 * frame_interval as u32);
        let divisor = 1 + frame_interval as u32;

        ((self.animation_counter % total_ticks) / divisor) as i32
    }
}

impl Default for MapRenderSystem {
    fn default() -> Self {
        Self::new()
    }
}



// ============================================================================
// RenderSystem 实现 - 渲染地图
// ============================================================================
impl RenderSystem for MapRenderSystem {
    fn update(&mut self, ctx: &mut GameContext, delta_time: f32) -> GameResult {
        // 检查是否启用动画
        let animations_enabled = {
            let mut config_query = ctx.world.query::<&RenderConfig>();
            config_query
                .iter()
                .next()
                .map(|(_, cfg)| cfg.show_animations)
                .unwrap_or(true) // 默认启用
        };

        // 如果动画被禁用,直接返回
        if !animations_enabled {
            return Ok(());
        }

        // 累积时间
        self.accumulated_time += delta_time;

        // 每个计数器间隔递增计数器
        while self.accumulated_time >= self.counter_interval {
            self.animation_counter = self.animation_counter.wrapping_add(1);
            self.accumulated_time -= self.counter_interval;
        }

        // 更新所有动画瓦片的 image_index
        for (_, (tile, anim)) in ctx.world.query_mut::<(&mut MapTile, &AnimatedTile)>() {
            let frame_offset = self.calculate_frame_offset(anim.frame_count, anim.frame_interval);
            tile.image_index = anim.base_image_index + frame_offset;
        }

        Ok(())
    }
    /// 主渲染方法
    ///
    /// 参数：
    /// - ctx: ggez上下文，用于创建纹理
    /// - canvas: 画布，用于绘制图形
    ///
    /// 返回：渲染结果
    fn draw(
        &mut self,
        ctx: &mut GraphicsContext,
        canvas: &mut ggez::graphics::Canvas,
        world: &GameWorld,
    ) -> GameResult {
        // tracing::info!("🗺️  MapRenderSystem::draw() 开始");
        // ====================================================================
        // 步骤0: 检查是否有地图瓦片数据（防止崩溃）
        // ====================================================================
        // 如果地图还没加载，直接返回避免崩溃
        let has_tiles = world.query::<&MapTile>().iter().next().is_some();
        if !has_tiles {
            // tracing::info!("⏭️  MapRenderSystem: 没有地图瓦片，跳过渲染");
            return Ok(());
        }
        // tracing::info!("✅ MapRenderSystem: 有地图瓦片，继续渲染");

        // ====================================================================
        // 步骤1: 获取相机和渲染配置
        // ====================================================================
        // 从ECS世界查询相机位置和渲染配置，这些组件控制渲染行为
        let (camera, camera_pos, config) = {
            // 查询相机实体（包含Camera和Position组件）
            let mut camera_opt = None;
            let mut camera_pos_opt = None;

            // 遍历所有拥有Camera和Position组件的实体
            for (_, (cam, pos)) in world.query::<(&Camera, &Position)>().iter() {
                camera_opt = Some(cam.clone()); // 克隆相机数据
                camera_pos_opt = Some(pos.clone()); // 克隆位置数据
                break; // 只需要第一个相机（通常只有一个）
            }

            // 查询渲染配置（控制各层、网格、障碍物等显示）
            let mut config_opt = None;
            for (_, cfg) in world.query::<&RenderConfig>().iter() {
                config_opt = Some(cfg.clone()); // 克隆配置数据
                break; // 只需要一个配置
            }

            // 检查是否成功获取所有必需数据
            match (camera_opt, camera_pos_opt, config_opt) {
                (Some(cam), Some(pos), Some(cfg)) => (cam, pos, cfg),
                _ => return Ok(()), // 缺少相机或配置，跳过渲染（避免崩溃）
            }
        };

        // ====================================================================
        // 步骤2: 计算视口范围（世界坐标）
        // ====================================================================
        // 视口是相机在世界中能看到的矩形区域
        // 缩放影响视口大小：zoom=2.0时看到的范围缩小一半，zoom=0.5时扩大两倍

        // 计算视口半宽（世界坐标）
        let half_width = (camera.screen_width / 2.0) / camera.zoom;
        // 计算视口半高（世界坐标）
        let half_height = (camera.screen_height / 2.0) / camera.zoom;

        // 视口左边界（世界X坐标）
        let view_left = camera_pos.x - half_width;
        // 视口右边界（世界X坐标）
        let view_right = camera_pos.x + half_width;
        // 视口上边界（世界Y坐标）
        let view_top = camera_pos.y - half_height;
        // 视口下边界（世界Y坐标）
        let view_bottom = camera_pos.y + half_height;

        // ====================================================================
        // 步骤3: 按层级渲染（Back → Middle → Front）
        // ====================================================================
        // 渲染顺序固定：从远到近（画家算法）
        // Back层：地面、地板
        // Middle层：装饰物、低矮墙壁
        // Front层：建筑、树木、前景物体

        let layers = [
            (TileLayer::Back, config.show_back),     // 背景层
            (TileLayer::Middle, config.show_middle), // 中间层
            (TileLayer::Front, config.show_front),   // 前景层
        ];

        // 遍历每个层级
        for (layer, should_show) in layers.iter() {
            // 检查该层是否启用（用户可通过1/2/3键切换）
            if !should_show {
                continue; // 跳过隐藏的层
            }

            // ================================================================
            // 步骤3.1: 准备瓦片收集容器
            // ================================================================
            // 收集该层的所有瓦片（包括动画瓦片）
            // 元组格式: (grid_x, grid_y, lib_index, img_index, is_anim, use_blend)
            let mut tiles_to_draw: Vec<(i32, i32, i16, usize, bool, bool)> = Vec::new();

            // ================================================================
            // 步骤3.2: 计算该层的视口扩展
            // ================================================================
            // Front 层需要更大的底部视口，因为建筑物高度可达数百像素
            // 建筑物的UV坐标在左下角，所以底部需要更多缓冲区
            let bottom_extra = if matches!(layer, TileLayer::Front) {
                800.0 // Front 层底部额外扩展 800 像素（容纳高大建筑）
            } else {
                200.0 // Back/Middle 层保持 200 像素（小装饰物）
            };

            // ================================================================
            // 步骤3.3: 标记动画瓦片实体（防止重复绘制）
            // ================================================================
            // 一个实体可能同时拥有 MapTile 和 AnimatedTile 组件
            // 为了避免在静态瓦片循环和动画瓦片循环中重复绘制，
            // 先用HashSet记录所有动画实体的ID
            use crate::ecs::components::AnimatedTile;
            let mut animated_entities = std::collections::HashSet::new();
            let current_layer = *layer; // 解引用layer以便在filter闭包中使用

            // 查询所有拥有MapTile和AnimatedTile的实体，并筛选当前层
            for (entity, (tile, _)) in world
                .query::<(&MapTile, &AnimatedTile)>()
                .iter()
                .filter(|(_, (t, _))| t.layer == current_layer)
            {
                animated_entities.insert(entity); // 记录实体ID
            }

            // ================================================================
            // 步骤3.4: 收集静态瓦片（排除动画瓦片）
            // ================================================================
            if config.show_static_tiles {
                // 检查静态瓦片开关（S键切换）
                // 查询所有MapTile实体，并筛选当前层
                for (entity, tile) in world
                    .query::<&MapTile>()
                    .iter()
                    .filter(|(_, t)| t.layer == current_layer)
                {
                    // 如果这个实体有动画组件，跳过（稍后在动画瓦片部分绘制）
                    if animated_entities.contains(&entity) {
                        continue; // 避免重复绘制
                    }

                    // --------------------------------------------------------
                    // 步骤3.4.1: 计算瓦片的世界坐标（格子→世界）
                    // --------------------------------------------------------
                    let world_x = (tile.grid_x * CELL_WIDTH) as f32; // 世界X = 格子X * 48
                    let world_y = (tile.grid_y * CELL_HEIGHT) as f32; // 世界Y = 格子Y * 32

                    // --------------------------------------------------------
                    // 步骤3.4.2: 视口裁剪（只绘制可见瓦片）
                    // --------------------------------------------------------
                    // 检查瓦片是否在视口范围内（带200px/800px缓冲区）
                    // 缓冲区防止瓦片突然出现/消失，提供平滑过渡
                    if world_x > view_right + 200.0       // 超出右边界
                    || world_x < view_left - 200.0     // 超出左边界
                    || world_y > view_bottom + bottom_extra  // 超出下边界（Front层更大）
                    || world_y < view_top - 200.0
                    // 超出上边界
                    {
                        continue; // 跳过不可见瓦片（性能优化）
                    }

                    // --------------------------------------------------------
                    // 步骤3.4.3: 添加瓦片到绘制列表
                    // --------------------------------------------------------
                    tiles_to_draw.push((
                        tile.grid_x,               // 格子X坐标
                        tile.grid_y,               // 格子Y坐标
                        tile.library_index,        // 图库索引（0-399）
                        tile.image_index as usize, // 图像索引（库内编号）
                        false,                     // 标记：不是动画瓦片
                        tile.use_blend,            // 是否使用ADD混合模式
                    ));
                }
            } // 静态瓦片收集结束

            // ================================================================
            // 步骤3.5: 收集动画瓦片
            // ================================================================
            // 动画瓦片开关（D键）独立于动画播放控制（A键）
            // - show_animated_tiles: 是否显示动画瓦片（D键）
            // - show_animations: 是否播放动画（A键）
            if config.show_animated_tiles {
                // 检查动画瓦片开关
                // 根据 show_animations 决定是否播放动画
                // 暂停时仍显示第一帧，只是不更新帧索引
                if config.show_animations {
                    // 检查动画播放开关
                    // 查询所有拥有MapTile和AnimatedTile的实体，并筛选当前层
                    for (_, (tile, anim)) in world
                        .query::<(&MapTile, &AnimatedTile)>()
                        .iter()
                        .filter(|(_, (t, _))| t.layer == current_layer)
                    {
                        // ------------------------------------------------
                        // 步骤3.5.1: 计算动画瓦片的世界坐标
                        // ------------------------------------------------
                        let world_x = (tile.grid_x * CELL_WIDTH) as f32;
                        let world_y = (tile.grid_y * CELL_HEIGHT) as f32;

                        // ------------------------------------------------
                        // 步骤3.5.2: 视口裁剪（与静态瓦片相同规则）
                        // ------------------------------------------------
                        if world_x > view_right + 200.0
                            || world_x < view_left - 200.0
                            || world_y > view_bottom + bottom_extra
                            || world_y < view_top - 200.0
                        {
                            continue; // 跳过不可见的动画瓦片
                        }

                        // ------------------------------------------------
                        // 步骤3.5.3: 添加动画瓦片到绘制列表
                        // ------------------------------------------------
                        // 注意：当前使用基础图像索引（简化版本）
                        // 完整实现应该由AnimationSystem计算当前帧：
                        //   current_frame = (elapsed_time / frame_interval) % frame_count
                        //   current_image_index = base_image_index + current_frame
                        tiles_to_draw.push((
                            tile.grid_x,               // 格子X坐标
                            tile.grid_y,               // 格子Y坐标
                            tile.library_index,        // 图库索引
                            tile.image_index as usize, // 图像索引（应该是当前帧）
                            true,                      // 标记：是动画瓦片
                            tile.use_blend,            // 混合模式
                        ));
                    }
                }
            } // 动画瓦片收集结束

            // ================================================================
            // 步骤3.6: 渲染该层的所有瓦片
            // ================================================================
            // 遍历绘制列表中的每个瓦片（静态+动画）
            for (grid_x, grid_y, lib_index, img_index, is_anim, use_blend) in tiles_to_draw {
                // --------------------------------------------------------
                // 步骤3.6.1: 获取图库
                // --------------------------------------------------------
                // 根据图库索引获取图库实例（如果图库不存在则跳过）
                if let Some(lib) = get_map_library(lib_index) {
                    // 获取图库锁（多线程安全）
                    if let Ok(mut lib_guard) = lib.lock() {
                        // ------------------------------------------------
                        // 步骤3.6.2: 获取图像尺寸
                        // ------------------------------------------------
                        // 从图库获取图像的宽度和高度
                        // 如果获取失败，使用默认格子尺寸（48x32）
                        let (tile_w, tile_h) = lib_guard
                            .get_size(img_index)
                            .unwrap_or((CELL_WIDTH as i16, CELL_HEIGHT as i16));

                        // ------------------------------------------------
                        // 步骤3.6.3: 获取纹理信息
                        // ------------------------------------------------
                        // 从图库获取或创建GPU纹理
                        // ImageInfo包含：image（纹理对象）、x/y（内部偏移）、width/height
                        if let Ok(info) = lib_guard.get_or_create_texture(ctx, img_index) {
                            // 检查纹理对象是否存在
                            if let Some(image) = &info.image {
                                // ============================================
                                // 步骤3.6.4: 计算基础世界坐标
                                // ============================================
                                // 格子坐标转换为世界坐标
                                let world_x = (grid_x * CELL_WIDTH) as f32; // X = grid_x * 48
                                let world_y = (grid_y * CELL_HEIGHT) as f32; // Y = grid_y * 32

                                // ============================================
                                // 步骤3.6.5: 计算Y轴偏移（核心逻辑）
                                // ============================================
                                // 这是地图渲染最复杂的部分，直接影响瓦片对齐
                                //
                                // 参考C# MapEditor渲染逻辑 (Main.cs:785-1030):
                                //
                                // **Back层** (line 971-993):
                                //   永远使用: drawY = Y * CellHeight
                                //   原因：背景层总是贴地，无需偏移
                                //
                                // **Middle层** (line 919-970):
                                //   - 标准尺寸: drawY = Y * CellHeight
                                //   - 非标准尺寸: drawY = (Y+1) * CellHeight - Height
                                //   原因：高于一格的物体需要向上偏移，使底部对齐格子
                                //
                                // **Front层** (line 785-876):
                                //   - 标准尺寸: drawY = Y * CellHeight
                                //   - 非标准 + Blend + 特殊库(14/27/100-199):
                                //       drawY = (Y+1)*CellHeight - 3*CellHeight = Y*CellHeight - 2*CellHeight
                                //       原因：火把/蜡烛等光效需要额外向上偏移2格（64px）
                                //   - 非标准 + 其他: drawY = (Y+1) * CellHeight - Height
                                //       原因：建筑/树木底部对齐格子底部

                                // --------------------------------------------
                                // 判断是否为标准尺寸
                                // --------------------------------------------
                                // 标准尺寸：48x32（1格）或96x64（2格）
                                let is_standard_size = (tile_w == CELL_WIDTH as i16
                                    && tile_h == CELL_HEIGHT as i16)
                                    || (tile_w == CELL_WIDTH as i16 * 2
                                        && tile_h == CELL_HEIGHT as i16 * 2);

                                // --------------------------------------------
                                // 计算调整后的坐标
                                // --------------------------------------------
                                let adjusted_x = world_x; // X坐标不需要偏移

                                let adjusted_y = if matches!(layer, TileLayer::Back) {
                                    // ========================================
                                    // Back层规则：永远不偏移
                                    // ========================================
                                    // 背景层（地面）总是直接使用格子Y坐标
                                    world_y
                                } else if is_standard_size {
                                    // ========================================
                                    // 标准尺寸规则：不偏移
                                    // ========================================
                                    // Middle层和Front层的标准尺寸（48x32或96x64）
                                    // 无需偏移，直接对齐格子顶部
                                    world_y
                                } else {
                                    // ========================================
                                    // 非标准尺寸规则：根据层级和属性偏移
                                    // ========================================
                                    // 参考C# GameScene.cs:11967-11972
                                    // Blend特殊处理仅用于Front层

                                    if matches!(layer, TileLayer::Front) && use_blend {
                                        // ------------------------------------
                                        // Front层 + 混合模式：检查库索引
                                        // ------------------------------------
                                        if lib_index == 14
                                            || lib_index == 27
                                            || (lib_index > 99 && lib_index < 199)
                                        {
                                            // 特殊库：火把、蜡烛等光效
                                            // 使用额外的向上偏移（-2格 = -64px）
                                            //
                                            // C#公式: drawY - (3*CellHeight)
                                            //       = (y+1)*32 - 96 = (y-2)*32
                                            // Rust公式: world_y = y*32
                                            //         所以 y*32 - 64 = (y-2)*32
                                            world_y - (2 * CELL_HEIGHT) as f32
                                        } else {
                                            // 普通混合：使用标准非标准偏移
                                            // 底部对齐格子底部
                                            //
                                            // C#公式: drawY - Height
                                            //       = (y+1)*32 - Height
                                            // Rust公式: y*32 + 32 - Height
                                            world_y + CELL_HEIGHT as f32 - tile_h as f32
                                        }
                                    } else {
                                        // ------------------------------------
                                        // 非Blend 或 非Front层：标准偏移
                                        // ------------------------------------
                                        // Middle层和Front层非混合的非标准尺寸
                                        // 向上偏移使底部对齐格子底部
                                        //
                                        // C#公式: drawY - Height (MapEditor统一规则)
                                        // Rust公式: y*32 + 32 - Height
                                        world_y + CELL_HEIGHT as f32 - tile_h as f32
                                    }
                                };

                                // ============================================
                                // 步骤3.6.6: 判断是否应用图像内部偏移
                                // ============================================
                                // 图像内部偏移 (info.x, info.y) 来自图库文件
                                // 用于精确定位图像，但并非所有瓦片都需要应用
                                //
                                // 参考C# GameScene.cs:11967-11980
                                // 只有特定情况才应用偏移：

                                let should_apply_offset = if matches!(layer, TileLayer::Front) {
                                    // Front层：根据混合模式和库索引判断
                                    if use_blend {
                                        // ------------------------------------
                                        // Blend瓦片：特殊库或特定索引
                                        // ------------------------------------
                                        // 库14/27/100-199：火把、蜡烛等光效
                                        // 索引2723-2732：特殊动画效果
                                        lib_index == 14
                                            || lib_index == 27
                                            || (lib_index > 99 && lib_index < 199)
                                            || (img_index >= 2723 && img_index <= 2732)
                                    } else if lib_index == 28 {
                                        // ------------------------------------
                                        // 库28：仅当有非空偏移时应用
                                        // ------------------------------------
                                        info.x != 0 || info.y != 0
                                    } else {
                                        // 其他Front层瓦片：不应用偏移
                                        false
                                    }
                                } else {
                                    // ========================================
                                    // Back/Middle层：永远不应用偏移
                                    // ========================================
                                    // 重要：这防止了Back层错位问题
                                    false
                                };

                                // --------------------------------------------
                                // 应用或跳过图像内部偏移
                                // --------------------------------------------
                                let (adjusted_x_final, adjusted_y_final) = if should_apply_offset {
                                    // 应用偏移（火把、蜡烛等）
                                    (adjusted_x + info.x as f32, adjusted_y + info.y as f32)
                                } else {
                                    // 不应用偏移（大部分瓦片）
                                    (adjusted_x, adjusted_y)
                                };

                                // ============================================
                                // 步骤3.6.7: 世界坐标转换为屏幕坐标
                                // ============================================
                                // 世界坐标：物体在游戏世界中的绝对位置
                                // 屏幕坐标：物体在屏幕上的像素位置
                                //
                                // 转换公式：
                                //   screen = (world - camera_pos) * zoom + screen_center
                                //
                                // 步骤：
                                // 1. 减去相机位置（相对坐标）
                                // 2. 乘以缩放系数（缩放效果）
                                // 3. 加上屏幕中心（屏幕坐标原点在中心）

                                let screen_x = (adjusted_x_final - camera_pos.x) * camera.zoom
                                    + camera.screen_width / 2.0;
                                let screen_y = (adjusted_y_final - camera_pos.y) * camera.zoom
                                    + camera.screen_height / 2.0;

                                // ============================================
                                // 步骤3.6.8: 设置混合模式
                                // ============================================
                                // 混合模式控制像素如何叠加
                                // - Normal: 标准Alpha混合（默认）
                                // - ADD: 叠加发光（火焰、灯光等）

                                let old_blend_mode = if use_blend {
                                    // 保存当前混合模式
                                    let current = canvas.blend_mode();
                                    // 切换到ADD混合（发光效果）
                                    canvas.set_blend_mode(ggez::graphics::BlendMode::ADD);
                                    Some(current) // 返回旧模式以便恢复
                                } else {
                                    None // 使用默认混合模式
                                };

                                // ============================================
                                // 步骤3.6.9: 绘制瓦片到画布
                                // ============================================
                                // 使用DrawParam配置绘制参数
                                canvas.draw(
                                    image, // GPU纹理对象
                                    DrawParam::default()
                                        .dest([screen_x, screen_y]) // 目标位置（屏幕坐标）
                                        .scale([camera.zoom, camera.zoom]) // 缩放（跟随相机）
                                        .color(ggez::graphics::Color::WHITE), // 白色=不着色，保留原图
                                );

                                // ============================================
                                // 步骤3.6.10: 恢复混合模式
                                // ============================================
                                // 如果之前切换了混合模式，恢复到原来的模式
                                // 避免影响后续渲染
                                if let Some(old_mode) = old_blend_mode {
                                    canvas.set_blend_mode(old_mode);
                                }
                            } // image存在检查结束
                        } // 纹理获取结果检查结束
                    } // 图库锁获取结束
                } // 图库存在检查结束
            } // 瓦片绘制循环结束
        } // 层级循环结束

        // ====================================================================
        // 步骤4: 返回渲染结果
        // ====================================================================
        Ok(()) // 渲染成功
    }
}

// ============================================================================
// 总结
// ============================================================================
//
// 地图渲染系统的核心流程：
// 1. 获取相机和配置
// 2. 计算视口范围
// 3. 按层级循环（Back → Middle → Front）
//    3.1 准备瓦片容器
//    3.2 计算视口扩展
//    3.3 标记动画瓦片
//    3.4 收集静态瓦片
//    3.5 收集动画瓦片
//    3.6 渲染所有瓦片
//        3.6.1 获取图库
//        3.6.2 获取图像尺寸
//        3.6.3 获取纹理信息
//        3.6.4 计算基础世界坐标
//        3.6.5 计算Y轴偏移（层级规则）
//        3.6.6 判断图像内部偏移（特殊情况）
//        3.6.7 世界→屏幕坐标转换
//        3.6.8 设置混合模式
//        3.6.9 绘制瓦片
//        3.6.10 恢复混合模式
// 4. 返回渲染结果
//
// 关键要点：
// - 视口裁剪减少99%渲染量
// - 层级顺序确保正确遮挡关系
// - 精确偏移计算确保对齐
// - 条件偏移防止Back层错位
// - 混合模式实现发光效果
//
// 调试技巧：
// - 按1/2/3切换层级查看
// - 按S/D分离静态/动画瓦片
// - 按G显示网格辅助定位
// - 检查LOG输出排查问题
//
// ============================================================================

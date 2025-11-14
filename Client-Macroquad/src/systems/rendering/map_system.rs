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

use crate::components::{AnimatedTile, MapTile, RenderConfig};
use crate::systems::RenderSystem;
use crate::game::{GameContext, GameResult};

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

        let total_ticks = frame_count as u32 + frame_count as u32 * frame_interval as u32;
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
        _world: &hecs::World,
    ) -> crate::game::GameResult {
        // TODO: 重写为 macroquad API
        // 原实现有 470+ 行，需要重写:
        // 1. 获取相机和渲染配置
        // 2. 计算可见区域
        // 3. 渲染地图瓦片 (back/middle/front 三层)
        // 4. 处理动画瓦片
        Ok(())
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

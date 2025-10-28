// ============================================================================
// HUD Render System - HUD渲染系统
// ============================================================================
//
// 🎯 Layer 4 - Rendering & Playback Layer（渲染与播放层）
//
// 职责：
// - 渲染HUD元素（血条、魔法条、迷你地图、buff图标、目标信息等）
// - HUD是"固定在屏幕上的游戏信息显示"
// - 从ECS组件读取游戏状态数据并渲染
//
// 不负责：
// - UI对话框渲染（由UIRenderSystem负责）
// - 游戏逻辑（只读取数据，不修改）
// - 事件处理（由Layer 5负责）
//
// 与UIRenderSystem的区别：
// - HUDRenderSystem: 游戏内信息显示（血条、地图、buff等）
// - UIRenderSystem: 菜单和对话框（背包、技能树、聊天等）
//
// ============================================================================

use ggez::{Context, GameResult};
use ggez::graphics::{self, Canvas, Color, DrawMode, DrawParam, Mesh, Rect, Text};
use ggez::glam::Vec2;
use hecs::World;
use crate::ecs::components::{Player, Position, Camera};

/// HUD渲染系统（Layer 4）
/// 
/// # 设计原则
/// - 仅负责渲染HUD元素
/// - 不处理点击、拖拽等交互（由Layer 5负责）
/// - 只读取组件数据，不修改游戏状态
pub struct HUDRenderSystem;

impl HUDRenderSystem {
    /// 渲染所有HUD元素
    /// 
    /// # 参数
    /// - `ctx`: ggez上下文
    /// - `canvas`: 渲染目标画布
    /// - `world`: ECS世界（用于查询玩家状态）
    pub fn render(ctx: &mut Context, canvas: &mut Canvas, world: &World) -> GameResult {
        // 查找玩家实体
        let player_data = Self::get_player_data(world);
        
        if let Some((health, max_health, mana, max_mana, buffs)) = player_data {
            // 渲染玩家状态栏（左上角）
            Self::render_player_status(ctx, canvas, health, max_health, mana, max_mana)?;
            
            // 渲染buff图标（血条下方）
            Self::render_buffs(ctx, canvas, &buffs)?;
        }
        
        // 渲染目标信息（如果有选中目标）
        Self::render_target_info(ctx, canvas, world)?;
        
        // 渲染迷你地图（右上角）
        Self::render_minimap(ctx, canvas, world)?;
        
        // 渲染调试信息（左下角，仅开发模式）
        #[cfg(debug_assertions)]
        Self::render_debug_info(ctx, canvas, world)?;
        
        Ok(())
    }
    
    // ========================================================================
    // 玩家状态渲染
    // ========================================================================
    
    /// 渲染玩家血条和魔法条
    fn render_player_status(
        ctx: &mut Context,
        canvas: &mut Canvas,
        health: i32,
        max_health: i32,
        mana: i32,
        max_mana: i32,
    ) -> GameResult {
        let x = 20.0;
        let y = 20.0;
        let bar_width = 200.0;
        let bar_height = 20.0;
        let spacing = 5.0;
        
        // 渲染血条
        Self::render_bar(
            ctx,
            canvas,
            Vec2::new(x, y),
            bar_width,
            bar_height,
            health as f32 / max_health as f32,
            Color::new(0.8, 0.0, 0.0, 1.0), // 红色
            Color::new(0.2, 0.0, 0.0, 0.5), // 深红色背景
        )?;
        
        // 渲染血量文本
        let health_text = format!("{}/{}", health, max_health);
        Self::render_text(ctx, canvas, &health_text, Vec2::new(x + 5.0, y + 2.0), Color::WHITE)?;
        
        // 渲染魔法条
        let mana_y = y + bar_height + spacing;
        Self::render_bar(
            ctx,
            canvas,
            Vec2::new(x, mana_y),
            bar_width,
            bar_height,
            mana as f32 / max_mana as f32,
            Color::new(0.0, 0.0, 0.8, 1.0), // 蓝色
            Color::new(0.0, 0.0, 0.2, 0.5), // 深蓝色背景
        )?;
        
        // 渲染魔法值文本
        let mana_text = format!("{}/{}", mana, max_mana);
        Self::render_text(ctx, canvas, &mana_text, Vec2::new(x + 5.0, mana_y + 2.0), Color::WHITE)?;
        
        Ok(())
    }
    
    /// 渲染进度条（通用方法）
    fn render_bar(
        ctx: &mut Context,
        canvas: &mut Canvas,
        pos: Vec2,
        width: f32,
        height: f32,
        fill_ratio: f32,
        fill_color: Color,
        bg_color: Color,
    ) -> GameResult {
        // 背景
        let bg_rect = Rect::new(pos.x, pos.y, width, height);
        let bg_mesh = Mesh::new_rectangle(ctx, DrawMode::fill(), bg_rect, bg_color)?;
        canvas.draw(&bg_mesh, DrawParam::default());
        
        // 前景（填充部分）
        let fill_width = width * fill_ratio.clamp(0.0, 1.0);
        if fill_width > 0.0 {
            let fill_rect = Rect::new(pos.x, pos.y, fill_width, height);
            let fill_mesh = Mesh::new_rectangle(ctx, DrawMode::fill(), fill_rect, fill_color)?;
            canvas.draw(&fill_mesh, DrawParam::default());
        }
        
        // 边框
        let border_rect = Rect::new(pos.x, pos.y, width, height);
        let border_mesh = Mesh::new_rectangle(ctx, DrawMode::stroke(1.0), border_rect, Color::BLACK)?;
        canvas.draw(&border_mesh, DrawParam::default());
        
        Ok(())
    }
    
    // ========================================================================
    // Buff图标渲染
    // ========================================================================
    
    /// 渲染buff图标
    fn render_buffs(ctx: &mut Context, canvas: &mut Canvas, buffs: &[Buff]) -> GameResult {
        let x = 20.0;
        let y = 70.0; // 在血条和魔法条下方
        let icon_size = 32.0;
        let spacing = 5.0;
        
        for (i, buff) in buffs.iter().enumerate() {
            let icon_x = x + (icon_size + spacing) * i as f32;
            
            // 渲染buff图标背景
            let icon_rect = Rect::new(icon_x, y, icon_size, icon_size);
            let bg_mesh = Mesh::new_rectangle(
                ctx,
                DrawMode::fill(),
                icon_rect,
                Color::new(0.2, 0.2, 0.2, 0.8),
            )?;
            canvas.draw(&bg_mesh, DrawParam::default());
            
            // 渲染buff名称（简化版，实际应该用图标）
            let buff_text = format!("{}", buff.buff_type as u8);
            Self::render_text(
                ctx,
                canvas,
                &buff_text,
                Vec2::new(icon_x + 10.0, y + 10.0),
                Color::WHITE,
            )?;
            
            // 渲染剩余时间
            if let Some(remaining) = buff.remaining_duration {
                let time_text = format!("{}s", remaining);
                Self::render_text(
                    ctx,
                    canvas,
                    &time_text,
                    Vec2::new(icon_x + 5.0, y + icon_size - 15.0),
                    Color::YELLOW,
                )?;
            }
        }
        
        Ok(())
    }
    
    // ========================================================================
    // 目标信息渲染
    // ========================================================================
    
    /// 渲染选中目标的信息
    fn render_target_info(ctx: &mut Context, canvas: &mut Canvas, world: &World) -> GameResult {
        // TODO: 实现目标选择系统后，这里从组件读取当前选中的目标
        // 暂时跳过
        Ok(())
    }
    
    // ========================================================================
    // 迷你地图渲染
    // ========================================================================
    
    /// 渲染迷你地图（右上角）
    fn render_minimap(ctx: &mut Context, canvas: &mut Canvas, world: &World) -> GameResult {
        let screen_width = ctx.gfx.drawable_size().0;
        let map_size = 150.0;
        let x = screen_width - map_size - 20.0;
        let y = 20.0;
        
        // 渲染地图背景
        let map_rect = Rect::new(x, y, map_size, map_size);
        let bg_mesh = Mesh::new_rectangle(
            ctx,
            DrawMode::fill(),
            map_rect,
            Color::new(0.1, 0.1, 0.1, 0.8),
        )?;
        canvas.draw(&bg_mesh, DrawParam::default());
        
        // 渲染地图边框
        let border_mesh = Mesh::new_rectangle(
            ctx,
            DrawMode::stroke(2.0),
            map_rect,
            Color::new(0.5, 0.5, 0.5, 1.0),
        )?;
        canvas.draw(&border_mesh, DrawParam::default());
        
        // 查找玩家位置
        if let Some(player_pos) = Self::get_player_position(world) {
            // 渲染玩家位置（简化版，实际应该根据地图坐标转换）
            let player_x = x + map_size / 2.0;
            let player_y = y + map_size / 2.0;
            let player_dot = Mesh::new_circle(
                ctx,
                DrawMode::fill(),
                Vec2::new(player_x, player_y),
                3.0,
                0.1,
                Color::GREEN,
            )?;
            canvas.draw(&player_dot, DrawParam::default());
        }
        
        Ok(())
    }
    
    // ========================================================================
    // 调试信息渲染
    // ========================================================================
    
    /// 渲染调试信息（FPS、实体数量等）
    #[cfg(debug_assertions)]
    fn render_debug_info(ctx: &mut Context, canvas: &mut Canvas, world: &World) -> GameResult {
        let x = 20.0;
        let screen_height = ctx.gfx.drawable_size().1;
        let y = screen_height - 100.0;
        
        // FPS
        let fps = ctx.time.fps();
        let fps_text = format!("FPS: {:.1}", fps);
        Self::render_text(ctx, canvas, &fps_text, Vec2::new(x, y), Color::YELLOW)?;
        
        // 实体数量
        let entity_count = world.len();
        let entity_text = format!("Entities: {}", entity_count);
        Self::render_text(ctx, canvas, &entity_text, Vec2::new(x, y + 20.0), Color::YELLOW)?;
        
        Ok(())
    }
    
    // ========================================================================
    // 辅助方法
    // ========================================================================
    
    /// 获取玩家数据（暂时使用模拟数据）
    fn get_player_data(world: &World) -> Option<(i32, i32, i32, i32, Vec<Buff>)> {
        for (_, _player) in world.query::<&Player>().iter() {
            // TODO: 从Health和Mana组件读取
            // 暂时返回模拟数据
            let buffs = Vec::new();
            return Some((100, 100, 50, 50, buffs));
        }
        None
    }
    
    /// 获取玩家位置
    fn get_player_position(world: &World) -> Option<Vec2> {
        for (_, (_, pos)) in world.query::<(&Player, &Position)>().iter() {
            return Some(Vec2::new(pos.x as f32, pos.y as f32));
        }
        None
    }
    
    /// 渲染文本（通用方法）
    fn render_text(
        ctx: &mut Context,
        canvas: &mut Canvas,
        text: &str,
        pos: Vec2,
        color: Color,
    ) -> GameResult {
        let mut text_obj = Text::new(text);
        text_obj.set_scale(16.0);
        
        canvas.draw(
            &text_obj,
            DrawParam::default()
                .dest(pos)
                .color(color),
        );
        
        Ok(())
    }
}

// ============================================================================
// 临时Buff类型（应该在components中定义）
// ============================================================================

#[derive(Debug, Clone)]
pub struct Buff {
    pub buff_type: BuffType,
    pub remaining_duration: Option<i32>,
}

#[derive(Debug, Clone, Copy)]
pub enum BuffType {
    AttackBoost,
    DefenseBoost,
    SpeedBoost,
    Poison,
}

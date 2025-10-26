// ============================================================================
// Render System - 渲染系统模块化
// ============================================================================

mod debug;
mod item;
mod monster;
mod npc;
mod player;
mod tiles;
mod ui;  // UI渲染方法 (RenderSystem::draw_ui)

// 重新导出所有公共函数
pub use debug::*;
pub use item::*;
pub use monster::*;
pub use npc::*;
pub use player::*;
pub use tiles::*;

use ggez::{Context};
use ggez::graphics::{self, Canvas, DrawParam, Color, BlendMode, BlendComponent, BlendFactor, BlendOperation, Text, TextFragment, PxScale, Rect, Mesh};
use crate::ecs::components::{Camera, QuestIcon};
/// 渲染系统主结构
pub struct RenderSystem;

impl RenderSystem {
    /// 绘制NPC名字(带半透明黑色背景)
    pub(crate) fn draw_npc_name(
        ctx: &Context,
        canvas: &mut Canvas,
        name: &str,
        center_x: f32,
        y: f32,
        camera: &Camera,
    ) {
        // 创建文本
        let text_fragment = TextFragment::new(name)
            .scale(PxScale::from(14.0 * camera.zoom))
            .color(Color::from_rgb(255, 255, 0)); // 黄色
        let text = Text::new(text_fragment);

        // 计算文本尺寸
        let text_dims = text.measure(ctx).unwrap();
        let text_width = text_dims.x;
        let text_height = text_dims.y;

        // 居中对齐
        let text_x = center_x - text_width / 2.0;

        // 绘制半透明黑色背景
        let bg_padding = 4.0 * camera.zoom;
        let bg_rect = Rect::new(
            text_x - bg_padding,
            y - bg_padding,
            text_width + bg_padding * 2.0,
            text_height + bg_padding * 2.0,
        );

        if let Ok(bg_mesh) = Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::fill(),
            bg_rect,
            Color::from_rgba(0, 0, 0, 180),
        ) {
            canvas.draw(&bg_mesh, DrawParam::default());
        }

        // 绘制文本
        canvas.draw(&text, DrawParam::default().dest([text_x, y]));
    }

    /// 绘制任务图标
    pub(crate) fn draw_quest_icon(
        ctx: &Context,
        canvas: &mut Canvas,
        icon: QuestIcon,
        center_x: f32,
        y: f32,
        camera: &Camera,
    ) {
        let (symbol, color) = match icon {
            QuestIcon::None => return,                                      // 无图标
            QuestIcon::Available => ("!", Color::from_rgb(255, 255, 0)),    // 黄色感叹号
            QuestIcon::Complete => ("?", Color::from_rgb(255, 255, 0)),     // 黄色问号
            QuestIcon::Incomplete => ("?", Color::from_rgb(150, 150, 150)), // 灰色问号
        };

        // 创建图标文本
        let text_fragment = TextFragment::new(symbol)
            .scale(PxScale::from(24.0 * camera.zoom))
            .color(color);
        let text = Text::new(text_fragment);

        // 居中绘制
        let text_dims = text.measure(ctx).unwrap();
        let text_x = center_x - text_dims.x / 2.0;

        canvas.draw(&text, DrawParam::default().dest([text_x, y]));
    }

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

    /// 将i32 ARGB颜色转换为ggez::Color
    pub(crate) fn argb_to_color(argb: i32) -> Color {
        if argb == 0 {
            return Color::WHITE; // 默认白色(无染色)
        }

        let a = ((argb >> 24) & 0xFF) as u8;
        let r = ((argb >> 16) & 0xFF) as u8;
        let g = ((argb >> 8) & 0xFF) as u8;
        let b = (argb & 0xFF) as u8;

        Color::from_rgba(r, g, b, a)
    }
}

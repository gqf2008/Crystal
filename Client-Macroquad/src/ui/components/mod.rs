// ============================================================================
// UI Components Module
// 基于egui的UI组件系统
// ============================================================================
// 
// 【设计理念】
// 1. 优先使用egui原生组件（Button, Label, TextEdit等）
// 2. 仅在需要纹理支持时才创建自定义组件
// 3. 所有组件均由egui负责绘制，而非macroquad
// 4. 保持API的简洁性和一致性
// 
// ============================================================================

pub mod dialog;
pub mod shop_viewer;
pub mod checkbox;
pub mod checkbox_test;
pub mod textured_widgets;
pub mod label;
pub mod message_box;

pub use dialog::*;
pub use shop_viewer::*;
pub use checkbox::*;
pub use checkbox_test::*;
pub use textured_widgets::*;
pub use label::*;
pub use message_box::*;

use egui_macroquad::egui;
use crate::resources::LibraryName;

/// UI组件基础特征
pub trait Control {
    /// 获取控件ID
    fn id(&self) -> egui::Id;
    
    /// 获取位置
    fn position(&self) -> egui::Pos2;
    
    /// 设置位置
    fn set_position(&mut self, pos: egui::Pos2);
    
    /// 获取尺寸
    fn size(&self) -> egui::Vec2;
    
    /// 是否可见
    fn visible(&self) -> bool;
    
    /// 设置可见性
    fn set_visible(&mut self, visible: bool);
    
    /// 绘制控件
    fn draw(&mut self, ui: &mut egui::Ui, ctx: &egui::Context);
}

/// 纹理控件基础特征
pub trait ImageControl: Control {
    /// 获取纹理库
    fn library(&self) -> LibraryName;
    
    /// 设置纹理库
    fn set_library(&mut self, library: LibraryName);
    
    /// 获取纹理索引
    fn index(&self) -> usize;
    
    /// 设置纹理索引
    fn set_index(&mut self, index: usize);
    
    /// 绘制纹理
    fn draw_texture(&self, ui: &mut egui::Ui, ctx: &egui::Context, rect: egui::Rect) {
        if let Some(info) = self.library().get_egui_texture(ctx, self.index()) {
            if let Some(texture) = info.egui_texture {
                ui.painter().image(
                    texture.id(),
                    rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
        }
    }
}

/// 通用响应处理
#[derive(Debug, Clone)]
pub struct UiResponse {
    pub clicked: bool,
    pub hovered: bool,
    pub pressed: bool,
    pub dragged: bool,
    pub drag_delta: egui::Vec2,
}

impl UiResponse {
    pub fn from_egui_response(response: egui::Response) -> Self {
        Self {
            clicked: response.clicked(),
            hovered: response.hovered(),
            pressed: response.is_pointer_button_down_on(),
            dragged: response.dragged(),
            drag_delta: response.drag_delta(),
        }
    }
}

/// 通用布局辅助函数
pub struct MirLayout;

impl MirLayout {
    /// 计算网格位置
    pub fn grid_position(index: usize, cols: usize, cell_size: egui::Vec2, start_pos: egui::Pos2) -> egui::Pos2 {
        let row = index / cols;
        let col = index % cols;
        egui::pos2(
            start_pos.x + (col as f32) * cell_size.x,
            start_pos.y + (row as f32) * cell_size.y,
        )
    }
    
    /// 创建居中矩形
    pub fn centered_rect(center: egui::Pos2, size: egui::Vec2) -> egui::Rect {
        egui::Rect::from_center_size(center, size)
    }
    
    /// 创建带偏移的矩形
    pub fn offset_rect(base_pos: egui::Pos2, offset: egui::Vec2, size: egui::Vec2) -> egui::Rect {
        egui::Rect::from_min_size(base_pos + offset, size)
    }
}
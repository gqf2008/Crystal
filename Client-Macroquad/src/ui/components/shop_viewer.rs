// ============================================================================
// ShopItemViewer - 基于原版GameShopViewer的商品预览对话框
// ============================================================================
// 
// 【功能说明】
// 1. 商品详情显示窗口
// 2. 左右方向键切换商品
// 3. 自动定位和纹理管理
// 4. 模态显示阻挡其他操作
// 
// ============================================================================

use egui_macroquad::egui;
use crate::resources::LibraryName;
use super::{TexturedDialog, DialogType, TexturedButton};

/// 商品预览对话框
pub struct ShopItemViewer {
    /// 基础对话框
    pub dialog: TexturedDialog,
    /// 左方向按钮
    pub left_button: TexturedButton,
    /// 右方向按钮
    pub right_button: TexturedButton,
    /// 当前商品索引
    pub current_item_index: i32,
    /// 商品总数
    pub total_items: i32,
    /// 商品详情回调
    pub on_item_changed: Option<Box<dyn FnMut(i32)>>,
}

impl ShopItemViewer {
    pub fn new() -> Self {
        // 创建基础对话框
        let dialog = TexturedDialog::new("shop_item_viewer", "商品预览")
            .with_type(DialogType::Modal)
            .with_background(LibraryName::Title, 785) // 原版的Title[785]
            .with_rect(egui::pos2(264.0, 162.0), egui::vec2(218.0, 158.0))
            .with_close_button(None); // 不使用默认关闭按钮，用方向按钮关闭
        
        // 创建左方向按钮（原版位置 81,282）
        let left_button = TexturedButton::new()
            .with_library(LibraryName::Prguse2)
            .with_states(202, Some(203), Some(204), None)
            .with_size(egui::vec2(16.0, 14.0))
            .with_tooltip("上一页");
        
        // 创建右方向按钮（原版位置 160,282）
        let right_button = TexturedButton::new()
            .with_library(LibraryName::Prguse2)
            .with_states(205, Some(206), Some(207), None)
            .with_size(egui::vec2(16.0, 14.0))
            .with_tooltip("下一页");
            
        Self {
            dialog,
            left_button,
            right_button,
            current_item_index: 0,
            total_items: 0,
            on_item_changed: None,
        }
    }
    
    /// 设置商品总数
    pub fn with_total_items(mut self, total: i32) -> Self {
        self.total_items = total;
        self
    }
    
    /// 设置回调
    pub fn with_callback<F>(mut self, callback: F) -> Self 
    where F: FnMut(i32) + 'static 
    {
        self.on_item_changed = Some(Box::new(callback));
        self
    }
    
    /// 显示对话框
    pub fn show(&mut self) {
        self.dialog.show();
    }
    
    /// 隐藏对话框
    pub fn hide(&mut self) {
        self.dialog.hide();
    }
    
    /// 绘制
    pub fn draw(&mut self, ctx: &egui::Context) {
        if !self.dialog.visible {
            return;
        }
        
        // 绘制基础对话框
        if self.dialog.draw_base(ctx) {
            self.hide();
        }
        
        // 在对话框区域内绘制内容
        let dialog_rect = egui::Rect::from_min_size(self.dialog.position, self.dialog.size);
        
        // 我们需要获取一个Ui实例来绘制按钮
        // 这里我们使用Area来覆盖在对话框上面
        egui::Area::new(egui::Id::new("shop_viewer_content"))
            .fixed_pos(self.dialog.position)
            .order(self.dialog.order)
            .show(ctx, |ui| {
                // 绘制左按钮
                let left_rect = egui::Rect::from_min_size(
                    egui::pos2(81.0, 120.0), // 相对位置
                    egui::vec2(16.0, 14.0)
                );
                let mut left_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(left_rect)
                        .layout(*ui.layout())
                );
                if self.left_button.draw(&mut left_ui) {
                    if self.current_item_index > 0 {
                        self.current_item_index -= 1;
                        if let Some(cb) = &mut self.on_item_changed {
                            cb(self.current_item_index);
                        }
                    }
                }
                
                // 绘制右按钮
                let right_rect = egui::Rect::from_min_size(
                    egui::pos2(120.0, 120.0), // 相对位置
                    egui::vec2(16.0, 14.0)
                );
                let mut right_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(right_rect)
                        .layout(*ui.layout())
                );
                if self.right_button.draw(&mut right_ui) {
                    if self.current_item_index < self.total_items - 1 {
                        self.current_item_index += 1;
                        if let Some(cb) = &mut self.on_item_changed {
                            cb(self.current_item_index);
                        }
                    }
                }
                
                // 这里可以添加更多商品详情的绘制逻辑
                // ...
            });
    }
}

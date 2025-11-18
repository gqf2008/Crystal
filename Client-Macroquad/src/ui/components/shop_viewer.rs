// ============================================================================
// GameShopViewer - 基于原版GameShopViewer的商品预览对话框
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
use super::{MirDialog, DialogType};

/// 商品预览对话框
pub struct GameShopViewer {
    /// 基础对话框
    pub dialog: MirDialog,
    /// 左方向按钮
    pub left_button: MirButton,
    /// 右方向按钮
    pub right_button: MirButton,
    /// 当前商品索引
    pub current_item_index: i32,
    /// 商品总数
    pub total_items: i32,
    /// 商品详情回调
    pub on_item_changed: Option<Box<dyn FnMut(i32)>>,
}

impl GameShopViewer {
    pub fn new() -> Self {
        // 创建基础对话框
        let mut dialog = MirDialog::new("game_shop_viewer", "商品预览")
            .with_type(DialogType::Modal)
            .with_background(LibraryName::Title, 785) // 原版的Title[785]
            .with_rect(egui::pos2(264.0, 162.0), egui::vec2(218.0, 158.0))
            .with_close_button(None); // 不使用默认关闭按钮，用方向按钮关闭
        
        // 创建左方向按钮（原版位置 81,282）
        let left_button = MirButton::new("shop_viewer_left")
            .with_library(LibraryName::Prguse2)
            .with_textures(202, Some(203), Some(204))
            .with_rect(egui::pos2(81.0, 120.0), egui::vec2(16.0, 14.0))
            .with_hint("上一页");
        
        // 创建右方向按钮（原版位置 160,282）
        let right_button = MirButton::new("shop_viewer_right")
            .with_library(LibraryName::Prguse2)
            .with_textures(205, Some(206), Some(207))
            .with_rect(egui::pos2(160.0, 120.0), egui::vec2(16.0, 14.0))
            .with_hint("下一页");
        
        Self {
            dialog,
            left_button,
            right_button,
            current_item_index: 0,
            total_items: 0,
            on_item_changed: None,
        }
    }
    
    /// 显示商品预览
    pub fn show_item(&mut self, item_index: i32, total_items: i32) {
        self.current_item_index = item_index;
        self.total_items = total_items;
        self.dialog.show();
        
        // 触发商品变化回调
        if let Some(ref mut callback) = self.on_item_changed {
            callback(item_index);
        }
    }
    
    /// 隐藏预览
    pub fn hide(&mut self) {
        self.dialog.hide();
    }
    
    /// 是否可见
    pub fn is_visible(&self) -> bool {
        self.dialog.visible
    }
    
    /// 设置商品变化回调
    pub fn set_item_changed_callback<F>(&mut self, callback: F)
    where 
        F: FnMut(i32) + 'static,
    {
        self.on_item_changed = Some(Box::new(callback));
    }
    
    /// 切换到上一个商品
    fn previous_item(&mut self) {
        if self.total_items > 0 {
            self.current_item_index = if self.current_item_index <= 0 {
                self.total_items - 1
            } else {
                self.current_item_index - 1
            };
            
            // 触发商品变化回调
            if let Some(ref mut callback) = self.on_item_changed {
                callback(self.current_item_index);
            }
        }
    }
    
    /// 切换到下一个商品
    fn next_item(&mut self) {
        if self.total_items > 0 {
            self.current_item_index = if self.current_item_index >= self.total_items - 1 {
                0
            } else {
                self.current_item_index + 1
            };
            
            // 触发商品变化回调
            if let Some(ref mut callback) = self.on_item_changed {
                callback(self.current_item_index);
            }
        }
    }
    
    /// 绘制预览窗口
    pub fn draw(&mut self, ctx: &egui::Context) -> bool {
        if !self.dialog.visible {
            return false;
        }
        
        let mut should_close = false;
        
        // 绘制对话框基础结构
        should_close = self.dialog.draw_base(ctx);
        
        // 绘制内容区域
        egui::Area::new(egui::Id::new("shop_viewer_content"))
            .fixed_pos(self.dialog.position)
            .movable(false)
            .order(egui::Order::Tooltip)
            .show(ctx, |ui| {
                // 绘制左方向按钮
                let left_pos = self.dialog.position + egui::vec2(self.left_button.position.x, self.left_button.position.y);
                let mut left_btn_copy = self.left_button.clone();
                left_btn_copy.position = left_pos;
                let left_response = left_btn_copy.show(ui, ctx);
                if left_response.clicked {
                    self.previous_item();
                }
                
                // 绘制右方向按钮
                let right_pos = self.dialog.position + egui::vec2(self.right_button.position.x, self.right_button.position.y);
                let mut right_btn_copy = self.right_button.clone();
                right_btn_copy.position = right_pos;
                let right_response = right_btn_copy.show(ui, ctx);
                if right_response.clicked {
                    self.next_item();
                }
                
                // ESC键关闭
                if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                    should_close = true;
                }
                
                // 左右箭头键导航
                if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
                    self.previous_item();
                }
                if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
                    self.next_item();
                }
                
                // 绘制商品信息区域（子类实现）
                self.draw_item_details(ui, ctx);
            });
        
        if should_close {
            self.hide();
        }
        
        should_close
    }
    
    /// 绘制商品详情（由子类实现具体内容）
    fn draw_item_details(&self, ui: &mut egui::Ui, ctx: &egui::Context) {
        // 商品详情区域
        let detail_area = egui::Rect::from_min_size(
            self.dialog.position + egui::vec2(18.0, 40.0),
            egui::vec2(180.0, 80.0)
        );
        
        // 绘制当前商品信息提示
        ui.painter().text(
            detail_area.min + egui::vec2(10.0, 10.0),
            egui::Align2::LEFT_TOP,
            format!("商品 {} / {}", self.current_item_index + 1, self.total_items),
            egui::FontId::proportional(14.0),
            egui::Color32::WHITE,
        );
        
        // 绘制操作提示
        ui.painter().text(
            detail_area.min + egui::vec2(10.0, 30.0),
            egui::Align2::LEFT_TOP,
            "← → 切换商品",
            egui::FontId::proportional(12.0),
            egui::Color32::from_rgb(200, 200, 200),
        );
        
        ui.painter().text(
            detail_area.min + egui::vec2(10.0, 45.0),
            egui::Align2::LEFT_TOP,
            "ESC 关闭",
            egui::FontId::proportional(12.0),
            egui::Color32::from_rgb(200, 200, 200),
        );
    }
}

/// 扩展GameShopViewer以支持具体的商品数据显示
pub struct ShopItemViewer {
    /// 基础查看器
    pub viewer: GameShopViewer,
    /// 当前显示的商品信息
    pub current_item_name: String,
    pub current_item_description: String,
    pub current_item_price: u32,
    pub current_item_icon_index: Option<usize>,
}

impl ShopItemViewer {
    pub fn new() -> Self {
        Self {
            viewer: GameShopViewer::new(),
            current_item_name: String::new(),
            current_item_description: String::new(),
            current_item_price: 0,
            current_item_icon_index: None,
        }
    }
    
    /// 显示特定商品
    pub fn show_shop_item(&mut self, 
        item_index: i32, 
        total_items: i32,
        name: &str,
        description: &str,
        price: u32,
        icon_index: Option<usize>
    ) {
        self.current_item_name = name.to_string();
        self.current_item_description = description.to_string();
        self.current_item_price = price;
        self.current_item_icon_index = icon_index;
        self.viewer.show_item(item_index, total_items);
    }
    
    /// 更新商品信息
    pub fn update_item_info(&mut self, 
        name: &str,
        description: &str,
        price: u32,
        icon_index: Option<usize>
    ) {
        self.current_item_name = name.to_string();
        self.current_item_description = description.to_string();
        self.current_item_price = price;
        self.current_item_icon_index = icon_index;
    }
    
    /// 绘制带商品信息的预览
    pub fn draw(&mut self, ctx: &egui::Context) -> bool {
        if !self.viewer.dialog.visible {
            return false;
        }
        
        let should_close = self.viewer.draw(ctx);
        
        // 绘制商品详细信息
        egui::Area::new(egui::Id::new("shop_item_details"))
            .fixed_pos(self.viewer.dialog.position + egui::vec2(18.0, 40.0))
            .movable(false)
            .order(egui::Order::Tooltip)
            .show(ctx, |ui| {
                let detail_rect = egui::Rect::from_min_size(
                    egui::pos2(0.0, 0.0),
                    egui::vec2(180.0, 80.0)
                );
                
                // 绘制商品图标
                if let Some(icon_idx) = self.current_item_icon_index {
                    if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, icon_idx) {
                        if let Some(texture) = info.egui_texture {
                            let icon_rect = egui::Rect::from_min_size(
                                egui::pos2(10.0, 10.0),
                                egui::vec2(32.0, 32.0)
                            );
                            ui.painter().image(
                                texture.id(),
                                icon_rect,
                                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                egui::Color32::WHITE,
                            );
                        }
                    }
                }
                
                // 绘制商品名称
                ui.painter().text(
                    egui::pos2(50.0, 15.0),
                    egui::Align2::LEFT_TOP,
                    &self.current_item_name,
                    egui::FontId::proportional(14.0),
                    egui::Color32::WHITE,
                );
                
                // 绘制价格
                ui.painter().text(
                    egui::pos2(50.0, 30.0),
                    egui::Align2::LEFT_TOP,
                    format!("价格: {} 金币", self.current_item_price),
                    egui::FontId::proportional(12.0),
                    egui::Color32::from_rgb(255, 215, 0), // 金色
                );
                
                // 绘制描述
                if !self.current_item_description.is_empty() {
                    ui.painter().text(
                        egui::pos2(10.0, 50.0),
                        egui::Align2::LEFT_TOP,
                        &self.current_item_description,
                        egui::FontId::proportional(11.0),
                        egui::Color32::from_rgb(200, 200, 200),
                    );
                }
            });
        
        should_close
    }
}
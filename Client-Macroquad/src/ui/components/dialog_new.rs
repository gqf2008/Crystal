// ============================================================================
// Dialog - 基于egui的对话框基类
// ============================================================================
// 
// 【功能说明】
// 1. 可拖拽的模态或非模态对话框
// 2. 自动纹理背景绘制
// 3. 标准的关闭按钮（使用egui::Button）
// 4. 层级管理和焦点处理
// 
// ============================================================================

use egui_macroquad::egui;
use crate::resources::LibraryName;
use super::{Control, ImageControl};

/// 对话框类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DialogType {
    /// 非模态对话框 - 不阻挡底层交互
    Normal,
    /// 模态对话框 - 阻挡底层交互，点击外部关闭
    Modal,
    /// 固定对话框 - 阻挡底层交互，只能通过按钮关闭
    Fixed,
}

/// 对话框基类
pub struct Dialog {
    /// 对话框ID
    pub id: String,
    /// 对话框标题
    pub title: String,
    /// 位置
    pub position: egui::Pos2,
    /// 尺寸
    pub size: egui::Vec2,
    /// 是否可见
    pub visible: bool,
    /// 对话框类型
    pub dialog_type: DialogType,
    /// 是否可拖拽
    pub movable: bool,
    /// 是否正在拖拽
    pub dragging: bool,
    /// 拖拽偏移
    pub drag_offset: egui::Vec2,
    /// 背景纹理库
    pub library: LibraryName,
    /// 背景纹理索引
    pub index: usize,
    /// 是否显示关闭按钮
    pub show_close_button: bool,
    /// 标题纹理索引（可选）
    pub title_index: Option<usize>,
    /// UI层级
    pub order: egui::Order,
}

impl Dialog {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            position: egui::pos2(100.0, 100.0),
            size: egui::vec2(400.0, 300.0),
            visible: false,
            dialog_type: DialogType::Normal,
            movable: true,
            dragging: false,
            drag_offset: egui::Vec2::ZERO,
            library: LibraryName::Title,
            index: 0,
            show_close_button: true,
            title_index: None,
            order: egui::Order::Middle,
        }
    }
    
    /// 设置对话框类型
    pub fn with_type(mut self, dialog_type: DialogType) -> Self {
        self.dialog_type = dialog_type;
        self.order = match dialog_type {
            DialogType::Normal => egui::Order::Middle,
            DialogType::Modal | DialogType::Fixed => egui::Order::Foreground,
        };
        self
    }
    
    /// 设置背景纹理
    pub fn with_background(mut self, library: LibraryName, index: usize) -> Self {
        self.library = library;
        self.index = index;
        self
    }
    
    /// 设置位置和尺寸
    pub fn with_rect(mut self, pos: egui::Pos2, size: egui::Vec2) -> Self {
        self.position = pos;
        self.size = size;
        self
    }
    
    /// 设置是否可拖拽
    pub fn with_movable(mut self, movable: bool) -> Self {
        self.movable = movable;
        self
    }
    
    /// 设置是否显示关闭按钮
    pub fn with_close_button(mut self, show: bool) -> Self {
        self.show_close_button = show;
        self
    }
    
    /// 设置标题纹理
    pub fn with_title_texture(mut self, index: usize) -> Self {
        self.title_index = Some(index);
        self
    }
    
    /// 显示对话框
    pub fn show(&mut self) {
        self.visible = true;
    }
    
    /// 隐藏对话框
    pub fn hide(&mut self) {
        self.visible = false;
    }
    
    /// 绘制对话框
    pub fn draw(&mut self, ctx: &egui::Context) -> bool {
        if !self.visible {
            return false;
        }
        
        let mut should_close = false;
        
        // 模态对话框的半透明背景
        if matches!(self.dialog_type, DialogType::Modal | DialogType::Fixed) {
            egui::Area::new("modal_backdrop")
                .order(egui::Order::Background)
                .show(ctx, |ui| {
                    let screen_rect = ui.max_rect();
                    ui.painter().rect_filled(
                        screen_rect,
                        0.0,
                        egui::Color32::from_black_alpha(128)
                    );
                    
                    // 点击背景关闭（仅模态对话框）
                    if self.dialog_type == DialogType::Modal {
                        let response = ui.allocate_rect(screen_rect, egui::Sense::click());
                        if response.clicked() {
                            should_close = true;
                        }
                    }
                });
        }
        
        // 主对话框窗口
        let dialog_rect = egui::Rect::from_min_size(self.position, self.size);
        
        egui::Area::new(&self.id)
            .order(self.order)
            .fixed_pos(self.position)
            .show(ctx, |ui| {
                // 绘制背景纹理
                self.draw_texture(ui, ctx, dialog_rect);
                
                // 绘制内容区域
                let content_rect = egui::Rect::from_min_size(
                    self.position + egui::vec2(10.0, 30.0),
                    self.size - egui::vec2(20.0, 40.0)
                );
                
                ui.allocate_ui_at_rect(content_rect, |ui| {
                    // 标题
                    if let Some(title_index) = self.title_index {
                        // 绘制标题纹理
                        let title_rect = egui::Rect::from_min_size(
                            egui::pos2(0.0, 0.0),
                            egui::vec2(self.size.x - 20.0, 25.0)
                        );
                        if let Some(info) = self.library.get_egui_texture(ctx, title_index) {
                            if let Some(texture) = info.egui_texture {
                                ui.painter().image(
                                    texture.id(),
                                    title_rect,
                                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                    egui::Color32::WHITE,
                                );
                            }
                        }
                    } else {
                        // 文本标题
                        ui.heading(&self.title);
                    }
                    
                    ui.separator();
                    
                    // 子类可以在这里绘制内容
                    self.draw_content(ui, ctx);
                });
                
                // 关闭按钮
                if self.show_close_button {
                    let close_button_pos = self.position + egui::vec2(self.size.x - 25.0, 5.0);
                    ui.allocate_ui_at_rect(
                        egui::Rect::from_min_size(close_button_pos, egui::vec2(20.0, 20.0)),
                        |ui| {
                            if ui.button("✕").clicked() {
                                should_close = true;
                            }
                        }
                    );
                }
                
                // 处理拖拽
                if self.movable {
                    let title_rect = egui::Rect::from_min_size(
                        self.position,
                        egui::vec2(self.size.x, 30.0)
                    );
                    let response = ui.allocate_rect(title_rect, egui::Sense::drag());
                    
                    if response.drag_started() {
                        self.dragging = true;
                        self.drag_offset = response.interact_pointer_pos().unwrap_or_default() - self.position;
                    }
                    
                    if self.dragging {
                        if let Some(pointer_pos) = response.interact_pointer_pos() {
                            self.position = pointer_pos - self.drag_offset;
                        }
                        
                        if response.drag_stopped() {
                            self.dragging = false;
                        }
                    }
                }
            });
        
        if should_close {
            self.hide();
        }
        
        should_close
    }
    
    /// 子类重写此方法来绘制具体内容
    pub fn draw_content(&mut self, _ui: &mut egui::Ui, _ctx: &egui::Context) {
        // 默认实现为空
    }
}

impl Control for Dialog {
    fn id(&self) -> egui::Id {
        egui::Id::new(&self.id)
    }
    
    fn position(&self) -> egui::Pos2 {
        self.position
    }
    
    fn set_position(&mut self, pos: egui::Pos2) {
        self.position = pos;
    }
    
    fn size(&self) -> egui::Vec2 {
        self.size
    }
    
    fn visible(&self) -> bool {
        self.visible
    }
    
    fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }
    
    fn draw(&mut self, _ui: &mut egui::Ui, ctx: &egui::Context) {
        self.draw(ctx);
    }
}

impl ImageControl for Dialog {
    fn library(&self) -> LibraryName {
        self.library
    }
    
    fn set_library(&mut self, library: LibraryName) {
        self.library = library;
    }
    
    fn index(&self) -> usize {
        self.index
    }
    
    fn set_index(&mut self, index: usize) {
        self.index = index;
    }
}
// ============================================================================
// TexturedDialog - 基于egui的纹理对话框组件
// ============================================================================

use egui_macroquad::egui;
use crate::resources::LibraryName;
use super::{Control, ImageControl, TexturedButton};

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

/// 纹理对话框组件
pub struct TexturedDialog {
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
    /// 关闭按钮
    pub close_button: Option<TexturedButton>,
    /// 关闭按钮相对位置
    pub close_button_offset: egui::Vec2,
    /// 标题纹理索引（可选）
    pub title_index: Option<usize>,
    /// UI层级
    pub order: egui::Order,
}

impl TexturedDialog {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        let dialog_id = id.into();
        
        Self {
            id: dialog_id.clone(),
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
            close_button: Some(
                TexturedButton::new()
                    .with_library(LibraryName::Prguse2)
                    .with_states(360, Some(361), Some(362), None)
                    .with_size(egui::vec2(20.0, 20.0))
                    .with_tooltip("关闭")
            ),
            close_button_offset: egui::vec2(375.0, 5.0), // 默认右上角
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
        // 更新关闭按钮默认位置（右上角）
        self.close_button_offset = egui::vec2(size.x - 25.0, 5.0);
        self
    }
    
    /// 设置关闭按钮
    pub fn with_close_button(mut self, button: Option<TexturedButton>) -> Self {
        self.close_button = button;
        self
    }
    
    /// 设置标题纹理
    pub fn with_title_texture(mut self, index: usize) -> Self {
        self.title_index = Some(index);
        self
    }
    
    /// 设置关闭按钮偏移
    pub fn with_close_button_offset(mut self, offset: egui::Vec2) -> Self {
        self.close_button_offset = offset;
        self
    }
    
    /// 显示对话框 (设置为可见)
    pub fn show(&mut self) {
        self.visible = true;
    }
    
    /// 隐藏对话框
    pub fn hide(&mut self) {
        self.visible = false;
        self.dragging = false;
    }
    
    /// 绘制对话框基础结构，返回是否应该关闭
    pub fn draw_base(&mut self, ctx: &egui::Context) -> bool {
        if !self.visible {
            return false;
        }
        
        let mut should_close = false;
        
        // 1. 绘制模态遮罩（如果是模态对话框）
        if self.dialog_type == DialogType::Modal {
            self.draw_modal_overlay(ctx, &mut should_close);
        }
        
        // 2. 绘制对话框主体
        let dialog_area = if self.dialog_type == DialogType::Normal {
            egui::Area::new(egui::Id::new(&self.id))
                .fixed_pos(self.position)
                .movable(false)
                .order(self.order)
        } else {
            egui::Area::new(egui::Id::new(&self.id))
                .fixed_pos(self.position)
                .movable(false)
                .order(egui::Order::Tooltip) // 模态对话框使用最高层级
        };
        
        dialog_area.show(ctx, |ui| {
            let dialog_rect = egui::Rect::from_min_size(self.position, self.size);
            
            // 处理拖拽
            if self.movable {
                should_close = self.handle_dragging(ui, ctx, dialog_rect) || should_close;
            }
            
            // 绘制背景
            self.draw_background(ui, ctx, dialog_rect);
            
            // 绘制标题
            if let Some(title_idx) = self.title_index {
                self.draw_title(ui, ctx, title_idx);
            }
            
            // 绘制关闭按钮
            if let Some(ref mut close_btn) = self.close_button {
                // 在绝对位置绘制按钮
                let btn_pos = self.position + self.close_button_offset;
                let btn_rect = egui::Rect::from_min_size(btn_pos, egui::vec2(20.0, 20.0)); // 假设按钮大小
                
                // 使用ui.new_child放置按钮
                let mut child_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(btn_rect)
                        .layout(*ui.layout())
                );
                if close_btn.draw(&mut child_ui) {
                    should_close = true;
                }
            }
        });
        
        should_close
    }
    
    /// 显示对话框并绘制内容
    pub fn show_content<R>(
        &mut self,
        ctx: &egui::Context,
        add_contents: impl FnOnce(&mut egui::Ui) -> R,
    ) -> Option<R> {
        if !self.visible {
            return None;
        }
        
        let mut should_close = false;
        
        // 1. 绘制模态遮罩
        if self.dialog_type == DialogType::Modal {
            self.draw_modal_overlay(ctx, &mut should_close);
        }
        
        // 2. 绘制对话框主体
        let dialog_area = if self.dialog_type == DialogType::Normal {
            egui::Area::new(egui::Id::new(&self.id))
                .fixed_pos(self.position)
                .movable(false)
                .order(self.order)
        } else {
            egui::Area::new(egui::Id::new(&self.id))
                .fixed_pos(self.position)
                .movable(false)
                .order(egui::Order::Tooltip)
        };
        
        let inner_response = dialog_area.show(ctx, |ui| {
            // 使用相对坐标 (0,0) 开始
            let relative_rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), self.size);
            
            // 处理拖拽 - 在最前面处理，确保不被内容遮挡
            if self.movable {
                // 整个背景区域都可以拖拽
                let drag_rect = relative_rect;
                let response = ui.interact(
                    drag_rect, 
                    egui::Id::new(format!("{}_drag", self.id)), 
                    egui::Sense::drag()
                );
                
                if response.dragged() {
                    self.position += response.drag_delta();
                }
            }
            
            // 绘制背景
            self.draw_background(ui, ctx, relative_rect);
            
            // 绘制标题
            if let Some(title_idx) = self.title_index {
                let title_pos = egui::pos2(18.0, 9.0);
                self.draw_title_at(ui, ctx, title_idx, title_pos);
            }
            
            // 绘制内容
            let result = add_contents(ui);

            // 绘制关闭按钮
            if let Some(ref mut close_btn) = self.close_button {
                let btn_pos = egui::pos2(self.close_button_offset.x, self.close_button_offset.y); // 相对位置
                let btn_rect = egui::Rect::from_min_size(btn_pos, egui::vec2(20.0, 20.0));
                
                let mut child_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(btn_rect)
                        .layout(*ui.layout())
                );
                if close_btn.draw(&mut child_ui) {
                    should_close = true;
                }
            }
            
            result
        });
        
        if should_close {
            self.hide();
        }
        
        Some(inner_response.inner)
    }

    /// 绘制模态遮罩
    fn draw_modal_overlay(&self, ctx: &egui::Context, should_close: &mut bool) {
        egui::Area::new(egui::Id::new(format!("{}_modal_overlay", self.id)))
            .fixed_pos(egui::pos2(0.0, 0.0))
            .movable(false)
            .interactable(true)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                let screen_size = ctx.screen_rect().size();
                let overlay_rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), screen_size);
                
                // 半透明遮罩
                ui.painter().rect_filled(
                    overlay_rect,
                    0.0,
                    egui::Color32::from_black_alpha(64),
                );
                
                // 点击外部关闭（仅限模态对话框）
                if self.dialog_type == DialogType::Modal {
                    if ui.allocate_rect(overlay_rect, egui::Sense::click()).clicked() {
                        *should_close = true;
                    }
                }
            });
    }
    
    /// 处理拖拽
    fn handle_dragging(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, dialog_rect: egui::Rect) -> bool {
        // 标题栏拖拽区域（顶部30像素）
        let title_area = egui::Rect::from_min_size(
            dialog_rect.min,
            egui::vec2(self.size.x, 30.0)
        );
        
        let title_response = ui.interact(
            title_area,
            egui::Id::new(format!("{}_title_drag", self.id)),
            egui::Sense::drag()
        );
        
        if title_response.drag_started() && !self.dragging {
            self.dragging = true;
            if let Some(pointer_pos) = ctx.input(|i| i.pointer.hover_pos()) {
                self.drag_offset = self.position.to_vec2() - pointer_pos.to_vec2();
            }
        } else if title_response.dragged() && self.dragging {
            self.position += title_response.drag_delta();
        } else if self.dragging && (!ctx.input(|i| i.pointer.primary_down()) || title_response.drag_stopped()) {
            self.dragging = false;
        }
        
        // 边界检查
        let screen_rect = ctx.screen_rect();
        if self.position.x < screen_rect.min.x - self.size.x + 50.0 {
            self.position.x = screen_rect.min.x - self.size.x + 50.0;
        }
        if self.position.x > screen_rect.max.x - 50.0 {
            self.position.x = screen_rect.max.x - 50.0;
        }
        if self.position.y < screen_rect.min.y {
            self.position.y = screen_rect.min.y;
        }
        if self.position.y > screen_rect.max.y - 50.0 {
            self.position.y = screen_rect.max.y - 50.0;
        }
        
        false // 拖拽不会关闭对话框
    }
    
    /// 绘制背景
    fn draw_background(&self, ui: &mut egui::Ui, ctx: &egui::Context, rect: egui::Rect) {
        if let Some(info) = self.library.get_egui_texture(ctx, self.index) {
            if let Some(texture) = info.egui_texture {
                ui.painter().image(
                    texture.id(),
                    rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
        } else {
            // 备用背景
            ui.painter().rect_filled(
                rect,
                5.0,
                egui::Color32::from_rgba_premultiplied(40, 40, 50, 240),
            );
            ui.painter().rect_stroke(
                rect,
                5.0,
                egui::Stroke::new(2.0, egui::Color32::from_rgb(200, 200, 200)),
                egui::epaint::StrokeKind::Outside,
            );
        }
    }
    
    /// 绘制标题纹理
    fn draw_title(&self, ui: &mut egui::Ui, ctx: &egui::Context, title_index: usize) {
        let title_pos = self.position + egui::vec2(18.0, 9.0);
        if let Some(info) = self.library.get_egui_texture(ctx, title_index) {
            if let Some(texture) = info.egui_texture {
                let title_rect = egui::Rect::from_min_size(title_pos, egui::vec2(200.0, 30.0));
                ui.painter().image(
                    texture.id(),
                    title_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
        }
    }
    
    /// 在指定相对位置绘制标题
    fn draw_title_at(&self, ui: &mut egui::Ui, ctx: &egui::Context, title_index: usize, pos: egui::Pos2) {
        if let Some(info) = self.library.get_egui_texture(ctx, title_index) {
            if let Some(texture) = info.egui_texture {
                let title_rect = egui::Rect::from_min_size(pos, egui::vec2(200.0, 30.0));
                ui.painter().image(
                    texture.id(),
                    title_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
        }
    }
}

impl Control for TexturedDialog {
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
        if self.draw_base(ctx) {
            self.hide();
        }
    }
}

impl ImageControl for TexturedDialog {
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
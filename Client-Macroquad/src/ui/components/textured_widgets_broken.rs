// ============================================================================
// TexturedButton - 基于egui::ImageButton的纹理按钮组件
// ============================================================================

use egui_macroquad::egui;
use crate::resources::LibraryName;

/// 纹理按钮状态
#[derive(Debug, Clone, Copy)]
pub enum ButtonState {
    Normal,
    Hovered,
    Pressed,
    Disabled,
}

/// 基于egui::ImageButton的纹理按钮
#[derive(Debug, Clone)]
pub struct TexturedButton {
    /// 各状态的纹理索引
    normal_index: usize,
    hover_index: Option<usize>,
    pressed_index: Option<usize>, 
    disabled_index: Option<usize>,
    
    /// 纹理库
    library: LibraryName,
    
    /// 按钮文本（可选）
    text: String,
    
    /// 是否启用
    enabled: bool,
    
    /// 提示文本
    tooltip: String,
    
    /// 按钮尺寸（None表示使用纹理原始尺寸）
    size: Option<egui::Vec2>,
}

impl TexturedButton {
    pub fn new() -> Self {
        Self {
            normal_index: 0,
            hover_index: None,
            pressed_index: None,
            disabled_index: None,
            library: LibraryName::Prguse,
            text: String::new(),
            enabled: true,
            tooltip: String::new(),
            size: None,
        }
    }
    
    /// 设置纹理
    pub fn with_texture(mut self, library: LibraryName, normal_index: usize) -> Self {
        self.library = library;
        self.normal_index = normal_index;
        self
    }
    
    /// 设置多状态纹理
    pub fn with_states(mut self, normal: usize, hover: Option<usize>, pressed: Option<usize>, disabled: Option<usize>) -> Self {
        self.normal_index = normal;
        self.hover_index = hover;
        self.pressed_index = pressed;
        self.disabled_index = disabled;
        self
    }
    
    /// 设置纹理库
    pub fn with_library(mut self, library: LibraryName) -> Self {
        self.library = library;
        self
    }
    
    /// 设置按钮文本
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text = text.into();
        self
    }
    
    /// 设置是否启用
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
    
    /// 设置提示文本
    pub fn with_tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = tooltip.into();
        self
    }
    
    /// 设置按钮尺寸
    pub fn with_size(mut self, size: egui::Vec2) -> Self {
        self.size = Some(size);
        self
    }
    
    /// 获取当前状态应该使用的纹理索引
    fn get_texture_index(&self, state: ButtonState) -> usize {
        if !self.enabled {
            return self.disabled_index.unwrap_or(self.normal_index);
        }
        
        match state {
            ButtonState::Normal => self.normal_index,
            ButtonState::Hovered => self.hover_index.unwrap_or(self.normal_index),
            ButtonState::Pressed => self.pressed_index.unwrap_or(self.normal_index),
            ButtonState::Disabled => self.disabled_index.unwrap_or(self.normal_index),
        }
    }
    
    /// 绘制按钮并返回是否被点击
    pub fn draw(&mut self, ui: &mut egui::Ui) -> bool {
        ui.scope(|ui| {
            ui.set_enabled(self.enabled);
            
            // 获取当前状态
            let state = if !self.enabled {
                ButtonState::Disabled
            } else {
                ButtonState::Normal // egui会自动处理hover/pressed状态
            };
            
            let texture_index = self.get_texture_index(state);
            
            // 尝试获取纹理
            if let Some(info) = self.library.get_egui_texture(ui.ctx(), texture_index) {
                if let Some(texture_id) = info.egui_texture.map(|t| t.id()) {
                    // 确定按钮尺寸
                    let button_size = self.size.unwrap_or_else(|| {
                        egui::vec2(info.width as f32, info.height as f32)
                    });
                    
                    // 创建ImageButton
                    let image_button = egui::ImageButton::new((texture_id, button_size));
                    
                    let response = ui.add(image_button);
                    
                    // 添加文本标签（如果有）
                    if !self.text.is_empty() {
                        let text_pos = response.rect.center();
                        
                        ui.painter().text(
                            text_pos,
                            egui::Align2::CENTER_CENTER,
                            &self.text,
                            egui::TextStyle::Button.resolve(ui.style()),
                            ui.visuals().text_color()
                        );
                    }
                    
                    // 添加提示文本
                    if !self.tooltip.is_empty() {
                        response.on_hover_text(&self.tooltip);
                    }
                    
                    return response.clicked();
                }
            }
            
            // 纹理加载失败，使用fallback按钮
            let fallback_text = if self.text.is_empty() {
                format!("Btn[{:?}:{}]", self.library, texture_index)
            } else {
                self.text.clone()
            };
            
            let response = ui.button(fallback_text);
            
            if !self.tooltip.is_empty() {
                response.on_hover_text(&self.tooltip);
            }
            
            response.clicked()
        }).inner
    }
}

impl Default for TexturedButton {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TexturedCheckBox - 基于egui的纹理复选框组件  
// ============================================================================

/// 基于egui的纹理复选框
#[derive(Debug, Clone)]
pub struct TexturedCheckBox {
    /// 选中状态纹理索引
    checked_index: usize,
    /// 未选中状态纹理索引
    unchecked_index: usize,
    
    /// 纹理库
    library: LibraryName,
    
    /// 复选框状态
    checked: bool,
    
    /// 标签文本
    text: String,
    
    /// 是否启用
    enabled: bool,
    
    /// 提示文本
    tooltip: String,
    
    /// 复选框尺寸
    size: Option<egui::Vec2>,
}

impl TexturedCheckBox {
    pub fn new() -> Self {
        Self {
            checked_index: 0,
            unchecked_index: 1,
            library: LibraryName::Prguse,
            checked: false,
            text: String::new(),
            enabled: true,
            tooltip: String::new(),
            size: None,
        }
    }
    
    /// 设置纹理索引
    pub fn with_textures(mut self, library: LibraryName, checked: usize, unchecked: usize) -> Self {
        self.library = library;
        self.checked_index = checked;
        self.unchecked_index = unchecked;
        self
    }
    
    /// 设置标签文本
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text = text.into();
        self
    }
    
    /// 设置初始状态
    pub fn with_checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }
    
    /// 设置是否启用
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
    
    /// 设置提示文本
    pub fn with_tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = tooltip.into();
        self
    }
    
    /// 设置复选框尺寸
    pub fn with_size(mut self, size: egui::Vec2) -> Self {
        self.size = Some(size);
        self
    }
    
    /// 获取状态
    pub fn checked(&self) -> bool {
        self.checked
    }
    
    /// 设置状态
    pub fn set_checked(&mut self, checked: bool) {
        self.checked = checked;
    }
    
    /// 切换状态
    pub fn toggle(&mut self) {
        self.checked = !self.checked;
    }
    
    /// 绘制复选框并返回是否被点击
    pub fn draw(&mut self, ui: &mut egui::Ui) -> bool {
        ui.scope(|ui| {
            ui.set_enabled(self.enabled);
            
            let texture_index = if self.checked { self.checked_index } else { self.unchecked_index };
            
            // 水平布局：纹理按钮 + 文本标签
            ui.horizontal(|ui| {
                let mut clicked = false;
                
                // 绘制纹理按钮
                if let Some(info) = self.library.get_egui_texture(ui.ctx(), texture_index) {
                    if let Some(texture_id) = info.egui_texture.map(|t| t.id()) {
                        let checkbox_size = self.size.unwrap_or_else(|| {
                            egui::vec2(info.width as f32, info.height as f32)
                        });
                        
                        let image_button = egui::ImageButton::new(
                            (texture_id, checkbox_size)
                        );
                        
                        let response = ui.add(image_button);
                        
                        if !self.tooltip.is_empty() {
                            response.on_hover_text(&self.tooltip);
                        }
                        
                        clicked = response.clicked();
                    }
                } else {
                    // 纹理加载失败，使用egui原生复选框
                    let response = ui.checkbox(&mut self.checked, "");
                    clicked = response.changed();
                }
                
                // 文本标签（可点击）
                if !self.text.is_empty() {
                    let label_response = ui.selectable_label(false, &self.text);
                    if label_response.clicked() {
                        clicked = true;
                    }
                }
                
                // 处理点击
                if clicked {
                    self.toggle();
                }
                
                clicked
            }).inner
        }).inner
    }
}

impl Default for TexturedCheckBox {
    fn default() -> Self {
        Self::new()
    }
}
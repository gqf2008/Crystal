// ============================================================================
// TexturedButton - 基于egui::ImageButton的纹理按钮组件
// ============================================================================

use egui_macroquad::egui;
use crate::resources::LibraryName;

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
    pub tooltip: String,
    
    /// 按钮尺寸（None表示使用纹理原始尺寸）
    pub size: Option<egui::Vec2>,
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
    
    /// 绘制按钮并返回是否被点击
    pub fn draw(&mut self, ui: &mut egui::Ui) -> bool {
        ui.add_enabled_ui(self.enabled, |ui| {
            let texture_index = if !self.enabled {
                self.disabled_index.unwrap_or(self.normal_index)
            } else {
                self.normal_index // egui会自动处理hover/pressed状态
            };
            
            // 尝试获取纹理
            if let Some(info) = self.library.get_egui_texture(ui.ctx(), texture_index) {
                if let Some(texture_id) = info.egui_texture.map(|t| t.id()) {
                    // 确定按钮尺寸
                    let button_size = self.size.unwrap_or_else(|| {
                        egui::vec2(info.width as f32, info.height as f32)
                    });
                    
                    // 创建ImageButton - 使用新的API格式
                    let image_button = egui::ImageButton::new((texture_id, button_size)).frame(false);
                    
                    let response = ui.add(image_button);
                    
                    // 添加文本标签（如果有）
                    if !self.text.is_empty() {
                        ui.painter().text(
                            response.rect.center(),
                            egui::Align2::CENTER_CENTER,
                            &self.text,
                            egui::TextStyle::Button.resolve(ui.style()),
                            ui.visuals().text_color()
                        );
                    }
                    
                    let clicked = response.clicked();
                    
                    // 添加提示文本
                    if !self.tooltip.is_empty() {
                        response.on_hover_text(&self.tooltip);
                    }
                    
                    return clicked;
                }
            }
            
            // 纹理加载失败，使用fallback按钮
            let fallback_text = if self.text.is_empty() {
                format!("Btn[{:?}:{}]", self.library, texture_index)
            } else {
                self.text.clone()
            };
            
            let response = ui.button(fallback_text);
            
            let clicked = response.clicked();
            
            if !self.tooltip.is_empty() {
                response.on_hover_text(&self.tooltip);
            }
            
            clicked
        }).inner
    }
}

impl Default for TexturedButton {
    fn default() -> Self {
        Self::new()
    }
}
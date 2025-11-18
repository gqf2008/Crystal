// ============================================================================
// CheckBox - 基于egui的复选框组件
// ============================================================================

use egui_macroquad::egui;
use crate::resources::LibraryName;

/// 简单的复选框组件，基于egui::Checkbox
#[derive(Debug, Clone)]
pub struct CheckBox {
    checked: bool,
    label_text: String,
    enabled: bool,
}

impl CheckBox {
    pub fn new() -> Self {
        Self {
            checked: false,
            label_text: String::new(),
            enabled: true,
        }
    }

    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.label_text = text.into();
        self
    }

    pub fn with_checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn checked(&self) -> bool {
        self.checked
    }

    pub fn set_checked(&mut self, checked: bool) {
        self.checked = checked;
    }

    pub fn toggle(&mut self) {
        self.checked = !self.checked;
    }

    /// 绘制复选框，返回是否发生了状态变化
    pub fn draw(&mut self, ui: &mut egui::Ui) -> bool {
        ui.scope(|ui| {
            ui.set_enabled(self.enabled);
            let response = ui.checkbox(&mut self.checked, &self.label_text);
            response.changed()
        }).inner
    }
}

impl Default for CheckBox {
    fn default() -> Self {
        Self::new()
    }
}

/// 纹理复选框组件，基于egui::ImageButton
#[derive(Debug, Clone)]
pub struct TexturedCheckBox {
    checked: bool,
    checked_index: usize,
    unchecked_index: usize,
    library: LibraryName,
    label_text: String,
    enabled: bool,
    tooltip: String,
}

impl TexturedCheckBox {
    pub fn new() -> Self {
        Self {
            checked: false,
            checked_index: 0,
            unchecked_index: 1,
            library: LibraryName::Prguse,
            label_text: String::new(),
            enabled: true,
            tooltip: String::new(),
        }
    }

    pub fn with_textures(mut self, library: LibraryName, checked: usize, unchecked: usize) -> Self {
        self.library = library;
        self.checked_index = checked;
        self.unchecked_index = unchecked;
        self
    }

    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.label_text = text.into();
        self
    }

    pub fn with_checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn with_tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = tooltip.into();
        self
    }

    pub fn checked(&self) -> bool {
        self.checked
    }

    pub fn set_checked(&mut self, checked: bool) {
        self.checked = checked;
    }

    pub fn toggle(&mut self) {
        self.checked = !self.checked;
    }

    /// 绘制纹理复选框，基于egui::ImageButton
    pub fn draw(&mut self, ui: &mut egui::Ui) -> bool {
        ui.scope(|ui| {
            ui.set_enabled(self.enabled);
            
            let texture_index = if self.checked { 
                self.checked_index 
            } else { 
                self.unchecked_index 
            };
            
            // 水平布局：纹理按钮 + 文本标签
            ui.horizontal(|ui| {
                let mut clicked = false;
                
                // 尝试绘制纹理按钮
                if let Some(info) = self.library.get_egui_texture(ui.ctx(), texture_index) {
                    if let Some(texture_id) = info.egui_texture.map(|t| t.id()) {
                        let checkbox_size = egui::vec2(info.width as f32, info.height as f32);
                        
                        let image_button = egui::ImageButton::new(
                            egui::Image::new(texture_id).fit_to_exact_size(checkbox_size)
                        );
                        
                        let response = ui.add(image_button);
                        
                        if !self.tooltip.is_empty() {
                            response.on_hover_text(&self.tooltip);
                        }
                        
                        clicked = response.clicked();
                    }
                } else {
                    // 纹理加载失败，使用egui原生复选框作为fallback
                    let response = ui.checkbox(&mut self.checked, "");
                    clicked = response.changed();
                }
                
                // 文本标签（可点击）
                if !self.label_text.is_empty() {
                    let label_response = ui.selectable_label(false, &self.label_text);
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
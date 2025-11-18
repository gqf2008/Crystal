// ============================================================================
// CheckBox Component Test - 复选框组件测试
// ============================================================================

use crate::ui::components::CheckBox;
use egui_macroquad::egui;

pub struct CheckBoxTest {
    checkbox1: CheckBox,
    checkbox2: CheckBox,
    checkbox3: CheckBox,
    
    // 状态显示
    status_text: String,
}

impl CheckBoxTest {
    pub fn new() -> Self {
        let checkbox1 = CheckBox::new()
            .with_text("启用音效");
        
        let checkbox2 = CheckBox::new()
            .with_text("显示血量");
        
        let checkbox3 = CheckBox::new()
            .with_text("自动拾取");
        
        Self {
            checkbox1,
            checkbox2,
            checkbox3,
            status_text: "复选框状态测试".to_string(),
        }
    }
    
    pub fn draw(&mut self, ui: &mut egui::Ui) {
        ui.heading("复选框组件测试");
        
        ui.separator();
        
        // 复选框组
        ui.vertical(|ui| {
            if self.checkbox1.draw(ui) {
                println!("复选框1被点击! 状态: {}", self.checkbox1.checked());
                self.update_status();
            }
            
            if self.checkbox2.draw(ui) {
                println!("复选框2被点击! 状态: {}", self.checkbox2.checked());
                self.update_status();
            }
            
            if self.checkbox3.draw(ui) {
                println!("复选框3被点击! 状态: {}", self.checkbox3.checked());
                self.update_status();
            }
        });
        
        ui.separator();
        
        // 状态显示
        ui.colored_label(egui::Color32::LIGHT_BLUE, &self.status_text);
        
        ui.separator();
        
        // 控制按钮
        ui.horizontal(|ui| {
            if ui.button("全选").clicked() {
                self.checkbox1.set_checked(true);
                self.checkbox2.set_checked(true);
                self.checkbox3.set_checked(true);
                self.update_status();
                println!("全选");
            }
            
            if ui.button("全不选").clicked() {
                self.checkbox1.set_checked(false);
                self.checkbox2.set_checked(false);
                self.checkbox3.set_checked(false);
                self.update_status();
                println!("全不选");
            }
            
            if ui.button("反选").clicked() {
                self.checkbox1.toggle();
                self.checkbox2.toggle();
                self.checkbox3.toggle();
                self.update_status();
                println!("反选");
            }
        });
    }
    
    fn update_status(&mut self) {
        self.status_text = format!(
            "状态 - 音效: {} | 血量: {} | 拾取: {}",
            if self.checkbox1.checked() { "开" } else { "关" },
            if self.checkbox2.checked() { "开" } else { "关" },
            if self.checkbox3.checked() { "开" } else { "关" }
        );
    }
}

impl Default for CheckBoxTest {
    fn default() -> Self {
        Self::new()
    }
}
// ============================================================================
// CheckBox组件测试程序 - 验证重构后的组件
// ============================================================================

use client_macroquad::ui::components::CheckBox;
use macroquad::prelude::*;

#[macroquad::main("测试CheckBox组件")]
async fn main() {
    // 创建测试复选框
    let mut checkbox1 = CheckBox::new()
        .with_text("启用音效");
    
    let mut checkbox2 = CheckBox::new()
        .with_text("显示血量");
    
    let mut checkbox3 = CheckBox::new()
        .with_text("自动拾取");
    
    loop {
        clear_background(Color::from_rgba(30, 30, 40, 255));
        
        // 处理按键
        if is_key_pressed(KeyCode::Escape) {
            break;
        }
        
        // 绘制UI
        egui_macroquad::ui(|ctx| {
            egui_macroquad::egui::Window::new("CheckBox测试")
                .default_size([300.0, 200.0])
                .show(ctx, |ui| {
                    ui.heading("CheckBox组件测试");
                    ui.separator();
                    
                    // 垂直布局显示复选框
                    ui.vertical(|ui| {
                        if checkbox1.draw(ui) {
                            println!("复选框1被点击! 状态: {}", checkbox1.checked());
                        }
                        
                        if checkbox2.draw(ui) {
                            println!("复选框2被点击! 状态: {}", checkbox2.checked());
                        }
                        
                        if checkbox3.draw(ui) {
                            println!("复选框3被点击! 状态: {}", checkbox3.checked());
                        }
                    });
                    
                    ui.separator();
                    
                    // 显示当前状态
                    ui.label(format!("音效: {}", if checkbox1.checked() { "开启" } else { "关闭" }));
                    ui.label(format!("血量显示: {}", if checkbox2.checked() { "开启" } else { "关闭" }));
                    ui.label(format!("自动拾取: {}", if checkbox3.checked() { "开启" } else { "关闭" }));
                    
                    ui.separator();
                    ui.small("按ESC退出");
                });
        });
        
        egui_macroquad::draw();
        next_frame().await;
    }
}
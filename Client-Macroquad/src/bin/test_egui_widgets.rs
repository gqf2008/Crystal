// ============================================================================
// 测试新的基于egui的纹理组件
// ============================================================================

use egui_macroquad::egui;
use macroquad::prelude::*;

fn window_conf() -> Conf {
    Conf {
        window_title: "egui纹理组件测试".to_owned(),
        window_width: 800,
        window_height: 600,
        high_dpi: false,
        window_resizable: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    println!("🎮 egui纹理组件测试");
    println!("✨ 基于egui::ImageButton的纹理按钮和复选框");
    
    // 模拟组件状态
    let mut textured_button_clicked = false;
    let mut textured_checkbox_checked = false;
    let mut simple_checkbox_checked = false;
    let mut button_count = 0;
    
    loop {
        clear_background(Color::from_rgba(25, 25, 25, 255));
        
        // 绘制egui界面
        egui_macroquad::ui(|ctx| {
            egui::Window::new("🎨 egui纹理组件测试")
                .default_size([400.0, 500.0])
                .show(ctx, |ui| {
                    ui.heading("组件测试");
                    ui.separator();
                    
                    // 说明文本
                    ui.label("这个测试展示了基于egui的组件设计理念：");
                    ui.label("• 优先使用egui原生组件");
                    ui.label("• 仅在需要纹理时才包装ImageButton");
                    ui.label("• 所有绘制由egui处理，不依赖macroquad");
                    
                    ui.separator();
                    
                    // egui原生组件测试
                    ui.heading("egui原生组件");
                    
                    if ui.button("普通按钮").clicked() {
                        button_count += 1;
                        println!("普通按钮被点击了 {} 次", button_count);
                    }
                    
                    ui.checkbox(&mut simple_checkbox_checked, "普通复选框");
                    
                    ui.separator();
                    
                    // 纹理组件模拟（当前还没有实际纹理）
                    ui.heading("纹理组件模拟");
                    
                    // 模拟纹理按钮
                    ui.horizontal(|ui| {
                        ui.label("纹理按钮：");
                        
                        // 这里我们用ImageButton模拟，但没有实际纹理
                        let button_response = ui.add(
                            egui::Button::new("🖼️ 纹理按钮")
                                .fill(if textured_button_clicked { 
                                    egui::Color32::GREEN 
                                } else { 
                                    egui::Color32::BLUE 
                                })
                        );
                        
                        if button_response.clicked() {
                            textured_button_clicked = !textured_button_clicked;
                            println!("纹理按钮状态切换: {}", textured_button_clicked);
                        }
                    });
                    
                    // 模拟纹理复选框
                    ui.horizontal(|ui| {
                        ui.label("纹理复选框：");
                        
                        // 用颜色按钮模拟不同状态的纹理
                        let checkbox_text = if textured_checkbox_checked { "☑️" } else { "☐" };
                        let checkbox_color = if textured_checkbox_checked { 
                            egui::Color32::GREEN 
                        } else { 
                            egui::Color32::GRAY 
                        };
                        
                        let checkbox_response = ui.add(
                            egui::Button::new(checkbox_text)
                                .fill(checkbox_color)
                                .min_size(egui::vec2(30.0, 30.0))
                        );
                        
                        if checkbox_response.clicked() {
                            textured_checkbox_checked = !textured_checkbox_checked;
                            println!("纹理复选框状态: {}", textured_checkbox_checked);
                        }
                        
                        ui.label("启用音效");
                    });
                    
                    ui.separator();
                    
                    // 设计理念说明
                    ui.heading("设计理念");
                    ui.label("✅ 优先使用egui原生组件");
                    ui.label("  - ui.button(), ui.checkbox(), ui.label()");
                    ui.label("  - 快速开发，无需维护自定义组件");
                    
                    ui.label("🎨 需要纹理时使用ImageButton包装");
                    ui.label("  - TexturedButton, TexturedCheckBox");
                    ui.label("  - 保持游戏原版UI风格");
                    
                    ui.label("🚀 所有绘制交给egui处理");
                    ui.label("  - 不依赖macroquad绘制");
                    ui.label("  - 更好的性能和兼容性");
                    
                    ui.separator();
                    
                    // 状态显示
                    ui.label(format!("普通按钮点击次数: {}", button_count));
                    ui.label(format!("普通复选框: {}", simple_checkbox_checked));
                    ui.label(format!("纹理按钮状态: {}", textured_button_clicked));
                    ui.label(format!("纹理复选框状态: {}", textured_checkbox_checked));
                });
        });
        
        // 绘制egui
        egui_macroquad::draw();
        
        // ESC退出
        if is_key_pressed(KeyCode::Escape) {
            break;
        }
        
        next_frame().await;
    }
    
    println!("✅ 测试结束");
}
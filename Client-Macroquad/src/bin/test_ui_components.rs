// ============================================================================
// 测试UI组件系统 - 简化版本 [需要更新 - 使用已删除的组件]
// ============================================================================

// FIXME: 此测试文件使用了已删除的MirButton和MirLabel组件
// 需要更新为使用egui原生组件或新的包装器
// use client_macroquad::ui::components::{MirButton, MirLabel};
use macroquad::prelude::*;

#[macroquad::main("测试UI组件")]
async fn main() {
    // 创建测试按钮
    let mut test_button = MirButton::new("test_btn")
        .with_rect(egui_macroquad::egui::pos2(100.0, 100.0), egui_macroquad::egui::vec2(100.0, 30.0))
        .with_text("测试按钮");
    
    // 创建测试标签
    let mut test_label = MirLabel::new("test_label", "这是一个测试标签")
        .with_position(egui_macroquad::egui::pos2(100.0, 50.0))
        .with_font_size(16.0)
        .with_color(egui_macroquad::egui::Color32::YELLOW);
    
    let mut click_count = 0;
    
    loop {
        clear_background(Color::from_rgba(30, 30, 40, 255));
        
        // 处理按键
        if is_key_pressed(KeyCode::Escape) {
            break;
        }
        
        // 绘制说明
        draw_text("按 ESC 退出", 20.0, 30.0, 20.0, WHITE);
        
        // 绘制UI组件
        egui_macroquad::ui(|ctx| {
            egui_macroquad::egui::Area::new(egui_macroquad::egui::Id::new("test_area"))
                .fixed_pos(egui_macroquad::egui::pos2(0.0, 0.0))
                .show(ctx, |ui| {
                    // 绘制标签
                    test_label.show(ui, ctx);
                    
                    // 绘制按钮
                    let response = test_button.show(ui, ctx);
                    if response.clicked {
                        click_count += 1;
                        test_label.set_text(format!("按钮被点击了 {} 次", click_count));
                    }
                    
                    // 显示点击统计
                    ui.painter().text(
                        egui_macroquad::egui::pos2(100.0, 150.0),
                        egui_macroquad::egui::Align2::LEFT_TOP,
                        format!("点击次数: {}", click_count),
                        egui_macroquad::egui::FontId::proportional(14.0),
                        egui_macroquad::egui::Color32::WHITE,
                    );
                });
        });
        
        egui_macroquad::draw();
        next_frame().await;
    }
}
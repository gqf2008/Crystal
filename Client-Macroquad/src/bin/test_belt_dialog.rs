/// 测试 BeltDialog（快捷栏）
/// 
/// 运行命令：cargo run --bin test_belt_dialog

use client_macroquad::scenes::dialogs::{Dialog, BeltDialog, MainDialog};
use egui_macroquad::egui;

#[macroquad::main("传奇2 - 快捷栏测试")]
async fn main() {
    println!("🎮 传奇2 - 快捷栏测试");
    println!("📐 窗口尺寸: 1024x768");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("💡 提示：");
    println!("   - 点击旋转按钮切换水平/垂直布局");
    println!("   - 点击关闭按钮隐藏快捷栏");
    println!("   - 按 ESC 退出");
    
    // 创建主界面（用于定位）
    let mut main_dialog = MainDialog::new();
    
    // 创建快捷栏（位于主界面上方）
    let screen_height = macroquad::prelude::screen_height();
    let screen_width = macroquad::prelude::screen_width();
    let main_dialog_x = (screen_width / 2.0) - 400.0; // 假设主界面宽度约800
    let mut belt_dialog = BeltDialog::new(main_dialog_x, screen_height);
    let mut belt_open = true;
    
    println!("🎬 进入快捷栏测试界面");
    
    loop {
        // 检测 ESC 退出
        if macroquad::input::is_key_pressed(macroquad::input::KeyCode::Escape) {
            println!("👋 按下 ESC，退出测试");
            break;
        }
        
        // 清空背景
        macroquad::window::clear_background(macroquad::color::Color::from_rgba(20, 20, 30, 255));
        
        // 使用 egui
        egui_macroquad::ui(|ctx| {
            // 主UI面板
            egui::CentralPanel::default()
                .frame(egui::Frame::default())
                .show(ctx, |_ui| {
                    // 绘制主界面（底部工具栏）
                    main_dialog.show(ctx);
                    
                    // 绘制快捷栏
                    belt_dialog.show(ctx, &mut belt_open);
                });
            
            // 状态信息（顶部左侧）
            egui::Window::new("状态")
                .fixed_pos(egui::pos2(10.0, 10.0))
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label("快捷栏测试 - 按 ESC 退出");
                    ui.label(format!("快捷栏: {}", if belt_open { "显示" } else { "隐藏" }));
                });
        });
        
        // 绘制 egui
        egui_macroquad::draw();
        
        // 下一帧
        macroquad::window::next_frame().await;
    }
}

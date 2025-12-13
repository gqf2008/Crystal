// 测试 BeltDialogMqui - macroquad UI 版本的快捷栏
// 支持 Group::draggable() 物品拖放

use macroquad::prelude::*;
use client_macroquad::scenes::dialogs::game::belt_dialog_mqui::BeltDialogMqui;
use client_macroquad::resources::{set_data_path, preload_libraries, LibraryName};

#[macroquad::main("Belt Dialog MQUI Test")]
async fn main() {
    // 初始化资源
    set_data_path("./Data/");
    preload_libraries(&[
        LibraryName::Prguse,
        LibraryName::Items,
    ]);
    
    // 创建快捷栏
    let mut belt = BeltDialogMqui::new();
    belt.load_textures().await;
    belt.open();
    
    println!("==========================================");
    println!("  快捷栏测试 (macroquad UI 版本)");
    println!("==========================================");
    println!("快捷键:");
    println!("  B - 打开/关闭快捷栏");
    println!("  R - 旋转布局 (水平/垂直)");
    println!("  1-6 - 使用对应格子物品");
    println!("  鼠标拖动 - 交换物品位置");
    println!("  ESC - 退出");
    println!("==========================================");
    
    loop {
        clear_background(Color::from_rgba(30, 30, 40, 255));
        
        // 绘制背景网格
        for x in (0..screen_width() as i32).step_by(50) {
            draw_line(x as f32, 0.0, x as f32, screen_height(), 1.0, Color::from_rgba(50, 50, 60, 255));
        }
        for y in (0..screen_height() as i32).step_by(50) {
            draw_line(0.0, y as f32, screen_width(), y as f32, 1.0, Color::from_rgba(50, 50, 60, 255));
        }
        
        // 处理输入
        if is_key_pressed(KeyCode::Escape) {
            break;
        }
        
        if is_key_pressed(KeyCode::B) {
            belt.toggle();
        }
        
        if is_key_pressed(KeyCode::R) {
            belt.flip_layout();
        }
        
        // 数字键使用物品
        if is_key_pressed(KeyCode::Key1) { belt.use_item(0); }
        if is_key_pressed(KeyCode::Key2) { belt.use_item(1); }
        if is_key_pressed(KeyCode::Key3) { belt.use_item(2); }
        if is_key_pressed(KeyCode::Key4) { belt.use_item(3); }
        if is_key_pressed(KeyCode::Key5) { belt.use_item(4); }
        if is_key_pressed(KeyCode::Key6) { belt.use_item(5); }
        
        // 绘制快捷栏
        belt.update_and_draw();
        
        // 绘制帮助文本
        draw_text("Belt Dialog MQUI Test", 20.0, 30.0, 24.0, WHITE);
        draw_text("B=Toggle, R=Rotate, 1-6=Use, Drag=Swap", 20.0, 55.0, 16.0, GRAY);
        
        // 绘制物品状态
        let mut y = 80.0;
        for i in 0..6 {
            if let Some(item) = belt.get_item(i) {
                draw_text(&format!("Slot {}: Icon={}, Count={}", i + 1, item.icon_index, item.count), 20.0, y, 14.0, GREEN);
            } else {
                draw_text(&format!("Slot {}: Empty", i + 1), 20.0, y, 14.0, DARKGRAY);
            }
            y += 18.0;
        }
        
        next_frame().await;
    }
}

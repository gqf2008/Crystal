// ============================================================================
// 测试程序：背包混合版（Native 绘制 + mqui 拖放）
// ============================================================================

// Windows: Release 模式不弹控制台（Debug 仍保留控制台便于调试）
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use macroquad::prelude::*;
use client_macroquad::resources::{set_data_path, preload_libraries, LibraryName};
use client_macroquad::scenes::dialogs::game::InventoryDialogHybrid;

fn window_conf() -> Conf {
    Conf {
        window_title: "Inventory Dialog Hybrid Test".to_owned(),
        window_width: 1024,
        window_height: 768,
        high_dpi: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    println!("==========================================");
    println!("  背包测试 (混合版: Native + mqui)");
    println!("==========================================");
    println!("快捷键:");
    println!("  I     - 打开/关闭背包");
    println!("  1/2/3 - 切换标签页");
    println!("  ESC   - 退出程序");
    println!("");
    println!("拖放操作:");
    println!("  - 拖动物品到其他格子 → 交换");
    println!("  - 拖动物品到窗口外 → 丢弃");
    println!("  - 双击物品 → 使用");
    println!("  - 滚轮 → 滚动");
    println!("==========================================");
    
    // 初始化资源
    set_data_path("./Data/");
    preload_libraries(&[
        LibraryName::Title,
        LibraryName::Prguse,
        LibraryName::Prguse2,
        LibraryName::Items,
    ]);
    
    // 创建背包
    let mut inventory = InventoryDialogHybrid::new();
    inventory.load_textures();
    inventory.set_position(vec2(100.0, 100.0));
    inventory.open();
    
    loop {
        clear_background(Color::from_rgba(30, 30, 40, 255));
        
        // 背景网格
        for x in (0..screen_width() as i32).step_by(50) {
            draw_line(x as f32, 0.0, x as f32, screen_height(), 1.0, Color::from_rgba(50, 50, 60, 100));
        }
        for y in (0..screen_height() as i32).step_by(50) {
            draw_line(0.0, y as f32, screen_width(), y as f32, 1.0, Color::from_rgba(50, 50, 60, 100));
        }
        
        draw_text("[I] Toggle | [1/2/3] Tabs | Drag=Swap | Drag out=Drop | Double-click=Use", 10.0, 30.0, 18.0, WHITE);
        draw_text("Hybrid: Native drawing + mqui Group::draggable()", 10.0, 55.0, 16.0, GRAY);
        
        // 快捷键
        if is_key_pressed(KeyCode::I) { inventory.toggle(); }
        if is_key_pressed(KeyCode::Escape) { break; }
        
        // 更新绘制
        inventory.update_and_draw();
        
        // 状态
        let status = if inventory.is_visible() { "背包: 打开" } else { "背包: 关闭 (按I打开)" };
        draw_text(status, 10.0, screen_height() - 40.0, 18.0, GREEN);
        draw_text(&format!("FPS: {}", get_fps()), screen_width() - 100.0, 30.0, 20.0, GREEN);
        
        next_frame().await;
    }
}

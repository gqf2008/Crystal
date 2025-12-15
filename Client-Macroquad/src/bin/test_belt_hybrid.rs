// ============================================================================
// 测试程序：快捷栏混合版本（Native 绘制 + mqui 拖放）
// ============================================================================
//
// 快捷键：
//   B      - 打开/关闭快捷栏
//   R      - 旋转布局（水平/垂直）
//   1-6    - 使用对应格子物品
//   ESC    - 退出程序
//
// 操作说明：
//   - 拖动物品到其他格子：交换位置
//   - 拖动物品到快捷栏外：丢弃物品
//   - 双击物品：使用物品
//   - 拖动空白区域：移动窗口
//
// ============================================================================

use macroquad::prelude::*;

use client_macroquad::resources::{set_data_path, preload_libraries, LibraryName};
use client_macroquad::scenes::dialogs::game::BeltDialogHybrid;

fn window_conf() -> Conf {
    Conf {
        window_title: "Belt Dialog Hybrid Test (Native + mqui)".to_owned(),
        window_width: 1024,
        window_height: 768,
        high_dpi: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    println!("==========================================");
    println!("  快捷栏测试 (混合版本: Native + mqui)");
    println!("==========================================");
    println!("快捷键:");
    println!("  B     - 打开/关闭快捷栏");
    println!("  R     - 旋转布局");
    println!("  1-6   - 使用物品");
    println!("  ESC   - 退出程序");
    println!("");
    println!("拖放操作:");
    println!("  - 拖动物品到其他格子 → 交换");
    println!("  - 拖动物品到窗口外 → 丢弃");
    println!("  - 双击物品 → 使用");
    println!("==========================================");
    
    // 初始化资源
    println!("🔄 正在加载纹理资源...");
    set_data_path("./Data/");
    preload_libraries(&[
        LibraryName::Prguse,
        LibraryName::Items,
    ]);
    println!("✅ 纹理库加载完成");
    
    // 创建混合版快捷栏
    let mut belt_dialog = BeltDialogHybrid::new();
    belt_dialog.load_textures();
    belt_dialog.set_position(vec2(
        screen_width() / 2.0 - belt_dialog.get_size().x / 2.0,
        screen_height() - 100.0
    ));
    belt_dialog.open();
    
    println!("🎒 混合版快捷栏已创建");
    
    loop {
        clear_background(Color::from_rgba(30, 30, 40, 255));
        
        // 绘制背景网格
        let grid_color = Color::from_rgba(50, 50, 60, 100);
        for x in (0..screen_width() as i32).step_by(50) {
            draw_line(x as f32, 0.0, x as f32, screen_height(), 1.0, grid_color);
        }
        for y in (0..screen_height() as i32).step_by(50) {
            draw_line(0.0, y as f32, screen_width(), y as f32, 1.0, grid_color);
        }
        
        // 绘制说明
        draw_text(
            "[B] Toggle | [R] Rotate | [1-6] Use | Drag to swap/drop | Double-click to use",
            10.0, 30.0, 18.0, WHITE
        );
        
        // 显示版本信息
        draw_text(
            "Hybrid Version: Native drawing + mqui Group::draggable()",
            10.0, 55.0, 16.0, GRAY
        );
        
        // 处理快捷键
        if is_key_pressed(KeyCode::B) {
            belt_dialog.toggle();
        }
        
        if is_key_pressed(KeyCode::R) && belt_dialog.is_visible() {
            belt_dialog.flip_layout();
        }
        
        // 数字键使用物品
        if is_key_pressed(KeyCode::Key1) { belt_dialog.use_item(0); }
        if is_key_pressed(KeyCode::Key2) { belt_dialog.use_item(1); }
        if is_key_pressed(KeyCode::Key3) { belt_dialog.use_item(2); }
        if is_key_pressed(KeyCode::Key4) { belt_dialog.use_item(3); }
        if is_key_pressed(KeyCode::Key5) { belt_dialog.use_item(4); }
        if is_key_pressed(KeyCode::Key6) { belt_dialog.use_item(5); }
        
        // ESC 退出
        if is_key_pressed(KeyCode::Escape) {
            break;
        }
        
        // 更新和绘制快捷栏
        belt_dialog.update_and_draw();
        
        // 显示状态
        let status = if belt_dialog.is_visible() {
            format!("快捷栏: 打开")
        } else {
            format!("快捷栏: 关闭 (按 B 打开)")
        };
        draw_text(&status, 10.0, screen_height() - 40.0, 18.0, GREEN);
        
        // 绘制 FPS
        draw_text(
            &format!("FPS: {}", get_fps()),
            screen_width() - 100.0, 30.0, 20.0, GREEN
        );
        
        next_frame().await;
    }
    
    println!("👋 程序退出");
}

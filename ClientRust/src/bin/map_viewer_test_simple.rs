// 最简化的地图查看器 - 用于测试基础渲染是否工作
// 只绘制纯色方块，不加载任何库

use macroquad::prelude::*;

const WINDOW_WIDTH: i32 = 1024;
const WINDOW_HEIGHT: i32 = 768;

fn window_conf() -> Conf {
    Conf {
        window_title: "超简化测试 - 只画方块".to_owned(),
        window_width: WINDOW_WIDTH,
        window_height: WINDOW_HEIGHT,
        window_resizable: false,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    println!("✅ 超简化测试启动");
    println!("🎮 控制: ESC 退出");
    
    let mut camera_x = 0.0;
    let mut camera_y = 0.0;
    let mut dragging = false;
    let mut last_mouse_pos = vec2(0.0, 0.0);
    
    loop {
        // 输入处理
        if is_key_pressed(KeyCode::Escape) {
            break;
        }
        
        // 鼠标拖拽
        if is_mouse_button_pressed(MouseButton::Left) {
            dragging = true;
            last_mouse_pos = vec2(mouse_position().0, mouse_position().1);
        }
        if is_mouse_button_released(MouseButton::Left) {
            dragging = false;
        }
        if dragging {
            let current_pos = vec2(mouse_position().0, mouse_position().1);
            let delta = current_pos - last_mouse_pos;
            camera_x -= delta.x;
            camera_y -= delta.y;
            last_mouse_pos = current_pos;
        }
        
        // 清屏
        clear_background(Color::from_rgba(40, 40, 50, 255));
        
        // 绘制10x10的彩色方块网格（模拟地图瓦片）
        for y in 0..10 {
            for x in 0..10 {
                let world_x = x as f32 * 100.0 - camera_x;
                let world_y = y as f32 * 100.0 - camera_y;
                
                // 彩虹色
                let hue = ((x + y) as f32 / 20.0) % 1.0;
                let color = Color::from_rgba(
                    ((hue * 255.0) as u8),
                    (((1.0 - hue) * 255.0) as u8),
                    128,
                    255
                );
                
                draw_rectangle(world_x, world_y, 90.0, 90.0, color);
                
                // 绘制边框
                draw_rectangle_lines(world_x, world_y, 90.0, 90.0, 2.0, WHITE);
            }
        }
        
        // UI信息
        draw_text(
            &format!("相机: ({:.0}, {:.0})", camera_x, camera_y),
            10.0, 30.0, 30.0, WHITE
        );
        draw_text("如果你能看到彩色方块并拖拽移动，说明渲染管道正常", 10.0, 60.0, 20.0, GREEN);
        draw_text("按 ESC 退出", 10.0, screen_height() - 20.0, 20.0, YELLOW);
        
        next_frame().await;
    }
    
    println!("✅ 测试结束");
}

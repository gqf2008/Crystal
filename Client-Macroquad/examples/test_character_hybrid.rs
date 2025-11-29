// ============================================================================
// 测试角色对话框（混合版本）
// ============================================================================
//
// 运行: cargo run --example test_character_hybrid
// 
// 功能测试：
// - 快捷键: C 打开/关闭，1-4 切换标签页
// - 拖拽装备到窗口外卸下
// - 拖拽交换左右戒指/手镯
// ============================================================================

use macroquad::prelude::*;

use client_macroquad::resources::libraries::{set_data_path, load_core_libraries};
use client_macroquad::scenes::dialogs::game::CharacterDialogHybrid;
use client_macroquad::ui::text_renderer::{init_chinese_font, draw_text_cn};

fn window_conf() -> Conf {
    Conf {
        window_title: "角色对话框 Hybrid 测试".to_string(),
        window_width: 800,
        window_height: 600,
        window_resizable: false,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    println!("============================================");
    println!("       角色对话框 Hybrid 测试程序");
    println!("============================================");
    println!("快捷键:");
    println!("  C        打开/关闭角色对话框");
    println!("  1-4      切换标签页");
    println!("操作:");
    println!("  拖拽装备 到窗口外 = 卸下装备");
    println!("  拖拽装备 到其他槽位 = 交换位置");
    println!("============================================\n");
    
    set_data_path("Data");
    let _ = load_core_libraries();
    init_chinese_font().await;
    
    let mut dialog = CharacterDialogHybrid::new();
    dialog.load_textures().await;
    dialog.open();
    dialog.set_position(vec2(268.0, 100.0));
    
    loop {
        clear_background(Color::from_rgba(30, 30, 40, 255));
        
        // 快捷键
        if is_key_pressed(KeyCode::C) {
            dialog.toggle();
        }
        if is_key_pressed(KeyCode::Escape) && dialog.is_visible() {
            dialog.close();
        }
        
        // 背景网格
        for i in 0..=20 {
            let x = i as f32 * 40.0;
            draw_line(x, 0.0, x, 600.0, 1.0, Color::from_rgba(60, 60, 70, 255));
        }
        for i in 0..=15 {
            let y = i as f32 * 40.0;
            draw_line(0.0, y, 800.0, y, 1.0, Color::from_rgba(60, 60, 70, 255));
        }
        
        // 操作提示
        draw_text_cn("C = 打开/关闭 | 1-4 = 切换标签", 10.0, 25.0, 20.0, WHITE);
        
        // 装备状态
        let y = 580.0;
        let mut x = 10.0;
        for i in 0..14 {
            let has_item = dialog.equipment[i].is_some();
            let color = if has_item { GREEN } else { GRAY };
            draw_circle(x + 8.0, y - 6.0, 5.0, color);
            x += 15.0;
        }
        draw_text_cn("装备槽:", 10.0, y - 20.0, 14.0, WHITE);
        
        // 对话框
        dialog.update_and_draw();
        
        next_frame().await;
    }
}

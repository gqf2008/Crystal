// ============================================================================
// 测试商城对话框（混合版本）
// ============================================================================
//
// 运行: cargo run --example test_shop_hybrid
// 
// 功能测试：
// - 打开/关闭商城
// - 分类筛选
// - 商品浏览
// - 翻页
// - 预览
// ============================================================================

use macroquad::prelude::*;

use client_macroquad::resources::libraries::{set_data_path, load_core_libraries};
use client_macroquad::scenes::dialogs::game::GameShopDialogHybrid;
use client_macroquad::ui::text_renderer::{init_chinese_font, draw_text_cn};

fn window_conf() -> Conf {
    Conf {
        window_title: "商城对话框 Hybrid 测试".to_string(),
        window_width: 1024,
        window_height: 768,
        window_resizable: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    println!("============================================");
    println!("       商城对话框 Hybrid 测试");
    println!("============================================");
    println!("快捷键:");
    println!("  S        打开/关闭商城");
    println!("  ESC      关闭");
    println!("============================================\n");
    
    // 初始化
    set_data_path("Data");
    let _ = load_core_libraries();
    init_chinese_font().await;
    
    // 创建对话框
    let mut shop_dialog = GameShopDialogHybrid::new();
    
    // 加载纹理
    println!("🛒 加载商城纹理...");
    shop_dialog.load_textures().await;
    println!("✅ 纹理加载完成");
    
    // 设置初始位置并打开
    shop_dialog.set_position(vec2(150.0, 100.0));
    shop_dialog.open();
    
    loop {
        // 背景
        clear_background(Color::from_rgba(30, 35, 40, 255));
        
        // 绘制网格
        let grid_color = Color::from_rgba(45, 50, 55, 255);
        for i in 0..=((screen_width() / 40.0) as i32 + 1) {
            let x = i as f32 * 40.0;
            draw_line(x, 0.0, x, screen_height(), 1.0, grid_color);
        }
        for i in 0..=((screen_height() / 40.0) as i32 + 1) {
            let y = i as f32 * 40.0;
            draw_line(0.0, y, screen_width(), y, 1.0, grid_color);
        }
        
        // 快捷键处理
        if is_key_pressed(KeyCode::S) {
            shop_dialog.toggle();
            println!("🛒 商城: {}", if shop_dialog.is_visible() { "打开" } else { "关闭" });
        }
        
        if is_key_pressed(KeyCode::Escape) {
            if shop_dialog.is_visible() {
                shop_dialog.close();
                println!("❌ 关闭商城");
            } else {
                println!("👋 退出");
                break;
            }
        }
        
        // 绘制商城对话框
        shop_dialog.update_and_draw();
        
        // 帮助文字
        draw_text_cn(
            "S = 打开/关闭商城 | ESC = 关闭/退出",
            10.0, screen_height() - 25.0, 14.0, Color::from_rgba(200, 200, 200, 180)
        );
        
        // 状态显示
        let status = if shop_dialog.is_visible() { "商城已打开" } else { "商城已关闭" };
        draw_text_cn(status, 10.0, 25.0, 16.0, WHITE);
        
        next_frame().await;
    }
}

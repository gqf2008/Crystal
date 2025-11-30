/// 测试 BeltDialog（快捷栏）- Hybrid 版本
/// 
/// 运行命令：cargo run --bin test_belt_dialog

use client_macroquad::scenes::dialogs::game::{BeltDialogHybrid, MainDialog};
use client_macroquad::ui::text_renderer::{init_chinese_font, draw_text_cn};
use macroquad::prelude::*;

#[macroquad::main("传奇2 - 快捷栏测试")]
async fn main() {
    println!("🎮 传奇2 - 快捷栏测试");
    println!("📐 窗口尺寸: 1024x768");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("💡 提示：");
    println!("   - 点击旋转按钮切换水平/垂直布局");
    println!("   - 点击关闭按钮隐藏快捷栏");
    println!("   - 按 ESC 退出");
    
    // 初始化中文字体
    println!("🔤 正在加载中文字体...");
    init_chinese_font().await;
    
    // 创建主界面（用于定位）
    let mut main_dialog = MainDialog::new();
    
    // 加载原生UI纹理
    main_dialog.load_native_textures().await;
    
    // 创建快捷栏
    let mut belt_dialog = BeltDialogHybrid::new();
    belt_dialog.load_textures().await;
    belt_dialog.open();
    
    // 设置位置
    let screen_height = screen_height();
    let screen_width = screen_width();
    let main_dialog_x = (screen_width / 2.0) - 400.0;
    belt_dialog.set_position(vec2(main_dialog_x + 230.0, screen_height - 150.0));
    
    println!("🎬 进入快捷栏测试界面");
    
    loop {
        // 检测 ESC 退出
        if is_key_pressed(KeyCode::Escape) {
            println!("👋 按下 ESC，退出测试");
            break;
        }
        
        // 清空背景
        clear_background(Color::from_rgba(20, 20, 30, 255));
        
        // 绘制主界面（底部工具栏）- 纯原生绘制
        main_dialog.update_and_draw();
        
        // 绘制快捷栏 - 纯原生绘制
        belt_dialog.update_and_draw();
        
        // 状态信息（使用中文字体）
        let status_text = format!(
            "快捷栏测试 - 按 ESC 退出 | 快捷栏: {}",
            if belt_dialog.is_visible() { "显示" } else { "隐藏" }
        );
        draw_text_cn(&status_text, 10.0, 25.0, 16.0, WHITE);
        
        // 下一帧
        next_frame().await;
    }
}

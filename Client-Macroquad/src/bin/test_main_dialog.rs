// 测试 MainDialog - 纯 Hybrid 版本

use client_macroquad::scenes::dialogs::game::MainDialog;
use client_macroquad::ui::text_renderer::{init_chinese_font, draw_text_cn, measure_text_cn};
use macroquad::prelude::*;

fn window_conf() -> Conf {
    Conf {
        window_title: "传奇2 - MainDialog 测试（Hybrid）".to_owned(),
        window_width: 1024,
        window_height: 768,
        high_dpi: false,
        window_resizable: false,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    println!("🎮 传奇2 - MainDialog 测试（纯 Hybrid 版本）");
    println!("📐 窗口尺寸: {}x{}", screen_width(), screen_height());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // 初始化中文字体（重要！）
    println!("🔤 正在加载中文字体...");
    init_chinese_font().await;
    
    // 创建 MainDialog（内部自动创建所有子对话框）
    let mut main_dialog = MainDialog::new();
    
    // 加载原生UI纹理
    main_dialog.load_native_textures().await;
    
    println!("✅ MainDialog 及所有子对话框已创建（纯 Hybrid 模式）");
    println!("💡 提示:");
    println!("   - 点击底部按钮打开各种对话框（背包、角色、技能、任务、选项、菜单、商城）");
    println!("   - 按 M 键快速切换小地图显示/隐藏");
    println!("   - 按 TAB 键切换小地图大小模式（大模式/小模式）");
    println!("   - 所有对话框都支持拖拽（拖拽标题栏）");
    println!("   - 按 ESC 退出");

    // FPS 统计
    let mut frame_times: Vec<f32> = Vec::with_capacity(60);
    let mut last_time = get_time();

    loop {
        let _frame_start = get_time();
        
        clear_background(Color::from_rgba(60, 80, 100, 255));

        // 绘制背景提示
        let text = "游戏主场景 - 点击 Size 按钮或按 Tab 切换聊天窗口大小";
        let font_size = 32.0;
        let text_size = measure_text_cn(&text, font_size);
        draw_text_cn(
            text,
            screen_width() / 2.0 - text_size.width / 2.0,
            screen_height() / 2.0 - 100.0,
            font_size,
            WHITE,
        );

        // 绘制主对话框 - 纯原生绘制
        main_dialog.update_and_draw();

        // 绘制所有子对话框 - 纯原生绘制
        let _ui_consumed = main_dialog.show_dialogs();

        // 计算FPS
        let current_time = get_time();
        let delta_time = (current_time - last_time) as f32;
        last_time = current_time;
        
        frame_times.push(delta_time);
        if frame_times.len() > 60 {
            frame_times.remove(0);
        }
        
        let avg_frame_time: f32 = frame_times.iter().sum::<f32>() / frame_times.len() as f32;
        let fps = if avg_frame_time > 0.0 { 1.0 / avg_frame_time } else { 0.0 };
        let frame_time_ms = avg_frame_time * 1000.0;

        // 绘制性能信息（左上角）
        let perf_text = format!(
            "FPS: {:.1}  帧时间: {:.2}ms  (纯 Hybrid 模式)",
            fps, frame_time_ms
        );
        draw_text_cn(&perf_text, 10.0, 25.0, 20.0, Color::from_rgba(0, 255, 0, 255));

        // 键盘快捷键处理（仅在没有输入框激活时）
        if !main_dialog.is_any_input_active() {
            if is_key_pressed(KeyCode::M) {
                main_dialog.toggle_minimap();
            }
            
            if is_key_pressed(KeyCode::Tab) {
                main_dialog.toggle_minimap_size();
            }
            
            // ESC 退出（仅在没有输入框激活时）
            if is_key_pressed(KeyCode::Escape) {
                println!("👋 退出测试");
                break;
            }
        }

        // 调试：打印接收到的字符输入
        // 注意：get_char_pressed() 会消耗字符队列；如果这里无条件读取，会抢走 ChatDialog 的输入。
        if !main_dialog.is_any_input_active() {
            while let Some(ch) = get_char_pressed() {
                println!("📝 收到字符: '{}' (U+{:04X})", ch, ch as u32);
            }
        }

        next_frame().await;
    }
}

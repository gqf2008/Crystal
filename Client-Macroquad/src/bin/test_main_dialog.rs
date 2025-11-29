// 测试 MainDialog

use client_macroquad::scenes::dialogs::MainDialog;
use macroquad::prelude::*;

fn window_conf() -> Conf {
    Conf {
        window_title: "传奇2 - MainDialog 测试".to_owned(),
        window_width: 1024,
        window_height: 768,
        high_dpi: false,
        window_resizable: false,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    println!("🎮 传奇2 - MainDialog 测试");
    println!("📐 窗口尺寸: {}x{}", screen_width(), screen_height());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
 // 配置 egui（只设置一次）
    egui_macroquad::cfg(|ctx| {
        let mut fonts = egui_macroquad::egui::FontDefinitions::default();
        
        // 加载中文字体
        let font_data = std::fs::read("assets/fonts/AlibabaPuHuiTi-3-55-Regular.ttf")
            .or_else(|_| std::fs::read("assets/fonts/Chinese.ttc"))
            .or_else(|_| std::fs::read("C:\\Windows\\Fonts\\msyh.ttc"))
            .unwrap_or_else(|_| {
                println!("⚠️  无法加载中文字体，使用默认字体");
                vec![]
            });

        if !font_data.is_empty() {
            fonts.font_data.insert(
                "chinese".to_owned(),
                std::sync::Arc::new(egui_macroquad::egui::FontData::from_owned(font_data)),
            );

            // 设置字体优先级
            fonts
                .families
                .get_mut(&egui_macroquad::egui::FontFamily::Proportional)
                .unwrap()
                .insert(0, "chinese".to_owned());

            fonts
                .families
                .get_mut(&egui_macroquad::egui::FontFamily::Monospace)
                .unwrap()
                .insert(0, "chinese".to_owned());
                
            println!("✅ 已加载中文字体");
        }

        ctx.set_fonts(fonts);

        // 设置 DPI 缩放
        let dpi_scale = screen_dpi_scale();
        ctx.set_pixels_per_point(dpi_scale);
    });
    
    // 创建 MainDialog（内部自动创建所有子对话框）
    let mut main_dialog = MainDialog::new();
    
    // 加载原生UI纹理
    main_dialog.load_native_textures().await;
    
    println!("✅ MainDialog 及所有子对话框已创建");
    println!("💡 提示:");
    println!("   - 点击底部按钮打开各种对话框（背包、角色、技能、任务、选项、菜单、商城）");
    println!("   - 按 M 键快速切换小地图显示/隐藏");
    println!("   - 按 TAB 键切换小地图大小模式（大模式/小模式）");
    println!("   - 按 N 键切换背包UI模式（原生/egui）");
    println!("   - 按 B 键切换快捷栏UI模式（原生/egui）");
    println!("   - 所有对话框都支持拖拽（拖拽标题栏）");
    println!("   - 按 ESC 退出");

    // FPS 统计
    let mut frame_times: Vec<f32> = Vec::with_capacity(60);
    let mut last_time = get_time();

    loop {
        let frame_start = get_time();
        
        clear_background(Color::from_rgba(60, 80, 100, 255));

        // 绘制背景提示
        let text = "游戏主场景 - 点击 Size 按钮或按 Tab 切换聊天窗口大小";
        let font_size = 32.0;
        let text_size = measure_text(&text, None, font_size as u16, 1.0);
        draw_text(
            text,
            screen_width() / 2.0 - text_size.width / 2.0,
            screen_height() / 2.0 - 100.0,
            font_size,
            WHITE,
        );

        // egui UI
        let egui_start = get_time();
        egui_macroquad::ui(|ctx| {
            // 绘制主对话框和所有子对话框
            main_dialog.show(ctx);
            main_dialog.show_dialogs(ctx);
        });
        let egui_time = (get_time() - egui_start) * 1000.0; // 转换为毫秒

        // 绘制 egui
        egui_macroquad::draw();

        // 绘制原生UI对话框（在 egui 之后）
        main_dialog.show_native_dialogs();

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
            "FPS: {:.1}  帧时间: {:.2}ms  UI渲染: {:.2}ms",
            fps, frame_time_ms, egui_time
        );
        draw_text(&perf_text, 10.0, 25.0, 20.0, Color::from_rgba(0, 255, 0, 255));

        // 键盘快捷键处理
        if is_key_pressed(KeyCode::M) {
            main_dialog.toggle_minimap();
        }
        
        if is_key_pressed(KeyCode::Tab) {
            main_dialog.toggle_minimap_size();
        }
        
        // N 键切换背包UI模式
        if is_key_pressed(KeyCode::N) {
            main_dialog.toggle_inventory_mode();
        }
        
        // B 键切换快捷栏UI模式
        if is_key_pressed(KeyCode::B) {
            main_dialog.toggle_belt_mode();
        }
        
        // ESC 退出
        if is_key_pressed(KeyCode::Escape) {
            println!("👋 退出测试");
            break;
        }

        next_frame().await;
    }
}

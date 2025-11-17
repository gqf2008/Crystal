use macroquad::prelude::*;

// 导入必要的trait
use client_macroquad::scenes::dialogs::game::inventory_dialog::InventoryDialog;
use client_macroquad::scenes::dialogs::Dialog;

fn window_conf() -> Conf {
    Conf {
        window_title: "传奇2 - Inventory Features 测试".to_owned(),
        window_width: 1024,
        window_height: 768,
        high_dpi: false,
        window_resizable: false,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    println!("🎒 传奇2 - Inventory Features 测试");
    println!("📐 窗口尺寸: {}x{}", screen_width(), screen_height());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // 配置 egui 中文字体
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

            fonts
                .families
                .get_mut(&egui_macroquad::egui::FontFamily::Proportional)
                .unwrap()
                .insert(0, "chinese".to_owned());

            ctx.set_fonts(fonts);
            println!("✅ 已加载中文字体");
        }
    });
    
    // 创建inventory dialog
    let mut inventory_dialog = InventoryDialog::new();
    let mut show_inventory = true;
    
    println!("✅ InventoryDialog 已创建并显示");
    println!("💡 测试功能:");
    println!("   - 鼠标左键：选择物品");
    println!("   - 再次左键：移动物品到新位置"); 
    println!("   - 鼠标右键：显示简洁菜单");
    println!("   - 按 I 键切换背包显示");
    println!("   - 按 ESC 退出");
    
    loop {
        clear_background(Color::from_rgba(60, 80, 100, 255));
        
        // 检查全局退出条件
        if is_key_pressed(KeyCode::Escape) {
            break;
        }
        
        // 切换背包显示
        if is_key_pressed(KeyCode::I) {
            inventory_dialog.toggle();
        }
        
        // 绘制背景提示
        let text = "背包系统测试 - 按 I 键切换背包 | 右键点击物品查看菜单";
        let font_size = 24.0;
        let text_size = measure_text(&text, None, font_size as u16, 1.0);
        draw_text(
            text,
            screen_width() / 2.0 - text_size.width / 2.0,
            screen_height() / 2.0 - 200.0,
            font_size,
            WHITE,
        );
        
        // 绘制操作提示
        let tips = [
            "🎒 背包操作说明:",
            "• 左键点击选择物品",
            "• 再次左键移动到新位置",
            "• 右键显示简洁菜单",
            "• 传奇2原版操作体验",
            "• 无tooltip干扰，简洁高效",
        ];
        
        for (i, tip) in tips.iter().enumerate() {
            draw_text(
                tip,
                50.0,
                100.0 + i as f32 * 25.0,
                18.0,
                LIGHTGRAY,
            );
        }
        
        // 处理egui
        egui_macroquad::ui(|egui_ctx| {
            // 使用Dialog trait的show方法
            inventory_dialog.show(egui_ctx, &mut show_inventory);
        });
        
        // 绘制egui
        egui_macroquad::draw();
        
        // 显示状态信息
        draw_text(
            &format!("FPS: {:.0}", get_fps()),
            10.0,
            20.0,
            20.0,
            GREEN,
        );
        
        draw_text(
            &format!("背包状态: {}", 
                if inventory_dialog.is_visible() { "打开" } else { "关闭" }
            ),
            10.0,
            40.0,
            16.0,
            if inventory_dialog.is_visible() { GREEN } else { RED },
        );
        
        draw_text(
            "按 I 键切换背包 | ESC 退出",
            10.0,
            screen_height() - 20.0,
            16.0,
            WHITE,
        );
        
        next_frame().await;
    }
    
    println!("👋 测试结束");
}
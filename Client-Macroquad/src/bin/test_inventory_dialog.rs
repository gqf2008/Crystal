// 测试 InventoryDialog（背包系统）

use client_macroquad::scenes::dialogs::game::InventoryDialog;
use client_macroquad::ui::text_renderer::*;
use macroquad::prelude::*;

fn window_conf() -> Conf {
    Conf {
        window_title: "传奇2 - 背包系统测试".to_owned(),
        window_width: 1024,
        window_height: 768,
        high_dpi: false,
        window_resizable: false,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    println!("🎮 传奇2 - 背包系统测试");
    println!("📐 窗口尺寸: {}x{}", screen_width(), screen_height());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // 初始化中文字体
    init_chinese_font().await;
    
    // 配置 egui 中文字体
    egui_macroquad::cfg(|ctx| {
        setup_egui_chinese_font(ctx);
    });
    
    // 创建背包对话框
    let mut inventory = InventoryDialog::new();
    
    // 默认显示背包
    let mut inventory_open = true;
    
    println!("✅ 背包对话框已创建并显示");
    println!("\n💡 测试功能清单:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🎯 基础功能:");
    println!("   ✓ 按 I 键切换背包显示/隐藏");
    println!("   ✓ 按 ESC 键关闭背包");
    println!("   ✓ 拖拽背景可移动窗口");
    println!("   ✓ 按 Tab 键切换标签页（物品1/物品2/任务）");
    println!("\n📦 物品操作:");
    println!("   ✓ 左键点击选择/交换物品");
    println!("   ✓ 拖拽物品到其他格子");
    println!("   ✓ 相同物品自动堆叠合并");
    println!("   ✓ Shift+点击：分离一半物品");
    println!("   ✓ Ctrl+点击：自定义分离数量");
    println!("   ✓ 右键点击：显示快捷菜单（使用/丢弃/查看属性）");
    println!("   ✓ 鼠标悬停0.8秒：显示物品详细信息");
    println!("\n⌨️  快捷键:");
    println!("   ✓ 1-9 数字键：快速使用对应格子的物品");
    println!("   ✓ Delete 键：丢弃选中的物品");
    println!("   ✓ Enter 键：使用选中的物品");
    println!("   ✓ 方向键：在格子间导航（8列网格）");
    println!("\n💰 金币系统:");
    println!("   ✓ 点击金币区域：触发拾取动画演示");
    println!("   ✓ 金币从屏幕底部飞入");
    println!("   ✓ 抛物线轨迹 + 渐变效果");
    println!("\n🎒 背包扩展:");
    println!("   ✓ 默认容量：46格（物品1页）");
    println!("   ✓ 扩展容量：最多80格（物品1+2页）");
    println!("   ✓ 物品2页：点击扩展按钮购买（需要金币）");
    println!("   ✓ 每次扩展4格，费用递增");
    println!("   ✓ 金币不足时显示警告");
    println!("\n💾 数据持久化:");
    println!("   ✓ 自动保存：关闭背包时保存数据");
    println!("   ✓ 自动加载：启动时加载上次数据");
    println!("   ✓ 存储位置：%LOCALAPPDATA%/Mir2Client/inventory.json");
    println!("   ✓ 格式：JSON（可读性好，便于调试）");
    println!("\n🔍 视觉反馈:");
    println!("   ✓ 鼠标悬停：绿色边框高亮");
    println!("   ✓ 选中物品：黄色边框标记");
    println!("   ✓ 负重条：颜色随重量变化（绿/橙/红）");
    println!("   ✓ 空格数量：实时显示");
    println!("   ✓ 锁定格子：显示锁图标");
    println!("\n📊 测试数据:");
    println!("   • 物品1页：已填充46格测试物品");
    println!("   • 物品2页：已填充40格测试物品");
    println!("   • 任务页：已填充40格任务物品");
    println!("   • 初始金币：123,456");
    println!("   • 当前负重：75/100");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // FPS 统计
    let mut frame_times: Vec<f32> = Vec::with_capacity(60);
    let mut last_time = get_time();

    loop {
        // 绘制背景
        clear_background(Color::from_rgba(40, 50, 60, 255));

        // 绘制测试说明
        let hints = [
            "🎮 背包系统测试中...",
            "按 I 键切换背包 | 按 Tab 切换标签页",
            "拖拽物品、点击金币查看动画效果",
        ];
        
        for (i, hint) in hints.iter().enumerate() {
            draw_text_centered(
                hint,
                screen_width() / 2.0,
                screen_height() / 2.0 - 60.0 + (i as f32 * 30.0),
                24.0,
                Color::from_rgba(200, 200, 200, 255),
            );
        }

        // egui UI
        let egui_start = get_time();
        egui_macroquad::ui(|ctx| {
            // 绘制背包
            use client_macroquad::scenes::dialogs::Dialog;
            inventory.show(ctx, &mut inventory_open);
            
            // 绘制测试控制面板
            egui_macroquad::egui::Window::new("🧪 测试控制面板")
                .default_pos(egui_macroquad::egui::pos2(10.0, 50.0))
                .default_size(egui_macroquad::egui::vec2(300.0, 400.0))
                .resizable(false)
                .show(ctx, |ui| {
                    ui.heading("背包状态");
                    ui.separator();
                    
                    ui.label(format!("可见状态: {}", if inventory_open { "✅ 显示中" } else { "❌ 已隐藏" }));
                    ui.label(format!("金币: {} 💰", inventory.get_gold()));
                    ui.label(format!("最大容量: {} 格", inventory.max_capacity));
                    
                    let empty_count = inventory.item_slots.iter().filter(|s| s.icon_index.is_none()).count();
                    ui.label(format!("空格数: {}", empty_count));
                    
                    let filled_count = inventory.item_slots.len() - empty_count;
                    ui.label(format!("已使用: {} 格", filled_count));
                    
                    ui.separator();
                    ui.heading("快捷操作");
                    
                    if ui.button("🎒 切换背包显示").clicked() {
                        inventory_open = !inventory_open;
                    }
                    
                    if ui.button("💰 增加 1000 金币").clicked() {
                        let new_gold = inventory.get_gold() + 1000;
                        inventory.set_gold(new_gold);
                        println!("💰 金币增加到: {}", new_gold);
                    }
                    
                    if ui.button("💸 减少 1000 金币").clicked() {
                        let current = inventory.get_gold();
                        let new_gold = if current >= 1000 { current - 1000 } else { 0 };
                        inventory.set_gold(new_gold);
                        println!("💸 金币减少到: {}", new_gold);
                    }
                    
                    if ui.button("📍 重置位置").clicked() {
                        // 重新创建对话框会重置位置
                        println!("💡 关闭并重新打开背包即可重置位置");
                    }
                    
                    ui.separator();
                    ui.heading("标签页切换");
                    
                    ui.horizontal(|ui| {
                        if ui.button("物品 I").clicked() {
                            inventory.active_tab = client_macroquad::scenes::dialogs::game::InventoryTab::Items;
                        }
                        if ui.button("物品 II").clicked() {
                            inventory.active_tab = client_macroquad::scenes::dialogs::game::InventoryTab::Items2;
                        }
                        if ui.button("任务").clicked() {
                            inventory.active_tab = client_macroquad::scenes::dialogs::game::InventoryTab::Quest;
                        }
                    });
                    
                    ui.separator();
                    ui.heading("数据操作");
                    
                    if ui.button("💾 手动保存数据").clicked() {
                        if let Err(e) = inventory.save_data() {
                            println!("❌ 保存失败: {}", e);
                        } else {
                            println!("✅ 数据已保存");
                        }
                    }
                    
                    if ui.button("📂 重新加载数据").clicked() {
                        if let Err(e) = inventory.load_data() {
                            println!("❌ 加载失败: {}", e);
                        } else {
                            println!("✅ 数据已重新加载");
                        }
                    }
                    
                    ui.separator();
                    ui.label("💡 提示：关闭背包时自动保存");
                });
        });
        let egui_time = (get_time() - egui_start) * 1000.0;

        // 绘制 egui
        egui_macroquad::draw();

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

        // 绘制性能信息（右上角）
        let perf_text = format!(
            "FPS: {:.1} | 帧时间: {:.2}ms | UI: {:.2}ms",
            fps, frame_time_ms, egui_time
        );
        
        draw_text_right_aligned(
            &perf_text,
            screen_width() - 10.0,
            25.0,
            16.0,
            Color::from_rgba(0, 255, 0, 255),
        );

        // 键盘快捷键处理
        if is_key_pressed(KeyCode::I) {
            inventory_open = !inventory_open;
        }
        
        // ESC 退出程序
        if is_key_pressed(KeyCode::Escape) && !inventory_open {
            println!("👋 退出测试");
            break;
        }

        next_frame().await;
    }
}

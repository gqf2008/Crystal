/// 金币拾取动画测试程序
/// 
/// 这个程序演示了原版传奇2的经典金币拾取动画效果：
/// - 金币从屏幕底部飞入背包的金币显示区域
/// - 带有抛物线轨迹和渐变透明度效果
/// - 显示金币数量文字

use egui_macroquad::egui;
use macroquad::prelude::*;
use client_macroquad::scenes::dialogs::game::inventory_dialog::InventoryDialog;
use client_macroquad::scenes::dialogs::Dialog;

const WINDOW_WIDTH: f32 = 1024.0;
const WINDOW_HEIGHT: f32 = 768.0;

#[macroquad::main("传奇2 金币拾取动画测试")]
async fn main() {
    // 设置窗口
    request_new_screen_size(WINDOW_WIDTH, WINDOW_HEIGHT);
    
    // 创建背包对话框
    let mut inventory_dialog = InventoryDialog::new();
    inventory_dialog.toggle(); // 显示背包
    
    // UI状态
    let mut open = true;
    
    loop {
        clear_background(Color::from_rgba(20, 30, 40, 255));
        
        // 绘制背景说明
        draw_text(
            "传奇2 金币拾取动画测试",
            20.0,
            30.0,
            24.0,
            WHITE,
        );
        
        draw_text(
            "操作说明：",
            20.0,
            60.0,
            16.0,
            LIGHTGRAY,
        );
        
        draw_text(
            "• I键: 打开/关闭背包",
            20.0,
            80.0,
            14.0,
            LIGHTGRAY,
        );
        
        draw_text(
            "• 点击背包中的金币: 触发拾取动画演示",
            20.0,
            100.0,
            14.0,
            LIGHTGRAY,
        );
        
        draw_text(
            "• Tab键: 切换背包标签页",
            20.0,
            120.0,
            14.0,
            LIGHTGRAY,
        );
        
        draw_text(
            "• ESC键: 退出",
            20.0,
            140.0,
            14.0,
            LIGHTGRAY,
        );
        
        // 处理键盘输入
        if is_key_pressed(KeyCode::Escape) {
            break;
        }
        
        if is_key_pressed(KeyCode::I) {
            inventory_dialog.toggle();
        }
        
        // 绘制egui界面
        egui_macroquad::ui(|ctx| {
            // 显示背包对话框
            inventory_dialog.show(ctx, &mut open);
            
            // 添加一些测试按钮
            egui::Window::new("测试控制面板")
                .fixed_pos(egui::pos2(20.0, 200.0))
                .resizable(false)
                .show(ctx, |ui| {
                    ui.heading("金币动画测试");
                    ui.separator();
                    
                    if ui.button("触发金币拾取动画").clicked() {
                        // 从屏幕底部随机位置触发动画
                        let screen_rect = ctx.screen_rect();
                        let time_nanos = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .subsec_nanos();
                        
                        let random_offset = ((time_nanos % 1000) as f32 / 1000.0 - 0.5) * 300.0;
                        let start_pos = egui::pos2(
                            screen_rect.center().x + random_offset,
                            screen_rect.max.y - 30.0,
                        );
                        
                        // 目标位置：背包金币显示区域（估算）
                        let target_pos = egui::pos2(
                            inventory_dialog.get_position().x + 100.0,
                            inventory_dialog.get_position().y + 220.0,
                        );
                        
                        let pickup_amount = 50 + (time_nanos % 500); // 50-549金币
                        
                        inventory_dialog.trigger_gold_pickup(start_pos, pickup_amount, target_pos);
                    }
                    
                    if ui.button("连续拾取测试").clicked() {
                        // 触发多个金币动画
                        for i in 0..5 {
                            let screen_rect = ctx.screen_rect();
                            let time_offset = i as f32 * 0.1;
                            
                            let start_pos = egui::pos2(
                                screen_rect.center().x + (i as f32 - 2.0) * 80.0,
                                screen_rect.max.y - 30.0,
                            );
                            
                            let target_pos = egui::pos2(
                                inventory_dialog.get_position().x + 100.0,
                                inventory_dialog.get_position().y + 220.0,
                            );
                            
                            let pickup_amount = 10 + i * 20;
                            
                            // 添加延时效果
                            std::thread::sleep(std::time::Duration::from_millis((time_offset * 100.0) as u64));
                            inventory_dialog.trigger_gold_pickup(start_pos, pickup_amount, target_pos);
                        }
                    }
                    
                    ui.separator();
                    ui.label(format!("当前金币: {}", inventory_dialog.get_gold()));
                    ui.label("点击背包中的金币数字也可以触发动画");
                });
        });
        
        // 绘制egui
        egui_macroquad::draw();
        
        next_frame().await;
    }
}
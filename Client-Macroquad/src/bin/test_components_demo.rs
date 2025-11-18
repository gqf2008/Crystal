// ============================================================================
// 公共组件功能测试演示程序
// 专门测试各个公共组件的基本功能和API
// ============================================================================

// FIXME: 此测试文件使用了已删除的组件，需要更新
// use client_macroquad::ui::components::{
//     MirButton, MirLabel, MirTextBox, MirListBox, MirProgressBar,
//     ListItem, MirControl
};
use macroquad::prelude::*;
use egui_macroquad::egui;

#[macroquad::main("公共组件功能测试")]
async fn main() {
    println!("🚀 启动公共组件功能测试程序...");
    
    // 创建组件实例
    let mut test_button = MirButton::new("test_btn")
        .with_rect(egui::pos2(50.0, 50.0), egui::vec2(120.0, 35.0))
        .with_text("点击我！");
    
    let mut status_label = MirLabel::new("status", "组件测试就绪")
        .with_position(egui::pos2(50.0, 100.0))
        .with_font_size(16.0)
        .with_color(egui::Color32::YELLOW);
    
    let mut name_input = MirTextBox::new("name_textbox")
        .with_rect(egui::pos2(50.0, 130.0), egui::vec2(200.0, 25.0))
        .with_placeholder("输入你的名字...");
    
    let mut password_input = MirTextBox::new("pass_textbox")
        .with_rect(egui::pos2(270.0, 130.0), egui::vec2(200.0, 25.0))
        .with_placeholder("输入密码...")
        .with_password(true);
    
    let mut items_list = MirListBox::new("items_listbox")
        .with_rect(egui::pos2(50.0, 170.0), egui::vec2(200.0, 150.0));
    
    // 添加列表项
    items_list.add_item(ListItem::new("屠龙刀", "weapon_1"));
    items_list.add_item(ListItem::new("倚天剑", "weapon_2"));
    items_list.add_item(ListItem::new("麻痹戒指", "ring_1"));
    items_list.add_item(ListItem::new("护身戒指", "ring_2"));
    items_list.add_item(ListItem::new("超级红药", "potion_1"));
    
    let mut hp_bar = MirProgressBar::new("hp_progress")
        .with_rect(egui::pos2(270.0, 170.0), egui::vec2(200.0, 20.0))
        .with_colors(egui::Color32::RED, egui::Color32::DARK_RED)
        .with_text(true, "HP: {current}/{max}");
    
    let mut mp_bar = MirProgressBar::new("mp_progress")
        .with_rect(egui::pos2(270.0, 200.0), egui::vec2(200.0, 20.0))
        .with_colors(egui::Color32::BLUE, egui::Color32::DARK_BLUE)
        .with_text(true, "MP: {current}/{max}");
    
    let mut exp_bar = MirProgressBar::new("exp_progress")  
        .with_rect(egui::pos2(270.0, 230.0), egui::vec2(200.0, 20.0))
        .with_colors(egui::Color32::YELLOW, egui::Color32::from_rgb(128, 128, 0))
        .with_text(true, "EXP: {percent}%");
    
    // 设置初始进度条数值
    hp_bar.set_value(85.0);
    mp_bar.set_value(62.0); 
    exp_bar.set_value(34.0);
    
    let mut click_count = 0;
    let mut frame_count = 0;
    
    println!("✅ 所有组件创建完成，开始渲染循环...");
    
    loop {
        clear_background(Color::from_rgba(40, 40, 40, 255));
        frame_count += 1;
        
        // 动画效果：让进度条值产生变化
        if frame_count % 60 == 0 {
            let time = get_time() as f32;
            let new_hp = 50.0 + (time * 0.5).sin() * 30.0;
            let new_mp = 40.0 + (time * 0.3).cos() * 25.0;
            hp_bar.set_value(new_hp);
            mp_bar.set_value(new_mp);
        }
        
        egui_macroquad::ui(|ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                // 标题
                ui.heading("🧪 Crystal公共组件功能测试");
                ui.separator();
                
                // 测试MirButton
                ui.label("🔘 MirButton测试:");
                let button_response = test_button.show(ui, ctx);
                if button_response.clicked {
                    click_count += 1;
                    let new_text = format!("已点击{}次", click_count);
                    test_button.text = Some(new_text);
                    println!("按钮被点击！总计：{}次", click_count);
                }
                
                ui.add_space(10.0);
                
                // 测试MirLabel  
                ui.label("🏷️ MirLabel测试:");
                let status_text = format!("状态: 运行中 | 帧数: {} | 点击: {}次", frame_count, click_count);
                status_label.set_text(&status_text);
                status_label.draw(ui, ctx);
                
                ui.add_space(10.0);
                
                // 测试MirTextBox
                ui.label("📝 MirTextBox测试:");
                ui.horizontal(|ui| {
                    name_input.draw(ui, ctx);
                    password_input.draw(ui, ctx);
                });
                
                // 显示输入内容
                if !name_input.text.is_empty() {
                    ui.label(format!("输入的名字: {}", name_input.text));
                }
                if !password_input.text.is_empty() {
                    ui.label(format!("密码长度: {}位", password_input.text.len()));
                }
                
                ui.add_space(10.0);
                
                // 测试MirListBox和MirProgressBar
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label("📋 MirListBox测试:");
                        items_list.draw(ui, ctx);
                    });
                    
                    ui.vertical(|ui| {
                        ui.label("📊 MirProgressBar测试:");
                        hp_bar.draw(ui, ctx);
                        mp_bar.draw(ui, ctx);
                        exp_bar.draw(ui, ctx);
                        
                        // 显示当前值
                        ui.label(format!("HP: {:.1}/100.0", hp_bar.get_value()));
                        ui.label(format!("MP: {:.1}/100.0", mp_bar.get_value()));
                        ui.label(format!("EXP: {:.1}%", exp_bar.get_percent()));
                    });
                });
                
                ui.separator();
                
                // 组件状态信息
                ui.collapsing("📈 组件状态信息", |ui| {
                    ui.label("✅ 组件实例化状态:");
                    ui.label(format!("• MirButton: ID='{}', 可见={}", test_button.id, test_button.visible()));
                    ui.label(format!("• MirLabel: ID='{}', 位置={:?}", status_label.id, status_label.position()));
                    ui.label(format!("• MirTextBox: 用户名长度={}, 密码长度={}", name_input.text.len(), password_input.text.len()));
                    ui.label(format!("• MirListBox: 项目数={}", items_list.items.len()));
                    ui.label(format!("• MirProgressBar: HP={:.1}, MP={:.1}, EXP={:.1}%", 
                        hp_bar.get_value(), mp_bar.get_value(), exp_bar.get_percent()));
                });
                
                ui.separator();
                
                // 控制面板
                ui.horizontal(|ui| {
                    if ui.button("🔄 重置所有组件").clicked() {
                        click_count = 0;
                        test_button.text = Some("点击我！".to_string());
                        name_input.text.clear();
                        password_input.text.clear();
                        hp_bar.set_value(85.0);
                        mp_bar.set_value(62.0);
                        exp_bar.set_value(34.0);
                        println!("🔄 所有组件已重置");
                    }
                    
                    if ui.button("📊 打印组件信息").clicked() {
                        println!("📊 === 组件状态报告 ===");
                        println!("按钮: ID={}, 点击次数={}", test_button.id, click_count);
                        println!("标签: 位置={:?}, 大小={:?}", status_label.position(), status_label.size());
                        println!("输入框: 用户名='{}', 密码长度={}", name_input.text, password_input.text.len());
                        println!("列表框: 项目数={}", items_list.items.len());
                        println!("进度条: HP={:.1}, MP={:.1}, EXP={:.1}%", 
                            hp_bar.get_value(), mp_bar.get_value(), exp_bar.get_percent());
                        println!("=========================");
                    }
                    
                    if ui.button("❌ 退出测试").clicked() {
                        println!("👋 退出组件测试程序");
                        std::process::exit(0);
                    }
                });
            });
        });
        
        egui_macroquad::draw();
        next_frame().await;
    }
}
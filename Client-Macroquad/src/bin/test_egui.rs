/// egui-macroquad 集成测试
/// 展示如何使用 egui 创建复杂的 UI 界面

use macroquad::prelude::*;
use egui_macroquad::egui;  // 导入 egui

fn window_conf() -> Conf {
    Conf {
        window_title: "egui-macroquad 测试".to_string(),
        window_width: 1024,
        window_height: 768,
        high_dpi: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    // 测试数据
    let mut account_id = String::new();
    let mut password1 = String::new();
    let mut password2 = String::new();
    let mut email = String::new();
    let mut username = String::new();
    let mut birthdate = String::new();
    let mut question = String::new();
    let mut answer = String::new();
    
    let mut show_new_account = false;
    let mut show_demo = false;
    
    loop {
        clear_background(Color::from_rgba(20, 20, 30, 255));
        
        // 绘制背景装饰
        draw_circle(screen_width() * 0.2, screen_height() * 0.3, 100.0, 
                   Color::from_rgba(50, 50, 100, 30));
        draw_circle(screen_width() * 0.8, screen_height() * 0.7, 150.0, 
                   Color::from_rgba(100, 50, 50, 30));
        
        // egui UI
        egui_macroquad::ui(|egui_ctx| {
            // 主菜单窗口
            egui::Window::new("🎮 传奇2 - egui 测试")
                .default_pos([10.0, 10.0])
                .default_size([300.0, 200.0])
                .show(egui_ctx, |ui| {
                    ui.heading("欢迎使用 egui-macroquad");
                    ui.separator();
                    
                    if ui.button("📝 新建账号对话框").clicked() {
                        show_new_account = true;
                    }
                    
                    if ui.button("🎨 显示 egui 演示窗口").clicked() {
                        show_demo = true;
                    }
                    
                    ui.separator();
                    ui.label("提示:");
                    ui.label("• egui 提供丰富的 UI 组件");
                    ui.label("• 支持复杂布局和样式");
                    ui.label("• 与 macroquad 完美集成");
                });
            
            // 新建账号对话框
            if show_new_account {
                egui::Window::new("创建新账号")
                    .default_pos([screen_width() / 2.0 - 200.0, screen_height() / 2.0 - 250.0])
                    .default_size([400.0, 500.0])
                    .collapsible(false)
                    .resizable(false)
                    .show(egui_ctx, |ui| {
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing = egui::vec2(8.0, 12.0);
                            
                            // 账号ID
                            ui.horizontal(|ui| {
                                ui.label("账号ID:");
                                ui.add(egui::TextEdit::singleline(&mut account_id)
                                    .desired_width(250.0));
                            });
                            
                            // 密码1
                            ui.horizontal(|ui| {
                                ui.label("密码:");
                                ui.add(egui::TextEdit::singleline(&mut password1)
                                    .password(true)
                                    .desired_width(250.0));
                            });
                            
                            // 密码2
                            ui.horizontal(|ui| {
                                ui.label("确认密码:");
                                ui.add(egui::TextEdit::singleline(&mut password2)
                                    .password(true)
                                    .desired_width(250.0));
                            });
                            
                            ui.separator();
                            
                            // 用户名
                            ui.horizontal(|ui| {
                                ui.label("用户名:");
                                ui.add(egui::TextEdit::singleline(&mut username)
                                    .desired_width(250.0));
                            });
                            
                            // 生日
                            ui.horizontal(|ui| {
                                ui.label("生日:");
                                ui.add(egui::TextEdit::singleline(&mut birthdate)
                                    .hint_text("YYYY-MM-DD")
                                    .desired_width(250.0));
                            });
                            
                            ui.separator();
                            
                            // 安全问题
                            ui.label("安全问题:");
                            ui.add(egui::TextEdit::singleline(&mut question)
                                .desired_width(350.0));
                            
                            // 答案
                            ui.label("答案:");
                            ui.add(egui::TextEdit::singleline(&mut answer)
                                .desired_width(350.0));
                            
                            ui.separator();
                            
                            // 邮箱
                            ui.horizontal(|ui| {
                                ui.label("邮箱:");
                                ui.add(egui::TextEdit::singleline(&mut email)
                                    .hint_text("example@email.com")
                                    .desired_width(250.0));
                            });
                            
                            ui.add_space(10.0);
                            
                            // 按钮
                            ui.horizontal(|ui| {
                                if ui.button("✅ 创建账号").clicked() {
                                    if validate_account(&account_id, &password1, &password2) {
                                        println!("✅ 创建账号: {}", account_id);
                                        println!("   用户名: {}", username);
                                        println!("   邮箱: {}", email);
                                        show_new_account = false;
                                        clear_form(&mut account_id, &mut password1, &mut password2,
                                                  &mut email, &mut username, &mut birthdate,
                                                  &mut question, &mut answer);
                                    }
                                }
                                
                                if ui.button("❌ 取消").clicked() {
                                    show_new_account = false;
                                    clear_form(&mut account_id, &mut password1, &mut password2,
                                              &mut email, &mut username, &mut birthdate,
                                              &mut question, &mut answer);
                                }
                            });
                        });
                    });
            }
            
            // egui 演示窗口 (可选)
            if show_demo {
                egui::Window::new("egui 组件演示")
                    .open(&mut show_demo)
                    .default_size([400.0, 500.0])
                    .show(egui_ctx, |ui| {
                        ui.heading("各种 UI 组件示例");
                        ui.separator();
                        
                        // 滑块
                        let mut value = 50.0;
                        ui.label("滑块:");
                        ui.add(egui::Slider::new(&mut value, 0.0..=100.0));
                        
                        ui.separator();
                        
                        // 复选框
                        let mut checked = false;
                        ui.checkbox(&mut checked, "复选框示例");
                        
                        ui.separator();
                        
                        // 单选按钮
                        let mut radio = 0;
                        ui.label("单选按钮:");
                        ui.radio_value(&mut radio, 0, "选项 1");
                        ui.radio_value(&mut radio, 1, "选项 2");
                        ui.radio_value(&mut radio, 2, "选项 3");
                        
                        ui.separator();
                        
                        // 颜色选择器
                        let mut color = egui::Color32::from_rgb(100, 150, 200);
                        ui.label("颜色选择器:");
                        ui.color_edit_button_srgba(&mut color);
                        
                        ui.separator();
                        
                        // 进度条
                        ui.label("进度条:");
                        ui.add(egui::ProgressBar::new(0.7).show_percentage());
                    });
            }
        });
        
        // 绘制 egui
        egui_macroquad::draw();
        
        // 在 egui 之后绘制 macroquad 图形
        draw_text(
            &format!("FPS: {:.0}", get_fps()),
            10.0,
            screen_height() - 10.0,
            20.0,
            GREEN,
        );
        
        next_frame().await;
    }
}

/// 验证账号信息
fn validate_account(account_id: &str, password1: &str, password2: &str) -> bool {
    if account_id.is_empty() {
        println!("⚠ 账号不能为空!");
        return false;
    }
    if password1.is_empty() {
        println!("⚠ 密码不能为空!");
        return false;
    }
    if password1 != password2 {
        println!("⚠ 两次密码输入不一致!");
        return false;
    }
    true
}

/// 清空表单
fn clear_form(
    account_id: &mut String,
    password1: &mut String,
    password2: &mut String,
    email: &mut String,
    username: &mut String,
    birthdate: &mut String,
    question: &mut String,
    answer: &mut String,
) {
    account_id.clear();
    password1.clear();
    password2.clear();
    email.clear();
    username.clear();
    birthdate.clear();
    question.clear();
    answer.clear();
}

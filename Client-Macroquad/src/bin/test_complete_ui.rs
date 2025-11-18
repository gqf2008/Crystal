// ============================================================================
// 综合UI组件测试 - 测试所有新的UI组件
// ============================================================================

// FIXME: 此测试文件使用了已删除的组件，需要更新
// use client_macroquad::ui::components::{
//     MirButton, MirLabel, MirTextBox, MirListBox, MirProgressBar,
//     ListItem
};
use macroquad::prelude::*;

#[macroquad::main("综合UI组件测试")]
async fn main() {
    // 创建测试组件
    let mut test_button = MirButton::new("test_btn")
        .with_rect(egui_macroquad::egui::pos2(20.0, 20.0), egui_macroquad::egui::vec2(100.0, 30.0))
        .with_text("测试按钮");
    
    let mut title_label = MirLabel::new("title", "🏠 综合UI组件测试")
        .with_position(egui_macroquad::egui::pos2(20.0, 60.0))
        .with_font_size(18.0)
        .with_color(egui_macroquad::egui::Color32::YELLOW);
    
    let mut name_textbox = MirTextBox::new("name_input")
        .with_rect(egui_macroquad::egui::pos2(20.0, 100.0), egui_macroquad::egui::vec2(200.0, 25.0))
        .with_placeholder("请输入你的名字...");
    
    let mut password_textbox = MirTextBox::new("pass_input")
        .with_rect(egui_macroquad::egui::pos2(20.0, 140.0), egui_macroquad::egui::vec2(200.0, 25.0))
        .with_placeholder("请输入密码...")
        .with_password(true);
    
    let mut item_list = MirListBox::new("item_list")
        .with_rect(egui_macroquad::egui::pos2(250.0, 20.0), egui_macroquad::egui::vec2(200.0, 200.0))
        .with_item_height(25.0);
    
    // 添加一些测试列表项
    item_list.add_item(ListItem::new("item1", "🗡️ 龙纹剑").with_icon(1));
    item_list.add_item(ListItem::new("item2", "🛡️ 魔法盾牌").with_icon(2));
    item_list.add_item(ListItem::new("item3", "🎓 智慧头盔").with_icon(3));
    item_list.add_item(ListItem::new("item4", "🧪 生命药水").with_icon(4));
    item_list.add_item(ListItem::new("item5", "🏃 速度靴子").with_icon(5));
    item_list.add_item(ListItem::new("disabled", "☠️ 禁用物品").with_enabled(false));
    
    let mut health_bar = MirProgressBar::new("health")
        .with_rect(egui_macroquad::egui::pos2(20.0, 200.0), egui_macroquad::egui::vec2(200.0, 20.0))
        .with_range(100.0)
        .with_colors(
            egui_macroquad::egui::Color32::from_rgb(60, 20, 20),
            egui_macroquad::egui::Color32::from_rgb(220, 20, 20)
        )
        .with_text(true, "❤️ {current}/{max}");
    
    let mut mana_bar = MirProgressBar::new("mana")
        .with_rect(egui_macroquad::egui::pos2(20.0, 230.0), egui_macroquad::egui::vec2(200.0, 20.0))
        .with_range(100.0)
        .with_colors(
            egui_macroquad::egui::Color32::from_rgb(20, 20, 60),
            egui_macroquad::egui::Color32::from_rgb(20, 100, 220)
        )
        .with_text(true, "🔮 {percent}%");
    
    let mut exp_bar = MirProgressBar::new("exp")
        .with_rect(egui_macroquad::egui::pos2(20.0, 260.0), egui_macroquad::egui::vec2(200.0, 15.0))
        .with_range(1000.0)
        .with_colors(
            egui_macroquad::egui::Color32::from_rgb(40, 40, 20),
            egui_macroquad::egui::Color32::from_rgb(255, 215, 0)
        )
        .with_text(true, "✨ EXP: {percent}%");
    
    // 设置初始值
    health_bar.set_value(75.0);
    mana_bar.set_value(60.0);
    exp_bar.set_value(350.0);
    
    let mut status_label = MirLabel::new("status", "点击按钮或列表项来交互...")
        .with_position(egui_macroquad::egui::pos2(20.0, 300.0))
        .with_font_size(14.0)
        .with_color(egui_macroquad::egui::Color32::LIGHT_GRAY);
    
    let mut click_count = 0;
    let mut selected_item_name = String::new();
    
    loop {
        clear_background(Color::from_rgba(25, 25, 35, 255));
        
        // 处理按键
        if is_key_pressed(KeyCode::Escape) {
            break;
        }
        
        // 绘制说明
        draw_text("按 ESC 退出", 500.0, 30.0, 16.0, WHITE);
        draw_text("组件功能测试:", 500.0, 60.0, 14.0, LIGHTGRAY);
        draw_text("• 按钮: 可点击", 500.0, 80.0, 12.0, LIGHTGRAY);
        draw_text("• 文本框: 可输入文本", 500.0, 100.0, 12.0, LIGHTGRAY);
        draw_text("• 列表: 可选中、滚动", 500.0, 120.0, 12.0, LIGHTGRAY);
        draw_text("• 进度条: 自动变化", 500.0, 140.0, 12.0, LIGHTGRAY);
        
        // 模拟数值变化
        let time = get_time() as f32;
        health_bar.set_value(50.0 + (time * 0.5).sin() * 40.0);
        mana_bar.set_value(50.0 + (time * 0.8).cos() * 45.0);
        exp_bar.set_value(200.0 + (time * 0.3).sin() * 300.0);
        
        // 绘制UI组件
        egui_macroquad::ui(|ctx| {
            egui_macroquad::egui::Area::new(egui_macroquad::egui::Id::new("test_area"))
                .fixed_pos(egui_macroquad::egui::pos2(0.0, 0.0))
                .show(ctx, |ui| {
                    // 绘制标题
                    title_label.show(ui, ctx);
                    
                    // 绘制按钮
                    let response = test_button.show(ui, ctx);
                    if response.clicked {
                        click_count += 1;
                        status_label.set_text(format!("按钮被点击了 {} 次！", click_count));
                    }
                    
                    // 绘制文本输入框
                    let name_changed = name_textbox.show(ui, ctx);
                    if name_changed {
                        if !name_textbox.get_text().is_empty() {
                            status_label.set_text(format!("你好, {}!", name_textbox.get_text()));
                        }
                    }
                    
                    password_textbox.show(ui, ctx);
                    
                    // 绘制列表
                    if let Some(selected_index) = item_list.show(ui, ctx) {
                        if let Some(item) = item_list.items.get(selected_index) {
                            selected_item_name = item.text.clone();
                            status_label.set_text(format!("选中了: {}", selected_item_name));
                        }
                    }
                    
                    // 绘制进度条
                    health_bar.show(ui, ctx);
                    mana_bar.show(ui, ctx);
                    exp_bar.show(ui, ctx);
                    
                    // 绘制状态标签
                    status_label.show(ui, ctx);
                    
                    // 绘制数据显示
                    ui.painter().text(
                        egui_macroquad::egui::pos2(20.0, 330.0),
                        egui_macroquad::egui::Align2::LEFT_TOP,
                        format!("📊 实时数据:"),
                        egui_macroquad::egui::FontId::proportional(12.0),
                        egui_macroquad::egui::Color32::LIGHT_BLUE,
                    );
                    
                    ui.painter().text(
                        egui_macroquad::egui::pos2(20.0, 350.0),
                        egui_macroquad::egui::Align2::LEFT_TOP,
                        format!("名字: '{}'", name_textbox.get_text()),
                        egui_macroquad::egui::FontId::proportional(11.0),
                        egui_macroquad::egui::Color32::WHITE,
                    );
                    
                    ui.painter().text(
                        egui_macroquad::egui::pos2(20.0, 365.0),
                        egui_macroquad::egui::Align2::LEFT_TOP,
                        format!("密码长度: {}", password_textbox.get_text().len()),
                        egui_macroquad::egui::FontId::proportional(11.0),
                        egui_macroquad::egui::Color32::WHITE,
                    );
                    
                    ui.painter().text(
                        egui_macroquad::egui::pos2(20.0, 380.0),
                        egui_macroquad::egui::Align2::LEFT_TOP,
                        format!("列表项数: {}", item_list.items.len()),
                        egui_macroquad::egui::FontId::proportional(11.0),
                        egui_macroquad::egui::Color32::WHITE,
                    );
                });
        });
        
        egui_macroquad::draw();
        next_frame().await;
    }
}
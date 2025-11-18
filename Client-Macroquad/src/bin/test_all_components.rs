// ============================================================================
// 所有UI组件统一测试程序
// 测试所有公共组件的功能和交互
// ============================================================================

// FIXME: 此测试文件使用了已删除的组件，需要更新
// use client_macroquad::ui::components::{
//     MirButton, MirLabel, MirTextBox, MirListBox, MirProgressBar,
//     MirDialog, ShopItemViewer, ListItem, MirControl
};
use macroquad::prelude::*;
use egui_macroquad::egui;

struct ComponentTestApp {
    // 基础组件
    test_button: MirButton,
    close_button: MirButton,
    info_label: MirLabel,
    title_label: MirLabel,
    status_label: MirLabel,
    
    // 输入组件
    name_textbox: MirTextBox,
    password_textbox: MirTextBox,
    search_textbox: MirTextBox,
    
    // 列表组件
    items_listbox: MirListBox,
    skills_listbox: MirListBox,
    
    // 进度条组件
    hp_progress: MirProgressBar,
    mp_progress: MirProgressBar,
    exp_progress: MirProgressBar,
    loading_progress: MirProgressBar,
    
    // 对话框组件
    test_dialog: MirDialog,
    info_dialog: MirDialog,
    
    // 商店查看器
    shop_viewer: ShopItemViewer,
    
    // 测试状态
    button_click_count: u32,
    progress_values: [f32; 4],
    time_elapsed: f32,
    show_dialog: bool,
    show_info_dialog: bool,
    show_shop: bool,
}

impl ComponentTestApp {
    fn new() -> Self {
        // 创建按钮组件
        let test_button = MirButton::new("test_btn")
            .with_rect(egui::pos2(20.0, 20.0), egui::vec2(120.0, 30.0))
            .with_text("点击测试 (0)");
        
        let close_button = MirButton::new("close_btn")
            .with_rect(egui::pos2(660.0, 20.0), egui::vec2(80.0, 30.0))
            .with_text("退出");
        
        // 创建标签组件
        let title_label = MirLabel::new("title", "🎮 Crystal UI组件统一测试平台")
            .with_position(egui::pos2(200.0, 25.0))
            .with_font_size(20.0)
            .with_color(egui::Color32::from_rgb(255, 215, 0));
        
        let info_label = MirLabel::new("info", "测试所有公共UI组件的功能和交互效果")
            .with_position(egui::pos2(20.0, 60.0))
            .with_font_size(14.0)
            .with_color(egui::Color32::LIGHT_GRAY);
        
        let status_label = MirLabel::new("status", "状态: 就绪")
            .with_position(egui::pos2(20.0, 85.0))
            .with_font_size(12.0)
            .with_color(egui::Color32::GREEN);
        
        // 创建文本输入框
        let name_textbox = MirTextBox::new("name_input")
            .with_rect(egui::pos2(20.0, 120.0), egui::vec2(200.0, 25.0))
            .with_placeholder("请输入用户名...");
        
        let password_textbox = MirTextBox::new("pass_input")
            .with_rect(egui::pos2(240.0, 120.0), egui::vec2(200.0, 25.0))
            .with_placeholder("请输入密码...")
            .with_password(true);
        
        let search_textbox = MirTextBox::new("search_input")
            .with_rect(egui::pos2(460.0, 120.0), egui::vec2(200.0, 25.0))
            .with_placeholder("搜索物品...");
        
        // 创建列表框
        let mut items_listbox = MirListBox::new("items_list")
            .with_rect(egui::pos2(20.0, 160.0), egui::vec2(200.0, 200.0));
        
        // 添加物品列表项
        let items = vec![
            ListItem::new("屠龙刀", "icon_weapon_1"),
            ListItem::new("倚天剑", "icon_weapon_2"),
            ListItem::new("麻痹戒指", "icon_ring_1"),
            ListItem::new("护身戒指", "icon_ring_2"),
            ListItem::new("复活戒指", "icon_ring_3"),
            ListItem::new("超级红药", "icon_potion_1"),
            ListItem::new("超级蓝药", "icon_potion_2"),
            ListItem::new("强效太阳水", "icon_potion_3"),
            ListItem::new("万年雪霜", "icon_potion_4"),
            ListItem::new("地狱雷光", "icon_scroll_1"),
        ];
        for item in items {
            items_listbox.add_item(item);
        }
        
        let mut skills_listbox = MirListBox::new("skills_list")
            .with_rect(egui::pos2(240.0, 160.0), egui::vec2(200.0, 200.0));
        
        // 添加技能列表项
        let skills = vec![
            ListItem::new("基本剑术", "skill_basic_sword"),
            ListItem::new("攻杀剑术", "skill_attack_sword"),
            ListItem::new("刺杀剑术", "skill_thrust_sword"),
            ListItem::new("半月弯刀", "skill_half_moon"),
            ListItem::new("烈火剑法", "skill_fire_sword"),
            ListItem::new("逐日剑法", "skill_sun_sword"),
            ListItem::new("开天斩", "skill_heaven_slash"),
        ];
        for skill in skills {
            skills_listbox.add_item(skill);
        }
        
        // 创建进度条
        let hp_progress = MirProgressBar::new("hp_bar")
            .with_rect(egui::pos2(460.0, 160.0), egui::vec2(200.0, 20.0))
            .with_colors(egui::Color32::RED, egui::Color32::DARK_RED)
            .with_text(true, "HP: {current}/{max}");
        
        let mp_progress = MirProgressBar::new("mp_bar")
            .with_rect(egui::pos2(460.0, 190.0), egui::vec2(200.0, 20.0))
            .with_colors(egui::Color32::BLUE, egui::Color32::DARK_BLUE)
            .with_text(true, "MP: {current}/{max}");
        
        let exp_progress = MirProgressBar::new("exp_bar")
            .with_rect(egui::pos2(460.0, 220.0), egui::vec2(200.0, 20.0))
            .with_colors(egui::Color32::YELLOW, egui::Color32::from_rgb(128, 128, 0))
            .with_text(true, "EXP: {percent}%");
        
        let loading_progress = MirProgressBar::new("loading_bar")
            .with_rect(egui::pos2(460.0, 250.0), egui::vec2(200.0, 15.0))
            .with_colors(egui::Color32::GREEN, egui::Color32::DARK_GREEN)
            .with_text(true, "Loading... {percent}%");
        
        // 创建对话框
        let mut test_dialog = MirDialog::new("test_dialog", "组件测试对话框");
        test_dialog.size = egui::vec2(400.0, 300.0);
        
        let mut info_dialog = MirDialog::new("info_dialog", "组件信息");
        info_dialog.size = egui::vec2(350.0, 250.0);
        
        // 创建商店查看器
        let shop_viewer = ShopItemViewer::new();
        
        Self {
            test_button,
            close_button,
            info_label,
            title_label,
            status_label,
            name_textbox,
            password_textbox,
            search_textbox,
            items_listbox,
            skills_listbox,
            hp_progress,
            mp_progress,
            exp_progress,
            loading_progress,
            test_dialog,
            info_dialog,
            shop_viewer,
            button_click_count: 0,
            progress_values: [80.0, 65.0, 45.0, 0.0],
            time_elapsed: 0.0,
            show_dialog: false,
            show_info_dialog: false,
            show_shop: false,
        }
    }
    
    fn update(&mut self, dt: f32) {
        self.time_elapsed += dt;
        
        // 更新进度条动画
        self.progress_values[3] = ((self.time_elapsed * 0.5).sin() * 0.5 + 0.5) * 100.0;
        
        // 模拟HP/MP变化
        if self.time_elapsed.fract() < 0.01 {
            self.progress_values[0] = (self.progress_values[0] + 0.5) % 100.0;
            self.progress_values[1] = (self.progress_values[1] + 0.3) % 100.0;
        }
        
        // 更新进度条数值
        self.hp_progress.set_value(self.progress_values[0]);
        self.mp_progress.set_value(self.progress_values[1]);
        self.exp_progress.set_value(self.progress_values[2]);
        self.loading_progress.set_value(self.progress_values[3]);
        
        // 更新状态标签
        let status_text = format!(
            "状态: 运行中 | 时间: {:.1}s | 点击: {}次", 
            self.time_elapsed, 
            self.button_click_count
        );
        self.status_label.set_text(&status_text);
    }
    
    fn draw(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // 绘制标题和信息
            self.title_label.draw(ui, ctx);
            self.info_label.draw(ui, ctx);
            self.status_label.draw(ui, ctx);
            
            // 绘制按钮
            let response = self.test_button.show(ui, ctx);
            if response.clicked {
                self.button_click_count += 1;
                self.test_button.text = Some(format!("点击测试 ({})", self.button_click_count));
                
                // 每5次点击显示测试对话框
                if self.button_click_count % 5 == 0 {
                    self.show_dialog = true;
                }
            }
            
            let close_response = self.close_button.show(ui, ctx);
            if close_response.clicked {
                std::process::exit(0);
            }
            
            // 绘制文本输入框
            self.name_textbox.draw(ui, ctx);
            self.password_textbox.draw(ui, ctx);
            self.search_textbox.draw(ui, ctx);
            
            // 绘制列表框
            self.items_listbox.draw(ui, ctx);
            ui.label("物品列表 (请点击选择)");
            
            self.skills_listbox.draw(ui, ctx);
            ui.label("技能列表 (请点击选择)");
            
            // 绘制进度条
            self.hp_progress.draw(ui, ctx);
            self.mp_progress.draw(ui, ctx);
            self.exp_progress.draw(ui, ctx);
            self.loading_progress.draw(ui, ctx);
            
            // 绘制控制按钮区域
            ui.horizontal(|ui| {
                ui.set_min_height(40.0);
                
                if ui.button("🔧 显示测试对话框").clicked() {
                    self.show_dialog = true;
                }
                
                if ui.button("ℹ️ 显示组件信息").clicked() {
                    self.show_info_dialog = true;
                }
                
                if ui.button("🛒 显示商店查看器").clicked() {
                    self.show_shop = true;
                }
                
                if ui.button("🔄 重置所有组件").clicked() {
                    self.reset_components();
                }
            });
            
            // 显示组件统计信息
            ui.separator();
            ui.label("📊 组件统计:");
            ui.label(format!("• 按钮组件: 2个"));
            ui.label(format!("• 标签组件: 3个"));
            ui.label(format!("• 输入框组件: 3个"));
            ui.label(format!("• 列表框组件: 2个 (物品10项, 技能7项)"));
            ui.label(format!("• 进度条组件: 4个"));
            ui.label(format!("• 对话框组件: 2个"));
            ui.label(format!("• 商店查看器: 1个"));
        });
        
        // 绘制对话框
        if self.show_dialog {
            if self.draw_test_dialog(ctx) {
                self.show_dialog = false;
            }
        }
        
        if self.show_info_dialog {
            if self.draw_info_dialog(ctx) {
                self.show_info_dialog = false;
            }
        }
        
        if self.show_shop {
            if self.shop_viewer.draw(ctx) {
                self.show_shop = false;
            }
        }
    }
    
    fn draw_test_dialog(&mut self, ctx: &egui::Context) -> bool {
        let mut should_close = false;
        
        egui::Window::new("🔧 组件测试对话框")
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                ui.label("🎉 恭喜！您已经点击了测试按钮足够多次！");
                ui.separator();
                
                ui.label("组件功能测试:");
                ui.label("✅ 按钮点击响应正常");
                ui.label("✅ 标签文本更新正常");
                ui.label("✅ 进度条动画正常");
                ui.label("✅ 对话框显示正常");
                
                ui.separator();
                
                ui.horizontal(|ui| {
                    if ui.button("确定").clicked() {
                        should_close = true;
                    }
                    if ui.button("取消").clicked() {
                        should_close = true;
                    }
                });
            });
        
        should_close
    }
    
    fn draw_info_dialog(&mut self, ctx: &egui::Context) -> bool {
        let mut should_close = false;
        
        egui::Window::new("ℹ️ 组件信息")
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                ui.label("📋 Crystal UI组件系统信息");
                ui.separator();
                
                ui.label("🏗️ 架构特点:");
                ui.label("• 基于egui框架构建");
                ui.label("• 统一的MirControl接口");
                ui.label("• 支持纹理和颜色渲染");
                ui.label("• 完整的交互事件处理");
                
                ui.separator();
                
                ui.label("🎨 支持的组件:");
                ui.label("• MirButton - 多状态按钮");
                ui.label("• MirLabel - 文本标签");
                ui.label("• MirTextBox - 文本输入框");
                ui.label("• MirListBox - 滚动列表");
                ui.label("• MirProgressBar - 进度条");
                ui.label("• MirDialog - 对话框");
                ui.label("• ShopItemViewer - 商店查看器");
                
                ui.separator();
                
                if ui.button("关闭").clicked() {
                    should_close = true;
                }
            });
        
        should_close
    }
    
    fn reset_components(&mut self) {
        self.button_click_count = 0;
        self.test_button.text = Some("点击测试 (0)".to_string());
        self.name_textbox.text = String::new();
        self.password_textbox.text = String::new();
        self.search_textbox.text = String::new();
        self.progress_values = [80.0, 65.0, 45.0, 0.0];
        self.time_elapsed = 0.0;
    }
}

#[macroquad::main("Crystal UI组件统一测试")]
async fn main() {
    let mut app = ComponentTestApp::new();
    
    loop {
        clear_background(Color::from_rgba(30, 30, 30, 255));
        
        let dt = get_frame_time();
        app.update(dt);
        
        // 绘制egui界面
        egui_macroquad::ui(|ctx| {
            app.draw(ctx);
        });
        
        egui_macroquad::draw();
        
        next_frame().await;
    }
}
// ============================================================================
// 带纹理的UI组件测试程序
// 展示真正的传奇游戏纹理效果
// ============================================================================

// FIXME: 此测试文件使用了已删除的组件，需要更新
// use client_macroquad::ui::components::{
//     MirButton, MirLabel, MirTextBox, MirListBox, MirProgressBar,
//     ListItem, MirControl
};
use client_macroquad::resources::{LibraryName, preload_libraries, set_data_path};
use macroquad::prelude::*;
use egui_macroquad::egui;

struct TexturedUITest {
    // 带纹理的按钮
    login_button: MirButton,
    exit_button: MirButton,
    options_button: MirButton,
    
    // 标签
    title_label: MirLabel,
    status_label: MirLabel,
    
    // 输入框
    username_input: MirTextBox,
    password_input: MirTextBox,
    
    // 列表框
    server_list: MirListBox,
    
    // 进度条
    loading_progress: MirProgressBar,
    
    // 状态
    click_count: u32,
    time: f32,
    textures_loaded: bool,
}

impl TexturedUITest {
    async fn new() -> Self {
        println!("🎨 初始化带纹理的UI测试...");
        
        // 设置数据路径
        set_data_path("Data");
        
        // 预加载核心UI纹理库
        println!("📦 预加载纹理库...");
        let libraries = vec![
            LibraryName::Prguse,
            LibraryName::Prguse2, 
            LibraryName::Title,
            LibraryName::ChrSel,
            LibraryName::BuffIcon,
        ];
        
        let textures_loaded = true;
        preload_libraries(&libraries);
        println!("✅ 纹理库预加载完成！");
        
        // 创建带纹理的登录按钮 (使用Prguse2库中的按钮纹理)
        let login_button = MirButton::new("login_btn")
            .with_library(LibraryName::Prguse2)
            .with_textures(4, Some(5), Some(6)) // 登录按钮的三种状态
            .with_rect(egui::pos2(100.0, 300.0), egui::vec2(120.0, 40.0))
            .with_text("登录游戏");
        
        // 退出按钮
        let exit_button = MirButton::new("exit_btn")
            .with_library(LibraryName::Prguse2)
            .with_textures(7, Some(8), Some(9)) // 退出按钮纹理
            .with_rect(egui::pos2(250.0, 300.0), egui::vec2(120.0, 40.0))
            .with_text("退出游戏");
        
        // 选项按钮
        let options_button = MirButton::new("options_btn")
            .with_library(LibraryName::Prguse2)
            .with_textures(10, Some(11), Some(12)) // 选项按钮纹理
            .with_rect(egui::pos2(400.0, 300.0), egui::vec2(120.0, 40.0))
            .with_text("游戏选项");
        
        // 创建标题标签 (使用Title库的纹理)
        let title_label = MirLabel::new("title", "🏛️ 传奇世界 - 纹理测试版")
            .with_position(egui::pos2(200.0, 50.0))
            .with_font_size(24.0)
            .with_color(egui::Color32::GOLD);
        
        let status_label = MirLabel::new("status", "纹理系统: 初始化中...")
            .with_position(egui::pos2(100.0, 100.0))
            .with_font_size(14.0)
            .with_color(egui::Color32::LIGHT_GREEN);
        
        // 用户名输入框
        let username_input = MirTextBox::new("username")
            .with_rect(egui::pos2(100.0, 150.0), egui::vec2(200.0, 30.0))
            .with_placeholder("请输入用户名...");
        
        // 密码输入框
        let password_input = MirTextBox::new("password")
            .with_rect(egui::pos2(320.0, 150.0), egui::vec2(200.0, 30.0))
            .with_placeholder("请输入密码...")
            .with_password(true);
        
        // 服务器列表
        let mut server_list = MirListBox::new("servers")
            .with_rect(egui::pos2(100.0, 200.0), egui::vec2(250.0, 150.0));
        
        // 添加服务器列表项
        server_list.add_item(ListItem::new("🌟 推荐服务器", "server_1"));
        server_list.add_item(ListItem::new("⚔️  战神服务器", "server_2"));
        server_list.add_item(ListItem::new("🏹 弓箭手服务器", "server_3"));
        server_list.add_item(ListItem::new("🔮 法师服务器", "server_4"));
        server_list.add_item(ListItem::new("⚡ 道士服务器", "server_5"));
        server_list.add_item(ListItem::new("🗡️  刺客服务器", "server_6"));
        
        // 加载进度条
        let loading_progress = MirProgressBar::new("loading")
            .with_rect(egui::pos2(370.0, 200.0), egui::vec2(200.0, 25.0))
            .with_colors(egui::Color32::from_rgb(0, 150, 255), egui::Color32::from_rgb(0, 50, 150))
            .with_text(true, "加载中... {percent}%");
        
        Self {
            login_button,
            exit_button,
            options_button,
            title_label,
            status_label,
            username_input,
            password_input,
            server_list,
            loading_progress,
            click_count: 0,
            time: 0.0,
            textures_loaded,
        }
    }
    
    fn update(&mut self, dt: f32) {
        self.time += dt;
        
        // 更新状态标签
        let status_text = if self.textures_loaded {
            format!("✅ 纹理系统: 已加载 | 运行时间: {:.1}s | 交互: {}次", 
                self.time, self.click_count)
        } else {
            format!("⚠️  纹理系统: 加载失败 | 运行时间: {:.1}s | 交互: {}次", 
                self.time, self.click_count)
        };
        self.status_label.set_text(&status_text);
        
        // 模拟加载进度
        let progress = ((self.time * 20.0).sin() * 0.5 + 0.5) * 100.0;
        self.loading_progress.set_value(progress);
    }
    
    fn draw(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // 背景色
            ui.painter().rect_filled(
                ui.available_rect_before_wrap(),
                0.0,
                egui::Color32::from_rgb(20, 25, 40),
            );
            
            // 绘制标题
            self.title_label.draw(ui, ctx);
            self.status_label.draw(ui, ctx);
            
            ui.add_space(20.0);
            
            // 输入区域
            ui.horizontal(|ui| {
                self.username_input.draw(ui, ctx);
                ui.add_space(20.0);
                self.password_input.draw(ui, ctx);
            });
            
            ui.add_space(20.0);
            
            // 主要内容区域
            ui.horizontal(|ui| {
                // 左侧：服务器列表
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new("🌐 选择服务器").size(16.0).color(egui::Color32::YELLOW));
                    self.server_list.draw(ui, ctx);
                });
                
                ui.add_space(20.0);
                
                // 右侧：系统信息
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new("📊 系统状态").size(16.0).color(egui::Color32::YELLOW));
                    
                    // 加载进度
                    ui.label("资源加载进度:");
                    self.loading_progress.draw(ui, ctx);
                    
                    ui.add_space(10.0);
                    
                    // 纹理信息
                    ui.group(|ui| {
                        ui.label("🎨 纹理库状态:");
                        let status_color = if self.textures_loaded { 
                            egui::Color32::GREEN 
                        } else { 
                            egui::Color32::RED 
                        };
                        
                        ui.colored_label(status_color, format!("• Prguse: {}", 
                            if self.textures_loaded { "✅ 已加载" } else { "❌ 失败" }));
                        ui.colored_label(status_color, format!("• Prguse2: {}", 
                            if self.textures_loaded { "✅ 已加载" } else { "❌ 失败" }));
                        ui.colored_label(status_color, format!("• Title: {}", 
                            if self.textures_loaded { "✅ 已加载" } else { "❌ 失败" }));
                        ui.colored_label(status_color, format!("• ChrSel: {}", 
                            if self.textures_loaded { "✅ 已加载" } else { "❌ 失败" }));
                    });
                });
            });
            
            ui.add_space(30.0);
            
            // 按钮区域
            ui.horizontal(|ui| {
                // 登录按钮
                let login_response = self.login_button.show(ui, ctx);
                if login_response.clicked {
                    self.click_count += 1;
                    println!("🎮 点击登录按钮！");
                }
                
                ui.add_space(20.0);
                
                // 退出按钮
                let exit_response = self.exit_button.show(ui, ctx);
                if exit_response.clicked {
                    println!("👋 用户点击退出");
                    std::process::exit(0);
                }
                
                ui.add_space(20.0);
                
                // 选项按钮
                let options_response = self.options_button.show(ui, ctx);
                if options_response.clicked {
                    self.click_count += 1;
                    println!("⚙️  点击选项按钮！");
                }
            });
            
            ui.separator();
            
            // 调试信息
            ui.collapsing("🔧 调试信息", |ui| {
                ui.label(format!("用户名: '{}'", self.username_input.text));
                ui.label(format!("密码长度: {} 位", self.password_input.text.len()));
                ui.label(format!("加载进度: {:.1}%", self.loading_progress.get_percent()));
                ui.label(format!("服务器列表项: {} 个", self.server_list.items.len()));
                ui.label(format!("按钮交互次数: {}", self.click_count));
                
                if ui.button("🖼️  测试纹理加载").clicked() {
                    println!("🧪 测试纹理系统...");
                    // 可以在这里添加纹理测试代码
                }
            });
        });
    }
}

#[macroquad::main("传奇纹理UI测试")]
async fn main() {
    println!("🚀 启动传奇纹理UI测试程序...");
    
    let mut app = TexturedUITest::new().await;
    
    println!("🎮 进入游戏循环...");
    
    loop {
        clear_background(Color::from_rgba(15, 20, 30, 255));
        
        let dt = get_frame_time();
        app.update(dt);
        
        egui_macroquad::ui(|ctx| {
            app.draw(ctx);
        });
        
        egui_macroquad::draw();
        next_frame().await;
    }
}
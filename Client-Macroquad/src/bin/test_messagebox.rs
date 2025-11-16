// MessageBox 组件测试程序

use client_macroquad::game::GameResult;
use client_macroquad::scenes::dialogs::{MessageBox, MessageBoxButtons, MessageBoxResult};
use macroquad::prelude::*;
use egui_macroquad::egui;

fn window_conf() -> Conf {
    Conf {
        window_title: "MessageBox Component Test".to_owned(),
        window_width: 1024,
        window_height: 768,
        window_resizable: false,
        ..Default::default()
    }
}

struct TestApp {
    // 三种类型的 MessageBox
    ok_box: MessageBox,
    ok_cancel_box: MessageBox,
    yes_no_box: MessageBox,
    
    // 测试结果记录
    last_result: String,
}

impl TestApp {
    fn new() -> GameResult<Self> {
        Ok(Self {
            ok_box: MessageBox::new(
                "信息",
                "这是一个只有 OK 按钮的消息框。\n用于显示简单的提示信息。",
                MessageBoxButtons::Ok
            ),
            ok_cancel_box: MessageBox::new(
                "确认操作",
                "这是一个 OK/Cancel 消息框。\n是否继续执行操作？",
                MessageBoxButtons::OkCancel
            ),
            yes_no_box: MessageBox::new(
                "删除确认",
                "这是一个 Yes/No 消息框。\n确定要删除这个项目吗？",
                MessageBoxButtons::YesNo
            ),
            
            last_result: "等待操作...".to_string(),
        })
    }
    
    fn load_resources(&mut self) -> GameResult {
        println!("📦 加载资源文件...");
        
        // ✅ 使用新 API
        use client_macroquad::resources::LibraryName;
        
        if let Some(lib) = LibraryName::Prguse.get_library() {
            println!("✓ Loaded Prguse: {} images", lib.borrow().count());
        }
        
        if let Some(lib) = LibraryName::Title.get_library() {
            println!("✓ Loaded Title: {} images", lib.borrow().count());
        }
        
        Ok(())
    }
    
    fn update(&mut self) -> GameResult {
        // 检查 MessageBox 结果
        if !self.ok_box.visible {
            let result = self.ok_box.result;
            if result != MessageBoxResult::None {
                self.last_result = format!("OK Box 结果: {:?}", result);
                self.ok_box.result = MessageBoxResult::None;
                println!("{}", self.last_result);
            }
        }
        
        if !self.ok_cancel_box.visible {
            let result = self.ok_cancel_box.result;
            if result != MessageBoxResult::None {
                self.last_result = format!("OkCancel Box 结果: {:?}", result);
                self.ok_cancel_box.result = MessageBoxResult::None;
                println!("{}", self.last_result);
            }
        }
        
        if !self.yes_no_box.visible {
            let result = self.yes_no_box.result;
            if result != MessageBoxResult::None {
                self.last_result = format!("YesNo Box 结果: {:?}", result);
                self.yes_no_box.result = MessageBoxResult::None;
                println!("{}", self.last_result);
            }
        }
        
        Ok(())
    }
    
    fn render(&mut self) -> GameResult {
        clear_background(Color::from_rgba(40, 40, 60, 255));
        
        // egui UI
        egui_macroquad::ui(|ctx| {
            // 主控制面板
            egui::Window::new("MessageBox 测试控制台")
                .fixed_pos(egui::pos2(20.0, 20.0))
                .resizable(false)
                .show(ctx, |ui| {
                    ui.heading("MessageBox 组件完整测试");
                    ui.add_space(10.0);
                    
                    ui.label("点击下面的按钮测试不同类型的消息框：");
                    ui.add_space(10.0);
                    
                    // OK 类型测试
                    ui.horizontal(|ui| {
                        if ui.button("测试 OK 消息框").clicked() {
                            println!("🔵 显示 OK 消息框");
                            self.ok_box = MessageBox::new(
                                "提示信息",
                                "这是一个信息提示框。\n\n只有一个 OK 按钮。\n常用于显示操作结果或提示信息。",
                                MessageBoxButtons::Ok
                            );
                            self.ok_box.show();
                        }
                        ui.label("- 只有 OK 按钮");
                    });
                    
                    ui.add_space(5.0);
                    
                    // OkCancel 类型测试
                    ui.horizontal(|ui| {
                        if ui.button("测试 OkCancel 消息框").clicked() {
                            println!("🟢 显示 OkCancel 消息框");
                            self.ok_cancel_box = MessageBox::new(
                                "操作确认",
                                "即将执行一个重要操作。\n\n这个操作可能会修改数据。\n确定要继续吗？",
                                MessageBoxButtons::OkCancel
                            );
                            self.ok_cancel_box.show();
                        }
                        ui.label("- OK 和 Cancel 按钮");
                    });
                    
                    ui.add_space(5.0);
                    
                    // YesNo 类型测试
                    ui.horizontal(|ui| {
                        if ui.button("测试 YesNo 消息框").clicked() {
                            println!("🟡 显示 YesNo 消息框");
                            self.yes_no_box = MessageBox::new(
                                "删除确认",
                                "确定要删除这个项目吗？\n\n删除后将无法恢复！\n是否继续？",
                                MessageBoxButtons::YesNo
                            );
                            self.yes_no_box.show();
                        }
                        ui.label("- Yes 和 No 按钮");
                    });
                    
                    ui.add_space(15.0);
                    ui.separator();
                    ui.add_space(10.0);
                    
                    // 显示最后的结果
                    ui.label("最后操作结果：");
                    ui.label(
                        egui::RichText::new(&self.last_result)
                            .color(egui::Color32::YELLOW)
                            .size(14.0)
                    );
                    
                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(10.0);
                    
                    // 测试说明
                    ui.label("测试要点：");
                    ui.label("• 按钮位置应该正确（X=260/360, Y=157）");
                    ui.label("• 按钮图片索引应该正确");
                    ui.label("• 鼠标悬停和点击效果正常");
                    ui.label("• ESC 键可以关闭消息框");
                    ui.label("• 消息框可以拖动");
                    ui.label("• result 字段正确返回点击结果");
                });
            
            // 绘制 MessageBox（如果可见）
            if self.ok_box.visible {
                self.ok_box.draw(ctx);
            }
            
            if self.ok_cancel_box.visible {
                self.ok_cancel_box.draw(ctx);
            }
            
            if self.yes_no_box.visible {
                self.yes_no_box.draw(ctx);
            }
        });
        
        egui_macroquad::draw();
        
        Ok(())
    }
}

#[macroquad::main(window_conf)]
async fn main() -> GameResult {
    println!("🎮 MessageBox 组件完整测试");
    println!("📐 窗口尺寸: {}x{}", screen_width(), screen_height());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // 配置 egui 字体和 DPI
    egui_macroquad::cfg(|ctx| {
        let mut fonts = egui::FontDefinitions::default();
        
        // 加载中文字体
        let font_data = std::fs::read("assets/fonts/AlibabaPuHuiTi-3-55-Regular.ttf")
            .or_else(|_| std::fs::read("assets/fonts/Chinese.ttc"))
            .unwrap_or_else(|_| {
                println!("⚠ 无法加载中文字体，使用默认字体");
                vec![]
            });
        
        if !font_data.is_empty() {
            fonts.font_data.insert(
                "chinese".to_owned(),
                std::sync::Arc::new(egui::FontData::from_owned(font_data)),
            );
            
            // 设置字体优先级
            fonts.families.get_mut(&egui::FontFamily::Proportional)
                .unwrap()
                .insert(0, "chinese".to_owned());
            
            fonts.families.get_mut(&egui::FontFamily::Monospace)
                .unwrap()
                .insert(0, "chinese".to_owned());
        }
        
        ctx.set_fonts(fonts);
        
        // 设置 DPI 缩放 - 使 egui 与 macroquad 坐标系统对齐
        ctx.set_pixels_per_point(screen_dpi_scale());
        
        // 设置全局字体大小
        let mut style = (*ctx.style()).clone();
        style.text_styles = [
            (egui::TextStyle::Heading, egui::FontId::new(24.0, egui::FontFamily::Proportional)),
            (egui::TextStyle::Body, egui::FontId::new(16.0, egui::FontFamily::Proportional)),
            (egui::TextStyle::Monospace, egui::FontId::new(14.0, egui::FontFamily::Monospace)),
            (egui::TextStyle::Button, egui::FontId::new(16.0, egui::FontFamily::Proportional)),
            (egui::TextStyle::Small, egui::FontId::new(12.0, egui::FontFamily::Proportional)),
        ].into();
        ctx.set_style(style);
    });
    
    let mut app = TestApp::new()?;
    app.load_resources()?;
    
    println!("\n💡 测试说明:");
    println!("  1. 点击左侧面板按钮测试不同类型的消息框");
    println!("  2. 测试按钮的正常/悬停/按下状态");
    println!("  3. 测试 ESC 键关闭功能");
    println!("  4. 测试拖动功能");
    println!("  5. 观察返回结果是否正确");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    
    loop {
        app.update()?;
        app.render()?;
        next_frame().await;
    }
}

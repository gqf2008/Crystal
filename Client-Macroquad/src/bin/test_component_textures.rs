// ============================================================================
// 公共组件纹理测试 - 验证组件是否真的能显示纹理
// ============================================================================

// FIXME: 此测试文件使用了已删除的组件，需要更新
// use client_macroquad::ui::components::{MirButton, MirLabel, MirControl};
use client_macroquad::resources::{LibraryName, set_data_path, load_library};
use macroquad::prelude::*;
use egui_macroquad::egui;

struct TextureComponentTest {
    // 不同纹理库的按钮
    prguse_button: MirButton,
    prguse2_button: MirButton,
    title_button: MirButton,
    chrsel_button: MirButton,
    
    // 状态标签
    status_label: MirLabel,
    debug_label: MirLabel,
    
    // 测试状态
    click_count: u32,
    current_test_index: usize,
}

impl TextureComponentTest {
    fn new() -> Self {
        println!("🖼️  初始化纹理组件测试...");
        
        // 设置数据路径
        set_data_path("Data");
        
        // 手动加载纹理库
        println!("📦 手动加载纹理库...");
        for lib in [LibraryName::Prguse, LibraryName::Prguse2, LibraryName::Title, LibraryName::ChrSel] {
            match load_library(lib) {
                Ok(_) => println!("✅ {:?} 加载成功", lib),
                Err(e) => println!("❌ {:?} 加载失败: {}", lib, e),
            }
        }
        
        // 创建使用不同纹理库的按钮
        let prguse_button = MirButton::new("prguse_btn")
            .with_library(LibraryName::Prguse)
            .with_textures(0, Some(1), Some(2)) // 使用前几个纹理索引
            .with_rect(egui::pos2(50.0, 100.0), egui::vec2(120.0, 40.0))
            .with_text("Prguse按钮");
        
        let prguse2_button = MirButton::new("prguse2_btn")
            .with_library(LibraryName::Prguse2)
            .with_textures(0, Some(1), Some(2))
            .with_rect(egui::pos2(200.0, 100.0), egui::vec2(120.0, 40.0))
            .with_text("Prguse2按钮");
        
        let title_button = MirButton::new("title_btn")
            .with_library(LibraryName::Title)
            .with_textures(0, Some(1), Some(2))
            .with_rect(egui::pos2(350.0, 100.0), egui::vec2(120.0, 40.0))
            .with_text("Title按钮");
        
        let chrsel_button = MirButton::new("chrsel_btn")
            .with_library(LibraryName::ChrSel)
            .with_textures(0, Some(1), Some(2))
            .with_rect(egui::pos2(500.0, 100.0), egui::vec2(120.0, 40.0))
            .with_text("ChrSel按钮");
        
        let status_label = MirLabel::new("status", "🔍 纹理状态检测中...")
            .with_position(egui::pos2(50.0, 50.0))
            .with_font_size(16.0)
            .with_color(egui::Color32::YELLOW);
        
        let debug_label = MirLabel::new("debug", "调试信息将在这里显示")
            .with_position(egui::pos2(50.0, 200.0))
            .with_font_size(12.0)
            .with_color(egui::Color32::LIGHT_GRAY);
        
        Self {
            prguse_button,
            prguse2_button,
            title_button,
            chrsel_button,
            status_label,
            debug_label,
            click_count: 0,
            current_test_index: 0,
        }
    }
    
    fn test_texture_availability(&mut self, ctx: &egui::Context) {
        let libraries = vec![
            (LibraryName::Prguse, "Prguse"),
            (LibraryName::Prguse2, "Prguse2"),
            (LibraryName::Title, "Title"),
            (LibraryName::ChrSel, "ChrSel"),
            (LibraryName::Items, "Items"),
        ];
        
        let mut status_parts = Vec::new();
        let mut debug_parts = Vec::new();
        
        for (lib, name) in libraries {
            // 测试多个纹理索引
            let mut available_textures = 0;
            for i in 0..10 {
                if let Some(_) = lib.get_egui_texture(ctx, i) {
                    available_textures += 1;
                }
            }
            
            if available_textures > 0 {
                status_parts.push(format!("{}✅", name));
                debug_parts.push(format!("{}: {}个纹理可用", name, available_textures));
            } else {
                status_parts.push(format!("{}❌", name));
                debug_parts.push(format!("{}: 无可用纹理", name));
            }
        }
        
        let status_text = format!("🎨 纹理库状态: {}", status_parts.join(" | "));
        self.status_label.set_text(&status_text);
        
        let debug_text = format!("🔍 详细: {} | 点击: {}次", 
            debug_parts.join(" • "), self.click_count);
        self.debug_label.set_text(&debug_text);
    }
    
    fn draw(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // 背景
            ui.painter().rect_filled(
                ui.available_rect_before_wrap(),
                0.0,
                egui::Color32::from_rgb(15, 20, 25),
            );
            
            ui.heading("🖼️  公共组件纹理显示测试");
            ui.separator();
            
            // 显示状态
            self.status_label.draw(ui, ctx);
            
            ui.add_space(20.0);
            
            // 纹理按钮测试区域
            ui.label("📱 纹理按钮测试 (如果显示彩色纹理说明成功，灰色矩形说明纹理加载失败):");
            
            // 绘制所有按钮
            let buttons = [
                &mut self.prguse_button,
                &mut self.prguse2_button, 
                &mut self.title_button,
                &mut self.chrsel_button,
            ];
            
            for button in buttons {
                let response = button.show(ui, ctx);
                if response.clicked {
                    self.click_count += 1;
                    println!("🖱️  点击了按钮: {}", button.id);
                }
            }
            
            ui.add_space(20.0);
            
            // 显示调试信息
            self.debug_label.draw(ui, ctx);
            
            ui.separator();
            
            // 纹理索引测试区域
            ui.label("🎯 纹理索引测试:");
            
            ui.horizontal(|ui| {
                if ui.button("⬅️ 上一组纹理").clicked() {
                    self.current_test_index = self.current_test_index.saturating_sub(10);
                    self.update_button_textures();
                }
                
                if ui.button("➡️ 下一组纹理").clicked() {
                    self.current_test_index += 10;
                    self.update_button_textures();
                }
                
                ui.label(format!("当前测试索引: {}-{}", 
                    self.current_test_index, 
                    self.current_test_index + 2));
            });
            
            ui.separator();
            
            // 直接纹理显示测试
            ui.label("🔍 直接纹理显示测试:");
            ui.horizontal(|ui| {
                let test_libs = [
                    (LibraryName::Prguse, "Prguse"),
                    (LibraryName::Prguse2, "Prguse2"),
                    (LibraryName::Title, "Title"),
                ];
                
                for (lib, name) in test_libs {
                    ui.vertical(|ui| {
                        ui.label(name);
                        
                        // 显示前3个纹理
                        for i in 0..3 {
                            if let Some(texture_info) = lib.get_egui_texture(ctx, i) {
                                if let Some(egui_texture) = texture_info.egui_texture {
                                    let size = egui::vec2(32.0, 32.0);
                                    ui.image(egui::ImageSource::Texture(egui::load::SizedTexture {
                                        id: egui_texture.id(),
                                        size,
                                    }));
                                    ui.small(format!("{}x{}", texture_info.width, texture_info.height));
                                } else {
                                    ui.colored_label(egui::Color32::RED, format!("纹理{}无效", i));
                                }
                            } else {
                                ui.colored_label(egui::Color32::GRAY, format!("纹理{}不存在", i));
                            }
                        }
                    });
                    ui.separator();
                }
            });
            
            ui.separator();
            
            // 控制面板
            ui.horizontal(|ui| {
                if ui.button("🔄 重新检测纹理").clicked() {
                    self.test_texture_availability(ctx);
                }
                
                if ui.button("📊 输出详细日志").clicked() {
                    self.print_detailed_log(ctx);
                }
                
                if ui.button("❌ 退出测试").clicked() {
                    std::process::exit(0);
                }
            });
        });
    }
    
    fn update_button_textures(&mut self) {
        let base_index = self.current_test_index;
        
        // 更新所有按钮的纹理索引
        self.prguse_button.index = base_index;
        self.prguse_button.hover_index = Some(base_index + 1);
        self.prguse_button.pressed_index = Some(base_index + 2);
        
        self.prguse2_button.index = base_index;
        self.prguse2_button.hover_index = Some(base_index + 1);
        self.prguse2_button.pressed_index = Some(base_index + 2);
        
        self.title_button.index = base_index;
        self.title_button.hover_index = Some(base_index + 1);
        self.title_button.pressed_index = Some(base_index + 2);
        
        self.chrsel_button.index = base_index;
        self.chrsel_button.hover_index = Some(base_index + 1);
        self.chrsel_button.pressed_index = Some(base_index + 2);
        
        println!("🔄 更新按钮纹理索引到: {}", base_index);
    }
    
    fn print_detailed_log(&self, ctx: &egui::Context) {
        println!("\n📊 === 详细纹理测试报告 ===");
        
        let libraries = vec![
            LibraryName::Prguse,
            LibraryName::Prguse2,
            LibraryName::Title,
            LibraryName::ChrSel,
            LibraryName::Items,
            LibraryName::BuffIcon,
        ];
        
        for lib in libraries {
            println!("🎨 {:?}:", lib);
            let mut found_textures = 0;
            
            for i in 0..20 {
                if let Some(texture_info) = lib.get_egui_texture(ctx, i) {
                    found_textures += 1;
                    println!("  ✅ 索引{}: {}x{} {:?}", 
                        i, texture_info.width, texture_info.height,
                        texture_info.egui_texture.is_some());
                } else if i < 5 {
                    println!("  ❌ 索引{}: 不存在", i);
                }
            }
            
            if found_textures == 0 {
                println!("  🚫 该库无可用纹理");
            } else {
                println!("  📈 总计发现 {} 个纹理", found_textures);
            }
            println!();
        }
        
        println!("🎯 按钮点击统计: {} 次", self.click_count);
        println!("================================\n");
    }
}

#[macroquad::main("公共组件纹理测试")]
async fn main() {
    println!("🚀 启动公共组件纹理测试程序...");
    
    let mut app = TextureComponentTest::new();
    
    loop {
        clear_background(Color::from_rgba(10, 15, 20, 255));
        
        egui_macroquad::ui(|ctx| {
            // 每帧检测纹理状态
            app.test_texture_availability(ctx);
            app.draw(ctx);
        });
        
        egui_macroquad::draw();
        next_frame().await;
    }
}
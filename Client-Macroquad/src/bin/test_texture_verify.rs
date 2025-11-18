// ============================================================================
// 直接纹理验证测试程序
// 用于验证纹理库是否能正确加载和显示
// ============================================================================

use client_macroquad::resources::{LibraryName, set_data_path};
use macroquad::prelude::*;
use egui_macroquad::egui;

#[macroquad::main("纹理验证测试")]
async fn main() {
    println!("🖼️  启动纹理验证测试...");
    
    // 设置数据路径
    set_data_path("Data");
    println!("📁 数据路径设置为: Data/");
    
    let mut frame_count = 0;
    
    loop {
        clear_background(Color::from_rgba(25, 30, 45, 255));
        frame_count += 1;
        
        egui_macroquad::ui(|ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.heading("🖼️  Crystal纹理验证测试");
                ui.separator();
                
                ui.label(format!("帧数: {}", frame_count));
                ui.label("数据路径: Data/");
                
                ui.separator();
                
                ui.label("🎨 纹理库测试:");
                
                // 测试不同的纹理库
                let test_libraries = vec![
                    (LibraryName::Prguse, "Prguse.Lib", "主要UI库"),
                    (LibraryName::Prguse2, "Prguse2.Lib", "次要UI库"),  
                    (LibraryName::Title, "Title.Lib", "标题库"),
                    (LibraryName::ChrSel, "ChrSel.Lib", "角色选择库"),
                    (LibraryName::Items, "Items.Lib", "物品库"),
                ];
                
                for (lib, filename, desc) in test_libraries {
                    ui.horizontal(|ui| {
                        ui.label(format!("• {}: ", desc));
                        
                        // 尝试获取第一个纹理
                        if let Some(texture_info) = lib.get_egui_texture(ctx, 0) {
                            ui.colored_label(egui::Color32::GREEN, "✅ 可用");
                            
                            if let Some(egui_texture) = texture_info.egui_texture {
                                ui.label(format!("大小：{}x{}", 
                                    texture_info.width, 
                                    texture_info.height
                                ));
                                
                                // 显示小图标预览
                                let image_size = egui::vec2(32.0, 32.0);
                                ui.image(egui::ImageSource::Texture(egui::load::SizedTexture {
                                    id: egui_texture.id(),
                                    size: image_size,
                                }));
                            }
                        } else {
                            ui.colored_label(egui::Color32::RED, "❌ 不可用");
                            ui.label(format!("({})", filename));
                        }
                    });
                }
                
                ui.separator();
                
                ui.label("🔍 详细测试:");
                
                // 测试Prguse2的多个纹理索引
                ui.label("Prguse2库纹理索引测试:");
                ui.horizontal(|ui| {
                    for i in 0..10 {
                        if let Some(texture_info) = LibraryName::Prguse2.get_egui_texture(ctx, i) {
                            if let Some(egui_texture) = texture_info.egui_texture {
                                let image_size = egui::vec2(24.0, 24.0);
                                ui.image(egui::ImageSource::Texture(egui::load::SizedTexture {
                                    id: egui_texture.id(),
                                    size: image_size,
                                }));
                                ui.label(format!("{}", i));
                            }
                        } else {
                            ui.colored_label(egui::Color32::RED, format!("×{}", i));
                        }
                    }
                });
                
                ui.separator();
                
                // 控制面板
                ui.horizontal(|ui| {
                    if ui.button("🔄 重新测试").clicked() {
                        println!("🔄 重新测试纹理库...");
                    }
                    
                    if ui.button("📊 输出日志").clicked() {
                        println!("📊 === 纹理测试日志 ===");
                        println!("数据路径: Data/");
                        println!("测试帧数: {}", frame_count);
                        
                        let test_libs = vec![
                            LibraryName::Prguse,
                            LibraryName::Prguse2,
                            LibraryName::Title,
                            LibraryName::Items,
                        ];
                        
                        for lib in test_libs {
                            match lib.get_egui_texture(ctx, 0) {
                                Some(info) => {
                                    println!("{:?}: ✅ {}x{}", lib, info.width, info.height);
                                }
                                None => {
                                    println!("{:?}: ❌ 加载失败", lib);
                                }
                            }
                        }
                        println!("=======================");
                    }
                    
                    if ui.button("❌ 退出").clicked() {
                        println!("👋 退出纹理验证测试");
                        std::process::exit(0);
                    }
                });
                
                ui.separator();
                ui.small("💡 提示：如果看到绿色✅说明纹理库加载成功，如果看到图标预览说明纹理显示正常");
            });
        });
        
        egui_macroquad::draw();
        next_frame().await;
    }
}